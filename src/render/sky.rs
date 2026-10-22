//! The open sky over the yard, all by direction: azimuth from straight back
//! (toward -z) turning positive toward +x, and elevation over the horizon.
//! The back of the house faces the sun's side of the sky.

use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};

use crate::sdf::{capsule, coverage, mix, smoothstep};
use crate::world::meteor::Streak;
use crate::world::passerby::{Passerby, Passing};
use crate::world::stars::{self, Constellation};
use crate::world::{BirdPicture, Keepsake, Season};

/// Radians one unit of a sky drawing spans: discs, sparks, and things
/// passing overhead are drawn in these.
pub const UNIT: f32 = 0.38;
/// Height of the cloud deck, in room units per unit of ray rise.
const CLOUD_DECK: f32 = 14.0;
/// The sky is drawn dimmer than its light so its glyphs stay airy and the
/// house and the ghost stand out against it.
const EXPOSURE: f32 = 0.27;
/// A rainbow's radius around the point opposite the sun, and the second
/// bow's.
const BOW: f32 = 0.733;
const SECOND_BOW: f32 = 0.89;

#[derive(Clone, Debug, PartialEq)]
pub struct Outside {
    pub hour: f32,
    pub rain: f32,
    pub storm: bool,
    pub flash: f32,
    pub last_flash: f64,
    pub rainbow: f32,
    pub season: Season,
    pub snow: f32,
    pub snow_cover: f32,
    pub moon_phase: f32,
    pub fireworks: bool,
    pub time: f64,
    pub bird: Option<BirdPicture>,
    pub cat: Option<crate::world::cat::CatPicture>,
    pub sill: Option<Keepsake>,
    /// A shooting star crossing the sky.
    pub meteor: Option<Streak>,
    pub constellations: Vec<StarSegment>,
    pub constellation_base: f32,
    pub constellation_glow: f32,
    pub wind: f32,
    pub window_lift: f32,
    pub wind_travel: f64,
    pub passersby: [Option<Passing>; 2],
}

/// One line of a constellation, from star to star along the dome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarSegment {
    start: Vec3,
    end: Vec3,
    middle: Vec3,
    /// The cosine of how far from the middle it reaches.
    reach: f32,
}

impl StarSegment {
    pub fn from_constellations(constellations: &[Constellation]) -> Vec<Self> {
        constellations
            .iter()
            .flat_map(|constellation| constellation.stars.windows(2))
            .filter_map(|pair| {
                let start = stars::field().get(usize::from(pair[0]))?.direction;
                let end = stars::field().get(usize::from(pair[1]))?.direction;
                let middle = (start + end).normalize_or_zero();
                Some(Self {
                    start,
                    end,
                    middle,
                    reach: (start.dot(middle).clamp(-1.0, 1.0).acos() + 0.01).cos(),
                })
            })
            .collect()
    }

    /// Radians from `direction` to the line.
    fn distance(&self, direction: Vec3) -> f32 {
        let along = self.end - self.start;
        let fraction = ((direction - self.start).dot(along) / along.length_squared().max(1e-12))
            .clamp(0.0, 1.0);
        direction.distance(self.start.lerp(self.end, fraction).normalize_or_zero())
    }
}

impl Outside {
    #[cfg(test)]
    pub fn clear(hour: f32, time: f64) -> Self {
        Self {
            hour,
            rain: 0.0,
            storm: false,
            flash: 0.0,
            last_flash: f64::NEG_INFINITY,
            rainbow: 0.0,
            season: Season::Summer,
            snow: 0.0,
            snow_cover: 0.0,
            moon_phase: 0.5,
            fireworks: false,
            time,
            bird: None,
            cat: None,
            sill: None,
            meteor: None,
            constellations: Vec::new(),
            constellation_base: 0.018,
            constellation_glow: 0.0,
            wind: 0.0,
            window_lift: 0.0,
            wind_travel: 0.0,
            passersby: [None; 2],
        }
    }

    /// The sun's (azimuth, elevation): up on the left, over the back of the
    /// house, down on the right; high and wide in summer, low in winter.
    pub fn sun(&self) -> Vec2 {
        let (width, height) = match self.season {
            Season::Summer => (1.5, 1.12),
            Season::Spring | Season::Autumn => (1.3, 0.85),
            Season::Winter => (1.05, 0.56),
        };
        let progress = (self.hour.rem_euclid(24.0) - 6.5) / 13.0;
        Vec2::new(
            (progress - 0.5) * 2.0 * width,
            height * (progress * PI).sin(),
        )
    }

    /// The moon's (azimuth, elevation), crossing the sky through the night.
    pub fn moon(&self) -> Vec2 {
        let progress = (self.hour - 18.5).rem_euclid(24.0) / 12.0;
        Vec2::new((progress - 0.5) * 2.4, 0.04 + 0.48 * (progress * PI).sin())
    }

    /// The way the sunlight comes from, kept a little over the horizon so
    /// shadows stay on the lawn.
    pub fn sunlight_direction(&self) -> Vec3 {
        let sun = self.sun();
        direction(sun.x, sun.y.max(0.1))
    }

    pub fn moonlight_direction(&self) -> Vec3 {
        let moon = self.moon();
        direction(moon.x, moon.y.max(0.15))
    }

    /// How strong and what color the sun's direct light is.
    pub fn sunlight(&self) -> Vec3 {
        let sky = Sky::at_hour(self.hour);
        let cloud = self.rain.max(self.snow * 0.85).clamp(0.0, 1.0);
        let warm = Vec3::new(1.0, 0.93, 0.8).lerp(Vec3::new(1.0, 0.62, 0.38), sky.twilight);
        warm * sky.day * 0.85 * (1.0 - cloud * 0.85)
    }

    /// The moon's soft light at night.
    pub fn moonlight(&self) -> Vec3 {
        let sky = Sky::at_hour(self.hour);
        let fullness = (1.0 - (self.moon_phase * TAU).cos()) * 0.5;
        let cloud = self.rain.max(self.snow * 0.85).clamp(0.0, 1.0);
        Vec3::new(0.32, 0.38, 0.55) * fullness * (1.0 - sky.day) * (1.0 - cloud * 0.7) * 0.5
    }

    /// The sky's color right at the horizon, for haze over the far ground.
    pub fn haze(&self) -> Vec3 {
        let sky = Sky::at_hour(self.hour);
        let cloud = self.rain.max(self.snow * 0.85);
        weather_color(sky.horizon, cloud) * (1.0 - self.storm_dimming()) * EXPOSURE
    }

    /// The middle of a rainbow: opposite the sun, swung round to the back
    /// of the house where it can be seen, and never so low it hides.
    fn bow_center(&self) -> Vec3 {
        let sun = self.sun();
        let opposite = (sun.x + PI + PI).rem_euclid(TAU) - PI;
        let azimuth = if opposite.abs() > PI * 0.5 {
            opposite.signum() * (PI - opposite.abs())
        } else {
            opposite
        };
        direction(azimuth * 0.5, -sun.y.clamp(0.3, 0.55))
    }

    fn storm_dimming(&self) -> f32 {
        if self.storm {
            self.rain.clamp(0.0, 1.0) * 0.35
        } else {
            0.0
        }
    }
}

/// A unit direction from its azimuth, from straight back toward -z and
/// positive toward +x, and its elevation.
pub fn direction(azimuth: f32, elevation: f32) -> Vec3 {
    Vec3::new(
        azimuth.sin() * elevation.cos(),
        elevation.sin(),
        -azimuth.cos() * elevation.cos(),
    )
}

/// Where `direction` is from around `center`, in units, flattened onto the
/// sky there: x to the right, y up. None from the far side.
pub fn local(direction: Vec3, center: Vec3) -> Option<Vec2> {
    let facing = direction.dot(center);
    if facing <= 1e-3 {
        return None;
    }
    let right = center.cross(Vec3::Y).normalize_or(Vec3::X);
    let up = right.cross(center);
    Some(Vec2::new(direction.dot(right), direction.dot(up)) / (facing * UNIT))
}

/// The sky's broad light, kept separate from the sun, stars, and rain so
/// tiny animated details stay steady. Fireworks briefly tint this light.
pub fn light_through(outside: &Outside) -> Vec3 {
    let sky = Sky::at_hour(outside.hour);
    let noon = (1.0 - (outside.hour.rem_euclid(24.0) - 12.0).abs() / 6.0).max(0.0);
    let strength = sky.day * (0.65 + 0.25 * noon) + sky.twilight * 0.15;
    let cloud = outside.rain.max(outside.snow * 0.65);
    weather_color((sky.horizon + sky.zenith) * 0.5, cloud)
        * strength
        * (1.0 - outside.storm_dimming())
        + Vec3::new(0.055, 0.075, 0.1) * outside.snow_cover.clamp(0.0, 1.0) * sky.day
        + Vec3::new(1.4, 1.7, 2.2) * outside.flash.clamp(0.0, 1.0)
        + firework(outside).map_or(Vec3::ZERO, |burst| burst.color * burst.brightness() * 0.5)
}

pub struct Sky {
    pub horizon: Vec3,
    pub zenith: Vec3,
    pub day: f32,
    pub twilight: f32,
}

impl Sky {
    pub fn at_hour(hour: f32) -> Self {
        let hour = hour.rem_euclid(24.0);
        let day = smoothstep(5.5, 8.0, hour) * (1.0 - smoothstep(18.0, 20.5, hour));
        let twilight = ((1.0 - (hour - 6.5).abs() / 1.5).max(0.0)
            + (1.0 - (hour - 19.5).abs() / 1.5).max(0.0))
        .min(1.0);
        Self {
            horizon: Vec3::new(0.025, 0.04, 0.09)
                .lerp(Vec3::new(0.68, 0.85, 1.0), day)
                .lerp(Vec3::new(1.0, 0.44, 0.32), twilight),
            zenith: Vec3::new(0.008, 0.015, 0.05)
                .lerp(Vec3::new(0.23, 0.5, 0.9), day)
                .lerp(Vec3::new(0.28, 0.18, 0.46), twilight),
            day,
            twilight,
        }
    }
}

/// The color of the sky toward `direction`, a pixel spanning `spread`
/// radians: the far hills, then the sky and everything in it.
#[cfg(test)]
pub fn sky(direction: Vec3, spread: f32, outside: &Outside) -> Vec3 {
    Heavens::new(outside).sky(direction, spread)
}

/// The sky for one frame, with what every pixel shares worked out once.
pub struct Heavens<'a> {
    outside: &'a Outside,
    sky: Sky,
    rain: f32,
    storm: f32,
    flash: f32,
    cloud: f32,
    night: f32,
    lunar_night: f32,
    sun: Vec3,
    moon: Vec3,
    bow: Vec3,
    /// Where tonight's lightning strikes, if it does.
    bolt: Vec3,
    /// Each passing thing's direction, and the cosine of its reach.
    passing: [Option<(Vec3, f32)>; 2],
    cloud_color: Vec3,
    drift: f32,
    twinkle: f32,
    firework: Option<Firework>,
    /// The far hills' colors, far to near, lit and hazed.
    hills: [Vec3; 3],
    dimming: f32,
}

impl<'a> Heavens<'a> {
    pub fn new(outside: &'a Outside) -> Self {
        let sky = Sky::at_hour(outside.hour);
        let rain = outside.rain.clamp(0.0, 1.0);
        let snow = outside.snow.clamp(0.0, 1.0);
        let cloud = rain.max(snow * 0.85);
        let flash = outside.flash.clamp(0.0, 1.0);
        let light = land_light(&sky, flash);
        let haze = outside.haze();
        let land = land_colors(outside.season);
        let hills = std::array::from_fn(|index| {
            let white = Vec3::new(0.78, 0.86, 0.96) + Vec3::splat(index as f32 * 0.015);
            let land = land[index].lerp(white, outside.snow_cover.clamp(0.0, 1.0));
            weather_color(land * light, cloud).lerp(haze, 0.45 - index as f32 * 0.12)
        });
        let sun = outside.sun();
        let moon = outside.moon();
        let passing = outside.passersby.map(|passing| {
            let passing = passing?;
            let reach = passerby_bounds(passing.kind)?.length() * UNIT + 0.02;
            Some((toward(passing.position), reach.min(1.4).cos()))
        });
        Self {
            rain,
            storm: if outside.storm { rain } else { 0.0 },
            flash,
            cloud,
            night: (1.0 - sky.day) * (1.0 - sky.twilight * 0.85) * (1.0 - cloud).powi(2),
            lunar_night: (1.0 - sky.day) * (1.0 - cloud * 0.8),
            sun: direction(sun.x, sun.y),
            moon: direction(moon.x, moon.y),
            bow: outside.bow_center(),
            bolt: direction((unit_from(outside.last_flash, 0) - 0.5) * 1.4, 0.17),
            passing,
            cloud_color: Vec3::new(0.055, 0.065, 0.1)
                .lerp(Vec3::new(0.94, 0.96, 1.0), sky.day)
                .lerp(Vec3::new(0.86, 0.57, 0.63), sky.twilight)
                * (EXPOSURE + 0.33),
            drift: phase(outside.time, 0.25, 4096.0)
                + (outside.wind_travel * 0.6).rem_euclid(4096.0) as f32,
            twinkle: phase(outside.time, 0.7, TAU),
            firework: firework(outside),
            hills,
            dimming: outside.storm_dimming(),
            sky,
            outside,
        }
    }

    /// The color of the sky toward `direction`, a pixel spanning `spread`
    /// radians: the far hills, then the sky and everything in it.
    pub fn sky(&self, direction: Vec3, spread: f32) -> Vec3 {
        let outside = self.outside;
        let sky = &self.sky;
        let footprint = (spread / UNIT).max(1e-5);
        let (rain, storm, flash, cloud, night) =
            (self.rain, self.storm, self.flash, self.cloud, self.night);
        let elevation = direction.y.clamp(-1.0, 1.0).asin();
        let mut color = sky
            .horizon
            .lerp(sky.zenith, smoothstep(-0.05, 0.75, elevation))
            * EXPOSURE;

        if night > 0.0 && elevation > 0.0 {
            let (light, seed) = star_light(direction, spread);
            let twinkle = 0.65 + 0.35 * (self.twinkle + seed * TAU).sin();
            color += Vec3::new(0.78, 0.86, 1.0) * light * twinkle * night;
        }

        if let Some(streak) = &outside.meteor {
            color += Vec3::new(0.86, 0.92, 1.0) * shooting_star(direction, spread, streak) * night;
        }

        let moon_near = direction.dot(self.moon) > 0.92;
        let moon_point = if moon_near {
            local(direction, self.moon).unwrap_or(Vec2::splat(9.0))
        } else {
            Vec2::splat(9.0)
        };
        let moon_distance = moon_point.length();
        if night > 0.0 && moon_distance > 0.06 && !outside.constellations.is_empty() {
            let mut lines: f32 = 0.0;
            for segment in &outside.constellations {
                if direction.dot(segment.middle) >= segment.reach {
                    lines = lines.max(coverage(segment.distance(direction) - 0.0003, spread * 0.5));
                }
            }
            color += Vec3::new(0.7, 0.8, 1.0)
                * lines
                * (outside.constellation_base + outside.constellation_glow)
                * night;
        }
        if self.lunar_night > 0.0 && moon_distance < 0.4 {
            let angle = outside.moon_phase * TAU;
            let across = if outside.moon_phase < 0.5 {
                moon_point.x
            } else {
                -moon_point.x
            };
            let terminator =
                angle.cos() * (0.05_f32.powi(2) - moon_point.y.powi(2)).max(0.0).sqrt();
            let lit = coverage(terminator - across, footprint * 0.5);
            let fullness = (1.0 - angle.cos()) * 0.5;
            let moon = coverage(moon_distance - 0.05, footprint * 0.5)
                * lit
                * smoothstep(0.0, 0.015, fullness)
                + (-moon_distance.powi(2) / 0.012).exp() * 0.12 * fullness;
            color += Vec3::new(0.78, 0.85, 1.0) * moon * self.lunar_night;
        }

        if sky.day > 0.0 && direction.dot(self.sun) > 0.97 {
            let sun_distance = direction.distance(self.sun) / UNIT;
            let sunlight = coverage(sun_distance - 0.04, footprint * 0.5)
                + (-sun_distance.powi(2) / 0.02).exp() * 0.4;
            color += Vec3::new(1.0, 0.77, 0.38) * sunlight * sky.day * (1.0 - cloud * 0.9);
        }

        // A deck of clouds overhead, drifting with the wind.
        if elevation > 0.02 {
            let rise = direction.y.max(0.02);
            let deck = Vec2::new(direction.x, direction.z) / rise * CLOUD_DECK;
            let deck = deck * 0.055 - Vec2::new(self.drift * 0.055, self.drift * 0.02);
            // Toward the horizon a pixel covers more of the deck; blur there.
            let blur = smoothstep(0.15, 0.8, spread * CLOUD_DECK * 0.055 / (rise * rise));
            let clouds = if blur >= 1.0 {
                0.5
            } else {
                mix(
                    noise(deck) * 0.65 + noise(deck * 2.1 + Vec2::splat(7.3)) * 0.35,
                    0.5,
                    blur,
                )
            };
            let clouds = smoothstep(
                0.5 - storm * 0.15 - cloud * 0.1,
                0.74 - storm * 0.15,
                clouds,
            ) * smoothstep(0.02, 0.12, elevation);
            color = color.lerp(self.cloud_color, clouds * 0.6);
        }
        color = weather_color(color, cloud) * (1.0 - self.dimming);
        if flash > 0.0 {
            color = color.lerp(Vec3::new(0.75, 0.86, 1.0), flash * 0.85);
        }

        if let Some(burst) = &self.firework
            && direction.dot(burst.center) > 0.97
            && let Some(point) = local(direction, burst.center)
        {
            color += burst.sparks(point, footprint);
        }

        if outside.rainbow > 0.0 && elevation > -0.02 {
            let radius = direction.dot(self.bow).clamp(-1.0, 1.0).acos();
            if (BOW - 0.06..SECOND_BOW + 0.08).contains(&radius) {
                let primary = spectrum((radius - (BOW - 0.032)) / 0.064)
                    * coverage((radius - BOW).abs() - 0.03, spread * 0.5)
                    * 0.38;
                let secondary = spectrum(1.0 - (radius - (SECOND_BOW - 0.045)) / 0.09)
                    * coverage((radius - SECOND_BOW).abs() - 0.045, spread * 0.5)
                    * 0.09;
                color += (primary + secondary) * outside.rainbow.min(1.0) * (1.0 - rain * 0.65);
            }
        }

        if flash > 0.0
            && direction.dot(self.bolt) > 0.9
            && let Some(point) = local(direction, self.bolt)
        {
            color += Vec3::new(0.85, 0.92, 1.0)
                * lightning(point, footprint, outside.last_flash)
                * flash;
        }
        for (passing, near) in outside.passersby.iter().zip(&self.passing) {
            if let (Some(passing), Some((center, reach))) = (passing, near)
                && direction.dot(*center) >= *reach
                && let Some(point) = local(direction, *center)
            {
                color = passerby(color, point, footprint, passing, sky, cloud);
            }
        }
        self.over_hills(color, direction, spread, elevation)
    }

    /// The far hills, ridge over ridge around the whole horizon.
    fn over_hills(&self, mut color: Vec3, direction: Vec3, spread: f32, elevation: f32) -> Vec3 {
        if elevation > 0.09 {
            return color;
        }
        let azimuth = direction.x.atan2(-direction.z);
        let ridges = [
            0.052 + 0.014 * (3.0 * azimuth + 0.6).sin() + 0.006 * (8.0 * azimuth).sin(),
            0.036 + 0.013 * (4.0 * azimuth + 1.8).sin() + 0.006 * (9.0 * azimuth).sin(),
            0.021 + 0.01 * (5.0 * azimuth + 0.2).sin() + 0.004 * (13.0 * azimuth + 1.0).sin(),
        ];
        for (ridge, land) in ridges.into_iter().zip(self.hills) {
            color = color.lerp(land, coverage(elevation - ridge, spread * 0.5));
        }
        color
    }
}

/// How lit the land is by day, twilight, and lightning.
pub fn land_light(sky: &Sky, flash: f32) -> f32 {
    0.04 + sky.day * 0.85 + sky.twilight * 0.18 + flash * 0.6
}

/// The hills' greens and golds for the season, far to near.
pub fn land_colors(season: Season) -> [Vec3; 3] {
    match season {
        Season::Spring => [
            Vec3::new(0.36, 0.58, 0.4),
            Vec3::new(0.4, 0.64, 0.32),
            Vec3::new(0.29, 0.51, 0.26),
        ],
        Season::Summer => [
            Vec3::new(0.29, 0.48, 0.46),
            Vec3::new(0.27, 0.51, 0.34),
            Vec3::new(0.21, 0.42, 0.28),
        ],
        Season::Autumn => [
            Vec3::new(0.62, 0.51, 0.31),
            Vec3::new(0.76, 0.57, 0.25),
            Vec3::new(0.62, 0.44, 0.22),
        ],
        Season::Winter => [
            Vec3::new(0.42, 0.5, 0.47),
            Vec3::new(0.44, 0.51, 0.38),
            Vec3::new(0.35, 0.43, 0.32),
        ],
    }
}

/// How much a star covers a pixel toward `direction`, and its seed.
fn star_light(direction: Vec3, spread: f32) -> (f32, f32) {
    let (column, row) = stars::cell(direction);
    let Some((_, star, seed)) = stars::in_cell(column, row) else {
        return (0.0, 0.0);
    };
    let distance = direction.distance(star);
    (
        coverage(distance - 0.001, spread * 0.5) * stars::brightness(seed),
        seed,
    )
}

/// The direction of a sky position, (azimuth, elevation).
pub fn toward(position: Vec2) -> Vec3 {
    direction(position.x, position.y)
}

/// How far a passing thing's drawing reaches from its middle, in units.
fn passerby_bounds(kind: Passerby) -> Option<Vec2> {
    match kind {
        Passerby::Geese => Some(Vec2::new(0.19, 0.10)),
        Passerby::Balloon => Some(Vec2::new(0.055, 0.09)),
        Passerby::Kite => Some(Vec2::new(0.17, 0.36)),
        Passerby::Plane => Some(Vec2::new(0.045, 0.03)),
        Passerby::Lanterns => Some(Vec2::new(0.11, 0.15)),
        Passerby::Squirrel | Passerby::Snowman => None,
    }
}

fn passerby(
    mut color: Vec3,
    mut point: Vec2,
    footprint: f32,
    passing: &Passing,
    sky: &Sky,
    cloud: f32,
) -> Vec3 {
    let Some(bounds) = passerby_bounds(passing.kind) else {
        return color;
    };
    if point.abs().cmpgt(bounds + Vec2::splat(footprint)).any() {
        return color;
    }
    let background = color;
    let edge = footprint * 0.6 + 0.0007;
    let light = 0.08 + sky.day * 0.85 + sky.twilight * 0.25;
    let lit = |tint: Vec3| weather_color(tint * light + sky.horizon * 0.08, cloud);
    let paint = |color: Vec3, tint, distance| color.lerp(tint, coverage(distance, edge));
    match passing.kind {
        Passerby::Geese => {
            let mut distance = f32::INFINITY;
            for index in -3_i32..=3 {
                let rank = index.abs() as f32;
                let center = Vec2::new(-passing.facing * rank * 0.043, index as f32 * 0.025);
                let point = point - center;
                let flap = (passing.motion * 2.0 + rank).sin() * 0.012;
                distance = distance
                    .min(capsule(point, Vec2::new(-0.012, flap), Vec2::ZERO, 0.0015))
                    .min(capsule(point, Vec2::ZERO, Vec2::new(0.012, flap), 0.0015));
            }
            color = paint(color, lit(Vec3::new(0.16, 0.18, 0.21)), distance);
        }
        Passerby::Balloon => {
            let envelope = (point / Vec2::new(0.04, 0.055)).length() - 1.0;
            let stripe = if passing.striped {
                (point.x * 210.0).cos() * 0.5 + 0.5
            } else {
                0.0
            };
            let warm = Vec3::new(1.0, 0.42, 0.24).lerp(Vec3::new(1.0, 0.83, 0.45), stripe);
            color = paint(color, lit(warm), envelope * 0.04);
            let basket = (point - Vec2::new(0.0, -0.075)).abs() - Vec2::new(0.012, 0.008);
            let mut ropes = f32::INFINITY;
            for side in [-1.0, 1.0] {
                ropes = ropes.min(capsule(
                    point,
                    Vec2::new(side * 0.018, -0.043),
                    Vec2::new(side * 0.01, -0.07),
                    0.001,
                ));
            }
            color = paint(
                color,
                lit(Vec3::new(0.65, 0.44, 0.27)),
                basket.max_element().min(ropes),
            );
        }
        Passerby::Kite => {
            let string = capsule(
                point,
                Vec2::new(0.0, -0.025),
                Vec2::new(-0.13, -0.33),
                0.0008,
            );
            color = paint(color, lit(Vec3::new(0.84, 0.82, 0.69)), string);
            point.x += point.y * (passing.motion.sin() * 0.2);
            let diamond = (point.x.abs() / 0.027 + point.y.abs() / 0.044 - 1.0) * 0.023;
            let tint = if point.x < 0.0 {
                Vec3::new(1.0, 0.5, 0.33)
            } else {
                Vec3::new(0.47, 0.75, 1.0)
            };
            color = paint(color, lit(tint), diamond);
            for index in 0..3 {
                let center = Vec2::new(
                    0.006 * (passing.motion + index as f32).sin(),
                    -0.06 - index as f32 * 0.017,
                );
                let bow = capsule(
                    point,
                    center - Vec2::new(0.006, 0.003),
                    center + Vec2::new(0.006, 0.003),
                    0.0015,
                );
                color = paint(color, lit(Vec3::new(1.0, 0.73, 0.42)), bow);
            }
        }
        Passerby::Plane => {
            for (offset, tint, on) in [
                (
                    -0.012,
                    Vec3::new(1.0, 0.2, 0.18),
                    passing.motion.sin() > 0.15,
                ),
                (
                    0.012,
                    Vec3::new(0.9, 0.95, 1.0),
                    passing.motion.cos() > 0.55,
                ),
            ] {
                if on {
                    let distance = point.distance(Vec2::X * offset);
                    color += tint
                        * (coverage(distance - 0.0018, edge) * 0.8
                            + (-distance.powi(2) / 0.000035).exp() * 0.15);
                }
            }
        }
        Passerby::Lanterns => {
            for offset in [Vec2::ZERO, Vec2::new(-0.05, -0.065), Vec2::new(0.04, -0.1)] {
                let point = point - offset;
                let distance = (point / Vec2::new(0.012, 0.019)).length() - 1.0;
                let glow = (-point.length_squared() / 0.00045).exp();
                color += Vec3::new(1.0, 0.38, 0.12) * glow * 0.1;
                color = paint(color, Vec3::new(1.0, 0.64, 0.27), distance * 0.012);
                color += Vec3::new(1.0, 0.84, 0.46) * coverage(point.length() - 0.003, edge) * 0.3;
            }
        }
        Passerby::Squirrel | Passerby::Snowman => unreachable!(),
    }
    let presence =
        smoothstep(0.0, 0.08, passing.phase) * (1.0 - smoothstep(0.92, 1.0, passing.phase));
    background.lerp(color, presence)
}

// How brightly a shooting star's streak covers a point of the sky: a thin
// line, brightest at its head and fading along its tail.
fn shooting_star(direction: Vec3, spread: f32, streak: &Streak) -> f32 {
    let head = toward(streak.head);
    let tail = toward(streak.tail);
    if streak.brightness <= 0.0 || direction.dot((head + tail) * 0.5) < 0.9 {
        return 0.0;
    }
    let along = head - tail;
    let fraction = if along.length_squared() > 1e-10 {
        ((direction - tail).dot(along) / along.length_squared()).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let nearest = tail.lerp(head, fraction).normalize_or_zero();
    let distance = direction.distance(nearest) / UNIT;
    let line = coverage(distance - 0.002 - 0.003 * fraction, spread / UNIT * 0.5);
    let to_head = direction.distance(head) / UNIT;
    let head = (-to_head.powi(2) / 0.0004).exp();
    (line * fraction.powf(1.5) + head * 0.8) * streak.brightness
}

/// A value from 0 to 1 for a lightning flash and a salt, steady through
/// the flash.
fn unit_from(last_flash: f64, salt: u64) -> f32 {
    let mut bits = last_flash.to_bits() ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    bits = (bits ^ (bits >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    bits = (bits ^ (bits >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((bits ^ (bits >> 31)) >> 40) as f32 / (1u64 << 24) as f32
}

/// The bolt around its middle, in units, from up in the clouds down to the
/// hills.
fn lightning(point: Vec2, footprint: f32, last_flash: f64) -> f32 {
    let points: [Vec2; 6] = std::array::from_fn(|index| {
        let side = if index % 2 == 0 { -1.0 } else { 1.0 };
        Vec2::new(
            side * (0.04 + unit_from(last_flash, index as u64 + 1) * 0.1),
            0.4 - index as f32 * 0.17,
        )
    });
    let mut distance = f32::INFINITY;
    for segment in points.windows(2) {
        distance = distance.min(capsule(point, segment[0], segment[1], 0.0025));
    }
    let side = if unit_from(last_flash, 7) < 0.5 {
        -1.0
    } else {
        1.0
    };
    let branch = points[2] + Vec2::new(side * 0.15, -0.06);
    let end = branch + Vec2::new(side * 0.075, -0.1);
    distance = distance
        .min(capsule(point, points[2], branch, 0.0015))
        .min(capsule(point, branch, end, 0.001));
    coverage(distance, footprint * 0.5) + 0.2 * coverage(distance - 0.008, footprint * 0.5)
}

struct Firework {
    center: Vec3,
    color: Vec3,
    age: f32,
    seed: f32,
}

impl Firework {
    fn brightness(&self) -> f32 {
        smoothstep(0.0, 0.12, self.age) * (1.0 - self.age / 1.8).max(0.0).powi(2)
    }

    /// The sparks at `point`, in units from the burst's middle.
    fn sparks(&self, point: Vec2, footprint: f32) -> Vec3 {
        if point.length_squared() > 0.36 {
            return Vec3::ZERO;
        }
        let mut light = 0.0;
        for index in 0..14 {
            let seed = Vec2::new(self.seed, index as f32);
            let angle = index as f32 * TAU / 14.0 + hash(seed) * 0.18;
            let velocity =
                Vec2::new(angle.cos(), angle.sin()) * (0.14 + 0.11 * hash(seed + Vec2::Y));
            let position = |age: f32| velocity * age - Vec2::Y * 0.05 * age * age;
            let start = position((self.age - 0.08).max(0.0));
            let end = position(self.age);
            let distance = if start.distance_squared(end) > 1e-12 {
                capsule(point, start, end, 0.0025)
            } else {
                point.distance(end) - 0.0025
            };
            light += coverage(distance, footprint * 0.5);
        }
        self.color * light * self.brightness() * 1.8
    }
}

// Each time slot holds a burst with its own position, color, and sparks.
// The sky and the yard's light share the same burst and fading.
fn firework(outside: &Outside) -> Option<Firework> {
    if !outside.fireworks || (6.5..19.5).contains(&outside.hour) {
        return None;
    }
    let time = phase(outside.time, 1.0, 2048.0);
    let slot = (time / 3.2).floor();
    let age = time - slot * 3.2 - hash(Vec2::new(slot, 5.0)) * 0.8;
    if !(0.0..1.8).contains(&age) {
        return None;
    }
    let colors = [
        Vec3::new(1.0, 0.5, 0.4),
        Vec3::new(0.45, 0.8, 1.0),
        Vec3::new(1.0, 0.85, 0.35),
        Vec3::new(0.8, 0.55, 1.0),
    ];
    Some(Firework {
        center: direction(
            (hash(Vec2::new(slot, 1.0)) - 0.5) * 1.3,
            0.22 + 0.12 * hash(Vec2::new(slot, 2.0)),
        ),
        color: colors[(hash(Vec2::new(slot, 3.0)) * colors.len() as f32) as usize % colors.len()],
        age,
        seed: slot,
    })
}

fn hash(cell: Vec2) -> f32 {
    scramble(cell * 7.0 + Vec2::new(311.0, 127.0))
}

pub fn weather_color(color: Vec3, rain: f32) -> Vec3 {
    let rain = rain.clamp(0.0, 1.0);
    let brightness = color.dot(Vec3::new(0.2126, 0.7152, 0.0722));
    color.lerp(Vec3::splat(brightness), rain * 0.9) * (1.0 - rain * 0.45)
}

fn spectrum(amount: f32) -> Vec3 {
    let colors = [
        Vec3::new(0.57, 0.26, 1.0),
        Vec3::new(0.25, 0.48, 1.0),
        Vec3::new(0.27, 0.9, 0.45),
        Vec3::new(1.0, 0.9, 0.28),
        Vec3::new(1.0, 0.56, 0.2),
        Vec3::new(1.0, 0.22, 0.22),
    ];
    let position = amount.clamp(0.0, 1.0) * (colors.len() - 1) as f32;
    let index = (position as usize).min(colors.len() - 2);
    colors[index].lerp(colors[index + 1], position - index as f32)
}

// Wrap in f64 before converting to keep animation moving on old saves and
// avoid losing precision or overflowing for long-running clocks.
pub fn phase(time: f64, speed: f64, period: f32) -> f32 {
    (time * speed).rem_euclid(f64::from(period)) as f32
}

pub fn noise(point: Vec2) -> f32 {
    let cell = point.floor();
    let fraction = point - cell;
    let weight = fraction * fraction * (Vec2::splat(3.0) - fraction * 2.0);
    mix(
        mix(scramble(cell), scramble(cell + Vec2::X), weight.x),
        mix(
            scramble(cell + Vec2::Y),
            scramble(cell + Vec2::ONE),
            weight.x,
        ),
        weight.y,
    )
}

/// A quick value from 0 to 1 for a lattice cell, for textures.
pub fn scramble(cell: Vec2) -> f32 {
    let mut bits = (cell.x as i32 as u32).wrapping_mul(0x8DA6_B343)
        ^ (cell.y as i32 as u32).wrapping_mul(0xD816_3841);
    bits ^= bits >> 13;
    bits = bits.wrapping_mul(0x5BD1_E995);
    bits ^= bits >> 15;
    (bits >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::meteor::Meteor;

    fn every_direction() -> impl Iterator<Item = Vec3> {
        (0..48).flat_map(|turn| {
            (0..12).map(move |rise| {
                direction(
                    turn as f32 / 48.0 * TAU - PI,
                    -0.2 + rise as f32 / 12.0 * 1.2,
                )
            })
        })
    }

    #[test]
    fn a_direction_flattens_around_itself_to_the_middle_with_right_and_up_kept() {
        let center = direction(0.4, 0.3);
        assert!(local(center, center).unwrap().length() < 1e-5);
        assert!(local(direction(0.45, 0.3), center).unwrap().x > 0.0);
        assert!(local(direction(0.4, 0.35), center).unwrap().y > 0.0);
        assert!(local(-center, center).is_none());
    }

    #[test]
    fn sky_samples_stay_finite_at_every_hour_in_every_weather() {
        for hour in [0.0, 5.9, 6.5, 12.0, 19.5, 23.9] {
            for (rain, snow, storm, flash) in [
                (0.0, 0.0, false, 0.0),
                (1.0, 0.0, true, 1.0),
                (0.0, 1.0, false, 0.0),
            ] {
                let outside = Outside {
                    rain,
                    snow,
                    storm,
                    flash,
                    last_flash: 3.0,
                    rainbow: 1.0,
                    snow_cover: snow,
                    fireworks: true,
                    meteor: Some(Meteor::at(1, 0.0, None).map_or(
                        Streak {
                            head: Vec2::new(0.1, 0.3),
                            tail: Vec2::new(0.1, 0.3),
                            brightness: 1.0,
                        },
                        |meteor| meteor.streak(0.3),
                    )),
                    wind_travel: 1e12,
                    ..Outside::clear(hour, 1e9)
                };
                for direction in every_direction() {
                    for spread in [1e-6, 0.004, 0.05] {
                        assert!(sky(direction, spread, &outside).is_finite());
                    }
                }
                assert!(light_through(&outside).is_finite());
                assert!(outside.sunlight().is_finite() && outside.moonlight().is_finite());
            }
        }
    }

    #[test]
    fn noon_is_brighter_than_midnight_and_rain_makes_the_sky_greyer() {
        let up = direction(0.0, 0.3);
        let noon = sky(up, 0.004, &Outside::clear(12.0, 0.0));
        let midnight = sky(up, 0.004, &Outside::clear(0.0, 0.0));
        assert!(noon.max_element() > midnight.max_element() * 3.0);
        let rainy = sky(
            up,
            0.004,
            &Outside {
                rain: 1.0,
                ..Outside::clear(12.0, 0.0)
            },
        );
        let spread = |color: Vec3| color.max_element() - color.min_element();
        assert!(spread(rainy) < spread(noon));
    }

    #[test]
    fn the_sun_rises_on_the_left_peaks_at_noon_and_sets_on_the_right() {
        let at = |hour| Outside::clear(hour, 0.0).sun();
        assert!(at(7.0).x < at(12.0).x && at(12.0).x < at(18.0).x);
        assert!(at(13.0).y > at(8.0).y && at(13.0).y > at(18.5).y);
        assert!(at(6.0).y < 0.0 && at(20.0).y < 0.0);
        let light = Outside::clear(13.0, 0.0).sunlight_direction();
        assert!(light.y > 0.5 && light.z < 0.0);
        let winter = Outside {
            season: Season::Winter,
            ..Outside::clear(13.0, 0.0)
        };
        assert!(winter.sun().y < at(13.0).y);
    }

    #[test]
    fn a_rainbow_stands_opposite_the_sun_across_the_back_sky() {
        for (hour, side) in [(8.0, 1.0), (17.0, -1.0)] {
            let outside = Outside::clear(hour, 0.0);
            let center = outside.bow_center();
            assert!(center.z < 0.0 && center.y < 0.0);
            assert_eq!(center.x.signum(), side);
            // Its top clears the hills.
            assert!(center.y.asin() + BOW > 0.15);
        }
    }

    #[test]
    fn far_hills_rise_all_the_way_around_the_horizon() {
        let outside = Outside::clear(12.0, 0.0);
        let heavens = Heavens::new(&outside);
        for turn in 0..64 {
            let azimuth = turn as f32 / 64.0 * TAU;
            let low = direction(azimuth, 0.005);
            let high = direction(azimuth, 0.2);
            let land = heavens.over_hills(Vec3::ONE * 9.0, low, 0.001, 0.005);
            assert!(land.max_element() < 2.0, "{azimuth}");
            assert_eq!(
                heavens.over_hills(Vec3::ONE * 9.0, high, 0.001, 0.2),
                Vec3::ONE * 9.0
            );
        }
    }
}
