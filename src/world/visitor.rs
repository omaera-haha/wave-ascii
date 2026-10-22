use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{
    ARRIVED, Activity, EventKind, Lamp, Life, ROOM_MAX, ROOM_MIN, Random, SHELF_POSITION,
    SILL_POSITION, Season, Sky, Thing, WINDOW_CENTER, World,
};

const BIRD_FLIGHT: f64 = 3.0;
const MOTH_FLIGHT: f64 = 4.0;

/// A little gift from the bird, kept between runs in the ghost's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Keepsake {
    Feather,
    Petal,
    Acorn,
    Leaf,
    Berry,
    Pebble,
}

impl Keepsake {
    pub fn name(self) -> &'static str {
        match self {
            Self::Feather => "a feather",
            Self::Petal => "a blossom petal",
            Self::Acorn => "an acorn",
            Self::Leaf => "a red leaf",
            Self::Berry => "a holly berry",
            Self::Pebble => "a smooth pebble",
        }
    }

    /// More than one of it, after a number.
    pub fn plural(self) -> &'static str {
        match self {
            Self::Feather => "feathers",
            Self::Petal => "blossom petals",
            Self::Acorn => "acorns",
            Self::Leaf => "red leaves",
            Self::Berry => "holly berries",
            Self::Pebble => "smooth pebbles",
        }
    }

    /// Its name in Korean.
    pub fn korean(self) -> &'static str {
        match self {
            Self::Feather => "깃털",
            Self::Petal => "꽃잎",
            Self::Acorn => "도토리",
            Self::Leaf => "빨간 나뭇잎",
            Self::Berry => "호랑가시 열매",
            Self::Pebble => "조약돌",
        }
    }

    fn choose(season: Season, random: &mut Random) -> Self {
        let choices = match season {
            Season::Spring => [Self::Petal, Self::Feather],
            Season::Summer => [Self::Feather, Self::Pebble],
            Season::Autumn => [Self::Acorn, Self::Leaf],
            Season::Winter => [Self::Berry, Self::Feather],
        };
        choices[usize::from(random.unit() >= 0.5)]
    }
}

/// Visits are temporary; the bird's trust and gifts live in `Life`.
#[derive(Clone, Debug)]
pub struct Visitors {
    pub bird: Option<Bird>,
    pub moth: Option<Moth>,
    pub cat: Option<super::cat::Cat>,
    pub next_cat: f64,
    /// The cat keeps its own draws so its visits never change the rest of
    /// the world's.
    pub(super) cat_random: Random,
    next_bird: f64,
    next_moth: f64,
    pub(super) next_friend: f64,
    pub(super) friend_evening: Option<jiff::civil::Date>,
    /// An evening the friend has come, or won't.
    pub(super) friend_visited: Option<jiff::civil::Date>,
}

impl Visitors {
    pub fn new(random: &mut Random, seed: u64) -> Self {
        let mut cat_random = Random::new(seed ^ 0x0063_6174_6361_7473);
        Self {
            bird: None,
            moth: None,
            cat: None,
            next_cat: f64::from(cat_random.range(600.0, 1800.0)),
            cat_random,
            next_bird: f64::from(random.range(300.0, 900.0)),
            next_moth: f64::from(random.range(300.0, 600.0)),
            next_friend: f64::INFINITY,
            friend_evening: None,
            friend_visited: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Perch {
    Tree,
    Sill,
}

/// A bird outside in the yard: it flies in, perches on the tree's low
/// branch or the window's outer sill, and flies off again.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bird {
    pub perch: Perch,
    pub noticed: bool,
    landed: bool,
    arrived_at: f64,
    leaves_at: f64,
    departing: Option<(f64, Vec3)>,
    side: f32,
    phase: f32,
    next_song: f64,
    sang_at: f64,
}

/// How the bird looks right now: where it is in the world, and its pose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BirdPicture {
    pub position: Vec3,
    pub flying: bool,
    pub flap: f32,
    pub facing: f32,
    pub peck: f32,
    pub singing: bool,
}

impl Bird {
    pub fn new(time: f64, trust: f32, random: &mut Random) -> Self {
        let arrived_at = time + BIRD_FLIGHT;
        Self {
            perch: if random.unit() < 1.0 - sill_chance(trust) {
                Perch::Tree
            } else {
                Perch::Sill
            },
            noticed: false,
            landed: false,
            arrived_at,
            leaves_at: arrived_at + f64::from(random.range(45.0, 120.0)),
            departing: None,
            side: if random.unit() < 0.5 { -1.0 } else { 1.0 },
            phase: random.range(0.0, TAU),
            next_song: arrived_at + f64::from(song_interval(random.range(7.0, 16.0), trust)),
            sang_at: f64::NEG_INFINITY,
        }
    }

    pub fn perched(&self, time: f64) -> bool {
        time >= self.arrived_at && self.departing.is_none()
    }

    pub fn picture(&self, time: f64) -> BirdPicture {
        let hopping = phase(time, 0.8) + self.phase;
        let anchor = match self.perch {
            Perch::Tree => super::home::PERCH + Vec3::new(0.06, 0.12, 0.0),
            Perch::Sill => super::home::SILL + Vec3::new(-0.42, 0.1, 0.0),
        };
        let hop = hopping.sin().max(0.0).powi(18);
        let perched = anchor + Vec3::new(0.02 * hopping.sin(), hop * 0.04, 0.0);
        // Far off over the fence, to one side.
        let away = Vec3::new(self.side * 7.5, 3.6, -7.0);
        let arc = |from: Vec3, to: Vec3, progress: f32| {
            from.lerp(to, progress) + Vec3::Y * (progress * std::f32::consts::PI).sin() * 0.6
        };
        let (position, flying, facing) = if let Some((start, from)) = self.departing {
            let progress = ((time - start) / BIRD_FLIGHT).clamp(0.0, 1.0) as f32;
            (arc(from, away, progress), true, self.side)
        } else if time < self.arrived_at {
            let progress = (1.0 - (self.arrived_at - time) / BIRD_FLIGHT).clamp(0.0, 1.0) as f32;
            (arc(away, anchor, progress), true, -self.side)
        } else {
            (
                perched,
                false,
                if (phase(time, 0.34) + self.phase).sin() > 0.0 {
                    1.0
                } else {
                    -1.0
                },
            )
        };
        BirdPicture {
            position,
            flying,
            flap: phase(time, 13.0) + self.phase,
            facing,
            peck: if flying {
                0.0
            } else {
                (phase(time, 1.7) + self.phase).sin().max(0.0).powi(20)
            },
            singing: (0.0..1.8).contains(&(time - self.sang_at)),
        }
    }

    pub fn position(&self, time: f64) -> Vec3 {
        self.picture(time).position
    }
}

/// A moth that wanders around the bulb and darts away from a close ghost.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moth {
    pub position: Vec3,
    pub noticed: bool,
    born: f64,
    leaves_at: f64,
    corner: Vec3,
    seed: f32,
    lamp_off_at: Option<f64>,
    departing: Option<(f64, Vec3)>,
    dart: Vec3,
    darted_at: f64,
}

impl Moth {
    pub fn new(time: f64, random: &mut Random) -> Self {
        let corner = Vec3::new(
            if random.unit() < 0.5 {
                ROOM_MIN.x + 0.15
            } else {
                ROOM_MAX.x - 0.15
            },
            ROOM_MAX.y - 0.15,
            ROOM_MIN.z + 0.15,
        );
        Self {
            position: corner,
            noticed: false,
            born: time,
            leaves_at: time + MOTH_FLIGHT + f64::from(random.range(60.0, 150.0)),
            corner,
            seed: random.range(0.0, TAU),
            lamp_off_at: None,
            departing: None,
            dart: Vec3::ZERO,
            darted_at: f64::NEG_INFINITY,
        }
    }

    pub fn flap(&self, time: f64) -> f32 {
        (phase(time, 45.0) + self.seed).sin()
    }

    fn orbit(&self, time: f64, lamp: Lamp) -> Vec3 {
        let wave = |speed| (phase(time, speed) + self.seed).sin();
        let dash = wave(3.7).powi(15) * 0.055;
        let radius = if self.lamp_off_at.is_some() {
            0.34
        } else {
            0.22
        };
        lamp.bulb()
            + Vec3::new(
                radius * (phase(time, 2.3) + self.seed).cos() + 0.04 * wave(7.1),
                0.15 * wave(3.1) + 0.04 * wave(8.3) + dash,
                radius * wave(2.3) + 0.05 * wave(5.7) - dash,
            )
    }

    fn advance(&mut self, time: f64, lamp: Lamp, ghost: Vec3) {
        if !lamp.on {
            self.lamp_off_at.get_or_insert(time);
        }
        let leaving =
            time >= self.leaves_at || self.lamp_off_at.is_some_and(|start| time - start >= 8.0);
        if leaving && self.departing.is_none() {
            self.departing = Some((time, self.position));
        }
        let position = if let Some((start, from)) = self.departing {
            from.lerp(
                self.corner,
                ((time - start) / MOTH_FLIGHT).clamp(0.0, 1.0) as f32,
            )
        } else {
            let progress = ((time - self.born) / MOTH_FLIGHT).clamp(0.0, 1.0) as f32;
            self.corner.lerp(self.orbit(time, lamp), progress)
        };
        let dart = self.dart * (-(time - self.darted_at) as f32 / 0.8).exp();
        self.position = position + dart;
        if self.position.distance(ghost) < 0.25 && time - self.darted_at > 0.35 {
            self.dart = (self.position - ghost).normalize_or(Vec3::X) * 0.4;
            let bounded = |point: Vec3| {
                point.clamp(ROOM_MIN + Vec3::splat(0.09), ROOM_MAX - Vec3::splat(0.09))
            };
            if bounded(position + self.dart).distance(ghost) < 0.3 {
                let away = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z]
                    .into_iter()
                    .map(|direction| bounded(position + direction * 0.4))
                    .max_by(|first, second| {
                        first
                            .distance_squared(ghost)
                            .total_cmp(&second.distance_squared(ghost))
                    })
                    .unwrap();
                self.dart = away - position;
            }
            self.darted_at = time;
            self.position = position + self.dart;
        }
        self.position = self
            .position
            .clamp(ROOM_MIN + Vec3::splat(0.09), ROOM_MAX - Vec3::splat(0.09));
    }
}

impl World {
    pub(super) fn advance_visitors(&mut self, seconds: f32) {
        let time = self.time;
        let daytime = (7.0..18.5).contains(&self.hour);
        let bird_weather = self.weather.sky != Sky::Rain && !self.weather.storm;
        if self.visitors.bird.is_none() && time >= self.visitors.next_bird {
            let snow = if self.weather.sky == Sky::Snow {
                2.0
            } else {
                1.0
            };
            self.visitors.next_bird = time
                + f64::from(visit_interval(
                    self.random.range(720.0, 1800.0) * snow,
                    self.life.bird_trust,
                ));
            if daytime && bird_weather {
                self.visitors.bird = Some(Bird::new(time, self.life.bird_trust, &mut self.random));
            }
        }
        if let Some(mut bird) = self.visitors.bird.take() {
            let perched = bird.perched(time);
            if perched && !bird.landed {
                bird.landed = true;
                self.record(EventKind::BirdLanded);
            }
            let offset = bird.position(time) - self.ghost.position;
            let near = offset.length();
            let sensitivity = startle_scale(self.life.bird_trust);
            let rushing = near < 1.1 * sensitivity
                && self.ghost.velocity.length() * sensitivity > 0.9
                && self.ghost.velocity.dot(offset.normalize_or_zero()) * sensitivity > 0.5;
            // A friend lets it reach gently through the pane for a gift.
            let reaching = self.life.bird_trust >= 0.7
                && self.ghost.plan.activity == Activity::Window
                && !self.ghost.plan.done
                && matches!(self.ghost.plan.count, 1 | 2);
            let startled = perched && (rushing || near < 0.5 * sensitivity && !reaching);
            if perched
                && bird.noticed
                && !startled
                && self.ghost.asleep < 0.3
                && self.ghost.plan.activity == Activity::Window
                && !self.ghost.plan.done
                && self.ghost.plan.count == 0
                && self.ghost.plan.arrived.is_some()
                && self.ghost.position.distance(self.ghost.plan.target) < ARRIVED
                && self.ghost.velocity.length() < 0.3
                && let Some(milestone) = self.life.watch_bird(seconds)
            {
                self.record(EventKind::Milestone(milestone));
            }
            if bird.departing.is_none()
                && (!daytime || !bird_weather || time >= bird.leaves_at || startled)
                && perched
            {
                self.record(EventKind::BirdFlewOff { startled });
                if startled && self.ghost.asleep < 0.5 {
                    self.think("Oh… I scared it.");
                }
                if startled {
                    self.life.bird_trust = startled_trust(self.life.bird_trust);
                } else if self.life.bird_trust >= 0.4
                    && self.life.sill.is_none()
                    && self.brings_gift_today()
                    && self.random.unit() < GIFT_ON_A_GIFT_DAY
                {
                    let gift = Keepsake::choose(self.season(), &mut self.random);
                    self.life.sill = Some(gift);
                    self.life.gift_decision = Some((self.date.to_string(), false));
                    self.ghost.saw_gift = false;
                    self.record(EventKind::GiftLeft(gift));
                }
                bird.departing = Some((time, bird.picture(time).position));
            }
            if bird.perched(time) && time >= bird.next_song {
                bird.sang_at = time;
                bird.next_song = time
                    + f64::from(song_interval(
                        self.random.range(20.0, 30.0),
                        self.life.bird_trust,
                    ));
                self.record(EventKind::BirdSang);
            }
            if (daytime && bird_weather || bird.landed)
                && !bird
                    .departing
                    .is_some_and(|(start, _)| time - start >= BIRD_FLIGHT)
            {
                self.visitors.bird = Some(bird);
            }
        }

        if self.visitors.moth.is_none() && time >= self.visitors.next_moth {
            let gap = self.random.range(600.0, 1500.0);
            self.visitors.next_moth = time + f64::from(gap);
            // An open window lets moths in; a shut one only now and then,
            // through the gaps.
            let finds_a_way = self.window_open || gap < 900.0;
            if self.is_night() && self.lamp.on && finds_a_way {
                self.visitors.moth = Some(Moth::new(time, &mut self.random));
                self.record(EventKind::MothArrived);
            }
        }
        if let Some(mut moth) = self.visitors.moth.take() {
            if !self.is_night() {
                moth.leaves_at = time;
            }
            moth.advance(time, self.lamp, self.ghost.position);
            if moth
                .departing
                .is_some_and(|(start, _)| time - start >= MOTH_FLIGHT)
            {
                self.record(EventKind::MothLeft);
            } else {
                self.visitors.moth = Some(moth);
            }
        }
    }

    /// A save settles a gift being carried onto the shelf, just as an
    /// interrupted plan does, without changing the running world's journey.
    pub fn saved_life(&self) -> Life {
        let mut life = self.life.clone();
        life.wearing = self.wearing;
        if let Some(gift) = self.ghost.keepsake {
            let kept = EventKind::Kept(gift);
            let granted = life
                .wish
                .as_ref()
                .filter(|wishing| wishing.wish.granted_by(&kept))
                .map(|wishing| wishing.wish.clone());
            life.keep(gift);
            life.note(&kept);
            if let Some(wish) = granted {
                life.grant(wish.clone(), &self.date.to_string());
                life.note(&EventKind::WishGranted(wish));
            }
        }
        // Cards still waiting to be pinned go straight onto the saved board,
        // as a carried gift goes onto the shelf.
        for pin in &self.pending_pins {
            let pinned = EventKind::Pinned {
                kind: pin.kind,
                name: pin.name.clone(),
            };
            life.pin(pin.clone());
            life.note(&pinned);
        }
        life
    }

    pub(super) fn put_keepsake_away(&mut self) {
        if let Some(gift) = self.ghost.keepsake.take() {
            self.life.keep(gift);
            self.ghost.needs.change(0.0, 0.12, 0.06);
            self.record(EventKind::Kept(gift));
        }
    }

    pub(super) fn watch_window(&mut self) -> Option<Vec3> {
        let position = self.ghost.position;
        let available = self.visitors.bird.is_none()
            || self
                .visitors
                .bird
                .as_ref()
                .is_some_and(|bird| bird.perched(self.time) && self.life.bird_trust >= 0.7);
        match self.ghost.plan.count {
            0 if self.life.sill.is_some() && available && self.ghost.plan.arrived.is_some() => {
                self.ghost.plan.count = 1;
                self.ghost.plan.arrived = None;
                self.ghost.plan.target = SILL_POSITION + Vec3::new(0.0, 0.27, 0.03);
            }
            1 => {
                if !available {
                    self.ghost.plan.count = 0;
                    self.ghost.plan.arrived = None;
                    self.ghost.plan.target = WINDOW_CENTER + Vec3::new(0.0, -0.05, 0.55);
                } else if position.distance(self.ghost.plan.target) < 0.08 {
                    self.ghost.keepsake = self.life.sill.take();
                    self.ghost.saw_gift = false;
                    self.ghost.plan.count = 2;
                    self.ghost.plan.arrived = None;
                    self.ghost.plan.target = SHELF_POSITION + Vec3::new(0.0, 0.25, 0.35);
                }
            }
            2 if position.distance(self.ghost.plan.target) < ARRIVED => {
                self.put_keepsake_away();
                self.ghost.plan.done = true;
            }
            0 if self.ghost.plan.stayed(self.time) > self.ghost.plan.length => {
                self.ghost.plan.done = true;
            }
            _ => {}
        }
        Some(match self.ghost.plan.count {
            1 => SILL_POSITION,
            2 => SHELF_POSITION,
            _ => self.position_of(Thing::Bird).unwrap_or(WINDOW_CENTER),
        })
    }
}

/// Quiet watching earns about two hundredths of trust each minute, slowing gently
/// as friendship grows. The exponential keeps the rate independent of steps.
pub(super) fn quiet_trust(trust: f32, seconds: f32) -> f32 {
    let trust = trust.clamp(0.0, 1.0);
    let growth = -(-0.4 * 0.02 / 60.0 * seconds.max(0.0)).exp_m1();
    (trust + (2.5 - trust) * growth).clamp(0.0, 1.0)
}

fn startled_trust(trust: f32) -> f32 {
    (trust - 0.05).clamp(0.0, 1.0)
}

fn startle_scale(trust: f32) -> f32 {
    1.0 - 0.6 * trust.clamp(0.0, 1.0)
}

fn visit_interval(seconds: f32, trust: f32) -> f32 {
    seconds / (1.0 + trust.clamp(0.0, 1.0))
}

fn song_interval(seconds: f32, trust: f32) -> f32 {
    seconds * (1.0 - 0.4 * trust.clamp(0.0, 1.0))
}

fn sill_chance(trust: f32) -> f32 {
    if trust < 0.35 {
        0.4
    } else {
        0.6 + 0.3 * trust.clamp(0.0, 1.0)
    }
}

/// The chance a trusting bird means to leave a gift on a given day, about
/// two days in five when it trusts the ghost fully.
fn gift_chance(trust: f32) -> f32 {
    if trust < 0.4 {
        0.0
    } else {
        0.1 + 0.3 * trust.clamp(0.0, 1.0)
    }
}

/// On a day it means to, the chance it leaves the gift as it flies off.
const GIFT_ON_A_GIFT_DAY: f32 = 0.35;

impl World {
    // Whether the bird means to leave a gift today, decided once a day and
    // kept in the saved life, so restarting neither repeats a gift nor
    // changes its mind.
    fn brings_gift_today(&mut self) -> bool {
        let today = self.date.to_string();
        match &self.life.gift_decision {
            Some((day, giving)) if *day == today => *giving,
            _ => {
                let giving = self.random.unit() < gift_chance(self.life.bird_trust);
                self.life.gift_decision = Some((today, giving));
                giving
            }
        }
    }
}

// The angle of an oscillation at `speed` radians a second, taken in
// double precision so it stays smooth however long the world runs.
fn phase(time: f64, speed: f64) -> f32 {
    (time * speed).rem_euclid(std::f64::consts::TAU) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oscillations_stay_smooth_however_long_the_world_runs() {
        for speed in [0.7, 3.1, 23.0] {
            for time in [10.0, 4095.99, 4096.0, 1.0e6, 3.0e7] {
                let step = 0.01;
                let before = phase(time, speed);
                let after = phase(time + step, speed);
                let turned = (after - before).rem_euclid(TAU);
                let expected = (speed * step) as f32;
                assert!((turned - expected).abs() < 1e-3, "{speed} {time}: {turned}");
            }
        }
    }

    fn quiet_world(hour: f32) -> World {
        let mut world = World::new(17, hour);
        world.weather.until = f64::INFINITY;
        world.next_mote_at = f64::INFINITY;
        world.ghost.plan.done = true;
        world
    }

    #[test]
    fn a_seeded_day_has_several_bird_visits_with_a_landing_then_a_departure() {
        let mut world = quiet_world(7.0);
        let mut landed = false;
        let mut visits = 0;
        let mut last_song = f64::NEG_INFINITY;
        for _ in 0..414_000 {
            world.hour = 7.0 + world.time as f32 / 3600.0;
            let before = world.time;
            world.step(0.1);
            for event in world.events.iter().filter(|event| event.time > before) {
                match event.kind {
                    EventKind::BirdLanded => {
                        assert!(!landed);
                        landed = true;
                        visits += 1;
                    }
                    EventKind::BirdFlewOff { startled } => {
                        assert!(landed);
                        assert!(!startled);
                        landed = false;
                    }
                    EventKind::BirdSang => {
                        assert!(landed);
                        assert!(event.time - last_song >= 20.0);
                        last_song = event.time;
                    }
                    _ => {}
                }
            }
        }
        // A visit every quarter hour or so, more as it comes to trust.
        assert!((15..80).contains(&visits), "{visits}");
        world.hour = 22.0;
        world.step(4.0);
        assert!(world.visitors.bird.is_none());
        assert!(last_song.is_finite());
    }

    #[test]
    fn birds_never_arrive_at_night_or_in_rain_or_storms() {
        for (hour, sky, storm) in [
            (6.9, Sky::Clear, false),
            (18.5, Sky::Clear, false),
            (22.0, Sky::Clear, false),
            (12.0, Sky::Rain, false),
            (12.0, Sky::Rain, true),
        ] {
            let mut world = quiet_world(hour);
            world.weather.sky = sky;
            world.weather.storm = storm;
            for _ in 0..36_000 {
                world.time += 0.1;
                world.advance_visitors(0.1);
                assert!(world.visitors.bird.is_none());
            }
            assert!(
                !world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::BirdLanded)
            );
        }
    }

    #[test]
    fn winter_birds_still_visit_in_snow_but_less_often() {
        let visits = |sky| {
            let mut world = quiet_world(12.0);
            world.weather.sky = sky;
            let mut visits = 0;
            for _ in 0..400_000 {
                world.time += 0.1;
                world.advance_visitors(0.1);
                visits += world
                    .events
                    .iter()
                    .filter(|event| event.time == world.time && event.kind == EventKind::BirdLanded)
                    .count();
            }
            visits
        };
        let clear = visits(Sky::Clear);
        let snowy = visits(Sky::Snow);
        assert!(snowy > 5 && snowy < clear, "{snowy} {clear}");
    }

    #[test]
    fn a_rushing_or_too_close_ghost_startles_a_bird_but_calm_window_watching_does_not() {
        for (distance, velocity, startled) in [
            (0.3, Vec3::ZERO, true),
            (0.7, -Vec3::Z * 1.2, true),
            (0.7, Vec3::Z * 1.2, false),
            (0.55, Vec3::ZERO, false),
        ] {
            let mut world = quiet_world(12.0);
            let bird = Bird::new(0.0, 0.0, &mut world.random);
            world.time = 4.0;
            world.ghost.position = bird.position(world.time) + Vec3::Z * distance;
            world.ghost.velocity = velocity;
            world.visitors.bird = Some(bird);
            world.advance_visitors(0.1);
            assert_eq!(
                world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::BirdFlewOff { startled: true }),
                startled
            );
            assert_eq!(world.visitors.bird.unwrap().perched(world.time), !startled);
            if startled {
                assert_eq!(
                    world.ghost.thought.as_ref().unwrap().text,
                    "Oh… I scared it."
                );
            }
        }
        let mut world = quiet_world(12.0);
        world.visitors.bird = Some(Bird::new(0.0, 0.0, &mut world.random));
        world.time = 4.0;
        world.ghost.position = WINDOW_CENTER + Vec3::new(0.0, -0.05, 0.55);
        world.begin(Activity::Window);
        world.step(5.0);
        assert!(world.visitors.bird.unwrap().perched(world.time));
        assert_eq!(world.ghost.look, world.position_of(Thing::Bird));
    }

    #[test]
    fn moths_arrive_on_lit_nights_mostly_through_open_windows_orbit_the_bulb_and_leave() {
        let mut through_open = 0;
        for (hour, lamp_on, window_open, expected) in [
            (12.0, true, true, Some(false)),
            (22.0, false, true, Some(false)),
            (22.0, true, true, Some(true)),
            (22.0, true, false, None),
        ] {
            let mut world = quiet_world(hour);
            world.lamp.on = lamp_on;
            world.window_open = window_open;
            let mut arrived = false;
            let mut visits = 0;
            let mut departures = 0;
            for _ in 0..36_000 {
                let before = world.time;
                world.time += 0.1;
                world.advance_visitors(0.1);
                for event in world.events.iter().filter(|event| event.time > before) {
                    match event.kind {
                        EventKind::MothArrived => {
                            assert!(!arrived);
                            arrived = true;
                            visits += 1;
                        }
                        EventKind::MothLeft => {
                            assert!(arrived);
                            arrived = false;
                            departures += 1;
                        }
                        _ => {}
                    }
                }
                if let Some(moth) = world.visitors.moth {
                    assert!(moth.position.is_finite());
                    assert!(
                        moth.position.cmpge(ROOM_MIN).all() && moth.position.cmple(ROOM_MAX).all()
                    );
                    if world.time - moth.born > MOTH_FLIGHT && moth.departing.is_none() {
                        assert!(moth.position.distance(world.lamp.bulb()) < 0.45);
                    }
                }
            }
            if let Some(expected) = expected {
                assert_eq!(visits > 0, expected);
                assert_eq!(departures > 0, expected);
            }
            if window_open && expected == Some(true) {
                through_open = visits;
            } else if !window_open {
                assert!(visits <= through_open);
            }
        }
    }

    #[test]
    fn a_moth_darts_from_a_close_ghost_and_leaves_after_the_light_goes_out() {
        let mut world = quiet_world(22.0);
        let mut moth = Moth::new(0.0, &mut world.random);
        moth.advance(10.0, world.lamp, Vec3::ZERO);
        let unstartled = moth.position;
        moth.advance(10.0, world.lamp, unstartled);
        assert!(moth.position.distance(unstartled) >= 0.3);
        world.time = 10.0;
        world.visitors.moth = Some(moth);
        world.lamp.on = false;
        world.step(6.0);
        assert!(world.visitors.moth.is_some());
        world.step(7.0);
        assert!(world.visitors.moth.is_none());
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::MothLeft)
                .count(),
            1
        );
    }

    #[test]
    fn visitors_are_noticed_once_per_visit_only_after_coming_into_sight() {
        for bird_visit in [true, false] {
            let mut world = quiet_world(if bird_visit { 12.0 } else { 22.0 });
            let (thing, words) = if bird_visit {
                (Thing::Bird, "a bird at the window")
            } else {
                (Thing::Moth, "a moth at the lamp")
            };
            for visit in 1..=2 {
                if bird_visit {
                    world.visitors.bird = Some(Bird::new(world.time, 0.0, &mut world.random));
                } else {
                    world.visitors.moth = Some(Moth::new(world.time, &mut world.random));
                }
                world.time += 5.0;
                world.advance_visitors(0.1);
                if let Some(moth) = &mut world.visitors.moth {
                    moth.position = world.lamp.bulb() + Vec3::new(0.3, 0.15, 0.3);
                }
                let position = world.position_of(thing).unwrap();
                world.ghost.position = Vec3::new(0.0, 1.2, 0.6);
                let toward = (position - world.ghost.position - Vec3::Y * 0.1).normalize();
                world.ghost.heading = -toward;
                world.look_around(0.0);
                let noticed = EventKind::Noticed(words.to_owned());
                assert_eq!(
                    world
                        .events
                        .iter()
                        .filter(|event| event.kind == noticed)
                        .count(),
                    visit - 1
                );
                world.ghost.heading = toward;
                world.look_around(0.0);
                world.look_around(0.0);
                assert_eq!(
                    world
                        .events
                        .iter()
                        .filter(|event| event.kind == noticed)
                        .count(),
                    visit
                );
            }
            assert_eq!(
                if bird_visit {
                    world.life.total.birds
                } else {
                    world.life.total.moths
                },
                2
            );
        }
    }

    #[test]
    fn birds_are_not_noticed_while_they_are_still_flying_in() {
        let mut world = quiet_world(12.0);
        world.visitors.bird = Some(Bird::new(0.0, 0.0, &mut world.random));
        world.ghost.heading =
            (world.position_of(Thing::Bird).unwrap() - world.ghost.position).normalize();
        world.look_around(0.0);
        assert!(!world.visitors.bird.unwrap().noticed);
    }

    #[test]
    fn birds_fly_in_from_over_the_fence_and_perch_outside_the_house() {
        for seed in 0..20 {
            let mut world = quiet_world(12.0);
            world.visitors.bird = Some(Bird::new(0.0, 0.0, &mut Random::new(seed)));
            for _ in 0..1000 {
                world.time += 0.1;
                world.advance_visitors(0.1);
                if let Some(bird) = world.visitors.bird {
                    let position = bird.position(world.time);
                    assert!(position.is_finite());
                    assert!(position.y > 0.5);
                    assert!(!super::super::home::indoors(position));
                    if bird.perched(world.time) {
                        let perch = match bird.perch {
                            Perch::Tree => super::super::home::PERCH,
                            Perch::Sill => super::super::home::SILL,
                        };
                        assert!(position.distance(perch) < 0.6);
                    }
                }
            }
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_visits_and_motion() {
        for hour in [12.0, 22.0] {
            let mut first = quiet_world(hour);
            let mut second = quiet_world(hour);
            for _ in 0..6000 {
                first.step(0.1);
                second.step(0.1);
                assert_eq!(first.visitors.bird, second.visitors.bird);
                assert_eq!(first.visitors.moth, second.visitors.moth);
                assert_eq!(first.events, second.events);
            }
        }
    }

    #[test]
    fn quiet_watching_earns_trust_slowly_and_startling_loses_some() {
        assert!((quiet_trust(0.0, 60.0) - 0.02).abs() < 0.001);
        assert!(quiet_trust(0.8, 60.0) - 0.8 < quiet_trust(0.0, 60.0));
        assert_eq!(quiet_trust(0.9, 10_000.0), 1.0);
        assert_eq!(quiet_trust(0.0, -10.0), 0.0);
        assert_eq!(startled_trust(0.02), 0.0);
        assert!((startled_trust(0.8) - 0.75).abs() < 1e-6);
        for trust in [-1.0, 0.0, 0.5, 1.0, 2.0] {
            assert!((0.0..=1.0).contains(&quiet_trust(trust, 1000.0)));
            assert!((0.0..=1.0).contains(&startled_trust(trust)));
        }
    }

    #[test]
    fn friendship_softens_startles_and_brings_closer_more_frequent_company() {
        assert!((startle_scale(1.0) - 0.4).abs() < 1e-6);
        assert_eq!(visit_interval(240.0, 1.0), 120.0);
        assert!((song_interval(25.0, 1.0) - 15.0).abs() < 1e-5);
        assert!(sill_chance(0.35) > sill_chance(0.34));
        for trust in [0.0, 1.0] {
            let mut world = quiet_world(12.0);
            let bird = Bird::new(0.0, trust, &mut world.random);
            world.time = 4.0;
            world.life.bird_trust = trust;
            world.ghost.position = bird.position(world.time) + Vec3::Z * 0.25;
            world.visitors.bird = Some(bird);
            world.advance_visitors(0.1);
            assert_eq!(
                world.visitors.bird.unwrap().perched(world.time),
                trust > 0.5
            );
        }
    }

    #[test]
    fn trust_grows_only_during_settled_noticed_window_watching() {
        for (noticed, arrived, done, moving, grows) in [
            (true, true, false, false, true),
            (false, true, false, false, false),
            (true, false, false, false, false),
            (true, true, true, false, false),
            (true, true, false, true, false),
        ] {
            let mut world = quiet_world(12.0);
            world.time = 4.0;
            let mut bird = Bird::new(0.0, 0.0, &mut world.random);
            bird.noticed = noticed;
            world.visitors.bird = Some(bird);
            world.begin(Activity::Window);
            world.ghost.position = world.ghost.plan.target;
            world.ghost.plan.arrived = arrived.then_some(world.time);
            world.ghost.plan.done = done;
            world.ghost.velocity = if moving { Vec3::X } else { Vec3::ZERO };
            world.advance_visitors(0.1);
            assert_eq!(world.life.bird_trust > 0.0, grows);
        }
    }

    #[test]
    fn only_an_unstartled_trusting_bird_can_leave_a_seasonal_gift_on_an_empty_sill() {
        for (month, choices) in [
            (4, [Keepsake::Petal, Keepsake::Feather]),
            (7, [Keepsake::Feather, Keepsake::Pebble]),
            (10, [Keepsake::Acorn, Keepsake::Leaf]),
            (1, [Keepsake::Berry, Keepsake::Feather]),
        ] {
            let mut received = Vec::new();
            for seed in 0..60 {
                let mut world = quiet_world(12.0);
                world.random = Random::new(seed);
                world.date = jiff::civil::Date::constant(2026, month, 10);
                world.life.bird_trust = 0.8;
                let mut bird = Bird::new(0.0, 0.8, &mut world.random);
                world.time = bird.leaves_at;
                bird.landed = true;
                world.visitors.bird = Some(bird);
                world.advance_visitors(0.1);
                if let Some(gift) = world.life.sill {
                    assert!(choices.contains(&gift));
                    assert!(
                        world
                            .events
                            .iter()
                            .any(|event| event.kind == EventKind::GiftLeft(gift))
                    );
                    received.push(gift);
                }
            }
            assert!(choices.into_iter().all(|gift| received.contains(&gift)));
        }
        for (trust, waiting, startled) in [
            (0.39, None, false),
            (1.0, Some(Keepsake::Petal), false),
            (1.0, None, true),
        ] {
            for seed in 0..30 {
                let mut world = quiet_world(12.0);
                world.random = Random::new(seed);
                world.life.bird_trust = trust;
                world.life.sill = waiting;
                let bird = Bird::new(0.0, trust, &mut world.random);
                world.time = bird.leaves_at;
                if startled {
                    world.ghost.position = bird.position(world.time) + Vec3::Z * 0.05;
                }
                world.visitors.bird = Some(bird);
                world.advance_visitors(0.1);
                assert_eq!(world.life.sill, waiting);
                assert!(
                    !world
                        .events
                        .iter()
                        .any(|event| matches!(event.kind, EventKind::GiftLeft(_)))
                );
            }
        }
    }

    #[test]
    fn a_window_plan_takes_the_sill_gift_carries_it_and_places_it_on_the_shelf() {
        let mut world = quiet_world(12.0);
        world.life.sill = Some(Keepsake::Feather);
        world.begin(Activity::Window);
        let mut carried = false;
        for _ in 0..600 {
            world.step(0.1);
            if world.ghost.keepsake.is_some() {
                carried = true;
                assert_eq!(world.life.sill, None);
                assert!(world.life.keepsakes.is_empty());
            }
            if world.ghost.plan.done {
                break;
            }
        }
        assert!(carried);
        assert!(world.ghost.plan.done);
        assert_eq!(world.ghost.keepsake, None);
        assert_eq!(world.life.keepsakes, [Keepsake::Feather]);
        assert_eq!(world.life.total.gifts, 1);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Kept(Keepsake::Feather))
        );
    }

    #[test]
    fn a_sill_gift_delights_the_ghost_only_when_noticed_and_only_once() {
        let mut world = quiet_world(12.0);
        world.time = 10.0;
        world.life.sill = Some(Keepsake::Feather);
        world.record(EventKind::GiftLeft(Keepsake::Feather));
        world.ghost.heading = Vec3::Z;
        world.look_around(0.0);
        assert!(!world.ghost.saw_gift);
        assert_ne!(world.feeling(), super::super::Mood::Excited);
        world.ghost.heading = (WINDOW_CENTER - world.ghost.position).normalize();
        world.look_around(0.0);
        assert!(world.ghost.saw_gift);
        assert_eq!(world.feeling(), super::super::Mood::Excited);
        world.look_around(0.0);
        assert_eq!(world.events.iter().filter(|event| {
            matches!(&event.kind, EventKind::Noticed(thing) if thing.starts_with("a gift on the sill"))
        }).count(), 1);
    }

    #[test]
    fn a_friend_allows_a_gift_to_be_taken_while_a_wary_bird_is_left_alone() {
        for trust in [0.5, 0.8] {
            let mut world = quiet_world(12.0);
            world.time = 4.0;
            world.life.bird_trust = trust;
            world.life.sill = Some(Keepsake::Petal);
            let mut bird = Bird::new(0.0, trust, &mut world.random);
            bird.leaves_at = 100.0;
            world.visitors.bird = Some(bird);
            world.begin(Activity::Window);
            world.ghost.position = world.ghost.plan.target;
            world.step(30.0);
            assert!(world.visitors.bird.unwrap().perched(world.time));
            if trust >= 0.7 {
                assert_eq!(world.life.keepsakes, [Keepsake::Petal]);
            } else {
                assert_eq!(world.life.sill, Some(Keepsake::Petal));
            }
        }
    }

    #[test]
    fn interrupting_or_saving_a_carry_keeps_the_gift_once() {
        for interrupt in ["activity", "poke", "no", "ball", "save"] {
            let mut world = quiet_world(12.0);
            world.life.sill = Some(Keepsake::Acorn);
            world.begin(Activity::Window);
            for _ in 0..300 {
                world.step(0.1);
                if world.ghost.keepsake.is_some() {
                    break;
                }
            }
            assert_eq!(world.ghost.keepsake, Some(Keepsake::Acorn));
            match interrupt {
                "activity" => world.begin(Activity::Float),
                "poke" => world.poke(),
                "no" => {
                    world.hear("no");
                }
                "ball" => world.kick_ball(Vec3::X),
                _ => {
                    world.life.wish = Some(crate::world::Wishing {
                        wish: crate::world::Wish::BirdGift,
                        day: world.date.to_string(),
                        said: None,
                    });
                    let first = world.saved_life();
                    let second = world.saved_life();
                    assert_eq!(first, second);
                    assert_eq!(first.keepsakes, [Keepsake::Acorn]);
                    assert_eq!(first.total.gifts, 1);
                    assert_eq!(world.life.total.gifts, 0);
                    // A gift settled by saving still grants the wish it fulfils.
                    assert_eq!(first.wish, None);
                    assert_eq!(first.granted.len(), 1);
                    assert_eq!(first.total.wishes_granted, 1);
                    assert!(world.life.wish.is_some());
                    continue;
                }
            }
            world.step(0.1);
            assert_eq!(world.ghost.keepsake, None);
            assert_eq!(world.life.keepsakes, [Keepsake::Acorn]);
            assert_eq!(world.life.total.gifts, 1);
        }
    }

    #[test]
    fn the_same_seed_keeps_friendship_gifts_and_journeys_deterministic() {
        let mut first = quiet_world(12.0);
        first.life.bird_trust = 0.8;
        let mut second = first.clone();
        for _ in 0..15_000 {
            for world in [&mut first, &mut second] {
                if world.needs_decision() {
                    world.begin(Activity::Window);
                }
                world.step(0.1);
            }
            assert_eq!(first.life, second.life);
            assert_eq!(first.events, second.events);
            assert_eq!(first.ghost.position, second.ghost.position);
            assert_eq!(first.ghost.keepsake, second.ghost.keepsake);
            assert_eq!(first.visitors.bird, second.visitors.bird);
        }
    }
}
