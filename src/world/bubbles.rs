use glam::Vec3;

use super::{
    Activity, EventKind, Heard, Lamp, Mood, Nook, ROOM_MAX, ROOM_MIN, World, books, breeze, flat,
    garden, music,
};

pub const LIMIT: usize = 16;
pub const POP_FOR: f64 = 0.35;

/// Soap and spray live only in this visit, never in the saved room.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bubble {
    pub position: Vec3,
    pub velocity: Vec3,
    pub radius: f32,
    pub seed: f32,
    pub born: f64,
    pub lifetime: f64,
    pub popping: Option<f64>,
}

impl Bubble {
    pub fn pop(&mut self, time: f64) -> bool {
        if self.popping.is_some() {
            return false;
        }
        self.popping = Some(time);
        self.velocity = Vec3::ZERO;
        true
    }

    pub fn hit(&self, origin: Vec3, direction: Vec3) -> Option<f32> {
        if self.popping.is_some() {
            return None;
        }
        let offset = self.position - origin;
        let along = offset.dot(direction);
        let square = self.radius * self.radius - (offset - direction * along).length_squared();
        if along <= 0.0 || square < 0.0 {
            return None;
        }
        Some((along - square.sqrt()).max(0.0))
    }
}

#[derive(Clone, Debug)]
pub(super) struct Session {
    total: u32,
    blown: u32,
    popped_by_viewer: u32,
    next_breath: f32,
    last_breath: Option<f64>,
    chase_at: f32,
    chase: Option<f64>,
    head: bool,
    giggled: bool,
}

pub fn requested(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    if lower.contains(['?', '？', '"', '“', '”']) {
        return false;
    }
    let words: Vec<_> = lower
        .split(|character: char| !character.is_alphabetic() && character != '\'')
        .filter(|word| !word.is_empty())
        .collect();
    if words.iter().any(|word| {
        matches!(
            *word,
            "no" | "not" | "never" | "don't" | "dont" | "stop" | "without" | "can't" | "cannot"
        )
    }) || [
        "말",
        "싫",
        "안 ",
        "안불",
        "안하",
        "못",
        "않",
        "뭐",
        "왜",
        "어떻게",
        "니",
        "까",
    ]
    .iter()
    .any(|piece| lower.contains(piece))
    {
        return false;
    }
    if words.contains(&"비눗방울") {
        return lower.ends_with("불어줘")
            || lower.ends_with("불어 줘")
            || lower.ends_with("하자")
            || lower.ends_with("불자");
    }
    let words: Vec<_> = words
        .into_iter()
        .filter(|word| !matches!(*word, "please" | "some" | "soap" | "let's" | "lets"))
        .collect();
    matches!(words.as_slice(), ["bubbles"] | ["blow" | "make", "bubbles"])
}

impl World {
    pub fn bubbles_are_open(&self) -> bool {
        self.ghost.asleep < 0.3
            && self.ghost.needs.energy > 0.3
            && self.ghost.needs.fun < 0.98
            && self.focus.is_none()
            && self.telling.is_none()
            && self.hunt.is_none()
            && !self
                .game
                .as_ref()
                .is_some_and(|game| game.outcome.is_none())
            && !self.puzzle_active()
            && self.bubble_session.is_none()
    }

    pub(super) fn hear_bubbles(&mut self, text: &str, korean: bool) -> Option<Heard> {
        if !requested(text) {
            return None;
        }
        self.record(EventKind::Heard(text.to_owned()));
        let continuing = self.ghost.plan.activity == Activity::Bubbles && !self.ghost.plan.done;
        let accepted = continuing || self.bubbles_are_open();
        if accepted && !continuing {
            self.begin(Activity::Bubbles);
        }
        Some(Heard {
            tone: None,
            understanding: super::Understanding::Bubbles { accepted },
            named: false,
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    pub(super) fn bubble_spot(&mut self) -> Vec3 {
        let depth = if self.window_open && self.wind.strength > 0.2 {
            -0.6
        } else {
            0.0
        };
        Vec3::new(
            self.random.range(-0.35, 0.35),
            self.random.range(1.1, 1.4),
            depth,
        )
    }

    pub(super) fn begin_bubbles(&mut self) {
        self.bubble_session = Some(Session {
            total: self.random.range(4.0, 11.0) as u32,
            blown: 0,
            popped_by_viewer: 0,
            next_breath: 1.0,
            last_breath: None,
            chase_at: self.ghost.plan.length * self.random.range(0.55, 0.8),
            chase: None,
            head: self.random.unit() < 0.35,
            giggled: false,
        });
    }

    pub fn bubble_breath(&self) -> f32 {
        if self.ghost.plan.activity != Activity::Bubbles || self.ghost.plan.done {
            return 0.0;
        }
        self.bubble_session
            .as_ref()
            .and_then(|session| session.last_breath)
            .map_or(0.0, |at| {
                (1.0 - (self.time - at).abs() as f32 / 0.8).max(0.0)
            })
    }

    pub fn bubble_wand(&self) -> Option<Vec3> {
        (self.ghost.plan.activity == Activity::Bubbles && !self.ghost.plan.done)
            .then(|| self.ghost.position + Vec3::new(0.09, 0.04, 0.39) * self.size())
    }

    fn emit_bubble(&mut self, position: Vec3) {
        // An overflowing breath retires the oldest bubble first.
        if self.bubbles.len() >= LIMIT {
            self.bubbles[0].pop(self.time);
            self.bubbles.remove(0);
        }
        self.bubbles.push(Bubble {
            position,
            velocity: Vec3::new(
                self.random.range(-0.12, 0.12),
                self.random.range(-0.02, 0.12),
                self.random.range(0.18, 0.32),
            ),
            radius: self.random.range(0.04, 0.11),
            seed: self.random.range(0.0, std::f32::consts::TAU),
            born: self.time,
            lifetime: f64::from(self.random.range(8.0, 20.0)),
            popping: None,
        });
    }

    pub(super) fn carry_out_bubbles(&mut self) -> Option<Vec3> {
        self.ghost.plan.arrived?;
        let stayed = self.ghost.plan.stayed(self.time);
        let mut session = self.bubble_session.take()?;
        if stayed >= session.next_breath && session.blown < session.total {
            let count = (self.random.range(1.0, 4.0) as u32).min(session.total - session.blown);
            let ring = self
                .bubble_wand()
                .unwrap_or(self.ghost.position + Vec3::new(0.09, 0.04, 0.39) * self.size());
            for _ in 0..count {
                let offset = Vec3::new(
                    self.random.range(-0.025, 0.025),
                    self.random.range(-0.025, 0.025),
                    self.random.range(0.02, 0.06),
                );
                self.emit_bubble(ring + offset);
            }
            session.blown += count;
            session.last_breath = Some(self.time);
            let remaining = session.total - session.blown;
            let breaths = (remaining as f32 / 2.0).ceil().max(1.0);
            session.next_breath =
                stayed + ((self.ghost.plan.length - stayed - 1.0) / breaths).max(1.0);
        }
        if stayed >= session.chase_at && session.chase.is_none() {
            session.chase = Some(self.time);
            session.chase_at = f32::INFINITY;
        }
        if let Some(started) = session.chase {
            let target = self
                .bubbles
                .iter()
                .filter(|bubble| bubble.popping.is_none())
                .min_by(|first, second| {
                    first
                        .position
                        .distance_squared(self.ghost.position)
                        .total_cmp(&second.position.distance_squared(self.ghost.position))
                })
                .map(|bubble| {
                    bubble.position
                        - if session.head {
                            Vec3::Y * 0.27 * self.size()
                        } else {
                            Vec3::ZERO
                        }
                });
            if self.time - started < 2.5
                && let Some(target) = target
            {
                self.ghost.plan.target = target;
            } else {
                session.chase = None;
                self.ghost.plan.target = self.ghost.plan.anchor;
            }
        }
        if stayed >= self.ghost.plan.length && session.blown == session.total {
            self.ghost.plan.done = true;
        }
        let look = self
            .bubbles
            .iter()
            .rev()
            .find(|bubble| bubble.popping.is_none())
            .map(|bubble| bubble.position);
        self.bubble_session = Some(session);
        look
    }

    pub fn pop_bubble(&mut self, origin: Vec3, direction: Vec3) -> bool {
        let direction = direction.normalize_or_zero();
        let nearest = self
            .bubbles
            .iter()
            .enumerate()
            .filter_map(|(index, bubble)| {
                bubble
                    .hit(origin, direction)
                    .map(|distance| (index, distance))
            })
            .min_by(|first, second| first.1.total_cmp(&second.1));
        let Some((index, _)) = nearest else {
            return false;
        };
        self.bubbles[index].pop(self.time);
        if let Some(session) = &mut self.bubble_session {
            session.popped_by_viewer += 1;
        }
        self.notice_viewer();
        true
    }

    pub(super) fn advance_bubbles(&mut self, seconds: f32) {
        if self.bubbles.is_empty() && self.bubble_session.is_none() {
            return;
        }
        // Furniture circles are shared with navigation; heights stop a
        // bubble flying above a cushion from hitting its floor footprint.
        let obstacles: Vec<_> = self
            .obstacles()
            .into_iter()
            .map(|(position, radius)| {
                let height = if position == self.lamp.position {
                    Lamp::HEIGHT
                } else if position == self.plant.position {
                    self.plant.height()
                } else if position == self.nook.position {
                    Nook::HEIGHT
                } else if position == books::POSITION {
                    0.24
                } else if position == music::POSITION {
                    music::HEIGHT
                } else if let Some(index) = garden::SPOTS.iter().position(|spot| *spot == position)
                {
                    self.seedlings
                        .get(index)
                        .map_or(0.3, |plant| plant.height())
                } else {
                    self.decoration()
                        .map_or(0.3, |decoration| decoration.height())
                };
                (position, radius, height)
            })
            .collect();
        let ghost_radius = 0.27 * self.size();
        let mut bumped = false;
        for bubble in &mut self.bubbles {
            if bubble.popping.is_some() {
                continue;
            }
            let age = (self.time - bubble.born) as f32;
            let phase = age * 1.6 + bubble.seed;
            let wander =
                Vec3::new(phase.sin(), (phase * 0.7).cos() * 0.3, (phase * 0.83).cos()) * 0.035;
            let buoyancy = Vec3::Y * (0.075 - bubble.radius) * 2.4;
            let air = breeze::breeze(self.window_open, &self.wind, self.time, bubble.position)
                + self.jostle.stir(bubble.position, bubble.seed);
            bubble.velocity += (air * 0.9 + wander + buoyancy - bubble.velocity * 0.65) * seconds;
            bubble.velocity = bubble.velocity.clamp_length_max(2.0);
            bubble.position += bubble.velocity * seconds;
            let low = ROOM_MIN + Vec3::splat(bubble.radius);
            let high = ROOM_MAX - Vec3::splat(bubble.radius);
            let wall = bubble.position.cmple(low).any() || bubble.position.cmpge(high).any();
            bubble.position = bubble.position.clamp(low, high);
            let furniture = obstacles.iter().any(|(position, radius, height)| {
                bubble.position.y - bubble.radius <= *height
                    && flat(bubble.position - *position).length_squared()
                        < (radius + bubble.radius).powi(2)
            }) || self.blocks.iter().any(|block| {
                bubble.position.distance_squared(block.position)
                    < (super::blocks::RADIUS + bubble.radius).powi(2)
            }) || bubble.position.distance_squared(self.ball.position)
                < (super::Ball::RADIUS + bubble.radius).powi(2);
            let ghost = bubble.position.distance_squared(self.ghost.position)
                < (ghost_radius + bubble.radius).powi(2);
            if wall || furniture || ghost || f64::from(age) >= bubble.lifetime {
                bubble.pop(self.time);
                bumped |= ghost;
            }
        }
        if bumped
            && self.ghost.plan.activity == Activity::Bubbles
            && self.ghost.asleep < 0.3
            && let Some(session) = &mut self.bubble_session
            && !session.giggled
        {
            session.giggled = true;
            self.suggest_mood(Mood::Excited, 4.0);
            self.say(self.spoken("Hee! Pop!", "헤헤! 톡!"));
        }
        self.bubbles
            .retain(|bubble| bubble.popping.is_none_or(|at| self.time - at < POP_FOR));
        let finished = self.ghost.plan.activity != Activity::Bubbles || self.ghost.plan.done;
        if finished
            && self.bubbles.is_empty()
            && let Some(session) = self.bubble_session.take()
        {
            self.bubbles_finished_at = Some(self.time);
            if session.blown > 0 {
                self.record(EventKind::BlewBubbles {
                    blown: session.blown,
                    popped_by_viewer: session.popped_by_viewer,
                });
                self.ghost.needs.change(0.0, 0.06, 0.0);
                if session.popped_by_viewer >= 3
                    && self.ghost.asleep < 0.3
                    && self.focus.is_none()
                    && self.telling.is_none()
                {
                    self.suggest_mood(Mood::Excited, 12.0);
                    self.say(self.spoken("You got them!", "다 터뜨렸다!"));
                }
            }
        }
    }

    pub fn bubble_words(&self) -> &'static str {
        if self.ghost.plan.activity == Activity::Bubbles && !self.ghost.plan.done {
            if self.bubbles.is_empty() {
                "you are getting ready to blow soap bubbles"
            } else if self.window_open {
                "you are blowing soap bubbles; several drift on the breeze from the window"
            } else {
                "you are blowing soap bubbles; little rainbow spheres drift around you"
            }
        } else if !self.bubbles.is_empty() {
            "the last soap bubbles are drifting and popping"
        } else {
            "no soap bubbles just now"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Understanding};

    fn bubble(position: Vec3, born: f64) -> Bubble {
        Bubble {
            position,
            velocity: Vec3::ZERO,
            radius: 0.07,
            seed: 0.7,
            born,
            lifetime: 12.0,
            popping: None,
        }
    }

    #[test]
    fn requests_in_both_languages_ignore_questions_negations_and_other_speech() {
        for text in [
            "blow bubbles",
            "make bubbles",
            "bubbles!",
            "Please blow some soap bubbles!",
            "비눗방울 불어줘",
            "비눗방울 하자",
            "비눗방울 불어 줘",
        ] {
            assert!(requested(text), "{text}");
            let mut world = World::new(13, 12.0);
            let heard = world.hear(text);
            assert_eq!(
                heard.understanding,
                Understanding::Bubbles { accepted: true }
            );
            assert_eq!(heard.tone, None);
            assert_eq!(world.ghost.plan.activity, Activity::Bubbles);
            let started = world.ghost.plan.started;
            world.time += 0.2;
            world.hear(text);
            assert_eq!(world.ghost.plan.started, started);
        }
        for text in [
            "no bubbles",
            "don't blow bubbles",
            "do not make bubbles",
            "bubbles?",
            "what are bubbles",
            "can you blow bubbles",
            "I like bubbles",
            "stop bubbles",
            "비눗방울 불어줄까",
            "비눗방울 불지 말아",
            "비눗방울 안 불어줘",
            "비눗방울 싫어",
            "비눗방울이 뭐야?",
            "my name is Bubbles",
        ] {
            assert!(!requested(text), "{text}");
        }
    }

    #[test]
    fn sleeping_working_and_busy_ghosts_defer_without_abandoning_their_plan() {
        for busy in 0..6 {
            let mut world = World::new(13, 12.0);
            match busy {
                0 => {
                    world.tuck_in();
                    world.ghost.asleep = 1.0;
                }
                1 => {
                    world.start_focus(25);
                }
                2 => {
                    world.challenge();
                }
                3 => {
                    world.hear("let's play a word game");
                }
                4 => {
                    world.tell(Vec::new());
                }
                _ => {
                    world.start_hiding().unwrap();
                }
            }
            let before = world.ghost.plan.clone();
            let asleep = world.ghost.asleep;
            let heard = world.hear("blow bubbles");
            assert_eq!(
                heard.understanding,
                Understanding::Bubbles { accepted: false },
                "{busy}"
            );
            assert_eq!(world.ghost.plan, before);
            assert_eq!(world.ghost.asleep, asleep);
            assert!(!world.options().contains(&Activity::Bubbles));
        }
    }

    #[test]
    fn bubbles_pop_on_every_wall_furniture_the_ghost_and_their_deadline() {
        let center = Vec3::new(0.0, 1.6, 0.0);
        for axis in 0..3 {
            for high in [false, true] {
                let mut world = World::new(3, 12.0);
                let mut position = center;
                position[axis] = if high {
                    ROOM_MAX[axis] - 0.03
                } else {
                    ROOM_MIN[axis] + 0.03
                };
                world.bubbles.push(bubble(position, world.time));
                world.advance_bubbles(0.1);
                assert!(world.bubbles[0].popping.is_some());
                assert!(world.bubbles[0].position.cmpge(ROOM_MIN).all());
                assert!(world.bubbles[0].position.cmple(ROOM_MAX).all());
            }
        }
        let mut world = World::new(3, 12.0);
        for position in [
            world.lamp.position + Vec3::Y * 0.6,
            world.plant.position + Vec3::Y * 0.2,
            world.nook.position + Vec3::Y * 0.08,
            books::POSITION + Vec3::Y * 0.1,
            music::POSITION + Vec3::Y * 0.2,
            world.blocks[0].position,
            world.ball.position,
            world.ghost.position,
        ] {
            world.bubbles = vec![bubble(position, world.time)];
            world.advance_bubbles(0.1);
            assert!(world.bubbles[0].popping.is_some(), "{position:?}");
        }
        world.bubbles = vec![bubble(world.nook.position + Vec3::Y * 1.5, world.time)];
        world.advance_bubbles(0.1);
        assert!(world.bubbles[0].popping.is_none());
        world.time = 12.1;
        world.advance_bubbles(0.1);
        assert!(world.bubbles[0].popping.is_some());
        world.time += POP_FOR + 0.01;
        world.advance_bubbles(0.1);
        assert!(world.bubbles.is_empty());
    }

    #[test]
    fn drift_and_resize_swirls_are_deterministic_finite_bounded_and_capped() {
        let mut world = World::new(11, 12.0);
        world.window_open = true;
        world.wind.strength = 0.5;
        for _ in 0..100 {
            world.emit_bubble(Vec3::new(0.0, 1.6, -0.7));
            assert!(world.bubbles.len() <= LIMIT);
        }
        let mut same = world.clone();
        let mut still = world.clone();
        world.jostle(0.4, 0.2);
        same.jostle(0.4, 0.2);
        for _ in 0..240 {
            for room in [&mut world, &mut same, &mut still] {
                room.time += 0.1;
                room.advance_bubbles(0.1);
                for bubble in &room.bubbles {
                    assert!(bubble.position.is_finite() && bubble.velocity.is_finite());
                    assert!(
                        bubble.position.cmpge(ROOM_MIN).all()
                            && bubble.position.cmple(ROOM_MAX).all()
                    );
                }
            }
            assert_eq!(world.bubbles, same.bubbles);
            if !world.bubbles.is_empty() && !still.bubbles.is_empty() {
                assert_ne!(
                    world.bubbles.last().unwrap().position,
                    still.bubbles.last().unwrap().position
                );
            }
        }
        assert!(world.bubbles.is_empty());
    }

    #[test]
    fn small_bubbles_rise_and_large_bubbles_sink_in_still_air() {
        let mut world = World::new(3, 12.0);
        world.ghost.position = Vec3::new(-1.5, 1.5, 1.0);
        let position = Vec3::new(0.0, 1.6, 0.0);
        for radius in [0.04, 0.11] {
            let mut soap = bubble(position, 0.0);
            soap.radius = radius;
            world.bubbles.push(soap);
        }
        for _ in 0..30 {
            world.time += 0.1;
            world.advance_bubbles(0.1);
        }
        assert!(world.bubbles[0].position.y > position.y);
        assert!(world.bubbles[1].position.y < position.y);
    }

    #[test]
    fn each_full_session_emits_four_to_ten_and_records_once_after_the_last_spray() {
        for seed in 0..40 {
            let mut world = World::new(seed, 12.0);
            world.begin(Activity::Bubbles);
            let total = world.bubble_session.as_ref().unwrap().total;
            assert!((4..=10).contains(&total));
            assert!((15.0..=30.0).contains(&world.ghost.plan.length));
            world.ghost.position = world.ghost.plan.target;
            for _ in 0..650 {
                world.step(0.1);
            }
            assert!(world.ghost.plan.done);
            assert!(world.bubbles.is_empty());
            assert!(world.bubble_session.is_none());
            assert_eq!(world.life.total.bubble_sessions, 1);
            assert_eq!(world.life.total.bubbles_blown, total);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| matches!(event.kind, EventKind::BlewBubbles { .. }))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn viewer_pops_count_once_per_bubble_and_finish_with_delight_and_one_milestone() {
        let mut world = World::new(5, 12.0);
        world.begin(Activity::Bubbles);
        world.ghost.position = world.ghost.plan.target;
        for _ in 0..500 {
            world.step(0.1);
            while let Some(soap) = world.bubbles.iter().find(|bubble| bubble.popping.is_none()) {
                let origin = soap.position + Vec3::Z;
                assert!(world.pop_bubble(origin, -Vec3::Z));
            }
        }
        assert!(world.life.total.bubbles_popped >= 3);
        assert_eq!(
            world.life.total.bubbles_popped,
            world.life.total.bubbles_blown
        );
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "You got them!");
        assert!(
            world
                .life
                .today
                .words()
                .contains("blew bubbles and you popped them")
        );
        let milestone = "You popped my bubbles for the first time.";
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == milestone)
                .count(),
            1
        );
        world.record(EventKind::BlewBubbles {
            blown: 4,
            popped_by_viewer: 3,
        });
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == milestone)
                .count(),
            1
        );
        let words = EventKind::BlewBubbles {
            blown: 9,
            popped_by_viewer: 4,
        }
        .words();
        assert_eq!(words, "blew nine bubbles and you popped four of them");
        assert!(!words.chars().any(char::is_numeric));
    }

    #[test]
    fn interrupted_sessions_count_only_the_bubbles_already_blown() {
        let mut world = World::new(3, 12.0);
        world.begin(Activity::Bubbles);
        world.ghost.position = world.ghost.plan.target;
        world.step(1.2);
        let blown = world.bubble_session.as_ref().unwrap().blown;
        assert!((1..=3).contains(&blown));
        world.begin(Activity::Explore);
        assert_eq!(world.life.total.bubble_sessions, 0);
        world.step(25.0);
        assert_eq!(world.life.total.bubble_sessions, 1);
        assert_eq!(world.life.total.bubbles_blown, blown);
    }

    #[test]
    fn old_saves_default_bubble_counts_and_damaged_counts_are_bounded() {
        let mut value = serde_json::to_value(Life::default()).unwrap();
        for name in ["today", "total"] {
            for field in ["bubble_sessions", "bubbles_blown", "bubbles_popped"] {
                value[name].as_object_mut().unwrap().remove(field);
            }
        }
        let mut life: Life = serde_json::from_value(value).unwrap();
        assert_eq!(life.total.bubble_sessions, 0);
        for tally in [&mut life.total, &mut life.today] {
            tally.bubble_sessions = u32::MAX;
            tally.bubbles_blown = u32::MAX;
            tally.bubbles_popped = u32::MAX;
        }
        let clean = life.sanitized();
        assert_eq!(clean.total.bubble_sessions, 1_000_000);
        assert_eq!(clean.total.bubbles_blown, 10_000_000);
        assert_eq!(clean.total.bubbles_popped, clean.total.bubbles_blown);
    }

    #[test]
    fn spontaneous_sessions_pause_for_an_hour_and_reset_the_daily_limit() {
        let mut world = World::new(3, 12.0);
        world.step(0.0);
        world.begin(Activity::Bubbles);
        world.ghost.position = world.ghost.plan.target;
        world.step(55.0);
        assert_eq!(world.life.total.bubble_sessions, 1);
        assert!(!world.options().contains(&Activity::Bubbles));
        world.time += 3601.0;
        assert!(world.options().contains(&Activity::Bubbles));
        for _ in 0..3 {
            world.life.note(&EventKind::Began(Activity::Bubbles));
        }
        assert!(!world.options().contains(&Activity::Bubbles));
        world.date = world.date.tomorrow().unwrap();
        world.step(0.0);
        assert!(world.options().contains(&Activity::Bubbles));
        world.bubbles_finished_at = Some(world.time);
        let heard = world.hear("bubbles!");
        assert_eq!(
            heard.understanding,
            Understanding::Bubbles { accepted: true }
        );
    }
}
