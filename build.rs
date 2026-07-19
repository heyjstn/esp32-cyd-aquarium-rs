use std::time::{SystemTime, UNIX_EPOCH};

fn build_epoch() -> i64 {
    std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or(0)
        })
}

fn main() {
    // Only run ESP-IDF build glue when actually targeting the chip.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("espidf") {
        embuild::espidf::sysenv::output();
    }

    // Firmware build timestamp, used as the fallback clock when no NTP sync
    // has happened yet (mirrors the __DATE__/__TIME__ fallback in C++).
    let epoch = build_epoch();
    println!("cargo:rustc-env=CYD_BUILD_EPOCH={epoch}");
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
}
