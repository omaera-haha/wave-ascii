use std::f32::consts::{PI, TAU};
use std::sync::OnceLock;

use glam::Vec2;

use super::{LearnedPicture, Sketch};

const POINTS: usize = 32;
// Weighted cloud distances for jittered templates stay below this; unrelated
// scribbles stay above it. No rotation is removed during normalization.
const THRESHOLD: f32 = 1.1;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Recognition {
    pub name: String,
    pub korean: String,
    pub sketch: Option<Sketch>,
    pub learned: bool,
}

struct Template {
    recognition: Recognition,
    cloud: Vec<Vec2>,
}

/// The $P point-cloud matcher (Vatavu, Anthony, Wobbrock, 2012):
/// https://depts.washington.edu/acelab/proj/dollar/pdollar.pdf
pub(super) fn recognize(strokes: &[Vec<Vec2>], learned: &[LearnedPicture]) -> Option<Recognition> {
    best_match(strokes, learned, THRESHOLD).map(|(recognition, _)| recognition)
}

pub(super) fn best_match(
    strokes: &[Vec<Vec2>],
    learned: &[LearnedPicture],
    threshold: f32,
) -> Option<(Recognition, f32)> {
    let cloud: Vec<Vec2> = normalized(strokes).into_iter().flatten().collect();
    if cloud.len() != POINTS {
        return None;
    }
    let mut best = threshold;
    let mut result = None;
    for template in templates() {
        let distance = cloud_match_below(&cloud, &template.cloud, best);
        if distance < best {
            best = distance;
            result = Some(template.recognition.clone());
        }
    }
    for picture in learned {
        // Learned pictures already contain resampled points. Resampling them
        // again would cut corners and lose the very shape the viewer taught.
        let template: Vec<Vec2> = if picture.strokes.iter().map(Vec::len).sum::<usize>() == POINTS {
            picture.strokes.iter().flatten().copied().collect()
        } else {
            normalized(&picture.strokes).into_iter().flatten().collect()
        };
        if template.len() == POINTS {
            let distance = cloud_match_below(&cloud, &template, best.max(0.03));
            // Saving rounds coordinates, which can break a near-exact tie
            // with a built-in. In that case the viewer's correction wins.
            if distance < best || distance < 0.03 && best < 0.03 {
                best = distance;
                result = Some(Recognition {
                    name: picture.name.clone(),
                    korean: picture.name.clone(),
                    sketch: None,
                    learned: true,
                });
            }
        }
    }
    result.map(|recognition| (recognition, best))
}

/// Resamples drawn length only, retaining the pen lifts for later copies.
/// Uniform scale preserves aspect ratio; the cloud's centroid is the origin.
pub(super) fn normalized(strokes: &[Vec<Vec2>]) -> Vec<Vec<Vec2>> {
    if strokes.iter().flatten().any(|point| !point.is_finite()) {
        return Vec::new();
    }
    let length: f32 = strokes
        .iter()
        .flat_map(|stroke| stroke.windows(2))
        .map(|pair| pair[0].distance(pair[1]))
        .sum();
    if !length.is_finite() || length < 1e-5 {
        return Vec::new();
    }
    let interval = length / (POINTS - 1) as f32;
    let mut next = 0.0;
    let mut walked = 0.0;
    let mut count = 0;
    let mut sampled = Vec::new();
    for stroke in strokes {
        let mut points = Vec::new();
        for pair in stroke.windows(2) {
            let distance = pair[0].distance(pair[1]);
            if distance <= 0.0 {
                continue;
            }
            while next <= walked + distance && count < POINTS {
                points.push(pair[0].lerp(pair[1], ((next - walked) / distance).clamp(0.0, 1.0)));
                count += 1;
                next = count as f32 * interval;
            }
            walked += distance;
        }
        if !points.is_empty() {
            sampled.push(points);
        }
    }
    if count < POINTS {
        sampled.last_mut().unwrap().push(
            *strokes
                .iter()
                .rev()
                .find_map(|stroke| stroke.last())
                .unwrap(),
        );
    }
    let (minimum, maximum, sum) = sampled.iter().flatten().fold(
        (
            Vec2::splat(f32::INFINITY),
            Vec2::splat(f32::NEG_INFINITY),
            Vec2::ZERO,
        ),
        |(minimum, maximum, sum), &point| (minimum.min(point), maximum.max(point), sum + point),
    );
    let size = (maximum - minimum).max_element();
    if size < 1e-5 {
        return Vec::new();
    }
    let center = sum / POINTS as f32;
    for point in sampled.iter_mut().flatten() {
        *point = (*point - center) / size;
    }
    sampled
}

#[cfg(test)]
fn cloud_match(first: &[Vec2], second: &[Vec2]) -> f32 {
    cloud_match_below(first, second, f32::INFINITY)
}

fn cloud_match_below(first: &[Vec2], second: &[Vec2], limit: f32) -> f32 {
    let mut best = limit;
    for start in (0..POINTS).step_by(5) {
        best = best.min(cloud_distance(first, second, start, best));
        best = best.min(cloud_distance(second, first, start, best));
    }
    best
}

fn cloud_distance(first: &[Vec2], second: &[Vec2], start: usize, limit: f32) -> f32 {
    let mut matched = [false; POINTS];
    let mut sum = 0.0;
    for offset in 0..POINTS {
        let point = first[(start + offset) % POINTS];
        let mut index = 0;
        let mut distance = f32::INFINITY;
        for (candidate, other) in second.iter().enumerate() {
            if !matched[candidate] {
                let across = point.x - other.x;
                let up = point.y - other.y;
                let separation = across * across + up * up;
                if separation < distance {
                    index = candidate;
                    distance = separation;
                }
            }
        }
        matched[index] = true;
        sum += (1.0 - offset as f32 / POINTS as f32) * distance.sqrt();
        // Every remaining contribution is positive. A losing start cannot
        // become the best match, which keeps bounded star searches cheap.
        if sum >= limit {
            return limit;
        }
    }
    sum
}

fn templates() -> &'static [Template] {
    static TEMPLATES: OnceLock<Vec<Template>> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        let korean = [
            "하트",
            "별",
            "웃는 얼굴",
            "소용돌이",
            "해",
            "꽃",
            "물결",
            "눈송이",
        ];
        let sketches = Sketch::ALL.into_iter().zip(korean).map(|(sketch, korean)| {
            (
                Recognition {
                    name: sketch.name().to_owned(),
                    korean: korean.to_owned(),
                    sketch: Some(sketch),
                    learned: false,
                },
                sketch.strokes(),
            )
        });
        sketches
            .chain(plain_shapes().into_iter().map(|(name, korean, strokes)| {
                (
                    Recognition {
                        name: name.to_owned(),
                        korean: korean.to_owned(),
                        sketch: None,
                        learned: false,
                    },
                    strokes,
                )
            }))
            .map(|(recognition, strokes)| Template {
                recognition,
                cloud: normalized(&strokes).into_iter().flatten().collect(),
            })
            .collect()
    })
}

type Shape = (&'static str, &'static str, Vec<Vec<Vec2>>);

fn plain_shapes() -> Vec<Shape> {
    let stroke = |points: &[(f32, f32)]| {
        points
            .iter()
            .map(|&(horizontal, vertical)| Vec2::new(horizontal, vertical))
            .collect()
    };
    let circle = (0..=64)
        .map(|index| Vec2::from_angle(TAU * index as f32 / 64.0))
        .collect();
    let mut moon: Vec<Vec2> = (0..=32)
        .map(|index| Vec2::from_angle(PI * 0.5 + PI * index as f32 / 32.0))
        .collect();
    moon.extend((0..=32).map(|index| {
        let angle = -PI * 0.5 + PI * index as f32 / 32.0;
        Vec2::new(-0.55 * angle.cos(), angle.sin())
    }));
    vec![
        ("circle", "동그라미", vec![circle]),
        (
            "triangle",
            "세모",
            vec![stroke(&[(0.0, 1.0), (-1.0, -1.0), (1.0, -1.0), (0.0, 1.0)])],
        ),
        (
            "square",
            "네모",
            vec![stroke(&[
                (-1.0, -1.0),
                (-1.0, 1.0),
                (1.0, 1.0),
                (1.0, -1.0),
                (-1.0, -1.0),
            ])],
        ),
        (
            "cross",
            "엑스",
            vec![
                stroke(&[(-1.0, -1.0), (1.0, 1.0)]),
                stroke(&[(-1.0, 1.0), (1.0, -1.0)]),
            ],
        ),
        (
            "tick",
            "체크",
            vec![stroke(&[(-0.8, 0.0), (-0.2, -0.6), (0.9, 0.9)])],
        ),
        (
            "zigzag",
            "번개",
            vec![stroke(&[(0.5, 1.0), (-0.6, 0.0), (0.5, 0.0), (-0.5, -1.0)])],
        ),
        (
            "house",
            "집",
            vec![
                stroke(&[
                    (-0.7, 0.2),
                    (-0.7, -1.0),
                    (0.7, -1.0),
                    (0.7, 0.2),
                    (-0.7, 0.2),
                ]),
                stroke(&[(-1.0, 0.2), (0.0, 1.0), (1.0, 0.2)]),
            ],
        ),
        ("moon", "달", vec![moon]),
        ("line", "선", vec![stroke(&[(-1.0, 0.0), (1.0, 0.0)])]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Random;

    #[test]
    fn distance_cutoffs_preserve_the_original_cloud_match() {
        let original_distance = |first: &[Vec2], second: &[Vec2], start: usize| {
            let mut matched = [false; POINTS];
            let mut sum = 0.0;
            for offset in 0..POINTS {
                let point = first[(start + offset) % POINTS];
                let (index, distance) = second
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| !matched[*index])
                    .map(|(index, &other)| (index, point.distance_squared(other)))
                    .min_by(|first, second| first.1.total_cmp(&second.1))
                    .unwrap();
                matched[index] = true;
                sum += (1.0 - offset as f32 / POINTS as f32) * distance.sqrt();
            }
            sum
        };
        let mut random = Random::new(19);
        for template in templates() {
            let changed: Vec<Vec2> = template
                .cloud
                .iter()
                .map(|&point| {
                    point + Vec2::new(random.range(-0.04, 0.04), random.range(-0.04, 0.04))
                })
                .collect();
            let distance = (0..POINTS)
                .step_by(5)
                .map(|start| {
                    original_distance(&changed, &template.cloud, start).min(original_distance(
                        &template.cloud,
                        &changed,
                        start,
                    ))
                })
                .fold(f32::INFINITY, f32::min);
            for limit in [0.1, 0.5, 1.35, f32::INFINITY] {
                assert!(
                    (cloud_match_below(&changed, &template.cloud, limit) - distance.min(limit))
                        .abs()
                        < 1e-6
                );
            }
        }
    }

    #[test]
    fn every_template_survives_scale_translation_reversed_strokes_and_small_jitter() {
        let shapes = Sketch::ALL
            .into_iter()
            .map(|sketch| (sketch.name(), sketch.strokes()))
            .chain(
                plain_shapes()
                    .into_iter()
                    .map(|(name, _, strokes)| (name, strokes)),
            );
        let mut random = Random::new(17);
        for (name, strokes) in shapes {
            let changed: Vec<Vec<Vec2>> = strokes
                .iter()
                .rev()
                .map(|stroke| {
                    stroke
                        .iter()
                        .rev()
                        .map(|&point| {
                            point * 0.63
                                + Vec2::new(3.0, -2.0)
                                + Vec2::new(
                                    random.range(-0.003, 0.003),
                                    random.range(-0.003, 0.003),
                                )
                        })
                        .collect()
                })
                .collect();
            assert_eq!(
                recognize(&changed, &[]).map(|result| result.name),
                Some(name.to_owned()),
                "{name}"
            );
        }
    }

    #[test]
    fn a_circle_and_square_have_different_nearest_templates() {
        for (name, _, strokes) in plain_shapes()
            .into_iter()
            .filter(|(name, _, _)| matches!(*name, "circle" | "square"))
        {
            assert_eq!(recognize(&strokes, &[]).unwrap().name, name);
        }
    }

    #[test]
    fn a_seeded_scribble_is_not_a_picture() {
        let mut random = Random::new(51);
        let strokes = vec![
            (0..45)
                .map(|_| Vec2::new(random.range(-1.0, 1.0), random.range(-1.0, 1.0)))
                .collect(),
        ];
        assert_eq!(recognize(&strokes, &[]), None);
    }

    #[test]
    fn cloud_distances_are_invariant_to_uniform_scale_and_translation() {
        let strokes = Sketch::Heart.strokes();
        let changed: Vec<Vec<Vec2>> = strokes
            .iter()
            .map(|stroke| {
                stroke
                    .iter()
                    .map(|point| *point * 3.7 + Vec2::new(8.0, -4.0))
                    .collect()
            })
            .collect();
        let original: Vec<Vec2> = normalized(&strokes).into_iter().flatten().collect();
        let changed: Vec<Vec2> = normalized(&changed).into_iter().flatten().collect();
        let template = &templates()[1].cloud;
        assert!(cloud_match(&original, &changed) < 1e-4);
        assert!((cloud_match(&original, template) - cloud_match(&changed, template)).abs() < 1e-4);
    }

    #[test]
    fn resampling_counts_drawn_segments_without_sampling_the_jumps() {
        let strokes = vec![
            vec![Vec2::ZERO, Vec2::X],
            vec![Vec2::X * 100.0, Vec2::X * 101.0],
        ];
        let sampled = normalized(&strokes);
        assert_eq!(sampled.iter().map(Vec::len).sum::<usize>(), POINTS);
        assert_eq!(sampled.len(), 2);
        assert!(sampled.iter().flatten().all(|point| point.x.abs() > 0.4));
    }

    #[test]
    fn orientation_is_preserved_and_empty_or_degenerate_clouds_are_rejected() {
        let horizontal = normalized(&[vec![-Vec2::X, Vec2::X]])
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let vertical = normalized(&[vec![-Vec2::Y, Vec2::Y]])
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        assert!(cloud_match(&horizontal, &vertical) > 1.0);
        for strokes in [
            vec![],
            vec![vec![Vec2::ZERO]],
            vec![vec![Vec2::ZERO; 5]],
            vec![vec![Vec2::ZERO, Vec2::splat(f32::NAN)]],
        ] {
            assert!(normalized(&strokes).is_empty());
            assert_eq!(recognize(&strokes, &[]), None);
        }
    }
}
