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
    BookInk,
    MusicNote(u8),
    Envelope,
    EnvelopeFlap,
    HeartSeal,
    Treasure,
    Origami(crate::world::acquaintance::Color, bool, bool),
    CraneThread,
    Cork(bool),
}

impl Material {
    /// The surface's color under white light. Colors stay mid to bright so
    /// shapes keep their contrast in plain styles.
    pub fn albedo(self, plant: &Plant) -> Vec3 {
        match self {
            Self::Envelope => Vec3::new(1.0, 0.98, 0.92),
            Self::EnvelopeFlap => Vec3::new(0.79, 0.81, 0.84),
            Self::HeartSeal => Vec3::new(1.0, 0.32, 0.42),
            Self::BookCover(index) => match index % 5 {
                0 => Vec3::new(0.62, 0.78, 0.9),
                1 => Vec3::new(0.9, 0.68, 0.72),
                2 => Vec3::new(0.7, 0.84, 0.6),
                3 => Vec3::new(0.86, 0.73, 0.95),
                _ => Vec3::new(0.98, 0.82, 0.56),
            },
            Self::BookInk => Vec3::new(0.48, 0.52, 0.65),
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
            Self::Treasure => Vec3::new(1.0, 0.75, 0.35),
            Self::Origami(color, crease, _) => color
                .rgb()
                .lerp(Vec3::ONE, if crease { 0.35 } else { 0.05 }),
            Self::CraneThread => Vec3::new(0.85, 0.82, 0.72),
            Self::Cork(dotted) => {
                if dotted {
                    Vec3::new(0.65, 0.43, 0.28)
                } else {
                    Vec3::new(0.76, 0.55, 0.35)
                }
            }
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
            Self::Origami(_, _, true) => 0.3,
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
    Shelf(&'a [Keepsake], Option<Keepsake>, Option<usize>),
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
        ink: Option<f32>,
    },
    MusicBox(bool, f32),
    MusicNote(Vec3, bool, u8),
    Mirror,
    Envelope,
    ViewerLetter(Vec3),
    Treasure {
        position: Vec3,
        radius: f32,
        brightness: f32,
    },
    Cranes(&'a Garland),
    FoldingPaper(crate::world::cranes::Paper, Vec3, Quat),
    BubbleWand(Vec3),
    Gallery(&'a [crate::world::gallery::Pin]),
    GalleryCard(Vec3),
    SongSheet(Vec3, Quat),
}

impl Furniture<'_> {
    pub fn bounds(&self) -> (Vec3, f32) {
        match self {
            Self::Mirror => (
                crate::world::mirror::CENTER - Vec3::X * crate::world::mirror::STANDOFF,
                0.8,
            ),
            Self::ViewerLetter(position) => (*position, 0.21),
            Self::Envelope => (crate::world::mail::POSITION + Vec3::Y * 0.012, 0.24),
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
            Self::Treasure {
                position, radius, ..
            } => (*position, radius + 0.01),
            Self::Cranes(_) => (crate::world::cranes::CENTER, 0.72),
            Self::FoldingPaper(_, position, _) => (*position, 0.12),
            Self::BubbleWand(position) => (*position - Vec3::Y * 0.07, 0.19),
            Self::Gallery(_) => (crate::world::gallery::CENTER, 0.54),
            Self::GalleryCard(position) => (*position, 0.21),
            Self::SongSheet(position, _) => (*position, 0.21),
        }
    }

    /// Signed distance from `point` to the furniture, and the material
    /// nearest it.
    pub fn distance(&self, point: Vec3) -> (f32, Material) {
        match self {
            Self::Mirror => {
                use crate::world::mirror::{
                    CENTER, FRAME_DEPTH, FRAME_WIDTH, HEIGHT, STANDOFF, WIDTH,
                };
                let local = point - (CENTER - Vec3::X * STANDOFF);
                let outer = box_distance(
                    local,
                    Vec3::new(
                        FRAME_DEPTH * 0.5,
                        HEIGHT * 0.5 + FRAME_WIDTH,
                        WIDTH * 0.5 + FRAME_WIDTH,
                    ),
                );
                let opening =
                    box_distance(local, Vec3::new(FRAME_DEPTH, HEIGHT * 0.5, WIDTH * 0.5));
                (outer.max(-opening), Material::Wood)
            }
            Self::ViewerLetter(position) => {
                let point = point - *position;
                let distance = box_distance(point, Vec3::new(0.16, 0.10, 0.008));
                let fold = (point.y - point.x.abs() * 0.5 + 0.03).abs();
                (
                    distance,
                    if fold < 0.006 {
                        Material::EnvelopeFlap
                    } else {
                        Material::Envelope
                    },
                )
            }
            Self::Envelope => envelope(point),
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
                ink,
            } => {
                let local = rotation.conjugate() * (point - *position) / *size;
                let (distance, material) = open_book_distance(local, *index, *turning, *ink);
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
            Self::Shelf(keepsakes, worn, borrowed) => {
                shelf_distance(point - SHELF_POSITION, keepsakes, *worn, *borrowed)
            }
            Self::Keepsake(gift, position) => keepsake_distance(point - *position, *gift),
            Self::Nook(nook) => (
                rounded_cylinder(point - nook.position, Nook::RADIUS, Nook::HEIGHT, 0.06),
                Material::Cushion,
            ),
            Self::Treasure {
                position, radius, ..
            } => ((point - *position).length() - radius, Material::Treasure),
            Self::Cranes(garland) => garland.distance(point),
            Self::FoldingPaper(paper, position, rotation) => {
                let point = rotation.inverse() * (point - *position);
                let bounds = box_distance(point, Vec3::splat(0.1));
                if bounds > 0.02 {
                    return (bounds, Material::Origami(paper.color, false, false));
                }
                let (distance, crease) = folding_paper_distance(point, paper.progress);
                (distance, Material::Origami(paper.color, crease, false))
            }
            Self::BubbleWand(position) => {
                let local = point - *position;
                let ring = Vec2::new(local.truncate().length() - 0.065, local.z).length() - 0.009;
                let stick = capsule(
                    local,
                    Vec3::new(0.0, -0.065, 0.0),
                    Vec3::new(0.02, -0.21, 0.0),
                    0.012,
                );
                (ring.min(stick), Material::Brass)
            }
            Self::Gallery(pins) => gallery_distance(point, pins),
            Self::GalleryCard(position) => gallery_card(point - *position, None, 0),
            Self::SongSheet(position, rotation) => {
                let local = rotation.conjugate() * (point - *position);
                let distance = box_distance(local, Vec3::new(0.15, 0.10, 0.008));
                let note = capsule(
                    local,
                    Vec3::new(0.025, -0.04, 0.0),
                    Vec3::new(0.025, 0.06, 0.0),
                    0.006,
                )
                .min((local - Vec3::new(0.005, -0.04, 0.0)).length() - 0.022);
                let fold = (local.x + local.y - 0.17).abs();
                (
                    distance,
                    if note < 0.012 {
                        Material::Wood
                    } else if fold < 0.006 {
                        Material::EnvelopeFlap
                    } else {
                        Material::Envelope
                    },
                )
            }
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
        let (enter, exit) = if let Self::Cranes(_) = self {
            let (near, far) = box_span(
                origin,
                direction,
                crate::world::cranes::CENTER,
                Vec3::new(0.39, 0.53, 0.17),
            )?;
            (enter.max(near), exit.min(far))
        } else {
            (enter, exit)
        };
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

#[derive(Clone, Debug, PartialEq)]
pub struct Garland {
    threads: Vec<CraneThread>,
    glowing: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct CraneThread {
    top: Vec3,
    end: Vec3,
    center: Vec3,
    half: Vec3,
    cranes: Vec<(Vec3, crate::world::acquaintance::Color)>,
}

impl Garland {
    pub fn of(world: &crate::world::World) -> Self {
        let borrowed = world.shown_token().and_then(|(token, _)| match token {
            crate::world::show::Token::Crane(_, index) => Some(index),
            _ => None,
        });
        let mut threads = Vec::new();
        for (index, colors) in world.life.cranes.colors.chunks(10).take(8).enumerate() {
            let last = crate::world::cranes::slot(index * 10 + colors.len() - 1);
            let top = Vec3::new(last.x, crate::world::ROOM_MAX.y, last.z);
            let breeze = world.breeze_at(last).clamp(Vec3::splat(-1.0), Vec3::ONE) * 0.012;
            let drift = Vec3::new(
                (world.time * 0.31 + index as f64).sin() as f32 * 0.007,
                0.0,
                (world.time * 0.23 + index as f64).cos() as f32 * 0.005,
            );
            let sway = Vec3::new(breeze.x, 0.0, breeze.z) + drift;
            let cranes: Vec<_> = colors
                .iter()
                .enumerate()
                .filter(|(level, _)| Some(index * 10 + level) != borrowed)
                .map(|(level, color)| {
                    let position = crate::world::cranes::slot(index * 10 + level);
                    let sway = sway * ((top.y - position.y) / (top.y - last.y));
                    (position + sway, *color)
                })
                .collect();
            let end = last + sway;
            let low = top.min(end) - Vec3::new(0.052, 0.04, 0.045);
            let high = top.max(end) + Vec3::new(0.052, 0.0, 0.045);
            threads.push(CraneThread {
                top,
                end,
                center: (low + high) * 0.5,
                half: (high - low) * 0.5,
                cranes,
            });
        }
        Self {
            threads,
            glowing: world.life.cranes.total >= 1000,
        }
    }

    fn distance(&self, point: Vec3) -> (f32, Material) {
        let bounds = box_distance(
            point - crate::world::cranes::CENTER,
            Vec3::new(0.39, 0.53, 0.17),
        );
        if bounds > 0.02 {
            return (bounds, Material::CraneThread);
        }
        let mut nearest = (f32::MAX, Material::CraneThread);
        for thread in &self.threads {
            let bound = box_distance(point - thread.center, thread.half);
            if bound > 0.01 {
                nearest.0 = nearest.0.min(bound);
                continue;
            }
            let distance = capsule(point, thread.top, thread.end, 0.002);
            if distance < nearest.0 {
                nearest = (distance, Material::CraneThread);
            }
            for (position, color) in &thread.cranes {
                let local = point - *position;
                let bound = local.length() - 0.052;
                if bound > nearest.0 {
                    continue;
                }
                let (distance, crease) = crane_distance(local);
                if distance < nearest.0 {
                    nearest = (distance, Material::Origami(*color, crease, self.glowing));
                }
            }
        }
        nearest
    }
}

pub(super) fn box_span(origin: Vec3, direction: Vec3, center: Vec3, half: Vec3) -> Option<(f32, f32)> {
    let mut enter = 0.0f32;
    let mut exit = f32::MAX;
    for axis in 0..3 {
        let offset = origin[axis] - center[axis];
        if direction[axis].abs() < 1e-8 {
            if offset.abs() > half[axis] {
                return None;
            }
        } else {
            let first = (-half[axis] - offset) / direction[axis];
            let second = (half[axis] - offset) / direction[axis];
            enter = enter.max(first.min(second));
            exit = exit.min(first.max(second));
        }
    }
    (exit >= enter).then_some((enter, exit))
}

fn crane_distance(point: Vec3) -> (f32, bool) {
    let bound = box_distance(point, Vec3::new(0.047, 0.035, 0.035));
    if bound > 0.01 {
        return (bound, false);
    }
    let body = ellipsoid(point, Vec3::new(0.011, 0.012, 0.022));
    let mut distance = body;
    for side in [-1.0, 1.0] {
        let local = Vec3::new(point.x * side, point.y, point.z);
        // A flattened, tapered wing shares its root with the central crease.
        let width = (0.029 - (local.x - 0.009).abs() * 0.7).max(0.001);
        let wing = box_distance(
            Vec3::new(local.x - 0.023, local.y - (local.x * 0.5), local.z),
            Vec3::new(0.021, 0.002, width),
        ) * 0.7;
        distance = distance.min(wing);
    }
    distance = distance.min(capsule(
        point,
        Vec3::new(0.0, 0.0, -0.013),
        Vec3::new(0.0, 0.028, -0.025),
        0.003,
    ));
    distance = distance.min(capsule(
        point,
        Vec3::new(0.0, 0.028, -0.025),
        Vec3::new(0.0, 0.023, -0.034),
        0.002,
    ));
    distance = distance.min(capsule(
        point,
        Vec3::new(0.0, 0.0, 0.013),
        Vec3::new(0.0, 0.02, 0.032),
        0.0025,
    ));
    (distance, point.x.abs() < 0.003)
}

fn folding_paper_distance(point: Vec3, progress: f32) -> (f32, bool) {
    let progress = progress.clamp(0.0, 1.0);
    if progress >= 0.75 {
        let (crane, crease) = crane_distance(point);
        let local = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4) * point;
        let triangle =
            box_distance(local, Vec3::new(0.062, 0.0025, 0.062)).max((-local.z - local.x) * 0.707);
        return (
            triangle + (crane - triangle) * ((progress - 0.75) * 4.0),
            crease,
        );
    }
    let angle = (progress / 0.35).min(1.0) * std::f32::consts::FRAC_PI_4;
    let local = Quat::from_rotation_y(angle) * point;
    let square = box_distance(local, Vec3::new(0.062, 0.0025, 0.062));
    let fold = ((progress - 0.35) / 0.4).clamp(0.0, 1.0);
    let folded = square.max((-local.z - local.x) * 0.707 - (1.0 - fold) * 0.09);
    (folded, (local.x + local.z).abs() < 0.003)
}

fn envelope(point: Vec3) -> (f32, Material) {
    let point = point - crate::world::mail::POSITION - Vec3::Y * 0.012;
    let bounds = box_distance(point, Vec3::new(0.20, 0.025, 0.13));
    if bounds > 0.03 {
        return (bounds, Material::Envelope);
    }
    let paper = box_distance(
        point,
        Vec3::new(
            crate::world::mail::HALF_WIDTH,
            0.008,
            crate::world::mail::HALF_DEPTH,
        ),
    );
    let fold = (point.z - (point.x.abs() * 0.65 - 0.04)).abs();
    let heart = Vec2::new(point.x, point.z) - Vec2::new(0.0, -0.025);
    let lobes = (heart - Vec2::new(0.012, 0.008))
        .length()
        .min((heart - Vec2::new(-0.012, 0.008)).length())
        - 0.019;
    let tip = (heart.x.abs() + (heart.y + 0.019).abs() - 0.025) * 0.707;
    let seal = lobes.min(tip).max((point.y - 0.01).abs() - 0.006);
    if seal < paper {
        (seal, Material::HeartSeal)
    } else {
        (
            paper,
            if fold < 0.006 {
                Material::EnvelopeFlap
            } else {
                Material::Envelope
            },
        )
    }
}

pub(super) fn box_distance(point: Vec3, half: Vec3) -> f32 {
    let corner = point.abs() - half;
    corner.max(Vec3::ZERO).length() + corner.max_element().min(0.0)
}

fn open_book_distance(
    point: Vec3,
    index: usize,
    turning: f32,
    ink: Option<f32>,
) -> (f32, Material) {
    let mut nearest = (f32::MAX, Material::BookPages);
    for side in [-1.0, 1.0] {
        let local = Quat::from_rotation_z(-side * 0.15) * point - Vec3::X * side * 0.105;
        let cover = box_distance(local, Vec3::new(0.108, 0.018, 0.135));
        let pages = box_distance(local - Vec3::Y * 0.013, Vec3::new(0.101, 0.014, 0.128));
        let surface = if pages < cover {
            let written = ink.is_some_and(|progress| {
                let line = ((local.z + 0.09) / 0.035).floor();
                local.y > 0.02
                    && local.x.abs() < 0.07
                    && (0.0..5.0).contains(&line)
                    && line < progress * 5.0
                    && ((local.z + 0.09).rem_euclid(0.035) - 0.016).abs() < 0.003
            });
            (
                pages,
                if written {
                    Material::BookInk
                } else {
                    Material::BookPages
                },
            )
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

fn gallery_distance(point: Vec3, pins: &[crate::world::gallery::Pin]) -> (f32, Material) {
    use crate::world::gallery::{CENTER, DEPTH, FRAME_WIDTH, HEIGHT, WIDTH};
    let local = point - CENTER;
    let bounds = box_distance(
        local,
        Vec3::new(DEPTH * 0.5 + 0.025, HEIGHT * 0.5, WIDTH * 0.5),
    );
    if bounds > 0.012 {
        return (bounds, Material::Wood);
    }
    let frame = box_distance(local, Vec3::new(DEPTH * 0.5, HEIGHT * 0.5, WIDTH * 0.5));
    let opening = box_distance(
        local,
        Vec3::new(DEPTH, HEIGHT * 0.5 - FRAME_WIDTH, WIDTH * 0.5 - FRAME_WIDTH),
    );
    let cork = box_distance(
        local,
        Vec3::new(
            DEPTH * 0.3,
            HEIGHT * 0.5 - FRAME_WIDTH,
            WIDTH * 0.5 - FRAME_WIDTH,
        ),
    );
    let dotted = (local.z * 85.0).fract().abs() < 0.23 && (local.y * 85.0).fract().abs() < 0.23;
    let mut nearest = if frame.max(-opening) < cork {
        (frame.max(-opening), Material::Wood)
    } else {
        (cork, Material::Cork(dotted))
    };
    for (index, pin) in pins.iter().take(4).enumerate() {
        let center = Vec2::new(
            if index % 2 == 0 { -0.205 } else { 0.205 },
            if index < 2 { 0.13 } else { -0.13 },
        );
        let across = Vec2::new(local.z, local.y) - center;
        let card_bounds = box_distance(
            Vec3::new(across.x, across.y, local.x - DEPTH * 0.5),
            Vec3::new(0.18, 0.12, 0.022),
        );
        if card_bounds > nearest.0 {
            continue;
        }
        // Fixed small tilts avoid trigonometry in the ray path.
        let (sine, cosine) = [
            (-0.06, 0.9982),
            (0.04, 0.9992),
            (0.05, 0.99875),
            (-0.03, 0.99955),
        ][index];
        let turned = Vec2::new(
            across.x * cosine + across.y * sine,
            across.y * cosine - across.x * sine,
        );
        let card = gallery_card(
            Vec3::new(turned.x, turned.y, local.x - DEPTH * 0.5 - 0.004),
            Some(pin),
            index,
        );
        if card.0 < nearest.0 {
            nearest = card;
        }
    }
    nearest
}

fn gallery_card(
    point: Vec3,
    pin: Option<&crate::world::gallery::Pin>,
    index: usize,
) -> (f32, Material) {
    let bounds = box_distance(point, Vec3::new(0.165, 0.105, 0.018));
    if bounds > 0.006 {
        return (bounds, Material::BookPages);
    }
    let paper = box_distance(point, Vec3::new(0.165, 0.105, 0.003));
    let thumbtack = point.distance(Vec3::new(0.0, 0.083, 0.008)) - 0.012;
    if thumbtack < paper {
        return (thumbtack, Material::Block(index as u8));
    }
    let ink = point.y < 0.068
        && pin.is_some_and(|pin| pin.distance(Vec2::new(point.x, point.y + 0.008) / 0.13) < 0.025);
    (
        paper,
        if ink {
            Material::BookInk
        } else {
            Material::BookPages
        },
    )
}

// A plank and two diagonal brackets, with the gifts in a row above it.
fn shelf_distance(
    point: Vec3,
    keepsakes: &[Keepsake],
    mut worn: Option<Keepsake>,
    borrowed: Option<usize>,
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
        if Some(index) == borrowed {
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
    // Planted in the garden bed: the soil is the bed's, no pot.
    let mut nearest = (f32::INFINITY, Material::Soil);
    if plant.growth < Seedling::SPROUT {
        let mound = ellipsoid(
            point - Vec3::Y * Seedling::POT_HEIGHT,
            Vec3::new(0.07, 0.025, 0.07),
        );
        return (mound, Material::Soil);
    }
    seedling_plant(point, plant, season, &mut nearest);
    nearest
}

fn seedling_plant(point: Vec3, plant: &Seedling, season: Season, nearest: &mut (f32, Material)) {
    let base = Vec3::Y * Seedling::POT_HEIGHT;
    let top = Vec3::new(
        0.12 * plant.droop,
        plant.height() - 0.15 * plant.droop * plant.growth,
        0.0,
    );
    let stem = capsule(point, base, top, 0.009);
    if stem < nearest.0 {
        *nearest = (stem, Material::Stem);
    }
    let leaves = if plant.growth < 0.35 { 2 } else { 4 };
    for index in 0..leaves {
        let attach = base.lerp(top, 0.2 + index as f32 * 0.15);
        let outward = if index % 2 == 0 { -1.0 } else { 1.0 };
        let center = attach + Vec3::new(outward * 0.045, 0.015 - plant.droop * 0.035, 0.0);
        let leaf = ellipsoid(point - center, Vec3::new(0.055, 0.012, 0.024));
        if leaf < nearest.0 {
            *nearest = (leaf, Material::Stem);
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
            *nearest = flower;
        }
    }
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
    if matches!(occasion, Occasion::HundredDays | Occasion::ThousandDays) {
        let second = occasion == Occasion::ThousandDays;
        let center = if second { -0.075 } else { 0.0 };
        for layer in 0..3 {
            include(
                rounded_cylinder(
                    point - Vec3::new(center, 0.14 + layer as f32 * 0.035, 0.0),
                    0.12 - layer as f32 * 0.012,
                    0.035,
                    0.003,
                ),
                Material::RiceCake(2),
            );
        }
        if second {
            for layer in 0..3 {
                include(
                    rounded_cylinder(
                        point - Vec3::new(0.13, 0.14 + layer as f32 * 0.025, 0.025),
                        0.064 - layer as f32 * 0.007,
                        0.025,
                        0.003,
                    ),
                    Material::RiceCake(2),
                );
            }
        }
        include(
            capsule(
                point,
                Vec3::new(center, 0.23, 0.0),
                Vec3::new(center, 0.31, 0.0),
                0.003,
            ),
            Material::Wood,
        );
        let flag = point - Vec3::new(center + 0.029, 0.29, 0.0);
        include(
            box_distance(flag, Vec3::new(0.029, 0.018, 0.002)),
            if flag.y > 0.0 {
                Material::Silk
            } else {
                Material::RiceCake(2)
            },
        );
    } else if occasion == Occasion::Seollal {
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
        Occasion::Birthday | Occasion::ViewerBirthday => {
            cake_distance(point, decoration.candles, time)
        }
        Occasion::Valentine | Occasion::HangulDay => (f32::INFINITY, Material::Frosting),
        Occasion::Seollal | Occasion::Chuseok | Occasion::HundredDays | Occasion::ThousandDays => {
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
        Occasion::Halloween | Occasion::Birthday | Occasion::ViewerBirthday => {
            Vec3::new(1.0, 0.57, 0.2) * 0.65 * flicker(time)
        }
        Occasion::WinterHolidays | Occasion::NewYear => {
            let phase = time.rem_euclid(3600.0) as f32 * 0.7;
            Vec3::new(0.8, 0.55, 0.65).lerp(Vec3::new(0.45, 0.8, 1.0), 0.5 + 0.5 * phase.sin())
                * 0.4
        }
        Occasion::Valentine
        | Occasion::HangulDay
        | Occasion::Seollal
        | Occasion::Chuseok
        | Occasion::HundredDays
        | Occasion::ThousandDays => Vec3::ZERO,
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
    fn the_envelope_distance_is_finite_and_contained_in_its_bounds() {
        let envelope = Furniture::Envelope;
        let (center, radius) = envelope.bounds();
        for across in -12..=12 {
            for height in -3..=3 {
                for depth in -12..=12 {
                    let point =
                        center + Vec3::new(across as f32, height as f32, depth as f32) * 0.025;
                    let (distance, _) = envelope.distance(point);
                    assert!(distance.is_finite());
                    assert!(envelope.normal(point).is_finite());
                    if (point - center).length() > radius {
                        assert!(distance > 0.0);
                    }
                }
            }
        }
        for direction in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE.normalize()] {
            if let Some((distance, _)) = envelope.trace(center + direction, -direction, 2.0) {
                assert!(distance.is_finite() && (0.0..=2.0).contains(&distance));
            }
        }
    }

    #[test]
    fn the_mirror_frame_has_bounded_finite_distances_normals_and_hits() {
        use crate::world::mirror;
        let piece = Furniture::Mirror;
        let (center, radius) = piece.bounds();
        for step in 0..64 {
            let angle = step as f32 * std::f32::consts::TAU / 64.0;
            for offset in [
                Vec3::new(0.0, angle.cos(), angle.sin()),
                Vec3::new(angle.cos(), angle.sin(), 0.0),
            ] {
                let point = center + offset * radius;
                assert!(piece.distance(point).0 > 0.0);
                assert!(piece.normal(point).is_finite());
            }
        }
        for target in [
            mirror::CENTER,
            mirror::CENTER + Vec3::Y * (mirror::HEIGHT + mirror::FRAME_WIDTH) * 0.5,
        ] {
            let origin = target - Vec3::X * 2.0;
            if let Some((distance, _)) = piece.trace(origin, Vec3::X, 3.0) {
                assert!(distance.is_finite() && (0.0..=3.0).contains(&distance));
            }
            assert!(piece.distance(target).0.is_finite());
        }
    }

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
                    ink: None,
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

    #[test]
    fn treasure_sphere_has_finite_distances_and_normals_and_bounded_ray_hits() {
        let position = Vec3::new(0.4, 0.06, -0.5);
        let piece = Furniture::Treasure {
            position,
            radius: crate::world::hunt::RADIUS,
            brightness: 1.0,
        };
        let (center, radius) = piece.bounds();
        assert!(center.is_finite() && radius.is_finite() && radius > 0.0);
        for direction in [Vec3::X, Vec3::Y, Vec3::Z] {
            let point = position + direction * crate::world::hunt::RADIUS;
            assert!(piece.distance(point).0.is_finite());
            assert!(piece.normal(point).is_finite());
            let (distance, _) = piece.trace(position + direction, -direction, 2.0).unwrap();
            assert!(distance.is_finite() && (0.9..1.0).contains(&distance));
        }
        assert!(piece.trace(position + Vec3::X, Vec3::Z, 2.0).is_none());
    }

    #[test]
    fn crane_and_folding_paper_distances_normals_and_bounds_stay_finite() {
        let mut world = crate::world::World::new(1, 18.0);
        world.life.cranes.colors = vec![crate::world::acquaintance::Color::Pink; 80];
        world.life.cranes.total = 1000;
        let garland = Garland::of(&world);
        let furniture = Furniture::Cranes(&garland);
        for progress in [0.0, 0.25, 0.5, 0.75, 0.9, 1.0] {
            let paper = Furniture::FoldingPaper(
                crate::world::cranes::Paper {
                    color: crate::world::acquaintance::Color::Blue,
                    progress,
                },
                Vec3::ZERO,
                Quat::IDENTITY,
            );
            for across in -6..=6 {
                for height in -5..=5 {
                    for depth in -6..=6 {
                        let point = Vec3::new(across as f32, height as f32, depth as f32) * 0.025;
                        assert!(paper.distance(point).0.is_finite());
                        assert!(paper.normal(point).is_finite());
                        if point.length() > paper.bounds().1 {
                            assert!(paper.distance(point).0 > 0.0);
                        }
                        let position = point + crate::world::cranes::CENTER;
                        assert!(furniture.distance(position).0.is_finite());
                        assert!(furniture.normal(position).is_finite());
                    }
                }
            }
        }
        for direction in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE.normalize()] {
            let center = crate::world::cranes::slot(20);
            if let Some((distance, _)) = furniture.trace(center - direction, direction, 3.0) {
                assert!(distance.is_finite() && distance >= 0.0);
            }
            assert!(box_span(center, direction, center, Vec3::splat(0.1)).is_some());
        }
        for thread in &garland.threads {
            for (position, _) in &thread.cranes {
                assert!(position.is_finite());
                assert!((*position - thread.center).abs().cmple(thread.half).all());
            }
        }
    }

    #[test]
    fn gallery_distances_and_normals_stay_finite_inside_and_outside_their_bounds() {
        let pins: Vec<crate::world::gallery::Pin> = serde_json::from_str(r#"[{"kind":"ViewerDrawing","name":"heart","korean":"하트","day":"2026-06-21","strokes":[[[-0.5,0.0],[0.0,0.5],[0.5,0.0],[0.0,-0.5],[-0.5,0.0]]]}]"#).unwrap();
        for furniture in [
            Furniture::Gallery(&[]),
            Furniture::Gallery(&pins),
            Furniture::GalleryCard(Vec3::ZERO),
        ] {
            let (center, radius) = furniture.bounds();
            assert!(center.is_finite() && radius.is_finite() && radius > 0.0);
            for horizontal in -7..=7 {
                for vertical in -7..=7 {
                    for depth in [-1.0, -0.03, 0.0, 0.03, 1.0] {
                        let point = center
                            + Vec3::new(depth, vertical as f32 * 0.1, horizontal as f32 * 0.1);
                        assert!(furniture.distance(point).0.is_finite());
                        assert!(furniture.normal(point).is_finite());
                    }
                }
            }
            assert!(
                furniture
                    .trace(center + Vec3::X * 2.0, -Vec3::X, 4.0)
                    .is_some()
            );
        }
    }

    #[test]
    fn a_song_sheet_has_finite_distances_normals_and_bounded_ray_hits() {
        for angle in [0.0, 0.7, 2.4] {
            let sheet =
                Furniture::SongSheet(Vec3::new(0.2, 0.8, 0.4), Quat::from_rotation_y(angle));
            let (center, radius) = sheet.bounds();
            for across in -6..=6 {
                for up in -6..=6 {
                    for depth in -6..=6 {
                        let point =
                            center + Vec3::new(across as f32, up as f32, depth as f32) * 0.04;
                        let distance = sheet.distance(point).0;
                        assert!(distance.is_finite());
                        assert!(sheet.normal(point).is_finite());
                        if distance <= 0.0 {
                            assert!(point.distance(center) <= radius);
                        }
                    }
                }
            }
            let origin = center + Vec3::Z * 2.0;
            if let Some((distance, _)) = sheet.trace(origin, -Vec3::Z, 4.0) {
                assert!(distance.is_finite());
                assert!((0.0..4.0).contains(&distance));
            }
        }
    }
}
