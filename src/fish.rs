//! Fish lifecycle: aging, health, reproduction, food chasing and the
//! body/motion pairing, ported from lib/Aquarium/Fish.h.

use crate::body::{create_body, Body, Fin, Head, Tail};
use crate::color::CHsv;
use crate::consts;
use crate::layer::Layer;
use crate::motion::{Motion, MotionKind};
use crate::rng::Rng;
use crate::vec2::Vec2;

/// Random creature mix for spontaneously spawned fish (CYD_RICH_CREATURE_MIX).
const SPAWN_TABLE: [(&str, f32); 5] = [
    ("Fish", 0.62),
    ("Star", 0.18),
    ("Turtle", 0.08),
    ("Snake", 0.06),
    ("Octopus", 0.06),
];

/// Serializable definition of one fish (AquariumStateManager JSON shape).
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FishDefinition {
    pub age: f32,
    pub health: f32,
    pub body_type: String,
    pub head_type: String,
    pub tail_type: String,
    pub fin_type: String,
    pub motion_type: String,
    pub colors: Vec<CHsv>,
}

/// Result of a single fish update.
pub enum FishUpdate {
    Alive,
    /// The fish reached its food; the aquarium must mark that food eaten.
    /// Carries the food id.
    AteFood(u64),
}

pub struct Fish {
    pos: Vec2,
    age: f32,
    health: f32,
    body: Body,
    motion: Motion,
    /// Id of the food item this fish is chasing (owned by the Aquarium).
    food_id: Option<u64>,
    offspring_count: u8,
    aging_rate: f32,
    last_update_ms: u64,
}

impl Fish {
    fn initialize_aging_rate(rng: &mut Rng) -> f32 {
        let base_rate = 1.0 / (consts::FISH_LIFESPAN_DAYS * 24.0 * 60.0 * 60.0 * 1000.0);
        let variation = consts::FISH_LIFESPAN_VARIATION * (rng.rand_max(200) as f32 / 100.0 - 1.0);
        base_rate * (1.0 + variation)
    }

    fn random_safe_position(x_res: u8, y_res: u8, rng: &mut Rng) -> Vec2 {
        let width = x_res as i32;
        let height = y_res as i32;

        let mut margin_x = if width > 24 { 8 } else { 1 };
        let mut margin_top = if height > 48 { 16 } else { 1 };
        let mut margin_bottom = if height > 48 { 16 } else { 1 };

        if margin_top + margin_bottom >= height {
            margin_top = 1;
            margin_bottom = 1;
        }
        if margin_x * 2 >= width {
            margin_x = 1;
        }

        let min_x = margin_x;
        let max_x = if width > margin_x {
            width - margin_x
        } else {
            width
        };
        let min_y = margin_top;
        let max_y = if height > margin_bottom {
            height - margin_bottom
        } else {
            height
        };

        Vec2::new(
            rng.range(min_x, max_x) as f32,
            rng.range(min_y, max_y) as f32,
        )
    }

    fn make_motion(kind: &str, pos: Vec2, x_res: u8, y_res: u8, rng: &mut Rng) -> Motion {
        let motion_kind = MotionKind::from_name(kind).unwrap_or(MotionKind::Fish);
        Motion::new(
            motion_kind,
            pos * consts::PHYSICS_SCALE,
            x_res as f32 * consts::PHYSICS_SCALE,
            y_res as f32 * consts::PHYSICS_SCALE,
            rng,
        )
    }

    /// Spawn a fish, either of a requested kind (curated boot population) or
    /// from the weighted random mix. Mirrors the C++ (matrix, pos, age,
    /// bodyType, health) constructor.
    pub fn spawn(
        x_res: u8,
        y_res: u8,
        pos: Vec2,
        age: f32,
        kind: Option<&str>,
        rng: &mut Rng,
        now_ms: u64,
    ) -> Self {
        let selected = match kind {
            Some("Fish") | Some("Star") | Some("Turtle") | Some("Snake") | Some("Octopus") => {
                kind.unwrap().to_string()
            }
            Some(_) | None => {
                let weights: Vec<f32> = SPAWN_TABLE.iter().map(|(_, w)| *w).collect();
                SPAWN_TABLE[rng.pick_weighted(&weights)].0.to_string()
            }
        };

        let mut pos = pos;
        if pos.x == 0.0 && pos.y == 0.0 {
            pos = Self::random_safe_position(x_res, y_res, rng);
        }

        let body = create_body(&selected, rng);
        let motion = Self::make_motion(&selected, pos, x_res, y_res, rng);

        Self {
            pos,
            age,
            health: 1.0,
            body,
            motion,
            food_id: None,
            offspring_count: 0,
            aging_rate: Self::initialize_aging_rate(rng),
            last_update_ms: now_ms,
        }
    }

    /// Restore a fish from a saved definition (state load path).
    pub fn from_definition(
        def: &FishDefinition,
        x_res: u8,
        y_res: u8,
        rng: &mut Rng,
        now_ms: u64,
    ) -> Self {
        let pos = Self::random_safe_position(x_res, y_res, rng);

        let mut body = if def.body_type == "Fish" {
            let head = Head::from_name(&def.head_type, rng);
            let tail = Tail::from_name(&def.tail_type, rng);
            let fin = Fin::from_name(&def.fin_type, rng);
            crate::body::create_fish_body_with_parts(rng, head, tail, fin)
        } else {
            create_body(&def.body_type, rng)
        };
        if !def.colors.is_empty() {
            body.palette = crate::body::ColorPalette::from_hsv(def.colors.clone());
        }

        let motion = Self::make_motion(&def.motion_type, pos, x_res, y_res, rng);

        Self {
            pos,
            age: def.age,
            health: def.health,
            body,
            motion,
            food_id: None,
            offspring_count: 0,
            aging_rate: Self::initialize_aging_rate(rng),
            last_update_ms: now_ms,
        }
    }

    pub fn definition(&self) -> FishDefinition {
        let (head, tail, fin) = match &self.body.shape {
            crate::body::BodyShape::Fish {
                head, tail, fin, ..
            } => (head.name(), tail.name(), fin.name()),
            _ => ("none", "noTail", "none"),
        };
        FishDefinition {
            age: self.age,
            health: self.health,
            body_type: self.body.kind_name().to_string(),
            head_type: head.to_string(),
            tail_type: tail.to_string(),
            fin_type: fin.to_string(),
            motion_type: self.motion.kind().name().to_string(),
            colors: self.body.palette.colors_hsv.clone(),
        }
    }

    pub fn position(&self) -> Vec2 {
        self.pos
    }

    pub fn age(&self) -> f32 {
        self.age
    }

    pub fn health(&self) -> f32 {
        self.health
    }

    pub fn food_id(&self) -> Option<u64> {
        self.food_id
    }

    pub fn set_food(&mut self, id: u64) {
        self.food_id = Some(id);
    }

    pub fn clear_food(&mut self) {
        self.food_id = None;
    }

    pub fn body_kind_name(&self) -> &'static str {
        self.body.kind_name()
    }

    /// food_pos: current position of the chased food, if any and still valid.
    pub fn update(
        &mut self,
        co2: i64,
        stay_inside: bool,
        now_ms: u64,
        food_pos: Option<Vec2>,
    ) -> FishUpdate {
        self.motion.update(self.age, co2, stay_inside, now_ms);
        self.pos = self.motion.position() / consts::PHYSICS_SCALE;
        self.update_age(co2, now_ms);
        self.update_health(co2);
        self.body.update(
            self.pos,
            self.motion.velocity(),
            self.motion.angle(),
            self.age,
            self.health,
        );

        let mut event = FishUpdate::Alive;
        if let Some(food_pos) = food_pos {
            let food_distance = food_pos - self.pos;
            if food_distance.mag() < 1.0 {
                if let Some(id) = self.food_id.take() {
                    event = FishUpdate::AteFood(id);
                }
            } else {
                self.motion.follow_food(food_pos * consts::PHYSICS_SCALE);
            }
        }

        // The original firmware wraps age back to zero (reincarnation as an
        // egg) instead of letting the fish die; mirrored deliberately.
        if self.age > 1.0 {
            self.age = 0.0;
        }

        event
    }

    pub fn display(&mut self, layer: &mut Layer, now_ms: u64) {
        if self.age < consts::AGE_EGG {
            self.body.display_egg(layer);
        } else {
            self.body.display(layer, now_ms);
        }
    }

    pub fn try_reproduce(&mut self, rng: &mut Rng) -> bool {
        if self.age > 0.5 && self.age < 0.9 && self.offspring_count < 2 && rng.chance(0.01) {
            self.offspring_count += 1;
            return true;
        }
        false
    }

    fn update_age(&mut self, co2: i64, now_ms: u64) {
        let time_diff = now_ms.wrapping_sub(self.last_update_ms) as f32;
        if co2 < consts::CO2_REALBAD {
            self.age += time_diff * self.aging_rate;
        }
        self.last_update_ms = now_ms;
    }

    fn update_health(&mut self, co2: i64) {
        if co2 >= consts::CO2_REALBAD {
            self.health -= consts::HEALTH_REDUCTION_RATE_REALBAD;
        } else if co2 >= consts::CO2_BAD {
            self.health -= consts::HEALTH_REDUCTION_RATE_BAD;
        } else {
            self.health += consts::HEALTH_INCREASE_RATE_GOOD;
        }
        self.health = self.health.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> Rng {
        Rng::new(0xF15A)
    }

    #[test]
    fn spawn_respects_requested_kind() {
        let f = Fish::spawn(80, 106, Vec2::ZERO, 0.5, Some("Turtle"), &mut rng(), 0);
        assert_eq!(f.body_kind_name(), "Turtle");
        // (0,0) is treated as "no position" and replaced with a safe spawn.
        assert!(f.position().x >= 1.0 || f.position().y >= 1.0);
    }

    #[test]
    fn spawn_random_mix_produces_valid_kinds() {
        let mut r = rng();
        for _ in 0..20 {
            let f = Fish::spawn(80, 106, Vec2::ZERO, 0.5, None, &mut r, 0);
            assert!(["Fish", "Star", "Turtle", "Snake", "Octopus"].contains(&f.body_kind_name()));
        }
    }

    #[test]
    fn unknown_kind_falls_back_to_random() {
        let f = Fish::spawn(80, 106, Vec2::ZERO, 0.5, Some("Blob"), &mut rng(), 0);
        assert!(["Fish", "Star", "Turtle", "Snake", "Octopus"].contains(&f.body_kind_name()));
    }

    #[test]
    fn definition_roundtrip_restores_kind_and_colors() {
        let mut r = rng();
        let f = Fish::spawn(
            80,
            106,
            Vec2::new(30.0, 40.0),
            0.66,
            Some("Fish"),
            &mut r,
            0,
        );
        let def = f.definition();
        assert_eq!(def.body_type, "Fish");
        assert!(!def.colors.is_empty());

        let restored = Fish::from_definition(&def, 80, 106, &mut r, 0);
        assert_eq!(restored.body_kind_name(), "Fish");
        assert_eq!(restored.age(), 0.66);
        assert_eq!(restored.body.palette.colors_hsv, f.body.palette.colors_hsv);
    }

    #[test]
    fn health_regenerates_in_good_air() {
        let mut f = Fish::spawn(
            80,
            106,
            Vec2::new(30.0, 40.0),
            0.5,
            Some("Fish"),
            &mut rng(),
            0,
        );
        f.health = 0.5;
        for i in 0..10 {
            f.update(420, true, 100 + i * 33, None);
        }
        assert_eq!(f.health(), 1.0);
    }

    #[test]
    fn health_drops_in_bad_air() {
        let mut f = Fish::spawn(
            80,
            106,
            Vec2::new(30.0, 40.0),
            0.5,
            Some("Fish"),
            &mut rng(),
            0,
        );
        for i in 0..4 {
            f.update(1500, true, 100 + i * 33, None);
        }
        assert!(f.health() < 1.0);
        assert!(f.health() > 0.5);
    }

    #[test]
    fn age_wraps_instead_of_dying() {
        let mut r = rng();
        let mut wrapped = false;
        for _ in 0..20 {
            // Start near the end of life and jump 8 days in one update; with
            // any reasonable aging rate the age crosses 1.0 and must wrap
            // back to the egg stage instead of killing the fish.
            let mut f = Fish::spawn(80, 106, Vec2::new(30.0, 40.0), 0.9, Some("Fish"), &mut r, 0);
            f.update(420, true, 8 * 24 * 60 * 60 * 1000, None);
            assert!(
                f.age() >= 0.0 && f.age() < 1.0,
                "age {} out of range",
                f.age()
            );
            if f.age() < 0.9 {
                wrapped = true; // age only grows, so a smaller age means a wrap
            }
        }
        assert!(wrapped, "at least one fish should have wrapped its age");
    }

    #[test]
    fn eats_food_within_one_pixel() {
        let mut f = Fish::spawn(
            80,
            106,
            Vec2::new(30.0, 40.0),
            0.5,
            Some("Fish"),
            &mut rng(),
            0,
        );
        f.update(420, true, 100, None);
        let pos = f.position();
        f.set_food(7);
        let event = f.update(420, true, 133, Some(pos));
        assert!(matches!(event, FishUpdate::AteFood(7)));
        assert_eq!(f.food_id(), None);
    }

    #[test]
    fn reproduction_requires_middle_age() {
        let mut r = rng();
        let mut young = Fish::spawn(80, 106, Vec2::new(30.0, 40.0), 0.2, Some("Fish"), &mut r, 0);
        for _ in 0..100 {
            assert!(!young.try_reproduce(&mut r));
        }
        let mut adult = Fish::spawn(80, 106, Vec2::new(30.0, 40.0), 0.7, Some("Fish"), &mut r, 0);
        let mut reproduced = false;
        for _ in 0..2000 {
            if adult.try_reproduce(&mut r) {
                reproduced = true;
                break;
            }
        }
        assert!(reproduced, "1% chance should hit within 2000 tries");
    }
}
