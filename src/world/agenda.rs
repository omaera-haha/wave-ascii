use jiff::civil::Date;
use serde::{Deserialize, Deserializer, Serialize};

use super::{Activity, EventKind, Feeling, Heard, Listening, Mood, Tone, Understanding, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Errand {
    Exam,
    Interview,
    Presentation,
    Meeting,
    Appointment,
    Date,
    Match,
    Performance,
    Deadline,
    Party,
    Moving,
    FirstDay,
    Result,
}

impl Errand {
    pub fn name(self) -> &'static str {
        match self {
            Self::Exam => "exam",
            Self::Interview => "interview",
            Self::Presentation => "presentation",
            Self::Meeting => "meeting",
            Self::Appointment => "appointment",
            Self::Date => "date",
            Self::Match => "match",
            Self::Performance => "performance",
            Self::Deadline => "deadline",
            Self::Party => "party",
            Self::Moving => "move",
            Self::FirstDay => "first day",
            Self::Result => "application result",
        }
    }

    pub fn korean(self) -> &'static str {
        match self {
            Self::Exam => "시험",
            Self::Interview => "면접",
            Self::Presentation => "발표",
            Self::Meeting => "회의",
            Self::Appointment => "진료",
            Self::Date => "데이트",
            Self::Match => "경기",
            Self::Performance => "공연",
            Self::Deadline => "마감",
            Self::Party => "파티",
            Self::Moving => "이사",
            Self::FirstDay => "새로운 시작",
            Self::Result => "지원 결과 발표",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Well,
    Badly,
    Unclear,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Engagement {
    pub errand: Errand,
    pub day: String,
    pub said: String,
    #[serde(default)]
    pub wished: bool,
    #[serde(default)]
    pub asked: bool,
    #[serde(default)]
    pub asked_on: Option<String>,
    #[serde(default)]
    pub outcome: Option<Outcome>,
}

fn date(text: &str) -> Option<Date> {
    text.parse::<Date>()
        .ok()
        .filter(|date| date.to_string() == text)
}

impl Engagement {
    pub fn days_from(&self, today: Date) -> Option<i32> {
        Some((date(&self.day)? - today).get_days())
    }

    pub fn valid(&self, today: Date) -> bool {
        let (Some(day), Some(said)) = (date(&self.day), date(&self.said)) else {
            return false;
        };
        said <= today
            && (-60..=60).contains(&(day - said).get_days())
            && self.asked_on.as_ref().is_none_or(|asked| {
                date(asked).is_some_and(|asked| {
                    self.asked && asked <= today && (0..=3).contains(&(asked - day).get_days())
                })
            })
            && (self.outcome.is_none() || self.asked)
    }
}

// A damaged entry, including an unknown kind, must not cost the rest of a life.
pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Engagement>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(value.as_array().map_or_else(Vec::new, |entries| {
        entries
            .iter()
            .filter_map(|entry| serde_json::from_value(entry.clone()).ok())
            .collect()
    }))
}

pub(super) fn sanitize(agenda: &mut Vec<Engagement>, today: &str) {
    let Some(today) = date(today) else {
        agenda.clear();
        return;
    };
    agenda.retain(|engagement| engagement.valid(today));
    let mut seen = Vec::new();
    agenda.reverse();
    agenda.retain(|engagement| {
        let key = (engagement.errand, engagement.day.clone());
        if seen.len() >= 8 || seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
    agenda.truncate(8);
    agenda.reverse();
}

fn words(text: &str) -> Vec<&str> {
    text.split(|character: char| !character.is_alphabetic())
        .filter(|word| !word.is_empty())
        .collect()
}

/// Only a dated statement about the viewer's own plans supplies a memory.
pub fn planned(text: &str, today: Date) -> Option<(Errand, Date)> {
    if text.contains(['?', '？', '"', '“', '”', '「', '」', '‘', '『', '』'])
        || super::question::ask(text).is_some()
    {
        return None;
    }
    let characters: Vec<char> = text.chars().collect();
    if characters.iter().enumerate().any(|(index, character)| {
        matches!(character, '\'' | '’')
            && !(index > 0
                && characters[index - 1].is_alphabetic()
                && characters
                    .get(index + 1)
                    .is_some_and(|next| next.is_alphabetic()))
    }) {
        return None;
    }
    let text = text.to_lowercase().replace(['\'', '’'], "");
    let mut found = None;
    for sentence in text.split(['.', '!', ';', '\n']) {
        let tokens = words(sentence);
        if super::feelings::denies(sentence)
            || tokens.iter().any(|word| {
                matches!(
                    *word,
                    "had"
                        | "was"
                        | "were"
                        | "took"
                        | "did"
                        | "didnt"
                        | "wont"
                        | "cant"
                        | "cancelled"
                        | "canceled"
                ) || ["없", "봤", "었", "았", "취소"]
                    .iter()
                    .any(|ending| word.contains(ending))
            })
        {
            continue;
        }
        let engagement = english(&tokens, today).or_else(|| korean(sentence, today));
        if let Some(engagement) = engagement {
            if found.is_some() {
                return None;
            }
            found = Some(engagement);
        }
    }
    found
}

fn weekday(today: Date, target: i32, next_week: bool, include_today: bool) -> Option<Date> {
    let current = i32::from(today.weekday().to_monday_one_offset());
    let mut days = if next_week {
        7 - current + target
    } else {
        (target - current + 7) % 7
    };
    if days == 0 && !include_today {
        days = 7;
    }
    today.checked_add(jiff::Span::new().days(days)).ok()
}

fn english_time<'a>(tokens: &[&'a str], today: Date) -> Option<(Vec<&'a str>, Date)> {
    let mut remaining = tokens.to_vec();
    let weekdays = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ];
    for (offset, name) in weekdays.into_iter().enumerate() {
        if let Some(position) = tokens.iter().position(|word| *word == name) {
            let prefix = position.checked_sub(1).map(|position| tokens[position]);
            let specified = matches!(prefix, Some("this" | "next" | "on"));
            let explicit_today = tokens.contains(&"today");
            if explicit_today
                && (offset as i32 + 1 != i32::from(today.weekday().to_monday_one_offset())
                    || prefix == Some("next"))
            {
                return None;
            }
            let day = weekday(
                today,
                offset as i32 + 1,
                prefix == Some("next"),
                prefix == Some("this") || explicit_today,
            )?;
            remaining.drain(position - usize::from(specified)..=position);
            remaining.retain(|word| *word != "today");
            return Some((remaining, day));
        }
    }
    let phrases: &[(&[&str], i32)] = &[
        (&["the", "day", "after", "tomorrow"], 2),
        (&["this", "morning"], 0),
        (&["this", "afternoon"], 0),
        (&["this", "evening"], 0),
        (&["tomorrow"], 1),
        (&["today"], 0),
        (&["tonight"], 0),
    ];
    for &(phrase, days) in phrases {
        if let Some(position) = tokens.windows(phrase.len()).position(|part| part == phrase) {
            remaining.drain(position..position + phrase.len());
            return Some((
                remaining,
                today.checked_add(jiff::Span::new().days(days)).ok()?,
            ));
        }
    }
    for (phrase, day) in [
        (["this", "weekend"], weekday(today, 6, false, true)?),
        (
            ["next", "week"],
            today.checked_add(jiff::Span::new().days(7)).ok()?,
        ),
    ] {
        if let Some(position) = tokens.windows(2).position(|part| part == phrase) {
            remaining.drain(position..position + 2);
            return Some((remaining, day));
        }
    }
    None
}

fn english(tokens: &[&str], today: Date) -> Option<(Errand, Date)> {
    let (remaining, day) = english_time(tokens, today)?;
    let mut statement = remaining.as_slice();
    let errand = match statement {
        ["i", "have", "to", "see", "the", "doctor" | "dentist"]
        | ["im", "seeing", "the", "doctor" | "dentist"]
        | ["i", "am", "seeing", "the", "doctor" | "dentist"] => Some(Errand::Appointment),
        ["im", "moving"]
        | ["i", "am", "moving"]
        | ["im", "moving", "house"]
        | ["i", "am", "moving", "house"] => Some(Errand::Moving),
        [
            "im",
            "playing" | "performing",
            "in",
            "a",
            "concert" | "recital" | "performance",
        ]
        | [
            "i",
            "am",
            "playing" | "performing",
            "in",
            "a",
            "concert" | "recital" | "performance",
        ] => Some(Errand::Performance),
        _ => None,
    };
    if let Some(errand) = errand {
        return Some((errand, day));
    }
    statement = match statement {
        ["ive", "got", rest @ ..]
        | ["i", "have", "got", rest @ ..]
        | ["i", "have", rest @ ..]
        | ["im", "taking", rest @ ..]
        | ["i", "am", "taking", rest @ ..]
        | ["my", rest @ ..]
        | ["is", "my", rest @ ..] => rest,
        _ => return None,
    };
    if let ["a" | "an" | "the" | "my", rest @ ..] = statement {
        statement = rest;
    }
    if let [rest @ .., "is"] = statement {
        statement = rest;
    }
    let errand = match statement {
        ["exam" | "test" | "quiz"] | ["math" | "english" | "final", "exam" | "test" | "quiz"] => {
            Errand::Exam
        }
        ["interview"] | ["job", "interview"] => Errand::Interview,
        ["presentation" | "talk"] => Errand::Presentation,
        ["meeting"] => Errand::Meeting,
        ["doctor" | "doctors" | "dentist" | "dentists", "appointment"] => Errand::Appointment,
        ["date"] => Errand::Date,
        ["match" | "tournament" | "race"] => Errand::Match,
        ["performance" | "recital"] => Errand::Performance,
        ["deadline" | "submission"] => Errand::Deadline,
        ["party"] => Errand::Party,
        ["move"] => Errand::Moving,
        ["first", "day", "at", "the" | "my", "new", "job" | "school"]
        | ["first", "day", "of", "school"] => Errand::FirstDay,
        ["job" | "school", "application", "result", "day"] => Errand::Result,
        _ => return None,
    };
    Some((errand, day))
}

fn korean(text: &str, today: Date) -> Option<(Errand, Date)> {
    let compact: String = text
        .chars()
        .filter(|character| character.is_alphabetic())
        .collect();
    let weekdays = [
        "월요일",
        "화요일",
        "수요일",
        "목요일",
        "금요일",
        "토요일",
        "일요일",
    ];
    let mut when = None;
    for (offset, name) in weekdays.into_iter().enumerate() {
        if let Some(position) = compact.find(name) {
            let before = &compact[..position];
            let prefix = if before.ends_with("다음주") {
                "다음주"
            } else if before.ends_with("이번주") {
                "이번주"
            } else if before.ends_with("오늘") {
                "오늘"
            } else {
                ""
            };
            let day = weekday(
                today,
                offset as i32 + 1,
                prefix == "다음주",
                prefix == "이번주" || before.ends_with("오늘"),
            )?;
            when = Some((position - prefix.len(), position + name.len(), day));
            break;
        }
    }
    if when.is_none() {
        for (phrase, days) in [
            ("모레", 2),
            ("내일", 1),
            ("오늘밤", 0),
            ("오늘", 0),
            ("이따", 0),
            (
                "이번주말",
                (6 - i32::from(today.weekday().to_monday_one_offset()) + 7) % 7,
            ),
            (
                "주말",
                (6 - i32::from(today.weekday().to_monday_one_offset()) + 7) % 7,
            ),
            ("다음주", 7),
        ] {
            if let Some(position) = compact.find(phrase) {
                when = Some((
                    position,
                    position + phrase.len(),
                    today.checked_add(jiff::Span::new().days(days)).ok()?,
                ));
                break;
            }
        }
    }
    let (start, end, day) = when?;
    let mut remaining = format!(
        "{}{}",
        &compact[..start],
        compact[end..].trim_start_matches(['에', '은', '는'])
    );
    for prefix in ["나는", "저는", "내가", "제가", "난", "나", "저", "내"] {
        if let Some(rest) = remaining.strip_prefix(prefix) {
            remaining = rest.to_owned();
            break;
        }
    }
    for (errand, names) in [
        (
            Errand::Result,
            &["합격발표날", "결과발표날", "지원결과발표"] as &[_],
        ),
        (Errand::Exam, &["시험", "테스트"]),
        (Errand::Interview, &["면접"]),
        (Errand::Presentation, &["발표"]),
        (Errand::Meeting, &["회의", "미팅"]),
        (Errand::Appointment, &["병원", "치과", "진료"]),
        (Errand::Date, &["데이트"]),
        (Errand::Match, &["경기", "시합", "대회"]),
        (Errand::Performance, &["공연", "연주회", "발표회"]),
        (Errand::Deadline, &["마감", "제출"]),
        (Errand::Party, &["파티"]),
        (Errand::Moving, &["이사"]),
        (Errand::FirstDay, &["첫출근", "첫등교", "입학"]),
    ] {
        for name in names {
            if let Some(ending) = remaining.strip_prefix(name)
                && matches!(
                    ending,
                    "있어"
                        | "있어요"
                        | "이있어"
                        | "가있어"
                        | "이있어요"
                        | "가있어요"
                        | "이야"
                        | "야"
                        | "이에요"
                        | "예요"
                        | "봐"
                        | "봐요"
                        | "을봐"
                        | "를봐"
                        | "해"
                        | "해요"
                        | "가"
                        | "가요"
                        | "에가"
                        | "가야돼"
                        | "에가야돼"
                        | "가야해"
                        | "해야돼"
                )
            {
                return Some((errand, day));
            }
        }
    }
    None
}

pub fn when(days: i32) -> String {
    match days {
        0 => "today".to_owned(),
        1 => "tomorrow".to_owned(),
        -1 => "yesterday".to_owned(),
        7 => "next week".to_owned(),
        days if days > 0 => format!("in {} days", super::friend::count_words(days as u32)),
        days => format!(
            "{} days ago",
            super::friend::count_words(days.unsigned_abs())
        ),
    }
}

fn korean_when(days: i32) -> String {
    match days {
        0 => "오늘".to_owned(),
        1 => "내일".to_owned(),
        2 => "모레".to_owned(),
        7 => "다음 주".to_owned(),
        _ => "다가오는 날".to_owned(),
    }
}

pub fn memories(agenda: &[Engagement], today: Date) -> String {
    agenda
        .iter()
        .filter_map(|engagement| {
            let days = engagement.days_from(today)?;
            if days < -3 {
                return None;
            }
            let name = engagement.errand.name();
            let when = when(days);
            let state = match engagement.outcome {
                Some(Outcome::Well) => "; they told you it went well",
                Some(Outcome::Badly) => "; it was hard, and you stayed close",
                Some(Outcome::Unclear) => "; they shared how it went",
                None if engagement.asked => "; you already asked how it went",
                None if days <= 0 => "; you haven't asked how it went",
                None => "",
            };
            Some(format!(
                "The viewer {} {name} {when}{state}.",
                if days < 0 { "had their" } else { "has their" }
            ))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn thoughts(world: &World) -> Vec<String> {
    let viewer = world.life.viewer_name.as_deref().unwrap_or("the viewer");
    world
        .life
        .agenda
        .iter()
        .filter_map(|engagement| {
            let days = engagement.days_from(world.date)?;
            ((0..=1).contains(&days) && engagement.outcome.is_none()).then(|| {
                format!(
                    "I hope {viewer}'s {} goes well {}.",
                    engagement.errand.name(),
                    when(days)
                )
            })
        })
        .collect()
}

impl World {
    pub fn awaiting_plan_answer(&self) -> bool {
        self.agenda_answer
            .as_ref()
            .is_some_and(|(_, _, until)| self.time <= *until)
    }

    fn agenda_heard(&self, understanding: Understanding, korean: bool) -> Heard {
        Heard {
            tone: None,
            understanding,
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        }
    }

    pub(super) fn hear_agenda(&mut self, text: &str, korean: bool) -> Option<Heard> {
        let (errand, day) = planned(text, self.date)?;
        self.begin_day();
        self.listening = Listening::default();
        self.notice_viewer();
        self.record(EventKind::Heard(text.to_owned()));
        let day_text = day.to_string();
        let previous = self
            .life
            .agenda
            .iter()
            .find(|engagement| engagement.errand == errand && engagement.day == day_text)
            .cloned();
        self.life
            .agenda
            .retain(|engagement| engagement.errand != errand || engagement.day != day_text);
        self.life.agenda.push(Engagement {
            errand,
            day: day_text,
            said: self.date.to_string(),
            wished: previous
                .as_ref()
                .is_some_and(|engagement| engagement.wished),
            asked: previous.as_ref().is_some_and(|engagement| engagement.asked),
            asked_on: previous
                .as_ref()
                .and_then(|engagement| engagement.asked_on.clone()),
            outcome: previous.and_then(|engagement| engagement.outcome),
        });
        if self.life.agenda.len() > 8 {
            self.life.agenda.remove(0);
        }
        let days = (day - self.date).get_days();
        self.record(EventKind::HeardPlan { errand, days });
        if self.ghost.asleep < 0.3 && self.focus.is_none() && self.telling.is_none() {
            self.begin(Activity::Visit);
        }
        self.receive_message_feeling(text, korean);
        if self.ghost.asleep < 0.3 && self.telling.is_none() {
            let nature = self.life.temperament();
            let (english, hangul) = if nature.tender >= 0.65 {
                ("I'll be here for you.", "내가 곁에 있을게.")
            } else if nature.playful >= 0.65 {
                ("I'll cheer for you!", "힘껏 응원할게!")
            } else {
                ("I'll remember.", "기억해 둘게.")
            };
            let line = if korean {
                let ending = match errand {
                    Errand::Presentation
                    | Errand::Result
                    | Errand::Appointment
                    | Errand::Date
                    | Errand::Match
                    | Errand::Meeting
                    | Errand::Party
                    | Errand::Moving => "구나",
                    _ => "이구나",
                };
                format!(
                    "{} {}{ending}. {hangul}",
                    korean_when(days),
                    errand.korean()
                )
            } else {
                format!("Your {} {}. {english}", errand.name(), when(days))
            };
            self.say(&line);
        }
        if days == 0
            && self.ghost.asleep < 0.3
            && self.telling.is_none()
            && !self.life.agenda.last()?.wished
        {
            self.life.agenda.last_mut()?.wished = true;
            self.record(EventKind::WishedLuck(errand));
            self.say_luck(errand, korean);
        }
        Some(self.agenda_heard(Understanding::Planned(errand), korean))
    }

    fn say_luck(&mut self, errand: Errand, korean: bool) {
        let line = if korean {
            format!("오늘 {} 잘 되길 바랄게! 응원하고 있어.", errand.korean())
        } else {
            format!("Good luck with your {} today!", errand.name())
        };
        self.say(&line);
    }

    pub(super) fn follow_up_agenda(&mut self) {
        if self
            .agenda_answer
            .as_ref()
            .is_some_and(|(_, _, until)| self.time > *until)
        {
            self.agenda_answer = None;
        }
        if self.life.agenda.is_empty()
            || !self.viewer_present()
            || self.viewer.typing
            || self.ghost.asleep >= 0.3
            || self.ghost.plan.activity != Activity::Visit
            || self.ghost.plan.done
            || !self
                .ghost
                .plan
                .arrived
                .is_some_and(|at| self.time - at >= 5.0)
            || self
                .ghost
                .speech
                .as_ref()
                .is_some_and(|line| self.time - line.time < 12.0)
            || self.asking.is_some()
            || self.feeling_follow_up.is_some()
            || self.awaiting_plan_answer()
            || !self.quiet_for_question()
        {
            return;
        }
        if let Some(index) =
            self.life.agenda.iter().position(|engagement| {
                !engagement.wished && engagement.days_from(self.date) == Some(0)
            })
        {
            let errand = self.life.agenda[index].errand;
            self.life.agenda[index].wished = true;
            self.record(EventKind::WishedLuck(errand));
            self.say_luck(errand, self.speaks_korean());
            return;
        }
        if self.agenda_visit_question
            || self.agenda_visit_asked
            || self.life.agenda.iter().any(|engagement| {
                engagement.asked_on.as_deref() == Some(&self.date.to_string())
                    && engagement.outcome.is_none()
            })
        {
            return;
        }
        let Some(index) = self.life.agenda.iter().position(|engagement| {
            !engagement.asked
                && engagement
                    .days_from(self.date)
                    .is_some_and(|days| (-3..=0).contains(&days) && (days < 0 || self.hour >= 18.0))
        }) else {
            return;
        };
        let engagement = &mut self.life.agenda[index];
        engagement.asked = true;
        engagement.asked_on = Some(self.date.to_string());
        let errand = engagement.errand;
        self.agenda_answer = Some((errand, engagement.day.clone(), self.time + 180.0));
        self.agenda_visit_asked = true;
        self.last_question_at = Some(self.time);
        self.ghost.plan.length = self
            .ghost
            .plan
            .length
            .max(self.ghost.plan.stayed(self.time) + 60.0);
        let line = if self.speaks_korean() {
            format!("{} 어떻게 됐어?", errand.korean())
        } else {
            format!("How did your {} go?", errand.name())
        };
        self.say(&line);
        self.record(EventKind::AskedAboutPlan(errand));
    }

    pub(super) fn answer_plan(&mut self, text: &str, korean: bool) -> Option<Heard> {
        let (errand, day, until) = self.agenda_answer.take()?;
        if self.time > until
            || self.ghost.asleep >= 0.3
            || self.telling.is_some()
            || self.puzzle_active()
            || self
                .game
                .as_ref()
                .is_some_and(|game| game.outcome.is_none())
        {
            return None;
        }
        // A request or a question of its own isn't the answer; the window
        // stays open for the reply that is.
        if super::question::ask(text).is_some()
            || super::focus::requested(text).is_some()
            || super::breeze::requested(text).is_some()
            || super::music::lesson(text).is_some()
            || super::music::lullaby_requested(text)
            || super::music::own_song_requested(text)
            || super::puzzle::requested(text)
            || super::cranes::requested(text)
            || super::bubbles::requested(text)
            || super::tidy::requested(text)
            || super::show::requested(text)
            || super::quirks::yawn_message(text)
            || super::vocabulary::introduced_name(text).is_some()
            || super::vocabulary::given_name(text).is_some()
        {
            self.agenda_answer = Some((errand, day, until));
            return None;
        }
        let outcome = outcome(text, &self.ghost.name);
        let engagement = self
            .life
            .agenda
            .iter_mut()
            .find(|engagement| engagement.errand == errand && engagement.day == day)?;
        engagement.outcome = Some(outcome);
        self.begin_day();
        self.listening = Listening::default();
        self.notice_viewer();
        self.record(EventKind::Heard(text.to_owned()));
        match outcome {
            Outcome::Well => {
                if self.focus.is_none() && self.telling.is_none() {
                    self.begin(Activity::Dance);
                }
                self.suggest_mood(Mood::Excited, 20.0);
                self.say(if korean {
                    "잘 됐다니 정말 기뻐! 같이 축하하자!"
                } else {
                    "I'm so glad it went well! A little cheer for you!"
                });
                self.record(EventKind::PlanWentWell(errand));
            }
            Outcome::Badly => {
                self.care_for_feeling(Feeling::Sad, korean);
                self.say(if korean {
                    "힘들었겠다. 내가 곁에 있을게."
                } else {
                    "That sounds hard. I'll stay close."
                });
                self.record(EventKind::PlanWentBadly(errand));
            }
            Outcome::Unclear => {
                self.suggest_mood(Mood::Content, 12.0);
                self.say(if korean {
                    "이야기해 줘서 고마워. 듣고 있어."
                } else {
                    "Thank you for telling me. I'm listening."
                });
            }
        }
        Some(self.agenda_heard(
            Understanding::PlanAnswered {
                errand,
                went_well: match outcome {
                    Outcome::Well => Some(true),
                    Outcome::Badly => Some(false),
                    Outcome::Unclear => None,
                },
            },
            korean,
        ))
    }
}

fn outcome(text: &str, name: &str) -> Outcome {
    let text = text.to_lowercase().replace(['\'', '’'], "");
    let tokens = words(&text);
    let compact: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let tone = super::vocabulary::read(&text, name).tone;
    let negator = |word: &str| {
        matches!(
            word,
            "not" | "didnt" | "wasnt" | "isnt" | "dont" | "never" | "couldnt" | "hardly"
        )
    };
    // A negator governs the next word or the one after ("not too bad",
    // "didn't go well").
    let negates = |matching: fn(&str) -> bool| {
        tokens.iter().enumerate().any(|(index, word)| {
            negator(word)
                && tokens[index + 1..]
                    .iter()
                    .take(2)
                    .any(|next| matching(next))
        })
    };
    // "Not bad" is a modest yes; "didn't go well" is a no.
    if negates(|word| matches!(word, "bad" | "badly" | "terrible" | "awful"))
        || compact.contains("나쁘지않")
    {
        return Outcome::Well;
    }
    if negates(|word| matches!(word, "good" | "great" | "well" | "pass" | "passed")) {
        return Outcome::Badly;
    }
    if tokens
        .iter()
        .any(|word| matches!(*word, "no" | "nope" | "아니" | "아니야"))
        || tokens
            .iter()
            .any(|word| matches!(*word, "bad" | "badly" | "failed" | "terrible" | "awful"))
        || tokens
            .windows(2)
            .any(|part| matches!(part, ["not", "well" | "good" | "great"] | ["didnt", "pass"]))
        || ["망했", "떨어졌", "별로였", "못봤", "못했", "안됐", "안좋"]
            .iter()
            .any(|word| compact.contains(word))
    {
        return Outcome::Badly;
    }
    if tone == Some(Tone::Praise)
        || tokens
            .iter()
            .any(|word| matches!(*word, "good" | "great" | "well" | "passed" | "aced"))
        || tokens.windows(2).any(|part| part == ["nailed", "it"])
        || ["잘봤", "잘됐", "합격", "붙었", "좋았", "괜찮았"]
            .iter()
            .any(|word| compact.contains(word))
    {
        Outcome::Well
    } else {
        Outcome::Unclear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Game, Life, Mark};

    fn today() -> Date {
        Date::constant(2026, 10, 21)
    }

    fn fresh_world() -> World {
        let mut world = World::new(3, 12.0);
        world.date = today();
        world.begin_life(&today().to_string());
        world.time = 100.0;
        world.notice_viewer();
        world
    }

    fn settled(world: &mut World) {
        world.begin(Activity::Visit);
        world.ghost.plan.arrived = Some(world.time - 6.0);
        world.ghost.plan.done = false;
        world.ghost.speech = None;
        world.notice_viewer();
    }

    fn asking() -> World {
        let mut world = fresh_world();
        world.hear("I have an exam tomorrow");
        world.date = world.date.tomorrow().unwrap();
        world.hour = 19.0;
        settled(&mut world);
        world.follow_up_agenda();
        world.time += 13.0;
        world.follow_up_agenda();
        assert!(world.awaiting_plan_answer());
        world
    }

    #[test]
    fn a_request_while_an_answer_waits_goes_to_its_own_handler() {
        let mut world = fresh_world();
        world.agenda_answer = Some((Errand::Exam, today().to_string(), world.time + 180.0));
        world.life.agenda.push(Engagement {
            errand: Errand::Exam,
            day: today().to_string(),
            said: today().to_string(),
            wished: true,
            asked: true,
            asked_on: Some(today().to_string()),
            outcome: None,
        });
        world.hear("tell me a story");
        assert!(world.telling.is_some());
        assert!(world.awaiting_plan_answer());
        world.telling = None;
        for request in ["tidy up", "show me something", "bubbles!"] {
            assert!(world.answer_plan(request, false).is_none(), "{request}");
            assert!(world.awaiting_plan_answer());
        }
        let heard = world.hear("It didn't go well");
        assert_eq!(
            heard.understanding,
            Understanding::PlanAnswered {
                errand: Errand::Exam,
                went_well: Some(false)
            }
        );
    }

    #[test]
    fn a_break_suggestion_waits_while_a_plan_answer_is_pending() {
        let mut world = fresh_world();
        world.agenda_answer = Some((Errand::Exam, today().to_string(), world.time + 180.0));
        assert!(!world.rhythm_quiet());
        world.agenda_answer = None;
        world.rhythm.reply_until = Some(world.time + 60.0);
        assert!(!world.quiet_for_question());
    }

    #[test]
    fn first_person_english_plans_have_explicit_local_dates() {
        for (text, errand, days) in [
            ("I have an exam tomorrow", Errand::Exam, 1),
            ("I've got a job interview on Friday", Errand::Interview, 2),
            ("my presentation is tomorrow", Errand::Presentation, 1),
            (
                "tomorrow is my first day at the new job",
                Errand::FirstDay,
                1,
            ),
            ("I'm taking a test tomorrow", Errand::Exam, 1),
            ("I have to see the dentist tomorrow", Errand::Appointment, 1),
            ("I have a quiz today", Errand::Exam, 0),
            ("I have a date tonight", Errand::Date, 0),
            ("I have a meeting this morning", Errand::Meeting, 0),
            ("I have a talk this afternoon", Errand::Presentation, 0),
            ("I have a party this evening", Errand::Party, 0),
            ("I have a race the day after tomorrow", Errand::Match, 2),
            ("I have a tournament this weekend", Errand::Match, 3),
            ("I'm moving house next week", Errand::Moving, 7),
            ("I have a deadline on Monday", Errand::Deadline, 5),
            ("My submission is next Tuesday", Errand::Deadline, 6),
            (
                "I have a doctor's appointment tomorrow",
                Errand::Appointment,
                1,
            ),
            ("I'm playing in a concert tomorrow", Errand::Performance, 1),
            ("My recital is tomorrow", Errand::Performance, 1),
            (
                "My school application result day is tomorrow",
                Errand::Result,
                1,
            ),
            ("I have an exam this Wednesday", Errand::Exam, 0),
            ("I have an exam today on Wednesday", Errand::Exam, 0),
            (
                "I don't feel well. I have an exam tomorrow",
                Errand::Exam,
                1,
            ),
            ("I have an exam on Wednesday", Errand::Exam, 7),
            ("I have an exam next Friday", Errand::Exam, 9),
            ("I'm stressed. I have an exam tomorrow", Errand::Exam, 1),
        ] {
            assert_eq!(
                planned(text, today()),
                Some((
                    errand,
                    today().checked_add(jiff::Span::new().days(days)).unwrap()
                )),
                "{text}"
            );
        }
    }

    #[test]
    fn korean_plans_accept_natural_statements_and_time_particles() {
        for (text, errand, days) in [
            ("내일 시험 있어", Errand::Exam, 1),
            ("내일 시험이야", Errand::Exam, 1),
            ("모레 면접 봐", Errand::Interview, 2),
            ("금요일에 발표가 있어", Errand::Presentation, 2),
            ("내일 치과 가야 돼", Errand::Appointment, 1),
            ("다음 주에 이사해", Errand::Moving, 7),
            ("나는 내일 테스트 봐", Errand::Exam, 1),
            ("내일 병원 가", Errand::Appointment, 1),
            ("내일 진료 있어", Errand::Appointment, 1),
            ("오늘 밤 데이트 있어", Errand::Date, 0),
            ("이따 회의 있어", Errand::Meeting, 0),
            ("내일 미팅이야", Errand::Meeting, 1),
            ("주말에 경기 있어", Errand::Match, 3),
            ("내일 시합이야", Errand::Match, 1),
            ("내일 대회 있어", Errand::Match, 1),
            ("내일 공연 있어", Errand::Performance, 1),
            ("내일 연주회 있어", Errand::Performance, 1),
            ("내일 발표회 있어", Errand::Performance, 1),
            ("내일 마감이야", Errand::Deadline, 1),
            ("내일 제출해야 돼", Errand::Deadline, 1),
            ("오늘 파티 있어", Errand::Party, 0),
            ("내일 첫 출근이야", Errand::FirstDay, 1),
            ("내일 첫 등교야", Errand::FirstDay, 1),
            ("내일 합격 발표날이야", Errand::Result, 1),
            ("이번 주 수요일에 시험 있어", Errand::Exam, 0),
            ("다음 주 금요일에 시험 있어", Errand::Exam, 9),
            ("수요일에 시험 있어", Errand::Exam, 7),
            ("내일 시험 있어. 불안해", Errand::Exam, 1),
            ("내일 시험 있어. 오늘 너무 힘들었어", Errand::Exam, 1),
            ("오늘 수요일에 시험 있어", Errand::Exam, 0),
        ] {
            assert_eq!(
                planned(text, today()),
                Some((
                    errand,
                    today().checked_add(jiff::Span::new().days(days)).unwrap()
                )),
                "{text}"
            );
        }
    }

    #[test]
    fn questions_negations_past_tense_quotations_and_other_people_supply_no_plan() {
        for text in [
            "I have an exam",
            "Do I have an exam tomorrow?",
            "I have an exam tomorrow？",
            "Do you remember my exam tomorrow",
            "I don't have an exam tomorrow",
            "I do not have an exam tomorrow",
            "I have no exam tomorrow",
            "I had an exam tomorrow",
            "My exam was tomorrow",
            "I took a test tomorrow",
            "My sister has an exam tomorrow",
            "My sister's exam is tomorrow",
            "She has an exam tomorrow",
            "We have an exam tomorrow",
            "My friend's interview is tomorrow",
            "They said I have an exam tomorrow",
            "\"I have an exam tomorrow\"",
            "'I have an exam tomorrow'",
            "I have an 'exam' tomorrow",
            "내일 시험 있어’",
            "I have an exam today on Friday",
            "“내일 시험 있어”",
            "‘내일 시험 있어’",
            "내일 시험 없어",
            "내일 시험 안 봐",
            "내일 시험을 보지 않아",
            "내일 시험 봤어",
            "내일 시험이었어",
            "친구가 내일 면접이야",
            "내일 친구가 면접이야",
            "내일 언니 시험 있어",
            "시험 있어",
            "내일 시험 있어?",
            "내일 시험 있는지 알아",
            "I have a concert tomorrow",
            "I'm going to a concert tomorrow",
            "내일 공연 보러 가",
            "내일 여행 가",
            "I'll be back tomorrow",
            "I'll be away next week",
            "I have an exam tomorrow and a meeting today",
            "If I have an exam tomorrow",
            "I have an exam someday",
            "I might have an exam tomorrow",
            "My exam is cancelled tomorrow",
        ] {
            assert_eq!(planned(text, today()), None, "{text}");
        }
    }

    #[test]
    fn weekdays_and_weekends_roll_across_months_years_and_leap_days() {
        for (date_text, phrase, expected) in [
            ("2026-10-23", "on Friday", "2026-10-30"),
            ("2026-10-23", "this Friday", "2026-10-23"),
            ("2026-10-23", "next Friday", "2026-10-30"),
            ("2026-10-24", "next Friday", "2026-10-30"),
            ("2026-10-25", "on Monday", "2026-10-26"),
            ("2026-10-24", "this weekend", "2026-10-24"),
            ("2026-10-25", "this weekend", "2026-10-31"),
            ("2026-12-31", "tomorrow", "2027-01-01"),
            ("2026-12-31", "the day after tomorrow", "2027-01-02"),
            ("2028-02-28", "tomorrow", "2028-02-29"),
            ("2026-01-31", "next week", "2026-02-07"),
        ] {
            assert_eq!(
                planned(
                    &format!("I have an exam {phrase}"),
                    date_text.parse().unwrap()
                )
                .unwrap()
                .1
                .to_string(),
                expected
            );
        }
        for (phrase, expected) in [
            ("이번 주 토요일", "2026-10-24"),
            ("주말", "2026-10-24"),
            ("다음 주 금요일", "2026-10-30"),
        ] {
            assert_eq!(
                planned(
                    &format!("{phrase}에 시험 있어"),
                    Date::constant(2026, 10, 24)
                )
                .unwrap()
                .1
                .to_string(),
                expected
            );
        }
    }

    #[test]
    fn messages_replace_the_same_plan_keep_flags_and_never_teach_words_or_praise() {
        let mut world = fresh_world();
        let vocabulary = world.life.vocabulary.clone();
        for text in ["I have an exam tomorrow", "내일 시험 있어"] {
            let heard = world.hear(text);
            assert_eq!(heard.understanding, Understanding::Planned(Errand::Exam));
            assert!(heard.learned.is_empty());
            assert_eq!(heard.tone, None);
            assert_eq!(world.life.vocabulary, vocabulary);
        }
        assert_eq!(world.life.agenda.len(), 1);
        assert_eq!(world.life.today.plans_heard, 2);
        assert_eq!(world.life.today.praises, 0);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        world.life.agenda[0].wished = true;
        world.hear("I have an exam tomorrow");
        assert!(world.life.agenda[0].wished);
        for days in 1..=10 {
            world.date = today().checked_add(jiff::Span::new().days(days)).unwrap();
            world.hear("I have an exam tomorrow");
        }
        assert_eq!(world.life.agenda.len(), 8);
        assert_eq!(world.life.agenda[0].said, "2026-10-24");
    }

    #[test]
    fn damaged_or_unknown_saved_entries_do_not_discard_the_life() {
        let mut world = fresh_world();
        world.hear("I have an exam tomorrow");
        let valid = serde_json::to_value(&world.life.agenda[0]).unwrap();
        let mut value = serde_json::to_value(&world.life).unwrap();
        let mut entries = vec![valid.clone()];
        for (field, damaged) in [
            ("errand", serde_json::json!("unknown")),
            ("day", serde_json::json!("2026-02-30")),
            ("day", serde_json::json!("2026-1-01")),
            ("day", serde_json::json!("2026-12-31")),
            ("day", serde_json::json!("2026-08-01")),
            ("said", serde_json::json!("2026-10-23")),
            ("said", serde_json::json!("broken")),
            ("asked_on", serde_json::json!("2026-10-23")),
            ("outcome", serde_json::json!("well")),
        ] {
            let mut entry = valid.clone();
            entry[field] = damaged;
            entries.push(entry);
        }
        value["agenda"] = entries.into();
        value["bond"] = 0.4.into();
        let life: Life = serde_json::from_value(value).unwrap();
        let life = life.sanitized();
        assert_eq!(life.agenda.len(), 1);
        assert_eq!(life.bond, 0.4);
        let mut value = serde_json::to_value(&life).unwrap();
        value.as_object_mut().unwrap().remove("agenda");
        assert!(
            serde_json::from_value::<Life>(value)
                .unwrap()
                .agenda
                .is_empty()
        );
    }

    #[test]
    fn saved_flags_and_outcomes_survive_and_tallies_are_bounded() {
        let mut world = asking();
        world.hear("It went well");
        world.life.total.plans_heard = u32::MAX;
        world.life.total.luck_wished = u32::MAX;
        world.life.total.plans_went_well = u32::MAX;
        world.life.total.plans_went_badly = u32::MAX;
        world.life.total.good_plans = vec![Errand::Exam; 50];
        let restored: Life =
            serde_json::from_str(&serde_json::to_string(&world.life).unwrap()).unwrap();
        let restored = restored.sanitized();
        assert_eq!(restored.agenda[0].outcome, Some(Outcome::Well));
        assert!(restored.agenda[0].wished && restored.agenda[0].asked);
        assert_eq!(restored.total.good_plans, vec![Errand::Exam]);
        assert_eq!(restored.total.plans_heard, 100_000_000);
        assert_eq!(restored.total.luck_wished, 100_000_000);
        assert_eq!(restored.total.plans_went_well, 100_000_000);
        assert_eq!(restored.total.plans_went_badly, 100_000_000);
    }

    #[test]
    fn a_plan_and_a_feeling_are_both_remembered_through_the_shared_care_path() {
        for (text, feeling) in [
            ("I'm stressed. I have an exam tomorrow", Feeling::Stressed),
            ("내일 시험 있어. 불안해", Feeling::Stressed),
        ] {
            let mut world = fresh_world();
            assert_eq!(
                world.hear(text).understanding,
                Understanding::Planned(Errand::Exam)
            );
            assert_eq!(world.life.agenda.len(), 1);
            assert_eq!(world.life.feelings.last().unwrap().feeling, feeling);
            assert_eq!(world.ghost.plan.activity, Activity::Comfort);
            assert_eq!(world.comforting.as_ref().unwrap().feeling, feeling);
            assert_eq!(world.life.today.heard, 1);
        }
    }

    #[test]
    fn plan_messages_leave_sleep_focus_and_stories_in_control() {
        for condition in 0..3 {
            let mut world = fresh_world();
            match condition {
                0 => {
                    world.hour = 23.0;
                    world.tuck_in();
                }
                1 => world.start_focus(25),
                2 => world.tell(vec!["A quiet story.".to_owned()]),
                _ => unreachable!(),
            }
            let activity = world.ghost.plan.activity;
            let speech = world.ghost.speech.clone();
            let heard = world.hear("I have an exam tomorrow");
            assert_eq!(heard.understanding, Understanding::Planned(Errand::Exam));
            assert_eq!(world.ghost.plan.activity, activity);
            if condition == 0 || condition == 2 {
                assert_eq!(world.ghost.speech, speech);
            }
            if condition == 1 {
                assert!(world.focus.is_some());
            }
        }
    }

    #[test]
    fn luck_is_wished_once_on_the_day_after_the_greeting_settles() {
        let mut world = fresh_world();
        world.hear("I have an interview tomorrow");
        settled(&mut world);
        world.follow_up_agenda();
        assert_eq!(world.life.total.luck_wished, 0);
        world.date = world.date.tomorrow().unwrap();
        world.say("Welcome back!");
        world.follow_up_agenda();
        assert_eq!(world.life.total.luck_wished, 0);
        world.time += 13.0;
        let before = world.life.bond;
        world.follow_up_agenda();
        assert_eq!(world.life.today.luck_wished, 1);
        assert!(world.life.bond > before);
        assert!(world.life.agenda[0].wished);
        world.time += 20.0;
        world.follow_up_agenda();
        assert_eq!(world.life.total.luck_wished, 1);
        let mut restored = fresh_world();
        restored.life = world.life.clone().sanitized();
        restored.date = world.date;
        settled(&mut restored);
        restored.follow_up_agenda();
        assert_eq!(restored.life.total.luck_wished, 1);
    }

    #[test]
    fn today_gets_immediate_luck_but_sleepers_wait() {
        let mut world = fresh_world();
        world.hear("오늘 면접 있어");
        assert!(world.life.agenda[0].wished);
        assert_eq!(world.life.today.luck_wished, 1);
        assert!(world.ghost.speech.as_ref().unwrap().text.contains("응원"));
        world.hear("오늘 면접 있어");
        assert_eq!(world.life.today.luck_wished, 1);
        let mut sleeping = fresh_world();
        sleeping.hour = 23.0;
        sleeping.tuck_in();
        sleeping.hear("I have an exam today");
        assert!(!sleeping.life.agenda[0].wished);
    }

    #[test]
    fn quiet_settled_presence_is_required_for_luck_and_follow_up() {
        for condition in 0..12 {
            let mut world = fresh_world();
            world.hear("I have an exam tomorrow");
            world.date = world.date.tomorrow().unwrap();
            world.hour = 19.0;
            settled(&mut world);
            match condition {
                0 => world.ghost.plan.arrived = None,
                1 => world.ghost.plan.arrived = Some(world.time - 1.0),
                2 => world.start_focus(25),
                3 => world.tell(vec!["A quiet story.".to_owned()]),
                4 => world.game = Some(Game::new(glam::Vec2::ZERO, Mark::Cross, world.time)),
                5 => world.begin(Activity::Puzzle),
                6 => world.ghost.asleep = 1.0,
                7 => world.viewer.typing = true,
                8 => {
                    world.asking =
                        Some((crate::world::acquaintance::Topic::Color, world.time + 180.0))
                }
                9 => world.feeling_follow_up = Some((Feeling::Sad, world.time + 180.0)),
                10 => {
                    world.look_away(true);
                    world.time += 30.0;
                }
                11 => world.begin(Activity::Explore),
                _ => unreachable!(),
            }
            world.follow_up_agenda();
            assert_eq!(world.life.total.luck_wished, 0, "condition {condition}");
            assert!(!world.life.agenda[0].asked, "condition {condition}");
        }
    }

    #[test]
    fn questions_start_in_the_evening_and_lapse_after_three_days() {
        for (days, hour, expected) in [
            (0, 17.99, false),
            (0, 18.0, true),
            (1, 8.0, true),
            (3, 21.0, true),
            (4, 19.0, false),
        ] {
            let mut world = fresh_world();
            world.hear("I have an exam today");
            world.date = world
                .date
                .checked_add(jiff::Span::new().days(days))
                .unwrap();
            world.hour = hour;
            settled(&mut world);
            world.follow_up_agenda();
            assert_eq!(world.awaiting_plan_answer(), expected, "{days}, {hour}");
        }
    }

    #[test]
    fn feelings_and_acquaintance_questions_take_priority_and_keep_the_visit() {
        let mut world = fresh_world();
        world.hear("I'm sad. I have an exam tomorrow");
        world.date = world.date.tomorrow().unwrap();
        world.hour = 19.0;
        settled(&mut world);
        world.follow_up_feeling();
        world.follow_up_agenda();
        assert!(world.feeling_follow_up.is_some());
        assert!(!world.awaiting_plan_answer());
        world.hear("I'm happy");
        world.time += 13.0;
        world.follow_up_agenda();
        assert!(!world.awaiting_plan_answer());
        let mut world = fresh_world();
        world.life.bond = 0.5;
        world.hear("I have an exam tomorrow");
        world.date = world.date.tomorrow().unwrap();
        world.hour = 19.0;
        settled(&mut world);
        world.ask_viewer();
        world.follow_up_agenda();
        assert!(world.asking.is_some());
        assert!(!world.awaiting_plan_answer());
    }

    #[test]
    fn every_answer_outcome_is_consumed_without_learning_and_success_warms_the_bond() {
        for (text, went_well) in [
            ("It went well", Some(true)),
            ("passed", Some(true)),
            ("잘 봤어", Some(true)),
            ("합격", Some(true)),
            ("It went badly", Some(false)),
            ("not well", Some(false)),
            ("망했어", Some(false)),
            ("못 봤어", Some(false)),
            ("I don't know yet", None),
            ("It was unusual", None),
        ] {
            let mut world = asking();
            let before = world.life.bond;
            let vocabulary = world.life.vocabulary.clone();
            let heard = world.hear(text);
            assert_eq!(
                heard.understanding,
                Understanding::PlanAnswered {
                    errand: Errand::Exam,
                    went_well
                },
                "{text}"
            );
            assert_eq!(heard.tone, None);
            assert_eq!(world.life.vocabulary, vocabulary);
            assert!(!world.awaiting_plan_answer());
            match went_well {
                Some(true) => {
                    assert!(world.life.bond > before);
                    assert_eq!(world.life.today.plans_went_well, 1);
                    assert_eq!(world.ghost.plan.activity, Activity::Dance);
                    assert!(
                        world
                            .life
                            .milestones
                            .iter()
                            .any(|entry| entry.text == "You told me your exam went well.")
                    );
                }
                Some(false) => {
                    assert_eq!(world.life.today.plans_went_badly, 1);
                    assert_eq!(world.comforting.as_ref().unwrap().feeling, Feeling::Sad);
                    assert_eq!(world.ghost.plan.activity, Activity::Comfort);
                }
                None => assert_eq!(world.life.agenda[0].outcome, Some(Outcome::Unclear)),
            }
        }
    }

    #[test]
    fn all_documented_outcome_words_and_negative_phrases_are_understood() {
        for text in [
            "good",
            "great",
            "well",
            "passed",
            "nailed it",
            "aced",
            "잘 봤어",
            "잘 됐어",
            "합격",
            "붙었어",
            "좋았어",
            "괜찮았어",
        ] {
            assert_eq!(outcome(text, "Pip"), Outcome::Well, "{text}");
        }
        for text in [
            "bad",
            "badly",
            "failed",
            "terrible",
            "not well",
            "not good",
            "망했어",
            "떨어졌어",
            "별로였어",
            "못 봤어",
        ] {
            assert_eq!(outcome(text, "Pip"), Outcome::Badly, "{text}");
        }
        assert_eq!(outcome("I'm not sure yet", "Pip"), Outcome::Unclear);
        for text in [
            "It didn't go well",
            "it wasnt good",
            "I didn't pass",
            "not so great",
        ] {
            assert_eq!(outcome(text, "Pip"), Outcome::Badly, "{text}");
        }
        for text in ["not bad!", "it wasn't too bad", "나쁘지 않았어"] {
            assert_eq!(outcome(text, "Pip"), Outcome::Well, "{text}");
        }
        assert_eq!(outcome("잘 못했어", "Pip"), Outcome::Badly);
    }

    #[test]
    fn answer_windows_expire_and_only_the_next_message_answers() {
        for elapsed in [180.0, 180.1] {
            let mut world = asking();
            world.time += elapsed;
            let heard = world.hear("well");
            assert_eq!(
                matches!(heard.understanding, Understanding::PlanAnswered { .. }),
                elapsed <= 180.0
            );
            assert!(!world.awaiting_plan_answer());
            assert!(!matches!(
                world.hear("passed").understanding,
                Understanding::PlanAnswered { .. }
            ));
        }
        let mut world = asking();
        world.begin(Activity::Rest);
        assert!(!world.awaiting_plan_answer());
    }

    #[test]
    fn an_unanswered_question_limits_the_day_even_after_a_restart() {
        let mut world = asking();
        world.hear_agenda("I have a meeting today", false);
        world.time += 181.0;
        world.begin(Activity::Float);
        settled(&mut world);
        world.follow_up_agenda();
        assert!(!world.awaiting_plan_answer());
        let mut restored = fresh_world();
        restored.date = world.date;
        restored.hour = 19.0;
        restored.life = world.life.clone().sanitized();
        settled(&mut restored);
        restored.follow_up_agenda();
        assert!(!restored.awaiting_plan_answer());
        restored.date = restored.date.tomorrow().unwrap();
        settled(&mut restored);
        restored.follow_up_agenda();
        assert!(restored.awaiting_plan_answer());
    }

    #[test]
    fn answering_allows_another_plan_on_a_later_visit_that_day() {
        let mut world = asking();
        world.hear("great");
        world.hear("I have a meeting today");
        settled(&mut world);
        world.follow_up_agenda();
        assert!(world.awaiting_plan_answer());
        assert_eq!(world.agenda_answer.as_ref().unwrap().0, Errand::Meeting);
    }

    #[test]
    fn an_agenda_question_holds_the_floor_for_its_whole_visit() {
        let mut world = asking();
        world.life.bond = 0.7;
        world.life.nature.as_mut().unwrap().curious = 0.8;
        world.time += 601.0;
        world.last_message_at = Some(world.time);
        world.notice_viewer();
        world.ask_viewer();
        world.follow_up_feeling();
        assert!(world.asking.is_none());
        assert!(world.feeling_follow_up.is_none());
    }

    #[test]
    fn plans_reach_diaries_observations_thoughts_and_dreams_in_words() {
        let mut world = fresh_world();
        world.life.viewer_name = Some("Sam".to_owned());
        world.hear("I have an exam tomorrow");
        assert!(thoughts(&world).contains(&"I hope Sam's exam goes well tomorrow.".to_owned()));
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("you told me about your exam")
        );
        assert!(memories(&world.life.agenda, world.date).contains("exam tomorrow"));
        let mut world = asking();
        assert!(memories(&world.life.agenda, world.date).contains("already asked"));
        world.hear("great");
        assert!(
            world
                .life
                .today
                .diary()
                .to_lowercase()
                .contains("your exam went well")
        );
        assert!(
            crate::mind::musing::dreams(&world)
                .iter()
                .any(|line| line.contains("exam"))
        );
        assert!(
            !memories(&world.life.agenda, world.date)
                .chars()
                .any(|character| character.is_ascii_digit())
        );
        for days in [0, 1, 2, 7, 9, -1, -3] {
            let words = EventKind::HeardPlan {
                errand: Errand::Exam,
                days,
            }
            .words();
            assert!(!words.chars().any(|character| character.is_ascii_digit()));
        }
    }
}
