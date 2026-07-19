//! Creature motion physics, ported from lib/Aquarium/Motion/*.
//!
//! Positions and velocities live in physics space (logical pixels *
//! PHYSICS_SCALE); creatures divide by the scale to render.

use crate::consts;
use crate::math8::arduino_map;
use crate::noise::OpenSimplex2S;
use crate::rng::Rng;
use crate::vec2::Vec2;
use core::f32::consts::PI;

const TWO_PI: f32 = 2.0 * PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionKind {
    Fish,
    Star,
    Turtle,
    Snake,
    Octopus,
}

impl MotionKind {
    pub fn name(self) -> &'static str {
        match self {
            MotionKind::Fish => "Fish",
            MotionKind::Star => "Star",
            MotionKind::Turtle => "Turtle",
            MotionKind::Snake => "Snake",
            MotionKind::Octopus => "Octopus",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "Fish" => Some(MotionKind::Fish),
            "Star" => Some(MotionKind::Star),
            "Turtle" => Some(MotionKind::Turtle),
            "Snake" => Some(MotionKind::Snake),
            "Octopus" => Some(MotionKind::Octopus),
            _ => None,
        }
    }

    fn params(self) -> (i32, i32, f32, i32, f32, f32, f32) {
        // (max_speed, min_speed, max_force, sin_amplitude, sin_frequency,
        //  noise_amplitude, noise_frequency)
        match self {
            MotionKind::Fish => (
                consts::FISH_MAX_SPEED,
                consts::FISH_MIN_SPEED,
                consts::FISH_MAX_FORCE,
                consts::FISH_SIN_AMPLITUDE,
                consts::FISH_SIN_FREQUENCY,
                consts::FISH_NOISE_AMPLITUDE,
                consts::FISH_NOISE_FREQUENCY,
            ),
            MotionKind::Star => (
                consts::STAR_MAX_SPEED,
                consts::STAR_MIN_SPEED,
                consts::STAR_MAX_FORCE,
                consts::STAR_SIN_AMPLITUDE,
                consts::STAR_SIN_FREQUENCY,
                consts::STAR_NOISE_AMPLITUDE,
                consts::STAR_NOISE_FREQUENCY,
            ),
            MotionKind::Turtle => (
                consts::TURTLE_MAX_SPEED,
                consts::TURTLE_MIN_SPEED,
                consts::TURTLE_MAX_FORCE,
                consts::TURTLE_SIN_AMPLITUDE,
                consts::TURTLE_SIN_FREQUENCY,
                consts::TURTLE_NOISE_AMPLITUDE,
                consts::TURTLE_NOISE_FREQUENCY,
            ),
            MotionKind::Snake => (
                consts::SNAKE_MAX_SPEED,
                consts::SNAKE_MIN_SPEED,
                consts::SNAKE_MAX_FORCE,
                consts::SNAKE_SIN_AMPLITUDE,
                consts::SNAKE_SIN_FREQUENCY,
                consts::SNAKE_NOISE_AMPLITUDE,
                consts::SNAKE_NOISE_FREQUENCY,
            ),
            MotionKind::Octopus => (
                consts::OCTOPUS_MAX_SPEED,
                consts::OCTOPUS_MIN_SPEED,
                consts::OCTOPUS_MAX_FORCE,
                consts::OCTOPUS_SIN_AMPLITUDE,
                consts::OCTOPUS_SIN_FREQUENCY,
                consts::OCTOPUS_NOISE_AMPLITUDE,
                consts::OCTOPUS_NOISE_FREQUENCY,
            ),
        }
    }
}

pub struct Motion {
    kind: MotionKind,
    pos: Vec2,
    vel: Vec2,
    acc: Vec2,
    angle: f32,
    max_speed: i32,
    min_speed: i32,
    #[allow(dead_code)] // kept for parity with the C++ member set
    max_force: f32,
    sin_amplitude: i32,
    sin_frequency: f32,
    noise_amplitude: f32,
    noise: OpenSimplex2S,
    angle_offset: f32,
    out_of_boundary: bool,
    following_food: bool,
    food_direction: Vec2,
    x_resolution: f32,
    y_resolution: f32,
}

impl Motion {
    /// pos/x_resolution/y_resolution are already in physics scale.
    pub fn new(
        kind: MotionKind,
        pos: Vec2,
        x_resolution: f32,
        y_resolution: f32,
        rng: &mut Rng,
    ) -> Self {
        let (
            max_speed,
            min_speed,
            max_force,
            sin_amplitude,
            sin_frequency,
            noise_amplitude,
            noise_frequency,
        ) = kind.params();

        let mut vel;
        loop {
            vel = Vec2::new(rng.range(-10, 10) as f32, rng.range(-10, 10) as f32);
            if vel.mag() >= 5.0 {
                break;
            }
        }

        Self {
            kind,
            pos,
            vel,
            acc: Vec2::ZERO,
            angle: 0.0,
            max_speed,
            min_speed,
            max_force,
            sin_amplitude,
            sin_frequency,
            noise_amplitude,
            noise: OpenSimplex2S::new(noise_frequency),
            angle_offset: rng.rand_max(6) as f32,
            out_of_boundary: false,
            following_food: false,
            food_direction: Vec2::ZERO,
            x_resolution,
            y_resolution,
        }
    }

    pub fn kind(&self) -> MotionKind {
        self.kind
    }

    pub fn position(&self) -> Vec2 {
        self.pos
    }

    pub fn velocity(&self) -> Vec2 {
        self.vel
    }

    pub fn angle(&self) -> f32 {
        self.angle
    }

    pub fn follow_food(&mut self, food_pos: Vec2) {
        self.food_direction = food_pos;
        self.following_food = true;
    }

    pub fn update(&mut self, age: f32, co2: i64, stay_inside: bool, now_ms: u64) {
        if age < consts::AGE_EGG {
            self.vel = Vec2::ZERO;
            return;
        }

        if stay_inside || co2 > consts::CO2_BAD {
            self.boundary_check(consts::BOUNDARY_FORCE * 10.0);
        } else {
            self.boundary_check(consts::BOUNDARY_FORCE);
        }

        if !self.following_food && !self.out_of_boundary {
            self.do_motion(now_ms);
        } else if self.following_food {
            let mut food_force = self.food_direction - self.pos;
            food_force.set_mag(consts::FOOD_FORCE);
            self.apply_force(food_force);
        }

        let mut desired_vel = self.vel;
        desired_vel += self.acc;
        if desired_vel.mag() < self.min_speed as f32 {
            desired_vel.set_mag(self.min_speed as f32);
        }
        let mut max_speed_co2 = arduino_map(
            co2,
            consts::CO2_BAD,
            consts::CO2_REALBAD,
            self.max_speed as i64,
            0,
        ) as f32;
        if max_speed_co2 < 0.0 {
            max_speed_co2 = 0.0;
        } else if max_speed_co2 > self.max_speed as f32 {
            max_speed_co2 = self.max_speed as f32;
        }
        desired_vel.limit(max_speed_co2);

        self.vel = self.vel.lerp(desired_vel, 1.0);
        self.pos += self.vel;
        self.acc = Vec2::ZERO;
        self.angle = self.vel.heading();

        self.following_food = false;
    }

    fn do_motion(&mut self, now_ms: u64) {
        match self.kind {
            MotionKind::Fish | MotionKind::Snake => {
                self.side_sine_motion(now_ms);
                self.noise_motion();
            }
            MotionKind::Star | MotionKind::Turtle => {
                self.front_sine_motion(now_ms);
                self.noise_motion();
            }
            MotionKind::Octopus => self.front_sine_motion(now_ms),
        }
    }

    fn apply_force(&mut self, force: Vec2) {
        self.acc += force;
    }

    fn boundary_check(&mut self, boundary_force: f32) {
        self.out_of_boundary = false;
        let mut margin_x = consts::BORDER_BUFFER;
        let mut margin_top = consts::BORDER_BUFFER;
        let mut margin_bottom = consts::BORDER_BUFFER;

        if boundary_force > consts::BOUNDARY_FORCE {
            margin_x = consts::KEEP_INSIDE_MARGIN_X * consts::PHYSICS_SCALE;
            margin_top = consts::KEEP_INSIDE_MARGIN_TOP * consts::PHYSICS_SCALE;
            margin_bottom = consts::KEEP_INSIDE_MARGIN_BOTTOM * consts::PHYSICS_SCALE;
        }

        if self.pos.x < margin_x {
            self.apply_force(Vec2::new(boundary_force, 0.0));
            self.out_of_boundary = true;
        }
        if self.pos.y < margin_top {
            self.apply_force(Vec2::new(0.0, boundary_force));
            self.out_of_boundary = true;
        }
        if self.pos.x > self.x_resolution - margin_x {
            self.apply_force(Vec2::new(-boundary_force, 0.0));
            self.out_of_boundary = true;
        }
        if self.pos.y > self.y_resolution - margin_bottom {
            self.apply_force(Vec2::new(0.0, -boundary_force));
            self.out_of_boundary = true;
        }
    }

    fn front_sine_motion(&mut self, now_ms: u64) {
        let theta = self.vel.heading();
        let angle = now_ms as f32 * self.sin_frequency + self.angle_offset;
        let y_offset = angle.sin() * self.sin_amplitude as f32;
        let mut sinusoidal_force = Vec2::from_angle(theta);
        sinusoidal_force *= y_offset;
        self.apply_force(sinusoidal_force);
    }

    fn side_sine_motion(&mut self, now_ms: u64) {
        let theta = self.vel.heading() + PI / 2.0;
        let angle = now_ms as f32 * self.sin_frequency + self.angle_offset;
        let y_offset = angle.sin() * self.sin_amplitude as f32;
        let mut sinusoidal_force = Vec2::from_angle(theta);
        sinusoidal_force *= y_offset;
        self.apply_force(sinusoidal_force);
    }

    fn noise_motion(&mut self) {
        let noise_value = self.noise.get_noise(self.pos.x, self.pos.y);
        let noise_angle = noise_value * TWO_PI;
        let mut noise_force = Vec2::from_angle(noise_angle);
        noise_force *= self.noise_amplitude * noise_value;
        self.apply_force(noise_force);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XR: f32 = 80.0 * consts::PHYSICS_SCALE;
    const YR: f32 = 106.0 * consts::PHYSICS_SCALE;

    fn motion(kind: MotionKind) -> Motion {
        Motion::new(
            kind,
            Vec2::new(XR / 2.0, YR / 2.0),
            XR,
            YR,
            &mut Rng::new(5),
        )
    }

    #[test]
    fn initial_velocity_above_min() {
        for kind in [
            MotionKind::Fish,
            MotionKind::Star,
            MotionKind::Turtle,
            MotionKind::Snake,
            MotionKind::Octopus,
        ] {
            let m = motion(kind);
            assert!(m.velocity().mag() >= 5.0);
        }
    }

    #[test]
    fn egg_does_not_move() {
        let mut m = motion(MotionKind::Fish);
        let before = m.position();
        m.update(0.05, 420, true, 1000);
        assert_eq!(m.position(), before);
    }

    #[test]
    fn adult_moves_and_stays_in_bounds() {
        let mut m = motion(MotionKind::Fish);
        for frame in 0..3000 {
            m.update(0.8, 420, true, frame * 33);
        }
        let p = m.position();
        assert!(p.x > -200.0 && p.x < XR + 200.0, "x={}", p.x);
        assert!(p.y > -200.0 && p.y < YR + 200.0, "y={}", p.y);
        // Speed should respect the CO2-scaled max.
        assert!(m.velocity().mag() <= 30.01, "speed={}", m.velocity().mag());
    }

    #[test]
    fn min_speed_is_enforced() {
        let mut m = motion(MotionKind::Turtle);
        m.update(0.8, 420, true, 100);
        assert!(m.velocity().mag() >= 5.0 - 1e-3);
    }

    #[test]
    fn octopus_motion_uses_only_front_sine_force() {
        let mut actual = motion(MotionKind::Octopus);
        let mut expected = motion(MotionKind::Octopus);

        actual.do_motion(1_234);
        expected.front_sine_motion(1_234);

        assert!((actual.acc.x - expected.acc.x).abs() < 1e-6);
        assert!((actual.acc.y - expected.acc.y).abs() < 1e-6);
    }

    #[test]
    fn follow_food_pulls_toward_target() {
        let mut m = motion(MotionKind::Fish);
        let start = m.position();
        let food = Vec2::new(start.x + 500.0, start.y);
        for i in 0..60 {
            m.follow_food(food);
            m.update(0.8, 420, true, 1000 + i * 33);
        }
        let after = m.position();
        assert!(
            after.x > start.x,
            "fish should move toward food: {start:?} -> {after:?}"
        );
    }

    #[test]
    fn co2_slows_creatures() {
        // Above CO2_REALBAD the max speed maps to zero.
        let mut m = motion(MotionKind::Fish);
        for i in 0..120 {
            m.update(0.8, 2500, true, i * 33);
        }
        assert!(m.velocity().mag() < 10.0, "speed={}", m.velocity().mag());
    }
}
