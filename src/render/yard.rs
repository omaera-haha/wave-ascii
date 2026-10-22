//! Everything outdoors: the lawn and the land past the fence, the cottage
//! from outside, the yard's things, critters, rain and snow, and the light
//! of the sun, the moon, and the lantern.

use std::f32::consts::TAU;

use glam::{Vec2, Vec3};

use super::View;
use super::shapes::{
    Furniture, Material, box_distance, box_span, capsule, ellipsoid, rounded_cylinder,
    sphere_span,
};
use super::sky::{self, Outside, Sky, noise, phase, weather_color};
use crate::sdf::{coverage, mix, smooth_min, smoothstep};
use crate::world::home::{
    self, BENCH, BIRDBATH, BUSHES, CLOTHESLINE, CLOTHESLINE_HEIGHT, GARDEN, GARDEN_HALF,
    HOUSE_MAX, HOUSE_MIN, LANTERN, LANTERN_HEIGHT, LOT_MAX, LOT_MIN, MAILBOX, PERCH, SILL,
    SILL_HALF, SWING_HALF_WIDTH, SWING_TOP, TREE, TREE_CROWN, TREE_CROWN_RADIUS,
};
use crate::world::passerby::{Passerby, Passing};
use crate::world::stars::hash;
use crate::world::{BirdPicture, Plant, Season};

const LANTERN_LIGHT: Vec3 = Vec3::new(1.0, 0.74, 0.42);
/// Big plain surfaces are drawn dimmer than the things on them, so the
/// yard's things and the ghost stand out in glyphs.
const GROUND_EXPOSURE: f32 = 0.55;
const SHELL_EXPOSURE: f32 = 0.7;
const SNOW: Vec3 = Vec3::new(0.9, 0.94, 1.0);
/// Where the snowman stands when there is one.
const SNOWMAN: Vec3 = Vec3::new(-2.8, 0.0, 1.7);
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
}

impl Piece {
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
                Vec3::new(SWING_HALF_WIDTH + 0.35, SWING_TOP.y * 0.5 + 0.12, 0.72),
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
                let half = ((CLOTHESLINE[1] - CLOTHESLINE[0]) * 0.5).abs()
                    + Vec3::new(0.4, 0.85, 0.4);
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
        }
    }
}

/// A point light outdoors.
struct Glow {
    position: Vec3,
    color: Vec3,
    falloff: f32,
}

/// A bird or squirrel drawn facing the viewer.
#[derive(Clone, Copy, Debug)]
enum Critter {
    Bird(BirdPicture, Vec3),
    Squirrel(Passing, Vec3),
}

/// The outdoors for one frame.
pub struct Yard<'a> {
    outside: &'a Outside,
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
    /// The ghost, when it is outdoors, for its soft shadow.
    ghost: Option<Vec3>,
    lantern: f32,
    swing: f32,
    flag: bool,
    gust: f32,
    wind: f32,
    wet: f32,
    view: View,
}

/// What the scene tells the yard each frame.
pub struct Setting<'a> {
    pub outside: &'a Outside,
    pub plant: &'a Plant,
    pub furniture: &'a [Furniture<'a>],
    pub view: View,
    /// Ghost glows that light the yard: position, color, falloff.
    pub glows: Vec<(Vec3, Vec3, f32)>,
    pub ghost: Option<Vec3>,
    pub lantern: bool,
    pub swing: f32,
    pub flag: bool,
    pub gust: f32,
    pub wet: f32,
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
        pieces.extend(BUSHES.iter().map(|&(center, radius)| Piece::Bush(center, radius)));
        let mut critters = Vec::new();
        for passing in outside.passersby.iter().flatten() {
            match passing.kind {
                Passerby::Snowman => pieces.push(Piece::Snowman(passing.phase)),
                Passerby::Squirrel => critters.push(Critter::Squirrel(*passing, squirrel_at(passing))),
                _ => {}
            }
        }
        if let Some(bird) = outside.bird {
            critters.push(Critter::Bird(bird, bird.position));
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
            sky,
            outside,
            plant: setting.plant,
            furniture: setting.furniture,
            pieces,
            critters,
            glows,
            ghost: setting.ghost,
            lantern,
            swing: setting.swing,
            flag: setting.flag,
            gust: setting.gust,
            wind: outside.wind,
            wet: setting.wet,
            view: setting.view,
        }
    }

    /// The first thing outdoors that a ray meets between `from` and `to`:
    /// its distance and color. `cut` skips the cottage's walls, for rays
    /// on their way into the room.
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

        if !cut && let Some((distance, normal, surface)) = self.shell(origin, direction, from, limit)
        {
            limit = distance;
            let point = origin + direction * distance;
            hit = Some((
                distance,
                self.shade(surface, point, normal, direction) * SHELL_EXPOSURE,
            ));
        }

        for piece in &self.pieces {
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

        for thing in self.furniture {
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

    /// The sky, or the far ground fading into haze.
    pub fn background(&self, direction: Vec3, spread: f32) -> Vec3 {
        sky::sky(direction, spread, self.outside)
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
                let seed = hash(cell + Vec2::splat(layer as f32 * 13.0));
                if seed < rain * 0.9 {
                    let fraction = cells - cell;
                    let wide = ((fraction.x - (0.2 + 0.6 * hash(cell + Vec2::X))).abs() * 0.16)
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
                let seed = hash(cell + Vec2::splat(layer as f32 * 7.0));
                if seed < snow * 0.55 {
                    let center = Vec2::new(hash(cell + Vec2::X), hash(cell + Vec2::Y)) * 0.6 + 0.2;
                    let distance = ((cells - cell - center) * 0.3).length();
                    let flake = coverage(distance - 0.011, pixel * 0.6 + 0.003);
                    color = color.lerp(SNOW * (0.35 + self.ambient.max_element()), flake * 0.85);
                }
            }
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
        if self.outside.season == Season::Summer && night > 0.3 {
            let time = self.outside.time;
            for index in 0..8 {
                let seed = hash(Vec2::new(index as f32, 41.0));
                let wander = phase(time, 0.11, TAU) + seed * TAU;
                let home = Vec3::new(
                    2.6 + seed * 5.6,
                    0.35 + hash(Vec2::new(index as f32, 3.0)) * 0.9,
                    -3.8 + hash(Vec2::new(index as f32, 9.0)) * 6.0,
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
        let surface = match (axis, normal[axis] > 0.0) {
            (2, true) => Surface::Section,
            (1, true) => Surface::Cap,
            _ if point.y < 0.14 => Surface::Stone,
            _ => Surface::Siding,
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
                return (nearest.0.min(bound), if nearest.0 < bound { Surface::Bark } else { Surface::Foliage });
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
            let bumps = ((crown.x * 5.3).sin() + (crown.y * 5.9 + 1.3).sin() + (crown.z * 6.1 + 2.1).sin())
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
        let mut wood = capsule(point, first, first + top, 0.045)
            .min(capsule(point, second, second + top, 0.045));
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
        let rope = capsule(point, first + top, middle, 0.008)
            .min(capsule(point, middle, second + top, 0.008));
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
            let size = [Vec2::new(0.26, 0.34), Vec2::new(0.17, 0.24), Vec2::new(0.15, 0.2)][index];
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
    fn light(&self, point: Vec3, normal: Vec3) -> Vec3 {
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
        let facing = (normal.dot(self.moon) + wrap) / (1.0 + wrap);
        if facing > 0.0 && self.moonlight.max_element() > 1e-3 {
            total += self.moonlight * facing * self.shadow(point, normal, self.moon);
        }
        for glow in &self.glows {
            let offset = glow.position - point;
            let distance_squared = offset.length_squared();
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
            Surface::Water | Surface::Glass | Surface::Thing(_) | Surface::Snow
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
                Vec3::new(0.42, 0.31, 0.24) * (0.8 + 0.35 * noise(Vec2::new(point.x + point.z, point.y) * 9.0))
            }
            Surface::Foliage => {
                let texture = noise(Vec2::new(point.x * 2.7 + point.z, point.y * 2.7))
                    * 0.6
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
                    let center = Vec2::new(hash(cell + Vec2::X), hash(cell + Vec2::Y)) * 0.5 + 0.25;
                    let blossom = coverage((cells - cell - center).length() - 0.28, 0.12)
                        * smoothstep(0.35, 0.6, hash(cell));
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
                Vec3::new(0.8, 0.6, 0.42) * (0.85 + 0.25 * noise(Vec2::new(point.x * 3.0, point.y * 40.0 + point.z * 3.0)))
            }
            Surface::Soil => Vec3::new(0.42, 0.3, 0.22),
            Surface::Stone => Vec3::new(0.68, 0.69, 0.7) * (0.9 + 0.15 * noise(Vec2::new(point.x + point.z, point.y) * 11.0)),
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
            let worn = 1.0 - smoothstep(0.25, 0.6, (at - Vec2::new(SWING_TOP.x, SWING_TOP.z)).length());
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
                ) * smoothstep(0.4, 0.7, hash(cell))
                    * near;
                albedo = albedo.lerp(Vec3::new(0.95, 0.45, 0.2), leaf * (1.0 - blur));
            }
        } else if matches!(season, Season::Spring | Season::Summer) {
            let cells = at * 3.0;
            let cell = cells.floor();
            let flower = coverage(
                (cells - cell - Vec2::splat(0.5)).length() - 0.09,
                footprint * 1.5 + 0.02,
            ) * smoothstep(0.82, 0.9, hash(cell));
            let tint = if hash(cell + Vec2::X) < 0.5 {
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
        if let Some(ghost) = self.ghost {
            let offset = Vec2::new(point.x - ghost.x, point.z - ghost.z).length();
            let height = ghost.y.max(0.2);
            color *= 1.0 - 0.45 * (-(offset * offset) / (0.08 * height)).exp();
        }
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
    (Vec3::new(0.0, 1.05, 0.0), Vec3::new(-1.3, 1.32, 0.85), 0.075),
    (Vec3::new(0.0, 1.9, 0.0), Vec3::new(-0.95, 3.05, 0.25), 0.075),
    (Vec3::new(0.0, 2.1, 0.0), Vec3::new(0.85, 3.2, -0.2), 0.07),
    (Vec3::new(0.0, 2.45, 0.0), Vec3::new(0.05, 3.95, 0.1), 0.065),
    (Vec3::new(0.0, 1.85, 0.0), Vec3::new(0.95, 2.55, 0.55), 0.055),
    (Vec3::new(-0.95, 3.05, 0.25), Vec3::new(-1.55, 3.55, 0.5), 0.03),
    (Vec3::new(-0.95, 3.05, 0.25), Vec3::new(-0.8, 3.85, -0.1), 0.03),
    (Vec3::new(-0.5, 2.45, 0.12), Vec3::new(-1.4, 2.7, -0.35), 0.03),
    (Vec3::new(0.85, 3.2, -0.2), Vec3::new(1.5, 3.65, -0.45), 0.03),
    (Vec3::new(0.85, 3.2, -0.2), Vec3::new(0.7, 4.0, 0.2), 0.03),
    (Vec3::new(0.45, 2.65, -0.1), Vec3::new(1.35, 2.85, 0.35), 0.028),
    (Vec3::new(0.05, 3.95, 0.1), Vec3::new(-0.4, 4.45, 0.05), 0.025),
    (Vec3::new(0.05, 3.95, 0.1), Vec3::new(0.45, 4.4, -0.2), 0.025),
    (Vec3::new(0.95, 2.55, 0.55), Vec3::new(1.5, 2.95, 1.0), 0.028),
    (Vec3::new(-0.65, 1.19, 0.42), Vec3::new(-0.95, 1.75, 0.75), 0.03),
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
    let index = (along / spacing).round().clamp(0.0, (length / spacing).floor());
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
    let cap = box_distance(head - Vec3::Y * 0.13, Vec3::new(0.11, 0.025, 0.11))
        .min(box_distance(head + Vec3::Y * 0.11, Vec3::new(0.09, 0.015, 0.09)));
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
        box_distance(local - Vec3::new(0.15, 1.22, -0.1), Vec3::new(0.008, 0.08, 0.025))
    } else {
        box_distance(local - Vec3::new(0.15, 1.09, -0.06), Vec3::new(0.008, 0.025, 0.09))
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
    let bumps = ((local.x * 9.0).sin() + (local.y * 8.0 + 1.0).sin() + (local.z * 9.5 + 2.0).sin())
        * 0.025;
    ((shape + bumps) * 0.85, Surface::Hedge)
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
    let eye = coverage(point.distance(head + Vec2::new(0.008, 0.006)) - 0.004, footprint * 0.5);
    (
        coverage(distance, footprint * 0.5),
        feather.lerp(Vec3::splat(0.05), eye),
    )
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
    let eye = coverage(point.distance(Vec2::new(0.025, 0.013)) - 0.0015, footprint * 0.5);
    let presence = smoothstep(0.0, 0.05, passing.phase) * (1.0 - smoothstep(0.95, 1.0, passing.phase));
    (
        coverage(distance, footprint * 0.5) * presence,
        Vec3::new(0.78, 0.42, 0.22).lerp(Vec3::splat(0.12), eye),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Camera;
    use crate::frame::Grid;

    fn yard<'a>(outside: &'a Outside, plant: &'a Plant) -> Yard<'a> {
        Yard::new(Setting {
            outside,
            plant,
            furniture: &[],
            view: Camera::default().view(Grid::cells(100, 32)),
            glows: Vec::new(),
            ghost: Some(Vec3::new(5.0, 1.0, 0.0)),
            lantern: true,
            swing: 0.4,
            flag: true,
            gust: 1.0,
            wet: 1.0,
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
        assert!(yard.trace(eye, up, 0.0, f32::INFINITY, false, 0.0, 0.003).is_none());
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
    fn every_yard_piece_has_finite_distances_and_normals_in_any_weather() {
        for season in [Season::Spring, Season::Summer, Season::Autumn, Season::Winter] {
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

