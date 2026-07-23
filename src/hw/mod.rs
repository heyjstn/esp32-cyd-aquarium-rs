//! ESP-IDF hardware drivers and the firmware entry point.
//! Only compiled for `target_os = "espidf"`.

pub mod bluetooth;
pub mod display;
mod led;
pub mod light;
pub mod store;
pub mod touch;
pub mod wifi_time;

use anyhow::Result;
use log::{info, warn};
use std::sync::mpsc;
use std::thread;

use crate::aquarium::Aquarium;
use crate::backlight::AmbientBacklight;
use crate::color::color565;
use crate::consts;
use crate::hw::led::{Color, LedMode};
use crate::layer::Layer;
use crate::renderer::DotRenderer;
use crate::rng::Rng;
use crate::text::render_clock_overlay;

/// Milliseconds since boot (esp_timer based).
pub fn millis() -> u64 {
    (unsafe { esp_idf_svc::sys::esp_timer_get_time() } as u64) / 1000
}

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_password: &'static str,
    #[default("CST6CDT,M3.2.0/2,M11.1.0/2")]
    timezone: &'static str,
}

pub fn run() {
    if let Err(err) = run_inner() {
        panic!("fatal error: {err:?}");
    }
}

fn run_inner() -> Result<()> {
    // Required by esp-idf-sys when using the std runtime.
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("CYD Aquarium (Rust) boot");
    info!(
        "logical={}x{} physical={}x{} viewport={},{} {}x{}",
        consts::LOGICAL_WIDTH,
        consts::LOGICAL_HEIGHT,
        consts::PHYSICAL_WIDTH,
        consts::PHYSICAL_HEIGHT,
        consts::VIEWPORT_X,
        consts::VIEWPORT_Y,
        consts::VIEWPORT_WIDTH,
        consts::VIEWPORT_HEIGHT
    );

    let peripherals = esp_idf_hal::peripherals::Peripherals::take()?;
    let sysloop = esp_idf_svc::eventloop::EspSystemEventLoop::take()?;

    let seed = unsafe { esp_idf_svc::sys::esp_random() };
    let mut rng = Rng::new(seed);

    // Display + backlight.
    let mut display = display::CydDisplay::new(
        peripherals.spi2,
        peripherals.pins.gpio14,
        peripherals.pins.gpio13,
        peripherals.pins.gpio15,
        peripherals.pins.gpio2,
        peripherals.ledc.timer0,
        peripherals.ledc.channel0,
        peripherals.pins.gpio21,
    )?;

    let (led_sender, led_receiver) = mpsc::sync_channel(1);

    // Led-render loop
    thread::spawn(move || -> Result<()> {
        let (red_pin, green_pin, blue_pin) = (
            peripherals.pins.gpio4,
            peripherals.pins.gpio16,
            peripherals.pins.gpio17,
        );
        let mut led = led::new(red_pin, green_pin, blue_pin);
        led.run(led_receiver)
    });

    let _ = led_sender.clone().send(LedMode::Solid(Color::RED));

    // Ambient light sensor + initial backlight level.
    let mut light = light::LightSensor::new(peripherals.adc1, peripherals.pins.gpio34)?;
    let initial_raw = light.read_averaged()?;
    let mut ambient = AmbientBacklight::new(initial_raw);
    display.set_backlight_percent(ambient.percent())?;
    info!(
        "auto_backlight=on pin=34 raw={} percent={}",
        initial_raw,
        ambient.percent()
    );
    let mut last_backlight_read = 0u64;

    // Touch.
    let mut touch = touch::CydTouch::new(
        peripherals.pins.gpio33,
        peripherals.pins.gpio25,
        peripherals.pins.gpio32,
        peripherals.pins.gpio39,
        peripherals.pins.gpio36,
    )?;

    // Clock: fallback base + background NTP sync.
    let mut clock = wifi_time::Clock::new(millis());

    let (wifi_modem, bluetooth_modem) = peripherals.modem.split();
    bluetooth::create_bluetooth_conn(bluetooth_modem)?;

    if CONFIG.wifi_ssid.is_empty() {
        info!("clock ntp=skipped reason=no_wifi_credentials");
    } else {
        wifi_time::spawn_clock_sync(
            wifi_modem,
            led_sender.clone(),
            sysloop.clone(),
            CONFIG.wifi_ssid,
            CONFIG.wifi_password,
            CONFIG.timezone,
            clock.shared(),
        );
    }

    // NVS state store (fish state is saved periodically, like the C++ build).
    let mut nvs_store = match store::NvsStore::new() {
        Ok(store) => Some(store),
        Err(err) => {
            warn!("nvs init failed: {err:?}; state saving disabled");
            None
        }
    };

    // Aquarium scene.
    let mut aquarium = Aquarium::new(consts::LOGICAL_WIDTH as u8, consts::LOGICAL_HEIGHT as u8);
    aquarium.begin(millis(), &mut rng);

    let mut foreground = Layer::new(consts::LOGICAL_WIDTH as i16, consts::LOGICAL_HEIGHT as i16);
    let mut background = Layer::new(consts::LOGICAL_WIDTH as i16, consts::LOGICAL_HEIGHT as i16);
    let mut renderer = DotRenderer::new();
    renderer.clear_screen(&mut display)?;

    let time_color = color565(224, 248, 238);
    let date_color = color565(132, 222, 190);
    let shadow_color = color565(0, 6, 8);

    let mut last_frame = 0u64;
    loop {
        let now = millis();

        if now.wrapping_sub(last_backlight_read) >= consts::AUTO_BACKLIGHT_READ_INTERVAL_MS {
            last_backlight_read = now;
            match light.read_averaged() {
                Ok(raw) => {
                    if let Some(percent) = ambient.update(raw) {
                        if let Err(err) = display.set_backlight_percent(percent) {
                            warn!("backlight write failed: {err:?}");
                        }
                    }
                }
                Err(err) => warn!("light sensor read failed: {err:?}"),
            }
        }

        if now.wrapping_sub(last_frame) >= consts::FRAME_INTERVAL_MS {
            last_frame = now;

            if let Err(err) = touch.update() {
                warn!("touch read failed: {err:?}");
            }
            if touch.pressed_started() {
                aquarium.on_touch_started(now, &mut rng);
            } else if touch.pressed_released() {
                aquarium.on_touch_released();
            }

            aquarium.update(now, &mut rng, &mut foreground, &mut background);

            let civil = clock.now_local(now);
            render_clock_overlay(
                &mut foreground,
                &civil,
                time_color,
                date_color,
                shadow_color,
            );

            if let Err(err) = renderer.composite(&mut foreground, &background, &mut display) {
                warn!("display transfer failed: {err:?}");
            }

            if aquarium.take_save_pending() {
                if let Some(store) = nvs_store.as_mut() {
                    let defs = aquarium.fish_definitions();
                    match store.save_fishes(&defs) {
                        Ok(()) => info!("state saved ({} fish)", defs.len()),
                        Err(err) => warn!("state save failed: {err:?}"),
                    }
                }
            }
        }

        esp_idf_hal::delay::FreeRtos::delay_ms(1);
    }
}
