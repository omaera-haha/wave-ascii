//! The kinds of creature that can live in the cottage. The world only needs
//! to know how each gets about and what to call it; how each looks is up to
//! the renderer.

use serde::{Deserialize, Serialize};

use super::Random;

/// What kind of creature one is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Species {
    #[default]
    Ghost,
    Fox,
    Bunny,
    Penguin,
    Frog,
    Slime,
    Owl,
    Robot,
    Hedgehog,
    Mushroom,
    Axolotl,
    Dragon,
    Jelly,
}

/// How a body gets about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gait {
    /// Drifts anywhere in the air, as the ghost always has.
    Float,
    /// Flies where it likes but lands to rest, flapping as it goes.
    Fly,
    /// Walks on whatever is under it and jumps up onto things.
    Walk,
    /// Gets about in springy hops.
    Hop,
}

impl Species {
    pub const ALL: [Self; 13] = [
        Self::Ghost,
        Self::Fox,
        Self::Bunny,
        Self::Penguin,
        Self::Frog,
        Self::Slime,
        Self::Owl,
        Self::Robot,
        Self::Hedgehog,
        Self::Mushroom,
        Self::Axolotl,
        Self::Dragon,
        Self::Jelly,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Ghost => "ghost",
            Self::Fox => "fox",
            Self::Bunny => "bunny",
            Self::Penguin => "penguin",
            Self::Frog => "frog",
            Self::Slime => "slime",
            Self::Owl => "owl",
            Self::Robot => "robot",
            Self::Hedgehog => "hedgehog",
            Self::Mushroom => "mushroom",
            Self::Axolotl => "axolotl",
            Self::Dragon => "dragon",
            Self::Jelly => "jelly",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().to_lowercase();
        Self::ALL.into_iter().find(|species| {
            species.key() == text
                || species.noun() == text
                || species.noun_korean() == text
                || (text.len() > 2 && species.noun().starts_with(&text))
        })
    }

    /// What it is, in a word.
    pub fn noun(self) -> &'static str {
        match self {
            Self::Ghost => "ghost",
            Self::Fox => "fox",
            Self::Bunny => "bunny",
            Self::Penguin => "penguin",
            Self::Frog => "frog",
            Self::Slime => "slime",
            Self::Owl => "owl",
            Self::Robot => "robot",
            Self::Hedgehog => "hedgehog",
            Self::Mushroom => "mushroom sprite",
            Self::Axolotl => "axolotl",
            Self::Dragon => "dragon",
            Self::Jelly => "jellyfish",
        }
    }

    pub fn noun_korean(self) -> &'static str {
        match self {
            Self::Ghost => "유령",
            Self::Fox => "여우",
            Self::Bunny => "토끼",
            Self::Penguin => "펭귄",
            Self::Frog => "개구리",
            Self::Slime => "슬라임",
            Self::Owl => "부엉이",
            Self::Robot => "로봇",
            Self::Hedgehog => "고슴도치",
            Self::Mushroom => "버섯 요정",
            Self::Axolotl => "우파루파",
            Self::Dragon => "아기 용",
            Self::Jelly => "해파리",
        }
    }

    /// The creature as a mind is told about it.
    pub fn portrait(self) -> &'static str {
        match self {
            Self::Ghost => "a small glowing ghost who floats and can turn faint",
            Self::Fox => "a fox kit with big ears and a bushy white-tipped tail, who trots on four paws",
            Self::Bunny => "a round little bunny with long ears and a cotton tail, who hops",
            Self::Penguin => "a fluffy penguin chick who waddles and flaps its flippers",
            Self::Frog => "a squat little frog with big eyes on top of its head, who hops",
            Self::Slime => "a wobbly see-through slime who bounces and squishes",
            Self::Owl => "a round fluffy owlet with ear tufts, who flies and perches",
            Self::Robot => "a tiny round robot on one wheel, with an antenna light and a screen face",
            Self::Hedgehog => "a prickly little hedgehog who scurries on tiny feet",
            Self::Mushroom => "a mushroom sprite with a spotted cap, who toddles on tiny feet",
            Self::Axolotl => "a pink axolotl with frilly gills and a wide smile, who crawls",
            Self::Dragon => "a baby dragon with stubby wings and a curly tail, who flutters about",
            Self::Jelly => "a little jellyfish who drifts through the air trailing ribbons",
        }
    }

    pub fn gait(self) -> Gait {
        match self {
            Self::Ghost | Self::Jelly => Gait::Float,
            Self::Owl | Self::Dragon => Gait::Fly,
            Self::Bunny | Self::Frog | Self::Slime => Gait::Hop,
            Self::Fox
            | Self::Penguin
            | Self::Robot
            | Self::Hedgehog
            | Self::Mushroom
            | Self::Axolotl => Gait::Walk,
        }
    }

    /// Whether it stays up in the air rather than on what is under it.
    pub fn airborne(self) -> bool {
        self.gait() == Gait::Float
    }

    /// How high its middle rides above what it stands on, at full size.
    pub fn stance(self) -> f32 {
        match self {
            Self::Ghost | Self::Jelly => 0.38,
            Self::Fox => 0.2,
            Self::Bunny => 0.16,
            Self::Penguin => 0.2,
            Self::Frog => 0.12,
            Self::Slime => 0.13,
            Self::Owl => 0.18,
            Self::Robot => 0.22,
            Self::Hedgehog => 0.12,
            Self::Mushroom => 0.2,
            Self::Axolotl => 0.1,
            Self::Dragon => 0.2,
        }
    }

    /// How quickly it gets about, compared with the ghost.
    pub fn pace(self) -> f32 {
        match self {
            Self::Ghost | Self::Jelly => 1.0,
            Self::Fox | Self::Hedgehog => 1.1,
            Self::Bunny | Self::Frog => 1.05,
            Self::Owl | Self::Dragon => 1.0,
            Self::Penguin | Self::Mushroom | Self::Axolotl => 0.75,
            Self::Slime => 0.85,
            Self::Robot => 0.95,
        }
    }

    /// How many coats the species comes in.
    pub fn coats(self) -> u8 {
        match self {
            Self::Ghost | Self::Robot | Self::Axolotl => 3,
            Self::Fox | Self::Penguin | Self::Owl | Self::Dragon | Self::Mushroom => 3,
            Self::Bunny => 4,
            Self::Frog | Self::Slime | Self::Jelly => 4,
            Self::Hedgehog => 2,
        }
    }

    /// The sound it makes when it's happy, for its words.
    pub fn sound(self) -> &'static str {
        match self {
            Self::Ghost => "ooo",
            Self::Fox => "yip",
            Self::Bunny => "*thump*",
            Self::Penguin => "peep",
            Self::Frog => "ribbit",
            Self::Slime => "blorp",
            Self::Owl => "hoo",
            Self::Robot => "beep",
            Self::Hedgehog => "huff",
            Self::Mushroom => "puff",
            Self::Axolotl => "blub",
            Self::Dragon => "rawr",
            Self::Jelly => "bloop",
        }
    }

    /// Species a new household starts with: three different ones, all
    /// living creatures rather than ghosts.
    pub fn starters(random: &mut Random) -> [Self; 3] {
        let mut pool: Vec<Self> = Self::ALL
            .into_iter()
            .filter(|species| *species != Self::Ghost)
            .collect();
        [(); 3].map(|_| {
            let index = (random.unit() * pool.len() as f32) as usize;
            pool.remove(index.min(pool.len() - 1))
        })
    }
}

/// What makes one creature look unlike others of its kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Genome {
    /// Which of its species' coats it has.
    pub coat: u8,
    /// A little variety in build, from slim at -1 to plump at 1.
    pub build: f32,
    /// Seeds its spots and freckles.
    pub seed: u32,
}

impl Genome {
    pub fn roll(species: Species, random: &mut Random) -> Self {
        Self {
            coat: ((random.unit() * f32::from(species.coats())) as u8)
                .min(species.coats().saturating_sub(1)),
            build: random.range(-1.0, 1.0),
            seed: (random.unit() * 1e9) as u32,
        }
    }

    pub fn sanitized(self, species: Species) -> Self {
        Self {
            coat: self.coat.min(species.coats().saturating_sub(1)),
            build: if self.build.is_finite() {
                self.build.clamp(-1.0, 1.0)
            } else {
                0.0
            },
            seed: self.seed,
        }
    }
}

/// Names a new creature might be given.
const NAMES: [&str; 40] = [
    "Mochi", "Pip", "Biscuit", "Nori", "Tofu", "Maple", "Pebble", "Clover", "Sprout", "Bean",
    "Juniper", "Pudding", "Miso", "Taro", "Plum", "Fig", "Wren", "Kiwi", "Sesame", "Button",
    "Dumpling", "Hazel", "Olive", "Sunny", "Momo", "Yuzu", "Koji", "Ume", "Basil", "Pickle",
    "Toffee", "Bramble", "Acorn", "Puddle", "Noodle", "Marble", "Cocoa", "Ginger", "Lumi", "Bao",
];

/// A name not already taken in `taken`.
pub fn pick_name(random: &mut Random, taken: &[String]) -> String {
    let free: Vec<&str> = NAMES
        .into_iter()
        .filter(|name| !taken.iter().any(|taken| taken.eq_ignore_ascii_case(name)))
        .collect();
    if free.is_empty() {
        return format!("Pip {}", taken.len() + 1);
    }
    free[((random.unit() * free.len() as f32) as usize).min(free.len() - 1)].to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn species_round_trip_by_key_and_noun() {
        for species in Species::ALL {
            assert_eq!(Species::parse(species.key()), Some(species));
            assert_eq!(Species::parse(species.noun()), Some(species));
            assert_eq!(Species::parse(species.noun_korean()), Some(species));
        }
        assert_eq!(Species::parse("hedge"), Some(Species::Hedgehog));
        assert_eq!(Species::parse("x"), None);
    }

    #[test]
    fn a_new_household_starts_with_three_different_living_creatures() {
        for seed in 0..50 {
            let mut random = Random::new(seed);
            let [first, second, third] = Species::starters(&mut random);
            assert!(first != second && second != third && first != third);
            assert!(![first, second, third].contains(&Species::Ghost));
        }
    }

    #[test]
    fn names_avoid_ones_already_taken() {
        let mut random = Random::new(4);
        let mut taken = Vec::new();
        for _ in 0..NAMES.len() + 2 {
            let name = pick_name(&mut random, &taken);
            assert!(!taken.contains(&name));
            taken.push(name);
        }
    }

    #[test]
    fn rolled_genomes_stay_in_their_species_coats() {
        let mut random = Random::new(9);
        for species in Species::ALL {
            for _ in 0..30 {
                let genome = Genome::roll(species, &mut random);
                assert!(genome.coat < species.coats());
                assert!((-1.0..=1.0).contains(&genome.build));
            }
        }
        let damaged = Genome {
            coat: 200,
            build: f32::NAN,
            seed: 1,
        }
        .sanitized(Species::Fox);
        assert!(damaged.coat < Species::Fox.coats() && damaged.build == 0.0);
    }
}
