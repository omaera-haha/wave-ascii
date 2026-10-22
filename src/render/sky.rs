//! The open sky over the yard, seen by direction. The old window's picture
//! now lives as a patch of the back sky, the "chart": picture coordinates
//! from (0, 0) at its bottom left to (1, 1) at its top right, keeping the
//! window's 1.5:1 proportions, so stars, constellations, shooting stars,
//! fireworks, and things passing overhead keep their old places.

use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};

use crate::sdf::{capsule, coverage, mix, smoothstep};
use crate::world::meteor::Streak;
use crate::world::passerby::{Passerby, Passing};
use crate::world::stars::{self, Constellation, hash};
use crate::world::{BirdPicture, Keepsake, Season};

/// Radians of azimuth across the chart, and of elevation up it.
pub const CHART_WIDTH: f32 = 0.57;
pub const CHART_HEIGHT: f32 = CHART_WIDTH / 1.5;
/// Elevation of the chart's bottom edge.
pub const CHART_BASE: f32 = 0.02;
/// Height of the cloud deck, in room units per unit of ray rise.
const CLOUD_DECK: f32 = 14.0;
/// The sky is drawn dimmer than its light so its glyphs stay airy and the
/// house and the ghost stand out against it.
const EXPOSURE: f32 = 0.36;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarSegment {
    start: Vec2,
    end: Vec2,
    minimum: Vec2,
    maximum: Vec2,
}

impl StarSegment {
    pub fn from_constellations(constellations: &[Constellation]) -> Vec<Self> {
        constellations
            .iter()
            .flat_map(|constellation| constellation.stars.windows(2))
            .filter_map(|pair| {
                let start = stars::field().get(usize::from(pair[0]))?.position;
                let end = stars::field().get(usize::from(pair[1]))?.position;
                Some(Self {
                    start,
                    end,
                    minimum: start.min(end),
                    maximum: start.max(end),
                })
            })
            .collect()
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

    /// Where the sun's disc is on the chart.
    pub fn sun(&self) -> Vec2 {
        let progress = (self.hour.rem_euclid(24.0) - 6.5) / 13.0;
        Vec2::new(-0.9 + 2.8 * progress, 0.06 + 0.56 * (progress * PI).sin())
    }

    /// Where the moon is on the chart, crossing the sky through the night.
    pub fn moon(&self) -> Vec2 {
        let progress = (self.hour - 18.5).rem_euclid(24.0) / 12.0;
        Vec2::new(-0.9 + 2.8 * progress, 0.1 + 0.5 * (progress * PI).sin())
    }

    /// The way the sunlight comes from, steeper than the drawn disc so
    /// shadows stay short and it shines in through the back window.
    pub fn sunlight_direction(&self) -> Vec3 {
        let sun = self.sun();
        let azimuth = (sun.x - 0.5) * CHART_WIDTH * 0.8;
        let elevation = 0.14 + 0.94 * (sun.y / 0.62).clamp(0.0, 1.0);
        direction(azimuth, elevation)
    }

    pub fn moonlight_direction(&self) -> Vec3 {
        let moon = self.moon();
        direction(
            (moon.x - 0.5) * CHART_WIDTH * 0.8,
            0.15 + 0.6 * (moon.y / 0.6).clamp(0.0, 1.0),
        )
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

/// The chart point a direction looks toward.
pub fn chart(direction: Vec3) -> Vec2 {
    let azimuth = direction.x.atan2(-direction.z);
    let elevation = direction.y.clamp(-1.0, 1.0).asin();
    Vec2::new(
        0.5 + azimuth / CHART_WIDTH,
        (elevation - CHART_BASE) / CHART_HEIGHT,
    )
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
pub fn sky(direction: Vec3, spread: f32, outside: &Outside) -> Vec3 {
    let coordinates = chart(direction);
    let footprint = (spread / CHART_HEIGHT).max(1e-5);
    let sky = Sky::at_hour(outside.hour);
    let rain = outside.rain.clamp(0.0, 1.0);
    let storm = if outside.storm { rain } else { 0.0 };
    let flash = outside.flash.clamp(0.0, 1.0);
    let snow = outside.snow.clamp(0.0, 1.0);
    let cloud = rain.max(snow * 0.85);
    let elevation = direction.y.clamp(-1.0, 1.0).asin();
    let mut color = sky
        .horizon
        .lerp(sky.zenith, smoothstep(-0.05, 0.75, elevation))
        * EXPOSURE;
    let night = (1.0 - sky.day) * (1.0 - sky.twilight * 0.85) * (1.0 - cloud).powi(2);

    if night > 0.0 && elevation > 0.0 {
        let inside = coordinates.cmpge(Vec2::ZERO).all() && coordinates.cmple(Vec2::ONE).all();
        let twinkle = |seed: f32| 0.65 + 0.35 * (phase(outside.time, 0.7, TAU) + seed * TAU).sin();
        if inside {
            let stars = coordinates * stars::GRID;
            let cell = stars.floor();
            let seed = hash(cell);
            let center = stars::center(cell);
            let distance = ((stars - cell - center) / stars::GRID).length();
            color += Vec3::new(0.78, 0.86, 1.0)
                * coverage(distance - 0.0025, footprint * 0.5)
                * stars::brightness(seed)
                * twinkle(seed)
                * night;
        } else {
            let (light, seed) = far_star(direction, spread);
            color += Vec3::new(0.78, 0.86, 1.0) * light * twinkle(seed) * night;
        }
    }

    let proportions = Vec2::new(1.0, CHART_HEIGHT / CHART_WIDTH);
    if let Some(streak) = &outside.meteor {
        color += Vec3::new(0.86, 0.92, 1.0)
            * shooting_star(coordinates, footprint, streak, proportions)
            * night;
    }

    let moon_point = (coordinates - outside.moon()) * proportions;
    let moon_distance = moon_point.length();
    if night > 0.0 && moon_distance > 0.06 && !outside.constellations.is_empty() {
        let margin = Vec2::splat(0.0007 + footprint);
        let mut lines: f32 = 0.0;
        for segment in &outside.constellations {
            if coordinates.cmpge(segment.minimum - margin).all()
                && coordinates.cmple(segment.maximum + margin).all()
            {
                lines = lines.max(coverage(
                    capsule(coordinates, segment.start, segment.end, 0.0007),
                    footprint * 0.5,
                ));
            }
        }
        color += Vec3::new(0.7, 0.8, 1.0)
            * lines
            * (outside.constellation_base + outside.constellation_glow)
            * night;
    }
    let lunar_night = (1.0 - sky.day) * (1.0 - cloud * 0.8);
    if lunar_night > 0.0 && moon_distance < 0.4 {
        let angle = outside.moon_phase * TAU;
        let across = if outside.moon_phase < 0.5 {
            moon_point.x
        } else {
            -moon_point.x
        };
        let terminator = angle.cos() * (0.05_f32.powi(2) - moon_point.y.powi(2)).max(0.0).sqrt();
        let lit = coverage(terminator - across, footprint * 0.5);
        let fullness = (1.0 - angle.cos()) * 0.5;
        let moon = coverage(moon_distance - 0.05, footprint * 0.5)
            * lit
            * smoothstep(0.0, 0.015, fullness)
            + (-moon_distance.powi(2) / 0.012).exp() * 0.12 * fullness;
        color += Vec3::new(0.78, 0.85, 1.0) * moon * lunar_night;
    }

    let sun_distance = ((coordinates - outside.sun()) * proportions).length();
    let sunlight = coverage(sun_distance - 0.04, footprint * 0.5)
        + (-sun_distance.powi(2) / 0.02).exp() * 0.4;
    color += Vec3::new(1.0, 0.77, 0.38) * sunlight * sky.day * (1.0 - cloud * 0.9);

    // A deck of clouds overhead, drifting with the wind.
    if direction.y > 0.004 {
        let rise = direction.y.max(0.02);
        let deck = Vec2::new(direction.x, direction.z) / rise * CLOUD_DECK;
        let drift = phase(outside.time, 0.25, 4096.0)
            + (outside.wind_travel * 0.6).rem_euclid(4096.0) as f32;
        let deck = deck * 0.055 - Vec2::new(drift * 0.055, drift * 0.02);
        // Toward the horizon a pixel covers more of the deck; blur there.
        let blur = smoothstep(0.15, 0.8, spread * CLOUD_DECK * 0.055 / (rise * rise));
        let clouds = noise(deck) * 0.65 + noise(deck * 2.1 + Vec2::splat(7.3)) * 0.35;
        let clouds = mix(clouds, 0.5, blur);
        let clouds = smoothstep(0.5 - storm * 0.15 - cloud * 0.1, 0.74 - storm * 0.15, clouds)
            * smoothstep(0.02, 0.12, elevation);
        let cloud_color = Vec3::new(0.055, 0.065, 0.1)
            .lerp(Vec3::new(0.94, 0.96, 1.0), sky.day)
            .lerp(Vec3::new(0.86, 0.57, 0.63), sky.twilight)
            * (EXPOSURE + 0.24);
        color = color.lerp(cloud_color, clouds * 0.6);
    }
    color = weather_color(color, cloud) * (1.0 - outside.storm_dimming());
    color = color.lerp(Vec3::new(0.75, 0.86, 1.0), flash * 0.85);

    if let Some(burst) = firework(outside) {
        color += burst.sparks(coordinates, footprint);
    }

    if outside.rainbow > 0.0 {
        let radius = coordinates.distance(Vec2::new(0.5, 0.04));
        let primary = spectrum((radius - 0.72) / 0.06)
            * coverage((radius - 0.75).abs() - 0.03, footprint * 0.5)
            * 0.38;
        let secondary = spectrum(1.0 - (radius - 0.815) / 0.09)
            * coverage((radius - 0.86).abs() - 0.045, footprint * 0.5)
            * 0.09;
        color += (primary + secondary) * outside.rainbow.min(1.0) * (1.0 - rain * 0.65);
    }

    if flash > 0.0 {
        color += Vec3::new(0.85, 0.92, 1.0)
            * lightning(coordinates, footprint, outside.last_flash)
            * flash;
    }
    for passing in outside.passersby.iter().flatten() {
        if matches!(passing.kind, Passerby::Squirrel | Passerby::Snowman) {
            continue;
        }
        color = passerby(color, coordinates, footprint, passing, &sky, cloud);
    }
    hills(color, direction, spread, outside, &sky, flash)
}

/// The far hills, ridge over ridge around the whole horizon.
fn hills(
    mut color: Vec3,
    direction: Vec3,
    spread: f32,
    outside: &Outside,
    sky: &Sky,
    flash: f32,
) -> Vec3 {
    let elevation = direction.y.clamp(-1.0, 1.0).asin();
    if elevation > 0.09 {
        return color;
    }
    let azimuth = direction.x.atan2(-direction.z);
    let land = land_colors(outside.season);
    let ridges = [
        0.052 + 0.014 * (3.0 * azimuth + 0.6).sin() + 0.006 * (8.0 * azimuth).sin(),
        0.036 + 0.013 * (4.0 * azimuth + 1.8).sin() + 0.006 * (9.0 * azimuth).sin(),
        0.021 + 0.01 * (5.0 * azimuth + 0.2).sin() + 0.004 * (13.0 * azimuth + 1.0).sin(),
    ];
    let cloud = outside.rain.max(outside.snow * 0.85);
    let light = land_light(sky, flash);
    let haze = outside.haze();
    for (index, (ridge, land)) in ridges.into_iter().zip(land).enumerate() {
        let white = Vec3::new(0.78, 0.86, 0.96) + Vec3::splat(index as f32 * 0.015);
        let land = land.lerp(white, outside.snow_cover.clamp(0.0, 1.0));
        let land = weather_color(land * light, cloud).lerp(haze, 0.45 - index as f32 * 0.12);
        color = color.lerp(land, coverage(elevation - ridge, spread * 0.5));
    }
    color
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

/// A faint star somewhere on the dome away from the chart, and its seed.
fn far_star(direction: Vec3, spread: f32) -> (f32, f32) {
    const CELLS: f32 = 40.0;
    let scaled = direction * CELLS;
    let cell = scaled.floor();
    let seed = hash(Vec2::new(cell.x + cell.z * 7.0, cell.y + cell.z * 3.0));
    let offset = Vec3::new(
        hash(Vec2::new(cell.x, cell.y + 0.5)),
        hash(Vec2::new(cell.y + 0.25, cell.z)),
        hash(Vec2::new(cell.z + 0.75, cell.x)),
    ) * 0.6
        + 0.2;
    let star = (cell + offset).normalize();
    let distance = direction.distance(star);
    (
        coverage(distance - 0.0014, spread * 0.5) * stars::brightness(seed),
        seed,
    )
}

fn passerby(
    mut color: Vec3,
    coordinates: Vec2,
    footprint: f32,
    passing: &Passing,
    sky: &Sky,
    cloud: f32,
) -> Vec3 {
    let mut point = (coordinates - passing.position) * Vec2::new(1.5, 1.0);
    let bounds = match passing.kind {
        Passerby::Geese => Vec2::new(0.19, 0.10),
        Passerby::Balloon => Vec2::new(0.055, 0.09),
        Passerby::Kite => Vec2::new(0.17, 0.36),
        Passerby::Plane => Vec2::new(0.045, 0.03),
        Passerby::Lanterns => Vec2::new(0.11, 0.15),
        Passerby::Squirrel | Passerby::Snowman => return color,
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
fn shooting_star(coordinates: Vec2, footprint: f32, streak: &Streak, proportions: Vec2) -> f32 {
    let reach = 0.03 + footprint;
    let low = streak.head.min(streak.tail) - Vec2::splat(reach);
    let high = streak.head.max(streak.tail) + Vec2::splat(reach);
    if streak.brightness <= 0.0 || coordinates.cmplt(low).any() || coordinates.cmpgt(high).any() {
        return 0.0;
    }
    let point = (coordinates - streak.tail) * proportions;
    let along = (streak.head - streak.tail) * proportions;
    let fraction = if along.length_squared() > 1e-8 {
        (point.dot(along) / along.length_squared()).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let distance = point.distance(along * fraction);
    let line = coverage(distance - 0.002 - 0.003 * fraction, footprint * 0.5);
    let head = (-point.distance_squared(along) / 0.0004).exp();
    (line * fraction.powf(1.5) + head * 0.8) * streak.brightness
}

fn lightning(coordinates: Vec2, footprint: f32, last_flash: f64) -> f32 {
    let random = |salt: u64| {
        let mut bits = last_flash.to_bits() ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        bits = (bits ^ (bits >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        bits = (bits ^ (bits >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((bits ^ (bits >> 31)) >> 40) as f32 / (1u64 << 24) as f32
    };
    let across = 0.3 + random(0) * 0.4;
    let points: [Vec2; 6] = std::array::from_fn(|index| {
        let side = if index % 2 == 0 { -1.0 } else { 1.0 };
        Vec2::new(
            across + side * (0.025 + random(index as u64 + 1) * 0.065),
            0.88 - index as f32 * 0.17,
        )
    });
    let mut distance = f32::INFINITY;
    for segment in points.windows(2) {
        distance = distance.min(capsule(coordinates, segment[0], segment[1], 0.0025));
    }
    let side = if random(7) < 0.5 { -1.0 } else { 1.0 };
    let branch = points[2] + Vec2::new(side * 0.1, -0.06);
    let end = branch + Vec2::new(side * 0.05, -0.1);
    distance = distance
        .min(capsule(coordinates, points[2], branch, 0.0015))
        .min(capsule(coordinates, branch, end, 0.001));
    coverage(distance, footprint * 0.5) + 0.2 * coverage(distance - 0.008, footprint * 0.5)
}

struct Firework {
    center: Vec2,
    color: Vec3,
    age: f32,
    seed: f32,
}

impl Firework {
    fn brightness(&self) -> f32 {
        smoothstep(0.0, 0.12, self.age) * (1.0 - self.age / 1.8).max(0.0).powi(2)
    }

    fn sparks(&self, point: Vec2, footprint: f32) -> Vec3 {
        if point.distance_squared(self.center) > 0.09 {
            return Vec3::ZERO;
        }
        let mut light = 0.0;
        for index in 0..14 {
            let seed = Vec2::new(self.seed, index as f32);
            let angle = index as f32 * TAU / 14.0 + hash(seed) * 0.18;
            let velocity =
                Vec2::new(angle.cos(), angle.sin()) * (0.08 + 0.07 * hash(seed + Vec2::Y));
            let position = |age: f32| self.center + velocity * age - Vec2::Y * 0.045 * age * age;
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
        center: Vec2::new(
            0.2 + 0.6 * hash(Vec2::new(slot, 1.0)),
            0.45 + 0.3 * hash(Vec2::new(slot, 2.0)),
        ),
        color: colors[(hash(Vec2::new(slot, 3.0)) * colors.len() as f32) as usize % colors.len()],
        age,
        seed: slot,
    })
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
        mix(hash(cell), hash(cell + Vec2::X), weight.x),
        mix(hash(cell + Vec2::Y), hash(cell + Vec2::ONE), weight.x),
        weight.y,
    )
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
    fn the_chart_runs_left_to_right_and_up_the_sky() {
        let middle = chart(direction(0.0, CHART_BASE + CHART_HEIGHT * 0.5));
        assert!(middle.distance(Vec2::new(0.5, 0.5)) < 1e-4);
        assert!(chart(direction(0.2, 0.2)).x > chart(direction(-0.2, 0.2)).x);
        assert!(chart(direction(0.0, 0.3)).y > chart(direction(0.0, 0.1)).y);
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
                            head: Vec2::new(0.5, 0.6),
                            tail: Vec2::new(0.5, 0.6),
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
        let light = Outside::clear(13.0, 0.0).sunlight_direction();
        assert!(light.y > 0.5 && light.z < 0.0);
    }

    #[test]
    fn far_hills_rise_all_the_way_around_the_horizon() {
        let outside = Outside::clear(12.0, 0.0);
        let sky = Sky::at_hour(12.0);
        for turn in 0..64 {
            let azimuth = turn as f32 / 64.0 * TAU;
            let low = direction(azimuth, 0.005);
            let high = direction(azimuth, 0.2);
            let land = hills(Vec3::ONE * 9.0, low, 0.001, &outside, &sky, 0.0);
            assert!(land.max_element() < 2.0, "{azimuth}");
            assert_eq!(hills(Vec3::ONE * 9.0, high, 0.001, &outside, &sky, 0.0), Vec3::ONE * 9.0);
        }
    }
}
