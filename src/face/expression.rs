use std::f32::consts::TAU;

use glam::Vec2;

use super::{Face, ease_through};
use crate::sdf::smoothstep;

/// Something the ghost's face does for a moment. Each expression starts and
/// ends with the face at rest, so expressions can follow one another in any
/// order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Expression {
    /// Both eyes shut and open again.
    Blink,
    /// Two quick blinks.
    DoubleBlink,
    /// The right eye shuts for a moment.
    Wink,
    /// The eyes dart toward `toward`, from -1 to 1 across and up, rest there,
    /// and come back.
    Glance { toward: Vec2 },
    /// The eyes look to one side, then up to the other.
    LookAround,
    /// The eyes close into happy arcs.
    Smile,
    /// The eyes pop open wide and look up, then settle.
    Surprise,
    /// The lids droop, sink further, and snap open.
    Doze,
    /// The eyes narrow and look from side to side.
    Suspicious,
    /// The lids slant down toward the middle into a glare.
    Glare,
    /// The lids slant down toward the sides and the eyes look down.
    Worry,
    /// One lid lowers while the other eye stays open.
    Skeptical,
    /// The eyes open wide and look up in wonder, with stars of light
    /// twinkling in them.
    Sparkle,
    /// The eyes turn into a terminal prompt, `>_`, whose cursor blinks.
    Prompt,
    /// The eyes squeeze shut, then open wide with a little upward lift.
    Stretch,
}

impl Expression {
    /// Seconds from the start of the expression until the face is back at
    /// rest.
    pub fn duration(self) -> f32 {
        match self {
            Self::Blink => 0.22,
            Self::DoubleBlink => 0.36,
            Self::Wink => 0.72,
            Self::Glance { .. } => 1.65,
            Self::LookAround => 4.3,
            Self::Smile => 2.25,
            Self::Surprise => 1.5,
            Self::Doze => 3.4,
            Self::Stretch => 3.6,
            Self::Suspicious => 2.65,
            Self::Glare => 2.05,
            Self::Worry => 2.5,
            Self::Skeptical => 2.15,
            Self::Sparkle => 2.7,
            Self::Prompt => 4.3,
        }
    }

    /// The face `time` seconds after the expression starts, at rest before
    /// it starts and after it ends.
    pub fn at(self, time: f32) -> Face {
        match self {
            Self::Blink => blink(time),
            Self::DoubleBlink => double_blink(time),
            Self::Wink => wink(time),
            Self::Glance { toward } => glance(time, toward),
            Self::LookAround => look_around(time),
            Self::Smile => smile(time),
            Self::Surprise => surprise(time),
            Self::Doze => doze(time),
            Self::Stretch => stretch(time),
            Self::Suspicious => suspicious(time),
            Self::Glare => glare(time),
            Self::Worry => worry(time),
            Self::Skeptical => skeptical(time),
            Self::Sparkle => sparkle(time),
            Self::Prompt => prompt(time),
        }
    }
}

fn blink(time: f32) -> Face {
    let shut = ease_through(&[(0.0, 0.0), (0.1, 1.0), (0.22, 0.0)], time);
    Face {
        shut: [shut; 2],
        ..Face::REST
    }
}

fn double_blink(time: f32) -> Face {
    let shut = ease_through(
        &[
            (0.0, 0.0),
            (0.08, 1.0),
            (0.17, 0.0),
            (0.25, 1.0),
            (0.36, 0.0),
        ],
        time,
    );
    Face {
        shut: [shut; 2],
        ..Face::REST
    }
}

fn wink(time: f32) -> Face {
    Face {
        shut: [0.0, swell(time, 0.12, 0.57, 0.72)],
        ..Face::REST
    }
}

// Eyes move quickly and return a little more slowly.
fn glance(time: f32, toward: Vec2) -> Face {
    Face {
        gaze: toward * swell(time, 0.2, 1.4, 1.65),
        ..Face::REST
    }
}

fn look_around(time: f32) -> Face {
    let across = ease_through(
        &[
            (0.0, 0.0),
            (0.6, -1.0),
            (1.6, -1.0),
            (2.3, 0.9),
            (3.6, 0.9),
            (4.3, 0.0),
        ],
        time,
    );
    let up = ease_through(&[(1.6, 0.0), (2.3, 0.7), (3.6, 0.7), (4.3, 0.0)], time);
    Face {
        gaze: Vec2::new(across, up),
        ..Face::REST
    }
}

fn smile(time: f32) -> Face {
    Face {
        smile: swell(time, 0.3, 1.9, 2.25),
        ..Face::REST
    }
}

fn surprise(time: f32) -> Face {
    let wide = ease_through(&[(0.0, 0.0), (0.12, 1.0), (0.9, 1.0), (1.5, 0.0)], time);
    Face {
        widen: wide,
        gaze: Vec2::new(0.0, 0.4 * wide),
        ..Face::REST
    }
}

// The lids sink, lift a little as the ghost fights it, sink almost shut, and
// snap open with a start.
fn doze(time: f32) -> Face {
    let lid = ease_through(
        &[
            (0.0, 0.0),
            (1.2, 0.5),
            (1.8, 0.42),
            (2.6, 0.85),
            (2.75, 0.0),
        ],
        time,
    );
    Face {
        lid: [lid; 2],
        widen: ease_through(&[(2.6, 0.0), (2.75, 0.7), (3.4, 0.0)], time),
        gaze: Vec2::new(
            0.0,
            ease_through(&[(0.0, 0.0), (1.2, -0.5), (2.6, -0.7), (2.75, 0.0)], time),
        ),
        ..Face::REST
    }
}

fn stretch(time: f32) -> Face {
    Face {
        shut: [swell(time, 0.55, 1.2, 1.9); 2],
        lid: [0.18 * swell(time, 0.4, 1.0, 1.9); 2],
        widen: ease_through(&[(1.2, 0.0), (1.9, 0.65), (2.7, 0.4), (3.6, 0.0)], time),
        gaze: Vec2::new(0.0, 0.35 * swell(time, 1.2, 2.7, 3.6)),
        ..Face::REST
    }
}

fn suspicious(time: f32) -> Face {
    let across = ease_through(
        &[
            (0.2, 0.0),
            (0.5, 0.9),
            (1.3, 0.9),
            (1.6, -0.9),
            (2.3, -0.9),
            (2.65, 0.0),
        ],
        time,
    );
    Face {
        lid: [0.45 * swell(time, 0.3, 2.35, 2.65); 2],
        gaze: Vec2::new(across, 0.0),
        ..Face::REST
    }
}

fn glare(time: f32) -> Face {
    let amount = swell(time, 0.25, 1.65, 2.05);
    Face {
        lid: [0.4 * amount; 2],
        slant: amount,
        ..Face::REST
    }
}

fn worry(time: f32) -> Face {
    let amount = swell(time, 0.4, 2.0, 2.5);
    Face {
        lid: [0.25 * amount; 2],
        slant: -0.9 * amount,
        widen: 0.2 * amount,
        gaze: Vec2::new(0.0, -0.6 * amount),
        ..Face::REST
    }
}

fn skeptical(time: f32) -> Face {
    let amount = swell(time, 0.3, 1.8, 2.15);
    Face {
        lid: [0.5 * amount, 0.0],
        widen: 0.25 * amount,
        gaze: Vec2::new(-0.3 * amount, 0.0),
        ..Face::REST
    }
}

fn sparkle(time: f32) -> Face {
    let amount = swell(time, 0.25, 2.3, 2.7);
    // The stars pulse a little as they twinkle.
    let twinkle = 0.8 + 0.2 * (TAU * time / 0.7).cos();
    Face {
        widen: 0.35 * amount,
        gaze: Vec2::new(0.0, 0.35 * amount),
        sparkle: amount * twinkle,
        ..Face::REST
    }
}

fn prompt(time: f32) -> Face {
    // The cursor blinks three times while the prompt is up.
    let cursor = if (0.7..3.4).contains(&time) {
        smoothstep(-0.35, 0.35, (TAU * (time - 0.7) / 0.9).cos())
    } else {
        1.0
    };
    Face {
        prompt: ease_through(&[(0.0, 0.0), (0.7, 1.0), (3.6, 1.0), (4.3, 0.0)], time),
        cursor,
        ..Face::REST
    }
}

/// Rises from 0 at time 0 to 1 at `full`, holds until `fade`, and falls back
/// to 0 at `end`.
fn swell(time: f32, full: f32, fade: f32, end: f32) -> f32 {
    ease_through(&[(0.0, 0.0), (full, 1.0), (fade, 1.0), (end, 0.0)], time)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Expression; 15] = [
        Expression::Blink,
        Expression::DoubleBlink,
        Expression::Wink,
        Expression::Glance {
            toward: Vec2::new(-0.8, 0.3),
        },
        Expression::LookAround,
        Expression::Smile,
        Expression::Surprise,
        Expression::Doze,
        Expression::Suspicious,
        Expression::Glare,
        Expression::Worry,
        Expression::Skeptical,
        Expression::Sparkle,
        Expression::Prompt,
        Expression::Stretch,
    ];

    #[test]
    fn every_expression_starts_and_ends_at_rest() {
        for expression in ALL {
            let end = expression.duration();
            for time in [-1.0, 0.0, end, end + 1.0] {
                assert_eq!(expression.at(time), Face::REST, "{expression:?} at {time}");
            }
        }
    }

    #[test]
    fn every_expression_is_still_moving_just_before_it_ends() {
        for expression in ALL {
            let time = expression.duration() - 0.02;
            assert_ne!(expression.at(time), Face::REST, "{expression:?}");
        }
    }

    #[test]
    fn every_expression_keeps_the_face_controls_in_range() {
        let unit = 0.0..=1.0;
        let signed = -1.0..=1.0;
        for expression in ALL {
            for step in 0..=200 {
                let time = expression.duration() * step as f32 / 200.0;
                let face = expression.at(time);
                let in_range = signed.contains(&face.gaze.x)
                    && signed.contains(&face.gaze.y)
                    && face
                        .shut
                        .iter()
                        .chain(&face.lid)
                        .all(|value| unit.contains(value))
                    && signed.contains(&face.slant)
                    && signed.contains(&face.widen)
                    && unit.contains(&face.smile)
                    && unit.contains(&face.sparkle)
                    && unit.contains(&face.prompt)
                    && unit.contains(&face.cursor);
                assert!(in_range, "{expression:?} at {time}: {face:?}");
            }
        }
    }

    #[test]
    fn every_expression_draws_eyes_inside_the_head() {
        for expression in ALL {
            for step in 0..=50 {
                let face = expression.at(expression.duration() * step as f32 / 50.0);
                // Some part of an eye is cut out, and nothing near the edge
                // of the head is.
                let cut = (-30..=30)
                    .flat_map(|column| (-30..=30).map(move |row| (column, row)))
                    .map(|(column, row)| Vec2::new(column as f32, row as f32) * 0.01)
                    .any(|point| face.distance(point) < 0.0);
                assert!(cut, "{expression:?} at step {step}");
                for turn in 0..32 {
                    let edge = Vec2::from_angle(TAU * turn as f32 / 32.0) * 0.4;
                    assert!(face.distance(edge) > 0.0, "{expression:?} at step {step}");
                }
            }
        }
    }
}
