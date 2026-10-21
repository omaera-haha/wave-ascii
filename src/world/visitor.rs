use std::f32::consts::TAU;

use glam::{Vec2, Vec3};

use super::{EventKind, Lamp, ROOM_MAX, ROOM_MIN, Random, Sky, World};

const BIRD_FLIGHT: f64 = 3.0;
const MOTH_FLIGHT: f64 = 4.0;

/// The visitors are passing company, with no state kept between runs.
#[derive(Clone, Debug)]
pub struct Visitors {
    pub bird: Option<Bird>,
    pub moth: Option<Moth>,
    next_bird: f64,
    next_moth: f64,
}

impl Visitors {
    pub fn new(random: &mut Random) -> Self {
        Self {
            bird: None,
            moth: None,
            next_bird: f64::from(random.range(180.0, 360.0)),
            next_moth: f64::from(random.range(300.0, 600.0)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Perch {
    Tree,
    Sill,
}

/// A bird outside, crossing the window's picture rather than the room.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bird {
    pub perch: Perch,
    pub noticed: bool,
    landed: bool,
    arrived_at: f64,
    leaves_at: f64,
    departing: Option<(f64, Vec2)>,
    side: f32,
    phase: f32,
    next_song: f64,
    sang_at: f64,
}

/// The bird in picture coordinates, from the window's bottom left to top right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BirdPicture {
    pub position: Vec2,
    pub flying: bool,
    pub flap: f32,
    pub facing: f32,
    pub peck: f32,
    pub singing: bool,
}

impl Bird {
    pub fn new(time: f64, random: &mut Random) -> Self {
        let arrived_at = time + BIRD_FLIGHT;
        Self {
            perch: if random.unit() < 0.6 {
                Perch::Tree
            } else {
                Perch::Sill
            },
            noticed: false,
            landed: false,
            arrived_at,
            leaves_at: arrived_at + f64::from(random.range(30.0, 90.0)),
            departing: None,
            side: if random.unit() < 0.5 { -1.0 } else { 1.0 },
            phase: random.range(0.0, TAU),
            next_song: arrived_at + f64::from(random.range(7.0, 16.0)),
            sang_at: f64::NEG_INFINITY,
        }
    }

    pub fn perched(&self, time: f64) -> bool {
        time >= self.arrived_at && self.departing.is_none()
    }

    pub fn picture(&self, time: f64) -> BirdPicture {
        let hopping = phase(time, 0.8) + self.phase;
        let anchor = match self.perch {
            Perch::Tree => Vec2::new(0.375, 0.44),
            Perch::Sill => Vec2::new(0.72, 0.095),
        };
        let hop = hopping.sin().max(0.0).powi(18);
        let perched = anchor + Vec2::new(0.012 * hopping.sin(), hop * 0.025);
        let side = Vec2::new(0.5 + 0.46 * self.side, 0.78);
        let (position, flying, facing) = if let Some((start, from)) = self.departing {
            let progress = ((time - start) / BIRD_FLIGHT).clamp(0.0, 1.0) as f32;
            (from.lerp(side, progress), true, self.side)
        } else if time < self.arrived_at {
            let progress = (1.0 - (self.arrived_at - time) / BIRD_FLIGHT).clamp(0.0, 1.0) as f32;
            (side.lerp(anchor, progress), true, -self.side)
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
        let picture = self.picture(time).position;
        Vec3::new(-0.75 + 1.5 * picture.x, 1.0 + picture.y, ROOM_MIN.z - 0.02)
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
    pub(super) fn advance_visitors(&mut self) {
        let time = self.time;
        let daytime = (7.0..18.5).contains(&self.hour);
        let bird_weather = self.weather.sky != Sky::Rain && !self.weather.storm;
        if self.visitors.bird.is_none() && time >= self.visitors.next_bird {
            let snow = if self.weather.sky == Sky::Snow {
                2.0
            } else {
                1.0
            };
            self.visitors.next_bird = time + f64::from(self.random.range(180.0, 360.0) * snow);
            if daytime && bird_weather {
                self.visitors.bird = Some(Bird::new(time, &mut self.random));
            }
        }
        if let Some(mut bird) = self.visitors.bird.take() {
            let perched = bird.perched(time);
            if perched && !bird.landed {
                bird.landed = true;
                self.record(EventKind::BirdLanded);
            }
            let offset = bird.position(time) - self.ghost.position;
            let depth = self.ghost.position.z - ROOM_MIN.z;
            let in_front = Vec2::new(offset.x, offset.y).length() < 0.5;
            let rushing = depth < 0.8
                && self.ghost.velocity.length() > 0.9
                && self.ghost.velocity.dot(offset.normalize_or_zero()) > 0.5;
            let startled = perched && in_front && (depth < 0.35 || rushing);
            if bird.departing.is_none()
                && (!daytime || !bird_weather || time >= bird.leaves_at || startled)
                && perched
            {
                self.record(EventKind::BirdFlewOff { startled });
                if startled && self.ghost.asleep < 0.5 {
                    self.think("Oh… I scared it.");
                }
                bird.departing = Some((time, bird.picture(time).position));
            }
            if bird.perched(time) && time >= bird.next_song {
                bird.sang_at = time;
                bird.next_song = time + f64::from(self.random.range(20.0, 30.0));
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
            self.visitors.next_moth = time + f64::from(self.random.range(300.0, 600.0));
            if self.is_night() && self.lamp.on {
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
}

fn phase(time: f64, speed: f64) -> f32 {
    (time.rem_euclid(4096.0) * speed).rem_euclid(f64::from(TAU)) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Activity, Thing, WINDOW_CENTER};

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
        assert!(visits >= 100, "{visits}");
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
                world.advance_visitors();
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
            for _ in 0..100_000 {
                world.time += 0.1;
                world.advance_visitors();
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
        for (depth, velocity, startled) in [
            (0.3, Vec3::ZERO, true),
            (0.7, -Vec3::Z * 1.2, true),
            (0.7, Vec3::Z * 1.2, false),
            (0.55, Vec3::ZERO, false),
        ] {
            let mut world = quiet_world(12.0);
            let bird = Bird::new(0.0, &mut world.random);
            world.time = 4.0;
            world.ghost.position = bird.position(world.time) + Vec3::Z * depth;
            world.ghost.position.z = ROOM_MIN.z + depth;
            world.ghost.velocity = velocity;
            world.visitors.bird = Some(bird);
            world.advance_visitors();
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
        world.visitors.bird = Some(Bird::new(0.0, &mut world.random));
        world.time = 4.0;
        world.ghost.position = WINDOW_CENTER + Vec3::new(0.0, -0.05, 0.55);
        world.begin(Activity::Window);
        world.step(5.0);
        assert!(world.visitors.bird.unwrap().perched(world.time));
        assert_eq!(world.ghost.look, world.position_of(Thing::Bird));
    }

    #[test]
    fn moths_arrive_only_on_lit_nights_orbit_the_bulb_and_leave() {
        for (hour, lamp_on, expected) in [
            (12.0, true, false),
            (22.0, false, false),
            (22.0, true, true),
        ] {
            let mut world = quiet_world(hour);
            world.lamp.on = lamp_on;
            let mut arrived = false;
            let mut visits = 0;
            let mut departures = 0;
            for _ in 0..36_000 {
                let before = world.time;
                world.step(0.1);
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
            assert_eq!(visits > 0, expected);
            assert_eq!(departures > 0, expected);
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
                    world.visitors.bird = Some(Bird::new(world.time, &mut world.random));
                } else {
                    world.visitors.moth = Some(Moth::new(world.time, &mut world.random));
                }
                world.time += 5.0;
                world.advance_visitors();
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
        world.visitors.bird = Some(Bird::new(0.0, &mut world.random));
        world.ghost.heading =
            (world.position_of(Thing::Bird).unwrap() - world.ghost.position).normalize();
        world.look_around(0.0);
        assert!(!world.visitors.bird.unwrap().noticed);
    }

    #[test]
    fn bird_positions_stay_finite_and_inside_the_window_through_a_visit() {
        for seed in 0..20 {
            let mut world = quiet_world(12.0);
            world.visitors.bird = Some(Bird::new(0.0, &mut Random::new(seed)));
            for _ in 0..1000 {
                world.time += 0.1;
                world.advance_visitors();
                if let Some(bird) = world.visitors.bird {
                    let position = bird.position(world.time);
                    assert!(position.is_finite());
                    assert!((-0.75..=0.75).contains(&position.x));
                    assert!((1.0..=2.0).contains(&position.y));
                    assert_eq!(position.z, ROOM_MIN.z - 0.02);
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
}
