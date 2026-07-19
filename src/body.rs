//! Creature bodies: color palette, heads, tails, fins and the five body
//! shapes (fish, star, turtle, snake, octopus), ported from
//! lib/Aquarium/Body/*.

use crate::color::{hsv2rgb_rainbow, CHsv, CRgb};
use crate::consts;
use crate::layer::Layer;
use crate::rng::Rng;
use crate::vec2::Vec2;
use core::f32::consts::PI;

const TWO_PI: f32 = 2.0 * PI;

// ---------------------------------------------------------------------------
// ColorPalette
// ---------------------------------------------------------------------------

pub struct ColorPalette {
    pub colors_hsv: Vec<CHsv>,
    pub colors: Vec<CRgb>,
}

impl ColorPalette {
    pub fn new(size: usize, stripes: bool, rng: &mut Rng) -> Self {
        let base_hue = rng.range(0, 256) as u8;
        let hue_step = rng.range(5, 31) as u8;

        let mut colors_hsv = Vec::with_capacity(size);
        let mut colors = Vec::with_capacity(size);
        for i in 0..size {
            let hue = base_hue.wrapping_add((i as u8).wrapping_mul(hue_step));
            let hsv = CHsv::new(hue, 130, 255);
            colors_hsv.push(hsv);
            colors.push(hsv2rgb_rainbow(hsv));
        }

        let mut palette = Self { colors_hsv, colors };
        if stripes {
            palette.apply_stripes(rng);
        }
        palette
    }

    pub fn from_hsv(colors: Vec<CHsv>) -> Self {
        let rgb = colors.iter().map(|c| hsv2rgb_rainbow(*c)).collect();
        Self {
            colors_hsv: colors,
            colors: rgb,
        }
    }

    fn update_rgb(&mut self) {
        for (i, hsv) in self.colors_hsv.iter().enumerate() {
            self.colors[i] = hsv2rgb_rainbow(*hsv);
        }
    }

    pub fn adjust_color_by_age_and_health(&mut self, age: f32, health: f32) {
        for hsv in &mut self.colors_hsv {
            let mut age_factor = 1.0f32;
            if age >= consts::AGE_ADULT {
                age_factor = 1.0
                    - ((age - consts::AGE_ADULT) / (consts::AGE_DEAD - consts::AGE_ADULT)) * 0.5;
            }
            hsv.s = (115.0 * health) as u8;
            hsv.v = (255.0 * age_factor) as u8;
        }
        self.update_rgb();
    }

    fn apply_stripes(&mut self, rng: &mut Rng) {
        let swap_type = rng.range(0, 4);
        for i in (1..self.colors_hsv.len()).step_by(2) {
            match swap_type {
                0 => self.colors_hsv[i].h = self.colors_hsv[i].h.wrapping_add(85),
                1 => self.colors_hsv[i].h = self.colors_hsv[i].h.wrapping_add(170),
                _ => {}
            }
        }
        self.update_rgb();
    }
}

// ---------------------------------------------------------------------------
// Heads
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Head {
    Triangle,
    Frog,
    Needle { nose_len_multiplier: i32 },
}

impl Head {
    pub fn random(rng: &mut Rng) -> Self {
        match rng.range(0, 3) {
            0 => Head::Triangle,
            1 => Head::Frog,
            _ => Head::Needle {
                nose_len_multiplier: rng.range(
                    consts::FISH_NEEDLE_NOSE_LENGTH_MULTIPLIER.0,
                    consts::FISH_NEEDLE_NOSE_LENGTH_MULTIPLIER.1,
                ),
            },
        }
    }

    pub fn from_name(name: &str, rng: &mut Rng) -> Self {
        match name {
            "TriangleHead" => Head::Triangle,
            "FrogHead" => Head::Frog,
            "NeedleHead" => Head::Needle {
                nose_len_multiplier: rng.range(
                    consts::FISH_NEEDLE_NOSE_LENGTH_MULTIPLIER.0,
                    consts::FISH_NEEDLE_NOSE_LENGTH_MULTIPLIER.1,
                ),
            },
            _ => Head::random(rng),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Head::Triangle => "TriangleHead",
            Head::Frog => "FrogHead",
            Head::Needle { .. } => "NeedleHead",
        }
    }

    fn display(&self, layer: &mut Layer, position: Vec2, angle: f32, size: u8, color: CRgb) {
        match self {
            Head::Triangle => {
                let mut shift = Vec2::from_angle(angle + PI);
                shift *= size as f32 * 0.25;
                let shifted = position + shift;

                let mut pt1 = Vec2::from_angle(angle);
                pt1 *= size as f32;
                pt1 += shifted;
                let mut pt2 = Vec2::from_angle(angle - PI / 2.0);
                pt2 *= size as f32 / 2.0;
                pt2 += shifted;
                let mut pt3 = Vec2::from_angle(angle + PI / 2.0);
                pt3 *= size as f32 / 2.0;
                pt3 += shifted;
                layer.fill_triangle(
                    pt1.x as i16,
                    pt1.y as i16,
                    pt2.x as i16,
                    pt2.y as i16,
                    pt3.x as i16,
                    pt3.y as i16,
                    color,
                );
            }
            Head::Frog => {
                // FrogHead draws with channels rotated (b, g, r) in C++.
                let swapped = CRgb::new(color.b, color.g, color.r);
                let mut heading = Vec2::from_angle(angle);
                heading *= 0.5;
                let mut pt = Vec2::from_angle(angle + PI / 2.0);
                pt.set_mag(size as f32);
                pt += heading;
                pt += position;
                layer.fill_circle(pt.x as i16, pt.y as i16, (size / 2) as i16, swapped);
                let mut pt = Vec2::from_angle(angle + PI / 2.0);
                pt.set_mag(-(size as f32));
                pt += heading;
                pt += position;
                layer.fill_circle(pt.x as i16, pt.y as i16, (size / 2) as i16, swapped);
            }
            Head::Needle {
                nose_len_multiplier,
            } => {
                let mut pt1 = Vec2::from_angle(angle);
                pt1 *= size as f32 * *nose_len_multiplier as f32;
                pt1 += position;
                layer.draw_line(
                    pt1.x as i16,
                    pt1.y as i16,
                    position.x as i16,
                    position.y as i16,
                    color,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tails
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Tail {
    None,
    Triangle,
    Curvy,
    Wavy { segment_positions: Vec<Vec2> },
}

impl Tail {
    pub fn random(rng: &mut Rng) -> Self {
        match rng.range(0, 3) {
            0 => Tail::Triangle,
            1 => Tail::Curvy,
            _ => {
                let count = rng.range(5, 15) as usize;
                Tail::Wavy {
                    segment_positions: vec![Vec2::ZERO; count],
                }
            }
        }
    }

    pub fn from_name(name: &str, rng: &mut Rng) -> Self {
        match name {
            "noTail" => Tail::None,
            "TriangleTail" => Tail::Triangle,
            "CurvyTail" => Tail::Curvy,
            "WavyTail" => {
                let count = rng.range(5, 15) as usize;
                Tail::Wavy {
                    segment_positions: vec![Vec2::ZERO; count],
                }
            }
            _ => Tail::random(rng),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Tail::None => "noTail",
            Tail::Triangle => "TriangleTail",
            Tail::Curvy => "CurvyTail",
            Tail::Wavy { .. } => "WavyTail",
        }
    }

    fn display(&mut self, layer: &mut Layer, pos: Vec2, angle: f32, size: u8, color: CRgb) {
        match self {
            Tail::None => {}
            Tail::Triangle => {
                let heading = Vec2::from_angle(angle);
                let mut pt1 = heading;
                pt1.set_mag(size as f32);
                let mut pt2 = heading;
                pt2.set_mag(-(size as f32));
                let heading3 = heading * 3.0;
                let mut pt3 = Vec2::from_angle(angle + PI / 2.0);
                pt3.set_mag(size as f32 * 3.0);
                pt3 -= heading3;
                pt1 += pos;
                pt2 += pos;
                pt3 += pos;
                layer.fill_triangle(
                    pt1.x as i16,
                    pt1.y as i16,
                    pt2.x as i16,
                    pt2.y as i16,
                    pt3.x as i16,
                    pt3.y as i16,
                    color,
                );
                let mut pt3 = Vec2::from_angle(angle + PI / 2.0);
                pt3.set_mag(-(size as f32) * 3.0);
                pt3 -= heading3;
                pt3 += pos;
                layer.fill_triangle(
                    pt1.x as i16,
                    pt1.y as i16,
                    pt2.x as i16,
                    pt2.y as i16,
                    pt3.x as i16,
                    pt3.y as i16,
                    color,
                );
            }
            Tail::Curvy => {
                layer.draw_circle_array(
                    pos.x as i16,
                    pos.y as i16,
                    (size / 3) as i16,
                    (size as i16) * 3,
                    angle + PI / 2.0,
                    color,
                );
            }
            Tail::Wavy { segment_positions } => {
                let len = segment_positions.len();
                let start = ((len - 2) as f32 * size as f32) as i32;
                for i in (0..=start).rev() {
                    let idx = (i + 1) as usize;
                    if idx >= len {
                        continue;
                    }
                    let vin = segment_positions[idx - 1];
                    let dv = vin - segment_positions[idx];
                    let segment_angle = dv.heading();
                    segment_positions[idx].x = vin.x - segment_angle.cos();
                    segment_positions[idx].y = vin.y - segment_angle.sin();
                    let p = segment_positions[idx];
                    layer.draw_pixel(p.x as i16, p.y as i16, color);
                }
                // Segment 0 follows the body anchor.
                let dv = pos - segment_positions[0];
                let segment_angle = dv.heading();
                segment_positions[0].x = pos.x - segment_angle.cos();
                segment_positions[0].y = pos.y - segment_angle.sin();
                let p = segment_positions[0];
                layer.draw_pixel(p.x as i16, p.y as i16, color);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Fins
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Fin {
    Triangle,
    Ellipse,
    Leg,
    Round,
}

impl Fin {
    pub fn random(rng: &mut Rng) -> Self {
        match rng.range(0, 4) {
            0 => Fin::Triangle,
            1 => Fin::Ellipse,
            2 => Fin::Leg,
            _ => Fin::Round,
        }
    }

    pub fn from_name(name: &str, rng: &mut Rng) -> Self {
        match name {
            "TriangleFin" => Fin::Triangle,
            "EllipseFin" => Fin::Ellipse,
            "LegFin" => Fin::Leg,
            "RoundFin" => Fin::Round,
            _ => Fin::random(rng),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Fin::Triangle => "TriangleFin",
            Fin::Ellipse => "EllipseFin",
            Fin::Leg => "LegFin",
            Fin::Round => "RoundFin",
        }
    }

    fn display(&self, layer: &mut Layer, pos: Vec2, angle: f32, size: u8, color: CRgb) {
        match self {
            Fin::Triangle => {
                let heading = Vec2::from_angle(angle);
                let mut pt1 = heading;
                pt1.set_mag(size as f32);
                pt1 += pos;
                let mut pt2 = heading;
                pt2.set_mag(-(size as f32) / 2.0);
                pt2 += pos;
                let heading3 = heading * 3.0;
                let mut pt3 = Vec2::from_angle(angle + PI / 2.0);
                pt3.set_mag(size as f32 * 2.0);
                pt3 -= heading3;
                pt3 += pos;
                layer.fill_triangle(
                    pt1.x as i16,
                    pt1.y as i16,
                    pt2.x as i16,
                    pt2.y as i16,
                    pt3.x as i16,
                    pt3.y as i16,
                    color,
                );
                let mut pt3 = Vec2::from_angle(angle + PI / 2.0);
                pt3.set_mag(-(size as f32) * 2.0);
                pt3 -= heading3;
                pt3 += pos;
                layer.fill_triangle(
                    pt1.x as i16,
                    pt1.y as i16,
                    pt2.x as i16,
                    pt2.y as i16,
                    pt3.x as i16,
                    pt3.y as i16,
                    color,
                );
            }
            Fin::Ellipse => {
                layer.draw_circle_array(
                    pos.x as i16,
                    pos.y as i16,
                    (size as i16) * 3,
                    (size / 3) as i16,
                    angle,
                    color,
                );
            }
            Fin::Leg => {
                let mut pt = Vec2::from_angle(angle + PI / 2.0);
                pt.set_mag(size as f32 * 2.0);
                pt += pos;
                layer.draw_line(pt.x as i16, pt.y as i16, pos.x as i16, pos.y as i16, color);
                let mut pt = Vec2::from_angle(angle + PI / 2.0);
                pt.set_mag(-(size as f32) * 2.0);
                pt += pos;
                layer.draw_line(pt.x as i16, pt.y as i16, pos.x as i16, pos.y as i16, color);
            }
            Fin::Round => {
                let swapped = CRgb::new(color.b, color.g, color.r);
                let mut pt = Vec2::from_angle(angle + PI / 2.0);
                pt.set_mag(size as f32 * 2.0);
                pt += pos;
                layer.draw_line(pt.x as i16, pt.y as i16, pos.x as i16, pos.y as i16, color);
                layer.fill_circle(pt.x as i16, pt.y as i16, (size / 3) as i16, swapped);
                let mut pt = Vec2::from_angle(angle + PI / 2.0);
                pt.set_mag(-(size as f32) * 2.0);
                pt += pos;
                layer.draw_line(pt.x as i16, pt.y as i16, pos.x as i16, pos.y as i16, color);
                layer.fill_circle(pt.x as i16, pt.y as i16, (size / 3) as i16, swapped);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Body shapes
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum BodyShape {
    Fish {
        segments: Vec<u8>,
        segment_positions: Vec<Vec2>,
        gap_between_segments: f32,
        head: Head,
        tail: Tail,
        fin: Fin,
    },
    Star {
        length: i32,
        rad: i32,
        arms: i32,
        nodes: bool,
        star_angle: f32,
        rotation_speed: f32,
    },
    Turtle {
        length: i32,
        rad: i32,
    },
    Snake {
        segment_positions: Vec<Vec2>,
    },
    Octopus {
        rad: i32,
        tentacle_length: i32,
        tentacle_segments: Vec<Vec<Vec2>>,
    },
}

/// A creature body: shape data plus the shared state (position, size/age,
/// health, palette) that Body carries in C++.
pub struct Body {
    pub shape: BodyShape,
    pub palette: ColorPalette,
    pos: Vec2,
    vel: Vec2,
    angle: f32,
    size: f32,
    health: f32,
}

impl Body {
    pub fn new(shape: BodyShape, palette: ColorPalette) -> Self {
        Self {
            shape,
            palette,
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            angle: 0.0,
            size: 0.5,
            health: 1.0,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self.shape {
            BodyShape::Fish { .. } => "Fish",
            BodyShape::Star { .. } => "Star",
            BodyShape::Turtle { .. } => "Turtle",
            BodyShape::Snake { .. } => "Snake",
            BodyShape::Octopus { .. } => "Octopus",
        }
    }

    pub fn update(&mut self, pos: Vec2, vel: Vec2, angle: f32, age: f32, health: f32) {
        self.pos = pos;
        self.angle = angle;
        self.vel = vel;
        self.size = age;
        self.health = health;
        self.palette
            .adjust_color_by_age_and_health(self.size, self.health);
    }

    pub fn display_egg(&self, layer: &mut Layer) {
        let c = self.palette.colors[0];
        layer.draw_pixel(self.pos.x as i16, self.pos.y as i16, c);
    }

    pub fn display(&mut self, layer: &mut Layer, now_ms: u64) {
        let Body {
            shape,
            palette,
            pos,
            vel,
            angle,
            size,
            ..
        } = self;
        let (pos, vel, angle, size) = (*pos, *vel, *angle, *size);
        match shape {
            BodyShape::Fish {
                segments,
                segment_positions,
                gap_between_segments,
                head,
                tail,
                fin,
            } => {
                let gap = *gap_between_segments;

                let len = segments.len();
                for i in (0..len.saturating_sub(1)).rev() {
                    let vin = segment_positions[i];
                    let color = palette.colors[i.min(palette.colors.len() - 1)];
                    draw_fish_segment(
                        layer,
                        segments,
                        segment_positions,
                        gap,
                        size,
                        fin,
                        tail,
                        i + 1,
                        vin,
                        color,
                        true,
                    );
                }

                // The original call passes the scaled head size into the red
                // channel slot (FishBody.h); colors shift by one. Mirrored
                // deliberately so heads look the same as on the C++ build.
                let head_size_arg = segments[0];
                let head_color = CRgb::new(
                    (size * segments[0] as f32) as u8,
                    palette.colors[0].r,
                    palette.colors[0].g,
                );
                head.display(layer, pos, angle, head_size_arg, head_color);

                let color0 = palette.colors[0];
                draw_fish_segment(
                    layer,
                    segments,
                    segment_positions,
                    gap,
                    size,
                    fin,
                    tail,
                    0,
                    pos,
                    color0,
                    false,
                );
            }
            BodyShape::Star {
                length,
                rad,
                arms,
                nodes,
                star_angle,
                rotation_speed,
            } => {
                let velocity_magnitude = vel.mag().max(0.1);
                *star_angle += velocity_magnitude * *rotation_speed;
                *star_angle = star_angle.rem_euclid(TWO_PI);

                let (length, rad, arms) = (*length, *rad, *arms);
                let node_rad = ((rad as f32 * 0.5 * size) as i32).max(1) as i16;

                if *nodes {
                    for i in 0..arms {
                        let arm_angle = *star_angle + TWO_PI / arms as f32 * i as f32;
                        let mut arm_pt = Vec2::from_angle(arm_angle);
                        arm_pt.set_mag(length as f32 * size);
                        let end_point = pos + arm_pt;
                        let c0 = palette.colors[0];
                        layer.draw_line(
                            pos.x as i16,
                            pos.y as i16,
                            end_point.x as i16,
                            end_point.y as i16,
                            c0,
                        );
                        let c1 = palette.colors[1];
                        layer.fill_circle(end_point.x as i16, end_point.y as i16, node_rad, c1);
                    }
                    let c2 = palette.colors[2];
                    layer.fill_circle(pos.x as i16, pos.y as i16, node_rad, c2);
                } else {
                    for i in 0..arms {
                        let arm_angle = *star_angle + TWO_PI / arms as f32 * i as f32;
                        let mut pt1 = Vec2::from_angle(arm_angle);
                        let mut pt2 = Vec2::from_angle(arm_angle - PI / arms as f32);
                        let mut pt3 = Vec2::from_angle(arm_angle + PI / arms as f32);
                        pt1.set_mag(length as f32 * size);
                        pt2.set_mag(rad as f32 * size);
                        pt3.set_mag(rad as f32 * size);
                        let c0 = palette.colors[0];
                        layer.fill_triangle(
                            (pos.x + pt1.x) as i16,
                            (pos.y + pt1.y) as i16,
                            (pos.x + pt2.x) as i16,
                            (pos.y + pt2.y) as i16,
                            (pos.x + pt3.x) as i16,
                            (pos.y + pt3.y) as i16,
                            c0,
                        );
                    }
                    let c2 = palette.colors[2];
                    layer.fill_circle(pos.x as i16, pos.y as i16, node_rad, c2);
                }
            }
            BodyShape::Turtle { length, rad } => {
                let rad2draw = ((*rad as f32 * size) as i32).max(1) as i16;
                let length2draw =
                    ((*length as f32 * (1.0 + vel.mag() / 20.0) * size) as i32).max(1) as i16;
                let c = palette.colors[0];
                layer.draw_circle_array(
                    pos.x as i16,
                    pos.y as i16,
                    rad2draw,
                    length2draw,
                    angle + PI / 2.0,
                    c,
                );
            }
            BodyShape::Snake { segment_positions } => {
                let len = segment_positions.len();
                let start = ((len - 2) as f32 * size) as i32;
                for i in (0..=start).rev() {
                    let idx = (i + 1) as usize;
                    if idx >= len {
                        continue;
                    }
                    let color = palette.colors[idx - 1];
                    let vin = segment_positions[idx - 1];
                    chain_pixel(layer, segment_positions, idx, vin, color);
                }
                let color0 = palette.colors[0];
                chain_pixel(layer, segment_positions, 0, pos, color0);
            }
            BodyShape::Octopus {
                rad,
                tentacle_length,
                tentacle_segments,
            } => {
                let rad2draw = ((*rad as f32 * size / 4.0) as i32).max(1) as i16;
                let length2draw = ((*rad as f32 * size / 2.0) as i32).max(1) as i16;
                let c0 = palette.colors[0];
                layer.draw_circle_array(
                    pos.x as i16,
                    pos.y as i16,
                    rad2draw,
                    length2draw,
                    angle,
                    c0,
                );

                let num_tentacles = tentacle_segments.len();
                for i in 0..num_tentacles {
                    draw_tentacle(
                        layer,
                        tentacle_segments,
                        i,
                        pos,
                        vel,
                        angle,
                        *rad,
                        *tentacle_length,
                        size,
                        &palette.colors,
                        now_ms,
                    );
                }
            }
        }
    }
}

/// Single chained snake-style segment: follow the leader one pixel behind.
fn chain_pixel(layer: &mut Layer, positions: &mut [Vec2], idx: usize, vin: Vec2, color: CRgb) {
    let dv = vin - positions[idx];
    let segment_angle = dv.heading();
    positions[idx].x = vin.x - segment_angle.cos();
    positions[idx].y = vin.y - segment_angle.sin();
    let p = positions[idx];
    layer.draw_pixel(p.x as i16, p.y as i16, color);
}

#[allow(clippy::too_many_arguments)]
fn draw_fish_segment(
    layer: &mut Layer,
    segments: &[u8],
    segment_positions: &mut [Vec2],
    gap: f32,
    size: f32,
    fin: &Fin,
    tail: &mut Tail,
    i: usize,
    vin: Vec2,
    color: CRgb,
    draw_extras: bool,
) {
    let len = segments.len();
    let dv = vin - segment_positions[i];
    let segment_angle = dv.heading();

    let current_segment_size = segments[i] as f32 * size;

    if i == 0 {
        segment_positions[0].x = vin.x - segment_angle.cos() * current_segment_size;
        segment_positions[0].y = vin.y - segment_angle.sin() * current_segment_size;
    } else {
        let max_segment_size = segments[i - 1].max(segments[i]) as f32 * size * gap;
        segment_positions[i].x = vin.x - segment_angle.cos() * max_segment_size;
        segment_positions[i].y = vin.y - segment_angle.sin() * max_segment_size;
    }

    if draw_extras && (i == 1 || i == 3) {
        fin.display(
            layer,
            segment_positions[i],
            segment_angle,
            current_segment_size as u8,
            color,
        );
    }

    if draw_extras && i == len - 1 {
        tail.display(
            layer,
            segment_positions[i],
            segment_angle,
            (current_segment_size * 2.0) as u8,
            color,
        );
    } else {
        let p = segment_positions[i];
        layer.fill_circle(p.x as i16, p.y as i16, current_segment_size as i16, color);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_tentacle(
    layer: &mut Layer,
    tentacle_segments: &mut [Vec<Vec2>],
    i: usize,
    body_pos: Vec2,
    vel: Vec2,
    angle: f32,
    rad: i32,
    tentacle_length: i32,
    size: f32,
    colors: &[CRgb],
    now_ms: u64,
) {
    let num_tentacles = tentacle_segments.len();
    let spread_angle = PI * 2.0;

    let back_offset = Vec2::new(angle.cos(), angle.sin()) * (rad as f32 * size / 6.0);
    let back_center = body_pos - back_offset;

    let tentacle_angle = angle
        + PI
        + (i as f32 - (num_tentacles - 1) as f32 / 2.0)
            * (spread_angle / (num_tentacles - 1) as f32);
    let tentacle_start = back_center
        + Vec2::new(tentacle_angle.cos(), tentacle_angle.sin()) * (rad as f32 * size / 6.0);

    let mut current = tentacle_start;
    let segment_length = (tentacle_length as f32 * size) / consts::OCTOPUS_TENTACLE_SEGMENTS as f32;

    let time = now_ms as f32 / 1000.0;
    let velocity_factor = (vel.mag() / 2.0).min(1.0);

    for j in 0..consts::OCTOPUS_TENTACLE_SEGMENTS {
        let dv = current - tentacle_segments[i][j];
        let mut segment_angle = dv.heading();

        let movement_angle =
            (time * 2.0 + i as f32 * 0.5 + j as f32 * 0.3).sin() * 0.2 * velocity_factor;
        segment_angle += movement_angle;

        tentacle_segments[i][j].x = current.x - segment_angle.cos() * segment_length;
        tentacle_segments[i][j].y = current.y - segment_angle.sin() * segment_length;

        let segment_color = colors[(j + 1).min(colors.len() - 1)];
        let next = tentacle_segments[i][j];
        layer.draw_line(
            current.x as i16,
            current.y as i16,
            next.x as i16,
            next.y as i16,
            segment_color,
        );

        current = next;
    }
}

// ---------------------------------------------------------------------------
// Body construction (BodyFactory port)
// ---------------------------------------------------------------------------

pub fn create_body(kind: &str, rng: &mut Rng) -> Body {
    match kind {
        "Star" => {
            let shape = BodyShape::Star {
                length: rng.range(consts::STAR_LENGTH.0, consts::STAR_LENGTH.1),
                rad: rng.range(consts::STAR_RAD.0, consts::STAR_RAD.1),
                arms: rng.range(consts::STAR_NUM_ARMS.0, consts::STAR_NUM_ARMS.1),
                nodes: rng.range(0, 2) == 1,
                star_angle: rng.rand_max(6) as f32,
                rotation_speed: rng
                    .range(consts::STAR_ROTATION_SPEED.0, consts::STAR_ROTATION_SPEED.1)
                    as f32
                    / 1000.0,
            };
            Body::new(shape, ColorPalette::new(3, false, rng))
        }
        "Turtle" => {
            let shape = BodyShape::Turtle {
                length: rng.range(consts::TURTLE_LENGTH.0, consts::TURTLE_LENGTH.1),
                rad: rng.range(consts::TURTLE_WIDTH.0, consts::TURTLE_WIDTH.1),
            };
            let palette_size = match &shape {
                BodyShape::Turtle { rad, .. } => *rad as usize,
                _ => 2,
            };
            Body::new(shape, ColorPalette::new(palette_size, false, rng))
        }
        "Snake" => {
            let num_segments =
                rng.range(consts::SNAKE_NUM_SEGMENTS.0, consts::SNAKE_NUM_SEGMENTS.1) as usize;
            let shape = BodyShape::Snake {
                segment_positions: vec![Vec2::ZERO; num_segments],
            };
            Body::new(shape, ColorPalette::new(num_segments, false, rng))
        }
        "Octopus" => {
            let rad = rng.range(consts::OCTOPUS_SIZE.0, consts::OCTOPUS_SIZE.1);
            let num_tentacles = rng.range(
                consts::OCTOPUS_MIN_TENTACLES,
                consts::OCTOPUS_MAX_TENTACLES + 1,
            ) as usize;
            let tentacle_length = rng.range(
                consts::OCTOPUS_TENTACLE_LENGTH.0,
                consts::OCTOPUS_TENTACLE_LENGTH.1,
            );
            let shape = BodyShape::Octopus {
                rad,
                tentacle_length,
                tentacle_segments: vec![
                    vec![Vec2::ZERO; consts::OCTOPUS_TENTACLE_SEGMENTS];
                    num_tentacles
                ],
            };
            Body::new(
                shape,
                ColorPalette::new(consts::OCTOPUS_TENTACLE_SEGMENTS + 1, false, rng),
            )
        }
        _ => create_fish_body(rng),
    }
}

pub fn create_fish_body(rng: &mut Rng) -> Body {
    let head = Head::random(rng);
    let tail = Tail::random(rng);
    let fin = Fin::random(rng);
    create_fish_body_with_parts(rng, head, tail, fin)
}

pub fn create_fish_body_with_parts(rng: &mut Rng, head: Head, tail: Tail, fin: Fin) -> Body {
    let num_segments = rng
        .range(consts::FISH_NUM_SEGMENTS.0, consts::FISH_NUM_SEGMENTS.1)
        .max(2) as usize;
    let base_size = consts::FISH_MIN_SEGMENT_SIZE;
    let max_add_size = rng.range(
        consts::FISH_MAX_SEGMENT_ADD.0,
        consts::FISH_MAX_SEGMENT_ADD.1,
    ) as f32;
    let gap = rng.range(
        consts::FISH_GAP_BETWEEN_SEGMENTS.0,
        consts::FISH_GAP_BETWEEN_SEGMENTS.1,
    ) as f32
        / 100.0;

    let mut segments = Vec::with_capacity(num_segments);
    for i in 0..num_segments {
        let phase = (PI * i as f32) / (num_segments - 1) as f32;
        let segment_size = (base_size + phase.sin() * max_add_size) as u8;
        segments.push(segment_size.max(1));
    }

    let shape = BodyShape::Fish {
        segment_positions: vec![Vec2::ZERO; num_segments],
        segments,
        gap_between_segments: gap,
        head,
        tail,
        fin,
    };
    Body::new(shape, ColorPalette::new(num_segments, true, rng))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> Rng {
        Rng::new(0xC0FFEE)
    }

    #[test]
    fn palette_sizes_and_stripes() {
        let p = ColorPalette::new(6, true, &mut rng());
        assert_eq!(p.colors.len(), 6);
        assert_eq!(p.colors_hsv.len(), 6);
    }

    #[test]
    fn palette_age_and_health_adjustment() {
        let mut p = ColorPalette::new(3, false, &mut rng());
        p.adjust_color_by_age_and_health(1.0, 1.0);
        for hsv in &p.colors_hsv {
            assert_eq!(hsv.s, 115);
            // age 1.0 -> ageFactor = 1 - (0.3/0.3)*0.5 = 0.5
            assert_eq!(hsv.v, 127);
        }
        p.adjust_color_by_age_and_health(0.4, 0.5);
        for hsv in &p.colors_hsv {
            assert_eq!(hsv.s, 57);
            assert_eq!(hsv.v, 255);
        }
    }

    #[test]
    fn fish_body_segment_distribution() {
        let body = create_fish_body(&mut rng());
        match &body.shape {
            BodyShape::Fish { segments, .. } => {
                assert!(segments.len() >= 2);
                assert!(segments.iter().all(|s| *s >= 1));
                // Sine distribution: middle segments are the largest.
                let mid = segments.len() / 2;
                assert!(segments[mid] >= segments[0]);
            }
            _ => panic!("expected fish"),
        }
        assert_eq!(body.kind_name(), "Fish");
    }

    #[test]
    fn create_each_body_kind() {
        let mut r = rng();
        for kind in ["Fish", "Star", "Turtle", "Snake", "Octopus"] {
            let body = create_body(kind, &mut r);
            assert_eq!(body.kind_name(), kind);
        }
        // Unknown kinds fall back to a fish body, like the C++ code.
        let body = create_body("Blob", &mut r);
        assert_eq!(body.kind_name(), "Fish");
    }

    #[test]
    fn bodies_render_without_panicking() {
        let mut layer = Layer::new(80, 106);
        let mut r = rng();
        for kind in ["Fish", "Star", "Turtle", "Snake", "Octopus"] {
            let mut body = create_body(kind, &mut r);
            for frame in 0..4 {
                layer.clear();
                body.update(
                    Vec2::new(40.0 + frame as f32, 50.0),
                    Vec2::new(0.5, 0.1),
                    0.3,
                    0.8,
                    1.0,
                );
                body.display(&mut layer, frame * 33);
            }
        }
    }

    #[test]
    fn fish_body_actually_paints() {
        let mut layer = Layer::new(80, 106);
        let mut r = rng();
        let mut body = create_body("Fish", &mut r);
        for _ in 0..3 {
            body.update(Vec2::new(40.0, 50.0), Vec2::new(0.5, 0.0), 0.0, 0.8, 1.0);
            body.display(&mut layer, 0);
        }
        let mut lit = 0;
        for y in 0..106 {
            for x in 0..80 {
                if layer.get(x, y) != CRgb::BLACK {
                    lit += 1;
                }
            }
        }
        assert!(lit > 3, "fish should paint pixels, got {lit}");
    }

    #[test]
    fn egg_is_single_pixel() {
        let mut layer = Layer::new(80, 106);
        let mut r = rng();
        let mut body = create_body("Fish", &mut r);
        body.update(Vec2::new(20.0, 20.0), Vec2::ZERO, 0.0, 0.05, 1.0);
        body.display_egg(&mut layer);
        assert_ne!(layer.get(20, 20), CRgb::BLACK);
    }
}
