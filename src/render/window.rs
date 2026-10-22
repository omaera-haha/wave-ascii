use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};

use crate::sdf::{capsule, coverage, mix, smoothstep};
use crate::world::meteor::Streak;
use crate::world::passerby::{Passerby, Passing};
use crate::world::stars::{self, Constellation, hash};
use crate::world::{BirdPicture, Keepsake, Season};

pub const WINDOW_LOW: Vec2 = Vec2::new(-0.75, 1.0);
pub const WINDOW_HIGH: Vec2 = Vec2::new(0.75, 2.0);

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
}

pub struct WindowHit {
    pub sky: Vec3,
    pub frame: f32,
}

/// A picture on the back wall, so the high camera sees the sky rather than
/// looking down at the ground outside. Only the wooden frame takes room light.
pub fn window(point: Vec2, footprint: f32, outside: &Outside) -> Option<WindowHit> {
    if point.cmplt(WINDOW_LOW).any() || point.cmpgt(WINDOW_HIGH).any() {
        return None;
    }
    let size = WINDOW_HIGH - WINDOW_LOW;
    let coordinates = (point - WINDOW_LOW) / size;
    // A point sample still needs a nonzero edge at exact shape boundaries.
    let footprint = footprint.max(1e-5);
    let border = (point - WINDOW_LOW).min(WINDOW_HIGH - point).min_element();
    let middle = WINDOW_LOW.y + size.y * 0.5;
    let lift = outside.window_lift.clamp(0.0, 1.0);
    let bottom = WINDOW_LOW.y + size.y * 0.5 * lift;
    let glazed = point.y >= bottom;
    let distance = (border - 0.05)
        .min(if glazed {
            (point.x - (WINDOW_LOW.x + size.x * 0.5)).abs() - 0.0175
        } else {
            1.0
        })
        .min((point.y - middle).abs() - 0.02)
        .min((point.y - bottom).abs() - 0.02)
        .min((point.y - middle - size.y * 0.5 * lift).abs() - 0.02);
    let sky = landscape(coordinates, footprint / size.min_element(), outside);
    let sheen = if glazed {
        let stripe = 1.0
            - smoothstep(
                0.0,
                0.07,
                (coordinates.x + coordinates.y * 0.3 - 0.62).abs(),
            );
        0.02 + stripe * 0.07
    } else {
        0.0
    };
    Some(WindowHit {
        sky: sky.lerp(Vec3::new(0.76, 0.87, 1.0), sheen),
        frame: coverage(distance, footprint * 0.5),
    })
}

/// The sky's broad light, kept separate from the sun, stars, and rain streaks
/// so tiny animated details stay steady. Fireworks briefly tint this light.
pub fn light_through(outside: &Outside) -> Vec3 {
    let sky = Sky::at_hour(outside.hour);
    let noon = (1.0 - (outside.hour.rem_euclid(24.0) - 12.0).abs() / 6.0).max(0.0);
    let strength = sky.day * (0.65 + 0.25 * noon) + sky.twilight * 0.15;
    let cloud = outside.rain.max(outside.snow * 0.65);
    let storm = if outside.storm { outside.rain } else { 0.0 };
    weather_color((sky.horizon + sky.zenith) * 0.5, cloud) * strength * (1.0 - storm * 0.35)
        + Vec3::new(0.055, 0.075, 0.1) * outside.snow_cover.clamp(0.0, 1.0) * sky.day
        + Vec3::new(1.4, 1.7, 2.2) * outside.flash.clamp(0.0, 1.0)
        + firework(outside).map_or(Vec3::ZERO, |burst| burst.color * burst.brightness() * 0.5)
}

struct Sky {
    horizon: Vec3,
    zenith: Vec3,
    day: f32,
    twilight: f32,
}

impl Sky {
    fn at_hour(hour: f32) -> Self {
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

fn landscape(coordinates: Vec2, footprint: f32, outside: &Outside) -> Vec3 {
    let sky = Sky::at_hour(outside.hour);
    let rain = outside.rain.clamp(0.0, 1.0);
    let storm = if outside.storm { rain } else { 0.0 };
    let flash = outside.flash.clamp(0.0, 1.0);
    let snow = outside.snow.clamp(0.0, 1.0);
    let cloud = rain.max(snow * 0.85);
    let mut color = sky.horizon.lerp(sky.zenith, coordinates.y);
    let night = (1.0 - sky.day) * (1.0 - sky.twilight * 0.85) * (1.0 - cloud).powi(2);

    let stars = coordinates * stars::GRID;
    let cell = stars.floor();
    let seed = hash(cell);
    let center = stars::center(cell);
    let distance = ((stars - cell - center) / stars::GRID).length();
    let twinkle = 0.65 + 0.35 * (phase(outside.time, 0.7, TAU) + seed * TAU).sin();
    color += Vec3::new(0.78, 0.86, 1.0)
        * coverage(distance - 0.0025, footprint * 0.5)
        * stars::brightness(seed)
        * twinkle
        * night;

    // Correct discs for the window's proportions; the landscape itself uses
    // picture coordinates so its composition stays fixed as the camera turns.
    let proportions = Vec2::new(
        1.0,
        (WINDOW_HIGH.y - WINDOW_LOW.y) / (WINDOW_HIGH.x - WINDOW_LOW.x),
    );
    if let Some(streak) = &outside.meteor {
        color += Vec3::new(0.86, 0.92, 1.0)
            * shooting_star(coordinates, footprint, streak, proportions)
            * night;
    }

    let moon_point = (coordinates - Vec2::new(0.79, 0.8)) * proportions;
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
    let angle = outside.moon_phase * TAU;
    let across = if outside.moon_phase < 0.5 {
        moon_point.x
    } else {
        -moon_point.x
    };
    let terminator = angle.cos() * (0.055_f32.powi(2) - moon_point.y.powi(2)).max(0.0).sqrt();
    let lit = coverage(terminator - across, footprint * 0.5);
    let fullness = (1.0 - angle.cos()) * 0.5;
    let moon =
        coverage(moon_distance - 0.055, footprint * 0.5) * lit * smoothstep(0.0, 0.015, fullness)
            + (-moon_distance.powi(2) / 0.012).exp() * 0.1 * fullness;
    color += Vec3::new(0.72, 0.8, 0.98) * moon * night;

    let progress = (outside.hour.rem_euclid(24.0) - 6.5) / 13.0;
    let sun = Vec2::new(0.1 + 0.8 * progress, 0.2 + 0.65 * (progress * PI).sin());
    let sun_distance = ((coordinates - sun) * proportions).length();
    let sunlight = coverage(sun_distance - 0.04, footprint * 0.5)
        + (-sun_distance.powi(2) / 0.018).exp() * 0.35;
    color += Vec3::new(1.0, 0.77, 0.38) * sunlight * sky.day * (1.0 - cloud * 0.9);

    let drift =
        phase(outside.time, 0.012, 64.0) + (outside.wind_travel * 0.035).rem_euclid(64.0) as f32;
    let clouds = noise(coordinates * Vec2::new(4.5, 6.0) - Vec2::X * drift) * 0.65
        + noise(coordinates * Vec2::new(9.0, 12.0) - Vec2::X * drift * 2.0) * 0.35;
    let clouds = smoothstep(0.5 - storm * 0.15, 0.74 - storm * 0.15, clouds)
        * smoothstep(0.26, 0.4, coordinates.y)
        * (1.0 - smoothstep(0.85, 1.0, coordinates.y));
    let cloud_color = Vec3::new(0.055, 0.065, 0.1)
        .lerp(Vec3::new(0.94, 0.96, 1.0), sky.day)
        .lerp(Vec3::new(0.86, 0.57, 0.63), sky.twilight);
    color = weather_color(color.lerp(cloud_color, clouds * 0.6), cloud) * (1.0 - storm * 0.35);
    color = color.lerp(Vec3::new(0.75, 0.86, 1.0), flash * 0.85);

    if let Some(burst) = firework(outside) {
        color += burst.sparks(coordinates, footprint);
    }

    let land = match outside.season {
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
    };

    let hills = [
        (
            0.215
                + 0.025 * (coordinates.x * 8.0 + 0.6).sin()
                + 0.012 * (coordinates.x * 19.0).sin(),
            land[0],
        ),
        (
            0.16 + 0.027 * (coordinates.x * 7.0 + 1.8).sin() + 0.012 * (coordinates.x * 13.0).sin(),
            land[1],
        ),
        (
            0.095
                + 0.024 * (coordinates.x * 9.0 + 0.2).sin()
                + 0.009 * (coordinates.x * 21.0 + 1.0).sin(),
            land[2],
        ),
    ];
    let radius = coordinates.distance(Vec2::new(0.5, 0.04));
    let primary = spectrum((radius - 0.72) / 0.06)
        * coverage((radius - 0.75).abs() - 0.03, footprint * 0.5)
        * 0.38;
    let secondary = spectrum(1.0 - (radius - 0.815) / 0.09)
        * coverage((radius - 0.86).abs() - 0.045, footprint * 0.5)
        * 0.09;
    color += (primary + secondary)
        * outside.rainbow.clamp(0.0, 1.0)
        * (1.0 - rain * 0.65)
        * smoothstep(hills[0].0, hills[0].0 + 0.08, coordinates.y);

    if flash > 0.0 {
        color += Vec3::new(0.85, 0.92, 1.0)
            * lightning(coordinates, footprint, outside.last_flash)
            * flash;
    }
    let land_light = 0.04 + sky.day * 0.85 + sky.twilight * 0.18 + flash * 0.6;
    for (index, (height, land)) in hills.into_iter().enumerate() {
        let white = Vec3::new(0.78, 0.86, 0.96) + Vec3::splat(index as f32 * 0.015);
        let land = land.lerp(white, outside.snow_cover.clamp(0.0, 1.0));
        color = color.lerp(
            weather_color(land * land_light, cloud),
            coverage(coordinates.y - height, footprint * 0.5),
        );
    }

    color = tree(color, coordinates, footprint, outside, land_light, cloud);
    for passing in outside.passersby.iter().flatten() {
        color = passerby(color, coordinates, footprint, passing, &sky, cloud);
    }
    if matches!(outside.season, Season::Autumn | Season::Spring) {
        color = falling_leaves(color, coordinates, footprint, outside, land_light);
    }
    if outside.season == Season::Summer {
        color += fireflies(coordinates, footprint, outside.time) * night;
    }
    if let Some(bird) = outside.bird {
        color = visiting_bird(color, coordinates, footprint, bird, outside.season);
    }
    if let Some(gift) = outside.sill {
        let local = (coordinates - Vec2::new(0.78, 0.06)) * Vec2::new(1.5, 1.0);
        if local.x.abs() < 0.14 + footprint && (-0.02..0.2).contains(&local.y) {
            let (distance, _) =
                super::shapes::keepsake_distance(Vec3::new(local.x, local.y, 0.0), gift);
            let tint = match gift {
                Keepsake::Feather => Vec3::new(0.78, 0.9, 1.0),
                Keepsake::Petal => Vec3::new(1.0, 0.56, 0.76),
                Keepsake::Acorn => Vec3::new(0.85, 0.58, 0.32),
                Keepsake::Leaf => Vec3::new(1.0, 0.42, 0.2),
                Keepsake::Berry => Vec3::new(1.0, 0.3, 0.38),
                Keepsake::Pebble => Vec3::new(0.7, 0.76, 0.84),
            };
            color = color.lerp(tint, coverage(distance, footprint));
        }
    }
    if snow > 0.0 {
        color = snowfall(
            color,
            coordinates,
            footprint,
            outside.time,
            snow,
            outside.wind,
        );
    }

    let falling = phase(outside.time, 0.85, 64.0);
    let drops = Vec2::new(
        (coordinates.x + coordinates.y * (0.18 + outside.wind * 0.3)) * 36.0,
        (coordinates.y + falling) * 7.0,
    );
    let cell = drops.floor();
    let fraction = drops - cell;
    let across = (fraction.x - (0.2 + 0.6 * hash(cell))).abs() / 36.0 - 0.0012;
    let along = (fraction.y - 0.5).abs() / 7.0 - 0.035;
    let streak = coverage(across.max(along), footprint * 0.5);
    color + Vec3::splat(0.36) * streak * rain
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
        Passerby::Squirrel => Vec2::new(0.065, 0.05),
        Passerby::Plane => Vec2::new(0.045, 0.03),
        Passerby::Snowman => Vec2::new(0.07, 0.10),
        Passerby::Lanterns => Vec2::new(0.11, 0.15),
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
        Passerby::Squirrel => {
            point.x *= passing.facing;
            let body = (point / Vec2::new(1.4, 1.0)).length() - 0.013;
            let head = point.distance(Vec2::new(0.021, 0.009)) - 0.009;
            let tail_point = point - Vec2::new(-0.025, 0.014);
            let tail = (tail_point / Vec2::new(0.014, 0.025)).length() - 1.0;
            color = paint(
                color,
                lit(Vec3::new(0.78, 0.4, 0.2)),
                body.min(head).min(tail * 0.014),
            );
            color = paint(
                color,
                lit(Vec3::splat(0.15)),
                point.distance(Vec2::new(0.025, 0.013)) - 0.0015,
            );
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
        Passerby::Snowman => {
            point.y /= 1.0 - passing.phase * 0.65;
            point.x -= point.y * passing.phase * 0.3;
            let body = point.distance(Vec2::new(0.0, 0.023)) - 0.029;
            let head = point.distance(Vec2::new(0.0, 0.063)) - 0.019;
            color = paint(color, lit(Vec3::new(0.88, 0.94, 1.0)), body.min(head));
            let scarf = capsule(
                point,
                Vec2::new(-0.018, 0.046),
                Vec2::new(0.018, 0.043),
                0.003,
            );
            color = paint(color, lit(Vec3::new(0.9, 0.35, 0.27)), scarf);
            let nose = capsule(
                point,
                Vec2::new(0.002, 0.06),
                Vec2::new(0.019, 0.057),
                0.002,
            );
            color = paint(color, lit(Vec3::new(1.0, 0.64, 0.28)), nose);
            for across in [-0.007, 0.006] {
                color = paint(
                    color,
                    lit(Vec3::splat(0.16)),
                    point.distance(Vec2::new(across, 0.069)) - 0.0015,
                );
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
    }
    let presence = if passing.kind == Passerby::Snowman {
        1.0
    } else {
        smoothstep(0.0, 0.08, passing.phase) * (1.0 - smoothstep(0.92, 1.0, passing.phase))
    };
    background.lerp(color, presence)
}

fn visiting_bird(
    color: Vec3,
    coordinates: Vec2,
    footprint: f32,
    bird: BirdPicture,
    season: Season,
) -> Vec3 {
    let mut point = (coordinates - bird.position) * Vec2::new(1.5, 1.0);
    // Nothing of the bird, wings and tail included, reaches farther.
    if point.length() > 0.11 + footprint {
        return color;
    }
    point.x *= bird.facing;
    point.y -= if bird.singing {
        0.006 * bird.flap.sin()
    } else {
        0.0
    };
    let head = Vec2::new(0.025 + bird.peck * 0.012, 0.025 - bird.peck * 0.03);
    let body = (point / Vec2::new(1.15, 1.0)).length() - 0.033;
    let mut distance = body
        .min(point.distance(head) - 0.021)
        .min(capsule(
            point,
            Vec2::new(-0.02, -0.01),
            Vec2::new(-0.065, 0.004),
            0.009,
        ))
        .min(capsule(
            point,
            head + Vec2::new(0.012, 0.0),
            head + Vec2::new(0.036, -0.005),
            0.004,
        ));
    if bird.flying {
        let wing = Vec2::new(-0.015, bird.flap.sin() * 0.07);
        distance = distance.min(capsule(point, Vec2::ZERO, wing, 0.012));
        distance = distance.min(capsule(
            point,
            Vec2::new(0.005, 0.0),
            Vec2::new(0.045, -wing.y * 0.75),
            0.009,
        ));
    }
    let breast = coverage(
        (point - Vec2::new(0.019, -0.009)).length() - 0.022,
        footprint * 0.5,
    );
    let warm = if season == Season::Winter {
        Vec3::new(0.72, 0.25, 0.13)
    } else {
        Vec3::new(0.52, 0.39, 0.29)
    };
    let feather = Vec3::new(0.22, 0.2, 0.19).lerp(warm, breast);
    color.lerp(feather, coverage(distance, footprint * 0.5))
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
            0.88 - index as f32 * 0.14,
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
// The window and its room light share the same burst and fading.
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
            0.57 + 0.28 * hash(Vec2::new(slot, 2.0)),
        ),
        color: colors[(hash(Vec2::new(slot, 3.0)) * colors.len() as f32) as usize % colors.len()],
        age,
        seed: slot,
    })
}

fn tree(
    mut color: Vec3,
    coordinates: Vec2,
    footprint: f32,
    outside: &Outside,
    light: f32,
    cloud: f32,
) -> Vec3 {
    let mut point = coordinates - Vec2::new(0.25, 0.145);
    point.x -= outside.wind * 0.24 * point.y.max(0.0).powi(2);
    let edge = footprint * 0.5;
    let branches = [
        (Vec2::ZERO, Vec2::new(0.012, 0.31), 0.009),
        (Vec2::new(0.006, 0.12), Vec2::new(-0.09, 0.27), 0.005),
        (Vec2::new(0.008, 0.17), Vec2::new(0.11, 0.31), 0.005),
        (Vec2::new(-0.055, 0.22), Vec2::new(-0.12, 0.25), 0.003),
        (Vec2::new(-0.06, 0.22), Vec2::new(-0.055, 0.32), 0.003),
        (Vec2::new(0.065, 0.25), Vec2::new(0.13, 0.26), 0.003),
        (Vec2::new(0.065, 0.25), Vec2::new(0.07, 0.35), 0.003),
        (Vec2::new(0.01, 0.26), Vec2::new(-0.025, 0.37), 0.003),
    ];
    let mut wood = f32::INFINITY;
    let mut dust = f32::INFINITY;
    for (start, end, radius) in branches {
        wood = wood.min(capsule(point, start, end, radius));
        if (end - start).x.abs() > 0.02 {
            let lift = Vec2::Y * radius;
            dust = dust.min(capsule(point, start + lift, end + lift, radius * 0.6));
        }
    }
    color = color.lerp(
        weather_color(Vec3::new(0.34, 0.25, 0.24) * light, cloud),
        coverage(wood, edge),
    );
    if outside.season == Season::Winter {
        return color.lerp(
            Vec3::new(0.88, 0.93, 1.0) * light,
            coverage(dust, edge) * outside.snow_cover.clamp(0.0, 1.0),
        );
    }

    let crown = [
        (Vec2::new(-0.07, 0.28), 0.09),
        (Vec2::new(0.065, 0.3), 0.1),
        (Vec2::new(-0.015, 0.36), 0.085),
    ]
    .into_iter()
    .map(|(center, radius)| point.distance(center) - radius)
    .fold(f32::INFINITY, f32::min);
    let texture = noise(point * 48.0);
    let foliage = match outside.season {
        Season::Spring => Vec3::new(0.48, 0.72, 0.34).lerp(Vec3::new(0.7, 0.85, 0.46), texture),
        Season::Summer => Vec3::new(0.2, 0.43, 0.28).lerp(Vec3::new(0.4, 0.64, 0.31), texture),
        Season::Autumn => Vec3::new(0.86, 0.28, 0.2).lerp(Vec3::new(1.0, 0.76, 0.28), texture),
        Season::Winter => unreachable!(),
    };
    let mut foliage = weather_color(foliage * light, cloud);
    if outside.season == Season::Spring {
        let flowers = point * 65.0;
        let cell = flowers.floor();
        let center = Vec2::new(hash(cell + Vec2::X), hash(cell + Vec2::Y)) * 0.5 + 0.25;
        let distance = (flowers - cell - center).length() / 65.0;
        let blossom = coverage(distance - 0.0035, edge) * smoothstep(0.55, 0.8, hash(cell));
        foliage = foliage.lerp(Vec3::new(1.0, 0.83, 0.9) * light, blossom);
    }
    color.lerp(foliage, coverage(crown, edge))
}

fn falling_leaves(
    mut color: Vec3,
    coordinates: Vec2,
    footprint: f32,
    outside: &Outside,
    light: f32,
) -> Vec3 {
    let time = outside.time;
    for index in 0..6 {
        let seed = hash(Vec2::new(index as f32, 23.0));
        let fall = (phase(time, 0.035, 1.0) + seed).fract();
        let sway = (phase(time, 0.4, TAU) + seed * TAU).sin();
        let center = Vec2::new(
            (0.18 + seed * 0.24 + fall * (0.25 + outside.wind * 0.8) + sway * 0.035)
                .rem_euclid(1.0),
            0.58 - fall * 0.65,
        );
        let offset = coordinates - center;
        let distance = (offset * Vec2::new(1.0, 1.7)).length() - 0.0055;
        let leaf = if outside.season == Season::Spring {
            Vec3::new(1.0, 0.78, 0.88)
        } else {
            Vec3::new(0.9, 0.3, 0.18).lerp(Vec3::new(1.0, 0.8, 0.3), seed)
        };
        color = color.lerp(leaf * light, coverage(distance, footprint * 0.5));
    }
    color
}

fn fireflies(coordinates: Vec2, footprint: f32, time: f64) -> Vec3 {
    let mut glow = 0.0;
    for index in 0..5 {
        let seed = hash(Vec2::new(index as f32, 41.0));
        let wander = phase(time, 0.12, TAU) + seed * TAU;
        let center = Vec2::new(
            0.1 + seed * 0.8 + wander.sin() * 0.025,
            0.1 + seed * 0.13 + (wander * 2.0).cos() * 0.015,
        );
        let distance = coordinates.distance(center);
        let blink = (phase(time, 0.8, TAU) + seed * 19.0).sin().max(0.0).powi(6);
        glow += (coverage(distance - 0.003, footprint * 0.5)
            + (-distance.powi(2) / 0.00015).exp() * 0.15)
            * blink;
    }
    Vec3::new(0.75, 1.0, 0.27) * glow
}

fn snowfall(
    mut color: Vec3,
    coordinates: Vec2,
    footprint: f32,
    time: f64,
    snow: f32,
    wind: f32,
) -> Vec3 {
    for layer in [0.0, 1.0] {
        let grid = Vec2::new(19.0 + layer * 12.0, 12.0 + layer * 8.0);
        let falling = phase(time, 0.035 + f64::from(layer) * 0.015, 64.0);
        let flakes = (coordinates + Vec2::new(coordinates.y * wind * 0.45, falling)) * grid;
        let cell = flakes.floor();
        let seed = hash(cell + Vec2::splat(layer * 17.0));
        let sway = (phase(time, 0.45, TAU) + seed * TAU).sin() * 0.15;
        let center = Vec2::new(
            0.25 + hash(cell + Vec2::X) * 0.5 + sway,
            0.2 + hash(cell + Vec2::Y) * 0.6,
        );
        let distance = ((flakes - cell - center) / grid).length();
        let flake = coverage(distance - (0.0035 - layer * 0.001), footprint * 0.5 + 0.001)
            * smoothstep(1.0 - snow, 1.15 - snow, seed);
        color = color.lerp(Vec3::new(0.88, 0.94, 1.0), flake * (0.8 - layer * 0.25));
    }
    color
}

fn weather_color(color: Vec3, rain: f32) -> Vec3 {
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
fn phase(time: f64, speed: f64, period: f32) -> f32 {
    (time * speed).rem_euclid(f64::from(period)) as f32
}

fn noise(point: Vec2) -> f32 {
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

    #[test]
    fn window_samples_stay_finite_through_every_passing_at_every_hour() {
        use crate::world::{Sky, World};
        use jiff::civil::Date;

        for kind in Passerby::ALL {
            let mut world = World::new(7, 9.0);
            world.date = match kind {
                Passerby::Kite => Date::constant(2026, 4, 12),
                Passerby::Lanterns => Date::constant(2026, 9, 25),
                _ => Date::constant(2026, 10, 12),
            };
            if matches!(kind, Passerby::Plane | Passerby::Lanterns) {
                world.hour = 23.0;
            }
            if kind == Passerby::Snowman {
                world.weather.snow_cover = 0.85;
                world.weather.settled_snow = Some(0.0);
            }
            world.weather.sky = Sky::Clear;
            let passing = (0..30_000)
                .find_map(|step| {
                    Passerby::at(
                        kind,
                        7,
                        f64::from(step) * 2.0,
                        world.date,
                        world.hour,
                        &world.weather,
                    )
                })
                .unwrap();
            for progress in [0.0, 0.5, 1.0] {
                let time = passing.began
                    + progress
                        * if passing.duration.is_finite() {
                            passing.duration
                        } else {
                            1200.0
                        };
                let passing = if kind == Passerby::Snowman {
                    Passing {
                        phase: progress as f32,
                        ..passing
                    }
                } else {
                    // Sample immediately inside both ends of the scheduled visit.
                    kind.at(
                        7,
                        time.clamp(
                            passing.began + 0.0001,
                            passing.began + passing.duration - 0.0001,
                        ),
                        world.date,
                        world.hour,
                        &world.weather,
                    )
                    .unwrap()
                };
                for hour in 0..24 {
                    for striped in [false, true] {
                        let mut outside = Outside::clear(hour as f32, time);
                        outside.passersby[0] = Some(Passing { striped, ..passing });
                        for row in 0..=8 {
                            for column in 0..=12 {
                                let coordinates = Vec2::new(column as f32 / 12.0, row as f32 / 8.0);
                                assert!(
                                    landscape(coordinates, 0.004, &outside).is_finite(),
                                    "{kind:?}, {progress}, {hour}"
                                );
                            }
                        }
                        for offset in [Vec2::ZERO, Vec2::new(0.001, 0.002)] {
                            assert!(
                                landscape(passing.position + offset, 0.0, &outside).is_finite()
                            );
                        }
                    }
                }
            }
        }
    }

    fn point(across: f32, up: f32) -> Vec2 {
        WINDOW_LOW + (WINDOW_HIGH - WINDOW_LOW) * Vec2::new(across, up)
    }

    #[test]
    fn window_samples_with_constellations_stay_finite_at_every_hour() {
        let known = [Constellation {
            stars: (0..7).collect(),
            ..Constellation::default()
        }];
        let segments = StarSegment::from_constellations(&known);
        for hour in 0..24 {
            for weather in [0.0, 0.5, 1.0] {
                let outside = Outside {
                    constellations: segments.clone(),
                    constellation_glow: 0.085,
                    rain: weather,
                    snow: weather,
                    ..Outside::clear(hour as f32, 30.0)
                };
                for footprint in [0.0, 0.001, 0.02] {
                    for row in 0..=16 {
                        for column in 0..=22 {
                            let hit = window(
                                point(column as f32 / 22.0, row as f32 / 16.0),
                                footprint,
                                &outside,
                            )
                            .unwrap();
                            assert!(hit.sky.is_finite() && hit.frame.is_finite());
                        }
                    }
                    for star in stars::field() {
                        assert!(
                            landscape(star.position, footprint.max(1e-5), &outside).is_finite()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn window_samples_stay_finite_with_a_flying_or_perched_bird_in_every_season() {
        use crate::world::{Random, visitor::Bird};

        let bird = Bird::new(0.0, 0.0, &mut Random::new(1));
        for season in [
            Season::Spring,
            Season::Summer,
            Season::Autumn,
            Season::Winter,
        ] {
            for time in [0.0, 1.5, 3.0, 20.0, 1e9, f64::MAX] {
                let outside = Outside {
                    season,
                    bird: Some(bird.picture(time)),
                    ..Outside::clear(12.0, time)
                };
                for row in 0..=20 {
                    for column in 0..=30 {
                        let hit = window(
                            point(column as f32 / 30.0, row as f32 / 20.0),
                            0.02,
                            &outside,
                        )
                        .unwrap();
                        assert!(hit.sky.is_finite() && hit.frame.is_finite());
                    }
                }
            }
        }
    }

    #[test]
    fn only_points_inside_the_window_rectangle_have_a_view() {
        let outside = Outside::clear(12.0, 0.0);
        for point in [WINDOW_LOW, WINDOW_HIGH, point(0.25, 0.35)] {
            assert!(window(point, 0.01, &outside).is_some());
        }
        for point in [
            point(-0.001, 0.5),
            point(1.001, 0.5),
            point(0.5, -0.001),
            point(0.5, 1.001),
        ] {
            assert!(window(point, 0.01, &outside).is_none());
        }
    }

    #[test]
    fn the_border_and_middle_bars_cover_the_frame_but_panes_do_not() {
        let outside = Outside::clear(12.0, 0.0);
        for point in [
            point(0.01, 0.3),
            point(0.99, 0.3),
            point(0.25, 0.01),
            point(0.25, 0.99),
            point(0.5, 0.3),
            point(0.25, 0.5),
        ] {
            assert_eq!(window(point, 0.005, &outside).unwrap().frame, 1.0);
        }
        for point in [point(0.25, 0.3), point(0.75, 0.8)] {
            assert_eq!(window(point, 0.005, &outside).unwrap().frame, 0.0);
        }
        let edge = Vec2::new(WINDOW_LOW.x + 0.05, 1.3);
        assert!((window(edge, 0.01, &outside).unwrap().frame - 0.5).abs() < 1e-4);
    }

    #[test]
    fn open_and_shut_window_samples_stay_finite_in_extreme_gusts() {
        for wind in [-1.0, 0.0, 1.0] {
            for window_open in [false, true] {
                for season in [
                    Season::Spring,
                    Season::Summer,
                    Season::Autumn,
                    Season::Winter,
                ] {
                    let outside = Outside {
                        wind,
                        window_lift: if window_open { 1.0 } else { 0.0 },
                        season,
                        rain: 1.0,
                        snow: 1.0,
                        wind_travel: 1e12,
                        ..Outside::clear(12.0, 1e12)
                    };
                    for column in 0..=16 {
                        for row in 0..=12 {
                            let sample = window(
                                point(column as f32 / 16.0, row as f32 / 12.0),
                                0.0,
                                &outside,
                            )
                            .unwrap();
                            assert!(sample.sky.is_finite() && sample.frame.is_finite());
                            assert!((0.0..=1.0).contains(&sample.frame));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn window_samples_stay_finite_across_the_day_and_weather_even_at_edges() {
        for hour in 0..=24 {
            for rain in [0.0, 0.5, 1.0] {
                for rainbow in [0.0, 1.0] {
                    for time in [-1e6, 0.0, 42.5, 1e9, f64::MAX] {
                        let outside = Outside {
                            hour: hour as f32,
                            rain,
                            rainbow,
                            time,
                            ..Outside::clear(hour as f32, time)
                        };
                        assert!(light_through(&outside).is_finite());
                        for column in 0..=12 {
                            for row in 0..=8 {
                                for footprint in [0.0, 0.02, 0.2] {
                                    let sample = window(
                                        point(column as f32 / 12.0, row as f32 / 8.0),
                                        footprint,
                                        &outside,
                                    )
                                    .unwrap();
                                    assert!(sample.sky.is_finite(), "{outside:?}");
                                    assert!((0.0..=1.0).contains(&sample.frame));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn seasonal_samples_stay_finite_with_snow_and_every_moon_phase() {
        for season in [
            Season::Spring,
            Season::Summer,
            Season::Autumn,
            Season::Winter,
        ] {
            for hour in [0.0, 6.5, 12.0, 19.5, 24.0] {
                for snow in [0.0, 0.5, 1.0] {
                    for snow_cover in [0.0, 0.5, 1.0] {
                        for moon_phase in [0.0, 0.1, 0.25, 0.4, 0.5, 0.6, 0.75, 0.9, 1.0] {
                            for time in [0.0, 42.5, 1e9, f64::MAX] {
                                let outside = Outside {
                                    season,
                                    snow,
                                    snow_cover,
                                    moon_phase,
                                    ..Outside::clear(hour, time)
                                };
                                assert!(light_through(&outside).is_finite());
                                for coordinates in [
                                    Vec2::ZERO,
                                    Vec2::ONE,
                                    Vec2::new(0.25, 0.4),
                                    Vec2::new(0.79, 0.8),
                                    Vec2::new(0.5, 0.6),
                                ] {
                                    for footprint in [0.0, 0.02, 0.2] {
                                        let sample = window(
                                            point(coordinates.x, coordinates.y),
                                            footprint,
                                            &outside,
                                        )
                                        .unwrap();
                                        assert!(sample.sky.is_finite(), "{outside:?}");
                                        assert!((0.0..=1.0).contains(&sample.frame));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn storm_samples_and_room_light_stay_finite_during_lightning() {
        for hour in [0.0, 6.5, 12.0, 19.5, 24.0] {
            for last_flash in [0.0, 42.5, 1e9, f64::MAX] {
                for flash in [0.0, 0.25, 0.5, 1.0] {
                    let outside = Outside {
                        rain: 1.0,
                        storm: true,
                        flash,
                        last_flash,
                        ..Outside::clear(hour, last_flash)
                    };
                    assert!(light_through(&outside).is_finite());
                    for column in 0..=12 {
                        for row in 0..=8 {
                            for footprint in [0.0, 0.02, 0.2] {
                                let sample = window(
                                    point(column as f32 / 12.0, row as f32 / 8.0),
                                    footprint,
                                    &outside,
                                )
                                .unwrap();
                                assert!(sample.sky.is_finite(), "{outside:?}");
                                assert!(sample.frame.is_finite());
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_rainbow_adds_light_on_its_arc_and_leaves_distant_panes_alone() {
        let clear = Outside::clear(12.0, 0.0);
        let rainbow = Outside {
            rainbow: 1.0,
            ..clear.clone()
        };
        let on_arc = point(0.25, 0.04 + (0.75_f32.powi(2) - 0.25_f32.powi(2)).sqrt());
        let far_away = point(0.25, 0.35);
        let sample = |point, outside: &Outside| window(point, 0.001, outside).unwrap();
        assert_eq!(sample(on_arc, &clear).frame, 0.0);
        assert!(sample(on_arc, &rainbow).sky.length() > sample(on_arc, &clear).sky.length());
        assert_eq!(sample(far_away, &rainbow).sky, sample(far_away, &clear).sky);
    }

    #[test]
    fn noon_is_brighter_than_midnight_and_rain_makes_the_sky_greyer() {
        let pane = point(0.3, 0.85);
        let clear = Outside::clear(12.0, 0.0);
        let rainy = Outside {
            rain: 1.0,
            ..clear.clone()
        };
        let noon = window(pane, 0.001, &clear).unwrap().sky;
        let midnight = window(pane, 0.001, &Outside::clear(0.0, 0.0)).unwrap().sky;
        let rain = window(pane, 0.001, &rainy).unwrap().sky;
        assert!(noon.element_sum() > midnight.element_sum());
        assert!(rain.max_element() - rain.min_element() < noon.max_element() - noon.min_element());
        assert!(rain.element_sum() < noon.element_sum());
    }

    #[test]
    fn window_light_peaks_at_midday_and_fades_with_rain_and_night() {
        let noon = light_through(&Outside::clear(12.0, 0.0));
        assert!(light_through(&Outside::clear(0.0, 0.0)).max_element() < 0.001);
        for hour in 0..=24 {
            assert!(
                light_through(&Outside::clear(hour as f32, 0.0)).element_sum()
                    <= noon.element_sum()
            );
        }
        let rainy = Outside {
            rain: 1.0,
            ..Outside::clear(12.0, 0.0)
        };
        assert!(light_through(&rainy).element_sum() < noon.element_sum());
    }

    #[test]
    fn fireworks_and_their_room_light_stay_finite_at_burst_boundaries_and_old_times() {
        for hour in [0.0, 0.999, 12.0, 23.5, 23.999] {
            for time in [-1e6, 0.0, 0.8, 1.799, 1.8, 3.2, 2048.0, 1e9, f64::MAX] {
                let outside = Outside {
                    fireworks: true,
                    ..Outside::clear(hour, time)
                };
                assert!(light_through(&outside).is_finite());
                for column in 0..=20 {
                    for row in 0..=16 {
                        let sample = window(
                            point(column as f32 / 20.0, row as f32 / 16.0),
                            0.0,
                            &outside,
                        )
                        .unwrap();
                        assert!(sample.sky.is_finite());
                    }
                }
            }
        }
    }
    #[test]
    fn nothing_of_a_shooting_star_lies_beyond_the_bound_that_skips_it() {
        let proportions = Vec2::new(1.0, 0.6);
        let streak = Streak {
            head: Vec2::new(0.5, 0.7),
            tail: Vec2::new(0.6, 0.78),
            brightness: 1.0,
        };
        let footprint = 0.01;
        // Just outside the bound, the unbounded streak is all but dark.
        for point in [
            Vec2::new(0.43, 0.7),
            Vec2::new(0.67, 0.8),
            Vec2::new(0.55, 0.62),
        ] {
            assert_eq!(shooting_star(point, footprint, &streak, proportions), 0.0);
        }
        let head = shooting_star(streak.head, footprint, &streak, proportions);
        let tail = shooting_star(streak.tail, footprint, &streak, proportions);
        assert!(head > tail && head.is_finite());
        let resting = Streak {
            tail: streak.head,
            ..streak
        };
        assert!(shooting_star(streak.head, footprint, &resting, proportions).is_finite());
    }

    #[test]
    fn a_spark_with_no_travel_yet_has_a_finite_distance() {
        for age in [0.0, 1e-8, 1e-6, 1e-4, 0.1] {
            let burst = Firework {
                center: Vec2::splat(0.5),
                color: Vec3::ONE,
                age,
                seed: 0.0,
            };
            assert!(burst.sparks(Vec2::splat(0.5), 0.0).is_finite());
        }
    }
}
