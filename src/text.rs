//! Clock overlay: 3x5 pixel glyphs, epoch-to-civil time conversion and the
//! text layout used by the CYD firmware's renderClockOverlay().

use crate::layer::Layer;

/// A 5-row glyph where each row is a string of '1' (lit) pixels.
pub type PixelGlyph = [&'static str; 5];

pub fn glyph_for(raw: char) -> PixelGlyph {
    let c = raw.to_ascii_uppercase();
    match c {
        '0' => ["111", "101", "101", "101", "111"],
        '1' => ["010", "110", "010", "010", "111"],
        '2' => ["111", "001", "111", "100", "111"],
        '3' => ["111", "001", "111", "001", "111"],
        '4' => ["101", "101", "111", "001", "001"],
        '5' => ["111", "100", "111", "001", "111"],
        '6' => ["111", "100", "111", "101", "111"],
        '7' => ["111", "001", "010", "010", "010"],
        '8' => ["111", "101", "111", "101", "111"],
        '9' => ["111", "101", "111", "001", "111"],
        'A' => ["010", "101", "111", "101", "101"],
        'B' => ["110", "101", "110", "101", "110"],
        'C' => ["111", "100", "100", "100", "111"],
        'D' => ["110", "101", "101", "101", "110"],
        'E' => ["111", "100", "111", "100", "111"],
        'F' => ["111", "100", "111", "100", "100"],
        'G' => ["111", "100", "101", "101", "111"],
        'H' => ["101", "101", "111", "101", "101"],
        'I' => ["111", "010", "010", "010", "111"],
        'J' => ["001", "001", "001", "101", "111"],
        'K' => ["101", "101", "110", "101", "101"],
        'L' => ["100", "100", "100", "100", "111"],
        'M' => ["101", "111", "111", "101", "101"],
        'N' => ["101", "111", "111", "111", "101"],
        'O' => ["111", "101", "101", "101", "111"],
        'P' => ["111", "101", "111", "100", "100"],
        'Q' => ["111", "101", "101", "111", "001"],
        'R' => ["111", "101", "111", "110", "101"],
        'S' => ["111", "100", "111", "001", "111"],
        'T' => ["111", "010", "010", "010", "010"],
        'U' => ["101", "101", "101", "101", "111"],
        'V' => ["101", "101", "101", "101", "010"],
        'W' => ["101", "101", "111", "111", "101"],
        'X' => ["101", "101", "010", "101", "101"],
        'Y' => ["101", "101", "010", "010", "010"],
        'Z' => ["111", "001", "010", "100", "111"],
        ':' => ["0", "1", "0", "1", "0"],
        '.' => ["0", "0", "0", "0", "1"],
        '/' => ["001", "001", "010", "100", "100"],
        '-' => ["000", "000", "111", "000", "000"],
        _ => ["0", "0", "0", "0", "0"],
    }
}

pub fn glyph_width(glyph: &PixelGlyph) -> u8 {
    glyph.iter().map(|row| row.len() as u8).max().unwrap_or(0)
}

pub fn pixel_text_width(text: &str, scale: u8, letter_spacing: u8) -> u16 {
    let mut width = 0u16;
    let len = text.chars().count();
    for (i, c) in text.chars().enumerate() {
        width += glyph_width(&glyph_for(c)) as u16 * scale as u16;
        if i + 1 < len {
            width += letter_spacing as u16;
        }
    }
    width
}

/// Draw pixel text into a layer. Colors are RGB565, matching the original
/// firmware which fills the foreground layer with 565 colors.
pub fn draw_pixel_text(
    layer: &mut Layer,
    text: &str,
    x: i16,
    y: i16,
    scale: u8,
    letter_spacing: u8,
    color565: u16,
) {
    let mut cursor_x = x;
    for c in text.chars() {
        let glyph = glyph_for(c);
        let width = glyph_width(&glyph);
        for (row, bits) in glyph.iter().enumerate() {
            for (col, bit) in bits.chars().enumerate() {
                if bit != '1' {
                    continue;
                }
                layer.fill_rect_565(
                    cursor_x + col as i16 * scale as i16,
                    y + row as i16 * scale as i16,
                    scale as i16,
                    scale as i16,
                    color565,
                );
            }
        }
        cursor_x += width as i16 * scale as i16 + letter_spacing as i16;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_centered_pixel_text(
    layer: &mut Layer,
    text: &str,
    center_x: i16,
    y: i16,
    scale: u8,
    letter_spacing: u8,
    color565: u16,
    shadow565: u16,
) {
    let width = pixel_text_width(text, scale, letter_spacing) as i16;
    let x = center_x - width / 2;
    draw_pixel_text(layer, text, x + 1, y + 1, scale, letter_spacing, shadow565);
    draw_pixel_text(layer, text, x, y, scale, letter_spacing, color565);
}

// ---------------------------------------------------------------------------
// Civil time
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Civil {
    pub year: i32,
    pub month: u8,  // 1..=12
    pub day: u8,    // 1..=31
    pub hour: u8,   // 0..=23
    pub minute: u8, // 0..=59
    pub second: u8, // 0..=59
    /// 0 = Sunday .. 6 = Saturday (matches tm_wday).
    pub weekday: u8,
}

/// Convert Unix epoch seconds (UTC) to civil time.
/// Uses Howard Hinnant's days-from-civil algorithm; no heap, no libc.
pub fn civil_from_epoch(epoch_secs: i64) -> Civil {
    let days = epoch_secs.div_euclid(86_400);
    let secs_of_day = epoch_secs.rem_euclid(86_400);

    // civil_from_days
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    Civil {
        year: year as i32,
        month: m as u8,
        day: d as u8,
        hour: (secs_of_day / 3600) as u8,
        minute: ((secs_of_day % 3600) / 60) as u8,
        second: (secs_of_day % 60) as u8,
        // 1970-01-01 was a Thursday (tm_wday 4).
        weekday: ((days + 4).rem_euclid(7)) as u8,
    }
}

pub const DAY_LABELS: [&str; 7] = ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"];
pub const MONTH_LABELS: [&str; 12] = [
    "JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC",
];

/// The three clock strings drawn on the top row, plus the date line.
/// Mirrors renderClockOverlay(): 12-hour time, blinking colon, AM/PM,
/// zero-padded seconds and "SUN 05 JAN" date.
pub struct ClockTexts {
    pub time: String,
    pub meridiem: String,
    pub seconds: String,
    pub date: String,
}

pub fn clock_texts(c: &Civil) -> ClockTexts {
    let is_pm = c.hour >= 12;
    let mut hour12 = c.hour % 12;
    if hour12 == 0 {
        hour12 = 12;
    }
    let colon = if c.second.is_multiple_of(2) { ':' } else { ' ' };

    ClockTexts {
        time: format!("{}{}{:02}", hour12, colon, c.minute),
        meridiem: if is_pm { "PM".into() } else { "AM".into() },
        seconds: format!("{:02}", c.second),
        date: format!(
            "{} {:02} {}",
            DAY_LABELS[c.weekday as usize],
            c.day,
            MONTH_LABELS[(c.month - 1) as usize]
        ),
    }
}

/// Layout constants from renderClockOverlay().
pub const CLOCK_TIME_SCALE: u8 = 2;
pub const CLOCK_TIME_SPACING: u8 = 1;
pub const CLOCK_SUFFIX_SCALE: u8 = 1;
pub const CLOCK_SUFFIX_SPACING: u8 = 1;
pub const CLOCK_SUFFIX_GAP: u8 = 2;
pub const CLOCK_TIME_Y: i16 = 3;
pub const CLOCK_SECONDS_OFFSET_Y: i16 = 6;
pub const CLOCK_DATE_BOTTOM_MARGIN: i16 = 8;

/// Draw the full clock overlay into the foreground layer.
/// Colors are RGB565 values (time/date/shadow), as in the C++ build.
pub fn render_clock_overlay(
    layer: &mut Layer,
    civil: &Civil,
    time_color: u16,
    date_color: u16,
    shadow_color: u16,
) {
    let texts = clock_texts(civil);
    let center_x = layer.width() / 2;

    let time_width = pixel_text_width(&texts.time, CLOCK_TIME_SCALE, CLOCK_TIME_SPACING) as i16;
    let suffix_width =
        pixel_text_width(&texts.meridiem, CLOCK_SUFFIX_SCALE, CLOCK_SUFFIX_SPACING) as i16;
    let total_width = time_width + CLOCK_SUFFIX_GAP as i16 + suffix_width;
    let time_x = center_x - total_width / 2;
    let suffix_x = time_x + time_width + CLOCK_SUFFIX_GAP as i16;
    let time_y = CLOCK_TIME_Y;
    let seconds_y = time_y + CLOCK_SECONDS_OFFSET_Y;

    draw_pixel_text(
        layer,
        &texts.time,
        time_x + 1,
        time_y + 1,
        CLOCK_TIME_SCALE,
        CLOCK_TIME_SPACING,
        shadow_color,
    );
    draw_pixel_text(
        layer,
        &texts.meridiem,
        suffix_x + 1,
        time_y + 1,
        CLOCK_SUFFIX_SCALE,
        CLOCK_SUFFIX_SPACING,
        shadow_color,
    );
    draw_pixel_text(
        layer,
        &texts.seconds,
        suffix_x + 1,
        seconds_y + 1,
        CLOCK_SUFFIX_SCALE,
        CLOCK_SUFFIX_SPACING,
        shadow_color,
    );
    draw_pixel_text(
        layer,
        &texts.time,
        time_x,
        time_y,
        CLOCK_TIME_SCALE,
        CLOCK_TIME_SPACING,
        time_color,
    );
    draw_pixel_text(
        layer,
        &texts.meridiem,
        suffix_x,
        time_y,
        CLOCK_SUFFIX_SCALE,
        CLOCK_SUFFIX_SPACING,
        time_color,
    );
    draw_pixel_text(
        layer,
        &texts.seconds,
        suffix_x,
        seconds_y,
        CLOCK_SUFFIX_SCALE,
        CLOCK_SUFFIX_SPACING,
        date_color,
    );
    draw_centered_pixel_text(
        layer,
        &texts.date,
        center_x,
        layer.height() - CLOCK_DATE_BOTTOM_MARGIN,
        1,
        1,
        date_color,
        shadow_color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_zero_is_1970_01_01_thursday() {
        let c = civil_from_epoch(0);
        assert_eq!(
            c,
            Civil {
                year: 1970,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
                weekday: 4
            }
        );
    }

    #[test]
    fn known_timestamp() {
        // 2024-02-29 12:34:56 UTC (leap day), a Thursday.
        let c = civil_from_epoch(1_709_210_096);
        assert_eq!(c.year, 2024);
        assert_eq!(c.month, 2);
        assert_eq!(c.day, 29);
        assert_eq!(c.hour, 12);
        assert_eq!(c.minute, 34);
        assert_eq!(c.second, 56);
        assert_eq!(c.weekday, 4);
    }

    #[test]
    fn negative_epoch() {
        // 1969-12-31 23:59:59 UTC, a Wednesday.
        let c = civil_from_epoch(-1);
        assert_eq!(
            (c.year, c.month, c.day, c.hour, c.minute, c.second),
            (1969, 12, 31, 23, 59, 59)
        );
        assert_eq!(c.weekday, 3);
    }

    #[test]
    fn clock_texts_formats() {
        let base = Civil {
            year: 2026,
            month: 7,
            day: 5,
            hour: 0,
            minute: 5,
            second: 0,
            weekday: 0,
        };
        let t = clock_texts(&base);
        assert_eq!(t.time, "12:05");
        assert_eq!(t.meridiem, "AM");
        assert_eq!(t.seconds, "00");
        assert_eq!(t.date, "SUN 05 JUL");

        let pm = clock_texts(&Civil {
            hour: 13,
            minute: 9,
            second: 3,
            ..base
        });
        assert_eq!(pm.time, "1 09"); // odd second: colon hidden
        assert_eq!(pm.meridiem, "PM");

        let noon = clock_texts(&Civil { hour: 12, ..base });
        assert_eq!(noon.time, "12:05");
        assert_eq!(noon.meridiem, "PM");
    }

    #[test]
    fn glyph_widths() {
        assert_eq!(glyph_width(&glyph_for('0')), 3);
        assert_eq!(glyph_width(&glyph_for(':')), 1);
        assert_eq!(glyph_width(&glyph_for(' ')), 1);
    }

    #[test]
    fn text_width_accounts_spacing() {
        // '1','2','0','5' are 3 wide, ':' is 1 wide: (3+3+1+3+3)*2 + 4 gaps.
        assert_eq!(pixel_text_width("12:05", 2, 1), 30);
        assert_eq!(pixel_text_width("AM", 1, 1), 3 + 1 + 3);
    }

    #[test]
    fn overlay_draws_pixels() {
        let mut layer = Layer::new(80, 106);
        let c = Civil {
            year: 2026,
            month: 1,
            day: 2,
            hour: 9,
            minute: 8,
            second: 7,
            weekday: 5,
        };
        render_clock_overlay(&mut layer, &c, 0xFFFF, 0x07E0, 0x0000);
        let lit = (0..80)
            .map(|x| layer.get(x, 3))
            .filter(|p| *p != crate::color::CRgb::BLACK)
            .count();
        assert!(lit > 4, "time row should contain lit pixels");
        let date_row = (0..80)
            .map(|x| layer.get(x, 98))
            .filter(|p| *p != crate::color::CRgb::BLACK)
            .count();
        assert!(date_row > 4, "date row should contain lit pixels");
    }
}
