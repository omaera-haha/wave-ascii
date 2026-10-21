use jiff::civil::Date;

use super::{Activity, Entry, EventKind, Life, Occasion, World, friend::count_words};

/// Korean day counting includes the day of birth as day one.
pub fn day_of_life(born: Date, today: Date) -> Option<u32> {
    if today < born {
        return None;
    }
    u32::try_from((today - born).get_days())
        .ok()?
        .checked_add(1)
}

pub fn landmark(born: Date, today: Date) -> Option<Occasion> {
    match day_of_life(born, today)? {
        100 => Some(Occasion::HundredDays),
        1000 => Some(Occasion::ThousandDays),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Span {
    Week,
    Month,
    Year(u32),
}

impl Span {
    pub fn ago(self) -> String {
        match self {
            Self::Week => "A week ago today".to_owned(),
            Self::Month => "A month ago today".to_owned(),
            Self::Year(1) => "A year ago today".to_owned(),
            Self::Year(years) => {
                let words = count_words(years);
                format!("{} years ago today", capitalized(&words))
            }
        }
    }

    fn elapsed(self) -> String {
        match self {
            Self::Week => "one week".to_owned(),
            Self::Month => "one month".to_owned(),
            Self::Year(1) => "one year".to_owned(),
            Self::Year(years) => format!("{} years", count_words(years)),
        }
    }

    fn korean(self) -> &'static str {
        match self {
            Self::Week => "있잖아, 일주일 전 오늘 우리한테 특별한 일이 있었어.",
            Self::Month => "있잖아, 한 달 전 오늘 우리한테 특별한 일이 있었어.",
            Self::Year(1) => "있잖아, 일 년 전 오늘 우리한테 특별한 일이 있었어.",
            Self::Year(_) => "있잖아, 여러 해 전 오늘 우리한테 특별한 일이 있었어.",
        }
    }
}

/// Month anniversaries never clamp to a shorter month. Year anniversaries
/// of leap day fall on February twenty-eighth in common years.
pub fn span(then: Date, today: Date) -> Option<Span> {
    if today <= then {
        return None;
    }
    let years = i32::from(today.year()) - i32::from(then.year());
    let yearly_day = if then.month() == 2 && then.day() == 29 && !today.in_leap_year() {
        28
    } else {
        then.day()
    };
    if years > 0 && today.month() == then.month() && today.day() == yearly_day {
        return Some(Span::Year(years as u32));
    }
    let months = years * 12 + i32::from(today.month()) - i32::from(then.month());
    if months == 1 && today.day() == then.day() {
        return Some(Span::Month);
    }
    ((today - then).get_days() == 7).then_some(Span::Week)
}

fn lowercased(text: &str) -> String {
    let mut letters = text.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_lowercase().collect::<String>() + letters.as_str()
    })
}

fn capitalized(text: &str) -> String {
    let mut letters = text.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().collect::<String>() + letters.as_str()
    })
}

fn sentence(text: &str) -> String {
    let text = text.trim();
    if text.starts_with("I ") || text.starts_with("I'") {
        return text.to_owned();
    }
    let first = text.split_whitespace().next().unwrap_or("");
    let without_subject = [
        "Saw",
        "Learned",
        "Caught",
        "Played",
        "Danced",
        "Drew",
        "Slept",
        "Sat",
        "Celebrated",
        "Ate",
        "Met",
        "Built",
        "Wrote",
        "Found",
        "Finished",
        "Planted",
    ]
    .contains(&first);
    let text = lowercased(text);
    if without_subject {
        format!("I {text}")
    } else {
        text
    }
}

pub fn recalled(span: Span, text: &str) -> String {
    format!("{}, {}", span.ago(), sentence(text))
}

/// Relationship milestones come before first discoveries and routine counts.
fn rank(text: &str) -> Option<u8> {
    let words = text.to_lowercase();
    if text == "I came into being in a glass box."
        || (words.starts_with("ate my ") && words.contains("light mote"))
    {
        return None;
    }
    Some(if with_viewer(text) || words.contains("name") {
        3
    } else if words.contains("friend") || words.contains("another ghost") || words.contains("trust")
    {
        2
    } else if words.contains("first") {
        1
    } else {
        0
    })
}

pub fn with_viewer(text: &str) -> bool {
    let words = text.to_lowercase();
    [
        "you", "we ", "we'", "together", "fond", "close", "devoted", "bond",
    ]
    .iter()
    .any(|word| words.contains(word))
}

impl Life {
    pub fn anniversaries(&self, today: &str) -> Vec<(Span, &Entry)> {
        let Ok(today) = today.parse::<Date>() else {
            return Vec::new();
        };
        let mut entries: Vec<_> = self
            .milestones
            .iter()
            .filter_map(|entry| {
                rank(&entry.text)?;
                Some((span(entry.day.parse().ok()?, today)?, entry))
            })
            .collect();
        entries.sort_by(|(first_span, first), (second_span, second)| {
            second_span
                .cmp(first_span)
                .then_with(|| rank(&second.text).cmp(&rank(&first.text)))
        });
        entries
    }
}

impl World {
    pub fn day_of_life(&self) -> Option<u32> {
        day_of_life(self.birthday?, self.date)
    }

    pub fn anniversary_observations(&self) -> Vec<String> {
        self.life
            .anniversaries(&self.date.to_string())
            .into_iter()
            .map(|(span, entry)| {
                let sentence = sentence(&entry.text);
                let subject = if let Some(rest) = sentence.strip_prefix("I ") {
                    format!("it {rest}")
                } else if let Some(rest) = sentence.strip_prefix("you ") {
                    format!("the viewer {rest}")
                } else {
                    return format!(
                        "Today marks {} since this milestone: {}",
                        span.elapsed(),
                        entry.text
                    );
                };
                let subject = subject.replace("your ", "the viewer's ");
                format!("Today is {} since {subject}", span.elapsed())
            })
            .collect()
    }

    /// Remembers one milestone aloud only when an unoccupied visit has settled.
    pub fn share_anniversary(&mut self) {
        let day = self.date.to_string();
        if self.life.anniversary_said.as_ref() == Some(&day)
            || self.asking.is_some()
            || !self.viewer_present()
            || self.ghost.asleep >= 0.3
            || self.focus.is_some()
            || self.telling.is_some()
            || self.reading.is_some()
            || self.ghost.plan.activity != Activity::Visit
            || self.ghost.plan.arrived.is_none()
            || self.ghost.plan.done
            || self.game.is_some()
            || self.puzzle_active()
            || self
                .ghost
                .speech
                .as_ref()
                .is_some_and(|line| self.time - line.time < 12.0)
        {
            return;
        }
        let Some((span, entry)) = self.life.anniversaries(&day).into_iter().next() else {
            return;
        };
        let recalled = recalled(span, &entry.text);
        if self.speaks_korean() {
            self.say(span.korean());
            self.think(&recalled);
        } else {
            self.say(&format!("Guess what? {recalled}"));
        }
        self.life.anniversary_said = Some(day);
        self.record(EventKind::Remembered(recalled));
    }

    pub(super) fn greet_landmark(&mut self, occasion: Occasion) -> bool {
        let day = self.date.to_string();
        if self.life.landmark_greeted.as_ref() == Some(&day)
            || self.telling.is_some()
            || self.game.is_some()
            || self.puzzle_active()
            || self.reading.is_some()
            || self.asking.is_some()
            || self
                .ghost
                .speech
                .as_ref()
                .is_some_and(|line| self.time - line.time < 12.0)
        {
            return false;
        }
        let (english, korean, thanks, korean_thanks) = match occasion {
            Occasion::HundredDays => (
                "Today is my hundredth day!",
                "오늘은 내 백일이야!",
                "Thank you for every one of them.",
                "매일 함께해줘서 고마워.",
            ),
            Occasion::ThousandDays => (
                "Today is my thousandth day!",
                "오늘은 내 천일이야!",
                "Thank you for every one of them.",
                "매일 함께해줘서 고마워.",
            ),
            _ => return false,
        };
        self.say(&format!(
            "{} {}",
            self.spoken(english, korean),
            self.spoken(thanks, korean_thanks)
        ));
        self.deliver(
            vec![
                self.spoken(english, korean).to_owned(),
                self.spoken(thanks, korean_thanks).to_owned(),
                self.spoken(
                    "Let's share a little white rice cake.",
                    "하얀 백설기를 함께 나누자.",
                )
                .to_owned(),
            ],
            self.speaks_korean(),
        );
        self.life.landmark_greeted = Some(day);
        true
    }
}

/// Ordinals for the life panel, including compound days and long lives.
pub fn ordinal(day: u32) -> String {
    let words = count_words(day);
    let (prefix, last) = words.rsplit_once(' ').unwrap_or(("", &words));
    let ending = match last {
        "one" => "first".to_owned(),
        "two" => "second".to_owned(),
        "three" => "third".to_owned(),
        "five" => "fifth".to_owned(),
        "eight" => "eighth".to_owned(),
        "nine" => "ninth".to_owned(),
        "twelve" => "twelfth".to_owned(),
        word if word.ends_with('y') => format!("{}ieth", &word[..word.len() - 1]),
        word => format!("{word}th"),
    };
    if prefix.is_empty() {
        ending
    } else if [
        "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ]
    .iter()
    .any(|tens| prefix.ends_with(tens))
    {
        format!("{prefix}-{ending}")
    } else {
        format!("{prefix} {ending}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> Date {
        text.parse().unwrap()
    }

    #[test]
    fn days_of_life_include_birth_and_reject_future_birth_dates() {
        let born = date("2026-03-03");
        for (today, expected) in [
            ("2026-03-02", None),
            ("2026-03-03", Some(1)),
            ("2026-06-08", Some(98)),
            ("2026-06-09", Some(99)),
            ("2026-06-10", Some(100)),
            ("2026-06-11", Some(101)),
            ("2028-11-26", Some(1000)),
        ] {
            assert_eq!(day_of_life(born, date(today)), expected, "{today}");
        }
        assert_eq!(day_of_life(Date::MIN, Date::MAX), Some(7_304_484));
    }

    #[test]
    fn only_the_hundredth_and_thousandth_days_are_landmarks() {
        for born in [date("2026-03-03"), date("2024-02-29"), date("2026-12-31")] {
            for elapsed in 0..=1100 {
                let today = born.checked_add(jiff::Span::new().days(elapsed)).unwrap();
                let expected = match elapsed {
                    99 => Some(Occasion::HundredDays),
                    999 => Some(Occasion::ThousandDays),
                    _ => None,
                };
                assert_eq!(landmark(born, today), expected);
            }
        }
    }

    #[test]
    fn weeks_cross_months_leap_days_and_years_without_repeating() {
        for (then, today) in [
            ("2026-12-28", "2027-01-04"),
            ("2024-02-25", "2024-03-03"),
            ("2026-01-28", "2026-02-04"),
        ] {
            let then = date(then);
            let today = date(today);
            assert_eq!(span(then, today), Some(Span::Week));
            assert_eq!(span(today, then), None);
            assert_eq!(span(then, then), None);
            for elapsed in [1, 6, 8, 14, 21] {
                assert_eq!(
                    span(
                        then,
                        then.checked_add(jiff::Span::new().days(elapsed)).unwrap()
                    ),
                    None
                );
            }
        }
    }

    #[test]
    fn every_month_end_matches_only_existing_days_in_the_next_month() {
        for year in 2023..=2028 {
            for month in 1..=12 {
                let (next_year, next_month) = if month == 12 {
                    (year + 1, 1)
                } else {
                    (year, month + 1)
                };
                for day in 1..=31 {
                    let Ok(then) = Date::new(year, month, day) else {
                        continue;
                    };
                    for next_day in 1..=31 {
                        let Ok(today) = Date::new(next_year, next_month, next_day) else {
                            continue;
                        };
                        let expected = if day == next_day {
                            Some(Span::Month)
                        } else if (today - then).get_days() == 7 {
                            Some(Span::Week)
                        } else {
                            None
                        };
                        assert_eq!(span(then, today), expected, "{then} to {today}");
                    }
                }
            }
        }
        assert_eq!(span(date("2026-01-31"), date("2026-02-28")), None);
        assert_eq!(span(date("2024-01-31"), date("2024-02-29")), None);
        assert_eq!(span(date("2026-01-31"), date("2026-03-31")), None);
    }

    #[test]
    fn yearly_spans_keep_leap_days_and_use_february_twenty_eighth_in_common_years() {
        let leap = date("2024-02-29");
        for (today, expected) in [
            ("2024-03-29", Some(Span::Month)),
            ("2025-02-28", Some(Span::Year(1))),
            ("2025-03-01", None),
            ("2026-02-28", Some(Span::Year(2))),
            ("2028-02-28", None),
            ("2028-02-29", Some(Span::Year(4))),
            ("2100-02-28", Some(Span::Year(76))),
        ] {
            assert_eq!(span(leap, date(today)), expected);
        }
        for year in 2025..=2035 {
            assert_eq!(
                span(date("2024-12-31"), Date::new(year, 12, 31).unwrap()),
                Some(Span::Year((year - 2024) as u32))
            );
            assert_eq!(
                span(date("2024-02-28"), Date::new(year, 2, 28).unwrap()),
                Some(Span::Year((year - 2024) as u32))
            );
        }
    }

    #[test]
    fn recalled_lines_keep_i_add_missing_subjects_and_spell_out_years() {
        for (span, text, expected) in [
            (
                Span::Month,
                "I learned your name: Sam.",
                "A month ago today, I learned your name: Sam.",
            ),
            (
                Span::Week,
                "Saw my first bird.",
                "A week ago today, I saw my first bird.",
            ),
            (
                Span::Year(1),
                "You found me hiding.",
                "A year ago today, you found me hiding.",
            ),
            (
                Span::Year(2),
                "We got to know each other.",
                "Two years ago today, we got to know each other.",
            ),
            (
                Span::Year(21),
                "My first wish came true.",
                "Twenty one years ago today, my first wish came true.",
            ),
            (
                Span::Week,
                "I've grown fond of you.",
                "A week ago today, I've grown fond of you.",
            ),
            (
                Span::Month,
                "Drew my first picture on the glass.",
                "A month ago today, I drew my first picture on the glass.",
            ),
        ] {
            assert_eq!(recalled(span, text), expected);
        }
        assert_eq!(recalled(Span::Week, ""), "A week ago today, ");
    }

    fn milestone(day: &str, text: &str) -> Entry {
        Entry {
            day: day.to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn anniversary_order_prefers_years_then_months_then_weeks_and_relationships() {
        let life = Life {
            milestones: vec![
                milestone("2026-06-02", "Saw my first bird."),
                milestone("2026-05-09", "I built my first tower of blocks."),
                milestone(
                    "2025-06-09",
                    "I met another ghost tonight. Their name is Lumi.",
                ),
                milestone("2026-05-09", "I learned your name: Sam."),
                milestone("2026-05-09", "Ate my hundredth light mote."),
                milestone("2026-05-09", "I came into being in a glass box."),
                milestone("damaged", "We got to know each other."),
                milestone("2026-05-09", "I'm good at catching motes now."),
            ],
            ..Life::default()
        };
        let entries = life.anniversaries("2026-06-09");
        assert_eq!(
            entries.iter().map(|(span, _)| *span).collect::<Vec<_>>(),
            vec![
                Span::Year(1),
                Span::Month,
                Span::Month,
                Span::Month,
                Span::Week
            ]
        );
        assert_eq!(entries[1].1.text, "I learned your name: Sam.");
        assert!(life.anniversaries("damaged").is_empty());
        assert!(life.anniversaries("2026-06-10").is_empty());
    }

    fn visiting(korean: bool) -> World {
        let mut world = World::new(12, 12.0);
        world.date = date("2026-06-09");
        world.life.begin_day("2026-06-09");
        world.life.korean = if korean { 1.0 } else { 0.0 };
        world
            .life
            .milestones
            .push(milestone("2026-05-09", "I learned your name: Sam."));
        world.notice_viewer();
        world.begin(Activity::Visit);
        world.ghost.plan.arrived = Some(world.time);
        world
    }

    #[test]
    fn anniversary_speech_uses_the_viewers_language_counts_once_and_survives_restarts() {
        for korean in [false, true] {
            let mut world = visiting(korean);
            world.share_anniversary();
            let speech = &world.ghost.speech.as_ref().unwrap().text;
            assert_eq!(
                speech,
                if korean {
                    "있잖아, 한 달 전 오늘 우리한테 특별한 일이 있었어."
                } else {
                    "Guess what? A month ago today, I learned your name: Sam."
                }
            );
            if korean {
                assert_eq!(
                    world.ghost.thought.as_ref().unwrap().text,
                    "A month ago today, I learned your name: Sam."
                );
            }
            assert_eq!(world.life.today.anniversaries, 1);
            assert_eq!(world.life.total.anniversaries, 1);
            assert!(
                world
                    .life
                    .today
                    .diary()
                    .contains("a month ago today, when I learned your name")
            );
            let saved = serde_json::to_string(&world.life).unwrap();
            world.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
            world.ghost.speech = None;
            world.share_anniversary();
            assert!(world.ghost.speech.is_none());
            assert_eq!(world.life.total.anniversaries, 1);
            world.date = date("2027-05-09");
            world.share_anniversary();
            assert_eq!(world.life.total.anniversaries, 2);
        }
    }

    #[test]
    fn anniversaries_wait_for_an_awake_settled_unoccupied_visit_and_present_viewer() {
        for condition in 0..9 {
            let mut world = visiting(false);
            match condition {
                0 => world.ghost.asleep = 1.0,
                1 => {
                    world.begin(Activity::Window);
                }
                2 => world.ghost.plan.arrived = None,
                3 => world.ghost.plan.done = true,
                4 => world.time += 100.0,
                5 => {
                    world.say("Hello!");
                }
                6 => {
                    world.asking =
                        Some((super::super::acquaintance::Topic::Color, world.time + 180.0))
                }
                7 => {
                    world.toggle_focus();
                }
                _ => {
                    world.tell(vec!["A small story.".to_owned()]);
                }
            }
            world.share_anniversary();
            assert_eq!(world.life.total.anniversaries, 0, "condition {condition}");
            assert!(world.life.anniversary_said.is_none());
        }
    }

    #[test]
    fn landmark_greetings_deliver_one_valid_letter_per_saved_local_day() {
        for (elapsed, occasion) in [(99, Occasion::HundredDays), (999, Occasion::ThousandDays)] {
            for korean in [false, true] {
                let mut world = visiting(korean);
                world.birthday = Some(
                    world
                        .date
                        .checked_sub(jiff::Span::new().days(elapsed))
                        .unwrap(),
                );
                world.step(0.1);
                assert_eq!(world.occasion(), Some(occasion));
                assert_eq!(world.life.today.celebrations, 1);
                assert_eq!(world.life.letters.len(), 1);
                let greeting = match (elapsed, korean) {
                    (99, false) => "Today is my hundredth day! Thank you for every one of them.",
                    (99, true) => "오늘은 내 백일이야! 매일 함께해줘서 고마워.",
                    (_, false) => "Today is my thousandth day! Thank you for every one of them.",
                    (_, true) => "오늘은 내 천일이야! 매일 함께해줘서 고마워.",
                };
                assert_eq!(world.ghost.speech.as_ref().unwrap().text, greeting);
                assert!(world.life.letters[0].valid());
                assert_eq!(world.life.letters[0].korean, korean);
                assert!(world.life.milestones.iter().any(|entry| entry.text
                    == if elapsed == 99 {
                        "My hundredth day in the glass box."
                    } else {
                        "My thousandth day in the glass box."
                    }));
                let saved = serde_json::to_string(&world.life).unwrap();
                world.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
                world.ghost.speech = None;
                world.step(0.1);
                assert_eq!(world.life.letters.len(), 1);
                assert!(world.ghost.speech.is_none());
            }
        }
    }

    #[test]
    fn landmark_days_last_all_day_and_yield_to_birthdays_before_holidays() {
        for (elapsed, expected) in [(99, Occasion::HundredDays), (999, Occasion::ThousandDays)] {
            for today in [date("2027-01-01"), date("2026-10-31"), date("2026-06-10")] {
                let born = today.checked_sub(jiff::Span::new().days(elapsed)).unwrap();
                let mut world = World::new(3, 12.0);
                world.birthday = Some(born);
                world.date = today;
                for hour in [0.0, 12.0, 23.999] {
                    world.hour = hour;
                    assert_eq!(world.occasion(), Some(expected));
                    assert!(world.decoration().is_some());
                }
                world.life.viewer.birthday = Some(super::super::acquaintance::Birthday {
                    month: today.month(),
                    day: today.day(),
                });
                assert_eq!(world.occasion(), Some(Occasion::ViewerBirthday));
                world.birthday = Some(today.with().year(today.year() - 1).build().unwrap());
                assert_eq!(world.occasion(), Some(Occasion::Birthday));
                world.birthday = None;
                world.life.viewer.birthday = None;
                assert!(!matches!(
                    world.occasion(),
                    Some(Occasion::HundredDays | Occasion::ThousandDays)
                ));
            }
        }
    }

    #[test]
    fn old_saves_default_daily_guards_and_reject_damaged_dates() {
        let life: Life = serde_json::from_str("{}").unwrap();
        assert!(life.anniversary_said.is_none());
        assert!(life.landmark_greeted.is_none());
        assert_eq!(life.today.anniversaries, 0);
        let life = Life {
            anniversary_said: Some("bad".to_owned()),
            landmark_greeted: Some("bad".to_owned()),
            ..life
        }
        .sanitized();
        assert!(life.anniversary_said.is_none());
        assert!(life.landmark_greeted.is_none());
    }

    #[test]
    fn life_day_ordinals_are_words_for_compound_and_landmark_days() {
        for (day, words) in [
            (1, "first"),
            (12, "twelfth"),
            (20, "twentieth"),
            (42, "forty-second"),
            (100, "one hundredth"),
            (101, "one hundred first"),
            (142, "one hundred forty-second"),
            (1000, "one thousandth"),
        ] {
            assert_eq!(ordinal(day), words);
        }
    }
}
