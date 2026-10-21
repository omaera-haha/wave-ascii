use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};

use crate::sdf::{coverage, mix, smoothstep};

pub const WINDOW_LOW: Vec2 = Vec2::new(-0.75, 1.0);
pub const WINDOW_HIGH: Vec2 = Vec2::new(0.75, 2.0);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outside {
    pub hour: f32,
    pub rain: f32,
    pub rainbow: f32,
    pub time: f64,
}

impl Outside {
    pub fn clear(hour: f32, time: f64) -> Self {
        Self {
            hour,
            rain: 0.0,
            rainbow: 0.0,
            time,
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
    let distance = (border - 0.05)
        .min((point.x - (WINDOW_LOW.x + size.x * 0.5)).abs() - 0.0175)
        .min((point.y - (WINDOW_LOW.y + size.y * 0.6)).abs() - 0.0175);
    Some(WindowHit {
        sky: landscape(coordinates, footprint / size.min_element(), outside),
        frame: coverage(distance, footprint * 0.5),
    })
}

/// The sky's broad light, kept separate from the sun, stars, and rain streaks
/// so tiny animated details do not make the room's illumination flicker.
pub fn light_through(outside: &Outside) -> Vec3 {
    let sky = Sky::at_hour(outside.hour);
    let noon = (1.0 - (outside.hour.rem_euclid(24.0) - 12.0).abs() / 6.0).max(0.0);
    let strength = sky.day * (0.65 + 0.25 * noon) + sky.twilight * 0.15;
    weather_color((sky.horizon + sky.zenith) * 0.5, outside.rain) * strength
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
    let mut color = sky.horizon.lerp(sky.zenith, coordinates.y);
    let night = (1.0 - sky.day) * (1.0 - sky.twilight * 0.85) * (1.0 - rain).powi(2);

    let stars = coordinates * Vec2::new(22.0, 16.0);
    let cell = stars.floor();
    let seed = hash(cell);
    let center = Vec2::new(hash(cell + Vec2::X), hash(cell + Vec2::Y)) * 0.6 + 0.2;
    let distance = ((stars - cell - center) / Vec2::new(22.0, 16.0)).length();
    let twinkle = 0.65 + 0.35 * (phase(outside.time, 0.7, TAU) + seed * TAU).sin();
    color += Vec3::new(0.78, 0.86, 1.0)
        * coverage(distance - 0.0025, footprint * 0.5)
        * smoothstep(0.78, 0.9, seed)
        * twinkle
        * night;

    // Correct discs for the window's proportions; the landscape itself uses
    // picture coordinates so its composition stays fixed as the camera turns.
    let proportions = Vec2::new(
        1.0,
        (WINDOW_HIGH.y - WINDOW_LOW.y) / (WINDOW_HIGH.x - WINDOW_LOW.x),
    );
    let moon_distance = ((coordinates - Vec2::new(0.79, 0.8)) * proportions).length();
    let moon = coverage(moon_distance - 0.055, footprint * 0.5)
        + (-moon_distance.powi(2) / 0.012).exp() * 0.1;
    color += Vec3::new(0.72, 0.8, 0.98) * moon * night;

    let progress = (outside.hour.rem_euclid(24.0) - 6.5) / 13.0;
    let sun = Vec2::new(0.1 + 0.8 * progress, 0.2 + 0.65 * (progress * PI).sin());
    let sun_distance = ((coordinates - sun) * proportions).length();
    let sunlight = coverage(sun_distance - 0.04, footprint * 0.5)
        + (-sun_distance.powi(2) / 0.018).exp() * 0.35;
    color += Vec3::new(1.0, 0.77, 0.38) * sunlight * sky.day * (1.0 - rain * 0.9);

    let drift = phase(outside.time, 0.012, 64.0);
    let clouds = noise(coordinates * Vec2::new(4.5, 6.0) - Vec2::X * drift) * 0.65
        + noise(coordinates * Vec2::new(9.0, 12.0) - Vec2::X * drift * 2.0) * 0.35;
    let clouds = smoothstep(0.5, 0.74, clouds)
        * smoothstep(0.26, 0.4, coordinates.y)
        * (1.0 - smoothstep(0.85, 1.0, coordinates.y));
    let cloud_color = Vec3::new(0.055, 0.065, 0.1)
        .lerp(Vec3::new(0.94, 0.96, 1.0), sky.day)
        .lerp(Vec3::new(0.86, 0.57, 0.63), sky.twilight);
    color = weather_color(color.lerp(cloud_color, clouds * 0.6), rain);

    let hills = [
        (
            0.215
                + 0.025 * (coordinates.x * 8.0 + 0.6).sin()
                + 0.012 * (coordinates.x * 19.0).sin(),
            Vec3::new(0.19, 0.35, 0.42),
        ),
        (
            0.16 + 0.027 * (coordinates.x * 7.0 + 1.8).sin() + 0.012 * (coordinates.x * 13.0).sin(),
            Vec3::new(0.18, 0.39, 0.3),
        ),
        (
            0.095
                + 0.024 * (coordinates.x * 9.0 + 0.2).sin()
                + 0.009 * (coordinates.x * 21.0 + 1.0).sin(),
            Vec3::new(0.13, 0.3, 0.23),
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

    let land_light = 0.04 + sky.day * 0.85 + sky.twilight * 0.18;
    for (height, land) in hills {
        color = color.lerp(
            weather_color(land * land_light, rain),
            coverage(coordinates.y - height, footprint * 0.5),
        );
    }

    let falling = phase(outside.time, 0.85, 64.0);
    let drops = Vec2::new(
        (coordinates.x + coordinates.y * 0.18) * 36.0,
        (coordinates.y + falling) * 7.0,
    );
    let cell = drops.floor();
    let fraction = drops - cell;
    let across = (fraction.x - (0.2 + 0.6 * hash(cell))).abs() / 36.0 - 0.0012;
    let along = (fraction.y - 0.5).abs() / 7.0 - 0.035;
    let streak = coverage(across.max(along), footprint * 0.5);
    color + Vec3::splat(0.36) * streak * rain
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

fn hash(cell: Vec2) -> f32 {
    let cell = Vec2::new(cell.x.rem_euclid(64.0), cell.y.rem_euclid(64.0));
    (cell.dot(Vec2::new(127.1, 311.7)).sin() * 43_758.547)
        .fract()
        .abs()
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

    fn point(across: f32, up: f32) -> Vec2 {
        WINDOW_LOW + (WINDOW_HIGH - WINDOW_LOW) * Vec2::new(across, up)
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
            point(0.25, 0.6),
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
    fn a_rainbow_adds_light_on_its_arc_and_leaves_distant_panes_alone() {
        let clear = Outside::clear(12.0, 0.0);
        let rainbow = Outside {
            rainbow: 1.0,
            ..clear
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
        let rainy = Outside { rain: 1.0, ..clear };
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
}
