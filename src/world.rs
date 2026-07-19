//! Aquarium world pieces: animated water, swaying plants, drifting food and
//! background boids, ported from Water.h / Plants.h / Food.h / BoidManager +
//! Boid.

use crate::color::{color_from_palette, CRgb, WATER_PALETTE};
use crate::consts;
use crate::layer::Layer;
use crate::math8::{arduino_map, constrain, map_f32};
use crate::noise::inoise8;
use crate::rng::Rng;
use crate::vec2::Vec2;
use core::f32::consts::PI;

const TWO_PI: f32 = 2.0 * PI;

// ---------------------------------------------------------------------------
// Water
// ---------------------------------------------------------------------------

pub struct Water {
    update_buffer: Vec<CRgb>,
    current_row: usize,
    simplex_color: CRgb,
    width: usize,
    height: usize,
}

impl Water {
    const ROWS_PER_UPDATE: usize = 8;
    const SCALE: u16 = 20;
    const SIMPLEX_SPEED: f32 = 0.002;

    pub fn new(width: usize, height: usize) -> Self {
        Self {
            update_buffer: vec![CRgb::BLACK; width * height],
            current_row: 0,
            simplex_color: CRgb::new(0, 100, 100),
            width,
            height,
        }
    }

    /// One update step: compute a slice of rows into the staging buffer, and
    /// once the whole frame is staged, blit it into the background layer
    /// (matching the C++ interleaved update/blit cycle).
    pub fn update(&mut self, temperature: i64, now_ms: u64, background: &mut Layer) {
        if self.current_row >= self.height {
            for row in 0..self.height {
                for col in 0..self.width {
                    let color = self.update_buffer[row * self.width + col];
                    background.draw_pixel(col as i16, row as i16, color);
                }
            }
            self.current_row = 0;
            return;
        }

        let limit_temperature = constrain(temperature, 10, 35);
        let mut color_index = arduino_map(limit_temperature, 10, 35, 0, 255) as u8;
        if limit_temperature < 34 {
            color_index = color_index.min(235);
        }
        self.simplex_color = color_from_palette(&WATER_PALETTE, color_index);

        // C++ passes (millis() * simplexSpeed) as a float into a uint16_t
        // parameter; replicate the truncation and 16-bit wrap.
        let z = ((now_ms as f32 * Self::SIMPLEX_SPEED) as u32 % 65_536) as u16;

        let end_row = (self.current_row + Self::ROWS_PER_UPDATE).min(self.height);
        for row in self.current_row..end_row {
            for col in 0..self.width {
                let mut noise_factor =
                    inoise8(col as u16 * Self::SCALE, row as u16 * Self::SCALE, z);
                if noise_factor < 96 {
                    noise_factor = 96;
                }
                self.update_buffer[row * self.width + col] =
                    self.simplex_color.scaled(noise_factor);
            }
        }

        self.current_row = end_row;
    }
}

// ---------------------------------------------------------------------------
// Plants
// ---------------------------------------------------------------------------

struct Branch {
    nodes: Vec<Vec2>,
}

pub struct Plants {
    pos: Vec2,
    branches: Vec<Branch>,
    phase_offsets: Vec<f32>,
}

impl Plants {
    const BRANCH_SIZE_BASE: f32 = 3.0;

    pub fn new(x: u8, y: u8, rng: &mut Rng) -> Self {
        let pos = Vec2::new(x as f32, y as f32);
        let num_branches = rng.range(10, 16);

        let mut branches = Vec::with_capacity(num_branches as usize);
        let mut phase_offsets = Vec::with_capacity(num_branches as usize);
        for _ in 0..num_branches {
            let mut branch_start = Vec2::from_angle(rng.range(3141, 6283) as f32 / 1000.0);
            branch_start *= Self::BRANCH_SIZE_BASE;
            let num_nodes = rng.range(4, 9);
            let mut nodes = Vec::with_capacity(num_nodes as usize);
            nodes.push(branch_start);
            for j in 1..num_nodes {
                let node = Self::build_node(nodes[j as usize - 1]);
                nodes.push(node);
            }
            branches.push(Branch { nodes });
            phase_offsets.push(rng.range(0, 500) as f32 / 100.0);
        }

        Self {
            pos,
            branches,
            phase_offsets,
        }
    }

    fn build_node(start_vec: Vec2) -> Vec2 {
        let theta = start_vec.heading();
        let target = 3.0 * PI / 2.0;
        let mut diff = target - theta;
        if diff > PI {
            diff -= TWO_PI;
        } else if diff < -PI {
            diff += TWO_PI;
        }
        let increment = diff * 0.5;

        let mut end_pos = Vec2::from_angle(theta + increment);
        end_pos *= Self::BRANCH_SIZE_BASE;
        end_pos += start_vec;
        end_pos
    }

    pub fn update(&self, humidity: u8, now_ms: u64, foreground: &mut Layer) {
        let current_time = now_ms as f32;
        let size_factor = arduino_map(humidity as i64, 0, 100, 0, 250) as f32 / 100.0;

        for (i, branch) in self.branches.iter().enumerate() {
            let first = branch.nodes[0];
            foreground.draw_line(
                (first.x * size_factor + self.pos.x) as i16,
                (first.y * size_factor + self.pos.y) as i16,
                self.pos.x as i16,
                self.pos.y as i16,
                CRgb::BLACK,
            );

            for j in 1..branch.nodes.len() {
                let node = branch.nodes[j];
                let prev_node = branch.nodes[j - 1];

                let sway_amplitude = 0.8 * j as f32;
                let sway = (current_time / 10_000.0 + self.phase_offsets[i]).sin() * sway_amplitude;

                foreground.draw_line(
                    (prev_node.x * size_factor + sway + self.pos.x) as i16,
                    (prev_node.y * size_factor + self.pos.y) as i16,
                    (node.x * size_factor + sway + self.pos.x) as i16,
                    (node.y * size_factor + self.pos.y) as i16,
                    CRgb::new(0, 32, 8),
                );

                // Flower glow at the branch tip.
                if j == branch.nodes.len() - 1 {
                    let glow_factor = (current_time / 10_000.0 + self.phase_offsets[i]).sin() - 0.8;
                    if glow_factor > 0.0 {
                        let glow_intensity = (glow_factor * 1000.0) as u8;
                        foreground.fill_circle(
                            (node.x * size_factor + sway + self.pos.x) as i16,
                            (node.y * size_factor + self.pos.y) as i16,
                            1,
                            CRgb::new(glow_intensity, glow_intensity, 0),
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Food
// ---------------------------------------------------------------------------

pub struct Food {
    position: Vec2,
    eaten: bool,
    off_screen: bool,
}

impl Food {
    const FALL_SPEED: f32 = 0.3;

    pub fn new(x: f32) -> Self {
        Self {
            position: Vec2::new(x, 0.0),
            eaten: false,
            off_screen: false,
        }
    }

    pub fn update(&mut self, y_res: u8) {
        self.position.y += Self::FALL_SPEED;
        if self.position.y >= y_res as f32 {
            self.off_screen = true;
        }
    }

    pub fn display(&self, foreground: &mut Layer) {
        if !self.eaten {
            foreground.draw_pixel(
                self.position.x as i16,
                self.position.y as i16,
                CRgb::new(255, 255, 0),
            );
        }
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn is_off_screen(&self) -> bool {
        self.off_screen
    }

    pub fn eat(&mut self) {
        self.eaten = true;
    }

    pub fn is_eaten(&self) -> bool {
        self.eaten
    }
}

// ---------------------------------------------------------------------------
// Boids
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Boid {
    location: Vec2,
    velocity: Vec2,
    acceleration: Vec2,
    max_force: f32,
    max_speed: f32,
    desired_separation: f32,
    neighbor_dist: f32,
    limits: Vec2,
}

impl Boid {
    fn new(x: f32, y: f32, limits: Vec2, rng: &mut Rng) -> Self {
        let randomf = |rng: &mut Rng| map_f32(rng.range(0, 255) as f32, 0.0, 255.0, -0.5, 0.5);
        Self {
            location: Vec2::new(x, y),
            velocity: Vec2::new(randomf(rng), randomf(rng)),
            acceleration: Vec2::ZERO,
            max_force: 0.05,
            max_speed: 1.5,
            desired_separation: 4.0,
            neighbor_dist: 8.0,
            limits,
        }
    }

    fn apply_force(&mut self, force: Vec2) {
        self.acceleration += force;
    }

    fn flock(&mut self, boids: &[Boid]) {
        let mut sep = self.separate(boids);
        let ali = self.align(boids);
        let coh = self.cohesion(boids);

        sep *= 1.5;

        self.apply_force(sep);
        self.apply_force(ali);
        self.apply_force(coh);
    }

    fn separate(&self, boids: &[Boid]) -> Vec2 {
        let mut steer = Vec2::ZERO;
        let mut count = 0;
        for other in boids {
            let d = self.location.dist(other.location);
            if d > 0.0 && d < self.desired_separation {
                let mut diff = self.location - other.location;
                diff.normalize();
                diff /= d;
                steer += diff;
                count += 1;
            }
        }
        if count > 0 {
            steer /= count as f32;
        }
        if steer.mag() > 0.0 {
            steer.normalize();
            steer *= self.max_speed;
            steer -= self.velocity;
            steer.limit(self.max_force);
        }
        steer
    }

    fn align(&self, boids: &[Boid]) -> Vec2 {
        let mut sum = Vec2::ZERO;
        let mut count = 0;
        for other in boids {
            let d = self.location.dist(other.location);
            if d > 0.0 && d < self.neighbor_dist {
                sum += other.velocity;
                count += 1;
            }
        }
        if count > 0 {
            sum /= count as f32;
            sum.normalize();
            sum *= self.max_speed;
            let mut steer = sum - self.velocity;
            steer.limit(self.max_force);
            return steer;
        }
        Vec2::ZERO
    }

    fn cohesion(&self, boids: &[Boid]) -> Vec2 {
        let mut sum = Vec2::ZERO;
        let mut count = 0;
        for other in boids {
            let d = self.location.dist(other.location);
            if d > 0.0 && d < self.neighbor_dist {
                sum += other.location;
                count += 1;
            }
        }
        if count > 0 {
            sum /= count as f32;
            return self.seek(sum);
        }
        Vec2::ZERO
    }

    fn seek(&self, target: Vec2) -> Vec2 {
        let mut desired = target - self.location;
        desired.normalize();
        desired *= self.max_speed;
        let mut steer = desired - self.velocity;
        steer.limit(self.max_force);
        steer
    }

    fn update(&mut self, speed_multiplier: f32) {
        self.velocity += self.acceleration;
        self.velocity.limit(self.max_speed * speed_multiplier);
        self.location += self.velocity;
        self.acceleration = Vec2::ZERO;
    }

    fn wrap_around_borders(&mut self) {
        if self.location.x < 0.0 {
            self.location.x = self.limits.x - 1.0;
        }
        if self.location.y < 0.0 {
            self.location.y = self.limits.y - 1.0;
        }
        if self.location.x >= self.limits.x {
            self.location.x = 0.0;
        }
        if self.location.y >= self.limits.y {
            self.location.y = 0.0;
        }
    }

    fn avoid_borders(&mut self) {
        let mut desired = self.velocity;

        if self.location.x < 0.0 {
            desired.x = self.max_speed;
        }
        if self.location.x > self.limits.x {
            desired.x = -self.max_speed;
        }
        if self.location.y < 0.0 {
            desired.y = self.max_speed;
        }
        if self.location.y > self.limits.y {
            desired.y = -self.max_speed;
        }

        if desired != self.velocity {
            let mut steer = desired - self.velocity;
            steer.limit(self.max_force);
            self.apply_force(steer);
        }
    }
}

pub struct BoidManager {
    groups: Vec<Vec<Boid>>,
}

impl Default for BoidManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BoidManager {
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    pub fn initialize(&mut self, x_res: u8, y_res: u8, rng: &mut Rng) {
        let limits = Vec2::new(x_res as f32, y_res as f32);
        self.groups = (0..consts::BOID_GROUPS)
            .map(|_| {
                let num_boids = rng.range(consts::NUM_BOIDS.0, consts::NUM_BOIDS.1);
                (0..num_boids)
                    .map(|_| {
                        let mut boid = Boid::new(
                            rng.range(0, x_res as i32) as f32,
                            rng.range(0, y_res as i32) as f32,
                            limits,
                            rng,
                        );
                        boid.max_speed =
                            rng.range(consts::BOID_MAX_SPEED.0, consts::BOID_MAX_SPEED.1) as f32
                                / 10.0;
                        boid.max_force =
                            rng.range(consts::BOID_MAX_FORCE.0, consts::BOID_MAX_FORCE.1) as f32
                                / 10.0;
                        boid
                    })
                    .collect()
            })
            .collect();
    }

    pub fn update(&mut self, co2: i64) {
        let mut speed_multiplier =
            arduino_map(co2, consts::CO2_BAD, consts::CO2_REALBAD, 100, 0) as f32;
        speed_multiplier = constrain(speed_multiplier, 0.0, 100.0);
        speed_multiplier /= 100.0;

        for group in &mut self.groups {
            // Snapshot so every boid flocks against the same frame state.
            let snapshot = group.clone();
            for boid in group.iter_mut() {
                boid.flock(&snapshot);
                boid.update(speed_multiplier);
                boid.wrap_around_borders();
                boid.avoid_borders();
            }
        }
    }

    pub fn render(&self, foreground: &mut Layer) {
        for group in &self.groups {
            for boid in group {
                let angle = boid.velocity.y.atan2(boid.velocity.x);
                let x2 = boid.location.x + angle.cos();
                let y2 = boid.location.y + angle.sin();
                foreground.draw_line(
                    boid.location.x as i16,
                    boid.location.y as i16,
                    x2 as i16,
                    y2 as i16,
                    CRgb::new(50, 200, 100),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> Rng {
        Rng::new(0xB01D)
    }

    #[test]
    fn water_fills_background_after_full_cycle() {
        let mut water = Water::new(80, 106);
        let mut bg = Layer::new(80, 106);
        // 106 rows / 8 per update -> 14 updates to stage, 15th blits.
        for i in 0..15 {
            water.update(22, i * 33, &mut bg);
        }
        let c = bg.get(40, 50);
        // 22C water is a blue-green; the red channel should be absent.
        assert_eq!(c.r, 0);
        assert!(c.b > 0 || c.g > 0);
    }

    #[test]
    fn water_temperature_changes_palette() {
        let mut cold = Water::new(80, 106);
        let mut hot = Water::new(80, 106);
        let mut bg_cold = Layer::new(80, 106);
        let mut bg_hot = Layer::new(80, 106);
        // 34C is the hottest temperature before FastLED's palette index wraps
        // around at index 255 (mirrored upstream behavior).
        for i in 0..15 {
            cold.update(10, i * 33, &mut bg_cold);
            hot.update(34, i * 33, &mut bg_hot);
        }
        let c = bg_cold.get(40, 50);
        let h = bg_hot.get(40, 50);
        assert!(c.b > c.r, "cold water should be blue: {c:?}");
        assert!(h.r > h.b, "hot water should be red: {h:?}");
    }

    #[test]
    fn water_stays_above_black_floor() {
        let mut water = Water::new(80, 106);
        let mut bg = Layer::new(80, 106);
        for i in 0..15 {
            water.update(22, i * 33, &mut bg);
        }
        // noiseFactor floor of 96 keeps pixels from going near-black.
        let c = bg.get(40, 50);
        assert!(c.g > 20 || c.b > 20, "water too dark: {c:?}");
    }

    #[test]
    fn plants_draw_upward_lines() {
        let plants = Plants::new(40, 113, &mut rng());
        let mut fg = Layer::new(80, 106);
        for i in 0..10 {
            fg.clear();
            plants.update(50, i * 100, &mut fg);
        }
        let mut lit = 0;
        for y in 0..106 {
            for x in 0..80 {
                if fg.get(x, y) != CRgb::BLACK {
                    lit += 1;
                }
            }
        }
        assert!(lit > 10, "plants should paint branches, got {lit}");
    }

    #[test]
    fn food_falls_and_goes_off_screen() {
        let mut food = Food::new(40.0);
        for _ in 0..400 {
            food.update(106);
        }
        assert!(food.is_off_screen());
    }

    #[test]
    fn boids_stay_within_limits() {
        let mut manager = BoidManager::new();
        manager.initialize(80, 106, &mut rng());
        let mut fg = Layer::new(80, 106);
        for _ in 0..500 {
            manager.update(420);
            fg.clear();
            manager.render(&mut fg);
        }
        for group in &manager.groups {
            for boid in group {
                assert!(
                    boid.location.x >= 0.0 && boid.location.x < 80.0,
                    "x={}",
                    boid.location.x
                );
                assert!(
                    boid.location.y >= 0.0 && boid.location.y < 106.0,
                    "y={}",
                    boid.location.y
                );
            }
        }
    }

    #[test]
    fn boids_move() {
        let mut manager = BoidManager::new();
        manager.initialize(80, 106, &mut rng());
        let start: Vec<Vec2> = manager.groups[0].iter().map(|b| b.location).collect();
        for _ in 0..50 {
            manager.update(420);
        }
        let moved = manager.groups[0]
            .iter()
            .zip(start.iter())
            .any(|(b, s)| b.location.dist(*s) > 0.5);
        assert!(moved, "boids should move over 50 updates");
    }
}
