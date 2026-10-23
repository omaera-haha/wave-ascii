use std::f32::consts::TAU;
use std::sync::Arc;

use glam::{Vec2, Vec3};

use super::doodle::Path;
use super::{Activity, EventKind, Heard, Keepsake, Mood, ROOM_MAX, Tone, Understanding, World};

const ENGLISH: &[&str] = &[
    "BALL", "MOON", "STAR", "BIRD", "LAMP", "PLANT", "SEED", "TOWER", "CLOUD", "HEART", "GHOST",
    "RAIN", "SNOW", "LIGHT", "DREAM",
];
const KOREAN: &[&str] = &[
    "하늘",
    "구름",
    "무지개",
    "눈사람",
    "고양이",
    "반딧불",
    "도토리",
    "풍선",
    "유령",
    "나무",
    "화분",
    "새싹",
    "마음",
    "별빛",
    "달빛",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Guess {
    Correct,
    Wrong,
    Repeated,
    Solved,
}

#[derive(Clone, Debug, PartialEq)]
struct Ink {
    path: Arc<Path>,
    minimum: Vec2,
    maximum: Vec2,
    started: f64,
    duration: f64,
    petal: Option<usize>,
}

impl Ink {
    fn new(strokes: Vec<Vec<Vec2>>, started: f64, duration: f64, petal: Option<usize>) -> Self {
        let (minimum, maximum) = strokes.iter().flatten().fold(
            (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
            |(minimum, maximum), &point| (minimum.min(point), maximum.max(point)),
        );
        Self {
            path: Arc::new(Path::new(strokes)),
            minimum,
            maximum,
            started,
            duration,
            petal,
        }
    }

    fn progress(&self, time: f64) -> f32 {
        ((time - self.started) / self.duration).clamp(0.0, 1.0) as f32
    }

    fn distance(&self, point: Vec2, time: f64) -> f32 {
        let margin = Vec2::splat(0.04);
        if point.cmplt(self.minimum - margin).any() || point.cmpgt(self.maximum + margin).any() {
            return f32::INFINITY;
        }
        self.path.distance(point, self.progress(time))
    }
}

/// A word and a flower in the fog, kept only until the board fades.
#[derive(Clone, Debug, PartialEq)]
pub struct Puzzle {
    pub word: String,
    pub korean: bool,
    pub center: Vec2,
    pub wrong: Vec<char>,
    pub ended: Option<f64>,
    pub solved: bool,
    letters: Vec<char>,
    shown: Vec<bool>,
    guesses: Vec<char>,
    ink: Vec<Ink>,
    board_started: Option<f64>,
    board_duration: f64,
    writing_until: f64,
    last_heard: f64,
    cancelled: bool,
    shaken_at: Option<f64>,
    pub worried_at: Option<f64>,
}

impl Puzzle {
    pub const SILENCE: f64 = 120.0;
    pub const EXTENT: Vec2 = Vec2::new(1.65, 0.55);

    fn new(word: String, korean: bool, time: f64) -> Self {
        let letters: Vec<char> = word.chars().collect();
        let mut puzzle = Self {
            shown: vec![false; letters.len()],
            letters,
            word,
            korean,
            center: Vec2::new(0.0, 1.3),
            wrong: Vec::new(),
            guesses: Vec::new(),
            ink: Vec::new(),
            board_started: None,
            board_duration: 0.0,
            writing_until: 0.0,
            last_heard: time,
            ended: None,
            solved: false,
            cancelled: false,
            shaken_at: None,
            worried_at: None,
        };
        for index in 0..puzzle.letters.len() {
            let position = puzzle.slot(index) + Vec2::new(0.0, -0.04);
            let width = if korean { 0.24 } else { 0.18 };
            puzzle.stroke(vec![position, position + Vec2::X * width], None, 0.14);
        }
        let flower = Vec2::new(1.3, 0.12);
        puzzle.stroke(vec![flower, flower + Vec2::new(0.0, -0.42)], None, 0.18);
        puzzle.stroke(circle(flower, Vec2::splat(0.045)), None, 0.18);
        for petal in 0..6 {
            let angle = TAU * petal as f32 / 6.0;
            let direction = Vec2::from_angle(angle);
            let across = Vec2::new(-direction.y, direction.x);
            let points = (0..=20)
                .map(|index| {
                    let phase = TAU * index as f32 / 20.0;
                    flower
                        + direction * (0.12 + 0.095 * phase.cos())
                        + across * (0.065 * phase.sin())
                })
                .collect();
            puzzle.stroke(points, Some(petal), 0.18);
        }
        puzzle.board_duration = puzzle.writing_until;
        puzzle
    }

    fn stroke(&mut self, points: Vec<Vec2>, petal: Option<usize>, duration: f64) {
        self.ink
            .push(Ink::new(vec![points], self.writing_until, duration, petal));
        self.writing_until += duration;
    }

    fn slot(&self, index: usize) -> Vec2 {
        let spacing = if self.korean { 0.32 } else { 0.25 };
        let width = if self.korean { 0.24 } else { 0.18 };
        Vec2::new(
            (index as f32 - (self.letters.len() - 1) as f32 * 0.5) * spacing - width * 0.5,
            0.0,
        )
    }

    fn glyph(&mut self, character: char, position: Vec2, size: f32, time: f64) {
        let strokes: Vec<Vec<Vec2>> = super::letters::strokes(character)
            .into_iter()
            .map(|stroke| {
                stroke
                    .into_iter()
                    .map(|(horizontal, vertical)| position + Vec2::new(horizontal, vertical) * size)
                    .collect()
            })
            .collect();
        let duration = (strokes.len() as f64 * 0.09).max(0.25);
        let started = (time - self.board_started.unwrap()).max(self.writing_until);
        self.ink.push(Ink::new(strokes, started, duration, None));
        self.writing_until = started + duration;
    }

    pub fn petals(&self) -> usize {
        6 - self.wrong.len()
    }

    pub fn shown(&self) -> usize {
        self.shown.iter().filter(|&&shown| shown).count()
    }

    pub(super) fn keeps_viewer_present(&self, time: f64) -> bool {
        self.ended.is_none() && time - self.last_heard < Self::SILENCE + 5.0
    }

    pub fn waiting(&self, time: f64) -> bool {
        self.ended.is_none()
            && self
                .board_started
                .is_some_and(|started| time >= started + self.writing_until)
    }

    fn board_ready(&self, time: f64) -> bool {
        self.board_started
            .is_some_and(|started| time >= started + self.board_duration)
    }

    fn accepts(&self, text: &str) -> bool {
        let text = text.trim().to_ascii_uppercase();
        text == self.word || (text.chars().count() == 1 && valid_script(&text, self.korean))
    }

    fn reveal(&mut self, character: Option<char>, time: f64) {
        for index in 0..self.letters.len() {
            if !self.shown[index]
                && character.is_none_or(|character| self.letters[index] == character)
            {
                self.shown[index] = true;
                self.glyph(
                    self.letters[index],
                    self.slot(index),
                    if self.korean { 0.24 } else { 0.3 },
                    time,
                );
            }
        }
    }

    fn guess(&mut self, text: &str, time: f64) -> Guess {
        self.last_heard = time;
        let text = text.trim().to_ascii_uppercase();
        if text == self.word {
            self.reveal(None, time);
            return Guess::Solved;
        }
        let character = text.chars().next().unwrap();
        if self.guesses.contains(&character) {
            return Guess::Repeated;
        }
        self.guesses.push(character);
        if self.letters.contains(&character) {
            self.reveal(Some(character), time);
            if self.shown.iter().all(|&shown| shown) {
                Guess::Solved
            } else {
                Guess::Correct
            }
        } else {
            let position = Vec2::new(-0.8 + self.wrong.len() as f32 * 0.17, -0.3);
            self.glyph(character, position, 0.12, time);
            self.wrong.push(character);
            Guess::Wrong
        }
    }

    fn ink_time(&self, time: f64) -> f64 {
        let time = if self.cancelled {
            time.min(self.ended.unwrap())
        } else {
            time
        };
        time - self.board_started.unwrap_or(time)
    }

    pub fn opacity(&self, time: f64) -> f32 {
        self.ended.map_or(1.0, |ended| {
            let finished = if self.cancelled {
                ended
            } else {
                ended.max(self.board_started.unwrap_or(ended) + self.writing_until)
            };
            (1.0 - (time - finished - 4.0).max(0.0) / 30.0).clamp(0.0, 1.0) as f32
        })
    }

    pub fn distance(&self, point: Vec2, time: f64) -> f32 {
        let offset = point - self.center;
        if !offset.is_finite() || offset.abs().cmpgt(Self::EXTENT).any() {
            return f32::INFINITY;
        }
        let drawing_time = self.ink_time(time);
        self.ink
            .iter()
            .filter(|ink| ink.petal.is_none_or(|petal| petal < self.petals()))
            .map(|ink| ink.distance(offset, drawing_time))
            .fold(f32::INFINITY, f32::min)
    }

    fn pen(&self, time: f64) -> Option<Vec2> {
        let drawing_time = self.ink_time(time);
        self.ink
            .iter()
            .find(|ink| (ink.started..ink.started + ink.duration).contains(&drawing_time))
            .map(|ink| self.center + ink.path.point_at(ink.progress(drawing_time)))
    }

    pub fn words(&self, time: f64) -> String {
        format!(
            "a word-guessing game in the fog; {}; {} of {} {} shown and {} petals left",
            if self.waiting(time) {
                "waiting for the viewer to guess"
            } else {
                "you are writing on the board"
            },
            super::friend::count_words(self.shown() as u32),
            super::friend::count_words(self.letters.len() as u32),
            if self.korean { "syllables" } else { "letters" },
            super::friend::count_words(self.petals() as u32)
        )
    }
}

fn circle(center: Vec2, radius: Vec2) -> Vec<Vec2> {
    (0..=20)
        .map(|index| center + Vec2::from_angle(TAU * index as f32 / 20.0) * radius)
        .collect()
}

fn valid_script(word: &str, korean: bool) -> bool {
    word.chars().all(|character| {
        if korean {
            ('가'..='힣').contains(&character)
        } else {
            character.is_ascii_alphabetic()
        }
    })
}

fn valid_word(word: &str, korean: bool) -> bool {
    let length = word.chars().count();
    (if korean { 2..=4 } else { 3..=8 }).contains(&length) && valid_script(word, korean)
}

pub(super) fn requested(text: &str) -> bool {
    let words = text
        .to_lowercase()
        .split(|character: char| !character.is_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    matches!(
        words.as_slice(),
        ["word", "game"]
            | ["let", "s", "play", "a", "word", "game"]
            | ["lets", "play", "a", "word", "game"]
            | ["play", "a", "word", "game"]
            | ["guess", "my", "word"]
            | ["단어", "게임", "하자"]
            | ["단어", "게임"]
            | ["낱말", "맞추기"]
            | ["낱말", "맞추기", "하자"]
    )
}

impl World {
    fn puzzle_say(&mut self, english: &str, korean: &str) {
        self.say(if self.latest_korean || self.speaks_korean() {
            korean
        } else {
            english
        });
    }

    fn puzzle_words(&self, korean: bool) -> Vec<String> {
        let mut words: Vec<String> = (if korean { KOREAN } else { ENGLISH })
            .iter()
            .map(|word| (*word).to_owned())
            .collect();
        for &keepsake in &self.life.keepsakes {
            let word = match (keepsake, korean) {
                (Keepsake::Feather, false) => "FEATHER",
                (Keepsake::Petal, false) => "PETAL",
                (Keepsake::Acorn, false) => "ACORN",
                (Keepsake::Leaf, false) => "LEAF",
                (Keepsake::Berry, false) => "BERRY",
                (Keepsake::Pebble, false) => "PEBBLE",
                (Keepsake::Feather, true) => "깃털",
                (Keepsake::Petal, true) => "꽃잎",
                (Keepsake::Acorn, true) => "도토리",
                (Keepsake::Leaf, true) => "나뭇잎",
                (Keepsake::Berry, true) => "열매",
                (Keepsake::Pebble, true) => "조약돌",
            };
            words.push(word.to_owned());
        }
        words.extend(
            self.life
                .pictures
                .iter()
                .map(|picture| picture.name.to_ascii_uppercase()),
        );
        words.extend(
            self.life
                .friend
                .iter()
                .map(|friend| friend.name.to_ascii_uppercase()),
        );
        words.push(self.ghost.name.to_ascii_uppercase());
        words.retain(|word| valid_word(word, korean));
        let mut unique = Vec::new();
        for word in words {
            if !unique.contains(&word) {
                unique.push(word);
            }
        }
        unique
    }

    pub fn puzzle_active(&self) -> bool {
        self.puzzle
            .as_ref()
            .is_some_and(|puzzle| puzzle.ended.is_none())
    }

    pub(super) fn begin_puzzle(&mut self) -> Vec3 {
        self.game = None;
        let korean = self.speaks_korean() || self.latest_korean;
        let words = self.puzzle_words(korean);
        let index = (self.random.unit() * words.len() as f32) as usize;
        let puzzle = Puzzle::new(words[index.min(words.len() - 1)].clone(), korean, self.time);
        self.doodles.retain(|doodle| {
            let reach = doodle.picture.extent() * doodle.size;
            ((doodle.center - puzzle.center).abs() - reach - Puzzle::EXTENT).max_element() >= 0.0
        });
        let target = puzzle.center.extend(ROOM_MAX.z - 0.3);
        self.puzzle = Some(puzzle);
        self.puzzle_say(
            "I have a word! Type a letter, or try the whole word.",
            "낱말을 생각했어! 한 글자씩, 아니면 낱말을 맞혀 봐.",
        );
        target
    }

    pub(super) fn abandon_puzzle(&mut self) {
        // What the house shares is the host's to end.
        if self.companion {
            return;
        }
        if let Some(puzzle) = &mut self.puzzle
            && puzzle.ended.is_none()
        {
            puzzle.ended = Some(self.time);
            puzzle.cancelled = true;
            if self.ghost.plan.activity == Activity::Puzzle {
                self.ghost.plan.done = true;
            }
        }
    }

    pub(super) fn hear_puzzle(&mut self, text: &str, korean: bool) -> Option<Heard> {
        if requested(text) {
            self.record(EventKind::Heard(text.to_owned()));
            if self.ghost.asleep >= 0.3 || self.focus.is_some() || self.telling.is_some() {
                self.puzzle_say("Let's play a little later.", "조금 이따 하자.");
            } else {
                self.notice_viewer();
                self.begin(Activity::Puzzle);
            }
        } else {
            if !self.puzzle_active() {
                return None;
            }
            self.notice_viewer();
            let puzzle = self.puzzle.as_mut().unwrap();
            puzzle.last_heard = self.time;
            if !puzzle.accepts(text) {
                if super::vocabulary::read(text, &self.ghost.name).tone == Some(Tone::Farewell) {
                    self.abandon_puzzle();
                }
                return None;
            }
            self.record(EventKind::Heard(text.to_owned()));
            let mut puzzle = self.puzzle.take().unwrap();
            if puzzle.board_ready(self.time) {
                match puzzle.guess(text, self.time) {
                    Guess::Repeated => {
                        puzzle.shaken_at = Some(self.time);
                        self.puzzle_say("We tried that one already.", "그 글자는 벌써 해 봤어.");
                    }
                    Guess::Solved => self.end_puzzle(&mut puzzle, true, false),
                    Guess::Correct => {
                        puzzle.worried_at = None;
                        self.suggest_mood(Mood::Excited, 3.0);
                        self.puzzle_say("Yes! That fits!", "맞아! 그 글자가 있어!");
                    }
                    Guess::Wrong => {
                        if puzzle.petals() == 0 {
                            self.end_puzzle(&mut puzzle, false, true);
                        } else if puzzle.petals() <= 2 {
                            puzzle.worried_at = Some(self.time);
                            self.suggest_mood(Mood::Curious, 3.0);
                            self.puzzle_say(
                                "Oh! My flower is getting shy.",
                                "어머! 꽃잎이 얼마 안 남았네.",
                            );
                        } else if self.good_sport() {
                            self.suggest_mood(Mood::Cheerful, 3.0);
                            self.puzzle_say(
                                "Heehee. Another petal floats away.",
                                "히히. 꽃잎 하나가 날아갔네.",
                            );
                        } else {
                            self.puzzle_say("Try another letter?", "다른 글자도 해 볼래?");
                        }
                    }
                }
            } else {
                self.puzzle_say("Let me finish the flower first.", "꽃을 먼저 다 그릴게.");
            }
            self.puzzle = Some(puzzle);
        }
        Some(Heard {
            tone: None,
            understanding: Understanding::WordGame,
            named: false,
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    fn end_puzzle(&mut self, puzzle: &mut Puzzle, solved: bool, reveal: bool) {
        if puzzle.ended.is_some() {
            return;
        }
        if reveal {
            puzzle.reveal(None, self.time);
        }
        puzzle.ended = Some(self.time);
        puzzle.solved = solved;
        // A board nobody guessed at was no game played together.
        let unanswered = !solved && !reveal && puzzle.guesses.is_empty();
        if unanswered {
            self.invitation_ignored();
        } else {
            self.record(EventKind::WordGame {
                word: puzzle.word.clone(),
                solved,
            });
        }
        if solved {
            self.suggest_mood(Mood::Excited, 6.0);
            self.ghost.needs.change(0.0, 0.15, 0.08);
            self.puzzle_say("You found my word!", "내 낱말을 맞혔어!");
        } else if reveal {
            self.suggest_mood(Mood::Cheerful, 4.0);
            self.puzzle_say("Oops. I'll write it for you.", "에헤. 내가 써 줄게.");
        } else {
            self.puzzle_say("We can try again another time.", "다음에 또 해 보자.");
        }
    }

    pub(super) fn play_puzzle(&mut self) -> Option<Vec3> {
        let mut puzzle = self.puzzle.take()?;
        if let Some(arrived) = self.ghost.plan.arrived
            && puzzle.board_started.is_none()
        {
            puzzle.board_started = Some(arrived);
            puzzle.last_heard = arrived + puzzle.board_duration;
        }
        if puzzle.ended.is_none()
            && puzzle.board_ready(self.time)
            && self.time - puzzle.last_heard >= Puzzle::SILENCE
        {
            self.end_puzzle(&mut puzzle, false, false);
        }
        let pen = puzzle.pen(self.time);
        self.ghost.plan.target = pen
            .unwrap_or(puzzle.center)
            .extend(ROOM_MAX.z - if pen.is_some() { 0.3 } else { 0.7 });
        if let Some(shaken) = puzzle.shaken_at
            && (0.0..0.6).contains(&(self.time - shaken))
        {
            self.ghost.plan.target.x += ((self.time - shaken) as f32 * TAU * 5.0).sin() * 0.08;
        }
        if let Some(ended) = puzzle.ended {
            let finished = ended.max(puzzle.board_started.unwrap_or(ended) + puzzle.writing_until);
            self.ghost.plan.done = self.time - finished > 4.0;
        }
        let look = pen.map(|pen| pen.extend(ROOM_MAX.z)).or_else(|| {
            Some(
                self.viewer
                    .pointer
                    .unwrap_or(puzzle.center.extend(ROOM_MAX.z))
                    + Vec3::Z * 3.0,
            )
        });
        self.puzzle = Some(puzzle);
        look
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Companion, LearnedPicture, Life, ROOM_MIN};

    fn ready(word: &str) -> World {
        let mut world = World::new(37, 12.0);
        world.notice_viewer();
        world.begin(Activity::Puzzle);
        let mut puzzle = Puzzle::new(word.to_owned(), valid_script(word, true), 0.0);
        puzzle.board_started = Some(0.0);
        world.time = puzzle.board_duration + 1.0;
        puzzle.last_heard = world.time;
        world.ghost.position = puzzle.center.extend(ROOM_MAX.z - 0.7);
        world.ghost.plan.arrived = Some(0.0);
        world.puzzle = Some(puzzle);
        world.step(0.01);
        world
    }

    fn results(world: &World) -> Vec<(String, bool)> {
        world
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                EventKind::WordGame { word, solved } => Some((word.clone(), *solved)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn words_from_life_keep_only_suitable_names_in_the_boards_script() {
        let mut world = World::new(7, 12.0);
        world.ghost.name = "Willow".to_owned();
        world.life.friend = Some(Companion {
            name: "Misty".to_owned(),
            ..Companion::default()
        });
        world.life.keepsakes = vec![
            Keepsake::Feather,
            Keepsake::Petal,
            Keepsake::Acorn,
            Keepsake::Leaf,
            Keepsake::Berry,
            Keepsake::Pebble,
        ];
        for name in [
            "rabbit",
            "rabbit",
            "ox",
            "longerthanaword",
            "two words",
            "café",
            "cat2",
            "하늘",
            "구름이",
            "별",
        ] {
            world.life.pictures.push(LearnedPicture {
                name: name.to_owned(),
                strokes: Vec::new(),
            });
        }
        let english = world.puzzle_words(false);
        for word in [
            "WILLOW", "MISTY", "FEATHER", "PETAL", "ACORN", "LEAF", "BERRY", "PEBBLE", "RABBIT",
        ] {
            assert!(english.iter().any(|candidate| candidate == word));
        }
        assert!(english.iter().all(|word| valid_word(word, false)));
        assert_eq!(
            english
                .iter()
                .filter(|word| word.as_str() == "RABBIT")
                .count(),
            1
        );
        assert!(!english.iter().any(|word| word.contains('2')));
        world.ghost.name = "뽀삐".to_owned();
        let korean = world.puzzle_words(true);
        assert!(korean.iter().all(|word| valid_word(word, true)));
        assert!(korean.iter().any(|word| word == "뽀삐"));
        assert!(korean.iter().any(|word| word == "조약돌"));
        assert!(!korean.iter().any(|word| word == "별"));
        for word in ENGLISH {
            assert!(valid_word(word, false));
        }
        for word in KOREAN {
            assert!(valid_word(word, true));
        }
    }

    #[test]
    fn korean_viewers_get_korean_words_and_seeded_choices_repeat() {
        for seed in 0..32 {
            let mut first = World::new(seed, 12.0);
            first.life.korean = 1.0;
            let mut second = first.clone();
            first.begin(Activity::Puzzle);
            second.begin(Activity::Puzzle);
            assert_eq!(first.puzzle, second.puzzle);
            assert!(valid_word(&first.puzzle.as_ref().unwrap().word, true));
        }
    }

    #[test]
    fn letters_and_syllables_reveal_every_match_and_repeats_cost_nothing() {
        for (word, correct, wrong) in [("MOON", "o", "z"), ("나나나", "나", "구")] {
            let mut world = ready(word);
            world.hear(correct);
            let shown = if word == "MOON" { 2 } else { 3 };
            assert_eq!(world.puzzle.as_ref().unwrap().shown(), shown);
            if word == "MOON" {
                world.hear(correct);
                assert_eq!(world.puzzle.as_ref().unwrap().shown(), 2);
                assert_eq!(world.puzzle.as_ref().unwrap().petals(), 6);
                world.hear(wrong);
                assert_eq!(world.puzzle.as_ref().unwrap().petals(), 5);
                world.hear(wrong);
                assert_eq!(world.puzzle.as_ref().unwrap().petals(), 5);
                assert_eq!(world.puzzle.as_ref().unwrap().wrong, vec!['Z']);
            } else {
                assert_eq!(results(&world), vec![(word.to_owned(), true)]);
            }
        }
        let mut world = ready("나무나무");
        world.hear("나");
        assert_eq!(world.puzzle.as_ref().unwrap().shown(), 2);
        world.hear("구");
        world.hear("구");
        assert_eq!(world.puzzle.as_ref().unwrap().petals(), 5);
    }

    #[test]
    fn all_letters_or_the_full_word_solve_once_and_warm_the_bond() {
        for guesses in [vec!["b", "a", "l"], vec!["ball"]] {
            let mut world = ready("BALL");
            let bond = world.life.bond;
            for guess in guesses {
                world.hear(guess);
            }
            assert_eq!(results(&world), vec![("BALL".to_owned(), true)]);
            assert_eq!(world.life.total.word_games, 1);
            assert_eq!(world.life.total.words_guessed, 1);
            assert!(world.life.bond > bond);
            assert!(world.puzzle.as_ref().unwrap().solved);
            world.step(10.0);
            assert_eq!(results(&world).len(), 1);
        }
        for word in ["무지개", "GOODBYE"] {
            let mut world = ready(word);
            world.hear(word);
            assert_eq!(results(&world), vec![(word.to_owned(), true)]);
        }
    }

    #[test]
    fn six_wrong_guesses_reveal_the_word_and_count_an_unsolved_game_once() {
        let mut world = ready("MOON");
        for guess in ["a", "b", "c", "d", "e", "f"] {
            world.hear(guess);
        }
        let puzzle = world.puzzle.as_ref().unwrap();
        assert_eq!(puzzle.petals(), 0);
        assert_eq!(puzzle.shown(), 4);
        assert!(!puzzle.solved);
        assert!(puzzle.ended.is_some());
        assert_eq!(results(&world), vec![("MOON".to_owned(), false)]);
        assert_eq!(world.life.total.word_games, 1);
        assert_eq!(world.life.total.words_guessed, 0);
        world.step(12.0);
        assert_eq!(results(&world).len(), 1);
    }

    #[test]
    fn two_minutes_without_speech_end_gently_before_idle_presence_expires() {
        let mut world = ready("MOON");
        world.step(119.0);
        assert!(world.puzzle_active());
        assert!(world.viewer_present());
        world.step(1.1);
        assert!(!world.puzzle_active());
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "We can try again another time."
        );
        // Nobody guessed, so it was no game played together.
        assert!(results(&world).is_empty());
        assert_eq!(world.life.total.word_games, 0);
        assert_eq!(world.puzzle.as_ref().unwrap().shown(), 0);
        assert_eq!(world.puzzle.as_ref().unwrap().petals(), 6);
        world.step(40.0);
        assert!(world.puzzle.is_none());
        assert!(results(&world).is_empty());
    }

    #[test]
    fn a_board_nobody_guessed_at_holds_off_the_next_invitation_for_a_while() {
        let mut world = ready("MOON");
        world.ghost.needs.fun = 0.2;
        world.step(125.0);
        world.notice_viewer();
        assert!(!world.options().contains(&Activity::Puzzle));
        assert!(!world.options().contains(&Activity::Game));
        world.step(super::super::IGNORED_PAUSE as f32);
        world.notice_viewer();
        world.ghost.needs.energy = 0.9;
        world.ghost.needs.fun = 0.2;
        assert!(world.options().contains(&Activity::Puzzle));
        assert!(world.options().contains(&Activity::Game));
        // A guess, even a wrong one, makes it a game played together.
        let mut world = ready("MOON");
        world.hear("z");
        world.step(125.0);
        assert_eq!(results(&world), vec![("MOON".to_owned(), false)]);
        world.notice_viewer();
        world.ghost.needs.fun = 0.2;
        assert!(world.options().contains(&Activity::Puzzle));
    }

    #[test]
    fn an_invitation_near_the_idle_limit_keeps_its_full_wait_for_a_guess() {
        let mut world = World::new(8, 12.0);
        world.notice_viewer();
        world.step(80.0);
        world.begin(Activity::Puzzle);
        world.step(110.0);
        assert!(world.puzzle_active());
        assert!(world.viewer_present());
        world.step(30.0);
        assert!(!world.puzzle_active());
        assert!(results(&world).is_empty());
    }

    #[test]
    fn interruptions_are_silent_but_pokes_and_ordinary_messages_keep_the_game() {
        for interruption in 0..6 {
            let mut world = ready("MOON");
            match interruption {
                0 => {
                    world.hear("no");
                }
                1 => world.begin(Activity::Rest),
                2 => {
                    world.look_away(true);
                    world.step(20.1);
                }
                3 => {
                    world.hear("bye");
                }
                4 => {
                    world.ghost.asleep = 0.8;
                    world.step(0.1);
                }
                _ => world.start_focus(25),
            }
            assert!(!world.puzzle_active(), "interruption {interruption}");
            assert!(results(&world).is_empty());
            assert_eq!(world.life.total.word_games, 0);
        }
        let mut world = ready("MOON");
        world.poke();
        world.step(90.0);
        let heard = world.hear("hello there");
        assert_ne!(heard.understanding, Understanding::WordGame);
        assert!(world.puzzle_active());
        assert_eq!(world.puzzle.as_ref().unwrap().shown(), 0);
        assert_eq!(world.puzzle.as_ref().unwrap().petals(), 6);
        assert!(matches!(
            world.hear("how are you?").understanding,
            Understanding::Asked { .. }
        ));
        world.step(119.0);
        assert!(world.puzzle_active());
    }

    #[test]
    fn other_scripts_and_ordinary_words_are_heard_without_spending_petals() {
        for (word, messages) in [
            ("MOON", ["나", "hello there", "cloud"]),
            ("무지개", ["a", "ㄱ", "안녕하세요"]),
        ] {
            let mut world = ready(word);
            for message in messages {
                assert_ne!(world.hear(message).understanding, Understanding::WordGame);
                assert!(world.puzzle_active());
                assert_eq!(world.puzzle.as_ref().unwrap().petals(), 6);
                assert_eq!(world.puzzle.as_ref().unwrap().shown(), 0);
            }
        }
    }

    #[test]
    fn switching_between_fog_games_wipes_the_old_board_without_counting_it() {
        let mut world = ready("MOON");
        world.challenge();
        assert!(world.puzzle.is_none());
        assert!(results(&world).is_empty());
        world.hear("word game");
        assert!(world.puzzle_active());
        assert!(world.game.is_none());
        assert_eq!(world.life.total.games, 0);
    }

    #[test]
    fn a_seeded_world_reaches_the_board_and_waits_past_the_generic_plan_timeout() {
        let mut world = World::new(12, 12.0);
        world.hear("word game");
        world.step(35.0);
        assert!(world.puzzle.as_ref().unwrap().waiting(world.time));
        assert!(!world.ghost.plan.done);
        assert!(world.ghost.position.is_finite());
        assert!(world.ghost.position.cmpge(ROOM_MIN).all());
        let word = world.puzzle.as_ref().unwrap().word.clone();
        world.hear(&word);
        world.step(12.0);
        assert!(world.ghost.plan.done);
        assert_eq!(results(&world), vec![(word, true)]);
    }

    #[test]
    fn observations_keep_the_answer_secret_and_express_counts_in_words() {
        let mut world = ready("MOON");
        let puzzle = world.puzzle.as_ref().unwrap();
        let words = puzzle.words(world.time);
        assert!(words.contains("waiting for the viewer to guess"));
        assert!(words.contains("zero of four letters shown and six petals left"));
        assert!(!words.contains("MOON"));
        assert!(!words.chars().any(|character| character.is_ascii_digit()));
        world.hear("o");
        world.step(2.0);
        let observation = world.puzzle.as_ref().unwrap().words(world.time);
        assert!(observation.contains("two of four letters shown"));
        assert!(!observation.contains("MOON"));
    }

    #[test]
    fn old_tallies_load_and_corrupt_counts_are_clamped_and_first_solve_is_kept_once() {
        let mut life: Life = serde_json::from_str("{}").unwrap();
        assert_eq!(life.total.word_games, 0);
        assert_eq!(life.total.words_guessed, 0);
        for solved in [false, true, true] {
            life.note(&EventKind::WordGame {
                word: "MOON".to_owned(),
                solved,
            });
        }
        assert_eq!(life.total.word_games, 3);
        assert_eq!(life.today.words_guessed, 2);
        assert_eq!(
            life.milestones
                .iter()
                .filter(|entry| entry.text == "You guessed my secret word for the first time.")
                .count(),
            1
        );
        let words = life.today.words();
        assert!(words.contains("three word games"));
        assert!(!words.chars().any(|character| character.is_ascii_digit()));
        life.total.word_games = u32::MAX;
        life.total.words_guessed = u32::MAX;
        life.today.word_games = 1;
        life.today.words_guessed = u32::MAX;
        let life = life.sanitized();
        assert_eq!(life.total.word_games, 100_000_000);
        assert_eq!(life.total.words_guessed, 100_000_000);
        assert_eq!(life.today.words_guessed, 1);
    }

    #[test]
    fn board_and_glyph_strokes_remain_finite_and_inside_the_glass() {
        for word in ENGLISH
            .iter()
            .chain(KOREAN)
            .copied()
            .chain(["ABCDEFGH", "가나다라"])
        {
            let mut world = ready(word);
            world.hear(word);
            let puzzle = world.puzzle.as_ref().unwrap();
            for ink in &puzzle.ink {
                for fraction in [0.0, 0.2, 0.5, 0.9, 1.0] {
                    let point = puzzle.center + ink.path.point_at(fraction);
                    assert!(point.is_finite());
                    assert!(point.cmpge(ROOM_MIN.truncate()).all());
                    assert!(point.cmple(ROOM_MAX.truncate()).all());
                    assert!((point - puzzle.center).abs().cmple(Puzzle::EXTENT).all());
                    let distance = puzzle.distance(point, world.time + 20.0);
                    assert!(!distance.is_nan() && distance >= 0.0);
                }
            }
            assert!(puzzle.distance(Vec2::splat(50.0), world.time).is_infinite());
        }
    }
}
