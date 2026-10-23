use glam::Vec3;

use super::{Activity, EventKind, Mood, WINDOW_CENTER, World};

/// A work session lives only as long as this open room.
#[derive(Clone, Debug, PartialEq)]
pub struct Focus {
    pub started: f64,
    pub length: u32,
    pub ended_early: bool,
}

impl Focus {
    pub fn remaining(&self, time: f64) -> f64 {
        (f64::from(self.length) - (time - self.started).max(0.0)).max(0.0)
    }

    pub fn minutes_left(&self, time: f64) -> u32 {
        (self.remaining(time) / 60.0).ceil() as u32
    }

    pub fn words_left(&self, time: f64) -> String {
        let minutes = self.minutes_left(time);
        if minutes <= 1 {
            "less than a minute left".to_owned()
        } else {
            let rounded = if minutes < 5 {
                minutes
            } else {
                (minutes + 2) / 5 * 5
            };
            format!("about {} minutes left", super::friend::count_words(rounded))
        }
    }
}

/// A direct invitation to work together, before any words are learned.
pub fn requested(text: &str) -> Option<u32> {
    let text = text.trim().to_lowercase();
    if text.ends_with(['?', '？']) {
        return None;
    }
    let words: Vec<&str> = text
        .split(|character: char| !character.is_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .collect();
    if words.first().is_some_and(|word| {
        matches!(
            *word,
            "can"
                | "could"
                | "would"
                | "will"
                | "do"
                | "does"
                | "is"
                | "are"
                | "why"
                | "what"
                | "how"
        )
    }) {
        return None;
    }
    let english = matches!(
        words.as_slice(),
        ["focus" | "pomodoro", ..]
            | ["let's" | "lets" | "please", "focus", ..]
            | ["let", "us", "focus", ..]
            | ["time", "to", "work", ..]
    );
    let korean = words
        .iter()
        .any(|word| matches!(*word, "집중" | "집중하자" | "공부하자" | "일하자"));
    if !english && !korean {
        return None;
    }
    let minutes = text
        .split(|character: char| !character.is_ascii_digit())
        .find(|word| !word.is_empty())
        .map(|word| word.parse::<u32>().unwrap_or(u32::MAX))
        .unwrap_or(25);
    Some(minutes.clamp(5, 120))
}

pub(crate) fn seeks_attention(activity: Activity) -> bool {
    matches!(
        activity,
        Activity::Game
            | Activity::Puzzle
            | Activity::Pounce
            | Activity::Hide
            | Activity::Visit
            | Activity::Dance
            | Activity::Fetch
            | Activity::Wait
            | Activity::Music
            | Activity::Seek
            | Activity::Bubbles
            | Activity::Show
    )
}

impl World {
    pub fn toggle_focus(&mut self) {
        if self.focus.is_some() {
            self.end_focus();
        } else {
            self.start_focus(25);
        }
    }

    pub fn start_focus(&mut self, minutes: u32) {
        self.cancel_hunt();
        if self.focus.is_some() {
            self.end_focus();
        }
        self.start_rhythm_focus();
        self.focus = Some(Focus {
            started: self.time,
            length: minutes.clamp(5, 120) * 60,
            ended_early: false,
        });
        self.rhythm.presence_started.get_or_insert(self.time);
        self.music = None;
        if self.queued_tune.take().is_some() && self.ghost.plan.activity == Activity::Rest {
            self.ghost.plan.target = self.sleeping_spot();
            self.ghost.plan.arrived = None;
        }
        self.break_until = 0.0;
        self.viewer_present = true;
        self.viewer_gone = false;
        self.absent_since = None;
        self.quiet_routine();
        if self.ghost.asleep < 0.3 && self.ghost.plan.activity != Activity::Rest {
            self.begin(Activity::Company);
        }
    }

    pub fn end_focus(&mut self) {
        let Some(mut focus) = self.focus.take() else {
            return;
        };
        focus.ended_early = focus.remaining(self.time) > 0.0;
        let seconds = (self.time - focus.started).clamp(0.0, f64::from(focus.length));
        let minutes = (seconds / 60.0).floor() as u32;
        self.record(EventKind::FocusEnded {
            minutes,
            completed: !focus.ended_early,
        });
        self.finish_rhythm_focus(!focus.ended_early);
        // Starting and ending company aren't arrivals or departures.
        self.viewer_present = self.viewer_present();
        if !self.viewer_present {
            self.absent_since = Some(self.time);
        }
        if focus.ended_early {
            if self.ghost.plan.activity == Activity::Company {
                self.ghost.plan.done = true;
            }
        } else {
            self.break_until = self.time + 300.0;
            // The session ends with a gentle wake, without a startled WokeUp reaction.
            self.ghost.asleep = 0.0;
            self.drowsy_until = 0.0;
            self.begin(Activity::Visit);
            self.suggest_mood(Mood::Excited, 8.0);
            self.ghost.mood = self.feeling();
            self.say(self.spoken("Time for a break!", "쉬는 시간이야!"));
        }
    }

    pub fn taking_break(&self) -> bool {
        self.focus.is_none() && self.time < self.break_until
    }

    pub(super) fn advance_focus(&mut self) {
        if self
            .focus
            .as_ref()
            .is_some_and(|focus| focus.remaining(self.time) <= 0.0)
        {
            self.end_focus();
        }
    }

    pub(super) fn keep_company(&mut self) -> Option<Vec3> {
        if self.focus.is_none() || self.ghost.needs.energy < 0.22 || self.ghost.needs.fun < 0.08 {
            self.ghost.plan.done = true;
        }
        let elapsed = self.ghost.plan.elapsed(self.time);
        let anchor = self.ghost.plan.anchor;
        self.ghost.plan.target = anchor + Vec3::Y * (elapsed * 0.55).sin() * 0.045;
        Some(if elapsed % 45.0 >= 35.0 && elapsed % 45.0 < 42.0 {
            WINDOW_CENTER
        } else if elapsed % 45.0 >= 42.0 {
            Vec3::new(0.0, 1.2, super::ROOM_MAX.z + 2.0)
        } else {
            anchor + Vec3::new(anchor.x.signum(), 0.25, 3.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{ROOM_MAX, Understanding};

    #[test]
    fn direct_english_and_korean_requests_choose_and_clamp_minutes() {
        for (text, minutes) in [
            ("let's focus", 25),
            ("focus", 25),
            ("focus for 40 minutes", 40),
            ("focus 50", 50),
            ("pomodoro", 25),
            ("time to work", 25),
            ("집중하자", 25),
            ("집중", 25),
            ("40분 집중", 40),
            ("공부하자", 25),
            ("일하자", 25),
            ("focus 1", 5),
            ("focus 200", 120),
            ("999999999999999999분 집중", 120),
        ] {
            assert_eq!(requested(text), Some(minutes), "{text}");
            let mut world = World::new(1, 12.0);
            let heard = world.hear(text);
            assert_eq!(heard.understanding, Understanding::Focusing { minutes });
            assert_eq!(world.focus.as_ref().unwrap().length, minutes * 60);
            assert_eq!(world.ghost.plan.activity, Activity::Company);
        }
        for text in [
            "can you focus?",
            "focus?",
            "집중?",
            "공부하자？",
            "focus 40 ? ",
            "can you focus",
            "what is focus",
            "hi",
            "refocus",
            "I lost focus",
            "don't focus",
            "집중하지 마",
            "작업",
        ] {
            assert_eq!(requested(text), None, "{text}");
            let mut world = World::new(1, 12.0);
            world.hear(text);
            assert!(world.focus.is_none(), "{text}");
        }
    }

    #[test]
    fn starting_with_words_or_a_key_leaves_a_sleeper_asleep() {
        for spoken in [false, true] {
            let mut world = World::new(1, 23.0);
            world.tuck_in();
            if spoken {
                world.hear("집중하자");
            } else {
                world.toggle_focus();
            }
            assert_eq!(world.ghost.asleep, 1.0);
            assert_eq!(world.ghost.plan.activity, Activity::Rest);
            world.step(1.0);
            assert_eq!(world.ghost.asleep, 1.0);
            assert!(
                !world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::WokeUp)
            );
        }
    }

    #[test]
    fn focusing_keeps_background_company_without_greetings_waits_or_pointer_games() {
        let mut world = World::new(1, 12.0);
        world.life.bond = 0.8;
        world.start_focus(25);
        world.look_away(true);
        world.step(100.0);
        assert!(world.viewer_present());
        assert_eq!(world.ghost.plan.activity, Activity::Company);
        assert!(!world.ghost.plan.done);
        assert_eq!(world.ghost.mood, Mood::Content);
        assert!(!world.options().contains(&Activity::Wait));
        world.plant.thirst = 0.7;
        for activity in Activity::ALL
            .into_iter()
            .filter(|activity| seeks_attention(*activity))
        {
            assert!(!world.options().contains(&activity), "{activity:?}");
        }
        for activity in [
            Activity::Company,
            Activity::Float,
            Activity::Rest,
            Activity::Tend,
            Activity::Window,
            Activity::Lamp,
            Activity::Explore,
        ] {
            assert!(world.options().contains(&activity), "{activity:?}");
        }
        world.begin(Activity::Float);
        world.point_at(Some(Vec3::new(-1.0, 1.0, ROOM_MAX.z)));
        world.point_at(Some(Vec3::new(1.0, 1.0, ROOM_MAX.z)));
        assert!(world.viewer.pointer_is_lively());
        assert!(!world.pounce_is_open());
        assert!(!world.ghost.plan.done);
        world.notice_viewer();
        world.step(0.1);
        world.look_away(true);
        world.step(21.0);
        world.end_focus();
        world.step(0.1);
        assert!(!world.events.iter().any(|event| matches!(
            event.kind,
            EventKind::ViewerArrived | EventKind::ViewerLeft | EventKind::ExpectedArrival
        )));
    }

    #[test]
    fn completion_counts_once_wakes_gently_visits_and_invites_a_break_in_both_languages() {
        for korean in [false, true] {
            let mut world = World::new(1, 23.0);
            world.life.korean = if korean { 1.0 } else { 0.0 };
            world.tuck_in();
            world.start_focus(25);
            world.time = 1499.95;
            world.ghost.needs.energy = 0.1;
            let bond = world.life.bond;
            world.step(0.1);
            assert!(world.focus.is_none());
            assert_eq!(world.ghost.asleep, 0.0);
            assert_eq!(world.ghost.plan.activity, Activity::Visit);
            assert_eq!(world.ghost.mood, Mood::Excited);
            assert_eq!(
                world.ghost.speech.as_ref().unwrap().text,
                if korean {
                    "쉬는 시간이야!"
                } else {
                    "Time for a break!"
                }
            );
            assert_eq!(world.life.total.focus_sessions, 1);
            assert_eq!(world.life.today.focus_sessions, 1);
            assert_eq!(world.life.total.focus_minutes, 25);
            assert!(world.life.bond > bond);
            assert!(
                world
                    .life
                    .milestones
                    .iter()
                    .any(|entry| entry.text == "We finished our first focus session together.")
            );
            assert!(world.taking_break());
            assert!(!world.events.iter().any(|event| matches!(
                event.kind,
                EventKind::WokeUp | EventKind::Poked { .. } | EventKind::Thunder
            )));
            world.end_focus();
            world.step(0.1);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| matches!(
                        event.kind,
                        EventKind::FocusEnded {
                            minutes: 25,
                            completed: true
                        }
                    ))
                    .count(),
                1
            );
            world.time += 300.0;
            assert!(!world.taking_break());
        }
    }

    #[test]
    fn ending_early_counts_whole_elapsed_minutes_without_a_completed_session() {
        let mut world = World::new(1, 12.0);
        world.start_focus(25);
        world.time = 139.0;
        world.end_focus();
        assert_eq!(world.life.total.focus_minutes, 2);
        assert_eq!(world.life.total.focus_sessions, 0);
        assert!(!world.taking_break());
        assert!(world.ghost.speech.is_none());
        assert!(world.life.milestones.is_empty());
        assert!(world.ghost.plan.done);
        assert!(world.events.iter().any(|event| event.kind
            == EventKind::FocusEnded {
                minutes: 2,
                completed: false
            }));
        world.end_focus();
        assert_eq!(world.life.total.focus_minutes, 2);
    }

    #[test]
    fn company_yields_to_needs_and_keeps_a_minds_mood_suggestion() {
        let mut world = World::new(1, 12.0);
        world.start_focus(25);
        world.suggest_mood(Mood::Curious, 10.0);
        world.step(0.1);
        assert_eq!(world.ghost.mood, Mood::Curious);
        world.ghost.needs.energy = 0.1;
        world.step(0.1);
        assert!(world.ghost.plan.done);
        assert!(!world.options().contains(&Activity::Company));
        assert!(world.focus.is_some());
    }

    #[test]
    fn remaining_minutes_only_change_at_minute_boundaries_and_words_have_no_digits() {
        let focus = Focus {
            started: 10.0,
            length: 1500,
            ended_early: false,
        };
        assert_eq!(focus.minutes_left(10.0), 25);
        assert_eq!(focus.minutes_left(69.9), 25);
        assert_eq!(focus.minutes_left(70.0), 24);
        assert_eq!(focus.minutes_left(1510.0), 0);
        for time in [10.0, 310.0, 900.0, 1509.0, 2000.0] {
            assert!(
                !focus
                    .words_left(time)
                    .chars()
                    .any(|character| character.is_ascii_digit())
            );
        }
        assert_eq!(focus.words_left(310.0), "about twenty minutes left");
    }
}
