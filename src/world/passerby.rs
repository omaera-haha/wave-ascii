use std::f32::consts::TAU;

use glam::{Vec2, Vec3};
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, Occasion, Random, Season, Sky, Weather, World};

/// Two independent lanes offer each moving kind about three visits per eligible
/// hour. Slots contain the whole visit, so adjacent slots cannot overlap.
const SLOT: f64 = 120.0;
const CHANCE: f32 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Passerby {
    Geese,
    Balloon,
    Kite,
    Squirrel,
    Plane,
    Snowman,
    Lanterns,
}

impl Passerby {
    pub const ALL: [Self; 7] = [
        Self::Geese,
        Self::Balloon,
        Self::Kite,
        Self::Squirrel,
        Self::Plane,
        Self::Snowman,
        Self::Lanterns,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Geese => "a V of geese honking across the sky",
            Self::Balloon => "a hot-air balloon drifting over the hills",
            Self::Kite => "a kite tugging in the wind",
            Self::Squirrel => "a squirrel scurrying in the tree",
            Self::Plane => "a plane's blinking lights high in the sky",
            Self::Snowman => "a snowman on the snowy hill",
            Self::Lanterns => "paper lanterns floating into the night",
        }
    }

    pub fn delighted(self) -> bool {
        matches!(self, Self::Balloon | Self::Lanterns)
    }

    pub fn thoughts(self) -> [&'static str; 2] {
        match self {
            Self::Geese => [
                "Geese! Where are they all going?",
                "I can almost hear those wings.",
            ],
            Self::Balloon => [
                "A balloon, like a slow sun.",
                "Is someone up there looking down?",
            ],
            Self::Kite => [
                "That kite wants to fly away.",
                "The wind is playing with a kite.",
            ],
            Self::Squirrel => [
                "A squirrel! Such a busy tail.",
                "What did it hide in that tree?",
            ],
            Self::Plane => [
                "Those plane lights are going somewhere far away.",
                "Who is awake up there?",
            ],
            Self::Snowman => [
                "The snowman is keeping the hill company.",
                "Does a snowman feel the cold?",
            ],
            Self::Lanterns => [
                "The lanterns are carrying little wishes.",
                "Look how gently those lanterns climb.",
            ],
        }
    }

    pub fn dream(self) -> &'static str {
        match self {
            Self::Geese => "…geese make room for me among their wings…",
            Self::Balloon => "…a balloon carries my little room over the hills…",
            Self::Kite => "…a kite pulls me gently through the clouds…",
            Self::Squirrel => "…a squirrel hides tiny moons in the tree…",
            Self::Plane => "…a plane blinks goodnight to every star…",
            Self::Snowman => "…a snowman comes inside for a cool little nap…",
            Self::Lanterns => "…lanterns lift our wishes into the moonlight…",
        }
    }

    /// Stateless, including the shared limit of two things outside at once.
    pub fn at(
        self,
        seed: u64,
        time: f64,
        date: Date,
        hour: f32,
        weather: &Weather,
    ) -> Option<Passing> {
        if !time.is_finite() || time < 0.0 || !hour.is_finite() {
            return None;
        }
        if self == Self::Snowman {
            return snowman(time, weather);
        }
        at(seed, time, date, hour, weather)
            .into_iter()
            .flatten()
            .find(|passing| passing.kind == self)
    }

    /// Seasonal possibilities shared by passing schedules and wishes.
    pub fn in_season(self, date: Date) -> bool {
        let season = Season::of(date);
        match self {
            Self::Geese => matches!(season, Season::Spring | Season::Autumn),
            Self::Balloon => matches!(season, Season::Summer | Season::Autumn),
            Self::Kite => matches!(season, Season::Spring | Season::Summer),
            Self::Squirrel => season == Season::Autumn,
            Self::Plane => true,
            Self::Snowman => season == Season::Winter,
            Self::Lanterns => matches!(
                super::occasion::occasion(date, 23.0, None),
                Some(Occasion::Chuseok | Occasion::Seollal)
            ),
        }
    }

    fn eligible(self, date: Date, hour: f32, weather: &Weather, gust: f32) -> bool {
        if !self.in_season(date) {
            return false;
        }
        let day = (7.0..18.5).contains(&hour);
        let dry = matches!(weather.sky, Sky::Clear | Sky::Rainbow) && !weather.storm;
        let clear = weather.sky == Sky::Clear && !weather.storm;
        match self {
            Self::Geese => day && dry,
            Self::Balloon => clear && ((7.0..11.0).contains(&hour) || (16.0..18.5).contains(&hour)),
            Self::Kite => day && dry && gust > 0.25,
            Self::Squirrel => day && dry,
            Self::Plane => clear && super::meteor::dark(hour),
            Self::Lanterns => dry && super::meteor::dark(hour),
            Self::Snowman => false,
        }
    }
}

/// A small picture-space visitor. Snowmen instead hold for the settled snow
/// spell; their phase measures melting rather than travel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Passing {
    pub kind: Passerby,
    pub identity: u64,
    pub position: Vec2,
    pub phase: f32,
    pub motion: f32,
    pub facing: f32,
    pub striped: bool,
    pub began: f64,
    pub duration: f64,
}

impl Passing {
    pub fn window_position(self) -> Vec3 {
        super::WINDOW_CENTER + Vec3::new((self.position.x - 0.5) * 1.5, self.position.y - 0.5, 0.0)
    }
}

/// The fallback wind holds a seeded gust for each slot. It never consumes
/// the world's random stream and can be replaced by the room's wind model.
pub fn gust(seed: u64, time: f64) -> f32 {
    let slot = (time / SLOT) as u64;
    Random::new(seed ^ slot.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5749_4E44).unit()
}

fn snowman(time: f64, weather: &Weather) -> Option<Passing> {
    let settled = weather.settled_snow?;
    if time >= settled + 20.0 && weather.snow_cover > 0.05 {
        return Some(Passing {
            kind: Passerby::Snowman,
            identity: settled.to_bits(),
            position: Vec2::new(0.69, 0.185),
            phase: (1.0 - weather.snow_cover).clamp(0.0, 1.0),
            motion: 0.0,
            facing: 1.0,
            striped: false,
            began: settled + 20.0,
            duration: f64::INFINITY,
        });
    }
    None
}

pub fn at(seed: u64, time: f64, date: Date, hour: f32, weather: &Weather) -> [Option<Passing>; 2] {
    if !time.is_finite() || time < 0.0 || !hour.is_finite() {
        return [None; 2];
    }
    let hour = hour.rem_euclid(24.0);
    let slot = (time / SLOT) as u64;
    let moving = [
        Passerby::Geese,
        Passerby::Balloon,
        Passerby::Kite,
        Passerby::Squirrel,
        Passerby::Plane,
        Passerby::Lanterns,
    ];
    let mut chosen = [None; 2];
    let mut pictures = [None; 2];
    pictures[0] = Passerby::Snowman.at(seed, time, date, hour, weather);
    for lane in 0..2 {
        let mut draw = Random::new(
            seed ^ slot.wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (lane as u64 + 1).wrapping_mul(0x5041_5353_4552),
        );
        if draw.unit() >= CHANCE {
            continue;
        }
        let kind = moving[(draw.unit() * moving.len() as f32) as usize];
        if lane == 1 && chosen[0] == Some(kind) {
            continue;
        }
        chosen[lane] = Some(kind);
        let duration = f64::from(20.0 + draw.unit() * 70.0);
        let began = slot as f64 * SLOT + f64::from(draw.unit()) * (SLOT - duration);
        if !(began..began + duration).contains(&time)
            || !kind.eligible(date, hour, weather, gust(seed, time))
        {
            continue;
        }
        let phase = ((time - began) / duration) as f32;
        let facing = if draw.unit() < 0.5 { -1.0 } else { 1.0 };
        let across = if facing > 0.0 { phase } else { 1.0 - phase };
        let height = draw.unit();
        let motion = ((time - began) * 3.0).rem_euclid(f64::from(TAU)) as f32;
        let position = match kind {
            Passerby::Geese => Vec2::new(-0.12 + across * 1.24, 0.72 + height * 0.18),
            Passerby::Balloon => {
                Vec2::new(-0.07 + across * 1.14, 0.55 + height * 0.12 + phase * 0.06)
            }
            Passerby::Kite => Vec2::new(
                0.59 + 0.035 * (motion * 0.3).sin(),
                0.52 + height * 0.1 + 0.025 * motion.sin(),
            ),
            Passerby::Squirrel => {
                let along = if phase < 0.4 {
                    phase / 0.4
                } else if phase < 0.65 {
                    1.0
                } else {
                    (1.0 - phase) / 0.35
                };
                if height < 0.5 {
                    Vec2::new(0.253 + along * 0.005, 0.2 + along * 0.15)
                } else {
                    Vec2::new(0.256 - along * 0.096, 0.265 + along * 0.15)
                }
            }
            Passerby::Plane => Vec2::new(-0.04 + across * 1.08, 0.87 + height * 0.08),
            Passerby::Lanterns => {
                Vec2::new(0.54 + height * 0.2 + 0.04 * phase, 0.23 + phase * 0.88)
            }
            Passerby::Snowman => unreachable!(),
        };
        let picture = Passing {
            kind,
            identity: slot,
            position,
            phase,
            motion,
            facing,
            striped: draw.unit() < 0.35,
            began,
            duration,
        };
        if let Some(empty) = pictures.iter_mut().find(|picture| picture.is_none()) {
            *empty = Some(picture);
        }
    }
    pictures
}

impl World {
    pub fn passersby(&self) -> [Option<Passing>; 2] {
        at(
            self.meteor_seed,
            self.time,
            self.date,
            self.hour,
            &self.weather,
        )
    }

    pub(super) fn notice_passersby(&mut self) {
        let pictures = self.passersby();
        for passing in pictures.into_iter().flatten().filter(|passing| {
            passing.position.cmpge(Vec2::ZERO).all() && passing.position.cmple(Vec2::ONE).all()
        }) {
            let seen = &mut self.seen_passersby[passing.kind as usize];
            if *seen != Some(passing.identity) {
                *seen = Some(passing.identity);
                self.record(EventKind::SawPasserby(passing.kind));
            }
        }
        if self.ghost.plan.activity == Activity::Window
            && self.focus.is_none()
            && pictures.iter().any(Option::is_some)
        {
            self.attend("window", 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene(kind: Passerby) -> World {
        let mut world = World::new(7, 9.0);
        world.date = Date::constant(2026, 10, 12);
        match kind {
            Passerby::Kite => world.date = Date::constant(2026, 4, 12),
            Passerby::Plane => world.hour = 23.0,
            Passerby::Lanterns => {
                world.date = Date::constant(2026, 9, 25);
                world.hour = 23.0;
            }
            Passerby::Snowman => {
                world.date = Date::constant(2026, 1, 12);
                world.weather.snow_cover = 0.85;
                world.weather.settled_snow = Some(0.0);
            }
            _ => {}
        }
        world.weather.until = 1e9;
        world
    }

    fn find(world: &mut World, kind: Passerby, after: f64) -> Passing {
        (0..30_000)
            .find_map(|step| {
                world.time = after + f64::from(step) * 2.0;
                kind.at(
                    world.meteor_seed,
                    world.time,
                    world.date,
                    world.hour,
                    &world.weather,
                )
                .filter(|passing| {
                    (0.3..0.6).contains(&passing.phase) || passing.kind == Passerby::Snowman
                })
            })
            .expect("an eligible kind should pass within several hours")
    }

    #[test]
    fn schedules_repeat_without_advancing_the_worlds_random_stream() {
        for kind in Passerby::ALL {
            let mut world = scene(kind);
            let passing = find(&mut world, kind, 25.0);
            let mut untouched = world.random.clone();
            for _ in 0..20 {
                assert_eq!(
                    kind.at(
                        world.meteor_seed,
                        world.time,
                        world.date,
                        world.hour,
                        &world.weather
                    ),
                    Some(passing)
                );
                world.passersby();
            }
            assert_eq!(world.random.next_u64(), untouched.next_u64());
        }
    }

    #[test]
    fn moving_visits_stay_in_their_slots_and_come_a_few_times_per_hour() {
        for kind in Passerby::ALL
            .into_iter()
            .filter(|kind| *kind != Passerby::Snowman)
        {
            let world = scene(kind);
            let mut count = 0;
            for slot in 0..3000 {
                let passing = (0..12).find_map(|step| {
                    kind.at(
                        7,
                        f64::from(slot) * SLOT + f64::from(step) * 10.0,
                        world.date,
                        world.hour,
                        &world.weather,
                    )
                });
                if let Some(passing) = passing {
                    count += 1;
                    assert!((20.0..=90.0).contains(&passing.duration));
                    assert!(passing.began >= f64::from(slot) * SLOT);
                    assert!(passing.began + passing.duration <= f64::from(slot + 1) * SLOT);
                    assert!(passing.position.is_finite() && passing.phase.is_finite());
                }
            }
            assert!(
                (150..400).contains(&count),
                "{kind:?}: {count} in a hundred eligible hours"
            );
        }
    }

    #[test]
    fn seasons_hours_weather_and_seeded_gusts_filter_each_kind() {
        for kind in Passerby::ALL
            .into_iter()
            .filter(|kind| *kind != Passerby::Snowman)
        {
            let mut world = scene(kind);
            find(&mut world, kind, 0.0);
            let time = world.time;
            for sky in [Sky::Rain, Sky::Snow] {
                world.weather.sky = sky;
                assert!(
                    kind.at(7, time, world.date, world.hour, &world.weather)
                        .is_none()
                );
            }
            world.weather.sky = Sky::Clear;
            world.weather.storm = true;
            assert!(
                kind.at(7, time, world.date, world.hour, &world.weather)
                    .is_none()
            );
            world.weather.storm = false;
            let wrong_hour = if matches!(kind, Passerby::Plane | Passerby::Lanterns) {
                12.0
            } else {
                23.0
            };
            assert!(
                kind.at(7, time, world.date, wrong_hour, &world.weather)
                    .is_none()
            );
            if !matches!(kind, Passerby::Plane) {
                let wrong_date = Date::constant(2026, 12, 5);
                assert!(
                    kind.at(7, time, wrong_date, world.hour, &world.weather)
                        .is_none()
                );
            }
        }
        let world = scene(Passerby::Kite);
        assert!(!Passerby::Kite.eligible(world.date, 12.0, &world.weather, 0.1));
        assert!(Passerby::Kite.eligible(world.date, 12.0, &world.weather, 0.9));
        for date in [Date::constant(2026, 2, 17), Date::constant(2026, 9, 25)] {
            assert!(Passerby::Lanterns.eligible(date, 23.0, &world.weather, 0.9));
        }
        for hour in [6.9, 11.0, 15.9, 18.5] {
            assert!(!Passerby::Balloon.eligible(
                Date::constant(2026, 8, 1),
                hour,
                &world.weather,
                0.9
            ));
        }
    }

    #[test]
    fn the_window_never_holds_more_than_two_or_two_of_the_same_kind() {
        for kind in Passerby::ALL {
            let world = scene(kind);
            for step in 0..12_000 {
                let pictures = at(
                    7,
                    f64::from(step) * 2.0,
                    world.date,
                    world.hour,
                    &world.weather,
                );
                let kinds: Vec<Passerby> = pictures
                    .into_iter()
                    .flatten()
                    .map(|passing| passing.kind)
                    .collect();
                assert!(kinds.len() <= 2);
                assert!(kinds.len() < 2 || kinds[0] != kinds[1]);
            }
        }
    }

    #[test]
    fn a_snowman_waits_for_settled_cover_and_slumps_until_the_hill_thaws() {
        let mut world = scene(Passerby::Snowman);
        world.time = 10.0;
        assert!(
            world
                .passersby()
                .into_iter()
                .flatten()
                .all(|passing| passing.kind != Passerby::Snowman)
        );
        world.time = 25.0;
        let first = Passerby::Snowman
            .at(7, world.time, world.date, world.hour, &world.weather)
            .unwrap();
        world.weather.sky = Sky::Snow;
        assert_eq!(
            Passerby::Snowman
                .at(7, world.time, world.date, world.hour, &world.weather)
                .unwrap()
                .identity,
            first.identity
        );
        world.weather.sky = Sky::Clear;
        world.time = 2500.0;
        world.weather.snow_cover = 0.15;
        let melting = Passerby::Snowman
            .at(7, world.time, world.date, world.hour, &world.weather)
            .unwrap();
        assert_eq!(first.identity, melting.identity);
        assert!(melting.phase > first.phase);
        world.weather.snow_cover = 0.0;
        assert!(
            Passerby::Snowman
                .at(7, world.time, world.date, world.hour, &world.weather)
                .is_none()
        );
        world.time = 25.0;
        world.weather.snow_cover = 0.3;
        world.weather.settled_snow = None;
        assert!(
            Passerby::Snowman
                .at(7, world.time, world.date, world.hour, &world.weather)
                .is_none()
        );
    }

    #[test]
    fn settled_deep_snow_keeps_its_snowman_through_day_and_night_thaws() {
        let mut world = scene(Passerby::Snowman);
        world.weather.settled_snow = None;
        world.weather.sky = Sky::Snow;
        world.weather.until = 1.0;
        let mut weather_random = Random::new(3);
        assert_eq!(
            world
                .weather
                .step(1.0, true, Season::Winter, &mut weather_random),
            Some(super::super::weather::Change::SnowStopped)
        );
        let identity = world.weather.settled_snow;
        assert_eq!(identity, Some(1.0));
        for (time, daytime) in [(201.0, true), (401.0, true), (601.0, false), (801.0, false)] {
            world.weather.advance(time, 200.0, daytime);
            assert_eq!(world.weather.settled_snow, identity);
            assert!(
                Passerby::Snowman
                    .at(7, time, world.date, world.hour, &world.weather)
                    .is_some()
            );
        }
        world.weather.advance(2001.0, 1200.0, true);
        assert_eq!(world.weather.settled_snow, None);
        assert!(
            Passerby::Snowman
                .at(7, 2001.0, world.date, world.hour, &world.weather)
                .is_none()
        );
    }

    #[test]
    fn looking_through_the_window_counts_each_passing_once_and_marks_each_first() {
        for kind in Passerby::ALL {
            let mut world = scene(kind);
            let first = find(&mut world, kind, 25.0);
            world.ghost.position = Vec3::new(0.0, 1.3, 0.4);
            world.ghost.heading = Vec3::Z;
            world.look_around(0.0);
            assert_eq!(world.life.total.passersby, 0);
            world.ghost.heading = (super::super::WINDOW_CENTER - world.ghost.position).normalize();
            world.ghost.asleep = 1.0;
            world.look_around(0.0);
            assert_eq!(world.life.total.passersby, 0);
            world.ghost.asleep = 0.0;
            world.ghost.plan.activity = Activity::Window;
            let mut untouched = world.random.clone();
            for _ in 0..4 {
                world.look_around(0.0);
            }
            assert_eq!(world.random.next_u64(), untouched.next_u64());
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| event.kind == EventKind::SawPasserby(kind))
                    .count(),
                1
            );
            assert!(world.life.today.passerby_kinds.contains(&kind));
            assert_eq!(world.life.today.passersby, world.life.total.passersby);
            assert!(world.life.today.words().contains(kind.name()));
            assert!(
                world
                    .ghost
                    .attention
                    .is_some_and(|(thing, _)| thing == super::super::Thing::Window)
            );
            let milestone = format!("I saw {} for the first time.", kind.name());
            assert_eq!(
                world
                    .life
                    .milestones
                    .iter()
                    .filter(|entry| entry.text == milestone)
                    .count(),
                1
            );
            if kind != Passerby::Snowman {
                find(&mut world, kind, first.began + SLOT);
                world.look_around(0.0);
                assert_eq!(
                    world
                        .events
                        .iter()
                        .filter(|event| event.kind == EventKind::SawPasserby(kind))
                        .count(),
                    2
                );
                assert_eq!(
                    world
                        .life
                        .milestones
                        .iter()
                        .filter(|entry| entry.text == milestone)
                        .count(),
                    1
                );
            }
            assert!(
                !EventKind::SawPasserby(kind)
                    .words()
                    .chars()
                    .any(|character| character.is_ascii_digit())
            );
        }
    }

    #[test]
    fn window_gazing_follows_motion_while_hidden_views_do_not_notice_it() {
        let mut world = scene(Passerby::Balloon);
        let passing = find(&mut world, Passerby::Balloon, 0.0);
        world.ghost.position = Vec3::new(0.0, 1.3, 0.4);
        world.ghost.heading = (super::super::WINDOW_CENTER - world.ghost.position).normalize();
        let middle = (world.ghost.position + super::super::WINDOW_CENTER) * 0.5;
        world.plant.position = middle - Vec3::Y * world.plant.height() * 0.5;
        world.look_around(0.0);
        assert_eq!(world.life.total.passersby, 0);
        world.plant.position = Vec3::new(1.5, 0.0, -0.9);
        world.ghost.plan.activity = Activity::Window;
        for _ in 0..20 {
            world.look_around(0.1);
        }
        let direction = (passing.window_position() - world.ghost.position).normalize();
        assert!(world.ghost.heading.is_finite());
        assert!(world.ghost.heading.angle_between(direction) < 0.01);
        assert!(world.life.total.passerby_kinds.contains(&Passerby::Balloon));
    }

    #[test]
    fn passersby_leave_focus_and_story_plans_with_their_existing_attention() {
        for focusing in [false, true] {
            let mut world = scene(Passerby::Balloon);
            find(&mut world, Passerby::Balloon, 0.0);
            if focusing {
                world.start_focus(25);
            } else {
                world.tell(vec![
                    "Once upon a time.".to_owned(),
                    "The moon waved.".to_owned(),
                ]);
            }
            let activity = world.ghost.plan.activity;
            let attention = world.ghost.attention;
            let telling = world.telling.clone();
            world.ghost.position = Vec3::new(0.0, 1.3, 0.4);
            world.ghost.heading = (super::super::WINDOW_CENTER - world.ghost.position).normalize();
            world.look_around(0.0);
            assert_eq!(world.ghost.plan.activity, activity);
            assert_eq!(world.ghost.attention, attention);
            assert_eq!(world.telling, telling);
            assert!(world.life.today.passerby_kinds.contains(&Passerby::Balloon));
        }
    }

    #[test]
    fn old_saves_default_the_tallies_and_sanitizing_bounds_the_new_fields() {
        let mut life = super::super::Life::default();
        life.note(&EventKind::SawPasserby(Passerby::Geese));
        let mut saved = serde_json::to_value(&life).unwrap();
        for tally in ["today", "total"] {
            saved[tally].as_object_mut().unwrap().remove("passersby");
            saved[tally]
                .as_object_mut()
                .unwrap()
                .remove("passerby_kinds");
        }
        let restored: super::super::Life = serde_json::from_value(saved).unwrap();
        assert_eq!(restored.total.passersby, 0);
        assert!(restored.today.passerby_kinds.is_empty());
        life.today.passersby = u32::MAX;
        life.today.passerby_kinds = vec![Passerby::Geese; 100];
        let cleaned = life.sanitized();
        assert_eq!(cleaned.today.passersby, 100_000_000);
        assert_eq!(cleaned.today.passerby_kinds, vec![Passerby::Geese]);
    }

    #[test]
    fn invalid_clocks_have_no_passersby() {
        let world = scene(Passerby::Geese);
        for time in [-1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(at(7, time, world.date, 9.0, &world.weather), [None; 2]);
        }
        assert_eq!(
            at(7, 100.0, world.date, f32::NAN, &world.weather),
            [None; 2]
        );
    }
}
