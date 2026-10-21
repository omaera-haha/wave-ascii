use jiff::civil::Date;

/// The northern hemisphere's season, by calendar month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    pub fn of(date: Date) -> Self {
        match date.month() {
            3..=5 => Self::Spring,
            6..=8 => Self::Summer,
            9..=11 => Self::Autumn,
            _ => Self::Winter,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Spring => "spring",
            Self::Summer => "summer",
            Self::Autumn => "autumn",
            Self::Winter => "winter",
        }
    }
}

/// The moon's mean phase, from new at zero through full at one half.
/// Local dates and hours are treated as UTC, within a day of the true phase.
pub fn moon_phase(date: Date, hour: f32) -> f32 {
    let epoch = Date::constant(2000, 1, 6);
    let days = f64::from((date - epoch).get_days());
    let elapsed = days + (f64::from(hour) - (18.0 + 14.0 / 60.0)) / 24.0;
    ((elapsed.rem_euclid(29.530_588_853) / 29.530_588_853) as f32).min(1.0 - f32::EPSILON)
}

/// The visible moon in words, or no moon close to the new moon.
pub fn moon_words(phase: f32) -> Option<&'static str> {
    let fullness = phase.min(1.0 - phase);
    match fullness {
        fullness if fullness < 0.035 => None,
        fullness if fullness < 0.19 => Some("a thin crescent moon"),
        fullness if fullness < 0.31 => Some("a half moon"),
        fullness if fullness < 0.465 => Some("a gibbous moon"),
        _ => Some("a full moon"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seasons_change_at_month_boundaries() {
        for (month, season) in [
            (1, Season::Winter),
            (2, Season::Winter),
            (3, Season::Spring),
            (4, Season::Spring),
            (5, Season::Spring),
            (6, Season::Summer),
            (7, Season::Summer),
            (8, Season::Summer),
            (9, Season::Autumn),
            (10, Season::Autumn),
            (11, Season::Autumn),
            (12, Season::Winter),
        ] {
            let first = Date::new(2024, month, 1).unwrap();
            let last = Date::new(2024, month, first.days_in_month()).unwrap();
            assert_eq!(Season::of(first), season);
            assert_eq!(Season::of(last), season);
        }
    }

    #[test]
    fn the_mean_phase_matches_known_new_and_full_moons() {
        for (date, expected) in [
            (Date::constant(2024, 4, 8), 0.0),
            (Date::constant(2026, 2, 17), 0.0),
            (Date::constant(2024, 4, 23), 0.5),
            (Date::constant(2026, 3, 3), 0.5),
        ] {
            let phase = moon_phase(date, 12.0);
            let difference = (phase - expected).abs();
            assert!(difference.min(1.0 - difference) < 0.045, "{date}: {phase}");
        }
        let epoch = moon_phase(Date::constant(2000, 1, 6), 18.0 + 14.0 / 60.0);
        assert!(epoch.min(1.0 - epoch) < 1e-6);
    }

    #[test]
    fn the_phase_stays_in_range_across_many_years() {
        for year in (-9000..=9000).step_by(11) {
            for month in 1..=12 {
                for hour in [0.0, 6.0, 12.0, 23.999, 24.0] {
                    let phase = moon_phase(Date::new(year, month, 1).unwrap(), hour);
                    assert!((0.0..1.0).contains(&phase), "{year}-{month}: {phase}");
                }
            }
        }
    }

    #[test]
    fn moon_words_follow_the_phase_on_both_sides_of_full() {
        for phase in [0.0, 0.01, 0.99] {
            assert_eq!(moon_words(phase), None);
        }
        for (phase, words) in [
            (0.1, "a thin crescent moon"),
            (0.25, "a half moon"),
            (0.4, "a gibbous moon"),
            (0.5, "a full moon"),
        ] {
            assert_eq!(moon_words(phase), Some(words));
            assert_eq!(moon_words(1.0 - phase), Some(words));
        }
    }
}
