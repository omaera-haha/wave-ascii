use jiff::civil::{Date, DateTime};
use serde::{Deserialize, Deserializer, Serialize};

use super::{
    Activity, EventKind, Heard, Life, Listening, Mood, Quirk, Sky, WINDOW_CENTER, World, flat,
};

const CHILL_THRESHOLD: f32 = 18.0;
const CATCH_CHANCE: f32 = 0.008;
const CARE_HOURS: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Care {
    Stroke,
    Words,
    Lamp,
    Window,
}

impl Care {
    pub fn words(self) -> &'static str {
        match self {
            Self::Stroke => "stroked you",
            Self::Words => "said kind words to you",
            Self::Lamp => "turned on the warm lamp for you",
            Self::Window => "shut the window for you",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cold {
    #[serde(default)]
    pub day: String,
    #[serde(default)]
    pub hour: f32,
    #[serde(default)]
    pub severity: f32,
    #[serde(default)]
    pub care: Vec<Care>,
    /// Simulation seconds left; restore subtracts the actual absence once.
    #[serde(default)]
    pub remaining: f64,
}

impl Cold {
    fn duration(severity: f32) -> f64 {
        (20.0 + 10.0 * f64::from((severity - 0.3) / 0.7)) * 3600.0
    }

    fn valid(&self, today: Date) -> bool {
        self.day.parse::<Date>().is_ok_and(|day| day <= today)
            && self.hour.is_finite()
            && (0.0..24.0).contains(&self.hour)
            && self.severity.is_finite()
            && self.remaining.is_finite()
            && self.remaining > 0.0
    }

    pub fn active_after(&self, now: DateTime, away: i64) -> bool {
        self.valid(now.date())
            && (0.3..=1.0).contains(&self.severity)
            && self.remaining <= Self::duration(self.severity) + 1.0
            && self.remaining > away.max(0) as f64
            && (self.day != now.date().to_string()
                || self.hour
                    <= f32::from(now.hour())
                        + f32::from(now.minute()) / 60.0
                        + f32::from(now.second()) / 3600.0)
    }
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Cold>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

pub(super) fn sanitize(life: &mut Life) {
    let today = life.day.parse::<Date>().ok();
    life.cold = life
        .cold
        .take()
        .filter(|cold| today.is_some_and(|today| cold.valid(today)));
    if let Some(cold) = &mut life.cold {
        cold.severity = cold.severity.clamp(0.3, 1.0);
        let mut care = Vec::new();
        for kind in &cold.care {
            if !care.contains(kind) {
                care.push(*kind);
            }
        }
        cold.care = care;
        cold.remaining = cold
            .remaining
            .min(Cold::duration(cold.severity) - cold.care.len() as f64 * CARE_HOURS * 3600.0);
        life.last_cold = Some(cold.day.clone());
    }
    life.last_cold = life.last_cold.take().filter(|day| {
        day.parse::<Date>()
            .ok()
            .zip(today)
            .is_some_and(|(day, today)| day <= today)
    });
    for tally in [&mut life.today, &mut life.total] {
        tally.colds = tally.colds.min(100_000_000);
        tally.care_received = tally.care_received.min(100_000_000);
        tally.recoveries = tally.recoveries.min(100_000_000);
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct Chill {
    amount: f32,
    checked_at: f64,
    lamp_request: Option<f64>,
    window_request: Option<f64>,
    recovered_at: Option<f64>,
}

impl Life {
    pub fn cold_words(&self) -> String {
        if let Some(cold) = &self.cold {
            let mut words =
                "you have a little cold; rest and warmth help, and you will get better".to_owned();
            if !cold.care.is_empty() {
                let care: Vec<_> = cold.care.iter().map(|kind| kind.words()).collect();
                words.push_str(&format!(
                    "; the viewer {} and you feel a bit better",
                    care.join(" and ")
                ));
            }
            words
        } else if self.total.care_received > 0 {
            "you remember the viewer looking after you when you had a little cold".to_owned()
        } else {
            "you are well".to_owned()
        }
    }
}

impl World {
    fn can_catch_cold(&self) -> bool {
        let age = self.age_days().unwrap_or(self.life.diary.len() as f32);
        self.life.cold.is_none()
            && age >= 3.0
            && self.occasion().is_none()
            && self.life.last_cold.as_ref().is_none_or(|last| {
                last.parse::<Date>()
                    .is_ok_and(|last| (self.date - last).get_days() >= 14)
            })
    }

    fn catch_cold(&mut self) {
        let severity = self.random.range(0.3, 1.0);
        self.life.cold = Some(Cold {
            day: self.date.to_string(),
            hour: self.hour,
            severity,
            care: Vec::new(),
            remaining: Cold::duration(severity),
        });
        self.life.last_cold = Some(self.date.to_string());
        self.life.cold_recovered = false;
        self.chill.amount = 0.0;
        self.chill.lamp_request = None;
        self.chill.window_request = None;
        self.record(EventKind::CaughtCold);
        self.reconsider_wardrobe();
        if self.viewer_present()
            && self.ghost.asleep < 0.3
            && self.focus.is_none()
            && self.telling.is_none()
        {
            self.say(self.spoken("Achoo… I think I caught a cold.", "에취… 감기 걸렸나 봐."));
        }
    }

    pub(super) fn advance_cold(&mut self, seconds: f32) {
        if self.life.cold.is_some() {
            self.elapse_cold(f64::from(seconds), self.viewer_present());
        }
        if self.time - self.chill.checked_at < 1.0 {
            return;
        }
        // Only the small timer runs per slice; exposure and calendar checks run once a second.
        let elapsed = (self.time - self.chill.checked_at).min(2.0) as f32;
        self.chill.checked_at = self.time;
        if self.life.cold.is_some() {
            return;
        }
        let shivering = self
            .active_quirk()
            .is_some_and(|spell| spell.kind == Quirk::Shiver);
        let near_window = self.ghost.position.distance_squared(WINDOW_CENTER) < 1.2 * 1.2;
        let wet_window =
            self.window_open && near_window && matches!(self.weather.sky, Sky::Rain | Sky::Snow);
        let puddle = self.puddle > 0.1
            && self.ghost.position.y < 0.8
            && flat(self.ghost.position - WINDOW_CENTER).length_squared() < 0.8 * 0.8;
        let warm = !self.window_open
            || (self.lamp.on && self.ghost.position.distance_squared(self.lamp.bulb()) < 1.1 * 1.1)
            || self
                .ghost
                .position
                .distance_squared(self.nook.bed(self.size()))
                < 0.9 * 0.9;
        if shivering || wet_window || puddle {
            self.chill.amount = (self.chill.amount + elapsed).min(CHILL_THRESHOLD);
        } else if warm {
            self.chill.amount = (self.chill.amount - elapsed * 0.025).max(0.0);
        }
        if self.chill.amount >= CHILL_THRESHOLD {
            self.chill.amount = 0.0;
            if self.can_catch_cold() && self.random.unit() < CATCH_CHANCE {
                self.catch_cold();
            }
        }
    }

    pub(crate) fn elapse_cold(&mut self, seconds: f64, present: bool) {
        let Some(cold) = &mut self.life.cold else {
            return;
        };
        cold.remaining -= seconds.max(0.0);
        if cold.remaining > 0.0 {
            return;
        }
        let cared = !cold.care.is_empty();
        self.life.cold = None;
        self.life.cold_recovered = false;
        self.chill.amount = 0.0;
        self.record(EventKind::Recovered { cared });
        self.reconsider_wardrobe();
        if present
            && self.ghost.asleep < 0.3
            && self.focus.is_none()
            && self.telling.is_none()
            && !self.reading_viewer_letter()
        {
            self.chill.recovered_at = Some(self.time);
            self.ghost.velocity.y += 0.3;
            self.suggest_mood(Mood::Cheerful, 10.0);
            self.say(self.spoken(
                if cared {
                    "I feel all better. Thank you for looking after me."
                } else {
                    "I feel all better!"
                },
                if cared {
                    "다 나았어. 돌봐 줘서 고마워."
                } else {
                    "이제 다 나았어!"
                },
            ));
        } else {
            self.life.cold_recovered = true;
        }
    }

    pub(super) fn mention_cold_recovery(&mut self) {
        if self.life.cold_recovered
            && self.life.cold.is_none()
            && self.viewer_present()
            && self.ghost.asleep < 0.3
            && self.focus.is_none()
            && self.telling.is_none()
            && self.hunt.is_none()
            && !self.reading_viewer_letter()
            && !self.puzzle_active()
            && self.game.as_ref().is_none_or(|game| game.outcome.is_some())
            && self
                .ghost
                .speech
                .as_ref()
                .is_none_or(|line| self.time - line.time > 10.0)
        {
            self.life.cold_recovered = false;
            self.say(self.spoken(
                "I had a little cold, but I'm better now.",
                "감기에 살짝 걸렸었는데, 이제 다 나았어.",
            ));
        }
    }

    pub(super) fn receive_cold_care(&mut self, kind: Care, korean: Option<bool>) -> bool {
        let Some(cold) = &mut self.life.cold else {
            return false;
        };
        if cold.care.contains(&kind) {
            return false;
        }
        cold.care.push(kind);
        cold.remaining -= CARE_HOURS * 3600.0;
        self.record(EventKind::CaredForMe(kind));
        if self.ghost.asleep < 0.3 && self.telling.is_none() {
            self.suggest_mood(Mood::Cheerful, 5.0);
            let korean = korean.unwrap_or_else(|| self.speaks_korean());
            self.say(if korean {
                "고마워… 좀 나아."
            } else {
                "Thank you… that helps."
            });
        }
        self.elapse_cold(0.0, self.viewer_present());
        true
    }

    pub(super) fn note_cold_request(&mut self, activity: Activity) {
        if self.life.cold.is_some() {
            match activity {
                Activity::Lamp => self.chill.lamp_request = Some(self.ghost.plan.started),
                Activity::Air => self.chill.window_request = Some(self.ghost.plan.started),
                _ => {}
            }
        }
    }

    pub(super) fn cold_lamp_switched(&mut self) {
        if self.chill.lamp_request.take() == Some(self.ghost.plan.started)
            && self.lamp.on
            && self.is_night()
        {
            self.receive_cold_care(Care::Lamp, None);
        }
    }

    pub(super) fn cold_window_moved(&mut self) {
        if self.chill.window_request.take() == Some(self.ghost.plan.started) && !self.window_open {
            self.receive_cold_care(Care::Window, None);
        }
    }

    pub(super) fn hear_cold_care(&mut self, text: &str, korean: bool) -> Option<Heard> {
        self.life.cold.as_ref()?;
        let sneezed = self
            .events
            .iter()
            .rev()
            .take_while(|event| self.time - event.time < 15.0)
            .any(|event| event.kind == EventKind::Quirk(Quirk::Sneeze));
        if !kind_words(text, sneezed) {
            return None;
        }
        self.listening = Listening::default();
        self.record(EventKind::Heard(text.to_owned()));
        if !self.receive_cold_care(Care::Words, Some(korean))
            && self.ghost.asleep < 0.3
            && self.telling.is_none()
        {
            self.say(if korean {
                "응, 조금 쉬면 나을 거야. 고마워."
            } else {
                "A little rest will help. Thank you."
            });
        }
        Some(Heard {
            tone: None,
            understanding: super::Understanding::Cared,
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    pub fn cold_leaning(&self, activity: Activity) -> f32 {
        if self.life.cold.is_none() {
            return 1.0;
        }
        match activity {
            Activity::Play
            | Activity::Chase
            | Activity::Dance
            | Activity::Bubbles
            | Activity::Hide
            | Activity::Build
            | Activity::Fetch
            | Activity::Pounce => 0.45,
            Activity::Rest => 1.6,
            Activity::Read => 1.5,
            Activity::Lamp if !self.lamp.on => 2.0,
            Activity::Air if !self.window_open => 0.0,
            Activity::Lamp => 0.1,
            _ => 1.0,
        }
    }

    pub fn cold_glow(&self) -> f32 {
        self.life
            .cold
            .as_ref()
            .map_or(1.0, |cold| 1.0 - 0.12 * cold.severity)
    }

    pub fn cold_stretch(&self) -> f32 {
        self.chill
            .recovered_at
            .filter(|_| self.ghost.asleep < 0.3)
            .map_or(0.0, |at| {
                let phase = ((self.time - at) / 4.5).clamp(0.0, 1.0) as f32;
                (phase * std::f32::consts::PI).sin().max(0.0)
            })
    }
}

fn kind_words(text: &str, sneezed: bool) -> bool {
    if text.contains(['"', '“', '”']) || super::feelings::denies(text) {
        return false;
    }
    let words = text
        .to_lowercase()
        .split(|character: char| !character.is_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>()
        .join(" ");
    matches!(
        words.as_str(),
        "get well"
            | "get well soon"
            | "feel better"
            | "feel better soon"
            | "are you okay"
            | "are you ok"
            | "괜찮아"
            | "빨리 나아"
            | "아프지 마"
            | "몸조심해"
    ) || (sneezed && words == "bless you")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Needs, Question, Understanding};

    fn winter(seed: u64) -> World {
        let mut world = World::new(seed, 12.0);
        world.date = jiff::civil::date(2026, 12, 5);
        world.birthday = Some(jiff::civil::date(2026, 10, 1));
        world.begin_life("2026-10-01");
        world.begin_day();
        world
    }

    fn sick() -> World {
        let mut world = winter(7);
        world.catch_cold();
        world
    }

    fn expose(world: &mut World) {
        world.weather.sky = Sky::Snow;
        world.window_open = true;
        world.ghost.position = WINDOW_CENTER;
        world.time += 1.0;
        world.advance_cold(1.0);
    }

    #[test]
    fn kind_words_for_a_sick_ghost_are_never_taken_as_another_answer() {
        let mut world = sick();
        world.asking = Some((super::super::acquaintance::Topic::Food, world.time + 180.0));
        let heard = world.hear("get well soon");
        assert_eq!(heard.understanding, Understanding::Cared);
        assert!(world.life.viewer.food.is_none());

        let mut world = sick();
        world.hour = 1.0;
        world.tuck_in();
        world.step(30.0);
        assert!(world.ghost.asleep > 0.5);
        world.hear("are you okay?");
        assert!(world.ghost.asleep > 0.5);
    }

    #[test]
    fn chill_needs_a_threshold_and_catching_repeats_for_the_same_seed() {
        let mut caught = 0;
        for seed in 0..600 {
            let mut first = winter(seed);
            let mut second = winter(seed);
            for _ in 0..17 {
                expose(&mut first);
                expose(&mut second);
                assert!(first.life.cold.is_none());
            }
            expose(&mut first);
            expose(&mut second);
            assert_eq!(first.life.cold, second.life.cold);
            assert_eq!(first.random_unit(), second.random_unit());
            if first.life.cold.is_some() {
                caught += 1;
                assert_eq!(first.life.total.colds, 1);
                assert_eq!(first.life.last_cold.as_deref(), Some("2026-12-05"));
            }
        }
        assert!((1..20).contains(&caught), "{caught}");
    }

    #[test]
    fn colds_wait_fourteen_local_days_and_spare_newborns_and_special_days() {
        let mut world = winter(1);
        assert!(world.can_catch_cold());
        world.life.last_cold = Some("2026-11-22".to_owned());
        assert!(!world.can_catch_cold());
        world.life.last_cold = Some("2026-11-21".to_owned());
        assert!(world.can_catch_cold());
        world.birthday = Some(world.date);
        assert!(!world.can_catch_cold());
        world.birthday = Some(jiff::civil::date(2026, 12, 3));
        assert!(!world.can_catch_cold());
        world.birthday = Some(jiff::civil::date(2026, 12, 2));
        assert!(world.can_catch_cold());
        world.birthday = Some(jiff::civil::date(2025, 12, 5));
        assert!(!world.can_catch_cold());
        world.date = jiff::civil::date(2026, 12, 24);
        assert!(!world.can_catch_cold());
    }

    #[test]
    fn shivers_and_nearby_puddles_add_chill_and_warmth_slowly_removes_it() {
        let mut world = winter(2);
        world.hour = 21.0;
        world.window_open = true;
        for _ in 0..200 {
            world.time += 10.0;
            world.advance_quirks();
            if world
                .active_quirk()
                .is_some_and(|spell| spell.kind == Quirk::Shiver)
            {
                break;
            }
        }
        assert!(
            world
                .active_quirk()
                .is_some_and(|spell| spell.kind == Quirk::Shiver)
        );
        world.advance_cold(1.0);
        let chill = world.chill.amount;
        assert!(chill > 0.0);
        world.quirk = None;
        world.window_open = false;
        world.time += 1.0;
        world.advance_cold(1.0);
        assert!(world.chill.amount > 0.0 && world.chill.amount < chill);
        world.puddle = 0.5;
        world.ghost.position = WINDOW_CENTER;
        world.ghost.position.y = 0.4;
        world.time += 1.0;
        world.advance_cold(1.0);
        assert!(world.chill.amount > chill);
    }

    #[test]
    fn severity_sets_a_day_of_illness_and_each_kind_of_care_shortens_it_once() {
        assert!((Cold::duration(0.3) - 20.0 * 3600.0).abs() < 0.1);
        assert!((Cold::duration(1.0) - 30.0 * 3600.0).abs() < 0.1);
        let mut world = sick();
        let remaining = world.life.cold.as_ref().unwrap().remaining;
        let bond = world.life.bond;
        for kind in [Care::Stroke, Care::Words, Care::Lamp, Care::Window] {
            assert!(world.receive_cold_care(kind, None));
            assert!(!world.receive_cold_care(kind, None));
        }
        let cold = world.life.cold.as_ref().unwrap();
        assert_eq!(cold.remaining, remaining - 8.0 * 3600.0);
        assert_eq!(cold.care.len(), 4);
        assert_eq!(world.life.total.care_received, 4);
        assert!(world.life.bond > bond);
    }

    #[test]
    fn kind_messages_in_both_languages_teach_nothing_and_do_not_wake_sleepers() {
        for text in [
            "get well",
            "feel better",
            "are you okay?",
            "괜찮아?",
            "빨리 나아",
            "아프지 마",
            "몸조심해",
        ] {
            let mut world = sick();
            world.life.korean = 1.0;
            let heard = world.hear(text);
            assert_eq!(heard.understanding, Understanding::Cared, "{text}");
            assert!(heard.learned.is_empty());
            assert!(world.life.vocabulary.known().is_empty());
            assert_eq!(world.life.total.care_received, 1);
            assert_eq!(
                world.ghost.speech.as_ref().unwrap().text,
                if heard.korean {
                    "고마워… 좀 나아."
                } else {
                    "Thank you… that helps."
                }
            );
            world.hear(text);
            assert_eq!(world.life.total.care_received, 1);
        }
        let mut world = sick();
        world.hour = 23.0;
        world.tuck_in();
        world.hear("get well soon");
        world.pet(1.0);
        assert_eq!(world.ghost.asleep, 1.0);
        assert_eq!(world.life.total.care_received, 2);
    }

    #[test]
    fn wellbeing_questions_follow_the_normal_path_when_well_and_bless_you_needs_a_sneeze() {
        for text in ["are you okay?", "괜찮아?"] {
            let mut world = winter(1);
            assert!(matches!(
                world.hear(text).understanding,
                Understanding::Asked {
                    question: Question::Feeling,
                    ..
                }
            ));
        }
        assert!(!kind_words("bless you", false));
        assert!(!kind_words("I do not feel better", true));
        assert!(!kind_words("she said get well", true));
        let mut world = sick();
        world.record(EventKind::Quirk(Quirk::Sneeze));
        assert_eq!(world.hear("bless you").understanding, Understanding::Cared);
        world.life.cold.as_mut().unwrap().care.clear();
        world.time += 16.0;
        assert!(world.hear_cold_care("bless you", false).is_none());
    }

    #[test]
    fn only_viewer_requested_warmth_counts_as_care_after_it_happens() {
        let mut world = sick();
        world.hour = 21.0;
        world.lamp.on = false;
        world.begin(Activity::Lamp);
        world.ghost.position = world.ghost.plan.target;
        world.ghost.plan.arrived = Some(world.time - 3.0);
        world.carry_out(0.1);
        assert!(world.lamp.on);
        assert_eq!(world.life.total.care_received, 0);
        world.lamp.on = false;
        world.ghost.plan.done = true;
        world.heed(Activity::Lamp);
        assert_eq!(world.life.total.care_received, 0);
        world.ghost.position = world.ghost.plan.target;
        world.ghost.plan.arrived = Some(world.time - 3.0);
        world.carry_out(0.1);
        assert!(world.life.cold.as_ref().unwrap().care.contains(&Care::Lamp));
        world.window_open = true;
        assert!(world.ask_window(false));
        assert!(
            !world
                .life
                .cold
                .as_ref()
                .unwrap()
                .care
                .contains(&Care::Window)
        );
        world.ghost.plan.arrived = Some(world.time - 3.0);
        world.carry_out_air();
        assert!(!world.window_open);
        assert!(
            world
                .life
                .cold
                .as_ref()
                .unwrap()
                .care
                .contains(&Care::Window)
        );
    }

    #[test]
    fn cold_tires_an_awake_ghost_a_little_and_slows_sleeping_recovery() {
        let mut well = winter(3);
        let mut unwell = winter(3);
        unwell.catch_cold();
        for asleep in [0.0, 1.0] {
            for world in [&mut well, &mut unwell] {
                world.ghost.needs = Needs {
                    energy: 0.5,
                    fun: 0.5,
                    company: 0.5,
                };
                world.ghost.asleep = asleep;
                world.update_needs(30.0);
            }
            assert!(unwell.ghost.needs.energy < well.ghost.needs.energy);
        }
    }

    #[test]
    fn a_cold_damps_spontaneous_play_but_never_blocks_requests() {
        let mut world = sick();
        let activities = [
            Activity::Play,
            Activity::Chase,
            Activity::Dance,
            Activity::Bubbles,
            Activity::Hide,
            Activity::Build,
            Activity::Fetch,
            Activity::Pounce,
        ];
        for activity in activities {
            assert!(world.cold_leaning(activity) < 1.0);
        }
        assert!(world.cold_leaning(Activity::Read) > 1.0);
        assert!(world.cold_leaning(Activity::Rest) > 1.0);
        world.life.cold = None;
        for activity in activities {
            assert_eq!(world.cold_leaning(activity), 1.0);
        }
        world.catch_cold();
        assert!(world.challenge());
        world.abandon_game();
        world.heed(Activity::Play);
        assert_eq!(world.ghost.plan.activity, Activity::Play);
        assert_eq!(
            world.hear("blow bubbles").understanding,
            Understanding::Bubbles { accepted: true }
        );
        world.date = jiff::civil::date(2026, 6, 15);
        world.hour = 12.0;
        world.weather.sky = Sky::Clear;
        world.weather.storm = false;
        world.wind.strength = 0.0;
        assert!(world.options().contains(&Activity::Air));
        world.heed(Activity::Air);
        assert_eq!(world.ghost.plan.activity, Activity::Air);
        assert!(world.ask_window(true));
    }

    #[test]
    fn recovery_records_once_with_paired_milestones_and_diary_words() {
        let mut world = sick();
        world.pet(1.0);
        world.elapse_cold(31.0 * 3600.0, true);
        world.elapse_cold(31.0 * 3600.0, true);
        assert!(world.life.cold.is_none());
        assert_eq!(world.life.total.recoveries, 1);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Recovered { cared: true })
        );
        assert!(world.life.milestones.iter().any(|entry| entry.text
            == "You looked after me when I had a cold."
            && entry.korean.is_some()));
        assert!(
            world
                .life
                .today
                .words()
                .contains("you looked after me while I was sick")
        );
        assert!(world.life.today.words_korean().contains("감기가 다 나았어"));
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "I feel all better. Thank you for looking after me."
        );
    }

    #[test]
    fn recovery_while_away_survives_a_save_and_is_told_once_after_return() {
        let mut world = sick();
        world.elapse_cold(31.0 * 3600.0, false);
        assert!(world.life.cold_recovered);
        let saved = serde_json::to_string(&world.saved_life()).unwrap();
        world.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
        world.time += 20.0;
        world.notice_viewer();
        world.mention_cold_recovery();
        assert!(!world.life.cold_recovered);
        world.mention_cold_recovery();
        let count = world
            .events
            .iter()
            .filter(
                |event| matches!(&event.kind, EventKind::Said(text) if text.contains("better now")),
            )
            .count();
        assert_eq!(count, 1);
        assert_eq!(world.life.total.recoveries, 1);
    }

    #[test]
    fn old_and_damaged_saves_are_safe_and_active_care_round_trips() {
        let old = serde_json::from_str::<Life>("{}").unwrap().sanitized();
        assert!(old.cold.is_none() && old.last_cold.is_none());
        let mut world = sick();
        world.receive_cold_care(Care::Words, None);
        let life =
            serde_json::from_str::<Life>(&serde_json::to_string(&world.saved_life()).unwrap())
                .unwrap()
                .sanitized();
        assert_eq!(life.cold, world.life.cold);
        for (day, hour, remaining) in [
            ("broken", 12.0, 20.0),
            ("2027-01-01", 12.0, 20.0),
            ("2026-12-05", 24.0, 20.0),
            ("2026-12-05", f32::NAN, 20.0),
            ("2026-12-05", 12.0, f64::INFINITY),
            ("2026-12-05", 12.0, -1.0),
        ] {
            let mut life = world.life.clone();
            let cold = life.cold.as_mut().unwrap();
            cold.day = day.to_owned();
            cold.hour = hour;
            cold.remaining = remaining;
            assert!(life.sanitized().cold.is_none());
        }
        let mut life = world.life.clone();
        let cold = life.cold.as_mut().unwrap();
        cold.severity = 100.0;
        cold.remaining = 1e20;
        cold.care = vec![Care::Words, Care::Stroke, Care::Words];
        life.total.colds = u32::MAX;
        let life = life.sanitized();
        let cold = life.cold.unwrap();
        assert_eq!(cold.severity, 1.0);
        assert_eq!(cold.care, [Care::Words, Care::Stroke]);
        assert!(cold.remaining <= 26.0 * 3600.0);
        assert_eq!(life.total.colds, 100_000_000);
        let life: Life =
            serde_json::from_value(serde_json::json!({ "cold": {"care": ["broken"]} })).unwrap();
        assert!(life.cold.is_none());
    }
}
