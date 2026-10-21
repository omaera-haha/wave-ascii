use std::f32::consts::{PI, TAU};
use std::sync::{Arc, OnceLock};

use glam::Vec2;

/// A little picture drawn in a few strokes in its own -1 to 1 space. The
/// pen lifts between strokes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sketch {
    Heart,
    Star,
    Smile,
    Spiral,
    Sun,
    Flower,
    Wave,
    Snowflake,
}

impl Sketch {
    pub const ALL: [Self; 8] = [
        Self::Heart,
        Self::Star,
        Self::Smile,
        Self::Spiral,
        Self::Sun,
        Self::Flower,
        Self::Wave,
        Self::Snowflake,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Heart => "heart",
            Self::Star => "star",
            Self::Smile => "smile",
            Self::Spiral => "spiral",
            Self::Sun => "sun",
            Self::Flower => "flower",
            Self::Wave => "wave",
            Self::Snowflake => "snowflake",
        }
    }

    /// The length of the whole stroke.
    pub fn length(self) -> f32 {
        self.path().length
    }

    /// The pen's position after drawing `progress` of the strokes. Lifting
    /// the pen between strokes takes no time.
    pub fn point_at(self, progress: f32) -> Vec2 {
        self.path().point_at(progress)
    }

    /// Distance to the strokes drawn so far.
    pub fn distance(self, point: Vec2, progress: f32) -> f32 {
        self.path().distance(point, progress)
    }

    fn path(self) -> &'static Path {
        static PATHS: OnceLock<[Path; Sketch::ALL.len()]> = OnceLock::new();
        let paths = PATHS.get_or_init(|| Sketch::ALL.map(|sketch| Path::new(sketch.strokes())));
        &paths[self as usize]
    }

    fn strokes(self) -> Vec<Vec<Vec2>> {
        match self {
            Self::Heart => vec![
                (0..=64)
                    .map(|index| {
                        let angle = TAU * index as f32 / 64.0;
                        Vec2::new(
                            16.0 * angle.sin().powi(3),
                            13.0 * angle.cos()
                                - 5.0 * (2.0 * angle).cos()
                                - 2.0 * (3.0 * angle).cos()
                                - (4.0 * angle).cos(),
                        ) / 18.0
                    })
                    .collect(),
            ],
            Self::Star => vec![
                (0..=10)
                    .map(|index| {
                        let radius = if index % 2 == 0 { 0.9 } else { 0.38 };
                        Vec2::from_angle(PI * 0.5 + TAU * index as f32 / 10.0) * radius
                    })
                    .collect(),
            ],
            Self::Smile => {
                let face = (0..=48)
                    .map(|index| Vec2::from_angle(PI * 0.5 + TAU * index as f32 / 48.0) * 0.9)
                    .collect();
                let eye = |center: Vec2| {
                    (0..=12)
                        .map(|index| center + Vec2::from_angle(TAU * index as f32 / 12.0) * 0.06)
                        .collect()
                };
                let mouth = (0..=24)
                    .map(|index| {
                        Vec2::from_angle(PI + PI * index as f32 / 24.0) * Vec2::new(0.55, 0.45)
                            - Vec2::Y * 0.1
                    })
                    .collect();
                vec![
                    face,
                    eye(Vec2::new(-0.3, 0.25)),
                    eye(Vec2::new(0.3, 0.25)),
                    mouth,
                ]
            }
            Self::Spiral => vec![
                (0..=96)
                    .map(|index| {
                        let progress = index as f32 / 96.0;
                        Vec2::from_angle(TAU * 2.5 * progress) * (0.04 + 0.86 * progress)
                    })
                    .collect(),
            ],
            Self::Sun => {
                let mut points = Vec::new();
                for index in 0..=48 {
                    let direction = Vec2::from_angle(TAU * index as f32 / 48.0);
                    points.push(direction * 0.48);
                    if index % 6 == 0 && index < 48 {
                        points.push(direction * 0.9);
                        points.push(direction * 0.48);
                    }
                }
                vec![points]
            }
            Self::Flower => {
                let petals = (0..=80)
                    .map(|index| {
                        let angle = -PI * 0.5 + TAU * index as f32 / 80.0;
                        Vec2::from_angle(angle) * (0.46 + 0.2 * (5.0 * angle).sin()) + Vec2::Y * 0.2
                    })
                    .collect();
                let stem = vec![Vec2::new(0.0, -0.06), Vec2::new(0.0, -0.85)];
                let leaf = vec![
                    Vec2::new(0.0, -0.45),
                    Vec2::new(0.4, -0.3),
                    Vec2::new(0.25, -0.6),
                    Vec2::new(0.0, -0.65),
                ];
                vec![petals, stem, leaf]
            }
            Self::Wave => vec![
                (0..=48)
                    .map(|index| {
                        let progress = index as f32 / 48.0;
                        Vec2::new(-0.9 + 1.8 * progress, 0.45 * (TAU * 1.5 * progress).sin())
                    })
                    .collect(),
            ],
            Self::Snowflake => {
                let mut points = Vec::new();
                for spoke in 0..6 {
                    let direction = Vec2::from_angle(PI * 0.5 + TAU * spoke as f32 / 6.0);
                    let joint = direction * 0.5;
                    points.push(Vec2::ZERO);
                    for side in [1.0, -1.0] {
                        points.push(joint);
                        points.push(
                            joint + direction.rotate(Vec2::from_angle(side * PI / 3.0)) * 0.28,
                        );
                    }
                    points.push(joint);
                    points.push(direction * 0.9);
                }
                points.push(Vec2::ZERO);
                vec![points]
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub(super) struct Path {
    strokes: Vec<Vec<Vec2>>,
    segments: Vec<Segment>,
    length: f32,
    extent: Vec2,
}

#[derive(Debug, PartialEq)]
struct Segment {
    start: Vec2,
    along: Vec2,
    length: f32,
    squared: f32,
}

impl Path {
    pub(super) fn new(strokes: Vec<Vec<Vec2>>) -> Self {
        // Measure only drawn segments, leaving out jumps between strokes.
        let segments: Vec<Segment> = strokes
            .iter()
            .flat_map(|stroke| stroke.windows(2))
            .map(|pair| {
                let along = pair[1] - pair[0];
                Segment {
                    start: pair[0],
                    along,
                    length: along.length(),
                    squared: along.length_squared(),
                }
            })
            .filter(|segment| segment.length > 0.0)
            .collect();
        let extent = strokes
            .iter()
            .flatten()
            .fold(Vec2::ZERO, |extent, point| extent.max(point.abs()));
        Self {
            length: segments.iter().map(|segment| segment.length).sum(),
            strokes,
            segments,
            extent,
        }
    }

    pub(super) fn point_at(&self, progress: f32) -> Vec2 {
        if progress >= 1.0 {
            return self.last();
        }
        let mut remaining = self.length * progress.clamp(0.0, 1.0);
        for segment in &self.segments {
            if remaining < segment.length {
                return segment.start + segment.along * (remaining / segment.length);
            }
            remaining -= segment.length;
        }
        self.last()
    }

    pub(super) fn distance(&self, point: Vec2, progress: f32) -> f32 {
        if progress <= 0.0 {
            return f32::INFINITY;
        }
        let mut remaining = self.length * progress.min(1.0);
        let mut nearest = f32::INFINITY;
        for segment in &self.segments {
            let drawn = (remaining / segment.length).min(1.0);
            let fraction =
                ((point - segment.start).dot(segment.along) / segment.squared).clamp(0.0, drawn);
            nearest = nearest.min(point.distance_squared(segment.start + segment.along * fraction));
            remaining -= segment.length;
            if remaining <= 0.0 {
                break;
            }
        }
        nearest.sqrt()
    }

    fn last(&self) -> Vec2 {
        *self
            .strokes
            .last()
            .and_then(|stroke| stroke.last())
            .unwrap()
    }
}

/// A word, measured once and shared when a plan or doodle is cloned.
#[derive(Clone, Debug, PartialEq)]
pub struct Writing {
    pub text: String,
    pub mirrored: bool,
    path: Arc<Path>,
    draw_time: f64,
}

impl Writing {
    pub fn new(text: String, mirrored: bool, level: f32, random: &mut super::Random) -> Self {
        let characters = text.chars().count();
        let width = writing_width(&text);
        let scale = 0.2_f32.min(1.6 / width);
        let mut offset = -width * 0.5;
        let mut strokes: Vec<Vec<Vec2>> = Vec::new();
        for character in text.chars() {
            for stroke in super::letters::strokes(character) {
                strokes.push(
                    stroke
                        .into_iter()
                        .map(|(horizontal, vertical)| {
                            let point = Vec2::new(horizontal + offset, vertical - 0.5) * scale;
                            Vec2::new(if mirrored { -point.x } else { point.x }, point.y)
                        })
                        .collect(),
                );
            }
            offset += super::letters::cell(character).1;
        }
        if level < 1.0 {
            for point in strokes.iter_mut().flatten() {
                let noise = Vec2::new(random.range(-1.0, 1.0), random.range(-1.0, 1.0));
                *point += writing_wobble(level, noise) * scale;
            }
        }
        Self {
            text,
            mirrored,
            path: Arc::new(Path::new(strokes)),
            draw_time: (characters as f64 * 0.7).max(Doodle::DRAW_TIME),
        }
    }
}

fn writing_wobble(level: f32, noise: Vec2) -> Vec2 {
    noise.clamp_length_max(1.0) * (0.04 * (1.0 - level))
}

#[derive(Clone, Debug, PartialEq)]
pub enum Picture {
    Sketch(Sketch),
    Word(Writing),
}

impl Picture {
    pub fn name(&self) -> String {
        match self {
            Self::Sketch(sketch) => sketch.name().to_owned(),
            Self::Word(writing) => format!(
                "“{}”{}",
                writing_words(&writing.text),
                if writing.mirrored {
                    ", the wrong way round for the viewer"
                } else {
                    ""
                }
            ),
        }
    }

    pub fn length(&self) -> f32 {
        match self {
            Self::Sketch(sketch) => sketch.length(),
            Self::Word(writing) => writing.path.length,
        }
    }

    pub fn point_at(&self, progress: f32) -> Vec2 {
        match self {
            Self::Sketch(sketch) => sketch.point_at(progress),
            Self::Word(writing) => writing.path.point_at(progress),
        }
    }

    pub fn distance(&self, point: Vec2, progress: f32) -> f32 {
        match self {
            Self::Sketch(sketch) => sketch.distance(point, progress),
            Self::Word(writing) => writing.path.distance(point, progress),
        }
    }

    pub fn extent(&self) -> Vec2 {
        match self {
            Self::Sketch(_) => Vec2::ONE,
            Self::Word(writing) => writing.path.extent,
        }
    }

    pub fn size(&self) -> f32 {
        match self {
            Self::Sketch(_) => 0.3,
            Self::Word(_) => 1.0,
        }
    }

    pub fn draw_time(&self) -> f64 {
        match self {
            Self::Sketch(_) => Doodle::DRAW_TIME,
            Self::Word(writing) => writing.draw_time,
        }
    }
}

/// Only the supported characters reach the pen.
pub(super) fn sanitize(text: &str) -> String {
    let mut syllables = 0;
    text.to_uppercase()
        .chars()
        .filter(|character| {
            if super::letters::decompose(*character).is_some() {
                syllables += 1;
                return syllables <= 5;
            }
            character.is_ascii_uppercase()
                || character.is_ascii_digit()
                || matches!(character, '!' | '?')
        })
        .take(8)
        .collect()
}

pub(super) fn writing_width(text: &str) -> f32 {
    let mut width = 0.0;
    let mut spacing = 0.0;
    for character in text.chars() {
        let (cell, advance) = super::letters::cell(character);
        width += advance;
        spacing = advance - cell;
    }
    (width - spacing).max(0.6)
}

/// Digits on the glass reach the minds as spoken words.
pub(super) fn writing_words(text: &str) -> String {
    const DIGITS: [&str; 10] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    if !text.chars().any(|character| character.is_ascii_digit()) {
        return text.to_owned();
    }
    text.chars()
        .map(|character| {
            character.to_digit(10).map_or_else(
                || character.to_string(),
                |digit| DIGITS[digit as usize].to_owned(),
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A picture in the fog on the glass, drawn over time and left to fade.
#[derive(Clone, Debug, PartialEq)]
pub struct Doodle {
    pub picture: Picture,
    /// The center's x and y coordinates on the glass.
    pub center: Vec2,
    pub size: f32,
    pub started: f64,
}

impl Doodle {
    pub const DRAW_TIME: f64 = 4.0;
    pub const LIFETIME: f64 = Self::DRAW_TIME + 150.0;

    /// How much of the stroke has been drawn at `time`.
    pub fn drawn(&self, time: f64) -> f32 {
        ((time - self.started) / self.picture.draw_time()).clamp(0.0, 1.0) as f32
    }

    /// How much of the fog remains, fading over its last forty seconds.
    pub fn opacity(&self, time: f64) -> f32 {
        ((Self::LIFETIME + self.picture.draw_time() - Self::DRAW_TIME - (time - self.started))
            / 40.0)
            .clamp(0.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writing_wobble_is_bounded_and_vanishes_with_practice() {
        for noise in [Vec2::ZERO, Vec2::X, Vec2::Y, Vec2::ONE, -Vec2::ONE] {
            assert_eq!(writing_wobble(1.0, noise), Vec2::ZERO);
            assert!(writing_wobble(0.0, noise).length() <= 0.040001);
            assert!(writing_wobble(0.5, noise).length() <= 0.020001);
        }
        let mut first = super::super::Random::new(7);
        let mut second = super::super::Random::new(7);
        let clumsy = Writing::new("HELLO".to_owned(), false, 0.0, &mut first);
        assert_eq!(
            clumsy,
            Writing::new("HELLO".to_owned(), false, 0.0, &mut second)
        );
        let steady = Writing::new("HELLO".to_owned(), false, 1.0, &mut second);
        assert_ne!(clumsy, steady);
        for (shaky, straight) in clumsy
            .path
            .strokes
            .iter()
            .flatten()
            .zip(steady.path.strokes.iter().flatten())
        {
            assert!(shaky.distance(*straight) <= 0.008001);
        }
    }

    #[test]
    fn every_sketch_measures_its_path_and_keeps_it_within_its_own_space() {
        for sketch in Sketch::ALL {
            let path = sketch.path();
            assert_eq!(sketch.point_at(0.0), path.strokes[0][0]);
            assert_eq!(sketch.point_at(1.0), path.last());
            let length: f32 = path
                .strokes
                .iter()
                .flat_map(|stroke| stroke.windows(2))
                .map(|pair| pair[0].distance(pair[1]))
                .sum();
            assert!((path.length - length).abs() < 1e-5);
            assert!(path.strokes.iter().all(|stroke| stroke.len() >= 2));
            for &point in path.strokes.iter().flatten() {
                assert!(point.abs().cmple(Vec2::ONE).all(), "{sketch:?}: {point}");
                assert!(sketch.distance(point, 1.0) < 1e-5, "{sketch:?}: {point}");
            }
            for step in 0..=20 {
                let progress = step as f32 / 20.0;
                let point = sketch.point_at(progress);
                assert!(sketch.distance(point, 1.0) < 1e-5, "{sketch:?}: {progress}");
                if progress > 0.0 {
                    assert!(sketch.distance(point, progress) < 1e-5);
                }
            }
        }
    }

    #[test]
    fn the_pen_lifts_between_strokes_without_drawing_the_jump() {
        let strokes = Sketch::Smile.path().strokes.clone();
        assert_eq!(strokes.len(), 4);
        // Halfway between the face and the first eye lies nothing drawn.
        let jump = strokes[0].last().unwrap().lerp(strokes[1][0], 0.5);
        assert!(Sketch::Smile.distance(jump, 1.0) > 0.05);
    }

    #[test]
    fn a_partly_drawn_path_is_a_subset_of_every_later_stroke() {
        for sketch in Sketch::ALL {
            for column in -5..=5 {
                for row in -5..=5 {
                    let point = Vec2::new(column as f32, row as f32) * 0.2;
                    assert_eq!(sketch.distance(point, 0.0), f32::INFINITY);
                    let mut previous = f32::INFINITY;
                    for step in 1..=10 {
                        let distance = sketch.distance(point, step as f32 / 10.0);
                        assert!(distance <= previous + 1e-5, "{sketch:?}: {point}");
                        previous = distance;
                    }
                }
            }
        }
    }

    #[test]
    fn a_doodle_is_drawn_before_its_fog_fades_away() {
        let doodle = Doodle {
            picture: Picture::Sketch(Sketch::Heart),
            center: Vec2::ZERO,
            size: 0.3,
            started: 10.0,
        };
        assert_eq!(doodle.drawn(9.0), 0.0);
        assert_eq!(doodle.drawn(10.0), 0.0);
        assert_eq!(doodle.drawn(12.0), 0.5);
        assert_eq!(doodle.drawn(14.0), 1.0);
        assert_eq!(doodle.drawn(200.0), 1.0);
        assert_eq!(doodle.opacity(14.0), 1.0);
        assert!((0.0..1.0).contains(&doodle.opacity(10.0 + Doodle::LIFETIME - 20.0)));
        assert_eq!(doodle.opacity(10.0 + Doodle::LIFETIME), 0.0);
        assert_eq!(doodle.opacity(200.0), 0.0);
    }

    #[test]
    fn words_fit_the_glass_and_mirroring_reflects_every_stroke() {
        for text in [
            "HI",
            "MINA♥",
            "2027",
            "ABCDEFGHIJKLMNOP",
            "민수♥",
            "HI한글!?",
            "가나다라마",
        ] {
            let forward = Writing::new(
                text.to_owned(),
                false,
                1.0,
                &mut super::super::Random::new(1),
            );
            let mirrored = Writing::new(
                text.to_owned(),
                true,
                1.0,
                &mut super::super::Random::new(1),
            );
            assert!(forward.path.extent.x * 2.0 <= 1.6 + 1e-5);
            assert!(forward.path.extent.y * 2.0 <= 0.2 + 1e-5);
            assert_eq!(forward.path.strokes.len(), mirrored.path.strokes.len());
            for (first, second) in forward
                .path
                .strokes
                .iter()
                .flatten()
                .zip(mirrored.path.strokes.iter().flatten())
            {
                assert_eq!(*first * Vec2::new(-1.0, 1.0), *second);
            }
            let copy = forward.clone();
            assert!(Arc::ptr_eq(&forward.path, &copy.path));
        }
    }

    #[test]
    fn mixed_words_keep_each_scripts_spacing_and_limit_hangul_syllables() {
        assert_eq!(sanitize(" hi 한글! ☃"), "HI한글!");
        assert_eq!(sanitize("가나다라마바사아"), "가나다라마");
        assert_eq!(sanitize("가나다라마바abcde"), "가나다라마ABC");
        assert_eq!(sanitize("ㄱㅏ a"), "A");
        assert!((writing_width("HI") - 1.4).abs() < 1e-5);
        assert!((writing_width("한글") - 1.95).abs() < 1e-5);
        assert!((writing_width("A한B") - 2.45).abs() < 1e-5);
        let writing = Writing::new(
            "A한B".to_owned(),
            false,
            1.0,
            &mut super::super::Random::new(1),
        );
        let first_hangul = writing.path.strokes[2][0];
        let expected = super::super::letters::strokes('한')[0][0];
        assert!((first_hangul.x - (expected.0 + 0.8 - 2.45 * 0.5) * 0.2).abs() < 1e-5);
        assert_eq!(writing_words("한글"), "한글");
        assert_eq!(writing_words("복2"), "복 two");
        let long = Writing::new(
            "한글".repeat(20),
            false,
            1.0,
            &mut super::super::Random::new(1),
        );
        assert!(long.path.extent.x * 2.0 <= 1.6 + 1e-5);
    }

    #[test]
    fn a_words_pen_walks_each_segment_in_order_without_drawing_the_lifts() {
        let picture = Picture::Word(Writing::new(
            "HI!?".to_owned(),
            false,
            1.0,
            &mut super::super::Random::new(1),
        ));
        let Picture::Word(writing) = &picture else {
            unreachable!()
        };
        let mut walked = 0.0;
        assert_eq!(picture.point_at(0.0), writing.path.strokes[0][0]);
        for segment in &writing.path.segments {
            let progress = (walked + segment.length * 0.5) / picture.length();
            let expected = segment.start + segment.along * 0.5;
            assert!(picture.point_at(progress).distance(expected) < 1e-5);
            assert!(picture.distance(expected, progress) < 1e-5);
            walked += segment.length;
        }
        assert!((walked - picture.length()).abs() < 1e-5);
        assert_eq!(picture.point_at(1.0), writing.path.last());
    }

    #[test]
    fn word_distances_match_sampling_only_the_strokes_drawn_so_far() {
        let picture = Picture::Word(Writing::new(
            "BOO!".to_owned(),
            true,
            1.0,
            &mut super::super::Random::new(1),
        ));
        let Picture::Word(writing) = &picture else {
            unreachable!()
        };
        assert_eq!(picture.distance(Vec2::ZERO, 0.0), f32::INFINITY);
        for progress in [0.1, 0.35, 0.8, 1.0] {
            for column in -5..=5 {
                for row in -3..=3 {
                    let point = Vec2::new(column as f32, row as f32) * 0.035;
                    let mut remaining = picture.length() * progress;
                    let mut nearest = f32::INFINITY;
                    for stroke in &writing.path.strokes {
                        for pair in stroke.windows(2) {
                            if remaining <= 0.0 {
                                break;
                            }
                            let length = pair[0].distance(pair[1]);
                            let drawn = (remaining / length).min(1.0);
                            for step in 0..=200 {
                                nearest =
                                    nearest.min(point.distance(
                                        pair[0].lerp(pair[1], drawn * step as f32 / 200.0),
                                    ));
                            }
                            remaining -= length;
                        }
                    }
                    assert!((picture.distance(point, progress) - nearest).abs() < 0.001);
                }
            }
        }
    }

    #[test]
    fn long_words_take_longer_to_draw_and_keep_their_fog_afterwards() {
        let short = Picture::Word(Writing::new(
            "HI".to_owned(),
            true,
            1.0,
            &mut super::super::Random::new(1),
        ));
        let long = Picture::Word(Writing::new(
            "ABCDEFGH".to_owned(),
            false,
            1.0,
            &mut super::super::Random::new(1),
        ));
        assert_eq!(short.draw_time(), Doodle::DRAW_TIME);
        assert!((long.draw_time() - 5.6).abs() < 1e-5);
        let doodle = Doodle {
            picture: long,
            size: 1.0,
            center: Vec2::ZERO,
            started: 10.0,
        };
        assert!((doodle.drawn(12.8) - 0.5).abs() < 1e-5);
        assert_eq!(doodle.opacity(15.6), 1.0);
        assert_eq!(doodle.opacity(165.6), 0.0);
        assert_eq!(writing_words("2027"), "two zero two seven");
    }
}
