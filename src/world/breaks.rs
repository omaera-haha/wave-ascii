use jiff::civil::Date;

use super::{Activity, EventKind, Heard, Life, Listening, Mood, Understanding, World};

const STRETCH_AFTER: f64 = 120.0 * 60.0;
const SESSION_GAP: f64 = 15.0 * 60.0;
const STREAK_BREAK: f64 = 10.0 * 60.0;
const REPLY_FOR: f64 = 120.0;
const DAILY_SUGGESTIONS: u8 = 2;

#[derive(Clone, Debug, Default)]
pub struct Rhythm {
    pub presence_started: Option<f64>,
    pub focus_streak: u32,
    pub(super) departed: Option<f64>,
    away_started: Option<f64>,
    last_focus_end: Option<f64>,
    skipped_break_at: Option<f64>,
    last_suggested: Option<(bool, f64)>,
    next_stretch_at: f64,
    pub(super) reply_until: Option<f64>,
    pending_long_at: Option<f64>,
    welcome_at: Option<f64>,
    stretch_waiting: bool,
    stretch_started: Option<f64>,
    agreed_at: Option<f64>,
}

fn evening(date: Date, hour: f32) -> Option<Date> {
    if hour < 12.0 {
        date.yesterday().ok()
    } else {
        Some(date)
    }
}

fn late(hour: f32) -> bool {
    (0.5..4.5).contains(&hour)
}

fn reply(text: &str) -> Option<bool> {
    let text = text.trim().to_lowercase();
    if text.ends_with(['?', '？']) {
        return None;
    }
    let text = text.trim_matches(|character: char| !character.is_alphanumeric());
    match text {
        "ok" | "okay" | "sure" | "good idea" | "thanks" | "알았어" | "응" | "그래" | "좋아"
        | "고마워" => Some(true),
        "no" | "later" | "not now" | "busy" | "싫어" | "나중에" | "바빠" => Some(false),
        _ => None,
    }
}

pub(super) fn sanitize(life: &mut Life) {
    let today = life.day.parse::<Date>().ok();
    for saved in [
        &mut life.late_night_said,
        &mut life.goodnight_together_said,
        &mut life.rhythm_day,
    ] {
        *saved = saved.take().filter(|day| {
            day.parse::<Date>()
                .ok()
                .zip(today)
                .is_some_and(|(day, today)| day <= today)
        });
    }
    life.rhythm_suggestions = if life.rhythm_day.is_some() {
        life.rhythm_suggestions.min(DAILY_SUGGESTIONS)
    } else {
        0
    };
    for tally in [&mut life.today, &mut life.total] {
        tally.breaks_suggested = tally.breaks_suggested.min(100_000_000);
        tally.breaks_taken = tally.breaks_taken.min(100_000_000);
        tally.late_nights = tally.late_nights.min(100_000_000);
        tally.goodnights_together = tally.goodnights_together.min(100_000_000);
    }
}

impl World {
    pub(super) fn start_rhythm_focus(&mut self) {
        let continued = self
            .rhythm
            .last_focus_end
            .into_iter()
            .chain(self.rhythm.skipped_break_at)
            .any(|at| self.time - at <= SESSION_GAP);
        if !continued
            || self
                .rhythm
                .away_started
                .is_some_and(|left| self.time - left >= STREAK_BREAK)
        {
            self.rhythm.focus_streak = 0;
        }
        self.rhythm.pending_long_at = None;
        self.rhythm.skipped_break_at = None;
        self.rhythm.away_started = None;
        if self.rhythm.departed.take().is_some() {
            self.rhythm.presence_started = Some(self.time);
        }
    }

    pub(super) fn finish_rhythm_focus(&mut self, completed: bool) {
        if completed {
            self.rhythm.focus_streak = self.rhythm.focus_streak.saturating_add(1);
            self.rhythm.last_focus_end = Some(self.time);
            if self.rhythm.focus_streak.is_multiple_of(4) {
                self.rhythm.pending_long_at = Some(self.time + 5.0);
            }
        }
    }

    pub(super) fn note_rhythm(&mut self, kind: &EventKind) {
        match kind {
            EventKind::ViewerLeft => {
                self.rhythm.presence_started = None;
                self.rhythm.departed = Some(self.time);
                self.rhythm.away_started = Some(self.absent_since.unwrap_or(self.time));
                self.rhythm.welcome_at = None;
                self.rhythm.stretch_waiting = false;
                self.rhythm.stretch_started = None;
            }
            EventKind::ViewerArrived => {
                self.rhythm.presence_started.get_or_insert(self.time);
                if let Some(left) = self.rhythm.departed.take() {
                    let away = self.time - left;
                    if self
                        .rhythm
                        .away_started
                        .take()
                        .is_some_and(|started| self.time - started >= STREAK_BREAK)
                    {
                        self.rhythm.focus_streak = 0;
                    }
                    if let Some((long, suggested)) = self.rhythm.last_suggested
                        && (0.0..=20.0 * 60.0).contains(&(left - suggested))
                        && (5.0 * 60.0..=45.0 * 60.0).contains(&away)
                    {
                        self.rhythm.welcome_at = Some(self.time + 5.0);
                        if long {
                            self.rhythm.focus_streak = 0;
                        }
                        self.rhythm.last_suggested = None;
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn rhythm_quiet(&self) -> bool {
        self.focus.is_none()
            && self.viewer_present()
            && self.telling.is_none()
            && !self.reading_viewer_letter()
            && !self.puzzle_active()
            && !self
                .game
                .as_ref()
                .is_some_and(|game| game.outcome.is_none())
            && self.asking.is_none()
            && self.feeling_follow_up.is_none()
            && !self.awaiting_plan_answer()
            && self.hunt.is_none()
            && self.comforting.is_none()
    }

    /// Whether a break suggestion still waits for the viewer's reply.
    pub fn awaiting_break_reply(&self) -> bool {
        self.rhythm
            .reply_until
            .is_some_and(|until| self.time <= until)
    }

    fn suggestion_available(&mut self) -> bool {
        let day = self.date.to_string();
        if self
            .life
            .rhythm_day
            .as_ref()
            .is_none_or(|saved| saved < &day)
        {
            self.life.rhythm_day = Some(day.clone());
            self.life.rhythm_suggestions = 0;
        }
        self.life.rhythm_day.as_ref() == Some(&day)
            && self.life.rhythm_suggestions < DAILY_SUGGESTIONS
    }

    fn suggest_break(&mut self, long: bool) {
        if !self.rhythm_quiet() || self.ghost.asleep >= 0.3 || !self.suggestion_available() {
            return;
        }
        self.life.rhythm_suggestions += 1;
        self.rhythm.last_suggested = Some((long, self.time));
        self.rhythm.next_stretch_at = self.time + STRETCH_AFTER;
        self.rhythm.reply_until = Some(self.time + REPLY_FOR);
        self.begin(Activity::Visit);
        if long {
            self.say(self.spoken(
                "Four sessions! How about a longer break? Stretch, have some water.",
                "네 번이나 했어! 이번엔 좀 길게 쉬자. 기지개 켜고 물도 마셔.",
            ));
        } else {
            self.rhythm.stretch_waiting = true;
            self.say(self.spoken(
                "We've been here a long while. Let's stretch!",
                "우리 오래 있었다. 기지개 켜자!",
            ));
        }
        self.record(EventKind::SuggestedBreak { long });
    }

    pub(super) fn advance_rhythm(&mut self) {
        if self
            .rhythm
            .away_started
            .is_some_and(|left| self.time - left >= STREAK_BREAK)
        {
            self.rhythm.focus_streak = 0;
        }
        if self.rhythm.stretch_waiting {
            if self.ghost.asleep >= 0.3 || self.ghost.plan.activity != Activity::Visit {
                self.rhythm.stretch_waiting = false;
            } else if self.ghost.plan.arrived.is_some() {
                self.rhythm.stretch_waiting = false;
                self.rhythm.stretch_started = Some(self.time);
            }
        }
        let stretch_due = self.time >= self.rhythm.next_stretch_at
            && self
                .rhythm
                .presence_started
                .is_some_and(|started| self.time - started >= STRETCH_AFTER)
            && self.time - self.viewer.active_at <= 600.0;
        if !stretch_due
            && self.rhythm.welcome_at.is_none_or(|at| self.time < at)
            && self.rhythm.pending_long_at.is_none_or(|at| self.time < at)
        {
            return;
        }
        if !self.rhythm_quiet() || self.ghost.asleep >= 0.3 || self.viewer.typing {
            return;
        }
        if self
            .ghost
            .speech
            .as_ref()
            .is_some_and(|line| self.time - line.time < 5.0)
        {
            return;
        }
        if let Some(at) = self.rhythm.welcome_at {
            if self.time - at > 120.0 {
                self.rhythm.welcome_at = None;
            } else if self.time >= at {
                self.rhythm.welcome_at = None;
                self.say(self.spoken("You took a break! Welcome back.", "쉬고 왔구나! 어서 와."));
                self.record(EventKind::TookBreak);
                return;
            }
        }
        if let Some(at) = self.rhythm.pending_long_at {
            if self.time - at > SESSION_GAP {
                self.rhythm.pending_long_at = None;
            } else if self.time >= at {
                self.rhythm.pending_long_at = None;
                self.suggest_break(true);
                return;
            }
        }
        if stretch_due {
            self.suggest_break(false);
        }
    }

    /// The very next message after a suggestion, whose reply window
    /// `hear` took as the message arrived, may answer it.
    pub(super) fn hear_break_reply(
        &mut self,
        text: &str,
        korean: bool,
        window: Option<f64>,
    ) -> Option<Heard> {
        let until = window?;
        if self.time > until {
            return None;
        }
        let agreed = reply(text)?;
        self.listening = Listening::default();
        self.record(EventKind::Heard(text.to_owned()));
        if agreed {
            if self.ghost.asleep < 0.3 {
                self.rhythm.agreed_at = Some(self.time);
                self.suggest_mood(Mood::Cheerful, 6.0);
            }
        } else {
            if self.rhythm.last_suggested.is_some_and(|(long, _)| long) {
                self.rhythm.skipped_break_at = Some(self.time);
            }
            self.rhythm.next_stretch_at = self.time + 3.0 * 3600.0;
            if self.ghost.asleep < 0.3 {
                self.say(if korean || self.speaks_korean() {
                    "알았어. 여기 있을게."
                } else {
                    "Okay. I'll be right here."
                });
            }
        }
        Some(Heard {
            tone: None,
            understanding: Understanding::BreakReply { agreed },
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    pub(super) fn notice_late_night(&mut self, korean: bool) {
        if !late(self.hour) {
            return;
        }
        if !self.rhythm_quiet() {
            return;
        }
        let Some(night) = evening(self.date, self.hour).map(|day| day.to_string()) else {
            return;
        };
        if self
            .life
            .late_night_said
            .as_ref()
            .is_some_and(|said| said >= &night)
            || !self.suggestion_available()
        {
            return;
        }
        self.life.late_night_said = Some(night);
        self.life.rhythm_suggestions += 1;
        self.rhythm.next_stretch_at = self.rhythm.next_stretch_at.max(self.time + STRETCH_AFTER);
        self.wake();
        self.drowsy_until = self.time + 120.0;
        let nature = self.life.temperament();
        let lines = if nature.tender > 0.6 {
            [
                (
                    "It's so late… won't you sleep too? I'll be here tomorrow.",
                    "너무 늦었어… 너도 이제 자자. 내일도 여기 있을게.",
                ),
                (
                    "Your eyes need a little rest too… shall we sleep?",
                    "너도 눈 좀 쉬어야지… 우리 이제 잘까?",
                ),
            ]
        } else if nature.playful > 0.6 {
            [
                (
                    "Even ghosts need bedtime… you too, sleepyhead!",
                    "유령도 잘 시간이야… 너도 자야지, 잠꾸러기!",
                ),
                (
                    "Still up? The moon and I think it's your bedtime too.",
                    "아직 안 자? 달이랑 나랑 네 잘 시간도 됐대.",
                ),
            ]
        } else {
            [
                (
                    "It's so late… won't you sleep too?",
                    "너무 늦었어… 너도 이제 자자.",
                ),
                (
                    "It's very late… let's rest our eyes together.",
                    "많이 늦었다… 우리 같이 눈 좀 쉬자.",
                ),
            ]
        };
        let index = self.random.pick(&[1.0; 2]).unwrap_or(0);
        let (english, hangul) = lines[index];
        self.say(if korean { hangul } else { english });
        self.record(EventKind::NoticedLateNight);
    }

    pub(super) fn goodnight_together(&mut self) {
        let Some(night) = evening(self.date, self.hour).map(|day| day.to_string()) else {
            return;
        };
        if self.life.late_night_said.as_ref() == Some(&night)
            && self.life.goodnight_together_said.as_ref() != Some(&night)
        {
            self.life.goodnight_together_said = Some(night);
            self.record(EventKind::SaidGoodnightTogether);
        }
    }

    pub fn late_night_just_said(&self) -> bool {
        self.events.last().is_some_and(|event| {
            event.time == self.time && event.kind == EventKind::NoticedLateNight
        })
    }

    pub fn stretch_amount(&self) -> f32 {
        if self.ghost.asleep >= 0.3
            || self.focus.is_some()
            || self.ghost.plan.activity != Activity::Visit
        {
            return 0.0;
        }
        self.rhythm.stretch_started.map_or(0.0, |started| {
            let elapsed = ((self.time - started) / 6.0).clamp(0.0, 1.0) as f32;
            (std::f32::consts::PI * elapsed).sin().max(0.0)
        })
    }

    pub fn rhythm_expression(&self) -> Option<(f64, crate::face::expression::Expression)> {
        if self.stretch_amount() > 0.0 {
            return self
                .rhythm
                .stretch_started
                .map(|at| (at, crate::face::expression::Expression::Stretch));
        }
        self.rhythm
            .agreed_at
            .filter(|at| self.time - at < 4.0)
            .map(|at| (at, crate::face::expression::Expression::Smile))
    }

    pub fn rhythm_words(&self) -> Vec<String> {
        let mut words = Vec::new();
        if self.viewer_present() {
            if self
                .rhythm
                .presence_started
                .is_some_and(|started| self.time - started >= STRETCH_AFTER)
            {
                words.push("you have been together for a long while without a break".to_owned());
            }
            if late(self.hour) {
                words.push("it is very late at night and the viewer is still up".to_owned());
            }
        }
        if self.rhythm.focus_streak > 0 {
            let sessions = super::friend::count_words(self.rhythm.focus_streak);
            words.push(format!(
                "the viewer has finished {sessions} focus {} in a row",
                if self.rhythm.focus_streak == 1 {
                    "session"
                } else {
                    "sessions"
                }
            ));
        }
        words
    }

    pub fn rhythm_thoughts(&self) -> Vec<String> {
        let mut thoughts = Vec::new();
        if self.viewer_present() {
            let name = self.life.viewer_name.as_deref().unwrap_or("They");
            if self.rhythm.focus_streak >= 3 {
                thoughts.push(format!(
                    "{name} {} been working so hard.",
                    if name == "They" { "have" } else { "has" }
                ));
            }
            if late(self.hour) {
                thoughts.push(format!("It's late. {name} should sleep."));
            }
        }
        thoughts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{GONE_AFTER, Tone};

    fn present() -> World {
        let mut world = World::new(7, 12.0);
        world.begin_life(&world.date.to_string());
        world.notice_viewer();
        world.step(0.1);
        world.ghost.speech = None;
        world
    }

    fn sitting() -> World {
        let mut world = present();
        world.time += STRETCH_AFTER;
        world.notice_viewer();
        world
    }

    fn finish(world: &mut World) {
        world.start_focus(25);
        world.time += 25.0 * 60.0;
        world.notice_viewer();
        world.end_focus();
    }

    #[test]
    fn only_the_very_next_message_can_answer_a_suggestion_and_requests_spare_sleepers() {
        let mut world = sitting();
        world.suggest_break(false);
        assert!(world.awaiting_break_reply());
        world.hear("show me something");
        assert!(!world.awaiting_break_reply());
        assert_ne!(
            world.hear("no").understanding,
            Understanding::BreakReply { agreed: false }
        );

        for request in ["fold a crane", "bubbles!"] {
            let mut world = present();
            world.hour = 1.0;
            world.tuck_in();
            world.step(30.0);
            assert!(world.ghost.asleep > 0.5);
            world.hear(request);
            assert!(world.ghost.asleep > 0.5, "{request}");
        }
    }

    #[test]
    fn sessions_extend_within_fifteen_minutes_and_every_fourth_completion_suggests_a_long_break() {
        let mut world = present();
        world.rhythm.next_stretch_at = f64::INFINITY;
        for session in 1..=8 {
            finish(&mut world);
            assert_eq!(world.rhythm.focus_streak, session);
            world.time += 6.0;
            world.rhythm.next_stretch_at = f64::INFINITY;
            world.advance_rhythm();
            assert_eq!(world.life.total.breaks_suggested, session / 4);
            world.time += SESSION_GAP - 6.0;
        }
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::SuggestedBreak { long: true })
        );
        assert_eq!(world.life.total.focus_sessions, 8);
        world.time += 0.1;
        finish(&mut world);
        assert_eq!(world.rhythm.focus_streak, 1);
    }

    #[test]
    fn early_sessions_do_not_count_and_a_ten_minute_absence_resets_the_streak() {
        let mut world = present();
        finish(&mut world);
        world.start_focus(25);
        world.time += 300.0;
        world.end_focus();
        assert_eq!(world.rhythm.focus_streak, 1);
        world.record(EventKind::ViewerLeft);
        world.time += STREAK_BREAK - 0.1;
        world.advance_rhythm();
        assert_eq!(world.rhythm.focus_streak, 1);
        world.time += 0.1;
        world.advance_rhythm();
        assert_eq!(world.rhythm.focus_streak, 0);
        finish(&mut world);
        assert_eq!(world.rhythm.focus_streak, 1);
    }

    #[test]
    fn only_a_real_departure_breaks_unbroken_presence() {
        let mut world = present();
        let started = world.rhythm.presence_started;
        world.look_away(true);
        world.step(30.0);
        world.look_away(false);
        world.step(0.1);
        assert_eq!(world.rhythm.presence_started, started);
        world.look_away(true);
        world.step(GONE_AFTER as f32 + 21.0);
        assert_eq!(world.rhythm.presence_started, None);
        assert!(world.rhythm.departed.is_some());
        world.look_away(false);
        world.step(0.1);
        assert!(world.rhythm.presence_started.unwrap() > started.unwrap());
    }

    #[test]
    fn a_ninety_minute_sitting_suggests_stretching_and_spaces_repeats() {
        let mut world = sitting();
        world.time = world.rhythm.presence_started.unwrap() + STRETCH_AFTER - 0.1;
        world.advance_rhythm();
        assert_eq!(world.life.total.breaks_suggested, 0);
        world.time += 0.1;
        world.advance_rhythm();
        assert_eq!(world.life.total.breaks_suggested, 1);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::SuggestedBreak { long: false })
        );
        world.time += STRETCH_AFTER - 0.1;
        world.notice_viewer();
        world.advance_rhythm();
        assert_eq!(world.life.total.breaks_suggested, 1);
        world.time += 0.1;
        world.advance_rhythm();
        assert_eq!(world.life.total.breaks_suggested, 2);
        assert!((0.0..=1.0).contains(&world.stretch_amount()));
    }

    #[test]
    fn suggestions_wait_for_awake_present_quiet_time_and_recent_input() {
        let blockers: [fn(&mut World); 11] = [
            |world| world.start_focus(25),
            |world| world.ghost.asleep = 1.0,
            |world| world.viewer.active_at = 0.0,
            |world| world.viewer.active_at = world.time - 601.0,
            |world| {
                world.tell(Vec::new());
            },
            |world| {
                world.challenge();
            },
            |world| {
                world.asking = Some((super::super::acquaintance::Topic::Color, world.time + 120.0))
            },
            |world| {
                world.feeling_follow_up = Some((super::super::Feeling::Sad, world.time + 120.0))
            },
            |world| world.viewer.typing = true,
            |world| {
                world.hear("let's play a word game");
            },
            |world| {
                world.read_aloud(false);
            },
        ];
        for block in blockers {
            let mut world = sitting();
            block(&mut world);
            world.ghost.speech = None;
            world.advance_rhythm();
            assert_eq!(world.life.total.breaks_suggested, 0);
        }
    }

    #[test]
    fn a_long_break_never_interrupts_the_next_focus_session_or_a_sleeping_ghost() {
        for sleep in [false, true] {
            let mut world = present();
            for _ in 0..4 {
                finish(&mut world);
            }
            world.time += 10.0;
            if sleep {
                world.ghost.asleep = 1.0;
            } else {
                world.start_focus(25);
            }
            world.advance_rhythm();
            assert_eq!(world.life.total.breaks_suggested, 0);
        }
    }

    #[test]
    fn break_replies_in_both_languages_bypass_learning_praise_scolding_and_work_requests() {
        for text in [
            "ok",
            "okay!",
            "sure",
            "good idea",
            "thanks",
            "알았어",
            "응",
            "그래",
            "좋아",
            "고마워",
            "no",
            "later",
            "not now",
            "busy",
            "싫어",
            "나중에",
            "바빠",
        ] {
            let mut world = sitting();
            world.suggest_break(false);
            let vocabulary = world.life.vocabulary.clone();
            let agreed = reply(text).unwrap();
            let heard = world.hear(text);
            assert_eq!(
                heard.understanding,
                Understanding::BreakReply { agreed },
                "{text}"
            );
            assert_eq!(heard.tone, None);
            assert!(heard.learned.is_empty());
            assert_eq!(world.life.vocabulary, vocabulary, "{text}");
            assert_eq!(world.life.total.praises, 0);
            assert_eq!(world.life.total.scolds, 0);
            assert_eq!(world.listening, Listening::default());
            if !agreed {
                assert_eq!(world.rhythm.next_stretch_at, world.time + 3.0 * 3600.0);
                world.time += 3.0 * 3600.0 - 0.1;
                world.notice_viewer();
                world.advance_rhythm();
                assert_eq!(world.life.total.breaks_suggested, 1);
                world.time += 0.1;
                world.advance_rhythm();
                assert_eq!(world.life.total.breaks_suggested, 2);
            }
        }
    }

    #[test]
    fn only_the_next_message_within_two_minutes_can_answer_a_break_suggestion() {
        for (gap, expected) in [(120.0, true), (120.1, false)] {
            let mut world = sitting();
            world.suggest_break(false);
            world.time += gap;
            let heard = world.hear("okay");
            assert_eq!(
                matches!(heard.understanding, Understanding::BreakReply { .. }),
                expected
            );
        }
        let mut world = sitting();
        world.suggest_break(false);
        world.hear("what is your name?");
        assert!(!matches!(
            world.hear("okay").understanding,
            Understanding::BreakReply { .. }
        ));
        for text in [
            "okay?",
            "thanks for the story",
            "no spin",
            "I'm busy",
            "응?",
            "좋아?",
        ] {
            assert_eq!(reply(text), None, "{text}");
        }
    }

    #[test]
    fn late_night_messages_and_pokes_notice_only_the_half_past_midnight_to_half_past_four_window() {
        for hour in [0.0, 0.49, 0.5, 1.0, 4.49, 4.5, 5.0, 23.9] {
            for poke in [false, true] {
                let mut world = World::new(3, hour);
                world.notice_viewer();
                world.tuck_in();
                if poke {
                    world.poke();
                } else {
                    world.hear("hello");
                }
                assert_eq!(
                    world.life.total.late_nights,
                    u32::from(late(hour)),
                    "{hour}, poke {poke}"
                );
                if late(hour) {
                    assert_eq!(world.ghost.asleep, 0.0);
                    assert!(world.drowsy());
                    world.time += 1801.0;
                    world.hear("hello again");
                    world.poke();
                    assert_eq!(world.life.total.late_nights, 1);
                }
            }
        }
    }

    #[test]
    fn the_evening_key_survives_midnight_and_a_saved_life_without_repeating_the_nudge() {
        let mut world = present();
        let evening_day = world.date;
        assert_eq!(evening(evening_day, 23.0), Some(evening_day));
        world.date = evening_day.tomorrow().unwrap();
        world.hour = 1.0;
        assert_eq!(evening(world.date, world.hour), Some(evening_day));
        world.hear("안녕");
        assert_eq!(world.life.late_night_said, Some(evening_day.to_string()));
        assert!(
            world
                .ghost
                .speech
                .as_ref()
                .unwrap()
                .text
                .chars()
                .any(|character| ('가'..='힣').contains(&character))
        );
        let saved = serde_json::to_string(&world.life).unwrap();
        let mut restored = World::new(3, 2.0);
        restored.date = world.date;
        restored.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
        restored.notice_viewer();
        restored.hear("hello");
        assert_eq!(restored.life.total.late_nights, 1);
        restored.date = restored.date.tomorrow().unwrap();
        restored.hear("hello");
        assert_eq!(restored.life.total.late_nights, 2);
    }

    #[test]
    fn a_late_goodnight_counts_once_per_night_and_keeps_the_existing_bedtime_path() {
        for goodnight in ["goodnight", "잘자"] {
            let mut world = World::new(3, 1.0);
            world.notice_viewer();
            world.hear("hello");
            world.time += 1.0;
            let vocabulary = world.life.vocabulary.clone();
            let heard = world.hear(goodnight);
            assert_eq!(heard.tone, Some(Tone::Farewell));
            assert_eq!(world.ghost.plan.activity, Activity::Rest);
            assert_eq!(world.life.total.goodnights_together, 1);
            assert_eq!(world.life.vocabulary, vocabulary);
            world.time += 1.0;
            world.hear(goodnight);
            assert_eq!(world.life.total.goodnights_together, 1);
            let saved = serde_json::to_string(&world.life).unwrap();
            world.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
            world.hear(goodnight);
            assert_eq!(world.life.total.goodnights_together, 1);
        }
    }

    #[test]
    fn welcomes_wait_until_after_arrival_and_only_count_five_to_forty_five_minute_breaks() {
        for (departure, away, expected) in [
            (1200.0, 300.0, true),
            (1200.1, 300.0, false),
            (10.0, 299.9, false),
            (10.0, 2700.0, true),
            (10.0, 2700.1, false),
        ] {
            let mut world = sitting();
            world.suggest_break(true);
            world.rhythm.focus_streak = 4;
            world.time += departure;
            world.record(EventKind::ViewerLeft);
            world.time += away;
            world.notice_viewer();
            world.record(EventKind::ViewerArrived);
            assert_eq!(world.life.total.breaks_taken, 0);
            world.say("Hello again!");
            world.time += 5.1;
            world.advance_rhythm();
            assert_eq!(
                world.life.total.breaks_taken,
                u32::from(expected),
                "{departure}, {away}"
            );
            world.time += 10.0;
            world.advance_rhythm();
            assert_eq!(world.life.total.breaks_taken, u32::from(expected));
            if expected {
                assert_eq!(world.rhythm.focus_streak, 0);
                assert!(
                    world
                        .life
                        .milestones
                        .iter()
                        .any(|entry| entry.text == "You took a break when I asked.")
                );
            }
        }
    }

    #[test]
    fn the_daily_cap_includes_night_nudges_survives_restarts_and_resets_on_the_next_day() {
        let mut world = sitting();
        world.suggest_break(false);
        world.time += STRETCH_AFTER;
        world.notice_viewer();
        world.hour = 1.0;
        world.hear("hello");
        assert_eq!(world.life.rhythm_suggestions, 2);
        world.ghost.speech = None;
        world.suggest_break(true);
        assert_eq!(world.life.total.breaks_suggested, 1);
        let saved = serde_json::to_string(&world.life).unwrap();
        let mut restored = sitting();
        restored.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
        restored.suggest_break(false);
        assert_eq!(restored.life.total.breaks_suggested, 1);
        restored.date = restored.date.tomorrow().unwrap();
        restored.begin_day();
        restored.suggest_break(false);
        assert_eq!(restored.life.total.breaks_suggested, 2);
        assert_eq!(restored.life.rhythm_suggestions, 1);
    }

    #[test]
    fn damaged_rhythm_saves_are_bounded_and_old_saves_default_to_no_reminders() {
        let old: Life = serde_json::from_str("{}").unwrap();
        assert_eq!(old.rhythm_suggestions, 0);
        assert_eq!(old.late_night_said, None);
        let mut world = present();
        world.life.late_night_said = Some("bad date".to_owned());
        world.life.goodnight_together_said = Some(world.date.tomorrow().unwrap().to_string());
        world.life.rhythm_day = Some(world.date.to_string());
        world.life.rhythm_suggestions = u8::MAX;
        world.life.total.breaks_suggested = u32::MAX;
        world.life.total.breaks_taken = u32::MAX;
        world.life.total.late_nights = u32::MAX;
        world.life.total.goodnights_together = u32::MAX;
        let clean = world.life.sanitized();
        assert_eq!(clean.late_night_said, None);
        assert_eq!(clean.goodnight_together_said, None);
        assert_eq!(clean.rhythm_suggestions, DAILY_SUGGESTIONS);
        assert_eq!(clean.total.breaks_suggested, 100_000_000);
        assert_eq!(clean.total.breaks_taken, 100_000_000);
        assert_eq!(clean.total.late_nights, 100_000_000);
        assert_eq!(clean.total.goodnights_together, 100_000_000);
    }

    #[test]
    fn care_events_warm_the_bond_and_reach_the_diary_in_words() {
        let mut world = present();
        let before = world.life.bond;
        for event in [
            EventKind::SuggestedBreak { long: false },
            EventKind::TookBreak,
            EventKind::NoticedLateNight,
            EventKind::SaidGoodnightTogether,
        ] {
            assert!(
                !event
                    .words()
                    .chars()
                    .any(|character| character.is_ascii_digit())
            );
            world.record(event);
        }
        assert!(world.life.bond > before);
        assert_eq!(world.life.total.breaks_taken, 1);
        assert_eq!(world.life.today.breaks_suggested, 1);
        let diary = world.life.today.words().to_lowercase();
        for phrase in [
            "you took a break when i asked",
            "you stayed up very late",
            "we said goodnight together late at night",
        ] {
            assert!(diary.contains(phrase), "{diary}");
        }
        world.rhythm.focus_streak = 3;
        world.life.viewer_name = Some("Sam".to_owned());
        world.hour = 1.0;
        assert!(
            world
                .rhythm_words()
                .iter()
                .any(|words| words.contains("three focus sessions"))
        );
        assert!(
            world
                .rhythm_thoughts()
                .iter()
                .any(|thought| thought == "Sam has been working so hard.")
        );
        assert!(
            world
                .rhythm_thoughts()
                .iter()
                .any(|thought| thought == "It's late. Sam should sleep.")
        );
    }

    #[test]
    fn the_ten_minute_reset_includes_the_departure_hysteresis() {
        let mut world = present();
        world.rhythm.focus_streak = 3;
        world.look_away(true);
        world.step(619.0);
        assert!(world.rhythm.departed.is_some());
        assert_eq!(world.rhythm.focus_streak, 3);
        world.step(2.0);
        assert_eq!(world.rhythm.focus_streak, 0);
    }

    #[test]
    fn welcome_back_waits_for_quiet_awake_time_and_night_nudges_leave_work_and_stories_alone() {
        for sleeping in [false, true] {
            let mut world = sitting();
            world.suggest_break(true);
            world.time += 60.0;
            world.record(EventKind::ViewerLeft);
            world.time += 600.0;
            world.notice_viewer();
            world.record(EventKind::ViewerArrived);
            world.time += 6.0;
            if sleeping {
                world.ghost.asleep = 1.0;
            } else {
                world.start_focus(5);
            }
            world.advance_rhythm();
            assert_eq!(world.life.total.breaks_taken, 0);
        }
        for story in [false, true] {
            let mut world = present();
            world.hour = 1.0;
            if story {
                world.tell(Vec::new());
            } else {
                world.start_focus(25);
            }
            world.hear("hello");
            assert_eq!(world.life.total.late_nights, 0);
        }
    }

    #[test]
    fn a_skipped_long_break_can_anchor_the_next_session_within_fifteen_minutes() {
        let mut world = present();
        for _ in 0..4 {
            finish(&mut world);
        }
        world.time += 6.0;
        world.advance_rhythm();
        world.time += 119.0;
        world.hear("not now");
        world.time += SESSION_GAP;
        finish(&mut world);
        assert_eq!(world.rhythm.focus_streak, 5);
    }
}
