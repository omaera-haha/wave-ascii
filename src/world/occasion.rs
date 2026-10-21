use glam::Vec3;
use jiff::civil::Date;
use serde::{Deserialize, Serialize};

/// A special day in the ghost's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Occasion {
    Halloween,
    WinterHolidays,
    NewYear,
    Valentine,
    Birthday,
    Seollal,
    Chuseok,
    HangulDay,
    ViewerBirthday,
    HundredDays,
    ThousandDays,
}

impl Occasion {
    pub fn name(self) -> &'static str {
        match self {
            Self::Halloween => "Halloween",
            Self::WinterHolidays => "winter holidays",
            Self::NewYear => "New Year",
            Self::Valentine => "Valentine's Day",
            Self::Birthday => "birthday",
            Self::ViewerBirthday => "your birthday",
            Self::HundredDays => "hundredth day",
            Self::ThousandDays => "thousandth day",
            Self::Seollal => "Seollal",
            Self::Chuseok => "Chuseok",
            Self::HangulDay => "Hangul Day",
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Self::WinterHolidays => "the winter holidays",
            Self::Birthday => "the ghost's birthday",
            Self::HundredDays => "the ghost's hundredth day",
            Self::ThousandDays => "the ghost's thousandth day",
            _ => self.name(),
        }
    }

    pub fn diary_words(self) -> &'static str {
        match self {
            Self::Halloween => "celebrated Halloween",
            Self::WinterHolidays => "celebrated the winter holidays",
            Self::NewYear => "celebrated the new year",
            Self::Valentine => "celebrated Valentine's Day",
            Self::Birthday => "had a birthday",
            Self::ViewerBirthday => "it was your birthday",
            Self::HundredDays => "celebrated my hundredth day",
            Self::ThousandDays => "celebrated my thousandth day",
            Self::Seollal => "celebrated Seollal",
            Self::Chuseok => "celebrated Chuseok",
            Self::HangulDay => "celebrated Hangul Day",
        }
    }
}

/// The day's occasion, with birthdays taking precedence over holidays.
pub fn occasion(date: Date, hour: f32, birthday: Option<Date>) -> Option<Occasion> {
    if birthday.is_some_and(|birthday| birthday_years(date, birthday).is_some()) {
        return Some(Occasion::Birthday);
    }
    if let Some(landmark) = birthday.and_then(|born| super::anniversary::landmark(born, date)) {
        return Some(landmark);
    }
    if matches!((date.month(), date.day()), (12, 31) if hour >= 20.0)
        || (date.month(), date.day()) == (1, 1)
    {
        return Some(Occasion::NewYear);
    }
    if let Some(holiday) = lunar_holiday(date) {
        return Some(holiday);
    }
    match (date.month(), date.day()) {
        (10, 24..=31) => Some(Occasion::Halloween),
        (12, 18..=31) => Some(Occasion::WinterHolidays),
        (2, 14) => Some(Occasion::Valentine),
        (10, 9) => Some(Occasion::HangulDay),
        _ => None,
    }
}

// The lunar holidays are tabulated only for these years; outside them
// there are no Seollal or Chuseok celebrations.
const LUNAR_DATES: [((i8, i8), (i8, i8)); 10] = [
    ((2, 17), (9, 25)),
    ((2, 7), (9, 15)),
    ((1, 27), (10, 3)),
    ((2, 13), (9, 22)),
    ((2, 3), (9, 12)),
    ((1, 23), (10, 1)),
    ((2, 11), (9, 19)),
    ((1, 31), (9, 8)),
    ((2, 19), (9, 27)),
    ((2, 8), (9, 16)),
];

fn lunar_holiday(date: Date) -> Option<Occasion> {
    if !matches!(date.month(), 1 | 2 | 9 | 10) {
        return None;
    }
    let index = usize::try_from(date.year() - 2026).ok()?;
    let &(seollal, chuseok) = LUNAR_DATES.get(index)?;
    for ((month, day), holiday) in [(seollal, Occasion::Seollal), (chuseok, Occasion::Chuseok)] {
        if date.month() == month && (date.day() - day).abs() <= 1 {
            return Some(holiday);
        }
        // Only January's end and October's beginning cross a month in this table.
        if (month == 1 && day == 31 && date.month() == 2 && date.day() == 1)
            || (month == 10 && day == 1 && date.month() == 9 && date.day() == 30)
        {
            return Some(holiday);
        }
    }
    None
}

/// A sebae begins after reaching the glass, then dips, holds, and rises.
#[derive(Clone, Copy, Debug)]
pub(super) struct Bow {
    pub anchor: Vec3,
    pub started: Option<f64>,
}

/// Years completed on this anniversary, or none on any other day.
pub fn birthday_years(date: Date, birthday: Date) -> Option<u16> {
    if date.year() <= birthday.year() {
        return None;
    }
    let day = if birthday.month() == 2 && birthday.day() == 29 && !date.in_leap_year() {
        28
    } else {
        birthday.day()
    };
    (date.month() == birthday.month() && date.day() == day)
        .then_some((i32::from(date.year()) - i32::from(birthday.year())) as u16)
}

pub fn ordinal(years: u16) -> &'static str {
    match years {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        5 => "fifth",
        6 => "sixth",
        7 => "seventh",
        8 => "eighth",
        9 => "ninth",
        10 => "tenth",
        _ => "another",
    }
}

/// The nights and hours when fireworks welcome the new year.
pub fn fireworks(date: Date, hour: f32) -> bool {
    matches!((date.month(), date.day()), (12, 31) if hour >= 23.5)
        || matches!((date.month(), date.day()), (1, 1) if hour < 1.0)
}

/// A temporary piece of furniture at the back right of the room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decoration {
    pub occasion: Occasion,
    pub candles: u16,
}

impl Decoration {
    pub const POSITION: Vec3 = Vec3::new(0.95, 0.0, -1.12);

    pub fn radius(self) -> f32 {
        match self.occasion {
            Occasion::WinterHolidays | Occasion::NewYear => 0.23,
            Occasion::Seollal
            | Occasion::Chuseok
            | Occasion::HundredDays
            | Occasion::ThousandDays => 0.25,
            _ => 0.18,
        }
    }

    pub fn height(self) -> f32 {
        match self.occasion {
            Occasion::WinterHolidays | Occasion::NewYear => 0.68,
            Occasion::Halloween => 0.37,
            _ => 0.32,
        }
    }

    pub fn words(self) -> &'static str {
        match self.occasion {
            Occasion::Halloween => "a glowing jack-o'-lantern",
            Occasion::WinterHolidays | Occasion::NewYear => "a little tree with twinkling lights",
            Occasion::Birthday | Occasion::ViewerBirthday => "a birthday cake with candles",
            Occasion::Valentine | Occasion::HangulDay => "",
            Occasion::Seollal => "a little table with rice-cake soup and a lucky pouch",
            Occasion::Chuseok => "a little table with half-moon rice cakes and persimmons",
            Occasion::HundredDays => {
                "a low table with a layered white rice cake and a tiny paper flag"
            }
            Occasion::ThousandDays => {
                "a low table with two layered white rice cakes and a tiny paper flag"
            }
        }
    }
}

impl super::World {
    pub fn occasion(&self) -> Option<Occasion> {
        if self
            .birthday
            .is_some_and(|birthday| birthday_years(self.date, birthday).is_some())
        {
            Some(Occasion::Birthday)
        } else if self
            .life
            .viewer
            .birthday
            .is_some_and(|birthday| birthday.falls_on(self.date))
        {
            Some(Occasion::ViewerBirthday)
        } else {
            occasion(self.date, self.hour, self.birthday)
        }
    }

    pub fn occasion_words(&self) -> String {
        match self.occasion() {
            Some(Occasion::Birthday) => {
                let years = birthday_years(self.date, self.birthday.unwrap()).unwrap();
                if years > 10 {
                    "another birthday".to_owned()
                } else {
                    format!("its {} birthday", ordinal(years))
                }
            }
            Some(Occasion::HundredDays) => "its hundredth day of life".to_owned(),
            Some(Occasion::ThousandDays) => "its thousandth day of life".to_owned(),
            Some(Occasion::NewYear) if self.date.month() == 12 => "New Year's Eve".to_owned(),
            Some(Occasion::NewYear) => "New Year's Day".to_owned(),
            Some(occasion) => occasion.words().to_owned(),
            None => "an ordinary day".to_owned(),
        }
    }

    pub fn decoration(&self) -> Option<Decoration> {
        let occasion = self.occasion()?;
        if matches!(occasion, Occasion::Valentine | Occasion::HangulDay) {
            return None;
        }
        let candles = self
            .birthday
            .and_then(|birthday| birthday_years(self.date, birthday))
            .unwrap_or(1)
            .min(5);
        Some(Decoration { occasion, candles })
    }

    pub fn fireworks(&self) -> bool {
        fireworks(self.date, self.hour)
    }

    /// The evening's New Year celebration comes before midnight.
    pub fn event_words(&self, event: &super::Event) -> String {
        if let super::EventKind::LearnedAboutViewer(topic) = event.kind
            && let Some(answer) = self.life.viewer.answer(topic, false)
        {
            return if topic == super::acquaintance::Topic::Birthday {
                format!("learned that your birthday is on {answer}")
            } else {
                format!("learned that your {} is {answer}", topic.name())
            };
        }
        if matches!(event.kind, super::EventKind::FriendArrived(_))
            && self.first_friend_at == Some(event.time)
        {
            return "a ghost you've never met floated in through the window".to_owned();
        }
        if event.kind == super::EventKind::Celebrated(Occasion::NewYear)
            && self.new_year_at != Some(event.time)
        {
            "it is New Year".to_owned()
        } else {
            event.kind.words()
        }
    }

    pub fn chuseok_night(&self) -> bool {
        (self.hour >= 18.0 || self.hour < 6.5)
            && self.occasion() == Some(Occasion::Chuseok)
            && self.weather.sky == super::Sky::Clear
    }

    pub(super) fn holiday_greeting(&mut self) {
        if self.focus.is_some() || !self.viewer_present() || self.ghost.asleep >= 0.3 {
            return;
        }
        let Some(occasion) = self.occasion() else {
            return;
        };
        if self.last_holiday_greeting == Some((self.date, occasion)) {
            return;
        }
        match occasion {
            Occasion::Seollal => {
                self.begin(super::Activity::Visit);
                self.sebae = Some(Bow {
                    anchor: self.ghost.plan.target,
                    started: None,
                });
            }
            Occasion::ViewerBirthday => {
                if self.telling.is_some()
                    || self.game.is_some()
                    || self.puzzle_active()
                    || self.reading.is_some()
                    || self.asking.is_some()
                    || self
                        .ghost
                        .speech
                        .as_ref()
                        .is_some_and(|line| self.time - line.time < 12.0)
                    || self.life.viewer.birthday_greeted.as_deref()
                        == Some(self.date.to_string().as_str())
                {
                    return;
                }
                let name = self
                    .life
                    .viewer_name
                    .as_ref()
                    .map_or(String::new(), |name| format!(", {name}"));
                let greeting = if self.speaks_korean() {
                    format!("생일 축하해{name}!")
                } else {
                    format!("Happy birthday{name}!")
                };
                self.say(&greeting);
                self.write(if self.speaks_korean() {
                    "생일축하"
                } else {
                    "HBD♥"
                });
                self.life.viewer.birthday_greeted = Some(self.date.to_string());
                self.reconsider_wardrobe();
                self.write_birthday_letter();
            }
            Occasion::HundredDays | Occasion::ThousandDays => {
                if !self.greet_landmark(occasion) {
                    return;
                }
            }
            Occasion::Chuseok => self.say(self.spoken("Happy Chuseok!", "즐거운 한가위 보내!")),
            Occasion::HangulDay if self.speaks_korean() => self.say("한글은 그리는 것도 재밌어!"),
            _ => return,
        }
        self.last_holiday_greeting = Some((self.date, occasion));
    }

    pub(super) fn bow(&mut self) {
        let Some(mut bow) = self.sebae else { return };
        self.ghost.plan.target = bow.anchor;
        if bow.started.is_none() && self.ghost.position.distance(bow.anchor) < super::ARRIVED {
            bow.started = Some(self.time);
            self.say(self.spoken(
                "Happy New Year! May luck find you.",
                "새해 복 많이 받으세요!",
            ));
        }
        if let Some(started) = bow.started {
            let elapsed = (self.time - started) as f32;
            let dip = if elapsed < 0.6 {
                elapsed / 0.6
            } else if elapsed < 1.3 {
                1.0
            } else {
                ((2.0 - elapsed) / 0.7).max(0.0)
            };
            self.ghost.plan.target = bow.anchor + Vec3::new(0.0, -0.42, 0.16) * dip;
            if elapsed >= 2.0 {
                self.sebae = None;
                self.sebae_gift_until = self.time + 20.0;
                return;
            }
        }
        self.sebae = Some(bow);
    }

    pub(super) fn wish_on_moon(&mut self) {
        if !self.chuseok_night() || self.ghost.asleep >= 0.3 || self.ghost.plan.arrived.is_none() {
            return;
        }
        let night = if self.hour < 6.5 {
            self.date.yesterday().unwrap()
        } else {
            self.date
        };
        if self.wished_night == Some(night) {
            return;
        }
        let lines = if self.life.bond >= 0.55 {
            [
                "I wish we could watch the moon together every year.",
                "May every harvest moon bring you back to my window.",
                "I wish this moonlight could keep you company too.",
            ]
        } else {
            [
                "I wish someone would watch the moon with me.",
                "May the coming days be gentle.",
                "I wish the moon could see my little room.",
            ]
        };
        let index = self.random.range(0.0, lines.len() as f32) as usize;
        self.think(lines[index]);
        self.attend("window", 30.0);
        self.record(super::EventKind::MoonWish);
        self.wished_night = Some(night);
    }

    pub(super) fn celebrate(&mut self) {
        let midnight = self.date.month() == 1
            && self.date.day() == 1
            && (self.calendar_at.is_some_and(|previous| {
                previous.month() == 12
                    && previous.day() == 31
                    && (self.date - previous).get_days() == 1
            }) || (self.calendar_at.is_none() && self.hour == 0.0));
        let occasion = self.occasion();
        if let Some(occasion) = occasion
            && self.last_celebrated != Some((self.date, occasion))
        {
            self.record(super::EventKind::Celebrated(occasion));
            self.last_celebrated = Some((self.date, occasion));
        }
        if midnight {
            if occasion != Some(Occasion::NewYear) {
                self.record(super::EventKind::Celebrated(Occasion::NewYear));
            }
            self.new_year_at = Some(self.time);
        }
        if matches!(
            occasion,
            Some(
                Occasion::Birthday
                    | Occasion::ViewerBirthday
                    | Occasion::HundredDays
                    | Occasion::ThousandDays
            )
        ) {
            self.ghost.mood = self.feeling();
        }
        self.calendar_at = Some(self.date);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Activity, EventKind, Mood, Sky, Thing, World};

    #[test]
    fn lunar_holidays_span_three_days_on_every_tabulated_year() {
        for (index, &(seollal, chuseok)) in LUNAR_DATES.iter().enumerate() {
            let year = 2026 + index as i16;
            for ((month, day), expected) in
                [(seollal, Occasion::Seollal), (chuseok, Occasion::Chuseok)]
            {
                let date = Date::new(year, month, day).unwrap();
                for neighbor in [date.yesterday().unwrap(), date, date.tomorrow().unwrap()] {
                    assert_eq!(occasion(neighbor, 12.0, None), Some(expected), "{neighbor}");
                }
                for outside in [
                    date.yesterday().unwrap().yesterday().unwrap(),
                    date.tomorrow().unwrap().tomorrow().unwrap(),
                ] {
                    assert_ne!(occasion(outside, 12.0, None), Some(expected), "{outside}");
                }
                let birthday = Date::new(year - 1, month, day).unwrap();
                assert_eq!(
                    occasion(date, 12.0, Some(birthday)),
                    Some(Occasion::Birthday)
                );
            }
        }
        for year in [2025, 2036] {
            let mut date = Date::new(year, 1, 1).unwrap();
            while date.year() == year {
                assert!(!matches!(
                    occasion(date, 12.0, None),
                    Some(Occasion::Seollal | Occasion::Chuseok)
                ));
                date = date.tomorrow().unwrap();
            }
        }
        assert_eq!(
            occasion(Date::constant(2029, 2, 14), 12.0, None),
            Some(Occasion::Seollal)
        );
        assert_eq!(
            occasion(Date::constant(2026, 10, 9), 12.0, None),
            Some(Occasion::HangulDay)
        );
        assert_eq!(occasion(Date::constant(2026, 10, 8), 12.0, None), None);
        assert_eq!(occasion(Date::constant(2026, 10, 10), 12.0, None), None);
    }

    fn finish_bow(world: &mut World) {
        world.holiday_greeting();
        world.ghost.position = world.sebae.unwrap().anchor;
        world.bow();
        world.time += 0.8;
        world.bow();
        assert!(world.ghost.plan.target.y < world.sebae.unwrap().anchor.y);
        world.time += 1.3;
        world.bow();
        assert!(world.sebae.is_none());
    }

    #[test]
    fn seollal_greets_an_awake_viewer_with_one_bow_each_day() {
        let mut world = World::new(8, 12.0);
        world.date = Date::constant(2027, 2, 7);
        world.life.korean = 1.0;
        world.notice_viewer();
        world.viewer_arrived();
        world.ghost.asleep = 1.0;
        world.holiday_greeting();
        assert!(world.sebae.is_none());
        assert!(world.ghost.speech.is_none());
        world.ghost.asleep = 0.0;
        world.holiday_greeting();
        world.hear("안녕");
        assert!(world.sebae.is_some());
        finish_bow(&mut world);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "새해 복 많이 받으세요!"
        );
        world.viewer_arrived();
        world.holiday_greeting();
        assert!(world.sebae.is_none());
        assert_eq!(world.events.iter().filter(|event| matches!(&event.kind, EventKind::Said(text) if text == "새해 복 많이 받으세요!")).count(), 1);
        world.date = world.date.tomorrow().unwrap();
        world.life.korean = 0.0;
        finish_bow(&mut world);
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "Happy New Year! May luck find you."
        );
    }

    #[test]
    fn an_already_present_viewer_gets_the_bow_when_the_ghost_wakes() {
        let mut world = World::new(8, 12.0);
        world.notice_viewer();
        world.step(0.1);
        world.date = Date::constant(2027, 2, 7);
        world.ghost.asleep = 1.0;
        world.begin(Activity::Rest);
        world.step(0.1);
        assert!(world.sebae.is_none());
        world.wake();
        world.step(0.1);
        assert!(world.sebae.is_some());
    }

    #[test]
    fn a_light_mote_soon_after_sebae_is_a_new_years_gift() {
        for korean in [false, true] {
            let mut world = World::new(8, 12.0);
            world.date = Date::constant(2027, 2, 7);
            world.life.korean = if korean { 1.0 } else { 0.0 };
            world.notice_viewer();
            finish_bow(&mut world);
            world.time += 19.0;
            world.drop_mote(world.ghost.position);
            assert_eq!(
                world.ghost.speech.as_ref().unwrap().text,
                if korean {
                    "세뱃돈이다!"
                } else {
                    "A New Year's gift!"
                }
            );
            assert_eq!(world.feeling(), Mood::Excited);
            assert_eq!(world.sebae_gift_until, 0.0);
            world.begin(Activity::Chase);
            // A miss now and then only delays the catch.
            for _ in 0..30 {
                if world.life.total.motes > 0 {
                    break;
                }
                world.step(0.1);
            }
            assert_eq!(world.life.total.motes, 1);
            world.time += 21.0;
            world.drop_mote(world.ghost.position);
            assert_eq!(world.events.iter().filter(|event| matches!(&event.kind, EventKind::Said(text) if text.contains("gift") || text.contains("세뱃돈"))).count(), 1);
        }
        let mut world = World::new(8, 12.0);
        world.date = Date::constant(2027, 2, 7);
        world.notice_viewer();
        finish_bow(&mut world);
        world.time += 20.1;
        world.drop_mote(world.ghost.position);
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "Happy New Year! May luck find you."
        );
    }

    #[test]
    fn chuseok_wishes_wait_for_a_clear_window_and_happen_once_per_night() {
        let mut world = World::new(8, 21.0);
        world.date = Date::constant(2026, 9, 25);
        world.life.bond = 0.8;
        world.begin(Activity::Window);
        world.wish_on_moon();
        assert_eq!(world.life.total.moon_wishes, 0);
        world.ghost.plan.arrived = Some(world.time);
        world.weather.sky = Sky::Rain;
        world.wish_on_moon();
        assert_eq!(world.life.total.moon_wishes, 0);
        world.weather.sky = Sky::Clear;
        world.wish_on_moon();
        let thought = world.ghost.thought.clone();
        assert_eq!(world.life.total.moon_wishes, 1);
        assert!(matches!(world.ghost.attention, Some((Thing::Window, _))));
        world.time += 10.0;
        world.wish_on_moon();
        assert_eq!(world.ghost.thought, thought);
        world.date = world.date.tomorrow().unwrap();
        world.hour = 0.5;
        world.wish_on_moon();
        assert_eq!(world.life.total.moon_wishes, 1);
        world.hour = 21.0;
        world.wish_on_moon();
        assert_eq!(world.life.total.moon_wishes, 2);
        assert!(
            world
                .life
                .today
                .words()
                .contains("wish on the Chuseok moon")
        );
    }

    #[test]
    fn korean_holiday_greetings_wait_for_the_viewer_and_repeat_only_on_a_new_day() {
        for (date, greeting) in [
            (Date::constant(2026, 9, 25), "즐거운 한가위 보내!"),
            (Date::constant(2026, 10, 9), "한글은 그리는 것도 재밌어!"),
        ] {
            let mut world = World::new(8, 12.0);
            world.date = date;
            world.life.korean = 1.0;
            world.holiday_greeting();
            assert!(world.ghost.speech.is_none());
            world.notice_viewer();
            world.step(0.1);
            world.step(0.1);
            assert_eq!(world.ghost.speech.as_ref().unwrap().text, greeting);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| matches!(event.kind, EventKind::Said(_)))
                    .count(),
                1
            );
            assert_eq!(world.life.total.celebrations, 1);
            assert!(
                world
                    .life
                    .milestones
                    .iter()
                    .any(|entry| entry.text.contains("first"))
            );
            if date.month() == 10 {
                assert!(world.decoration().is_none());
            }
        }
    }

    #[test]
    fn holidays_begin_and_end_at_their_calendar_boundaries() {
        for (month, day, hour, expected) in [
            (10, 23, 23.999, None),
            (10, 24, 0.0, Some(Occasion::Halloween)),
            (10, 31, 23.999, Some(Occasion::Halloween)),
            (11, 1, 0.0, None),
            (12, 17, 23.999, None),
            (12, 18, 0.0, Some(Occasion::WinterHolidays)),
            (12, 30, 23.999, Some(Occasion::WinterHolidays)),
            (12, 31, 19.999, Some(Occasion::WinterHolidays)),
            (12, 31, 20.0, Some(Occasion::NewYear)),
            (12, 31, 23.999, Some(Occasion::NewYear)),
            (1, 1, 0.0, Some(Occasion::NewYear)),
            (1, 1, 23.999, Some(Occasion::NewYear)),
            (1, 2, 0.0, None),
            (2, 13, 23.999, None),
            (2, 14, 0.0, Some(Occasion::Valentine)),
            (2, 14, 23.999, Some(Occasion::Valentine)),
            (2, 15, 0.0, None),
        ] {
            let date = Date::new(2026, month, day).unwrap();
            assert_eq!(occasion(date, hour, None), expected, "{date} {hour}");
        }
    }

    #[test]
    fn birthdays_take_precedence_over_every_holiday() {
        for (month, day, hour) in [
            (10, 31, 12.0),
            (12, 24, 12.0),
            (12, 31, 22.0),
            (1, 1, 0.0),
            (2, 14, 12.0),
        ] {
            let birthday = Date::new(2025, month, day).unwrap();
            let date = Date::new(2026, month, day).unwrap();
            assert_eq!(
                occasion(date, hour, Some(birthday)),
                Some(Occasion::Birthday)
            );
        }
    }

    #[test]
    fn birthdays_start_with_the_first_anniversary_and_respect_leap_days() {
        let birthday = Date::constant(2024, 2, 29);
        for (date, years) in [
            (Date::constant(2020, 2, 29), None),
            (birthday, None),
            (Date::constant(2025, 2, 27), None),
            (Date::constant(2025, 2, 28), Some(1)),
            (Date::constant(2025, 3, 1), None),
            (Date::constant(2028, 2, 28), None),
            (Date::constant(2028, 2, 29), Some(4)),
        ] {
            assert_eq!(birthday_years(date, birthday), years);
            assert_eq!(
                occasion(date, 12.0, Some(birthday)),
                years.map(|_| Occasion::Birthday)
            );
        }
        let birthday = Date::constant(2026, 3, 1);
        assert_eq!(occasion(birthday, 12.0, Some(birthday)), None);
        assert_eq!(
            occasion(Date::constant(2025, 3, 1), 12.0, Some(birthday)),
            None
        );
        assert_eq!(
            birthday_years(Date::constant(2027, 3, 1), birthday),
            Some(1)
        );
    }

    #[test]
    fn birthday_ordinals_are_words_through_ten_and_then_another() {
        let words = [
            "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth",
            "tenth", "another",
        ];
        for (index, word) in words.into_iter().enumerate() {
            assert_eq!(ordinal(index as u16 + 1), word);
        }
        assert_eq!(ordinal(100), "another");
    }

    #[test]
    fn fireworks_span_the_last_half_hour_and_the_first_hour() {
        let eve = Date::constant(2026, 12, 31);
        let day = Date::constant(2027, 1, 1);
        assert!(!fireworks(eve, 23.499));
        assert!(fireworks(eve, 23.5));
        assert!(fireworks(eve, 23.999));
        assert!(fireworks(day, 0.0));
        assert!(fireworks(day, 0.999));
        assert!(!fireworks(day, 1.0));
        assert!(!fireworks(Date::constant(2027, 1, 2), 0.0));
    }

    #[test]
    fn the_world_uses_its_own_date_hour_and_birth_date() {
        let mut world = super::super::World::new(1, 19.0);
        assert_eq!(world.birthday, None);
        assert_eq!(world.occasion(), None);
        world.date = Date::constant(2026, 12, 31);
        assert_eq!(world.occasion(), Some(Occasion::WinterHolidays));
        world.hour = 20.0;
        assert_eq!(world.occasion(), Some(Occasion::NewYear));
        world.birthday = Some(Date::constant(2025, 12, 31));
        assert_eq!(world.occasion(), Some(Occasion::Birthday));
        assert_eq!(world.occasion_words(), "its first birthday");
        world.date = Date::constant(2030, 12, 31);
        assert_eq!(world.decoration().unwrap().candles, 5);
        world.date = Date::constant(2031, 12, 31);
        assert_eq!(world.decoration().unwrap().candles, 5);
        world.date = Date::constant(2036, 12, 31);
        assert_eq!(world.occasion_words(), "another birthday");
        // Past the first moments' excitement, the day stays cheerful.
        world.step(7.0);
        assert_eq!(world.ghost.mood, super::super::Mood::Cheerful);
    }
}
