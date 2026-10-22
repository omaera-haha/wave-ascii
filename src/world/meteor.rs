use glam::Vec2;
use jiff::civil::Date;

/// Seconds in each slot of the night that may hold one shooting star.
const SLOT: f64 = 10.0;
/// Mean seconds between shooting stars on an ordinary dark, clear night.
const ORDINARY: f64 = 480.0;
/// How long a shooting star's glowing tail grows, in picture coordinates.
const TAIL: f32 = 0.13;

/// A yearly meteor shower and the evenings that begin its best nights.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shower {
    pub name: &'static str,
    month: i8,
    first: i8,
    last: i8,
    /// Mean seconds between shooting stars at its best.
    every: f64,
}

const SHOWERS: [Shower; 7] = [
    Shower {
        name: "the Quadrantids",
        month: 1,
        first: 2,
        last: 3,
        every: 60.0,
    },
    Shower {
        name: "the Lyrids",
        month: 4,
        first: 21,
        last: 22,
        every: 90.0,
    },
    Shower {
        name: "the Eta Aquariids",
        month: 5,
        first: 5,
        last: 6,
        every: 90.0,
    },
    Shower {
        name: "the Perseids",
        month: 8,
        first: 11,
        last: 12,
        every: 40.0,
    },
    Shower {
        name: "the Orionids",
        month: 10,
        first: 20,
        last: 21,
        every: 75.0,
    },
    Shower {
        name: "the Leonids",
        month: 11,
        first: 16,
        last: 17,
        every: 90.0,
    },
    Shower {
        name: "the Geminids",
        month: 12,
        first: 13,
        last: 14,
        every: 40.0,
    },
];

/// The shower lighting the night that `date` and `hour` fall in; the small
/// hours belong to the evening before.
pub fn shower(date: Date, hour: f32) -> Option<Shower> {
    let evening = if hour < 12.0 {
        date.yesterday().ok()?
    } else {
        date
    };
    SHOWERS.into_iter().find(|shower| {
        evening.month() == shower.month && (shower.first..=shower.last).contains(&evening.day())
    })
}

/// Whether the sky is dark enough for shooting stars, later than twilight.
pub fn dark(hour: f32) -> bool {
    !(5.0..20.5).contains(&hour.rem_euclid(24.0))
}

/// A shooting star crossing the window's sky, in picture coordinates from
/// the window's bottom left to its top right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Meteor {
    /// The slot of time it belongs to, so it is seen only once.
    pub slot: u64,
    pub start: Vec2,
    pub travel: Vec2,
    pub began: f64,
    pub duration: f64,
}

/// What the window draws of a shooting star at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Streak {
    pub head: Vec2,
    pub tail: Vec2,
    pub brightness: f32,
}

impl Meteor {
    /// The shooting star crossing the sky at `time`, if any. Each slot of
    /// time holds one with a chance that keeps `every` seconds between them
    /// on average; the slot's hash with `seed` decides, so a seeded world
    /// sees the same ones without drawing from its random source.
    pub fn at(seed: u64, time: f64, shower: Option<Shower>) -> Option<Self> {
        if !time.is_finite() || time < 0.0 {
            return None;
        }
        let every = shower.map_or(ORDINARY, |shower| shower.every);
        let slot = (time / SLOT) as u64;
        let mut draw = Draws::new(seed, slot);
        if f64::from(draw.unit()) >= SLOT / every {
            return None;
        }
        let duration = f64::from(0.5 + 0.5 * draw.unit());
        let began = slot as f64 * SLOT + f64::from(draw.unit()) * (SLOT - duration);
        if !(began..began + duration).contains(&time) {
            return None;
        }
        let start = Vec2::new(0.15 + 0.7 * draw.unit(), 0.7 + 0.25 * draw.unit());
        let angle = 0.3 + 0.55 * draw.unit();
        let side = if draw.unit() < 0.5 { -1.0 } else { 1.0 };
        // It heads back across the window rather than out of it, and ends
        // inside the frame.
        let room = |side: f32| {
            if side > 0.0 {
                0.97 - start.x
            } else {
                start.x - 0.03
            }
        };
        let side = if room(side) < 0.2 { -side } else { side };
        let length = (0.22 + 0.2 * draw.unit()).min(room(side) / angle.cos());
        Some(Self {
            slot,
            start,
            travel: Vec2::new(side * angle.cos(), -angle.sin()) * length,
            began,
            duration,
        })
    }

    /// How far across the sky it has come, from 0 to 1.
    pub fn progress(&self, time: f64) -> f32 {
        ((time - self.began) / self.duration).clamp(0.0, 1.0) as f32
    }

    pub fn streak(&self, time: f64) -> Streak {
        let progress = self.progress(time);
        // It flares quickly and fades out at the end of its run.
        let along = 1.0 - (1.0 - progress).powi(2);
        let head = self.start + self.travel * along;
        let traveled = self.travel.length() * along;
        let tail = head - self.travel.normalize_or_zero() * traveled.min(TAIL);
        Streak {
            head,
            tail,
            brightness: (progress * 6.0).min(1.0) * (1.0 - progress).powf(0.6),
        }
    }
}

/// Values from 0 to 1 hashed from a seed and a slot.
struct Draws {
    state: u64,
}

impl Draws {
    fn new(seed: u64, slot: u64) -> Self {
        Self {
            state: seed ^ slot.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5EED_5747,
        }
    }

    fn unit(&mut self) -> f32 {
        // SplitMix64, as the world's own random source uses.
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = self.state;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        mixed ^= mixed >> 31;
        (mixed >> 40) as f32 / (1u64 << 24) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(seed: u64, hours: f64, shower: Option<Shower>) -> usize {
        let slots = (hours * 3600.0 / SLOT) as u64;
        (0..slots)
            .filter(|slot| {
                // Sample each slot finely enough to catch any star in it.
                (0..100).any(|step| {
                    Meteor::at(seed, *slot as f64 * SLOT + f64::from(step) * 0.1, shower).is_some()
                })
            })
            .count()
    }

    #[test]
    fn showers_belong_to_their_evenings_and_the_small_hours_after() {
        let date = |month, day| Date::constant(2026, month, day);
        assert_eq!(shower(date(8, 12), 23.0).unwrap().name, "the Perseids");
        assert_eq!(shower(date(8, 13), 2.0).unwrap().name, "the Perseids");
        assert_eq!(shower(date(8, 13), 22.0), None);
        assert_eq!(shower(date(8, 11), 3.0), None);
        assert_eq!(shower(date(10, 21), 22.0).unwrap().name, "the Orionids");
        assert_eq!(shower(date(1, 1), 1.0), None);
        assert_eq!(shower(date(1, 3), 4.0).unwrap().name, "the Quadrantids");
        assert!(dark(23.0) && dark(4.9) && dark(20.5));
        assert!(!dark(12.0) && !dark(5.0) && !dark(20.4));
    }

    #[test]
    fn shooting_stars_come_about_as_often_as_their_night_promises() {
        let ordinary = count(7, 8.0, None);
        let perseids = count(7, 8.0, shower(Date::constant(2026, 8, 12), 23.0));
        // Eight hours hold about sixty ordinary and seven hundred shower
        // stars on average.
        assert!((30..=100).contains(&ordinary), "{ordinary}");
        assert!((550..=850).contains(&perseids), "{perseids}");
    }

    #[test]
    fn a_shooting_star_crosses_the_sky_once_within_its_slot_and_stays_in_the_window() {
        let mut seen = 0;
        for slot in 0..20_000_u64 {
            let start = slot as f64 * SLOT;
            let Some(meteor) =
                (0..100).find_map(|step| Meteor::at(3, start + f64::from(step) * 0.1, None))
            else {
                continue;
            };
            seen += 1;
            assert_eq!(meteor.slot, slot);
            assert!(meteor.began >= start && meteor.began + meteor.duration <= start + SLOT);
            assert_eq!(
                Meteor::at(3, meteor.began + meteor.duration * 0.5, None),
                Some(meteor)
            );
            assert_eq!(
                Meteor::at(3, meteor.began + meteor.duration + 0.01, None),
                None
            );
            for progress in [0.0, 0.3, 0.7, 1.0] {
                let streak = meteor.streak(meteor.began + meteor.duration * progress);
                for point in [streak.head, streak.tail] {
                    assert!(point.cmpge(Vec2::ZERO).all() && point.cmple(Vec2::ONE).all());
                }
                assert!(streak.tail.distance(streak.head) <= TAIL + 1e-5);
                assert!((0.0..=1.0).contains(&streak.brightness));
            }
            assert!(meteor.travel.y < 0.0, "it falls");
        }
        assert!(seen > 100, "{seen}");
    }

    #[test]
    fn shooting_stars_are_the_same_for_a_seed_and_differ_between_seeds() {
        let times: Vec<f64> = (0..36_000).map(|step| f64::from(step) * 0.5).collect();
        let stars = |seed| {
            times
                .iter()
                .filter_map(|time| Meteor::at(seed, *time, None).map(|meteor| meteor.slot))
                .collect::<Vec<_>>()
        };
        assert_eq!(stars(1), stars(1));
        assert_ne!(stars(1), stars(2));
        assert_eq!(Meteor::at(1, -5.0, None), None);
        assert_eq!(Meteor::at(1, f64::NAN, None), None);
    }
}
