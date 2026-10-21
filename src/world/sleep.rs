use glam::Vec3;

use super::{Activity, EventKind, Mood, Plan, World};

const BEDTIME_HOUR: f32 = 22.0;
const MORNING_HOUR: f32 = 6.5;

impl World {
    /// Bedtime runs from ten at night until half past six in the morning.
    pub fn bedtime(&self) -> bool {
        !(MORNING_HOUR..BEDTIME_HOUR).contains(&self.hour)
    }

    /// How far through bedtime it is, from just starting to nearly morning.
    pub(crate) fn bedtime_progress(&self) -> f32 {
        if self.bedtime() {
            (self.hour - BEDTIME_HOUR).rem_euclid(24.0) / (24.0 - BEDTIME_HOUR + MORNING_HOUR)
        } else {
            0.0
        }
    }

    /// Whether being woken in the night still leaves it drowsy.
    pub fn drowsy(&self) -> bool {
        self.time < self.drowsy_until
    }

    /// The first ten minutes after waking naturally from a night's sleep.
    pub fn morning(&self) -> bool {
        self.morning_at
            .is_some_and(|woke| (0.0..600.0).contains(&(self.time - woke)))
    }

    /// Puts a live ghost to bed at bedtime without counting another falling
    /// asleep when the app starts.
    pub fn tuck_in(&mut self) {
        if !self.bedtime() {
            return;
        }
        self.abandon_game();
        self.drop_ball();
        let bed = self.nook.bed(self.size());
        let mut plan = Plan::new(Activity::Rest, self.time, bed, 0.0);
        plan.overnight = true;
        plan.arrived = Some(self.time);
        self.ghost.plan = plan;
        self.ghost.position = bed;
        self.ghost.velocity = Vec3::ZERO;
        self.ghost.asleep = 1.0;
        self.ghost.fade = 0.0;
        self.ghost.look = None;
        self.ghost.mood = Mood::Sleepy;
    }

    pub(super) fn greet_morning(&mut self) {
        self.drowsy_until = 0.0;
        self.morning_at = Some(self.time);
        self.ghost.velocity.y += 0.6;
        self.record(EventKind::Morning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bedtime_has_later_bounds_than_the_rooms_night() {
        let mut world = World::new(1, 21.99);
        assert!(world.is_night());
        assert!(!world.bedtime());
        for hour in [22.0, 23.99, 0.0, 6.49] {
            world.hour = hour;
            assert!(world.bedtime(), "{hour}");
        }
        for hour in [6.5, 12.0, 21.99] {
            world.hour = hour;
            assert!(!world.bedtime(), "{hour}");
        }
    }

    #[test]
    fn an_overnight_rest_waits_for_morning_even_with_full_energy() {
        let mut world = World::new(4, 23.0);
        world.ghost.needs.energy = 1.0;
        assert!(world.options().contains(&Activity::Rest));
        world.begin(Activity::Rest);
        assert!(world.ghost.plan.overnight);
        world.step(120.0);
        assert_eq!(world.ghost.asleep, 1.0);
        assert!(!world.needs_decision());
        assert!(
            !world
                .events
                .iter()
                .any(|event| event.kind == EventKind::WokeUp)
        );
        world.hour = 6.5;
        world.step(0.1);
        assert!(world.needs_decision());
        assert_eq!(world.ghost.asleep, 0.0);
        let woke = world
            .events
            .iter()
            .position(|event| event.kind == EventKind::WokeUp)
            .unwrap();
        assert_eq!(world.events[woke + 1].kind, EventKind::Morning);
        assert!(world.morning());
        assert!(world.ghost.velocity.y > 0.0);
        world.step(0.1);
        assert_eq!(world.life.total.nights, 1);
        world.time += 600.0;
        assert!(!world.morning());
    }

    #[test]
    fn a_nap_that_runs_into_bedtime_still_ends_when_rested() {
        let mut world = World::new(4, 14.0);
        world.ghost.needs.energy = 0.3;
        world.begin(Activity::Rest);
        world.step(20.0);
        assert!(!world.ghost.plan.overnight);
        assert!(world.ghost.asleep > 0.9);
        world.hour = 22.0;
        world.step(70.0);
        assert!(world.needs_decision());
        assert_eq!(world.ghost.asleep, 0.0);
        assert!(!world.drowsy());
        assert!(!world.morning());
        assert_eq!(world.life.total.nights, 0);
    }

    #[test]
    fn a_poke_at_bedtime_leaves_the_ghost_drowsy_for_two_minutes() {
        let mut world = World::new(4, 23.0);
        world.tuck_in();
        world.ghost.needs.energy = 1.0;
        world.poke();
        world.step(0.1);
        assert_eq!(world.ghost.asleep, 0.0);
        assert!(world.drowsy());
        assert_eq!(world.ghost.mood, Mood::Sleepy);
        world.step(119.0);
        assert_eq!(world.ghost.mood, Mood::Sleepy);
        world.step(1.0);
        assert!(!world.drowsy());
        assert_ne!(world.ghost.mood, Mood::Sleepy);
        assert_eq!(world.life.total.nights, 0);
    }

    #[test]
    fn delight_annoyance_and_a_minds_suggestion_take_precedence_over_drowsiness() {
        let mut world = World::new(4, 23.0);
        world.tuck_in();
        world.poke();
        assert_eq!(world.feeling(), Mood::Sleepy);
        world.record(EventKind::AteMote);
        assert_eq!(world.feeling(), Mood::Excited);
        world.poke();
        world.poke();
        assert_eq!(world.feeling(), Mood::Grumpy);
        world.suggest_mood(Mood::Curious, 20.0);
        assert_eq!(world.feeling(), Mood::Curious);
    }

    #[test]
    fn noticing_the_viewer_leaves_a_sleeping_ghost_asleep() {
        let mut world = World::new(4, 23.0);
        world.life.bond = 0.8;
        world.tuck_in();
        world.notice_viewer();
        world.step(0.1);
        assert!(world.viewer_present());
        assert_eq!(world.ghost.asleep, 1.0);
        assert!(!world.needs_decision());
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert!(
            !world
                .events
                .iter()
                .any(|event| event.kind == EventKind::WokeUp)
        );
    }

    #[test]
    fn tucking_in_starts_asleep_on_the_cushion_without_another_sleep_event() {
        let mut world = World::new(4, 23.0);
        world.tuck_in();
        assert_eq!(world.ghost.position, world.nook.bed(world.size()));
        assert_eq!(world.ghost.velocity, Vec3::ZERO);
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert!(world.ghost.plan.overnight);
        assert_eq!(world.ghost.plan.arrived, Some(world.time));
        assert_eq!(world.ghost.asleep, 1.0);
        world.step(0.1);
        assert!(
            !world
                .events
                .iter()
                .any(|event| event.kind == EventKind::FellAsleep)
        );
        assert_eq!(world.life.total.naps, 0);
    }

    #[test]
    fn tucking_in_by_day_leaves_the_current_plan_alone() {
        let mut world = World::new(4, 12.0);
        let plan = world.ghost.plan.clone();
        world.tuck_in();
        assert_eq!(world.ghost.plan, plan);
        assert_eq!(world.ghost.asleep, 0.0);
    }

    #[test]
    fn talking_and_thunder_leave_a_ghost_woken_at_bedtime_drowsy_too() {
        let mut world = World::new(4, 23.0);
        world.tuck_in();
        world.hear("hello");
        assert!(world.drowsy());
        assert_eq!(world.ghost.asleep, 0.0);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        let mut awakened = 0;
        for seed in 0..30 {
            let mut world = World::new(seed, 23.0);
            world.tuck_in();
            world.hear_thunder();
            if world.ghost.asleep == 0.0 {
                assert!(world.drowsy());
                assert_eq!(world.feeling(), Mood::Sleepy);
                awakened += 1;
            }
        }
        assert!(awakened > 0);
    }
}
