use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{
    Activity, EventKind, ROOM_MAX, ROOM_MIN, Random, Sky, Thing, WINDOW_CENTER, World, flat,
};

pub const NAMES: [&str; 12] = [
    "Lumi", "Pell", "Nim", "Tavi", "Oro", "Wren", "Mote", "Sola", "Fen", "Ilo", "Puck", "Rue",
];

/// The same little ghost returns; only its identity and friendship are saved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Companion {
    pub name: String,
    pub hue: f32,
    pub visits: u32,
    pub closeness: f32,
}

impl Default for Companion {
    fn default() -> Self {
        Self {
            name: String::new(),
            hue: 0.0,
            visits: 0,
            closeness: 0.0,
        }
    }
}

impl Companion {
    fn new(random: &mut Random) -> Self {
        Self {
            name: NAMES[(random.unit() * NAMES.len() as f32) as usize].to_owned(),
            hue: random.unit(),
            ..Self::default()
        }
    }

    pub fn closeness_words(&self) -> &'static str {
        match self.closeness {
            closeness if closeness < 0.2 => "still a new friend",
            closeness if closeness < 0.5 => "a familiar friend",
            closeness if closeness < 0.8 => "a good friend",
            _ => "an old friend",
        }
    }

    pub fn visit_words(&self) -> String {
        match self.visits {
            0 => "not yet".to_owned(),
            1 => "once".to_owned(),
            2 => "twice".to_owned(),
            count => format!("{} times", count_words(count)),
        }
    }
}

/// Counts spoken to the minds must stay words, even after years of visits.
pub fn count_words(count: u32) -> String {
    const SMALL: [&str; 20] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
    ];
    const TENS: [&str; 10] = [
        "", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    if count < 20 {
        return SMALL[count as usize].to_owned();
    }
    if count < 100 {
        return if count.is_multiple_of(10) {
            TENS[(count / 10) as usize].to_owned()
        } else {
            format!(
                "{} {}",
                TENS[(count / 10) as usize],
                SMALL[(count % 10) as usize]
            )
        };
    }
    for (size, word) in [
        (1_000_000_000, "billion"),
        (1_000_000, "million"),
        (1_000, "thousand"),
        (100, "hundred"),
    ] {
        if count >= size {
            let prefix = format!("{} {word}", count_words(count / size));
            return if count.is_multiple_of(size) {
                prefix
            } else {
                format!("{prefix} {}", count_words(count % size))
            };
        }
    }
    unreachable!()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Arriving,
    Greeting,
    Wandering,
    Playing,
    Leaving,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Game {
    Tag,
    Twirl,
    MoonGazing,
}

impl Game {
    pub fn words(self) -> &'static str {
        match self {
            Self::Tag => "tag",
            Self::Twirl => "a twirl",
            Self::MoonGazing => "moon gazing",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Playing {
    pub game: Game,
    pub friend_is_it: bool,
    last_tag: f64,
    center: Vec3,
}

/// A visit in progress, gone after a restart.
#[derive(Clone, Debug, PartialEq)]
pub struct Friend {
    pub position: Vec3,
    pub velocity: Vec3,
    pub heading: Vec3,
    pub phase: f32,
    pub arrived_at: f64,
    pub leaves_at: f64,
    pub state: State,
    pub poked_at: f64,
    pub(super) game: Option<Playing>,
    target: Vec3,
    next_wander: f64,
    greeting_until: f64,
    departed_at: Option<f64>,
    hurried: bool,
    look: Option<Vec3>,
}

impl Friend {
    fn new(time: f64, random: &mut Random) -> Self {
        Self {
            position: WINDOW_CENTER - Vec3::Z * 0.45,
            velocity: Vec3::Z * 0.3,
            heading: Vec3::Z,
            phase: random.range(0.0, TAU),
            arrived_at: time,
            leaves_at: time + f64::from(random.range(180.0, 360.0)),
            state: State::Arriving,
            poked_at: f64::NEG_INFINITY,
            game: None,
            target: WINDOW_CENTER + Vec3::Z * 0.65,
            next_wander: time,
            greeting_until: time,
            departed_at: None,
            hurried: false,
            look: None,
        }
    }

    pub fn visibility(&self) -> f32 {
        ((self.position.z - ROOM_MIN.z + 0.45) / 0.45).clamp(0.0, 1.0)
    }

    pub fn doing(&self) -> &'static str {
        match self.state {
            State::Arriving => "floating in through the window",
            State::Greeting => "greeting you with a happy bob",
            State::Leaving if self.hurried => "hurrying away through the window",
            State::Leaving => "waving goodbye and floating toward the window",
            State::Playing => match &self.game {
                Some(playing) => match playing.game {
                    Game::Tag if playing.friend_is_it => {
                        "playing tag with you, and your friend is it"
                    }
                    Game::Tag => "playing tag with you, and you are it",
                    Game::Twirl => "twirling with you",
                    Game::MoonGazing => "gazing at the moon beside you",
                },
                None => "waiting for another game",
            },
            State::Wandering if self.look == Some(WINDOW_CENTER) => "peeking out of the window",
            State::Wandering if self.look.is_some() => "wandering near the plant",
            State::Wandering => "drifting near you and watching what you do",
        }
    }
}

pub fn visit_chance(phase: f32, company: f32) -> f32 {
    let fullness = (1.0 - (phase * TAU).cos()) * 0.5;
    0.15 + 0.5 * fullness + 0.25 * (1.0 - company.clamp(0.0, 1.0))
}

impl World {
    pub(crate) fn friend_is_open(&self) -> bool {
        self.ghost.asleep < 0.3
            && self.ghost.needs.energy > 0.15
            && self
                .friend
                .as_ref()
                .is_some_and(|friend| friend.state != State::Leaving)
    }

    pub(super) fn invite_friend(&mut self) {
        if self.life.friend.is_none() {
            self.life.friend = Some(Companion::new(&mut self.random));
        }
        let companion = self.life.friend.as_ref().unwrap();
        let name = companion.name.clone();
        if companion.visits == 0 {
            self.first_friend_at = Some(self.time);
        }
        self.friend = Some(Friend::new(self.time, &mut self.random));
        self.visitors.friend_visited = Some(self.date);
        self.record(EventKind::FriendArrived(name));
        self.attend("friend", 12.0);
        if matches!(
            self.ghost.plan.activity,
            Activity::Float | Activity::Explore | Activity::Window | Activity::Wait
        ) {
            self.ghost.plan.done = true;
        }
    }

    pub(super) fn advance_friend(&mut self) {
        if self.friend.is_none() {
            if !(17.5..21.75).contains(&self.hour)
                || self.visitors.friend_visited == Some(self.date)
            {
                return;
            }
            if self.visitors.friend_evening != Some(self.date) {
                self.visitors.friend_evening = Some(self.date);
                if self.life.today.friend_visits > 0 && self.life.day == self.date.to_string() {
                    self.visitors.friend_visited = Some(self.date);
                    return;
                }
                self.visitors.next_friend = self.time + f64::from(self.random.range(20.0, 65.0));
            }
            if self.time < self.visitors.next_friend {
                return;
            }
            self.visitors.next_friend = self.time + f64::from(self.random.range(90.0, 180.0));
            if self.ghost.asleep < 0.3
                && matches!(self.weather.sky, Sky::Clear | Sky::Rainbow)
                && !self.weather.storm
                && self.random.unit() < visit_chance(self.moon_phase(), self.ghost.needs.company)
            {
                self.invite_friend();
            }
            return;
        }
        let wet = matches!(self.weather.sky, Sky::Rain | Sky::Snow) || self.weather.storm;
        let bedtime = self.bedtime();
        let friend = self.friend.as_mut().unwrap();
        if friend.state != State::Leaving
            && (wet || bedtime || self.ghost.asleep >= 0.3 || self.time >= friend.leaves_at)
        {
            friend.state = State::Leaving;
            friend.departed_at = Some(self.time);
            friend.hurried = wet;
            friend.game = None;
            friend.velocity.y += if wet { 0.0 } else { 0.3 };
            if self.ghost.plan.activity == Activity::Friend {
                self.ghost.plan.done = true;
            }
        }
    }

    pub(super) fn move_friend(&mut self, seconds: f32) {
        let Some(mut friend) = self.friend.take() else {
            return;
        };
        let time = self.time;
        let phase = (time.rem_euclid(1000.0) as f32) + friend.phase;
        let closeness = self
            .life
            .friend
            .as_ref()
            .map_or(0.0, |friend| friend.closeness);
        match friend.state {
            State::Arriving => {
                if friend.position.distance(friend.target) < 0.15 {
                    friend.state = State::Greeting;
                    friend.greeting_until = time + 4.0;
                }
            }
            State::Greeting => {
                friend.target =
                    self.ghost.position + Vec3::new(-0.65, 0.08 * (phase * 3.0).sin(), -0.2);
                if time >= friend.greeting_until {
                    friend.state = State::Wandering;
                }
            }
            State::Playing
                if self.ghost.plan.activity != Activity::Friend || self.ghost.plan.done =>
            {
                friend.state = State::Wandering;
                friend.game = None;
            }
            State::Wandering => {
                if time >= friend.next_wander {
                    friend.next_wander = time + f64::from(self.random.range(5.0, 10.0));
                    let destination = self.random.pick(&[5.0, 1.0, 1.0]).unwrap();
                    friend.look = match destination {
                        1 => Some(self.plant.top()),
                        2 => Some(WINDOW_CENTER),
                        _ => None,
                    };
                    friend.target = match destination {
                        0 => {
                            self.ghost.position
                                + Vec3::new(self.random.range(-0.8, 0.8), 0.12, -0.35)
                        }
                        1 => self.plant.top() + Vec3::new(-0.4, 0.3, 0.4),
                        _ => WINDOW_CENTER + Vec3::new(0.3, 0.0, 0.65),
                    };
                }
            }
            State::Leaving => {
                let waving = !friend.hurried
                    && friend
                        .departed_at
                        .is_some_and(|departed| time - departed < 1.2);
                friend.target = WINDOW_CENTER
                    + if waving {
                        Vec3::new((phase * 6.0).sin() * 0.12, 0.1 * (phase * 6.0).cos(), 0.3)
                    } else {
                        -Vec3::Z * 0.6
                    };
            }
            State::Playing => {}
        }
        let crossing = matches!(friend.state, State::Arriving | State::Leaving);
        let low = ROOM_MIN + Vec3::new(0.25, 0.5, 0.25);
        let high = ROOM_MAX - Vec3::new(0.25, 0.3, 0.8 - 0.4 * closeness);
        let target = if crossing {
            friend.target
        } else {
            friend.target.clamp(low, high)
        };
        let speed = if friend.hurried {
            2.2
        } else if friend.state == State::Playing {
            1.45
        } else {
            0.75
        };
        let offset = target + Vec3::Y * (0.035 * (phase * 2.0).sin()) - friend.position;
        let desired = offset.normalize_or_zero() * speed * (offset.length() / 0.35).min(1.0);
        let mut steer = (desired - friend.velocity) * 3.0;
        for axis in 0..3 {
            if crossing && axis == 2 {
                continue;
            }
            if friend.position[axis] < low[axis] {
                steer[axis] += (low[axis] - friend.position[axis]) * 8.0;
            }
            if friend.position[axis] > high[axis] {
                steer[axis] -= (friend.position[axis] - high[axis]) * 8.0;
            }
        }
        for (center, radius) in self.obstacles() {
            let offset = flat(friend.position - center);
            let distance = offset.length();
            let reach = radius + 0.25;
            if friend.position.y < 1.65 && distance < reach && distance > 1e-4 {
                let push = offset / distance * (reach - distance) * 8.0;
                steer += Vec3::new(push.x, 0.0, push.y);
            }
        }
        friend.velocity += steer.clamp_length_max(4.0) * seconds;
        friend.position += friend.velocity * seconds;
        let mut minimum = ROOM_MIN + Vec3::new(0.15, 0.38, 0.15);
        if crossing {
            minimum.z = ROOM_MIN.z - 0.7;
        }
        friend.position = friend.position.clamp(minimum, ROOM_MAX - Vec3::splat(0.15));
        let looking = if friend
            .game
            .as_ref()
            .is_some_and(|game| game.game == Game::MoonGazing)
        {
            WINDOW_CENTER
        } else if friend.state == State::Wandering {
            friend.look.unwrap_or(self.ghost.position)
        } else {
            self.ghost.position
        };
        friend.heading = friend
            .heading
            .lerp(
                (looking - friend.position).normalize_or(Vec3::Z),
                (seconds * 2.5).min(1.0),
            )
            .normalize_or(Vec3::Z);
        if friend.state == State::Leaving && friend.position.z < ROOM_MIN.z - 0.44 {
            self.record(EventKind::FriendLeft(
                self.life.friend.as_ref().unwrap().name.clone(),
            ));
        } else {
            self.friend = Some(friend);
        }
    }

    pub(super) fn begin_friend_game(&mut self) {
        if !self.friend_is_open() {
            self.ghost.plan.done = true;
            return;
        }
        let playful = self.life.nature.unwrap_or_default().playful;
        let happy = matches!(
            self.ghost.mood,
            super::Mood::Excited | super::Mood::Cheerful
        );
        let moonlit = self.is_night() && (self.moon_phase() * TAU).cos() < 0.0;
        let index = self
            .random
            .pick(&[
                1.0 + playful,
                if happy { 1.6 } else { 0.6 },
                if moonlit { 2.2 } else { 0.15 },
            ])
            .unwrap();
        let game = [Game::Tag, Game::Twirl, Game::MoonGazing][index];
        let length = match game {
            Game::Tag => self.random.range(25.0, 40.0),
            Game::Twirl => 10.0,
            Game::MoonGazing => 20.0,
        };
        let center = ((self.ghost.position + self.friend.as_ref().unwrap().position) * 0.5)
            .clamp(Vec3::new(-1.0, 0.9, -0.5), Vec3::new(1.0, 1.65, 0.5));
        let friend = self.friend.as_mut().unwrap();
        friend.game = Some(Playing {
            game,
            friend_is_it: self.random.unit() < 0.5,
            last_tag: self.time,
            center,
        });
        // Finish the entrance before joining the game.
        self.ghost.plan.length = length;
        self.ghost.plan.anchor = center;
    }

    pub(super) fn play_with_friend(&mut self) {
        let Some(friend) = &mut self.friend else {
            self.ghost.plan.done = true;
            return;
        };
        if friend.state == State::Leaving || self.ghost.asleep >= 0.3 {
            self.ghost.plan.done = true;
            return;
        }
        if matches!(friend.state, State::Arriving | State::Greeting) {
            self.ghost.plan.started = self.time;
            return;
        }
        let Some(playing) = &mut friend.game else {
            self.ghost.plan.done = true;
            return;
        };
        friend.state = State::Playing;
        let elapsed = self.ghost.plan.elapsed(self.time);
        let phase = elapsed + friend.phase;
        let mut tagged = None;
        match playing.game {
            Game::Tag => {
                if self.time - playing.last_tag > 1.5
                    && friend.position.distance(self.ghost.position) < 0.42
                {
                    tagged = Some(playing.friend_is_it);
                    playing.friend_is_it = !playing.friend_is_it;
                    playing.last_tag = self.time;
                }
                let away = if playing.friend_is_it {
                    self.ghost.position - friend.position
                } else {
                    friend.position - self.ghost.position
                };
                let away = away.normalize_or(Vec3::X);
                let swerve =
                    Vec3::new(-away.z, 0.15 * (phase * 1.7).sin(), away.x) * (phase * 1.3).sin();
                let runner = (if playing.friend_is_it {
                    self.ghost.position
                } else {
                    friend.position
                }) + away * 1.1
                    + swerve * 0.7;
                let runner = runner.clamp(Vec3::new(-1.4, 0.8, -0.8), Vec3::new(1.4, 1.9, 0.8));
                if playing.friend_is_it {
                    friend.target = self.ghost.position;
                    self.ghost.plan.target = runner;
                } else {
                    self.ghost.plan.target = friend.position;
                    friend.target = runner;
                }
            }
            Game::Twirl => {
                let offset = Vec3::new(
                    (elapsed * 1.2).cos() * 0.48,
                    (elapsed * 1.8).sin() * 0.18,
                    (elapsed * 1.2).sin() * 0.48,
                );
                self.ghost.plan.target = playing.center + offset;
                friend.target = playing.center - offset;
            }
            Game::MoonGazing => {
                self.ghost.plan.target = WINDOW_CENTER + Vec3::new(-0.35, 0.0, 0.6);
                friend.target = WINDOW_CENTER + Vec3::new(0.35, 0.0, 0.6);
                self.ghost.attention = Some((Thing::Window, self.time + 1.0));
            }
        }
        let finished = (elapsed >= self.ghost.plan.length).then_some(playing.game);
        if let Some(by_friend) = tagged {
            self.record(EventKind::Tagged { by_friend });
        }
        if let Some(game) = finished {
            self.record(EventKind::PlayedWithFriend(game));
            if game == Game::Twirl {
                self.record(EventKind::Danced);
            }
            self.ghost.plan.done = true;
            self.ghost.needs.change(0.0, 0.08, 0.04);
        }
    }

    pub fn poke_friend(&mut self) {
        let Some(friend) = &mut self.friend else {
            return;
        };
        if friend.visibility() <= 0.0 {
            return;
        }
        friend.poked_at = self.time;
        friend.velocity += Vec3::new(self.random.range(-0.35, 0.35), 0.65, -0.45);
        self.record(EventKind::PokedFriend);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Mood, Question, Understanding};

    fn evening() -> World {
        let mut world = World::new(1, 20.0);
        world.date = jiff::civil::Date::constant(2026, 10, 25);
        world.weather.until = f64::INFINITY;
        world.step(0.0);
        world
    }

    fn visiting() -> World {
        let mut world = evening();
        world.invite_friend();
        world
    }

    #[test]
    fn moonlight_and_loneliness_raise_the_chance_of_a_visit() {
        assert!((visit_chance(0.0, 1.0) - 0.15).abs() < 1e-5);
        assert!((visit_chance(0.5, 1.0) - 0.65).abs() < 1e-5);
        assert!((visit_chance(0.5, 0.0) - 0.9).abs() < 1e-5);
        assert_eq!(visit_chance(0.25, 0.4), visit_chance(0.75, 0.4));
    }

    #[test]
    fn visits_require_an_awake_ghost_and_a_clear_evening() {
        for (hour, sky, storm, asleep) in [
            (12.0, Sky::Clear, false, 0.0),
            (17.49, Sky::Clear, false, 0.0),
            (21.75, Sky::Clear, false, 0.0),
            (22.0, Sky::Clear, false, 0.0),
            (20.0, Sky::Rain, false, 0.0),
            (20.0, Sky::Snow, false, 0.0),
            (20.0, Sky::Clear, true, 0.0),
            (20.0, Sky::Clear, false, 1.0),
        ] {
            let mut world = evening();
            world.hour = hour;
            world.weather.sky = sky;
            world.weather.storm = storm;
            world.ghost.asleep = asleep;
            for _ in 0..12 {
                world.advance_friend();
                world.time += 180.0;
            }
            assert!(world.friend.is_none(), "{hour} {sky:?} {storm} {asleep}");
        }
    }

    #[test]
    fn an_evening_started_in_the_app_gets_a_short_first_wait() {
        for hour in [17.5, 20.0, 21.7] {
            let mut world = evening();
            world.hour = hour;
            world.advance_friend();
            assert!(world.friend.is_none());
            assert!((20.0..=65.0).contains(&world.visitors.next_friend));
        }
    }

    #[test]
    fn a_seeded_full_moon_evening_brings_the_friend_within_five_minutes() {
        let mut world = evening();
        assert!((world.moon_phase() - 0.5).abs() < 0.02);
        for _ in 0..3000 {
            world.step(0.1);
            if world.friend.is_some() {
                break;
            }
        }
        assert!(world.friend.is_some());
        assert!(world.time < 300.0);
        assert_eq!(world.life.total.friend_visits, 1);
    }

    #[test]
    fn the_first_meeting_names_the_friend_and_later_visits_keep_that_identity() {
        let mut world = visiting();
        let companion = world.life.friend.clone().unwrap();
        assert!(NAMES.contains(&companion.name.as_str()));
        assert_eq!(companion.visits, 1);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::FriendArrived(companion.name.clone()))
        );
        let arrival = world
            .events
            .iter()
            .find(|event| matches!(event.kind, EventKind::FriendArrived(_)))
            .unwrap();
        assert_eq!(
            world.event_words(arrival),
            "a ghost you've never met floated in through the window"
        );
        let meeting = format!(
            "I met another ghost tonight. Their name is {}.",
            companion.name
        );
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == meeting)
                .count(),
            1
        );
        world.friend = None;
        world.time += 86_400.0;
        world.date = world.date.tomorrow().unwrap();
        world.invite_friend();
        let returning = world.life.friend.as_ref().unwrap();
        assert_eq!(returning.name, companion.name);
        assert_eq!(returning.hue, companion.hue);
        assert_eq!(returning.visits, 2);
        assert!(returning.closeness > companion.closeness);
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == meeting)
                .count(),
            1
        );
        assert!(
            world
                .events
                .iter()
                .rev()
                .find(|event| matches!(event.kind, EventKind::FriendArrived(_)))
                .is_some_and(|event| world.event_words(event).contains(&companion.name))
        );
    }

    #[test]
    fn a_visit_ends_on_time_through_the_window_and_cannot_repeat_that_evening() {
        let mut world = visiting();
        let friend = world.friend.as_ref().unwrap();
        let leaves_at = friend.leaves_at;
        assert!((180.0..=360.0).contains(&leaves_at));
        assert_eq!(friend.position, WINDOW_CENTER - Vec3::Z * 0.45);
        world.step(12.0);
        world.step((leaves_at - world.time - 0.2) as f32);
        assert_ne!(world.friend.as_ref().unwrap().state, State::Leaving);
        world.time = leaves_at;
        world.advance_friend();
        assert_eq!(world.friend.as_ref().unwrap().state, State::Leaving);
        let mut crossed = false;
        for _ in 0..200 {
            if let Some(friend) = &world.friend {
                crossed |= friend.position.z < ROOM_MIN.z;
            }
            world.step(0.1);
            if world.friend.is_none() {
                break;
            }
        }
        assert!(crossed);
        assert!(world.friend.is_none());
        assert!(
            world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::FriendLeft(_)))
        );
        assert_eq!(world.ghost.mood, Mood::Lonely);
        world.step(300.0);
        assert!(world.friend.is_none());
        assert_eq!(world.life.total.friend_visits, 1);
        world.date = world.date.tomorrow().unwrap();
        world.advance_friend();
        assert!(world.visitors.next_friend > world.time);
        assert!(world.visitors.next_friend < world.time + 66.0);
    }

    #[test]
    fn bedtime_sleep_and_wet_weather_end_a_visit_and_interrupt_its_game() {
        for cause in 0..5 {
            let mut world = visiting();
            world.begin(Activity::Friend);
            match cause {
                0 => world.hour = 22.0,
                1 => world.ghost.asleep = 1.0,
                2 => world.weather.sky = Sky::Rain,
                3 => world.weather.sky = Sky::Snow,
                _ => world.weather.storm = true,
            }
            world.advance_friend();
            let friend = world.friend.as_ref().unwrap();
            assert_eq!(friend.state, State::Leaving);
            assert_eq!(friend.hurried, cause >= 2);
            assert!(world.ghost.plan.done);
            world.step(20.0);
            assert!(world.friend.is_none());
        }
    }

    #[test]
    fn a_weather_change_in_the_world_sends_the_friend_home_at_once() {
        let mut world = visiting();
        world.step(12.0);
        world.weather.until = 0.0;
        world.step(0.1);
        assert_eq!(world.friend.as_ref().unwrap().state, State::Leaving);
        assert!(world.friend.as_ref().unwrap().hurried);
    }

    #[test]
    fn playing_is_only_offered_with_a_friend_and_enough_waking_energy() {
        let mut world = evening();
        assert!(!world.options().contains(&Activity::Friend));
        world.invite_friend();
        assert!(world.options().contains(&Activity::Friend));
        world.ghost.needs.energy = 0.15;
        assert!(!world.options().contains(&Activity::Friend));
        world.ghost.needs.energy = 0.8;
        world.ghost.asleep = 0.3;
        assert!(!world.options().contains(&Activity::Friend));
        world.ghost.asleep = 0.0;
        world.friend.as_mut().unwrap().state = State::Leaving;
        assert!(!world.options().contains(&Activity::Friend));
    }

    #[test]
    fn each_game_finishes_counts_once_and_keeps_both_ghosts_in_the_room() {
        for game in [Game::Tag, Game::Twirl, Game::MoonGazing] {
            let mut world = visiting();
            world.begin(Activity::Friend);
            let friend = world.friend.as_mut().unwrap();
            friend.state = State::Wandering;
            friend.position = WINDOW_CENTER + Vec3::Z * 0.65;
            friend.game.as_mut().unwrap().game = game;
            world.ghost.plan.length = match game {
                Game::Tag => 30.0,
                Game::Twirl => 10.0,
                Game::MoonGazing => 20.0,
            };
            for _ in 0..450 {
                world.step(0.1);
                for position in [
                    world.ghost.position,
                    world.friend.as_ref().unwrap().position,
                ] {
                    assert!(
                        position.cmpge(ROOM_MIN).all() && position.cmple(ROOM_MAX).all(),
                        "{game:?}: {position}"
                    );
                }
                if world.ghost.plan.done {
                    break;
                }
            }
            assert!(world.ghost.plan.done, "{game:?}");
            assert_eq!(world.life.total.friend_games, 1);
            assert_eq!(world.life.total.dances, u32::from(game == Game::Twirl));
            world.step(1.0);
            assert_eq!(world.life.total.friend_games, 1);
        }
    }

    #[test]
    fn a_tag_switches_the_chaser_and_leaves_time_to_run_away() {
        for friend_is_it in [false, true] {
            let mut world = visiting();
            world.begin(Activity::Friend);
            let friend = world.friend.as_mut().unwrap();
            friend.state = State::Playing;
            friend.position = world.ghost.position + Vec3::X * 0.1;
            let playing = friend.game.as_mut().unwrap();
            playing.game = Game::Tag;
            playing.friend_is_it = friend_is_it;
            world.time = 2.0;
            world.play_with_friend();
            assert_eq!(
                world
                    .friend
                    .as_ref()
                    .unwrap()
                    .game
                    .as_ref()
                    .unwrap()
                    .friend_is_it,
                !friend_is_it
            );
            assert!(world.events.iter().any(|event| event.kind
                == EventKind::Tagged {
                    by_friend: friend_is_it
                }));
            world.time += 0.1;
            world.play_with_friend();
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| matches!(event.kind, EventKind::Tagged { .. }))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn nearby_company_helps_and_a_poke_bobs_the_friend_without_cooling_the_bond() {
        let mut world = visiting();
        world.friend.as_mut().unwrap().position = world.ghost.position + Vec3::X * 0.7;
        let company = world.ghost.needs.company;
        world.update_needs(10.0);
        assert!(world.ghost.needs.company > company);
        let mut alone = world.clone();
        alone.friend = None;
        let company = alone.ghost.needs.company;
        alone.update_needs(10.0);
        assert!(alone.ghost.needs.company < company);
        let bond = world.life.bond;
        world.poke_friend();
        assert_eq!(world.life.bond, bond);
        assert!(world.friend.as_ref().unwrap().velocity.y > 0.0);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::PokedFriend)
        );
    }

    #[test]
    fn friendship_landmarks_are_kept_once_and_visits_and_games_reach_the_diary() {
        let mut world = visiting();
        let name = world.life.friend.as_ref().unwrap().name.clone();
        for _ in 1..5 {
            world.record(EventKind::FriendArrived(name.clone()));
        }
        for _ in 0..16 {
            world.record(EventKind::PlayedWithFriend(Game::Tag));
        }
        for text in [
            format!("{name} has come to see me five times now."),
            format!("{name} and I are good friends now."),
        ] {
            assert_eq!(
                world
                    .life
                    .milestones
                    .iter()
                    .filter(|entry| entry.text == text)
                    .count(),
                1
            );
            assert!(!text.chars().any(|character| character.is_ascii_digit()));
        }
        assert!(
            world
                .life
                .today
                .words()
                .contains(&format!("{name} visited five times and we played tag"))
        );
        let closeness = world.life.friend.as_ref().unwrap().closeness;
        world.life.miss(86_400.0 * 30.0);
        assert_eq!(world.life.friend.as_ref().unwrap().closeness, closeness);
    }

    #[test]
    fn old_lives_load_without_a_friend_and_the_identity_round_trips() {
        let old: Life = serde_json::from_str("{}").unwrap();
        assert!(old.friend.is_none());
        assert_eq!(old.total.friend_games, 0);
        let life = visiting().life;
        let restored: Life = serde_json::from_str(&serde_json::to_string(&life).unwrap()).unwrap();
        assert_eq!(restored, life);
        let mut fresh = evening();
        fresh.life = restored;
        assert!(fresh.friend.is_none());
        fresh.invite_friend();
        assert_eq!(fresh.life.friend.as_ref().unwrap().visits, 2);
    }

    #[test]
    fn damaged_saved_friend_values_are_clamped_and_invalid_names_are_dropped() {
        let mut life = Life {
            friend: Some(Companion {
                name: "Lumi".to_owned(),
                hue: f32::NAN,
                visits: u32::MAX,
                closeness: 4.0,
            }),
            ..Life::default()
        }
        .sanitized();
        let friend = life.friend.as_ref().unwrap();
        assert_eq!(friend.hue, 0.0);
        assert_eq!(friend.closeness, 1.0);
        assert_eq!(friend.visits, 1_000_000);
        for name in ["", " ", "Lu\nmi", "Lu\u{1b}mi"] {
            life.friend.as_mut().unwrap().name = name.to_owned();
            assert!(life.clone().sanitized().friend.is_none());
        }
        life.friend.as_mut().unwrap().name = "Lumi".to_owned();
        life.friend.as_mut().unwrap().hue = 9.0;
        life.friend.as_mut().unwrap().closeness = f32::INFINITY;
        let clamped = life.sanitized().friend.unwrap();
        assert_eq!(clamped.hue, 1.0);
        assert_eq!(clamped.closeness, 0.0);
    }

    #[test]
    fn the_friend_is_a_visible_thing_and_questions_do_not_teach_activity_words() {
        let mut world = visiting();
        assert_eq!(
            world.position_of(Thing::Friend),
            Some(world.friend.as_ref().unwrap().position)
        );
        world.attend("friend", 4.0);
        assert_eq!(world.ghost.attention.unwrap().0, Thing::Friend);
        for text in ["Who is your friend?", "친구 이름은 뭐야?"] {
            let heard = world.hear(text);
            assert!(matches!(
                heard.understanding,
                Understanding::Asked {
                    question: Question::Friend,
                    ..
                }
            ));
        }
        world.friend = None;
        assert_eq!(world.position_of(Thing::Friend), None);
    }

    #[test]
    fn the_same_seed_repeats_the_visit_games_and_friendship() {
        let play = || {
            let mut world = evening();
            for _ in 0..2400 {
                if world.needs_decision() {
                    world.begin(if world.friend_is_open() {
                        Activity::Friend
                    } else {
                        Activity::Float
                    });
                }
                world.step(0.1);
            }
            world
        };
        let first = play();
        let second = play();
        assert_eq!(first.friend, second.friend);
        assert_eq!(first.life, second.life);
        assert_eq!(first.events, second.events);
        assert_eq!(first.ghost.position, second.ghost.position);
        assert!(first.life.total.friend_visits > 0);
        assert!(first.life.total.friend_games > 0);
    }

    #[test]
    fn reopening_the_same_evening_does_not_invite_the_friend_twice() {
        let saved = visiting().saved_life();
        let mut restored = evening();
        restored.life = saved;
        for _ in 0..6 {
            restored.time += 180.0;
            restored.advance_friend();
        }
        assert!(restored.friend.is_none());
        assert_eq!(restored.life.friend.as_ref().unwrap().visits, 1);
        restored.date = restored.date.tomorrow().unwrap();
        restored.step(0.0);
        restored.advance_friend();
        assert!(restored.visitors.next_friend < restored.time + 66.0);
    }

    #[test]
    fn a_farewell_is_lonely_until_it_fades_or_the_viewer_returns() {
        let mut world = visiting();
        world.time += 1.0;
        world.friend = None;
        world.record(EventKind::FriendLeft(
            world.life.friend.as_ref().unwrap().name.clone(),
        ));
        assert_eq!(world.feeling(), Mood::Lonely);
        world.notice_viewer();
        assert_ne!(world.feeling(), Mood::Lonely);
        world.viewer.active_at = 0.0;
        world.time += 21.0;
        assert_ne!(world.feeling(), Mood::Lonely);
    }

    #[test]
    fn even_large_visit_counts_are_written_only_in_words() {
        assert_eq!(count_words(25), "twenty five");
        assert_eq!(count_words(105), "one hundred five");
        assert!(
            !count_words(u32::MAX)
                .chars()
                .any(|character| character.is_ascii_digit())
        );
    }
}
