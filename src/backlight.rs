//! Ambient-light driven backlight control, ported from main_cyd.cpp
//! (readAmbientLightRaw / mapAmbientRawToBacklight / stepBacklightToward and
//! the EMA smoothing in updateAmbientBacklight).

use crate::consts;

pub fn backlight_percent_to_duty(percent: u8, max_duty: u32, active_high: bool) -> u32 {
    let percent = percent.min(100) as u32;
    let on_duty = (percent * max_duty + 50) / 100;
    if active_high {
        on_duty
    } else {
        max_duty - on_duty
    }
}

/// Map an averaged raw LDR reading to a backlight percentage.
/// Raw values rise as the room gets darker on this board's LDR circuit.
pub fn map_ambient_raw_to_backlight(raw: u16) -> u8 {
    let bright_raw = consts::AUTO_BACKLIGHT_BRIGHT_RAW;
    let dark_raw = consts::AUTO_BACKLIGHT_DARK_RAW;
    let min_percent = consts::AUTO_BACKLIGHT_MIN_PERCENT;
    let max_percent = consts::AUTO_BACKLIGHT_MAX_PERCENT;

    if dark_raw <= bright_raw {
        return max_percent;
    }
    if raw <= bright_raw {
        return max_percent;
    }
    if raw >= dark_raw {
        return min_percent;
    }

    let span = dark_raw - bright_raw;
    let darkness = raw - bright_raw;
    let range = max_percent - min_percent;
    max_percent - ((darkness as u32 * range as u32 + span as u32 / 2) / span as u32) as u8
}

/// Move one step toward the target percentage (hysteresis to avoid flicker).
pub fn step_backlight_toward(current: u8, target: u8) -> u8 {
    let step = consts::AUTO_BACKLIGHT_STEP_PERCENT;
    if current < target {
        let next = current as u16 + step as u16;
        if next > target as u16 {
            target
        } else {
            next as u8
        }
    } else if current > target {
        if current > target + step {
            current - step
        } else {
            target
        }
    } else {
        current
    }
}

/// Stateful ambient backlight controller (EMA smoothing + stepped output).
pub struct AmbientBacklight {
    raw_ema: u16,
    percent: u8,
}

impl AmbientBacklight {
    pub fn new(initial_raw: u16) -> Self {
        Self {
            raw_ema: initial_raw,
            percent: map_ambient_raw_to_backlight(initial_raw),
        }
    }

    pub fn percent(&self) -> u8 {
        self.percent
    }

    pub fn raw_ema(&self) -> u16 {
        self.raw_ema
    }

    /// Feed a new averaged raw sample; returns the new percent if it changed.
    pub fn update(&mut self, raw: u16) -> Option<u8> {
        self.raw_ema = ((self.raw_ema as u32 * 7 + raw as u32 + 4) / 8) as u16;
        let target = map_ambient_raw_to_backlight(self.raw_ema);
        let next = step_backlight_toward(self.percent, target);
        if next == self.percent {
            return None;
        }
        self.percent = next;
        Some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapping_boundaries() {
        assert_eq!(map_ambient_raw_to_backlight(0), 100);
        assert_eq!(map_ambient_raw_to_backlight(10), 100);
        assert_eq!(map_ambient_raw_to_backlight(650), 38);
        assert_eq!(map_ambient_raw_to_backlight(4000), 38);
    }

    #[test]
    fn pwm_duty_clamps_rounds_and_supports_active_low() {
        assert_eq!(backlight_percent_to_duty(0, 255, true), 0);
        assert_eq!(backlight_percent_to_duty(50, 255, true), 128);
        assert_eq!(backlight_percent_to_duty(100, 255, true), 255);
        assert_eq!(backlight_percent_to_duty(120, 255, true), 255);
        assert_eq!(backlight_percent_to_duty(25, 100, false), 75);
    }

    #[test]
    fn mapping_midpoint() {
        // Halfway between bright(10) and dark(650): 100 - (320*62 + 320)/640
        assert_eq!(map_ambient_raw_to_backlight(330), 69);
    }

    #[test]
    fn stepper_moves_toward_target() {
        assert_eq!(step_backlight_toward(100, 38), 96);
        assert_eq!(step_backlight_toward(40, 38), 38);
        assert_eq!(step_backlight_toward(38, 100), 42);
        assert_eq!(step_backlight_toward(98, 100), 100);
        assert_eq!(step_backlight_toward(50, 50), 50);
    }

    #[test]
    fn ema_smooths_and_eventually_converges() {
        let mut bl = AmbientBacklight::new(10);
        assert_eq!(bl.percent(), 100);
        // Simulate the room going instantly dark; the EMA needs several
        // samples before the stepper reaches the minimum.
        let mut last = 100;
        for _ in 0..200 {
            if let Some(p) = bl.update(650) {
                last = p;
            }
        }
        assert_eq!(last, 38);
    }

    #[test]
    fn ema_ignores_single_spike() {
        let mut bl = AmbientBacklight::new(10);
        bl.update(650);
        // EMA after one spike: (10*7 + 650 + 4)/8 = 90 -> target still high.
        assert_eq!(bl.raw_ema(), 90);
        assert!(bl.percent() >= 96);
    }
}
