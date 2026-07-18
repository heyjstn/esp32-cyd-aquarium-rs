//! Color types and conversions, ported from FastLED's pixeltypes/hsv2rgb/
//! colorpalettes as bundled in GFX_Lite, plus TFT_eSPI's color565.

use crate::math8::{scale8, scale8_video};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CRgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CRgb {
    pub const BLACK: CRgb = CRgb { r: 0, g: 0, b: 0 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// FastLED CRGB::nscale8 (scale8 with FASTLED_SCALE8_FIXED = 1).
    pub fn nscale8(&mut self, scale_down: u8) {
        self.r = scale8(self.r, scale_down);
        self.g = scale8(self.g, scale_down);
        self.b = scale8(self.b, scale_down);
    }

    pub fn scaled(self, scale_down: u8) -> Self {
        let mut c = self;
        c.nscale8(scale_down);
        c
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CHsv {
    pub h: u8,
    pub s: u8,
    pub v: u8,
}

impl CHsv {
    pub const fn new(h: u8, s: u8, v: u8) -> Self {
        Self { h, s, v }
    }
}

// Hue wheel constants used by hsv2rgb_rainbow.
const K255: u8 = 255;
const K171: u8 = 171;
const K170: u8 = 170;
const K85: u8 = 85;

/// FastLED hsv2rgb_rainbow (rainbow hue curve, Y1 yellow boost path,
/// FASTLED_SCALE8_FIXED = 1 saturation/value handling).
pub fn hsv2rgb_rainbow(hsv: CHsv) -> CRgb {
    let hue = hsv.h;
    let sat = hsv.s;
    let val = hsv.v;

    let offset = hue & 0x1F; // 0..31
    let offset8 = offset << 3;
    let third = scale8(offset8, 85); // max 85

    let (mut r, mut g, mut b): (u8, u8, u8);
    if hue & 0x80 == 0 {
        if hue & 0x40 == 0 {
            if hue & 0x20 == 0 {
                // case 0: R -> O
                r = K255 - third;
                g = third;
                b = 0;
            } else {
                // case 1: O -> Y (Y1)
                r = K171;
                g = K85 + third;
                b = 0;
            }
        } else if hue & 0x20 == 0 {
            // case 2: Y -> G (Y1)
            let twothirds = scale8(offset8, 170); // max 170
            r = K171 - twothirds;
            g = K170 + third;
            b = 0;
        } else {
            // case 3: G -> A
            r = 0;
            g = K255 - third;
            b = third;
        }
    } else if hue & 0x40 == 0 {
        if hue & 0x20 == 0 {
            // case 4: A -> B
            let twothirds = scale8(offset8, 170); // max 170
            r = 0;
            g = K171 - twothirds;
            b = K85 + twothirds;
        } else {
            // case 5: B -> P
            r = third;
            g = 0;
            b = K255 - third;
        }
    } else if hue & 0x20 == 0 {
        // case 6: P -> K
        r = K85 + third;
        g = 0;
        b = K171 - third;
    } else {
        // case 7: K -> R
        r = K170 + third;
        g = 0;
        b = K85 - third;
    }

    if sat != 255 {
        if sat == 0 {
            r = 255;
            g = 255;
            b = 255;
        } else {
            let desat = scale8_video(255 - sat, 255 - sat);
            let satscale = 255 - desat;
            r = scale8(r, satscale);
            g = scale8(g, satscale);
            b = scale8(b, satscale);
            r = r.wrapping_add(desat);
            g = g.wrapping_add(desat);
            b = b.wrapping_add(desat);
        }
    }

    if val != 255 {
        let v = scale8_video(val, val);
        if v == 0 {
            r = 0;
            g = 0;
            b = 0;
        } else {
            r = scale8(r, v);
            g = scale8(g, v);
            b = scale8(b, v);
        }
    }

    CRgb { r, g, b }
}

/// 16-entry RGB palette (FastLED CRGBPalette16).
pub type Palette16 = [CRgb; 16];

/// FastLED ColorFromPalette with LINEARBLEND; the aquarium water always
/// passes brightness 255, so the brightness scaling path is omitted.
pub fn color_from_palette(pal: &Palette16, index: u8) -> CRgb {
    let hi4 = (index >> 4) as usize;
    let lo4 = index & 0x0F;

    let entry = pal[hi4];
    if lo4 == 0 {
        return entry;
    }

    let next = if hi4 == 15 { pal[0] } else { pal[hi4 + 1] };
    let f2 = lo4 << 4;
    let f1 = 255 - f2;

    CRgb {
        r: scale8(entry.r, f1).wrapping_add(scale8(next.r, f2)),
        g: scale8(entry.g, f1).wrapping_add(scale8(next.g, f2)),
        b: scale8(entry.b, f1).wrapping_add(scale8(next.b, f2)),
    }
}

/// The aquarium water temperature palette (Water.h), 10°C deep blue to
/// 34–35°C bright red.
pub const WATER_PALETTE: Palette16 = [
    CRgb::new(0, 28, 72),
    CRgb::new(0, 32, 80),
    CRgb::new(0, 40, 88),
    CRgb::new(0, 52, 96),
    CRgb::new(0, 64, 96),
    CRgb::new(0, 84, 92),
    CRgb::new(0, 96, 86),
    CRgb::new(0, 100, 80),
    CRgb::new(0, 100, 80),
    CRgb::new(12, 96, 72),
    CRgb::new(28, 88, 56),
    CRgb::new(56, 76, 36),
    CRgb::new(80, 60, 20),
    CRgb::new(96, 36, 12),
    CRgb::new(100, 16, 8),
    CRgb::new(100, 0, 0),
];

/// TFT_eSPI color565 packing.
pub fn color565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | (b as u16 >> 3)
}

/// GFX_Layer's uint16_t -> CRGB expansion (drawPixel(int16, int16, uint16)).
pub fn crgb_from_565(color: u16) -> CRgb {
    let r = ((((color >> 11) & 0x1F) * 527 + 23) >> 6) as u8;
    let g = ((((color >> 5) & 0x3F) * 259 + 33) >> 6) as u8;
    let b = (((color & 0x1F) * 527 + 23) >> 6) as u8;
    CRgb { r, g, b }
}

/// Rec.601 luma used by the TFT foreground profile.
pub fn luma8(r: u8, g: u8, b: u8) -> u8 {
    ((r as u16 * 77 + g as u16 * 150 + b as u16 * 29) >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nscale8_scales_channels() {
        let mut c = CRgb::new(255, 128, 64);
        c.nscale8(128);
        assert_eq!(c, CRgb::new(128, 64, 32));
        c.nscale8(0);
        assert_eq!(c, CRgb::BLACK);
    }

    #[test]
    fn hsv_rainbow_primary_hues() {
        assert_eq!(hsv2rgb_rainbow(CHsv::new(0, 255, 255)), CRgb::new(255, 0, 0));
        let green = hsv2rgb_rainbow(CHsv::new(96, 255, 255));
        assert_eq!(green, CRgb::new(0, 255, 0));
        let blue = hsv2rgb_rainbow(CHsv::new(160, 255, 255));
        assert_eq!(blue, CRgb::new(0, 0, 255));
    }

    #[test]
    fn hsv_rainbow_zero_saturation_is_white() {
        assert_eq!(hsv2rgb_rainbow(CHsv::new(0, 0, 255)), CRgb::new(255, 255, 255));
    }

    #[test]
    fn hsv_rainbow_zero_value_is_black() {
        assert_eq!(hsv2rgb_rainbow(CHsv::new(200, 130, 0)), CRgb::BLACK);
    }

    #[test]
    fn palette_exact_entries() {
        for (i, entry) in WATER_PALETTE.iter().enumerate() {
            assert_eq!(color_from_palette(&WATER_PALETTE, (i * 16) as u8), *entry);
        }
    }

    #[test]
    fn palette_blends_between_entries() {
        let blended = color_from_palette(&WATER_PALETTE, 8);
        let a = WATER_PALETTE[0];
        let b = WATER_PALETTE[1];
        let expected = CRgb::new(
            scale8(a.r, 127).wrapping_add(scale8(b.r, 128)),
            scale8(a.g, 127).wrapping_add(scale8(b.g, 128)),
            scale8(a.b, 127).wrapping_add(scale8(b.b, 128)),
        );
        assert_eq!(blended, expected);
    }

    #[test]
    fn color565_packing() {
        assert_eq!(color565(255, 0, 0), 0xF800);
        assert_eq!(color565(0, 255, 0), 0x07E0);
        assert_eq!(color565(0, 0, 255), 0x001F);
        assert_eq!(color565(255, 255, 255), 0xFFFF);
        assert_eq!(color565(0, 0, 0), 0x0000);
    }

    #[test]
    fn rgb565_roundtrip_approx() {
        let c = crgb_from_565(color565(224, 248, 238));
        // The expansion intentionally rounds toward the top of each cell.
        assert!((c.r as i16 - 224).abs() <= 8);
        assert!((c.g as i16 - 248).abs() <= 4);
        assert!((c.b as i16 - 238).abs() <= 8);
    }

    #[test]
    fn luma_weights() {
        assert_eq!(luma8(255, 255, 255), 255);
        assert_eq!(luma8(255, 0, 0), 76);
        assert_eq!(luma8(0, 0, 0), 0);
    }
}
