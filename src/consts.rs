//! Compile-time configuration, mirroring `platformio.ini` build flags,
//! `AquariumSettings.h` and `CydMatrixSettings.h` from the C++ firmware.

// ---------------------------------------------------------------------------
// Logical aquarium panel and physical display geometry (CydMatrixSettings.h)
// ---------------------------------------------------------------------------

pub const LOGICAL_WIDTH: usize = 80;
pub const LOGICAL_HEIGHT: usize = 106;

pub const PHYSICAL_WIDTH: u16 = 240;
pub const PHYSICAL_HEIGHT: u16 = 320;

// 80x106 scaled by 3 becomes 240x318, centered in 240x320.
pub const VIEWPORT_X: u16 = 0;
pub const VIEWPORT_Y: u16 = 1;
pub const VIEWPORT_WIDTH: u16 = 240;
pub const VIEWPORT_HEIGHT: u16 = 318;

/// Physics positions are tracked at this fixed-point scale (PHYSICS_SCALE).
pub const PHYSICS_SCALE: f32 = 80.0;

/// Screen x of the left edge of a logical pixel column.
pub const fn screen_x_for_logical_edge(logical_x: u16) -> u16 {
    VIEWPORT_X + (logical_x * VIEWPORT_WIDTH) / LOGICAL_WIDTH as u16
}

/// Screen y of the top edge of a logical pixel row.
pub const fn screen_y_for_logical_edge(logical_y: u16) -> u16 {
    VIEWPORT_Y + (logical_y * VIEWPORT_HEIGHT) / LOGICAL_HEIGHT as u16
}

pub const fn logical_x_from_screen(screen_x: i32) -> u16 {
    let rel = screen_x - VIEWPORT_X as i32;
    if rel <= 0 {
        return 0;
    }
    if rel >= VIEWPORT_WIDTH as i32 {
        return LOGICAL_WIDTH as u16 - 1;
    }
    (rel * LOGICAL_WIDTH as i32 / VIEWPORT_WIDTH as i32) as u16
}

pub const fn logical_y_from_screen(screen_y: i32) -> u16 {
    let rel = screen_y - VIEWPORT_Y as i32;
    if rel <= 0 {
        return 0;
    }
    if rel >= VIEWPORT_HEIGHT as i32 {
        return LOGICAL_HEIGHT as u16 - 1;
    }
    (rel * LOGICAL_HEIGHT as i32 / VIEWPORT_HEIGHT as i32) as u16
}

// ---------------------------------------------------------------------------
// Pinout (ESP32-2432S028R / CYD)
// ---------------------------------------------------------------------------

pub mod pins {
    pub const TFT_MISO: i32 = 12;
    pub const TFT_MOSI: i32 = 13;
    pub const TFT_SCLK: i32 = 14;
    pub const TFT_CS: i32 = 15;
    pub const TFT_DC: i32 = 2;
    pub const TFT_BL: i32 = 21;

    pub const TOUCH_MOSI: i32 = 32;
    pub const TOUCH_MISO: i32 = 39;
    pub const TOUCH_SCLK: i32 = 25;
    pub const TOUCH_CS: i32 = 33;
    pub const TOUCH_IRQ: i32 = 36;

    pub const LIGHT_SENSOR: i32 = 34;
}

pub const TFT_SPI_HZ: u32 = 55_000_000;
pub const TFT_BACKLIGHT_ON_HIGH: bool = true;

// Backlight LEDC PWM.
pub const BACKLIGHT_PWM_FREQ_HZ: u32 = 5000;
pub const BACKLIGHT_PWM_RESOLUTION_BITS: u32 = 8;

// ---------------------------------------------------------------------------
// Dot renderer / tone correction (platformio.ini CYD_TFT_* flags)
// ---------------------------------------------------------------------------

pub const DOT_RADIUS_RATIO: f32 = 0.43;
/// Logical rows per pushed tile (rowBufferLogicalHeight in CydMatrix.cpp).
pub const TILE_LOGICAL_ROWS: u16 = 4;

pub const TONE_CORRECTION: bool = true;
pub const TONE_CURVE_STRENGTH: u16 = 150;
pub const TONE_BLACK_THRESHOLD: u8 = 2;
pub const BACKGROUND_TONE_CURVE_STRENGTH: u16 = 0;
pub const FOREGROUND_TONE_CURVE_STRENGTH: u16 = 0;
pub const FOREGROUND_GAIN: u16 = 165;
pub const FOREGROUND_PROFILE_SATURATION: u16 = 235;

pub const BACKGROUND_BRIGHTNESS: u8 = 200;
pub const BACKGROUND_BLACK_THRESHOLD: u8 = 2;
pub const FOREGROUND_BRIGHTNESS: u8 = 235;
pub const FOREGROUND_SATURATION: u16 = 160;

// ---------------------------------------------------------------------------
// Frame loop
// ---------------------------------------------------------------------------

pub const IDEAL_FPS: u64 = 30;
pub const FRAME_INTERVAL_MS: u64 = 1000 / IDEAL_FPS;

// ---------------------------------------------------------------------------
// Fixed environment (FIXED_ENVIRONMENT build; no real SCD40 attached)
// ---------------------------------------------------------------------------

pub const DEFAULT_TEMPERATURE_C: f32 = 22.0;
pub const DEFAULT_CO2_PPM: i64 = 420;
pub const FIXED_HUMIDITY_PERCENT: u8 = 50;

// CO2 thresholds (SCD40Settings.h).
pub const CO2_OK: i64 = 600;
pub const CO2_BAD: i64 = 1000;
pub const CO2_REALBAD: i64 = 2000;

// ---------------------------------------------------------------------------
// Aquarium population (AquariumSettings.h, PANEL_CYD_TFT branch)
// ---------------------------------------------------------------------------

pub const NUM_FISH_START: usize = 8;
pub const NUM_FISH_IDEAL: usize = 12;
pub const NUM_PLANTS: usize = 3;

pub const AQUARIUM_SAVE_INTERVAL_MS: u64 = 30 * 60 * 1000;

pub const FISH_LIFESPAN_DAYS: f32 = 7.0;
pub const FISH_LIFESPAN_VARIATION: f32 = 1.0;

pub const HEALTH_REDUCTION_RATE_BAD: f32 = 0.1;
pub const HEALTH_REDUCTION_RATE_REALBAD: f32 = 0.2;
pub const HEALTH_INCREASE_RATE_GOOD: f32 = 0.05;

// Age thresholds.
pub const AGE_EGG: f32 = 0.1;
pub const AGE_ADULT: f32 = 0.7;
pub const AGE_SENIOR: f32 = 0.9;
pub const AGE_DEAD: f32 = 1.0;

// Forces.
pub const MAX_FORCE: f32 = 3.0;
pub const FOOD_FORCE: f32 = MAX_FORCE * 2.0;
pub const BOUNDARY_FORCE: f32 = 0.2;
pub const BORDER_BUFFER: f32 = 0.0;

// Keep-on-screen margins (logical pixels, scaled by PHYSICS_SCALE when active).
pub const KEEP_INSIDE_MARGIN_X: f32 = 6.0;
pub const KEEP_INSIDE_MARGIN_TOP: f32 = 14.0;
pub const KEEP_INSIDE_MARGIN_BOTTOM: f32 = 12.0;

// Body parameter ranges as (min, max_exclusive), matching Arduino random(a, b).
pub const FISH_NUM_SEGMENTS: (i32, i32) = (4, 12);
pub const FISH_MIN_SEGMENT_SIZE: f32 = 2.0;
pub const FISH_MAX_SEGMENT_ADD: (i32, i32) = (2, 6);
pub const FISH_GAP_BETWEEN_SEGMENTS: (i32, i32) = (70, 100);
pub const FISH_NEEDLE_NOSE_LENGTH_MULTIPLIER: (i32, i32) = (2, 6);

pub const FISH_MAX_SPEED: i32 = 30;
pub const FISH_MIN_SPEED: i32 = 10;
pub const FISH_MAX_FORCE: f32 = 0.3;
pub const FISH_SIN_AMPLITUDE: i32 = 2;
pub const FISH_SIN_FREQUENCY: f32 = 0.002;
pub const FISH_NOISE_AMPLITUDE: f32 = 6.0;
pub const FISH_NOISE_FREQUENCY: f32 = 0.01;

pub const SNAKE_NUM_SEGMENTS: (i32, i32) = (8, 60);
pub const SNAKE_MAX_SPEED: i32 = 40;
pub const SNAKE_MIN_SPEED: i32 = 20;
pub const SNAKE_MAX_FORCE: f32 = 0.3;
pub const SNAKE_SIN_AMPLITUDE: i32 = 5;
pub const SNAKE_SIN_FREQUENCY: f32 = 0.005;
pub const SNAKE_NOISE_AMPLITUDE: f32 = 6.0;
pub const SNAKE_NOISE_FREQUENCY: f32 = 0.01;

pub const STAR_LENGTH: (i32, i32) = (4, 6);
pub const STAR_RAD: (i32, i32) = (2, 4);
pub const STAR_NUM_ARMS: (i32, i32) = (5, 10);
pub const STAR_ROTATION_SPEED: (i32, i32) = (5, 10);
pub const STAR_MAX_SPEED: i32 = 30;
pub const STAR_MIN_SPEED: i32 = 10;
pub const STAR_MAX_FORCE: f32 = 0.2;
pub const STAR_SIN_AMPLITUDE: i32 = 5;
pub const STAR_SIN_FREQUENCY: f32 = 0.005;
pub const STAR_NOISE_AMPLITUDE: f32 = 2.0;
pub const STAR_NOISE_FREQUENCY: f32 = 0.01;

pub const TURTLE_LENGTH: (i32, i32) = (4, 10);
pub const TURTLE_WIDTH: (i32, i32) = (2, 4);
pub const TURTLE_MAX_SPEED: i32 = 40;
pub const TURTLE_MIN_SPEED: i32 = 5;
pub const TURTLE_MAX_FORCE: f32 = 0.3;
pub const TURTLE_SIN_AMPLITUDE: i32 = 10;
pub const TURTLE_SIN_FREQUENCY: f32 = 0.001;
pub const TURTLE_NOISE_AMPLITUDE: f32 = 1.0;
pub const TURTLE_NOISE_FREQUENCY: f32 = 0.01;

pub const OCTOPUS_SIZE: (i32, i32) = (10, 20);
pub const OCTOPUS_MIN_TENTACLES: i32 = 6;
pub const OCTOPUS_MAX_TENTACLES: i32 = 10;
pub const OCTOPUS_TENTACLE_SEGMENTS: usize = 6;
pub const OCTOPUS_TENTACLE_LENGTH: (i32, i32) = (10, 25);
pub const OCTOPUS_MAX_SPEED: i32 = 35;
pub const OCTOPUS_MIN_SPEED: i32 = 5;
pub const OCTOPUS_MAX_FORCE: f32 = 0.25;
pub const OCTOPUS_SIN_AMPLITUDE: i32 = 8;
pub const OCTOPUS_SIN_FREQUENCY: f32 = 0.002;
pub const OCTOPUS_NOISE_AMPLITUDE: f32 = 0.1;
pub const OCTOPUS_NOISE_FREQUENCY: f32 = 0.01;

// Boids.
pub const BOID_GROUPS: usize = 2;
pub const NUM_BOIDS: (i32, i32) = (10, 20);
pub const BOID_MAX_SPEED: (i32, i32) = (4, 8);
pub const BOID_MAX_FORCE: (i32, i32) = (1, 2);

// ---------------------------------------------------------------------------
// Autonomous life / feeding (platformio.ini CYD_AUTONOMOUS_* flags)
// ---------------------------------------------------------------------------

pub const AUTONOMOUS_BOOT_MIN_MS: u64 = 1500;
pub const AUTONOMOUS_BOOT_MAX_MS: u64 = 3500;
pub const AUTONOMOUS_FOOD_MIN_MS: u64 = 5500;
pub const AUTONOMOUS_FOOD_MAX_MS: u64 = 12000;
pub const AUTONOMOUS_MAX_FOOD: u8 = 3;
/// Interval between food drops while the screen is touched.
pub const TOUCH_FOOD_INTERVAL_MS: u64 = 200;

// ---------------------------------------------------------------------------
// Clock / Wi-Fi / NTP (platformio.ini CYD_WIFI_* / CYD_NTP_* flags)
// ---------------------------------------------------------------------------

pub const WIFI_CONNECT_TIMEOUT_MS: u64 = 8000;
pub const NTP_SYNC_TIMEOUT_MS: u64 = 5000;
pub const NTP_RESYNC_INTERVAL_MS: u64 = 21_600_000;
pub const NTP_RETRY_INTERVAL_MS: u64 = 300_000;
pub const WIFI_DISCONNECT_AFTER_NTP: bool = true;

// ---------------------------------------------------------------------------
// Auto backlight (platformio.ini CYD_AUTO_BACKLIGHT_* flags)
// ---------------------------------------------------------------------------

pub const AUTO_BACKLIGHT_BRIGHT_RAW: u16 = 10;
pub const AUTO_BACKLIGHT_DARK_RAW: u16 = 650;
pub const AUTO_BACKLIGHT_MIN_PERCENT: u8 = 38;
pub const AUTO_BACKLIGHT_MAX_PERCENT: u8 = 100;
pub const AUTO_BACKLIGHT_READ_INTERVAL_MS: u64 = 750;
pub const AUTO_BACKLIGHT_STEP_PERCENT: u8 = 4;

// ---------------------------------------------------------------------------
// Touch calibration (CydMatrixSettings.h)
// ---------------------------------------------------------------------------

pub const TOUCH_RAW_MIN_X: i32 = 200;
pub const TOUCH_RAW_MAX_X: i32 = 3900;
pub const TOUCH_RAW_MIN_Y: i32 = 200;
pub const TOUCH_RAW_MAX_Y: i32 = 3900;
/// The firmware runs the panel rotated 180 degrees (CYD_TFT_ROTATION = 2), so
/// touch coordinates are flipped as well.
pub const TOUCH_ROTATE_180: bool = true;
