use glam::Vec3;

use super::{Activity, EventKind, Heard, ROOM_MAX, ROOM_MIN, Thing, Understanding, World, flat};

pub const RADIUS: f32 = 0.06;
const COLUMNS: usize = 20;
const ROWS: usize = 14;
const CELLS: usize = COLUMNS * ROWS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hint {
    Warmer,
    Hot,
    Colder,
    Cold,
}

impl Hint {
    pub fn warmer(self) -> bool {
        matches!(self, Self::Warmer | Self::Hot)
    }

    pub fn words(self) -> &'static str {
        match self {
            Self::Warmer => "getting warmer",
            Self::Hot => "very close",
            Self::Colder => "getting colder",
            Self::Cold => "far away",
        }
    }
}

pub fn hint(text: &str) -> Option<Hint> {
    let text = text
        .trim()
        .trim_end_matches(['!', '.', '?', '！', '。', '？'])
        .to_lowercase();
    Some(match text.as_str() {
        "warmer" | "getting warmer" | "warm" | "따뜻해" | "따뜻해져" | "점점 따뜻해" => {
            Hint::Warmer
        }
        "hot" | "hotter" | "very warm" | "you're close" | "뜨거워" | "가까워" => Hint::Hot,
        "colder" | "getting colder" | "cool" | "차가워" | "차가워져" | "점점 차가워" => {
            Hint::Colder
        }
        "cold" | "freezing" | "far" | "추워" | "멀어" => Hint::Cold,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Counting,
    Searching,
    Revealing(f64),
    Swooping,
    Carrying,
    Showing(f64),
}

/// Only the sight check and the final reveal consult the hiding spot.
#[derive(Clone, Debug)]
pub struct Hunt {
    pub position: Vec3,
    pub hidden_at: f64,
    pub phase: Phase,
    pub hints: u32,
    pub last_hint: Option<Hint>,
    belief: [f32; CELLS],
    previous_hint: Vec3,
    target: usize,
    target_at: f64,
    next_scan: f64,
    asked: bool,
    gave_up: bool,
    seconds: u32,
}

fn center(index: usize) -> Vec3 {
    Vec3::new(
        ROOM_MIN.x
            + (index % COLUMNS) as f32 * (ROOM_MAX.x - ROOM_MIN.x) / COLUMNS as f32
            + (ROOM_MAX.x - ROOM_MIN.x) / COLUMNS as f32 * 0.5,
        RADIUS,
        ROOM_MIN.z
            + (index / COLUMNS) as f32 * (ROOM_MAX.z - ROOM_MIN.z) / ROWS as f32
            + (ROOM_MAX.z - ROOM_MIN.z) / ROWS as f32 * 0.5,
    )
}

impl Hunt {
    fn new(world: &World) -> Self {
        let obstacles = world.obstacles();
        let mut hunt = Self {
            position: Vec3::ZERO,
            hidden_at: world.time,
            phase: Phase::Counting,
            hints: 0,
            last_hint: None,
            belief: std::array::from_fn(|index| {
                let spot = center(index);
                if obstacles
                    .iter()
                    .any(|(obstacle, radius)| flat(spot - *obstacle).length() < radius + RADIUS)
                {
                    return 0.0;
                }
                let near = obstacles
                    .iter()
                    .any(|(obstacle, radius)| flat(spot - *obstacle).length() < radius + 0.6);
                let behind = obstacles.iter().any(|(obstacle, radius)| {
                    (spot.x - obstacle.x).abs() < radius + 0.15 && spot.z < obstacle.z
                });
                1.0 + f32::from(near) * 0.3 + f32::from(behind) * 0.2
            }),
            previous_hint: world.ghost.position,
            target: 0,
            target_at: world.time,
            next_scan: world.time,
            asked: false,
            gave_up: false,
            seconds: 0,
        };
        hunt.normalize();
        hunt
    }

    fn normalize(&mut self) {
        let sum: f32 = self.belief.iter().sum();
        if sum > 0.0 {
            for belief in &mut self.belief {
                *belief /= sum;
            }
        }
    }

    fn apply_hint(&mut self, hint: Hint, position: Vec3) {
        for (index, belief) in self.belief.iter_mut().enumerate() {
            if *belief == 0.0 {
                continue;
            }
            let distance = flat(center(index) - position).length();
            let nearer = distance < flat(center(index) - self.previous_hint).length();
            let factor = match hint {
                Hint::Warmer if nearer => 2.5,
                Hint::Colder if !nearer => 2.5,
                Hint::Warmer | Hint::Colder => 0.35,
                Hint::Hot if distance < 0.8 => 4.0,
                Hint::Hot => 0.1,
                Hint::Cold if distance < 0.8 => 0.1,
                Hint::Cold => 1.0,
            };
            *belief = (*belief * factor).max(1e-12);
        }
        self.normalize();
        self.previous_hint = position;
        self.hints = self.hints.saturating_add(1);
        self.last_hint = Some(hint);
        self.target_at = f64::NEG_INFINITY;
    }

    fn choose_target(&self, position: Vec3) -> usize {
        (0..CELLS)
            .filter(|index| self.belief[*index] > 0.0)
            .max_by(|first, second| {
                let score =
                    |index| self.belief[index] - 0.0004 * flat(center(index) - position).length();
                score(*first).total_cmp(&score(*second))
            })
            .unwrap_or(0)
    }
}

impl World {
    pub fn hunt_counting(&self) -> bool {
        self.hunt
            .as_ref()
            .is_some_and(|hunt| hunt.phase == Phase::Counting)
    }

    pub fn start_hiding(&mut self) -> Result<(), String> {
        if self.focus.is_some() {
            return Err("Treasure hunts wait until after focus time.".to_owned());
        }
        if self.ghost.asleep >= 0.3 {
            return Err(format!("{} is asleep.", self.ghost.name));
        }
        if self.ghost.needs.energy <= 0.25 {
            return Err(format!("{} needs a rest first.", self.ghost.name));
        }
        if self.telling.is_some()
            || self.puzzle_active()
            || self.hunt.is_some()
            || self
                .game
                .as_ref()
                .is_some_and(|game| game.outcome.is_none())
        {
            return Err("Finish this story or game first.".to_owned());
        }
        self.begin_day();
        self.notice_viewer();
        self.begin(Activity::Float);
        // Answers waiting from before belong to that conversation, not
        // to the game's warmer and colder.
        self.asking = None;
        self.agenda_answer = None;
        self.feeling_follow_up = None;
        self.rhythm.reply_until = None;
        self.ghost.plan.target = self.ghost.position;
        self.ghost.velocity = Vec3::ZERO;
        self.ghost.attention = None;
        self.ghost.heading = -Vec3::Z;
        self.hunt = Some(Hunt::new(self));
        Ok(())
    }

    pub fn cancel_hunt(&mut self) {
        // What the house shares is the host's to end.
        if self.companion {
            return;
        }
        if self.hunt.take().is_some() {
            self.ghost.plan.done = true;
        }
    }

    pub fn hide_treasure(&mut self, position: Vec3) -> bool {
        if !self.hunt_counting()
            || !position.is_finite()
            || !position.cmpge(ROOM_MIN).all()
            || !position.cmple(ROOM_MAX).all()
        {
            return false;
        }
        let position = self.treasure_spot(position);
        let hunt = self.hunt.as_mut().unwrap();
        hunt.position = position;
        hunt.hidden_at = self.time;
        hunt.next_scan = self.time;
        hunt.previous_hint = self.ghost.position;
        hunt.phase = Phase::Searching;
        self.begin(Activity::Seek);
        self.say(self.spoken("…ready or not!", "…찾는다!"));
        true
    }

    fn treasure_spot(&self, position: Vec3) -> Vec3 {
        let margin = RADIUS + 0.04;
        let minimum = ROOM_MIN + Vec3::new(margin, RADIUS, margin);
        let maximum = Vec3::new(ROOM_MAX.x - margin, RADIUS, ROOM_MAX.z - margin);
        let obstacles = self.obstacles();
        let mut spot = position.clamp(minimum, maximum);
        for _ in 0..32 {
            for (obstacle, radius) in &obstacles {
                let offset = flat(spot - *obstacle);
                let reach = radius + margin;
                if offset.length() < reach {
                    let direction = offset.normalize_or(glam::Vec2::X);
                    spot.x = obstacle.x + direction.x * (reach + 0.001);
                    spot.z = obstacle.z + direction.y * (reach + 0.001);
                    spot = spot.clamp(minimum, maximum);
                }
            }
        }
        // Closely packed furniture can trap a projection against a wall.
        if obstacles
            .iter()
            .any(|(obstacle, radius)| flat(spot - *obstacle).length() < radius + margin)
        {
            spot = (0..CELLS)
                .map(center)
                .filter(|spot| spot.cmpge(minimum).all() && spot.cmple(maximum).all())
                .filter(|spot| {
                    obstacles.iter().all(|(obstacle, radius)| {
                        flat(*spot - *obstacle).length() >= radius + margin
                    })
                })
                .min_by(|first, second| {
                    first
                        .distance_squared(position)
                        .total_cmp(&second.distance_squared(position))
                })
                .unwrap_or(spot);
        }
        spot
    }

    pub(super) fn hear_hunt(&mut self, text: &str, korean: bool) -> Option<Heard> {
        if self.hunt.as_ref()?.phase != Phase::Searching {
            return None;
        }
        let hint = hint(text)?;
        self.record(EventKind::Heard(text.to_owned()));
        self.hunt.as_mut()?.apply_hint(hint, self.ghost.position);
        self.record(EventKind::TreasureHint(hint));
        let (english, korean_line) = match hint {
            Hint::Warmer => ("Warmer? This way!", "따뜻해? 이쪽이구나!"),
            Hint::Hot => ("So close… I'll look carefully!", "가까워? 잘 찾아볼게!"),
            Hint::Colder => (
                "Colder? I'll try another way.",
                "차가워? 다른 쪽으로 가 볼게.",
            ),
            Hint::Cold => ("Far away… over there, then!", "멀구나… 그럼 저쪽으로!"),
        };
        self.say(if korean { korean_line } else { english });
        Some(Heard {
            tone: None,
            understanding: Understanding::Hint {
                warmer: hint.warmer(),
            },
            named: false,
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    fn hunt_sees(&self, position: Vec3) -> bool {
        let eye = self.ghost.position + Vec3::Y * 0.1;
        let offset = position - eye;
        offset.length_squared() <= 4.0
            && offset
                .normalize_or(self.ghost.heading)
                .dot(self.ghost.heading)
                >= 1.05_f32.cos()
            && self.blocker(eye, position, Thing::Treasure).is_none()
    }

    fn scan_hunt(&self, hunt: &mut Hunt) {
        for (index, belief) in hunt.belief.iter_mut().enumerate() {
            if *belief > 0.0 && self.hunt_sees(center(index)) {
                *belief = (*belief * 0.08).max(1e-12);
            }
        }
        hunt.normalize();
    }

    pub(super) fn seek_treasure(&mut self) -> Option<Vec3> {
        let mut hunt = self.hunt.take()?;
        self.ghost.attention = None;
        let elapsed = self.time - hunt.hidden_at;
        let position = self.ghost.position;
        let mut look = hunt.position;
        match hunt.phase {
            Phase::Counting => {}
            Phase::Searching => {
                if self.time >= hunt.next_scan && elapsed < 240.0 {
                    hunt.next_scan += 0.25;
                    if self.hunt_sees(hunt.position) {
                        hunt.seconds = elapsed.ceil() as u32;
                        hunt.phase = Phase::Swooping;
                        self.record(EventKind::SpottedTreasure);
                        self.say(self.spoken("Oh!", "앗!"));
                    } else {
                        self.scan_hunt(&mut hunt);
                    }
                }
                if hunt.phase == Phase::Searching {
                    if elapsed >= 240.0 {
                        hunt.gave_up = true;
                        hunt.phase = Phase::Revealing(self.time);
                        self.say(self.spoken(
                            "A little help? Oh… there it is!",
                            "조금만 도와줄래? 아… 저기 있네!",
                        ));
                    } else if elapsed >= 150.0 && !hunt.asked {
                        hunt.asked = true;
                        self.say(self.spoken("Am I warm?", "나 가까워?"));
                        self.record(EventKind::AskedTreasureHint);
                    }
                    if self.time - hunt.target_at >= 4.0
                        || position.distance(self.ghost.plan.target) < 0.2
                    {
                        hunt.target = hunt.choose_target(position);
                        hunt.target_at = self.time;
                    }
                    look = center(hunt.target);
                    self.ghost.plan.target = Vec3::new(look.x, 0.6, look.z);
                }
            }
            Phase::Revealing(started) => {
                if self.time - started >= 3.0 {
                    hunt.seconds = elapsed.ceil() as u32;
                    hunt.phase = Phase::Swooping;
                }
            }
            Phase::Swooping => {
                self.ghost.plan.target = hunt.position + Vec3::Y * 0.43;
                if position.distance(self.ghost.plan.target) < 0.24 {
                    hunt.phase = Phase::Carrying;
                }
            }
            Phase::Carrying => {
                self.ghost.plan.target = Vec3::new(0.0, 0.9, ROOM_MAX.z - 0.45);
                look = self.ghost.plan.target + Vec3::Z * 3.0;
                if position.distance(self.ghost.plan.target) < 0.25 {
                    hunt.phase = Phase::Showing(self.time);
                    self.ghost.needs.change(0.0, 0.16, 0.08);
                    self.suggest_mood(super::Mood::Excited, 6.0);
                    self.record(EventKind::FoundTreasure {
                        seconds: hunt.seconds,
                        hints: hunt.hints,
                        gave_up: hunt.gave_up,
                    });
                    self.say(if hunt.gave_up {
                        self.spoken("Hehe, you hid it well. Here!", "헤헤, 잘 숨겼네. 여기!")
                    } else {
                        self.spoken("I found your treasure! Here!", "네 보물 찾았어! 여기!")
                    });
                }
            }
            Phase::Showing(started) => {
                look = self.ghost.plan.target + Vec3::Z * 3.0;
                if self.time - started >= 4.0 {
                    self.ghost.plan.done = true;
                    return Some(look);
                }
            }
        }
        self.hunt = Some(hunt);
        Some(look)
    }

    pub fn treasure(&self) -> Option<(Vec3, f32)> {
        let hunt = self.hunt.as_ref()?;
        let brightness = match hunt.phase {
            Phase::Counting => return None,
            Phase::Revealing(_) => 2.5,
            Phase::Showing(started) => (1.0 - ((self.time - started) / 4.0) as f32).max(0.0),
            _ => 1.0,
        };
        let position = if matches!(hunt.phase, Phase::Carrying | Phase::Showing(_)) {
            self.ghost.position - Vec3::Y * 0.43
        } else {
            hunt.position
        };
        Some((position, brightness))
    }

    pub fn hunt_words(&self) -> String {
        match self.hunt.as_ref() {
            Some(hunt) if hunt.phase == Phase::Counting => {
                "you are counting with your eyes closed while the viewer hides a treasure"
                    .to_owned()
            }
            Some(hunt) if hunt.phase == Phase::Searching => format!(
                "you are searching for the treasure the viewer hid{}",
                hunt.last_hint.map_or(String::new(), |hint| format!(
                    "; they said you are {}",
                    hint.words()
                ))
            ),
            Some(_) => {
                "you spotted the viewer's treasure and are bringing it back to show them".to_owned()
            }
            None => "no treasure hunt just now".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn searching(seed: u64, start: Vec3, spot: Vec3) -> World {
        let mut world = World::new(seed, 12.0);
        world.ghost.position = start;
        world.start_hiding().unwrap();
        assert!(world.hide_treasure(spot));
        world
    }

    fn finish(world: &mut World, hints: bool) -> u32 {
        for _ in 0..2800 {
            if hints
                && let Some(hunt) = &world.hunt
                && hunt.phase == Phase::Searching
                && hunt.hints < 4
                && world.time - hunt.hidden_at >= f64::from(hunt.hints + 1) * 4.0
            {
                let distance = flat(world.ghost.position - hunt.position).length();
                let earlier = flat(hunt.previous_hint - hunt.position).length();
                world.hear(if distance < 0.8 {
                    "hot"
                } else if distance < earlier {
                    "warmer"
                } else {
                    "colder"
                });
            }
            world.step(0.1);
            if world.hunt.is_none() {
                break;
            }
        }
        assert_eq!(
            world.life.total.treasures_found,
            1,
            "seed search failed: time {}, phase {:?}, position {:?}, target {:?}",
            world.time,
            world.hunt.as_ref().map(|hunt| hunt.phase),
            world.ghost.position,
            world.ghost.plan.target
        );
        world.life.total.fastest_treasure.unwrap()
    }

    #[test]
    fn the_window_waits_for_the_hunt_to_end() {
        let mut world = World::new(1, 12.0);
        world.start_hiding().unwrap();
        world.hide_treasure(glam::Vec3::new(-1.8, 0.0, -1.2));
        assert!(!world.ask_window(true));
        assert_eq!(world.ghost.plan.activity, Activity::Seek);
    }

    #[test]
    fn hiding_closes_earlier_answer_windows_and_a_lesson_waits_for_the_hunt() {
        let mut world = World::new(1, 12.0);
        world.rhythm.reply_until = Some(world.time + 100.0);
        world.start_hiding().unwrap();
        assert!(!world.awaiting_break_reply());
        world.hide_treasure(glam::Vec3::new(-1.8, 0.0, -1.2));
        world.hear("do re mi fa sol");
        assert_eq!(world.ghost.plan.activity, Activity::Seek);
        world.cancel_hunt();
        world.step(0.1);
        world.step(0.1);
        assert_eq!(world.ghost.plan.activity, Activity::Music);
    }

    #[test]
    fn hints_preserve_nearer_farther_and_radius_beliefs_and_normalize() {
        let world = World::new(1, 12.0);
        for hint in [Hint::Warmer, Hint::Colder, Hint::Hot, Hint::Cold] {
            let mut hunt = Hunt::new(&world);
            hunt.belief.fill(1.0 / CELLS as f32);
            hunt.previous_hint = Vec3::new(-1.0, 0.6, 0.0);
            let position = Vec3::new(1.0, 0.6, 0.0);
            let before = hunt.belief;
            hunt.apply_hint(hint, position);
            let mut boosted = Vec::new();
            let mut damped = Vec::new();
            for (index, belief) in hunt.belief.iter().enumerate() {
                let distance = flat(center(index) - position).length();
                let near = match hint {
                    Hint::Warmer | Hint::Colder => {
                        distance < flat(center(index) - Vec3::new(-1.0, 0.6, 0.0)).length()
                    }
                    _ => distance < 0.8,
                };
                if near == hint.warmer() {
                    boosted.push(belief / before[index]);
                } else {
                    damped.push(belief / before[index]);
                }
            }
            assert!(
                boosted
                    .iter()
                    .all(|boost| damped.iter().all(|damp| boost > damp))
            );
            for _ in 0..1000 {
                hunt.apply_hint(hint, position);
            }
            assert!(
                hunt.belief
                    .iter()
                    .all(|belief| belief.is_finite() && *belief > 0.0)
            );
            assert!((hunt.belief.iter().sum::<f32>() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn furniture_blocks_search_evidence_as_well_as_the_treasure() {
        let mut world = World::new(1, 12.0);
        world.ghost.position = Vec3::new(-1.3, 0.6, 1.2);
        let hidden = (0..CELLS)
            .find(|index| {
                let spot = center(*index);
                world.blocker(world.ghost.position + Vec3::Y * 0.1, spot, Thing::Treasure)
                    == Some(Thing::Cushion)
            })
            .unwrap();
        world.ghost.heading = (center(hidden) - world.ghost.position - Vec3::Y * 0.1).normalize();
        let mut hunt = Hunt::new(&world);
        hunt.belief.fill(1.0 / CELLS as f32);
        let before = hunt.belief[hidden];
        assert!(!world.hunt_sees(center(hidden)));
        world.scan_hunt(&mut hunt);
        assert!(hunt.belief[hidden] >= before);
        assert!(hunt.belief.iter().any(|belief| *belief < before));
    }

    #[test]
    fn unhindered_search_finds_treasure_behind_the_lamp_from_all_room_sides() {
        for seed in [1, 7, 17] {
            for start in [
                Vec3::new(-1.5, 1.1, 1.0),
                Vec3::new(1.5, 1.1, 1.0),
                Vec3::new(1.5, 1.1, -1.0),
                Vec3::new(-1.5, 1.1, -1.0),
            ] {
                let mut world = searching(seed, start, Vec3::new(-1.55, 0.0, -1.3));
                assert!(finish(&mut world, false) <= 240);
            }
        }
    }

    #[test]
    fn accurate_hints_shorten_searches_on_average() {
        let mut guided = 0;
        let mut alone = 0;
        for seed in 1..=24 {
            let mut random = super::super::Random::new(seed);
            let spot = Vec3::new(random.range(-1.8, 1.8), 0.0, random.range(-1.2, 1.2));
            let start = Vec3::new(random.range(-1.7, 1.7), 1.1, random.range(-1.1, 1.1));
            alone += finish(&mut searching(seed, start, spot), false);
            guided += finish(&mut searching(seed, start, spot), true);
        }
        println!(
            "paired treasure searches: guided {guided} seconds, alone {alone} seconds over twenty-four seeds"
        );
        assert!(guided < alone, "guided {guided}, alone {alone}");
    }

    #[test]
    fn hiding_snaps_out_of_every_obstacle_and_inside_the_walls() {
        let world = World::new(1, 12.0);
        for position in world
            .obstacles()
            .into_iter()
            .map(|(position, _)| position)
            .chain([ROOM_MIN, Vec3::new(ROOM_MAX.x, 0.0, ROOM_MAX.z)])
        {
            let spot = world.treasure_spot(position);
            assert!(spot.is_finite());
            assert_eq!(spot.y, RADIUS);
            assert!(spot.x >= ROOM_MIN.x + RADIUS + 0.039);
            assert!(spot.z >= ROOM_MIN.z + RADIUS + 0.039);
            assert!(spot.x <= ROOM_MAX.x - RADIUS - 0.039);
            assert!(spot.z <= ROOM_MAX.z - RADIUS - 0.039);
            for (obstacle, radius) in world.obstacles() {
                assert!(flat(spot - obstacle).length() >= radius + RADIUS + 0.039);
            }
            assert_eq!(spot, world.treasure_spot(position));
        }
    }

    #[test]
    fn hints_recognize_both_languages_only_during_a_search_and_teach_nothing() {
        for (text, expected) in [
            ("Warmer!", Hint::Warmer),
            ("getting warmer", Hint::Warmer),
            ("warm", Hint::Warmer),
            ("따뜻해", Hint::Warmer),
            ("따뜻해져", Hint::Warmer),
            ("점점 따뜻해", Hint::Warmer),
            ("hot", Hint::Hot),
            ("hotter", Hint::Hot),
            ("very warm", Hint::Hot),
            ("you're close", Hint::Hot),
            ("뜨거워", Hint::Hot),
            ("가까워", Hint::Hot),
            ("colder", Hint::Colder),
            ("getting colder", Hint::Colder),
            ("cool", Hint::Colder),
            ("차가워", Hint::Colder),
            ("차가워져", Hint::Colder),
            ("점점 차가워", Hint::Colder),
            ("cold", Hint::Cold),
            ("freezing", Hint::Cold),
            ("far", Hint::Cold),
            ("추워", Hint::Cold),
            ("멀어", Hint::Cold),
        ] {
            assert_eq!(hint(text), Some(expected));
            let mut world = searching(1, Vec3::new(0.0, 1.0, 1.0), Vec3::new(-1.8, 0.0, -1.2));
            let heard = world.hear(text);
            assert_eq!(
                heard.understanding,
                Understanding::Hint {
                    warmer: expected.warmer()
                }
            );
            assert_eq!(world.life.today.feelings_heard, 0);
            assert_eq!(world.life.today.treasure_hints, 1);
            assert_eq!(world.ghost.plan.activity, Activity::Seek);
            assert_eq!(
                world
                    .ghost
                    .speech
                    .as_ref()
                    .unwrap()
                    .text
                    .chars()
                    .any(|letter| ('가'..='힣').contains(&letter)),
                heard.korean
            );
            let mut outside = World::new(1, 12.0);
            assert!(!matches!(
                outside.hear(text).understanding,
                Understanding::Hint { .. }
            ));
        }
        assert_eq!(hint("I'm cold"), None);
        assert!(!matches!(
            World::new(1, 12.0).hear("I'm cold").understanding,
            Understanding::Hint { .. }
        ));
        assert_eq!(hint("no, warmer"), None);
        assert_eq!(hint("good job"), None);
    }

    #[test]
    fn counting_keeps_eyes_off_the_room_and_cancellation_removes_the_game() {
        let mut world = World::new(1, 12.0);
        world.start_hiding().unwrap();
        let position = world.ghost.position;
        world.step(10.0);
        assert_eq!(world.ghost.position, position);
        assert!(world.sight().in_view.is_empty());
        assert_eq!(world.ghost.heading, -Vec3::Z);
        assert!(world.treasure().is_none());
        world.cancel_hunt();
        assert!(world.hunt.is_none());
        assert_eq!(world.life.total.hunts, 0);
    }

    #[test]
    fn a_find_counts_once_warms_the_bond_and_marks_its_first_in_words() {
        let mut world = searching(7, Vec3::new(0.0, 1.1, 1.0), Vec3::new(-1.8, 0.0, -1.2));
        let bond = world.life.bond;
        finish(&mut world, false);
        assert_eq!(world.life.today.hunts, 1);
        assert_eq!(world.life.total.treasures_found, 1);
        assert!(world.life.bond > bond);
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "I found the treasure you hid for the first time.")
        );
        assert!(
            world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::FoundTreasure { .. }))
                .all(|event| !event
                    .kind
                    .words()
                    .contains(|character: char| character.is_ascii_digit()))
        );
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::FoundTreasure { .. }))
                .count(),
            1
        );
        world.step(30.0);
        assert_eq!(world.life.total.treasures_found, 1);
    }

    #[test]
    fn unanswered_search_asks_once_then_reveals_without_counting_a_find() {
        let mut world = searching(1, Vec3::new(0.0, 1.1, 1.0), Vec3::new(-1.8, 0.0, -1.2));
        world.time = 151.0;
        world.hunt.as_mut().unwrap().next_scan = f64::INFINITY;
        world.seek_treasure();
        world.seek_treasure();
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::AskedTreasureHint)
                .count(),
            1
        );
        world.time = 241.0;
        world.seek_treasure();
        assert!(matches!(
            world.hunt.as_ref().unwrap().phase,
            Phase::Revealing(_)
        ));
        for _ in 0..600 {
            world.step(0.1);
        }
        assert!(world.hunt.is_none());
        assert_eq!(world.life.total.treasures_found, 0);
        assert!(
            world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::FoundTreasure { gave_up: true, .. }))
        );
    }

    #[test]
    fn saved_tallies_load_without_hunt_fields_and_drop_damaged_records() {
        let tally: super::super::life::Tally = serde_json::from_str("{}").unwrap();
        assert_eq!(tally.hunts, 0);
        let mut life = super::super::Life::default();
        life.total.hunts = 1;
        life.total.treasures_found = 20;
        life.total.fastest_treasure = Some(999);
        let life = life.sanitized();
        assert_eq!(life.total.treasures_found, 1);
        assert_eq!(life.total.fastest_treasure, None);
    }

    #[test]
    fn the_same_seed_and_hints_repeat_the_search_and_found_time() {
        let start = Vec3::new(1.5, 1.1, 1.0);
        let spot = Vec3::new(-1.8, 0.0, -1.2);
        let mut first = searching(17, start, spot);
        let mut second = searching(17, start, spot);
        assert_eq!(finish(&mut first, true), finish(&mut second, true));
        assert_eq!(first.ghost.position, second.ghost.position);
        assert_eq!(first.life, second.life);
        assert_eq!(first.events, second.events);
    }
}
