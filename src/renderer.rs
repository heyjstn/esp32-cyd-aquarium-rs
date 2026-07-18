//! Dot-matrix renderer: composites the foreground/background layers into
//! RGB565, expands each logical pixel into a 3x3 "LED dot" and pushes dirty
//! screen tiles to the display, ported from CydMatrix.cpp's row-buffer path
//! (CYD_FRAMEBUFFER_RENDERER = 0, CYD_DOT_RENDERER = 1).

use crate::consts;
use crate::layer::{Layer, TRANSPARENT};
use crate::tone::{apply_display_profile, pack_rgb565_for_display, ColorProfile};

const TFT_BLACK: u16 = 0x0000;
const UNSET: u16 = 0xFFFF;

/// Display backend consumed by the renderer (implemented by the ESP-IDF
/// display driver; tests use a recording sink).
pub trait FrameSink {
    fn fill_screen(&mut self, color: u16);
    fn push_rect(&mut self, x: u16, y: u16, w: u16, h: u16, pixels: &[u16]);
}

pub struct DotRenderer {
    /// Previous frame's logical RGB565 pixels for dirty-tile detection.
    prev_logical: Vec<u16>,
    /// Tile staging buffer (one strip of screen rows).
    tile: Vec<u16>,
    max_tile_height: u16,
    tile_logical_y: i32,
    tile_y0: u16,
    tile_y1: u16,
    tile_dirty: bool,
}

impl DotRenderer {
    pub fn new() -> Self {
        let max_tile_height = max_row_buffer_height();
        Self {
            prev_logical: vec![UNSET; consts::LOGICAL_WIDTH * consts::LOGICAL_HEIGHT],
            tile: vec![TFT_BLACK; consts::VIEWPORT_WIDTH as usize * max_tile_height as usize],
            max_tile_height,
            tile_logical_y: -1,
            tile_y0: 0,
            tile_y1: 0,
            tile_dirty: false,
        }
    }

    /// Composite foreground over background, render the dots and push dirty
    /// tiles to the sink. Clears the foreground layer, like the C++ version.
    pub fn composite(&mut self, foreground: &mut Layer, background: &Layer, sink: &mut dyn FrameSink) {
        for y in 0..consts::LOGICAL_HEIGHT as i16 {
            for x in 0..consts::LOGICAL_WIDTH as i16 {
                let fg_pixel = foreground.get(x, y);
                let has_foreground = fg_pixel != TRANSPARENT;
                let source = if has_foreground { fg_pixel } else { background.get(x, y) };
                let corrected = if has_foreground {
                    apply_display_profile(source, consts::FOREGROUND_BRIGHTNESS, consts::FOREGROUND_SATURATION, 0)
                } else {
                    apply_display_profile(source, consts::BACKGROUND_BRIGHTNESS, 128, consts::BACKGROUND_BLACK_THRESHOLD)
                };
                let color = pack_rgb565_for_display(
                    corrected.r,
                    corrected.g,
                    corrected.b,
                    255,
                    if has_foreground { ColorProfile::Foreground } else { ColorProfile::Background },
                );
                self.write_scaled_logical_pixel(x as u16, y as u16, color, sink);
            }
        }

        foreground.clear();
        self.flush(sink);
    }

    /// Reset dirty tracking and clear the physical screen (CydMatrix
    /// clearScreen equivalent).
    pub fn clear_screen(&mut self, sink: &mut dyn FrameSink) {
        self.prev_logical.fill(UNSET);
        self.tile_logical_y = -1;
        sink.fill_screen(TFT_BLACK);
    }

    fn reset_tile(&mut self, logical_y: u16) {
        let tile_start = (logical_y / consts::TILE_LOGICAL_ROWS) * consts::TILE_LOGICAL_ROWS;
        let tile_end = (tile_start + consts::TILE_LOGICAL_ROWS).min(consts::LOGICAL_HEIGHT as u16);

        self.tile_logical_y = tile_start as i32;
        self.tile_dirty = false;
        self.tile_y0 = consts::screen_y_for_logical_edge(tile_start);
        self.tile_y1 = consts::screen_y_for_logical_edge(tile_end);
        if self.tile_y1 <= self.tile_y0 {
            self.tile_y1 = self.tile_y0 + 1;
        }

        let row_height = (self.tile_y1 - self.tile_y0) as usize;
        self.tile[..consts::VIEWPORT_WIDTH as usize * row_height].fill(TFT_BLACK);
    }

    fn flush(&mut self, sink: &mut dyn FrameSink) {
        if self.tile_logical_y < 0 {
            return;
        }
        if self.tile_dirty {
            let row_height = self.tile_y1 - self.tile_y0;
            let len = consts::VIEWPORT_WIDTH as usize * row_height as usize;
            sink.push_rect(consts::VIEWPORT_X, self.tile_y0, consts::VIEWPORT_WIDTH, row_height, &self.tile[..len]);
        }
        self.tile_logical_y = -1;
    }

    fn write_scaled_logical_pixel(&mut self, x: u16, y: u16, color: u16, sink: &mut dyn FrameSink) {
        if x as usize >= consts::LOGICAL_WIDTH || y as usize >= consts::LOGICAL_HEIGHT {
            return;
        }

        if self.tile_logical_y < 0
            || y < self.tile_logical_y as u16
            || y >= self.tile_logical_y as u16 + consts::TILE_LOGICAL_ROWS
        {
            self.flush(sink);
            self.reset_tile(y);
        }

        let logical_index = y as usize * consts::LOGICAL_WIDTH + x as usize;
        if self.prev_logical[logical_index] != color {
            self.prev_logical[logical_index] = color;
            self.tile_dirty = true;
        }

        let x0 = consts::screen_x_for_logical_edge(x) - consts::VIEWPORT_X;
        let mut x1 = consts::screen_x_for_logical_edge(x + 1) - consts::VIEWPORT_X;
        if x1 <= x0 {
            x1 = x0 + 1;
        }

        let y0 = consts::screen_y_for_logical_edge(y) - self.tile_y0;
        let mut y1 = consts::screen_y_for_logical_edge(y + 1) - self.tile_y0;
        if y1 <= y0 {
            y1 = y0 + 1;
        }

        // Dot renderer: black pixels leave the (pre-cleared) tile untouched.
        if color == TFT_BLACK {
            return;
        }

        let width = if x1 > x0 { x1 - x0 } else { 1 };
        let height = if y1 > y0 { y1 - y0 } else { 1 };
        if width == 3 && height == 3 {
            // Fast path for the exact 3x3 cell: a plus-shaped dot.
            let cx = (x0 + 1) as usize;
            let cy = (y0 + 1) as usize;
            let stride = consts::VIEWPORT_WIDTH as usize;
            self.tile[(cy - 1) * stride + cx] = color;
            self.tile[cy * stride + cx - 1] = color;
            self.tile[cy * stride + cx] = color;
            self.tile[cy * stride + cx + 1] = color;
            self.tile[(cy + 1) * stride + cx] = color;
            return;
        }

        let min_dimension = width.min(height);
        let radius = (min_dimension as f32 * consts::DOT_RADIUS_RATIO).max(0.75);
        let radius_sq = radius * radius;
        let center_x = (x0 as f32 + x1 as f32 - 1.0) * 0.5;
        let center_y = (y0 as f32 + y1 as f32 - 1.0) * 0.5;

        let stride = consts::VIEWPORT_WIDTH as usize;
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = px as f32 - center_x;
                let dy = py as f32 - center_y;
                if dx * dx + dy * dy <= radius_sq {
                    self.tile[py as usize * stride + px as usize] = color;
                }
            }
        }
    }
}

/// CydMatrix's maxRowBufferHeight(): tallest screen-row strip any logical
/// tile maps to.
fn max_row_buffer_height() -> u16 {
    let mut max_height = 1;
    let mut y = 0;
    while y < consts::LOGICAL_HEIGHT as u16 {
        let y0 = consts::screen_y_for_logical_edge(y);
        let next_y = (y + consts::TILE_LOGICAL_ROWS).min(consts::LOGICAL_HEIGHT as u16);
        let y1 = consts::screen_y_for_logical_edge(next_y);
        let height = if y1 > y0 { y1 - y0 } else { 1 };
        max_height = max_height.max(height);
        y += consts::TILE_LOGICAL_ROWS;
    }
    max_height
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::CRgb;

    #[derive(Default)]
    struct RecordingSink {
        pushes: Vec<(u16, u16, u16, u16, Vec<u16>)>,
        fills: Vec<u16>,
    }

    impl FrameSink for RecordingSink {
        fn fill_screen(&mut self, color: u16) {
            self.fills.push(color);
        }
        fn push_rect(&mut self, x: u16, y: u16, w: u16, h: u16, pixels: &[u16]) {
            self.pushes.push((x, y, w, h, pixels.to_vec()));
        }
    }

    fn solid_layer(color: CRgb) -> Layer {
        let mut l = Layer::new(80, 106);
        l.fill_rect(0, 0, 80, 106, color);
        l
    }

    #[test]
    fn pushes_all_tiles_for_a_full_frame() {
        let mut renderer = DotRenderer::new();
        let mut sink = RecordingSink::default();
        let mut fg = Layer::new(80, 106);
        let bg = solid_layer(CRgb::new(0, 100, 80));

        renderer.composite(&mut fg, &bg, &mut sink);

        // 106 logical rows in tiles of 4 -> 27 tile pushes.
        assert_eq!(sink.pushes.len(), 27);
        let covered: u16 = sink.pushes.iter().map(|p| p.3).sum();
        assert_eq!(covered, 318);
        assert_eq!(sink.pushes[0].0, 0);
        assert_eq!(sink.pushes[0].1, 1);
        // Foreground layer is cleared after compositing.
        assert_eq!(fg.get(40, 50), TRANSPARENT);
    }

    #[test]
    fn identical_frame_pushes_nothing() {
        let mut renderer = DotRenderer::new();
        let mut sink = RecordingSink::default();
        let mut fg = Layer::new(80, 106);
        let bg = solid_layer(CRgb::new(0, 100, 80));

        renderer.composite(&mut fg, &bg, &mut sink);
        renderer.composite(&mut fg, &bg, &mut sink);
        assert_eq!(sink.pushes.len(), 27, "second identical frame should be a no-op");
    }

    #[test]
    fn only_dirty_tiles_are_pushed() {
        let mut renderer = DotRenderer::new();
        let mut sink = RecordingSink::default();
        let mut fg = Layer::new(80, 106);
        let bg = solid_layer(CRgb::new(0, 100, 80));

        renderer.composite(&mut fg, &bg, &mut sink);
        assert_eq!(sink.pushes.len(), 27);

        // Change one pixel in logical row 40 (tile 40..43, screen y 121..133).
        fg.draw_pixel(10, 40, CRgb::new(255, 0, 0));
        renderer.composite(&mut fg, &bg, &mut sink);

        assert_eq!(sink.pushes.len(), 28);
        let (x, y, w, h, _) = sink.pushes[27];
        assert_eq!((x, w), (0, 240));
        assert_eq!(y, consts::screen_y_for_logical_edge(40));
        assert!(h >= 3);
    }

    #[test]
    fn foreground_overrides_background() {
        let mut renderer = DotRenderer::new();
        let mut sink = RecordingSink::default();
        let mut fg = Layer::new(80, 106);
        let bg = solid_layer(CRgb::new(0, 100, 80));
        fg.draw_pixel(0, 0, CRgb::new(255, 255, 0));

        renderer.composite(&mut fg, &bg, &mut sink);

        // First tile contains the yellow pixel's dot (foreground profile)
        // on top of the uniform water background: exactly two colors.
        let tile = &sink.pushes[0].4;
        let mut distinct: Vec<u16> = tile.iter().copied().filter(|p| *p != 0).collect();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(distinct.len(), 2, "expected water + foreground colors, got {distinct:?}");
    }

    #[test]
    fn dot_is_plus_shaped_for_3x3_cells() {
        // 80x106 on 240x318 gives exact 3x3 cells, so the fast path applies.
        let mut renderer = DotRenderer::new();
        let mut sink = RecordingSink::default();
        let mut fg = Layer::new(80, 106);
        let mut bg = Layer::new(80, 106);
        bg.draw_pixel(5, 5, CRgb::new(200, 200, 200));

        renderer.composite(&mut fg, &bg, &mut sink);

        // Tile containing logical (5,5): rows 4..7 -> screen y 13..25.
        let tile_push = &sink.pushes[1];
        assert_eq!(tile_push.1, consts::screen_y_for_logical_edge(4));
        let tile = &tile_push.4;
        let lit: Vec<usize> = tile
            .iter()
            .enumerate()
            .filter(|(_, p)| **p != 0)
            .map(|(i, _)| i)
            .collect();
        // The dot occupies 5 pixels in a plus pattern.
        assert_eq!(lit.len(), 5);
        let base_x = 5 * 3;
        let base_y = (5 * 3 + 1) - tile_push.1 as usize; // screen y of logical row 5 minus tile origin
        let stride = 240;
        let center = (base_y + 1) * stride + base_x + 1;
        let mut expected = vec![center - stride, center - 1, center, center + 1, center + stride];
        expected.sort();
        let mut lit_sorted = lit.clone();
        lit_sorted.sort();
        assert_eq!(lit_sorted, expected);
    }

    #[test]
    fn clear_resets_dirty_tracking() {
        let mut renderer = DotRenderer::new();
        let mut sink = RecordingSink::default();
        let mut fg = Layer::new(80, 106);
        let bg = solid_layer(CRgb::new(0, 100, 80));

        renderer.composite(&mut fg, &bg, &mut sink);
        renderer.clear_screen(&mut sink);
        assert_eq!(sink.fills, vec![0]);
        renderer.composite(&mut fg, &bg, &mut sink);
        assert_eq!(sink.pushes.len(), 27 + 27);
    }
}
