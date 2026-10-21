//! The ghost feels a shower coming a few minutes before it falls: the air
//! changes, its hem tingles. It says so, and is pleased when it was right.
//! The weather is already scheduled, so sensing draws nothing from the
//! world's random source.

use super::{EventKind, GREETS_FROM, Season, Sky, World};

/// Seconds of warning the ghost feels before rain or snow, from the
/// least curious ghost to the most.
const LEAD: (f64, f64) = (240.0, 480.0);
/// Seconds after the feeling within which the weather proves it right.
const PROVEN_WITHIN: f64 = 900.0;
/// Seconds its own words get before it speaks of the weather.
const QUIET_FOR: f64 = 12.0;

// From 0 to 1, how faintly the shower due at `until` gives itself away: a
// SplitMix64 hash of its time, so feeling it draws nothing from the world.
fn felt(until: f64) -> f64 {
    let mut mixed = until.to_bits().wrapping_add(0x9E37_79B9_7F4A_7C15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^= mixed >> 31;
    (mixed >> 11) as f64 / (1u64 << 53) as f64
}

/// What the ghost felt coming, for the shower due at `until`; a shower it
/// didn't notice is kept with no time it was felt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Forecast {
    pub until: f64,
    pub snow: bool,
    pub at: f64,
}

impl World {
    /// Lets an awake ghost feel the next shower coming, once per shower.
    pub(super) fn sense_weather(&mut self) {
        let until = self.weather.until;
        if self.weather.sky != Sky::Clear
            || self.ghost.asleep >= 0.3
            || self
                .forecast
                .is_some_and(|forecast| forecast.until == until)
        {
            return;
        }
        let curious = f64::from(self.life.temperament().curious);
        let lead = LEAD.0 + (LEAD.1 - LEAD.0) * curious;
        let ahead = until - self.time;
        if !(0.0..=lead).contains(&ahead) {
            return;
        }
        // Not every shower gives itself away; a curious ghost notices more.
        if felt(until) >= 0.35 + 0.4 * curious {
            self.forecast = Some(Forecast {
                until,
                snow: false,
                at: f64::NEG_INFINITY,
            });
            return;
        }
        let snow = self.season() == Season::Winter;
        self.forecast = Some(Forecast {
            until,
            snow,
            at: self.time,
        });
        self.think(if snow {
            "The air smells like snow."
        } else {
            "My hem tingles… rain is coming."
        });
        let speaking = self.telling.is_some()
            || self.asking.is_some()
            || self.focus.is_some()
            || self
                .ghost
                .speech
                .as_ref()
                .is_some_and(|line| self.time - line.time < QUIET_FOR);
        if self.viewer_present() && self.life.bond >= GREETS_FROM && !speaking {
            let korean = self.speaks_korean();
            self.say(match (snow, korean) {
                (false, false) => "I think it's going to rain soon.",
                (false, true) => "곧 비가 올 것 같아.",
                (true, false) => "I think it's going to snow soon.",
                (true, true) => "곧 눈이 올 것 같아.",
            });
        }
    }

    /// When rain or snow begins, a feeling that foretold it comes true.
    pub(super) fn weather_came(&mut self) {
        let Some(forecast) = self.forecast else {
            return;
        };
        if forecast.until.is_infinite() || self.time - forecast.at > PROVEN_WITHIN {
            return;
        }
        self.forecast = Some(Forecast {
            until: f64::INFINITY,
            ..forecast
        });
        if self.ghost.asleep < 0.3 {
            self.think("I knew it!");
        }
        self.record(EventKind::ForecastCameTrue {
            snow: forecast.snow,
        });
    }

    /// The shower the ghost feels coming, if it does.
    pub fn sensed_weather(&self) -> Option<bool> {
        self.forecast
            .filter(|forecast| {
                forecast.at.is_finite() && forecast.until.is_finite() && forecast.until > self.time
            })
            .map(|forecast| forecast.snow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A clear sky with a shower due in about `seconds`, one that gives
    // itself away.
    fn clear_with_rain_due(seconds: f64) -> World {
        let mut world = World::new(5, 12.0);
        world.step(0.0);
        world.weather.sky = Sky::Clear;
        world.weather.storm = false;
        let mut until = world.time + seconds;
        while felt(until) >= 0.35 {
            until += 0.001;
        }
        world.weather.until = until;
        world.ghost.asleep = 0.0;
        world
    }

    #[test]
    fn some_showers_go_unnoticed_and_curiosity_notices_more() {
        let noticed = |curious: f32| {
            (0..400)
                .filter(|index| {
                    let mut world = World::new(5, 12.0);
                    world.life.nature = Some(crate::world::Nature {
                        curious,
                        ..Default::default()
                    });
                    world.weather.sky = Sky::Clear;
                    world.weather.until = world.time + 60.0 + f64::from(*index) * 7.3;
                    world.time = world.weather.until - 60.0;
                    world.ghost.asleep = 0.0;
                    world.sense_weather();
                    world.sensed_weather().is_some()
                })
                .count()
        };
        let (dull, keen) = (noticed(0.0), noticed(1.0));
        assert!((100..180).contains(&dull), "{dull}");
        assert!((260..340).contains(&keen), "{keen}");
    }

    #[test]
    fn a_shower_is_felt_minutes_ahead_once_and_its_coming_is_remembered() {
        let mut world = clear_with_rain_due(LEAD.0 - 1.0);
        let mut drawn = world.random.clone();
        world.sense_weather();
        assert_eq!(world.sensed_weather(), Some(false));
        assert_eq!(
            world.ghost.thought.as_ref().unwrap().text,
            "My hem tingles… rain is coming."
        );
        // Sensing is no choice: nothing was drawn.
        assert_eq!(world.random.next_u64(), drawn.next_u64());
        world.ghost.thought = None;
        world.sense_weather();
        assert!(world.ghost.thought.is_none());
        world.time = world.weather.until;
        world.weather.sky = Sky::Rain;
        world.weather_came();
        assert_eq!(world.sensed_weather(), None);
        assert_eq!(world.ghost.thought.as_ref().unwrap().text, "I knew it!");
        assert_eq!(world.life.today.forecasts, 1);
        assert_eq!(
            world.life.milestones.last().unwrap().text,
            "I felt the rain coming before it fell."
        );
        let before = world.life.today.forecasts;
        world.weather_came();
        assert_eq!(world.life.today.forecasts, before);
    }

    #[test]
    fn far_off_showers_sleepers_and_a_wet_sky_go_unfelt() {
        let mut world = clear_with_rain_due(LEAD.1 + 60.0);
        world.sense_weather();
        assert_eq!(world.sensed_weather(), None);
        let mut world = clear_with_rain_due(60.0);
        world.ghost.asleep = 1.0;
        world.sense_weather();
        assert_eq!(world.sensed_weather(), None);
        let mut world = clear_with_rain_due(60.0);
        world.weather.sky = Sky::Rain;
        world.sense_weather();
        assert_eq!(world.sensed_weather(), None);
        // Rain that falls without a feeling proves nothing.
        let mut world = clear_with_rain_due(60.0);
        world.weather_came();
        assert_eq!(world.life.today.forecasts, 0);
    }

    #[test]
    fn a_trusting_ghost_tells_a_present_viewer_in_their_language() {
        for (korean, snow, expected) in [
            (false, false, "I think it's going to rain soon."),
            (true, false, "곧 비가 올 것 같아."),
            (false, true, "I think it's going to snow soon."),
            (true, true, "곧 눈이 올 것 같아."),
        ] {
            let mut world = clear_with_rain_due(60.0);
            if snow {
                world.date = jiff::civil::Date::constant(2027, 1, 10);
            }
            world.notice_viewer();
            world.life.bond = 0.5;
            world.life.korean = if korean { 1.0 } else { 0.0 };
            world.ghost.speech = None;
            world.sense_weather();
            assert_eq!(world.ghost.speech.as_ref().unwrap().text, expected);
        }
        let mut stranger = clear_with_rain_due(60.0);
        stranger.notice_viewer();
        stranger.life.bond = 0.0;
        stranger.ghost.speech = None;
        stranger.sense_weather();
        assert!(stranger.ghost.speech.is_none());
    }
}
