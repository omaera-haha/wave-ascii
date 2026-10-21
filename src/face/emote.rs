use std::f32::consts::TAU;

use glam::{Vec2, Vec3};

use super::mood::random;
use crate::world::{Activity, EventKind, Mood, Outcome, Thing, World};

/// Seconds a symbol takes to rise and drift away.
const LIFETIME: f64 = 2.2;
const SLEEP_BEAT: f64 = 1.6;

/// A wordless feeling that rises over the ghost's head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Emote {
    Sleep,
    Note,
    Heart,
    Longing,
    Alarm,
    Question,
    Anger,
    Sigh,
    Sparkle,
}

impl Emote {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Sleep => "z",
            Self::Note => "♪",
            Self::Heart => "♥",
            Self::Longing => "♡",
            Self::Alarm => "!",
            Self::Question => "?",
            Self::Anger => "#",
            Self::Sigh => "…",
            Self::Sparkle => "✦",
        }
    }

    /// The symbol's color, with humming following the ghost's glow.
    pub fn color(self, mood_color: Vec3) -> Vec3 {
        match self {
            Self::Sleep => Vec3::new(0.7, 0.7, 1.0),
            Self::Note => mood_color,
            Self::Heart | Self::Longing => Vec3::new(1.0, 0.55, 0.75),
            Self::Alarm => Vec3::new(1.0, 0.9, 0.45),
            Self::Question => Vec3::new(0.45, 0.8, 1.0),
            Self::Anger => Vec3::new(1.0, 0.45, 0.3),
            Self::Sigh => Vec3::new(0.7, 0.65, 0.8),
            Self::Sparkle => Vec3::new(1.0, 0.8, 0.35),
        }
    }
}

/// One symbol on its way up from the ghost's head.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Puff {
    pub emote: Emote,
    pub glyph: &'static str,
    /// From just born at 0 to gone at 1.
    pub age: f32,
    /// In cells from the head, right and up.
    pub offset: Vec2,
}

/// The symbols rising now, oldest first. Beats and time slots keep this
/// stateless; at most the newest three symbols survive a busy moment.
pub fn puffs(world: &World) -> Vec<Puff> {
    let ghost = &world.ghost;
    // A hiding ghost gives nothing away.
    if ghost.fade > 0.3 {
        return Vec::new();
    }
    let mut starts = Vec::new();
    let mut add = |start: f64, emote| {
        if (0.0..LIFETIME).contains(&(world.time - start)) {
            starts.push((start, emote));
        }
    };

    let source = if ghost.asleep > 0.5 {
        if world
            .dream
            .is_some_and(|dream| dream.envelope(world.time) > 0.0)
        {
            None
        } else {
            Some((Emote::Sleep, SLEEP_BEAT, 1.0))
        }
    } else if world.petting() >= 0.35 {
        Some((Emote::Heart, 1.1, 0.85))
    } else {
        match ghost.mood {
            Mood::Grumpy => Some((Emote::Anger, 1.3, 1.0)),
            Mood::Cheerful | Mood::Excited
                if matches!(
                    ghost.plan.activity,
                    Activity::Float | Activity::Play | Activity::Tend
                ) =>
            {
                Some((Emote::Note, 5.0, 0.55))
            }
            Mood::Bored => Some((Emote::Sigh, 12.0, 0.6)),
            Mood::Lonely => Some((Emote::Longing, 10.0, 0.65)),
            Mood::Curious if ghost.plan.activity == Activity::Explore => {
                Some((Emote::Question, 8.0, 0.6))
            }
            _ => None,
        }
    };
    if let Some((emote, beat, chance)) = source {
        let current = (world.time / beat).floor();
        let seed = emote as u32 + 11;
        for slot in [current - 1.0, current] {
            if slot < 0.0 || random(slot, seed, 0) >= chance {
                continue;
            }
            let delay = if chance < 1.0 {
                f64::from(random(slot, seed, 1)) * (beat - LIFETIME).max(0.0)
            } else {
                0.0
            };
            add(slot * beat + delay, emote);
        }
    }

    if ghost.asleep <= 0.5 {
        for (index, event) in world.events.iter().enumerate() {
            if event.kind == EventKind::Tended && world.garden_recovering() {
                add(event.time, Emote::Heart);
                continue;
            }
            if event.kind == EventKind::BirdSang {
                let watching = world.sight().in_view.iter().any(|seen| {
                    seen.thing == Thing::Bird
                        && seen.hidden_behind.is_none()
                        && seen.off_center < 0.3
                });
                if watching {
                    add(event.time, Emote::Note);
                }
                continue;
            }
            if matches!(event.kind, EventKind::GameOver(Outcome::ViewerWon { .. }))
                && world.good_sport()
            {
                add(event.time, Emote::Heart);
                continue;
            }
            if matches!(event.kind, EventKind::Toppled { by_viewer: true }) {
                add(
                    event.time + 0.6,
                    if ghost.mood == Mood::Grumpy {
                        Emote::Anger
                    } else {
                        Emote::Note
                    },
                );
            }
            // What the ghost made of a message says more than having
            // heard it.
            let answered = matches!(event.kind, EventKind::Heard(_))
                && world.events[index + 1..]
                    .iter()
                    .take_while(|later| later.time == event.time)
                    .any(|later| reaction(&later.kind, world.life.bond).is_some());
            if let Some(emote) = reaction(&event.kind, world.life.bond)
                && !answered
            {
                add(event.time, emote);
            }
        }
    }

    starts.sort_by(|first, second| first.0.total_cmp(&second.0));
    let keep_from = starts.len().saturating_sub(3);
    starts
        .into_iter()
        .skip(keep_from)
        .map(|(start, emote)| {
            let age = ((world.time - start) / LIFETIME) as f32;
            let phase = random(start, emote as u32 + 11, 2) * TAU;
            let glyph = if emote == Emote::Sleep && (start / SLEEP_BEAT).round() as u64 % 2 == 1 {
                "Z"
            } else {
                emote.glyph()
            };
            Puff {
                emote,
                glyph,
                age,
                offset: Vec2::new(1.6 + 0.5 * (phase + age * TAU).sin(), 1.0 + 3.0 * age),
            }
        })
        .collect()
}

// The symbol an event puffs, if any. Greetings get a heart once the ghost
// has grown fond of the viewer.
fn reaction(event: &EventKind, bond: f32) -> Option<Emote> {
    Some(match event {
        EventKind::Noticed(thing)
            if thing.starts_with("the rainbow") || thing == "the falling snow" =>
        {
            Emote::Sparkle
        }
        EventKind::ShootingStar => Emote::Sparkle,
        EventKind::Noticed(_)
        | EventKind::KickedBall
        | EventKind::DroppedMote
        | EventKind::WokeUp
        | EventKind::Thunder
        | EventKind::Poked { count: 1 } => Emote::Alarm,
        EventKind::Poked { count } if *count >= 3 => Emote::Anger,
        EventKind::Jumped | EventKind::Toppled { .. } => Emote::Alarm,
        EventKind::GameOver(Outcome::GhostWon { .. })
        | EventKind::AteMote
        | EventKind::Bloomed
        | EventKind::Sprouted(_)
        | EventKind::Flowered(_)
        | EventKind::Fetched
        | EventKind::Kept(_)
        | EventKind::Pounced
        | EventKind::Built { .. }
        | EventKind::Found
        | EventKind::MoonWish
        | EventKind::Celebrated(_)
        | EventKind::Morning
        | EventKind::Milestone(_) => Emote::Sparkle,
        EventKind::Planted(_) => Emote::Note,
        EventKind::Drooped => Emote::Sigh,
        EventKind::Tended | EventKind::NudgedBall | EventKind::Danced => Emote::Note,
        EventKind::Doodled(_) | EventKind::Wrote { .. } if bond >= 0.3 => Emote::Heart,
        EventKind::Doodled(_) | EventKind::Wrote { .. } => Emote::Note,
        EventKind::Praised => Emote::Heart,
        EventKind::ExpectedArrival => Emote::Heart,
        EventKind::MissedVisit => Emote::Longing,
        EventKind::FriendArrived(_) => Emote::Sparkle,
        EventKind::FriendLeft(_) => Emote::Longing,
        EventKind::Tagged { .. } => Emote::Alarm,
        EventKind::PokedFriend | EventKind::PlayedWithFriend(_) => Emote::Note,
        EventKind::Scolded => Emote::Sigh,
        EventKind::SawDrawing { name: Some(name) } if name == "heart" => Emote::Heart,
        EventKind::SawDrawing { name: Some(name) } if matches!(name.as_str(), "star" | "sun") => {
            Emote::Sparkle
        }
        EventKind::SawDrawing { name: None } => Emote::Question,
        EventKind::LearnedPicture(_) => Emote::Sparkle,
        EventKind::Copied(_) => Emote::Note,
        EventKind::Puzzled(_) => Emote::Question,
        EventKind::LearnedWord { .. } | EventKind::LearnedName(_) | EventKind::Named(_) => {
            Emote::Sparkle
        }
        EventKind::Heard(_) | EventKind::ViewerArrived if bond >= 0.3 => Emote::Heart,
        EventKind::Heard(_) | EventKind::ViewerArrived => Emote::Alarm,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Event;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn every_symbol_fits_in_one_terminal_cell() {
        for emote in [
            Emote::Sleep,
            Emote::Note,
            Emote::Heart,
            Emote::Longing,
            Emote::Alarm,
            Emote::Question,
            Emote::Anger,
            Emote::Sigh,
            Emote::Sparkle,
        ] {
            assert_eq!(emote.glyph().width(), 1);
        }
        assert_eq!("Z".width(), 1);
    }

    #[test]
    fn an_asleep_ghost_has_a_sleep_puff_throughout_every_two_second_window() {
        let mut world = World::new(1, 12.0);
        world.ghost.asleep = 1.0;
        for window in 0..100 {
            for step in 0..20 {
                world.time = f64::from(window * 20 + step) * 0.1;
                assert!(puffs(&world).iter().any(|puff| puff.emote == Emote::Sleep));
            }
        }
    }

    #[test]
    fn a_fresh_notice_starts_an_alarm_puff() {
        let mut world = World::new(1, 12.0);
        world.events.push(Event {
            time: world.time,
            kind: EventKind::Noticed("a light mote".to_owned()),
        });
        let puffs = puffs(&world);
        assert_eq!(puffs.len(), 1);
        assert_eq!(puffs[0].emote, Emote::Alarm);
        assert_eq!(puffs[0].age, 0.0);
    }

    #[test]
    fn a_poke_streak_of_three_starts_an_anger_puff() {
        let mut world = World::new(1, 12.0);
        for _ in 0..3 {
            world.poke();
            world.time += 0.1;
        }
        assert_eq!(puffs(&world).last().unwrap().emote, Emote::Anger);
    }

    #[test]
    fn a_content_ghost_starts_without_puffs() {
        assert!(puffs(&World::new(1, 12.0)).is_empty());
    }

    #[test]
    fn at_most_three_puffs_survive_and_the_newest_win() {
        let mut world = World::new(1, 12.0);
        world.time = 1.0;
        for (index, kind) in [
            EventKind::KickedBall,
            EventKind::Poked { count: 3 },
            EventKind::AteMote,
            EventKind::Tended,
            EventKind::ViewerArrived,
        ]
        .into_iter()
        .enumerate()
        {
            world.events.push(Event {
                time: index as f64 * 0.1,
                kind,
            });
        }
        for mood in Mood::ALL {
            world.ghost.mood = mood;
            for step in 0..30 {
                world.time = f64::from(step) * 0.1;
                assert!(puffs(&world).len() <= 3);
            }
        }
        world.ghost.mood = Mood::Content;
        world.time = 1.0;
        assert_eq!(
            puffs(&world)
                .iter()
                .map(|puff| puff.emote)
                .collect::<Vec<_>>(),
            [Emote::Sparkle, Emote::Note, Emote::Alarm]
        );
    }

    #[test]
    fn a_puff_ages_and_rises_then_disappears_at_the_end_of_its_life() {
        let mut world = World::new(1, 12.0);
        world.events.push(Event {
            time: 1.0,
            kind: EventKind::AteMote,
        });
        assert!(puffs(&world).is_empty());
        let mut previous_height = 0.0;
        for step in 0..22 {
            world.time = 1.0 + f64::from(step) * 0.1;
            let puff = puffs(&world)[0];
            assert!((0.0..=1.0).contains(&puff.age));
            assert!(puff.offset.is_finite());
            assert!(puff.offset.y > previous_height);
            previous_height = puff.offset.y;
        }
        world.time = 1.0 + LIFETIME;
        assert!(puffs(&world).is_empty());
    }

    #[test]
    fn a_hiding_ghost_puffs_nothing() {
        let mut world = World::new(1, 12.0);
        world.ghost.asleep = 1.0;
        world.ghost.fade = 0.85;
        assert!(puffs(&world).is_empty());
    }

    #[test]
    fn sleeping_suppresses_other_feelings_and_event_reactions() {
        let mut world = World::new(1, 12.0);
        world.ghost.asleep = 1.0;
        world.ghost.mood = Mood::Cheerful;
        for kind in [EventKind::Tended, EventKind::AteMote, EventKind::KickedBall] {
            world.events.push(Event { time: 0.0, kind });
        }
        assert!(puffs(&world).iter().all(|puff| puff.emote == Emote::Sleep));
    }

    #[test]
    fn greetings_get_a_heart_once_the_ghost_is_fond_of_the_viewer() {
        let mut world = World::new(1, 12.0);
        for kind in [
            EventKind::Heard("hello".to_owned()),
            EventKind::ViewerArrived,
        ] {
            world.events = vec![Event { time: 0.0, kind }];
            for (bond, expected) in [(0.29, Emote::Alarm), (0.3, Emote::Heart)] {
                world.life.bond = bond;
                assert_eq!(puffs(&world)[0].emote, expected);
            }
        }
    }

    #[test]
    fn occasional_moods_have_puffs_and_quiet_slots_and_respect_the_activity() {
        let mut world = World::new(1, 12.0);
        for (mood, activity, expected) in [
            (Mood::Cheerful, Activity::Float, Emote::Note),
            (Mood::Excited, Activity::Play, Emote::Note),
            (Mood::Cheerful, Activity::Tend, Emote::Note),
            (Mood::Bored, Activity::Float, Emote::Sigh),
            (Mood::Lonely, Activity::Float, Emote::Longing),
            (Mood::Curious, Activity::Explore, Emote::Question),
        ] {
            world.ghost.mood = mood;
            world.ghost.plan.activity = activity;
            let mut active = 0;
            let mut quiet = 0;
            for step in 0..1200 {
                world.time = f64::from(step) * 0.25;
                let puffs = puffs(&world);
                if puffs.is_empty() {
                    quiet += 1;
                } else {
                    active += 1;
                    assert!(puffs.iter().all(|puff| puff.emote == expected));
                }
            }
            assert!(active > 0 && quiet > 0, "{mood:?}");
        }
        for mood in [Mood::Cheerful, Mood::Excited, Mood::Curious] {
            world.ghost.mood = mood;
            world.ghost.plan.activity = Activity::Rest;
            for step in 0..100 {
                world.time = f64::from(step);
                assert!(puffs(&world).is_empty());
            }
        }
    }

    #[test]
    fn the_same_world_gives_the_same_puffs_without_changing_its_random_source() {
        let mut world = World::new(1, 12.0);
        world.ghost.mood = Mood::Grumpy;
        let mut untouched = world.clone();
        for step in 0..200 {
            world.time = f64::from(step) * 0.17;
            untouched.time = world.time;
            assert_eq!(puffs(&world), puffs(&world));
            assert_eq!(puffs(&world), puffs(&untouched));
            assert!(
                puffs(&world)
                    .iter()
                    .all(|puff| (0.0..=1.0).contains(&puff.age))
            );
        }
        let weights = [1.0, 2.0, 3.0];
        assert_eq!(world.pick(&weights), untouched.pick(&weights));
    }
}
