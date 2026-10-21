//! Incoming letters and the small, deterministic understanding behind replies.
use glam::{Vec2, Vec3};
use jiff::{
    Span,
    civil::{Date, DateTime, Time},
};
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

use super::{
    Activity, EventKind, Feeling, Life, Mood, Question, SHELF_POSITION, Stage, Thing, World, books,
    mail, question,
    vocabulary::{self, Tone},
};
use crate::face::expression::Expression;

pub const POSITION: Vec3 = Vec3::new(-0.55, 1.05, 1.32);
pub const APPROACH: Vec3 = Vec3::new(-0.55, 1.05, 0.85);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewerLetter {
    pub day: String,
    pub lines: Vec<String>,
    pub korean: bool,
    pub answered: bool,
    pub read: bool,
    pub reply: Option<PendingReply>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingReply {
    pub arrival: String,
    pub morning: String,
    pub due: String,
    pub remaining: f32,
}

impl ViewerLetter {
    pub fn valid(&self) -> bool {
        self.day
            .parse::<Date>()
            .is_ok_and(|date| date.to_string() == self.day)
            && valid_lines(&self.lines)
    }

    pub(super) fn sanitize(&mut self) {
        self.reply = self.reply.take().filter(|reply| {
            self.read
                && !self.answered
                && reply.remaining.is_finite()
                && (0.0..=2400.0).contains(&reply.remaining)
                && [&reply.arrival, &reply.morning, &reply.due]
                    .into_iter()
                    .all(|time| {
                        time.parse::<DateTime>()
                            .is_ok_and(|time| time.date().to_string() >= self.day)
                    })
        });
        // A damaged schedule should retry reading, rather than lose the reply.
        if self.read && !self.answered && self.reply.is_none() {
            self.read = false;
        }
    }

    pub fn date_words(&self) -> String {
        mail::Letter {
            day: self.day.clone(),
            ..mail::Letter::default()
        }
        .date_words()
    }
}

fn valid_lines(lines: &[String]) -> bool {
    (1..=crate::draft::LINE_LIMIT).contains(&lines.len())
        && lines.iter().any(|line| !line.trim().is_empty())
        && lines.iter().map(|line| line.chars().count()).sum::<usize>() + lines.len() - 1
            <= crate::draft::CHARACTER_LIMIT
        && lines.iter().all(|line| {
            line.chars().count() <= crate::draft::CHARACTER_LIMIT
                && !line.chars().any(char::is_control)
        })
}

/// How the viewer says they feel in a letter: a sentence saying it plainly,
/// as a message would, or a few words letters often use, in sentences that
/// neither ask nor deny.
pub fn feelings(text: &str) -> Vec<Feeling> {
    const WORDS: [(Feeling, &[&str], &[&str]); 8] = [
        (
            Feeling::Tired,
            &["tired", "exhausted"],
            &["피곤", "지쳤", "지쳐"],
        ),
        (
            Feeling::Sad,
            &["sad", "down"],
            &["슬퍼", "슬펐", "우울", "속상"],
        ),
        (Feeling::Lonely, &["lonely"], &["외로"]),
        (
            Feeling::Stressed,
            &["stressed", "worried", "anxious"],
            &["스트레스", "걱정", "불안"],
        ),
        (Feeling::Unwell, &["sick"], &["아파", "아팠", "감기"]),
        (Feeling::Bored, &["bored"], &["심심"]),
        (
            Feeling::Happy,
            &["happy", "glad"],
            &["행복", "기뻐", "기뻤"],
        ),
        (Feeling::Proud, &["proud"], &["뿌듯"]),
    ];
    let mut found = Vec::new();
    for sentence in text.split_inclusive(['.', '!', '?', '。', '！', '？', '\n']) {
        let sentence = sentence.trim();
        let feeling = super::feelings::felt(sentence).or_else(|| {
            let lower = sentence.to_lowercase();
            let words: Vec<&str> = lower
                .split(|character: char| !character.is_alphabetic())
                .collect();
            let asks_or_denies = sentence.contains(['?', '？'])
                || question::ask(sentence).is_some()
                || words
                    .iter()
                    .any(|word| matches!(*word, "not" | "never" | "dont" | "안" | "못"))
                || lower.contains("not ")
                || lower.contains('않');
            if asks_or_denies {
                return None;
            }
            WORDS.iter().find_map(|(feeling, english, korean)| {
                (english.iter().any(|word| words.contains(word))
                    || korean.iter().any(|word| lower.contains(word)))
                .then_some(*feeling)
            })
        });
        if let Some(feeling) = feeling
            && !found.contains(&feeling)
        {
            found.push(feeling);
        }
    }
    found
}

pub fn korean(text: &str) -> bool {
    text.chars().any(|character| matches!(character, '\u{1100}'..='\u{11ff}' | '\u{3130}'..='\u{318f}' | '가'..='힣'))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Understood {
    pub tones: Vec<Tone>,
    pub viewer_name: Option<String>,
    pub ghost_name: Option<String>,
    pub questions: Vec<Question>,
    pub activities: Vec<(String, Activity)>,
    /// English and Korean counterparts, including the names it recognizes.
    pub things: Vec<(String, String)>,
    pub feelings: Vec<Feeling>,
}

pub fn understand(text: &str, life: &Life, ghost_name: &str) -> Understood {
    let mut understood = Understood {
        feelings: feelings(text),
        ..Understood::default()
    };
    for sentence in text.split_inclusive(['.', '!', '?', '。', '！', '？', '\n']) {
        let reading = vocabulary::read(sentence, ghost_name);
        if let Some(tone) = reading.tone
            && !understood.tones.contains(&tone)
        {
            understood.tones.push(tone);
        }
        // "I'm stressed." says how they feel, not who they are.
        if super::feelings::felt(sentence).is_none()
            && let Some(name) = vocabulary::introduced_name(sentence)
        {
            understood.viewer_name = Some(name);
        } else if let Some(name) = vocabulary::given_name(sentence) {
            understood.ghost_name = Some(name);
        } else if let Some(question) = Question::ask(sentence)
            && understood.questions.len() < 2
            && !understood.questions.contains(&question)
        {
            understood.questions.push(question);
        }
        for word in &reading.words {
            if let Some(meaning) = life.vocabulary.meaning(std::slice::from_ref(word))
                && !understood.activities.contains(&meaning)
            {
                understood.activities.push(meaning);
            }
        }
    }
    let lower = text.to_lowercase();
    let tokens: Vec<_> = lower
        .split(|character: char| !character.is_alphabetic())
        .collect();
    let mut recognize = |english: &str, korean: &str| {
        let name = english.to_lowercase();
        let found = if name.contains(' ') {
            lower.contains(&name)
        } else {
            tokens.contains(&name.as_str())
        };
        if (found || lower.contains(korean))
            && !understood.things.iter().any(|(known, _)| known == english)
        {
            understood
                .things
                .push((english.to_owned(), korean.to_owned()));
        }
    };
    for (english, korean) in Thing::NAMES.into_iter().zip([
        "보는 사람",
        "창문",
        "식물",
        "공",
        "램프",
        "쿠션",
        "빛 알갱이",
        "장식",
        "새",
        "나방",
        "선반",
        "블록",
        "친구",
        "그림",
        "화분",
        "책",
        "오르골",
        "거울",
        "편지",
    ]) {
        recognize(english, korean);
    }
    for gift in &life.keepsakes {
        let name = gift
            .name()
            .trim_start_matches("a ")
            .trim_start_matches("an ");
        recognize(name, gift.korean());
        if let Some(word) = name.split_whitespace().last() {
            recognize(word, gift.korean());
        }
    }
    if let Some(friend) = &life.friend {
        recognize(&friend.name, &friend.name);
    }
    for owned in &life.books.owned {
        if let Some(book) = books::LIBRARY.get(owned.index) {
            recognize(book.title(false), book.title(true));
        }
    }
    understood
}

#[derive(Clone, Debug)]
pub(super) struct Reading {
    index: usize,
    reaction: Option<(f64, bool)>,
}

impl World {
    pub fn receive_letter(&mut self, text: &str, korean: bool) -> bool {
        let letter = ViewerLetter {
            day: self.date.to_string(),
            lines: text.split('\n').map(str::to_owned).collect(),
            korean,
            ..ViewerLetter::default()
        };
        if !letter.valid() {
            return false;
        }
        if self.life.received.len() >= mail::KEPT {
            self.life.received.remove(0);
            // A carried paper goes back to the glass when its index changes.
            self.viewer_letter = None;
            if self.ghost.plan.activity == Activity::Letter {
                self.ghost.plan.done = true;
            }
        }
        self.life.received.push(letter);
        self.notice_viewer();
        if text.chars().any(char::is_alphabetic) {
            self.life.korean += (f32::from(u8::from(korean)) - self.life.korean) / 3.0;
        }
        self.record(EventKind::GotLetter);
        if self.letter_is_open() {
            self.begin(Activity::Letter);
        }
        true
    }

    pub fn waiting_letter(&self) -> Option<usize> {
        self.life.received.iter().position(|letter| !letter.read)
    }

    pub fn letter_is_open(&self) -> bool {
        self.waiting_letter().is_some()
            && self.ghost.asleep < 0.3
            && self.focus.is_none()
            && self.telling.is_none()
    }

    pub fn reading_viewer_letter(&self) -> bool {
        self.ghost.plan.activity == Activity::Letter && !self.ghost.plan.done
    }

    pub fn viewer_paper(&self) -> Option<Vec3> {
        self.waiting_letter()?;
        Some(
            if self.reading_viewer_letter() && self.ghost.plan.count > 0 {
                self.ghost.position - Vec3::Y * (0.38 * self.size())
            } else {
                POSITION
            },
        )
    }

    pub(super) fn begin_viewer_letter(&mut self) {
        self.viewer_letter = self.waiting_letter().map(|index| Reading {
            index,
            reaction: None,
        });
        self.ghost.speech = None;
    }

    pub(super) fn carry_out_letter(&mut self) -> Option<Vec3> {
        let index = self.viewer_letter.as_ref()?.index;
        if self.focus.is_some() || self.ghost.asleep >= 0.3 || self.life.received.get(index)?.read {
            self.viewer_letter = None;
            self.ghost.plan.done = true;
            return Some(POSITION);
        }
        let count = self.ghost.plan.count;
        if count == 0 && self.ghost.plan.arrived.is_some() {
            self.ghost.plan.count = 1;
        }
        if self.ghost.plan.count == 1 {
            let stayed = self.ghost.plan.stayed(self.time);
            let lines = &self.life.received[index].lines;
            let line = ((stayed / 2.0) as usize).min(lines.len() - 1);
            if count <= 1
                && self
                    .viewer_letter
                    .as_ref()
                    .unwrap()
                    .reaction
                    .is_none_or(|(at, _)| {
                        ((at - self.ghost.plan.arrived.unwrap()) / 2.0) as usize != line
                    })
            {
                let reading = vocabulary::read(&lines[line], &self.ghost.name);
                let warm = reading.named
                    || reading.tone == Some(Tone::Praise)
                    || lines[line].contains('♥')
                    || lines[line].contains("<3")
                    || !feelings(&lines[line]).is_empty();
                let curious = lines[line].contains(['?', '？']);
                // Keep the line boundary even for a neutral line.
                self.viewer_letter.as_mut().unwrap().reaction =
                    Some((self.ghost.plan.arrived.unwrap() + line as f64 * 2.0, warm));
                if warm {
                    self.suggest_mood(Mood::Cheerful, 2.0);
                } else if curious {
                    self.suggest_mood(Mood::Curious, 2.0);
                }
            }
            if stayed >= self.ghost.plan.length {
                self.ghost.plan.count = 2;
                self.ghost.plan.target = SHELF_POSITION + Vec3::new(0.0, -0.1, 0.4);
                self.ghost.plan.arrived = None;
            }
        } else if count == 2 && self.ghost.plan.arrived.is_some() {
            self.finish_viewer_letter(index);
        }
        self.viewer_paper()
    }

    pub fn letter_expression(&self) -> Option<(f64, Expression)> {
        if !self.reading_viewer_letter() || self.ghost.plan.count != 1 {
            return None;
        }
        let (at, warm) = self.viewer_letter.as_ref()?.reaction?;
        let index = self.viewer_letter.as_ref()?.index;
        let line = ((at - self.ghost.plan.arrived?) / 2.0) as usize;
        if warm {
            Some((at, Expression::Smile))
        } else if self.life.received[index]
            .lines
            .get(line)?
            .contains(['?', '？'])
        {
            Some((
                at,
                Expression::Glance {
                    toward: Vec2::new(0.4, 0.25),
                },
            ))
        } else {
            None
        }
    }

    pub fn letter_heart(&self) -> Option<f64> {
        if !self.reading_viewer_letter() || self.ghost.plan.count != 1 {
            return None;
        }
        self.viewer_letter
            .as_ref()?
            .reaction
            .filter(|(_, warm)| *warm)
            .map(|(time, _)| time)
    }

    fn finish_viewer_letter(&mut self, index: usize) {
        let text = self.life.received[index].lines.join("\n");
        let understood = understand(&text, &self.life, &self.ghost.name);
        if let Some(name) = understood.viewer_name
            && self.life.viewer_name.as_ref() != Some(&name)
        {
            self.life.viewer_name = Some(name.clone());
            self.record(EventKind::LearnedName(name));
        }
        if let Some(name) = understood.ghost_name {
            self.take_name(&name);
        }
        // How the letter says the viewer feels is remembered as if said, so
        // the ghost asks after them later.
        if let Some(feeling) = understood.feelings.first() {
            self.remember_feeling(*feeling);
        }
        let now = self.letter_now();
        let delay = self.random.range(1200.0, 2400.0);
        let morning = if now.hour() < 7 {
            now.date()
        } else {
            now.date().tomorrow().unwrap()
        }
        .at(7, 0, 0, 0);
        let letter = &mut self.life.received[index];
        letter.read = true;
        letter.reply = Some(PendingReply {
            arrival: now.checked_add(Span::new().hours(1)).unwrap().to_string(),
            morning: morning.to_string(),
            due: now
                .checked_add(Span::new().seconds(delay as i64))
                .unwrap()
                .to_string(),
            remaining: delay,
        });
        let korean = letter.korean;
        self.viewer_letter = None;
        self.ghost.plan.done = true;
        self.record(EventKind::ReadViewerLetter);
        self.say(if korean {
            "답장 쓸게!"
        } else {
            "I'll write back!"
        });
        self.think("They wrote to me...");
    }

    fn letter_now(&self) -> DateTime {
        let seconds = (self.hour.clamp(0.0, 23.999_7) * 3600.0) as i32;
        self.date.to_datetime(
            Time::new(
                (seconds / 3600) as i8,
                (seconds / 60 % 60) as i8,
                (seconds % 60) as i8,
                0,
            )
            .unwrap(),
        )
    }

    pub(super) fn advance_letters(&mut self, seconds: f32) {
        self.deliver_replies(false, seconds);
        if self.letter_is_open() && self.ghost.plan.done {
            self.begin(Activity::Letter);
        }
    }

    /// Restoring is an arrival; local civil dates allow the promise to survive
    /// time away, while the remaining timer also works in seeded worlds.
    pub fn deliver_replies(&mut self, arriving: bool, seconds: f32) {
        let now = self.letter_now();
        // A quiet viewer can keep the terminal in front of them without
        // touching a key every minute. Focus loss, rather than input expiry,
        // defers the staying-viewer timer.
        let present = self.viewer.looked_away_at.is_none();
        let mut ready = Vec::new();
        for (index, letter) in self.life.received.iter_mut().enumerate() {
            let Some(reply) = &mut letter.reply else {
                continue;
            };
            if letter.answered {
                continue;
            }
            if present {
                reply.remaining = (reply.remaining - seconds).max(0.0);
            }
            let morning = reply
                .morning
                .parse::<DateTime>()
                .is_ok_and(|time| now >= time);
            let arrival = arriving
                && reply
                    .arrival
                    .parse::<DateTime>()
                    .is_ok_and(|time| now >= time);
            if morning || arrival || (present && !arriving && reply.remaining <= 0.0) {
                ready.push(index);
            }
        }
        for index in ready {
            self.write_penpal_reply(index);
        }
    }

    fn write_penpal_reply(&mut self, index: usize) {
        let source = self.life.received[index].clone();
        let lines = self.compose_reply(&source);
        if self.deliver(lines, source.korean) {
            self.life.letters.last_mut().unwrap().reply_to = Some(Box::new(source));
            self.life.received[index].answered = true;
            self.life.received[index].reply = None;
        }
    }

    fn compose_reply(&mut self, source: &ViewerLetter) -> Vec<String> {
        let korean = source.korean;
        let say = |english: &str, hangul: &str| {
            if korean {
                hangul.to_owned()
            } else {
                english.to_owned()
            }
        };
        let signature = format!("— {}", mail::name_for_letter(&self.ghost.name, 34));
        if self.stage() == Stage::Newborn {
            return vec![say("Ooh! Letter! Yoo!", "우! 편지! 조아!"), signature];
        }
        let understood = understand(&source.lines.join("\n"), &self.life, &self.ghost.name);
        let name = self
            .life
            .viewer_name
            .as_deref()
            .map(|name| mail::name_for_letter(name, 30));
        let greeting = match (korean, name) {
            (false, Some(name)) => format!("Dear {name},"),
            (false, None) => "Dear you,".to_owned(),
            (true, Some(name)) => format!("{name}에게,"),
            (true, None) => "소중한 너에게,".to_owned(),
        };
        let nature = self.life.temperament();
        let thanks = if nature.tender >= 0.65 {
            say(
                "Thank you; your letter warmed me.",
                "편지 고마워. 마음이 따뜻해.",
            )
        } else if nature.shy >= 0.65 {
            say("Thank you. I read it shyly.", "편지 고마워. 수줍게 읽었어.")
        } else if nature.playful >= 0.65 {
            say("A letter from you! Thank you!", "네 편지다! 고마워!")
        } else {
            say("Thank you for your letter.", "편지 고마워.")
        };
        let mut body = vec![thanks];
        for question in understood.questions {
            let doing = (self.ghost.asleep < 0.3).then_some(if self.ghost.plan.done {
                Activity::Float
            } else {
                self.ghost.plan.activity
            });
            let mut answers = crate::mind::musing::answer(self, question, doing, korean);
            answers = answers
                .into_iter()
                .map(|answer| mail::name_for_letter(&answer, 80))
                .filter(|answer| !answer.trim().is_empty())
                .collect();
            answers.sort_by_key(|answer| answer.width());
            if let Some(answer) = answers.first() {
                body.push(answer.clone());
            }
        }
        if let Some(feeling) = understood.feelings.first() {
            body.push(match feeling {
                Feeling::Tired => say("Rest with me a little.", "나랑 조금 쉬자."),
                Feeling::Sad => say("I'll sit beside you.", "네 곁에 있을게."),
                Feeling::Lonely => say(
                    "You're not alone. I'm right here.",
                    "외로워하지 마. 내가 여기 있어.",
                ),
                Feeling::Stressed => say(
                    "Let's breathe slowly together.",
                    "우리 같이 천천히 숨 쉬자.",
                ),
                Feeling::Angry => say(
                    "I hope your heart feels calm soon.",
                    "마음이 곧 편안해지길 바라.",
                ),
                Feeling::Unwell => say("Get well soon. Rest lots.", "얼른 나아. 푹 쉬어."),
                Feeling::Bored => say("Come play with me!", "나랑 같이 놀자!"),
                Feeling::Happy => say("Your happiness makes me glow!", "네가 행복하면 나도 빛나!"),
                Feeling::Proud => say("You did it! I'm proud of you.", "해냈구나! 정말 대단해."),
            });
        }
        if let Some((word, activity)) = understood.activities.first() {
            body.push(if korean {
                format!("{}! 곧 같이 하자.", mail::name_for_letter(word, 24))
            } else if matches!(activity, Activity::Fetch | Activity::Play) {
                format!("{}! Let's play soon.", mail::name_for_letter(word, 24))
            } else {
                format!(
                    "{}! We can {} together.",
                    mail::name_for_letter(word, 20),
                    activity.name()
                )
            });
        } else if !understood.things.is_empty() {
            let index = self.pick(&vec![1.0; understood.things.len()]).unwrap();
            let (english, hangul) = &understood.things[index];
            body.push(if korean {
                format!("{} 이야기 기억할게.", mail::name_for_letter(hangul, 30))
            } else {
                format!("I remember {} too.", mail::name_for_letter(english, 34))
            });
        }
        let memory = if self.life.today.pages > 0 {
            say("I read a page today.", "오늘 책을 읽었어.")
        } else if self.life.today.motes > 0 {
            say("I caught a light today.", "오늘 빛을 잡았어.")
        } else if self.life.diary.last().is_some_and(|entry| {
            self.date
                .yesterday()
                .is_ok_and(|day| entry.day == day.to_string())
                && entry.text.contains("bird")
        }) {
            say("The bird came by.", "새가 다녀갔어.")
        } else {
            say("I kept your letter safe.", "네 편지를 잘 간직했어.")
        };
        body.push(memory);
        // Pack short sentences into at most four body lines, respecting cells
        // rather than bytes so Korean has the same paper budget.
        let mut lines = vec![greeting];
        for sentence in body {
            if let Some(line) = lines
                .iter_mut()
                .skip(1)
                .find(|line| line.width() + 1 + sentence.width() <= 80)
            {
                line.push(' ');
                line.push_str(&sentence);
            } else if lines.len() < 5 {
                lines.push(sentence);
            }
        }
        lines.push(signature);
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Keepsake, Nature};

    fn read(world: &mut World) {
        world.begin(Activity::Letter);
        world.ghost.position = APPROACH;
        world.step(0.1);
        world.step(25.0);
        world.ghost.position = world.ghost.plan.target;
        world.step(0.1);
        assert!(world.life.received.last().unwrap().read);
    }

    #[test]
    fn returning_to_a_background_focus_session_delivers_an_eligible_reply() {
        let mut world = World::new(4, 12.0);
        world.receive_letter("Hello!", false);
        read(&mut world);
        world.start_focus(120);
        world.look_away(true);
        let remaining = world.life.received[0].reply.as_ref().unwrap().remaining;
        world.deliver_replies(false, 2500.0);
        assert!(world.life.letters.is_empty());
        assert_eq!(
            world.life.received[0].reply.as_ref().unwrap().remaining,
            remaining
        );
        world.hour = 13.0;
        world.look_away(false);
        assert_eq!(world.life.letters.len(), 1);
        assert!(world.focus.is_some());
    }

    #[test]
    fn newborn_replies_babble_in_two_lines_and_other_replies_use_the_current_activity() {
        for korean in [false, true] {
            let mut world = World::new(4, 12.0);
            let source = ViewerLetter {
                day: world.date.to_string(),
                korean,
                lines: vec![
                    if korean {
                        "뭐 하고 있어?"
                    } else {
                        "What are you doing?"
                    }
                    .to_owned(),
                ],
                ..ViewerLetter::default()
            };
            world.birthday = Some(world.date);
            let lines = world.compose_reply(&source);
            assert_eq!(lines.len(), 2);
            assert!(mail::Letter::valid_lines(&lines));
            world.birthday = None;
            world.begin(Activity::Play);
            let lines = world.compose_reply(&source);
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains(if korean { "공" } else { "ball" })),
                "{lines:?}"
            );
        }
    }

    #[test]
    fn understanding_finds_names_tones_questions_feelings_and_known_words_in_both_languages() {
        for (text, korean, viewer, ghost, word) in [
            (
                "Hello! My name is Mina. Your name is Pip. Good ghost! I am tired. How are you? What is your name? Are you bored? Ball.",
                false,
                "Mina",
                "Pip",
                "ball",
            ),
            (
                "안녕! 내 이름은 수민이야. 너의 이름은 뽀삐야. 잘했어! 오늘 피곤해. 기분이 어때? 이름이 뭐야? 심심해? 공.",
                true,
                "수민",
                "뽀삐",
                "공",
            ),
        ] {
            let mut life = Life::default();
            life.vocabulary
                .teach(&[word.to_owned()], Activity::Play, 4.0);
            let before = life.vocabulary.clone();
            let understood = understand(text, &life, "Ghost");
            assert_eq!(understood.viewer_name.as_deref(), Some(viewer), "{text}");
            assert_eq!(understood.ghost_name.as_deref(), Some(ghost), "{text}");
            assert!(understood.tones.contains(&Tone::Greeting));
            assert!(understood.tones.contains(&Tone::Praise));
            assert_eq!(understood.questions.len(), 2, "{understood:?}");
            assert_eq!(understood.feelings, vec![Feeling::Tired]);
            assert!(
                understood
                    .activities
                    .contains(&(word.to_owned(), Activity::Play))
            );
            assert!(understood.things.iter().any(|(thing, _)| thing == "ball"));
            assert_eq!(life.vocabulary, before);
            assert_eq!(super::korean(text), korean);
        }
        for (text, tone, feeling) in [
            ("Bye! I am sad.", Tone::Farewell, Feeling::Sad),
            ("No! So tired today.", Tone::Scolding, Feeling::Tired),
            ("잘가! 오늘 행복해.", Tone::Farewell, Feeling::Happy),
            ("안돼! 피곤해.", Tone::Scolding, Feeling::Tired),
        ] {
            let understood = understand(text, &Life::default(), "Pip");
            assert!(understood.tones.contains(&tone), "{text}");
            assert_eq!(understood.feelings, vec![feeling]);
        }
        assert!(feelings("A tiredness test with busywork and a happyish word").is_empty());
        assert!(feelings("Are you tired? I'm not sad. 안 슬퍼.").is_empty());
        assert_eq!(
            feelings("I was so stressed this week. But today I'm happy!"),
            vec![Feeling::Stressed, Feeling::Happy]
        );
    }

    #[test]
    fn known_keepsakes_and_owned_book_titles_are_recognized_in_both_languages() {
        let mut life = Life::default();
        life.keepsakes.push(Keepsake::Feather);
        for text in ["A feather and The Lantern Fox.", "깃털이랑 등불 여우."] {
            let understood = understand(text, &life, "Pip");
            assert!(understood.things.iter().any(|(name, _)| name == "feather"));
            assert!(
                understood
                    .things
                    .iter()
                    .any(|(name, _)| name == "The Lantern Fox")
            );
        }
    }

    #[test]
    fn receiving_counts_warms_the_bond_and_reading_learns_names_without_teaching_words() {
        let mut world = World::new(4, 12.0);
        let vocabulary = world.life.vocabulary.clone();
        let bond = world.life.bond;
        assert!(world.receive_letter(
            "My name is Mina.\nYour name is Pip.\nI love you, Ghost ♥",
            false
        ));
        assert!((world.life.bond - bond - 0.008).abs() < 0.00001);
        assert_eq!(world.life.total.letters_received, 1);
        assert!(
            world
                .life
                .today
                .diary()
                .to_lowercase()
                .contains("received your letter")
        );
        assert!(world.events.iter().any(|event| matches!(&event.kind, EventKind::Milestone(text) if text == "You wrote me my first letter.")));
        read(&mut world);
        assert_eq!(world.life.viewer_name.as_deref(), Some("Mina"));
        assert_eq!(world.ghost.name, "Pip");
        assert_eq!(world.life.vocabulary, vocabulary);
        assert_eq!(world.life.total.viewer_letters_read, 1);
        assert!(world.viewer_paper().is_none());
        assert!(world.life.received[0].reply.is_some());
        assert!(
            world
                .events
                .iter()
                .any(|event| matches!(&event.kind, EventKind::LearnedName(name) if name == "Mina"))
        );
        assert!(
            world
                .events
                .iter()
                .any(|event| matches!(&event.kind, EventKind::Named(name) if name == "Pip"))
        );
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "I'll write back!"
        );
        assert_eq!(
            world.ghost.thought.as_ref().unwrap().text,
            "They wrote to me..."
        );
    }

    #[test]
    fn sleeping_and_working_defer_reading_and_interruptions_leave_the_paper_waiting() {
        let mut world = World::new(4, 12.0);
        world.begin(Activity::Rest);
        world.ghost.asleep = 1.0;
        world.receive_letter("Hello, Pip!", false);
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert_eq!(world.ghost.asleep, 1.0);
        assert!(world.viewer_paper().is_some());
        world.ghost.asleep = 0.0;
        world.start_focus(25);
        world.step(1.0);
        assert!(!world.life.received[0].read);
        assert!(!world.options().contains(&Activity::Letter));
        world.end_focus();
        world.begin(Activity::Letter);
        world.ghost.position = APPROACH;
        world.step(0.1);
        assert_eq!(world.ghost.plan.count, 1);
        world.say("Should stay quiet");
        assert!(world.ghost.speech.is_none());
        world.begin(Activity::Float);
        assert_eq!(world.viewer_paper(), Some(POSITION));
        assert!(!world.life.received[0].read);
        world.ghost.plan.done = true;
        world.step(0.1);
        assert_eq!(world.ghost.plan.activity, Activity::Letter);
        read(&mut world);
    }

    #[test]
    fn the_ghost_reaches_reads_and_keeps_a_letter_without_teleporting() {
        let mut world = World::new(13, 12.0);
        world.receive_letter("Dear Ghost,\nYou are great! ♥\nHow are you?", false);
        let started = world.time;
        let mut held_at = None;
        for _ in 0..1200 {
            world.step(0.1);
            if world.ghost.plan.count == 1 && held_at.is_none() {
                held_at = Some(world.time);
            }
            if world.life.received[0].read {
                break;
            }
        }
        assert!(world.life.received[0].read, "{:?}", world.ghost.plan);
        assert!(world.time - held_at.unwrap() >= 6.0);
        assert!(world.time - started < 60.0);
    }

    #[test]
    fn a_feeling_in_a_letter_is_no_introduction() {
        let life = Life {
            viewer_name: Some("Mina".to_owned()),
            ..Life::default()
        };
        for text in ["I'm Stressed.", "I'm Tired!\nI am Happy."] {
            let understood = understand(text, &life, "Pip");
            assert_eq!(understood.viewer_name, None, "{text}");
            assert!(!understood.feelings.is_empty(), "{text}");
        }
        assert_eq!(
            understand("I'm Mina.", &life, "Pip").viewer_name.as_deref(),
            Some("Mina")
        );
    }

    #[test]
    fn a_letter_saying_how_the_viewer_feels_is_remembered_for_asking_after_them() {
        let mut world = World::new(13, 12.0);
        world.receive_letter("Dear Ghost,\nWork was long.\nI'm so tired today.", false);
        for _ in 0..1200 {
            world.step(0.1);
            if world.life.received[0].read {
                break;
            }
        }
        assert!(world.life.received[0].read);
        let note = world.life.feelings.last().unwrap();
        assert_eq!(note.feeling, Feeling::Tired);
        assert_eq!(note.day, world.date.to_string());
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::ViewerFelt(Feeling::Tired))
        );
    }

    #[test]
    fn replies_answer_state_questions_in_the_letters_language_and_are_valid_and_seeded() {
        for seed in 0..30 {
            for korean in [false, true] {
                let mut world = World::new(seed, 12.0);
                world.ghost.name = "Pip".to_owned();
                world.life.viewer_name = Some("Mina".to_owned());
                world.life.korean = if korean { 0.0 } else { 1.0 };
                world.life.nature = Some(Nature::default());
                world.life.today.pages = 1;
                let source = ViewerLetter {
                    day: world.date.to_string(),
                    korean,
                    lines: vec![
                        if korean {
                            "이름이 뭐야? 내 이름 알아? 오늘 피곤해. 공."
                        } else {
                            "What is your name? Do you know my name? I am tired. Ball."
                        }
                        .to_owned(),
                    ],
                    ..ViewerLetter::default()
                };
                let lines = world.compose_reply(&source);
                assert!(mail::Letter::valid_lines(&lines), "{lines:?}");
                let text = lines.join(" ");
                assert!(text.contains("Pip"));
                assert!(text.contains("Mina"));
                assert!(
                    text.contains(if korean {
                        "나랑 조금 쉬자"
                    } else {
                        "Rest with me"
                    }),
                    "{lines:?}"
                );
                assert!(
                    text.contains(if korean {
                        "오늘 책을 읽었어"
                    } else {
                        "I read a page today"
                    }),
                    "{lines:?}"
                );
                assert!(
                    text.contains(if korean { "공" } else { "ball" }),
                    "{lines:?}"
                );
                if korean {
                    assert!(!text.contains("Thank you"));
                } else {
                    assert!(!super::korean(&text));
                }
                let mut second = World::new(seed, 12.0);
                second.ghost.name = world.ghost.name.clone();
                second.life = world.life.clone();
                // Match random consumption since construction.
                assert_eq!(lines, second.compose_reply(&source));
            }
        }
    }

    #[test]
    fn replies_wait_for_the_seeded_delay_an_hour_later_arrival_or_next_morning_once() {
        let mut world = World::new(4, 12.0);
        world.receive_letter("How are you?", false);
        read(&mut world);
        let delay = world.life.received[0].reply.as_ref().unwrap().remaining;
        assert!((1200.0..=2400.0).contains(&delay));
        world.notice_viewer();
        world.deliver_replies(false, delay - 1.0);
        assert!(world.life.letters.is_empty());
        world.deliver_replies(false, 1.0);
        assert_eq!(world.life.letters.len(), 1);
        assert!(world.life.received[0].answered);
        assert!(world.life.letters[0].reply_to.is_some());
        world.deliver_replies(false, 5000.0);
        assert_eq!(world.life.letters.len(), 1);
        for (hour, next_day, expected) in [(12.9, false, 0), (13.0, false, 1), (7.0, true, 1)] {
            let mut restored = World::new(4, hour);
            restored.life = serde_json::from_str::<Life>(
                &serde_json::to_string(&{
                    let mut original = World::new(4, 12.0);
                    original.receive_letter("Hello!", false);
                    read(&mut original);
                    original.life
                })
                .unwrap(),
            )
            .unwrap()
            .sanitized();
            if next_day {
                restored.date = restored.date.tomorrow().unwrap();
            }
            restored.deliver_replies(true, 0.0);
            assert_eq!(restored.life.letters.len(), expected, "{hour} {next_day}");
        }
    }

    #[test]
    fn received_letters_default_round_trip_validate_and_keep_only_the_newest_twenty() {
        let mut world = World::new(4, 12.0);
        for day in 1..=25 {
            world.date = format!("2026-06-{day:02}").parse().unwrap();
            assert!(world.receive_letter("Hello\n안녕", true));
        }
        assert_eq!(world.life.received.len(), 20);
        let encoded = serde_json::to_string(&world.life).unwrap();
        assert_eq!(
            serde_json::from_str::<Life>(&encoded).unwrap().received,
            world.life.received
        );
        let mut old = serde_json::to_value(&world.life).unwrap();
        old.as_object_mut().unwrap().remove("received");
        assert!(
            serde_json::from_value::<Life>(old)
                .unwrap()
                .received
                .is_empty()
        );
        let valid = world.life.received[0].clone();
        for damaged in ["", "bad\u{1b}", &"a".repeat(601), &"line\n".repeat(13)] {
            let mut letter = valid.clone();
            letter.lines = damaged.split('\n').map(str::to_owned).collect();
            world.life.received.push(letter);
        }
        let mut damaged = valid.clone();
        damaged.day = "2026-02-30".to_owned();
        world.life.received.push(damaged);
        let life = world.life.sanitized();
        assert_eq!(life.received.len(), 20);
        assert_eq!(life.received[0].day, "2026-06-06");
    }
}
