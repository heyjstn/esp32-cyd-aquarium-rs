//! XPT2046 resistive touch reader, bit-banged exactly like CydTouch.cpp.

use anyhow::Result;
use esp_idf_hal::gpio::{Gpio25, Gpio32, Gpio33, Gpio36, Gpio39, Input, Output, PinDriver};
use esp_idf_hal::peripheral::Peripheral;

use crate::consts;
use crate::touch::map_raw_axis;

#[derive(Clone, Copy, Debug, Default)]
pub struct TouchPoint {
    pub pressed: bool,
    pub raw_x: u16,
    pub raw_y: u16,
    pub screen_x: u16,
    pub screen_y: u16,
    pub logical_x: u16,
    pub logical_y: u16,
}

pub struct CydTouch {
    cs: PinDriver<'static, Gpio33, Output>,
    sclk: PinDriver<'static, Gpio25, Output>,
    mosi: PinDriver<'static, Gpio32, Output>,
    miso: PinDriver<'static, Gpio39, Input>,
    irq: PinDriver<'static, Gpio36, Input>,
    point: TouchPoint,
    previous_pressed: bool,
}

impl CydTouch {
    pub fn new(
        cs: impl Peripheral<P = Gpio33> + 'static,
        sclk: impl Peripheral<P = Gpio25> + 'static,
        mosi: impl Peripheral<P = Gpio32> + 'static,
        miso: impl Peripheral<P = Gpio39> + 'static,
        irq: impl Peripheral<P = Gpio36> + 'static,
    ) -> Result<Self> {
        let mut cs = PinDriver::output(cs)?;
        let mut sclk = PinDriver::output(sclk)?;
        let mut mosi = PinDriver::output(mosi)?;
        let miso = PinDriver::input(miso)?;
        let irq = PinDriver::input(irq)?;

        cs.set_high()?;
        sclk.set_low()?;
        mosi.set_low()?;

        Ok(Self {
            cs,
            sclk,
            mosi,
            miso,
            irq,
            point: TouchPoint::default(),
            previous_pressed: false,
        })
    }

    fn delay_us(us: u64) {
        let start = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
        while unsafe { esp_idf_svc::sys::esp_timer_get_time() } - start < us as i64 {
            core::hint::spin_loop();
        }
    }

    fn read_raw_axis(&mut self, command: u8) -> Result<u16> {
        let mut value: u16 = 0;

        self.cs.set_low()?;
        Self::delay_us(2);

        for bit in (0..8).rev() {
            if (command >> bit) & 0x01 != 0 {
                self.mosi.set_high()?;
            } else {
                self.mosi.set_low()?;
            }
            self.sclk.set_high()?;
            Self::delay_us(1);
            self.sclk.set_low()?;
            Self::delay_us(1);
        }

        for _ in (0..16).rev() {
            self.sclk.set_high()?;
            Self::delay_us(1);
            value <<= 1;
            if self.miso.is_high() {
                value |= 1;
            }
            self.sclk.set_low()?;
            Self::delay_us(1);
        }

        self.cs.set_high()?;
        Ok((value >> 3) & 0x0FFF)
    }

    pub fn update(&mut self) -> Result<TouchPoint> {
        self.previous_pressed = self.point.pressed;
        self.point.pressed = self.irq.is_low();

        if !self.point.pressed {
            return Ok(self.point);
        }

        self.point.raw_x = self.read_raw_axis(0xD0)?;
        self.point.raw_y = self.read_raw_axis(0x90)?;

        let mut screen_x = map_raw_axis(
            self.point.raw_x,
            consts::TOUCH_RAW_MIN_X,
            consts::TOUCH_RAW_MAX_X,
            consts::PHYSICAL_WIDTH - 1,
            false,
        );
        let mut screen_y = map_raw_axis(
            self.point.raw_y,
            consts::TOUCH_RAW_MIN_Y,
            consts::TOUCH_RAW_MAX_Y,
            consts::PHYSICAL_HEIGHT - 1,
            false,
        );
        if consts::TOUCH_ROTATE_180 {
            screen_x = consts::PHYSICAL_WIDTH - 1 - screen_x;
            screen_y = consts::PHYSICAL_HEIGHT - 1 - screen_y;
        }

        self.point.screen_x = screen_x;
        self.point.screen_y = screen_y;
        self.point.logical_x = consts::logical_x_from_screen(screen_x as i32);
        self.point.logical_y = consts::logical_y_from_screen(screen_y as i32);

        Ok(self.point)
    }

    pub fn is_pressed(&self) -> bool {
        self.point.pressed
    }

    pub fn pressed_started(&self) -> bool {
        self.point.pressed && !self.previous_pressed
    }

    pub fn pressed_released(&self) -> bool {
        !self.point.pressed && self.previous_pressed
    }
}
