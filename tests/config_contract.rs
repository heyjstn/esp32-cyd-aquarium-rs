use std::fs;

#[test]
fn example_config_matches_toml_cfg_package_contract() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/cfg.toml.example");
    let source = fs::read_to_string(path).expect("cfg.toml.example should be readable");
    let document: toml::Table = toml::from_str(&source).expect("example should be valid TOML");
    let package = document
        .get(env!("CARGO_PKG_NAME"))
        .and_then(toml::Value::as_table)
        .expect("toml-cfg requires settings under the package-name table");

    for key in ["wifi_ssid", "wifi_password", "timezone"] {
        assert!(package.contains_key(key), "missing {key} setting");
    }
}

#[test]
fn cargo_defaults_to_the_esp_idf_target_for_ide_indexing() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/.cargo/config.toml");
    let source = fs::read_to_string(path).expect("Cargo config should be readable");
    let document: toml::Table = toml::from_str(&source).expect("Cargo config should be valid TOML");
    let target = document
        .get("build")
        .and_then(toml::Value::as_table)
        .and_then(|build| build.get("target"))
        .and_then(toml::Value::as_str);

    assert_eq!(target, Some("xtensa-esp32-espidf"));
}

#[test]
fn wifi_sync_does_not_log_credentials() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/hw/wifi_time.rs");
    let source = fs::read_to_string(path).expect("Wi-Fi implementation should be readable");

    for credential in ["{ssid}", "{password}"] {
        assert!(
            !source.lines().any(|line| {
                (line.contains("info!(") || line.contains("warn!(")) && line.contains(credential)
            }),
            "Wi-Fi logs must not interpolate {credential}"
        );
    }
}
