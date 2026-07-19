//! TFT color pipeline: brightness scaling, tone curve, saturation/gain
//! profiles and RGB565 packing, ported from CydMatrix.cpp.

use crate::color::{color565, luma8, CRgb};
use crate::consts;
use crate::math8::constrain;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorProfile {
    Background,
    Foreground,
}

pub fn scale_channel_by_brightness(value: u8, brightness: u8) -> u8 {
    ((value as u16 * brightness as u16 + 127) / 255) as u8
}

/// Quadratic TFT tone curve with a black threshold (applyTftToneCurve).
pub fn apply_tone_curve(value: u8, strength: u16, black_threshold: u8) -> u8 {
    if !consts::TONE_CORRECTION {
        return value;
    }
    if value == 0 || value == 255 {
        return value;
    }

    let strength = strength.min(255);
    let linear_weight = 255 - strength;
    let quadratic = (value as u16 * value as u16 + 127) / 255;
    let corrected = (value as u16 * linear_weight + quadratic * strength + 127) / 255;

    if black_threshold > 0 && corrected < black_threshold as u16 {
        return 0;
    }
    corrected.min(255) as u8
}

pub fn saturate_channel(value: u8, luma: u8, saturation: u16) -> u8 {
    // C++ promotes the int16_t operands to 32-bit int here; do the same to
    // avoid overflow for large deltas/saturation values.
    let delta = value as i32 - luma as i32;
    let saturated = luma as i32 + (delta * saturation as i32) / 128;
    constrain(saturated, 0, 255) as u8
}

pub fn gain_channel(value: u8, gain: u16) -> u8 {
    constrain((value as u16 * gain + 64) / 128, 0, 255) as u8
}

/// Saturation + gain applied to foreground pixels before the tone curve
/// (applyTftForegroundProfile).
pub fn foreground_profile(mut color: CRgb) -> CRgb {
    if color == CRgb::BLACK {
        return color;
    }

    let luma = luma8(color.r, color.g, color.b);
    color.r = saturate_channel(color.r, luma, consts::FOREGROUND_PROFILE_SATURATION);
    color.g = saturate_channel(color.g, luma, consts::FOREGROUND_PROFILE_SATURATION);
    color.b = saturate_channel(color.b, luma, consts::FOREGROUND_PROFILE_SATURATION);

    color.r = gain_channel(color.r, consts::FOREGROUND_GAIN);
    color.g = gain_channel(color.g, consts::FOREGROUND_GAIN);
    color.b = gain_channel(color.b, consts::FOREGROUND_GAIN);
    color
}

/// Brightness/saturation/black-threshold profile applied per layer at
/// composite time (applyDisplayProfile).
pub fn apply_display_profile(
    color: CRgb,
    brightness: u8,
    saturation: u16,
    black_threshold: u8,
) -> CRgb {
    if color == CRgb::BLACK {
        return CRgb::BLACK;
    }

    let mut c = color;
    if saturation != 128 {
        let luma = luma8(c.r, c.g, c.b);
        c.r = saturate_channel(c.r, luma, saturation);
        c.g = saturate_channel(c.g, luma, saturation);
        c.b = saturate_channel(c.b, luma, saturation);
    }

    c.r = scale_channel_by_brightness(c.r, brightness);
    c.g = scale_channel_by_brightness(c.g, brightness);
    c.b = scale_channel_by_brightness(c.b, brightness);

    if black_threshold > 0 && c.r.max(c.g).max(c.b) < black_threshold {
        return CRgb::BLACK;
    }

    c
}

/// Full CydMatrix packRgb565ForDisplay pipeline.
pub fn pack_rgb565_for_display(
    r: u8,
    g: u8,
    b: u8,
    brightness_scale: u8,
    profile: ColorProfile,
) -> u16 {
    let mut color = CRgb::new(
        scale_channel_by_brightness(r, brightness_scale),
        scale_channel_by_brightness(g, brightness_scale),
        scale_channel_by_brightness(b, brightness_scale),
    );

    match profile {
        ColorProfile::Foreground => {
            color = foreground_profile(color);
            color.r = apply_tone_curve(color.r, consts::FOREGROUND_TONE_CURVE_STRENGTH, 0);
            color.g = apply_tone_curve(color.g, consts::FOREGROUND_TONE_CURVE_STRENGTH, 0);
            color.b = apply_tone_curve(color.b, consts::FOREGROUND_TONE_CURVE_STRENGTH, 0);
        }
        ColorProfile::Background => {
            color.r = apply_tone_curve(
                color.r,
                consts::BACKGROUND_TONE_CURVE_STRENGTH,
                consts::TONE_BLACK_THRESHOLD,
            );
            color.g = apply_tone_curve(
                color.g,
                consts::BACKGROUND_TONE_CURVE_STRENGTH,
                consts::TONE_BLACK_THRESHOLD,
            );
            color.b = apply_tone_curve(
                color.b,
                consts::BACKGROUND_TONE_CURVE_STRENGTH,
                consts::TONE_BLACK_THRESHOLD,
            );
        }
    }

    color565(color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightness_scaling_rounds() {
        assert_eq!(scale_channel_by_brightness(255, 255), 255);
        assert_eq!(scale_channel_by_brightness(255, 0), 0);
        assert_eq!(scale_channel_by_brightness(128, 128), 64);
    }

    #[test]
    fn tone_curve_endpoints_passthrough() {
        assert_eq!(apply_tone_curve(0, 150, 2), 0);
        assert_eq!(apply_tone_curve(255, 150, 2), 255);
    }

    #[test]
    fn tone_curve_black_threshold() {
        assert_eq!(apply_tone_curve(1, 150, 2), 0);
        // Even without a threshold, (1*105 + 127)/255 truncates to 0.
        assert_eq!(apply_tone_curve(1, 150, 0), 0);
    }

    #[test]
    fn tone_curve_disabled_strength_is_identity() {
        assert_eq!(apply_tone_curve(100, 0, 0), 100);
        assert_eq!(apply_tone_curve(37, 0, 0), 37);
    }

    #[test]
    fn saturation_128_is_neutral() {
        // saturation 128: delta * 128 / 128 == delta
        assert_eq!(saturate_channel(200, 100, 128), 200);
        assert_eq!(saturate_channel(50, 100, 128), 50);
    }

    #[test]
    fn display_profile_black_threshold_kills_dim_pixels() {
        assert_eq!(
            apply_display_profile(CRgb::new(1, 1, 1), 200, 128, 2),
            CRgb::BLACK
        );
        assert_eq!(
            apply_display_profile(CRgb::new(1, 1, 1), 200, 128, 0),
            CRgb::new(1, 1, 1)
        );
        assert_eq!(
            apply_display_profile(CRgb::new(4, 4, 4), 200, 128, 2),
            CRgb::new(3, 3, 3)
        );
    }

    #[test]
    fn pack_background_black_stays_black() {
        assert_eq!(
            pack_rgb565_for_display(0, 0, 0, 255, ColorProfile::Background),
            0
        );
        assert_eq!(
            pack_rgb565_for_display(0, 0, 0, 255, ColorProfile::Foreground),
            0
        );
    }

    #[test]
    fn pack_foreground_applies_gain() {
        let fg = pack_rgb565_for_display(100, 100, 100, 255, ColorProfile::Foreground);
        let bg = pack_rgb565_for_display(100, 100, 100, 255, ColorProfile::Background);
        assert_ne!(fg, bg);
        // Foreground gain 165/128 > 1 should be at least as bright.
        assert!(fg > bg);
    }
}
