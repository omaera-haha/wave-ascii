use std::f64::consts::TAU;

use glam::{Mat2, Vec2, Vec3};

use crate::face::Face;
use crate::sdf::{capsule, coverage, smooth_min, smoothstep, star, trapezoid};
use crate::world::{Accessory, Plant};

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
/// The corners of a box around the ghost, accessories, and ring in local space,
/// measured over its most extreme poses: at full speed the hem streams out
/// far to one side.
pub const BOUNDS: [Vec2; 2] = [Vec2::new(-1.3, -1.05), Vec2::new(1.3, 1.35)];
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
        // The head tips into the motion and the hem streams out behind it,
        // so `trail` bends the lower body the opposite way to `lean`. The
        // idle sway rocks the body the same way, gently.
        let rock = 0.05 * sway.cos() * awake;
        Self {
            lift: 0.045 * bob.sin() * awake,
            lean: (-0.16 * sideways).clamp(-0.25, 0.25) - rock,
            trail: (0.28 * sideways).clamp(-0.36, 0.36) + 1.6 * rock,
            stretch: 0.025 * bob.cos() * awake + (0.04 * rising).clamp(-0.05, 0.05),
            ripple: phase(time, TAU as f32 / (2.2 + 1.5 * (1.0 - asleep))),
        }
    }

    pub fn in_breeze(mut self, sideways: f32, shake: f32) -> Self {
        self.trail = (self.trail - sideways * 0.6).clamp(-0.44, 0.44);
        self.lean += shake * 0.08;
        self
    }

    pub fn with_quirk(mut self, spell: Option<crate::world::quirks::Spell>, time: f64) -> Self {
        if let Some(spell) = spell {
            let motion = spell.motion(time);
            self.lift += motion.lift;
            self.lean += motion.lean;
            self.stretch += motion.stretch;
        }
        self
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
    /// How much of the ghost remains visible while hiding, from 0 to 1.
    pub visibility: f32,
    /// From dim at 0 to bright at 1.
    pub brightness: f32,
    /// Beats per second of the ring's pulse.
    pub beat: f32,
}

/// A camera-facing body, shared by the resident and its smaller visitor.
#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub position: Vec3,
    pub scale: f32,
    pub pose: Pose,
    pub face: Face,
    pub aura: Aura,
    pub opacity: f32,
    pub accessory: Option<Accessory>,
}

/// A soft mint-to-lavender glow, distinct from the resident's mood palette.
pub fn friend_color(hue: f32) -> Vec3 {
    Vec3::new(0.55, 1.0, 0.78).lerp(Vec3::new(0.88, 0.68, 1.0), hue.clamp(0.0, 1.0))
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
pub fn shade(point: Vec2, pixel: f32, time: f64, body: &Body) -> Glow {
    let behind = body_shade(point, pixel, time, body);
    let accessory = body
        .accessory
        .map(|accessory| accessory_shade(body.pose.place(point), pixel, accessory, body.aura))
        .unwrap_or(Glow::NONE);
    let glow = Glow {
        light: accessory.over(behind.light),
        cover: accessory.cover + behind.cover * (1.0 - accessory.cover),
    };
    Glow {
        light: glow.light * body.opacity,
        cover: glow.cover * body.opacity,
    }
}

fn body_shade(point: Vec2, pixel: f32, time: f64, body: &Body) -> Glow {
    let Body {
        pose, face, aura, ..
    } = body;
    let point = pose.place(point);
    let silhouette = pose.silhouette(point);
    let gap = GAP.max(1.5 * pixel);
    let ring_width = RING_WIDTH.max(1.2 * pixel);
    if silhouette > gap + ring_width + pixel {
        return Glow::NONE;
    }

    let visibility = aura.visibility.clamp(0.0, 1.0);
    let features = face.distance(point - HEAD_CENTER);
    let eyes =
        coverage(features, pixel * 0.5) * coverage(silhouette, pixel * 0.5) * (1.0 - visibility)
            / 3.0;
    let eye_light = aura.color.lerp(Vec3::ONE, 0.7) * eyes;
    let shape = silhouette.max(-features);
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
            light: color * brightness * texture * inside * visibility + eye_light,
            cover: inside * visibility + eyes,
        };
    }
    if eyes > 0.0 {
        return Glow {
            light: eye_light,
            cover: eyes,
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
        light: aura.color.lerp(Vec3::ONE, 0.15 * (1.0 + wave) * 0.5) * strength * visibility,
        cover: ring * 0.8 * visibility,
    }
}

// Distances are in the posed body's space, so dangling parts follow its hem.
fn accessory_shade(point: Vec2, pixel: f32, accessory: Accessory, aura: Aura) -> Glow {
    let (center, radius) = match accessory {
        Accessory::Flower(_) | Accessory::Feather | Accessory::Holly => {
            (Vec2::new(0.37, 0.49), 0.27)
        }
        Accessory::PartyHat | Accessory::WitchHat => (Vec2::new(0.0, 0.8), 0.37),
        Accessory::Scarf { .. } => (Vec2::new(0.0, -0.08), 0.64),
        Accessory::LuckyPouch => (Vec2::new(0.48, -0.25), 0.29),
    };
    if aura.visibility <= 0.0 || point.distance_squared(center) > (radius + pixel).powi(2) {
        return Glow::NONE;
    }
    let local = point - center;
    let (distance, color) = match accessory {
        Accessory::Flower(kind) => {
            let petals = (0..5)
                .map(|index| {
                    let petal =
                        Vec2::from_angle(index as f32 * std::f32::consts::TAU / 5.0) * 0.073;
                    local.distance(petal) - 0.058
                })
                .fold(f32::INFINITY, f32::min);
            let center = local.length() - 0.042;
            let color = if center < 0.0 {
                super::shapes::Material::Center(kind).albedo(&Plant::default())
            } else {
                super::shapes::Material::Flower(kind).albedo(&Plant::default())
            };
            (petals.min(center), color)
        }
        Accessory::PartyHat | Accessory::WitchHat => {
            let cone = trapezoid(local - Vec2::Y * 0.025, 0.22, 0.012, 0.245);
            let brim = capsule(local, Vec2::new(-0.28, -0.2), Vec2::new(0.28, -0.2), 0.026);
            let band = local.y < -0.11 && local.y > -0.17;
            let color = if accessory == Accessory::PartyHat {
                if star(local - Vec2::new(0.0, -0.06), 0.055) < 0.0 {
                    Vec3::new(1.0, 0.95, 0.5)
                } else {
                    Vec3::new(1.0, 0.57, 0.72)
                }
            } else if band {
                Vec3::new(1.0, 0.67, 0.3)
            } else {
                Vec3::new(0.58, 0.48, 0.85)
            };
            let pom = if accessory == Accessory::PartyHat {
                (local - Vec2::Y * 0.29).length() - 0.045
            } else {
                f32::INFINITY
            };
            (cone.min(brim).min(pom), color)
        }
        Accessory::Scarf { color } => {
            let collar = capsule(local, Vec2::new(-0.4, 0.03), Vec2::new(0.4, 0.03), 0.073);
            let first = capsule(local, Vec2::new(0.24, 0.03), Vec2::new(0.34, -0.38), 0.059);
            let second = capsule(local, Vec2::new(0.07, 0.01), Vec2::new(0.12, -0.3), 0.052);
            let palette = [
                Vec3::new(1.0, 0.48, 0.36),
                Vec3::new(1.0, 0.72, 0.36),
                Vec3::new(0.95, 0.48, 0.7),
            ];
            let knit = 0.9 + 0.1 * (local.y * 140.0 + (local.x * 100.0).sin()).cos();
            (
                collar.min(first).min(second),
                palette[usize::from(color).min(2)] * knit,
            )
        }
        Accessory::Feather => {
            let turned = Mat2::from_angle(0.35) * local;
            let vane = capsule(turned, Vec2::new(0.0, -0.035), Vec2::new(0.0, 0.13), 0.065);
            let shaft = capsule(turned, Vec2::new(0.0, -0.2), Vec2::new(0.0, 0.18), 0.012);
            let color = if turned.x.abs() < 0.012 {
                Vec3::new(1.0, 0.94, 0.76)
            } else {
                Vec3::new(0.72, 0.86, 1.0)
            };
            (vane.min(shaft), color)
        }
        Accessory::LuckyPouch => {
            let bag = (local - Vec2::new(0.02, -0.12)).length() - 0.105;
            let neck = trapezoid(local - Vec2::new(0.02, -0.035), 0.04, 0.075, 0.045);
            let cord = capsule(local, Vec2::new(-0.08, 0.22), Vec2::new(0.02, -0.04), 0.012);
            let knot = star(local - Vec2::new(0.02, -0.04), 0.05);
            let color = if cord < bag.min(neck) || knot < 0.0 {
                Vec3::new(1.0, 0.86, 0.4)
            } else {
                Vec3::new(1.0, 0.43, 0.55)
            };
            (bag.min(neck).min(cord).min(knot), color)
        }
        Accessory::Holly => {
            let first = star(
                Mat2::from_angle(-0.5) * (local - Vec2::new(-0.06, 0.045)),
                0.13,
            );
            let second = star(
                Mat2::from_angle(0.6) * (local - Vec2::new(0.06, 0.025)),
                0.12,
            );
            let berries = [
                Vec2::new(-0.035, -0.025),
                Vec2::new(0.035, -0.025),
                Vec2::new(0.0, -0.075),
            ]
            .into_iter()
            .map(|center| local.distance(center) - 0.037)
            .fold(f32::INFINITY, f32::min);
            let color = if berries < 0.0 {
                Vec3::new(1.0, 0.4, 0.48)
            } else {
                Vec3::new(0.4, 0.88, 0.57)
            };
            (first.min(second).min(berries), color)
        }
    };
    let cover = coverage(distance, pixel * 0.5) * aura.visibility.clamp(0.0, 1.0);
    Glow {
        light: color * (0.8 + 0.2 * aura.brightness) * cover,
        cover,
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

    fn shade(point: Vec2, pixel: f32, time: f64, pose: &Pose, face: &Face, aura: &Aura) -> Glow {
        super::shade(
            point,
            pixel,
            time,
            &Body {
                position: Vec3::ZERO,
                scale: 1.0,
                pose: *pose,
                face: *face,
                aura: *aura,
                opacity: 1.0,
                accessory: None,
            },
        )
    }

    fn aura() -> Aura {
        Aura {
            color: Vec3::new(0.3, 0.8, 1.0),
            visibility: 1.0,
            brightness: 1.0,
            beat: 1.0,
        }
    }

    #[test]
    fn accessories_remain_finite_in_tiny_bodies_extreme_poses_and_full_fade() {
        use crate::world::Flower;
        for accessory in [
            Accessory::Flower(Flower::Daisy),
            Accessory::Flower(Flower::Tulip),
            Accessory::Flower(Flower::Sunflower),
            Accessory::Flower(Flower::Bluebell),
            Accessory::Flower(Flower::Poppy),
            Accessory::Scarf { color: 0 },
            Accessory::Scarf { color: 2 },
            Accessory::PartyHat,
            Accessory::WitchHat,
            Accessory::Feather,
            Accessory::LuckyPouch,
            Accessory::Holly,
        ] {
            for scale in [0.001, 0.1, 1.0] {
                for velocity in [-100.0, 0.0, 100.0] {
                    for visibility in [0.0, 0.3, 1.0] {
                        let body = Body {
                            position: Vec3::ZERO,
                            scale,
                            pose: Pose::new(1e12, velocity, velocity, 0.0)
                                .in_breeze(velocity.signum(), 1.0),
                            face: Face::REST,
                            aura: Aura {
                                visibility,
                                ..aura()
                            },
                            opacity: 1.0,
                            accessory: Some(accessory),
                        };
                        for column in -16..=16 {
                            for row in -16..=16 {
                                let point = Vec2::new(column as f32, row as f32) * 0.1;
                                let glow = super::shade(point, 0.02 / scale, 1e12, &body);
                                assert!(glow.light.is_finite() && glow.cover.is_finite());
                                assert!((0.0..=1.0).contains(&glow.cover));
                                let worn = accessory_shade(
                                    body.pose.place(point),
                                    0.02 / scale,
                                    accessory,
                                    body.aura,
                                );
                                assert!(worn.light.is_finite() && worn.cover.is_finite());
                                if visibility == 0.0 {
                                    assert_eq!(worn, Glow::NONE);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn accessory_distances_are_bounded_before_the_drawing_math() {
        for accessory in [
            Accessory::Flower(crate::world::Flower::Daisy),
            Accessory::Scarf { color: 0 },
            Accessory::PartyHat,
            Accessory::WitchHat,
            Accessory::Feather,
            Accessory::LuckyPouch,
            Accessory::Holly,
        ] {
            for direction in [Vec2::X, -Vec2::X, Vec2::Y, -Vec2::Y] {
                assert_eq!(
                    accessory_shade(direction * 10.0, 0.02, accessory, aura()),
                    Glow::NONE
                );
            }
            for velocity in [-10.0, 0.0, 10.0] {
                let pose =
                    Pose::new(1.3, velocity, velocity, 0.0).in_breeze(velocity.signum(), 1.0);
                for column in -40..=40 {
                    for row in -40..=40 {
                        let point = Vec2::new(column as f32, row as f32) * 0.04;
                        if point.x < BOUNDS[0].x
                            || point.x > BOUNDS[1].x
                            || point.y < BOUNDS[0].y
                            || point.y > BOUNDS[1].y
                        {
                            assert_eq!(
                                accessory_shade(pose.place(point), 0.02, accessory, aura()).cover,
                                0.0
                            );
                        }
                    }
                }
            }
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
    fn extreme_wind_and_head_shakes_keep_the_pose_math_finite() {
        for wind in [-1.0, 0.0, 1.0] {
            for shake in [-1.0, 0.0, 1.0] {
                let pose = Pose::new(1e12, 4.0, -4.0, 0.0).in_breeze(wind, shake);
                for column in -20..=20 {
                    for row in -15..=15 {
                        let local = pose.place(Vec2::new(column as f32 * 0.1, row as f32 * 0.1));
                        assert!(local.is_finite() && pose.silhouette(local).is_finite());
                    }
                }
            }
        }
    }

    #[test]
    fn everything_drawn_stays_inside_the_bounds() {
        for (sideways, rising) in [(0.0, 0.0), (3.0, 1.0), (-10.0, -10.0), (10.0, 10.0)] {
            let pose = Pose::new(1.3, sideways, rising, 0.0);
            for column in -85..=85 {
                for row in -85..=85 {
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
    fn moving_leans_the_head_forward_and_streams_the_hem_behind() {
        // The middle of the body across one height of the billboard.
        let middle = |pose: &Pose, height: f32| {
            let inside: Vec<f32> = (-150..=150)
                .map(|step| step as f32 * 0.01)
                .filter(|x| pose.silhouette(pose.place(Vec2::new(*x, height))) < 0.0)
                .collect();
            inside.iter().sum::<f32>() / inside.len() as f32
        };
        // Bobbing and swaying are left out at a quarter of the sway's cycle.
        let time = 1.5;
        let still = Pose::new(time, 0.0, 0.0, 0.0);
        for sideways in [2.0, -2.0] {
            let moving = Pose::new(time, sideways, 0.0, 0.0);
            let head = middle(&moving, 0.55) - middle(&still, 0.55);
            let hem = middle(&moving, -0.5) - middle(&still, -0.5);
            assert!(head * sideways > 0.0, "{sideways}: head {head}");
            assert!(hem * sideways < 0.0, "{sideways}: hem {hem}");
        }
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

    #[test]
    fn visibility_reduces_the_bodys_coverage_and_light() {
        let pose = Pose::new(3.0, 0.0, 0.0, 0.0);
        let visible = aura();
        let hidden = Aura {
            visibility: 0.0,
            ..visible
        };
        let middle = Vec2::new(0.0, -0.2);
        let full = shade(middle, 0.02, 3.0, &pose, &Face::REST, &visible);
        let faint = shade(middle, 0.02, 3.0, &pose, &Face::REST, &hidden);
        assert!(faint.cover < full.cover * 0.2);
        assert!(faint.light.length() < full.light.length() * 0.2);
    }
}
