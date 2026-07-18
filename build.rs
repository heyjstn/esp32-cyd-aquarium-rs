use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Only run ESP-IDF build glue when actually targeting the chip.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("espidf") {
        embuild::espidf::sysenv::output();
    }

    // Firmware build timestamp, used as the fallback clock when no NTP sync
    // has happened yet (mirrors the __DATE__/__TIME__ fallback in C++).
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    println!("cargo:rustc-env=CYD_BUILD_EPOCH={epoch}");
    println!("cargo:rerun-if-changed=build.rs");
}
