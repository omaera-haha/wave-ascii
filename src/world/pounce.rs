use glam::Vec3;

use super::{Activity, EventKind, Mood, Viewer, World};

/// Recent travel on the glass that makes a pointer worth chasing.
const LIVELY_FROM: f32 = 0.35;
/// Seconds between interruptions by the pointer, even if it keeps moving.
const INTERRUPT_EVERY: f64 = 10.0;
/// Seconds between catches during a game.
const POUNCE_EVERY: f64 = 1.2;

impl Viewer {
    /// Whether the pointer is darting about on the glass.
    pub fn pointer_is_lively(&self) -> bool {
        self.pointer.is_some() && self.liveliness > LIVELY_FROM
    }
}

impl World {
    /// Moves the viewer's pointer on the glass, remembering its recent travel.
    pub fn point_at(&mut self, pointer: Option<Vec3>) {
        let was_lively = self.viewer.pointer_is_lively();
        if let Some(position) = pointer
            && self.viewer.pointer != pointer
        {
            // A finger stroking the ghost isn't darting about.
            if let Some(previous) = self.viewer.pointer
                && self.petting() < 0.05
            {
                self.viewer.liveliness += position.truncate().distance(previous.truncate());
            }
            self.viewer.moved_at = self.time;
        }
        self.viewer.pointer = pointer;
        // Darting the pointer right after speaking may be what the words
        // were about.
        if self.focus.is_none()
            && !self.puzzle_active()
            && !was_lively
            && self.viewer.pointer_is_lively()
        {
            self.shown(Activity::Pounce);
        }

        if !was_lively
            && self.pounce_is_open()
            && !matches!(self.feeling(), Mood::Sleepy | Mood::Grumpy)
            && !self.ghost.plan.done
            && matches!(
                self.ghost.plan.activity,
                Activity::Float | Activity::Explore | Activity::Window | Activity::Visit
            )
            && self
                .pointer_interrupted_at
                .is_none_or(|time| self.time - time >= INTERRUPT_EVERY)
        {
            self.ghost.plan.done = true;
            self.pointer_interrupted_at = Some(self.time);
        }
    }

    pub(super) fn pounce_is_open(&self) -> bool {
        self.focus.is_none()
            && self.ghost.asleep < 0.3
            && self.ghost.needs.energy > 0.25
            && self.viewer.pointer_is_lively()
    }

    // Trails the finger, with a little time to spring back after each catch.
    pub(super) fn pounce(&mut self, seconds: f32) -> Option<Vec3> {
        let Some(pointer) = self.viewer.pointer else {
            self.ghost.plan.done = true;
            return self.ghost.look;
        };
        if self.time - self.viewer.moved_at >= 4.0
            || self.ghost.plan.elapsed(self.time) >= self.ghost.plan.length
        {
            self.ghost.plan.done = true;
            return Some(pointer);
        }

        let spot = self.glass_spot();
        self.ghost.plan.target += (spot - self.ghost.plan.target) * (1.0 - (-seconds / 0.25).exp());
        let close = self.ghost.position.truncate().distance(spot.truncate()) < 0.2
            && (self.ghost.position.z - spot.z).abs() < 0.25;
        let ready = self.ghost.plan.count == 0
            || self
                .ghost
                .plan
                .arrived
                .is_none_or(|time| self.time - time >= POUNCE_EVERY);
        if close && ready {
            self.ghost.velocity += Vec3::new(0.0, 0.45, -1.0);
            self.ghost.needs.change(0.0, 0.1, 0.04);
            self.ghost.plan.count += 1;
            self.ghost.plan.arrived = Some(self.time);
            self.record(EventKind::Pounced);
            if self.ghost.plan.count >= 5 {
                self.ghost.plan.done = true;
            }
        }
        Some(pointer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{ROOM_MAX, ROOM_MIN};

    fn pointer(across: f32) -> Vec3 {
        Vec3::new(across, 1.2, ROOM_MAX.z)
    }

    fn wiggle(world: &mut World) {
        world.point_at(Some(pointer(-0.5)));
        world.point_at(Some(pointer(0.5)));
    }

    #[test]
    fn pointer_travel_builds_liveliness_and_stillness_lets_it_fade() {
        let mut world = World::new(1, 12.0);
        world.point_at(Some(pointer(-0.5)));
        assert_eq!(world.viewer.liveliness, 0.0);
        world.step(0.1);
        world.point_at(Some(pointer(0.5)));
        assert_eq!(world.viewer.liveliness, 1.0);
        assert_eq!(world.viewer.moved_at, world.time);
        assert!(world.viewer.pointer_is_lively());

        let moved = world.viewer.moved_at;
        world.step(0.1);
        world.point_at(Some(pointer(0.5)));
        assert_eq!(world.viewer.moved_at, moved);
        let liveliness = world.viewer.liveliness;
        world.point_at(None);
        assert_eq!(world.viewer.liveliness, liveliness);
        assert_eq!(world.viewer.moved_at, moved);
        assert!(!world.viewer.pointer_is_lively());
        world.point_at(Some(pointer(-0.5)));
        assert_eq!(world.viewer.liveliness, liveliness);

        world.step(3.0);
        assert!(world.viewer.liveliness < 0.02);
        assert!(!world.viewer.pointer_is_lively());
    }

    #[test]
    fn a_slow_drift_does_not_make_the_pointer_lively() {
        let mut world = World::new(1, 12.0);
        for step in 0..100 {
            world.point_at(Some(pointer(step as f32 * 0.01)));
            world.step(0.1);
            assert!(!world.viewer.pointer_is_lively());
        }
    }

    #[test]
    fn pouncing_needs_a_lively_pointer_and_an_awake_ghost_with_energy() {
        let mut world = World::new(1, 12.0);
        assert!(!world.options().contains(&Activity::Pounce));
        world.point_at(Some(pointer(-0.5)));
        assert!(!world.options().contains(&Activity::Pounce));
        world.point_at(Some(pointer(0.5)));
        assert!(world.options().contains(&Activity::Pounce));
        world.ghost.needs.energy = 0.25;
        assert!(!world.options().contains(&Activity::Pounce));
        world.ghost.needs.energy = 0.26;
        world.ghost.asleep = 0.3;
        assert!(!world.options().contains(&Activity::Pounce));
        world.ghost.asleep = 0.29;
        assert!(world.options().contains(&Activity::Pounce));
        world.point_at(None);
        assert!(!world.options().contains(&Activity::Pounce));
    }

    #[test]
    fn a_darting_pointer_interrupts_idle_plans_and_leaves_other_activities_alone() {
        for activity in Activity::ALL {
            let mut world = World::new(1, 12.0);
            if activity == Activity::Friend {
                world.invite_friend();
            }
            if activity == Activity::Letter {
                world.receive_letter("Hello!", false);
            }
            if activity == Activity::Comfort {
                world.hear("I'm tired");
            }
            world.begin(activity);
            wiggle(&mut world);
            assert_eq!(
                world.needs_decision(),
                matches!(
                    activity,
                    Activity::Float | Activity::Explore | Activity::Window | Activity::Visit
                ),
                "{activity:?}"
            );
        }
        for (energy, asleep) in [(0.2, 0.0), (0.8, 0.5)] {
            let mut world = World::new(1, 12.0);
            world.ghost.needs.energy = energy;
            world.ghost.asleep = asleep;
            wiggle(&mut world);
            assert!(!world.needs_decision());
        }
    }

    #[test]
    fn the_pointer_only_interrupts_when_it_turns_lively_and_waits_between_interruptions() {
        let mut world = World::new(1, 12.0);
        wiggle(&mut world);
        assert!(world.needs_decision());
        world.begin(Activity::Float);
        world.point_at(Some(pointer(-0.5)));
        assert!(!world.needs_decision());

        world.step(3.0);
        assert!(!world.viewer.pointer_is_lively());
        world.point_at(Some(pointer(0.5)));
        assert!(!world.needs_decision());

        world.step(7.1);
        world.begin(Activity::Float);
        world.point_at(Some(pointer(-0.5)));
        assert!(world.needs_decision());
    }

    #[test]
    fn the_chase_target_lags_behind_the_pointer_and_the_ghost_looks_at_it() {
        let mut world = World::new(1, 12.0);
        world.point_at(Some(pointer(-0.5)));
        world.begin(Activity::Pounce);
        world.point_at(Some(pointer(0.5)));
        world.step(0.1);
        assert!(world.ghost.plan.target.x > -0.5 && world.ghost.plan.target.x < 0.5);
        assert_eq!(world.ghost.look, Some(pointer(0.5)));
        assert!((world.ghost.plan.target.z - (ROOM_MAX.z - 0.45)).abs() < 1e-5);
    }

    #[test]
    fn a_catch_bounces_back_warms_the_ghost_and_cannot_count_again_at_once() {
        let mut world = World::new(1, 12.0);
        world.ghost.needs.fun = 0.4;
        world.ghost.needs.company = 0.4;
        world.point_at(Some(pointer(0.5)));
        world.begin(Activity::Pounce);
        let spot = world.ghost.plan.target;
        world.ghost.position = spot - Vec3::Z * 1.0;
        world.step(0.05);
        assert_eq!(world.ghost.plan.count, 0);

        world.ghost.position = spot;
        world.ghost.velocity = Vec3::ZERO;
        world.step(0.05);
        assert_eq!(world.ghost.plan.count, 1);
        assert!(world.ghost.velocity.z < 0.0 && world.ghost.velocity.y > 0.0);
        assert!(world.ghost.needs.fun > 0.49);
        assert!(world.ghost.needs.company > 0.43);
        assert_eq!(world.ghost.mood, Mood::Excited);
        for _ in 0..23 {
            world.ghost.position = spot;
            world.step(0.05);
            assert_eq!(world.ghost.plan.count, 1);
        }
        world.ghost.position = spot;
        world.step(0.1);
        assert_eq!(world.ghost.plan.count, 2);
        let catches: Vec<_> = world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Pounced)
            .collect();
        assert_eq!(catches.len(), 2);
        assert!(catches[1].time - catches[0].time >= POUNCE_EVERY);
    }

    #[test]
    fn chasing_a_finger_back_and_forth_finishes_after_five_catches_and_stays_in_the_room() {
        for seed in [1, 5, 9] {
            let mut world = World::new(seed, 12.0);
            wiggle(&mut world);
            world.begin(Activity::Pounce);
            for _ in 0..520 {
                let across = (world.time as f32 * 1.4).sin() * 0.8;
                world.point_at(Some(pointer(across)));
                world.step(0.05);
                assert!(world.ghost.position.cmpge(ROOM_MIN).all());
                assert!(world.ghost.position.cmple(ROOM_MAX).all());
                if world.needs_decision() {
                    break;
                }
            }
            assert!(world.needs_decision());
            assert_eq!(world.ghost.plan.count, 5, "seed {seed}");
            assert_eq!(world.life.today.pounces, 5);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| event.kind == EventKind::Pounced)
                    .count(),
                5
            );
        }
    }

    #[test]
    fn chasing_ends_when_the_pointer_holds_still_leaves_or_the_plan_runs_out() {
        let mut world = World::new(1, 12.0);
        wiggle(&mut world);
        world.begin(Activity::Pounce);
        world.step(3.9);
        assert!(!world.needs_decision());
        world.step(0.2);
        assert!(world.needs_decision());
        assert_eq!(world.ghost.look, world.viewer.pointer);

        wiggle(&mut world);
        world.begin(Activity::Pounce);
        world.point_at(None);
        world.step(0.05);
        assert!(world.needs_decision());

        wiggle(&mut world);
        world.begin(Activity::Pounce);
        world.ghost.plan.length = 0.3;
        for step in 0..4 {
            world.point_at(Some(pointer(step as f32 * 0.1)));
            world.step(0.1);
        }
        assert!(world.needs_decision());
    }
}
