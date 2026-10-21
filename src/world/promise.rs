use jiff::civil::Date;
use serde::{Deserialize, Deserializer, Serialize};

use super::{Activity, EventKind, Heard, Listening, Mood, Tone, Understanding, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Return {
    Later,
    Tomorrow,
    Days(u32),
    Weekend,
    Week,
    Weeks(u32),
    Month,
}

impl Return {
    pub fn trip(self) -> bool {
        !matches!(self, Self::Later | Self::Tomorrow)
    }

    pub fn words(self) -> String {
        match self {
            Self::Later => "later today".to_owned(),
            Self::Tomorrow => "tomorrow".to_owned(),
            Self::Days(1) => "in a day".to_owned(),
            Self::Days(days) => format!("in {} days", super::friend::count_words(days)),
            Self::Weekend => "by Sunday".to_owned(),
            Self::Week => "in a week".to_owned(),
            Self::Weeks(1) => "in a week".to_owned(),
            Self::Weeks(weeks) => format!("in {} weeks", super::friend::count_words(weeks)),
            Self::Month => "in a month".to_owned(),
        }
    }

    pub fn korean(self) -> String {
        match self {
            Self::Later => "이따".to_owned(),
            Self::Tomorrow => "내일".to_owned(),
            Self::Days(days) => format!("{} 뒤에", korean_days(days)),
            Self::Weekend => "일요일까지".to_owned(),
            Self::Week => "일주일 뒤에".to_owned(),
            Self::Weeks(weeks) => format!("{} 주 뒤에", korean_number(weeks)),
            Self::Month => "한 달 뒤에".to_owned(),
        }
    }
}

fn korean_number(number: u32) -> String {
    let ones = ["", "일", "이", "삼", "사", "오", "육", "칠", "팔", "구"];
    let tens = ["", "십", "이십", "삼십", "사십", "오십", "육십"];
    format!(
        "{}{}",
        tens[(number / 10) as usize],
        ones[(number % 10) as usize]
    )
}

fn korean_days(days: u32) -> String {
    match days {
        1 => "하루".to_owned(),
        2 => "이틀".to_owned(),
        3 => "사흘".to_owned(),
        4 => "나흘".to_owned(),
        _ => format!("{}일", korean_number(days)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Promise {
    pub kind: Return,
    #[serde(with = "date")]
    pub back_by: Date,
    #[serde(with = "date")]
    pub said: Date,
}

mod date {
    use super::*;
    use serde::Serializer;

    pub fn serialize<S: Serializer>(date: &Date, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&date.to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Date, D::Error> {
        let text = String::deserialize(deserializer)?;
        let date: Date = text.parse().map_err(serde::de::Error::custom)?;
        if date.to_string() != text {
            return Err(serde::de::Error::custom("invalid promise date"));
        }
        Ok(date)
    }
}

// A damaged optional promise must not discard the rest of a saved life.
pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Promise>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

impl Promise {
    pub fn valid(self) -> bool {
        let days = self.back_by.since(self.said).map(|span| span.get_days());
        days.is_ok_and(|days| (0..=60).contains(&days))
            && match self.kind {
                Return::Days(days) => (1..=60).contains(&days),
                Return::Weeks(weeks) => (1..=8).contains(&weeks),
                _ => true,
            }
    }

    pub fn holds(self, today: Date) -> bool {
        self.valid()
            && today >= self.said
            && today
                <= self
                    .back_by
                    .checked_add(jiff::Span::new().days(1))
                    .unwrap_or(self.back_by)
    }

    pub fn words(self) -> String {
        let date = super::acquaintance::Birthday {
            month: self.back_by.month(),
            day: self.back_by.day(),
        }
        .words();
        format!(
            "The viewer said they'd return {}, by {date}.",
            self.kind.words()
        )
    }
}

/// Weekend means the end of the coming Sunday (including today on Sunday).
/// Trips without a duration mean one week. Days are capped at sixty; longer
/// week counts become sixty days. A month uses the same day next month,
/// constrained to its last day, and never exceeds thirty-one days.
pub fn promised(text: &str, today: Date, hour: f32) -> Option<Promise> {
    if !hour.is_finite()
        || !(0.0..24.0).contains(&hour)
        || text.contains(['?', '？', '"', '“', '”'])
    {
        return None;
    }
    let text = text.trim().to_lowercase().replace('’', "'");
    let words: Vec<&str> = text
        .split(|character: char| !character.is_alphanumeric() && !matches!(character, '\'' | '-'))
        .filter(|word| !word.is_empty())
        .collect();
    let korean_text: String = text
        .trim_end_matches(['.', '!'])
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let kind = english(&words).or_else(|| korean(&korean_text))?;
    let back_by = match kind {
        Return::Month => today.checked_add(jiff::Span::new().months(1)).ok()?,
        Return::Weekend => {
            let days = (7 - today.weekday().to_monday_one_offset()) % 7;
            today
                .checked_add(jiff::Span::new().days(i32::from(days)))
                .ok()?
        }
        _ => {
            let days = match kind {
                Return::Later => 0,
                Return::Tomorrow => 1,
                Return::Days(days) => days,
                Return::Week => 7,
                Return::Weeks(weeks) => weeks * 7,
                _ => unreachable!(),
            };
            today
                .checked_add(jiff::Span::new().days(days as i32))
                .ok()?
        }
    };
    let promise = Promise {
        kind,
        back_by,
        said: today,
    };
    promise.valid().then_some(promise)
}

fn number(word: &str) -> Option<u32> {
    let count = match word {
        "a" | "an" | "one" | "한" | "일" => 1,
        "two" | "두" | "이" => 2,
        "three" | "세" | "삼" => 3,
        "four" | "네" | "사" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        "eleven" => 11,
        "twelve" => 12,
        _ => word.parse().ok()?,
    };
    (count > 0).then_some(count)
}

fn duration(words: &[&str]) -> Option<Return> {
    match words {
        [count, "day" | "days"] => Some(Return::Days(number(count)?.min(60))),
        ["a" | "one", "week"] => Some(Return::Week),
        [count, "week" | "weeks"] => match number(count)? {
            weeks @ 1..=8 => Some(Return::Weeks(weeks)),
            _ => Some(Return::Days(60)),
        },
        ["a" | "one", "month"] => Some(Return::Month),
        _ => None,
    }
}

fn returning(words: &[&str]) -> Option<Return> {
    match words {
        ["tomorrow"] => Some(Return::Tomorrow),
        ["later"] | ["later", "today"] => Some(Return::Later),
        ["this" | "next", "weekend"] => Some(Return::Weekend),
        ["next", "week"] => Some(Return::Week),
        ["in", rest @ ..] => duration(rest),
        _ => None,
    }
}

fn english(mut words: &[&str]) -> Option<Return> {
    if let ["goodnight" | "goodbye" | "bye", rest @ ..] = words {
        words = rest;
    }
    match words {
        ["see", "you", rest @ ..]
        | ["back", rest @ ..]
        | ["be", "back", rest @ ..]
        | ["i'll", "be", "back", rest @ ..]
        | ["i", "will", "be", "back", rest @ ..] => returning(rest),
        ["i", "won't", "be", "here", "this", "weekend"]
        | ["i", "will", "not", "be", "here", "this", "weekend"] => Some(Return::Weekend),
        _ => {
            let away = match words {
                ["i'm", "going", "away", rest @ ..]
                | ["i", "am", "going", "away", rest @ ..]
                | ["i'll", "be", "away", rest @ ..]
                | ["i", "will", "be", "away", rest @ ..]
                | ["going", "away", rest @ ..]
                | ["vacation", rest @ ..]
                | ["on", "vacation", rest @ ..]
                | ["i'm", "on", "vacation", rest @ ..]
                | ["going", "on", "a", "trip", rest @ ..]
                | ["i'm", "going", "on", "a", "trip", rest @ ..]
                | ["i", "am", "going", "on", "a", "trip", rest @ ..] => rest,
                _ => return None,
            };
            match away {
                [] => Some(Return::Week),
                ["for", rest @ ..] => duration(rest),
                _ => None,
            }
        }
    }
}

fn korean(mut text: &str) -> Option<Return> {
    for prefix in ["나는", "저는", "나", "저"] {
        if let Some(rest) = text.strip_prefix(prefix) {
            // Do not strip the first syllable of 나중에.
            if !text.starts_with("나중에") {
                text = rest;
            }
            break;
        }
    }
    let text = text.strip_suffix('요').unwrap_or(text);
    match text {
        "내일봐" | "내일만나" => return Some(Return::Tomorrow),
        "이따봐" | "나중에봐" => return Some(Return::Later),
        "다음주에봐" | "다음주에만나" => return Some(Return::Week),
        "주말에못와" => return Some(Return::Weekend),
        "휴가다녀올게" | "출장가" | "여행가" => return Some(Return::Week),
        _ => {}
    }
    let duration = ["뒤에올게", "후에올게", "동안여행가", "동안출장가"]
        .into_iter()
        .find_map(|ending| text.strip_suffix(ending))?;
    match duration {
        "하루" => Some(Return::Days(1)),
        "이틀" => Some(Return::Days(2)),
        "사흘" => Some(Return::Days(3)),
        "나흘" => Some(Return::Days(4)),
        "일주일" | "한주" => Some(Return::Week),
        "한달" => Some(Return::Month),
        _ => {
            if let Some(count) = duration.strip_suffix('일') {
                Some(Return::Days(number(count)?.min(60)))
            } else if let Some(count) = duration.strip_suffix('주') {
                let weeks = number(count)?;
                Some(if weeks <= 8 {
                    Return::Weeks(weeks)
                } else {
                    Return::Days(60)
                })
            } else {
                None
            }
        }
    }
}

impl World {
    pub(super) fn hear_promise(&mut self, text: &str, korean: bool) -> Option<Heard> {
        let promise = promised(text, self.date, self.hour)?;
        self.notice_viewer();
        self.listening = Listening::default();
        self.life.begin_day(&self.date.to_string());
        self.life.promise = Some(promise);
        self.promise_said = Some(korean);
        self.promise_departed = false;
        self.quiet_routine();
        self.record(EventKind::Heard(text.to_owned()));
        self.record(EventKind::Promised(promise.kind));
        if self.ghost.asleep < 0.3 {
            if self.focus.is_none() && self.telling.is_none() {
                self.begin(Activity::Visit);
                self.ghost.plan.length = 6.0;
            }
            let nature = self.life.temperament();
            let words = if korean {
                promise.kind.korean()
            } else {
                promise.kind.words()
            };
            let language = |english: &str, hangul: &str| {
                if korean {
                    hangul.to_owned()
                } else {
                    english.to_owned()
                }
            };
            let lines = if promise.kind.trip() {
                vec![
                    language(
                        "Have a wonderful trip. I'll keep the room cozy.",
                        "여행 잘 다녀와. 방을 따뜻하게 해 둘게.",
                    ),
                    if korean {
                        format!("잘 다녀와! {words} 기다릴게.")
                    } else {
                        format!("Safe travels! I'll be here {words}.")
                    },
                    language(
                        "I'll look after our little room. Come home safely.",
                        "작은 방은 내가 돌볼게. 잘 다녀와.",
                    ),
                ]
            } else {
                vec![
                    if korean {
                        format!("{words} 봐! 여기서 기다릴게.")
                    } else {
                        format!("See you {words}! I'll be right here.")
                    },
                    language(
                        "I'll save our cozy spot. See you soon!",
                        "우리 자리를 포근하게 남겨 둘게. 또 봐!",
                    ),
                    language(
                        "Bye for now. I'll have a game ready!",
                        "잘 가. 같이 놀 준비 하고 있을게!",
                    ),
                ]
            };
            let index = self
                .pick(&[1.0 + nature.shy, 1.0 + nature.tender, 1.0 + nature.playful])
                .unwrap();
            self.say(&lines[index]);
        }
        Some(Heard {
            tone: Some(Tone::Farewell),
            understanding: Understanding::Promised(promise.kind),
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    /// Resolves only a real arrival, never the viewer's continued presence
    /// immediately after saying goodbye. Sleeping company stays asleep.
    pub(super) fn return_to_promise(&mut self) -> bool {
        if !self.promise_departed {
            return false;
        }
        let Some(promise) = self.life.promise.take() else {
            return false;
        };
        self.promise_said = None;
        let kept = promise.holds(self.date);
        self.record(if kept {
            EventKind::PromiseKept(promise.kind)
        } else {
            EventKind::PromiseLate
        });
        if self.ghost.asleep < 0.3 {
            if self.focus.is_none() && self.telling.is_none() {
                self.begin(Activity::Visit);
            }
            let line = if kept {
                self.suggest_mood(Mood::Excited, 12.0);
                self.ghost.mood = Mood::Excited;
                self.spoken("You came back, just like you said!", "약속대로 와 줬구나!")
            } else {
                self.suggest_mood(Mood::Cheerful, 8.0);
                self.spoken("You're back! I missed you.", "돌아왔구나! 보고 싶었어.")
            };
            self.say(line);
        }
        true
    }

    pub fn promised_this_session(&self) -> Option<(Promise, bool)> {
        self.life.promise.zip(self.promise_said)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Nature};

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    #[test]
    fn english_and_korean_departures_choose_a_return_date_without_a_model() {
        for (text, kind, days) in [
            ("see you tomorrow", Return::Tomorrow, 1),
            ("SEE YOU LATER!", Return::Later, 0),
            ("back tomorrow", Return::Tomorrow, 1),
            ("I'll be back in three days", Return::Days(3), 3),
            ("I’ll be back in twelve days.", Return::Days(12), 12),
            ("I'm going away for a week", Return::Week, 7),
            ("I'll be away for two weeks", Return::Weeks(2), 14),
            ("going on a trip", Return::Week, 7),
            ("vacation for 5 days", Return::Days(5), 5),
            ("I won't be here this weekend", Return::Weekend, 4),
            ("see you next week", Return::Week, 7),
            ("be back in a month", Return::Month, 31),
            ("goodnight, see you tomorrow", Return::Tomorrow, 1),
            ("내일 봐", Return::Tomorrow, 1),
            ("내일 만나", Return::Tomorrow, 1),
            ("이따 봐", Return::Later, 0),
            ("나중에 봐", Return::Later, 0),
            ("3일 뒤에 올게", Return::Days(3), 3),
            ("사흘 뒤에 올게", Return::Days(3), 3),
            ("일주일 동안 여행 가", Return::Week, 7),
            ("다음 주에 봐", Return::Week, 7),
            ("한 달 뒤에 올게", Return::Month, 31),
            ("주말에 못 와", Return::Weekend, 4),
            ("휴가 다녀올게", Return::Week, 7),
            ("출장 가", Return::Week, 7),
            ("나는 이틀 뒤에 올게요!", Return::Days(2), 2),
            ("나중에 봐요!", Return::Later, 0),
            ("두 주 뒤에 올게", Return::Weeks(2), 14),
        ] {
            let today = date("2026-10-21");
            let promise = promised(text, today, 20.0).unwrap_or_else(|| panic!("{text}"));
            assert_eq!(promise.kind, kind, "{text}");
            assert_eq!(promise.said, today);
            assert_eq!(
                promise.back_by,
                today.checked_add(jiff::Span::new().days(days)).unwrap(),
                "{text}"
            );
        }
    }

    #[test]
    fn every_number_word_and_bounded_duration_is_computed_in_code() {
        for (index, word) in [
            "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
            "eleven", "twelve",
        ]
        .into_iter()
        .enumerate()
        {
            let promise = promised(
                &format!("I'll be back in {word} days"),
                date("2026-01-01"),
                12.0,
            )
            .unwrap();
            assert_eq!(promise.kind, Return::Days(index as u32 + 1));
        }
        for (text, days) in [
            ("back in 60 days", 60),
            ("back in 90 days", 60),
            ("back in 12 weeks", 60),
            ("60일 뒤에 올게", 60),
            ("99일 뒤에 올게", 60),
        ] {
            let promise = promised(text, date("2026-01-01"), 12.0).unwrap();
            assert_eq!(
                promise.back_by,
                date("2026-01-01")
                    .checked_add(jiff::Span::new().days(days))
                    .unwrap()
            );
            assert!(promise.valid());
        }
    }

    #[test]
    fn questions_negations_other_subjects_and_unrelated_time_words_are_not_promises() {
        for text in [
            "are you back tomorrow?",
            "are you back tomorrow",
            "I won't see you tomorrow?",
            "I won't see you tomorrow",
            "I'll not be back tomorrow",
            "not back tomorrow",
            "don't see you later",
            "will you be back in three days",
            "see you tomorrow?",
            "he'll be back tomorrow",
            "she is going on a trip",
            "I like tomorrow",
            "call you later",
            "see you tomorrow if I can",
            "I'm not going away for a week",
            "vacation for 0 days",
            "back in -3 days",
            "I'll be away for zero weeks",
            "see you in two months",
            "내일 봐?",
            "내일 만나？",
            "내일 안 와",
            "내일 못 봐",
            "내일 볼까",
            "사흘 뒤에 안 올게",
            "여행 안 가",
            "출장 안 가",
            "친구가 출장 가",
            "주말에 올 수 있어?",
            "한 달 뒤에 올까",
            "0일 뒤에 올게",
            "3.5일 뒤에 올게",
            "-3일 뒤에 올게",
            "\"see you tomorrow\"",
        ] {
            assert!(promised(text, date("2026-10-21"), 20.0).is_none(), "{text}");
        }
    }

    #[test]
    fn month_ends_leap_days_year_ends_and_weekends_have_local_deadlines() {
        for (text, today, expected) in [
            ("see you tomorrow", "2026-12-31", "2027-01-01"),
            ("back in three days", "2026-01-30", "2026-02-02"),
            ("back in a month", "2026-01-31", "2026-02-28"),
            ("한 달 뒤에 올게", "2028-01-31", "2028-02-29"),
            ("back in a month", "2026-03-31", "2026-04-30"),
            ("back in a month", "2026-12-31", "2027-01-31"),
            ("see you tomorrow", "2028-02-28", "2028-02-29"),
            ("see you this weekend", "2026-10-23", "2026-10-25"),
            ("주말에 못 와", "2026-10-24", "2026-10-25"),
            ("see you this weekend", "2026-10-25", "2026-10-25"),
            ("see you this weekend", "2026-10-26", "2026-11-01"),
        ] {
            assert_eq!(
                promised(text, date(today), 23.9).unwrap().back_by,
                date(expected),
                "{text}, {today}"
            );
        }
        for hour in [f32::NAN, f32::INFINITY, -1.0, 24.0] {
            assert!(promised("see you later", date("2026-10-21"), hour).is_none());
        }
    }

    #[test]
    fn promises_survive_serialization_and_damaged_optional_data_is_discarded() {
        let promise = promised("back in a week", date("2026-10-21"), 12.0).unwrap();
        let life = Life {
            day: "2026-10-21".to_owned(),
            promise: Some(promise),
            ..Life::default()
        };
        let saved = serde_json::to_value(&life).unwrap();
        let loaded: Life = serde_json::from_value(saved.clone()).unwrap();
        assert_eq!(loaded.sanitized().promise, Some(promise));
        for damaged in [
            serde_json::json!({"kind":"Tomorrow", "said":"damaged", "back_by":"2026-10-22"}),
            serde_json::json!({"kind":"Tomorrow", "said":"2026-02-30", "back_by":"2026-10-22"}),
            serde_json::json!({"kind":"Tomorrow", "said":"2026-10-21", "back_by":"2026-10-20"}),
            serde_json::json!({"kind":"Tomorrow", "said":"2026-10-21", "back_by":"2027-10-22"}),
            serde_json::json!({"kind":{"Days":0}, "said":"2026-10-21", "back_by":"2026-10-22"}),
            serde_json::json!({"kind":"Unknown", "said":"2026-10-21", "back_by":"2026-10-22"}),
            serde_json::json!({"kind":"Tomorrow", "said":"2026-10-22", "back_by":"2026-10-23"}),
        ] {
            let mut saved = saved.clone();
            saved["promise"] = damaged;
            let loaded: Life = serde_json::from_value(saved).unwrap();
            assert!(loaded.sanitized().promise.is_none());
        }
        assert!(
            serde_json::from_str::<Life>("{}")
                .unwrap()
                .promise
                .is_none()
        );
    }

    #[test]
    fn a_held_promise_protects_the_bond_and_only_overdue_time_fades() {
        let promise = promised("back in a week", date("2026-10-01"), 12.0).unwrap();
        let initial = Life {
            bond: 0.8,
            closest: 0.8,
            promise: Some(promise),
            nature: Some(Nature::default()),
            ..Life::default()
        };
        for day in ["2026-10-01", "2026-10-08", "2026-10-09"] {
            let mut life = initial.clone();
            life.day = day.to_owned();
            life.miss_at(9.0 * 86400.0, 23.9);
            assert_eq!(life.bond, initial.bond);
            assert!(life.alone > 0.0);
        }
        let mut late = initial.clone();
        late.day = "2026-10-10".to_owned();
        late.miss_at(9.0 * 86400.0, 12.0);
        let mut ordinary = initial;
        ordinary.promise = None;
        ordinary.alone = 86400.0;
        ordinary.miss(43200.0);
        assert!((late.bond - ordinary.bond).abs() < 0.000001);
        let before = late.bond;
        late.miss_at(3600.0, 13.0);
        assert!(late.bond < before);
    }

    #[test]
    fn hearing_a_promise_teaches_nothing_replaces_it_and_keeps_sleep_and_work() {
        for text in ["see you tomorrow", "내일 봐", "I'll be away for two weeks"] {
            let mut world = World::new(11, 12.0);
            let vocabulary = world.life.vocabulary.clone();
            let heard = world.hear(text);
            assert!(matches!(heard.understanding, Understanding::Promised(_)));
            assert_eq!(heard.tone, Some(Tone::Farewell));
            assert!(heard.learned.is_empty());
            assert_eq!(world.life.vocabulary, vocabulary);
            assert_eq!(world.life.today.promises_made, 1);
            assert_eq!(world.ghost.plan.activity, Activity::Visit);
            assert!(world.ghost.speech.is_some());
            assert_eq!(
                world
                    .ghost
                    .speech
                    .as_ref()
                    .unwrap()
                    .text
                    .chars()
                    .any(|character| ('가'..='힣').contains(&character)),
                heard.korean
            );
            world.hear("back in three days");
            assert_eq!(world.life.promise.unwrap().kind, Return::Days(3));
            assert_eq!(world.life.total.promises_made, 2);
        }
        let mut sleeping = World::new(12, 23.0);
        sleeping.begin(Activity::Rest);
        sleeping.ghost.asleep = 1.0;
        sleeping.hear("goodnight, see you tomorrow");
        assert_eq!(sleeping.ghost.asleep, 1.0);
        assert_eq!(sleeping.ghost.plan.activity, Activity::Rest);
        assert!(sleeping.ghost.speech.is_none());
        let mut working = World::new(12, 12.0);
        working.start_focus(25);
        working.hear("see you tomorrow");
        assert_eq!(working.ghost.plan.activity, Activity::Company);
    }

    #[test]
    fn saying_goodbye_is_not_an_arrival_but_returning_later_that_day_is() {
        let mut world = World::new(7, 12.0);
        world.notice_viewer();
        world.hear("see you later");
        world.step(0.1);
        assert!(world.life.promise.is_some());
        assert_eq!(world.life.total.promises_kept, 0);
        world.look_away(true);
        world.step(330.0);
        world.notice_viewer();
        world.step(0.1);
        assert!(world.life.promise.is_none());
        assert_eq!(world.life.total.promises_kept, 1);
        assert_eq!(world.ghost.mood, Mood::Excited);
        assert!(
            world
                .ghost
                .speech
                .as_ref()
                .unwrap()
                .text
                .contains("just like you said")
        );
        world.step(0.1);
        assert_eq!(world.life.total.promises_kept, 1);
    }

    #[test]
    fn early_and_grace_day_returns_keep_the_promise_and_late_arrivals_do_not_cool_it() {
        for (returned, kept) in [
            ("2026-10-21", true),
            ("2026-10-28", true),
            ("2026-10-29", true),
            ("2026-10-30", false),
        ] {
            let mut world = World::new(6, 12.0);
            world.date = date(returned);
            world.life.begin_day(returned);
            world.life.promise = promised("back in a week", date("2026-10-21"), 12.0);
            let bond = world.life.bond;
            world.notice_viewer();
            world.step(0.1);
            assert!(world.life.promise.is_none());
            assert_eq!(world.life.total.promises_kept, u32::from(kept));
            assert_eq!(
                world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::PromiseLate),
                !kept
            );
            assert!(world.life.bond >= bond);
            world.notice_viewer();
            world.step(0.1);
            assert_eq!(world.life.total.promises_kept, u32::from(kept));
        }
    }

    #[test]
    fn a_sleeping_return_records_once_without_waking_or_speaking() {
        let mut world = World::new(6, 23.0);
        world.life.promise = promised("see you later", world.date, 20.0);
        world.begin(Activity::Rest);
        world.ghost.asleep = 1.0;
        world.notice_viewer();
        world.step(0.1);
        assert_eq!(world.life.total.promises_kept, 1);
        assert!(world.ghost.asleep > 0.9);
        assert!(world.ghost.speech.is_none());
    }

    #[test]
    fn kept_promises_have_bounded_warmth_and_once_only_milestones_and_diary_deeds() {
        let mut everyday = Life::default();
        everyday.note(&EventKind::Promised(Return::Tomorrow));
        everyday.note(&EventKind::PromiseKept(Return::Tomorrow));
        assert!((everyday.bond - 0.01).abs() < 0.000001);
        assert!(
            everyday
                .today
                .words()
                .contains("you came back when you said you would")
        );
        let mut trip = Life::default();
        trip.note(&EventKind::PromiseKept(Return::Week));
        assert!((trip.bond - 0.02).abs() < 0.000001);
        assert!(
            trip.today
                .words()
                .to_lowercase()
                .contains("you came home from your trip")
        );
        trip.note(&EventKind::PromiseKept(Return::Days(3)));
        assert_eq!(trip.milestones.len(), 2);
        assert_eq!(
            trip.milestones[0].text,
            "You came back when you said you would."
        );
        assert_eq!(trip.milestones[1].text, "You came home from a trip.");
        let bond = trip.bond;
        trip.note(&EventKind::PromiseLate);
        assert_eq!(trip.bond, bond);
    }

    #[test]
    fn a_promise_takes_precedence_over_an_answer_to_an_acquaintance_question() {
        let mut world = World::new(6, 12.0);
        world.asking = Some((crate::world::acquaintance::Topic::Animal, 60.0));
        let heard = world.hear("going on a trip");
        assert_eq!(heard.understanding, Understanding::Promised(Return::Week));
        assert!(world.life.viewer.animal.is_none());
    }
}
