use jiff::civil::{Date, DateTime};

use super::{Activity, Doodle, Entry, Life, Picture, Random, Season, Sketch, Stage, World};

/// A small remembered story, rather than a replay of unattended simulation.
#[derive(Clone, Debug, PartialEq)]
pub struct Away {
    pub happenings: Vec<Happening>,
    pub days: Vec<Day>,
    pub nights: u32,
    pub rested: bool,
    pub quiet_before: Option<Date>,
    pub told: bool,
    pub returned_at: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Day {
    pub date: Date,
    pub happenings: Vec<Happening>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Happening {
    SleptNight,
    Napped,
    WatchedRain,
    WatchedStorm,
    WatchedSnow,
    SawRainbow,
    WatchedBird,
    BuiltTower { standing: bool },
    PlayedBall,
    Drew(Sketch),
    WateredPlant,
    WishedOnStar,
    FriendVisited,
    Waited,
    MissedViewer,
}

impl Happening {
    pub fn words(self) -> String {
        match self {
            Self::SleptNight => "slept through the night",
            Self::Napped => "napped on the cushion",
            Self::WatchedRain => "watched the rain",
            Self::WatchedStorm => "watched a storm from the cushion",
            Self::WatchedSnow => "watched the snow fall",
            Self::SawRainbow => "saw a rainbow after the rain",
            Self::WatchedBird => "watched the bird at the window",
            Self::BuiltTower { standing: true } => "built a tower and left it standing",
            Self::BuiltTower { standing: false } => "built a tower and knocked it down",
            Self::PlayedBall => "played with the ball",
            Self::Drew(sketch) => return format!("drew a {} in the fog", sketch.name()),
            Self::WateredPlant => "watered the plant",
            Self::WishedOnStar => "wished on a shooting star",
            Self::FriendVisited => "spent an evening with my ghost friend",
            Self::Waited => "waited at the glass for you",
            Self::MissedViewer => "missed you",
        }
        .to_owned()
    }

    fn importance(self) -> u8 {
        match self {
            Self::BuiltTower { .. } | Self::WishedOnStar | Self::FriendVisited => 5,
            Self::Drew(_) | Self::SawRainbow | Self::Waited => 4,
            Self::WatchedSnow | Self::WatchedStorm | Self::WatchedBird | Self::MissedViewer => 3,
            Self::PlayedBall | Self::WateredPlant | Self::WatchedRain => 2,
            Self::SleptNight | Self::Napped => 1,
        }
    }
}

fn sentence(happenings: &[Happening]) -> String {
    let words: Vec<String> = happenings
        .iter()
        .map(|happening| happening.words())
        .collect();
    let joined = match words.as_slice() {
        [] => "spent some quiet time in the room".to_owned(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        _ => format!(
            "{}, and {}",
            words[..words.len() - 1].join(", "),
            words.last().unwrap()
        ),
    };
    format!("While you were away: {joined}.")
}

impl Away {
    pub fn summary(&self) -> String {
        let mut happenings = self.happenings.clone();
        happenings.sort_by_key(|happening| std::cmp::Reverse(happening.importance()));
        let mut unique = Vec::new();
        for happening in happenings {
            if !unique.contains(&happening) {
                unique.push(happening);
            }
            if unique.len() == 3 {
                break;
            }
        }
        sentence(&unique)
    }

    pub fn notable(&self) -> Option<Happening> {
        self.happenings
            .iter()
            .max_by_key(|happening| happening.importance())
            .copied()
    }

    pub fn write_diary(&self, life: &mut Life, left: Date, today: Date) {
        if let Some(date) = self.quiet_before {
            life.remember_day(Entry {
                day: date.to_string(),
                text: "While you were away: many quiet days passed in the little room.".to_owned(),
            });
        }
        for day in &self.days {
            if day.date > left && day.date < today {
                life.remember_day(Entry {
                    day: day.date.to_string(),
                    text: sentence(&day.happenings),
                });
            }
        }
    }
}

fn hour(time: DateTime) -> f32 {
    f32::from(time.hour()) + f32::from(time.minute()) / 60.0 + f32::from(time.second()) / 3600.0
}

fn crossed(left: DateTime, now: DateTime, hour: i8, minute: i8) -> bool {
    let boundary = left.date().at(hour, minute, 0, 0);
    let next = if left < boundary {
        Some(boundary)
    } else {
        boundary.tomorrow().ok()
    };
    next.is_some_and(|next| next <= now)
}

fn nights(left: DateTime, now: DateTime) -> u32 {
    let first = if left <= left.date().at(22, 0, 0, 0) {
        Some(left.date())
    } else {
        left.date().tomorrow().ok()
    };
    let last = now.date().yesterday().ok().and_then(|date| {
        if now >= now.date().at(6, 30, 0, 0) {
            Some(date)
        } else {
            date.yesterday().ok()
        }
    });
    first.zip(last).map_or(0, |(first, last)| {
        ((last - first).get_days() + 1).max(0) as u32
    })
}

/// Explicit local times keep this deterministic, including in tests. Work
/// stays bounded for years away: only the latest week needs detailed stories.
pub fn story(
    left: DateTime,
    now: DateTime,
    seed: u64,
    life: &Life,
    birthday: Option<Date>,
) -> Away {
    let mut away = Away {
        happenings: Vec::new(),
        days: Vec::new(),
        nights: 0,
        rested: false,
        quiet_before: None,
        told: false,
        returned_at: 0.0,
    };
    let seconds = now.duration_since(left).as_secs_f64();
    if seconds <= 1800.0 {
        return away;
    }
    away.nights = nights(left, now);
    away.rested = crossed(left, now, 22, 0) || crossed(left, now, 6, 30);
    let first = now
        .date()
        .checked_add(jiff::Span::new().days(-7))
        .unwrap_or(left.date())
        .max(left.date());
    if (first - left.date()).get_days() > 1 {
        away.quiet_before = first.yesterday().ok();
    }
    let nature = life.nature.unwrap_or_default().sanitized();
    let usual = life.usual_time().and_then(|_| {
        life.arrivals
            .iter()
            .enumerate()
            .max_by(|first, second| first.1.total_cmp(second.1))
            .map(|(hour, _)| hour as f32)
    });
    let mut date = first;
    loop {
        let start = if date == left.date() { hour(left) } else { 0.0 };
        let end = if date == now.date() { hour(now) } else { 24.0 };
        if end > start {
            let day_seed = date.duration_since(Date::constant(2000, 1, 1)).as_secs() as u64;
            let mut random = Random::new(seed ^ day_seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let season = Season::of(date);
            let age = birthday.map(|born| (date - born).get_days());
            let stage = match age {
                Some(days) if days < 3 => Stage::Newborn,
                Some(days) if days < 30 => Stage::Young,
                _ => Stage::Grown,
            };
            let overlaps = |low: f32, high: f32| start < high && end > low;
            let daylight = overlaps(7.0, 18.5);
            let awake = overlaps(6.5, 22.0);
            let evening = overlaps(17.5, 21.75);
            let dark = overlaps(20.5, 24.0) || overlaps(0.0, 5.0);
            // One weather spell per day keeps rainbows and stars consistent.
            let weather = random
                .pick(&match season {
                    Season::Winter => [0.6, 0.0, 0.0, 0.4],
                    Season::Spring => [0.5, 0.35, 0.15, 0.0],
                    Season::Summer => [0.7, 0.15, 0.15, 0.0],
                    Season::Autumn => [0.55, 0.35, 0.1, 0.0],
                })
                .unwrap();
            let mut choices = Vec::new();
            let mut add = |happening, activity, weight| {
                choices.push((
                    happening,
                    weight * nature.leaning(activity, life.bond) * life.habit(activity),
                ));
            };
            add(
                Happening::Napped,
                Activity::Rest,
                match stage {
                    Stage::Newborn => 6.0,
                    Stage::Young => 1.5,
                    Stage::Grown => 1.0,
                },
            );
            if awake {
                add(
                    Happening::BuiltTower {
                        standing: random.unit() >= 0.2 + nature.playful * 0.65,
                    },
                    Activity::Build,
                    1.2,
                );
                add(Happening::PlayedBall, Activity::Play, 1.2);
                add(
                    Happening::Drew(
                        Sketch::ALL[(random.unit() * Sketch::ALL.len() as f32) as usize],
                    ),
                    Activity::Doodle,
                    1.0,
                );
                add(
                    Happening::WateredPlant,
                    Activity::Tend,
                    if season == Season::Spring { 1.5 } else { 1.0 },
                );
                match weather {
                    1 => add(Happening::WatchedRain, Activity::Window, 1.5),
                    2 => add(Happening::WatchedStorm, Activity::Rest, 1.5),
                    3 => add(Happening::WatchedSnow, Activity::Window, 2.0),
                    _ => {}
                }
            }
            if daylight && weather == 1 {
                add(Happening::SawRainbow, Activity::Window, 0.8);
            }
            if daylight && weather == 0 {
                add(
                    Happening::WatchedBird,
                    Activity::Window,
                    0.5 + life.bird_trust * 2.0,
                );
            }
            if dark && weather == 0 {
                add(Happening::WishedOnStar, Activity::Window, 0.6);
            }
            if evening && weather == 0 && life.friend.is_some() {
                add(Happening::FriendVisited, Activity::Friend, 0.8);
            }
            if awake && usual.is_some_and(|usual| overlaps(usual, usual + 1.0)) {
                add(Happening::Waited, Activity::Wait, 0.5 + life.bond * 2.0);
            }
            add(
                Happening::MissedViewer,
                Activity::Visit,
                0.1 + life.bond * 1.5,
            );
            let mut happenings = Vec::new();
            if date > left.date() && left <= date.yesterday().unwrap().at(22, 0, 0, 0) && end >= 6.5
            {
                happenings.push(Happening::SleptNight);
            }
            let count = (1 + (random.unit() * 3.0) as usize).max(happenings.len());
            while happenings.len() < count {
                let weights: Vec<f32> = choices.iter().map(|(_, weight)| *weight).collect();
                let Some(index) = random.pick(&weights) else {
                    break;
                };
                happenings.push(choices.remove(index).0);
            }
            away.days.push(Day { date, happenings });
        }
        if date == now.date() {
            break;
        }
        date = date.tomorrow().unwrap();
    }
    if seconds <= 86_400.0 && away.days.len() > 1 {
        // Crossing midnight does not turn a short absence into two full days.
        let mut random = Random::new(seed ^ 0x0073_686F_7274);
        let count = 1 + (random.unit() * 3.0) as usize;
        let mut choices: Vec<(usize, Happening)> = away
            .days
            .iter()
            .enumerate()
            .flat_map(|(index, day)| {
                day.happenings
                    .iter()
                    .map(move |happening| (index, *happening))
            })
            .collect();
        for day in &mut away.days {
            day.happenings.clear();
        }
        let mut chosen = Vec::new();
        if let Some(index) = choices
            .iter()
            .position(|(_, happening)| *happening == Happening::SleptNight)
        {
            chosen.push(choices.remove(index));
        }
        while chosen.len() < count && !choices.is_empty() {
            let index = random.pick(&vec![1.0; choices.len()]).unwrap();
            let choice = choices.remove(index);
            if !chosen.iter().any(|(_, happening)| *happening == choice.1) {
                chosen.push(choice);
            }
        }
        for (index, happening) in chosen {
            away.days[index].happenings.push(happening);
        }
        away.days.retain(|day| !day.happenings.is_empty());
    }
    if seconds > 86_400.0
        && let Some(day) = away.days.last_mut()
        && !day.happenings.contains(&Happening::MissedViewer)
    {
        if day.happenings.len() == 3 {
            day.happenings.pop();
        }
        day.happenings.push(Happening::MissedViewer);
    }
    away.happenings = away
        .days
        .iter()
        .flat_map(|day| day.happenings.iter().copied())
        .collect();
    away
}

impl World {
    pub fn away_summary(&self) -> Option<String> {
        self.away
            .as_ref()
            .filter(|away| !away.told && self.time - away.returned_at < 900.0)
            .map(Away::summary)
    }

    pub fn remember_away(&mut self, mut away: Away, seed: u64) {
        let mut random = Random::new(seed ^ 0x6177_6179);
        if let Some(day) = away.days.last() {
            for happening in &day.happenings {
                match *happening {
                    Happening::BuiltTower { standing } => self.away_blocks(standing, &mut random),
                    Happening::PlayedBall => self.away_ball(&mut random),
                    Happening::Drew(sketch) => {
                        self.doodles.push(Doodle {
                            picture: Picture::Sketch(sketch),
                            center: glam::Vec2::new(
                                random.range(-0.6, 0.6),
                                random.range(0.9, 1.5),
                            ),
                            size: 0.25,
                            started: self.time - Doodle::LIFETIME
                                + f64::from(random.range(12.0, 30.0)),
                        });
                    }
                    _ => {}
                }
            }
        }
        away.returned_at = self.time;
        self.away = Some(away);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(text: &str) -> DateTime {
        text.parse().unwrap()
    }

    #[test]
    fn a_story_is_deterministic_and_only_starts_after_half_an_hour() {
        let left = time("2026-06-21T12:00:00");
        let life = Life::default();
        for now in [
            left,
            time("2026-06-21T11:00:00"),
            time("2026-06-21T12:30:00"),
        ] {
            assert!(story(left, now, 7, &life, None).happenings.is_empty());
        }
        for now in [time("2026-06-21T12:30:01"), time("2026-06-24T15:00:00")] {
            let first = story(left, now, 7, &life, None);
            assert_eq!(first, story(left, now, 7, &life, None));
            assert!(!first.happenings.is_empty());
            assert!(
                first
                    .days
                    .iter()
                    .all(|day| (1..=3).contains(&day.happenings.len()))
            );
            assert!(
                !first
                    .summary()
                    .chars()
                    .any(|character| character.is_ascii_digit())
            );
        }
    }

    #[test]
    fn full_nights_are_counted_across_midnight_at_the_sleep_boundaries() {
        for (left, now, expected) in [
            ("2026-06-21T22:00:00", "2026-06-22T06:30:00", 1),
            ("2026-06-21T22:00:01", "2026-06-22T06:30:00", 0),
            ("2026-06-21T21:00:00", "2026-06-22T06:29:59", 0),
            ("2026-06-21T21:00:00", "2026-06-24T08:00:00", 3),
            ("2026-06-21T00:00:00", "2026-06-21T08:00:00", 0),
        ] {
            assert_eq!(
                story(time(left), time(now), 1, &Life::default(), None).nights,
                expected
            );
        }
        assert!(
            story(
                time("2026-06-21T21:00:00"),
                time("2026-06-21T23:00:00"),
                1,
                &Life::default(),
                None
            )
            .rested
        );
    }

    #[test]
    fn a_short_absence_across_midnight_has_at_most_three_happenings() {
        for seed in 0..100 {
            for (left, now) in [
                ("2026-06-21T23:45:00", "2026-06-22T00:16:00"),
                ("2026-06-21T21:00:00", "2026-06-22T08:00:00"),
            ] {
                let away = story(time(left), time(now), seed, &Life::default(), None);
                assert!((1..=3).contains(&away.happenings.len()));
                if away.nights > 0 {
                    assert!(away.happenings.contains(&Happening::SleptNight));
                }
            }
        }
    }

    #[test]
    fn seasonal_weather_and_company_respect_the_date_and_the_life() {
        let mut seen = Vec::new();
        for month in [1, 4, 7, 10] {
            for seed in 0..100 {
                let away = story(
                    Date::new(2026, month, 1).unwrap().at(0, 0, 0, 0),
                    Date::new(2026, month, 8).unwrap().at(23, 0, 0, 0),
                    seed,
                    &Life::default(),
                    None,
                );
                for day in away.days {
                    for happening in day.happenings {
                        assert!(!matches!(
                            happening,
                            Happening::FriendVisited | Happening::Waited
                        ));
                        if month == 1 {
                            assert!(!matches!(
                                happening,
                                Happening::WatchedStorm
                                    | Happening::WatchedRain
                                    | Happening::SawRainbow
                            ));
                        } else {
                            assert_ne!(happening, Happening::WatchedSnow);
                        }
                        seen.push(happening);
                    }
                }
            }
        }
        for expected in [
            Happening::WatchedStorm,
            Happening::WatchedSnow,
            Happening::WatchedRain,
            Happening::SawRainbow,
            Happening::WishedOnStar,
        ] {
            assert!(seen.contains(&expected), "{expected:?}");
        }
    }

    #[test]
    fn long_absences_miss_the_viewer_and_keep_only_a_week_of_detail() {
        let away = story(
            time("2000-01-01T12:00:00"),
            time("2026-06-21T12:00:00"),
            1,
            &Life::default(),
            None,
        );
        assert!(away.happenings.contains(&Happening::MissedViewer));
        assert_eq!(away.days.len(), 8);
        assert_eq!(away.quiet_before, Some(Date::constant(2026, 6, 13)));
        assert!(away.nights > 9000);
    }

    #[test]
    fn known_friends_visit_in_evenings_and_a_learned_hour_invites_waiting() {
        let mut life = Life {
            friend: Some(super::super::Companion {
                name: "Lumi".to_owned(),
                ..Default::default()
            }),
            days_seen: 3,
            routine_days: 3.0,
            ..Life::default()
        };
        life.arrivals[19] = 3.0;
        let mut friend_visits = 0;
        let mut waits = 0;
        for seed in 0..200 {
            let evening = story(
                time("2026-06-21T17:30:00"),
                time("2026-06-21T21:45:00"),
                seed,
                &life,
                None,
            );
            friend_visits += evening.happenings.contains(&Happening::FriendVisited) as usize;
            waits += evening.happenings.contains(&Happening::Waited) as usize;
            let morning = story(
                time("2026-06-21T08:00:00"),
                time("2026-06-21T12:00:00"),
                seed,
                &life,
                None,
            );
            assert!(!morning.happenings.contains(&Happening::FriendVisited));
            assert!(!morning.happenings.contains(&Happening::Waited));
        }
        assert!(friend_visits > 0 && waits > 0);
    }

    #[test]
    fn traces_are_deterministic_settled_and_do_not_advance_the_worlds_random_source() {
        let mut away = story(
            time("2026-06-21T12:00:00"),
            time("2026-06-21T16:00:00"),
            1,
            &Life::default(),
            None,
        );
        for standing in [false, true] {
            away.days.last_mut().unwrap().happenings = vec![
                Happening::BuiltTower { standing },
                Happening::PlayedBall,
                Happening::Drew(Sketch::Star),
            ];
            for seed in 0..100 {
                let mut world = World::new(7, 12.0);
                let mut control = world.clone();
                let life = world.life.clone();
                world.remember_away(away.clone(), seed);
                control.remember_away(away.clone(), seed);
                assert_eq!(world.saved_room(), control.saved_room());
                assert_eq!(world.doodles, control.doodles);
                assert_eq!(world.blocks.len(), 5);
                assert_eq!(world.tower.is_some(), standing);
                assert_eq!(world.tower_height() > 0, standing);
                assert!(
                    world
                        .blocks
                        .iter()
                        .all(|block| block.stacked || block.on_floor())
                );
                let mut restored = World::new(99, 12.0);
                restored.restore_room(&world.saved_room());
                assert_eq!(restored.ball, world.ball);
                assert_eq!(restored.tower, world.tower);
                for (restored, original) in restored.blocks.iter().zip(&world.blocks) {
                    assert_eq!(restored.position, original.position);
                    assert!(restored.orientation.angle_between(original.orientation) < 0.001);
                    assert_eq!(restored.color, original.color);
                    assert_eq!(restored.stacked, original.stacked);
                }
                let doodle = world.doodles.last().unwrap();
                assert_eq!(doodle.drawn(world.time), 1.0);
                assert!((0.0..1.0).contains(&doodle.opacity(world.time)));
                assert_eq!(doodle.opacity(world.time + 40.0), 0.0);
                assert_eq!(world.life, life);
                let mut untouched = World::new(7, 12.0);
                assert_eq!(world.random_unit(), untouched.random_unit());
            }
        }
        let mut world = World::new(7, 12.0);
        let room = world.saved_room();
        away.days.last_mut().unwrap().happenings = vec![Happening::Napped];
        world.remember_away(away, 1);
        assert_eq!(world.saved_room(), room);
    }

    #[test]
    fn newborns_nap_more_and_trusted_birds_are_watched_more() {
        let left = time("2026-06-21T08:00:00");
        let now = time("2026-06-21T18:00:00");
        let life = Life::default();
        let trusted = Life {
            bird_trust: 1.0,
            ..Life::default()
        };
        let mut naps = [0; 2];
        let mut birds = [0; 2];
        for seed in 0..500 {
            for (index, birthday) in [None, Some(left.date())].into_iter().enumerate() {
                naps[index] += story(left, now, seed, &life, birthday)
                    .happenings
                    .contains(&Happening::Napped) as usize;
            }
            for (index, life) in [&life, &trusted].into_iter().enumerate() {
                birds[index] += story(left, now, seed, life, None)
                    .happenings
                    .contains(&Happening::WatchedBird) as usize;
            }
        }
        assert!(naps[1] > naps[0] * 2);
        assert!(birds[1] > birds[0] * 2);
    }
}
