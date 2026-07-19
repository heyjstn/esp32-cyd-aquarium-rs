//! Aquarium orchestration: population, feeding, world updates and periodic
//! state saving, ported from lib/Aquarium/Aquarium.h (CYD build path).

use std::collections::HashMap;

use crate::consts;
use crate::fish::{Fish, FishDefinition, FishUpdate};
use crate::layer::Layer;
use crate::rng::Rng;
use crate::vec2::Vec2;
use crate::world::{BoidManager, Food, Plants, Water};

struct FoodItem {
    id: u64,
    food: Food,
}

pub struct Aquarium {
    x_res: u8,
    y_res: u8,
    water: Water,
    fish: Vec<Fish>,
    plants: Vec<Plants>,
    food: Vec<FoodItem>,
    boids: BoidManager,
    next_food_id: u64,

    touch_active: bool,
    last_food_ms: u64,
    last_autonomous_food_ms: u64,
    next_autonomous_food_delay: u64,

    last_save_ms: u64,
    save_pending: bool,
}

impl Aquarium {
    pub fn new(x_res: u8, y_res: u8) -> Self {
        Self {
            x_res,
            y_res,
            water: Water::new(x_res as usize, y_res as usize),
            fish: Vec::new(),
            plants: Vec::new(),
            food: Vec::new(),
            boids: BoidManager::new(),
            next_food_id: 0,
            touch_active: false,
            last_food_ms: 0,
            last_autonomous_food_ms: 0,
            next_autonomous_food_delay: 0,
            last_save_ms: 0,
            save_pending: false,
        }
    }

    /// Curated boot population (CYD_CURATED_BOOT_POPULATION).
    pub fn begin(&mut self, now_ms: u64, rng: &mut Rng) {
        self.fish.clear();
        for i in 0..consts::NUM_FISH_START {
            let pos = self.safe_spawn_position(i, rng);
            let age = initial_creature_age(i, rng);
            let kind = curated_creature_type(i, rng);
            self.fish.push(Fish::spawn(
                self.x_res,
                self.y_res,
                pos,
                age,
                Some(kind),
                rng,
                now_ms,
            ));
        }

        for i in 0..consts::NUM_PLANTS {
            let x = (self.x_res as usize * i / consts::NUM_PLANTS) as u8;
            self.plants.push(Plants::new(x, self.y_res + 7, rng));
        }

        self.boids.initialize(self.x_res, self.y_res, rng);
        self.schedule_next_autonomous_food(true, now_ms, rng);
    }

    /// Restore fish from saved definitions (used when the curated boot
    /// population is disabled; kept for parity with the C++ loadState path).
    pub fn load_fish(&mut self, defs: &[FishDefinition], now_ms: u64, rng: &mut Rng) {
        self.fish.clear();
        for def in defs {
            self.fish.push(Fish::from_definition(
                def, self.x_res, self.y_res, rng, now_ms,
            ));
        }
    }

    pub fn fish_definitions(&self) -> Vec<FishDefinition> {
        self.fish.iter().map(Fish::definition).collect()
    }

    pub fn fish_count(&self) -> usize {
        self.fish.len()
    }

    pub fn active_food_count(&self) -> u8 {
        self.food
            .iter()
            .filter(|f| !f.food.is_off_screen() && !f.food.is_eaten())
            .count() as u8
    }

    pub fn take_save_pending(&mut self) -> bool {
        core::mem::take(&mut self.save_pending)
    }

    pub fn on_touch_started(&mut self, now_ms: u64, rng: &mut Rng) {
        if !self.touch_active {
            self.touch_active = true;
            self.add_food(None, rng);
            self.last_food_ms = now_ms;
        }
    }

    pub fn on_touch_released(&mut self) {
        self.touch_active = false;
    }

    fn handle_touch_input(&mut self, now_ms: u64, rng: &mut Rng) {
        if self.touch_active
            && now_ms.wrapping_sub(self.last_food_ms) >= consts::TOUCH_FOOD_INTERVAL_MS
        {
            self.add_food(None, rng);
            self.last_food_ms = now_ms;
        }
    }

    fn schedule_next_autonomous_food(&mut self, boot_window: bool, now_ms: u64, rng: &mut Rng) {
        self.last_autonomous_food_ms = now_ms;
        self.next_autonomous_food_delay = if boot_window {
            random_delay_between(
                rng,
                consts::AUTONOMOUS_BOOT_MIN_MS,
                consts::AUTONOMOUS_BOOT_MAX_MS,
            )
        } else {
            random_delay_between(
                rng,
                consts::AUTONOMOUS_FOOD_MIN_MS,
                consts::AUTONOMOUS_FOOD_MAX_MS,
            )
        };
    }

    fn autonomous_food_x(&self, rng: &mut Rng) -> f32 {
        let width = self.x_res as i32;
        if width <= 8 {
            return rng.range(0, width) as f32;
        }
        rng.range(4, width - 4) as f32
    }

    fn handle_autonomous_life(&mut self, now_ms: u64, rng: &mut Rng) {
        if self.next_autonomous_food_delay == 0 {
            self.schedule_next_autonomous_food(true, now_ms, rng);
        }

        if now_ms.wrapping_sub(self.last_autonomous_food_ms) < self.next_autonomous_food_delay {
            return;
        }

        if self.active_food_count() < consts::AUTONOMOUS_MAX_FOOD {
            let x = self.autonomous_food_x(rng);
            self.add_food(Some(x), rng);
        }
        self.schedule_next_autonomous_food(false, now_ms, rng);
    }

    /// Drop food at x (random when None) and assign the closest eligible
    /// fish to chase it.
    pub fn add_food(&mut self, x: Option<f32>, rng: &mut Rng) {
        let x = match x {
            Some(x) if x >= 0.0 => x,
            _ => rng.range(0, self.x_res as i32) as f32,
        };

        let id = self.next_food_id;
        self.next_food_id += 1;
        self.food.push(FoodItem {
            id,
            food: Food::new(x),
        });

        let mut min_distance = f32::MAX;
        let mut closest: Option<usize> = None;
        for (i, fish) in self.fish.iter().enumerate() {
            if fish.food_id().is_none()
                && fish.age() > consts::AGE_EGG
                && fish.age() < consts::AGE_SENIOR
            {
                let distance = fish.position().dist(Vec2::new(x, 0.0));
                if distance < min_distance {
                    min_distance = distance;
                    closest = Some(i);
                }
            }
        }
        if let Some(i) = closest {
            self.fish[i].set_food(id);
        }
    }

    fn update_fish(&mut self, now_ms: u64, foreground: &mut Layer, rng: &mut Rng) {
        let food_positions: HashMap<u64, (Vec2, bool)> = self
            .food
            .iter()
            .map(|item| (item.id, (item.food.position(), item.food.is_off_screen())))
            .collect();

        for fish in &mut self.fish {
            let food_pos = match fish.food_id() {
                Some(id) => match food_positions.get(&id) {
                    Some((pos, false)) => Some(*pos),
                    _ => {
                        fish.clear_food();
                        None
                    }
                },
                None => None,
            };

            if let FishUpdate::AteFood(id) =
                fish.update(consts::DEFAULT_CO2_PPM, true, now_ms, food_pos)
            {
                if let Some(item) = self.food.iter_mut().find(|item| item.id == id) {
                    item.food.eat();
                }
            }
            fish.display(foreground, now_ms);
        }

        // Population control: one new fish per update at most.
        if self.fish.len() < consts::NUM_FISH_IDEAL {
            for i in 0..self.fish.len() {
                if self.fish[i].try_reproduce(rng) {
                    let pos = self.fish[i].position();
                    let baby = Fish::spawn(self.x_res, self.y_res, pos, 0.0, None, rng, now_ms);
                    self.fish.push(baby);
                    break;
                }
            }
        }
    }

    fn update_food(&mut self, foreground: &mut Layer) {
        let y_res = self.y_res;
        self.food.retain_mut(|item| {
            item.food.update(y_res);
            if item.food.is_off_screen() || item.food.is_eaten() {
                return false;
            }
            item.food.display(foreground);
            true
        });
    }

    pub fn update(
        &mut self,
        now_ms: u64,
        rng: &mut Rng,
        foreground: &mut Layer,
        background: &mut Layer,
    ) {
        self.handle_touch_input(now_ms, rng);
        self.handle_autonomous_life(now_ms, rng);

        self.water
            .update(consts::DEFAULT_TEMPERATURE_C as i64, now_ms, background);
        self.boids.update(consts::DEFAULT_CO2_PPM);
        self.boids.render(foreground);
        self.update_fish(now_ms, foreground, rng);
        self.update_food(foreground);
        for plant in &self.plants {
            plant.update(consts::FIXED_HUMIDITY_PERCENT, now_ms, foreground);
        }

        if now_ms.wrapping_sub(self.last_save_ms) >= consts::AQUARIUM_SAVE_INTERVAL_MS {
            self.last_save_ms = now_ms;
            self.save_pending = true;
        }
    }

    fn safe_spawn_position(&self, index: usize, rng: &mut Rng) -> Vec2 {
        let width = self.x_res as f32;
        let height = self.y_res as f32;
        const COLS: usize = 3;
        const ROWS: usize = 3;
        let cell = index % (COLS * ROWS);
        let col = (cell % COLS) as f32;
        let row = (cell / COLS) as f32;

        let mut margin_x = if width > 24.0 { 8.0 } else { 1.0 };
        let mut margin_top = if height > 48.0 { 18.0 } else { 1.0 };
        let mut margin_bottom = if height > 48.0 { 18.0 } else { 1.0 };
        if margin_top + margin_bottom >= height {
            margin_top = 1.0;
            margin_bottom = 1.0;
        }
        if margin_x * 2.0 >= width {
            margin_x = 1.0;
        }

        let mut usable_w = width - margin_x * 2.0;
        let mut usable_h = height - margin_top - margin_bottom;
        if usable_w < 1.0 {
            usable_w = 1.0;
        }
        if usable_h < 1.0 {
            usable_h = 1.0;
        }

        let jitter_x = rng.range(20, 80) as f32 / 100.0;
        let jitter_y = rng.range(18, 82) as f32 / 100.0;
        Vec2::new(
            margin_x + (col + jitter_x) * (usable_w / COLS as f32),
            margin_top + (row + jitter_y) * (usable_h / ROWS as f32),
        )
    }
}

fn random_delay_between(rng: &mut Rng, min_delay: u64, max_delay: u64) -> u64 {
    if max_delay <= min_delay {
        return min_delay;
    }
    min_delay + rng.next_u32() as u64 % (max_delay - min_delay + 1)
}

/// Curated boot ages (Aquarium.h initialCreatureAge).
fn initial_creature_age(index: usize, rng: &mut Rng) -> f32 {
    match index {
        0 => rng.range(74, 84) as f32 / 100.0,
        1 => rng.range(66, 78) as f32 / 100.0,
        _ => rng.range(52, 69) as f32 / 100.0,
    }
}

/// Curated boot creature mix (Aquarium.h curatedCreatureType).
fn curated_creature_type(index: usize, rng: &mut Rng) -> &'static str {
    match index {
        0 => "Turtle",
        1 => {
            if rng.range(0, 3) == 0 {
                "Octopus"
            } else {
                "Fish"
            }
        }
        2 | 3 | 5 | 6 => "Fish",
        4 | 7 => "Star",
        _ => {
            if rng.range(0, 5) == 0 {
                "Turtle"
            } else {
                "Fish"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng() -> Rng {
        Rng::new(0xACE0)
    }

    #[test]
    fn begin_creates_curated_population() {
        let mut aq = Aquarium::new(80, 106);
        aq.begin(0, &mut rng());
        assert_eq!(aq.fish_count(), 8);
        let defs = aq.fish_definitions();
        assert_eq!(defs[0].body_type, "Turtle");
        assert!(defs[0].age >= 0.74 && defs[0].age <= 0.83);
        assert!(["Octopus", "Fish"].contains(&defs[1].body_type.as_str()));
        assert_eq!(defs[4].body_type, "Star");
        assert_eq!(defs[7].body_type, "Star");
    }

    #[test]
    fn autonomous_food_appears_after_boot_window() {
        let mut aq = Aquarium::new(80, 106);
        let mut r = rng();
        aq.begin(0, &mut r);
        let mut fg = Layer::new(80, 106);
        let mut bg = Layer::new(80, 106);

        // Simulate 10 seconds; at least one food drop should have happened.
        let mut saw_food = false;
        for frame in 0..300 {
            aq.update(frame * 33, &mut r, &mut fg, &mut bg);
            if aq.active_food_count() > 0 {
                saw_food = true;
                break;
            }
            fg.clear();
        }
        assert!(saw_food, "autonomous feeding should drop food within 10 s");
    }

    #[test]
    fn autonomous_food_is_capped() {
        let mut aq = Aquarium::new(80, 106);
        let mut r = rng();
        aq.begin(0, &mut r);
        let mut fg = Layer::new(80, 106);
        let mut bg = Layer::new(80, 106);
        for frame in 0..3000 {
            aq.update(frame * 33, &mut r, &mut fg, &mut bg);
            assert!(aq.active_food_count() <= 3);
            fg.clear();
        }
    }

    #[test]
    fn touch_feeding_drops_food_immediately_and_periodically() {
        let mut aq = Aquarium::new(80, 106);
        let mut r = rng();
        aq.begin(0, &mut r);
        aq.on_touch_started(10_000, &mut r);
        assert_eq!(aq.active_food_count(), 1);

        let mut fg = Layer::new(80, 106);
        let mut bg = Layer::new(80, 106);
        for frame in 0..10 {
            aq.update(10_100 + frame * 33, &mut r, &mut fg, &mut bg);
        }
        assert!(
            aq.active_food_count() >= 2,
            "held touch should keep feeding"
        );

        aq.on_touch_released();
        let count = aq.active_food_count();
        aq.update(10_500, &mut r, &mut fg, &mut bg);
        assert!(aq.active_food_count() <= count + 1);
    }

    #[test]
    fn save_flag_fires_every_30_minutes() {
        let mut aq = Aquarium::new(80, 106);
        let mut r = rng();
        aq.begin(0, &mut r);
        let mut fg = Layer::new(80, 106);
        let mut bg = Layer::new(80, 106);
        assert!(!aq.take_save_pending());
        aq.update(1_800_000, &mut r, &mut fg, &mut bg);
        assert!(aq.take_save_pending());
        assert!(!aq.take_save_pending());
        aq.update(1_800_000 + 33, &mut r, &mut fg, &mut bg);
        assert!(!aq.take_save_pending());
        aq.update(3_600_001, &mut r, &mut fg, &mut bg);
        assert!(aq.take_save_pending());
    }

    #[test]
    fn fish_stay_on_screen_over_time() {
        let mut aq = Aquarium::new(80, 106);
        let mut r = rng();
        aq.begin(0, &mut r);
        let mut fg = Layer::new(80, 106);
        let mut bg = Layer::new(80, 106);
        for frame in 0..2000 {
            aq.update(frame * 33, &mut r, &mut fg, &mut bg);
            fg.clear();
        }
        for def in aq.fish_definitions() {
            let _ = def;
        }
        for fish in aq.fish.iter() {
            let p = fish.position();
            assert!(p.x > -20.0 && p.x < 100.0, "x={}", p.x);
            assert!(p.y > -20.0 && p.y < 126.0, "y={}", p.y);
        }
    }

    #[test]
    fn population_grows_via_reproduction() {
        let mut aq = Aquarium::new(80, 106);
        let mut r = rng();
        aq.begin(0, &mut r);
        let mut fg = Layer::new(80, 106);
        let mut bg = Layer::new(80, 106);
        // Adults reproduce at 1% per update per eligible fish; over 10k
        // frames the population should approach NUM_FISH_IDEAL.
        for frame in 0..10_000 {
            aq.update(frame * 33, &mut r, &mut fg, &mut bg);
            fg.clear();
        }
        assert!(
            aq.fish_count() > 8,
            "population should grow, got {}",
            aq.fish_count()
        );
        assert!(aq.fish_count() <= 12);
    }
}
