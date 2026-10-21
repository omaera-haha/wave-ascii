use glam::Vec3;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{EventKind, Stage, World};

pub const POSITION: Vec3 = Vec3::new(0.0, 0.0, 1.12);
pub const HALF_WIDTH: f32 = 0.19;
pub const HALF_DEPTH: f32 = 0.12;
pub const KEPT: usize = 20;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Letter {
    pub day: String,
    pub lines: Vec<String>,
    pub korean: bool,
    pub read: bool,
    #[serde(default)]
    pub claude: bool,
    pub reply_to: Option<Box<super::penpal::ViewerLetter>>,
}

impl Letter {
    pub fn valid(&self) -> bool {
        self.day
            .parse::<Date>()
            .is_ok_and(|date| date.to_string() == self.day)
            && Self::valid_lines(&self.lines)
            && self.reply_to.as_ref().is_none_or(|letter| letter.valid())
    }

    pub fn valid_lines(lines: &[String]) -> bool {
        (2..=6).contains(&lines.len())
            && lines.iter().all(|line| {
                !line.trim().is_empty()
                    && line.width() <= 80
                    && line.len() <= 320
                    && !line
                        .chars()
                        .any(|character| character.is_control() || character.is_numeric())
            })
    }

    pub fn date_words(&self) -> String {
        let Ok(date) = self.day.parse::<Date>() else {
            return String::new();
        };
        let month = [
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
        ][date.month() as usize - 1];
        let day = [
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
            "twenty first",
            "twenty second",
            "twenty third",
            "twenty fourth",
            "twenty fifth",
            "twenty sixth",
            "twenty seventh",
            "twenty eighth",
            "twenty ninth",
            "thirtieth",
            "thirty first",
        ][date.day() as usize - 1];
        format!("{month} {day}")
    }
}

pub fn hit(point: Vec3) -> bool {
    let offset = point - POSITION;
    offset.x.abs() <= HALF_WIDTH + 0.04 && offset.z.abs() <= HALF_DEPTH + 0.04
}

pub(super) fn name_for_letter(name: &str, columns: usize) -> String {
    let mut used = 0;
    name.chars()
        .filter(|character| !character.is_control() && !character.is_numeric())
        .take_while(|character| {
            used += character.width().unwrap_or(0);
            used <= columns
        })
        .collect()
}

fn language(english: &str, korean: &str, use_korean: bool) -> String {
    if use_korean { korean } else { english }.to_owned()
}

impl World {
    pub fn unread_letter(&self) -> Option<usize> {
        self.life.letters.iter().rposition(|letter| !letter.read)
    }

    pub fn write_return_letter(&mut self, away: f32) {
        if away < 6.0 * 3600.0 || self.age_days().is_some_and(|days| days < 1.0) {
            return;
        }
        let Some(story) = &self.away else { return };
        let happenings = story.ranked();
        let korean = self.speaks_korean();
        let name = self.life.viewer_name.as_deref();
        let salutation = match (korean, name) {
            (false, Some(name)) => format!("Dear {name},"),
            (false, None) => "Dear you,".to_owned(),
            (true, Some(name)) => format!("{name}에게,"),
            (true, None) => "보고 싶은 너에게,".to_owned(),
        };
        let ghost_name = name_for_letter(&self.ghost.name, 34);
        let signature = if korean {
            format!("— {ghost_name}")
        } else {
            format!("Love, {ghost_name}")
        };
        let recently_unhappy = self.life.feelings.iter().any(|note| {
            note.feeling.negative()
                && note
                    .days_ago(self.date)
                    .is_some_and(|days| (0..=3).contains(&days))
        });
        let lines = if self.stage() == Stage::Newborn {
            let mut lines = vec![language("Ooh! Miss yoo!", "응! 보고 시퍼!", korean)];
            if recently_unhappy {
                lines.push(language(
                    "I hope you're feeling better.",
                    "네 기분이 좀 나아졌으면 좋겠어.",
                    korean,
                ));
            }
            lines.push(signature);
            lines
        } else {
            let mut lines = vec![salutation];
            let count = 1 + usize::from(self.random_unit() < 0.5);
            for happening in happenings.into_iter().take(count) {
                let choices = happening.lines(korean).map(|line| {
                    line.replace(" while you were gone", "")
                        .replace("when you usually visit", "at your usual time")
                });
                let weights = choices
                    .each_ref()
                    .map(|line| f32::from(u8::from(line.width() <= 40)));
                let index = self.pick(&weights).unwrap_or(0);
                lines.push(choices[index].clone());
            }
            let mut now = Vec::new();
            if self.plant.blooming()
                || self
                    .seedlings
                    .iter()
                    .any(|plant| plant.flowering(self.season()))
            {
                now.push(language(
                    "A flower is opening here.",
                    "여기 꽃이 피고 있어.",
                    korean,
                ));
            }
            if !self.life.keepsakes.is_empty() {
                now.push(language(
                    "I kept the bird's gift on my shelf.",
                    "새의 선물을 선반에 간직했어.",
                    korean,
                ));
            }
            if self.life.wish.is_some() {
                now.push(language(
                    "I'm keeping my little wish safe.",
                    "작은 소원을 소중히 간직하고 있어.",
                    korean,
                ));
            }
            if self
                .life
                .books
                .owned
                .iter()
                .any(|book| book.page > 0 || book.finished > 0)
            {
                now.push(language(
                    "My book is waiting for us.",
                    "우리 같이 읽을 책이 기다리고 있어.",
                    korean,
                ));
            }
            if now.is_empty() {
                now.push(language(
                    "I saved a cozy spot for you.",
                    "네 자리를 포근하게 남겨 뒀어.",
                    korean,
                ));
            }
            let index = self.pick(&vec![1.0; now.len()]).unwrap();
            lines.push(now.swap_remove(index));
            let nature = self.life.temperament();
            let feeling = if recently_unhappy {
                language(
                    "I hope you're feeling better.",
                    "네 기분이 좀 나아졌으면 좋겠어.",
                    korean,
                )
            } else if self.life.bond >= 0.55 || nature.tender >= 0.65 {
                language("I missed you.", "보고 싶었어.", korean)
            } else if nature.shy >= 0.65 {
                language("It was quiet without you.", "네가 없으니 조용했어.", korean)
            } else if nature.playful >= 0.65 {
                language("Come play soon!", "곧 같이 놀자!", korean)
            } else {
                language("I'm glad you're here again.", "다시 와 줘서 기뻐.", korean)
            };
            lines.push(feeling);
            lines.push(signature);
            lines
        };
        self.deliver(lines, korean);
    }

    /// A short birthday letter left by the glass on the viewer's birthday,
    /// beside the greeting.
    pub(super) fn write_birthday_letter(&mut self) {
        if self.age_days().is_some_and(|days| days < 1.0) {
            return;
        }
        let korean = self.speaks_korean();
        let name = self.life.viewer_name.as_deref();
        let mut lines = vec![match (korean, name) {
            (false, Some(name)) => format!("Dear {name},"),
            (false, None) => "Dear you,".to_owned(),
            (true, Some(name)) => format!("{name}에게,"),
            (true, None) => "소중한 너에게,".to_owned(),
        }];
        lines.push(language("Happy birthday!", "생일 축하해!", korean));
        lines.push(language(
            "I'm so glad you were born.",
            "네가 태어나서 정말 기뻐.",
            korean,
        ));
        let nature = self.life.temperament();
        lines.push(if nature.playful >= 0.6 {
            language("Let's play all day!", "오늘은 하루 종일 같이 놀자!", korean)
        } else {
            language(
                "I made a wish just for you.",
                "너를 위해 소원을 빌었어.",
                korean,
            )
        });
        let ghost_name = name_for_letter(&self.ghost.name, 34);
        lines.push(if korean {
            format!("— {ghost_name}")
        } else {
            format!("Love, {ghost_name}")
        });
        self.deliver(lines, korean);
    }

    pub(super) fn deliver(&mut self, lines: Vec<String>, korean: bool) -> bool {
        let letter = Letter {
            day: self.date.to_string(),
            lines,
            korean,
            read: false,
            claude: false,
            reply_to: None,
        };
        if !letter.valid() {
            return false;
        }
        self.life.letters.push(letter);
        if self.life.letters.len() > KEPT {
            self.life.letters.drain(..self.life.letters.len() - KEPT);
        }
        self.record(EventKind::WroteLetter);
        true
    }

    pub fn read_letter(&mut self, index: usize) -> bool {
        let Some(letter) = self
            .life
            .letters
            .get_mut(index)
            .filter(|letter| !letter.read)
        else {
            return false;
        };
        letter.read = true;
        self.record(EventKind::ReadLetter);
        if self.ghost.asleep < 0.3 {
            self.attend("viewer", 30.0);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn away_letters_remember_recent_negative_feelings_in_both_languages_including_newborns() {
        use crate::world::feelings::{Feeling, FeelingNote};
        for korean in [false, true] {
            for newborn in [false, true] {
                for days in [0, 3, 4] {
                    let mut world = returning(5, korean);
                    if newborn {
                        world.birthday = world.date.yesterday().ok();
                    }
                    let day = world
                        .date
                        .checked_sub(jiff::Span::new().days(days))
                        .unwrap();
                    world.life.feelings.push(FeelingNote {
                        feeling: Feeling::Tired,
                        day: day.to_string(),
                        hour: 12.0,
                        asked: false,
                    });
                    world.write_return_letter(21600.0);
                    let letter = &world.life.letters[0];
                    assert!(letter.valid());
                    let hope = if korean {
                        "네 기분이 좀 나아졌으면 좋겠어."
                    } else {
                        "I hope you're feeling better."
                    };
                    assert_eq!(letter.lines.iter().any(|line| line == hope), days <= 3);
                }
            }
        }
    }

    use super::*;
    use crate::world::{
        Life, Nature,
        away::{self, Happening},
    };

    fn returning(seed: u64, korean: bool) -> World {
        let mut world = World::new(seed, 12.0);
        world.life.korean = if korean { 1.0 } else { 0.0 };
        world.away = Some(away::story(
            "2026-06-20T12:00:00".parse().unwrap(),
            "2026-06-21T12:00:00".parse().unwrap(),
            seed,
            &world.life,
            None,
        ));
        world
    }

    #[test]
    fn letters_follow_the_viewers_language_and_use_their_name_when_known() {
        for (korean, name, greeting) in [
            (false, None, "Dear you,"),
            (false, Some("Sam"), "Dear Sam,"),
            (true, None, "보고 싶은 너에게,"),
            (true, Some("수민"), "수민에게,"),
        ] {
            let mut world = returning(4, korean);
            world.life.viewer_name = name.map(str::to_owned);
            world.ghost.name = if korean { "뽀삐" } else { "Pip" }.to_owned();
            world.write_return_letter(21600.0);
            let letter = &world.life.letters[0];
            assert_eq!(letter.lines[0], greeting);
            assert_eq!(letter.korean, korean);
            assert_eq!(
                letter.lines.last().unwrap(),
                if korean { "— 뽀삐" } else { "Love, Pip" }
            );
            assert!(!letter.read);
            assert!(!letter.claude);
        }
    }

    #[test]
    fn letters_are_short_digit_free_and_deterministic_for_a_seed() {
        for seed in 0..100 {
            for korean in [false, true] {
                let mut first = returning(seed, korean);
                let mut second = returning(seed, korean);
                first.write_return_letter(86400.0);
                second.write_return_letter(86400.0);
                assert_eq!(first.life.letters, second.life.letters);
                let letter = &first.life.letters[0];
                assert!((3..=6).contains(&letter.lines.len()));
                assert!(letter.valid());
                for line in &letter.lines {
                    assert!(line.width() <= 40, "{line}");
                    assert!(!line.chars().any(char::is_numeric));
                }
                assert_eq!(first.random_unit(), second.random_unit());
            }
        }
    }

    #[test]
    fn a_personal_line_follows_closeness_tenderness_shyness_and_playfulness() {
        for (bond, tender, shy, playful, english, korean) in [
            (0.7, 0.0, 0.0, 0.0, "I missed you.", "보고 싶었어."),
            (0.0, 0.9, 0.0, 0.0, "I missed you.", "보고 싶었어."),
            (
                0.0,
                0.0,
                0.9,
                0.0,
                "It was quiet without you.",
                "네가 없으니 조용했어.",
            ),
            (0.0, 0.0, 0.0, 0.9, "Come play soon!", "곧 같이 놀자!"),
        ] {
            for use_korean in [false, true] {
                let mut world = returning(5, use_korean);
                world.life.bond = bond;
                world.life.nature = Some(Nature {
                    tender,
                    shy,
                    playful,
                    ..Nature::default()
                });
                world.write_return_letter(21600.0);
                assert!(
                    world.life.letters[0]
                        .lines
                        .iter()
                        .any(|line| line == if use_korean { korean } else { english })
                );
            }
        }
    }

    #[test]
    fn newborns_under_a_day_write_nothing_and_older_newborns_babble_in_two_lines() {
        for korean in [false, true] {
            let mut world = returning(7, korean);
            world.birthday = Some(world.date);
            world.write_return_letter(21600.0);
            assert!(world.life.letters.is_empty());
            world.birthday = world.date.yesterday().ok();
            world.write_return_letter(21600.0);
            let letter = &world.life.letters[0];
            assert_eq!(letter.lines.len(), 2);
            assert_eq!(
                letter.lines[0],
                if korean {
                    "응! 보고 시퍼!"
                } else {
                    "Ooh! Miss yoo!"
                }
            );
        }
    }

    #[test]
    fn a_letter_uses_the_highest_ranked_happening_and_something_known_now() {
        let mut world = returning(3, false);
        world.away.as_mut().unwrap().happenings = vec![Happening::Napped, Happening::WishedOnStar];
        world.life.keepsakes.push(crate::world::Keepsake::Feather);
        world.write_return_letter(21600.0);
        let lines = &world.life.letters[0].lines;
        assert!(lines.iter().any(|line| line.contains("shooting star")));
        assert!(lines.iter().any(|line| line.contains("shelf")));
        assert_eq!(world.life.total.letters_written, 1);
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("wrote you a letter")
        );
        assert!(world.events.iter().any(|event| matches!(&event.kind, EventKind::Milestone(text) if text == "I wrote you my first letter.")));
    }

    #[test]
    fn short_absences_do_not_make_letters_and_reading_counts_once() {
        let mut world = returning(1, false);
        world.write_return_letter(21599.0);
        assert!(world.life.letters.is_empty());
        world.write_return_letter(21600.0);
        let bond = world.life.bond;
        assert!(world.read_letter(0));
        assert!(world.life.bond > bond);
        let bond = world.life.bond;
        assert!(!world.read_letter(0));
        assert!(!world.read_letter(20));
        assert_eq!(world.life.bond, bond);
        assert_eq!(world.life.total.letters_read, 1);
        assert_eq!(world.unread_letter(), None);
    }

    #[test]
    fn saved_letters_round_trip_and_sanitizing_keeps_only_the_newest_twenty_valid_ones() {
        let mut world = returning(8, false);
        world.write_return_letter(21600.0);
        let letter = world.life.letters[0].clone();
        let encoded = serde_json::to_string(&world.life).unwrap();
        let decoded: Life = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.letters, world.life.letters);
        let mut old = serde_json::to_value(&world.life).unwrap();
        old.as_object_mut().unwrap().remove("letters");
        assert!(
            serde_json::from_value::<Life>(old)
                .unwrap()
                .letters
                .is_empty()
        );
        let mut life = Life::default();
        for day in 1..=25 {
            let mut letter = letter.clone();
            letter.day = format!("2026-06-{day:02}");
            life.letters.push(letter);
        }
        for damaged in [
            String::new(),
            "a".repeat(81),
            "hello\nthere".to_owned(),
            "\u{1b}[m".to_owned(),
            "hi 2".to_owned(),
        ] {
            let mut letter = letter.clone();
            letter.lines[0] = damaged;
            life.letters.push(letter);
        }
        for day in ["bad", "2026-02-30", "2026-6-21"] {
            let mut letter = letter.clone();
            letter.day = day.to_owned();
            life.letters.push(letter);
        }
        let life = life.sanitized();
        assert_eq!(life.letters.len(), 20);
        assert_eq!(life.letters[0].day, "2026-06-06");
        assert_eq!(life.letters.last().unwrap().day, "2026-06-25");
    }

    #[test]
    fn the_date_heading_uses_month_and_ordinal_words() {
        let letter = Letter {
            day: "2026-10-21".to_owned(),
            ..Letter::default()
        };
        assert_eq!(letter.date_words(), "October twenty first");
    }
}
