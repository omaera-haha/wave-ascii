use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

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
        let path = self.path();
        if progress >= 1.0 {
            return path.last();
        }
        let mut remaining = self.length() * progress.clamp(0.0, 1.0);
        for (start, end) in path.segments() {
            let length = start.distance(end);
            if remaining < length {
                return start.lerp(end, remaining / length);
            }
            remaining -= length;
        }
        path.last()
    }

    /// Distance to the part of the stroke drawn so far, or infinity before
    /// any of it has been drawn.
    pub fn distance(self, point: Vec2, progress: f32) -> f32 {
        if progress <= 0.0 {
            return f32::INFINITY;
        }
        let mut remaining = self.length() * progress.min(1.0);
        let mut distance = f32::INFINITY;
        for (start, end) in self.path().segments() {
            let length = start.distance(end);
            if length == 0.0 {
                continue;
            }
            let along = end - start;
            let fraction = ((point - start).dot(along) / (length * length))
                .clamp(0.0, (remaining / length).min(1.0));
            distance = distance.min(point.distance(start + along * fraction));
            remaining -= length;
            if remaining <= 0.0 {
                break;
            }
        }
        distance
    }

    fn path(self) -> &'static Path {
        static PATHS: OnceLock<[Path; Sketch::ALL.len()]> = OnceLock::new();
        let paths = PATHS.get_or_init(|| {
            Sketch::ALL.map(|sketch| {
                let mut path = Path {
                    strokes: sketch.strokes(),
                    length: 0.0,
                };
                path.length = path
                    .segments()
                    .map(|(start, end)| start.distance(end))
                    .sum();
                path
            })
        });
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

struct Path {
    strokes: Vec<Vec<Vec2>>,
    length: f32,
}

impl Path {
    /// The drawn segments, in drawing order, leaving out the pen's jumps
    /// between strokes.
    fn segments(&self) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.strokes
            .iter()
            .flat_map(|stroke| stroke.windows(2).map(|pair| (pair[0], pair[1])))
    }

    fn last(&self) -> Vec2 {
        *self
            .strokes
            .last()
            .and_then(|stroke| stroke.last())
            .unwrap()
    }
}

/// A picture in the fog on the glass, drawn over time and left to fade.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Doodle {
    pub sketch: Sketch,
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
        ((time - self.started) / Self::DRAW_TIME).clamp(0.0, 1.0) as f32
    }

    /// How much of the fog remains, fading over its last forty seconds.
    pub fn opacity(&self, time: f64) -> f32 {
        ((Self::LIFETIME - (time - self.started)) / 40.0).clamp(0.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            assert!((sketch.length() - length).abs() < 1e-5);
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
            sketch: Sketch::Heart,
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
}
