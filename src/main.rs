#[cfg(target_os = "espidf")]
fn main() {
    aquarium::hw::run();
}

#[cfg(not(target_os = "espidf"))]
fn main() {
    eprintln!("esp32-cyd-aquarium-rs: firmware binary, run on an ESP32-2432S028R.");
    eprintln!("Use `cargo +esp build --target xtensa-esp32-espidf --release` to build.");
}
