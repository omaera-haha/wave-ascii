//! How each creature looks this frame: its face from its mood and what just
//! happened to it, and its body's motion from where it is going and what it
//! watches. Worked out while the creature has the world.

use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};

use crate::face::{self, expression::Expression, mood::Cues};
use crate::render::View;
use crate::render::creature::{Figure, Motion};
use crate::world::{Activity, EventKind, Gait, Mood, Outcome, World, together::Pastime};

/// Seconds of events the face reacts to.
const REACTION_WINDOW: f64 = 6.0;
/// Seconds the eyes keep following the pointer after it stops.
const WATCH_POINTER: f64 = 4.0;

/// What the app knows about a creature beyond the world: how the viewer
/// handles it and the glow its mood has eased to.
#[derive(Clone, Copy, Debug)]
pub struct Handling {
    /// Where the pointer is on screen and when it moved, for the creature
    /// the viewer is with.
    pub pointer: Option<(Vec2, f64)>,
    /// Which way the latest stroke went across the screen.
    pub stroke_side: f32,
    pub glow: Vec3,
}

/// The face of the creature in `world` at this moment.
pub fn face(world: &World, view: &View, handling: &Handling) -> face::Face {
    let ghost = &world.ghost;
    // Each creature blinks and glances on its own beat.
    let offset = f64::from(ghost.id) * 7.31;
    let center = view.project(ghost.body());
    let gaze = gaze(world, view, center, handling);
    let mut reactions: Vec<_> = world
        .events
        .iter()
        .filter(|event| world.time - event.time < REACTION_WINDOW)
        .filter(|event| !matches!(event.kind, EventKind::Quirk(_)))
        .filter(|event| event.kind != EventKind::BirdSang || ghost.asleep <= 0.5)
        .filter_map(|event| {
            let reaction = if matches!(event.kind, EventKind::GameOver(Outcome::ViewerWon { .. }))
                && world.good_sport()
            {
                Some(Expression::Smile)
            } else {
                face::mood::reaction(&event.kind)
            };
            reaction.map(|reaction| (event.time + offset, reaction))
        })
        .collect();
    if !world.companion
        && let Some(puzzle) = &world.puzzle
        && puzzle.ended.is_none()
        && let Some(worried) = puzzle.worried_at
    {
        reactions.push((worried + offset, Expression::Worry));
    }
    // The first morning also earns a milestone; let the stretch take
    // precedence over reactions recorded at that same moment.
    reactions.sort_by(|(time, expression), (other_time, other_expression)| {
        time.total_cmp(other_time).then_with(|| {
            matches!(expression, Expression::Stretch)
                .cmp(&matches!(other_expression, Expression::Stretch))
        })
    });
    let quirk = world.active_quirk();
    if let Some(spell) = quirk
        && let Some(expression) = face::mood::reaction(&EventKind::Quirk(spell.kind))
    {
        reactions.push((spell.pulse_started(world.time) + offset, expression));
    }
    if !world.companion {
        for (time, expression) in [
            world.letter_expression(),
            world.mirror_expression(),
            world.rhythm_expression(),
        ]
        .into_iter()
        .flatten()
        {
            reactions.push((time + offset, expression));
        }
    } else if let Some((time, expression)) = world.mirror_expression() {
        reactions.push((time + offset, expression));
    }
    let mut face = face::mood::face(
        world.time + offset,
        &Cues {
            mood: ghost.mood,
            asleep: if world.hunt_counting() && !world.companion {
                1.0
            } else {
                ghost.asleep
            },
            petting: world.petting(),
            gaze,
            reactions,
            focusing: world.focus.is_some(),
        },
    );
    if world.music.is_some()
        && ghost.plan.activity == Activity::Music
        && ghost.plan.count > 0
        && world.life.temperament().tender > 0.55
    {
        face.shut = [1.0; 2];
    }
    if !world.companion {
        let blowing = world.bubble_breath();
        if blowing > 0.0 {
            face.mouth = 0.35 * blowing;
            face.widen = -0.3 * blowing;
            face.smile *= 1.0 - blowing;
        }
        if world.quiet_comfort() {
            face.lid = [0.55; 2];
        }
    }
    // A body that turns its head to look needs its eyes to move less.
    if ghost.species != crate::world::Species::Ghost {
        face.gaze *= 0.55;
    }
    face
}

// Where the eyes go: to the pointer while it moves, else the way the
// creature faces, so its face shows what it is looking at.
pub(crate) fn gaze(
    world: &World,
    view: &View,
    center: Option<Vec2>,
    handling: &Handling,
) -> Option<(Vec2, f32)> {
    let center = center?;
    let ghost = &world.ghost;
    if ghost.asleep > 0.5 {
        return None;
    }
    if let Some((pointer, moved)) = handling.pointer
        && world.focus.is_none()
        && world.time - moved < WATCH_POINTER
    {
        return Some(((pointer - center) / 0.5, 0.85));
    }
    let ahead = view.project(ghost.body() + ghost.heading)?;
    Some(((ahead - center) / 0.45, 0.75))
}

/// The whole figure of the creature in `world`.
pub fn figure(world: &World, view: &View, handling: &Handling) -> Figure {
    let ghost = &world.ghost;
    let footing = ghost.footing;
    let body = ghost.body();
    let facing = footing.facing;
    // The head turns toward what it watches: the viewer while the pointer
    // moves, else what it is looking at.
    let watching_viewer = !world.companion
        && handling
            .pointer
            .is_some_and(|(_, moved)| world.time - moved < WATCH_POINTER);
    let target = if watching_viewer {
        Some(view.eye)
    } else {
        ghost.look.or(match ghost.plan.activity {
            Activity::Visit | Activity::Wait | Activity::Company if !ghost.plan.done => {
                Some(view.eye)
            }
            _ => None,
        })
    };
    let look = target.map_or(Vec2::ZERO, |target| {
        let offset = target - (body + Vec3::Y * 0.15 * world.size());
        let across = offset.x.atan2(offset.z) - facing;
        let across = (across + PI).rem_euclid(TAU) - PI;
        let up = offset.y.atan2(Vec2::new(offset.x, offset.z).length());
        // Behind it, it turns only so far and the body will follow.
        Vec2::new(across.clamp(-1.0, 1.0), up)
    });
    let mood = ghost.mood;
    let wag = match mood {
        Mood::Excited => 1.0,
        Mood::Cheerful => 0.75,
        Mood::Content | Mood::Curious => 0.35,
        Mood::Grumpy => 0.05,
        _ => 0.12,
    } + world.petting() * 0.6;
    let perk = match mood {
        Mood::Excited | Mood::Curious => 1.0,
        Mood::Cheerful => 0.6,
        Mood::Content => 0.3,
        Mood::Sleepy => -0.5,
        Mood::Lonely | Mood::Bored => -0.7,
        Mood::Grumpy => -0.3,
    };
    let dancing = (ghost.plan.activity == Activity::Dance
        || (ghost.plan.activity == Activity::Together
            && ghost.plan.pastime == Some(Pastime::Dance)))
        && !ghost.plan.done;
    let blush = (world.petting() * 1.5).max(match mood {
        Mood::Cheerful | Mood::Excited => 0.45,
        _ => 0.0,
    });
    // Quirks, music, pride, and stretches move the body as they moved the
    // ghost's.
    let quirk = world.active_quirk();
    let twitch = quirk.map(|spell| spell.motion(world.time)).unwrap_or_default();
    let shiver = quirk
        .filter(|spell| spell.kind == crate::world::Quirk::Shiver)
        .map_or(0.0, |spell| spell.envelope(world.time));
    let beat = world
        .music
        .as_ref()
        .filter(|_| ghost.plan.activity == Activity::Music && ghost.plan.count > 0)
        .map(|playing| playing.beat(world.time) * TAU);
    let (pride, calm, shake) = if world.companion {
        (0.0, None, 0.0)
    } else {
        (world.show_bob(), world.comfort_breath(), world.window_shake())
    };
    let lift = twitch.lift + 0.05 * pride + beat.map_or(0.0, |beat| 0.025 * beat.cos());
    let sway = twitch.lean * 1.5
        + beat.map_or(0.0, |beat| 0.12 * (beat * 0.5).sin())
        + shake * 0.12;
    let reach = (twitch.stretch * 8.0)
        .max(world.stretch_amount())
        .max(world.cold_stretch());
    Figure {
        id: ghost.id,
        species: ghost.species,
        genome: ghost.genome,
        position: body,
        facing,
        size: world.size(),
        face: face(world, view, handling),
        glow: handling.glow,
        opacity: 1.0 - ghost.fade.clamp(0.0, 1.0) * 0.85,
        accessory: world.wearing,
        motion: Motion {
            time: world.time,
            stride: footing.stride,
            pace: footing.pace,
            airborne: !footing.grounded && ghost.species.gait() != Gait::Float,
            rise: footing.rise,
            landed: footing.landed,
            asleep: ghost.asleep,
            perched: if ghost.species.gait() == Gait::Fly {
                footing.perched
            } else {
                1.0
            },
            look,
            wag: wag.min(1.0),
            perk,
            blush: blush.min(1.0),
            dance: if dancing { 1.0 } else { 0.0 },
            lean: handling.stroke_side * world.petting(),
            energy: ghost.needs.energy,
            lift,
            sway,
            reach,
            shiver,
            calm,
        },
    }
}
