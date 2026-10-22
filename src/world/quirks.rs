use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, Mood, ROOM_MAX, ROOM_MIN, Season, World, flat};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quirk {
    Hiccups,
    Sneeze,
    Yawn,
    Shiver,
    Sigh,
}

impl Quirk {
    pub const ALL: [Self; 5] = [
        Self::Hiccups,
        Self::Sneeze,
        Self::Yawn,
        Self::Shiver,
        Self::Sigh,
    ];

    pub fn words(self) -> &'static str {
        match self {
            Self::Hiccups => "got the hiccups",
            Self::Sneeze => "sneezed",
            Self::Yawn => "yawned",
            Self::Shiver => "shivered with cold",
            Self::Sigh => "sighed contentedly",
        }
    }

    pub fn ongoing(self) -> &'static str {
        match self {
            Self::Hiccups => "you have the hiccups",
            Self::Sneeze => "your nose tickles and you are sneezing",
            Self::Yawn => "you are stretching through a slow yawn",
            Self::Shiver => "you are shivering with cold",
            Self::Sigh => "you are letting out a contented sigh",
        }
    }

    pub fn thoughts(self) -> &'static [&'static str] {
        match self {
            Self::Hiccups => &["Hic! …excuse me.", "My whole body keeps jumping."],
            Self::Sneeze => &["Achoo! Something tickles.", "Even I get a tickly nose."],
            Self::Yawn => &["Haaah… so sleepy.", "That yawn caught me too."],
            Self::Shiver => &[
                "Brr. The lamp looks warm.",
                "Perhaps I should shut the window.",
            ],
            Self::Sigh => &[
                "Aah… that feels nice.",
                "A little softer, a little lighter.",
            ],
        }
    }

    pub fn diary(self, count: u32) -> String {
        let deed = if self == Self::Hiccups {
            "had the hiccups"
        } else {
            self.words()
        };
        match count {
            1 => deed.to_owned(),
            2 => format!("{deed} twice"),
            _ => format!("{deed} {} times", super::friend::count_words(count)),
        }
    }
}

/// One spell owns its impulses as well as the times used by the face and pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spell {
    pub kind: Quirk,
    pub started: f64,
    pub ends: f64,
    pulse_at: f64,
    next_at: f64,
    remaining: u8,
    twice: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Motion {
    pub lift: f32,
    pub lean: f32,
    pub stretch: f32,
}

impl Spell {
    pub fn pulse_started(self, time: f64) -> f64 {
        if self.kind == Quirk::Sneeze && self.twice && time >= self.started + 2.2 {
            self.started + 2.2
        } else {
            self.pulse_at
        }
    }

    pub fn envelope(self, time: f64) -> f32 {
        let elapsed = (time - self.started).max(0.0) as f32;
        let left = (self.ends - time).max(0.0) as f32;
        let amount = elapsed.min(left).clamp(0.0, 1.0);
        amount * amount * (3.0 - 2.0 * amount)
    }

    pub fn motion(self, time: f64) -> Motion {
        if !(self.started..self.ends).contains(&time) {
            return Motion::default();
        }
        let elapsed = (time - self.pulse_started(time)) as f32;
        let swell = |duration: f32| {
            (std::f32::consts::PI * (elapsed / duration).clamp(0.0, 1.0))
                .sin()
                .max(0.0)
        };
        match self.kind {
            Quirk::Hiccups => Motion {
                stretch: 0.035 * swell(0.65),
                ..Motion::default()
            },
            Quirk::Sneeze => Motion {
                lean: if elapsed < 1.0 {
                    -0.065 * swell(2.0)
                } else {
                    0.09 * ((1.8 - elapsed) / 0.8).clamp(0.0, 1.0)
                },
                stretch: 0.025 * swell(1.8),
                ..Motion::default()
            },
            Quirk::Yawn => Motion {
                lift: 0.025 * swell(4.5),
                stretch: 0.065 * swell(4.5),
                ..Motion::default()
            },
            Quirk::Shiver => {
                let phase = (time.rem_euclid(1.0) * std::f64::consts::TAU) as f32;
                let envelope = self.envelope(time);
                Motion {
                    lift: 0.006 * (phase * 13.0).sin() * envelope,
                    lean: 0.022 * (phase * 17.0).sin() * envelope,
                    ..Motion::default()
                }
            }
            Quirk::Sigh => Motion {
                lift: -0.055 * swell(4.0),
                stretch: -0.025 * swell(4.0),
                ..Motion::default()
            },
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Triggers {
    next_check: f64,
    ready_at: f64,
    yawn_at: Option<f64>,
    yawned_until: f64,
    dust_until: f64,
    stroke_at: Option<f64>,
    stroke_travel: f32,
}

impl Default for Triggers {
    fn default() -> Self {
        Self {
            next_check: 8.0,
            ready_at: 0.0,
            yawn_at: None,
            yawned_until: 0.0,
            dust_until: 0.0,
            stroke_at: None,
            stroke_travel: 0.0,
        }
    }
}

/// A performed yawn, rather than a word embedded in a sentence or a question.
pub fn yawn_message(text: &str) -> bool {
    let sound = text
        .trim_matches(|character: char| !character.is_alphanumeric())
        .to_lowercase();
    if sound.len() > 80 || sound.chars().any(|character| !character.is_alphabetic()) {
        return false;
    }
    let english = sound
        .strip_suffix("ing")
        .or_else(|| sound.strip_suffix('s'))
        .unwrap_or(&sound);
    if let Some(vowels) = english
        .strip_prefix('y')
        .and_then(|word| word.strip_suffix("wn"))
        && !vowels.is_empty()
        && vowels.chars().all(|character| character == 'a')
    {
        return true;
    }
    let korean = sound.replace("ㅎㅏ", "하").replace("ㅇㅏ", "아");
    if matches!(korean.as_str(), "하품" | "하품중" | "하품한다" | "하품해") {
        return true;
    }
    korean
        .strip_prefix('하')
        .and_then(|word| word.strip_suffix('암'))
        .is_some_and(|middle| middle.chars().all(|character| character == '아'))
}

pub(super) fn delighted(event: &EventKind) -> bool {
    matches!(
        event,
        EventKind::Poked { count: 2 }
            | EventKind::PokedFriend
            | EventKind::PlayedWithFriend(_)
            | EventKind::Danced
            | EventKind::Pounced
            | EventKind::NudgedBall
            | EventKind::Praised
    )
}

impl World {
    fn quirks_allowed(&self) -> bool {
        self.ghost.asleep < 0.3
            && self.telling.is_none()
            && !matches!(self.ghost.plan.activity, Activity::Game | Activity::Puzzle)
            && self.game.as_ref().is_none_or(|game| game.outcome.is_some())
            && self
                .puzzle
                .as_ref()
                .is_none_or(|puzzle| puzzle.ended.is_some())
    }

    pub fn active_quirk(&self) -> Option<Spell> {
        self.quirk.filter(|spell| {
            self.quirks_allowed() && (spell.started..spell.ends).contains(&self.time)
        })
    }

    pub fn quirk_words(&self) -> &'static str {
        self.active_quirk()
            .map_or("none just now", |spell| spell.kind.ongoing())
    }

    pub fn quirk_leaning(&self, activity: Activity) -> f32 {
        if self
            .active_quirk()
            .is_some_and(|spell| spell.kind == Quirk::Shiver)
        {
            match activity {
                Activity::Air if self.window_open => return 3.0,
                Activity::Lamp if !self.lamp.on => return 8.0,
                Activity::Lamp => return 0.1,
                Activity::Rest => return 4.0,
                _ => {}
            }
        }
        if activity == Activity::Rest
            && self.bedtime()
            && self.time < self.quirk_triggers.yawned_until
        {
            2.5
        } else {
            1.0
        }
    }

    fn start_quirk(&mut self, kind: Quirk) -> bool {
        if !self.quirks_allowed() || self.quirk.is_some() {
            return false;
        }
        let twice = kind == Quirk::Sneeze && self.random.unit() < 0.25;
        let duration = match kind {
            Quirk::Hiccups => self.random.range(20.0, 40.0),
            Quirk::Sneeze if twice => 4.0,
            Quirk::Sneeze => 1.8,
            Quirk::Yawn => 4.5,
            Quirk::Shiver => self.random.range(5.0, 9.0),
            Quirk::Sigh => 4.0,
        };
        let next_at = match kind {
            Quirk::Hiccups => self.time + f64::from(self.random.range(2.5, 5.0)),
            Quirk::Sneeze => self.time + 1.0,
            _ => f64::INFINITY,
        };
        self.quirk = Some(Spell {
            kind,
            started: self.time,
            ends: self.time + f64::from(duration),
            pulse_at: self.time,
            next_at,
            remaining: if twice { 2 } else { 1 },
            twice,
        });
        let (low, high) = if kind == Quirk::Sneeze && self.life.cold.is_some() {
            (90.0, 180.0)
        } else {
            rest_after(kind)
        };
        self.quirk_triggers.ready_at =
            self.time + f64::from(duration + self.random.range(low, high));
        self.quirk_triggers.yawn_at = None;
        if kind == Quirk::Hiccups {
            self.ghost.velocity.y += 0.35;
        }
        if kind == Quirk::Yawn {
            self.quirk_triggers.yawned_until = self.time + 60.0;
        }
        self.record(EventKind::Quirk(kind));
        if kind == Quirk::Sneeze
            && self.life.cold.is_some()
            && self.viewer_present()
            && self.focus.is_none()
            && self.random.unit() < 0.1
        {
            self.say(self.spoken("Sniff… I’ll be better soon.", "훌쩍… 곧 나을 거야."));
        }
        true
    }

    pub(super) fn quirk_after_event(&mut self) {
        if !self.quirks_allowed()
            || self.quirk.is_some()
            || self.time < self.quirk_triggers.ready_at
        {
            return;
        }
        let recent = || {
            self.events
                .iter()
                .rev()
                .take_while(|event| self.time - event.time <= 30.0)
        };
        let motes = recent()
            .filter(|event| event.kind == EventKind::AteMote)
            .take(3)
            .count();
        let laughter = recent()
            .filter(|event| self.time - event.time <= 20.0 && delighted(&event.kind))
            .take(4)
            .count();
        if (motes >= 3 || laughter >= 4) && self.random.unit() < 0.25 {
            self.start_quirk(Quirk::Hiccups);
        }
    }

    pub(super) fn cure_hiccups(&mut self, speak: bool) {
        if self.quirk.is_some_and(|spell| spell.kind == Quirk::Hiccups) {
            self.quirk = None;
            if speak && self.quirks_allowed() {
                self.say(self.spoken("…oh! They're gone.", "…어! 딸꾹질이 멎었어."));
            }
        }
    }

    pub(super) fn catch_yawn(&mut self) {
        if self.quirks_allowed() && self.quirk.is_none() && self.quirk_triggers.yawn_at.is_none() {
            self.quirk_triggers.yawn_at = Some(self.time + f64::from(self.random.range(0.7, 2.5)));
        }
    }

    pub(super) fn catch_yawn_message(&mut self, text: &str) {
        if self.quirks_allowed() {
            self.record(EventKind::Heard(text.to_owned()));
            self.catch_yawn();
        }
    }

    pub(super) fn note_quirk_stroke(&mut self, travel: f32) {
        if !travel.is_finite() || travel <= 0.0 || !self.quirks_allowed() {
            return;
        }
        if self
            .quirk_triggers
            .stroke_at
            .is_none_or(|at| self.time - at > 1.5)
        {
            self.quirk_triggers.stroke_travel = 0.0;
        }
        self.quirk_triggers.stroke_at = Some(self.time);
        self.quirk_triggers.stroke_travel = (self.quirk_triggers.stroke_travel + travel).min(10.0);
    }

    pub(super) fn sigh_on_cushion(&mut self) {
        if matches!(self.ghost.mood, Mood::Content | Mood::Cheerful)
            && self.ghost.needs.fun > 0.5
            && self.ghost.needs.energy > 0.35
            && self.quirks_allowed()
            && self.quirk.is_none()
            && self.time >= self.quirk_triggers.ready_at
            && self.random.unit() < 0.55
        {
            self.start_quirk(Quirk::Sigh);
        }
    }

    fn pollen_tickles(&self) -> bool {
        self.window_open
            && self.season() == Season::Spring
            && (7.0..19.5).contains(&self.hour)
            && (self
                .petals
                .iter()
                .any(|petal| petal.position.distance(self.ghost.position) < 0.8)
                || (self.plant.blossom > 0.2
                    && self
                        .ghost
                        .position
                        .distance(self.plant.position + Vec3::Y * self.plant.height())
                        < 0.9))
    }

    fn behind_furniture(&self) -> bool {
        self.ghost.plan.activity == Activity::Explore
            && [
                (self.lamp.position, super::Lamp::HEIGHT),
                (self.plant.position, self.plant.height()),
            ]
            .into_iter()
            .any(|(position, height)| {
                self.ghost.position.z < position.z
                    && flat(self.ghost.position - position).length() < 0.75
                    && self.ghost.position.y < height + 0.3
            })
    }

    fn chilly(&self) -> bool {
        (self.season() == Season::Winter && self.is_night() && self.window_open)
            || (self.weather.snow(self.time) > 0.1
                && (!self.lamp.on || self.ghost.position.distance(self.lamp.bulb()) > 1.1)
                && self.ghost.position.distance(self.sleeping_spot()) > 0.9)
    }

    fn sleepy_yawn(&self) -> bool {
        self.ghost.needs.energy < 0.35 || self.hour >= 20.5 || self.bedtime() || self.drowsy()
    }

    fn sneeze_impulse(&mut self) {
        let position = self.ghost.position;
        self.ghost.velocity -= self.ghost.heading.normalize_or(Vec3::Z) * 0.45;
        for mote in &mut self.motes {
            let offset = mote.position - position;
            if offset.length_squared() < 1.0 {
                mote.position = (mote.position + offset.normalize_or(Vec3::Y) * 0.22).clamp(
                    ROOM_MIN + Vec3::new(0.25, 0.3, 0.25),
                    ROOM_MAX - Vec3::splat(0.25),
                );
            }
        }
        for petal in &mut self.petals {
            let offset = petal.position - position;
            if offset.length_squared() < 1.0 {
                let direction = offset.normalize_or(Vec3::Y);
                petal.velocity += direction * 0.5;
                petal.position = (petal.position + direction * 0.12)
                    .clamp(ROOM_MIN + Vec3::splat(0.01), ROOM_MAX - Vec3::splat(0.01));
            }
        }
    }

    pub(super) fn advance_quirks(&mut self) {
        if !self.quirks_allowed() {
            self.quirk = None;
            self.quirk_triggers.yawn_at = None;
            self.quirk_triggers.stroke_at = None;
            return;
        }
        if let Some(mut spell) = self.quirk {
            if self.time >= spell.ends {
                self.quirk = None;
            } else {
                if self.time >= spell.next_at {
                    if spell.kind == Quirk::Hiccups {
                        self.ghost.velocity.y += 0.35;
                        spell.pulse_at = self.time;
                        spell.next_at = self.time + f64::from(self.random.range(2.5, 5.0));
                    } else if spell.kind == Quirk::Sneeze && spell.remaining > 0 {
                        self.sneeze_impulse();
                        spell.remaining -= 1;
                        spell.next_at = if spell.remaining > 0 {
                            spell.next_at + 2.2
                        } else {
                            f64::INFINITY
                        };
                    }
                }
                self.quirk = Some(spell);
                return;
            }
        }
        if let Some(at) = self.quirk_triggers.yawn_at
            && self.time >= at
        {
            self.start_quirk(Quirk::Yawn);
            return;
        }
        if self.quirk_triggers.yawn_at.is_some() {
            return;
        }
        if self
            .quirk_triggers
            .stroke_at
            .is_some_and(|at| self.time - at >= 1.5)
        {
            self.quirk_triggers.stroke_at = None;
            let long = self.quirk_triggers.stroke_travel >= 0.8;
            self.quirk_triggers.stroke_travel = 0.0;
            if long && self.time >= self.quirk_triggers.ready_at && self.random.unit() < 0.7 {
                self.start_quirk(Quirk::Sigh);
                return;
            }
        }
        if self.time < self.quirk_triggers.next_check || self.time < self.quirk_triggers.ready_at {
            return;
        }
        self.quirk_triggers.next_check = self.time + 10.0;
        if self.behind_furniture() {
            self.quirk_triggers.dust_until = self.time + 20.0;
        }
        let candidate = if self.life.cold.is_some() {
            Some((Quirk::Sneeze, 0.35))
        } else if self.pollen_tickles() || self.time < self.quirk_triggers.dust_until {
            Some((Quirk::Sneeze, 0.25))
        } else if self.chilly() {
            Some((Quirk::Shiver, 0.35))
        } else if self.sleepy_yawn() {
            Some((Quirk::Yawn, 0.18))
        } else {
            None
        };
        if let Some((kind, chance)) = candidate
            && self.random.unit() < chance
        {
            self.start_quirk(kind);
        }
    }
}

/// Seconds after a spell before another may begin: a sleepy evening
/// brings a few yawns rather than dozens, and hiccups are rare.
fn rest_after(kind: Quirk) -> (f32, f32) {
    match kind {
        Quirk::Yawn => (480.0, 900.0),
        Quirk::Hiccups => (1800.0, 3600.0),
        Quirk::Sneeze | Quirk::Sigh => (120.0, 300.0),
        Quirk::Shiver => (90.0, 180.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Mote, Petal, Understanding};

    fn kind(world: &World) -> Option<Quirk> {
        world.quirk.map(|spell| spell.kind)
    }

    fn spring(seed: u64) -> World {
        let mut world = World::new(seed, 12.0);
        world.date = jiff::civil::Date::constant(2026, 4, 15);
        world.window_open = true;
        world.plant.blossom = 1.0;
        world.ghost.position = world.plant.position + Vec3::Y * world.plant.height();
        world
    }

    #[test]
    fn performed_yawns_are_recognized_in_both_languages_without_matching_ordinary_words() {
        for message in [
            "yawn",
            "*yawns*",
            "YAWN!",
            "(yawning)",
            "yaaawn…",
            "하품",
            "*하품중*",
            "하암~",
            "하아아암",
            "ㅎㅏ암",
            "ㅎㅏ아암",
        ] {
            assert!(yawn_message(message), "{message}");
        }
        for message in [
            "",
            "...",
            "awn",
            "yawningly",
            "yawnsome",
            "spawn",
            "I yawn sometimes",
            "why do we yawn?",
            "don't yawn",
            "하마",
            "하암이라고",
            "하품이 뭐야?",
            "하품하지 마",
            "하품해줘",
        ] {
            assert!(!yawn_message(message), "{message}");
        }
    }

    #[test]
    fn three_recent_motes_use_the_worlds_random_source_to_start_hiccups() {
        let mut spells = 0;
        for seed in 0..64 {
            let mut world = World::new(seed, 12.0);
            world.record(EventKind::AteMote);
            world.time = 10.0;
            world.record(EventKind::AteMote);
            assert!(world.quirk.is_none());
            let mut expected = world.random.clone();
            let starts = expected.unit() < 0.25;
            world.time = 29.9;
            world.record(EventKind::AteMote);
            assert_eq!(kind(&world), starts.then_some(Quirk::Hiccups));
            if let Some(spell) = world.quirk {
                spells += 1;
                assert_eq!(
                    spell.ends,
                    world.time + f64::from(expected.range(20.0, 40.0))
                );
                assert_eq!(
                    spell.next_at,
                    world.time + f64::from(expected.range(2.5, 5.0))
                );
                expected.range(1800.0, 3600.0);
            }
            assert_eq!(world.random.unit(), expected.unit());
        }
        assert!(spells > 0 && spells < 64);
    }

    #[test]
    fn old_motes_and_spaced_out_delight_do_not_cause_hiccups() {
        for seed in 0..32 {
            let mut world = World::new(seed, 12.0);
            for time in [0.0, 15.0, 30.1] {
                world.time = time;
                world.record(EventKind::AteMote);
            }
            for time in [60.0, 71.0, 82.0, 93.0] {
                world.time = time;
                world.record(EventKind::Praised);
            }
            assert!(world.quirk.is_none());
        }
    }

    #[test]
    fn several_delighted_reactions_can_start_one_hiccup_spell() {
        let mut spells = 0;
        for seed in 0..64 {
            let mut world = World::new(seed, 12.0);
            for index in 0..4 {
                world.time = f64::from(index) * 4.0;
                world.record(EventKind::Praised);
                if index < 3 {
                    assert!(world.quirk.is_none());
                }
            }
            if kind(&world) == Some(Quirk::Hiccups) {
                spells += 1;
            }
        }
        assert!(spells > 0 && spells < 64);
    }

    #[test]
    fn hiccups_hop_repeatedly_but_count_once_and_end_on_time() {
        let mut world = World::new(9, 12.0);
        assert!(world.start_quirk(Quirk::Hiccups));
        let spell = world.quirk.unwrap();
        assert!((20.0..=40.0).contains(&(spell.ends - spell.started)));
        assert!(world.ghost.velocity.y > 0.0);
        let mut hops = 1;
        while world.quirk.unwrap().next_at < spell.ends {
            world.time = world.quirk.unwrap().next_at;
            world.ghost.velocity = Vec3::ZERO;
            world.advance_quirks();
            assert!(world.ghost.velocity.y > 0.0);
            hops += 1;
        }
        assert!(hops >= 4);
        assert_eq!(world.life.total.quirks[&Quirk::Hiccups], 1);
        world.time = spell.ends;
        world.advance_quirks();
        assert!(world.quirk.is_none());
        world.quirk_after_event();
        assert!(world.quirk.is_none());
    }

    #[test]
    fn a_poke_or_thunder_cures_hiccups_immediately_and_cures_speak_in_the_viewers_language() {
        for korean in [false, true] {
            let mut world = World::new(9, 12.0);
            world.life.korean = if korean { 1.0 } else { 0.0 };
            world.start_quirk(Quirk::Hiccups);
            world.poke();
            assert!(world.quirk.is_none());
            assert_eq!(
                world.ghost.speech.as_ref().unwrap().text,
                if korean {
                    "…어! 딸꾹질이 멎었어."
                } else {
                    "…oh! They're gone."
                }
            );
            world.start_quirk(Quirk::Hiccups);
            world.record(EventKind::Thunder);
            assert!(world.quirk.is_none());
        }
    }

    #[test]
    fn spring_pollen_requires_daylight_an_open_window_and_a_nearby_source() {
        let mut successes = 0;
        for seed in 0..64 {
            let mut world = spring(seed);
            assert!(world.pollen_tickles());
            world.time = 8.0;
            let mut expected = world.random.clone();
            let starts = expected.unit() < 0.25;
            world.advance_quirks();
            assert_eq!(kind(&world), starts.then_some(Quirk::Sneeze));
            if starts {
                successes += 1;
            }
            for condition in 0..4 {
                let mut unsuitable = spring(seed);
                match condition {
                    0 => unsuitable.window_open = false,
                    1 => unsuitable.date = jiff::civil::Date::constant(2026, 7, 15),
                    2 => unsuitable.hour = 20.0,
                    _ => unsuitable.ghost.position = Vec3::new(-0.5, 1.0, 1.0),
                }
                assert!(!unsuitable.pollen_tickles());
                unsuitable.time = 8.0;
                unsuitable.advance_quirks();
                assert!(unsuitable.quirk.is_none());
            }
        }
        assert!(successes > 0 && successes < 64);
        let mut petals = spring(1);
        petals.plant.blossom = 0.0;
        assert!(!petals.pollen_tickles());
        petals.petals.push(Petal {
            position: petals.ghost.position + Vec3::Y * 0.2,
            velocity: Vec3::ZERO,
            spin: 0.0,
            dropped: 0.0,
        });
        assert!(petals.pollen_tickles());
    }

    #[test]
    fn exploring_behind_furniture_leaves_a_brief_dust_trigger() {
        for seed in 0..32 {
            let mut world = World::new(seed, 12.0);
            world.ghost.position = world.lamp.position + Vec3::new(0.0, 0.8, -0.1);
            assert!(!world.behind_furniture());
            world.ghost.plan.activity = Activity::Explore;
            assert!(world.behind_furniture());
            world.time = 8.0;
            world.advance_quirks();
            assert_eq!(world.quirk_triggers.dust_until, 28.0);
            if let Some(spell) = world.quirk {
                assert_eq!(spell.kind, Quirk::Sneeze);
            }
            world.ghost.position = Vec3::new(0.0, 1.0, 1.0);
            assert!(!world.behind_furniture());
            world.quirk = None;
            world.quirk_triggers.ready_at = 0.0;
            world.time = 29.0;
            world.advance_quirks();
            assert!(world.quirk.is_none());
        }
    }

    #[test]
    fn a_sneeze_waits_for_its_inhale_then_pushes_nearby_motes_and_petals_within_the_room() {
        let mut world = World::new(2, 12.0);
        world.ghost.position = Vec3::new(1.4, 1.0, 0.0);
        let origin = world.ghost.position;
        world.motes = vec![
            Mote {
                id: 1,
                position: origin + Vec3::X * 0.15,
                born: 0.0,
                seed: 0.0,
            },
            Mote {
                id: 2,
                position: origin,
                born: 0.0,
                seed: 0.0,
            },
            Mote {
                id: 3,
                position: Vec3::new(-1.5, 1.0, 0.0),
                born: 0.0,
                seed: 0.0,
            },
        ];
        world.petals.push(Petal {
            position: origin + Vec3::Y * 0.2,
            velocity: Vec3::ZERO,
            spin: 0.0,
            dropped: 0.0,
        });
        world.start_quirk(Quirk::Sneeze);
        let motes = world.motes.clone();
        world.time = 0.99;
        world.advance_quirks();
        assert_eq!(world.motes, motes);
        world.time = 1.0;
        world.advance_quirks();
        for (index, mote) in world.motes.iter().enumerate() {
            assert!(mote.position.is_finite());
            assert!(mote.position.cmpge(ROOM_MIN).all() && mote.position.cmple(ROOM_MAX).all());
            if index < 2 {
                assert!(mote.position.distance(origin) > motes[index].position.distance(origin));
            } else {
                assert_eq!(*mote, motes[index]);
            }
        }
        assert!(world.ghost.velocity.dot(world.ghost.heading) < 0.0);
        assert!(world.petals[0].velocity.y > 0.0);
        assert!(world.petals[0].position.distance(origin) > 0.2);
        world.step(4.0);
        assert!(world.petals[0].position.cmpge(ROOM_MIN).all());
        assert!(world.petals[0].position.cmple(ROOM_MAX).all());
    }

    #[test]
    fn sneezes_sometimes_repeat_once_without_counting_a_second_spell() {
        let mut doubles = 0;
        for seed in 0..64 {
            let mut world = World::new(seed, 12.0);
            let mut expected = world.random.clone();
            let twice = expected.unit() < 0.25;
            world.start_quirk(Quirk::Sneeze);
            let spell = world.quirk.unwrap();
            assert_eq!(spell.twice, twice);
            world.time = 1.0;
            world.advance_quirks();
            let first = world.ghost.velocity;
            world.time = 3.2;
            world.advance_quirks();
            assert_eq!(world.quirk.is_some(), twice);
            if twice {
                doubles += 1;
                assert!(world.ghost.velocity.length() > first.length());
                assert_eq!(world.quirk.unwrap().remaining, 0);
            }
            assert_eq!(world.life.total.quirks[&Quirk::Sneeze], 1);
        }
        assert!(doubles > 0 && doubles < 64);
    }

    #[test]
    fn natural_yawns_need_tiredness_evening_or_an_interrupted_night() {
        let mut starts = 0;
        for condition in 0..4 {
            for seed in 0..32 {
                let mut world = World::new(seed, 12.0);
                match condition {
                    0 => world.ghost.needs.energy = 0.2,
                    1 => world.hour = 21.0,
                    2 => world.drowsy_until = 100.0,
                    _ => world.ghost.needs.energy = 1.0,
                }
                assert_eq!(world.sleepy_yawn(), condition < 3);
                world.time = 8.0;
                world.advance_quirks();
                if condition == 3 {
                    assert!(world.quirk.is_none());
                }
                if let Some(spell) = world.quirk {
                    assert_eq!(spell.kind, Quirk::Yawn);
                    starts += 1;
                }
            }
        }
        assert!(starts > 0);
    }

    #[test]
    fn a_viewers_yawn_is_contagious_within_a_few_seconds_and_teaches_no_words() {
        for message in ["*yawns*", "하아암", "ㅎㅏ암"] {
            let mut world = World::new(3, 22.0);
            world.begin(Activity::Play);
            let plan = world.ghost.plan.clone();
            let before = world.quirk_leaning(Activity::Rest);
            let heard = world.hear(message);
            assert_eq!(heard.understanding, Understanding::Yawning);
            assert!(world.life.vocabulary.known().is_empty());
            assert!(heard.learned.is_empty());
            let at = world.quirk_triggers.yawn_at.unwrap();
            assert!((0.7..=2.5).contains(&at));
            world.time = at;
            world.advance_quirks();
            assert_eq!(kind(&world), Some(Quirk::Yawn));
            assert_eq!(world.ghost.plan.activity, plan.activity);
            assert!(!world.ghost.plan.done);
            assert!(world.quirk_leaning(Activity::Rest) > before);
        }
    }

    #[test]
    fn cold_shivers_need_a_wintry_open_night_or_snow_away_from_warmth() {
        let mut starts = 0;
        for seed in 0..64 {
            let mut world = World::new(seed, 21.0);
            world.date = jiff::civil::Date::constant(2026, 12, 21);
            world.window_open = true;
            assert!(world.chilly());
            world.time = 8.0;
            world.advance_quirks();
            if kind(&world) == Some(Quirk::Shiver) {
                starts += 1;
                assert!(world.quirk_leaning(Activity::Air) > 1.0);
                assert!(world.quirk_leaning(Activity::Rest) > 1.0);
                assert!(world.quirk_leaning(Activity::Lamp) < 1.0);
                world.lamp.on = false;
                assert!(world.quirk_leaning(Activity::Lamp) > 1.0);
            }
            world.window_open = false;
            assert!(!world.chilly());
            world.window_open = true;
            world.hour = 12.0;
            assert!(!world.chilly());
            world.weather.sky = super::super::Sky::Snow;
            world.weather.since = 0.0;
            world.time = 100.0;
            world.ghost.position = Vec3::new(1.0, 1.0, 0.0);
            assert!(world.chilly());
            world.lamp.on = true;
            world.ghost.position = world.lamp.bulb();
            assert!(!world.chilly());
            world.ghost.position = world.nook.bed(world.size());
            assert!(!world.chilly());
        }
        assert!(starts > 0 && starts < 64);
    }

    #[test]
    fn a_long_stroke_can_end_in_a_sigh_but_a_short_or_ongoing_one_cannot() {
        let mut sighs = 0;
        for seed in 0..32 {
            let mut world = World::new(seed, 12.0);
            world.pet(0.1);
            world.time = 1.5;
            world.advance_quirks();
            assert!(world.quirk.is_none());
            for _ in 0..5 {
                world.pet(0.2);
                world.time += 0.2;
                world.advance_quirks();
                assert!(world.quirk.is_none());
            }
            world.time += 1.5;
            world.advance_quirks();
            if let Some(spell) = world.quirk {
                assert_eq!(spell.kind, Quirk::Sigh);
                sighs += 1;
            }
        }
        assert!(sighs > 0 && sighs < 32);
    }

    #[test]
    fn settling_contentedly_on_the_cushion_can_sigh_before_falling_asleep() {
        let mut sighs = 0;
        for seed in 0..32 {
            let mut world = World::new(seed, 12.0);
            world.ghost.mood = Mood::Content;
            world.ghost.needs.energy = 0.6;
            world.ghost.needs.fun = 0.9;
            world.begin(Activity::Rest);
            world.ghost.position = world.ghost.plan.target;
            world.step(0.1);
            if kind(&world) == Some(Quirk::Sigh) {
                sighs += 1;
                assert_eq!(world.ghost.asleep, 0.0);
                world.step(5.0);
                assert!(world.quirk.is_none());
                assert!(world.ghost.asleep > 0.0);
            }
        }
        assert!(sighs > 0 && sighs < 32);
    }

    #[test]
    fn sleep_stories_and_both_games_suppress_every_quirk_and_contagious_yawns() {
        for protected in 0..4 {
            let mut world = spring(7);
            match protected {
                0 => world.ghost.asleep = 1.0,
                1 => world.tell(Vec::new()),
                2 => world.begin(Activity::Game),
                _ => world.begin(Activity::Puzzle),
            }
            let activity = world.ghost.plan.activity;
            let asleep = world.ghost.asleep;
            let mut expected = world.random.clone();
            for quirk in Quirk::ALL {
                assert!(!world.start_quirk(quirk));
            }
            world.hear("*yawns*");
            world.time = 8.0;
            world.advance_quirks();
            assert!(world.quirk.is_none());
            assert!(world.quirk_triggers.yawn_at.is_none());
            assert_eq!(world.ghost.asleep, asleep);
            assert_eq!(world.ghost.plan.activity, activity);
            assert_eq!(world.random.unit(), expected.unit());
        }
    }

    #[test]
    fn a_protected_activity_clears_an_existing_spell_without_resuming_it_later() {
        for quirk in Quirk::ALL {
            let mut world = World::new(1, 12.0);
            world.start_quirk(quirk);
            world.tell(Vec::new());
            assert!(world.active_quirk().is_none());
            world.time += 0.1;
            world.advance_quirks();
            assert!(world.quirk.is_none());
            assert!(world.telling.is_some());
        }
    }

    #[test]
    fn quirk_spells_are_exclusive_end_on_time_and_do_not_drop_carried_toys() {
        for quirk in Quirk::ALL {
            let mut world = World::new(1, 12.0);
            world.ghost.plan.activity = Activity::Fetch;
            world.ball.held = true;
            world.blocks[0].held = true;
            world.start_quirk(quirk);
            let spell = world.quirk.unwrap();
            for other in Quirk::ALL {
                assert!(!world.start_quirk(other));
            }
            world.time = spell.started + 1.0;
            world.advance_quirks();
            assert!(world.ball.held && world.blocks[0].held);
            assert_eq!(world.ghost.plan.activity, Activity::Fetch);
            world.time = spell.ends;
            world.advance_quirks();
            assert!(world.quirk.is_none());
            assert!(world.ball.held && world.blocks[0].held);
        }
    }

    #[test]
    fn dormant_checks_do_not_draw_randomness_and_identical_worlds_have_identical_spells() {
        let mut quiet = World::new(1, 12.0);
        let mut expected = quiet.random.clone();
        for time in [8.0, 18.0, 28.0] {
            quiet.time = time;
            quiet.advance_quirks();
        }
        assert_eq!(quiet.random.unit(), expected.unit());
        let mut first = spring(13);
        let mut second = first.clone();
        for index in 0..1000 {
            first.time = f64::from(index) * 0.1;
            second.time = first.time;
            first.advance_quirks();
            second.advance_quirks();
            assert_eq!(first.quirk, second.quirk);
            assert_eq!(first.ghost.velocity, second.ghost.velocity);
        }
        assert_eq!(first.random.unit(), second.random.unit());
    }

    #[test]
    fn quirk_counts_load_from_old_lives_round_trip_and_are_bounded_without_milestones() {
        let mut life: Life = serde_json::from_str("{}").unwrap();
        assert!(life.total.quirks.is_empty());
        for quirk in Quirk::ALL {
            assert!(life.note(&EventKind::Quirk(quirk)).is_none());
            assert!(life.note(&EventKind::Quirk(quirk)).is_none());
            assert_eq!(life.total.quirks[&quirk], 2);
            assert_eq!(life.today.quirks[&quirk], 2);
            assert!(life.today.words().to_lowercase().contains(&quirk.diary(2)));
            assert!(
                !quirk
                    .diary(2)
                    .chars()
                    .any(|character| character.is_ascii_digit())
            );
            life.total.quirks.insert(quirk, u32::MAX);
        }
        let saved = serde_json::to_string(&life).unwrap();
        let life: Life = serde_json::from_str(&saved).unwrap();
        let life = life.sanitized();
        assert!(
            life.total
                .quirks
                .values()
                .all(|count| *count == 100_000_000)
        );
        assert!(life.milestones.is_empty());
    }

    #[test]
    fn ongoing_words_and_emote_pulses_follow_spell_lifetimes_and_stay_bounded() {
        use crate::face::emote::{Emote, puffs};
        for quirk in Quirk::ALL {
            let mut world = World::new(2, 12.0);
            world.start_quirk(quirk);
            let spell = world.quirk.unwrap();
            assert_eq!(world.quirk_words(), quirk.ongoing());
            for step in 0..80 {
                world.time = spell.started + (spell.ends - spell.started) * f64::from(step) / 80.0;
                world.advance_quirks();
                assert!(puffs(&world).len() <= 3);
                if quirk == Quirk::Sneeze && world.time < spell.started + 1.0 {
                    assert!(puffs(&world).iter().all(|puff| puff.emote != Emote::Sneeze));
                }
            }
            world.time = spell.ends;
            assert_eq!(world.quirk_words(), "none just now");
        }
    }

    #[test]
    fn hearing_a_yawn_leaves_sleeping_reactions_quiet_and_cancels_before_an_inhale_if_sleep_begins()
    {
        use crate::face::{emote, mood};
        let mut world = World::new(1, 23.0);
        world.tuck_in();
        world.hear("하암");
        assert!(world.events.is_empty());
        assert!(mood::reaction(&EventKind::Heard("하암".to_owned())).is_none());
        assert!(
            emote::puffs(&world)
                .iter()
                .all(|puff| puff.emote == emote::Emote::Sleep)
        );
        assert!(world.quirk_triggers.yawn_at.is_none());
        world.ghost.asleep = 0.0;
        world.ghost.plan.activity = Activity::Float;
        world.start_quirk(Quirk::Sneeze);
        let velocity = world.ghost.velocity;
        world.ghost.asleep = 1.0;
        world.time = 1.1;
        world.advance_quirks();
        assert!(world.quirk.is_none());
        assert_eq!(world.ghost.velocity, velocity);
    }

    #[test]
    fn render_samples_stay_finite_during_quirks_at_ordinary_and_very_large_times() {
        use crate::face::{Face, mood};
        use crate::render::ghost::{Aura, Body, Pose, shade};
        for quirk in Quirk::ALL {
            for started in [0.0, 1e12] {
                let mut world = World::new(2, 12.0);
                world.time = started;
                world.start_quirk(quirk);
                let spell = world.quirk.unwrap();
                for step in 0..24 {
                    let time = started + (spell.ends - started) * f64::from(step) / 24.0;
                    let motion = spell.motion(time);
                    assert!(motion.lift.is_finite() && motion.lean.is_finite());
                    assert!(motion.stretch.is_finite());
                    let face = mood::reaction(&EventKind::Quirk(quirk))
                        .map_or(Face::REST, |expression| {
                            expression.at((time - spell.pulse_started(time)) as f32)
                        });
                    let body = Body {
                        position: world.ghost.position,
                        scale: 1.0,
                        pose: Pose::new(time, 3.0, world.ghost.velocity.y, 0.0).in_breeze(1.0, 1.0),
                        face,
                        aura: Aura {
                            color: Vec3::ONE,
                            visibility: 1.0,
                            brightness: 1.0,
                            beat: 1.0,
                        },
                        opacity: 1.0,
                        accessory: None,
                    };
                    for column in -15..=15 {
                        for row in -15..=15 {
                            let point = glam::Vec2::new(column as f32, row as f32) * 0.1;
                            let glow = shade(point, 0.02, time, &body);
                            assert!(glow.light.is_finite() && glow.cover.is_finite());
                            assert!((0.0..=1.0).contains(&glow.cover));
                            let bounds = crate::render::ghost::BOUNDS;
                            if point.x < bounds[0].x
                                || point.x > bounds[1].x
                                || point.y < bounds[0].y
                                || point.y > bounds[1].y
                            {
                                assert_eq!(glow.cover, 0.0, "{quirk:?} at {time}: {point}");
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn cold_sneezes_keep_one_spell_and_the_shared_pause_between_spells() {
        let mut world = World::new(2, 12.0);
        world.life.cold = Some(super::super::cold::Cold {
            day: world.date.to_string(),
            hour: 12.0,
            severity: 0.5,
            care: Vec::new(),
            remaining: 80_000.0,
        });
        assert!(world.start_quirk(Quirk::Sneeze));
        assert!(!world.start_quirk(Quirk::Sneeze));
        let ends = world.quirk.unwrap().ends;
        let ready = world.quirk_triggers.ready_at;
        assert!((90.0..=180.0).contains(&(ready - ends)));
        world.time = ends + 1.0;
        world.advance_quirks();
        assert!(world.quirk.is_none());
        world.time = ready - 0.01;
        world.advance_quirks();
        assert!(world.quirk.is_none());
        for _ in 0..100 {
            world.time += 10.0;
            world.advance_quirks();
            if world.quirk.is_some() {
                break;
            }
        }
        assert_eq!(world.quirk.unwrap().kind, Quirk::Sneeze);
        assert!(world.quirk.unwrap().started >= ready);
    }
}
