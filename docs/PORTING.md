# Porting notes

## Scope analysis

The C++ repository contains several display backends and application services, but its default `platformio.ini` selects `PANEL_CYD_TFT`, `AQUARIUM_ONLY`, `FIXED_ENVIRONMENT`, `main_cyd.cpp`, the dot renderer, autonomous life, curated boot creatures, touch feeding, Wi-Fi time, and automatic backlight control. The Rust firmware ports that compiled behavior rather than inactive HUB75, web-server, MQTT, DMX, image, and external-sensor paths.

## Source mapping

| C++ source | Rust destination | Ported responsibility |
| --- | --- | --- |
| `src/main_cyd.cpp` | `src/hw/mod.rs`, `src/text.rs`, `src/backlight.rs`, `src/hw/wifi_time.rs` | Boot, frame pacing, clock overlay, NTP, touch events, and backlight policy |
| `lib/Aquarium/Aquarium.h` | `src/aquarium.rs` | Population, food assignment, autonomous life, update order, and periodic snapshots |
| `lib/Aquarium/Fish.h` | `src/fish.rs` | Age, health, reproduction, food pursuit, body and motion ownership |
| `lib/Aquarium/Body` | `src/body.rs`, `src/color.rs` | Fish, snake, star, turtle, and octopus drawing plus palettes |
| `lib/Aquarium/Motion` | `src/motion.rs`, `src/vec2.rs` | Steering, speed limits, noise, and boundary forces |
| `Water.h`, `Plants.h`, `Food.h`, `BoidManager.cpp` | `src/world.rs` | Environmental scene elements and flocking |
| `CydMatrix.cpp` | `src/renderer.rs`, `src/tone.rs`, `src/hw/display.rs` | Foreground/background composition, tone calibration, 3x3 dot expansion, dirty tiles, ILI9341 SPI output, and PWM |
| `CydTouch.cpp` | `src/touch.rs`, `src/hw/touch.rs` | Calibration, rotation, coordinate mapping, and XPT2046 reads |
| `AquariumStateManager.cpp` | `src/state.rs`, `src/hw/store.rs` | Compatible fish JSON fields and bounded NVS snapshots |

## Design changes

- Hardware-independent behavior is separated from ESP-IDF drivers, allowing the engine, geometry, color math, renderer, state format, touch mapping, and backlight policy to run under ordinary `cargo test`.
- Ownership replaces the C++ `unique_ptr` object graph. Fish, food, plants, and boids are stored directly in vectors, while food targets use stable numeric IDs instead of raw pointers.
- Rendering keeps two logical RGB layers and stages four logical rows at a time. A previous-frame buffer marks dirty tiles so unchanged TFT regions are not transferred, while failed transfers retain dirty state for retry.
- The display model runs TFT_eSPI's selected `ILI9341_2_DRIVER` register sequence at the active 55 MHz source SPI frequency, with normal inversion, BGR order, rotation 2, and RGB565 pixels. ESP-IDF HAL drivers provide SPI, LEDC, ADC, GPIO, Wi-Fi, SNTP, and NVS integration.
- Wi-Fi/NTP runs on a background thread. Connection and IP readiness are awaited with bounded blocking ESP-IDF events, so the 30 FPS scene loop is not stalled.
- Wi-Fi credentials and timezone are supplied through ignored `cfg.toml` data rather than a C++ secrets header.

## Behavior retained from the CYD build

- Physical 240x320 display, logical 80x106 aquarium, 240x318 centered viewport, and 180 degree rotation.
- Tuned background/foreground brightness, saturation, tone curve, RGB565 transport, and plus-shaped 3x3 dots.
- Eight curated creatures at boot, growth toward twelve creatures, keep-on-screen margins, autonomous food, and touch-to-feed.
- Periodic NVS fish snapshots are saved, but curated default boot intentionally starts fresh instead of restoring them.
- Fixed 22 C, 420 ppm CO2, and 50 percent humidity environment.
- GPIO34 ambient-light mapping with EMA smoothing and stepped 38 to 100 percent backlight output.
- Compile-time clock fallback, POSIX local timezone after NTP, Wi-Fi shutdown after sync, retry, and six-hour resync.

## Verification strategy

Host tests cover deterministic math and noise vectors, shapes, motion, aquarium life-cycle behavior, state compatibility, text, renderer dirty tiles, tone processing, PWM conversion, and touch mapping. The target build separately compiles and links every ESP-IDF-only driver for `xtensa-esp32-espidf`.
