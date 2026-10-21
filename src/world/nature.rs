use serde::{Deserialize, Serialize};

use super::{Activity, Random};

/// The inclinations a ghost is born with, kept for its whole life. Each
/// trait runs from zero to one, with a half as average.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Nature {
    pub playful: f32,
    pub curious: f32,
    pub shy: f32,
    pub sleepy: f32,
    pub tender: f32,
}

impl Default for Nature {
    fn default() -> Self {
        Self {
            playful: 0.5,
            curious: 0.5,
            shy: 0.5,
            sleepy: 0.5,
            tender: 0.5,
        }
    }
}

impl Nature {
    /// Rolls mostly middling traits, with one clear inclination and room
    /// for a quieter second one.
    pub fn roll(random: &mut Random) -> Self {
        let mut traits = [0.0; 5];
        for value in &mut traits {
            *value = (random.unit() + random.unit() + random.unit()) / 3.0;
        }
        let mut order = [0, 1, 2, 3, 4];
        order.sort_by(|first, second| {
            (traits[*second] - 0.5)
                .abs()
                .total_cmp(&(traits[*first] - 0.5).abs())
        });
        for (rank, index) in order.into_iter().enumerate() {
            let offset = traits[index] - 0.5;
            let strength = match rank {
                0 => offset.abs().clamp(0.32, 0.44),
                1 => offset.abs().min(0.24),
                _ => offset.abs().min(0.14),
            };
            traits[index] = 0.5 + strength.copysign(offset);
        }
        let [playful, curious, shy, sleepy, tender] = traits;
        Self {
            playful,
            curious,
            shy,
            sleepy,
            tender,
        }
    }

    /// Gives an older ghost a stable nature from its birth's Unix seconds.
    pub fn from_birth(born: i64) -> Self {
        Self::roll(&mut Random::new(born as u64 ^ 0x6E61_7475_7265))
    }

    /// Keeps loaded traits in range; a nonfinite value becomes average.
    pub fn sanitized(mut self) -> Self {
        for value in [
            &mut self.playful,
            &mut self.curious,
            &mut self.shy,
            &mut self.sleepy,
            &mut self.tender,
        ] {
            *value = if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.5
            };
        }
        self
    }

    /// How its nature tips an activity's weight, from three quarters to
    /// a little over a third extra. Familiarity softens shyness at the glass
    /// and lets tenderness draw it toward a friend.
    pub fn leaning(self, activity: Activity, bond: f32) -> f32 {
        let signed = |value: f32| 2.0 * (value - 0.5);
        let inclination = match activity {
            Activity::Play
            | Activity::Fetch
            | Activity::Dance
            | Activity::Pounce
            | Activity::Game
            | Activity::Puzzle
            | Activity::Friend => signed(self.playful),
            Activity::Hide => (signed(self.playful) + signed(self.shy)) / 2.0,
            Activity::Explore
            | Activity::Window
            | Activity::Build
            | Activity::Look
            | Activity::Mirror => signed(self.curious),
            Activity::Read => (signed(self.curious) + signed(self.tender)) / 2.0,
            Activity::Rest => signed(self.sleepy),
            Activity::Music => (signed(self.tender) + signed(self.playful)) / 2.0,
            Activity::Tend | Activity::Doodle => signed(self.tender),
            Activity::Visit => {
                -signed(self.shy) * (1.0 - bond / 0.3).clamp(0.0, 1.0)
                    + signed(self.tender) * ((bond - 0.3) / 0.5).clamp(0.0, 1.0)
            }
            Activity::Float
            | Activity::Chase
            | Activity::Lamp
            | Activity::Wait
            | Activity::Company
            | Activity::Air => 0.0,
        };
        let strength = if inclination > 0.0 { 0.35 } else { 0.25 };
        (1.0 + strength * inclination).clamp(0.75, 1.35)
    }

    /// Its clearest one or two inclinations, in words the minds can use.
    pub fn words(self) -> String {
        let mut traits = [
            (self.playful, "playful", "quiet"),
            (self.curious, "curious", "settled"),
            (self.shy, "shy", "outgoing"),
            (self.sleepy, "sleepy", "slow to sleep"),
            (self.tender, "tender", "independent"),
        ];
        traits.sort_by(|first, second| (second.0 - 0.5).abs().total_cmp(&(first.0 - 0.5).abs()));
        let words: Vec<String> = traits
            .into_iter()
            .filter(|(value, _, _)| (value - 0.5).abs() >= 0.16)
            .take(2)
            .map(|(value, high, low)| {
                let word = if value >= 0.5 { high } else { low };
                if (value - 0.5).abs() < 0.28 {
                    format!("a little {word}")
                } else {
                    word.to_owned()
                }
            })
            .collect();
        if words.is_empty() {
            "even-tempered".to_owned()
        } else {
            words.join(" and ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(nature: Nature) -> [f32; 5] {
        [
            nature.playful,
            nature.curious,
            nature.shy,
            nature.sleepy,
            nature.tender,
        ]
    }

    #[test]
    fn seeds_give_repeatable_and_distinct_natures() {
        let rolled = |seed| Nature::roll(&mut Random::new(seed));
        assert_eq!(rolled(7), rolled(7));
        assert_ne!(rolled(7), rolled(8));
    }

    #[test]
    fn most_traits_are_middling_with_one_or_two_clear_inclinations() {
        let mut middling = 0;
        let mut sums = [0.0; 5];
        for seed in 0..2000 {
            let traits = values(Nature::roll(&mut Random::new(seed)));
            assert!(traits.iter().all(|value| (0.0..=1.0).contains(value)));
            assert!(traits.iter().any(|value| (value - 0.5).abs() >= 0.31));
            assert!(
                traits
                    .iter()
                    .filter(|value| (**value - 0.5).abs() >= 0.16)
                    .count()
                    <= 2
            );
            middling += traits
                .iter()
                .filter(|value| (0.25..=0.75).contains(*value))
                .count();
            for (sum, value) in sums.iter_mut().zip(traits) {
                *sum += value;
            }
        }
        assert!(middling > 7000);
        assert!(
            sums.iter()
                .all(|sum| (0.47..0.53).contains(&(sum / 2000.0)))
        );
    }

    #[test]
    fn a_birth_timestamp_always_gives_the_same_nature() {
        for born in [-1, 0, 1, 1_790_000_000, i64::MAX] {
            assert_eq!(Nature::from_birth(born), Nature::from_birth(born));
            assert_ne!(Nature::from_birth(born), Nature::from_birth(born - 1));
        }
    }

    #[test]
    fn every_activity_leans_moderately_even_with_combined_extremes() {
        for combination in 0..243 {
            let mut remaining = combination;
            let mut traits = [0.0; 5];
            for value in &mut traits {
                *value = (remaining % 3) as f32 * 0.5;
                remaining /= 3;
            }
            let [playful, curious, shy, sleepy, tender] = traits;
            let nature = Nature {
                playful,
                curious,
                shy,
                sleepy,
                tender,
            };
            for activity in Activity::ALL {
                for bond in [0.0, 0.15, 0.3, 0.55, 0.8, 1.0] {
                    assert!((0.75..=1.35).contains(&nature.leaning(activity, bond)));
                }
            }
        }
        for activity in Activity::ALL {
            assert_eq!(Nature::default().leaning(activity, 0.5), 1.0);
        }
    }

    #[test]
    fn shyness_yields_to_familiarity_and_tenderness_draws_it_to_a_friend() {
        let shy = Nature {
            shy: 1.0,
            ..Nature::default()
        };
        assert!(shy.leaning(Activity::Visit, 0.0) < 1.0);
        assert_eq!(shy.leaning(Activity::Visit, 0.8), 1.0);
        assert!(shy.leaning(Activity::Hide, 0.0) > 1.0);
        let tender = Nature {
            tender: 1.0,
            ..Nature::default()
        };
        assert_eq!(tender.leaning(Activity::Visit, 0.0), 1.0);
        assert!(tender.leaning(Activity::Visit, 0.8) > 1.0);
        assert!(tender.leaning(Activity::Tend, 0.0) > 1.0);
        assert!(tender.leaning(Activity::Doodle, 0.0) > 1.0);
    }

    #[test]
    fn words_name_the_pronounced_traits_without_digits() {
        let nature = Nature {
            playful: 0.95,
            shy: 0.72,
            ..Nature::default()
        };
        assert_eq!(nature.words(), "playful and a little shy");
        assert_eq!(Nature::default().words(), "even-tempered");
        for (index, word) in ["playful", "curious", "shy", "sleepy", "tender"]
            .into_iter()
            .enumerate()
        {
            let mut value = serde_json::to_value(Nature::default()).unwrap();
            value[word] = serde_json::json!(1.0);
            let pronounced: Nature = serde_json::from_value(value).unwrap();
            assert_eq!(pronounced.words(), word, "trait {index}");
        }
        for seed in 0..200 {
            let words = Nature::roll(&mut Random::new(seed)).words();
            assert!(!words.chars().any(|character| character.is_ascii_digit()));
            assert_ne!(words, "even-tempered");
        }
        let night_owl = Nature {
            sleepy: 0.0,
            ..Nature::default()
        };
        assert_eq!(night_owl.words(), "slow to sleep");
    }
}
