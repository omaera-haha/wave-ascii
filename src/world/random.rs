/// A small seeded random source (SplitMix64), so a world started from the
/// same seed plays out the same way.
#[derive(Clone, Debug)]
pub struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut bits = self.0;
        bits = (bits ^ (bits >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        bits = (bits ^ (bits >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        bits ^ (bits >> 31)
    }

    /// A number from 0 up to but not including 1.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.unit()
    }

    /// An index picked with chances in proportion to `weights`, or `None`
    /// when no weight is positive.
    pub fn pick(&mut self, weights: &[f32]) -> Option<usize> {
        let total: f32 = weights.iter().filter(|weight| **weight > 0.0).sum();
        if total <= 0.0 {
            return None;
        }
        let mut remaining = self.unit() * total;
        let mut last = None;
        for (index, weight) in weights.iter().enumerate() {
            if *weight <= 0.0 {
                continue;
            }
            last = Some(index);
            remaining -= weight;
            if remaining < 0.0 {
                return Some(index);
            }
        }
        last
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_numbers() {
        let mut first = Random::new(7);
        let mut second = Random::new(7);
        for _ in 0..100 {
            assert_eq!(first.next_u64(), second.next_u64());
        }
    }

    #[test]
    fn unit_numbers_fall_in_zero_to_one() {
        let mut random = Random::new(1);
        let numbers: Vec<f32> = (0..2000).map(|_| random.unit()).collect();
        assert!(numbers.iter().all(|number| (0.0..1.0).contains(number)));
        let mean = numbers.iter().sum::<f32>() / numbers.len() as f32;
        assert!((0.45..0.55).contains(&mean), "{mean}");
    }

    #[test]
    fn picks_follow_their_weights_and_skip_empty_ones() {
        let mut random = Random::new(3);
        let mut counts = [0; 3];
        for _ in 0..4000 {
            counts[random.pick(&[1.0, 0.0, 3.0]).unwrap()] += 1;
        }
        assert_eq!(counts[1], 0);
        let share = counts[2] as f32 / 4000.0;
        assert!((0.7..0.8).contains(&share), "{share}");
        assert_eq!(random.pick(&[0.0, -1.0]), None);
    }
}
