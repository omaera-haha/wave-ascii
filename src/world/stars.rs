use std::sync::OnceLock;

use glam::Vec2;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, LearnedPicture, Random, Sky, World, recognizer};
use crate::sdf::smoothstep;

pub const GRID: Vec2 = Vec2::new(22.0, 16.0);
const STEP_LIMIT: f32 = 0.23;
const SAMPLES: usize = 224;
const KEPT: usize = 12;
// Sparse chains cut corners more than finger drawings; the recognizer's
// usual limit is too strict for those small gaps and wobbles.
const THRESHOLD: f32 = 1.35;
const ADJECTIVES: [(&str, &str); 8] = [
    ("Little", "작은"),
    ("Sleepy", "졸린"),
    ("Great", "커다란"),
    ("Shy", "수줍은"),
    ("Dancing", "춤추는"),
    ("Lost", "길 잃은"),
    ("Bright", "밝은"),
    ("Quiet", "조용한"),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Star {
    pub position: Vec2,
    pub brightness: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Constellation {
    pub name: String,
    pub korean: String,
    pub stars: Vec<u16>,
    /// Local date of discovery, as YYYY-MM-DD.
    pub found: String,
}

impl Constellation {
    pub(super) fn valid(&self) -> bool {
        let named = |name: &str| {
            !name.trim().is_empty()
                && name.chars().count() <= 128
                && !name.chars().any(char::is_control)
        };
        named(&self.name)
            && named(&self.korean)
            && self.found.parse::<Date>().is_ok()
            && (3..=7).contains(&self.stars.len())
            && self.stars.iter().enumerate().all(|(index, &star)| {
                usize::from(star) < field().len() && !self.stars[..index].contains(&star)
            })
    }
}

pub fn hash(cell: Vec2) -> f32 {
    let cell = Vec2::new(cell.x.rem_euclid(64.0), cell.y.rem_euclid(64.0));
    (cell.dot(Vec2::new(127.1, 311.7)).sin() * 43_758.547)
        .fract()
        .abs()
}

pub fn center(cell: Vec2) -> Vec2 {
    Vec2::new(hash(cell + Vec2::X), hash(cell + Vec2::Y)) * 0.6 + 0.2
}

pub fn brightness(seed: f32) -> f32 {
    smoothstep(0.78, 0.9, seed)
}

/// The bright, unobscured stars, in a fixed order used by saved pictures.
pub fn field() -> &'static [Star] {
    static FIELD: OnceLock<Vec<Star>> = OnceLock::new();
    FIELD.get_or_init(|| {
        let mut stars = Vec::new();
        for row in 0..16 {
            for column in 0..22 {
                let cell = Vec2::new(column as f32, row as f32);
                let position = (cell + center(cell)) / GRID;
                let brightness = brightness(hash(cell));
                // The highest hill is below this line. Keep the whole leafy
                // tree's bounds clear even when winter exposes its branches.
                let tree = (0.08..0.43).contains(&position.x) && position.y < 0.6;
                let moon =
                    ((position - Vec2::new(0.79, 0.8)) * Vec2::new(1.0, 2.0 / 3.0)).length() < 0.06;
                if brightness >= 0.35 && position.y > 0.26 && !tree && !moon {
                    stars.push(Star {
                        position,
                        brightness,
                    });
                }
            }
        }
        stars
    })
}

fn neighbors() -> &'static [Vec<usize>] {
    static NEIGHBORS: OnceLock<Vec<Vec<usize>>> = OnceLock::new();
    NEIGHBORS.get_or_init(|| {
        field()
            .iter()
            .enumerate()
            .map(|(index, star)| {
                let mut near: Vec<usize> = field()
                    .iter()
                    .enumerate()
                    .filter(|(other, candidate)| {
                        *other != index && star.position.distance(candidate.position) <= STEP_LIMIT
                    })
                    .map(|(other, _)| other)
                    .collect();
                near.sort_by(|&first, &second| {
                    star.position
                        .distance_squared(field()[first].position)
                        .total_cmp(&star.position.distance_squared(field()[second].position))
                });
                near
            })
            .collect()
    })
}

/// Searches a bounded set of nearby-star chains without changing the world.
pub fn find(
    random: &mut Random,
    existing: &[Constellation],
    learned: &[LearnedPicture],
) -> Option<Constellation> {
    let mut best = THRESHOLD;
    let mut found = None;
    for _ in 0..SAMPLES {
        let length = 4 + (random.unit() * 4.0) as usize;
        let mut chain = vec![(random.unit() * field().len() as f32) as usize];
        while chain.len() < length {
            let nearest: Vec<usize> = neighbors()[*chain.last().unwrap()]
                .iter()
                .copied()
                .filter(|index| !chain.contains(index))
                .take(3)
                .collect();
            if nearest.is_empty() {
                break;
            }
            chain.push(nearest[(random.unit() * nearest.len() as f32) as usize]);
        }
        if chain.len() < 4
            || existing.iter().any(|known| {
                chain
                    .iter()
                    .filter(|&&index| known.stars.contains(&(index as u16)))
                    .count()
                    > 1
            })
        {
            continue;
        }
        let stroke = chain.iter().map(|&index| field()[index].position).collect();
        if let Some((recognition, distance)) = recognizer::best_match(&[stroke], learned, best)
            && distance < best
        {
            best = distance;
            found = Some((chain, recognition));
        }
    }
    found.map(|(chain, recognition)| {
        let (adjective, korean) = ADJECTIVES[(random.unit() * ADJECTIVES.len() as f32) as usize];
        let thing = if recognition.learned {
            recognition.name
        } else {
            recognition
                .name
                .split_whitespace()
                .map(|word| {
                    let mut letters = word.chars();
                    letters.next().unwrap().to_uppercase().collect::<String>() + letters.as_str()
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        Constellation {
            name: format!("the {adjective} {thing}"),
            korean: format!("{korean} {}", recognition.korean),
            stars: chain.into_iter().map(|index| index as u16).collect(),
            found: String::new(),
        }
    })
}

impl World {
    pub fn starry_night(&self) -> bool {
        super::meteor::dark(self.hour)
            && self.weather.sky == Sky::Clear
            && !self.weather.storm
            && self.weather.rain(self.time) <= 0.0
            && self.weather.snow(self.time) <= 0.0
    }

    fn star_night(&self) -> Date {
        if self.hour < 12.0 {
            self.date.yesterday().unwrap_or(self.date)
        } else {
            self.date
        }
    }

    pub fn constellation_pending(&self) -> bool {
        self.starry_night()
            && self.life.constellation_night.as_deref()
                != Some(self.star_night().to_string().as_str())
    }

    pub(super) fn find_constellation(&mut self) {
        let plan = &self.ghost.plan;
        if plan.activity != Activity::Window
            || plan.done
            || plan.count != 0
            || self.ghost.asleep >= 0.3
            || plan.stayed(self.time) < 20.0
            || !self.constellation_pending()
        {
            return;
        }
        self.life.constellation_night = Some(self.star_night().to_string());
        let curiosity = self.life.temperament().curious;
        if self.random.unit() >= 0.65 + curiosity * 0.3 {
            return;
        }
        let Some(mut constellation) = find(
            &mut self.random,
            &self.life.constellations,
            &self.life.pictures,
        ) else {
            self.think("The stars won't make a picture tonight.");
            return;
        };
        constellation.found = self.date.to_string();
        let english = format!("I found a picture in the stars! {}.", constellation.name);
        let korean = format!("별들이 그림이 됐어! {} 별자리야.", constellation.korean);
        let name = constellation.name.clone();
        self.keep_constellation(&constellation);
        self.life.constellations.push(constellation);
        if self.life.constellations.len() > KEPT {
            self.life.constellations.remove(0);
        }
        self.found_constellation_at = Some(self.time);
        self.record(EventKind::FoundConstellation(name));
        self.think(&english);
        if self.viewer_present() {
            self.say(if self.speaks_korean() {
                &korean
            } else {
                &english
            });
        }
    }

    pub fn constellation_glow(&self) -> f32 {
        let plan = &self.ghost.plan;
        let watching = if self.starry_night()
            && plan.activity == Activity::Window
            && !plan.done
            && plan.count == 0
            && self.ghost.asleep < 0.3
        {
            smoothstep(0.0, 3.0, plan.stayed(self.time))
        } else {
            0.0
        };
        let recent = self.found_constellation_at.map_or(0.0, |time| {
            1.0 - smoothstep(0.0, 30.0, (self.time - time) as f32)
        });
        watching.max(recent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Nature, WINDOW_CENTER};

    fn watching(seed: u64) -> World {
        let mut world = World::new(seed, 21.0);
        world.weather.sky = Sky::Clear;
        world.weather.storm = false;
        world.weather.until = 10_000.0;
        world.life.nature = Some(Nature {
            curious: 1.0,
            ..Nature::default()
        });
        world.begin(Activity::Window);
        world.ghost.position = WINDOW_CENTER + glam::Vec3::new(0.0, -0.05, 0.55);
        world.ghost.plan.arrived = Some(0.0);
        world.time = 20.1;
        world
    }

    #[test]
    fn every_cell_keeps_the_original_star_placement_and_brightness() {
        let old_hash = |cell: Vec2| {
            let cell = Vec2::new(cell.x.rem_euclid(64.0), cell.y.rem_euclid(64.0));
            (cell.dot(Vec2::new(127.1, 311.7)).sin() * 43_758.547)
                .fract()
                .abs()
        };
        for row in 0..16 {
            for column in 0..22 {
                let cell = Vec2::new(column as f32, row as f32);
                let old_center =
                    Vec2::new(old_hash(cell + Vec2::X), old_hash(cell + Vec2::Y)) * 0.6 + 0.2;
                assert_eq!(hash(cell), old_hash(cell));
                assert_eq!(center(cell), old_center);
                assert_eq!(
                    brightness(hash(cell)),
                    smoothstep(0.78, 0.9, old_hash(cell))
                );
            }
        }
    }

    #[test]
    fn the_cached_field_is_stable_and_stays_above_the_hills_and_clear_of_the_tree_and_moon() {
        assert!(std::ptr::eq(field(), field()));
        assert!(field().len() >= 30, "{} stars", field().len());
        for star in field() {
            assert!(star.position.is_finite());
            assert!(star.position.cmpge(Vec2::ZERO).all() && star.position.cmple(Vec2::ONE).all());
            assert!(star.brightness >= 0.35 && star.brightness <= 1.0);
            let hill = 0.215
                + 0.025 * (star.position.x * 8.0 + 0.6).sin()
                + 0.012 * (star.position.x * 19.0).sin();
            assert!(star.position.y > hill);
            assert!(!(0.08..0.43).contains(&star.position.x) || star.position.y >= 0.6);
            assert!(
                ((star.position - Vec2::new(0.79, 0.8)) * Vec2::new(1.0, 2.0 / 3.0)).length()
                    >= 0.06
            );
        }
    }

    #[test]
    fn seeded_searches_find_distinct_short_chains_on_most_nights_and_keep_known_stars_apart() {
        let mut successes = 0;
        for seed in 0..16 {
            let first = find(&mut Random::new(seed), &[], &[]);
            assert_eq!(first, find(&mut Random::new(seed), &[], &[]));
            let Some(first) = first else { continue };
            successes += 1;
            assert!((4..=7).contains(&first.stars.len()));
            assert!(
                ADJECTIVES
                    .iter()
                    .any(|(adjective, _)| first.name.starts_with(&format!("the {adjective} ")))
            );
            assert!(!first.korean.is_empty());
            for (index, &star) in first.stars.iter().enumerate() {
                assert!(!first.stars[..index].contains(&star));
            }
            for pair in first.stars.windows(2) {
                assert!(
                    field()[usize::from(pair[0])]
                        .position
                        .distance(field()[usize::from(pair[1])].position)
                        <= STEP_LIMIT
                );
            }
            if let Some(second) = find(
                &mut Random::new(seed + 50),
                std::slice::from_ref(&first),
                &[],
            ) {
                assert!(
                    second
                        .stars
                        .iter()
                        .filter(|star| first.stars.contains(star))
                        .count()
                        <= 1
                );
            }
        }
        assert!(successes >= 13, "{successes} successful nights");
    }

    #[test]
    fn sparse_jittered_template_chains_match_more_readily_than_random_chains() {
        let mut random = Random::new(813);
        let mut shaped = 0;
        let mut unstructured = 0;
        let mut strict = 0;
        for _ in 0..64 {
            let jitter = |random: &mut Random| {
                Vec2::new(random.range(-0.18, 0.18), random.range(-0.18, 0.18))
            };
            let chain = vec![
                Vec2::new(0.0, 1.0) + jitter(&mut random),
                Vec2::new(-0.5, 0.0) + jitter(&mut random),
                Vec2::new(-1.0, -1.0) + jitter(&mut random),
                Vec2::new(0.0, -1.0) + jitter(&mut random),
                Vec2::new(1.0, -1.0) + jitter(&mut random),
                Vec2::new(0.5, 0.0) + jitter(&mut random),
                Vec2::new(0.0, 1.0) + jitter(&mut random),
            ];
            strict +=
                usize::from(recognizer::recognize(std::slice::from_ref(&chain), &[]).is_some());
            shaped += usize::from(recognizer::best_match(&[chain], &[], THRESHOLD).is_some());
            let chain = (0..7)
                .map(|_| Vec2::new(random.unit(), random.unit()))
                .collect();
            unstructured += usize::from(recognizer::best_match(&[chain], &[], THRESHOLD).is_some());
        }
        assert!(shaped > strict, "strict {strict}, loose {shaped}");
        assert!(shaped >= 60, "{shaped} jittered templates recognized");
        assert!(unstructured < 32, "{unstructured} random chains recognized");
    }

    #[test]
    fn a_learned_star_picture_keeps_the_viewers_name_in_both_languages() {
        let first = find(&mut Random::new(91), &[], &[]).unwrap();
        let stroke = first
            .stars
            .iter()
            .map(|&index| field()[usize::from(index)].position)
            .collect();
        let learned = LearnedPicture {
            name: "little dragon".to_owned(),
            strokes: recognizer::normalized(&[stroke]),
        };
        let found = find(&mut Random::new(91), &[], &[learned]).unwrap();
        assert!(found.name.ends_with("little dragon"));
        assert!(found.korean.ends_with("little dragon"));
    }

    #[test]
    fn gazing_finds_at_most_one_picture_per_night_and_records_its_life() {
        let mut world = watching(7);
        world.find_constellation();
        assert_eq!(world.life.constellations.len(), 1);
        let first = world.life.constellations[0].clone();
        assert_eq!(first.found, world.date.to_string());
        assert_eq!(world.life.total.constellations, 1);
        assert_eq!(world.life.today.constellations, 1);
        assert!(world.life.today.words().contains("picture in the stars"));
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::FoundConstellation(first.name.clone()))
        );
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text
                    == format!("I found my first constellation: {}.", first.name))
        );
        assert!(
            world
                .ghost
                .thought
                .as_ref()
                .unwrap()
                .text
                .contains(&first.name)
        );
        for hour in [21.5, 23.5, 0.5, 4.5] {
            if hour < 5.0 && world.date == Date::constant(2026, 6, 21) {
                world.date = world.date.tomorrow().unwrap();
            }
            world.hour = hour;
            world.find_constellation();
            assert_eq!(world.life.total.constellations, 1);
        }
        world.hour = 21.0;
        assert!(world.constellation_pending());
        world.find_constellation();
        assert!(!world.constellation_pending());
        assert!(world.life.total.constellations <= 2);
        let mut reopened = watching(7);
        reopened.life = world.life.clone();
        reopened.date = world.date;
        assert!(!reopened.constellation_pending());
    }

    #[test]
    fn discovery_waits_for_dark_clear_awake_and_twenty_seconds_of_settled_gazing() {
        for condition in 0..9 {
            let mut world = watching(7);
            match condition {
                0 => world.hour = 12.0,
                1 => world.weather.sky = Sky::Rain,
                2 => world.weather.sky = Sky::Snow,
                3 => world.weather.storm = true,
                4 => world.ghost.asleep = 1.0,
                5 => world.time = 19.9,
                6 => world.ghost.plan.arrived = None,
                7 => world.ghost.plan.activity = Activity::Rest,
                _ => world.ghost.plan.count = 1,
            }
            world.find_constellation();
            assert!(
                world.life.constellations.is_empty(),
                "condition {condition}"
            );
            assert!(world.life.constellation_night.is_none());
        }
        let mut world = watching(7);
        world.step(0.1);
        assert_eq!(world.life.total.constellations, 1);
    }

    #[test]
    fn curiosity_helps_discovery_and_a_failed_chance_uses_up_the_night_even_after_reopening() {
        let mut world = watching(7);
        world.random = Random::new(0);
        world.life.nature.as_mut().unwrap().curious = 0.0;
        world.find_constellation();
        assert_eq!(world.life.total.constellations, 0);
        assert!(!world.constellation_pending());
        let saved = serde_json::to_string(&world.life).unwrap();
        let mut reopened = watching(7);
        reopened.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
        assert!(!reopened.constellation_pending());
        reopened.find_constellation();
        assert_eq!(reopened.life.total.constellations, 0);
        let mut curious = watching(7);
        curious.random = Random::new(0);
        curious.find_constellation();
        assert_eq!(curious.life.total.constellations, 1);
    }

    #[test]
    fn constellation_speech_follows_the_viewers_language_and_needs_a_present_viewer() {
        for korean in [false, true] {
            let mut world = watching(7);
            world.life.korean = if korean { 1.0 } else { 0.0 };
            world.viewer.active_at = world.time;
            world.find_constellation();
            let constellation = world.life.constellations.last().unwrap();
            let speech = &world.ghost.speech.as_ref().unwrap().text;
            assert!(speech.contains(if korean {
                &constellation.korean
            } else {
                &constellation.name
            }));
            assert!(
                world
                    .ghost
                    .thought
                    .as_ref()
                    .unwrap()
                    .text
                    .contains(&constellation.name)
            );
        }
        let mut absent = watching(7);
        absent.viewer.active_at = -1000.0;
        absent.find_constellation();
        assert!(absent.ghost.speech.is_none());
    }

    #[test]
    fn saved_star_pictures_round_trip_and_damaged_entries_are_dropped() {
        let valid = Constellation {
            name: "the Little Heart".to_owned(),
            korean: "작은 하트".to_owned(),
            stars: vec![0, 1, 2, 3],
            found: "2026-06-21".to_owned(),
        };
        let mut life = Life {
            constellations: vec![valid.clone()],
            constellation_night: Some("2026-06-21".to_owned()),
            ..Life::default()
        };
        let restored: Life = serde_json::from_str(&serde_json::to_string(&life).unwrap()).unwrap();
        assert_eq!(restored.sanitized(), life);
        for invalid in [
            Constellation {
                stars: vec![0, 1],
                ..valid.clone()
            },
            Constellation {
                stars: vec![0, 1, 1],
                ..valid.clone()
            },
            Constellation {
                stars: vec![0, 1, u16::MAX],
                ..valid.clone()
            },
            Constellation {
                name: " ".to_owned(),
                ..valid.clone()
            },
            Constellation {
                korean: String::new(),
                ..valid.clone()
            },
            Constellation {
                found: "tomorrow".to_owned(),
                ..valid.clone()
            },
        ] {
            life.constellations.push(invalid);
        }
        assert_eq!(life.sanitized().constellations, vec![valid.clone()]);
        let life = Life {
            constellations: (0..15)
                .map(|index| Constellation {
                    name: format!("{index}"),
                    ..valid.clone()
                })
                .collect(),
            ..Life::default()
        }
        .sanitized();
        assert_eq!(life.constellations.len(), 12);
        assert_eq!(life.constellations[0].name, "3");
        let old: Life = serde_json::from_str("{}").unwrap();
        assert!(old.constellations.is_empty() && old.constellation_night.is_none());
    }
}
