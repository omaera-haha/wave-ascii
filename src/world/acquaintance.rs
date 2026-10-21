use glam::Vec3;
use jiff::civil::Date;
use serde::{Deserialize, Deserializer, Serialize};

use super::{Activity, EventKind, Heard, Listening, Mood, Season, Understanding, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Topic {
    Color,
    Season,
    Animal,
    Food,
    Birthday,
}

impl Topic {
    pub const ALL: [Self; 5] = [
        Self::Color,
        Self::Season,
        Self::Animal,
        Self::Food,
        Self::Birthday,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Color => "favorite color",
            Self::Season => "favorite season",
            Self::Animal => "favorite animal",
            Self::Food => "favorite food",
            Self::Birthday => "birthday",
        }
    }

    pub fn question(self, korean: bool) -> &'static str {
        match (self, korean) {
            (Self::Color, false) => "What's your favorite color?",
            (Self::Color, true) => "무슨 색 좋아해?",
            (Self::Season, false) => "What's your favorite season?",
            (Self::Season, true) => "어떤 계절 좋아해?",
            (Self::Animal, false) => "What's your favorite animal?",
            (Self::Animal, true) => "무슨 동물 좋아해?",
            (Self::Food, false) => "What's your favorite food?",
            (Self::Food, true) => "무슨 음식 좋아해?",
            (Self::Birthday, false) => "When is your birthday?",
            (Self::Birthday, true) => "생일이 언제야?",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Pink,
    White,
    Black,
    Brown,
    Gray,
}

impl Color {
    pub const ALL: [Self; 11] = [
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
        Self::Pink,
        Self::White,
        Self::Black,
        Self::Brown,
        Self::Gray,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Pink => "pink",
            Self::White => "white",
            Self::Black => "black",
            Self::Brown => "brown",
            Self::Gray => "gray",
        }
    }

    pub fn korean(self) -> &'static str {
        match self {
            Self::Red => "빨간색",
            Self::Orange => "주황색",
            Self::Yellow => "노란색",
            Self::Green => "초록색",
            Self::Blue => "파란색",
            Self::Purple => "보라색",
            Self::Pink => "분홍색",
            Self::White => "하얀색",
            Self::Black => "검은색",
            Self::Brown => "갈색",
            Self::Gray => "회색",
        }
    }

    pub fn rgb(self) -> Vec3 {
        match self {
            Self::Red => Vec3::new(1.0, 0.38, 0.34),
            Self::Orange => Vec3::new(1.0, 0.6, 0.3),
            Self::Yellow => Vec3::new(1.0, 0.86, 0.35),
            Self::Green => Vec3::new(0.4, 0.9, 0.52),
            Self::Blue => Vec3::new(0.38, 0.65, 1.0),
            Self::Purple => Vec3::new(0.74, 0.48, 1.0),
            Self::Pink => Vec3::new(1.0, 0.48, 0.7),
            Self::White => Vec3::splat(0.96),
            Self::Black => Vec3::splat(0.38),
            Self::Brown => Vec3::new(0.65, 0.44, 0.32),
            Self::Gray => Vec3::splat(0.65),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Birthday {
    pub month: i8,
    pub day: i8,
}

impl Birthday {
    pub fn valid(self) -> bool {
        Date::new(2000, self.month, self.day).is_ok()
    }

    pub fn falls_on(self, date: Date) -> bool {
        let day = if self.month == 2 && self.day == 29 && !date.in_leap_year() {
            28
        } else {
            self.day
        };
        self.valid() && self.month == date.month() && day == date.day()
    }

    pub fn words(self) -> String {
        const MONTHS: [&str; 12] = [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ];
        const DAYS: [&str; 31] = [
            "first",
            "second",
            "third",
            "fourth",
            "fifth",
            "sixth",
            "seventh",
            "eighth",
            "ninth",
            "tenth",
            "eleventh",
            "twelfth",
            "thirteenth",
            "fourteenth",
            "fifteenth",
            "sixteenth",
            "seventeenth",
            "eighteenth",
            "nineteenth",
            "twentieth",
            "twenty-first",
            "twenty-second",
            "twenty-third",
            "twenty-fourth",
            "twenty-fifth",
            "twenty-sixth",
            "twenty-seventh",
            "twenty-eighth",
            "twenty-ninth",
            "thirtieth",
            "thirty-first",
        ];
        if !self.valid() {
            return "an unknown day".to_owned();
        }
        format!(
            "{} {}",
            MONTHS[self.month as usize - 1],
            DAYS[self.day as usize - 1]
        )
    }

    pub fn korean(self) -> String {
        const MONTHS: [&str; 12] = [
            "일", "이", "삼", "사", "오", "유", "칠", "팔", "구", "시", "십일", "십이",
        ];
        if !self.valid() {
            return "아직 모르는 날".to_owned();
        }
        format!(
            "{}월 {}일",
            MONTHS[self.month as usize - 1],
            korean_number(self.day)
        )
    }
}

fn korean_number(number: i8) -> String {
    const ONES: [&str; 10] = ["", "일", "이", "삼", "사", "오", "육", "칠", "팔", "구"];
    let tens = match number / 10 {
        0 => "",
        1 => "십",
        2 => "이십",
        _ => "삼십",
    };
    format!("{tens}{}", ONES[(number % 10) as usize])
}

// Invalid optional answers should not prevent the rest of a saved life loading.
fn optional_answer<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Acquaintance {
    #[serde(default, deserialize_with = "optional_answer")]
    pub color: Option<Color>,
    #[serde(default, deserialize_with = "optional_answer")]
    pub season: Option<Season>,
    #[serde(default, deserialize_with = "optional_answer")]
    pub animal: Option<String>,
    #[serde(default, deserialize_with = "optional_answer")]
    pub food: Option<String>,
    #[serde(default, deserialize_with = "optional_answer")]
    pub birthday: Option<Birthday>,
    #[serde(default)]
    pub last_asked: Option<String>,
    #[serde(default)]
    pub asked_today: Vec<Topic>,
    #[serde(default)]
    pub season_greeted: Option<String>,
    #[serde(default)]
    pub birthday_greeted: Option<String>,
}

impl Acquaintance {
    pub fn sanitize(&mut self) {
        self.animal = self.animal.take().filter(|text| valid_text(text));
        self.food = self.food.take().filter(|text| valid_text(text));
        self.birthday = self.birthday.filter(|birthday| birthday.valid());
        for day in [
            &mut self.last_asked,
            &mut self.season_greeted,
            &mut self.birthday_greeted,
        ] {
            *day = day.take().filter(|day| day.parse::<Date>().is_ok());
        }
        self.asked_today.truncate(2);
        self.asked_today.dedup();
        if self.last_asked.is_none() {
            self.asked_today.clear();
        }
    }

    pub fn answer(&self, topic: Topic, korean: bool) -> Option<String> {
        match topic {
            Topic::Color => self
                .color
                .map(|color| if korean { color.korean() } else { color.name() }.to_owned()),
            Topic::Season => self.season.map(|season| {
                if korean {
                    season.korean()
                } else {
                    season.name()
                }
                .to_owned()
            }),
            Topic::Animal => self.animal.clone(),
            Topic::Food => self.food.clone(),
            Topic::Birthday => self.birthday.map(|birthday| {
                if korean {
                    birthday.korean()
                } else {
                    birthday.words()
                }
            }),
        }
    }

    pub fn words(&self) -> String {
        Topic::ALL
            .into_iter()
            .filter_map(|topic| {
                self.answer(topic, false).map(|answer| {
                    if topic == Topic::Birthday {
                        format!("their birthday is {answer}")
                    } else {
                        format!("their {} is {answer}", topic.name())
                    }
                })
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn reply(&self, topic: Topic, korean: bool) -> String {
        self.answer(topic, korean).map_or_else(
            || {
                if korean {
                    "아직 말 안 해 줬어!"
                } else {
                    "You haven't told me yet!"
                }
                .to_owned()
            },
            |answer| {
                if korean {
                    format!("{answer}! 기억하고 있어.")
                } else {
                    format!("{}! I remember.", capitalized(&answer))
                }
            },
        )
    }
}

fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + characters.as_str()
    })
}

fn valid_text(text: &str) -> bool {
    !text.trim().is_empty()
        && text.chars().count() <= 24
        && !text
            .chars()
            .any(|character| character.is_control() || character.is_numeric())
        && text.chars().any(char::is_alphabetic)
}

fn unrelated(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    text.contains(['?', '？'])
        || super::question::ask(text).is_some()
        || super::focus::requested(text).is_some()
        || super::breeze::requested(text).is_some()
        || super::puzzle::requested(text)
        || super::music::lullaby_requested(text)
        || super::music::lesson(text).is_some()
        || super::quirks::yawn_message(text)
        || super::vocabulary::introduced_name(text).is_some()
        || super::vocabulary::given_name(text).is_some()
        || [
            "please ",
            "can you ",
            "could you ",
            "tell me ",
            "let's ",
            "lets ",
            "fetch ",
            "play ",
            "open ",
            "close ",
            "go ",
            "stop ",
            "what ",
            "why ",
            "how ",
            "when ",
            "그만",
            "해줘",
            "해 줘",
            "벗어",
        ]
        .iter()
        .any(|part| lower.starts_with(part) || (!part.is_ascii() && lower.contains(part)))
        || matches!(
            lower.as_str(),
            "hello"
                | "hi"
                | "bye"
                | "no"
                | "stop"
                | "yes"
                | "yeah"
                | "yep"
                | "ok"
                | "okay"
                | "sure"
                | "안녕"
                | "아니"
                | "싫어"
                | "응"
                | "네"
                | "좋아"
        )
}

fn answer_text(text: &str) -> Option<String> {
    if text.chars().any(char::is_control) || unrelated(text) {
        return None;
    }
    let trimmed = text.trim().trim_end_matches(['.', '!']).trim();
    let lower = trimmed.to_lowercase();
    let mut answer = trimmed;
    for prefix in [
        "my favorite color is ",
        "my favourite color is ",
        "my favorite season is ",
        "my favorite animal is ",
        "my favourite animal is ",
        "my favorite food is ",
        "my favourite food is ",
        "my birthday is ",
        "my favorite is ",
        "my favourite is ",
        "i really like ",
        "i like ",
        "i love ",
        "it's ",
        "it’s ",
        "it is ",
        "on ",
    ] {
        if lower.starts_with(prefix) {
            answer = trimmed.get(prefix.len()..)?.trim();
            break;
        }
    }
    for prefix in [
        "나는 ",
        "난 ",
        "저는 ",
        "내가 좋아하는 동물은 ",
        "내가 좋아하는 음식은 ",
        "내 생일은 ",
        "생일은 ",
    ] {
        if let Some(rest) = answer.strip_prefix(prefix) {
            answer = rest.trim();
            break;
        }
    }
    for suffix in [
        "을 좋아해요",
        "를 좋아해요",
        "을 좋아해",
        "를 좋아해",
        " 좋아해요",
        " 좋아해",
        "이에요",
        "예요",
    ] {
        if let Some(rest) = answer.strip_suffix(suffix) {
            answer = rest.trim();
            break;
        }
    }
    (!answer.is_empty()).then(|| answer.to_owned())
}

pub fn color(text: &str) -> Option<Color> {
    let answer = answer_text(text)?.to_lowercase();
    let answer = answer
        .strip_suffix("이야")
        .or_else(|| answer.strip_suffix('야'))
        .unwrap_or(&answer);
    let answer = answer
        .strip_prefix("light ")
        .or_else(|| answer.strip_prefix("dark "))
        .unwrap_or(answer);
    Color::ALL.into_iter().find(|color| {
        answer == color.name()
            || answer == color.korean()
            || match color {
                Color::Red => matches!(answer, "빨강" | "빨강색"),
                Color::Orange => answer == "주황",
                Color::Yellow => answer == "노랑",
                Color::Green => matches!(answer, "초록" | "녹색"),
                Color::Blue => matches!(answer, "파랑" | "하늘색" | "sky blue"),
                Color::Purple => matches!(answer, "보라" | "violet"),
                Color::Pink => matches!(answer, "분홍" | "핑크" | "핑크색"),
                Color::White => matches!(answer, "하양" | "흰색"),
                Color::Black => matches!(answer, "검정" | "검정색"),
                Color::Brown => false,
                Color::Gray => answer == "grey",
            }
    })
}

pub fn season(text: &str) -> Option<Season> {
    let answer = answer_text(text)?.to_lowercase();
    let answer = answer
        .strip_suffix("이야")
        .or_else(|| answer.strip_suffix('야'))
        .unwrap_or(&answer);
    [
        Season::Spring,
        Season::Summer,
        Season::Autumn,
        Season::Winter,
    ]
    .into_iter()
    .find(|season| {
        answer == season.name()
            || answer == season.korean()
            || (*season == Season::Autumn && answer == "fall")
    })
}

pub fn birthday(text: &str) -> Option<Birthday> {
    let answer = answer_text(text)?.to_lowercase();
    let answer = answer
        .strip_suffix("이야")
        .or_else(|| answer.strip_suffix('야'))
        .unwrap_or(&answer);
    let number = |text: &str| -> Option<i8> {
        text.parse()
            .ok()
            .or_else(|| (1..=31).find(|&number| korean_number(number) == text))
    };
    let (month, day) = if let Some((month, day)) = answer.split_once('월') {
        (
            match month.trim() {
                "유" => 6,
                "시" => 10,
                month => number(month)?,
            },
            number(day.trim().strip_suffix('일').unwrap_or(day.trim()))?,
        )
    } else if let Some((first, second)) = answer.split_once('/') {
        let first: i8 = first.trim().parse().ok()?;
        let second: i8 = second.trim().parse().ok()?;
        if first > 12 && second <= 12 {
            (second, first)
        } else if second > 12 && first <= 12 || first == second {
            (first, second)
        } else {
            return None;
        }
    } else {
        let words: Vec<&str> = answer.split_whitespace().collect();
        let [first, second] = words.as_slice() else {
            return None;
        };
        let month = |text: &str| {
            [
                "january",
                "february",
                "march",
                "april",
                "may",
                "june",
                "july",
                "august",
                "september",
                "october",
                "november",
                "december",
            ]
            .iter()
            .position(|&month| month == text)
            .map(|index| index as i8 + 1)
        };
        let day = |text: &str| {
            text.trim_end_matches([','])
                .trim_end_matches("st")
                .trim_end_matches("nd")
                .trim_end_matches("rd")
                .trim_end_matches("th")
                .parse::<i8>()
                .ok()
        };
        if let Some(month) = month(first) {
            (month, day(second)?)
        } else {
            (month(second)?, day(first)?)
        }
    };
    let birthday = Birthday { month, day };
    birthday.valid().then_some(birthday)
}

pub fn free_text(text: &str) -> Option<String> {
    let answer = answer_text(text)?;
    let lower = answer.to_lowercase();
    if matches!(
        lower.as_str(),
        "i don't know" | "idk" | "not sure" | "maybe" | "모르겠어" | "잘 모르겠어"
    ) || ["i am ", "i'm ", "today ", "내일 "]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return None;
    }
    valid_text(&answer).then_some(answer)
}

impl World {
    fn quiet_for_question(&self) -> bool {
        self.focus.is_none()
            && self.telling.is_none()
            && self.game.is_none()
            && !self.puzzle_active()
            && self.reading.is_none()
            && !self
                .drawing
                .as_ref()
                .is_some_and(|drawing| drawing.awaiting_name(self.time))
            && self.sebae.is_none()
            && self.pending_wish.is_none()
    }

    pub(super) fn ask_viewer(&mut self) {
        if self.asking.is_some_and(|(_, until)| self.time > until) {
            self.asking = None;
        }
        if self.asking.is_some()
            || !self.viewer_present()
            || self.viewer.typing
            || self.ghost.asleep >= 0.3
            || self.life.bond < 0.35
            || self.ghost.plan.activity != Activity::Visit
            || self.ghost.plan.done
            || !self
                .ghost
                .plan
                .arrived
                .is_some_and(|at| self.time - at >= 5.0)
            || !self
                .last_message_at
                .is_some_and(|at| (0.0..=600.0).contains(&(self.time - at)))
            || self
                .last_question_at
                .is_some_and(|at| self.time - at < 600.0)
            || self
                .ghost
                .speech
                .as_ref()
                .is_some_and(|line| self.time - line.time < 12.0)
            || !self.quiet_for_question()
        {
            return;
        }
        let day = self.date.to_string();
        let viewer = &self.life.viewer;
        let today = viewer.last_asked.as_ref() == Some(&day);
        let limit = if self.life.nature.unwrap_or_default().curious >= 0.65 {
            2
        } else {
            1
        };
        if today && viewer.asked_today.len() >= limit {
            return;
        }
        let topics: Vec<Topic> = Topic::ALL
            .into_iter()
            .filter(|&topic| {
                viewer.answer(topic, false).is_none()
                    && (topic != Topic::Birthday || self.life.bond >= 0.5)
                    && (!today || !viewer.asked_today.contains(&topic))
            })
            .collect();
        let Some(index) = self.pick(&vec![1.0; topics.len()]) else {
            return;
        };
        let topic = topics[index];
        if !today {
            self.life.viewer.asked_today.clear();
        }
        self.life.viewer.last_asked = Some(day);
        self.life.viewer.asked_today.push(topic);
        self.asking = Some((topic, self.time + 180.0));
        self.last_question_at = Some(self.time);
        // It stays at the glass a minute for the answer.
        self.ghost.plan.length = self
            .ghost
            .plan
            .length
            .max(self.ghost.plan.stayed(self.time) + 60.0);
        self.say(topic.question(self.speaks_korean()));
        self.record(EventKind::AskedViewer(topic));
    }

    pub(super) fn answer_viewer(&mut self, text: &str, korean: bool) -> Option<Heard> {
        let (topic, until) = self.asking.take()?;
        let definite_answer = match topic {
            Topic::Color => color(text).is_some(),
            Topic::Season => season(text).is_some(),
            Topic::Birthday => birthday(text).is_some(),
            _ => answer_text(text)
                .is_some_and(|answer| answer != text.trim().trim_end_matches(['.', '!'])),
        };
        // Farewells, greetings, and scolding keep their usual meaning, and a
        // guess in a game on the glass belongs to the game.
        let tone = super::vocabulary::read(text, &self.ghost.name).tone;
        if self.time > until
            || self.ghost.asleep >= 0.3
            || self.puzzle_active()
            || self
                .game
                .as_ref()
                .is_some_and(|game| game.outcome.is_none())
            || matches!(
                tone,
                Some(
                    super::vocabulary::Tone::Farewell
                        | super::vocabulary::Tone::Greeting
                        | super::vocabulary::Tone::Scolding
                )
            )
            || unrelated(text)
            || (!definite_answer
                && self
                    .life
                    .vocabulary
                    .meaning(&super::vocabulary::tokens(text))
                    .is_some())
            || super::wardrobe::requested_off(text, self.wearing)
        {
            return None;
        }
        let understood = match topic {
            Topic::Color => {
                self.life.viewer.color = color(text);
                self.life.viewer.color.is_some()
            }
            Topic::Season => {
                self.life.viewer.season = season(text);
                self.life.viewer.season.is_some()
            }
            Topic::Animal => {
                self.life.viewer.animal = free_text(text);
                self.life.viewer.animal.is_some()
            }
            Topic::Food => {
                self.life.viewer.food = free_text(text);
                self.life.viewer.food.is_some()
            }
            Topic::Birthday => {
                self.life.viewer.birthday = birthday(text);
                self.life.viewer.birthday.is_some()
            }
        };
        self.listening = Listening::default();
        self.wake();
        self.notice_viewer();
        self.record(EventKind::Heard(text.to_owned()));
        if understood {
            self.record(EventKind::LearnedAboutViewer(topic));
            self.suggest_mood(Mood::Cheerful, 6.0);
            let answer = self.life.viewer.answer(topic, korean).unwrap();
            self.say(&if korean {
                format!("{answer}! 기억할게.")
            } else {
                format!("{}! I'll remember.", capitalized(&answer))
            });
            if topic == Topic::Birthday {
                self.reconsider_wardrobe();
            }
        } else {
            self.say(if korean {
                "음, 다음에 또 물어볼게."
            } else {
                "Hmm, I'll ask again another day."
            });
        }
        Some(Heard {
            tone: None,
            understanding: if understood {
                Understanding::Answered(topic)
            } else {
                Understanding::Unanswered(topic)
            },
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    pub(super) fn greet_favorite_season(&mut self) {
        if self.date.day() != 1
            || !matches!(self.date.month(), 3 | 6 | 9 | 12)
            || !(6.5..12.0).contains(&self.hour)
            || !self.viewer_present()
            || self.ghost.asleep >= 0.3
            || self.life.viewer.season != Some(self.season())
            || !self.quiet_for_question()
            || self.asking.is_some()
            || self
                .ghost
                .speech
                .as_ref()
                .is_some_and(|line| self.time - line.time < 12.0)
        {
            return;
        }
        let day = self.date.to_string();
        if self.life.viewer.season_greeted.as_ref() == Some(&day) {
            return;
        }
        self.say(self.spoken(
            "Your favorite season is here!",
            "네가 좋아하는 계절이 왔어!",
        ));
        self.life.viewer.season_greeted = Some(day);
    }

    pub fn viewer_blush(&self) -> Option<(Vec3, f32)> {
        let color = self.life.viewer.color?;
        let arrived = self.viewer_blush_at?;
        let strength = (1.0 - ((self.time - arrived) / 8.0) as f32).clamp(0.0, 1.0);
        (strength > 0.0 && self.ghost.asleep < 0.3).then_some((color.rgb(), strength * 0.65))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Game, Life, Mark, Nature, Occasion, Question};

    fn visiting(seed: u64) -> World {
        let mut world = World::new(seed, 12.0);
        world.begin_life("2026-06-21");
        world.life.nature = Some(Nature::default());
        world.life.bond = 0.4;
        world.time = 20.0;
        world.notice_viewer();
        world.last_message_at = Some(15.0);
        world.begin(Activity::Visit);
        world.ghost.plan.arrived = Some(10.0);
        world
    }

    #[test]
    fn farewells_scolding_yes_and_game_guesses_keep_their_meaning_while_a_question_waits() {
        for text in ["goodnight", "no!", "yes", "okay", "잘자"] {
            let mut world = visiting(5);
            world.asking = Some((Topic::Food, world.time + 180.0));
            let heard = world.hear(text);
            assert_ne!(
                heard.understanding,
                Understanding::Answered(Topic::Food),
                "{text}"
            );
            assert_eq!(world.life.viewer.food, None, "{text}");
        }
        let mut world = visiting(5);
        world.asking = Some((Topic::Food, world.time + 180.0));
        world.begin(Activity::Puzzle);
        assert!(world.asking.is_none(), "a game ends the wait for an answer");
        assert_eq!(world.life.viewer.food, None);
    }

    #[test]
    fn it_waits_at_the_glass_a_minute_for_an_answer() {
        let mut world = visiting(5);
        world.ghost.plan.length = 1.0;
        world.ghost.plan.arrived = Some(world.time - 6.0);
        world.ask_viewer();
        let (_, until) = world.asking.unwrap();
        world.asking = Some((Topic::Food, until));
        world.step(50.0);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(!world.ghost.plan.done);
        let heard = world.hear("pizza");
        assert_eq!(heard.understanding, Understanding::Answered(Topic::Food));
        assert_eq!(world.life.viewer.food.as_deref(), Some("pizza"));
    }

    #[test]
    fn colors_are_canonical_in_both_languages() {
        for (text, expected) in [
            ("blue", Color::Blue),
            ("light blue", Color::Blue),
            ("파란색", Color::Blue),
            ("파랑", Color::Blue),
            ("하늘색", Color::Blue),
            ("빨간색", Color::Red),
            ("노란색", Color::Yellow),
            ("초록색", Color::Green),
            ("보라색", Color::Purple),
            ("분홍색", Color::Pink),
            ("하얀색", Color::White),
            ("검은색", Color::Black),
            ("주황색", Color::Orange),
            ("I like BLUE!", Color::Blue),
            ("my favorite color is red", Color::Red),
            ("초록색 좋아해", Color::Green),
            ("흰색이야", Color::White),
        ] {
            assert_eq!(color(text), Some(expected), "{text}");
        }
        for color_value in Color::ALL {
            assert_eq!(color(color_value.name()), Some(color_value));
            assert_eq!(color(color_value.korean()), Some(color_value));
        }
        for text in [
            "blueberries",
            "blue and red",
            "are you blue?",
            "play with the blue ball",
        ] {
            assert_eq!(color(text), None, "{text}");
        }
    }

    #[test]
    fn seasons_accept_both_languages_and_autumn_aliases() {
        for (text, expected) in [
            ("spring", Season::Spring),
            ("봄", Season::Spring),
            ("summer", Season::Summer),
            ("여름", Season::Summer),
            ("fall", Season::Autumn),
            ("autumn", Season::Autumn),
            ("가을", Season::Autumn),
            ("winter", Season::Winter),
            ("겨울", Season::Winter),
            ("I like spring", Season::Spring),
        ] {
            assert_eq!(season(text), Some(expected), "{text}");
        }
        assert_eq!(season("summerhouse"), None);
    }

    #[test]
    fn birthdays_accept_named_months_and_unambiguous_numeric_dates() {
        for (text, expected) in [
            ("March 3", Birthday { month: 3, day: 3 }),
            ("3 March", Birthday { month: 3, day: 3 }),
            ("march 3rd", Birthday { month: 3, day: 3 }),
            ("My birthday is March 3rd", Birthday { month: 3, day: 3 }),
            ("3/3", Birthday { month: 3, day: 3 }),
            ("3/14", Birthday { month: 3, day: 14 }),
            ("14/3", Birthday { month: 3, day: 14 }),
            ("3월 14일", Birthday { month: 3, day: 14 }),
            ("삼월 십사일", Birthday { month: 3, day: 14 }),
            ("February 29", Birthday { month: 2, day: 29 }),
            ("유월 삼일", Birthday { month: 6, day: 3 }),
        ] {
            assert_eq!(birthday(text), Some(expected), "{text}");
        }
        for text in [
            "February 30",
            "April 31",
            "3/4",
            "14/14",
            "0/14",
            "13/40",
            "-1/14",
            "2월 30일",
            "삼월 삼십이일",
            "March 0",
            "March 32",
            "3/14/2000",
            "March 3 please dance",
            "when is my birthday?",
        ] {
            assert_eq!(birthday(text), None, "{text}");
        }
        for month in 1..=12 {
            for day in 1..=31 {
                let date = Birthday { month, day };
                if date.valid() {
                    assert!(!date.words().chars().any(char::is_numeric));
                    assert_eq!(birthday(&date.korean()), Some(date));
                }
            }
        }
    }

    #[test]
    fn short_free_answers_keep_spelling_and_remove_filler() {
        for (text, expected) in [
            ("I like cats", "cats"),
            ("my favorite food is kimchi", "kimchi"),
            ("고양이 좋아해", "고양이"),
            ("고양이를 좋아해", "고양이"),
            ("떡볶이", "떡볶이"),
            ("파파야", "파파야"),
            ("I love Red pandas!", "Red pandas"),
            ("나는 김치를 좋아해", "김치"),
        ] {
            assert_eq!(free_text(text).as_deref(), Some(expected), "{text}");
        }
        assert_eq!(free_text(&"가".repeat(24)), Some("가".repeat(24)));
        for text in [
            "",
            "\ncat",
            "a\u{1b}cat",
            "1234",
            "I don't know",
            "I'm tired",
            "hello",
            "what are you doing?",
            "open the window",
            "let's work for five minutes",
            "word game",
            "자장가",
            "do re mi",
            "벗어줘",
        ] {
            assert_eq!(free_text(text), None, "{text}");
        }
        assert_eq!(free_text(&"가".repeat(25)), None);
    }

    #[test]
    fn questions_use_the_world_random_source_and_only_unanswered_eligible_topics() {
        let mut selected = Vec::new();
        for seed in 0..40 {
            let mut world = visiting(seed);
            world.life.viewer.color = Some(Color::Green);
            world.life.viewer.season = Some(Season::Summer);
            let mut expected = world.clone();
            let index = expected.pick(&[1.0; 2]).unwrap();
            world.ask_viewer();
            let topic = world.asking.unwrap().0;
            assert_eq!(topic, [Topic::Animal, Topic::Food][index]);
            assert_eq!(world.random_unit(), expected.random_unit());
            selected.push(topic);
        }
        assert!(selected.contains(&Topic::Animal) && selected.contains(&Topic::Food));
        let mut world = visiting(9);
        world.life.viewer.color = Some(Color::Blue);
        world.life.viewer.season = Some(Season::Summer);
        world.life.viewer.animal = Some("cats".to_owned());
        world.life.viewer.food = Some("kimchi".to_owned());
        world.ask_viewer();
        assert!(world.asking.is_none());
        world.life.bond = 0.5;
        world.ask_viewer();
        assert_eq!(world.asking.unwrap().0, Topic::Birthday);
    }

    #[test]
    fn asking_waits_for_presence_typing_history_bond_and_a_quiet_settled_visit() {
        for condition in 0..13 {
            let mut world = visiting(3);
            match condition {
                0 => world.life.bond = 0.34,
                1 => world.viewer.active_at = -1000.0,
                2 => world.last_message_at = None,
                3 => world.last_message_at = Some(-581.0),
                4 => world.ghost.asleep = 0.5,
                5 => world.ghost.plan.arrived = None,
                6 => world.begin(Activity::Float),
                7 => {
                    world.start_focus(25);
                }
                8 => world.tell(Vec::new()),
                9 => world.game = Some(Game::new(glam::Vec2::ZERO, Mark::Cross, world.time)),
                10 => world.begin(Activity::Puzzle),
                11 => world.viewer.typing = true,
                12 => {
                    world.reading = Some(super::super::books::Reading {
                        writing: false,
                        index: 0,
                        next_at: None,
                        remaining: 1,
                        turned_at: None,
                        lighting: false,
                    })
                }
                _ => unreachable!(),
            }
            world.ask_viewer();
            assert!(world.asking.is_none(), "condition {condition}");
        }
        let mut world = visiting(3);
        world.ask_viewer();
        assert!(world.asking.is_some());
        assert!(
            world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::AskedViewer(_)))
        );
    }

    #[test]
    fn daily_limits_survive_restarts_and_curious_questions_are_spaced_out() {
        for curious in [0.5, 0.8] {
            let mut world = visiting(3);
            world.life.nature.as_mut().unwrap().curious = curious;
            world.ask_viewer();
            let first = world.asking.unwrap().0;
            world.hear("1234");
            assert!(world.asking.is_none());
            world.time += 30.0;
            world.notice_viewer();
            world.ask_viewer();
            assert!(world.asking.is_none());
            world.time += 601.0;
            world.last_message_at = Some(world.time);
            world.notice_viewer();
            world.ask_viewer();
            if curious >= 0.65 {
                assert!(world.asking.is_some());
                assert_ne!(world.asking.unwrap().0, first);
            } else {
                assert!(world.asking.is_none());
            }
            let saved = serde_json::to_string(&world.life).unwrap();
            let mut restored = visiting(3);
            restored.life = serde_json::from_str(&saved).unwrap();
            restored.ask_viewer();
            assert!(restored.asking.is_none());
            restored.date = restored.date.tomorrow().unwrap();
            restored.ask_viewer();
            assert!(restored.asking.is_some());
        }
    }

    #[test]
    fn answers_bypass_vocabulary_and_record_learning_and_both_birthday_milestones_once() {
        let mut world = visiting(5);
        let vocabulary = world.life.vocabulary.clone();
        let bond = world.life.bond;
        world.asking = Some((Topic::Birthday, world.time + 180.0));
        let heard = world.hear("March 3rd");
        assert_eq!(
            heard.understanding,
            Understanding::Answered(Topic::Birthday)
        );
        assert_eq!(world.life.vocabulary, vocabulary);
        assert_eq!(world.life.total.learned_about, 1);
        assert!(world.life.bond > bond);
        for text in ["You told me about yourself.", "I know your birthday now."] {
            assert_eq!(
                world
                    .life
                    .milestones
                    .iter()
                    .filter(|entry| entry.text == text)
                    .count(),
                1
            );
        }
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "March third! I'll remember."
        );
        let event = world
            .events
            .iter()
            .find(|event| matches!(event.kind, EventKind::LearnedAboutViewer(_)))
            .unwrap();
        assert_eq!(
            world.event_words(event),
            "learned that your birthday is on March third"
        );
        assert!(
            !world
                .events
                .iter()
                .any(|event| world.event_words(event).chars().any(char::is_numeric))
        );
        world.hear("March 3rd");
        assert_eq!(world.life.total.learned_about, 1);
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("learned a little more about you")
        );
    }

    #[test]
    fn immediate_replies_follow_the_answer_language() {
        for (text, reply) in [
            ("blue", "Blue! I'll remember."),
            ("파란색", "파란색! 기억할게."),
        ] {
            let mut world = visiting(5);
            world.asking = Some((Topic::Color, world.time + 180.0));
            assert_eq!(
                world.hear(text).understanding,
                Understanding::Answered(Topic::Color)
            );
            assert_eq!(world.life.viewer.color, Some(Color::Blue));
            assert_eq!(world.ghost.speech.as_ref().unwrap().text, reply);
        }
        let mut world = visiting(5);
        world.life.korean = 1.0;
        world.ask_viewer();
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            world.asking.unwrap().0.question(true)
        );
    }

    #[test]
    fn questions_and_existing_requests_end_waiting_and_follow_their_normal_routes() {
        for (text, expected) in [
            ("what's my favorite color?", Question::Viewer(Topic::Color)),
            ("내 생일 언제야?", Question::Viewer(Topic::Birthday)),
        ] {
            let mut world = visiting(5);
            world.asking = Some((Topic::Animal, world.time + 180.0));
            assert!(
                matches!(world.hear(text).understanding, Understanding::Asked { question, .. } if question == expected)
            );
            assert!(world.asking.is_none());
            assert_eq!(world.life.total.learned_about, 0);
        }
        for text in [
            "open the window",
            "word game",
            "let's work for five minutes",
            "play a lullaby",
            "do re mi",
        ] {
            let mut world = visiting(5);
            world.asking = Some((Topic::Food, world.time + 180.0));
            assert!(
                !matches!(
                    world.hear(text).understanding,
                    Understanding::Answered(_) | Understanding::Unanswered(_)
                ),
                "{text}"
            );
            assert!(world.asking.is_none());
            assert!(world.life.viewer.food.is_none());
        }
    }

    #[test]
    fn unreadable_answers_leave_topics_open_and_timeouts_end_waiting() {
        for (text, reply) in [
            ("blurble", "Hmm, I'll ask again another day."),
            ("모르겠어", "음, 다음에 또 물어볼게."),
        ] {
            let mut world = visiting(5);
            world.asking = Some((Topic::Color, world.time + 180.0));
            assert_eq!(
                world.hear(text).understanding,
                Understanding::Unanswered(Topic::Color)
            );
            assert_eq!(world.ghost.speech.as_ref().unwrap().text, reply);
            assert!(world.life.viewer.color.is_none());
            assert!(world.asking.is_none());
        }
        let mut world = visiting(5);
        world.asking = Some((Topic::Color, world.time + 180.0));
        world.time += 181.0;
        assert!(!matches!(
            world.hear("blue").understanding,
            Understanding::Answered(_)
        ));
        assert!(world.asking.is_none());
        world.asking = Some((Topic::Color, world.time - 1.0));
        world.ask_viewer();
        assert!(world.asking.is_none());
    }

    #[test]
    fn saved_answers_round_trip_and_damaged_answers_are_dropped() {
        let life = Life {
            viewer: Acquaintance {
                color: Some(Color::Purple),
                season: Some(Season::Autumn),
                animal: Some("고양이".to_owned()),
                food: Some("kimchi".to_owned()),
                birthday: Some(Birthday { month: 2, day: 29 }),
                last_asked: Some("2026-06-21".to_owned()),
                asked_today: vec![Topic::Birthday],
                ..Acquaintance::default()
            },
            ..Life::default()
        };
        assert_eq!(
            serde_json::from_str::<Life>(&serde_json::to_string(&life).unwrap())
                .unwrap()
                .sanitized(),
            life
        );
        let mut saved = serde_json::to_value(&life).unwrap();
        saved["viewer"]["color"] = serde_json::json!("invisible");
        saved["viewer"]["season"] = serde_json::json!("monsoon");
        saved["viewer"]["animal"] = serde_json::json!("x".repeat(25));
        saved["viewer"]["food"] = serde_json::json!("ki\nmchi");
        saved["viewer"]["birthday"] = serde_json::json!({"month":2,"day":30});
        saved["viewer"]["last_asked"] = serde_json::json!("not a day");
        let clean = serde_json::from_value::<Life>(saved).unwrap().sanitized();
        assert_eq!(clean.viewer, Acquaintance::default());
        assert_eq!(
            serde_json::from_str::<Life>("{}").unwrap().viewer,
            Acquaintance::default()
        );
    }

    #[test]
    fn birthday_priority_and_leap_day_fallback_follow_local_dates() {
        let mut world = visiting(5);
        world.life.viewer.birthday = Some(Birthday { month: 1, day: 1 });
        world.date = Date::constant(2027, 1, 1);
        assert_eq!(world.occasion(), Some(Occasion::ViewerBirthday));
        world.birthday = Some(Date::constant(2026, 1, 1));
        assert_eq!(world.occasion(), Some(Occasion::Birthday));
        world.birthday = None;
        world.date = Date::constant(2027, 1, 2);
        assert_eq!(world.occasion(), None);
        world.life.viewer.birthday = Some(Birthday { month: 2, day: 29 });
        for (date, expected) in [
            (Date::constant(2027, 2, 28), true),
            (Date::constant(2028, 2, 28), false),
            (Date::constant(2028, 2, 29), true),
            (Date::constant(2027, 3, 1), false),
        ] {
            world.date = date;
            assert_eq!(world.occasion() == Some(Occasion::ViewerBirthday), expected);
        }
    }

    #[test]
    fn birthday_greetings_wait_for_an_awake_present_viewer_and_happen_once_each_day() {
        for korean in [false, true] {
            for named in [false, true] {
                let mut world = visiting(5);
                world.life.viewer.birthday = Some(Birthday { month: 6, day: 21 });
                world.life.korean = if korean { 1.0 } else { 0.0 };
                world.life.viewer_name = named.then(|| "Sam".to_owned());
                world.ghost.asleep = 1.0;
                world.holiday_greeting();
                assert!(world.life.viewer.birthday_greeted.is_none());
                world.ghost.asleep = 0.0;
                world.viewer.active_at = -1000.0;
                world.holiday_greeting();
                assert!(world.life.viewer.birthday_greeted.is_none());
                world.notice_viewer();
                world.holiday_greeting();
                let speech = world.ghost.speech.as_ref().unwrap().text.clone();
                let expected = match (korean, named) {
                    (false, false) => "Happy birthday!",
                    (false, true) => "Happy birthday, Sam!",
                    (true, false) => "생일 축하해!",
                    (true, true) => "생일 축하해, Sam!",
                };
                assert_eq!(speech, expected);
                assert_eq!(world.ghost.plan.activity, Activity::Doodle);
                if let Some(super::super::Picture::Word(writing)) = &world.ghost.plan.picture {
                    assert_eq!(writing.text, if korean { "생일축하" } else { "HBD♥" });
                } else {
                    panic!("the birthday greeting should leave a word in the fog");
                }
                let said = world
                    .events
                    .iter()
                    .filter(|event| matches!(event.kind, EventKind::Said(_)))
                    .count();
                world.holiday_greeting();
                assert_eq!(
                    world
                        .events
                        .iter()
                        .filter(|event| matches!(event.kind, EventKind::Said(_)))
                        .count(),
                    said
                );
                // A birthday letter waits by the glass, once.
                assert_eq!(world.life.letters.len(), 1);
                assert!(world.unread_letter().is_some());
                let letter = &world.life.letters[0];
                assert_eq!(letter.korean, korean);
                assert!(letter.lines.iter().any(|line| line.contains(if korean {
                    "생일 축하해"
                } else {
                    "Happy birthday"
                })));
                let mut restored = visiting(5);
                restored.life = world.life.clone();
                restored.holiday_greeting();
                assert!(restored.ghost.speech.is_none());
                assert_eq!(restored.life.letters.len(), 1);
                world.celebrate();
                world.celebrate();
                assert_eq!(world.life.total.celebrations, 1);
                assert!(
                    world
                        .life
                        .today
                        .words()
                        .to_lowercase()
                        .contains("it was your birthday")
                );
                assert_eq!(
                    world.decoration().unwrap().occasion,
                    Occasion::ViewerBirthday
                );
            }
        }
    }

    #[test]
    fn favorite_season_is_greeted_only_on_its_first_morning() {
        let mut world = visiting(5);
        world.life.viewer.season = Some(Season::Autumn);
        world.date = Date::constant(2026, 9, 1);
        world.hour = 8.0;
        world.greet_favorite_season();
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "Your favorite season is here!"
        );
        world.time += 30.0;
        world.greet_favorite_season();
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::Said(_)))
                .count(),
            1
        );
        world.life.viewer.season_greeted = None;
        world.date = Date::constant(2026, 9, 2);
        world.greet_favorite_season();
        assert!(world.life.viewer.season_greeted.is_none());
    }
}
