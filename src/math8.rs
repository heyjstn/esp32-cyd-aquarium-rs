//! Fixed-point 8-bit math, ported from FastLED's lib8tion (as bundled in
//! GFX_Lite with FASTLED_SCALE8_FIXED = 1) plus the Arduino map/constrain
//! helpers the original firmware relies on.

/// Saturating 8-bit add (FastLED qadd8).
pub fn qadd8(i: u8, j: u8) -> u8 {
    i.saturating_add(j)
}

/// Average of two i8 values with rounding toward the sign of `i`
/// (FastLED avg7).
pub fn avg7(i: i8, j: i8) -> i8 {
    (i >> 1) + (j >> 1) + (i & 0x1)
}

/// Average of two i16 values (FastLED avg15, C implementation).
pub fn avg15(i: i16, j: i16) -> i16 {
    ((i as i32 + j as i32) >> 1) as i16
}

/// Scale a u8 by a fraction in n/256 units (FastLED scale8 with
/// FASTLED_SCALE8_FIXED = 1).
pub fn scale8(i: u8, scale: u8) -> u8 {
    ((i as u16 * (1 + scale as u16)) >> 8) as u8
}

/// "Video" version of scale8 that never dims a non-zero input to zero
/// unless the scale itself is zero (FastLED scale8_video).
pub fn scale8_video(i: u8, scale: u8) -> u8 {
    let bonus = u8::from(i != 0 && scale != 0);
    ((i as u16 * scale as u16) >> 8) as u8 + bonus
}

/// Quadratic ease-in/ease-out (FastLED ease8InOutQuad, C implementation).
pub fn ease8_in_out_quad(i: u8) -> u8 {
    let mut j = i;
    if j & 0x80 != 0 {
        j = 255 - j;
    }
    let jj = scale8(j, j);
    let mut jj2 = jj << 1;
    if i & 0x80 != 0 {
        jj2 = 255 - jj2;
    }
    jj2
}

/// Linear interpolation between two i8 values with an 8-bit fraction
/// (the lerp7by8 helper local to FastLED's noise.cpp).
pub fn lerp7by8(a: i8, b: i8, frac: u8) -> i8 {
    // Deltas are computed in wider math and truncated to u8, like the C++
    // `uint8_t delta = b - a` assignment (b - a can exceed the i8 range).
    if b > a {
        let delta = (b as i16 - a as i16) as u8;
        a.wrapping_add(scale8(delta, frac) as i8)
    } else {
        let delta = (a as i16 - b as i16) as u8;
        a.wrapping_sub(scale8(delta, frac) as i8)
    }
}

/// Map a full-range u8 into a narrower range (FastLED map8).
pub fn map8(i: u8, range_start: u8, range_end: u8) -> u8 {
    let width = range_end.wrapping_sub(range_start);
    scale8(i, width).wrapping_add(range_start)
}

/// Arduino map() with integer (long) math.
pub fn arduino_map(x: i64, in_min: i64, in_max: i64, out_min: i64, out_max: i64) -> i64 {
    if in_max == in_min {
        return out_min;
    }
    (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
}

/// Floating point variant used by the boid code (Boid::mapfloat).
pub fn map_f32(x: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
}

/// Arduino constrain().
pub fn constrain<T: PartialOrd>(x: T, lo: T, hi: T) -> T {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qadd8_saturates() {
        assert_eq!(qadd8(200, 100), 255);
        assert_eq!(qadd8(10, 5), 15);
    }

    #[test]
    fn avg7_matches_fastled() {
        assert_eq!(avg7(64, -64), 0);
        assert_eq!(avg7(127, 127), 127);
        assert_eq!(avg7(-128, -128), -128);
        assert_eq!(avg7(5, 4), 5);
    }

    #[test]
    fn avg15_matches_fastled() {
        assert_eq!(avg15(100, 200), 150);
        assert_eq!(avg15(-100, 100), 0);
    }

    #[test]
    fn scale8_fixed_semantics() {
        // With FASTLED_SCALE8_FIXED=1, scale 255 is an exact identity.
        assert_eq!(scale8(255, 255), 255);
        assert_eq!(scale8(128, 255), 128);
        // ...and scale 0 still blacks out non-zero input.
        assert_eq!(scale8(200, 0), 0);
        assert_eq!(scale8(255, 128), 128);
        assert_eq!(scale8(0, 255), 0);
    }

    #[test]
    fn scale8_video_keeps_nonzero_alive() {
        assert_eq!(scale8_video(1, 1), 1);
        assert_eq!(scale8_video(0, 200), 0);
        assert_eq!(scale8_video(100, 0), 0);
    }

    #[test]
    fn ease8_endpoints_and_midpoint() {
        assert_eq!(ease8_in_out_quad(0), 0);
        assert_eq!(ease8_in_out_quad(255), 255);
        assert_eq!(ease8_in_out_quad(128), 129);
    }

    #[test]
    fn lerp7by8_interpolates() {
        assert_eq!(lerp7by8(-64, 64, 0), -64);
        assert_eq!(lerp7by8(-64, 64, 128), 0);
        assert_eq!(lerp7by8(64, -64, 255), -64);
    }

    #[test]
    fn map8_maps_range() {
        assert_eq!(map8(0, 10, 20), 10);
        assert_eq!(map8(255, 10, 20), 20);
    }

    #[test]
    fn arduino_map_behaves_like_cpp() {
        assert_eq!(arduino_map(22, 10, 35, 0, 255), 122);
        assert_eq!(arduino_map(1500, 1000, 2000, 30, 0), 15);
        // map() does not clamp; out-of-range input overshoots (Motion clamps).
        assert_eq!(arduino_map(500, 1000, 2000, 30, 0), 45);
    }

    #[test]
    fn constrain_clamps() {
        assert_eq!(constrain(5, 0, 10), 5);
        assert_eq!(constrain(-5, 0, 10), 0);
        assert_eq!(constrain(50, 0, 10), 10);
    }
}
