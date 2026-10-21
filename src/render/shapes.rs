use glam::{Quat, Vec2, Vec3};

use crate::sdf::{star, trapezoid};
use crate::world::{
    Block, Decoration, Flower, Keepsake, Lamp, Nook, Occasion, Petal, Plant, SHELF_POSITION,
    Season, Seedling,
};

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
    Wood,
    Quill,
    Feather,
    Acorn,
    AcornCap,
    AutumnLeaf,
    Berry,
    Pebble,
    Porcelain,
    RiceCake(u8),
    Silk,
    Steam,
    Soil,
    Flower(Flower),
    Center(Flower),
    Seed,
    Curtain,
    Water,
    BookCover(u8),
    BookPages,
    MusicNote(u8),
}

impl Material {
    /// The surface's color under white light. Colors stay mid to bright so
    /// shapes keep their contrast in plain styles.
    pub fn albedo(self, plant: &Plant) -> Vec3 {
        match self {
            Self::BookCover(index) => match index % 5 {
                0 => Vec3::new(0.62, 0.78, 0.9),
                1 => Vec3::new(0.9, 0.68, 0.72),
                2 => Vec3::new(0.7, 0.84, 0.6),
                3 => Vec3::new(0.86, 0.73, 0.95),
                _ => Vec3::new(0.98, 0.82, 0.56),
            },
            Self::BookPages => Vec3::new(1.0, 0.96, 0.84),
            Self::MusicNote(fade) => Vec3::new(1.0, 0.78, 0.3) * (f32::from(fade) / 255.0),
            Self::Curtain => Vec3::new(0.95, 0.9, 0.8),
            Self::Water => Vec3::new(0.46, 0.65, 0.78),
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
            Self::Soil => Vec3::new(0.49, 0.36, 0.28),
            Self::Flower(kind) => match kind {
                Flower::Daisy => Vec3::new(1.0, 0.97, 0.85),
                Flower::Tulip => Vec3::new(1.0, 0.32, 0.42),
                Flower::Sunflower => Vec3::new(1.0, 0.85, 0.24),
                Flower::Bluebell => Vec3::new(0.46, 0.57, 1.0),
                Flower::Poppy => Vec3::new(1.0, 0.42, 0.27),
            },
            Self::Center(Flower::Daisy) => Vec3::new(1.0, 0.8, 0.25),
            Self::Center(Flower::Poppy) => Vec3::new(0.38, 0.3, 0.42),
            Self::Center(_) => Vec3::new(0.58, 0.39, 0.27),
            Self::Seed => Vec3::new(1.0, 0.85, 0.5),
            Self::Cushion => Vec3::new(0.64, 0.57, 0.96),
            Self::Pumpkin => Vec3::new(1.0, 0.5, 0.16),
            Self::Fir => Vec3::new(0.35, 0.78, 0.48),
            Self::Frosting => Vec3::new(1.0, 0.74, 0.83),
            Self::Candle => Vec3::new(0.65, 0.85, 1.0),
            Self::Ember => Vec3::new(1.0, 0.72, 0.24),
            Self::Wood => Vec3::new(0.8, 0.57, 0.36),
            Self::Quill => Vec3::new(1.0, 0.91, 0.7),
            Self::Feather => Vec3::new(0.72, 0.86, 1.0),
            Self::Acorn => Vec3::new(0.85, 0.58, 0.32),
            Self::AcornCap => Vec3::new(0.58, 0.43, 0.3),
            Self::AutumnLeaf => Vec3::new(1.0, 0.42, 0.2),
            Self::Berry => Vec3::new(1.0, 0.3, 0.38),
            Self::Pebble => Vec3::new(0.7, 0.76, 0.84),
            Self::Porcelain => Vec3::new(0.95, 0.95, 0.9),
            Self::RiceCake(index) => match index % 3 {
                0 => Vec3::new(1.0, 0.7, 0.8),
                1 => Vec3::new(0.65, 0.88, 0.6),
                _ => Vec3::new(1.0, 0.96, 0.87),
            },
            Self::Silk => Vec3::new(0.95, 0.3, 0.38),
            Self::Steam => Vec3::new(0.68, 0.75, 0.8),
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
            Self::Seed => 0.5,
            Self::MusicNote(_) => 1.4,
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
pub enum Furniture<'a> {
    Lamp(Lamp),
    Plant(Plant),
    Nook(Nook),
    Block(Block),
    Decoration(Decoration, f64),
    Shelf(&'a [Keepsake], Option<Keepsake>),
    Keepsake(Keepsake, Vec3),
    Seedling(&'a Seedling, Vec3, Season),
    Petal(Petal, f64),
    Seed(Vec3),
    Curtain(f32, f32),
    Puddle(f32),
    Leaf(crate::world::Leaf, f64),
    SwayingPlant(Plant, Vec3),
    Books(&'a [crate::world::books::OwnedBook], Option<usize>),
    OpenBook {
        index: usize,
        position: Vec3,
        size: f32,
        rotation: Quat,
        turning: f32,
    },
    MusicBox(bool, f32),
    MusicNote(Vec3, bool, u8),
}

impl Furniture<'_> {
    pub fn bounds(&self) -> (Vec3, f32) {
        match self {
            Self::Books(_, _) => (crate::world::books::POSITION + Vec3::Y * 0.09, 0.29),
            Self::OpenBook { position, size, .. } => (*position, 0.29 * size),
            Self::MusicBox(_, _) => (crate::world::music::POSITION + Vec3::Y * 0.19, 0.4),
            Self::MusicNote(position, _, _) => (*position + Vec3::Y * 0.04, 0.14),
            Self::Curtain(side, _) => (
                Vec3::new(side * 0.88, 1.5, crate::world::ROOM_MIN.z + 0.14),
                0.68,
            ),
            Self::Puddle(amount) => (
                Vec3::new(0.0, 0.008, crate::world::ROOM_MIN.z + 0.4),
                0.52 * amount.sqrt().max(0.01),
            ),
            Self::Leaf(leaf, _) => (leaf.position, 0.065),
            Self::Seedling(plant, position, _) => {
                let height = plant.height();
                (*position + Vec3::Y * height * 0.5, height * 0.5 + 0.14)
            }
            Self::Petal(petal, _) => (petal.position, 0.035),
            Self::Seed(position) => (*position, 0.025),
            Self::Block(block) => (block.position, crate::world::blocks::RADIUS),
            Self::Lamp(lamp) => (lamp.position + Vec3::Y * 0.72, 0.78),
            Self::Plant(plant) | Self::SwayingPlant(plant, _) => {
                let height = plant.height();
                (plant.position + Vec3::Y * height * 0.5, height * 0.5 + 0.28)
            }
            Self::Nook(nook) => (nook.position + Vec3::Y * 0.08, Nook::RADIUS + 0.06),
            Self::Shelf(..) => (SHELF_POSITION, 0.58),
            Self::Keepsake(_, position) => (*position + Vec3::Y * 0.08, 0.14),
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
            Self::Books(owned, held) => {
                let point = point - crate::world::books::POSITION;
                let mut nearest = (f32::MAX, Material::BookPages);
                for (level, book) in owned
                    .iter()
                    .filter(|book| Some(book.index) != *held)
                    .take(5)
                    .enumerate()
                {
                    let local = point - Vec3::Y * (0.018 + level as f32 * 0.035);
                    let bound = local.length() - 0.23;
                    if bound > nearest.0 {
                        continue;
                    }
                    let local = Quat::from_rotation_y((level as f32 * 1.7).sin() * 0.13) * local;
                    let cover = box_distance(local, Vec3::new(0.18, 0.016, 0.12));
                    let pages = box_distance(local, Vec3::new(0.174, 0.011, 0.121));
                    let surface = if pages < cover {
                        (pages, Material::BookPages)
                    } else {
                        (cover, Material::BookCover(book.index as u8))
                    };
                    if surface.0 < nearest.0 {
                        nearest = surface;
                    }
                }
                nearest
            }
            Self::OpenBook {
                index,
                position,
                size,
                rotation,
                turning,
            } => {
                let local = rotation.conjugate() * (point - *position) / *size;
                let (distance, material) = open_book_distance(local, *index, *turning);
                (distance * size, material)
            }
            Self::MusicBox(open, crank) => {
                music_box_distance(point - crate::world::music::POSITION, *open, *crank)
            }
            Self::MusicNote(position, flagged, fade) => {
                let fade_scale = (f32::from(*fade) / 255.0).max(0.001);
                let local = (point - *position) / fade_scale;
                let corner = (local - Vec3::Y * 0.04).abs() - Vec3::new(0.08, 0.11, 0.03);
                if corner.max_element() > 0.0 {
                    return (
                        corner.max(Vec3::ZERO).length() * fade_scale,
                        Material::MusicNote(*fade),
                    );
                }
                let head = ellipsoid(local, Vec3::new(0.043, 0.026, 0.018));
                let stem = capsule(
                    local,
                    Vec3::new(0.034, 0.0, 0.0),
                    Vec3::new(0.034, 0.135, 0.0),
                    0.009,
                );
                let flag = if *flagged {
                    capsule(
                        local,
                        Vec3::new(0.034, 0.13, 0.0),
                        Vec3::new(0.065, 0.075, 0.0),
                        0.009,
                    )
                } else {
                    f32::INFINITY
                };
                (
                    head.min(stem).min(flag) * fade_scale,
                    Material::MusicNote(*fade),
                )
            }
            Self::Curtain(side, gust) => {
                let mut local =
                    point - Vec3::new(side * 0.88, 1.5, crate::world::ROOM_MIN.z + 0.045);
                let hanging = ((0.58 - local.y) / 1.16).clamp(0.0, 1.0);
                local.x -= side * gust * 0.035 * hanging;
                local.z -= 0.018 * (local.x * 90.0).sin() + gust * 0.28 * hanging.powi(2);
                let corner = local.abs() - Vec3::new(0.13, 0.58, 0.009);
                (
                    (corner.max(Vec3::ZERO).length() + corner.max_element().min(0.0)) * 0.55,
                    Material::Curtain,
                )
            }
            Self::Puddle(amount) => {
                let center = Vec3::new(0.0, 0.008, crate::world::ROOM_MIN.z + 0.4);
                (
                    ellipsoid(
                        point - center,
                        Vec3::new(0.48, 0.008, 0.24) * amount.sqrt().max(0.01),
                    ),
                    Material::Water,
                )
            }
            Self::Leaf(leaf, time) => {
                let local = point - leaf.position;
                let (sine, cosine) = leaf.spin.sin_cos();
                let turned = Vec3::new(
                    local.x * cosine + local.z * sine,
                    local.y,
                    local.z * cosine - local.x * sine,
                );
                (
                    ellipsoid(
                        turned,
                        Vec3::new(0.05, 0.006, 0.025) * leaf.fade(*time).max(0.001),
                    ),
                    Material::AutumnLeaf,
                )
            }
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
            Self::SwayingPlant(plant, breeze) => {
                let mut local = point - plant.position;
                let height = ((local.y - Plant::POT_HEIGHT) / plant.height()).clamp(0.0, 1.0);
                local -= Vec3::new(breeze.x, 0.0, breeze.z) * height.powi(2) * 0.25;
                let (distance, material) = plant_distance(local, plant);
                (distance * 0.85, material)
            }
            Self::Seedling(plant, position, season) => {
                seedling_distance(point - *position, plant, *season)
            }
            Self::Seed(position) => (point.distance(*position) - 0.018, Material::Seed),
            Self::Petal(petal, time) => {
                let local = point - petal.position;
                let (sine, cosine) = petal.spin.sin_cos();
                let turned = Vec3::new(
                    local.x * cosine + local.z * sine,
                    local.y,
                    local.z * cosine - local.x * sine,
                );
                let fade =
                    ((Petal::LIFETIME - (*time - petal.dropped)) / 15.0).clamp(0.0, 1.0) as f32;
                (
                    ellipsoid(turned, Vec3::new(0.027, 0.006, 0.014) * fade.max(0.001)),
                    Material::Petal,
                )
            }
            Self::Shelf(keepsakes, worn) => {
                shelf_distance(point - SHELF_POSITION, keepsakes, *worn)
            }
            Self::Keepsake(gift, position) => keepsake_distance(point - *position, *gift),
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

fn box_distance(point: Vec3, half: Vec3) -> f32 {
    let corner = point.abs() - half;
    corner.max(Vec3::ZERO).length() + corner.max_element().min(0.0)
}

fn open_book_distance(point: Vec3, index: usize, turning: f32) -> (f32, Material) {
    let mut nearest = (f32::MAX, Material::BookPages);
    for side in [-1.0, 1.0] {
        let local = Quat::from_rotation_z(-side * 0.15) * point - Vec3::X * side * 0.105;
        let cover = box_distance(local, Vec3::new(0.108, 0.018, 0.135));
        let pages = box_distance(local - Vec3::Y * 0.013, Vec3::new(0.101, 0.014, 0.128));
        let surface = if pages < cover {
            (pages, Material::BookPages)
        } else {
            (cover, Material::BookCover(index as u8))
        };
        if surface.0 < nearest.0 {
            nearest = surface;
        }
    }
    if turning > 0.0 {
        let local = Quat::from_rotation_z(-turning * std::f32::consts::PI)
            * (point - Vec3::Y * 0.03)
            - Vec3::X * 0.1;
        let page = box_distance(local, Vec3::new(0.1, 0.0025, 0.126));
        if page < nearest.0 {
            nearest = (page, Material::BookPages);
        }
    }
    nearest
}

fn music_box_distance(point: Vec3, open: bool, crank: f32) -> (f32, Material) {
    let wooden_box = |point: Vec3, half: Vec3| {
        let corner = point.abs() - half;
        corner.max(Vec3::ZERO).length() + corner.max_element().min(0.0) - 0.012
    };
    let body = wooden_box(point - Vec3::Y * 0.11, Vec3::new(0.18, 0.09, 0.145));
    let lid = if open {
        let local = point - Vec3::new(0.0, 0.22, -0.15);
        let turned = glam::Quat::from_rotation_x(0.65) * local - Vec3::Z * 0.15;
        wooden_box(turned, Vec3::new(0.185, 0.016, 0.15))
    } else {
        wooden_box(point - Vec3::Y * 0.225, Vec3::new(0.185, 0.016, 0.15))
    };
    let axle = Vec3::new(0.19, 0.13, 0.0);
    let tip = Vec3::new(0.23, 0.13 + crank.cos() * 0.045, crank.sin() * 0.045);
    let handle =
        capsule(point, axle, tip, 0.012).min(capsule(point, tip, tip + Vec3::X * 0.02, 0.016));
    if handle < body.min(lid) {
        (handle, Material::Brass)
    } else {
        (body.min(lid), Material::Wood)
    }
}

// A plank and two diagonal brackets, with the gifts in a row above it.
fn shelf_distance(
    point: Vec3,
    keepsakes: &[Keepsake],
    mut worn: Option<Keepsake>,
) -> (f32, Material) {
    let outside = point.abs() - Vec3::new(0.5, 0.025, 0.12);
    let plank = outside.max(Vec3::ZERO).length() + outside.max_element().min(0.0) - 0.005;
    let mut nearest = (plank, Material::Wood);
    for across in [-0.34, 0.34] {
        let back = Vec3::new(across, -0.18, -0.105);
        let front = Vec3::new(across, -0.025, 0.09);
        let bracket = capsule(point, back, front, 0.018).min(capsule(
            point,
            back,
            Vec3::new(across, 0.0, -0.105),
            0.018,
        ));
        if bracket < nearest.0 {
            nearest = (bracket, Material::Wood);
        }
    }
    for (index, gift) in keepsakes.iter().take(8).enumerate() {
        if Some(*gift) == worn {
            worn = None;
            continue;
        }
        let local = point - Vec3::new(-0.42 + index as f32 * 0.12, 0.03, 0.01);
        if (local - Vec3::Y * 0.08).length() - 0.14 < nearest.0 {
            let surface = keepsake_distance(local, *gift);
            if surface.0 < nearest.0 {
                nearest = surface;
            }
        }
    }
    nearest
}

/// A gift standing on the origin, also used in the window's picture.
pub(super) fn keepsake_distance(point: Vec3, gift: Keepsake) -> (f32, Material) {
    match gift {
        Keepsake::Feather => {
            let vane = ellipsoid(
                point - Vec3::new(0.014, 0.095, 0.0),
                Vec3::new(0.032, 0.075, 0.012),
            );
            let middle = Vec3::new(0.014, 0.09, 0.0);
            let quill = capsule(point, Vec3::ZERO, middle, 0.006).min(capsule(
                point,
                middle,
                Vec3::new(0.004, 0.17, 0.0),
                0.004,
            ));
            if quill < vane {
                (quill, Material::Quill)
            } else {
                (vane, Material::Feather)
            }
        }
        Keepsake::Petal => (
            ellipsoid(point - Vec3::Y * 0.018, Vec3::new(0.05, 0.015, 0.035)),
            Material::Petal,
        ),
        Keepsake::Acorn => {
            let body = ellipsoid(point - Vec3::Y * 0.043, Vec3::new(0.033, 0.043, 0.032));
            let cap = ellipsoid(point - Vec3::Y * 0.078, Vec3::new(0.04, 0.019, 0.038));
            if cap < body {
                (cap, Material::AcornCap)
            } else {
                (body, Material::Acorn)
            }
        }
        Keepsake::Leaf => {
            let leaf = ellipsoid(point - Vec3::Y * 0.016, Vec3::new(0.05, 0.012, 0.037));
            let midrib = capsule(
                point,
                Vec3::new(-0.045, 0.025, -0.02),
                Vec3::new(0.045, 0.025, 0.02),
                0.004,
            );
            if midrib < leaf {
                (midrib, Material::Quill)
            } else {
                (leaf, Material::AutumnLeaf)
            }
        }
        Keepsake::Berry => {
            let berry = [
                Vec3::new(-0.018, 0.025, 0.0),
                Vec3::new(0.018, 0.025, 0.0),
                Vec3::new(0.0, 0.045, -0.02),
            ]
            .into_iter()
            .map(|center| point.distance(center) - 0.024)
            .fold(f32::INFINITY, f32::min);
            (berry, Material::Berry)
        }
        Keepsake::Pebble => (
            ellipsoid(point - Vec3::Y * 0.025, Vec3::new(0.047, 0.025, 0.035)),
            Material::Pebble,
        ),
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
    let top = plant.top() - plant.position;
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
        let droop = Vec3::Y * (0.03 - 0.07 * plant.thirst - 0.1 * plant.droop);
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

    if plant.growth >= Plant::BLOOM {
        let opening = plant.blossom;
        let head = top - Vec3::Y * (0.08 * plant.droop);
        let bud = ellipsoid(point - head, Vec3::new(0.025, 0.045, 0.025));
        if bud < nearest.0 {
            nearest = (bud, Material::Petal);
        }
        for index in 0..5 {
            let angle = index as f32 * std::f32::consts::TAU / 5.0;
            let center = head
                + Vec3::new(
                    angle.cos() * 0.065 * opening,
                    0.025 * (1.0 - opening),
                    angle.sin() * 0.065 * opening,
                );
            let petal = ellipsoid(
                point - center,
                Vec3::new(
                    0.025 + 0.012 * opening,
                    0.012 + 0.02 * (1.0 - opening),
                    0.025 + 0.012 * opening,
                ),
            );
            if petal < nearest.0 {
                nearest = (petal, Material::Petal);
            }
        }
        if plant.seeds > 0 {
            let seed = point.distance(head + Vec3::Y * 0.06) - 0.025;
            if seed < nearest.0 {
                nearest = (seed, Material::Seed);
            }
        }
    }
    nearest
}

// Each little pot has its own bounded field; stages share the same stem
// and leaves, with the flower head chosen by kind.
fn seedling_distance(point: Vec3, plant: &Seedling, season: Season) -> (f32, Material) {
    let half = Seedling::POT_HEIGHT * 0.5;
    let radial = Vec2::new(point.x, point.z).length();
    let pot = trapezoid(
        Vec2::new(radial, point.y - half),
        0.075,
        Seedling::POT_RADIUS,
        half,
    ) - 0.006;
    let hollow = (radial - 0.092).max(Seedling::POT_HEIGHT - 0.032 - point.y);
    let pot = pot.max(-hollow);
    let soil = rounded_cylinder(
        point - Vec3::Y * (Seedling::POT_HEIGHT - 0.013),
        0.092,
        0.014,
        0.003,
    );
    let mut nearest = if soil < pot {
        (soil, Material::Soil)
    } else {
        (pot, Material::Pot)
    };
    if plant.growth < Seedling::SPROUT {
        return nearest;
    }
    let base = Vec3::Y * Seedling::POT_HEIGHT;
    let top = Vec3::new(
        0.12 * plant.droop,
        plant.height() - 0.15 * plant.droop * plant.growth,
        0.0,
    );
    let stem = capsule(point, base, top, 0.009);
    if stem < nearest.0 {
        nearest = (stem, Material::Stem);
    }
    let leaves = if plant.growth < 0.35 { 2 } else { 4 };
    for index in 0..leaves {
        let attach = base.lerp(top, 0.2 + index as f32 * 0.15);
        let outward = if index % 2 == 0 { -1.0 } else { 1.0 };
        let center = attach + Vec3::new(outward * 0.045, 0.015 - plant.droop * 0.035, 0.0);
        let leaf = ellipsoid(point - center, Vec3::new(0.055, 0.012, 0.024));
        if leaf < nearest.0 {
            nearest = (leaf, Material::Stem);
        }
    }
    if plant.growth >= Seedling::BUD {
        let flower = if plant.flowering(season) {
            flower_distance(point - top, plant.kind)
        } else {
            (
                ellipsoid(point - top, Vec3::new(0.022, 0.035, 0.022)),
                Material::Stem,
            )
        };
        if flower.0 < nearest.0 {
            nearest = flower;
        }
    }
    nearest
}

fn flower_distance(point: Vec3, kind: Flower) -> (f32, Material) {
    if kind == Flower::Tulip {
        let outside = ellipsoid(point, Vec3::new(0.06, 0.075, 0.06));
        let inside = ellipsoid(point - Vec3::Y * 0.032, Vec3::new(0.043, 0.068, 0.043));
        return (
            outside.max(-inside).max(point.y - 0.055),
            Material::Flower(kind),
        );
    }
    if kind == Flower::Bluebell {
        let mut nearest = (f32::INFINITY, Material::Stem);
        let mut previous = Vec3::ZERO;
        for index in 0..3 {
            let along = index as f32 + 1.0;
            let end = Vec3::new(along * 0.035, 0.06 - (along - 1.0).powi(2) * 0.022, 0.0);
            let stem = capsule(point, previous, end, 0.007);
            if stem < nearest.0 {
                nearest = (stem, Material::Stem);
            }
            let local = point - end + Vec3::Y * 0.035;
            let bell = trapezoid(
                Vec2::new(Vec2::new(local.x, local.z).length(), local.y),
                0.025,
                0.012,
                0.03,
            );
            if bell < nearest.0 {
                nearest = (bell, Material::Flower(kind));
            }
            previous = end;
        }
        return nearest;
    }
    let (petals, reach, radius) = match kind {
        Flower::Sunflower => (10, 0.075, 0.033),
        Flower::Poppy => (4, 0.045, 0.045),
        _ => (8, 0.055, 0.03),
    };
    let mut nearest = (
        ellipsoid(point, Vec3::new(radius, 0.022, radius)),
        Material::Center(kind),
    );
    for index in 0..petals {
        let angle = index as f32 * std::f32::consts::TAU / petals as f32;
        let outward = Vec3::new(angle.cos(), 0.0, angle.sin());
        let local = point - outward * reach;
        let turned = Vec3::new(
            local.dot(outward),
            local.y,
            local.dot(outward.cross(Vec3::Y)),
        );
        let petal = ellipsoid(turned, Vec3::new(radius, 0.012, radius * 0.6));
        if petal < nearest.0 {
            nearest = (petal, Material::Flower(kind));
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

fn holiday_table_distance(point: Vec3, occasion: Occasion, time: f64) -> (f32, Material) {
    let mut nearest = (
        rounded_cylinder(point - Vec3::Y * 0.1, 0.23, 0.04, 0.012),
        Material::Wood,
    );
    let mut include = |distance, material| {
        if distance < nearest.0 {
            nearest = (distance, material);
        }
    };
    for horizontal in [-0.13, 0.13] {
        for depth in [-0.13, 0.13] {
            include(
                rounded_cylinder(
                    point - Vec3::new(horizontal, 0.0, depth),
                    0.018,
                    0.11,
                    0.004,
                ),
                Material::Wood,
            );
        }
    }
    if point.y < 0.1 {
        return nearest;
    }
    if occasion == Occasion::Seollal {
        let bowl = point - Vec3::new(-0.09, 0.2, 0.0);
        let outside = ellipsoid(bowl, Vec3::new(0.085, 0.06, 0.085));
        let inside = ellipsoid(bowl, Vec3::new(0.077, 0.052, 0.077));
        include(outside.max(-inside).max(bowl.y), Material::Porcelain);
        include(
            rounded_cylinder(point - Vec3::new(-0.09, 0.182, 0.0), 0.069, 0.005, 0.002),
            Material::Lampshade,
        );
        for (horizontal, depth) in [(-0.11, 0.025), (-0.07, 0.02), (-0.09, -0.025)] {
            include(
                ellipsoid(
                    point - Vec3::new(horizontal, 0.189, depth),
                    Vec3::new(0.018, 0.003, 0.011),
                ),
                Material::RiceCake(2),
            );
        }
        let pouch = point - Vec3::new(0.12, 0.205, 0.0);
        include(
            ellipsoid(pouch, Vec3::new(0.065, 0.066, 0.06)),
            Material::Silk,
        );
        let neck = point - Vec3::new(0.12, 0.25, 0.0);
        include(
            Vec2::new(Vec2::new(neck.x, neck.z).length() - 0.038, neck.y).length() - 0.004,
            Material::Brass,
        );
        include(
            capsule(
                point,
                Vec3::new(0.14, 0.25, 0.038),
                Vec3::new(0.15, 0.18, 0.048),
                0.003,
            ),
            Material::Brass,
        );
        if point.x < 0.0 && point.y > 0.2 {
            let sway = (time.rem_euclid(3600.0) as f32 * 1.3).sin() * 0.008;
            for horizontal in [-0.11, -0.07] {
                include(
                    capsule(
                        point,
                        Vec3::new(horizontal, 0.21, 0.0),
                        Vec3::new(horizontal + sway, 0.3, 0.008),
                        0.0015,
                    ),
                    Material::Steam,
                );
            }
        }
    } else {
        include(
            rounded_cylinder(point - Vec3::new(-0.06, 0.14, 0.0), 0.125, 0.012, 0.005),
            Material::Porcelain,
        );
        for (index, (horizontal, depth)) in [(-0.12, -0.025), (-0.04, -0.04), (-0.075, 0.045)]
            .into_iter()
            .enumerate()
        {
            let cake = point - Vec3::new(horizontal, 0.17, depth);
            include(
                ellipsoid(cake, Vec3::new(0.045, 0.022, 0.04)).max(-cake.z - 0.006),
                Material::RiceCake(index as u8),
            );
        }
        for depth in [-0.075, 0.075] {
            let fruit = point - Vec3::new(0.13, 0.185, depth);
            include(
                ellipsoid(fruit, Vec3::new(0.055, 0.045, 0.05)),
                Material::Pumpkin,
            );
            include(
                ellipsoid(fruit - Vec3::Y * 0.043, Vec3::new(0.025, 0.006, 0.02)),
                Material::Leaf,
            );
        }
    }
    nearest
}

fn decoration_distance(point: Vec3, decoration: Decoration, time: f64) -> (f32, Material) {
    match decoration.occasion {
        Occasion::Halloween => pumpkin_distance(point),
        Occasion::WinterHolidays | Occasion::NewYear => tree_distance(point),
        Occasion::Birthday => cake_distance(point, decoration.candles, time),
        Occasion::Valentine | Occasion::HangulDay => (f32::INFINITY, Material::Frosting),
        Occasion::Seollal | Occasion::Chuseok => {
            holiday_table_distance(point, decoration.occasion, time)
        }
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
        Occasion::Valentine | Occasion::HangulDay | Occasion::Seollal | Occasion::Chuseok => {
            Vec3::ZERO
        }
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

    #[test]
    fn music_box_and_rising_notes_have_finite_distances_normals_and_ray_hits() {
        for open in [false, true] {
            for phase in [0.0, 1.5, 3.0, 5.0] {
                for piece in [
                    Furniture::MusicBox(open, phase),
                    Furniture::MusicNote(Vec3::new(1.7, 0.9, 0.3), true, 255),
                    Furniture::MusicNote(Vec3::new(1.7, 1.0, 0.3), false, 1),
                ] {
                    let (center, radius) = piece.bounds();
                    for offset in [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z, -Vec3::ONE] {
                        let point = center + offset * radius;
                        let (distance, material) = piece.distance(point);
                        assert!(distance.is_finite());
                        assert!(piece.normal(point).is_finite());
                        assert!(material.emission(10.0, &Plant::default()).is_finite());
                        if let Some((distance, _)) =
                            piece.trace(point + Vec3::Z * 4.0, -Vec3::Z, 10.0)
                        {
                            assert!(distance.is_finite() && (0.0..=10.0).contains(&distance));
                        }
                    }
                }
            }
        }
    }

    use super::*;

    #[test]
    fn curtains_puddles_and_windfalls_have_finite_distances_normals_and_ray_hits() {
        for gust in [0.0, 0.5, 1.0] {
            for amount in [0.0001, 0.1, 1.0] {
                for piece in [
                    Furniture::Curtain(-1.0, gust),
                    Furniture::Curtain(1.0, gust),
                    Furniture::Puddle(amount),
                    Furniture::SwayingPlant(
                        Plant {
                            growth: 1.0,
                            ..Plant::default()
                        },
                        Vec3::new(gust * 0.17, 0.015, gust * 0.36),
                    ),
                    Furniture::Leaf(
                        crate::world::Leaf {
                            position: Vec3::new(0.0, 0.012, -0.9),
                            spin: gust * 20.0,
                            dropped: 0.0,
                        },
                        200.0,
                    ),
                ] {
                    let (center, radius) = piece.bounds();
                    assert!(center.is_finite() && radius.is_finite() && radius > 0.0);
                    for offset in [Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z, -Vec3::ONE] {
                        let point = center + offset * radius;
                        assert!(piece.distance(point).0.is_finite());
                        assert!(piece.normal(point).is_finite());
                        if let Some((distance, _)) =
                            piece.trace(point + Vec3::Z * 4.0, -Vec3::Z, 10.0)
                        {
                            assert!(distance.is_finite() && (0.0..=10.0).contains(&distance));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn book_piles_and_turning_pages_have_finite_bounded_distances_and_ray_hits() {
        let books = crate::world::books::Books::default();
        let mut pieces = vec![Furniture::Books(&books.owned, None)];
        for size in [0.68, 1.0] {
            for turning in [0.0, 0.25, 0.5, 0.75, 1.0] {
                pieces.push(Furniture::OpenBook {
                    index: 0,
                    position: Vec3::new(0.1, 0.8, 0.2),
                    size,
                    rotation: Quat::from_rotation_y(0.7),
                    turning,
                });
            }
        }
        for piece in pieces {
            let (center, radius) = piece.bounds();
            for across in -6..=6 {
                for up in -6..=6 {
                    for depth in -6..=6 {
                        let point =
                            center + Vec3::new(across as f32, up as f32, depth as f32) * 0.06;
                        let distance = piece.distance(point).0;
                        assert!(distance.is_finite());
                        assert!(piece.normal(point).is_finite());
                        if distance <= 0.0 {
                            assert!(point.distance(center) <= radius);
                        }
                    }
                }
            }
            let origin = center + Vec3::Z * 2.0;
            if let Some((distance, _)) = piece.trace(origin, -Vec3::Z, 4.0) {
                assert!(distance.is_finite());
                assert!((0.0..4.0).contains(&distance));
            }
        }
    }

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
                ..Plant::default()
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
    fn simple_shapes_measure_zero_on_their_surfaces() {
        assert!(rounded_cylinder(Vec3::new(0.0, 0.14, 0.0), 0.42, 0.14, 0.06).abs() < 1e-5);
        assert!(ellipsoid(Vec3::new(0.1, 0.0, 0.0), Vec3::new(0.1, 0.02, 0.05)).abs() < 1e-5);
        assert!(capsule(Vec3::new(0.02, 0.5, 0.0), Vec3::ZERO, Vec3::Y, 0.02).abs() < 1e-5);
    }
}
