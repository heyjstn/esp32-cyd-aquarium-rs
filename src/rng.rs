//! Small deterministic PRNG used in place of Arduino `random()`.
//!
//! Behavior does not need to match Arduino's random stream bit-for-bit (the
//! original firmware seeds it from hardware entropy anyway), only the value
//! ranges matter. A seedable generator keeps unit tests deterministic.

/// xorshift32 PRNG.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u32,
}

impl Rng {
    pub fn new(seed: u32) -> Self {
        // A zero seed would lock xorshift at zero forever.
        let state = if seed == 0 { 0x9E37_79B9 } else { seed };
        Self { state }
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Arduino random(min, max): uniform in [min, max).
    pub fn range(&mut self, min: i32, max_excl: i32) -> i32 {
        if max_excl <= min {
            return min;
        }
        let span = (max_excl - min) as u32;
        min + (self.next_u32() % span) as i32
    }

    /// Arduino random(max): uniform in [0, max).
    pub fn rand_max(&mut self, max_excl: i32) -> i32 {
        self.range(0, max_excl)
    }

    /// Uniform f32 in [0, 1).
    pub fn f32_unit(&mut self) -> f32 {
        // Use the top 24 bits (exact f32 mantissa range).
        (self.next_u32() >> 8) as f32 / 16_777_216.0
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32_unit() < p
    }

    /// Pick an index with probability proportional to its weight
    /// (std::discrete_distribution equivalent).
    pub fn pick_weighted(&mut self, weights: &[f32]) -> usize {
        let total: f32 = weights.iter().sum();
        if total <= 0.0 {
            return 0;
        }
        let mut roll = self.f32_unit() * total;
        for (i, w) in weights.iter().enumerate() {
            roll -= w;
            if roll < 0.0 {
                return i;
            }
        }
        weights.len() - 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn zero_seed_does_not_lock() {
        let mut r = Rng::new(0);
        assert_ne!(r.next_u32(), 0);
        assert_ne!(r.next_u32(), 0);
    }

    #[test]
    fn range_bounds() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let v = r.range(4, 12);
            assert!((4..12).contains(&v));
        }
        assert_eq!(r.range(5, 5), 5);
        assert_eq!(r.range(5, 3), 5);
    }

    #[test]
    fn unit_interval() {
        let mut r = Rng::new(9);
        for _ in 0..1000 {
            let v = r.f32_unit();
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn weighted_pick_distribution() {
        let mut r = Rng::new(1234);
        let mut counts = [0usize; 3];
        for _ in 0..10_000 {
            counts[r.pick_weighted(&[0.5, 0.3, 0.2])] += 1;
        }
        assert!(counts[0] > counts[1]);
        assert!(counts[1] > counts[2]);
        assert!(counts[0] > 4000);
    }

    #[test]
    fn chance_never_and_always() {
        let mut r = Rng::new(3);
        for _ in 0..100 {
            assert!(!r.chance(0.0));
            assert!(r.chance(1.0));
        }
    }
}
