use super::{Season, random::Random};
use crate::sdf::smoothstep;

/// What the sky outside the window is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sky {
    Clear,
    Rain,
    Snow,
    /// The bright spell after daytime rain, with a rainbow.
    Rainbow,
}

/// What changed outside when the weather moved on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    RainStarted,
    StormStarted,
    Thunder,
    RainStopped,
    SnowStarted,
    SnowStopped,
    RainbowAppeared,
    RainbowFaded,
}

/// The weather outside the window. Clear spells give way to showers now and
/// then, sometimes with thunder; winter brings snow, and rain that ends by
/// day leaves a rainbow.
#[derive(Clone, Debug, PartialEq)]
pub struct Weather {
    pub sky: Sky,
    /// Whether the current rain is a thunderstorm.
    pub storm: bool,
    /// When lightning last flashed, and when the next strike is due.
    pub last_flash: f64,
    pub next_flash: f64,
    /// When the last strike's thunder will reach the room.
    pub thunder_at: Option<f64>,
    /// When the sky took its current state, in world seconds.
    pub since: f64,
    /// When it changes next.
    pub until: f64,
    /// How much snow lies on the hills, from bare to fully covered.
    pub snow_cover: f32,
    season: Season,
    /// The last deep snowfall that settled, held until its cover thaws.
    pub settled_snow: Option<f64>,
}

/// Seconds rain or snow takes to set in or clear, and a rainbow to appear or fade.
const RAIN_EASE: f64 = 20.0;
const RAINBOW_EASE: f64 = 10.0;

impl Weather {
    /// Clear skies, with the first shower due in a few minutes.
    pub fn new(random: &mut Random) -> Self {
        Self {
            sky: Sky::Clear,
            storm: false,
            last_flash: f64::NEG_INFINITY,
            next_flash: f64::INFINITY,
            thunder_at: None,
            since: 0.0,
            until: f64::from(random.range(180.0, 420.0)) * clear_factor(Season::Summer),
            snow_cover: 0.0,
            season: Season::Summer,
            settled_snow: None,
        }
    }

    /// Moves the weather on once its spell is over. `daytime` decides
    /// whether a shower ends in a rainbow.
    pub fn step(
        &mut self,
        time: f64,
        daytime: bool,
        season: Season,
        random: &mut Random,
    ) -> Option<Change> {
        if self.sky == Sky::Clear && season != self.season {
            self.until = time
                + (self.until - time).max(0.0) * clear_factor(season) / clear_factor(self.season);
        }
        self.season = season;
        let thaw = self.sky == Sky::Snow && season != Season::Winter;
        let freeze = self.sky == Sky::Rain && season == Season::Winter;
        if time < self.until && !thaw && !freeze {
            return None;
        }
        let (sky, length, change) = match self.sky {
            Sky::Clear | Sky::Rain if season == Season::Winter => {
                (Sky::Snow, random.range(120.0, 300.0), Change::SnowStarted)
            }
            Sky::Clear => {
                let chance = if season == Season::Summer { 0.3 } else { 0.15 };
                let change = if random.unit() < chance {
                    Change::StormStarted
                } else {
                    Change::RainStarted
                };
                (Sky::Rain, random.range(120.0, 300.0), change)
            }
            Sky::Rain if daytime => (
                Sky::Rainbow,
                random.range(90.0, 180.0),
                Change::RainbowAppeared,
            ),
            Sky::Rain => (Sky::Clear, random.range(600.0, 1800.0), Change::RainStopped),
            Sky::Snow => (Sky::Clear, random.range(600.0, 1800.0), Change::SnowStopped),
            Sky::Rainbow => (
                Sky::Clear,
                random.range(600.0, 1800.0),
                Change::RainbowFaded,
            ),
        };
        if change == Change::SnowStopped && self.snow_cover >= 0.65 {
            self.settled_snow = Some(time);
        }
        self.sky = sky;
        self.storm = change == Change::StormStarted;
        self.next_flash = f64::INFINITY;
        self.last_flash = f64::NEG_INFINITY;
        self.since = time;
        self.until = time
            + f64::from(length)
                * if sky == Sky::Clear {
                    clear_factor(season)
                } else {
                    1.0
                };
        Some(change)
    }

    /// Builds snow during snowfall and melts it more slowly at night.
    pub fn advance(&mut self, time: f64, seconds: f32, daytime: bool) {
        let snow = self.snow(time);
        let melting_time = if daytime { 1200.0 } else { 3600.0 };
        self.snow_cover = (self.snow_cover
            + seconds.max(0.0) * (snow / 120.0 - (1.0 - snow) / melting_time))
            .clamp(0.0, 1.0);
        if self.snow_cover <= 0.05 {
            self.settled_snow = None;
        }
    }

    /// Schedules lightning in heavy storm rain and delivers each strike's
    /// thunder a little later. The world calls this each slice.
    pub fn advance_storm(&mut self, time: f64, random: &mut Random) -> Option<Change> {
        let thunder = self.thunder_at.is_some_and(|due| time >= due);
        if thunder {
            self.thunder_at = None;
        }
        if self.storm && self.rain(time) > 0.6 {
            if self.next_flash.is_infinite() {
                self.next_flash = time + f64::from(random.range(6.0, 25.0));
            } else if time >= self.next_flash {
                self.last_flash = self.next_flash;
                self.next_flash = self.last_flash + f64::from(random.range(6.0, 25.0));
                self.thunder_at = Some(self.last_flash + f64::from(random.range(1.0, 4.0)));
            }
        }
        thunder.then_some(Change::Thunder)
    }

    /// The brightness of a sharp double flicker just after a lightning strike.
    pub fn flash(&self, time: f64) -> f32 {
        let elapsed = time - self.last_flash;
        if !self.storm || self.sky != Sky::Rain || !(0.0..0.35).contains(&elapsed) {
            return 0.0;
        }
        let first = (1.0 - elapsed / 0.1).max(0.0);
        let second = if elapsed >= 0.14 {
            0.8 * (1.0 - (elapsed - 0.14) / 0.21).max(0.0)
        } else {
            0.0
        };
        first.max(second) as f32
    }

    /// How hard it snows at `time`, from 0 to 1, setting in and clearing gradually.
    pub fn snow(&self, time: f64) -> f32 {
        if self.sky != Sky::Snow {
            return 0.0;
        }
        ease(time, self.since, self.until, RAIN_EASE)
    }

    /// How hard it rains at `time`, from 0 to 1, setting in and clearing
    /// gradually.
    pub fn rain(&self, time: f64) -> f32 {
        if self.sky != Sky::Rain {
            return 0.0;
        }
        ease(time, self.since, self.until, RAIN_EASE)
    }

    /// How bright the rainbow is at `time`, from 0 to 1.
    pub fn rainbow(&self, time: f64) -> f32 {
        if self.sky != Sky::Rainbow {
            return 0.0;
        }
        ease(time, self.since, self.until, RAINBOW_EASE)
    }

    /// The weather in a few words.
    pub fn words(&self, time: f64, night: bool) -> &'static str {
        match self.sky {
            Sky::Rain if self.storm => "a thunderstorm",
            Sky::Rain if self.rain(time) < 0.5 => "a light rain",
            Sky::Rain => "rain",
            Sky::Snow if self.snow_cover > 0.5 => "snow falling, the hills white",
            Sky::Snow => "snow falling",
            Sky::Rainbow => "a rainbow after the rain",
            Sky::Clear if night => "a clear night sky",
            Sky::Clear => "a clear sky",
        }
    }
}

fn clear_factor(season: Season) -> f64 {
    match season {
        Season::Spring => 0.7,
        Season::Summer => 1.4,
        Season::Autumn | Season::Winter => 1.0,
    }
}

// Rises from 0 at `start` to 1 within `ease` seconds and falls back to 0
// over the last `ease` seconds before `end`.
fn ease(time: f64, start: f64, end: f64, ease: f64) -> f32 {
    let rise = smoothstep(0.0, ease as f32, (time - start) as f32);
    let fall = smoothstep(0.0, ease as f32, (end - time) as f32);
    rise * fall
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn showers_come_and_end_in_a_rainbow_only_by_day() {
        for (daytime, ending) in [(true, Sky::Rainbow), (false, Sky::Clear)] {
            let mut random = Random::new(1);
            let mut weather = Weather::new(&mut random);
            assert!((252.0..588.0).contains(&weather.until));
            let start = weather.until;
            assert!(matches!(
                weather.step(start, daytime, Season::Summer, &mut random),
                Some(Change::RainStarted | Change::StormStarted)
            ));
            assert_eq!(weather.sky, Sky::Rain);
            assert_eq!(
                weather.step(start + 1.0, daytime, Season::Summer, &mut random),
                None
            );
            let change = weather.step(weather.until, daytime, Season::Summer, &mut random);
            assert_eq!(weather.sky, ending);
            if daytime {
                assert_eq!(change, Some(Change::RainbowAppeared));
                assert_eq!(
                    weather.step(weather.until, daytime, Season::Summer, &mut random),
                    Some(Change::RainbowFaded)
                );
            } else {
                assert_eq!(change, Some(Change::RainStopped));
            }
            assert_eq!(weather.sky, Sky::Clear);
        }
    }

    #[test]
    fn rain_and_rainbows_set_in_and_fade_gradually() {
        let weather = Weather {
            sky: Sky::Rain,
            since: 100.0,
            until: 300.0,
            ..Weather::new(&mut Random::new(1))
        };
        assert_eq!(weather.rain(100.0), 0.0);
        assert!((0.1..0.9).contains(&weather.rain(110.0)));
        assert_eq!(weather.rain(200.0), 1.0);
        assert_eq!(weather.rain(300.0), 0.0);
        assert_eq!(weather.rainbow(200.0), 0.0);
        let rainbow = Weather {
            sky: Sky::Rainbow,
            ..weather
        };
        assert_eq!(rainbow.rainbow(200.0), 1.0);
        assert_eq!(rainbow.rain(200.0), 0.0);
        assert_eq!(rainbow.words(200.0, false), "a rainbow after the rain");
    }

    #[test]
    fn winter_showers_are_snow_and_never_leave_a_rainbow() {
        for daytime in [false, true] {
            for seed in 0..40 {
                let mut random = Random::new(seed);
                let mut weather = Weather::new(&mut random);
                weather.step(0.0, daytime, Season::Winter, &mut random);
                for _ in 0..10 {
                    let start = weather.until;
                    assert_eq!(
                        weather.step(start, daytime, Season::Winter, &mut random),
                        Some(Change::SnowStarted)
                    );
                    assert_eq!(weather.sky, Sky::Snow);
                    assert!(!weather.storm);
                    assert_eq!(weather.snow(start), 0.0);
                    assert!((0.0..1.0).contains(&weather.snow(start + 10.0)));
                    assert_eq!(weather.snow(start + 30.0), 1.0);
                    assert_eq!(weather.rain(start + 30.0), 0.0);
                    assert_eq!(weather.rainbow(start + 30.0), 0.0);
                    assert_eq!(weather.snow(weather.until), 0.0);
                    assert_eq!(
                        weather.step(weather.until, daytime, Season::Winter, &mut random),
                        Some(Change::SnowStopped)
                    );
                    assert_eq!(weather.sky, Sky::Clear);
                }
            }
        }
    }

    #[test]
    fn snow_builds_in_minutes_and_melts_more_slowly_at_night() {
        let mut weather = Weather::new(&mut Random::new(1));
        weather.sky = Sky::Snow;
        weather.until = 10_000.0;
        for slice in 0..1200 {
            weather.advance(30.0 + f64::from(slice) * 0.1, 0.1, true);
        }
        assert!(weather.snow_cover > 0.999);
        weather.advance(200.0, 600.0, true);
        assert_eq!(weather.snow_cover, 1.0);
        weather.sky = Sky::Clear;
        let mut night = weather.clone();
        for slice in 0..12_000 {
            let time = 800.0 + f64::from(slice) * 0.1;
            weather.advance(time, 0.1, true);
            night.advance(time, 0.1, false);
            assert!((0.0..=1.0).contains(&weather.snow_cover));
            assert!((0.0..=1.0).contains(&night.snow_cover));
        }
        assert!(weather.snow_cover < 0.001);
        assert!((night.snow_cover - 2.0 / 3.0).abs() < 0.001);
        night.advance(2000.0, 2400.0, false);
        assert!(night.snow_cover < 0.001);
        night.advance(4400.0, 10_000.0, false);
        assert_eq!(night.snow_cover, 0.0);
    }

    #[test]
    fn leaving_winter_stops_snow_and_allows_the_cover_to_melt() {
        let mut random = Random::new(2);
        let mut weather = Weather::new(&mut random);
        weather.sky = Sky::Snow;
        weather.until = 1000.0;
        weather.snow_cover = 1.0;
        assert_eq!(
            weather.step(100.0, true, Season::Spring, &mut random),
            Some(Change::SnowStopped)
        );
        assert_eq!(weather.snow(100.0), 0.0);
        weather.advance(100.0, 600.0, true);
        assert!((weather.snow_cover - 0.5).abs() < 1e-5);
        weather.sky = Sky::Rain;
        assert_eq!(
            weather.step(101.0, true, Season::Winter, &mut random),
            Some(Change::SnowStarted)
        );
        assert_eq!(weather.rain(101.0), 0.0);
    }

    #[test]
    fn spring_clear_spells_are_shorter_than_summer_clear_spells() {
        let average = |season| {
            let mut total = 0.0;
            for seed in 0..200 {
                let mut random = Random::new(seed);
                let mut weather = Weather::new(&mut random);
                weather.step(0.0, false, season, &mut random);
                total += weather.until;
                for _ in 0..5 {
                    weather.step(weather.until, false, season, &mut random);
                    weather.step(weather.until, false, season, &mut random);
                    total += weather.until - weather.since;
                }
            }
            total / 1200.0
        };
        let spring = average(Season::Spring);
        let summer = average(Season::Summer);
        assert!((spring / summer - 0.5).abs() < 1e-6, "{spring} {summer}");
    }

    #[test]
    fn storms_are_more_common_in_summer_and_never_start_in_winter() {
        let fraction = |season| {
            let mut storms = 0;
            for seed in 0..2000 {
                let mut random = Random::new(seed);
                let mut weather = Weather::new(&mut random);
                let change = weather.step(weather.until, true, season, &mut random);
                assert_eq!(weather.storm, change == Some(Change::StormStarted));
                if weather.storm {
                    storms += 1;
                    assert_eq!(weather.words(weather.since, false), "a thunderstorm");
                }
            }
            storms as f32 / 2000.0
        };
        let summer = fraction(Season::Summer);
        assert!((0.24..0.36).contains(&summer), "{summer}");
        for season in [Season::Spring, Season::Autumn] {
            let other = fraction(season);
            assert!((0.1..0.2).contains(&other), "{season:?}: {other}");
            assert!(other < summer);
        }
        assert_eq!(fraction(Season::Winter), 0.0);
    }

    #[test]
    fn every_strike_has_one_delayed_thunderclap_and_the_next_strike_is_scheduled() {
        for seed in 0..40 {
            let mut random = Random::new(seed);
            let mut weather = Weather::new(&mut random);
            weather.sky = Sky::Rain;
            weather.storm = true;
            weather.until = 1000.0;
            weather.next_flash = 30.0;
            let mut strikes = 0;
            while weather.next_flash < 950.0 {
                let strike = weather.next_flash;
                assert_eq!(weather.advance_storm(strike - 0.001, &mut random), None);
                assert_eq!(weather.advance_storm(strike, &mut random), None);
                assert_eq!(weather.last_flash, strike);
                assert!((6.0..=25.0).contains(&(weather.next_flash - strike)));
                let thunder = weather.thunder_at.unwrap();
                assert!((1.0..=4.0).contains(&(thunder - strike)));
                assert_eq!(weather.advance_storm(thunder - 0.001, &mut random), None);
                assert_eq!(
                    weather.advance_storm(thunder, &mut random),
                    Some(Change::Thunder)
                );
                assert_eq!(weather.thunder_at, None);
                assert_eq!(weather.advance_storm(thunder + 0.001, &mut random), None);
                strikes += 1;
            }
            assert!(strikes > 30);
        }
    }

    #[test]
    fn lightning_only_strikes_in_heavy_storm_rain_and_flashes_stay_bounded() {
        let mut random = Random::new(3);
        let mut weather = Weather::new(&mut random);
        weather.sky = Sky::Rain;
        weather.storm = true;
        weather.until = 200.0;
        weather.next_flash = 30.0;
        for time in [0.0, 5.0, 10.0] {
            assert_eq!(weather.advance_storm(time, &mut random), None);
            assert_eq!(weather.last_flash, f64::NEG_INFINITY);
            assert_eq!(weather.flash(time), 0.0);
        }
        weather.advance_storm(30.0, &mut random);
        for step in -10..=100 {
            let time = 30.0 + f64::from(step) * 0.01;
            let brightness = weather.flash(time);
            assert!((0.0..=1.0).contains(&brightness));
            if !(30.0..30.35).contains(&time) {
                assert_eq!(brightness, 0.0);
            }
        }
        assert!(weather.flash(30.0) > weather.flash(30.12));
        assert!(weather.flash(30.15) > weather.flash(30.12));
        weather.next_flash = 195.0;
        weather.advance_storm(195.0, &mut random);
        assert_eq!(weather.last_flash, 30.0);
        for sky in [Sky::Clear, Sky::Snow, Sky::Rainbow] {
            weather.sky = sky;
            weather.next_flash = 30.0;
            weather.advance_storm(30.1, &mut random);
            assert_eq!(weather.flash(30.1), 0.0);
            assert_eq!(weather.last_flash, 30.0);
        }
        weather.sky = Sky::Rain;
        weather.storm = false;
        weather.advance_storm(30.1, &mut random);
        assert_eq!(weather.flash(30.1), 0.0);
        assert_eq!(weather.last_flash, 30.0);
    }

    #[test]
    fn a_storm_waits_for_heavy_rain_before_scheduling_its_first_strike() {
        let mut random = Random::new(1);
        let mut weather = Weather::new(&mut random);
        weather.sky = Sky::Rain;
        weather.storm = true;
        weather.until = 200.0;
        weather.advance_storm(10.0, &mut random);
        assert_eq!(weather.next_flash, f64::INFINITY);
        weather.advance_storm(12.0, &mut random);
        assert!((6.0..=25.0).contains(&(weather.next_flash - 12.0)));
        assert_eq!(weather.last_flash, f64::NEG_INFINITY);
        assert_eq!(weather.thunder_at, None);
    }
}
