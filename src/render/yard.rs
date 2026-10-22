//! Everything outdoors: the lawn and the land past the fence, the cottage
//! from outside, the yard's things, critters, rain and snow, and the light
//! of the sun, the moon, and the lantern.

use std::f32::consts::TAU;

use glam::{Vec2, Vec3};

use super::View;
use super::shapes::{
    Furniture, Material, box_distance, box_span, capsule, ellipsoid, rounded_cylinder, sphere_span,
};
use super::sky::{self, Heavens, Outside, Sky, noise, phase, scramble, weather_color};
use crate::sdf::{coverage, mix, smooth_min, smoothstep};
use crate::world::cat::{CatPicture, Coat, Pose};
use crate::world::home::{
    self, BEDROOM_MAX, BEDROOM_MIN, BEDROOM_WINDOW_HIGH, BEDROOM_WINDOW_LOW, BENCH, BIRDBATH,
    BUSHES, CHIMNEY, CHIMNEY_HALF, CHIMNEY_TOP, CLOTHESLINE, CLOTHESLINE_HEIGHT, DOOR_HIGH,
    DOOR_LOW, EAVE, EAVE_DROP, GABLE, GARDEN, GARDEN_HALF, HOUSE_MAX, HOUSE_MIN, LANTERN,
    LANTERN_HEIGHT, LOT_MAX, LOT_MIN, MAILBOX, PERCH, RIDGE, SILL, SILL_HALF, SWING_HALF_WIDTH,
    SWING_TOP, TREE, TREE_CROWN, TREE_CROWN_RADIUS,
};
use crate::world::passerby::{Passerby, Passing};
use crate::world::{BirdPicture, Occasion, Plant, Season};
use crate::world::{ROOM_MAX, ROOM_MIN};

const LANTERN_LIGHT: Vec3 = Vec3::new(1.0, 0.74, 0.42);
/// Big plain surfaces are drawn dimmer than the things on them, so the
/// yard's things and the ghost stand out in glyphs.
const GROUND_EXPOSURE: f32 = 0.55;
const SHELL_EXPOSURE: f32 = 0.7;
const SNOW: Vec3 = Vec3::new(0.9, 0.94, 1.0);
/// The jack-o'-lantern on the step by the door at Halloween.
const PUMPKIN: Vec3 = Vec3::new(2.32, 0.0, 1.66);
use crate::world::home::{SNOW_GHOST, SNOWMAN};
/// Stepping stones from the door to the mailbox, (x, z) and radius.
const STONES: [(Vec2, f32); 7] = [
    (Vec2::new(2.45, 0.8), 0.2),
    (Vec2::new(2.9, 1.05), 0.18),
    (Vec2::new(3.25, 1.75), 0.19),
    (Vec2::new(3.75, 2.05), 0.17),
    (Vec2::new(4.35, 2.2), 0.18),
    (Vec2::new(5.0, 2.3), 0.17),
    (Vec2::new(5.65, 2.45), 0.18),
];

/// What a yard surface is made of.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Surface {
    Bark,
    Foliage,
    Hedge,
    Picket,
    Timber,
    Soil,
    Stone,
    Water,
    Ice,
    Iron,
    Glass,
    Mailbox,
    Flag,
    Rope,
    Cloth(u8),
    Snow,
    Coal,
    Carrot,
    Scarf,
    Siding,
    Pumpkin,
    Shingle,
    Brick,
    Door,
    Trim,
    /// The cut edge of the walls around the glass front.
    Section,
    Cap,
    Thing(Material),
}

/// A thing standing in the yard.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Piece {
    Tree,
    Fence(Vec3, Vec3),
    Swing,
    Garden,
    Birdbath,
    Bench,
    Lantern,
    Mailbox,
    Clothesline,
    Bush(Vec3, f32),
    Sill,
    Snowman(f32),
    /// The door swung out by this many radians.
    Door(f32),
    Chimney,
    Pumpkin,
    /// The ghost's snow sculpture, how built and how melted.
    SnowGhost(f32, f32),
}

impl Piece {
    /// A box around the piece, as its middle and half size, holding all the
    /// bounds `span` tests.
    fn bounding_box(&self) -> (Vec3, Vec3) {
        let ball = |center: Vec3, radius: f32| (center, Vec3::splat(radius));
        match *self {
            Self::Fence(start, end) => (
                (start + end) * 0.5 + Vec3::Y * 0.45,
                ((end - start) * 0.5).abs() + Vec3::new(0.08, 0.5, 0.08),
            ),
            Self::Tree => {
                let crown = TREE_CROWN_RADIUS + 0.75;
                let low =
                    (TREE_CROWN - Vec3::splat(crown)).min(TREE + Vec3::new(-2.05, -0.05, -0.65));
                let high =
                    (TREE_CROWN + Vec3::splat(crown)).max(TREE + Vec3::new(1.65, 4.65, 1.35));
                ((low + high) * 0.5, (high - low) * 0.5)
            }
            Self::Swing => (
                SWING_TOP - Vec3::Y * (SWING_TOP.y * 0.5 - 0.05),
                Vec3::new(SWING_HALF_WIDTH + 0.35, SWING_TOP.y * 0.5 + 0.12, 0.9),
            ),
            Self::Garden => ball(GARDEN + Vec3::Y * 0.12, 0.95),
            Self::Birdbath => ball(BIRDBATH + Vec3::Y * 0.45, 0.6),
            Self::Bench => ball(BENCH + Vec3::Y * 0.45, 0.85),
            Self::Lantern => ball(
                LANTERN + Vec3::Y * (LANTERN_HEIGHT * 0.5 + 0.1),
                LANTERN_HEIGHT * 0.5 + 0.3,
            ),
            Self::Mailbox => ball(MAILBOX + Vec3::Y * 0.65, 0.68),
            Self::Clothesline => (
                (CLOTHESLINE[0] + CLOTHESLINE[1]) * 0.5 + Vec3::Y * 0.85,
                ((CLOTHESLINE[1] - CLOTHESLINE[0]) * 0.5).abs() + Vec3::new(0.4, 0.85, 0.4),
            ),
            Self::Bush(center, radius) => ball(center + Vec3::Y * radius * 0.55, radius * 1.15),
            Self::Sill => (SILL, SILL_HALF + Vec3::splat(0.02)),
            Self::Snowman(_) => ball(SNOWMAN + Vec3::Y * 0.65, 0.75),
            Self::Door(_) => ball(door_hinge() + Vec3::Y * 0.9, 1.15),
            Self::Pumpkin => ball(PUMPKIN + Vec3::Y * 0.15, 0.24),
            Self::SnowGhost(..) => ball(SNOW_GHOST + Vec3::Y * 0.3, 0.42),
            Self::Chimney => (
                CHIMNEY + Vec3::Y * ((CHIMNEY_TOP + 2.55) * 0.5),
                Vec3::new(
                    CHIMNEY_HALF + 0.05,
                    (CHIMNEY_TOP - 2.55) * 0.5 + 0.05,
                    CHIMNEY_HALF + 0.05,
                ),
            ),
        }
    }

    /// Where a ray from `origin` passes through the piece's bounds.
    fn span(&self, origin: Vec3, direction: Vec3) -> Option<(f32, f32)> {
        match *self {
            Self::Fence(start, end) => {
                let center = (start + end) * 0.5 + Vec3::Y * 0.45;
                let half = ((end - start) * 0.5).abs() + Vec3::new(0.08, 0.5, 0.08);
                box_span(origin, direction, center, half)
            }
            Self::Tree => {
                let crown = sphere_span(origin, direction, TREE_CROWN, TREE_CROWN_RADIUS + 0.75);
                let wood = box_span(
                    origin,
                    direction,
                    TREE + Vec3::new(-0.2, 2.3, 0.35),
                    Vec3::new(1.85, 2.35, 1.0),
                );
                match (crown, wood) {
                    (Some(first), Some(second)) => {
                        Some((first.0.min(second.0), first.1.max(second.1)))
                    }
                    (one, other) => one.or(other),
                }
            }
            Self::Swing => box_span(
                origin,
                direction,
                SWING_TOP - Vec3::Y * (SWING_TOP.y * 0.5 - 0.05),
                Vec3::new(SWING_HALF_WIDTH + 0.35, SWING_TOP.y * 0.5 + 0.12, 0.9),
            ),
            Self::Garden => sphere_span(origin, direction, GARDEN + Vec3::Y * 0.12, 0.95),
            Self::Birdbath => sphere_span(origin, direction, BIRDBATH + Vec3::Y * 0.45, 0.6),
            Self::Bench => sphere_span(origin, direction, BENCH + Vec3::Y * 0.45, 0.85),
            Self::Lantern => sphere_span(
                origin,
                direction,
                LANTERN + Vec3::Y * (LANTERN_HEIGHT * 0.5 + 0.1),
                LANTERN_HEIGHT * 0.5 + 0.3,
            ),
            Self::Mailbox => sphere_span(origin, direction, MAILBOX + Vec3::Y * 0.65, 0.68),
            Self::Clothesline => {
                let center = (CLOTHESLINE[0] + CLOTHESLINE[1]) * 0.5 + Vec3::Y * 0.85;
                let half =
                    ((CLOTHESLINE[1] - CLOTHESLINE[0]) * 0.5).abs() + Vec3::new(0.4, 0.85, 0.4);
                box_span(origin, direction, center, half)
            }
            Self::Bush(center, radius) => sphere_span(
                origin,
                direction,
                center + Vec3::Y * radius * 0.55,
                radius * 1.15,
            ),
            Self::Sill => box_span(origin, direction, SILL, SILL_HALF + Vec3::splat(0.02)),
            Self::Snowman(_) => sphere_span(origin, direction, SNOWMAN + Vec3::Y * 0.65, 0.75),
            Self::Door(_) => sphere_span(origin, direction, door_hinge() + Vec3::Y * 0.9, 1.15),
            Self::Pumpkin => sphere_span(origin, direction, PUMPKIN + Vec3::Y * 0.15, 0.24),
            Self::SnowGhost(..) => sphere_span(origin, direction, SNOW_GHOST + Vec3::Y * 0.3, 0.42),
            Self::Chimney => box_span(
                origin,
                direction,
                CHIMNEY + Vec3::Y * ((CHIMNEY_TOP + 2.55) * 0.5),
                Vec3::new(
                    CHIMNEY_HALF + 0.05,
                    (CHIMNEY_TOP - 2.55) * 0.5 + 0.05,
                    CHIMNEY_HALF + 0.05,
                ),
            ),
        }
    }
}

/// The door's hinge foot, at its front edge on the outside of the wall.
fn door_hinge() -> Vec3 {
    Vec3::new(HOUSE_MAX.x, 0.0, DOOR_HIGH.x)
}

/// Which yard pieces and outdoor things a ray may meet, by index.
#[derive(Clone, Copy, Debug)]
pub struct Cull<'c> {
    pub pieces: &'c [u16],
    pub furniture: &'c [u16],
}

/// A point light outdoors.
struct Glow {
    position: Vec3,
    color: Vec3,
    falloff: f32,
}

/// A bird or squirrel drawn facing the viewer, or a leaf or petal falling
/// from the big tree.
#[derive(Clone, Copy, Debug)]
enum Critter {
    Bird(BirdPicture, Vec3),
    Squirrel(Passing, Vec3),
    Leaf(Vec3, Vec3),
    Cat(CatPicture, Vec3),
}

/// The outdoors for one frame.
pub struct Yard<'a> {
    outside: &'a Outside,
    heavens: Heavens<'a>,
    sky: Sky,
    plant: &'a Plant,
    furniture: &'a [Furniture<'a>],
    pieces: Vec<Piece>,
    critters: Vec<Critter>,
    sun: Vec3,
    sunlight: Vec3,
    moon: Vec3,
    moonlight: Vec3,
    ambient: Vec3,
    haze: Vec3,
    glows: Vec<Glow>,
    /// Creatures out in the yard, for their soft shadows: where each body
    /// is and how far it reaches.
    shadows: Vec<(Vec3, f32)>,
    lantern: f32,
    swing: f32,
    flag: bool,
    gust: f32,
    wind: f32,
    wet: f32,
    view: View,
    door: f32,
    smoke: bool,
    /// Fairy lights along the front eave for the holidays.
    lights: bool,
    roofless: bool,
}

/// What the scene tells the yard each frame.
pub struct Setting<'a> {
    pub outside: &'a Outside,
    pub plant: &'a Plant,
    pub furniture: &'a [Furniture<'a>],
    pub view: View,
    /// Ghost glows that light the yard: position, color, falloff.
    pub glows: Vec<(Vec3, Vec3, f32)>,
    pub shadows: Vec<(Vec3, f32)>,
    pub lantern: bool,
    pub swing: f32,
    pub flag: bool,
    pub gust: f32,
    pub wet: f32,
    /// How far the door stands open, 0 to 1.
    pub door: f32,
    /// Whether smoke curls from the chimney.
    pub smoke: bool,
    /// The lamp's light spilling out the window and the glass at night.
    pub spill: Vec3,
    pub occasion: Option<Occasion>,
    /// Lift the roof off, as when looking down into the room from above.
    pub roofless: bool,
    /// The ghost's snow sculpture: how built and how melted.
    pub snow_ghost: Option<(f32, f32)>,
    /// Seconds since the viewer shook the big tree, while it rains leaves.
    pub shaken: Option<f32>,
}

impl<'a> Yard<'a> {
    pub fn new(setting: Setting<'a>) -> Self {
        let outside = setting.outside;
        let sky = Sky::at_hour(outside.hour);
        let mut pieces = vec![
            Piece::Tree,
            Piece::Fence(
                Vec3::new(LOT_MIN.x, 0.0, LOT_MIN.y),
                Vec3::new(LOT_MAX.x, 0.0, LOT_MIN.y),
            ),
            Piece::Fence(
                Vec3::new(LOT_MIN.x, 0.0, LOT_MIN.y),
                Vec3::new(LOT_MIN.x, 0.0, LOT_MAX.y),
            ),
            Piece::Fence(
                Vec3::new(LOT_MAX.x, 0.0, LOT_MIN.y),
                Vec3::new(LOT_MAX.x, 0.0, LOT_MAX.y),
            ),
            Piece::Swing,
            Piece::Garden,
            Piece::Birdbath,
            Piece::Bench,
            Piece::Lantern,
            Piece::Mailbox,
            Piece::Clothesline,
            Piece::Sill,
        ];
        if !setting.roofless {
            pieces.push(Piece::Chimney);
        }
        if let Some((built, melt)) = setting.snow_ghost.filter(|(built, _)| *built > 0.02) {
            pieces.push(Piece::SnowGhost(built, melt));
        }
        if setting.door > 0.02 {
            pieces.push(Piece::Door(setting.door * 1.35));
        }
        pieces.extend(
            BUSHES
                .iter()
                .map(|&(center, radius)| Piece::Bush(center, radius)),
        );
        let mut critters = Vec::new();
        for passing in outside.passersby.iter().flatten() {
            match passing.kind {
                Passerby::Snowman => pieces.push(Piece::Snowman(passing.phase)),
                Passerby::Squirrel => {
                    critters.push(Critter::Squirrel(*passing, squirrel_at(passing)))
                }
                _ => {}
            }
        }
        if let Some(bird) = outside.bird {
            critters.push(Critter::Bird(bird, bird.position));
        }
        if let Some(cat) = outside.cat {
            critters.push(Critter::Cat(cat, cat.position + Vec3::Y * 0.24));
        }
        if matches!(outside.season, Season::Spring | Season::Autumn) {
            critters.extend(falling(outside));
        }
        if let Some(since) = setting.shaken {
            critters.extend(shaken(outside, since));
        }
        // Nearest first, so what they hide is skipped sooner.
        let eye = setting.view.eye;
        pieces.sort_by(|first, second| {
            piece_center(first)
                .distance_squared(eye)
                .total_cmp(&piece_center(second).distance_squared(eye))
        });
        let lantern = if setting.lantern { 1.0 } else { 0.0 };
        let mut glows: Vec<_> = setting
            .glows
            .iter()
            .map(|&(position, color, falloff)| Glow {
                position,
                color,
                falloff,
            })
            .collect();
        if lantern > 0.0 {
            glows.push(Glow {
                position: LANTERN + Vec3::Y * (LANTERN_HEIGHT + 0.1),
                color: LANTERN_LIGHT * 0.9,
                falloff: 1.4,
            });
        }
        if setting.spill.max_element() > 1e-3 {
            for position in [
                Vec3::new(0.0, 1.4, HOUSE_MIN.z - 0.5),
                Vec3::new(0.0, 1.0, HOUSE_MAX.z + 0.6),
            ] {
                glows.push(Glow {
                    position,
                    color: setting.spill,
                    falloff: 0.9,
                });
            }
        }
        let halloween = setting.occasion == Some(Occasion::Halloween);
        if halloween {
            pieces.push(Piece::Pumpkin);
            if lantern > 0.0 {
                glows.push(Glow {
                    position: PUMPKIN + Vec3::Y * 0.2,
                    color: Vec3::new(1.0, 0.55, 0.2) * 0.5,
                    falloff: 6.0,
                });
            }
        }
        let flash = outside.flash.clamp(0.0, 1.0);
        Self {
            sun: outside.sunlight_direction(),
            sunlight: outside.sunlight(),
            moon: outside.moonlight_direction(),
            moonlight: outside.moonlight(),
            ambient: sky::light_through(outside) * 0.95
                + Vec3::new(0.03, 0.035, 0.055)
                + Vec3::splat(flash * 0.2),
            haze: outside.haze(),
            heavens: Heavens::new(outside),
            sky,
            outside,
            plant: setting.plant,
            furniture: setting.furniture,
            pieces,
            critters,
            glows,
            shadows: setting.shadows,
            lantern,
            swing: setting.swing,
            flag: setting.flag,
            gust: setting.gust,
            wind: outside.wind,
            wet: setting.wet,
            view: setting.view,
            door: setting.door,
            smoke: setting.smoke && !setting.roofless,
            lights: matches!(
                setting.occasion,
                Some(Occasion::WinterHolidays | Occasion::NewYear)
            ),
            roofless: setting.roofless,
        }
    }

    /// The first thing outdoors that a ray meets between `from` and `to`:
    /// its distance and color. `cut` skips the cottage's walls, for rays
    /// on their way into the room.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub fn trace(
        &self,
        origin: Vec3,
        direction: Vec3,
        from: f32,
        to: f32,
        cut: bool,
        traveled: f32,
        spread: f32,
    ) -> Option<(f32, Vec3)> {
        self.trace_among(origin, direction, from, to, cut, traveled, spread, None)
    }

    /// As `trace`, testing only the pieces and things `cull` lists, when
    /// it does: those whose bounds cover the ray's part of the screen.
    #[allow(clippy::too_many_arguments)]
    pub fn trace_among(
        &self,
        origin: Vec3,
        direction: Vec3,
        from: f32,
        to: f32,
        cut: bool,
        traveled: f32,
        spread: f32,
        cull: Option<Cull>,
    ) -> Option<(f32, Vec3)> {
        let mut limit = to;
        let mut hit: Option<(f32, Vec3)> = None;

        // The ground, past the cottage's footprint.
        if direction.y < -1e-5 && origin.y > 0.0 {
            let distance = -origin.y / direction.y;
            if distance >= from && distance < limit {
                let point = origin + direction * distance;
                if !home::in_house(point) {
                    limit = distance;
                    let footprint = (traveled + distance) * spread / (-direction.y).max(0.08);
                    hit = Some((distance, self.ground(point, footprint, traveled + distance)));
                }
            }
        }

        if !cut
            && !self.roofless
            && let Some((distance, normal, surface)) = self.roof(origin, direction, from, limit)
        {
            limit = distance;
            let point = origin + direction * distance;
            hit = Some((distance, self.shade(surface, point, normal, direction)));
        }
        if !cut
            && let Some((distance, normal, surface)) = self.shell(origin, direction, from, limit)
        {
            limit = distance;
            let point = origin + direction * distance;
            hit = Some((
                distance,
                self.shade(surface, point, normal, direction) * SHELL_EXPOSURE,
            ));
        }

        let pieces: &mut dyn Iterator<Item = &Piece> = match cull {
            Some(cull) => &mut cull
                .pieces
                .iter()
                .map(|&index| &self.pieces[usize::from(index)]),
            None => &mut self.pieces.iter(),
        };
        for piece in pieces {
            let Some((enter, exit)) = piece.span(origin, direction) else {
                continue;
            };
            let mut distance = enter.max(from);
            let end = exit.min(limit);
            for _ in 0..80 {
                if distance > end {
                    break;
                }
                let point = origin + direction * distance;
                let (gap, surface) = self.distance(piece, point);
                let close = (0.0015 * distance).max((traveled + distance) * spread * 0.35);
                if gap < close.max(0.0015) {
                    limit = distance;
                    let normal = self.normal(piece, point);
                    hit = Some((distance, self.shade(surface, point, normal, direction)));
                    break;
                }
                distance += gap * 0.8;
            }
        }

        let things: &mut dyn Iterator<Item = &Furniture> = match cull {
            Some(cull) => &mut cull
                .furniture
                .iter()
                .map(|&index| &self.furniture[usize::from(index)]),
            None => &mut self.furniture.iter(),
        };
        for thing in things {
            let Some((distance, material)) = thing.trace(origin, direction, limit) else {
                continue;
            };
            if distance < from {
                continue;
            }
            limit = distance;
            let point = origin + direction * distance;
            let normal = thing.normal(point);
            let color = self.shade(Surface::Thing(material), point, normal, direction)
                + material.emission(self.outside.time, self.plant);
            hit = Some((distance, color));
        }

        // Birds and squirrels, drawn facing the viewer.
        for critter in &self.critters {
            let (position, reach) = match critter {
                Critter::Bird(_, position) => (*position, 0.2),
                Critter::Squirrel(_, position) => (*position, 0.16),
                Critter::Leaf(position, _) => (*position, 0.05),
                Critter::Cat(_, center) => (*center, 0.62),
            };
            let along = (position - origin).dot(self.view.forward)
                / direction.dot(self.view.forward).max(1e-4);
            if along <= from.max(0.0) || along >= limit {
                continue;
            }
            let offset = origin + direction * along - position;
            let local = Vec2::new(offset.dot(self.view.right), offset.dot(self.view.up));
            let pixel = ((traveled + along) * spread).max(1e-5);
            if local.length() > reach + pixel * 2.0 {
                continue;
            }
            let (cover, color) = match critter {
                Critter::Bird(bird, _) => bird_shade(local, pixel, *bird, self.outside.season),
                Critter::Squirrel(passing, _) => squirrel_shade(local, pixel, passing),
                Critter::Leaf(_, tint) => (
                    coverage((local * Vec2::new(1.0, 1.8)).length() - 0.028, pixel * 0.6),
                    *tint,
                ),
                Critter::Cat(cat, _) => cat_shade(local + Vec2::Y * 0.24, pixel, cat),
            };
            if cover > 0.0 {
                let lit = color * self.lit(position, -direction, 1.0).max(Vec3::splat(0.12));
                let behind = hit.map_or_else(|| self.background(direction, spread), |hit| hit.1);
                hit = Some((
                    if cover > 0.5 { along } else { limit },
                    behind.lerp(lit, cover),
                ));
                if cover > 0.5 {
                    limit = along;
                }
            }
        }
        hit
    }

    /// Boxes around each yard piece, in the order `Cull` counts them.
    pub fn piece_boxes(&self) -> impl Iterator<Item = (Vec3, Vec3)> + '_ {
        self.pieces.iter().map(Piece::bounding_box)
    }

    /// The sky, or the far ground fading into haze.
    pub fn background(&self, direction: Vec3, spread: f32) -> Vec3 {
        self.heavens.sky(direction, spread)
    }

    /// Rain and snow falling between `shelter` and `limit` along the ray,
    /// wherever it is outdoors. Rays looking into the room through the
    /// glass start past it, so the room stays clear to see.
    #[allow(clippy::too_many_arguments)]
    pub fn weather(
        &self,
        origin: Vec3,
        direction: Vec3,
        shelter: f32,
        limit: f32,
        traveled: f32,
        spread: f32,
        mut color: Vec3,
    ) -> Vec3 {
        let rain = self.outside.rain.clamp(0.0, 1.0);
        let snow = self.outside.snow.clamp(0.0, 1.0);
        if rain <= 0.0 && snow <= 0.0 {
            return color;
        }
        let azimuth = direction.x.atan2(-direction.z);
        for (layer, depth) in [1.4_f32, 2.6, 4.4, 7.0, 11.0].into_iter().enumerate() {
            if depth >= limit {
                break;
            }
            if depth < shelter {
                continue;
            }
            let point = origin + direction * depth;
            if point.y < 0.0 || home::in_house(point) && point.y < HOUSE_MAX.y {
                continue;
            }
            let pixel = ((traveled + depth) * spread).max(1e-4);
            let across = azimuth * depth + layer as f32 * 3.7;
            if rain > 0.0 {
                let falling = phase(self.outside.time, 7.5, 4096.0);
                let slant = across + point.y * (0.15 + self.wind * 0.35);
                let cells = Vec2::new(slant / 0.16, (point.y + falling) / 0.85);
                let cell = cells.floor();
                let seed = scramble(cell + Vec2::splat(layer as f32 * 13.0));
                if seed < rain * 0.9 {
                    let fraction = cells - cell;
                    let wide = ((fraction.x - (0.2 + 0.6 * scramble(cell + Vec2::X))).abs() * 0.16)
                        - 0.004;
                    let long = ((fraction.y - 0.5).abs() * 0.85) - 0.14;
                    let streak = coverage(wide.max(long), pixel * 0.6);
                    color += Vec3::splat(0.22) * streak * (1.0 - layer as f32 * 0.12);
                }
            }
            if snow > 0.0 {
                let falling = phase(self.outside.time, 0.45, 4096.0);
                let sway = (phase(self.outside.time, 0.7, TAU) + layer as f32).sin() * 0.06;
                let drifting = across + point.y * self.wind * 0.5 + sway;
                let cells = Vec2::new(drifting / 0.3, (point.y + falling) / 0.3);
                let cell = cells.floor();
                let seed = scramble(cell + Vec2::splat(layer as f32 * 7.0));
                if seed < snow * 0.55 {
                    let center =
                        Vec2::new(scramble(cell + Vec2::X), scramble(cell + Vec2::Y)) * 0.6 + 0.2;
                    let distance = ((cells - cell - center) * 0.3).length();
                    let flake = coverage(distance - 0.011, pixel * 0.6 + 0.003);
                    color = color.lerp(SNOW * (0.35 + self.ambient.max_element()), flake * 0.85);
                }
            }
        }
        color
    }

    /// Puffs of smoke curling up from the chimney and drifting off with the
    /// wind, laid over `color` where they are nearer than `limit`.
    pub fn smoke(
        &self,
        origin: Vec3,
        direction: Vec3,
        limit: f32,
        traveled: f32,
        spread: f32,
        mut color: Vec3,
    ) -> Vec3 {
        if !self.smoke {
            return color;
        }
        let top = CHIMNEY + Vec3::Y * (CHIMNEY_TOP + 0.08);
        let gray = Vec3::new(0.78, 0.78, 0.8) * (self.ambient * 1.2 + Vec3::splat(0.08));
        for index in 0..6 {
            let age = (phase(self.outside.time, 0.11, 1.0) + index as f32 / 6.0).fract();
            let wobble = (phase(self.outside.time, 0.9, TAU) + index as f32 * 2.1).sin();
            let center = top
                + Vec3::new(
                    (self.wind * 1.4 + 0.25) * age + wobble * 0.04 * age,
                    1.1 * age,
                    -0.15 * age,
                );
            let radius = 0.07 + 0.2 * age;
            let along = (center - origin).dot(direction);
            if along <= 0.0 || along >= limit {
                continue;
            }
            let miss = (origin + direction * along).distance(center);
            let pixel = (traveled + along) * spread;
            let puff = coverage(miss - radius, pixel + radius * 0.5);
            let fade = smoothstep(0.0, 0.08, age) * (1.0 - age);
            color = color.lerp(gray, puff * fade * 0.55);
        }
        color
    }

    /// Glowing specks outdoors: fireflies on summer nights, and the
    /// lantern's halo.
    pub fn sparkles(
        &self,
        origin: Vec3,
        direction: Vec3,
        limit: f32,
        traveled: f32,
        spread: f32,
    ) -> Vec3 {
        let mut light = Vec3::ZERO;
        if self.lantern > 0.0 {
            light += LANTERN_LIGHT
                * super::sprite(
                    origin,
                    direction,
                    LANTERN + Vec3::Y * (LANTERN_HEIGHT + 0.1),
                    limit + 0.2,
                    spread,
                    0.1,
                    traveled,
                )
                * 0.35
                * self.lantern;
        }
        let night = 1.0 - self.sky.day;
        if self.lights {
            let colors = [
                Vec3::new(1.0, 0.35, 0.35),
                Vec3::new(0.4, 0.8, 1.0),
                Vec3::new(1.0, 0.85, 0.35),
                Vec3::new(0.5, 1.0, 0.55),
            ];
            let left = HOUSE_MIN.x - GABLE + 0.1;
            let right = HOUSE_MAX.x + GABLE - 0.1;
            let count = ((right - left) / 0.26) as usize;
            // Only bulbs near the ray are worth a look.
            let eave = Vec3::new(0.0, EAVE_DROP - 0.04, HOUSE_MAX.z + EAVE - 0.03);
            let along = (eave - origin).dot(direction);
            if along > 0.0 && along < limit + 0.3 {
                let near = origin + direction * along;
                if (near.y - eave.y).abs() < 0.4 && (near.z - eave.z).abs() < 0.6 {
                    for index in 0..=count {
                        let x = left + (right - left) * index as f32 / count as f32;
                        let sag = 0.05 * ((index as f32 * 0.5).sin() * 0.5 + 0.5);
                        let bulb = Vec3::new(x, eave.y - sag, eave.z);
                        let twinkle = 0.55
                            + 0.45
                                * (phase(self.outside.time, 1.7, TAU) + index as f32 * 2.3).sin();
                        light += colors[index % colors.len()]
                            * super::sprite(
                                origin,
                                direction,
                                bulb,
                                limit + 0.1,
                                spread,
                                0.025,
                                traveled,
                            )
                            * twinkle
                            * (0.35 + 0.65 * night);
                    }
                }
            }
        }
        if self.outside.season == Season::Summer && night > 0.3 {
            let time = self.outside.time;
            for index in 0..8 {
                let seed = scramble(Vec2::new(index as f32, 41.0));
                let wander = phase(time, 0.11, TAU) + seed * TAU;
                let home = Vec3::new(
                    2.6 + seed * 5.6,
                    0.35 + scramble(Vec2::new(index as f32, 3.0)) * 0.9,
                    -3.8 + scramble(Vec2::new(index as f32, 9.0)) * 6.0,
                );
                let position = home
                    + Vec3::new(
                        wander.sin() * 0.45,
                        (wander * 2.0).cos() * 0.15,
                        (wander * 1.3).cos() * 0.4,
                    );
                let blink = (phase(time, 0.8, TAU) + seed * 19.0).sin().max(0.0).powi(6);
                light += Vec3::new(0.75, 1.0, 0.27)
                    * super::sprite(origin, direction, position, limit, spread, 0.03, traveled)
                    * blink
                    * night;
            }
        }
        light
    }

    fn shell(
        &self,
        origin: Vec3,
        direction: Vec3,
        from: f32,
        to: f32,
    ) -> Option<(f32, Vec3, Surface)> {
        let center = (HOUSE_MIN + HOUSE_MAX) * 0.5;
        let half = (HOUSE_MAX - HOUSE_MIN) * 0.5;
        let (enter, _) = box_span(origin, direction, center, half)?;
        if enter <= 1e-4 || enter < from || enter >= to {
            return None;
        }
        let point = origin + direction * enter;
        let offset = (point - center) / half;
        let axis = (0..3)
            .max_by(|first, second| offset[*first].abs().total_cmp(&offset[*second].abs()))
            .unwrap_or(0);
        let mut normal = Vec3::ZERO;
        normal[axis] = offset[axis].signum();
        let high = normal[axis] > 0.0;
        let surface = match (axis, high) {
            (2, true) => {
                // The glass front lets the ray in.
                let living = point.x > ROOM_MIN.x && point.x < ROOM_MAX.x;
                let bedroom = point.x > BEDROOM_MIN.x && point.x < BEDROOM_MAX.x;
                if (living || bedroom) && point.y < ROOM_MAX.y {
                    return None;
                }
                Surface::Section
            }
            (2, false) => {
                let at = Vec2::new(point.x, point.y);
                let (low, top, lift) = if point.x < home::PARTITION {
                    (BEDROOM_WINDOW_LOW, BEDROOM_WINDOW_HIGH, 0.0)
                } else {
                    (
                        super::window::WINDOW_LOW,
                        super::window::WINDOW_HIGH,
                        self.outside.window_lift,
                    )
                };
                if at.cmpgt(low).all() && at.cmplt(top).all() {
                    match super::window::pane_in(at, 0.01, lift, low, top) {
                        Some(pane) if pane.frame < 0.5 => return None,
                        _ => Surface::Trim,
                    }
                } else if at.cmpgt(low - Vec2::splat(0.09)).all()
                    && at.cmplt(top + Vec2::splat(0.09)).all()
                {
                    Surface::Trim
                } else if point.y < 0.14 {
                    Surface::Stone
                } else {
                    Surface::Siding
                }
            }
            (0, true) => {
                let at = Vec2::new(point.z, point.y);
                if at.cmpgt(DOOR_LOW).all() && at.cmplt(DOOR_HIGH).all() {
                    if self.door > 0.02 {
                        return None;
                    }
                    Surface::Door
                } else if at.cmpgt(DOOR_LOW - Vec2::new(0.08, 0.0)).all()
                    && at.cmplt(DOOR_HIGH + Vec2::splat(0.08)).all()
                {
                    Surface::Trim
                } else if point.y < 0.14 {
                    Surface::Stone
                } else {
                    Surface::Siding
                }
            }
            (1, true) => Surface::Cap,
            _ if point.y < 0.14 => Surface::Stone,
            _ => Surface::Siding,
        };
        Some((enter, normal, surface))
    }

    /// The low gable roof: slopes down to the eaves front and back, gable
    /// ends left and right, and the soffit underneath.
    fn roof(
        &self,
        origin: Vec3,
        direction: Vec3,
        from: f32,
        to: f32,
    ) -> Option<(f32, Vec3, Surface)> {
        let ridge_z = (HOUSE_MIN.z + HOUSE_MAX.z) * 0.5;
        let reach = (HOUSE_MAX.z - HOUSE_MIN.z) * 0.5 + EAVE;
        let slope = (RIDGE - EAVE_DROP) / reach;
        let rise = Vec3::new(0.0, 1.0, slope).normalize();
        let fall = Vec3::new(0.0, 1.0, -slope).normalize();
        // Each side as an outward normal and the plane's offset.
        let planes = [
            (Vec3::X, HOUSE_MAX.x + GABLE),
            (-Vec3::X, -(HOUSE_MIN.x - GABLE)),
            (-Vec3::Y, -EAVE_DROP),
            (rise, rise.dot(Vec3::new(0.0, RIDGE, ridge_z))),
            (fall, fall.dot(Vec3::new(0.0, RIDGE, ridge_z))),
        ];
        let mut enter = f32::NEG_INFINITY;
        let mut exit = f32::INFINITY;
        let mut normal = Vec3::Y;
        for (outward, offset) in planes {
            let toward = outward.dot(direction);
            let gap = offset - outward.dot(origin);
            if toward.abs() < 1e-8 {
                if gap < 0.0 {
                    return None;
                }
                continue;
            }
            let crossing = gap / toward;
            if toward < 0.0 {
                if crossing > enter {
                    enter = crossing;
                    normal = outward;
                }
            } else {
                exit = exit.min(crossing);
            }
        }
        if enter > exit || enter <= 1e-4 || enter < from || enter >= to {
            return None;
        }
        let surface = if normal.y > 0.5 {
            Surface::Shingle
        } else if normal.y < -0.5 {
            Surface::Trim
        } else {
            Surface::Siding
        };
        Some((enter, normal, surface))
    }

    fn distance(&self, piece: &Piece, point: Vec3) -> (f32, Surface) {
        match *piece {
            Piece::Tree => self.tree(point),
            Piece::Fence(start, end) => fence(point, start, end),
            Piece::Swing => self.swing_set(point),
            Piece::Garden => garden(point),
            Piece::Birdbath => self.birdbath(point),
            Piece::Bench => bench(point - BENCH),
            Piece::Lantern => lantern(point - LANTERN),
            Piece::Mailbox => mailbox(point - MAILBOX, self.flag),
            Piece::Clothesline => self.clothesline(point),
            Piece::Bush(center, radius) => bush(point - center, radius),
            Piece::Sill => (box_distance(point - SILL, SILL_HALF), Surface::Trim),
            Piece::Snowman(melt) => snowman(point - SNOWMAN, melt),
            Piece::Door(angle) => {
                let local = point - door_hinge();
                let (sine, cosine) = angle.sin_cos();
                // Turn back to where the leaf lies when shut, along -z.
                let turned = Vec3::new(
                    local.x * cosine + local.z * sine,
                    local.y,
                    local.z * cosine - local.x * sine,
                );
                let width = DOOR_HIGH.x - DOOR_LOW.x;
                let leaf = box_distance(
                    turned - Vec3::new(0.03, DOOR_HIGH.y * 0.5, -width * 0.5),
                    Vec3::new(0.025, DOOR_HIGH.y * 0.5, width * 0.5),
                );
                (leaf, Surface::Door)
            }
            Piece::SnowGhost(built, melt) => snow_ghost(point - SNOW_GHOST, built, melt),
            Piece::Pumpkin => {
                let local = point - PUMPKIN - Vec3::Y * 0.13;
                let ribs = (local.z.atan2(local.x) * 8.0).cos() * 0.008;
                let body = ellipsoid(local, Vec3::new(0.19, 0.13, 0.19)) + ribs;
                let stem = capsule(local, Vec3::Y * 0.1, Vec3::new(0.02, 0.19, 0.0), 0.022);
                if stem < body {
                    (stem, Surface::Bark)
                } else {
                    (body * 0.9, Surface::Pumpkin)
                }
            }
            Piece::Chimney => {
                let local = point - CHIMNEY;
                let stack = box_distance(
                    local - Vec3::Y * ((CHIMNEY_TOP + 2.55) * 0.5),
                    Vec3::new(CHIMNEY_HALF, (CHIMNEY_TOP - 2.55) * 0.5, CHIMNEY_HALF),
                );
                let cap = box_distance(
                    local - Vec3::Y * (CHIMNEY_TOP - 0.02),
                    Vec3::new(CHIMNEY_HALF + 0.035, 0.035, CHIMNEY_HALF + 0.035),
                );
                if cap < stack {
                    (cap, Surface::Stone)
                } else {
                    (stack, Surface::Brick)
                }
            }
        }
    }

    fn normal(&self, piece: &Piece, point: Vec3) -> Vec3 {
        let step = 0.003;
        [
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ]
        .iter()
        .map(|corner| *corner * self.distance(piece, point + *corner * step).0)
        .sum::<Vec3>()
        .normalize_or(Vec3::Y)
    }

    fn tree(&self, point: Vec3) -> (f32, Surface) {
        let mut local = point - TREE;
        // The wind leans the upper tree a little.
        let bend = ((local.y - 1.2).max(0.0) / 2.4).powi(2);
        local.x -= self.wind * self.gust.max(0.15) * 0.08 * bend;
        let trunk_fraction = (local.y / 2.7).clamp(0.0, 1.0);
        let trunk = capsule(
            local,
            Vec3::new(0.0, -0.3, 0.0),
            Vec3::new(-0.05, 2.7, 0.04),
            0.0,
        ) - mix(0.22, 0.1, trunk_fraction)
            - 0.08 * (1.0 - (local.y / 0.35).clamp(0.0, 1.0));
        let mut wood = trunk;
        let bare = self.outside.season == Season::Winter;
        // Twigs hide inside the crown unless the tree is bare.
        let limbs = if bare { BRANCHES.len() } else { 5 };
        for (start, end, radius) in &BRANCHES[..limbs] {
            wood = wood.min(limb(local, *start, *end, *radius));
        }
        let mut nearest = (wood, Surface::Bark);
        if !bare {
            let crown = local - (TREE_CROWN - TREE);
            // Far from the crown, its outer bound will do.
            let bound = crown.length() - (TREE_CROWN_RADIUS + 0.55);
            if bound > 0.3 {
                return (
                    nearest.0.min(bound),
                    if nearest.0 < bound {
                        Surface::Bark
                    } else {
                        Surface::Foliage
                    },
                );
            }
            let mut blob = crown.length() - 1.2;
            for (offset, radius) in [
                (Vec3::new(-0.95, -0.35, 0.25), 0.85),
                (Vec3::new(0.9, -0.3, -0.15), 0.9),
                (Vec3::new(0.05, 0.6, -0.15), 0.9),
                (Vec3::new(-0.25, -0.2, 0.8), 0.82),
                (Vec3::new(0.45, 0.15, 0.75), 0.75),
                (Vec3::new(-0.6, 0.35, -0.6), 0.8),
            ] {
                blob = smooth_min(blob, (crown - offset).length() - radius, 0.35);
            }
            let bumps =
                ((crown.x * 5.3).sin() + (crown.y * 5.9 + 1.3).sin() + (crown.z * 6.1 + 2.1).sin())
                    * 0.045;
            let crown = blob + bumps;
            if crown < nearest.0 {
                nearest = (crown, Surface::Foliage);
            }
        }
        nearest
    }

    fn swing_set(&self, point: Vec3) -> (f32, Surface) {
        let local = point - SWING_TOP;
        let reach = SWING_HALF_WIDTH + 0.15;
        let mut frame = capsule(
            local,
            Vec3::new(-reach, 0.0, 0.0),
            Vec3::new(reach, 0.0, 0.0),
            0.055,
        );
        for side in [-1.0, 1.0] {
            for lean in [-1.0, 1.0] {
                frame = frame.min(capsule(
                    local,
                    Vec3::new(side * reach, 0.0, 0.0),
                    Vec3::new(side * (reach + 0.12), -SWING_TOP.y, lean * 0.6),
                    0.045,
                ));
            }
        }
        let length = SWING_TOP.y - home::SWING_SEAT.y;
        let (sine, cosine) = self.swing.sin_cos();
        let seat = Vec3::new(0.0, -length * cosine, length * sine);
        let mut ropes = f32::INFINITY;
        for side in [-0.24, 0.24] {
            ropes = ropes.min(capsule(
                local,
                Vec3::new(side, 0.0, 0.0),
                seat + Vec3::new(side, 0.03, 0.0),
                0.013,
            ));
        }
        let relative = local - seat;
        let turned = Vec3::new(
            relative.x,
            relative.y * cosine + relative.z * sine,
            relative.z * cosine - relative.y * sine,
        );
        let board = box_distance(turned, Vec3::new(0.27, 0.025, 0.11)) - 0.01;
        let mut nearest = (frame, Surface::Timber);
        if ropes < nearest.0 {
            nearest = (ropes, Surface::Rope);
        }
        if board < nearest.0 {
            nearest = (board, Surface::Timber);
        }
        nearest
    }

    fn birdbath(&self, point: Vec3) -> (f32, Surface) {
        let local = point - BIRDBATH;
        let base = rounded_cylinder(local, 0.2, 0.07, 0.025);
        let radial = Vec2::new(local.x, local.z).length();
        let pedestal = Vec2::new(
            radial - mix(0.1, 0.065, (local.y / 0.75).clamp(0.0, 1.0)),
            (local.y - 0.38).abs() - 0.38,
        )
        .max_element();
        let bowl = rounded_cylinder(local - Vec3::Y * 0.72, 0.38, 0.12, 0.04);
        let hollow = rounded_cylinder(local - Vec3::Y * 0.79, 0.32, 0.2, 0.03);
        let stone = base.min(pedestal).min(bowl.max(-hollow));
        let water = rounded_cylinder(local - Vec3::Y * 0.76, 0.33, 0.05, 0.0);
        if water < stone {
            let frozen = self.outside.season == Season::Winter;
            (water, if frozen { Surface::Ice } else { Surface::Water })
        } else {
            (stone, Surface::Stone)
        }
    }

    fn clothesline(&self, point: Vec3) -> (f32, Surface) {
        let [first, second] = CLOTHESLINE;
        let top = Vec3::Y * CLOTHESLINE_HEIGHT;
        let mut wood = capsule(point, first, first + top, 0.045).min(capsule(
            point,
            second,
            second + top,
            0.045,
        ));
        let line = (second - first).normalize();
        let across = line.cross(Vec3::Y);
        for post in [first, second] {
            wood = wood.min(capsule(
                point,
                post + top - across * 0.25,
                post + top + across * 0.25,
                0.035,
            ));
        }
        let middle = (first + second) * 0.5 + top - Vec3::Y * 0.12;
        let rope = capsule(point, first + top, middle, 0.008).min(capsule(
            point,
            middle,
            second + top,
            0.008,
        ));
        let mut nearest = (wood, Surface::Timber);
        if rope < nearest.0 {
            nearest = (rope, Surface::Rope);
        }
        let tilt = self.wind * (0.3 + 0.5 * self.gust);
        for (index, fraction) in [0.27_f32, 0.5, 0.73].into_iter().enumerate() {
            let sag = 0.12 * (1.0 - (fraction * 2.0 - 1.0).powi(2));
            let hang = first + top + (second - first) * fraction - Vec3::Y * (sag + 0.01);
            let relative = point - hang;
            let along = relative.dot(line);
            let out = relative.dot(across);
            let down = relative.y;
            let flutter =
                (phase(self.outside.time, 2.3, TAU) + index as f32 * 1.7).sin() * self.gust * 0.1;
            let (sine, cosine) = (tilt + flutter).sin_cos();
            let turned_out = out * cosine - down * sine;
            let turned_down = down * cosine + out * sine;
            let size = [
                Vec2::new(0.26, 0.34),
                Vec2::new(0.17, 0.24),
                Vec2::new(0.15, 0.2),
            ][index];
            let cloth = box_distance(
                Vec3::new(along, turned_down + size.y, turned_out),
                Vec3::new(size.x, size.y, 0.008),
            ) * 0.8;
            if cloth < nearest.0 {
                nearest = (cloth, Surface::Cloth(index as u8));
            }
        }
        nearest
    }

    /// Light falling on an outdoor point facing `normal`.
    pub fn light(&self, point: Vec3, normal: Vec3) -> Vec3 {
        self.lit(point, normal, 0.0)
    }

    /// Light with `wrap` letting some of it come around the edges, as it
    /// glows through leaves.
    fn lit(&self, point: Vec3, normal: Vec3, wrap: f32) -> Vec3 {
        let mut total = self.ambient * (0.72 + 0.28 * normal.y.max(-0.5));
        let facing = (normal.dot(self.sun) + wrap) / (1.0 + wrap);
        if facing > 0.0 && self.sunlight.max_element() > 1e-3 {
            total += self.sunlight * facing * self.shadow(point, normal, self.sun);
        }
        // Moonlight is soft enough to need no shadows.
        let facing = (normal.dot(self.moon) + wrap) / (1.0 + wrap);
        if facing > 0.0 && self.moonlight.max_element() > 1e-3 {
            total += self.moonlight * facing;
        }
        for glow in &self.glows {
            let offset = glow.position - point;
            let distance_squared = offset.length_squared();
            // Far off, a little light has faded to nothing.
            if glow.falloff * distance_squared > 60.0 {
                continue;
            }
            let facing = normal
                .dot(offset / distance_squared.sqrt().max(1e-4))
                .max(0.0);
            total += glow.color * (0.25 + 0.75 * facing) / (1.0 + glow.falloff * distance_squared);
        }
        total
    }

    /// How much of the light from `toward` reaches the point: the house
    /// blocks it, the tree's crown dapples it.
    fn shadow(&self, point: Vec3, normal: Vec3, toward: Vec3) -> f32 {
        let start = point + normal * 0.01;
        let center = (HOUSE_MIN + HOUSE_MAX) * 0.5;
        let half = (HOUSE_MAX - HOUSE_MIN) * 0.5;
        if box_span(start, toward, center, half).is_some_and(|(_, exit)| exit > 0.0)
            && !home::in_house(start)
        {
            return 0.0;
        }
        let mut light = 1.0;
        if self.outside.season != Season::Winter
            && point.distance(TREE_CROWN) > TREE_CROWN_RADIUS + 0.4
            && sphere_span(start, toward, TREE_CROWN, TREE_CROWN_RADIUS)
                .is_some_and(|(enter, _)| enter > 0.0)
        {
            light *= 0.3;
        }
        light
    }

    fn shade(&self, surface: Surface, point: Vec3, normal: Vec3, direction: Vec3) -> Vec3 {
        let cloud = self.outside.rain.max(self.outside.snow * 0.85);
        let mut albedo = self.albedo(surface, point);
        let snowy = !matches!(
            surface,
            Surface::Water | Surface::Glass | Surface::Thing(_) | Surface::Snow | Surface::Door
        );
        if snowy && self.outside.snow_cover > 0.0 {
            let cover = self.outside.snow_cover.clamp(0.0, 1.0) * smoothstep(0.45, 0.85, normal.y);
            albedo = albedo.lerp(SNOW, cover);
        }
        let wrap = match surface {
            Surface::Foliage | Surface::Hedge => 0.8,
            Surface::Cloth(_) => 0.5,
            _ => 0.0,
        };
        let mut color = if surface == Surface::Section {
            // The cut wall shows evenly, whichever way the sun is.
            albedo * (self.ambient + Vec3::splat(0.22 * self.sky.day + 0.04))
        } else {
            albedo * self.lit(point, normal, wrap)
        };
        match surface {
            Surface::Water | Surface::Ice => {
                let glance = (1.0 - normal.dot(-direction).abs()).powi(3);
                color += self.haze * (0.25 + 0.5 * glance);
            }
            Surface::Glass => color += LANTERN_LIGHT * self.lantern * 1.2,
            Surface::Pumpkin => {
                // A candle glows through its face toward the yard.
                let face = normal.dot(Vec3::new(1.0, 0.0, 0.3).normalize()).max(0.0);
                let carved = ((point.y - PUMPKIN.y - 0.15).abs() < 0.03
                    || (point.y - PUMPKIN.y - 0.1).abs() < 0.015)
                    && face > 0.6;
                if carved {
                    color = Vec3::new(1.0, 0.7, 0.25) * (0.35 + 0.65 * self.lantern);
                }
            }
            _ => {}
        }
        if self.wet > 0.0 && normal.y > 0.5 && !matches!(surface, Surface::Thing(_)) {
            color *= 1.0 - self.wet * 0.18;
        }
        weather_color(color, cloud * 0.4)
    }

    fn albedo(&self, surface: Surface, point: Vec3) -> Vec3 {
        let season = self.outside.season;
        match surface {
            Surface::Bark => {
                Vec3::new(0.42, 0.31, 0.24)
                    * (0.8 + 0.35 * noise(Vec2::new(point.x + point.z, point.y) * 9.0))
            }
            Surface::Foliage => {
                let texture = noise(Vec2::new(point.x * 2.7 + point.z, point.y * 2.7)) * 0.6
                    + noise(Vec2::new(point.x - point.z, point.y) * 8.0) * 0.4;
                let leaves = match season {
                    Season::Spring => {
                        Vec3::new(0.55, 0.8, 0.42).lerp(Vec3::new(0.78, 0.92, 0.55), texture)
                    }
                    Season::Summer => {
                        Vec3::new(0.3, 0.6, 0.32).lerp(Vec3::new(0.5, 0.78, 0.36), texture)
                    }
                    Season::Autumn => {
                        Vec3::new(0.88, 0.3, 0.18).lerp(Vec3::new(1.0, 0.76, 0.28), texture)
                    }
                    Season::Winter => Vec3::new(0.4, 0.42, 0.4),
                };
                if season == Season::Spring {
                    let cells = Vec2::new(point.x + point.z * 0.7, point.y) * 9.0;
                    let cell = cells.floor();
                    let center =
                        Vec2::new(scramble(cell + Vec2::X), scramble(cell + Vec2::Y)) * 0.5 + 0.25;
                    let blossom = coverage((cells - cell - center).length() - 0.28, 0.12)
                        * smoothstep(0.35, 0.6, scramble(cell));
                    leaves.lerp(Vec3::new(1.0, 0.8, 0.88), blossom)
                } else {
                    leaves
                }
            }
            Surface::Hedge => {
                let texture = noise(Vec2::new(point.x + point.z, point.y) * 7.0);
                match season {
                    Season::Autumn => Vec3::new(0.5, 0.42, 0.22),
                    Season::Winter => Vec3::new(0.26, 0.36, 0.28),
                    _ => Vec3::new(0.22, 0.45, 0.25),
                }
                .lerp(Vec3::new(0.36, 0.6, 0.32), texture * 0.6)
            }
            Surface::Picket | Surface::Trim => Vec3::new(0.93, 0.92, 0.87),
            Surface::Timber => {
                Vec3::new(0.8, 0.6, 0.42)
                    * (0.85
                        + 0.25 * noise(Vec2::new(point.x * 3.0, point.y * 40.0 + point.z * 3.0)))
            }
            Surface::Soil => Vec3::new(0.42, 0.3, 0.22),
            Surface::Stone => {
                Vec3::new(0.68, 0.69, 0.7)
                    * (0.9 + 0.15 * noise(Vec2::new(point.x + point.z, point.y) * 11.0))
            }
            Surface::Water => Vec3::new(0.3, 0.45, 0.55),
            Surface::Ice => Vec3::new(0.78, 0.88, 0.98),
            Surface::Iron => Vec3::new(0.3, 0.32, 0.36),
            Surface::Glass => {
                if self.lantern > 0.0 {
                    Vec3::new(1.0, 0.9, 0.7)
                } else {
                    Vec3::new(0.62, 0.66, 0.7)
                }
            }
            Surface::Mailbox => Vec3::new(0.38, 0.56, 0.78),
            Surface::Flag => Vec3::new(1.0, 0.3, 0.25),
            Surface::Rope => Vec3::new(0.82, 0.74, 0.58),
            Surface::Cloth(index) => match index {
                0 => Vec3::new(0.97, 0.96, 0.93),
                1 => Vec3::new(0.5, 0.7, 0.98),
                _ => Vec3::new(1.0, 0.66, 0.74),
            },
            Surface::Snow => SNOW,
            Surface::Coal => Vec3::splat(0.12),
            Surface::Carrot => Vec3::new(1.0, 0.55, 0.2),
            Surface::Scarf => Vec3::new(0.92, 0.3, 0.3),
            Surface::Siding => {
                let seam = ((point.y / 0.16).fract() - 0.05).abs() < 0.05;
                Vec3::new(0.58, 0.7, 0.66) * if seam { 0.75 } else { 1.0 }
            }
            Surface::Cap => Vec3::new(0.62, 0.5, 0.44),
            Surface::Shingle => {
                // Rows of shingles down the slope, staggered.
                let row = (point.y / 0.07).floor();
                let across = point.x / 0.22 + row * 0.5;
                let seam = ((point.y / 0.07).fract() < 0.18) || (across.fract() < 0.08);
                let tone = 0.88 + 0.2 * scramble(Vec2::new(across.floor(), row));
                Vec3::new(0.66, 0.36, 0.3) * tone * if seam { 0.72 } else { 1.0 }
            }
            Surface::Brick => {
                let row = (point.y / 0.08).floor();
                let across = (point.x + point.z) / 0.16 + row * 0.5;
                let seam = ((point.y / 0.08).fract() < 0.15) || (across.fract() < 0.1);
                Vec3::new(0.7, 0.4, 0.32) * if seam { 0.65 } else { 1.0 }
            }
            Surface::Door => Vec3::new(0.62, 0.42, 0.3),
            Surface::Pumpkin => Vec3::new(1.0, 0.5, 0.16),
            Surface::Section => Vec3::new(0.78, 0.66, 0.52),
            Surface::Thing(material) => material.albedo(self.plant),
        }
    }

    fn ground(&self, point: Vec3, footprint: f32, distance: f32) -> Vec3 {
        let at = Vec2::new(point.x, point.z);
        let season = self.outside.season;
        let lawn = at.cmpge(LOT_MIN).all() && at.cmple(LOT_MAX).all();
        let grass = match season {
            Season::Spring => Vec3::new(0.42, 0.66, 0.34),
            Season::Summer => Vec3::new(0.32, 0.56, 0.29),
            Season::Autumn => Vec3::new(0.58, 0.55, 0.3),
            Season::Winter => Vec3::new(0.44, 0.49, 0.38),
        };
        let meadow = grass.lerp(Vec3::new(0.66, 0.62, 0.36), 0.3);
        let blur = smoothstep(0.08, 0.6, footprint);
        let texture = mix(
            noise(at * 1.3) * 0.6 + noise(at * 4.1 + Vec2::splat(3.0)) * 0.4,
            0.5,
            blur,
        );
        let mut albedo = if lawn { grass } else { meadow } * (0.82 + 0.3 * texture);
        if lawn {
            // Mown stripes.
            let stripe = ((at.x / 0.75).floor() as i32).rem_euclid(2) as f32;
            albedo *= 1.0 + mix(stripe * 0.07 - 0.035, 0.0, blur);
            // A worn patch under the swing.
            let worn = 1.0
                - smoothstep(
                    0.25,
                    0.6,
                    (at - Vec2::new(SWING_TOP.x, SWING_TOP.z)).length(),
                );
            albedo = albedo.lerp(Vec3::new(0.5, 0.4, 0.3), worn * 0.6);
            for (center, radius) in STONES {
                let stone = coverage((at - center).length() - radius, footprint * 0.5 + 0.01);
                albedo = albedo.lerp(Vec3::new(0.72, 0.71, 0.68), stone);
            }
            if season == Season::Autumn {
                let cells = at * 5.0;
                let cell = cells.floor();
                let near = 1.0 - smoothstep(1.2, 2.6, (at - Vec2::new(TREE.x, TREE.z)).length());
                let leaf = coverage(
                    (cells - cell - Vec2::splat(0.5)).length() - 0.2,
                    footprint * 2.5 + 0.05,
                ) * smoothstep(0.4, 0.7, scramble(cell))
                    * near;
                albedo = albedo.lerp(Vec3::new(0.95, 0.45, 0.2), leaf * (1.0 - blur));
            }
        } else if matches!(season, Season::Spring | Season::Summer) {
            let cells = at * 3.0;
            let cell = cells.floor();
            let flower = coverage(
                (cells - cell - Vec2::splat(0.5)).length() - 0.09,
                footprint * 1.5 + 0.02,
            ) * smoothstep(0.82, 0.9, scramble(cell));
            let tint = if scramble(cell + Vec2::X) < 0.5 {
                Vec3::new(1.0, 0.95, 0.6)
            } else {
                Vec3::new(1.0, 0.7, 0.85)
            };
            albedo = albedo.lerp(tint, flower * (1.0 - blur));
        }
        let cover = self.outside.snow_cover.clamp(0.0, 1.0);
        if cover > 0.0 {
            albedo = albedo.lerp(SNOW, (cover * (0.85 + 0.3 * texture)).min(1.0));
        }
        let mut color = albedo * self.light(point, Vec3::Y);
        color *= super::shadow_at(point, &self.shadows);
        if self.wet > 0.0 {
            color *= 1.0 - self.wet * 0.25;
            let puddle = smoothstep(0.62, 0.7, noise(at * 0.9 + Vec2::splat(11.0))) * self.wet;
            color = color.lerp(self.haze * 0.7, puddle * 0.55);
        }
        let cloud = self.outside.rain.max(self.outside.snow * 0.85);
        let color = weather_color(color, cloud * 0.4) * GROUND_EXPOSURE;
        color.lerp(self.haze, 1.0 - (-distance / 55.0).exp())
    }
}

/// The tree's limbs from the trunk, start, end, and radius, relative to its
/// foot; the twigs only show in winter, inside the crown otherwise.
const BRANCHES: [(Vec3, Vec3, f32); 15] = [
    (
        Vec3::new(0.0, 1.05, 0.0),
        Vec3::new(-1.3, 1.32, 0.85),
        0.075,
    ),
    (
        Vec3::new(0.0, 1.9, 0.0),
        Vec3::new(-0.95, 3.05, 0.25),
        0.075,
    ),
    (Vec3::new(0.0, 2.1, 0.0), Vec3::new(0.85, 3.2, -0.2), 0.07),
    (Vec3::new(0.0, 2.45, 0.0), Vec3::new(0.05, 3.95, 0.1), 0.065),
    (
        Vec3::new(0.0, 1.85, 0.0),
        Vec3::new(0.95, 2.55, 0.55),
        0.055,
    ),
    (
        Vec3::new(-0.95, 3.05, 0.25),
        Vec3::new(-1.55, 3.55, 0.5),
        0.03,
    ),
    (
        Vec3::new(-0.95, 3.05, 0.25),
        Vec3::new(-0.8, 3.85, -0.1),
        0.03,
    ),
    (
        Vec3::new(-0.5, 2.45, 0.12),
        Vec3::new(-1.4, 2.7, -0.35),
        0.03,
    ),
    (
        Vec3::new(0.85, 3.2, -0.2),
        Vec3::new(1.5, 3.65, -0.45),
        0.03,
    ),
    (Vec3::new(0.85, 3.2, -0.2), Vec3::new(0.7, 4.0, 0.2), 0.03),
    (
        Vec3::new(0.45, 2.65, -0.1),
        Vec3::new(1.35, 2.85, 0.35),
        0.028,
    ),
    (
        Vec3::new(0.05, 3.95, 0.1),
        Vec3::new(-0.4, 4.45, 0.05),
        0.025,
    ),
    (
        Vec3::new(0.05, 3.95, 0.1),
        Vec3::new(0.45, 4.4, -0.2),
        0.025,
    ),
    (
        Vec3::new(0.95, 2.55, 0.55),
        Vec3::new(1.5, 2.95, 1.0),
        0.028,
    ),
    (
        Vec3::new(-0.65, 1.19, 0.42),
        Vec3::new(-0.95, 1.75, 0.75),
        0.03,
    ),
];

/// About where a piece stands, for drawing order.
fn piece_center(piece: &Piece) -> Vec3 {
    match *piece {
        Piece::Tree => TREE + Vec3::Y * 2.0,
        Piece::Fence(start, end) => (start + end) * 0.5,
        Piece::Swing => SWING_TOP,
        Piece::Garden => GARDEN,
        Piece::Birdbath => BIRDBATH,
        Piece::Bench => BENCH,
        Piece::Lantern => LANTERN,
        Piece::Mailbox => home::MAILBOX,
        Piece::Clothesline => (CLOTHESLINE[0] + CLOTHESLINE[1]) * 0.5,
        Piece::Bush(center, _) => center,
        Piece::Sill => SILL,
        Piece::Snowman(_) => SNOWMAN,
        Piece::Door(_) => door_hinge(),
        Piece::Chimney => CHIMNEY + Vec3::Y * 3.0,
        Piece::Pumpkin => PUMPKIN,
        Piece::SnowGhost(..) => SNOW_GHOST,
    }
}

fn limb(point: Vec3, start: Vec3, end: Vec3, radius: f32) -> f32 {
    let along = end - start;
    let fraction = ((point - start).dot(along) / along.length_squared()).clamp(0.0, 1.0);
    point.distance(start + along * fraction) - radius * (1.0 - 0.55 * fraction)
}

fn fence(point: Vec3, start: Vec3, end: Vec3) -> (f32, Surface) {
    let run = end - start;
    let length = run.length();
    let along_axis = run / length;
    let across_axis = along_axis.cross(Vec3::Y);
    let local = point - start;
    let along = local.dot(along_axis);
    let across = local.dot(across_axis);
    let spacing = 0.2;
    let index = (along / spacing)
        .round()
        .clamp(0.0, (length / spacing).floor());
    let offset = along - index * spacing;
    let post = index % 12.0 == 0.0;
    let half = if post {
        Vec3::new(0.05, 0.48, 0.05)
    } else {
        Vec3::new(0.032, 0.4, 0.016)
    };
    let picket = box_distance(Vec3::new(offset, point.y - half.y, across), half)
        .max(point.y - half.y * 2.0 - 0.05 + offset.abs() * 1.2);
    let mut rails = f32::INFINITY;
    for height in [0.22, 0.6] {
        rails = rails.min(box_distance(
            Vec3::new(along - length * 0.5, point.y - height, across + 0.034),
            Vec3::new(length * 0.5, 0.03, 0.016),
        ));
    }
    (picket.min(rails) * 0.9, Surface::Picket)
}

fn garden(point: Vec3) -> (f32, Surface) {
    let local = point - GARDEN;
    let outer = box_distance(local - Vec3::Y * GARDEN_HALF.y, GARDEN_HALF);
    let inner = box_distance(
        local - Vec3::Y * (GARDEN_HALF.y * 2.0),
        GARDEN_HALF - Vec3::new(0.055, 0.0, 0.055),
    );
    let frame = outer.max(-inner);
    let soil = box_distance(
        local - Vec3::Y * 0.1,
        Vec3::new(GARDEN_HALF.x - 0.05, 0.1, GARDEN_HALF.z - 0.05),
    );
    if soil < frame {
        (soil, Surface::Soil)
    } else {
        (frame, Surface::Timber)
    }
}

fn bench(local: Vec3) -> (f32, Surface) {
    let seat = box_distance(local - Vec3::new(0.0, 0.45, 0.0), Vec3::new(0.7, 0.03, 0.2)) - 0.008;
    let back = box_distance(
        local - Vec3::new(0.0, 0.72, -0.19),
        Vec3::new(0.7, 0.13, 0.025),
    ) - 0.008;
    let mut legs = f32::INFINITY;
    for x in [-0.6, 0.6] {
        for z in [-0.15, 0.15] {
            legs = legs.min(box_distance(
                local - Vec3::new(x, 0.22, z),
                Vec3::new(0.035, 0.22, 0.035),
            ));
        }
        legs = legs.min(box_distance(
            local - Vec3::new(x, 0.62, -0.19),
            Vec3::new(0.035, 0.2, 0.03),
        ));
    }
    (seat.min(back).min(legs), Surface::Timber)
}

fn lantern(local: Vec3) -> (f32, Surface) {
    let post = box_distance(
        local - Vec3::Y * (LANTERN_HEIGHT * 0.5),
        Vec3::new(0.035, LANTERN_HEIGHT * 0.5, 0.035),
    );
    let head = local - Vec3::Y * (LANTERN_HEIGHT + 0.1);
    let glass = box_distance(head, Vec3::new(0.075, 0.1, 0.075));
    let cap = box_distance(head - Vec3::Y * 0.13, Vec3::new(0.11, 0.025, 0.11)).min(box_distance(
        head + Vec3::Y * 0.11,
        Vec3::new(0.09, 0.015, 0.09),
    ));
    let iron = post.min(cap);
    if glass < iron {
        (glass, Surface::Glass)
    } else {
        (iron, Surface::Iron)
    }
}

fn mailbox(local: Vec3, flag: bool) -> (f32, Surface) {
    let post = box_distance(local - Vec3::Y * 0.5, Vec3::new(0.04, 0.5, 0.04));
    let box_shape = box_distance(local - Vec3::Y * 1.08, Vec3::new(0.11, 0.08, 0.2)) - 0.025;
    let flag_shape = if flag {
        box_distance(
            local - Vec3::new(0.15, 1.22, -0.1),
            Vec3::new(0.008, 0.08, 0.025),
        )
    } else {
        box_distance(
            local - Vec3::new(0.15, 1.09, -0.06),
            Vec3::new(0.008, 0.025, 0.09),
        )
    };
    let mut nearest = (post, Surface::Timber);
    if box_shape < nearest.0 {
        nearest = (box_shape, Surface::Mailbox);
    }
    if flag_shape < nearest.0 {
        nearest = (flag_shape, Surface::Flag);
    }
    nearest
}

fn bush(local: Vec3, radius: f32) -> (f32, Surface) {
    let center = Vec3::Y * radius * 0.55;
    let shape = ellipsoid(local - center, Vec3::new(radius, radius * 0.75, radius));
    let bumps =
        ((local.x * 9.0).sin() + (local.y * 8.0 + 1.0).sin() + (local.z * 9.5 + 2.0).sin()) * 0.025;
    ((shape + bumps) * 0.85, Surface::Hedge)
}

/// A little ghost of packed snow: a round head on a flared skirt with a
/// wavy hem, coal eyes, growing as it is built and slumping as it melts.
fn snow_ghost(local: Vec3, built: f32, melt: f32) -> (f32, Surface) {
    let size = (0.25 + 0.75 * built.clamp(0.0, 1.0)) * (1.0 - 0.35 * melt.clamp(0.0, 1.0));
    let local = local / size;
    let head = (local - Vec3::Y * 0.42).length() - 0.17;
    let radial = Vec2::new(local.x, local.z).length();
    let angle = local.z.atan2(local.x);
    // The skirt widens toward its wavy hem on the snow.
    let flare = 0.16 + 0.1 * (1.0 - (local.y / 0.42).clamp(0.0, 1.0));
    let hem = 0.025 * (angle * 5.0).sin();
    let skirt = Vec2::new(radial - flare, (local.y - 0.21).abs() - 0.21 - hem).max_element();
    let body = smooth_min(head, skirt, 0.08);
    let mut nearest = (body * size, Surface::Snow);
    if built > 0.8 {
        for side in [-1.0, 1.0] {
            let eye = (local - Vec3::new(side * 0.06, 0.45, 0.15)).length() - 0.025;
            if eye * size < nearest.0 {
                nearest = (eye * size, Surface::Coal);
            }
        }
    }
    nearest
}

fn snowman(local: Vec3, melt: f32) -> (f32, Surface) {
    let melt = melt.clamp(0.0, 1.0);
    let squash = 1.0 - melt * 0.6;
    let shrink = 1.0 - melt * 0.3;
    let heights = [0.3, 0.76, 1.1].map(|height| height * squash);
    let radii = [0.32, 0.23, 0.16].map(|radius| radius * shrink);
    let mut body = (local - Vec3::Y * heights[0]).length() - radii[0];
    for (height, radius) in heights.into_iter().zip(radii).skip(1) {
        body = smooth_min(body, (local - Vec3::Y * height).length() - radius, 0.04);
    }
    let head = Vec3::Y * heights[2];
    let mut nearest = (body, Surface::Snow);
    let nose = capsule(
        local,
        head + Vec3::new(0.0, 0.0, radii[2] * 0.8),
        head + Vec3::new(0.0, -0.01, radii[2] + 0.12),
        0.022,
    );
    if nose < nearest.0 {
        nearest = (nose, Surface::Carrot);
    }
    for side in [-1.0, 1.0] {
        let eye = (local - head - Vec3::new(side * 0.055, 0.04, radii[2] * 0.88)).length() - 0.022;
        if eye < nearest.0 {
            nearest = (eye, Surface::Coal);
        }
        let arm = capsule(
            local,
            Vec3::new(side * radii[1] * 0.9, heights[1] + 0.05, 0.0),
            Vec3::new(side * (radii[1] + 0.3), heights[1] + 0.25, 0.04),
            0.015,
        );
        if arm < nearest.0 {
            nearest = (arm, Surface::Bark);
        }
    }
    let scarf = rounded_cylinder(
        local - Vec3::Y * (heights[1] + radii[1] * 0.75),
        radii[2] + 0.03,
        0.06,
        0.02,
    );
    if scarf < nearest.0 {
        nearest = (scarf, Surface::Scarf);
    }
    nearest
}

/// Leaves in autumn, or blossom petals in spring, drifting down from the
/// big tree's crown and away on the wind.
fn falling(outside: &Outside) -> impl Iterator<Item = Critter> + '_ {
    (0..7).map(move |index| {
        let seed = scramble(Vec2::new(index as f32, 23.0));
        let fall = (phase(outside.time, 0.045, 1.0) + seed).fract();
        let sway = (phase(outside.time, 0.5, TAU) + seed * TAU).sin();
        let start = TREE_CROWN
            + Vec3::new(
                (seed - 0.5) * 2.4,
                -0.6 + scramble(Vec2::new(index as f32, 4.0)) * 0.8,
                (scramble(Vec2::new(index as f32, 7.0)) - 0.3) * 2.0,
            );
        let drop = start.y * fall;
        let drift = Vec3::new(
            (0.4 + outside.wind * 1.6) * fall + sway * 0.18,
            -drop,
            fall * 0.5 + (sway * 1.3).cos() * 0.1,
        );
        let tint = if outside.season == Season::Spring {
            Vec3::new(1.0, 0.8, 0.88)
        } else {
            Vec3::new(0.95, 0.38, 0.18).lerp(Vec3::new(1.0, 0.78, 0.3), seed)
        };
        Critter::Leaf(start + drift, tint)
    })
}

/// A shower from the crown just after the tree is shaken: petals, leaves
/// green or gold, or snow off bare branches.
fn shaken(outside: &Outside, since: f32) -> impl Iterator<Item = Critter> + '_ {
    (0..14).filter_map(move |index| {
        let seed = scramble(Vec2::new(index as f32, 61.0));
        let start_at = seed * 1.2;
        let fall = ((since - start_at) / 3.5).clamp(0.0, 1.0);
        if fall <= 0.0 || fall >= 1.0 {
            return None;
        }
        let start = TREE_CROWN
            + Vec3::new(
                (seed - 0.5) * 2.6,
                -0.9 + scramble(Vec2::new(index as f32, 5.0)) * 1.0,
                (scramble(Vec2::new(index as f32, 8.0)) - 0.3) * 2.2,
            );
        let sway = (since * 2.5 + seed * TAU).sin();
        let position = start
            + Vec3::new(
                outside.wind * 0.8 * fall + sway * 0.12,
                -start.y * fall * fall,
                fall * 0.4,
            );
        let tint = match outside.season {
            Season::Spring => Vec3::new(1.0, 0.8, 0.88),
            Season::Summer => Vec3::new(0.4, 0.75, 0.38),
            Season::Autumn => Vec3::new(0.95, 0.38, 0.18).lerp(Vec3::new(1.0, 0.78, 0.3), seed),
            Season::Winter => Vec3::new(0.9, 0.94, 1.0),
        };
        Some(Critter::Leaf(position, tint))
    })
}

/// Where the squirrel is on the tree as it scurries up the trunk and out
/// along the low branch, and back.
fn squirrel_at(passing: &Passing) -> Vec3 {
    let along = if passing.phase < 0.4 {
        passing.phase / 0.4
    } else if passing.phase < 0.65 {
        1.0
    } else {
        (1.0 - passing.phase) / 0.35
    };
    let up = TREE + Vec3::new(-0.05, 0.25, 0.24);
    let fork = TREE + Vec3::new(-0.05, 1.35, 0.2);
    let out = fork.lerp(PERCH + Vec3::Y * 0.08, 0.6);
    if along < 0.6 {
        up.lerp(fork, along / 0.6)
    } else {
        fork.lerp(out, (along - 0.6) / 0.4)
    }
}

/// How much of a pixel the bird covers at `local`, in room units on its
/// billboard, and its color.
fn bird_shade(local: Vec2, pixel: f32, bird: BirdPicture, season: Season) -> (f32, Vec3) {
    // Drawn at picture scale and enlarged to room size.
    const SCALE: f32 = 2.0;
    let footprint = (pixel / SCALE).max(1e-5);
    let mut point = local / SCALE;
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
        .min(crate::sdf::capsule(
            point,
            Vec2::new(-0.02, -0.01),
            Vec2::new(-0.065, 0.004),
            0.009,
        ))
        .min(crate::sdf::capsule(
            point,
            head + Vec2::new(0.012, 0.0),
            head + Vec2::new(0.036, -0.005),
            0.004,
        ));
    if bird.flying {
        let wing = Vec2::new(-0.015, bird.flap.sin() * 0.07);
        distance = distance.min(crate::sdf::capsule(point, Vec2::ZERO, wing, 0.012));
        distance = distance.min(crate::sdf::capsule(
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
        Vec3::new(1.0, 0.36, 0.2)
    } else {
        Vec3::new(1.0, 0.58, 0.3)
    };
    let feather = Vec3::new(0.62, 0.5, 0.4).lerp(warm, breast);
    let eye = coverage(
        point.distance(head + Vec2::new(0.008, 0.006)) - 0.004,
        footprint * 0.5,
    );
    (
        coverage(distance, footprint * 0.5),
        feather.lerp(Vec3::splat(0.05), eye),
    )
}

/// How much of a pixel the cat covers at `local`, measured from its feet
/// in room units on its billboard, and its color there.
fn cat_shade(local: Vec2, pixel: f32, cat: &CatPicture) -> (f32, Vec3) {
    use crate::sdf::capsule as line;
    // Drawn at a small size and enlarged.
    const SCALE: f32 = 1.5;
    let edge = (pixel * 0.5 / SCALE).max(1e-4);
    let mut point = local / SCALE;
    point.x *= cat.facing;
    let blob = |point: Vec2, center: Vec2, radii: Vec2| {
        ((point - center) / radii).length() * radii.min_element() - radii.min_element()
    };
    let ears = |head: Vec2, size: f32| {
        line(
            point,
            head + Vec2::new(-0.035, 0.04) * size,
            head + Vec2::new(-0.045, 0.085) * size,
            0.017,
        )
        .min(line(
            point,
            head + Vec2::new(0.025, 0.045) * size,
            head + Vec2::new(0.035, 0.09) * size,
            0.017,
        ))
    };
    let (distance, head, eyes_open) = match cat.pose {
        Pose::Walking | Pose::Running => {
            let running = cat.pose == Pose::Running;
            let stretch = if running { 1.2 } else { 1.0 };
            let body = blob(point, Vec2::new(0.0, 0.16), Vec2::new(0.16 * stretch, 0.07));
            let head = if running {
                Vec2::new(0.2, 0.2)
            } else {
                Vec2::new(0.17, 0.25)
            };
            let mut distance = body.min(point.distance(head) - 0.065).min(ears(head, 1.0));
            let stride = if running { 0.09 } else { 0.045 };
            for (x, offset) in [(0.1, 0.0), (0.06, 3.1), (-0.1, 1.6), (-0.14, 4.7)] {
                let swing = (cat.step + offset).sin() * stride;
                distance = distance.min(line(
                    point,
                    Vec2::new(x * stretch, 0.13),
                    Vec2::new(x * stretch + swing, 0.0),
                    0.021,
                ));
            }
            let tip = Vec2::new(-0.3, 0.33 + cat.tail * 0.04);
            distance = distance
                .min(line(
                    point,
                    Vec2::new(-0.15 * stretch, 0.18),
                    Vec2::new(-0.25, 0.24),
                    0.019,
                ))
                .min(line(point, Vec2::new(-0.25, 0.24), tip, 0.017));
            (distance, head, true)
        }
        Pose::Sitting => {
            let body = blob(point, Vec2::new(-0.02, 0.13), Vec2::new(0.1, 0.13))
                .min(point.distance(Vec2::new(0.04, 0.15)) - 0.07);
            let head = Vec2::new(0.04, 0.3);
            let mut distance = body
                .min(point.distance(head) - 0.068)
                .min(ears(head, 1.0))
                .min(line(
                    point,
                    Vec2::new(0.07, 0.12),
                    Vec2::new(0.08, 0.0),
                    0.02,
                ));
            let curl = Vec2::new(0.14, 0.04 + cat.tail * 0.015);
            distance = distance
                .min(line(
                    point,
                    Vec2::new(-0.1, 0.03),
                    Vec2::new(0.1, 0.02),
                    0.018,
                ))
                .min(line(point, Vec2::new(0.1, 0.02), curl, 0.016));
            if cat.washing {
                distance = distance.min(line(
                    point,
                    Vec2::new(0.07, 0.15),
                    Vec2::new(0.1, 0.27),
                    0.02,
                ));
            }
            (distance, head, true)
        }
        Pose::Napping => {
            let loaf = blob(point, Vec2::new(0.0, 0.085), Vec2::new(0.16, 0.085));
            let head = Vec2::new(0.12, 0.11);
            let distance = loaf
                .min(point.distance(head) - 0.062)
                .min(ears(head, 0.75))
                .min(line(
                    point,
                    Vec2::new(-0.13, 0.03),
                    Vec2::new(0.1, 0.015),
                    0.022,
                ));
            (distance, head, false)
        }
    };
    let (base, patch) = match cat.coat {
        Coat::Ginger => (Vec3::new(0.98, 0.62, 0.32), Vec3::new(0.78, 0.4, 0.18)),
        Coat::Gray => (Vec3::new(0.62, 0.64, 0.7), Vec3::new(0.46, 0.48, 0.54)),
        Coat::Black => (Vec3::new(0.24, 0.24, 0.28), Vec3::new(0.16, 0.16, 0.2)),
        Coat::Calico => (Vec3::new(0.95, 0.93, 0.88), Vec3::new(0.95, 0.55, 0.25)),
    };
    // Tabby stripes, or a calico's patches.
    let marked = match cat.coat {
        Coat::Calico => {
            (point - Vec2::new(-0.05, 0.2)).length() < 0.09
                || point.distance(head) < 0.05 && point.x < head.x
        }
        _ => ((point.x * 28.0).sin() > 0.55) && point.y > 0.1,
    };
    let mut color = if marked { patch } else { base };
    let eye = head + Vec2::new(0.03, 0.012);
    if eyes_open {
        let open = coverage(point.distance(eye) - 0.012, edge);
        color = color.lerp(Vec3::new(0.6, 0.85, 0.3), open);
        color = color.lerp(
            Vec3::splat(0.05),
            coverage(point.distance(eye + Vec2::X * 0.004) - 0.005, edge),
        );
    } else {
        let shut = line(
            point,
            eye - Vec2::X * 0.014,
            eye + Vec2::new(0.012, -0.004),
            0.003,
        );
        color = color.lerp(Vec3::splat(0.1), coverage(shut, edge));
    }
    (coverage(distance, edge), color)
}

fn squirrel_shade(local: Vec2, pixel: f32, passing: &Passing) -> (f32, Vec3) {
    const SCALE: f32 = 2.2;
    let footprint = (pixel / SCALE).max(1e-5);
    let mut point = local / SCALE;
    point.x *= passing.facing;
    let body = (point / Vec2::new(1.4, 1.0)).length() - 0.013;
    let head = point.distance(Vec2::new(0.021, 0.009)) - 0.009;
    let tail_point = point - Vec2::new(-0.025, 0.014);
    let tail = (tail_point / Vec2::new(0.014, 0.025)).length() - 1.0;
    let distance = body.min(head).min(tail * 0.014);
    let eye = coverage(
        point.distance(Vec2::new(0.025, 0.013)) - 0.0015,
        footprint * 0.5,
    );
    let presence =
        smoothstep(0.0, 0.05, passing.phase) * (1.0 - smoothstep(0.95, 1.0, passing.phase));
    (
        coverage(distance, footprint * 0.5) * presence,
        Vec3::new(0.78, 0.42, 0.22).lerp(Vec3::splat(0.12), eye),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Grid;
    use crate::render::Camera;

    fn yard<'a>(outside: &'a Outside, plant: &'a Plant) -> Yard<'a> {
        Yard::new(Setting {
            outside,
            plant,
            furniture: &[],
            view: Camera::default().view(Grid::cells(100, 32)),
            glows: Vec::new(),
            shadows: vec![(Vec3::new(5.0, 1.0, 0.0), 0.2)],
            lantern: true,
            swing: 0.4,
            flag: true,
            gust: 1.0,
            wet: 1.0,
            door: 1.0,
            smoke: true,
            spill: Vec3::splat(0.2),
            occasion: Some(Occasion::Halloween),
            roofless: false,
            snow_ghost: Some((0.6, 0.2)),
            shaken: Some(1.0),
        })
    }

    #[test]
    fn rays_down_at_the_lawn_meet_the_ground_and_rays_up_meet_nothing() {
        let outside = Outside::clear(12.0, 0.0);
        let plant = Plant::default();
        let yard = yard(&outside, &plant);
        let eye = Vec3::new(4.0, 3.0, 6.0);
        let down = (Vec3::new(4.5, 0.0, 2.0) - eye).normalize();
        let (distance, color) = yard
            .trace(eye, down, 0.0, f32::INFINITY, false, 0.0, 0.003)
            .unwrap();
        assert!(((eye + down * distance).y).abs() < 1e-3);
        assert!(color.is_finite() && color.max_element() > 0.05);
        let up = Vec3::new(0.0, 1.0, -1.0).normalize();
        assert!(
            yard.trace(eye, up, 0.0, f32::INFINITY, false, 0.0, 0.003)
                .is_none()
        );
    }

    #[test]
    fn the_house_shadows_the_lawn_on_its_far_side_from_the_sun() {
        let outside = Outside::clear(13.0, 0.0);
        let plant = Plant::default();
        let yard = yard(&outside, &plant);
        let sun = yard.sun;
        let behind = Vec3::new(0.0, 0.0, 0.0) + Vec3::new(-sun.x, 0.0, -sun.z).normalize() * 2.6;
        assert!(!home::in_house(behind));
        assert_eq!(yard.shadow(behind, Vec3::Y, sun), 0.0);
        assert_eq!(yard.shadow(Vec3::new(6.5, 0.0, 2.5), Vec3::Y, sun), 1.0);
    }

    #[test]
    fn each_pieces_box_holds_everything_its_span_lets_rays_test() {
        let outside = Outside::clear(12.0, 0.0);
        let plant = Plant::default();
        let mut yard = yard(&outside, &plant);
        yard.pieces
            .extend([Piece::Snowman(0.2), Piece::Door(1.0), Piece::Pumpkin]);
        for piece in &yard.pieces {
            let (center, half) = piece.bounding_box();
            for step in 0..400 {
                let angle = step as f32 * 2.399;
                let origin = center
                    + Vec3::new(
                        angle.cos() * 9.0,
                        0.5 + (step % 7) as f32,
                        angle.sin() * 9.0,
                    );
                let aim = center
                    + Vec3::new(
                        ((step * 7) % 11) as f32 / 11.0 - 0.5,
                        ((step * 3) % 13) as f32 / 13.0 - 0.5,
                        ((step * 5) % 17) as f32 / 17.0 - 0.5,
                    ) * half
                        * 2.4;
                let direction = (aim - origin).normalize();
                if let Some((enter, exit)) = piece.span(origin, direction) {
                    let middle = origin + direction * ((enter.max(0.0) + exit) * 0.5);
                    let outside = ((middle - center).abs() - half).max_element();
                    assert!(outside < 1e-3, "{piece:?} {middle}");
                }
            }
        }
    }

    #[test]
    fn every_yard_piece_has_finite_distances_and_normals_in_any_weather() {
        for season in [
            Season::Spring,
            Season::Summer,
            Season::Autumn,
            Season::Winter,
        ] {
            let outside = Outside {
                season,
                snow_cover: 1.0,
                rain: 1.0,
                snow: 1.0,
                ..Outside::clear(21.0, 1e9)
            };
            let plant = Plant::default();
            let mut yard = yard(&outside, &plant);
            yard.pieces.push(Piece::Snowman(0.5));
            for piece in &yard.pieces {
                for step in 0..200 {
                    let point = Vec3::new(
                        -7.0 + (step % 20) as f32 * 0.8,
                        (step / 20) as f32 * 0.4,
                        -6.0 + (step % 13) as f32 * 0.75,
                    );
                    let (distance, _) = yard.distance(piece, point);
                    assert!(distance.is_finite(), "{piece:?}");
                    assert!(yard.normal(piece, point).is_finite(), "{piece:?}");
                }
            }
            let eye = Vec3::new(2.0, 2.5, 7.0);
            for step in 0..64 {
                let direction = (Vec3::new(-6.0 + step as f32 * 0.25, 0.8, -3.0) - eye).normalize();
                let color = yard
                    .trace(eye, direction, 0.0, f32::INFINITY, false, 0.0, 0.004)
                    .map_or(yard.background(direction, 0.004), |hit| hit.1);
                assert!(color.is_finite());
                assert!(
                    yard.weather(eye, direction, 0.0, 20.0, 0.0, 0.004, color)
                        .is_finite()
                );
                assert!(yard.sparkles(eye, direction, 20.0, 0.0, 0.004).is_finite());
            }
        }
    }
}
