//! Slow changes learned from finished local days, without random draws.
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

use super::{
    Life, Nature, Stage,
    life::{Entry, Tally},
};

/// Traits always follow Nature's order: playful, curious, shy, sleepy, tender.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Upbringing {
    pub drift: [f32; 5],
    /// Last finished local day applied, rather than the day now being lived.
    pub day: Option<String>,
}

/// Signed daily influences in -1..1, before age and the daily rate. Counts
/// saturate so repeating a deed cannot raise a ghost in a single day.
/// Twenty playful deeds, sixteen discoveries, or sixteen acts of care fill
/// their respective days. A quiet half hour together mildly softens play.
/// Four hours and a dozen messages ease shyness; repeated pokes and scolds
/// counter that. An absent day adds only a small shy influence when empty.
/// Four naps, two lullabies and a full night favor sleep, while hundreds of
/// activities begun favor liveliness. All sums use floats to avoid overflow
/// from damaged or very long-lived saved tallies.
pub fn influences(tally: &Tally) -> [f32; 5] {
    let together = if tally.together.is_finite() {
        tally.together.clamp(0.0, 86400.0) as f32
    } else {
        0.0
    };
    let absent = together == 0.0
        && tally.heard == 0
        && tally.scolds == 0
        && tally.pets == 0
        && tally.pokes == 0
        && tally.letters_received == 0;
    let play = [
        tally.games,
        tally.fetches,
        tally.nudges,
        tally.dances,
        tally.finds,
        tally.toppled,
        tally.word_games,
    ]
    .into_iter()
    .map(|count| count as f32)
    .sum::<f32>();
    let discoveries = [
        tally.pages,
        tally.constellations,
        tally.drawings_seen,
        tally.pictures_learned,
        tally.learned,
        tally.learned_about,
        tally.word_games,
        tally.mirror_visits,
    ]
    .into_iter()
    .map(|count| count as f32)
    .sum::<f32>();
    let care = [
        tally.pets,
        tally.comforts,
        tally.letters_read,
        tally.letters_received,
        tally.viewer_letters_read,
        tally.stories,
        tally.focus_sessions,
        tally.viewer_wishes_granted,
    ]
    .into_iter()
    .map(|count| count as f32)
    .sum::<f32>();
    let poking = ((tally.pokes as f32 - 3.0) / 12.0).clamp(0.0, 1.0);
    let activities = tally
        .activities
        .values()
        .map(|count| *count as f32)
        .sum::<f32>();
    let quiet = if together >= 1800.0 && play < 1.0 {
        0.2
    } else {
        0.0
    };
    [
        (play / 20.0).min(1.0) - quiet,
        (discoveries / 16.0).min(1.0),
        if absent {
            0.4
        } else {
            -0.65 * (together / 14400.0).min(1.0) - 0.35 * (tally.heard as f32 / 12.0).min(1.0)
                + 0.35 * poking
                + 0.35 * (tally.scolds as f32 / 6.0).min(1.0)
        },
        0.45 * (tally.naps as f32 / 4.0).min(1.0)
            + 0.3 * (tally.lullaby_sleeps as f32 / 2.0).min(1.0)
            + 0.25 * (tally.nights as f32).min(1.0)
            - 0.65 * ((activities - 150.0) / 500.0).clamp(0.0, 1.0),
        ((care / 16.0).min(1.0) - 0.25 * poking).clamp(-1.0, 1.0),
    ]
    .map(|value| value.clamp(-1.0, 1.0))
}

// Each pair shares a lifetime milestone, whichever direction is reached first.
const MILESTONES: [[&str; 2]; 5] = [
    [
        "I've grown quieter in how I play.",
        "I think I'm growing more playful, like you.",
    ],
    [
        "I've grown more settled in my little world.",
        "I'm becoming more curious about everything.",
    ],
    [
        "I'm not so shy anymore.",
        "I think I'm a little shyer than I was.",
    ],
    [
        "I'm livelier than I used to be.",
        "I've become a sleepier ghost.",
    ],
    [
        "I've grown a little more independent.",
        "I've grown gentler with you around.",
    ],
];
const CHANGES: [[&str; 2]; 5] = [
    ["growing quieter", "growing more playful"],
    ["becoming more settled", "growing more curious"],
    ["becoming less shy", "becoming more shy"],
    ["becoming livelier", "becoming sleepier"],
    ["becoming more independent", "growing gentler"],
];
const THOUGHTS: [[&str; 2]; 5] = [
    [
        "Quiet games suit me more than they used to.",
        "I think I like games more than I used to.",
    ],
    [
        "My little world feels more familiar now.",
        "I wonder about more things than I used to.",
    ],
    [
        "It feels easier to come close now.",
        "I think I'm a little shyer than I was.",
    ],
    [
        "I have more bounce than I used to.",
        "I like a little more sleep these days.",
    ],
    [
        "I'm learning to enjoy my own company.",
        "I think caring comes a little easier now.",
    ],
];

pub(super) fn valid_change(text: &str) -> bool {
    MILESTONES.iter().flatten().any(|known| *known == text)
}

impl Upbringing {
    pub(super) fn sanitize(&mut self, today: &str) {
        self.drift = self.drift.map(|value| {
            if value.is_finite() {
                value.clamp(-0.2, 0.2)
            } else {
                0.0
            }
        });
        // A marker ahead of today (after the clock went back) still holds:
        // dropping it would let a day already lived shape the ghost twice.
        let _ = today;
        self.day = self.day.take().filter(|day| day.parse::<Date>().is_ok());
    }

    fn strongest(&self) -> Option<(usize, usize)> {
        self.drift
            .iter()
            .enumerate()
            .filter(|(_, value)| value.abs() > 0.05)
            .max_by(|first, second| first.1.abs().total_cmp(&second.1.abs()))
            .map(|(index, value)| (index, usize::from(*value > 0.0)))
    }

    pub fn words(&self) -> Option<&'static str> {
        self.strongest()
            .map(|(index, direction)| CHANGES[index][direction])
    }

    pub fn thought(&self) -> Option<&'static str> {
        self.strongest()
            .map(|(index, direction)| THOUGHTS[index][direction])
    }

    fn apply(&mut self, day: Date, birthday: Option<Date>, influences: [f32; 5]) {
        let age = birthday.map(|birthday| (day - birthday).get_days().max(0) as f32);
        let youth = match Stage::from_age(age) {
            Stage::Newborn => 1.5,
            Stage::Young => 1.0,
            Stage::Grown => 0.35,
        };
        for (drift, influence) in self.drift.iter_mut().zip(influences) {
            *drift = (*drift + 0.004 * youth * influence).clamp(-0.2, 0.2);
        }
        self.day = Some(day.to_string());
    }
}

impl Life {
    /// Inborn traits stay saved intact; shared days change only their drift.
    pub fn temperament(&self) -> Nature {
        let nature = self.nature.unwrap_or_default();
        let [playful, curious, shy, sleepy, tender] = self.upbringing.drift;
        Nature {
            playful: nature.playful + playful,
            curious: nature.curious + curious,
            shy: nature.shy + shy,
            sleepy: nature.sleepy + sleepy,
            tender: nature.tender + tender,
        }
        .sanitized()
    }

    pub(super) fn raise_through(&mut self, next: &str, birthday: Option<Date>) {
        let Some((finished, next)) = self.day.parse::<Date>().ok().zip(next.parse::<Date>().ok())
        else {
            return;
        };
        if next <= finished {
            return;
        }
        let last = self
            .upbringing
            .day
            .as_deref()
            .and_then(|day| day.parse::<Date>().ok());
        if last.is_none_or(|last| finished > last) {
            self.upbringing
                .apply(finished, birthday, influences(&self.today));
            self.mark_temperament_changes(finished, true);
        }
        // Away stories invent no practice or care. Only the latest week of
        // wholly skipped days contributes absence, once per local day.
        let skipped = (next - finished).get_days().saturating_sub(1).min(7);
        for offset in (1..=skipped).rev() {
            let day = next.checked_sub(jiff::Span::new().days(offset)).unwrap();
            if self
                .upbringing
                .day
                .as_deref()
                .and_then(|day| day.parse::<Date>().ok())
                .is_none_or(|last| day > last)
            {
                self.upbringing
                    .apply(day, birthday, influences(&Tally::default()));
                self.mark_temperament_changes(day, false);
            }
        }
    }

    // Marks a trait that has grown far enough, dated the day it grew. Only
    // a day with its own tally (`lived`) keeps the change for its diary; a
    // day skipped while the app was closed has no entry of its own here.
    fn mark_temperament_changes(&mut self, day: Date, lived: bool) {
        for (index, pair) in MILESTONES.iter().enumerate() {
            let drift = self.upbringing.drift[index];
            if drift.abs() >= 0.1
                && !self
                    .milestones
                    .iter()
                    .any(|entry| pair.contains(&entry.text.as_str()))
            {
                let text = pair[usize::from(drift > 0.0)].to_owned();
                self.milestones.push(Entry {
                    day: day.to_string(),
                    text: text.clone(),
                });
                self.total.temperament_changes.push(text.clone());
                if lived {
                    self.today.temperament_changes.push(text);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Activity, EventKind, Outcome, Wish, World};

    fn close_day(tally: Tally, birthday: Option<Date>) -> Life {
        let mut life = Life {
            nature: Some(Nature::default()),
            day: "2026-06-21".to_owned(),
            today: tally,
            ..Life::default()
        };
        life.begin_day_with_birth("2026-06-22", birthday);
        life
    }

    #[test]
    fn completed_play_discoveries_and_care_each_shape_their_trait() {
        for field in [
            "games",
            "fetches",
            "nudges",
            "dances",
            "finds",
            "toppled",
            "word_games",
            "pages",
            "constellations",
            "drawings_seen",
            "pictures_learned",
            "learned",
            "learned_about",
            "mirror_visits",
            "pets",
            "comforts",
            "letters_read",
            "letters_received",
            "viewer_letters_read",
            "stories",
            "focus_sessions",
            "viewer_wishes_granted",
        ] {
            let mut value = serde_json::to_value(Tally::default()).unwrap();
            value[field] = serde_json::json!(1);
            let tally: Tally = serde_json::from_value(value).unwrap();
            let index = match field {
                "games" | "fetches" | "nudges" | "dances" | "finds" | "toppled" | "word_games" => 0,
                "pages" | "constellations" | "drawings_seen" | "pictures_learned" | "learned"
                | "learned_about" | "mirror_visits" => 1,
                _ => 4,
            };
            assert!(influences(&tally)[index] > 0.0, "{field}");
        }
        assert!(
            influences(&Tally {
                word_games: 1,
                ..Tally::default()
            })[1]
                > 0.0
        );
    }

    #[test]
    fn quiet_company_talking_absence_and_roughness_have_distinct_influences() {
        assert_eq!(influences(&Tally::default()), [0.0, 0.0, 0.4, 0.0, 0.0]);
        let company = Tally {
            together: 14400.0,
            heard: 12,
            ..Tally::default()
        };
        assert_eq!(influences(&company)[0], -0.2);
        assert_eq!(influences(&company)[2], -1.0);
        let roughness = Tally {
            pokes: 20,
            scolds: 12,
            ..Tally::default()
        };
        assert!(influences(&roughness)[2] > 0.0);
        assert!(influences(&roughness)[4] < 0.0);
        assert!(
            influences(&Tally {
                scolds: 6,
                ..Tally::default()
            })[2]
                > 0.0
        );
    }

    #[test]
    fn rest_and_long_active_days_move_sleepiness_in_opposite_directions() {
        for tally in [
            Tally {
                naps: 1,
                ..Tally::default()
            },
            Tally {
                lullaby_sleeps: 1,
                ..Tally::default()
            },
            Tally {
                nights: 1,
                ..Tally::default()
            },
        ] {
            assert!(influences(&tally)[3] > 0.0);
        }
        let tally = Tally {
            activities: [(Activity::Explore.name().to_owned(), 800)].into(),
            ..Tally::default()
        };
        assert!(influences(&tally)[3] < 0.0);
    }

    #[test]
    fn newborns_learn_more_than_young_ghosts_and_unaged_ghosts_are_grown() {
        for (birthday, expected) in [
            (Some(Date::constant(2026, 6, 21)), 0.006),
            (Some(Date::constant(2026, 6, 18)), 0.004),
            (Some(Date::constant(2026, 5, 22)), 0.0014),
            (None, 0.0014),
        ] {
            let life = close_day(
                Tally {
                    games: u32::MAX,
                    ..Tally::default()
                },
                birthday,
            );
            assert!((life.upbringing.drift[0] - expected).abs() < 0.000001);
            assert_eq!(life.nature, Some(Nature::default()));
            assert_eq!(life.today, Tally::default());
            assert_eq!(life.upbringing.day.as_deref(), Some("2026-06-21"));
        }
    }

    #[test]
    fn daily_influences_and_lifetime_drift_stay_bounded_in_both_directions() {
        let tally: Tally = serde_json::from_value(serde_json::json!({
            "games": u32::MAX, "fetches": u32::MAX, "pages": u32::MAX,
            "heard": u32::MAX, "pets": u32::MAX, "scolds": u32::MAX,
            "pokes": u32::MAX, "naps": u32::MAX, "lullaby_sleeps": u32::MAX,
            "nights": u32::MAX, "together": 1e100, "activities": {"explore": u32::MAX}
        }))
        .unwrap();
        assert!(
            influences(&tally)
                .iter()
                .all(|value| (-1.0..=1.0).contains(value))
        );
        for sign in [-1.0, 1.0] {
            let mut upbringing = Upbringing::default();
            for _ in 0..1000 {
                upbringing.apply(Date::constant(2026, 6, 21), None, [sign; 5]);
            }
            assert_eq!(upbringing.drift, [sign * 0.2; 5]);
        }
    }

    #[test]
    fn skipped_days_add_only_absence_for_at_most_a_week() {
        let mut life = close_day(
            Tally {
                games: 20,
                ..Tally::default()
            },
            None,
        );
        let before = life.upbringing.drift;
        life.begin_day("2026-07-22");
        for index in [0, 1, 3, 4] {
            assert_eq!(life.upbringing.drift[index], before[index]);
        }
        // The finished empty day and seven skipped days each count once.
        assert!((life.upbringing.drift[2] - before[2] - 8.0 * 0.0014 * 0.4).abs() < 0.000001);
        assert_eq!(life.upbringing.day.as_deref(), Some("2026-07-21"));
        let drift = life.upbringing.drift;
        life.begin_day("2026-07-22");
        life.begin_day("2026-07-20");
        life.begin_day("2026-07-22");
        assert_eq!(life.upbringing.drift, drift);
    }

    #[test]
    fn matching_inborn_natures_grow_apart_through_different_shared_days() {
        let birthday = Date::constant(2026, 6, 21);
        let mut playful = Life {
            nature: Some(Nature::default()),
            ..Life::default()
        };
        let mut quiet = playful.clone();
        for offset in 0..30 {
            let day = birthday
                .checked_add(jiff::Span::new().days(offset))
                .unwrap();
            for life in [&mut playful, &mut quiet] {
                life.begin_day_with_birth(&day.to_string(), Some(birthday));
                life.today.together = 14400.0;
                life.today.heard = 12;
            }
            playful.today.games = 20;
            quiet.today.pets = 16;
        }
        assert_eq!(playful.nature, quiet.nature);
        assert!(playful.temperament().playful > quiet.temperament().playful);
        assert!(quiet.temperament().tender > playful.temperament().tender);
        assert!(
            playful
                .upbringing
                .drift
                .iter()
                .all(|value| value.abs() < 0.14)
        );
        assert!(
            quiet
                .upbringing
                .drift
                .iter()
                .all(|value| value.abs() < 0.14)
        );
    }

    #[test]
    fn a_change_is_remembered_even_if_skipped_days_reverse_it_before_returning() {
        let mut life = Life {
            day: "2026-06-21".to_owned(),
            upbringing: Upbringing {
                drift: [0.0, 0.0, -0.099, 0.0, 0.0],
                ..Upbringing::default()
            },
            today: Tally {
                together: 14400.0,
                heard: 12,
                ..Tally::default()
            },
            ..Life::default()
        };
        life.begin_day_with_birth("2026-06-29", Some(Date::constant(2026, 6, 21)));
        assert!(life.upbringing.drift[2].abs() < 0.1);
        assert!(
            life.milestones
                .iter()
                .any(|entry| entry.text == MILESTONES[2][0])
        );
    }

    #[test]
    fn a_change_during_skipped_days_is_dated_the_day_it_came() {
        let mut life = Life {
            day: "2026-06-21".to_owned(),
            upbringing: Upbringing {
                drift: [0.0, 0.0, 0.0995, 0.0, 0.0],
                ..Upbringing::default()
            },
            // The viewer came by on the saved day, so only the days skipped
            // after it count as days alone.
            today: Tally {
                pets: 1,
                ..Tally::default()
            },
            ..Life::default()
        };
        life.begin_day_with_birth("2026-07-22", Some(Date::constant(2026, 6, 1)));
        let change = life
            .milestones
            .iter()
            .find(|entry| MILESTONES[2].contains(&entry.text.as_str()))
            .unwrap();
        assert!(change.day.as_str() > "2026-06-21", "{}", change.day);
        assert!(change.day.as_str() < "2026-07-22", "{}", change.day);
        // The skipped day had no diary of its own here, so the finished day
        // doesn't claim the change.
        assert!(life.diary.iter().all(|entry| !entry.text.contains("shy")));
    }

    #[test]
    fn saved_days_are_not_reapplied_and_old_saves_start_without_drift() {
        let old: Life = serde_json::from_str("{}").unwrap();
        assert_eq!(old.upbringing, Upbringing::default());
        assert_eq!(old.temperament(), Nature::default());
        let life = close_day(
            Tally {
                games: 20,
                ..Tally::default()
            },
            None,
        );
        let mut restored: Life =
            serde_json::from_str(&serde_json::to_string(&life).unwrap()).unwrap();
        restored = restored.sanitized();
        assert_eq!(restored, life);
        restored.day = "2026-06-21".to_owned();
        restored.today.games = 20;
        restored.begin_day("2026-06-22");
        assert_eq!(restored.upbringing, life.upbringing);
    }

    #[test]
    fn loaded_drift_and_dates_are_sanitized_and_effective_traits_are_clamped() {
        let mut life = Life {
            day: "2026-06-22".to_owned(),
            nature: Some(Nature {
                playful: 0.95,
                curious: 0.05,
                ..Nature::default()
            }),
            upbringing: Upbringing {
                drift: [0.9, -0.9, f32::NAN, f32::INFINITY, f32::NEG_INFINITY],
                day: Some("damaged".to_owned()),
            },
            ..Life::default()
        }
        .sanitized();
        assert_eq!(life.upbringing.drift, [0.2, -0.2, 0.0, 0.0, 0.0]);
        assert_eq!(life.upbringing.day, None);
        assert_eq!(life.temperament().playful, 1.0);
        assert_eq!(life.temperament().curious, 0.0);
        life.upbringing.day = Some("2026-02-30".to_owned());
        life = life.sanitized();
        assert_eq!(life.upbringing.day, None);
        // A marker at or past today, after the clock went back, holds, so
        // the day it marks is never lived through twice.
        for day in ["2026-06-22", "2027-01-01"] {
            life.upbringing.day = Some(day.to_owned());
            life = life.sanitized();
            assert_eq!(life.upbringing.day.as_deref(), Some(day));
        }
    }

    #[test]
    fn each_trait_marks_only_one_lifetime_change_even_after_reversing() {
        let mut life = Life {
            day: "2026-06-21".to_owned(),
            upbringing: Upbringing {
                drift: [0.101; 5],
                ..Upbringing::default()
            },
            ..Life::default()
        };
        life.begin_day("2026-06-22");
        assert_eq!(life.milestones.len(), 5);
        assert_eq!(life.diary.len(), 1);
        assert!(life.diary[0].text.contains("growing more playful"));
        assert!(
            life.milestones
                .iter()
                .all(|entry| !entry.text.chars().any(|letter| letter.is_ascii_digit()))
        );
        life.upbringing.drift = [-0.101; 5];
        life.begin_day("2026-06-23");
        assert_eq!(life.milestones.len(), 5);
    }

    #[test]
    fn crossing_a_threshold_delivers_a_milestone_event_without_drawing_randomness() {
        let mut world = World::new(7, 12.0);
        world.birthday = Some(world.date);
        world.begin_day();
        world.life.upbringing.drift[0] = 0.099;
        for _ in 0..20 {
            world.record(EventKind::GameOver(Outcome::Draw));
        }
        let mut random = world.random.clone();
        world.date = world.date.tomorrow().unwrap();
        world.begin_day();
        assert!(world.events.iter().any(
            |event| matches!(&event.kind, EventKind::Milestone(text) if text == MILESTONES[0][1])
        ));
        assert_eq!(world.random.unit(), random.unit());
        assert!(
            world.life.diary[0]
                .text
                .contains(MILESTONES[0][1].trim_end_matches('.'))
        );
    }

    #[test]
    fn scolds_and_only_shared_wishes_supply_the_new_tallies() {
        let mut life = Life::default();
        life.note(&EventKind::Scolded);
        assert_eq!(life.today.scolds, 1);
        assert_eq!(life.total.scolds, 1);
        for wish in [Wish::Rainbow, Wish::Company] {
            life.note(&EventKind::WishGranted(wish));
        }
        assert_eq!(life.today.wishes_granted, 2);
        assert_eq!(life.today.viewer_wishes_granted, 1);
        assert_eq!(life.total.viewer_wishes_granted, 1);
    }

    #[test]
    fn strongest_changes_are_described_in_words_after_the_notice_threshold() {
        let mut upbringing = Upbringing {
            drift: [0.05; 5],
            ..Upbringing::default()
        };
        assert_eq!(upbringing.words(), None);
        assert_eq!(upbringing.thought(), None);
        upbringing.drift[2] = -0.06;
        assert_eq!(upbringing.words(), Some("becoming less shy"));
        assert!(upbringing.thought().is_some());
        for index in 0..5 {
            for drift in [-0.12, 0.12] {
                upbringing.drift = [0.0; 5];
                upbringing.drift[index] = drift;
                assert!(
                    !upbringing
                        .words()
                        .unwrap()
                        .chars()
                        .any(|letter| letter.is_ascii_digit())
                );
            }
        }
    }
}
