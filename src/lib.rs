//! ESP32 CYD Aquarium, Rust port.
//!
//! The crate is split in two:
//! * pure, host-testable modules (everything below), and
//! * the `hw` module with the ESP-IDF hardware drivers, which only compiles
//!   for the ESP32 target.

pub mod aquarium;
pub mod backlight;
pub mod body;
pub mod color;
pub mod consts;
pub mod fish;
pub mod layer;
pub mod math8;
pub mod motion;
pub mod noise;
pub mod panel;
pub mod renderer;
pub mod rng;
pub mod state;
pub mod text;
pub mod tone;
pub mod touch;
pub mod vec2;
pub mod wifi;
pub mod world;

#[cfg(target_os = "espidf")]
pub mod hw;
