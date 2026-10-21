use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, World, life::Tally};

/// The first month of a ghost's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Newborn,
    Young,
    Grown,
}

impl Stage {
    pub fn words(self) -> &'static str {
        match self {
            Self::Newborn => "a newborn wisp",
            Self::Young => "a young ghost",
            Self::Grown => "a grown ghost",
        }
    }
}

impl World {
    /// Age from the local birth date, including today's fraction. Seeded
    /// pictures have no age unless a birth date was supplied.
    pub fn age_days(&self) -> Option<f32> {
        self.birthday
            .map(|birthday| ((self.date - birthday).get_days() as f32 + self.hour / 24.0).max(0.0))
    }

    pub fn stage(&self) -> Stage {
        match self.age_days() {
            Some(days) if days < 3.0 => Stage::Newborn,
            Some(days) if days < 30.0 => Stage::Young,
            _ => Stage::Grown,
        }
    }

    /// Body scale, eased from a wisp to full size over the first month.
    pub fn size(&self) -> f32 {
        let progress = self.age_days().map_or(1.0, |days| (days / 30.0).min(1.0));
        0.68 + 0.32 * progress * progress * (3.0 - 2.0 * progress)
    }

    pub(super) fn mark_growth(&mut self) {
        let Some(days) = self.age_days() else {
            return;
        };
        for (age, text) in [
            (3.0, "I'm not a newborn wisp any more."),
            (30.0, "I'm all grown up now."),
        ] {
            if days >= age
                && let Some(milestone) = self.life.reach(text.to_owned())
            {
                self.record(EventKind::Milestone(milestone));
            }
        }
    }
}

/// Things that improve through completed practice, rather than choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skill {
    Stacking,
    Catching,
    Fetching,
    Games,
    Writing,
    Dancing,
}

impl Skill {
    pub const ALL: [Self; 6] = [
        Self::Stacking,
        Self::Catching,
        Self::Fetching,
        Self::Games,
        Self::Writing,
        Self::Dancing,
    ];

    pub fn for_activity(activity: Activity) -> Option<Self> {
        match activity {
            Activity::Build => Some(Self::Stacking),
            Activity::Chase => Some(Self::Catching),
            Activity::Fetch => Some(Self::Fetching),
            Activity::Game => Some(Self::Games),
            Activity::Doodle => Some(Self::Writing),
            Activity::Dance => Some(Self::Dancing),
            _ => None,
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Self::Stacking => "stacking blocks",
            Self::Catching => "catching motes",
            Self::Fetching => "fetching the ball",
            Self::Games => "noughts and crosses",
            Self::Writing => "writing",
            Self::Dancing => "dancing",
        }
    }

    pub(super) fn milestone(self) -> &'static str {
        match self {
            Self::Stacking => "I can stack a tall tower without knocking it over now.",
            Self::Catching => "I'm good at catching motes now.",
            Self::Fetching => "I'm good at bringing the ball back now.",
            Self::Games => "I'm good at noughts and crosses now.",
            Self::Writing => "I'm good at writing on the glass now.",
            Self::Dancing => "I'm good at dancing now.",
        }
    }
}

/// Practice kept between runs. The missing-field default marks old saves
/// for migration without mistaking an explicitly saved beginner for one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Skills {
    pub stacking: f32,
    pub catching: f32,
    pub fetching: f32,
    pub games: f32,
    pub writing: f32,
    pub dancing: f32,
    #[serde(skip)]
    from_old_save: bool,
}

impl Skills {
    pub(super) fn from_old_save() -> Self {
        Self {
            from_old_save: true,
            ..Self::default()
        }
    }

    pub fn practice(&self, skill: Skill) -> f32 {
        match skill {
            Skill::Stacking => self.stacking,
            Skill::Catching => self.catching,
            Skill::Fetching => self.fetching,
            Skill::Games => self.games,
            Skill::Writing => self.writing,
            Skill::Dancing => self.dancing,
        }
    }

    pub fn level(&self, skill: Skill) -> f32 {
        let scale = match skill {
            Skill::Stacking => 30.0,
            Skill::Catching => 40.0,
            Skill::Fetching => 15.0,
            Skill::Games | Skill::Dancing => 12.0,
            Skill::Writing => 10.0,
        };
        1.0 - (-self.practice(skill) / scale).exp()
    }

    pub fn words(&self, skill: Skill) -> &'static str {
        match self.level(skill) {
            level if level < 0.15 => "clumsy",
            level if level < 0.4 => "learning",
            level if level < 0.65 => "getting the hang of it",
            level if level < 0.85 => "good",
            _ => "masterful",
        }
    }

    pub fn description(&self, skill: Skill) -> String {
        match self.words(skill) {
            "learning" => format!("learning {}", skill.words()),
            "getting the hang of it" => format!("getting the hang of {}", skill.words()),
            words => format!("{words} at {}", skill.words()),
        }
    }

    pub(super) fn note(&mut self, kind: &EventKind) -> Option<Skill> {
        let (skill, amount) = match kind {
            EventKind::Built { height } => (Skill::Stacking, *height as f32),
            EventKind::AteMote => (Skill::Catching, 1.0),
            EventKind::Fetched => (Skill::Fetching, 1.0),
            EventKind::GameOver(outcome) if outcome.finished() => (Skill::Games, 1.0),
            EventKind::Wrote { .. } => (Skill::Writing, 1.0),
            EventKind::Danced => (Skill::Dancing, 1.0),
            _ => return None,
        };
        let practice = match skill {
            Skill::Stacking => &mut self.stacking,
            Skill::Catching => &mut self.catching,
            Skill::Fetching => &mut self.fetching,
            Skill::Games => &mut self.games,
            Skill::Writing => &mut self.writing,
            Skill::Dancing => &mut self.dancing,
        };
        *practice = (*practice + amount).min(1_000_000.0);
        Some(skill)
    }

    pub(super) fn migrate(&mut self, total: &Tally) {
        if self.from_old_save {
            *self = std::mem::take(self).sanitized(total);
        }
    }

    pub(super) fn sanitized(mut self, total: &Tally) -> Self {
        if self.from_old_save {
            self = Self {
                // Old tallies kept towers, not individual placements.
                stacking: (total.towers as f32 * 3.0).max(total.tallest as f32),
                catching: total.motes as f32,
                fetching: total.fetches as f32,
                games: total.games as f32,
                writing: total.writings as f32,
                dancing: total.dances as f32,
                ..Self::default()
            };
        }
        for practice in [
            &mut self.stacking,
            &mut self.catching,
            &mut self.fetching,
            &mut self.games,
            &mut self.writing,
            &mut self.dancing,
        ] {
            *practice = if practice.is_finite() {
                practice.clamp(0.0, 1_000_000.0)
            } else {
                0.0
            };
        }
        self
    }
}

pub(super) fn chase_speed(level: f32) -> f32 {
    0.8 + 0.35 * level
}

pub(super) fn miss_chance(level: f32) -> f32 {
    0.25 * (1.0 - level).powi(2)
}

pub(super) fn carrying_speed(level: f32) -> f32 {
    0.85 + 0.25 * level
}

pub(super) fn dance_length(length: f32, level: f32) -> f32 {
    length * (1.0 + 0.3 * level)
}

pub(super) fn dance_offset(progress: f32, level: f32) -> glam::Vec3 {
    use glam::Vec3;
    use std::f32::consts::{PI, TAU};

    if progress < 0.8 {
        let phase = progress / 0.8 * TAU * 2.0;
        let simple = Vec3::new(phase.sin(), -phase.cos(), 0.0) * 0.3;
        let practiced = Vec3::new(phase.sin() * 0.5, (phase * 2.0).sin() * 0.25 - 0.3, 0.0);
        simple.lerp(practiced, level)
    } else {
        let bounce = ((progress - 0.8) / 0.2).min(1.0);
        Vec3::Y * (-0.3 * (1.0 - bounce) + 0.15 * (bounce * PI).sin())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Outcome};
    use jiff::civil::Date;

    #[test]
    fn age_includes_the_local_day_fraction_and_stage_boundaries() {
        let mut world = World::new(1, 0.0);
        assert_eq!(world.age_days(), None);
        assert_eq!(world.stage(), Stage::Grown);
        assert_eq!(world.size(), 1.0);
        world.birthday = Some(Date::constant(2026, 6, 21));
        for (date, hour, age, stage) in [
            (Date::constant(2026, 6, 21), 0.0, 0.0, Stage::Newborn),
            (Date::constant(2026, 6, 23), 12.0, 2.5, Stage::Newborn),
            (Date::constant(2026, 6, 24), 0.0, 3.0, Stage::Young),
            (Date::constant(2026, 7, 20), 18.0, 29.75, Stage::Young),
            (Date::constant(2026, 7, 21), 0.0, 30.0, Stage::Grown),
            (Date::constant(2026, 6, 20), 0.0, 0.0, Stage::Newborn),
        ] {
            world.date = date;
            world.hour = hour;
            assert_eq!(world.age_days(), Some(age));
            assert_eq!(world.stage(), stage);
        }
        world.birthday = Some(Date::constant(2024, 2, 28));
        world.date = Date::constant(2024, 3, 1);
        world.hour = 6.0;
        assert_eq!(world.age_days(), Some(2.25));
    }

    #[test]
    fn size_grows_smoothly_and_monotonically_until_the_first_month() {
        let mut world = World::new(1, 0.0);
        world.birthday = Some(world.date);
        assert_eq!(world.size(), 0.68);
        let mut previous = world.size();
        for day in 0..=60 {
            world.date = Date::constant(2026, 6, 21)
                .checked_add(jiff::Span::new().days(day))
                .unwrap();
            let size = world.size();
            assert!((0.68..=1.0).contains(&size));
            assert!(size >= previous);
            if day == 15 {
                assert!((size - 0.84).abs() < 1e-6);
            }
            if day >= 30 {
                assert_eq!(size, 1.0);
            }
            previous = size;
        }
    }

    #[test]
    fn completed_events_practice_each_skill_and_abandoned_games_do_not() {
        let mut life = Life::default();
        for kind in [
            EventKind::Built { height: 4 },
            EventKind::AteMote,
            EventKind::Fetched,
            EventKind::GameOver(Outcome::Draw),
            EventKind::Wrote {
                text: "HI".to_owned(),
                mirrored: false,
                watched: true,
            },
            EventKind::Danced,
        ] {
            life.note(&kind);
        }
        assert_eq!(life.skills.stacking, 4.0);
        for skill in Skill::ALL.into_iter().skip(1) {
            assert_eq!(life.skills.practice(skill), 1.0);
        }
        let practiced = life.skills.clone();
        for kind in [
            EventKind::GameOver(Outcome::Abandoned),
            EventKind::Began(Activity::Dance),
            EventKind::Toppled { by_viewer: false },
        ] {
            life.note(&kind);
        }
        assert_eq!(life.skills, practiced);
    }

    #[test]
    fn older_tallies_supply_practice_once_and_saved_counts_survive_restoring() {
        let old = r#"{"total":{"towers":20,"tallest":5,"motes":28,"fetches":10,"games":8,"writings":7,"dances":8}}"#;
        let life: Life = serde_json::from_str(old).unwrap();
        let mut life = life.sanitized();
        for (skill, practice) in Skill::ALL
            .into_iter()
            .zip([60.0, 28.0, 10.0, 8.0, 7.0, 8.0])
        {
            assert_eq!(life.skills.practice(skill), practice);
            assert!(life.skills.level(skill) > 0.45);
        }
        life.note(&EventKind::Fetched);
        assert_eq!(life.skills.fetching, 11.0);
        let saved = serde_json::to_string(&life).unwrap();
        let loaded: Life = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded.sanitized(), life);
        let explicit: Life = serde_json::from_str(r#"{"total":{"motes":28},"skills":{}}"#).unwrap();
        assert_eq!(explicit.sanitized().skills.catching, 0.0);
    }

    #[test]
    fn levels_and_words_follow_practice_with_diminishing_returns() {
        let mut skills = Skills::default();
        assert_eq!(skills.level(Skill::Writing), 0.0);
        for (level, words) in [
            (0.0, "clumsy"),
            (0.149, "clumsy"),
            (0.151, "learning"),
            (0.399, "learning"),
            (0.401, "getting the hang of it"),
            (0.649, "getting the hang of it"),
            (0.651, "good"),
            (0.849, "good"),
            (0.851, "masterful"),
        ] {
            skills.writing = -10.0 * (1.0_f32 - level).ln();
            assert!((skills.level(Skill::Writing) - level).abs() < 1e-6);
            assert_eq!(skills.words(Skill::Writing), words);
        }
        skills.writing = 10.0;
        assert!((skills.level(Skill::Writing) - (1.0 - (-1.0_f32).exp())).abs() < 1e-6);
        skills.writing = 1_000_000.0;
        assert_eq!(skills.level(Skill::Writing), 1.0);
        assert!(Skill::ALL.into_iter().all(|skill| {
            !skills
                .description(skill)
                .chars()
                .any(|letter| letter.is_ascii_digit())
        }));
    }

    #[test]
    fn damaged_practice_is_finite_non_negative_and_bounded_after_loading() {
        let skills = Skills {
            stacking: f32::NAN,
            catching: f32::INFINITY,
            fetching: f32::NEG_INFINITY,
            games: -3.0,
            writing: f32::MAX,
            dancing: 12.0,
            ..Skills::default()
        };
        let skills = skills.sanitized(&Tally::default());
        for skill in Skill::ALL {
            assert!(skills.practice(skill).is_finite());
            assert!((0.0..=1.0).contains(&skills.level(skill)));
        }
        assert_eq!(skills.writing, 1_000_000.0);
        assert_eq!(skills.dancing, 12.0);
        assert_eq!(skills.games, 0.0);
    }

    #[test]
    fn stage_milestones_are_recorded_once_on_new_days_including_time_away() {
        let mut world = World::new(1, 12.0);
        world.birthday = Some(world.date);
        world.step(0.0);
        assert!(world.life.milestones.is_empty());
        for date in [
            Date::constant(2026, 6, 24),
            Date::constant(2026, 7, 25),
            Date::constant(2026, 7, 26),
        ] {
            world.date = date;
            world.step(0.0);
            world.step(0.0);
        }
        for text in ["I'm not a newborn wisp any more.", "I'm all grown up now."] {
            assert_eq!(
                world
                    .life
                    .milestones
                    .iter()
                    .filter(|entry| entry.text == text)
                    .count(),
                1
            );
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| event.kind == EventKind::Milestone(text.to_owned()))
                    .count(),
                1
            );
        }
        let mut restored = World::new(1, 12.0);
        restored.birthday = Some(Date::constant(2026, 5, 1));
        restored.life.begin_day(&restored.date.to_string());
        restored.step(0.0);
        assert_eq!(restored.life.milestones.len(), 2);
        restored.step(0.0);
        assert_eq!(restored.life.milestones.len(), 2);
    }

    #[test]
    fn every_skill_keeps_its_good_milestone_once_alongside_other_firsts() {
        let mut world = World::new(1, 12.0);
        world.life.skills.stacking = 31.0;
        world.record(EventKind::Built { height: 5 });
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "I built my first tower of blocks.")
        );
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "I built a tower five blocks high.")
        );
        for _ in 0..50 {
            for kind in [
                EventKind::Built { height: 5 },
                EventKind::AteMote,
                EventKind::Fetched,
                EventKind::GameOver(Outcome::Draw),
                EventKind::Wrote {
                    text: "HI".to_owned(),
                    mirrored: false,
                    watched: true,
                },
                EventKind::Danced,
            ] {
                world.record(kind);
            }
        }
        for skill in Skill::ALL {
            assert_eq!(
                world
                    .life
                    .milestones
                    .iter()
                    .filter(|entry| entry.text == skill.milestone())
                    .count(),
                1
            );
        }
    }

    #[test]
    fn practice_changes_speeds_misses_and_dance_length_within_their_limits() {
        assert_eq!(chase_speed(0.0), 0.8);
        assert_eq!(chase_speed(1.0), 1.15);
        assert_eq!(carrying_speed(0.0), 0.85);
        assert_eq!(carrying_speed(1.0), 1.1);
        assert_eq!(miss_chance(0.0), 0.25);
        assert_eq!(miss_chance(1.0), 0.0);
        assert_eq!(dance_length(6.0, 0.0), 6.0);
        assert!((dance_length(6.0, 1.0) - 7.8).abs() < 1e-6);
        for index in 0..=100 {
            let level = index as f32 / 100.0;
            assert!((0.8..=1.15).contains(&chase_speed(level)));
            assert!((0.85..=1.1).contains(&carrying_speed(level)));
            assert!((0.0..=0.25).contains(&miss_chance(level)));
            assert!((6.0..=7.8).contains(&dance_length(6.0, level)));
        }
    }
}
