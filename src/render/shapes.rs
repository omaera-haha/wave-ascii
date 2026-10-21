use glam::{Vec2, Vec3};

use crate::sdf::trapezoid;
use crate::world::{Lamp, Nook, Plant};

/// What a surface is made of, which sets its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Material {
    Ball,
    Brass,
    Lampshade,
    Bulb,
    Pot,
    Stem,
    Leaf,
    Petal,
    Cushion,
}

impl Material {
    /// The surface's color under white light. Colors stay mid to bright so
    /// shapes keep their contrast in plain styles.
    pub fn albedo(self, plant: &Plant) -> Vec3 {
        match self {
            Self::Ball => Vec3::new(1.0, 0.45, 0.35),
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
        }
    }
}

/// A piece of furniture: a signed distance field inside a bounding sphere,
/// so rays that miss the sphere skip it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Furniture {
    Lamp(Lamp),
    Plant(Plant),
    Nook(Nook),
}

impl Furniture {
    pub fn bounds(&self) -> (Vec3, f32) {
        match self {
            Self::Lamp(lamp) => (lamp.position + Vec3::Y * 0.72, 0.78),
            Self::Plant(plant) => {
                let height = plant.height();
                (plant.position + Vec3::Y * height * 0.5, height * 0.5 + 0.28)
            }
            Self::Nook(nook) => (nook.position + Vec3::Y * 0.08, Nook::RADIUS + 0.06),
        }
    }

    /// Signed distance from `point` to the furniture, and the material
    /// nearest it.
    pub fn distance(&self, point: Vec3) -> (f32, Material) {
        match self {
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
