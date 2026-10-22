use glam::Vec3;

use super::{Activity, Ball, EventKind, Mood, World};

/// How long the swirl the jolt stirred up takes to fade to a third.
const SETTLES: f32 = 1.4;
/// A spin that keeps turning the motes a while after the push is gone.
const SPIN_SETTLES: f32 = 2.6;
/// The strongest push one change of the terminal's size gives.
const HARDEST: f32 = 2.2;
/// Jolts this close together count as one shaking.
const SHAKING_FOR: f64 = 8.0;

/// The glass box was jostled: the terminal it lives in changed size, and
/// like a snow globe the light motes swirl and slowly settle again. Kept
/// only for this session.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Jostle {
    /// The push still carrying the motes, in units per second.
    pub velocity: Vec3,
    /// How fast the motes swirl around the room's middle, in radians per
    /// second.
    pub spin: f32,
    /// When the latest shaking began and how many jolts it held.
    shaking: Option<(f64, u32)>,
}

impl Jostle {
    pub fn settle(&mut self, seconds: f32) {
        self.velocity *= (-seconds / SETTLES).exp();
        self.spin *= (-seconds / SPIN_SETTLES).exp();
        if self.velocity.length_squared() < 1e-6 && self.spin.abs() < 1e-3 {
            self.velocity = Vec3::ZERO;
            self.spin = 0.0;
        }
    }

    /// The swirl's flow at `position` for a mote or petal set apart by
    /// `seed`: the push, a turn around the room's middle, and a lift that
    /// lets them drift back down as it fades.
    pub fn stir(&self, position: Vec3, seed: f32) -> Vec3 {
        if self.velocity == Vec3::ZERO && self.spin == 0.0 {
            return Vec3::ZERO;
        }
        let share = 0.6 + 0.8 * (seed * 12.9898).sin().abs();
        let around = Vec3::new(-position.z, 0.0, position.x) * self.spin * 0.45;
        let lift = Vec3::Y * self.spin.abs() * 0.3 * (seed * 7.31).cos();
        self.velocity * share + around + lift
    }
}

impl World {
    /// The terminal's columns and rows changed by these fractions of their
    /// old sizes (a narrower window is negative), shaking the box sideways
    /// and up and down by how much. A narrow sleeper sleeps on; an awake
    /// ghost startles, and a long rattling makes a playful one giggle and
    /// another one grumble.
    pub fn jostle(&mut self, columns: f32, rows: f32) {
        if !(columns.is_finite() && rows.is_finite()) {
            return;
        }
        let push = Vec3::new(columns * 3.0, rows * 2.0, 0.0).clamp_length_max(HARDEST);
        let strength = push.length();
        if strength < 0.02 {
            return;
        }
        self.jostle.velocity = (self.jostle.velocity + push).clamp_length_max(HARDEST * 1.5);
        let turn = if columns >= 0.0 { 1.0 } else { -1.0 };
        self.jostle.spin = (self.jostle.spin + turn * strength * 0.8).clamp(-2.0, 2.0);
        let floor_push = Vec3::new(push.x, 0.0, push.y * 0.4) * 0.7;
        if !self.ball.held && self.ball.position.y <= Ball::RADIUS + 0.01 {
            self.ball.velocity += floor_push;
        }
        if strength > 0.6 {
            for block in &mut self.blocks {
                if !block.held && !block.stacked {
                    block.velocity += floor_push * 0.4 + Vec3::Y * strength * 0.3;
                    block.spin += Vec3::new(0.0, push.x, 0.0) * 2.0;
                }
            }
        }
        let asleep = self.ghost.asleep > 0.5;
        self.ghost.velocity += push * if asleep { 0.15 } else { 0.5 };
        if asleep {
            return;
        }
        let (began, jolts) = match self.jostle.shaking {
            Some((began, jolts)) if self.time - began < SHAKING_FOR => (began, jolts + 1),
            _ => (self.time, 1),
        };
        self.jostle.shaking = Some((began, jolts));
        // A drag across the screen sends many small jolts; only the first
        // of a shaking and the moment it turns into a rattling are news.
        let rattled = jolts == 6;
        if jolts != 1 && !rattled {
            return;
        }
        self.record(EventKind::Jostled { rattled });
        if !rattled {
            // Counting while the viewer hides a treasure floats too; the
            // jolt mustn't end the game.
            if self.ghost.plan.activity == Activity::Float
                && !self.ghost.plan.done
                && !self.hunt_counting()
            {
                self.ghost.plan.done = true;
            }
            return;
        }
        let playful = self.life.temperament().playful >= 0.5;
        let (mood, english, korean) = if playful {
            (Mood::Excited, "Wheee! Again!", "우와아! 또!")
        } else {
            (Mood::Grumpy, "Whoa… stop shaking!", "으아… 흔들지 마!")
        };
        self.suggest_mood(mood, 20.0);
        let line = self.spoken(english, korean);
        self.say(line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::blocks;

    fn count(world: &World, rattled: bool) -> usize {
        world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Jostled { rattled })
            .count()
    }

    #[test]
    fn a_resize_swirls_the_motes_and_settles_again() {
        let mut world = World::new(3, 12.0);
        let mote = Vec3::new(0.5, 1.2, 0.3);
        world.add_mote(mote);
        world.jostle(-0.4, 0.0);
        assert!(world.jostle.velocity.x < 0.0);
        assert!(world.jostle.stir(mote, 0.3).length() > 0.1);
        world.step(12.0);
        assert!(world.jostle.stir(mote, 0.3).length() < 0.02);
        for mote in &world.motes {
            assert!(mote.position.is_finite());
        }
    }

    #[test]
    fn a_dragged_resize_startles_once_and_a_long_rattle_once_more() {
        let mut world = World::new(5, 12.0);
        world.step(0.1);
        for _ in 0..20 {
            world.jostle(0.03, 0.0);
            world.step(0.1);
        }
        assert_eq!(count(&world, false), 1);
        assert_eq!(count(&world, true), 1);
        assert!(world.ghost.speech.is_some());
        world.step(SHAKING_FOR as f32 + 1.0);
        world.jostle(0.2, 0.1);
        assert_eq!(count(&world, false), 2);
    }

    #[test]
    fn a_jolt_while_the_viewer_hides_a_treasure_keeps_the_game() {
        let mut world = World::new(5, 12.0);
        world.start_hiding().unwrap();
        world.jostle(0.3, 0.0);
        world.step(0.5);
        assert!(world.hunt_counting());
    }

    #[test]
    fn sleepers_sleep_through_and_towers_stay_standing() {
        let mut world = World::new(9, 23.0);
        world.tuck_in();
        world.step(30.0);
        assert!(world.ghost.asleep > 0.5);
        let spot = Vec3::new(0.0, 0.0, 0.3);
        world.tower = Some(spot);
        for (level, block) in world.blocks.iter_mut().take(3).enumerate() {
            block.position = spot + Vec3::Y * ((level as f32 + 0.5) * blocks::EDGE);
            block.stacked = true;
        }
        world.ball.position = Vec3::new(1.7, Ball::RADIUS, 1.2);
        for _ in 0..8 {
            world.jostle(0.5, -0.5);
            world.step(0.1);
        }
        assert!(world.ghost.asleep > 0.5);
        assert_eq!(count(&world, false) + count(&world, true), 0);
        assert_eq!(world.tower_height(), 3);
    }

    #[test]
    fn the_strongest_push_is_bounded_and_nonsense_is_ignored() {
        let mut world = World::new(1, 12.0);
        world.jostle(f32::NAN, 0.0);
        world.jostle(f32::INFINITY, 1.0);
        assert_eq!(world.jostle.velocity, Vec3::ZERO);
        for _ in 0..50 {
            world.jostle(5.0, 5.0);
        }
        assert!(world.jostle.velocity.length() <= HARDEST * 1.5 + 1e-4);
        assert!(world.jostle.spin.abs() <= 2.0);
        assert!(world.ball.velocity.is_finite());
    }
}
