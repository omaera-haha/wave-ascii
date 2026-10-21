use std::f32::consts::TAU;

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, Mood, ROOM_MAX, Stage, World};
use crate::face::expression::Expression;

pub const CENTER: Vec3 = Vec3::new(ROOM_MAX.x, 1.15, -0.3);
pub const WIDTH: f32 = 0.76;
pub const HEIGHT: f32 = 1.2;
pub const FRAME_WIDTH: f32 = 0.05;
pub const FRAME_DEPTH: f32 = 0.03;
pub const STANDOFF: f32 = 0.02;
pub const VISIT_DISTANCE: f32 = 0.65;
pub const VISIT: Vec3 = Vec3::new(CENTER.x - VISIT_DISTANCE, CENTER.y, CENTER.z);
const WITH_ACCESSORY: u32 = 4;
const WITHOUT_ACCESSORY: u32 = 10;
const SESSIONS_LIMIT: u32 = 100_000_000;

pub fn contains(point: Vec3) -> bool {
    (point.y - CENTER.y).abs() <= HEIGHT * 0.5 && (point.z - CENTER.z).abs() <= WIDTH * 0.5
}

pub fn reflect_direction(direction: Vec3) -> Vec3 {
    Vec3::new(-direction.x, direction.y, direction.z)
}

pub fn reflect_point(point: Vec3) -> Vec3 {
    Vec3::new(2.0 * CENTER.x - point.x, point.y, point.z)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MirrorLife {
    #[serde(default)]
    pub sessions: u32,
    #[serde(default)]
    pub recognized: Option<String>,
}

impl MirrorLife {
    pub fn recognizes_self(&self) -> bool {
        self.recognized.is_some()
    }

    pub fn words(&self) -> &'static str {
        if self.recognizes_self() {
            "it's me!"
        } else if self.sessions >= 2 {
            "it copies me"
        } else {
            "a stranger lives there"
        }
    }

    pub(super) fn sanitize(&mut self, today: &str) {
        self.sessions = self.sessions.min(SESSIONS_LIMIT);
        self.recognized = self.recognized.take().filter(|day| {
            self.sessions >= WITH_ACCESSORY
                && day.parse::<jiff::civil::Date>().is_ok_and(|date| {
                    today
                        .parse::<jiff::civil::Date>()
                        .is_ok_and(|today| date <= today)
                })
        });
    }
}

#[derive(Clone, Debug)]
pub(super) struct Visit {
    familiar: bool,
    twirl: bool,
    startled: bool,
    recognition_at: Option<f64>,
}

impl World {
    pub(super) fn begin_mirror(&mut self) {
        let familiar = self.life.mirror.recognizes_self();
        let twirl = familiar
            && self.focus.is_none()
            && self.ghost.needs.energy > 0.35
            && self.random.unit() < 0.4;
        self.mirror_visit = Some(Visit {
            familiar,
            twirl,
            startled: false,
            recognition_at: None,
        });
        self.ghost.look = Some(CENTER);
    }

    pub(super) fn carry_out_mirror(&mut self) -> Option<Vec3> {
        let stayed = self.ghost.plan.stayed(self.time);
        let Some(visit) = &self.mirror_visit else {
            self.ghost.plan.done = true;
            return None;
        };
        if self.ghost.plan.arrived.is_none() {
            if !visit.familiar
                && !visit.startled
                && self.ghost.position.x > VISIT.x - 0.3
                && self.ghost.velocity.x > 0.4 - 0.2 * self.life.nature.unwrap_or_default().shy
            {
                self.mirror_visit.as_mut().unwrap().startled = true;
                self.ghost.velocity.x = -0.2 - 0.3 * self.life.nature.unwrap_or_default().shy;
                self.suggest_mood(Mood::Curious, 3.0);
            }
            return Some(CENTER);
        }
        if !self.life.total.met_reflection {
            self.record(EventKind::MetReflection);
            self.think("Hello? Who are you?");
        }
        if stayed >= 6.0
            && !self.life.mirror.recognizes_self()
            && self.stage() != Stage::Newborn
            && self.life.mirror.sessions
                >= if self.wearing.is_some() {
                    WITH_ACCESSORY
                } else {
                    WITHOUT_ACCESSORY
                }
        {
            self.life.mirror.recognized = Some(self.date.to_string());
            let visit = self.mirror_visit.as_mut().unwrap();
            visit.recognition_at = Some(self.time);
            visit.familiar = true;
            self.record(EventKind::RecognizedSelf);
            self.suggest_mood(Mood::Excited, 8.0);
            self.think("The ghost in the mirror is me!");
        }
        let visit = self.mirror_visit.as_ref().unwrap();
        let mut target = self.ghost.plan.anchor;
        let mut look = CENTER;
        if let Some(recognized) = visit.recognition_at
            && self.time - recognized < 3.0
        {
            look.y += 0.3;
            target.y += 0.07;
        } else if visit.familiar {
            if self.wearing.is_some() && stayed < 5.0 {
                look.y += 0.25;
            }
            if visit.twirl && (8.0..12.0).contains(&stayed) {
                let phase = (stayed - 8.0) * TAU / 4.0;
                target += Vec3::new(0.1 * phase.sin(), 0.03 * phase.sin(), 0.12 * phase.cos());
            }
            if visit.twirl && stayed >= 12.0 && self.ghost.plan.count == 0 {
                self.ghost.plan.count = 1;
                self.record(EventKind::Danced);
            }
        } else if self.life.mirror.sessions >= 2 {
            target.y += 0.09 * (stayed * 1.5).sin();
            target.z += 0.16 * (stayed * 0.9).sin();
        } else {
            target.y += 0.07 * (stayed * 2.0).sin().max(0.0);
            target.z += 0.08 * (stayed * 0.8).sin();
            if (stayed % 9.0) > 6.0 {
                look = target - Vec3::Z;
            }
        }
        self.ghost.plan.target = target;
        if stayed >= self.ghost.plan.length {
            self.life.mirror.sessions = self
                .life
                .mirror
                .sessions
                .saturating_add(1)
                .min(SESSIONS_LIMIT);
            self.record(EventKind::VisitedMirror);
            self.ghost.needs.change(0.0, 0.06, 0.0);
            self.ghost.plan.done = true;
            self.mirror_visit = None;
        }
        Some(look)
    }

    pub fn mirror_expression(&self) -> Option<(f64, Expression)> {
        if self.ghost.plan.activity != Activity::Mirror || self.ghost.plan.done {
            return None;
        }
        let arrived = self.ghost.plan.arrived?;
        let visit = self.mirror_visit.as_ref()?;
        if !visit.familiar
            || visit
                .recognition_at
                .is_some_and(|recognized| self.time - recognized < 3.0)
        {
            return None;
        }
        let stayed = (self.time - arrived) as f32;
        if self.wearing.is_some() && stayed < 3.0 {
            return Some((
                arrived,
                Expression::Glance {
                    toward: Vec2::new(0.0, 0.6),
                },
            ));
        }
        let slot = (stayed / 4.0).floor() as u32;
        let expression = match slot % 3 {
            0 => Expression::Smile,
            1 => Expression::Wink,
            _ => Expression::Glare,
        };
        Some((arrived + f64::from(slot) * 4.0, expression))
    }

    pub fn mirror_thoughts(&self) -> Vec<&'static str> {
        if self.life.mirror.recognizes_self() {
            let mut lines = vec![
                "That's my own little glow.",
                "I can make such funny faces.",
                "A twirl, just for me.",
            ];
            if let Some(accessory) = self.wearing {
                lines.push(match accessory {
                    super::Accessory::Scarf { .. } => "This scarf suits me.",
                    super::Accessory::PartyHat | super::Accessory::WitchHat => {
                        "My hat looks good on me."
                    }
                    super::Accessory::Flower(_) => "My flower looks lovely there.",
                    _ => "That suits me rather well.",
                });
            }
            lines
        } else if self.life.mirror.sessions >= 2 {
            vec![
                "It copies everything I do…",
                "If I bob, it bobs too.",
                "It waves when I wave!",
            ]
        } else {
            vec![
                "Hello? Who are you?",
                "Are you a ghost too?",
                "Does it live behind the wall?",
            ]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let mut world = World::new(1, 12.0);
        world.begin_life(&world.date.to_string());
        world.calendar_at = Some(world.date);
        world
    }

    fn arrive(world: &mut World) {
        world.begin(Activity::Mirror);
        world.ghost.position = world.ghost.plan.anchor;
        world.ghost.plan.arrived = Some(world.time);
        world.carry_out_mirror();
    }

    fn finish(world: &mut World) {
        world.time += f64::from(world.ghost.plan.length) + 0.01;
        world.carry_out_mirror();
        assert!(world.ghost.plan.done);
    }

    #[test]
    fn visits_target_the_glass_and_finish_after_their_settled_length() {
        let mut world = world();
        world.begin(Activity::Mirror);
        assert_eq!(world.ghost.plan.target, VISIT);
        assert_eq!(world.ghost.look, Some(CENTER));
        assert!((CENTER.x - world.ghost.plan.target.x - VISIT_DISTANCE).abs() < 1e-6);
        assert!((15.0..=30.0).contains(&world.ghost.plan.length));
        world.ghost.position = VISIT;
        let length = world.ghost.plan.length;
        world.step(0.1);
        assert!(!world.ghost.plan.done);
        world.step(length + 0.2);
        assert!(world.ghost.plan.done);
        assert_eq!(world.life.mirror.sessions, 1);
        assert_eq!(world.life.today.mirror_visits, 1);
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("looked in the mirror")
        );
        assert!(
            world
                .life
                .today
                .words()
                .contains("met the ghost in the mirror")
        );
    }

    #[test]
    fn an_interrupted_first_meeting_is_remembered_without_counting_a_session() {
        let mut world = world();
        arrive(&mut world);
        world.begin(Activity::Float);
        assert_eq!(world.life.mirror.sessions, 0);
        arrive(&mut world);
        finish(&mut world);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::MetReflection)
                .count(),
            1
        );
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == "I met a ghost in the mirror.")
                .count(),
            1
        );
        assert_eq!(world.life.mirror.sessions, 1);
    }

    #[test]
    fn testing_begins_on_the_third_visit_and_quick_approaches_startle_shy_ghosts_more() {
        let mut world = world();
        world.life.mirror.sessions = 2;
        arrive(&mut world);
        world.time += 1.0;
        assert_eq!(world.carry_out_mirror(), Some(CENTER));
        assert_ne!(world.ghost.plan.target.y, VISIT.y);
        assert_ne!(world.ghost.plan.target.z, VISIT.z);
        let recoil = |shy| {
            let mut world = World::new(1, 12.0);
            world.life.nature.as_mut().unwrap().shy = shy;
            world.begin(Activity::Mirror);
            world.ghost.position = VISIT - Vec3::X * 0.25;
            world.ghost.velocity = Vec3::X;
            world.carry_out_mirror();
            world.ghost.velocity.x
        };
        assert!(recoil(1.0) < recoil(0.0));
        assert!(recoil(0.0) < 0.0);
    }

    #[test]
    fn recognition_needs_completed_visits_and_age_and_happens_only_once() {
        for (birthday, accessory, threshold) in [
            (None, None, WITHOUT_ACCESSORY),
            (
                Some(jiff::civil::Date::constant(2026, 6, 10)),
                Some(super::super::Accessory::Feather),
                WITH_ACCESSORY,
            ),
        ] {
            let mut world = world();
            world.birthday = birthday;
            world.wearing = accessory;
            world.life.mirror.sessions = threshold - 1;
            arrive(&mut world);
            finish(&mut world);
            assert!(!world.life.mirror.recognizes_self());
            arrive(&mut world);
            world.time += 6.1;
            world.carry_out_mirror();
            assert_eq!(world.life.mirror.recognized, Some(world.date.to_string()));
            assert_eq!(world.ghost.suggested_mood.unwrap().0, Mood::Excited);
            assert!(world.ghost.look.is_some());
            assert!(world.mirror_expression().is_none());
            world.time += 3.1;
            world.carry_out_mirror();
            assert!(world.mirror_expression().is_some());
            finish(&mut world);
            arrive(&mut world);
            finish(&mut world);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| event.kind == EventKind::RecognizedSelf)
                    .count(),
                1
            );
            assert_eq!(
                world
                    .life
                    .milestones
                    .iter()
                    .filter(|entry| entry.text == "I know the ghost in the mirror is me.")
                    .count(),
                1
            );
            assert!(
                world
                    .life
                    .today
                    .words()
                    .contains("learned the ghost in the mirror is me")
            );
        }
    }

    #[test]
    fn newborns_never_recognize_themselves_and_sleepers_cannot_begin_a_visit() {
        let mut world = world();
        world.birthday = Some(world.date);
        world.life.mirror.sessions = 100;
        world.wearing = Some(super::super::Accessory::Feather);
        arrive(&mut world);
        finish(&mut world);
        assert!(!world.life.mirror.recognizes_self());
        world.hour = 23.0;
        world.tuck_in();
        let previous = world.ghost.plan.clone();
        assert!(!world.options().contains(&Activity::Mirror));
        world.begin(Activity::Mirror);
        assert_eq!(world.ghost.plan, previous);
    }

    #[test]
    fn familiar_twirls_count_as_dancing_once_and_focus_keeps_them_quiet() {
        let mut dances = 0;
        for seed in 0..20 {
            for focusing in [false, true] {
                let mut world = World::new(seed, 12.0);
                world.life.mirror.sessions = 10;
                world.life.mirror.recognized = Some(world.date.to_string());
                if focusing {
                    world.start_focus(25);
                }
                arrive(&mut world);
                assert!(world.mirror_expression().is_some());
                assert!(world.options().contains(&Activity::Mirror));
                world.time += 12.1;
                world.carry_out_mirror();
                world.carry_out_mirror();
                if focusing {
                    assert_eq!(world.life.today.dances, 0);
                } else {
                    assert!(world.life.today.dances <= 1);
                    dances += world.life.today.dances;
                    if world.life.today.dances > 0 {
                        assert!(world.life.skills.practice(super::super::Skill::Dancing) > 0.0);
                    }
                }
            }
        }
        assert!(dances > 0 && dances < 20);
    }

    #[test]
    fn mirror_memories_round_trip_default_for_old_saves_and_drop_invalid_recognition_days() {
        use super::super::Life;
        let mut world = world();
        world.life.mirror.sessions = 10;
        world.life.mirror.recognized = Some(world.date.to_string());
        let encoded = serde_json::to_string(&world.life).unwrap();
        let restored: Life = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.clone().sanitized().mirror, world.life.mirror);
        let old: Life = serde_json::from_str("{}").unwrap();
        assert_eq!(old.mirror, MirrorLife::default());
        for (sessions, day) in [
            (0, "2026-06-21"),
            (10, "bad"),
            (10, "2026-02-30"),
            (10, "2099-01-01"),
        ] {
            let mut damaged = restored.clone();
            damaged.mirror.sessions = sessions;
            damaged.mirror.recognized = Some(day.to_owned());
            assert!(!damaged.sanitized().mirror.recognizes_self());
        }
        let mut damaged = restored;
        damaged.mirror.sessions = u32::MAX;
        damaged.total.mirror_visits = u32::MAX;
        let repaired = damaged.sanitized();
        assert_eq!(repaired.mirror.sessions, SESSIONS_LIMIT);
        assert_eq!(repaired.total.mirror_visits, SESSIONS_LIMIT);
    }

    #[test]
    fn reflection_flips_only_the_normal_and_fixes_the_mirror_plane() {
        let direction = Vec3::new(0.3, -0.7, 1.2);
        let reflected = reflect_direction(direction);
        assert_eq!(reflected, Vec3::new(-direction.x, direction.y, direction.z));
        assert_eq!(direction.length_squared(), reflected.length_squared());
        assert_eq!(reflect_point(CENTER), CENTER);
        let point = Vec3::new(-0.4, 0.8, 0.2);
        assert!(reflect_point(reflect_point(point)).distance(point) < 1e-6);
    }
}
