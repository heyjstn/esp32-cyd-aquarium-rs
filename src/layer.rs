//! Off-screen RGB pixel layer with Adafruit-GFX-style drawing primitives,
//! ported from GFX_Lite's GFX_Layer / GFX (the geometry the aquarium draws
//! into before compositing).

use crate::color::{crgb_from_565, CRgb};

/// A pixel equal to this color is treated as transparent when compositing
/// (GFX_Layer's BLACK_BACKGROUND_PIXEL_COLOUR).
pub const TRANSPARENT: CRgb = CRgb::BLACK;

pub struct Layer {
    width: i16,
    height: i16,
    /// Row-major pixel buffer, `pixels[y * width + x]`.
    pixels: Vec<CRgb>,
}

impl Layer {
    pub fn new(width: i16, height: i16) -> Self {
        Self {
            width,
            height,
            pixels: vec![TRANSPARENT; width as usize * height as usize],
        }
    }

    pub fn width(&self) -> i16 {
        self.width
    }

    pub fn height(&self) -> i16 {
        self.height
    }

    pub fn clear(&mut self) {
        self.pixels.fill(TRANSPARENT);
    }

    pub fn get(&self, x: i16, y: i16) -> CRgb {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return TRANSPARENT;
        }
        self.pixels[y as usize * self.width as usize + x as usize]
    }

    pub fn draw_pixel(&mut self, x: i16, y: i16, color: CRgb) {
        if x < 0 || x >= self.width || y < 0 || y >= self.height {
            return;
        }
        self.pixels[y as usize * self.width as usize + x as usize] = color;
    }

    pub fn draw_pixel_565(&mut self, x: i16, y: i16, color: u16) {
        self.draw_pixel(x, y, crgb_from_565(color));
    }

    pub fn fill_rect(&mut self, x: i16, y: i16, w: i16, h: i16, color: CRgb) {
        for j in y..y + h {
            for i in x..x + w {
                self.draw_pixel(i, j, color);
            }
        }
    }

    pub fn fill_rect_565(&mut self, x: i16, y: i16, w: i16, h: i16, color: u16) {
        self.fill_rect(x, y, w, h, crgb_from_565(color));
    }

    pub fn draw_line(&mut self, x0: i16, y0: i16, x1: i16, y1: i16, color: CRgb) {
        let (mut x0, mut y0, mut x1, mut y1) = (x0, y0, x1, y1);
        if x0 == x1 {
            if y0 > y1 {
                core::mem::swap(&mut y0, &mut y1);
            }
            self.fill_rect(x0, y0, 1, y1 - y0 + 1, color);
            return;
        }
        if y0 == y1 {
            if x0 > x1 {
                core::mem::swap(&mut x0, &mut x1);
            }
            self.fill_rect(x0, y0, x1 - x0 + 1, 1, color);
            return;
        }

        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            core::mem::swap(&mut x0, &mut y0);
            core::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            core::mem::swap(&mut x0, &mut x1);
            core::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = (y1 - y0).abs();
        let mut err = dx / 2;
        let ystep = if y0 < y1 { 1 } else { -1 };

        while x0 <= x1 {
            if steep {
                self.draw_pixel(y0, x0, color);
            } else {
                self.draw_pixel(x0, y0, color);
            }
            err -= dy;
            if err < 0 {
                y0 += ystep;
                err += dx;
            }
            x0 += 1;
        }
    }

    pub fn fill_circle(&mut self, x0: i16, y0: i16, r: i16, color: CRgb) {
        self.fill_rect(x0, y0 - r, 1, 2 * r + 1, color);
        self.fill_circle_helper(x0, y0, r, 3, 0, color);
    }

    fn fill_circle_helper(
        &mut self,
        x0: i16,
        y0: i16,
        r: i16,
        corners: u8,
        delta: i16,
        color: CRgb,
    ) {
        let mut f = 1 - r;
        let mut dd_f_x = 1;
        let mut dd_f_y = -2 * r;
        let mut x = 0;
        let mut y = r;
        let mut px = x;
        let mut py = y;
        let delta = delta + 1;

        while x < y {
            if f >= 0 {
                y -= 1;
                dd_f_y += 2;
                f += dd_f_y;
            }
            x += 1;
            dd_f_x += 2;
            f += dd_f_x;
            // These checks avoid double-drawing certain lines.
            if x < y + 1 {
                if corners & 1 != 0 {
                    self.fill_rect(x0 + x, y0 - y, 1, 2 * y + delta, color);
                }
                if corners & 2 != 0 {
                    self.fill_rect(x0 - x, y0 - y, 1, 2 * y + delta, color);
                }
            }
            if y != py {
                if corners & 1 != 0 {
                    self.fill_rect(x0 + py, y0 - px, 1, 2 * px + delta, color);
                }
                if corners & 2 != 0 {
                    self.fill_rect(x0 - py, y0 - px, 1, 2 * px + delta, color);
                }
                py = y;
            }
            px = x;
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill_triangle(
        &mut self,
        x0: i16,
        y0: i16,
        x1: i16,
        y1: i16,
        x2: i16,
        y2: i16,
        color: CRgb,
    ) {
        let (mut x0, mut y0, mut x1, mut y1, mut x2, mut y2) = (x0, y0, x1, y1, x2, y2);

        if y0 > y1 {
            core::mem::swap(&mut y0, &mut y1);
            core::mem::swap(&mut x0, &mut x1);
        }
        if y1 > y2 {
            core::mem::swap(&mut y2, &mut y1);
            core::mem::swap(&mut x2, &mut x1);
        }
        if y0 > y1 {
            core::mem::swap(&mut y0, &mut y1);
            core::mem::swap(&mut x0, &mut x1);
        }

        if y0 == y2 {
            // Degenerate all-on-same-line case. GFX_Lite (what the original
            // firmware links against) fills this vertically; keep identical
            // behavior so the rendered scene matches.
            let mut a = x0;
            let mut b = x0;
            if x1 < a {
                a = x1;
            } else if x1 > b {
                b = x1;
            }
            if x2 < a {
                a = x2;
            } else if x2 > b {
                b = x2;
            }
            self.fill_rect(a, y0, 1, b - a + 1, color);
            return;
        }

        let dx01 = x1 - x0;
        let dy01 = (y1 - y0) as i32;
        let dx02 = x2 - x0;
        let dy02 = (y2 - y0) as i32;
        let dx12 = x2 - x1;
        let dy12 = (y2 - y1) as i32;
        let mut sa: i32 = 0;
        let mut sb: i32 = 0;

        let last = if y1 == y2 { y1 } else { y1 - 1 };
        let mut y = y0;
        while y <= last {
            let mut a = x0 + (sa / dy01) as i16;
            let mut b = x0 + (sb / dy02) as i16;
            sa += dx01 as i32;
            sb += dx02 as i32;
            if a > b {
                core::mem::swap(&mut a, &mut b);
            }
            self.fill_rect(a, y, b - a + 1, 1, color);
            y += 1;
        }

        sa = dx12 as i32 * (y - y1) as i32;
        sb = dx02 as i32 * (y - y0) as i32;
        while y <= y2 {
            let mut a = x1 + (sa / dy12) as i16;
            let mut b = x0 + (sb / dy02) as i16;
            sa += dx12 as i32;
            sb += dx02 as i32;
            if a > b {
                core::mem::swap(&mut a, &mut b);
            }
            self.fill_rect(a, y, b - a + 1, 1, color);
            y += 1;
        }
    }

    /// Row of filled circles forming a rotated ellipse (GFX drawCircleArray).
    pub fn draw_circle_array(
        &mut self,
        x: i16,
        y: i16,
        rad: i16,
        length: i16,
        angle: f32,
        color: CRgb,
    ) {
        let cos_angle = angle.cos();
        let sin_angle = angle.sin();

        let r = rad.min(length);
        let l = rad.max(length);

        let num_circles = r.max(l / 2);
        for i in -num_circles..=num_circles {
            let dx = i as f32 * -sin_angle;
            let dy = i as f32 * cos_angle;
            let circle_x = x + dx as i16;
            let circle_y = y + dy as i16;
            self.fill_circle(circle_x, circle_y, r, color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: CRgb = CRgb { r: 255, g: 0, b: 0 };

    #[test]
    fn draw_pixel_bounds_checked() {
        let mut l = Layer::new(8, 8);
        l.draw_pixel(0, 0, RED);
        l.draw_pixel(7, 7, RED);
        l.draw_pixel(-1, 0, RED);
        l.draw_pixel(0, 8, RED);
        assert_eq!(l.get(0, 0), RED);
        assert_eq!(l.get(7, 7), RED);
        assert_eq!(l.get(1, 1), TRANSPARENT);
    }

    #[test]
    fn fill_rect_covers_area() {
        let mut l = Layer::new(10, 10);
        l.fill_rect(2, 3, 4, 2, RED);
        assert_eq!(l.get(2, 3), RED);
        assert_eq!(l.get(5, 4), RED);
        assert_eq!(l.get(6, 4), TRANSPARENT);
        assert_eq!(l.get(2, 5), TRANSPARENT);
    }

    #[test]
    fn draw_line_horizontal_vertical_diagonal() {
        let mut l = Layer::new(10, 10);
        l.draw_line(0, 5, 9, 5, RED);
        for x in 0..10 {
            assert_eq!(l.get(x, 5), RED);
        }
        l.draw_line(2, 0, 2, 9, RED);
        for y in 0..10 {
            assert_eq!(l.get(2, y), RED);
        }
        l.draw_line(0, 0, 9, 9, RED);
        for i in 0..10 {
            assert_eq!(l.get(i, i), RED);
        }
    }

    #[test]
    fn fill_circle_touches_center_and_edges() {
        let mut l = Layer::new(21, 21);
        l.fill_circle(10, 10, 5, RED);
        assert_eq!(l.get(10, 10), RED);
        assert_eq!(l.get(10, 5), RED);
        assert_eq!(l.get(10, 15), RED);
        assert_eq!(l.get(5, 10), RED);
        assert_eq!(l.get(15, 10), RED);
        assert_eq!(l.get(0, 0), TRANSPARENT);
    }

    #[test]
    fn fill_triangle_covers_interior() {
        let mut l = Layer::new(20, 20);
        l.fill_triangle(2, 2, 18, 2, 10, 15, RED);
        assert_eq!(l.get(10, 3), RED);
        assert_eq!(l.get(10, 10), RED);
        assert_eq!(l.get(2, 2), RED);
        assert_eq!(l.get(18, 2), RED);
        assert_eq!(l.get(2, 15), TRANSPARENT);
    }

    #[test]
    fn circle_array_draws_stamps() {
        let mut l = Layer::new(40, 40);
        l.draw_circle_array(20, 20, 2, 6, 0.0, RED);
        // Several stamps along the perpendicular axis must exist.
        let mut painted = 0;
        for y in 0..40 {
            if l.get(20, y) == RED {
                painted += 1;
            }
        }
        assert!(painted > 6);
    }

    #[test]
    fn rgb565_path_expands_color() {
        let mut l = Layer::new(4, 4);
        l.fill_rect_565(0, 0, 1, 1, 0xF800);
        let c = l.get(0, 0);
        assert!(c.r > 240 && c.g == 0 && c.b == 0);
    }
}
