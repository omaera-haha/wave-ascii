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
}

impl Occasion {
    pub fn name(self) -> &'static str {
        match self {
            Self::Halloween => "Halloween",
            Self::WinterHolidays => "winter holidays",
            Self::NewYear => "New Year",
            Self::Valentine => "Valentine's Day",
            Self::Birthday => "birthday",
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Self::WinterHolidays => "the winter holidays",
            Self::Birthday => "the ghost's birthday",
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
        }
    }
}

/// The day's occasion, with birthdays taking precedence over holidays.
pub fn occasion(date: Date, hour: f32, birthday: Option<Date>) -> Option<Occasion> {
    if birthday.is_some_and(|birthday| birthday_years(date, birthday).is_some()) {
        return Some(Occasion::Birthday);
    }
    match (date.month(), date.day()) {
        (12, 31) if hour >= 20.0 => Some(Occasion::NewYear),
        (1, 1) => Some(Occasion::NewYear),
        (10, 24..=31) => Some(Occasion::Halloween),
        (12, 18..=31) => Some(Occasion::WinterHolidays),
        (2, 14) => Some(Occasion::Valentine),
        _ => None,
    }
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
            Occasion::Birthday => "a birthday cake with candles",
            Occasion::Valentine => "",
        }
    }
}

impl super::World {
    pub fn occasion(&self) -> Option<Occasion> {
        occasion(self.date, self.hour, self.birthday)
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
            Some(Occasion::NewYear) if self.date.month() == 12 => "New Year's Eve".to_owned(),
            Some(Occasion::NewYear) => "New Year's Day".to_owned(),
            Some(occasion) => occasion.words().to_owned(),
            None => "an ordinary day".to_owned(),
        }
    }

    pub fn decoration(&self) -> Option<Decoration> {
        let occasion = self.occasion()?;
        if occasion == Occasion::Valentine {
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
        if event.kind == super::EventKind::Celebrated(Occasion::NewYear)
            && self.new_year_at != Some(event.time)
        {
            "it is New Year".to_owned()
        } else {
            event.kind.words()
        }
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
        if occasion == Some(Occasion::Birthday) {
            self.ghost.mood = self.feeling();
        }
        self.calendar_at = Some(self.date);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        world.step(0.0);
        assert_eq!(world.ghost.mood, super::super::Mood::Cheerful);
    }
}
