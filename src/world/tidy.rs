use glam::Vec3;

use super::{
    ARRIVED, Activity, Ball, EventKind, Heard, Listening, Mood, Understanding, World,
    blocks::{EDGE, place_neatly},
    flat,
};

pub const SPOTS: [Vec3; 2] = [Vec3::new(1.48, 0.0, 0.95), Vec3::new(1.10, 0.0, 0.95)];
pub(super) const EVENING_HOUR: f32 = super::sleep::BEDTIME_HOUR - 0.5;
const CARRY_HEIGHT: f32 = 0.42;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    Gather,
    Carry,
    Ball,
    CarryBall,
    Admire,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Tidying {
    stage: Stage,
    requested: bool,
    leave_one: bool,
    blocks: u32,
    ball: bool,
}

pub(super) fn requested(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    if lower.contains('?') || lower.contains('？') {
        return false;
    }
    let lower = lower.trim_end_matches(['.', '!', '。', '！']);
    let words: Vec<_> = lower
        .split_whitespace()
        .filter(|word| !matches!(*word, "please" | "let's" | "lets"))
        .collect();
    matches!(
        words.as_slice(),
        ["tidy", "up"]
            | ["tidy", "your", "room"]
            | ["clean", "up", "your", "room"]
            | ["정리하자"]
            | ["방", "치워"]
            | ["방", "치워줘"]
            | ["방", "정리하자"]
    )
}

impl World {
    pub(super) fn tidy_height(&self, index: usize) -> usize {
        self.blocks
            .iter()
            .filter(|block| block.stacked && block.tidy == Some(index))
            .count()
    }

    pub(super) fn tidy_stacks(&self) -> impl Iterator<Item = (usize, Vec3, usize)> + '_ {
        SPOTS.into_iter().enumerate().filter_map(|(index, spot)| {
            let height = self.tidy_height(index);
            (height > 0).then_some((index, spot, height))
        })
    }

    pub(super) fn building_block(&self, index: usize) -> bool {
        let block = &self.blocks[index];
        block.on_floor()
            || block.tidy.is_some_and(|stack| {
                !block.held
                    && block.stacked
                    && (block.position.y - (self.tidy_height(stack) as f32 - 0.5) * EDGE).abs()
                        < 0.01
            })
    }

    pub(super) fn topple_tidy(&mut self, index: usize, direction: Vec3) {
        if self.tidy_height(index) > 0 {
            self.topple_stack(SPOTS[index], Some(index), direction);
        }
    }

    pub fn tidy_ball_spot(&self) -> Vec3 {
        self.nook.position + Vec3::new(0.57, Ball::RADIUS, 0.03)
    }

    fn ball_needs_tidying(&self) -> bool {
        flat(self.ball.position - self.nook.position).length() > 0.6
    }

    fn loose_for_tidy(&self, index: usize) -> bool {
        self.blocks[index].on_floor()
    }

    fn loose_to_tidy(&self) -> usize {
        (0..self.blocks.len())
            .filter(|index| self.loose_for_tidy(*index))
            .count()
    }

    fn can_tidy(&self) -> bool {
        self.ghost.asleep < 0.01
            && self.ghost.needs.energy > 0.3
            && self.focus.is_none()
            && self.telling.is_none()
            && self.hunt.is_none()
            && self.friend.is_none()
            && !self.puzzle_active()
            && !self
                .game
                .as_ref()
                .is_some_and(|game| game.outcome.is_none())
            && (self.ghost.plan.done
                || matches!(
                    self.ghost.plan.activity,
                    Activity::Float
                        | Activity::Explore
                        | Activity::Visit
                        | Activity::Window
                        | Activity::Wait
                        | Activity::Rest
                        | Activity::Tidy
                ))
    }

    // The evening a night's chores belong to: the small hours count for the
    // evening before.
    fn tidy_evening(&self) -> jiff::civil::Date {
        if self.hour < super::sleep::MORNING_HOUR {
            self.date.yesterday().unwrap_or(self.date)
        } else {
            self.date
        }
    }

    pub fn tidy_is_open(&self) -> bool {
        // Midnight must not offer a second round of the same night's chores.
        let evening = self.tidy_evening();
        (self.bedtime() || self.hour >= EVENING_HOUR)
            && self
                .life
                .tidied_day
                .as_ref()
                .is_none_or(|day| day.as_str() < evening.to_string().as_str())
            && self.can_tidy()
            && (self
                .blocks
                .iter()
                .filter(|block| {
                    block.on_floor()
                        && SPOTS
                            .iter()
                            .all(|spot| flat(block.position - *spot).length() > EDGE)
                })
                .count()
                >= 2
                || self.ball_needs_tidying())
    }

    pub(super) fn may_begin_tidy(&self) -> bool {
        self.can_tidy()
            && if self.pending_tidy {
                self.loose_to_tidy() > 0 || self.ball_needs_tidying()
            } else {
                self.tidy_is_open()
            }
    }

    pub(super) fn hear_tidy(&mut self, text: &str, korean: bool) -> Option<Heard> {
        if !requested(text) {
            return None;
        }
        self.listening = Listening::default();
        self.record(EventKind::Heard(text.to_owned()));
        let continuing = self.ghost.plan.activity == Activity::Tidy && !self.ghost.plan.done;
        if continuing {
            if let Some(tidying) = &mut self.tidying {
                tidying.requested = true;
                tidying.leave_one = false;
            }
        } else {
            self.pending_tidy = true;
            self.resume_tidy_request();
        }
        let lines = if self.ghost.plan.activity == Activity::Tidy && !self.ghost.plan.done {
            ("I'll put everything away.", "응, 제자리에 정리할게.")
        } else if self.pending_tidy {
            ("I'll tidy up a little later.", "조금 이따 정리할게.")
        } else {
            (
                "Everything is in its place already.",
                "벌써 다 제자리에 있어.",
            )
        };
        self.say(if korean { lines.1 } else { lines.0 });
        Some(Heard {
            tone: None,
            understanding: Understanding::Tidying,
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    pub(super) fn resume_tidy_request(&mut self) {
        if !self.pending_tidy {
            return;
        }
        if self.loose_to_tidy() == 0 && !self.ball_needs_tidying() {
            self.pending_tidy = false;
        } else if self.may_begin_tidy() {
            self.begin(Activity::Tidy);
        }
    }

    pub(super) fn begin_tidy(&mut self) {
        let requested = std::mem::take(&mut self.pending_tidy);
        let playful = self.life.temperament().playful;
        self.tidying = Some(Tidying {
            stage: Stage::Gather,
            requested,
            leave_one: !requested && playful > 0.5 && self.random.unit() < playful * 0.35,
            blocks: 0,
            ball: false,
        });
    }

    pub fn tidy_words(&self) -> &'static str {
        if self.ghost.plan.activity == Activity::Tidy && !self.ghost.plan.done {
            "you are tidying the blocks into neat stacks and putting the ball beside the cushion before bed"
        } else if self.tidy_stacks().next().is_some() {
            "neat stacks of blocks rest in the front-right corner"
        } else {
            "the loose blocks wait for another evening tidy-up"
        }
    }

    fn tidy_target(&mut self, target: Vec3) {
        if self.ghost.plan.target != target {
            self.ghost.plan.target = target;
            self.ghost.plan.arrived = None;
        }
    }

    pub(super) fn carry_out_tidy(&mut self) -> Option<Vec3> {
        let mut tidying = self.tidying?;
        if !self.can_tidy() || self.ghost.plan.elapsed(self.time) > 180.0 {
            self.drop_block();
            self.drop_ball();
            self.tidying = None;
            self.ghost.plan.done = true;
            return None;
        }
        let mut look = SPOTS[0] + Vec3::Y * EDGE;
        match tidying.stage {
            Stage::Gather => {
                let remaining = self.loose_to_tidy();
                if remaining <= usize::from(tidying.leave_one)
                    || !SPOTS
                        .iter()
                        .enumerate()
                        .any(|(index, _)| self.tidy_height(index) < 3)
                {
                    tidying.stage = Stage::Ball;
                } else if let Some(index) = (0..self.blocks.len())
                    .filter(|index| self.loose_for_tidy(*index))
                    .min_by(|first, second| {
                        self.blocks[*first]
                            .position
                            .distance_squared(self.ghost.position)
                            .total_cmp(
                                &self.blocks[*second]
                                    .position
                                    .distance_squared(self.ghost.position),
                            )
                    })
                {
                    look = self.blocks[index].position;
                    let target = look + Vec3::Y * CARRY_HEIGHT;
                    self.tidy_target(target);
                    if self.ghost.position.distance(target) < ARRIVED {
                        self.blocks[index].held = true;
                        self.blocks[index].spin = Vec3::ZERO;
                        tidying.stage = Stage::Carry;
                    }
                }
            }
            Stage::Carry => {
                if let Some((stack, spot)) = SPOTS
                    .into_iter()
                    .enumerate()
                    .find(|(index, _)| self.tidy_height(*index) < 3)
                {
                    let height = self.tidy_height(stack);
                    let target = spot + Vec3::Y * ((height as f32 + 0.5) * EDGE + CARRY_HEIGHT);
                    self.tidy_target(target);
                    look = spot;
                    if self.ghost.position.distance(target) < ARRIVED {
                        if let Some(index) = self.blocks.iter().position(|block| block.held) {
                            place_neatly(&mut self.blocks[index], spot, height, &mut self.random);
                            self.blocks[index].tidy = Some(stack);
                            tidying.blocks += 1;
                        }
                        tidying.stage = Stage::Gather;
                    }
                } else {
                    self.drop_block();
                    tidying.stage = Stage::Ball;
                }
            }
            Stage::Ball => {
                if self.ball_needs_tidying() {
                    look = self.ball.position;
                    let target = look + Vec3::Y * CARRY_HEIGHT;
                    self.tidy_target(target);
                    if self.ghost.position.distance(target) < ARRIVED
                        && self.ball.velocity.length() < 0.8
                    {
                        self.ball.held = true;
                        self.ball.velocity = Vec3::ZERO;
                        tidying.stage = Stage::CarryBall;
                    }
                } else {
                    tidying.stage = Stage::Admire;
                    self.tidy_target(Vec3::new(0.45, 0.85, 0.65));
                }
            }
            Stage::CarryBall => {
                if !self.ball.held {
                    self.tidying = None;
                    self.ghost.plan.done = true;
                    return None;
                }
                let spot = self.tidy_ball_spot();
                self.tidy_target(spot + Vec3::Y * CARRY_HEIGHT);
                look = spot;
                if self.ghost.position.distance(self.ghost.plan.target) < ARRIVED {
                    self.ball = Ball::at(spot.x, spot.z);
                    tidying.ball = true;
                    tidying.stage = Stage::Admire;
                    self.tidy_target(Vec3::new(0.45, 0.85, 0.65));
                }
            }
            Stage::Admire => {
                look = SPOTS[0].lerp(
                    self.nook.position,
                    (self.ghost.plan.stayed(self.time) / 2.0).clamp(0.0, 1.0),
                );
                if self.ghost.plan.stayed(self.time) >= 2.0 {
                    self.tidying = None;
                    self.ghost.plan.done = true;
                    if tidying.blocks > 0 || tidying.ball {
                        if !tidying.requested {
                            self.life.tidied_day = Some(self.tidy_evening().to_string());
                        }
                        self.record(EventKind::Tidied {
                            blocks: tidying.blocks,
                            ball: tidying.ball,
                        });
                        self.suggest_mood(Mood::Cheerful, 4.0);
                        self.say(self.spoken("Everything in its place.", "이제 다 제자리에 있네."));
                    }
                    if !tidying.requested && (self.bedtime() || self.hour >= EVENING_HOUR) {
                        if self.lamp.on {
                            self.begin(Activity::Lamp);
                            self.tidy_rest = true;
                        } else {
                            self.begin(Activity::Rest);
                            self.ghost.plan.overnight = true;
                        }
                    }
                    return Some(look);
                }
            }
        }
        self.tidying = Some(tidying);
        Some(look)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Block, Life, Nature, ROOM_MAX, ROOM_MIN, blocks::RADIUS};

    fn messy(hour: f32) -> World {
        let mut world = World::new(7, hour);
        world.begin_life("2026-06-21");
        world.life.nature = Some(Nature {
            playful: 0.1,
            tender: 0.9,
            ..Nature::default()
        });
        world.ghost.needs.energy = 0.8;
        world.blocks = [
            (0.0, -0.2),
            (0.45, -0.1),
            (-0.3, 0.7),
            (0.8, 0.45),
            (0.9, -0.6),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (across, depth))| Block::at(Vec3::new(across, 0.0, depth), index as u8))
        .collect();
        world.ball = Ball::at(0.0, 0.9);
        world
    }

    fn finish(world: &mut World) {
        for _ in 0..1800 {
            world.step(0.1);
            if world.life.total.tidies > 0 {
                return;
            }
        }
        panic!(
            "unfinished tidy: {:?}, position {:?}, target {:?}, blocks {:?}",
            world.tidying, world.ghost.position, world.ghost.plan.target, world.blocks
        );
    }

    #[test]
    fn the_corner_stacks_and_ball_rest_clear_furniture_and_stay_inside_the_room() {
        let mut world = messy(22.0);
        for date in ["2026-06-21", "2026-12-24", "2027-01-01"] {
            world.date = date.parse().unwrap();
            for position in SPOTS {
                for axis in [0, 2] {
                    assert!(position[axis] > ROOM_MIN[axis] + RADIUS);
                    assert!(position[axis] < ROOM_MAX[axis] - RADIUS);
                }
                assert!(world.obstacles().iter().all(|(center, radius)| {
                    flat(position - *center).length() > radius + RADIUS + 0.05
                }));
                for pot in crate::world::garden::SPOTS {
                    assert!(
                        flat(position - pot).length() > crate::world::Seedling::POT_RADIUS + RADIUS
                    );
                }
                assert!(flat(position - crate::world::mirror::VISIT).length() > RADIUS + 0.4);
                assert!(flat(position - crate::world::cranes::SEAT).length() > RADIUS + 0.3);
                assert!(flat(position - crate::world::cranes::CENTER).length() > RADIUS + 0.5);
                assert!(flat(position - Vec3::new(0.0, 0.0, 1.12)).length() > RADIUS + 0.3);
                assert!(flat(position).length() > RADIUS + 0.5);
            }
        }
        assert!(flat(SPOTS[0] - SPOTS[1]).length() > RADIUS * 2.0 + 0.05);
        let ball = world.tidy_ball_spot();
        assert!(
            world
                .obstacles()
                .iter()
                .all(|(center, radius)| { flat(ball - *center).length() > radius + Ball::RADIUS })
        );
        assert!(ball.cmpge(ROOM_MIN).all() && ball.cmple(ROOM_MAX).all());
        assert!(flat(ball - world.nook.position).length() < 0.6);
    }

    #[test]
    fn gathering_is_nearest_first_and_fills_one_stack_before_the_next() {
        let mut world = messy(12.0);
        world.hear("tidy up");
        for placed in 0..5 {
            let nearest = (0..world.blocks.len())
                .filter(|index| world.loose_for_tidy(*index))
                .min_by(|first, second| {
                    world.blocks[*first]
                        .position
                        .distance_squared(world.ghost.position)
                        .total_cmp(
                            &world.blocks[*second]
                                .position
                                .distance_squared(world.ghost.position),
                        )
                })
                .unwrap();
            world.carry_out_tidy();
            assert_eq!(
                world.ghost.plan.target,
                world.blocks[nearest].position + Vec3::Y * CARRY_HEIGHT
            );
            world.ghost.position = world.ghost.plan.target;
            world.carry_out_tidy();
            assert!(world.blocks[nearest].held);
            world.carry_out_tidy();
            world.ghost.position = world.ghost.plan.target;
            world.carry_out_tidy();
            assert_eq!(world.blocks[nearest].tidy, Some(placed / 3));
            assert!(
                (world.blocks[nearest].position.y - ((placed % 3) as f32 + 0.5) * EDGE).abs()
                    < 0.001
            );
            assert_eq!(world.tidy_height(0), (placed + 1).min(3));
            assert_eq!(world.tidy_height(1), (placed + 1).saturating_sub(3));
        }
    }

    #[test]
    fn a_complete_tidy_carries_the_ball_to_the_cushion_and_stays_at_rest() {
        let mut world = messy(12.0);
        world.hear("tidy up");
        finish(&mut world);
        assert_eq!(world.tidy_height(0), 3);
        assert_eq!(world.tidy_height(1), 2);
        assert_eq!(world.tower_height(), 0);
        assert!(flat(world.ball.position - world.nook.position).length() < 0.6);
        assert_eq!(world.ball.velocity, Vec3::ZERO);
        assert!(world.events.iter().any(|event| event.kind
            == EventKind::Tidied {
                blocks: 5,
                ball: true
            }));
        let room = world.saved_room();
        world.step(15.0);
        assert_eq!(world.saved_room(), room);
        assert!(world.life.tidied_day.is_none());
    }

    #[test]
    fn a_standing_tower_is_left_alone_while_loose_blocks_are_tidied() {
        let mut world = messy(12.0);
        let spot = Vec3::new(0.0, 0.0, 0.3);
        world.tower = Some(spot);
        for index in 0..2 {
            place_neatly(&mut world.blocks[index], spot, index, &mut world.random);
        }
        let standing = world.blocks[..2].to_vec();
        world.hear("tidy up");
        finish(&mut world);
        assert_eq!(&world.blocks[..2], standing.as_slice());
        assert_eq!(world.tower_height(), 2);
        assert_eq!(world.tidy_height(0), 3);
        let room = world.saved_room();
        let mut restored = World::new(8, 12.0);
        restored.restore_room(&room);
        assert_eq!(restored.saved_room(), room);
        restored.topple(Vec3::X);
        assert_eq!(restored.tidy_height(0), 3);
    }

    #[test]
    fn spontaneous_tidying_requires_a_mess_and_an_unused_evening_slot() {
        let mut world = messy(12.0);
        for hour in [6.5, 12.0, 21.49] {
            world.hour = hour;
            assert!(!world.tidy_is_open());
        }
        for hour in [21.5, 22.0, 23.5, 0.5, 6.49] {
            world.hour = hour;
            assert!(world.tidy_is_open());
        }
        world.life.tidied_day = Some(world.date.to_string());
        assert!(!world.tidy_is_open());
        world.date = world.date.tomorrow().unwrap();
        assert!(!world.tidy_is_open());
        world.hour = 22.0;
        assert!(world.tidy_is_open());
        world.ball = Ball::at(world.tidy_ball_spot().x, world.tidy_ball_spot().z);
        world.blocks.truncate(1);
        assert!(!world.tidy_is_open());
        world.blocks.push(Block::at(Vec3::new(0.5, 0.0, 0.7), 1));
        assert!(world.tidy_is_open());
        world.blocks.clear();
        world.ball = Ball::at(0.0, 0.0);
        assert!(world.tidy_is_open());
        world.ghost.needs.energy = 0.1;
        assert!(!world.tidy_is_open());
    }

    #[test]
    fn sleep_work_stories_games_hunts_and_visiting_friends_defer_tidying() {
        for busy in 0..8 {
            let mut world = messy(22.0);
            match busy {
                0 => world.tuck_in(),
                1 => world.start_focus(25),
                2 => world.tell(vec!["A little story.".to_owned(); 5]),
                3 => {
                    world.begin(Activity::Game);
                }
                4 => {
                    world.begin(Activity::Puzzle);
                }
                5 => {
                    world.start_hiding().unwrap();
                }
                6 => world.invite_friend(),
                _ => world.begin(Activity::Build),
            }
            let plan = world.ghost.plan.clone();
            let asleep = world.ghost.asleep;
            assert!(!world.options().contains(&Activity::Tidy), "{busy}");
            world.begin(Activity::Tidy);
            assert_eq!(world.ghost.plan, plan, "{busy}");
            let heard = world.hear("방 치워");
            assert_eq!(heard.understanding, Understanding::Tidying);
            assert!(world.pending_tidy, "{busy}");
            assert_eq!(world.ghost.plan, plan, "{busy}");
            assert_eq!(world.ghost.asleep, asleep);
            assert_eq!(
                world.ghost.speech.as_ref().unwrap().text,
                "조금 이따 정리할게."
            );
        }
    }

    #[test]
    fn requests_bypass_learning_in_both_languages_and_do_not_use_the_evening_slot() {
        for message in [
            "tidy up",
            "Please clean up your room!",
            "정리하자",
            "방 치워",
        ] {
            let mut world = messy(12.0);
            let vocabulary = world.life.vocabulary.clone();
            let heard = world.hear(message);
            assert_eq!(heard.understanding, Understanding::Tidying, "{message}");
            assert!(heard.tone.is_none());
            assert!(heard.learned.is_empty());
            assert_eq!(world.life.vocabulary, vocabulary);
            assert_eq!(world.ghost.plan.activity, Activity::Tidy);
            finish(&mut world);
            assert!(world.life.tidied_day.is_none());
        }
        for message in [
            "don't tidy up",
            "tidy up?",
            "I tidied up",
            "방 치우지 마",
            "정리하자고 했어",
            "the clean up your room song",
        ] {
            assert!(!requested(message), "{message}");
        }
    }

    #[test]
    fn deferred_requests_wait_for_waking_and_an_interruption_releases_a_carried_block() {
        let mut world = messy(22.0);
        world.tuck_in();
        world.hear("tidy up");
        world.step(1.0);
        assert_eq!(world.ghost.asleep, 1.0);
        world.hour = 6.5;
        world.step(0.1);
        assert_eq!(world.ghost.plan.activity, Activity::Tidy);
        assert!(!world.pending_tidy);
        for _ in 0..400 {
            world.step(0.1);
            if world.blocks.iter().any(|block| block.held) {
                break;
            }
        }
        let index = world.blocks.iter().position(|block| block.held).unwrap();
        let position = world.blocks[index].position;
        world.begin(Activity::Visit);
        assert!(!world.blocks[index].held);
        assert_eq!(world.blocks[index].position, position);
        assert_eq!(world.life.total.tidies, 0);
        assert!(world.life.tidied_day.is_none());
        world.hour = 22.0;
        assert!(world.tidy_is_open());
    }

    #[test]
    fn tidy_stacks_round_trip_and_clicks_or_a_fast_ball_topple_only_the_struck_stack() {
        let mut world = messy(12.0);
        world.hear("tidy up");
        finish(&mut world);
        let room = world.saved_room();
        let encoded = serde_json::to_string(&room).unwrap();
        for ball in [false, true] {
            let mut restored = World::new(8, 12.0);
            restored.restore_room(&serde_json::from_str(&encoded).unwrap());
            assert_eq!(restored.saved_room(), room);
            assert_eq!(restored.tidy_height(0), 3);
            if ball {
                restored.ball = Ball::at(SPOTS[0].x, SPOTS[0].z - 0.35);
                restored.ball.velocity = Vec3::Z * 2.0;
                restored.advance_blocks(0.1);
            } else {
                let index = restored
                    .blocks
                    .iter()
                    .position(|block| block.tidy == Some(0))
                    .unwrap();
                restored.nudge_block(index, Vec3::X);
                assert!(
                    restored
                        .events
                        .iter()
                        .any(|event| event.kind == EventKind::Toppled { by_viewer: true })
                );
            }
            assert_eq!(restored.tidy_height(0), 0);
            assert_eq!(restored.tidy_height(1), 2);
        }
    }

    #[test]
    fn the_next_days_build_can_take_blocks_from_the_top_of_a_tidy_stack() {
        let mut world = messy(12.0);
        world.hear("tidy up");
        finish(&mut world);
        assert!(world.options().contains(&Activity::Build));
        world.begin(Activity::Build);
        world.step(90.0);
        assert!(world.tower_height() > 0);
        assert!(world.tidy_height(0) + world.tidy_height(1) < 5);
        assert!(!world.blocks.iter().any(|block| block.held));
    }

    #[test]
    fn a_tidy_finished_after_midnight_belongs_to_the_evening_before() {
        let mut world = messy(23.9);
        world.begin(Activity::Tidy);
        world.hour = 0.5;
        world.date = world.date.tomorrow().unwrap();
        finish(&mut world);
        assert_eq!(world.life.tidied_day.as_deref(), Some("2026-06-21"));
    }

    #[test]
    fn finishing_at_bedtime_switches_off_the_lamp_before_overnight_rest() {
        let mut world = messy(22.0);
        world.lamp.on = true;
        world.begin(Activity::Tidy);
        finish(&mut world);
        assert_eq!(world.life.tidied_day.as_deref(), Some("2026-06-21"));
        assert_eq!(world.ghost.plan.activity, Activity::Lamp);
        world.step(60.0);
        assert!(!world.lamp.on);
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert!(world.ghost.plan.overnight);
        assert_eq!(world.ghost.asleep, 1.0);
        assert!(!world.options().contains(&Activity::Tidy));
    }

    #[test]
    fn tidying_counts_in_the_diary_and_marks_its_first_once_with_bounded_saved_data() {
        let mut life = Life::default();
        life.begin("2026-06-21");
        let tidy = EventKind::Tidied {
            blocks: 5,
            ball: true,
        };
        assert_eq!(
            life.note(&tidy).as_deref(),
            Some("I tidied my room before bed for the first time.")
        );
        assert!(life.note(&tidy).is_none());
        assert_eq!(life.total.tidies, 2);
        assert_eq!(life.today.tidies, 2);
        assert!(life.today.words().contains("Tidied up before bed"));
        assert!(
            !tidy
                .words()
                .chars()
                .any(|character| character.is_ascii_digit())
        );
        for day in ["broken", "2026-06-22"] {
            life.tidied_day = Some(day.to_owned());
            assert!(life.clone().sanitized().tidied_day.is_none());
        }
        life.tidied_day = Some("2026-06-21".to_owned());
        life.total.tidies = u32::MAX;
        let clean = life.sanitized();
        assert_eq!(clean.total.tidies, 1_000_000);
        let encoded = serde_json::to_string(&clean).unwrap();
        let restored: Life = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.tidied_day, clean.tidied_day);
        let mut old: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        old.as_object_mut().unwrap().remove("tidied_day");
        old["today"].as_object_mut().unwrap().remove("tidies");
        old["total"].as_object_mut().unwrap().remove("tidies");
        let restored: Life = serde_json::from_value(old).unwrap();
        assert!(restored.tidied_day.is_none());
        assert_eq!(restored.total.tidies, 0);
    }

    #[test]
    fn finishing_in_the_half_hour_before_bed_stays_asleep_until_morning() {
        for lamp in [false, true] {
            let mut world = messy(21.5);
            world.lamp.on = lamp;
            world.begin(Activity::Tidy);
            finish(&mut world);
            world.step(60.0);
            assert_eq!(world.ghost.plan.activity, Activity::Rest);
            assert!(world.ghost.plan.overnight);
            assert_eq!(world.ghost.asleep, 1.0);
            assert!(!world.lamp.on);
            world.hour = 22.0;
            world.step(0.1);
            assert_eq!(world.ghost.asleep, 1.0);
            world.hour = 6.5;
            world.step(0.1);
            assert_eq!(world.ghost.asleep, 0.0);
            assert!(world.morning());
        }
    }

    #[test]
    fn a_tidy_request_after_midnight_does_not_wake_a_sleeper_for_a_night_nudge() {
        let mut world = messy(1.0);
        world.tuck_in();
        let heard = world.hear("tidy up");
        assert_eq!(heard.understanding, Understanding::Tidying);
        world.step(0.1);
        assert_eq!(world.ghost.asleep, 1.0);
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert_eq!(world.life.total.late_nights, 0);
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "I'll tidy up a little later."
        );
    }

    #[test]
    fn playful_ghosts_sometimes_leave_the_last_block_but_requests_put_it_away() {
        let mut left_one = 0;
        for seed in 0..24 {
            let mut world = messy(22.0);
            world.random = super::super::Random::new(seed);
            world.life.nature.as_mut().unwrap().playful = 1.0;
            world.begin(Activity::Tidy);
            let leaving_one = world.tidying.unwrap().leave_one;
            finish(&mut world);
            assert_eq!(
                world.tidy_height(0) + world.tidy_height(1),
                if leaving_one { 4 } else { 5 }
            );
            left_one += usize::from(leaving_one);
            if leaving_one {
                world.hour = 12.0;
                world.poke();
                world.ghost.plan.done = true;
                world.hear("tidy up");
                assert_eq!(world.ghost.plan.activity, Activity::Tidy);
                for _ in 0..900 {
                    world.step(0.1);
                    if world.life.total.tidies == 2 {
                        break;
                    }
                }
                assert_eq!(world.tidy_height(0) + world.tidy_height(1), 5);
            }
        }
        assert!(left_one > 0 && left_one < 24);
    }

    #[test]
    fn damaged_tidy_membership_and_missing_stack_levels_reject_the_room_snapshot() {
        let mut world = messy(12.0);
        world.hear("tidy up");
        finish(&mut world);
        let room = world.saved_room();
        for corruption in 0..4 {
            let mut damaged = room.clone();
            let index = damaged
                .blocks
                .iter()
                .position(|block| block.tidy == Some(0))
                .unwrap();
            match corruption {
                0 => damaged.blocks[index].tidy = Some(2),
                1 => damaged.blocks[index].stacked = false,
                2 => damaged.blocks[index].position[1] += EDGE,
                _ => damaged.blocks[index].position[0] = 0.0,
            }
            let mut restored = World::new(8, 12.0);
            let seeded = restored.saved_room();
            restored.restore_room(&damaged);
            assert_eq!(restored.saved_room(), seeded, "{corruption}");
        }
        let mut encoded = serde_json::to_value(World::new(8, 12.0).saved_room()).unwrap();
        for block in encoded["blocks"].as_array_mut().unwrap() {
            block.as_object_mut().unwrap().remove("tidy");
        }
        let mut restored = World::new(8, 12.0);
        let seeded = restored.saved_room();
        restored.restore_room(&serde_json::from_value(encoded).unwrap());
        assert_eq!(restored.saved_room(), seeded);
    }

    #[test]
    fn flicking_the_carried_ball_ends_tidying_without_moving_it_back_to_the_cushion() {
        let mut world = messy(22.0);
        world.begin(Activity::Tidy);
        for _ in 0..1800 {
            world.step(0.1);
            if world.ball.held {
                break;
            }
        }
        assert!(world.ball.held);
        world.kick_ball(Vec3::X);
        let ball = world.ball;
        world.carry_out_tidy();
        assert!(world.ghost.plan.done);
        assert!(!world.ball.held);
        assert_eq!(world.ball.position, ball.position);
        assert_eq!(world.ball.velocity, ball.velocity);
        assert_eq!(world.life.total.tidies, 0);
        assert!(world.life.tidied_day.is_none());
    }
}
