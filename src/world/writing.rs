//! Paired life writing stays separate from the English words sent to minds.
use jiff::civil::Date;

use super::{Entry, Life, Occasion, World};

pub(super) const TEXT_LIMIT: usize = 4096;

pub(super) fn valid(text: &str) -> bool {
    !text.trim().is_empty()
        && text.chars().count() <= TEXT_LIMIT
        && !text.chars().any(char::is_control)
}

pub(super) fn final_consonant(text: &str) -> bool {
    text.trim_end().chars().last().is_some_and(|letter| {
        if ('가'..='힣').contains(&letter) {
            (u32::from(letter) - u32::from('가')) % 28 != 0
        } else {
            letter.is_ascii_alphabetic() && !"aeiouAEIOU".contains(letter)
        }
    })
}

pub(super) fn particle<'a>(name: &str, consonant: &'a str, vowel: &'a str) -> &'a str {
    let rieul = name.trim_end().chars().last().is_some_and(|letter| {
        ('가'..='힣').contains(&letter) && (u32::from(letter) - u32::from('가')) % 28 == 8
    });
    if final_consonant(name) && !(consonant == "으로" && rieul) {
        consonant
    } else {
        vowel
    }
}

pub(super) fn count(number: u32, counter: &str) -> String {
    let native = [
        "", "한", "두", "세", "네", "다섯", "여섯", "일곱", "여덟", "아홉", "열",
    ];
    if (1..=10).contains(&number) {
        // Minutes and years take Sino-Korean numbers even in a quiet diary.
        let words = if matches!(counter, "분" | "년") {
            korean_number(number)
        } else {
            native[number as usize].to_owned()
        };
        format!("{words} {counter}")
    } else {
        format!("{number}{counter}")
    }
}

pub(super) fn learned_name(name: &str) -> String {
    format!(
        "네 이름이 {name}{} 걸 알게 됐어",
        particle(name, "이라는", "라는")
    )
}

pub(super) fn nouns(words: &[&str]) -> String {
    match words {
        [] => String::new(),
        [word] => (*word).to_owned(),
        [rest @ .., last] => {
            let rest = rest.join(", ");
            format!("{rest}{} {last}", particle(&rest, "과", "와"))
        }
    }
}

/// Deeds use the same past-tense diary ending, so they can share a sentence.
/// An unpaired deed from a newer feature remains readable in English.
pub(super) fn sentence(deeds: &[String]) -> String {
    if deeds.is_empty() {
        return "조용한 하루였어.".to_owned();
    }
    let mut text = String::new();
    for (index, deed) in deeds.iter().enumerate() {
        let deed = deed.trim_end_matches(['.', '!']);
        if index + 1 < deeds.len() {
            if let Some(stem) = deed.strip_suffix(['어', '아']) {
                text.push_str(stem);
                text.push_str("고, ");
            } else {
                text.push_str(deed);
                text.push_str(". ");
            }
        } else {
            text.push_str(deed);
            text.push('.');
        }
    }
    text
}

pub(super) fn hour_words(hour: u32) -> &'static str {
    [
        "열두", "한", "두", "세", "네", "다섯", "여섯", "일곱", "여덟", "아홉", "열", "열한",
    ][(hour % 12) as usize]
}

pub(super) fn duration(seconds: f64) -> String {
    let minutes = seconds / 60.0;
    if minutes < 5.0 {
        "잠깐".to_owned()
    } else if minutes < 60.0 {
        format!("{}쯤", count(minutes.round() as u32, "분"))
    } else {
        let hours = (seconds / 3600.0).round().max(1.0) as u32;
        format!("{}쯤", count(hours, "시간"))
    }
}

impl Entry {
    pub fn in_language(&self, korean: bool) -> &str {
        if korean {
            self.korean
                .as_deref()
                .filter(|text| !text.trim().is_empty())
                .unwrap_or(&self.text)
        } else {
            &self.text
        }
    }
}

impl Life {
    pub fn speaks_korean(&self) -> bool {
        self.korean > 0.5
    }

    pub fn recall_yesterday(&self, today: Date, korean: bool) -> String {
        let yesterday = today.yesterday().ok().map(|day| day.to_string());
        let deeds: Vec<String> = self
            .yesterday
            .iter()
            .filter(|entry| Some(&entry.day) == yesterday.as_ref())
            .map(|entry| entry.in_language(korean).to_owned())
            .collect();
        super::life::recalled_deeds(&deeds, korean)
    }
}

pub fn time_answer(world: &World, korean: bool) -> String {
    let hour = world.hour.rem_euclid(24.0);
    let hours = hour.floor() as u32;
    let minutes = ((hour - hours as f32) * 60.0).floor() as u32;
    let mut text = if korean {
        let part = if hours < 12 { "오전" } else { "오후" };
        let when = match minutes {
            0..=4 => "야",
            5..=19 => " 조금 넘었어",
            20..=39 => " 반쯤 됐어",
            _ => "가 꽤 지났어",
        };
        format!("{part} {} 시{when}.", hour_words(hours))
    } else {
        let part = match hours {
            0..=4 | 22..=23 => "at night",
            5..=11 => "in the morning",
            12..=16 => "in the afternoon",
            _ => "in the evening",
        };
        let when = match minutes {
            0..=4 => "",
            5..=19 => "a little past ",
            20..=39 => "half past ",
            _ => "well past ",
        };
        let hours = if hours.is_multiple_of(12) {
            12
        } else {
            hours % 12
        };
        format!("It's {when}{} {part}.", super::friend::count_words(hours))
    };
    if world.is_night() && world.lamp.on {
        text.push_str(if korean {
            " 등불이 곁을 밝혀 주고 있어."
        } else {
            " The lamp is keeping us company."
        });
    }
    text
}

fn korean_number(number: u32) -> String {
    if number == 0 {
        return "영".to_owned();
    }
    let digits = ["", "일", "이", "삼", "사", "오", "육", "칠", "팔", "구"];
    let mut text = String::new();
    for (place, unit) in [(1000, "천"), (100, "백"), (10, "십"), (1, "")] {
        let digit = number / place % 10;
        if digit > 0 {
            if digit != 1 || place == 1 {
                text.push_str(digits[digit as usize]);
            }
            text.push_str(unit);
        }
    }
    text
}

pub fn date_answer(world: &World, korean: bool) -> String {
    let date = world.date;
    let weekday = date.weekday().to_monday_zero_offset() as usize;
    let occasion = world.occasion();
    if korean {
        let weekday = [
            "월요일",
            "화요일",
            "수요일",
            "목요일",
            "금요일",
            "토요일",
            "일요일",
        ][weekday];
        let month = match date.month() {
            6 => "유".to_owned(),
            10 => "시".to_owned(),
            month => korean_number(month as u32),
        };
        let mut text = format!(
            "오늘은 {}년 {month}월 {}일 {weekday}이야.",
            korean_number(date.year().unsigned_abs() as u32),
            korean_number(date.day() as u32)
        );
        if let Some(occasion) = occasion {
            text.push_str(&format!(" 오늘은 {}이기도 해.", occasion_korean(occasion)));
        }
        text
    } else {
        let weekday = [
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
            "Sunday",
        ][weekday];
        let mut text = format!(
            "It's {weekday}, {}, {}.",
            super::mail::date_words(date),
            super::friend::count_words(date.year().unsigned_abs() as u32)
        );
        if let Some(occasion) = occasion {
            text.push_str(&format!(" It's {} too.", occasion.words()));
        }
        text
    }
}

pub(super) fn occasion_korean(occasion: Occasion) -> &'static str {
    match occasion {
        Occasion::Halloween => "할로윈",
        Occasion::WinterHolidays => "겨울 명절",
        Occasion::NewYear => "새해",
        Occasion::Valentine => "밸런타인데이",
        Occasion::Birthday => "내 생일",
        Occasion::Seollal => "설날",
        Occasion::Chuseok => "추석",
        Occasion::HangulDay => "한글날",
        Occasion::ViewerBirthday => "네 생일",
        Occasion::HundredDays => "내 백일",
        Occasion::ThousandDays => "내 천일",
    }
}

pub(super) fn comfort(feeling: super::Feeling) -> String {
    use super::Feeling;
    match feeling {
        Feeling::Stressed | Feeling::Angry => "같이 천천히 숨을 쉬었어".to_owned(),
        Feeling::Tired | Feeling::Unwell => {
            format!("네가 ‘{}’라고 해서 조용히 곁에 있었어", feeling.korean())
        }
        Feeling::Sad | Feeling::Lonely => {
            format!("네가 ‘{}’라고 해서 하트를 그려 줬어", feeling.korean())
        }
        Feeling::Bored => "네가 심심하다고 해서 같이 놀자고 했어".to_owned(),
        Feeling::Happy | Feeling::Proud => {
            format!("네가 ‘{}’라고 해서 같이 기뻐했어", feeling.korean())
        }
    }
}

pub(super) fn quirk(kind: super::Quirk, number: u32) -> String {
    use super::Quirk;
    let times = if number == 1 {
        String::new()
    } else {
        format!("{} ", count(number, "번"))
    };
    match kind {
        Quirk::Hiccups => format!("딸꾹질을 {times}했어"),
        Quirk::Sneeze => format!("재채기를 {times}했어"),
        Quirk::Yawn => format!("하품을 {times}했어"),
        Quirk::Shiver => format!("추워서 {times}떨었어"),
        Quirk::Sigh => format!("편안하게 한숨을 {times}내쉬었어"),
    }
}

pub(super) fn passerby(kind: super::Passerby) -> &'static str {
    use super::Passerby;
    match kind {
        Passerby::Geese => "기러기 떼",
        Passerby::Balloon => "열기구",
        Passerby::Kite => "연",
        Passerby::Squirrel => "다람쥐",
        Passerby::Plane => "비행기의 불빛",
        Passerby::Snowman => "눈사람",
        Passerby::Lanterns => "풍등",
    }
}

pub(super) fn friend_visit(tally: &super::life::Tally, name: &str) -> String {
    use super::friend::Game;
    let visits = if tally.friend_visits == 1 {
        String::new()
    } else {
        format!("{} ", count(tally.friend_visits, "번"))
    };
    let visit = format!("{name}{} {visits}놀러", particle(name, "이", "가"));
    if tally.friend_played.is_empty() {
        return format!("{visit} 왔어");
    }
    let games = tally
        .friend_played
        .iter()
        .enumerate()
        .map(
            |(index, game)| match (game, index + 1 == tally.friend_played.len()) {
                (Game::Tag, false) => "술래잡기하고",
                (Game::Tag, true) => "술래잡기했어",
                (Game::Twirl, false) => "빙글 돌며 춤추고",
                (Game::Twirl, true) => "빙글 돌며 춤췄어",
                (Game::MoonGazing, false) => "달을 구경하고",
                (Game::MoonGazing, true) => "달을 구경했어",
            },
        )
        .collect::<Vec<_>>()
        .join(" ");
    format!("{visit} 와서 같이 {games}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_counts_use_native_counter_forms_and_zero_and_larger_counts_use_digits() {
        for counter in ["번", "권", "개", "마리", "쪽", "곡", "장", "시간"] {
            for (index, native) in [
                "한", "두", "세", "네", "다섯", "여섯", "일곱", "여덟", "아홉", "열",
            ]
            .into_iter()
            .enumerate()
            {
                assert_eq!(
                    count(index as u32 + 1, counter),
                    format!("{native} {counter}")
                );
            }
            for number in [0, 11, 12, 37, u32::MAX] {
                assert_eq!(count(number, counter), format!("{number}{counter}"));
            }
        }
        assert_eq!(count(5, "분"), "오 분");
        assert_eq!(count(2, "년"), "이 년");
    }

    #[test]
    fn korean_diary_sentences_join_one_two_and_many_past_deeds() {
        let deeds = [
            "공을 밀었어".to_owned(),
            "낮잠을 잤어".to_owned(),
            "책을 읽었어".to_owned(),
        ];
        assert_eq!(sentence(&[]), "조용한 하루였어.");
        assert_eq!(sentence(&deeds[..1]), "공을 밀었어.");
        assert_eq!(sentence(&deeds[..2]), "공을 밀었고, 낮잠을 잤어.");
        assert_eq!(sentence(&deeds), "공을 밀었고, 낮잠을 잤고, 책을 읽었어.");
        assert_eq!(
            sentence(&["친구가 왔어.".to_owned(), "참 반가웠어!".to_owned()]),
            "친구가 왔고, 참 반가웠어."
        );
        assert_eq!(
            sentence(&["a future deed".to_owned(), deeds[0].clone()]),
            "a future deed. 공을 밀었어."
        );
        assert_eq!(
            sentence(&[
                "놀이를 했어".to_owned(),
                "조용한 하루였어".to_owned(),
                "네가 물을 줬어".to_owned(),
                "친구가 왔어".to_owned(),
                "장난기가 늘어난 것 같아".to_owned(),
                "책을 읽었어".to_owned(),
            ]),
            "놀이를 했고, 조용한 하루였고, 네가 물을 줬고, 친구가 왔고, 장난기가 늘어난 것 같고, 책을 읽었어."
        );
    }

    #[test]
    fn particles_follow_hangul_and_latin_final_consonants() {
        for (name, consonant) in [
            ("민수", false),
            ("민석", true),
            ("Mina", false),
            ("Pip", true),
        ] {
            for (closed, open) in [
                ("이", "가"),
                ("을", "를"),
                ("은", "는"),
                ("과", "와"),
                ("이랑", "랑"),
            ] {
                assert_eq!(
                    particle(name, closed, open),
                    if consonant { closed } else { open },
                    "{name}"
                );
            }
        }
        for (name, expected) in [("민준", "으로"), ("수아", "로"), ("하늘", "로")] {
            assert_eq!(particle(name, "으로", "로"), expected);
        }
    }

    #[test]
    fn noun_lists_join_the_last_two_items_with_the_right_particle() {
        assert_eq!(nouns(&[]), "");
        assert_eq!(nouns(&["도토리"]), "도토리");
        assert_eq!(nouns(&["도토리", "깃털"]), "도토리와 깃털");
        assert_eq!(nouns(&["깃털", "도토리"]), "깃털과 도토리");
        assert_eq!(
            nouns(&["도토리", "깃털", "조약돌"]),
            "도토리, 깃털과 조약돌"
        );
    }

    #[test]
    fn friend_visits_share_one_subject_and_join_games_without_repeating_together() {
        use crate::world::{friend::Game, life::Tally};
        let mut tally = Tally {
            friend_visits: 1,
            ..Default::default()
        };
        assert_eq!(friend_visit(&tally, "Wren"), "Wren이 놀러 왔어");
        assert_eq!(friend_visit(&tally, "수아"), "수아가 놀러 왔어");
        tally.friend_played = vec![Game::Tag, Game::Twirl];
        assert_eq!(
            friend_visit(&tally, "Wren"),
            "Wren이 놀러 와서 같이 술래잡기하고 빙글 돌며 춤췄어"
        );
        tally.friend_visits = 2;
        tally.friend_played.push(Game::MoonGazing);
        assert_eq!(
            sentence(&[friend_visit(&tally, "수아"), "공을 밀었어".to_owned()]),
            "수아가 두 번 놀러 와서 같이 술래잡기하고 빙글 돌며 춤추고 달을 구경했고, 공을 밀었어."
        );
    }

    #[test]
    fn clock_and_calendar_answers_use_the_worlds_local_time_and_occasion() {
        let mut world = World::new(7, 15.1);
        assert!(
            time_answer(&world, false).starts_with("It's a little past three in the afternoon.")
        );
        assert!(time_answer(&world, true).starts_with("오후 세 시 조금 넘었어."));
        for (hour, english, korean) in [
            (0.0, "twelve at night", "오전 열두 시야"),
            (12.0, "twelve in the afternoon", "오후 열두 시야"),
            (8.5, "half past eight", "오전 여덟 시 반쯤 됐어"),
        ] {
            world.hour = hour;
            assert!(time_answer(&world, false).contains(english));
            assert!(time_answer(&world, true).contains(korean));
        }
        world.date = "2026-10-21".parse().unwrap();
        assert!(
            date_answer(&world, false)
                .contains("Wednesday, October twenty first, two thousand twenty six")
        );
        assert!(date_answer(&world, true).contains("이천이십육년 시월 이십일일 수요일"));
        world.date = "2026-10-09".parse().unwrap();
        assert!(date_answer(&world, true).contains("한글날"));
        assert!(date_answer(&world, false).contains("Hangul Day"));
        for korean in [false, true] {
            assert!(
                !date_answer(&world, korean)
                    .chars()
                    .any(|letter| letter.is_ascii_digit())
            );
            assert!(
                !time_answer(&world, korean)
                    .chars()
                    .any(|letter| letter.is_ascii_digit())
            );
        }
    }
}
