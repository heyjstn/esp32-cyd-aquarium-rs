//! Onboard LDR light sensor on GPIO34 (ADC1 channel 6), read through the
//! ESP-IDF oneshot ADC driver with 0dB attenuation, matching the C++ build's
//! analogRead + ADC_0db configuration.

use anyhow::Result;
use esp_idf_hal::adc::oneshot::config::AdcChannelConfig;
use esp_idf_hal::adc::oneshot::{AdcChannelDriver, AdcDriver};
use esp_idf_hal::adc::ADC1;
use esp_idf_hal::gpio::Gpio34;
use esp_idf_hal::peripheral::Peripheral;

const SAMPLE_COUNT: usize = 16;

pub struct LightSensor<'d> {
    channel: AdcChannelDriver<'d, Gpio34, AdcDriver<'d, ADC1>>,
}

impl<'d> LightSensor<'d> {
    pub fn new(
        adc1: impl Peripheral<P = ADC1> + 'd,
        pin: impl Peripheral<P = Gpio34> + 'd,
    ) -> Result<Self> {
        let adc = AdcDriver::new(adc1)?;
        // Default channel config: 0dB attenuation, 12-bit resolution.
        let channel = AdcChannelDriver::new(adc, pin, &AdcChannelConfig::new())?;
        Ok(Self { channel })
    }

    /// Averaged raw 12-bit reading (readAmbientLightRaw in main_cyd.cpp).
    pub fn read_averaged(&mut self) -> Result<u16> {
        let mut total: u32 = 0;
        for _ in 0..SAMPLE_COUNT {
            total += self.channel.read_raw()? as u32;
            busy_delay_us(150);
        }
        Ok((total / SAMPLE_COUNT as u32) as u16)
    }
}

fn busy_delay_us(us: u64) {
    let start = unsafe { esp_idf_svc::sys::esp_timer_get_time() };
    while unsafe { esp_idf_svc::sys::esp_timer_get_time() } - start < us as i64 {
        core::hint::spin_loop();
    }
}
