use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

use glam::{Mat2, Vec2};

use super::doodle::Path;
use super::{Sketch, World};

/// A dream holds as long as the thought that brought it on.
pub const HOLDS: f64 = 12.0;
pub const SLEEPING: f32 = 0.9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DreamPicture {
    Tower,
    Ball,
    Bird,
    Moon,
    Star,
    Heart,
    Flower,
    Cloud,
    Snowflake,
    Friend,
    Letter,
    Noughts,
    Pumpkin,
    Spiral,
}

impl DreamPicture {
    pub const ALL: [Self; 14] = [
        Self::Tower,
        Self::Ball,
        Self::Bird,
        Self::Moon,
        Self::Star,
        Self::Heart,
        Self::Flower,
        Self::Cloud,
        Self::Snowflake,
        Self::Friend,
        Self::Letter,
        Self::Noughts,
        Self::Pumpkin,
        Self::Spiral,
    ];

    /// Read in word order so the first image in a dream leads its picture.
    pub fn from_line(line: &str, friend_name: Option<&str>) -> Self {
        let line = line.to_lowercase();
        let friend_name = friend_name.map(str::to_lowercase);
        for word in line.split(|character: char| !character.is_alphabetic()) {
            if word.is_empty() {
                continue;
            }
            if friend_name.as_deref() == Some(word) {
                return Self::Friend;
            }
            for (roots, picture) in [
                (&["tower", "block"][..], Self::Tower),
                (&["ball", "fetch"][..], Self::Ball),
                (&["bird", "feather", "song", "wing"][..], Self::Bird),
                (&["moon"][..], Self::Moon),
                (&["star", "wish", "shooting"][..], Self::Star),
                (&["heart", "love"][..], Self::Heart),
                (
                    &["flower", "seed", "garden", "petal", "bloom"][..],
                    Self::Flower,
                ),
                (
                    &["rain", "thunder", "storm", "cloud", "drum"][..],
                    Self::Cloud,
                ),
                (&["snow"][..], Self::Snowflake),
                (&["friend", "ghost"][..], Self::Friend),
                (&["letter", "write", "word"][..], Self::Letter),
                (&["nought", "game"][..], Self::Noughts),
                (&["pumpkin"][..], Self::Pumpkin),
                (&["picture", "draw", "door"][..], Self::Spiral),
            ] {
                if roots.iter().any(|root| word.starts_with(root)) {
                    return picture;
                }
            }
        }
        Self::Moon
    }

    pub fn strokes(self) -> &'static [Vec<Vec2>] {
        static STROKES: OnceLock<[Vec<Vec<Vec2>>; DreamPicture::ALL.len()]> = OnceLock::new();
        &STROKES.get_or_init(|| Self::ALL.map(Self::make_strokes))[self as usize]
    }

    pub fn distance(self, point: Vec2, progress: f32) -> f32 {
        static PATHS: OnceLock<[Path; DreamPicture::ALL.len()]> = OnceLock::new();
        PATHS.get_or_init(|| Self::ALL.map(|picture| Path::new(picture.strokes().to_vec())))
            [self as usize]
            .distance(point, progress)
    }

    fn make_strokes(self) -> Vec<Vec<Vec2>> {
        let line = |points: &[(f32, f32)]| points.iter().map(|&(x, y)| Vec2::new(x, y)).collect();
        match self {
            Self::Heart => Sketch::Heart.strokes(),
            Self::Star => Sketch::Star.strokes(),
            Self::Flower => Sketch::Flower.strokes(),
            Self::Snowflake => Sketch::Snowflake.strokes(),
            Self::Spiral => Sketch::Spiral.strokes(),
            Self::Ball => vec![
                arc(Vec2::ZERO, Vec2::splat(0.8), 0.0, TAU),
                arc(Vec2::ZERO, Vec2::new(0.3, 0.8), -PI * 0.5, PI * 0.5),
            ],
            Self::Bird => vec![
                arc(Vec2::new(0.0, -0.25), Vec2::new(0.2, 0.4), 0.0, TAU),
                arc(Vec2::new(-0.48, 0.05), Vec2::new(0.45, 0.55), 0.0, PI),
                arc(Vec2::new(0.48, 0.05), Vec2::new(0.45, 0.55), 0.0, PI),
            ],
            Self::Moon => {
                let mut crescent = arc(Vec2::ZERO, Vec2::splat(0.85), PI * 0.3, PI * 1.7);
                let tip = crescent[0];
                let bottom = *crescent.last().unwrap();
                crescent.extend((1..=32).map(|index| {
                    let progress = index as f32 / 32.0;
                    bottom.lerp(tip, progress) - Vec2::X * (PI * progress).sin() * 0.65
                }));
                vec![crescent]
            }
            Self::Tower => (0..3)
                .map(|level| {
                    let turn = Mat2::from_angle(if level == 1 { 0.12 } else { -0.05 });
                    let center = Vec2::new(
                        if level == 1 { 0.06 } else { 0.0 },
                        -0.56 + level as f32 * 0.56,
                    );
                    [
                        (-0.3, -0.28),
                        (0.3, -0.28),
                        (0.3, 0.28),
                        (-0.3, 0.28),
                        (-0.3, -0.28),
                    ]
                    .map(|(x, y)| center + turn * Vec2::new(x, y))
                    .to_vec()
                })
                .collect(),
            Self::Cloud => {
                let mut outline: Vec<Vec2> = (0..=64)
                    .map(|index| {
                        let angle = TAU * index as f32 / 64.0;
                        let radius = 1.0 + 0.12 * (5.0 * angle).cos();
                        Vec2::from_angle(angle) * Vec2::new(0.77, 0.36) * radius + Vec2::Y * 0.3
                    })
                    .collect();
                outline.push(outline[0]);
                let mut strokes = vec![outline];
                for x in [-0.5, 0.0, 0.5] {
                    strokes.push(line(&[(x, -0.3), (x - 0.1, -0.75)]));
                }
                strokes
            }
            Self::Friend => {
                let mut outline = arc(Vec2::new(0.0, 0.2), Vec2::splat(0.6), 0.0, PI);
                outline.push(Vec2::new(-0.65, -0.65));
                outline.extend((1..=24).map(|index| {
                    let progress = index as f32 / 24.0;
                    Vec2::new(
                        -0.65 + 1.3 * progress,
                        -0.65 + 0.1 * (3.0 * TAU * progress).sin(),
                    )
                }));
                outline.push(Vec2::new(0.6, 0.2));
                vec![
                    outline,
                    arc(Vec2::new(-0.22, 0.2), Vec2::splat(0.055), 0.0, TAU),
                    arc(Vec2::new(0.22, 0.2), Vec2::splat(0.055), 0.0, TAU),
                ]
            }
            Self::Letter => vec![
                line(&[(-0.7, -0.8), (0.0, 0.8), (0.7, -0.8)]),
                line(&[(-0.4, -0.1), (0.4, -0.1)]),
            ],
            Self::Noughts => vec![
                line(&[(-0.28, -0.9), (-0.28, 0.9)]),
                line(&[(0.28, -0.9), (0.28, 0.9)]),
                line(&[(-0.9, -0.28), (0.9, -0.28)]),
                line(&[(-0.9, 0.28), (0.9, 0.28)]),
                arc(Vec2::new(-0.58, 0.58), Vec2::splat(0.17), 0.0, TAU),
                line(&[(-0.16, -0.16), (0.16, 0.16)]),
                line(&[(-0.16, 0.16), (0.16, -0.16)]),
            ],
            Self::Pumpkin => vec![
                arc(Vec2::new(0.0, -0.1), Vec2::new(0.85, 0.65), 0.0, TAU),
                line(&[(-0.12, 0.53), (-0.08, 0.85), (0.15, 0.88), (0.12, 0.53)]),
                line(&[
                    (-0.5, -0.22),
                    (-0.32, -0.4),
                    (-0.15, -0.22),
                    (0.0, -0.4),
                    (0.18, -0.22),
                    (0.35, -0.4),
                    (0.5, -0.22),
                ]),
            ],
        }
    }
}

fn arc(center: Vec2, radii: Vec2, start: f32, end: f32) -> Vec<Vec2> {
    (0..=32)
        .map(|index| center + Vec2::from_angle(start + (end - start) * index as f32 / 32.0) * radii)
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dream {
    pub picture: DreamPicture,
    pub started: f64,
}

impl Dream {
    pub fn envelope(self, time: f64) -> f32 {
        let elapsed = time - self.started;
        if !(0.0..HOLDS).contains(&elapsed) {
            return 0.0;
        }
        let ease = |amount: f64| {
            let amount = amount.clamp(0.0, 1.0) as f32;
            amount * amount * (3.0 - 2.0 * amount)
        };
        ease(elapsed / 1.5) * ease((HOLDS - elapsed) / 2.0)
    }

    pub fn drawn(self, time: f64) -> f32 {
        ((time - self.started) / 2.0).clamp(0.0, 1.0) as f32
    }
}

impl World {
    pub fn dream_of(&mut self, line: &str) {
        if self.ghost.asleep <= SLEEPING {
            return;
        }
        self.dream = Some(Dream {
            picture: DreamPicture::from_line(
                line,
                self.life.friend.as_ref().map(|friend| friend.name.as_str()),
            ),
            started: self.time,
        });
    }

    pub(super) fn advance_dream(&mut self) {
        if self.dream.is_some_and(|dream| {
            self.ghost.asleep <= SLEEPING || self.time - dream.started >= HOLDS
        }) {
            self.dream = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Activity;

    #[test]
    fn the_first_image_in_a_line_wins_and_unknown_dreams_have_a_moon() {
        for (line, picture) in [
            (
                "a tower of blocks all the way to the moon",
                DreamPicture::Tower,
            ),
            ("bringing the moon back like a ball", DreamPicture::Moon),
            ("fetching a ball past a tower", DreamPicture::Ball),
            ("a feather and a song", DreamPicture::Bird),
            ("wings of a bird", DreamPicture::Bird),
            ("a shooting star and a wish", DreamPicture::Star),
            ("love leaves hearts", DreamPicture::Heart),
            ("seeds and garden petals bloom", DreamPicture::Flower),
            (
                "storm clouds drum with rain and thunder",
                DreamPicture::Cloud,
            ),
            ("snowflakes", DreamPicture::Snowflake),
            ("another ghost and a friend", DreamPicture::Friend),
            ("letters write words", DreamPicture::Letter),
            ("a game of noughts", DreamPicture::Noughts),
            ("pumpkins", DreamPicture::Pumpkin),
            ("draw pictures on a door", DreamPicture::Spiral),
            ("…LUMI and I twirl among the stars…", DreamPicture::Friend),
            ("stars above Lumi", DreamPicture::Star),
            ("…zzz…", DreamPicture::Moon),
            ("", DreamPicture::Moon),
        ] {
            assert_eq!(
                DreamPicture::from_line(line, Some("Lumi")),
                picture,
                "{line}"
            );
        }
    }

    #[test]
    fn dreams_start_replace_each_other_and_expire_without_changing_the_random_source() {
        let mut world = World::new(4, 23.0);
        world.dream_of("ball");
        assert!(world.dream.is_none());
        world.tuck_in();
        world.dream_of("ball");
        assert_eq!(world.dream.unwrap().picture, DreamPicture::Ball);
        world.step(2.0);
        world.dream_of("flower");
        let dream = world.dream.unwrap();
        assert_eq!(dream.picture, DreamPicture::Flower);
        assert_eq!(dream.started, world.time);
        let mut same = world.clone();
        world.dream_of("star");
        assert_eq!(world.random_unit(), same.random_unit());
        world.step(HOLDS as f32 + 0.1);
        assert!(world.dream.is_none());
    }

    #[test]
    fn waking_ends_a_dream_through_interruptions_morning_naps_and_fading_sleep() {
        let mut world = World::new(4, 23.0);
        for waking in 0..4 {
            world.hour = 23.0;
            world.tuck_in();
            world.dream_of("star");
            match waking {
                0 => world.poke(),
                1 => world.hour = 7.0,
                2 => {
                    world.ghost.plan.overnight = false;
                    world.ghost.needs.energy = 1.0;
                }
                _ => world.ghost.plan.done = true,
            }
            if waking == 0 {
                assert!(world.dream.is_none());
            }
            world.step(0.2);
            assert!(world.dream.is_none(), "wake path {waking}");
        }
        world.tuck_in();
        world.dream_of("ball");
        world.ghost.asleep = SLEEPING;
        world.begin(Activity::Float);
        world.step(0.1);
        assert!(world.dream.is_none());
    }

    #[test]
    fn the_envelope_rises_holds_and_fades_while_strokes_draw_in_two_seconds() {
        let dream = Dream {
            picture: DreamPicture::Moon,
            started: 10.0,
        };
        for elapsed in [-1.0, 0.0, HOLDS, HOLDS + 1.0] {
            assert_eq!(dream.envelope(dream.started + elapsed), 0.0);
        }
        assert!((dream.envelope(10.75) - 0.5).abs() < 1e-6);
        for elapsed in [1.5, 5.0, HOLDS - 2.0] {
            assert_eq!(dream.envelope(dream.started + elapsed), 1.0);
        }
        assert!((dream.envelope(21.0) - 0.5).abs() < 1e-6);
        assert_eq!(dream.drawn(10.0), 0.0);
        assert_eq!(dream.drawn(11.0), 0.5);
        assert_eq!(dream.drawn(12.0), 1.0);
    }

    #[test]
    fn every_stroke_is_finite_inside_the_unit_box_and_has_positive_length() {
        for picture in DreamPicture::ALL {
            let strokes = picture.strokes();
            assert!(strokes.iter().all(|stroke| stroke.len() >= 2));
            for point in strokes.iter().flatten() {
                assert!(
                    point.is_finite() && point.abs().max_element() <= 1.0,
                    "{picture:?}: {point}"
                );
            }
            let path = Path::new(strokes.to_vec());
            assert!(path.length().is_finite() && path.length() > 0.0);
            assert!(path.distance(Vec2::ZERO, 1.0).is_finite());
        }
    }
}
