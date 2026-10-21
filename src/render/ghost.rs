use std::f64::consts::TAU;

use glam::{Mat2, Vec2, Vec3};

use crate::face::Face;
use crate::sdf::{coverage, smooth_min, smoothstep, trapezoid};

pub const HEAD_CENTER: Vec2 = Vec2::new(0.0, 0.2);
const HEAD_RADIUS: f32 = 0.46;
const HEM_HEIGHT: f32 = -0.56;
// Round lobes hang from the hem, rising and falling by `LOBE_SWING`.
const LOBES: usize = 4;
const LOBE_SWING: f32 = 0.03;
// How much wider the ghost is at the hem than at the head.
const FLARE: f32 = 0.07;
// The outline ring's distance from the ghost and its width.
const GAP: f32 = 0.1;
const RING_WIDTH: f32 = 0.065;
const BODY_TOP: Vec3 = Vec3::ONE;
const BODY_BOTTOM: Vec3 = Vec3::new(0.8, 0.87, 1.0);
/// The corners of a box around the ghost and its ring, in its own space,
/// measured over its most extreme poses: a hard lean and a trailing hem
/// reach far below.
pub const BOUNDS: [Vec2; 2] = [Vec2::new(-0.8, -1.4), Vec2::new(0.8, 0.92)];
/// Dots in the body sit this far apart, in the ghost's own space.
const DOT_SPACING: f32 = 0.085;

/// How the body is held at one moment: bobbing in place, leaning and
/// trailing its hem as it moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    lift: f32,
    lean: f32,
    /// How far the hem trails behind the head.
    trail: f32,
    /// How far the hem stretches down while the ghost rises.
    stretch: f32,
    /// The phase of the waves running along the hem.
    ripple: f32,
}

impl Pose {
    /// The pose at `time` while moving `sideways` and `rising` on screen, in
    /// the ghost's own units per second, `asleep` from 0 to 1.
    pub fn new(time: f64, sideways: f32, rising: f32, asleep: f32) -> Self {
        let bob = phase(time, 3.0);
        let sway = phase(time, 6.0);
        let awake = 1.0 - 0.6 * asleep;
        Self {
            lift: 0.045 * bob.sin() * awake,
            lean: (-0.18 * sideways).clamp(-0.35, 0.35) - 0.05 * sway.cos() * awake,
            trail: (-0.3 * sideways).clamp(-0.45, 0.45) - 0.08 * sway.cos() * awake,
            stretch: 0.025 * bob.cos() * awake + (0.04 * rising).clamp(-0.05, 0.05),
            ripple: phase(time, TAU as f32 / (2.2 + 1.5 * (1.0 - asleep))),
        }
    }

    /// Moves a point of the billboard into the body's frame, where it stands
    /// upright with its head at `HEAD_CENTER`.
    fn place(&self, point: Vec2) -> Vec2 {
        let point = Mat2::from_angle(-self.lean) * (point - Vec2::new(0.0, self.lift));
        // The lower body lags behind the head as the ghost moves.
        let below = (HEAD_CENTER.y - point.y).max(0.0);
        Vec2::new(point.x + self.trail * below * below, point.y)
    }

    /// Signed distance to the ghost's outline: a round head over a skirt
    /// that flares a little and ends in round lobes, which rise and fall in a
    /// wave running along the hem.
    fn silhouette(&self, point: Vec2) -> f32 {
        let head = (point - HEAD_CENTER).length() - HEAD_RADIUS;

        // The skirt's distance is exact outside it too, so the outline ring,
        // which is an offset of this distance, rounds its top corners
        // instead of squaring them off where they meet the head.
        let hem = HEM_HEIGHT - self.stretch;
        let skirt = trapezoid(
            point - Vec2::new(0.0, 0.5 * (HEAD_CENTER.y + hem)),
            HEAD_RADIUS * (1.0 + FLARE),
            HEAD_RADIUS,
            0.5 * (HEAD_CENTER.y - hem),
        );

        let hem_width = HEAD_RADIUS * (1.0 + FLARE);
        let radius = hem_width / LOBES as f32;
        let lobes = (0..LOBES)
            .map(|index| {
                let center = Vec2::new(
                    radius * (2 * index + 1) as f32 - hem_width,
                    hem + LOBE_SWING * (self.ripple - 1.3 * index as f32).sin(),
                );
                point.distance(center) - radius
            })
            .fold(f32::INFINITY, f32::min);
        smooth_min(head.min(skirt), lobes, 0.04)
    }
}

/// The light around the ghost, which shows how it is: the ring's color
/// follows its mood, and its brightness and beat follow its energy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aura {
    pub color: Vec3,
    /// From dim at 0 to bright at 1.
    pub brightness: f32,
    /// Beats per second of the ring's pulse.
    pub beat: f32,
}

/// Light from the ghost at one point: its color, already weighted by how
/// much of the point it covers, and that coverage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glow {
    pub light: Vec3,
    pub cover: f32,
}

impl Glow {
    pub const NONE: Self = Self {
        light: Vec3::ZERO,
        cover: 0.0,
    };

    /// This glow laid over `behind`.
    pub fn over(self, behind: Vec3) -> Vec3 {
        self.light + behind * (1.0 - self.cover)
    }
}

/// The ghost at `point` of its billboard, in its own units, where a pixel
/// spans `pixel` units.
pub fn shade(point: Vec2, pixel: f32, time: f64, pose: &Pose, face: &Face, aura: &Aura) -> Glow {
    let point = pose.place(point);
    let silhouette = pose.silhouette(point);
    let gap = GAP.max(1.5 * pixel);
    let ring_width = RING_WIDTH.max(1.2 * pixel);
    if silhouette > gap + ring_width + pixel {
        return Glow::NONE;
    }

    let shape = silhouette.max(-face.distance(point - HEAD_CENTER));
    let inside = coverage(shape, pixel * 0.5);
    if inside > 0.0 {
        let height =
            ((point.y - HEM_HEIGHT) / (HEAD_CENTER.y + HEAD_RADIUS - HEM_HEIGHT)).clamp(0.0, 1.0);
        let rim = 1.0 - smoothstep(0.0, 0.035_f32.max(1.2 * pixel), -shape);
        // The body is a lattice of dots where the dots are big enough to
        // see, so the ghost reads as a point of light given a shape.
        let dots = smoothstep(2.2, 4.0, DOT_SPACING / pixel);
        let nearest = (point / DOT_SPACING).round() * DOT_SPACING;
        let dot = coverage(point.distance(nearest) - 0.3 * DOT_SPACING, pixel * 0.5);
        let texture = 1.0 - dots * (1.0 - (0.35 + 0.65 * dot));
        let color = BODY_BOTTOM
            .lerp(BODY_TOP, height.sqrt())
            .lerp(aura.color, 0.15);
        let brightness = (0.84 + 0.06 * height + 0.16 * rim) * (0.7 + 0.3 * aura.brightness);
        return Glow {
            light: color * brightness * texture * inside,
            cover: inside,
        };
    }

    let ring = coverage(
        (silhouette - gap - 0.5 * ring_width).abs() - 0.5 * ring_width,
        pixel * 0.5,
    );
    if ring <= 0.0 {
        return Glow::NONE;
    }
    // Light runs around the ring in two waves that pass through each other,
    // and the whole ring pulses with the ghost's beat.
    let around = (point.y + 0.15).atan2(point.x);
    let wave = 0.6 * (3.0 * around - phase(time, 1.0 / 1.7 * TAU as f32)).sin()
        + 0.4 * (5.0 * around + phase(time, 1.0 / 1.1 * TAU as f32)).sin();
    let pulse = 0.85 + 0.15 * phase(time, 1.0 / aura.beat.max(0.1)).sin();
    let strength = ring * (0.62 + 0.38 * wave) * pulse * (0.35 + 0.65 * aura.brightness);
    Glow {
        light: aura.color.lerp(Vec3::ONE, 0.15 * (1.0 + wave) * 0.5) * strength,
        cover: ring * 0.8,
    }
}

/// The angle, from 0 to a full turn, of a cycle `period` seconds long at
/// `time`. It is reduced in double precision so motion stays smooth however
/// long the app runs.
fn phase(time: f64, period: f32) -> f32 {
    (time.rem_euclid(f64::from(period)) / f64::from(period) * TAU) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn aura() -> Aura {
        Aura {
            color: Vec3::new(0.3, 0.8, 1.0),
            brightness: 1.0,
            beat: 1.0,
        }
    }

    #[test]
    fn the_body_covers_its_middle_and_nothing_far_outside() {
        let pose = Pose::new(3.0, 0.0, 0.0, 0.0);
        let middle = shade(Vec2::new(0.0, -0.2), 0.02, 3.0, &pose, &Face::REST, &aura());
        assert_eq!(middle.cover, 1.0);
        let far = shade(Vec2::new(0.0, 1.5), 0.02, 3.0, &pose, &Face::REST, &aura());
        assert_eq!(far, Glow::NONE);
    }

    #[test]
    fn everything_drawn_stays_inside_the_bounds() {
        for (sideways, rising) in [(0.0, 0.0), (3.0, 1.0), (-10.0, -10.0), (10.0, 10.0)] {
            let pose = Pose::new(1.3, sideways, rising, 0.0);
            for column in -75..=75 {
                for row in -75..=75 {
                    let point = Vec2::new(column as f32, row as f32) * 0.02;
                    let outside = point.x < BOUNDS[0].x
                        || point.x > BOUNDS[1].x
                        || point.y < BOUNDS[0].y
                        || point.y > BOUNDS[1].y;
                    if outside {
                        let glow = shade(point, 0.02, 1.3, &pose, &Face::REST, &aura());
                        assert_eq!(glow.cover, 0.0, "{point} at {sideways}");
                    }
                }
            }
        }
    }

    #[test]
    fn moving_sideways_leans_the_head_into_the_motion() {
        let still = Pose::new(0.0, 0.0, 0.0, 0.0);
        let moving = Pose::new(0.0, 2.0, 0.0, 0.0);
        let head_top = |pose: &Pose| {
            (-90..=90)
                .map(|step| step as f32 * 0.01)
                .filter(|x| pose.silhouette(pose.place(Vec2::new(*x, 0.6))) < 0.0)
                .sum::<f32>()
        };
        assert!(head_top(&moving) > head_top(&still));
    }

    #[test]
    fn phases_stay_smooth_far_into_a_long_run() {
        let late = 1.0e7;
        let step = phase(late + 0.01, 3.0) - phase(late, 3.0);
        assert!((step - (TAU as f32) * 0.01 / 3.0).abs() < 1e-3, "{step}");
    }

    #[test]
    fn a_glow_covers_what_is_behind_it_by_its_coverage() {
        let glow = Glow {
            light: Vec3::splat(0.5),
            cover: 0.5,
        };
        assert_eq!(glow.over(Vec3::ONE), Vec3::ONE);
        assert_eq!(Glow::NONE.over(Vec3::ONE), Vec3::ONE);
    }
}
