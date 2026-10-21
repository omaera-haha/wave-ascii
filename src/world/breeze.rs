use glam::Vec3;

use super::{
    Activity, EventKind, Mood, Petal, ROOM_MAX, ROOM_MIN, Random, Season, Sky, WINDOW_CENTER, World,
};

/// The wind under which opening the window seems fair again.
const CALM_ENOUGH: f32 = 0.6;
/// Seconds after the sash last moved before the ghost thinks of opening it
/// on its own again.
const AIRING_PAUSE: f64 = 1200.0;

/// Wind has its own stream so its spells never change the sky's draws.
#[derive(Clone, Debug)]
pub struct Wind {
    pub strength: f32,
    until: f64,
    target: f32,
    phase: f64,
    direction: f32,
    random: Random,
    pub travel: f64,
}

impl Wind {
    pub fn new(seed: u64) -> Self {
        let mut random = Random::new(seed ^ 0x6272_6565_7a65);
        Self {
            strength: 0.0,
            until: 0.0,
            target: 0.0,
            phase: f64::from(random.unit()) * std::f64::consts::TAU,
            direction: if random.unit() < 0.5 { -1.0 } else { 1.0 },
            random,
            travel: 0.0,
        }
    }

    fn advance(
        &mut self,
        time: f64,
        seconds: f32,
        season: Season,
        night: bool,
        sky: Sky,
        storm: bool,
    ) {
        if time >= self.until {
            self.target = self.random.unit();
            self.until = time + f64::from(self.random.range(600.0, 2400.0));
        }
        let target = if storm {
            0.8 + self.target * 0.2
        } else if sky == Sky::Snow {
            0.08 + self.target * 0.18
        } else if season == Season::Summer && night {
            self.target * 0.12
        } else {
            match season {
                Season::Spring | Season::Autumn => 0.15 + self.target * 0.8,
                Season::Summer => 0.04 + self.target * 0.5,
                Season::Winter => 0.1 + self.target * 0.65,
            }
        };
        self.strength += (target - self.strength) * (seconds / 8.0).min(1.0);
        self.travel += f64::from(self.gust(time) * seconds);
    }

    /// Smooth, bounded gusts can be sampled repeatedly without consuming randomness.
    pub fn gust(&self, time: f64) -> f32 {
        let phase = (time * 0.73 + self.phase).rem_euclid(std::f64::consts::TAU);
        let slow = (time * 0.19 + self.phase * 2.0).rem_euclid(std::f64::consts::TAU);
        self.strength.clamp(0.0, 1.0) * (0.72 + 0.18 * phase.sin() as f32 + 0.1 * slow.sin() as f32)
    }

    pub fn sideways(&self, time: f64) -> f32 {
        self.direction * self.gust(time)
    }

    pub fn words(&self) -> &'static str {
        match self.strength {
            strength if strength < 0.1 => "still air",
            strength if strength < 0.4 => "a gentle breeze",
            strength if strength < 0.75 => "a steady wind",
            _ => "blustery, gusty wind",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leaf {
    pub position: Vec3,
    pub spin: f32,
    pub dropped: f64,
}

impl Leaf {
    pub const LIFETIME: f64 = 240.0;

    pub fn fade(&self, time: f64) -> f32 {
        ((Self::LIFETIME - (time - self.dropped)) / 45.0).clamp(0.0, 1.0) as f32
    }
}

impl World {
    pub fn wants_window_shut(&self) -> bool {
        self.life.cold.is_some() || self.window_needs_shelter()
    }

    fn window_needs_shelter(&self) -> bool {
        matches!(self.weather.sky, Sky::Rain | Sky::Snow)
            || self.weather.storm
            || self.bedtime()
            || (self.season() == Season::Winter && self.is_night())
            || self.wind.strength >= 0.75
    }

    /// Wind that would make it shut the sash must also ease well below
    /// that before opening it seems fair, so a gusty spell doesn't swing it
    /// back and forth.
    pub fn fresh_air_is_fair(&self) -> bool {
        !self.window_needs_shelter()
            && self.wind.strength < CALM_ENOUGH
            && !self.is_night()
            && self.season() != Season::Winter
    }

    pub(super) fn air_is_open(&self) -> bool {
        self.ghost.asleep < 0.3
            && if self.window_open {
                self.wants_window_shut()
            } else {
                self.fresh_air_is_fair()
                    && self
                        .window_moved_at
                        .is_none_or(|at| self.time - at >= AIRING_PAUSE)
            }
    }

    /// A request fixes the intended sash position before the ghost sets out.
    pub fn ask_window(&mut self, open: bool) -> bool {
        if self.ghost.asleep >= 0.3 {
            return false;
        }
        if self.telling.is_some() || (open && !self.fresh_air_is_fair()) {
            self.window_refused_at = Some(self.time);
            return false;
        }
        if self.window_open == open {
            if self.ghost.plan.activity == Activity::Air {
                self.ghost.plan.done = true;
            }
            return true;
        }
        if self.ghost.plan.activity != Activity::Air
            || self.ghost.plan.done
            || self.ghost.plan.window != Some(open)
        {
            self.begin(Activity::Air);
            self.ghost.plan.window = Some(open);
        }
        if !open {
            self.note_cold_request(Activity::Air);
        }
        true
    }

    pub fn window_lift(&self) -> f32 {
        let resting = if self.window_open { 1.0 } else { 0.0 };
        let plan = &self.ghost.plan;
        if plan.activity != Activity::Air || plan.done {
            return resting;
        }
        let target = if plan.window.unwrap_or(!self.window_open) {
            1.0
        } else {
            0.0
        };
        let progress = (plan.stayed(self.time) / 0.9).clamp(0.0, 1.0);
        resting + (target - resting) * progress * progress * (3.0 - 2.0 * progress)
    }

    pub fn air_doing(&self) -> &'static str {
        if self.ghost.plan.window.unwrap_or(!self.window_open) {
            "opening the window for fresh air"
        } else {
            "closing the window"
        }
    }

    pub fn window_shake(&self) -> f32 {
        self.window_refused_at.map_or(0.0, |at| {
            let elapsed = (self.time - at) as f32;
            if (0.0..0.9).contains(&elapsed) {
                (elapsed * 24.0).sin() * (1.0 - elapsed / 0.9)
            } else {
                0.0
            }
        })
    }

    pub(super) fn carry_out_air(&mut self) -> Option<Vec3> {
        let open = self.ghost.plan.window.unwrap_or(!self.window_open);
        if open && !self.fresh_air_is_fair() {
            self.ghost.plan.done = true;
        } else if self.ghost.plan.stayed(self.time) >= 0.9 {
            if open != self.window_open {
                self.window_open = open;
                self.window_moved_at = Some(self.time);
                self.record(if open {
                    EventKind::OpenedWindow
                } else {
                    EventKind::ClosedWindow
                });
                self.cold_window_moved();
            }
            self.ghost.plan.done = true;
        }
        Some(WINDOW_CENTER)
    }

    /// Flow weakens toward the front glass, and outside the window's corridor.
    pub fn breeze_at(&self, position: Vec3) -> Vec3 {
        breeze(self.window_open, &self.wind, self.time, position)
    }

    pub fn enjoying_air(&self) -> bool {
        self.window_open
            && !self.wants_window_shut()
            && self.ghost.position.distance(WINDOW_CENTER) < 0.95
    }

    pub(super) fn advance_breeze(&mut self, seconds: f32) {
        self.wind.advance(
            self.time,
            seconds,
            self.season(),
            self.is_night(),
            self.weather.sky,
            self.weather.storm,
        );
        let wet = self.window_open && matches!(self.weather.sky, Sky::Rain | Sky::Snow);
        if wet {
            self.puddle = (self.puddle + seconds / 30.0).min(1.0);
        } else {
            self.puddle = (self.puddle - seconds / 600.0).max(0.0);
        }
        if self.puddle == 0.0 {
            self.saw_puddle = false;
        }
        if self.ghost.asleep < 0.3 {
            let noticed = self.puddle > 0.03 && !self.saw_puddle;
            if noticed {
                self.saw_puddle = true;
                self.record(EventKind::Puddle);
                self.think("Oh no, the floor is wet.");
                self.suggest_mood(Mood::Grumpy, 5.0);
            }
            let nonurgent = self.ghost.plan.done
                || matches!(
                    self.ghost.plan.activity,
                    Activity::Float
                        | Activity::Explore
                        | Activity::Window
                        | Activity::Visit
                        | Activity::Wait
                        | Activity::Company
                        | Activity::Dance
                        | Activity::Hide
                )
                || (self.ghost.plan.activity == Activity::Rest && self.ghost.needs.energy > 0.2);
            if self.window_open
                && (noticed || self.wants_window_shut())
                && (noticed || nonurgent)
                && (self.ghost.plan.activity != Activity::Air || self.ghost.plan.done)
            {
                self.telling = None;
                self.begin(Activity::Air);
            }
        }
        self.leaves
            .retain(|leaf| self.time - leaf.dropped < Leaf::LIFETIME);
        for leaf in &mut self.leaves {
            if leaf.position.y > 0.012 {
                let flow = breeze(self.window_open, &self.wind, self.time, leaf.position);
                leaf.position += (flow * 1.3 - Vec3::Y * 0.12) * seconds;
                leaf.position.y = leaf.position.y.max(0.012);
                leaf.spin += seconds * (1.3 + flow.length());
                leaf.position = leaf
                    .position
                    .clamp(ROOM_MIN + Vec3::splat(0.01), ROOM_MAX - Vec3::splat(0.01));
            }
        }
        if self.time >= self.next_windfall_at {
            self.next_windfall_at = self.time + f64::from(self.windfall_random.range(8.0, 24.0));
            if self.window_open && self.wind.strength > 0.25 && !wet {
                let position =
                    WINDOW_CENTER + Vec3::new(self.windfall_random.range(-0.55, 0.55), -0.2, 0.08);
                if self.season() == Season::Spring && self.petals.len() < 12 {
                    self.petals.push(Petal {
                        position,
                        velocity: self.breeze_at(position),
                        spin: self.windfall_random.range(0.0, std::f32::consts::TAU),
                        dropped: self.time,
                    });
                } else if self.season() == Season::Autumn && self.leaves.len() < 12 {
                    self.leaves.push(Leaf {
                        position,
                        spin: self.windfall_random.range(0.0, std::f32::consts::TAU),
                        dropped: self.time,
                    });
                }
            }
        }
    }
}

pub(super) fn breeze(open: bool, wind: &Wind, time: f64, position: Vec3) -> Vec3 {
    if !open {
        return Vec3::ZERO;
    }
    let depth = ((ROOM_MAX.z - position.z) / (ROOM_MAX.z - ROOM_MIN.z)).clamp(0.0, 1.0);
    let corridor = (1.0 - position.x.abs() / 2.4).clamp(0.0, 1.0);
    Vec3::new(
        wind.sideways(time) * 0.17,
        0.015 * wind.gust(time),
        wind.gust(time) * 0.36,
    ) * depth
        * corridor
}

pub fn requested(text: &str) -> Option<bool> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|character: char| !character.is_alphabetic())
        .filter(|word| !word.is_empty())
        .collect();
    if words.iter().any(|word| matches!(*word, "not" | "dont"))
        || lower.contains("don't")
        || lower.contains("말")
    {
        return None;
    }
    for (phrase, open) in [
        ("open the window", true),
        ("open window", true),
        ("close the window", false),
        ("close window", false),
        ("shut the window", false),
    ] {
        let phrase: Vec<&str> = phrase.split_whitespace().collect();
        if words.windows(phrase.len()).any(|run| run == phrase) {
            return Some(open);
        }
    }
    if lower.contains("창문") {
        if lower.contains("열어") || lower.contains("열자") {
            return Some(true);
        }
        if lower.contains("닫아") || lower.contains("닫자") {
            return Some(false);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Plan, Room, Understanding};

    #[test]
    fn wind_spells_repeat_without_advancing_the_worlds_random_source() {
        let mut first = Wind::new(17);
        let mut second = Wind::new(17);
        let mut different = Wind::new(18);
        let mut differs = false;
        for step in 0..4000 {
            let time = step as f64 * 0.1;
            for wind in [&mut first, &mut second, &mut different] {
                wind.advance(time, 0.1, Season::Autumn, false, Sky::Clear, false);
            }
            assert_eq!(first.strength, second.strength);
            assert_eq!(first.gust(time), second.gust(time));
            assert_eq!(first.until, second.until);
            differs |= first.gust(time) != different.gust(time);
            assert!((0.0..=1.0).contains(&first.gust(time)));
        }
        assert!(differs);
        let mut world = World::new(17, 12.0);
        let mut untouched = world.clone();
        world.wind = first;
        for _ in 0..20 {
            assert_eq!(world.random_unit(), untouched.random_unit());
        }
    }

    #[test]
    fn gusts_are_smooth_finite_and_bounded_even_after_a_long_time() {
        let mut wind = Wind::new(9);
        wind.strength = 1.0;
        for time in [0.0, 1.0, 1e6, 1e12, f64::MAX] {
            let gust = wind.gust(time);
            assert!(gust.is_finite() && (0.0..=1.0).contains(&gust));
            assert!(wind.sideways(time).is_finite());
            assert_eq!(gust, wind.gust(time));
        }
        for step in 0..1000 {
            let time = step as f64;
            assert!((wind.gust(time + 0.001) - wind.gust(time)).abs() < 0.001);
        }
        let mut storm = wind.clone();
        let mut snow = wind.clone();
        let mut night = wind.clone();
        for step in 0..1000 {
            let time = step as f64 * 0.1;
            storm.advance(time, 0.1, Season::Summer, false, Sky::Rain, true);
            snow.advance(time, 0.1, Season::Winter, false, Sky::Snow, false);
            night.advance(time, 0.1, Season::Summer, true, Sky::Clear, false);
        }
        assert!(storm.strength > 0.79);
        assert!(snow.strength < 0.27);
        assert!(night.strength < 0.13);
    }

    fn finish_air(world: &mut World) {
        world.ghost.position = world.ghost.plan.target;
        world.ghost.velocity = Vec3::ZERO;
        world.step(1.2);
        assert!(world.ghost.plan.done);
    }

    #[test]
    fn a_gusty_spell_or_a_freshly_moved_sash_keeps_it_from_reopening_the_window() {
        let mut world = World::new(5, 12.0);
        world.wind.strength = 0.0;
        assert!(world.options().contains(&Activity::Air));
        world.wind.strength = 0.65;
        assert!(!world.wants_window_shut());
        assert!(!world.options().contains(&Activity::Air));
        world.wind.strength = 0.0;
        world.begin(Activity::Air);
        finish_air(&mut world);
        assert!(world.window_open);
        world.wind.strength = 0.8;
        assert!(world.options().contains(&Activity::Air));
        world.begin(Activity::Air);
        finish_air(&mut world);
        assert!(!world.window_open);
        world.wind.strength = 0.0;
        assert!(!world.options().contains(&Activity::Air));
        world.time += AIRING_PAUSE;
        assert!(world.options().contains(&Activity::Air));
    }

    #[test]
    fn the_sash_changes_once_after_a_push_and_its_position_survives_room_saves() {
        let mut world = World::new(7, 12.0);
        assert!(!world.window_open);
        assert!(world.options().contains(&Activity::Air));
        assert!(world.ask_window(true));
        let started = world.ghost.plan.started;
        world.step(0.2);
        assert!(world.ask_window(true));
        assert_eq!(world.ghost.plan.started, started);
        assert!(!world.window_open);
        finish_air(&mut world);
        assert!(world.window_open);
        assert!(!world.options().contains(&Activity::Air));
        assert!(world.ask_window(true));
        world.step(1.0);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::OpenedWindow)
                .count(),
            1
        );
        assert_eq!(world.life.total.airings, 1);
        assert!(world.life.today.words().contains("fresh air"));
        let mut saved = serde_json::to_value(world.saved_room()).unwrap();
        let mut restored = World::new(2, 12.0);
        restored.restore_room(&serde_json::from_value::<Room>(saved.clone()).unwrap());
        assert!(restored.window_open);
        saved.as_object_mut().unwrap().remove("window_open");
        restored.restore_room(&serde_json::from_value::<Room>(saved).unwrap());
        assert!(!restored.window_open);
        assert!(world.ask_window(false));
        finish_air(&mut world);
        world.step(1.0);
        assert!(!world.window_open);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::ClosedWindow)
                .count(),
            1
        );
        restored.restore_room(&world.saved_room());
        assert!(!restored.window_open);
    }

    #[test]
    fn wet_weather_interrupts_idle_plans_but_never_wakes_a_sleeper_to_shut_the_sash() {
        for activity in [Activity::Float, Activity::Window, Activity::Company] {
            let mut world = World::new(7, 12.0);
            if activity == Activity::Company {
                world.start_focus(25);
            } else {
                world.begin(activity);
            }
            world.window_open = true;
            world.weather.until = 0.01;
            world.step(0.1);
            assert!(matches!(world.weather.sky, Sky::Rain));
            assert_eq!(world.ghost.plan.activity, Activity::Air);
            assert_eq!(world.ghost.plan.window, Some(false));
            finish_air(&mut world);
            assert!(!world.window_open);
        }
        let mut world = World::new(7, 23.0);
        world.date = jiff::civil::Date::constant(2026, 12, 5);
        world.tuck_in();
        world.window_open = true;
        world.weather.until = 0.01;
        world.step(35.0);
        assert_eq!(world.weather.sky, Sky::Snow);
        assert!(world.window_open);
        assert!(world.ghost.asleep > 0.9);
        assert!(world.puddle > 0.9);
        assert!(
            !world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Puddle)
        );
        world.ghost.asleep = 0.0;
        world.ghost.plan = Plan::new(Activity::Float, world.time, world.ghost.position, 10.0);
        world.step(0.1);
        assert_eq!(world.ghost.plan.activity, Activity::Air);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Puddle)
        );
        finish_air(&mut world);
        world.step(601.0);
        assert_eq!(world.puddle, 0.0);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Puddle)
                .count(),
            1
        );
    }

    #[test]
    fn direct_window_requests_bypass_words_and_leave_sleeping_ghosts_asleep() {
        for (text, open) in [
            ("Please OPEN the WINDOW!", true),
            ("창문 열어 줘", true),
            ("close the window", false),
            ("창문 닫아", false),
        ] {
            let mut world = World::new(7, 12.0);
            world.window_open = !open;
            let heard = world.hear(text);
            assert_eq!(
                heard.understanding,
                Understanding::Airing {
                    open,
                    accepted: true
                }
            );
            assert_eq!(world.ghost.plan.activity, Activity::Air);
            assert_eq!(world.ghost.plan.window, Some(open));
            assert!(world.life.vocabulary.words.is_empty());
            world.hour = 23.0;
            world.tuck_in();
            let before = world.ghost.plan.clone();
            world.hear(text);
            assert_eq!(world.ghost.asleep, 1.0);
            assert_eq!(world.ghost.plan, before);
        }
        for text in [
            "reopen the windowsill",
            "don't open the window",
            "do not close the window",
            "창문 열지 말아",
        ] {
            assert_eq!(requested(text), None);
        }
        let mut world = World::new(7, 12.0);
        world.weather.sky = Sky::Rain;
        assert!(!world.ask_window(true));
        world.step(0.1);
        assert!(world.window_shake().abs() > 0.0);
        assert!(!world.window_open);
    }

    #[test]
    fn rain_blows_in_on_a_sleeping_ghost_and_airing_counts_migrate_and_are_clamped() {
        let mut world = World::new(7, 23.0);
        world.window_open = true;
        world.tuck_in();
        world.weather.sky = Sky::Rain;
        world.weather.until = 1000.0;
        world.step(35.0);
        assert!(world.puddle > 0.9 && world.window_open && world.ghost.asleep > 0.9);
        world.window_open = false;
        world.step(601.0);
        assert_eq!(world.puddle, 0.0);
        assert!(world.ghost.asleep > 0.9);
        let mut old = serde_json::to_value(&world.life).unwrap();
        for tally in ["today", "total"] {
            old[tally].as_object_mut().unwrap().remove("airings");
        }
        let mut life = serde_json::from_value::<crate::world::Life>(old).unwrap();
        assert_eq!(life.total.airings, 0);
        life.total.airings = u32::MAX;
        life.today.airings = u32::MAX;
        let clean = life.sanitized();
        assert_eq!(clean.total.airings, 100_000_000);
        assert_eq!(clean.today.airings, 100_000_000);
    }

    #[test]
    fn windfalls_are_capped_expire_and_stay_inside_and_fresh_air_helps() {
        for season in [Season::Spring, Season::Autumn] {
            let mut world = World::new(7, 12.0);
            world.date = if season == Season::Spring {
                jiff::civil::Date::constant(2026, 4, 1)
            } else {
                jiff::civil::Date::constant(2026, 10, 1)
            };
            world.window_open = true;
            world.ghost.asleep = 1.0;
            for _ in 0..2000 {
                world.time += 1.0;
                world.wind.strength = 0.7;
                world.advance_breeze(1.0);
                world.advance_garden(1.0);
                assert!(world.leaves.len() <= 12 && world.petals.len() <= 12);
                for position in world
                    .leaves
                    .iter()
                    .map(|leaf| leaf.position)
                    .chain(world.petals.iter().map(|petal| petal.position))
                {
                    assert!(
                        position.is_finite()
                            && position.cmpge(ROOM_MIN).all()
                            && position.cmple(ROOM_MAX).all()
                    );
                }
            }
            assert!(if season == Season::Spring {
                !world.petals.is_empty()
            } else {
                !world.leaves.is_empty()
            });
            world.window_open = false;
            world.time += Leaf::LIFETIME + Petal::LIFETIME;
            world.advance_breeze(0.1);
            world.advance_garden(0.1);
            assert!(world.leaves.is_empty() && world.petals.is_empty());
        }
        let mut world = World::new(7, 12.0);
        world.window_open = true;
        world.ghost.position = WINDOW_CENTER + Vec3::Z * 0.5;
        let mut shut = world.clone();
        shut.window_open = false;
        world.step(1.0);
        shut.step(1.0);
        assert!(world.ghost.needs.fun > shut.ghost.needs.fun);
        assert_eq!(world.ghost.mood, Mood::Cheerful);
        assert_eq!(shut.breeze_at(WINDOW_CENTER), Vec3::ZERO);
        assert!(world.breeze_at(WINDOW_CENTER).length() > world.breeze_at(Vec3::ZERO).length());
    }
}
