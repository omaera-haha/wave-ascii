use glam::{Vec2, Vec3};

use crate::sdf::{star, trapezoid};
use crate::world::{Block, Decoration, Lamp, Nook, Occasion, Plant};

/// What a surface is made of, which sets its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Ball,
    Block(u8),
    Brass,
    Lampshade,
    Bulb,
    Pot,
    Stem,
    Leaf,
    Petal,
    Cushion,
    Pumpkin,
    Fir,
    Frosting,
    Candle,
    Ember,
    TreeLight(u8),
}

impl Material {
    /// The surface's color under white light. Colors stay mid to bright so
    /// shapes keep their contrast in plain styles.
    pub fn albedo(self, plant: &Plant) -> Vec3 {
        match self {
            Self::Ball => Vec3::new(1.0, 0.45, 0.35),
            Self::Block(index) => match index % 5 {
                0 => Vec3::new(1.0, 0.4, 0.4),
                1 => Vec3::new(1.0, 0.86, 0.3),
                2 => Vec3::new(0.4, 0.65, 1.0),
                3 => Vec3::new(0.4, 0.9, 0.5),
                _ => Vec3::new(0.8, 0.45, 1.0),
            },
            Self::Brass => Vec3::new(0.8, 0.66, 0.42),
            Self::Lampshade => Vec3::new(0.95, 0.86, 0.66),
            Self::Bulb => Vec3::new(1.0, 0.92, 0.75),
            Self::Pot => Vec3::new(0.86, 0.5, 0.34),
            Self::Stem => Vec3::new(0.42, 0.74, 0.36),
            Self::Leaf => {
                Vec3::new(0.35, 0.82, 0.42).lerp(Vec3::new(0.78, 0.72, 0.32), plant.thirst)
            }
            Self::Petal => Vec3::new(1.0, 0.56, 0.76),
            Self::Cushion => Vec3::new(0.64, 0.57, 0.96),
            Self::Pumpkin => Vec3::new(1.0, 0.5, 0.16),
            Self::Fir => Vec3::new(0.35, 0.78, 0.48),
            Self::Frosting => Vec3::new(1.0, 0.74, 0.83),
            Self::Candle => Vec3::new(0.65, 0.85, 1.0),
            Self::Ember => Vec3::new(1.0, 0.72, 0.24),
            Self::TreeLight(index) => match index % 4 {
                0 => Vec3::new(1.0, 0.4, 0.44),
                1 => Vec3::new(0.45, 0.72, 1.0),
                2 => Vec3::new(1.0, 0.85, 0.35),
                _ => Vec3::new(0.6, 1.0, 0.7),
            },
        }
    }

    pub fn emission(self, time: f64, plant: &Plant) -> Vec3 {
        let strength = match self {
            Self::Ember => 1.4 * flicker(time),
            Self::TreeLight(index) => {
                0.9 + 0.5 * ((time.rem_euclid(3600.0) as f32) * 2.1 + f32::from(index) * 1.7).sin()
            }
            _ => 0.0,
        };
        self.albedo(plant) * strength
    }
}

/// A piece of furniture: a signed distance field inside a bounding sphere,
/// so rays that miss the sphere skip it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Furniture {
    Lamp(Lamp),
    Plant(Plant),
    Nook(Nook),
    Block(Block),
    Decoration(Decoration, f64),
}

impl Furniture {
    pub fn bounds(&self) -> (Vec3, f32) {
        match self {
            Self::Block(block) => (block.position, crate::world::blocks::RADIUS),
            Self::Lamp(lamp) => (lamp.position + Vec3::Y * 0.72, 0.78),
            Self::Plant(plant) => {
                let height = plant.height();
                (plant.position + Vec3::Y * height * 0.5, height * 0.5 + 0.28)
            }
            Self::Nook(nook) => (nook.position + Vec3::Y * 0.08, Nook::RADIUS + 0.06),
            Self::Decoration(decoration, _) => {
                let half = decoration.height() * 0.5;
                (
                    Decoration::POSITION + Vec3::Y * half,
                    (half * half + decoration.radius().powi(2)).sqrt() + 0.02,
                )
            }
        }
    }

    /// Signed distance from `point` to the furniture, and the material
    /// nearest it.
    pub fn distance(&self, point: Vec3) -> (f32, Material) {
        match self {
            Self::Block(block) => {
                let point = block.orientation.conjugate() * (point - block.position);
                let corner = point.abs() - Vec3::splat(crate::world::blocks::EDGE * 0.5 - 0.012);
                (
                    corner.max(Vec3::ZERO).length() + corner.max_element().min(0.0) - 0.012,
                    Material::Block(block.color),
                )
            }
            Self::Decoration(decoration, time) => {
                decoration_distance(point - Decoration::POSITION, *decoration, *time)
            }
            Self::Lamp(lamp) => lamp_distance(point - lamp.position),
            Self::Plant(plant) => plant_distance(point - plant.position, plant),
            Self::Nook(nook) => (
                rounded_cylinder(point - nook.position, Nook::RADIUS, Nook::HEIGHT, 0.06),
                Material::Cushion,
            ),
        }
    }

    pub fn normal(&self, point: Vec3) -> Vec3 {
        // Four samples at the corners of a tetrahedron.
        let step = 0.002;
        let corners = [
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];
        corners
            .iter()
            .map(|corner| *corner * self.distance(point + *corner * step).0)
            .sum::<Vec3>()
            .normalize_or(Vec3::Y)
    }

    /// Where `direction` from `origin` first meets the furniture before
    /// `limit`: the distance along the ray and the material there.
    pub fn trace(&self, origin: Vec3, direction: Vec3, limit: f32) -> Option<(f32, Material)> {
        let (center, radius) = self.bounds();
        let (enter, exit) = sphere_span(origin, direction, center, radius)?;
        let mut distance = enter.max(0.0);
        let end = exit.min(limit);
        for _ in 0..72 {
            if distance > end {
                return None;
            }
            let (gap, material) = self.distance(origin + direction * distance);
            if gap < 0.0015 * distance.max(1.0) {
                return Some((distance, material));
            }
            distance += gap * 0.9;
        }
        None
    }
}

// The lamp from its base: a round foot, a thin pole, and a shade over the
// bulb.
fn lamp_distance(point: Vec3) -> (f32, Material) {
    let foot = rounded_cylinder(point, 0.17, 0.035, 0.012);
    let pole = capsule(point, Vec3::Y * 0.03, Vec3::Y * (Lamp::HEIGHT - 0.06), 0.02);
    let bulb_center = Vec3::Y * (Lamp::HEIGHT - 0.04);
    let shade_point = point - bulb_center - Vec3::Y * 0.06;
    let shade = trapezoid(
        Vec2::new(
            Vec2::new(shade_point.x, shade_point.z).length(),
            shade_point.y,
        ),
        0.2,
        0.1,
        0.1,
    );
    let bulb = point.distance(bulb_center) - 0.06;
    [
        (foot.min(pole), Material::Brass),
        (shade, Material::Lampshade),
        (bulb, Material::Bulb),
    ]
    .into_iter()
    .min_by(|first, second| first.0.total_cmp(&second.0))
    .unwrap_or((f32::INFINITY, Material::Brass))
}

// The plant from the bottom of its pot: a tapered pot, a stem as tall as the
// plant has grown, leaves in turns up the stem that droop when it is thirsty,
// and petals at the top once it blooms.
fn plant_distance(point: Vec3, plant: &Plant) -> (f32, Material) {
    let pot_half = Plant::POT_HEIGHT * 0.5;
    let pot = trapezoid(
        Vec2::new(Vec2::new(point.x, point.z).length(), point.y - pot_half),
        0.15,
        Plant::POT_RADIUS,
        pot_half,
    ) - 0.01;
    let top = Vec3::Y * plant.height();
    let base = Vec3::Y * (Plant::POT_HEIGHT - 0.03);
    let stem = capsule(point, base, top, 0.018);

    let mut nearest = [(pot, Material::Pot), (stem, Material::Stem)]
        .into_iter()
        .min_by(|first, second| first.0.total_cmp(&second.0))
        .unwrap_or((f32::INFINITY, Material::Pot));

    let leaves = 2 + (plant.growth * 5.0) as usize;
    for index in 0..leaves {
        let along = (index as f32 + 1.0) / (leaves as f32 + 1.0);
        let attach = base.lerp(top, along);
        let angle = index as f32 * 2.4;
        let outward = Vec3::new(angle.cos(), 0.0, angle.sin());
        let droop = Vec3::Y * (0.03 - 0.07 * plant.thirst);
        let center = attach + outward * 0.09 + droop;
        // Turn the point into the leaf's own frame, long along `outward`.
        let local = point - center;
        let across = outward.cross(Vec3::Y);
        let leaf_point = Vec3::new(local.dot(outward), local.y, local.dot(across));
        let leaf = ellipsoid(leaf_point, Vec3::new(0.1, 0.022, 0.045));
        if leaf < nearest.0 {
            nearest = (leaf, Material::Leaf);
        }
    }

    if plant.blooming() {
        let bloom = ((plant.growth - Plant::BLOOM) / (1.0 - Plant::BLOOM)).clamp(0.3, 1.0);
        for index in 0..5 {
            let angle = index as f32 * std::f32::consts::TAU / 5.0;
            let center = top + Vec3::new(angle.cos(), 0.15, angle.sin()) * 0.05 * bloom;
            let petal = point.distance(center) - 0.035 * bloom;
            if petal < nearest.0 {
                nearest = (petal, Material::Petal);
            }
        }
    }
    nearest
}

// A hollow ribbed pumpkin, with eyes and a grin cut through its front.
fn pumpkin_distance(point: Vec3) -> (f32, Material) {
    let center = point - Vec3::Y * 0.17;
    let angle = center.x.atan2(center.z);
    let radius = 0.17 + 0.008 * (angle * 10.0).cos();
    let body = ellipsoid(center, Vec3::new(radius, 0.15, radius));
    let inside = ellipsoid(center, Vec3::new(0.14, 0.12, 0.14));
    let eye = trapezoid(
        Vec2::new(point.x.abs() - 0.058, point.y - 0.215),
        0.028,
        0.0,
        0.027,
    );
    let mouth_point = Vec2::new(point.x, (point.y - 0.168) * 1.3);
    let grin = (mouth_point.length() - 0.085).abs() - 0.016;
    let grin = grin.max(point.y - 0.15);
    let tooth = ((point.x.abs() - 0.034).abs() - 0.011).max(0.113 - point.y);
    let grin = grin.max(-tooth);
    let cuts = eye.min(grin).max(0.075 - point.z);
    let shell = body.max(-inside).max(-cuts);
    let stem = capsule(point, Vec3::Y * 0.31, Vec3::new(0.015, 0.355, 0.0), 0.014);
    let ember = ellipsoid(center, Vec3::new(0.126, 0.106, 0.126));
    [
        (shell, Material::Pumpkin),
        (stem, Material::Stem),
        (ember, Material::Ember),
    ]
    .into_iter()
    .min_by(|first, second| first.0.total_cmp(&second.0))
    .unwrap()
}

// Three boughs around a little trunk, with a star and a spiral of lights.
fn tree_distance(point: Vec3) -> (f32, Material) {
    let mut nearest = (rounded_cylinder(point, 0.035, 0.15, 0.008), Material::Brass);
    for (bottom, radius, height) in [(0.08, 0.22, 0.3), (0.25, 0.17, 0.25), (0.4, 0.12, 0.2)] {
        let cone = trapezoid(
            Vec2::new(
                Vec2::new(point.x, point.z).length(),
                point.y - bottom - height * 0.5,
            ),
            radius,
            0.0,
            height * 0.5,
        );
        if cone < nearest.0 {
            nearest = (cone, Material::Fir);
        }
    }
    let crown = star(Vec2::new(point.x, point.y - 0.635), 0.04).max(point.z.abs() - 0.012);
    if crown < nearest.0 {
        nearest = (crown, Material::TreeLight(2));
    }
    for index in 0..16 {
        let height = 0.12 + index as f32 * 0.028;
        let radius = if height < 0.25 {
            0.22 * (0.38 - height) / 0.3
        } else if height < 0.4 {
            0.17 * (0.5 - height) / 0.25
        } else {
            0.12 * (0.6 - height) / 0.2
        };
        let angle = index as f32 * 2.4;
        let center = Vec3::new(
            angle.cos() * (radius + 0.006),
            height,
            angle.sin() * (radius + 0.006),
        );
        let light = point.distance(center) - 0.016;
        if light < nearest.0 {
            nearest = (light, Material::TreeLight(index));
        }
    }
    nearest
}

// A frosted cake with one candle per year, up to five.
fn cake_distance(point: Vec3, candles: u16, time: f64) -> (f32, Material) {
    let cake = rounded_cylinder(point, 0.17, 0.17, 0.025);
    let icing = rounded_cylinder(point - Vec3::Y * 0.145, 0.177, 0.035, 0.012);
    let mut nearest = (cake.min(icing), Material::Frosting);
    for index in 0..candles {
        let angle = f32::from(index) * std::f32::consts::TAU / f32::from(candles);
        let radius = if candles == 1 { 0.0 } else { 0.085 };
        let base = Vec3::new(angle.cos() * radius, 0.175, angle.sin() * radius);
        let candle = rounded_cylinder(point - base, 0.013, 0.09, 0.003);
        if candle < nearest.0 {
            nearest = (candle, Material::Candle);
        }
        let flame = ellipsoid(
            point
                - base
                - Vec3::new(
                    0.003 * (time.rem_euclid(3600.0) as f32 * 7.0 + angle).sin(),
                    0.112,
                    0.0,
                ),
            Vec3::new(0.012, 0.024 * flicker(time), 0.012),
        );
        if flame < nearest.0 {
            nearest = (flame, Material::Ember);
        }
    }
    nearest
}

fn decoration_distance(point: Vec3, decoration: Decoration, time: f64) -> (f32, Material) {
    match decoration.occasion {
        Occasion::Halloween => pumpkin_distance(point),
        Occasion::WinterHolidays | Occasion::NewYear => tree_distance(point),
        Occasion::Birthday => cake_distance(point, decoration.candles, time),
        Occasion::Valentine => (f32::INFINITY, Material::Frosting),
    }
}

fn flicker(time: f64) -> f32 {
    let phase = time.rem_euclid(3600.0) as f32;
    0.86 + 0.08 * (phase * 7.1).sin() + 0.06 * (phase * 11.3 + 0.7).sin()
}

pub fn decoration_light(decoration: Decoration, time: f64) -> Vec3 {
    match decoration.occasion {
        Occasion::Halloween | Occasion::Birthday => {
            Vec3::new(1.0, 0.57, 0.2) * 0.65 * flicker(time)
        }
        Occasion::WinterHolidays | Occasion::NewYear => {
            let phase = time.rem_euclid(3600.0) as f32 * 0.7;
            Vec3::new(0.8, 0.55, 0.65).lerp(Vec3::new(0.45, 0.8, 1.0), 0.5 + 0.5 * phase.sin())
                * 0.4
        }
        Occasion::Valentine => Vec3::ZERO,
    }
}

/// Where a ray meets a sphere: the distances at which it enters and leaves.
pub fn sphere_span(origin: Vec3, direction: Vec3, center: Vec3, radius: f32) -> Option<(f32, f32)> {
    let offset = origin - center;
    let half_b = offset.dot(direction);
    let c = offset.length_squared() - radius * radius;
    let discriminant = half_b * half_b - c;
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let exit = -half_b + root;
    (exit > 0.0).then_some((-half_b - root, exit))
}

pub fn capsule(point: Vec3, start: Vec3, end: Vec3, radius: f32) -> f32 {
    let along = end - start;
    let fraction = ((point - start).dot(along) / along.length_squared()).clamp(0.0, 1.0);
    point.distance(start + along * fraction) - radius
}

/// A vertical cylinder standing on the origin, `height` tall, with its edges
/// rounded by `round`.
pub fn rounded_cylinder(point: Vec3, radius: f32, height: f32, round: f32) -> f32 {
    let across = Vec2::new(point.x, point.z).length() - radius + round;
    let up = (point.y - height * 0.5).abs() - height * 0.5 + round;
    Vec2::new(across, up).max(Vec2::ZERO).length() + across.max(up).min(0.0) - round
}

/// An ellipsoid centered on the origin; the distance is approximate but
/// good enough for marching rays.
pub fn ellipsoid(point: Vec3, radii: Vec3) -> f32 {
    let scaled = (point / radii).length();
    let gradient = (point / (radii * radii)).length();
    if gradient <= 0.0 {
        return -radii.min_element();
    }
    scaled * (scaled - 1.0) / gradient
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ray_through_a_sphere_enters_before_it_leaves() {
        let (enter, exit) =
            sphere_span(Vec3::new(0.0, 0.0, 5.0), -Vec3::Z, Vec3::ZERO, 1.0).unwrap();
        assert!((enter - 4.0).abs() < 1e-5 && (exit - 6.0).abs() < 1e-5);
        assert!(sphere_span(Vec3::new(3.0, 0.0, 5.0), -Vec3::Z, Vec3::ZERO, 1.0).is_none());
        // A sphere wholly behind the ray is missed.
        assert!(sphere_span(Vec3::ZERO, Vec3::Z, Vec3::new(0.0, 0.0, -5.0), 1.0).is_none());
    }

    #[test]
    fn rays_find_each_piece_of_furniture_from_the_front() {
        let pieces = [
            Furniture::Lamp(Lamp {
                position: Vec3::ZERO,
                on: true,
            }),
            Furniture::Plant(Plant {
                position: Vec3::ZERO,
                growth: 1.0,
                thirst: 0.0,
            }),
            Furniture::Nook(Nook {
                position: Vec3::ZERO,
            }),
        ];
        for piece in pieces {
            let (center, _) = piece.bounds();
            let origin = Vec3::new(0.0, center.y.min(0.1), 4.0);
            let hit = piece.trace(origin, -Vec3::Z, 10.0);
            assert!(hit.is_some(), "{piece:?}");
            let (distance, _) = hit.unwrap();
            assert!(distance < 4.0, "{piece:?} {distance}");
        }
    }

    #[test]
    fn the_plant_flowers_only_once_it_blooms() {
        let mut plant = Plant {
            position: Vec3::ZERO,
            growth: 0.5,
            thirst: 0.0,
        };
        let top = Vec3::Y * plant.height();
        let petal_at = |plant: &Plant| plant_distance(top + Vec3::new(0.05, 0.01, 0.0), plant).1;
        assert_ne!(petal_at(&plant), Material::Petal);
        plant.growth = 1.0;
        let top = Vec3::Y * plant.height();
        assert_eq!(
            plant_distance(top + Vec3::new(0.05, 0.007, 0.0), &plant).1,
            Material::Petal
        );
    }

    #[test]
    fn simple_shapes_measure_zero_on_their_surfaces() {
        assert!(rounded_cylinder(Vec3::new(0.0, 0.14, 0.0), 0.42, 0.14, 0.06).abs() < 1e-5);
        assert!(ellipsoid(Vec3::new(0.1, 0.0, 0.0), Vec3::new(0.1, 0.02, 0.05)).abs() < 1e-5);
        assert!(capsule(Vec3::new(0.02, 0.5, 0.0), Vec3::ZERO, Vec3::Y, 0.02).abs() < 1e-5);
    }
}
