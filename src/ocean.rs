use glam::{Mat3, Vec2, Vec3};

use crate::frame::{Frame, Grid, Shade};

// Wave crests reach this high above the troughs at y = 0.
const WAVE_HEIGHT: f32 = 0.55;
const CAMERA_HEIGHT: f32 = 1.7;
// Rays that travel this far without meeting a wave see the horizon haze.
const FAR: f32 = 160.0;
const MARCH_STEPS: usize = 28;
const REFINE_STEPS: usize = 6;
const SUN_COLOR: Vec3 = Vec3::new(1.0, 0.72, 0.42);

/// One train of waves: direction, spatial frequency, and amplitude.
struct Wave {
    direction: Vec2,
    frequency: f32,
    amplitude: f32,
}

// Each octave is smaller, faster, and turned away from the last.
const WAVES: [Wave; 6] = [
    Wave::new(0.8, 0.6, 0.55, 1.0),
    Wave::new(-0.43, 0.9, 0.96, 0.55),
    Wave::new(0.96, -0.28, 1.68, 0.3),
    Wave::new(-0.71, -0.71, 2.95, 0.17),
    Wave::new(0.24, 0.97, 5.16, 0.09),
    Wave::new(-0.99, 0.12, 9.03, 0.05),
];

impl Wave {
    const fn new(x: f32, z: f32, frequency: f32, amplitude: f32) -> Self {
        Self {
            direction: Vec2::new(x, z),
            frequency,
            amplitude,
        }
    }
}

/// A sunset over an open sea, ray marched against a sum of sharp-crested
/// waves. `yaw` and `pitch` turn the camera; `zoom` narrows the lens.
pub fn render(grid: Grid, time: f32, yaw: f32, pitch: f32, zoom: f32) -> Frame {
    let camera = Vec3::new(0.0, CAMERA_HEIGHT + 0.06 * (time * 0.7).sin(), -1.1 * time);
    let orientation = Mat3::from_rotation_y(-yaw) * Mat3::from_rotation_x(pitch - 0.12);
    let sun = Vec3::new(0.28, 0.1, -1.0).normalize();
    let focal_length = 1.4 * zoom;

    Frame::sample(grid, |point| {
        let direction = orientation * Vec3::new(point.x, point.y, -focal_length).normalize();
        let light = trace(camera, direction, sun, time);
        // Exposure curve: highlights roll off instead of clipping.
        Shade::from_rgb(Vec3::ONE - (-1.6 * light).exp())
    })
}

/// Height of the surface, from 0 to `WAVE_HEIGHT`, using the first
/// `octaves` waves.
fn height(position: Vec2, time: f32, octaves: usize) -> f32 {
    let mut total = 0.0;
    let mut weight = 0.0;
    for wave in &WAVES[..octaves] {
        // Deep-water waves travel at a speed proportional to the square root
        // of their wavelength.
        let phase = wave.direction.dot(position) * wave.frequency
            + time * (2.0 * wave.frequency).sqrt() * 1.6;
        // exp(sin - 1) gives narrow crests and broad troughs.
        total += (phase.sin() - 1.0).exp() * wave.amplitude;
        weight += wave.amplitude;
    }
    total / weight * WAVE_HEIGHT
}

fn trace(origin: Vec3, direction: Vec3, sun: Vec3, time: f32) -> Vec3 {
    if direction.y > -1e-3 {
        return sky(direction, sun);
    }
    // Every crest lies between y = WAVE_HEIGHT and y = 0.
    let start = (origin.y - WAVE_HEIGHT) / -direction.y;
    let end = (origin.y / -direction.y).min(FAR);
    let gap = |distance: f32| {
        let point = origin + direction * distance;
        point.y - height(Vec2::new(point.x, point.z), time, 4)
    };

    let mut above = start;
    let mut hit = None;
    for step in 1..=MARCH_STEPS {
        let distance = start + (end - start) * step as f32 / MARCH_STEPS as f32;
        if gap(distance) < 0.0 {
            let mut below = distance;
            for _ in 0..REFINE_STEPS {
                let middle = (above + below) * 0.5;
                if gap(middle) < 0.0 {
                    below = middle;
                } else {
                    above = middle;
                }
            }
            hit = Some((above + below) * 0.5);
            break;
        }
        above = distance;
    }
    let horizon = sky(Vec3::new(direction.x, 0.0, direction.z).normalize(), sun);
    let Some(distance) = hit else {
        return horizon;
    };

    let point = origin + direction * distance;
    let surface = Vec2::new(point.x, point.z);
    // Wider differences far away keep distant normals from shimmering.
    let step = 0.02 + distance * 0.004;
    let center = height(surface, time, WAVES.len());
    let normal = Vec3::new(
        center - height(surface + Vec2::X * step, time, WAVES.len()),
        step,
        center - height(surface + Vec2::Y * step, time, WAVES.len()),
    )
    .normalize();

    let facing = (-direction).dot(normal).max(0.0);
    let fresnel = 0.04 + 0.96 * (1.0 - facing).powi(5);
    let mut reflected = direction - 2.0 * direction.dot(normal) * normal;
    reflected.y = reflected.y.abs();

    let crest = center / WAVE_HEIGHT;
    let water = Vec3::new(0.01, 0.04, 0.07) + Vec3::new(0.06, 0.26, 0.27) * crest.powi(2);
    let glitter = SUN_COLOR * 4.0 * reflected.dot(sun).max(0.0).powi(220);
    let foam = ((crest - 0.84) / 0.16).clamp(0.0, 1.0);
    let color = water.lerp(sky(reflected, sun), fresnel) + glitter;
    let color = color.lerp(Vec3::new(0.85, 0.8, 0.78), foam * 0.45);
    let haze = 1.0 - (-distance * 0.018).exp();
    color.lerp(horizon, haze)
}

fn sky(direction: Vec3, sun: Vec3) -> Vec3 {
    let zenith = Vec3::new(0.025, 0.03, 0.11);
    let horizon = Vec3::new(0.9, 0.4, 0.28);
    let base = horizon.lerp(zenith, (direction.y * 3.0).clamp(0.0, 1.0).powf(0.6));
    let alignment = direction.dot(sun).max(0.0);
    let glow = SUN_COLOR * (0.3 * alignment.powi(16) + 0.6 * alignment.powi(90));
    let disk = if alignment > 0.9995 { 3.0 } else { 0.0 };
    base + glow + SUN_COLOR * disk
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_surface_stays_between_the_troughs_and_crests() {
        for step in 0..200 {
            let position = Vec2::new(step as f32 * 0.37, step as f32 * -0.91);
            let height = height(position, step as f32 * 0.1, WAVES.len());
            assert!((0.0..=WAVE_HEIGHT + 1e-5).contains(&height));
        }
    }

    #[test]
    fn rays_above_the_horizon_see_sky_and_rays_below_see_water() {
        let sun = Vec3::new(0.28, 0.1, -1.0).normalize();
        let origin = Vec3::new(0.0, CAMERA_HEIGHT, 0.0);
        let up = Vec3::new(0.0, 0.5, -1.0).normalize();
        assert_eq!(trace(origin, up, sun, 0.0), sky(up, sun));
        let down = Vec3::new(0.0, -0.6, -1.0).normalize();
        assert!(trace(origin, down, sun, 0.0).max_element() < sky(down, sun).max_element());
    }
}
