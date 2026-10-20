use std::f32::consts::TAU;

use glam::{Vec2, Vec3};

use crate::frame::{Frame, Grid, Shade};

// The lake's far shore. Everything below it is reflection.
const SHORE: f32 = -0.3;
const STAR_SPACING: f32 = 0.075;
const GREEN: Vec3 = Vec3::new(0.25, 1.0, 0.55);
const CYAN: Vec3 = Vec3::new(0.2, 0.85, 0.95);
const VIOLET: Vec3 = Vec3::new(0.65, 0.35, 1.0);

/// A fold of light: its height, how far it reaches up, and how it ripples.
struct Curtain {
    height: f32,
    reach: f32,
    strength: f32,
    // Two sine waves bend the curtain's lower edge.
    waves: [(f32, f32, f32); 2],
    seed: i32,
}

const CURTAINS: [Curtain; 3] = [
    Curtain {
        height: 0.12,
        reach: 0.32,
        strength: 1.0,
        waves: [(1.1, 0.13, 0.17), (2.9, -0.21, 0.06)],
        seed: 3,
    },
    Curtain {
        height: 0.32,
        reach: 0.4,
        strength: 0.65,
        waves: [(0.8, -0.09, 0.2), (2.3, 0.17, 0.07)],
        seed: 11,
    },
    Curtain {
        height: -0.02,
        reach: 0.18,
        strength: 0.55,
        waves: [(1.6, 0.11, 0.1), (3.7, -0.27, 0.04)],
        seed: 23,
    },
];

/// The northern lights folding over mountains and a pine forest, mirrored in
/// a still lake under the stars. `yaw` pans along the shore, `pitch` tilts
/// the view, and `zoom` scales it.
pub fn render(grid: Grid, time: f32, yaw: f32, pitch: f32, zoom: f32) -> Frame {
    let pixel = grid.scale() / zoom;
    Frame::sample(grid, |point| {
        let point = point / zoom + Vec2::new(yaw * 0.8, -pitch * 0.5);
        if point.y >= SHORE {
            Shade::from_rgb(landscape(point, time, pixel))
        } else {
            // The water mirrors the land and sky, broken by slow ripples that
            // grow toward the viewer.
            let depth = SHORE - point.y;
            let ripple = (depth * 0.04 + 0.002)
                * ((point.y * 90.0 / (depth + 0.15) + time * 1.3).sin()
                    + 0.5 * (point.x * 7.0 - time * 0.6).sin());
            let mirrored = Vec2::new(point.x + ripple, 2.0 * SHORE - point.y);
            let reflection = landscape(mirrored, time, pixel);
            let water = Vec3::new(0.01, 0.03, 0.06);
            Shade::from_rgb(water + reflection * Vec3::new(0.5, 0.6, 0.7))
        }
    })
}

// The scene above the water line.
fn landscape(point: Vec2, time: f32, pixel: f32) -> Vec3 {
    let light = aurora(point, time, pixel);
    let forest = forest_line(point.x);
    if point.y < forest {
        // Near hills under the trees, barely lit.
        let edge = ((forest - point.y) / pixel).min(1.0);
        return Vec3::new(0.02, 0.05, 0.06).lerp(Vec3::new(0.03, 0.08, 0.08), 1.0 - edge);
    }
    let ridge = mountain_line(point.x);
    if point.y < ridge {
        // Far mountains, their snowy crests tinted by the aurora.
        let below = ridge - point.y;
        let snow = (-below / 0.05).exp();
        let glow = Vec3::splat(0.07).lerp(GREEN * 0.35, 0.5) * snow;
        return Vec3::new(0.04, 0.06, 0.1) + glow + light * 0.12;
    }

    let height = (point.y - SHORE).max(0.0);
    let sky =
        Vec3::new(0.03, 0.12, 0.16).lerp(Vec3::new(0.008, 0.015, 0.05), (height * 1.4).min(1.0));
    sky + light + Vec3::splat(stars(point, time, pixel))
}

// The light of all curtains at a point, already colored.
fn aurora(point: Vec2, time: f32, pixel: f32) -> Vec3 {
    // Fine rays would alias on coarse grids, so they fade as pixels grow.
    let fine = (0.02 / pixel).clamp(0.0, 1.0);
    let mut total = Vec3::ZERO;
    for curtain in &CURTAINS {
        let [
            (frequency, speed, amplitude),
            (frequency_2, speed_2, amplitude_2),
        ] = curtain.waves;
        let phase = curtain.seed as f32;
        let base = curtain.height
            + amplitude * (point.x * frequency + time * speed + phase).sin()
            + amplitude_2 * (point.x * frequency_2 + time * speed_2 + phase * 2.0).sin();
        let above = point.y - base;
        if above < -0.06 {
            continue;
        }
        // A sharp lower hem, fading upward.
        let hem = ((above + 0.06) / 0.06).clamp(0.0, 1.0).powi(2);
        let fade = (-above.max(0.0) / curtain.reach).exp();
        // Vertical rays drift along the curtain, and folds brighten and dim.
        let drift = base * 12.0 + time * 0.25;
        let rays = 0.45
            + 0.25 * noise(point.x * 8.0 + drift, curtain.seed + 1)
            + 0.3 * fine * noise(point.x * 26.0 + drift, curtain.seed)
            + 0.15 * (1.0 - fine);
        let folds = 0.55 + 0.45 * (point.x * 2.2 - time * 0.3 + phase).sin();
        let intensity = curtain.strength * hem * fade * rays * folds;

        let along = above.max(0.0) / curtain.reach;
        let color = if along < 0.4 {
            GREEN.lerp(CYAN, along / 0.4)
        } else {
            CYAN.lerp(VIOLET, ((along - 0.4) / 0.8).min(1.0))
        };
        total += color * intensity;
    }
    total * 0.85
}

fn mountain_line(x: f32) -> f32 {
    let peaks = 1.0 - (x * 1.3 + 0.4).sin().abs();
    let smaller = 1.0 - (x * 3.1 + 1.7).sin().abs();
    SHORE + 0.06 + 0.2 * peaks.powf(1.5) + 0.06 * smaller + 0.03 * noise(x * 6.0, 5)
}

// Rolling hills topped with a row of pines.
fn forest_line(x: f32) -> f32 {
    let ground = SHORE + 0.015 + 0.03 * noise(x * 1.5, 9);
    let spacing = 0.036;
    let slot = (x / spacing).floor();
    let mut tallest = ground;
    // A pine can lean into its neighbors' slots, so check them too.
    for neighbor in [slot - 1.0, slot, slot + 1.0] {
        let seed = neighbor as i32;
        let height = 0.05 + 0.09 * hash(seed, 31) * (0.5 + 0.5 * noise(x * 0.9, 13));
        let center = (neighbor + 0.5 + 0.3 * (hash(seed, 37) - 0.5)) * spacing;
        let half_width = 0.3 * height;
        let along = (x - center).abs() / half_width;
        if along < 1.0 {
            tallest = tallest.max(ground + height * (1.0 - along));
        }
    }
    tallest
}

// Stars on a jittered lattice, twinkling, each about a pixel across.
fn stars(point: Vec2, time: f32, pixel: f32) -> f32 {
    let cell = (point / STAR_SPACING).floor();
    let (column, row) = (cell.x as i32, cell.y as i32);
    let seed = column.wrapping_mul(7919) ^ row.wrapping_mul(104_729);
    // Coarse grids show fewer stars, so the sky doesn't fill with dots.
    let density = 0.45 * (STAR_SPACING / (pixel * 4.0)).powi(2).min(1.0);
    if hash(seed, 1) > density {
        return 0.0;
    }
    let center = (cell + Vec2::new(hash(seed, 2), hash(seed, 3))) * STAR_SPACING;
    let distance = (point - center).length() / (pixel * 0.7);
    let twinkle = 0.65 + 0.35 * (time * (1.5 + 2.0 * hash(seed, 4)) + TAU * hash(seed, 5)).sin();
    let fade_near_horizon = ((point.y - SHORE - 0.1) / 0.3).clamp(0.0, 1.0);
    (0.25 + 0.75 * hash(seed, 6).powi(3))
        * twinkle
        * fade_near_horizon
        * (-distance * distance).exp()
}

// Smooth 1D value noise from 0 to 1.
fn noise(x: f32, seed: i32) -> f32 {
    let cell = x.floor();
    let along = x - cell;
    let blend = along * along * (3.0 - 2.0 * along);
    let left = hash(cell as i32, seed);
    let right = hash(cell as i32 + 1, seed);
    left + (right - left) * blend
}

// A pseudo-random number from 0 to 1 for an integer and a seed.
fn hash(value: i32, seed: i32) -> f32 {
    let mut bits =
        (value as u32).wrapping_mul(0x9E37_79B9) ^ (seed as u32).wrapping_mul(0x85EB_CA6B);
    bits ^= bits >> 16;
    bits = bits.wrapping_mul(0x7FEB_352D);
    bits ^= bits >> 15;
    bits = bits.wrapping_mul(0x846C_A68B);
    bits ^= bits >> 16;
    bits as f32 / u32::MAX as f32
}
