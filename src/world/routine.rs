use super::{Activity, EventKind, World};

/// Consecutive expected hours are one visit, including across midnight.
#[derive(Clone, Copy, Debug)]
struct VisitWindow {
    start: i64,
    end: i64,
}

#[derive(Clone, Debug, Default)]
pub(super) struct Routine {
    waiting: Option<(VisitWindow, i64)>,
    missed_until: Option<i64>,
}

impl World {
    pub(super) fn quiet_routine(&mut self) {
        self.routine.waiting = None;
    }

    pub(super) fn wait_is_open(&self) -> bool {
        !self.viewer_present()
            && self.ghost.asleep < 0.3
            && !self.bedtime()
            && self.expected_window().is_some_and(|window| {
                !self.arrived_in(window)
                    && self.routine.missed_until.is_none_or(|end| window.end > end)
            })
    }

    pub(super) fn begin_wait(&mut self) {
        if self.life.expectation(self.hour) >= 0.5
            && let Some(window) = self.expected_window()
            && !self.arrived_in(window)
            && self.routine.missed_until.is_none_or(|end| window.end > end)
        {
            let until = self.local_hour().floor() as i64 + 1;
            self.routine.waiting.get_or_insert((window, until));
        }
    }

    // The clock, rather than simulation seconds, ends an expected hour. A seeded
    // picture's fixed hour can never turn a wait into a missed visit.
    pub(super) fn advance_routine(&mut self) {
        if let Some((window, until)) = self.routine.waiting
            && self.local_hour() >= until as f64
        {
            self.routine.waiting = None;
            if !self.arrived_in(window)
                && self.routine.missed_until.is_none_or(|end| end < window.end)
            {
                self.routine.missed_until = Some(window.end);
                self.record(EventKind::MissedVisit);
                self.ghost.needs.change(0.0, 0.0, -0.12);
                self.think("They usually come by now...");
            }
        }
        if self.ghost.plan.activity == Activity::Wait && !self.ghost.plan.done {
            self.begin_wait();
        }
    }

    pub(super) fn viewer_arrived(&mut self) {
        if self.focus.is_some() {
            return;
        }
        let expected = self.life.expectation(self.hour) >= 0.5;
        let waiting = self.ghost.plan.activity == Activity::Wait && !self.ghost.plan.done;
        self.viewer_blush_at = Some(self.time);
        self.record(EventKind::ViewerArrived);
        self.deliver_replies(true, 0.0);
        if let Some(milestone) = self.life.arrive(self.hour) {
            self.record(EventKind::Milestone(milestone));
        }
        if self
            .routine
            .waiting
            .is_some_and(|(window, _)| self.arrived_in(window))
        {
            self.routine.waiting = None;
        }
        if expected {
            self.record(EventKind::ExpectedArrival);
            self.ghost.needs.change(0.0, 0.0, 0.15);
            let lines = match &self.life.viewer_name {
                Some(name) if self.speaks_korean() => vec![
                    format!("안녕, {name}!"),
                    "딱 맞춰 왔네!".to_owned(),
                    format!("{name}, 기다렸어."),
                ],
                Some(name) => vec![
                    format!("There you are, {name}!"),
                    "Right on time!".to_owned(),
                    format!("I hoped you'd come, {name}."),
                ],
                None => vec![
                    self.spoken("There you are!", "왔구나!").to_owned(),
                    self.spoken("Right on time!", "딱 맞춰 왔네!").to_owned(),
                    self.spoken("I hoped you'd come.", "기다렸어.").to_owned(),
                ],
            };
            if let Some(index) = self.pick(&[1.0; 3]) {
                self.say(&lines[index]);
            }
        }
        if waiting {
            self.ghost.plan.done = true;
            self.begin(Activity::Visit);
        } else {
            self.greet();
        }
    }

    fn local_hour(&self) -> f64 {
        day_hour(self.date) as f64 + f64::from(self.hour)
    }

    fn expected_window(&self) -> Option<VisitWindow> {
        let now = self.local_hour();
        let hour = [now, now + 1.0 / 3.0]
            .into_iter()
            .find(|hour| self.life.expectation(hour.rem_euclid(24.0) as f32) >= 0.5)?
            .floor() as i64;
        let expected = |hour: i64| self.life.expectation(hour.rem_euclid(24) as f32) >= 0.5;
        let mut start = hour;
        let mut end = hour + 1;
        while end - start < 24 && expected(start - 1) {
            start -= 1;
        }
        while end - start < 24 && expected(end) {
            end += 1;
        }
        if end - start == 24 {
            start = hour - hour.rem_euclid(24);
            end = start + 24;
        }
        Some(VisitWindow { start, end })
    }

    fn arrived_in(&self, window: VisitWindow) -> bool {
        if self.life.last_arrival.as_ref().is_some_and(|(day, hour)| {
            day.parse().is_ok_and(|date| {
                let arrived = day_hour(date) as f64 + f64::from(*hour);
                (window.start as f64 - 1.0 / 3.0..window.end as f64).contains(&arrived)
            })
        }) {
            return true;
        }
        self.life.day.parse().is_ok_and(|date| {
            (0..24).any(|hour| {
                let arrived = day_hour(date) + hour;
                self.life.arrived_hours & (1 << hour) != 0
                    && (window.start..window.end).contains(&arrived)
            })
        })
    }
}

fn day_hour(date: jiff::civil::Date) -> i64 {
    i64::from((date - jiff::civil::Date::constant(1970, 1, 1)).get_days()) * 24
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Mood, ROOM_MAX};

    fn expecting(arrival_hour: f32, hour: f32) -> World {
        let mut world = World::new(6, hour);
        for day in 1..=3 {
            world.life.begin_day(&format!("2026-06-{day:02}"));
            world.life.arrive(arrival_hour);
        }
        world.date = jiff::civil::Date::constant(2026, 6, 4);
        world.life.begin_day(&world.date.to_string());
        world
    }

    #[test]
    fn waiting_is_open_only_when_expected_awake_and_away_before_bedtime() {
        let mut world = expecting(20.0, 20.0);
        assert!(world.options().contains(&Activity::Wait));
        world.hour = 19.5;
        assert!(!world.options().contains(&Activity::Wait));
        world.hour = 19.75;
        assert!(world.options().contains(&Activity::Wait));
        world.hour = 20.0;
        world.ghost.asleep = 0.5;
        assert!(!world.options().contains(&Activity::Wait));
        world.ghost.asleep = 0.0;
        world.notice_viewer();
        assert!(!world.options().contains(&Activity::Wait));
        let world = expecting(23.0, 23.0);
        assert!(world.life.expectation(23.0) >= 0.5);
        assert!(!world.options().contains(&Activity::Wait));
    }

    #[test]
    fn a_wait_drifts_along_the_glass_and_finishes_after_its_full_length() {
        let mut world = expecting(20.0, 20.0);
        world.visitors.friend_visited = Some(world.date);
        world.begin(Activity::Wait);
        let length = world.ghost.plan.length;
        assert!((60.0..=120.0).contains(&length));
        world.step(30.0);
        assert!(!world.ghost.plan.done);
        assert!(world.ghost.plan.count > 1);
        assert!((world.ghost.position.z - (ROOM_MAX.z - 0.45)).abs() < 0.1);
        assert!(world.ghost.position.x.abs() < 0.5);
        world.step(length - 30.0 + 0.1);
        assert!(world.ghost.plan.done);
        assert!(world.ghost.plan.elapsed(world.time) >= length);
    }

    #[test]
    fn an_expected_arrival_delights_the_ghost_and_greets_only_once() {
        for bond in [0.0, 0.6] {
            let mut world = expecting(20.0, 20.0);
            world.life.bond = bond;
            world.ghost.needs.company = 0.4;
            world.begin(Activity::Wait);
            world.step(1.0);
            world.notice_viewer();
            world.step(0.1);
            assert_eq!(world.life.total.expected, 1);
            assert_eq!(world.life.today.expected, 1);
            assert_eq!(world.ghost.mood, Mood::Excited);
            assert!(world.ghost.needs.company > 0.5);
            assert!(world.ghost.speech.is_some());
            assert_eq!(world.ghost.plan.activity, Activity::Visit);
            assert_eq!(world.life.today.times(Activity::Visit), 1);
            world.hour = 22.0;
            world.step(0.1);
            assert_eq!(world.life.total.missed, 0);
        }
    }

    #[test]
    fn an_arrival_stops_waiting_again_in_the_same_window_even_after_reloading() {
        let mut world = expecting(20.0, 20.0);
        world.notice_viewer();
        world.step(0.1);
        assert_eq!(world.life.total.expected, 1);
        world.step(91.0);
        world.notice_viewer();
        world.step(0.1);
        assert_eq!(
            world.life.total.expected, 1,
            "a quiet minute is no new arrival"
        );
        world.step(91.0 + super::super::GONE_AFTER as f32);
        assert!(!world.options().contains(&Activity::Wait));
        let arrivals = world.life.arrivals;
        world.notice_viewer();
        world.step(0.1);
        assert_eq!(world.life.total.expected, 2);
        assert_eq!(world.life.arrivals, arrivals);
        let saved = serde_json::to_string(&world.life).unwrap();
        let mut loaded = World::new(6, 21.0);
        loaded.date = world.date;
        loaded.life = serde_json::from_str(&saved).unwrap();
        assert!(loaded.life.expectation(21.0) >= 0.5);
        assert!(!loaded.options().contains(&Activity::Wait));
    }

    #[test]
    fn a_missed_window_counts_once_even_after_the_wait_itself_finished() {
        let mut world = expecting(20.0, 20.0);
        world.begin(Activity::Wait);
        world.step(121.0);
        assert!(world.ghost.plan.done);
        let company = world.ghost.needs.company;
        world.hour = 21.0;
        world.step(0.1);
        assert_eq!(world.life.total.missed, 1);
        assert_eq!(world.life.today.missed, 1);
        assert!(world.ghost.needs.company < company - 0.1);
        assert_eq!(world.ghost.mood, Mood::Lonely);
        assert_eq!(
            world.ghost.thought.as_ref().unwrap().text,
            "They usually come by now..."
        );
        world.step(1.0);
        assert_eq!(world.life.total.missed, 1);
        world.date = jiff::civil::Date::constant(2026, 6, 5);
        world.step(0.1);
        assert!(
            world
                .life
                .diary
                .last()
                .unwrap()
                .text
                .to_lowercase()
                .contains("waited for you at your usual time, but you didn't come")
        );
    }

    #[test]
    fn early_waits_only_count_if_they_reach_the_expected_hour() {
        let mut world = expecting(20.0, 19.75);
        world.begin(Activity::Wait);
        world.step(121.0);
        world.hour = 21.0;
        world.step(0.1);
        assert_eq!(world.life.total.missed, 0);
        let mut world = expecting(20.0, 19.99);
        world.begin(Activity::Wait);
        world.step(1.0);
        world.hour = 20.0;
        world.step(0.1);
        world.hour = 21.0;
        world.step(0.1);
        assert_eq!(world.life.total.missed, 1);
    }

    #[test]
    fn an_early_arrival_while_waiting_satisfies_the_upcoming_visit() {
        let mut world = expecting(20.0, 19.75);
        world.begin(Activity::Wait);
        world.notice_viewer();
        world.step(0.1);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert_eq!(world.life.total.expected, 0);
        world.step(91.0);
        world.hour = 20.0;
        assert!(!world.options().contains(&Activity::Wait));
        world.hour = 22.0;
        world.step(0.1);
        assert_eq!(world.life.total.missed, 0);
    }

    #[test]
    fn adjacent_expected_hours_share_one_missed_visit() {
        let mut world = expecting(20.0, 20.0);
        world.life.arrivals[21] = world.life.arrivals[20];
        assert!(world.life.expectation(21.0) >= 0.5);
        world.begin(Activity::Wait);
        world.hour = 21.0;
        world.step(0.1);
        assert_eq!(world.life.total.missed, 1);
        assert!(!world.options().contains(&Activity::Wait));
        world.begin(Activity::Wait);
        world.hour = 22.0;
        world.step(0.1);
        assert_eq!(world.life.total.missed, 1);
        world.begin(Activity::Wait);
        world.step(0.1);
        assert_eq!(world.life.total.missed, 1);
    }

    #[test]
    fn a_wait_before_midnight_goes_in_the_day_it_waited() {
        let mut world = expecting(21.0, 21.0);
        world.begin(Activity::Wait);
        world.date = jiff::civil::Date::constant(2026, 6, 5);
        world.hour = 0.0;
        world.step(0.1);
        assert_eq!(world.life.total.missed, 1);
        assert_eq!(world.life.today.missed, 0);
        let yesterday = world.life.diary.last().unwrap();
        assert_eq!(yesterday.day, "2026-06-04");
        assert!(
            yesterday
                .text
                .to_lowercase()
                .contains("waited for you at your usual time")
        );
    }

    #[test]
    fn unwaited_windows_and_fixed_seeded_hours_never_count_as_missed() {
        let mut world = expecting(20.0, 20.0);
        world.hour = 22.0;
        world.step(0.1);
        assert_eq!(world.life.total.missed, 0);
        let mut world = expecting(20.0, 20.0);
        world.begin(Activity::Wait);
        world.step(300.0);
        assert_eq!(world.life.total.missed, 0);
        let mut world = World::new(6, 20.0);
        assert!(!world.options().contains(&Activity::Wait));
        world.step(300.0);
        assert_eq!(world.life.total.expected, 0);
        assert_eq!(world.life.total.missed, 0);
    }

    #[test]
    fn focus_cancels_a_routine_wait_without_turning_work_into_a_missed_visit() {
        let mut world = expecting(20.0, 20.0);
        world.begin(Activity::Wait);
        world.start_focus(25);
        assert!(!world.wait_is_open());
        world.hour = 21.0;
        world.step(0.1);
        assert_eq!(world.life.total.missed, 0);
        assert_eq!(world.life.total.expected, 0);
        assert_eq!(world.ghost.plan.activity, Activity::Company);
        world.end_focus();
        world.step(0.1);
        assert_eq!(world.life.total.missed, 0);
    }
}
