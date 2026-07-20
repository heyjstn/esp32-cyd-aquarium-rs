//! ILI9341 display driver (mipidsi over an ESP-IDF SPI device) and the LEDC
//! backlight PWM, replacing TFT_eSPI + CydBacklightPwm.

use anyhow::Result as AnyResult;
use core::fmt::Debug;
use embedded_graphics_core::pixelcolor::Rgb565;
use esp_idf_hal::gpio::{AnyInputPin, Gpio13, Gpio14, Gpio15, Gpio2, Gpio21, Output, PinDriver};
use esp_idf_hal::ledc::{
    config::TimerConfig, LedcDriver, LedcTimerDriver, Resolution, CHANNEL0, TIMER0,
};
use esp_idf_hal::spi::{config as spi_config, SpiDeviceDriver, SpiDriver, SpiDriverConfig, SPI2};
use esp_idf_hal::units::FromValueType;
use esp_idf_svc::sys::EspError;
use mipidsi::dcs::SetAddressMode;
use mipidsi::interface::{Interface, InterfaceKind};
use mipidsi::models::{Model, ModelInitError};
use mipidsi::options::{ColorOrder, Orientation};
use mipidsi::{Builder, Display, NoResetPin};

use crate::backlight::backlight_percent_to_duty;
use crate::color::embedded_rgb565;
use crate::consts;
use crate::renderer::FrameSink;

/// mipidsi command/data interface implemented directly on the ESP-IDF SPI
/// device driver (avoids embedded-hal version bridging).
struct EspSpiInterface {
    spi: SpiDeviceDriver<'static, SpiDriver<'static>>,
    dc: PinDriver<'static, Output>,
}

impl Interface for EspSpiInterface {
    type Word = u8;
    type Error = EspError;

    const KIND: InterfaceKind = InterfaceKind::Serial4Line;

    fn send_command(&mut self, command: u8, args: &[u8]) -> Result<(), Self::Error> {
        self.dc.set_low()?;
        self.spi.write(&[command])?;
        if !args.is_empty() {
            self.dc.set_high()?;
            self.spi.write(args)?;
        }
        Ok(())
    }

    fn send_pixels<const N: usize>(
        &mut self,
        pixels: impl IntoIterator<Item = [u8; N]>,
    ) -> Result<(), Self::Error> {
        self.dc.set_high()?;
        let mut buf = [0u8; 512];
        let mut filled = 0usize;
        for pixel in pixels {
            buf[filled..filled + N].copy_from_slice(&pixel);
            filled += N;
            if filled + N > buf.len() {
                self.spi.write(&buf[..filled])?;
                filled = 0;
            }
        }
        if filled > 0 {
            self.spi.write(&buf[..filled])?;
        }
        Ok(())
    }

    fn send_repeated_pixel<const N: usize>(
        &mut self,
        pixel: [u8; N],
        count: u32,
    ) -> Result<(), Self::Error> {
        self.dc.set_high()?;
        let mut buf = [0u8; 512];
        for chunk in buf.chunks_exact_mut(N) {
            chunk.copy_from_slice(&pixel);
        }
        let per_write = buf.len() / N;
        let mut remaining = count as usize;
        while remaining > 0 {
            let n = remaining.min(per_write);
            self.spi.write(&buf[..n * N])?;
            remaining -= n;
        }
        Ok(())
    }
}

impl Debug for EspSpiInterface {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EspSpiInterface").finish_non_exhaustive()
    }
}

/// Busy-wait delay implementing embedded-hal 1.x DelayNs for mipidsi's init.
struct SysDelay;

impl embedded_hal::delay::DelayNs for SysDelay {
    fn delay_ns(&mut self, ns: u32) {
        let us = (ns as u64).div_ceil(1000);
        let start = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
        while unsafe { esp_idf_svc::sys::esp_timer_get_time() } - start < us as i64 {
            core::hint::spin_loop();
        }
    }
}

struct CydIli9341;

impl Model for CydIli9341 {
    type ColorFormat = Rgb565;

    const FRAMEBUFFER_SIZE: (u16, u16) = (consts::PHYSICAL_WIDTH, consts::PHYSICAL_HEIGHT);

    fn init<DELAY, DI>(
        &mut self,
        interface: &mut DI,
        delay: &mut DELAY,
        options: &mipidsi::options::ModelOptions,
    ) -> Result<SetAddressMode, ModelInitError<DI::Error>>
    where
        DELAY: embedded_hal::delay::DelayNs,
        DI: Interface,
    {
        delay.delay_ms(crate::panel::RESET_DELAY_MS.into());
        for step in crate::panel::ILI9341_2_INIT {
            interface
                .send_command(step.command, step.args)
                .map_err(ModelInitError::Interface)?;
            delay.delay_ms(step.delay_after_ms.into());
        }

        let madctl = SetAddressMode::from(options);
        interface
            .send_command(
                crate::panel::ROTATION_2.command,
                crate::panel::ROTATION_2.args,
            )
            .map_err(ModelInitError::Interface)?;
        Ok(madctl)
    }
}

type MipidsiDisplay = Display<EspSpiInterface, CydIli9341, NoResetPin>;

pub struct CydDisplay {
    display: MipidsiDisplay,
    backlight: LedcDriver<'static>,
}

impl CydDisplay {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spi2: SPI2<'static>,
        sclk: Gpio14<'static>,
        sdo: Gpio13<'static>,
        cs: Gpio15<'static>,
        dc: Gpio2<'static>,
        timer: TIMER0<'static>,
        channel: CHANNEL0<'static>,
        backlight_pin: Gpio21<'static>,
    ) -> AnyResult<Self> {
        let spi_driver = SpiDriver::new(
            spi2,
            sclk,
            sdo,
            Option::<AnyInputPin<'static>>::None,
            &SpiDriverConfig::new(),
        )?;
        let spi_config = spi_config::Config::new()
            .baudrate(consts::TFT_SPI_HZ.Hz())
            .data_mode(spi_config::MODE_0);
        // The device driver owns the bus driver (T: Borrow<SpiDriver>).
        let spi_device = SpiDeviceDriver::new(spi_driver, Some(cs), &spi_config)?;

        let dc_pin = PinDriver::output(dc)?;
        let interface = EspSpiInterface {
            spi: spi_device,
            dc: dc_pin,
        };

        let mut delay = SysDelay;
        let display = Builder::new(CydIli9341, interface)
            .display_size(consts::PHYSICAL_WIDTH, consts::PHYSICAL_HEIGHT)
            .orientation(Orientation::new().flip_vertical())
            .color_order(ColorOrder::Bgr)
            .init(&mut delay)
            .map_err(|e| anyhow::anyhow!("display init failed: {e:?}"))?;

        let timer_config = TimerConfig::new()
            .frequency(consts::BACKLIGHT_PWM_FREQ_HZ.Hz())
            .resolution(Resolution::Bits8);
        let timer_driver = LedcTimerDriver::new(timer, &timer_config)?;
        let backlight = LedcDriver::new(channel, timer_driver, backlight_pin)?;

        Ok(Self { display, backlight })
    }

    pub fn set_backlight_percent(&mut self, percent: u8) -> AnyResult<()> {
        let max_duty = self.backlight.get_max_duty();
        let duty = backlight_percent_to_duty(percent, max_duty, consts::TFT_BACKLIGHT_ON_HIGH);
        self.backlight.set_duty(duty)?;
        Ok(())
    }
}

impl FrameSink for CydDisplay {
    type Error = EspError;

    fn fill_screen(&mut self, color: u16) -> Result<(), Self::Error> {
        self.display.set_pixels(
            0,
            0,
            consts::PHYSICAL_WIDTH - 1,
            consts::PHYSICAL_HEIGHT - 1,
            core::iter::repeat_n(
                embedded_rgb565(color),
                consts::PHYSICAL_WIDTH as usize * consts::PHYSICAL_HEIGHT as usize,
            ),
        )
    }

    fn push_rect(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
        pixels: &[u16],
    ) -> Result<(), Self::Error> {
        debug_assert_eq!(pixels.len(), w as usize * h as usize);
        self.display.set_pixels(
            x,
            y,
            x + w - 1,
            y + h - 1,
            pixels.iter().copied().map(embedded_rgb565),
        )
    }
}
