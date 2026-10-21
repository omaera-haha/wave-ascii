use super::random::Random;
use crate::sdf::smoothstep;

/// What the sky outside the window is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sky {
    Clear,
    Rain,
    /// The bright spell after daytime rain, with a rainbow.
    Rainbow,
}

/// What changed outside when the weather moved on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    RainStarted,
    RainStopped,
    RainbowAppeared,
    RainbowFaded,
}

/// The weather outside the window. Clear spells give way to showers now and
/// then; a shower that ends by day leaves a rainbow for a while.
#[derive(Clone, Debug, PartialEq)]
pub struct Weather {
    pub sky: Sky,
    /// When the sky took its current state, in world seconds.
    pub since: f64,
    /// When it changes next.
    pub until: f64,
}

/// Seconds rain takes to set in or clear, and a rainbow to appear or fade.
const RAIN_EASE: f64 = 20.0;
const RAINBOW_EASE: f64 = 10.0;

impl Weather {
    /// Clear skies, with the first shower due in a few minutes.
    pub fn new(random: &mut Random) -> Self {
        Self {
            sky: Sky::Clear,
            since: 0.0,
            until: f64::from(random.range(180.0, 420.0)),
        }
    }

    /// Moves the weather on once its spell is over. `daytime` decides
    /// whether a shower ends in a rainbow.
    pub fn step(&mut self, time: f64, daytime: bool, random: &mut Random) -> Option<Change> {
        if time < self.until {
            return None;
        }
        let (sky, length, change) = match self.sky {
            Sky::Clear => (Sky::Rain, random.range(120.0, 300.0), Change::RainStarted),
            Sky::Rain if daytime => (
                Sky::Rainbow,
                random.range(90.0, 180.0),
                Change::RainbowAppeared,
            ),
            Sky::Rain => (Sky::Clear, random.range(600.0, 1800.0), Change::RainStopped),
            Sky::Rainbow => (
                Sky::Clear,
                random.range(600.0, 1800.0),
                Change::RainbowFaded,
            ),
        };
        self.sky = sky;
        self.since = time;
        self.until = time + f64::from(length);
        Some(change)
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
            Sky::Rain if self.rain(time) < 0.5 => "a light rain",
            Sky::Rain => "rain",
            Sky::Rainbow => "a rainbow after the rain",
            Sky::Clear if night => "a clear night sky",
            Sky::Clear => "a clear sky",
        }
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
            assert!((180.0..420.0).contains(&weather.until));
            let start = weather.until;
            assert_eq!(
                weather.step(start, daytime, &mut random),
                Some(Change::RainStarted)
            );
            assert_eq!(weather.sky, Sky::Rain);
            assert_eq!(weather.step(start + 1.0, daytime, &mut random), None);
            let change = weather.step(weather.until, daytime, &mut random);
            assert_eq!(weather.sky, ending);
            if daytime {
                assert_eq!(change, Some(Change::RainbowAppeared));
                assert_eq!(
                    weather.step(weather.until, daytime, &mut random),
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
}
