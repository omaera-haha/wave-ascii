use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

use glam::Vec2;

/// A little picture drawn as one continuous stroke in its own -1 to 1 space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sketch {
    Heart,
    Star,
    Smile,
    Spiral,
    Sun,
    Flower,
    Wave,
}

impl Sketch {
    pub const ALL: [Self; 7] = [
        Self::Heart,
        Self::Star,
        Self::Smile,
        Self::Spiral,
        Self::Sun,
        Self::Flower,
        Self::Wave,
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
        }
    }

    /// The length of the whole stroke.
    pub fn length(self) -> f32 {
        self.path().length
    }

    /// The pen's position after drawing `progress` of the stroke.
    pub fn point_at(self, progress: f32) -> Vec2 {
        let path = self.path();
        if progress >= 1.0 {
            return *path.points.last().unwrap();
        }
        let mut remaining = self.length() * progress.clamp(0.0, 1.0);
        for segment in path.points.windows(2) {
            let length = segment[0].distance(segment[1]);
            if remaining < length {
                return segment[0].lerp(segment[1], remaining / length);
            }
            remaining -= length;
        }
        *path.points.last().unwrap()
    }

    /// Distance to the part of the stroke drawn so far, or infinity before
    /// any of it has been drawn.
    pub fn distance(self, point: Vec2, progress: f32) -> f32 {
        if progress <= 0.0 {
            return f32::INFINITY;
        }
        let mut remaining = self.length() * progress.min(1.0);
        let mut distance = f32::INFINITY;
        for segment in self.path().points.windows(2) {
            let length = segment[0].distance(segment[1]);
            if length == 0.0 {
                continue;
            }
            let along = segment[1] - segment[0];
            let fraction = ((point - segment[0]).dot(along) / (length * length))
                .clamp(0.0, (remaining / length).min(1.0));
            distance = distance.min(point.distance(segment[0] + along * fraction));
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
                let points = sketch.points();
                let length = points
                    .windows(2)
                    .map(|pair| pair[0].distance(pair[1]))
                    .sum();
                Path { points, length }
            })
        });
        &paths[self as usize]
    }

    fn points(self) -> Vec<Vec2> {
        match self {
            Self::Heart => (0..=64)
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
            Self::Star => (0..=10)
                .map(|index| {
                    let radius = if index % 2 == 0 { 0.9 } else { 0.38 };
                    Vec2::from_angle(PI * 0.5 + TAU * index as f32 / 10.0) * radius
                })
                .collect(),
            Self::Smile => {
                let mut points: Vec<Vec2> = (0..=48)
                    .map(|index| Vec2::from_angle(PI * 0.5 + TAU * index as f32 / 48.0) * 0.9)
                    .collect();
                for center in [Vec2::new(-0.3, 0.25), Vec2::new(0.3, 0.25)] {
                    points.extend(
                        (0..=12).map(|index| {
                            center + Vec2::from_angle(TAU * index as f32 / 12.0) * 0.06
                        }),
                    );
                }
                points.extend((0..=24).map(|index| {
                    Vec2::from_angle(PI + PI * index as f32 / 24.0) * Vec2::new(0.55, 0.45)
                        - Vec2::Y * 0.1
                }));
                points
            }
            Self::Spiral => (0..=96)
                .map(|index| {
                    let progress = index as f32 / 96.0;
                    Vec2::from_angle(TAU * 2.5 * progress) * (0.04 + 0.86 * progress)
                })
                .collect(),
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
                points
            }
            Self::Flower => {
                let mut points: Vec<Vec2> = (0..=80)
                    .map(|index| {
                        let angle = -PI * 0.5 + TAU * index as f32 / 80.0;
                        Vec2::from_angle(angle) * (0.46 + 0.2 * (5.0 * angle).sin()) + Vec2::Y * 0.2
                    })
                    .collect();
                points.extend([
                    Vec2::new(0.0, -0.85),
                    Vec2::new(0.0, -0.45),
                    Vec2::new(0.4, -0.3),
                    Vec2::new(0.25, -0.6),
                    Vec2::new(0.0, -0.65),
                ]);
                points
            }
            Self::Wave => (0..=48)
                .map(|index| {
                    let progress = index as f32 / 48.0;
                    Vec2::new(-0.9 + 1.8 * progress, 0.45 * (TAU * 1.5 * progress).sin())
                })
                .collect(),
        }
    }
}

struct Path {
    points: Vec<Vec2>,
    length: f32,
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
            let points = &sketch.path().points;
            assert_eq!(sketch.point_at(0.0), points[0]);
            assert_eq!(sketch.point_at(1.0), *points.last().unwrap());
            let length: f32 = points
                .windows(2)
                .map(|pair| pair[0].distance(pair[1]))
                .sum();
            assert!((sketch.length() - length).abs() < 1e-5);
            for &point in points {
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
