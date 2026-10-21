use glam::Vec2;

use super::expression::Expression;
use super::{Face, GAP, HEAD_CENTER, RING_WIDTH, smoothstep};
use crate::scene::Viewer;

/// Seconds without input before the ghost starts falling asleep, and how
/// long its eyes take to close.
const SLEEP_AFTER: f32 = 40.0;
const FALLING_ASLEEP: f32 = 4.0;
/// Seconds the ghost keeps watching the pointer after it stops moving.
const WATCH_FOR: f32 = 2.5;
/// Seconds the eyes take to turn to the pointer when it starts moving.
const TURN_TO: f32 = 0.3;
/// Seconds a reaction takes to take over the face and to give it back.
const FADE: f32 = 0.15;
/// How far out from the ghost the pointer touches it: to the outside of its
/// ring.
const REACH: f32 = GAP + RING_WIDTH;
/// Where the eyes look from, in the ghost's space.
const EYES: Vec2 = Vec2::new(HEAD_CENTER.x, HEAD_CENTER.y - 0.03);
/// How far from the eyes the pointer draws them all the way over.
const GAZE_RANGE: f32 = 0.5;

const ASLEEP: Face = Face {
    shut: [1.0; 2],
    gaze: Vec2::new(0.0, -0.5),
    ..Face::REST
};

/// What the ghost does while it stays in one situation: in each slot of
/// time it may make one expression, picked at random by weight and started
/// at a random moment.
struct Repertoire {
    expressions: &'static [(Expression, f32)],
    /// Seconds per slot, at least as long as the longest expression.
    slot: f32,
    /// The chance that a slot passes without an expression.
    rest: f32,
    /// Keeps each repertoire's picks apart from the others'.
    seed: u32,
}

/// Left alone, the ghost looks around and makes faces of its own.
const IDLE: Repertoire = Repertoire {
    expressions: &[
        (
            Expression::Glance {
                toward: Vec2::new(-0.9, 0.15),
            },
            2.0,
        ),
        (
            Expression::Glance {
                toward: Vec2::new(0.85, 0.4),
            },
            2.0,
        ),
        (
            Expression::Glance {
                toward: Vec2::new(0.35, -0.75),
            },
            1.0,
        ),
        (Expression::LookAround, 2.0),
        (Expression::Smile, 1.5),
        (Expression::Prompt, 1.2),
        (Expression::Sparkle, 1.0),
        (Expression::Suspicious, 1.0),
        (Expression::Skeptical, 1.0),
        (Expression::Doze, 0.7),
        (Expression::Wink, 0.5),
        (Expression::Worry, 0.5),
    ],
    slot: 6.0,
    rest: 0.3,
    seed: 1,
};

/// While the pointer moves about, the ghost watches it, now and then in
/// wonder or with suspicion. Its eyes stay on the pointer.
const WATCHING: Repertoire = Repertoire {
    expressions: &[
        (Expression::Sparkle, 2.0),
        (Expression::Suspicious, 1.5),
        (Expression::Skeptical, 1.0),
        (Expression::Smile, 1.0),
    ],
    slot: 4.5,
    rest: 0.45,
    seed: 2,
};

/// Blinks go on under every situation but sleep.
const BLINKS: Repertoire = Repertoire {
    expressions: &[(Expression::Blink, 4.0), (Expression::DoubleBlink, 1.0)],
    slot: 3.2,
    rest: 0.15,
    seed: 3,
};

impl Repertoire {
    /// The face at `time`: the expression playing in the slot that `time`
    /// falls in, if any.
    fn at(&self, time: f32) -> Face {
        let slot = (time / self.slot).floor();
        let roll = |salt| random(slot, self.seed, salt);
        if roll(0) < self.rest {
            return Face::REST;
        }
        let total: f32 = self.expressions.iter().map(|(_, weight)| weight).sum();
        let mut pick = roll(1) * total;
        let Some((expression, _)) = self
            .expressions
            .iter()
            .find(|(_, weight)| {
                pick -= weight;
                pick < 0.0
            })
            .or(self.expressions.last())
        else {
            return Face::REST;
        };
        let start = slot * self.slot + roll(2) * (self.slot - expression.duration()).max(0.0);
        expression.at(time - start)
    }
}

/// The face at `time`, from what the viewer is doing. `locate` tells where a
/// scene point falls on the ghost at a moment: its place in the ghost's space
/// and its distance outside the ghost's outline.
///
/// Situations that last are laid over one another by how much each applies:
/// idle, then watching the moving pointer, hovered over, and asleep. Events
/// come last, each taking over the face for one reaction.
pub fn face(time: f32, viewer: &Viewer, locate: impl Fn(f32, Vec2) -> (Vec2, f32)) -> Face {
    let mut face = IDLE.at(time);

    if let Some(pointer) = viewer.pointer {
        let (place, outside) = locate(time, pointer.position);
        let gaze = ((place - EYES) / GAZE_RANGE).clamp_length_max(1.0);
        let watching = smoothstep(pointer.since, pointer.since + TURN_TO, time)
            * (1.0
                - smoothstep(
                    pointer.moved_at + WATCH_FOR,
                    pointer.moved_at + WATCH_FOR + 1.0,
                    time,
                ));
        face = face.lerp(
            Face {
                gaze,
                ..WATCHING.at(time)
            },
            watching,
        );
        // With the pointer on it, the ghost only looks at it and blinks.
        let hovered = 1.0 - smoothstep(REACH - 0.05, REACH + 0.05, outside);
        face = face.lerp(Face { gaze, ..Face::REST }, hovered);
    }

    let asleep = smoothstep(
        SLEEP_AFTER,
        SLEEP_AFTER + FALLING_ASLEEP,
        time - viewer.active_at,
    );
    face = face.lerp(ASLEEP, asleep);
    // A blink doesn't suit eyes that have become other shapes.
    let blink = BLINKS.at(time).shut[0] * (1.0 - asleep) * (1.0 - face.prompt) * (1.0 - face.smile);
    face.shut = face.shut.map(|shut| shut.max(blink));

    for (start, expression) in reactions(viewer, &locate) {
        let elapsed = time - start;
        let duration = expression.duration();
        if (0.0..duration).contains(&elapsed) {
            let fade =
                smoothstep(0.0, FADE, elapsed) * smoothstep(duration, duration - FADE, elapsed);
            face = face.lerp(expression.at(elapsed), fade);
        }
    }
    face
}

/// What the ghost reacts to, as when each event happened and the expression
/// it answers with, earliest first.
fn reactions(
    viewer: &Viewer,
    locate: &impl Fn(f32, Vec2) -> (Vec2, f32),
) -> Vec<(f32, Expression)> {
    // It greets the viewer when the scene is shown.
    let mut reactions = vec![(viewer.shown_at + 0.5, Expression::Wink)];
    // Any input after a long quiet wakes it with a start.
    if viewer.quiet >= SLEEP_AFTER && viewer.active_at > viewer.shown_at {
        reactions.push((viewer.active_at, Expression::Surprise));
    }
    // A click on it is a poke, and pokes in a row annoy it.
    if let Some(click) = viewer.click
        && click.time >= viewer.shown_at
        && locate(click.time, click.position).1 < REACH
    {
        let reaction = match click.count {
            1 => Expression::Surprise,
            2 => Expression::Smile,
            _ => Expression::Glare,
        };
        reactions.push((click.time, reaction));
    }
    reactions.sort_by(|first, second| first.0.total_cmp(&second.0));
    reactions
}

/// A number from 0 to 1 that looks random but is always the same for the
/// same `slot`, `seed`, and `salt`.
fn random(slot: f32, seed: u32, salt: u32) -> f32 {
    let mut bits = (slot as i64 as u32).wrapping_mul(0x9E37_79B9)
        ^ seed.wrapping_mul(0x85EB_CA6B)
        ^ salt.wrapping_mul(0xC2B2_AE35);
    bits ^= bits >> 16;
    bits = bits.wrapping_mul(0x7FEB_352D);
    bits ^= bits >> 15;
    bits = bits.wrapping_mul(0x846C_A68B);
    bits ^= bits >> 16;
    (bits >> 8) as f32 / (1 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Click, Pointer};

    // Places scene points in the ghost's space unchanged, `outside` away
    // from its outline.
    fn at_distance(outside: f32) -> impl Fn(f32, Vec2) -> (Vec2, f32) {
        move |_, point| (point, outside)
    }

    fn pointer(position: Vec2, moved_at: f32) -> Option<Pointer> {
        Some(Pointer {
            position,
            moved_at,
            since: moved_at - 1.0,
        })
    }

    fn click(time: f32, count: u32) -> Option<Click> {
        Some(Click {
            position: Vec2::ZERO,
            time,
            count,
        })
    }

    #[test]
    fn every_slot_fits_the_longest_expression_it_can_play() {
        for repertoire in [&IDLE, &WATCHING, &BLINKS] {
            for (expression, weight) in repertoire.expressions {
                assert!(expression.duration() <= repertoire.slot, "{expression:?}");
                assert!(*weight > 0.0, "{expression:?}");
            }
        }
    }

    #[test]
    fn random_numbers_are_between_zero_and_one_and_vary() {
        let numbers: Vec<f32> = (-50..50).map(|slot| random(slot as f32, 1, 0)).collect();
        assert!(numbers.iter().all(|number| (0.0..1.0).contains(number)));
        let mean = numbers.iter().sum::<f32>() / numbers.len() as f32;
        assert!((0.35..0.65).contains(&mean), "{mean}");
        assert_ne!(random(3.0, 1, 0), random(3.0, 2, 0));
        assert_ne!(random(3.0, 1, 0), random(3.0, 1, 1));
    }

    #[test]
    fn the_idle_ghost_makes_several_kinds_of_faces() {
        let faces: Vec<Face> = (0..2400).map(|step| IDLE.at(step as f32 * 0.5)).collect();
        assert!(faces.iter().any(|face| face.prompt > 0.9));
        assert!(faces.iter().any(|face| face.smile > 0.9));
        assert!(faces.iter().any(|face| face.gaze.x < -0.8));
        assert!(faces.iter().any(|face| face.sparkle > 0.5));
        assert!(faces.contains(&Face::REST));
    }

    #[test]
    fn the_eyes_follow_a_moving_pointer_and_let_it_go_once_it_stops() {
        let away = at_distance(1.0);
        for (side, position) in [(1.0, Vec2::new(1.2, 0.2)), (-1.0, Vec2::new(-1.2, 0.2))] {
            let viewer = Viewer {
                pointer: pointer(position, 10.0),
                ..Viewer::default()
            };
            let face = face(10.5, &viewer, &away);
            assert!(face.gaze.x * side > 0.9, "{position}: {face:?}");
        }

        let viewer = Viewer {
            pointer: pointer(Vec2::new(1.2, 0.2), 10.0),
            active_at: 10.0,
            ..Viewer::default()
        };
        let alone = Viewer {
            active_at: 10.0,
            ..Viewer::default()
        };
        assert_eq!(face(20.0, &viewer, &away), face(20.0, &alone, &away));
    }

    #[test]
    fn hovering_over_the_ghost_leaves_only_looking_and_blinking() {
        let on = at_distance(-0.1);
        for step in 0..400 {
            let time = step as f32 * 0.25;
            let viewer = Viewer {
                pointer: pointer(Vec2::new(0.3, 0.6), time - 0.5),
                active_at: time,
                ..Viewer::default()
            };
            let face = face(time + 2.0, &viewer, &on);
            let blinking = Face {
                shut: face.shut,
                ..Face::REST
            };
            assert_eq!(
                Face {
                    gaze: Vec2::ZERO,
                    ..face
                },
                blinking,
                "{time}"
            );
            assert!(face.gaze.x > 0.0 && face.gaze.y > 0.0, "{time}");
        }
    }

    #[test]
    fn a_click_on_the_ghost_startles_it_and_more_clicks_annoy_it() {
        let on = at_distance(0.0);
        let poked = |count| {
            let viewer = Viewer {
                click: click(5.0, count),
                active_at: 5.0,
                ..Viewer::default()
            };
            face(5.4, &viewer, &on)
        };
        assert!(poked(1).widen > 0.9);
        assert!(poked(2).smile > 0.9);
        assert!(poked(3).slant > 0.9);
        assert!(poked(7).slant > 0.9);
    }

    #[test]
    fn a_click_beside_the_ghost_or_before_the_scene_was_shown_is_ignored() {
        let alone = Viewer {
            active_at: 5.0,
            shown_at: 2.0,
            ..Viewer::default()
        };
        let beside = Viewer {
            click: click(5.0, 1),
            ..alone
        };
        let earlier = Viewer {
            click: click(1.0, 1),
            ..alone
        };
        assert_eq!(
            face(5.4, &beside, at_distance(1.0)),
            face(5.4, &alone, at_distance(1.0))
        );
        assert_eq!(
            face(1.4, &earlier, at_distance(0.0)),
            face(1.4, &alone, at_distance(0.0))
        );
    }

    #[test]
    fn left_alone_the_ghost_falls_asleep_and_wakes_with_a_start() {
        let away = at_distance(1.0);
        let asleep = face(60.0, &Viewer::default(), &away);
        assert_eq!(asleep.shut, [1.0; 2]);

        let woken = Viewer {
            active_at: 60.0,
            quiet: 60.0,
            ..Viewer::default()
        };
        let face = face(60.4, &woken, &away);
        assert!(face.widen > 0.9, "{face:?}");
        assert!(face.shut.iter().all(|shut| *shut < 0.1), "{face:?}");
    }

    #[test]
    fn the_ghost_winks_when_its_scene_is_shown() {
        let viewer = Viewer {
            shown_at: 30.0,
            active_at: 30.0,
            ..Viewer::default()
        };
        assert!(face(30.8, &viewer, at_distance(1.0)).shut[1] > 0.9);
    }
}
