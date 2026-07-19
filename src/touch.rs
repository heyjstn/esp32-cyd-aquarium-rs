#[cfg(any(target_os = "espidf", test))]
use crate::math8::arduino_map;

#[cfg(any(target_os = "espidf", test))]
pub(crate) fn map_raw_axis(
    raw: u16,
    raw_min: i32,
    raw_max: i32,
    out_max: u16,
    invert: bool,
) -> u16 {
    if raw_max <= raw_min {
        return 0;
    }

    let limited = (raw as i32).clamp(raw_min, raw_max);
    let mapped = arduino_map(
        limited as i64,
        raw_min as i64,
        raw_max as i64,
        0,
        out_max as i64,
    );
    let value = mapped.clamp(0, out_max as i64) as u16;
    if invert {
        out_max - value
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_axis_mapping_clamps_and_scales() {
        assert_eq!(map_raw_axis(0, 200, 3900, 239, false), 0);
        assert_eq!(map_raw_axis(200, 200, 3900, 239, false), 0);
        assert_eq!(map_raw_axis(2050, 200, 3900, 239, false), 119);
        assert_eq!(map_raw_axis(3900, 200, 3900, 239, false), 239);
        assert_eq!(map_raw_axis(4095, 200, 3900, 239, false), 239);
    }

    #[test]
    fn raw_axis_mapping_can_invert_and_reject_invalid_ranges() {
        assert_eq!(map_raw_axis(200, 200, 3900, 239, true), 239);
        assert_eq!(map_raw_axis(3900, 200, 3900, 239, true), 0);
        assert_eq!(map_raw_axis(500, 1000, 1000, 239, false), 0);
    }
}
