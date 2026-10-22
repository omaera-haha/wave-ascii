//! Creatures as solid little bodies: each is a handful of rounded shapes
//! melted together, posed for the moment, and traced against the scene's
//! rays like the furniture. The face is the same face the ghost always had,
//! printed onto the front of the head.

use std::f32::consts::{PI, TAU};

use glam::{Mat3, Vec2, Vec3};

use crate::face::Face;
use crate::sdf::{coverage, mix, smooth_min, smoothstep};
use crate::world::{Accessory, Genome, Species};

/// Most steps a ray takes through a body before giving up.
const STEPS: usize = 56;

/// Everything about one creature that its picture needs.
#[derive(Clone, Copy, Debug)]
pub struct Figure {
    pub id: u32,
    pub species: Species,
    pub genome: Genome,
    /// The middle of its body, in the world.
    pub position: Vec3,
    /// Which way it faces, as an angle about the vertical; zero faces the
    /// glass.
    pub facing: f32,
    pub size: f32,
    pub face: Face,
    /// The color of its mood, for whatever on it glows.
    pub glow: Vec3,
    pub opacity: f32,
    pub accessory: Option<Accessory>,
    pub motion: Motion,
}

/// How a body is moving at one moment.
#[derive(Clone, Copy, Debug, Default)]
pub struct Motion {
    pub time: f64,
    /// Strides or hops taken so far, in whole cycles.
    pub stride: f32,
    /// How fast its feet go across the ground.
    pub pace: f32,
    pub airborne: bool,
    /// Upward speed while in the air.
    pub rise: f32,
    /// Seconds since it last landed.
    pub landed: f32,
    pub asleep: f32,
    /// For a flier, from flying at 0 to settled at 1.
    pub perched: f32,
    /// Which way and how far the head turns from the body, across and up.
    pub look: Vec2,
    /// From a still tail at 0 to wagging hard at 1.
    pub wag: f32,
    /// Ears from drooping at -1 to pricked at 1.
    pub perk: f32,
    pub blush: f32,
    /// From still to dancing at 1.
    pub dance: f32,
    /// Sideways lean, as into a stroking hand, from -1 to 1.
    pub lean: f32,
    /// Its energy, from 0 to 1, for how brightly what it has of light glows.
    pub energy: f32,
    /// A little extra rise, as with pride or a yawn, in its own units.
    pub lift: f32,
    /// A little extra roll to one side, as to music or in a sneeze.
    pub sway: f32,
    /// From 0 to 1, how far it stretches up tall.
    pub reach: f32,
    /// From 0 to 1, how hard it shivers.
    pub shiver: f32,
    /// A deep slow breath, while it keeps someone company in their mood:
    /// where in the breath it is, from -1 to 1.
    pub calm: Option<f32>,
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    /// An ellipsoid turned by the transpose of `inverse`.
    Ellipsoid {
        center: Vec3,
        radii: Vec3,
        inverse: Mat3,
    },
    /// A capsule whose ends have different radii.
    Cone { a: Vec3, b: Vec3, ra: f32, rb: f32 },
    Block {
        center: Vec3,
        half: Vec3,
        round: f32,
        inverse: Mat3,
    },
    /// A ring around the local y axis.
    Ring {
        center: Vec3,
        inverse: Mat3,
        major: f32,
        minor: f32,
    },
}

impl Shape {
    fn distance(&self, point: Vec3) -> f32 {
        match *self {
            Self::Ellipsoid {
                center,
                radii,
                inverse,
            } => {
                let local = inverse * (point - center);
                let scaled = (local / radii).length();
                let gradient = (local / (radii * radii)).length();
                if gradient > 1e-6 {
                    scaled * (scaled - 1.0) / gradient
                } else {
                    -radii.min_element()
                }
            }
            Self::Cone { a, b, ra, rb } => round_cone(point, a, b, ra, rb),
            Self::Block {
                center,
                half,
                round,
                inverse,
            } => {
                let local = (inverse * (point - center)).abs() - half + Vec3::splat(round);
                local.max(Vec3::ZERO).length() + local.max_element().min(0.0) - round
            }
            Self::Ring {
                center,
                inverse,
                major,
                minor,
            } => {
                let local = inverse * (point - center);
                Vec2::new(Vec2::new(local.x, local.z).length() - major, local.y).length() - minor
            }
        }
    }

    fn bound(&self) -> (Vec3, f32) {
        match *self {
            Self::Ellipsoid { center, radii, .. } => (center, radii.max_element()),
            Self::Cone { a, b, ra, rb } => ((a + b) * 0.5, a.distance(b) * 0.5 + ra.max(rb)),
            Self::Block { center, half, .. } => (center, half.length()),
            Self::Ring {
                center,
                major,
                minor,
                ..
            } => (center, major + minor),
        }
    }
}

/// Signed distance to a capsule from `a` to `b` whose radius runs from `ra`
/// to `rb`.
fn round_cone(point: Vec3, a: Vec3, b: Vec3, ra: f32, rb: f32) -> f32 {
    let along = b - a;
    let length_squared = along.length_squared();
    if length_squared < 1e-10 {
        return point.distance(a) - ra.max(rb);
    }
    let difference = ra - rb;
    let slant = length_squared - difference * difference;
    let inverse = 1.0 / length_squared;
    let offset = point - a;
    let y = offset.dot(along);
    let z = y - length_squared;
    let x = offset * length_squared - along * y;
    let x2 = x.length_squared();
    let y2 = y * y * length_squared;
    let z2 = z * z * length_squared;
    let k = difference.signum() * difference * difference * x2;
    if z.signum() * slant * z2 > k {
        return (x2 + z2).sqrt() * inverse - rb;
    }
    if y.signum() * slant * y2 < k {
        return (x2 + y2).sqrt() * inverse - ra;
    }
    ((x2 * slant * inverse).sqrt() + y * difference) * inverse - ra
}

/// How a part is colored.
#[derive(Clone, Copy, Debug)]
enum Paint {
    Plain(Vec3),
    /// Round spots of `spot` on `base`, about `scale` apart.
    Spots {
        base: Vec3,
        spot: Vec3,
        scale: f32,
        seed: f32,
        glows: bool,
    },
    /// From `top` to `bottom` over the part's height around `middle`.
    Fade {
        top: Vec3,
        bottom: Vec3,
        middle: f32,
        span: f32,
    },
    /// Light of its own.
    Glow(Vec3),
    /// Glossy dark, for noses and beads.
    Gloss(Vec3),
}

impl Paint {
    fn color(&self, point: Vec3) -> Vec3 {
        match *self {
            Self::Plain(color) | Self::Glow(color) | Self::Gloss(color) => color,
            Self::Spots {
                base,
                spot,
                scale,
                seed,
                ..
            } => base.lerp(spot, spottiness(point, scale, seed)),
            Self::Fade {
                top,
                bottom,
                middle,
                span,
            } => bottom.lerp(top, smoothstep(middle - span, middle + span, point.y)),
        }
    }

    fn emission(&self, point: Vec3) -> Vec3 {
        match *self {
            Self::Glow(color) => color,
            Self::Spots {
                spot,
                scale,
                seed,
                glows: true,
                ..
            } => spot * spottiness(point, scale, seed) * 0.8,
            _ => Vec3::ZERO,
        }
    }

    fn sheen(&self) -> f32 {
        match self {
            Self::Gloss(_) => 1.0,
            _ => 0.0,
        }
    }
}

/// How much `point` lies in one of the round spots about `scale` apart,
/// from 0 to 1.
fn spottiness(point: Vec3, scale: f32, seed: f32) -> f32 {
    let cell = point / scale + Vec3::splat(seed);
    let whole = cell.floor();
    let jitter = Vec3::new(
        hash(whole.dot(Vec3::new(1.0, 57.0, 113.0))),
        hash(whole.dot(Vec3::new(7.0, 31.0, 91.0))),
        hash(whole.dot(Vec3::new(13.0, 5.0, 47.0))),
    );
    let center = whole + Vec3::splat(0.25) + jitter * 0.5;
    let size = 0.22 + 0.18 * hash(whole.dot(Vec3::new(3.0, 17.0, 29.0)));
    smoothstep(size + 0.06, size - 0.02, cell.distance(center))
}

fn hash(value: f32) -> f32 {
    ((value * 12.9898).sin() * 43_758.547).fract().abs()
}

#[derive(Clone, Copy, Debug)]
struct Part {
    shape: Shape,
    paint: Paint,
    /// How wide the seam is where it melts into what came before.
    blend: f32,
    bound: (Vec3, f32),
}

/// A place and a turn, for posing parts relative to a body or a head.
#[derive(Clone, Copy, Debug)]
struct Frame {
    origin: Vec3,
    axes: Mat3,
}

impl Frame {
    const ROOT: Self = Self {
        origin: Vec3::ZERO,
        axes: Mat3::IDENTITY,
    };

    fn point(&self, point: Vec3) -> Vec3 {
        self.origin + self.axes * point
    }

    /// A frame turned by `turn` about `pivot` of this one.
    fn child(&self, pivot: Vec3, turn: Mat3) -> Self {
        Self {
            origin: self.point(pivot),
            axes: self.axes * turn,
        }
    }
}

/// Where the face is printed, in the model's own space.
#[derive(Clone, Copy, Debug)]
struct Plate {
    center: Vec3,
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    /// Model units for one unit of the face's own space.
    unit: f32,
    /// Whether it glows, as on a screen, rather than being dark.
    lit: bool,
}

/// A body being put together for one frame.
struct Sculpt {
    parts: Vec<Part>,
    frame: Frame,
    blend: f32,
}

impl Sculpt {
    fn push(&mut self, shape: Shape, paint: Paint) {
        let (center, radius) = shape.bound();
        self.parts.push(Part {
            shape,
            paint,
            blend: self.blend,
            bound: (center, radius + self.blend),
        });
    }

    fn ellipsoid(&mut self, center: Vec3, radii: Vec3, paint: Paint) {
        self.turned(center, radii, Mat3::IDENTITY, paint);
    }

    fn turned(&mut self, center: Vec3, radii: Vec3, turn: Mat3, paint: Paint) {
        let axes = self.frame.axes * turn;
        self.push(
            Shape::Ellipsoid {
                center: self.frame.point(center),
                radii: radii.max(Vec3::splat(1e-3)),
                inverse: axes.transpose(),
            },
            paint,
        );
    }

    fn ball(&mut self, center: Vec3, radius: f32, paint: Paint) {
        self.ellipsoid(center, Vec3::splat(radius), paint);
    }

    fn cone(&mut self, a: Vec3, b: Vec3, ra: f32, rb: f32, paint: Paint) {
        self.push(
            Shape::Cone {
                a: self.frame.point(a),
                b: self.frame.point(b),
                ra: ra.max(1e-3),
                rb: rb.max(1e-3),
            },
            paint,
        );
    }

    fn block(&mut self, center: Vec3, half: Vec3, round: f32, paint: Paint) {
        self.push(
            Shape::Block {
                center: self.frame.point(center),
                half,
                round: round.min(half.min_element()),
                inverse: self.frame.axes.transpose(),
            },
            paint,
        );
    }

    fn ring(&mut self, center: Vec3, turn: Mat3, major: f32, minor: f32, paint: Paint) {
        self.push(
            Shape::Ring {
                center: self.frame.point(center),
                inverse: (self.frame.axes * turn).transpose(),
                major,
                minor,
            },
            paint,
        );
    }

    /// Runs `build` with parts melting together over `blend`.
    fn melted(&mut self, blend: f32, build: impl FnOnce(&mut Self)) {
        let before = self.blend;
        self.blend = blend;
        build(self);
        self.blend = before;
    }

    /// Runs `build` with parts posed in `frame`.
    fn within(&mut self, frame: Frame, build: impl FnOnce(&mut Self)) {
        let before = self.frame;
        self.frame = frame;
        build(self);
        self.frame = before;
    }
}

/// What the tracer needs to light one point of a creature.
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub distance: f32,
    pub point: Vec3,
    pub normal: Vec3,
    pub albedo: Vec3,
    /// Light of its own.
    pub emission: Vec3,
    /// How glossy, from matte at 0.
    pub sheen: f32,
    /// How much of what is behind shows through, from 0 to 1.
    pub clear: f32,
}

/// A creature posed for one frame, ready to trace.
#[derive(Clone, Debug)]
pub struct Model {
    pub id: u32,
    origin: Vec3,
    turn: Mat3,
    unturn: Mat3,
    scale: f32,
    bound: (Vec3, f32),
    parts: Vec<Part>,
    plate: Option<Plate>,
    face: Face,
    blush: f32,
    /// How see-through the body is.
    clear: f32,
    /// How much it glows all over, as a ghost does.
    shine: f32,
    glow: Vec3,
    pub opacity: f32,
    /// Where its feet touch, for its shadow, and how wide that is.
    pub ground: Vec3,
    pub reach: f32,
    pub crown: Vec3,
    /// The light it casts, if it glows.
    pub light: Vec3,
}

impl Model {
    pub fn of(figure: &Figure) -> Self {
        let rig = Rig::new(figure);
        let mut sculpt = Sculpt {
            parts: Vec::with_capacity(32),
            frame: Frame::ROOT,
            blend: 0.0,
        };
        let coat = figure.genome.coat;
        let mut look = Look::default();
        let plate = match figure.species {
            Species::Fox => fox(&mut sculpt, &rig, coat, &mut look),
            Species::Bunny => bunny(&mut sculpt, &rig, coat, figure.genome.seed, &mut look),
            Species::Penguin => penguin(&mut sculpt, &rig, coat, &mut look),
            Species::Frog => frog(&mut sculpt, &rig, coat, figure.genome.seed, &mut look),
            Species::Slime => slime(&mut sculpt, &rig, coat, &mut look),
            Species::Owl => owl(&mut sculpt, &rig, coat, figure.genome.seed, &mut look),
            Species::Robot => robot(&mut sculpt, &rig, coat, figure.glow, &mut look),
            Species::Hedgehog => hedgehog(&mut sculpt, &rig, coat, &mut look),
            Species::Mushroom => mushroom(&mut sculpt, &rig, coat, figure.genome.seed, &mut look),
            Species::Axolotl => axolotl(&mut sculpt, &rig, coat, figure.genome.seed, &mut look),
            Species::Dragon => dragon(&mut sculpt, &rig, coat, &mut look),
            Species::Ghost => ghost(&mut sculpt, &rig, coat, figure.glow, &mut look),
            Species::Jelly => jelly(&mut sculpt, &rig, coat, figure.genome.seed, &mut look),
        };
        if let (Some(accessory), Some(crown)) = (figure.accessory, look.hat) {
            wear(&mut sculpt, accessory, crown, look.neck);
        }
        let parts = sculpt.parts;
        let mut low = Vec3::splat(f32::INFINITY);
        let mut high = Vec3::splat(f32::NEG_INFINITY);
        for part in &parts {
            let (center, radius) = part.bound;
            low = low.min(center - Vec3::splat(radius));
            high = high.max(center + Vec3::splat(radius));
        }
        let middle = (low + high) * 0.5;
        let radius = parts
            .iter()
            .map(|part| part.bound.0.distance(middle) + part.bound.1)
            .fold(0.0, f32::max);
        let scale = figure.size.max(0.05) * figure.species.scale();
        let turn = Mat3::from_rotation_y(figure.facing);
        let to_world = |local: Vec3| figure.position + turn * local * scale;
        let stance = figure.species.stance();
        Self {
            id: figure.id,
            origin: figure.position,
            turn,
            unturn: turn.transpose(),
            scale,
            bound: (middle, radius),
            parts,
            plate,
            face: figure.face,
            blush: figure.motion.blush,
            clear: look.clear,
            shine: look.shine,
            glow: figure.glow,
            opacity: figure.opacity.clamp(0.0, 1.0),
            ground: figure.position - Vec3::Y * stance * figure.size.max(0.05),
            reach: crate::world::household::body_reach(figure.species, figure.size),
            crown: to_world(look.top),
            light: look.light * figure.opacity.clamp(0.0, 1.0),
        }
    }

    /// The world box around it, for culling.
    pub fn bounds(&self) -> (Vec3, f32) {
        (
            self.origin + self.turn * self.bound.0 * self.scale,
            self.bound.1 * self.scale,
        )
    }

    fn distance(&self, point: Vec3, parts: &[u8]) -> f32 {
        let mut nearest = f32::INFINITY;
        for &index in parts {
            let part = &self.parts[usize::from(index)];
            let distance = part.shape.distance(point);
            nearest = if part.blend > 0.0 && nearest.is_finite() {
                smooth_min(nearest, distance, part.blend)
            } else {
                nearest.min(distance)
            };
        }
        nearest
    }

    /// Where a ray from `origin` along `direction` first meets the body,
    /// short of `limit`, with what it looks like there. `spread` is how wide
    /// a pixel is per unit of distance, and `traveled` how far the ray came
    /// before `origin`.
    pub fn trace(
        &self,
        origin: Vec3,
        direction: Vec3,
        limit: f32,
        spread: f32,
        traveled: f32,
    ) -> Option<Hit> {
        if self.opacity <= 0.01 {
            return None;
        }
        let start = self.unturn * (origin - self.origin) / self.scale;
        let along = self.unturn * direction;
        let (center, radius) = self.bound;
        let (enter, exit) = sphere_span(start, along, center, radius)?;
        let limit = limit / self.scale;
        if exit <= 0.0 || enter >= limit {
            return None;
        }
        // Only parts the ray passes near can be met.
        let mut near = [0u8; 64];
        let mut count = 0;
        for (index, part) in self.parts.iter().enumerate().take(64) {
            if sphere_span(start, along, part.bound.0, part.bound.1).is_some() {
                near[count] = index as u8;
                count += 1;
            }
        }
        if count == 0 {
            return None;
        }
        let near = &near[..count];
        let mut distance = enter.max(0.0);
        let end = exit.min(limit);
        let mut hit = None;
        for _ in 0..STEPS {
            let point = start + along * distance;
            let gap = self.distance(point, near);
            let pixel = ((traveled + distance * self.scale) * spread / self.scale).max(1e-4);
            if gap < pixel * 0.35 {
                hit = Some(distance);
                break;
            }
            distance += gap.max(pixel * 0.2);
            if distance > end {
                break;
            }
        }
        let distance = hit?;
        let point = start + along * distance;
        let normal = self.normal(point, near);
        let pixel = ((traveled + distance * self.scale) * spread / self.scale).max(1e-4);
        let (albedo, emission, sheen) = self.paint(point, normal, near, pixel);
        Some(Hit {
            distance: distance * self.scale,
            point: self.origin + self.turn * point * self.scale,
            normal: (self.turn * normal).normalize_or(Vec3::Y),
            albedo,
            emission: emission + self.glow * self.shine * (0.5 + 0.5 * normal.dot(-along).max(0.0)),
            sheen,
            clear: self.clear,
        })
    }

    fn normal(&self, point: Vec3, parts: &[u8]) -> Vec3 {
        let step = 0.0015;
        let corners = [
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        ];
        corners
            .into_iter()
            .map(|corner| corner * self.distance(point + corner * step, parts))
            .sum::<Vec3>()
            .normalize_or(Vec3::Y)
    }

    /// The color at a point of the body: the parts near it blended across
    /// their seams, with the face printed over.
    fn paint(&self, point: Vec3, normal: Vec3, parts: &[u8], pixel: f32) -> (Vec3, Vec3, f32) {
        let mut distances = [0.0f32; 64];
        let mut nearest = f32::INFINITY;
        for (slot, &index) in parts.iter().enumerate() {
            let distance = self.parts[usize::from(index)].shape.distance(point);
            distances[slot] = distance;
            nearest = nearest.min(distance);
        }
        let mut total = Vec3::ZERO;
        let mut glow = Vec3::ZERO;
        let mut sheen = 0.0;
        let mut weights = 0.0;
        for (slot, &index) in parts.iter().enumerate() {
            let part = &self.parts[usize::from(index)];
            let width = part.blend.max(0.004);
            let weight = (1.0 - (distances[slot] - nearest) / width).clamp(0.0, 1.0).powi(2);
            if weight <= 0.0 {
                continue;
            }
            let color = part.paint.color(point);
            total += color * weight;
            glow += part.paint.emission(point) * weight;
            sheen += part.paint.sheen() * weight;
            weights += weight;
        }
        let weights = weights.max(1e-6);
        let mut albedo = total / weights;
        let mut emission = glow / weights;
        let mut sheen = sheen / weights;
        if let Some(plate) = &self.plate {
            let facing = normal.dot(plate.forward);
            let offset = point - plate.center;
            if facing > 0.1 && offset.dot(plate.forward) > -plate.unit * 0.3 {
                let local = Vec2::new(offset.dot(plate.right), offset.dot(plate.up)) / plate.unit;
                let edge = pixel / plate.unit * 0.5;
                let fade = smoothstep(0.1, 0.35, facing);
                let ink = coverage(self.face.distance(local), edge) * fade;
                if self.blush > 0.0 {
                    let cheeks = [Vec2::new(-0.29, -0.15), Vec2::new(0.29, -0.15)]
                        .into_iter()
                        .map(|cheek| ((local - cheek) / Vec2::new(0.1, 0.055)).length())
                        .fold(f32::INFINITY, f32::min);
                    let rosy = smoothstep(1.2, 0.6, cheeks) * self.blush * fade;
                    albedo = albedo.lerp(Vec3::new(1.0, 0.5, 0.6), rosy * 0.6);
                }
                if ink > 0.0 {
                    if plate.lit {
                        let light = self.glow.lerp(Vec3::new(0.6, 1.0, 0.95), 0.5);
                        emission = emission.lerp(light * 1.3, ink);
                        albedo = albedo.lerp(light * 0.3, ink);
                    } else {
                        let shine = coverage(self.face.shine(local), edge) * fade;
                        let eye = Vec3::new(0.08, 0.07, 0.1).lerp(Vec3::ONE, shine);
                        albedo = albedo.lerp(eye, ink);
                        emission = emission.lerp(Vec3::splat(0.35) * shine, ink);
                        sheen = mix(sheen, 1.0, ink);
                    }
                }
            }
        }
        (albedo, emission, sheen)
    }
}

/// A creature on its own against `backdrop`, as on a card: seen from a
/// little above and in front, turned by `turn`, under a soft studio light.
pub fn study(figure: &Figure, grid: crate::frame::Grid, backdrop: Vec3, turn: f32) -> crate::frame::Frame {
    let model = Model::of(figure);
    let (center, radius) = model.bounds();
    let around = Vec3::new(turn.sin(), 0.32, turn.cos()).normalize();
    let eye = center + around * radius * 3.4;
    let forward = (center - eye).normalize();
    let right = forward.cross(Vec3::Y).normalize();
    let up = right.cross(forward);
    let light = Vec3::new(0.5, 0.8, 0.6).normalize();
    let spread = grid.scale() / super::FOCAL;
    crate::frame::Frame::sample(grid, |point| {
        let direction = (forward * super::FOCAL + right * point.x + up * point.y).normalize();
        let color = match model.trace(eye, direction, f32::INFINITY, spread, 0.0) {
            Some(hit) => {
                let lit = Vec3::splat(0.42)
                    + Vec3::new(1.0, 0.96, 0.9) * hit.normal.dot(light).max(0.0) * 0.75
                    + Vec3::new(0.5, 0.6, 0.8) * (1.0 - hit.normal.dot(-direction).max(0.0)).powi(3) * 0.4;
                let half = (-direction + light).normalize();
                let glint = hit.sheen * hit.normal.dot(half).max(0.0).powi(24) * 0.7;
                let shaded = hit.albedo * lit + hit.emission + Vec3::splat(glint);
                backdrop.lerp(shaded, 1.0 - hit.clear * 0.5)
            }
            None => backdrop,
        };
        let color = color.clamp(Vec3::ZERO, Vec3::ONE);
        let roots = Vec3::new(color.x.sqrt(), color.y.sqrt(), color.z.sqrt());
        crate::frame::Shade::from_rgb(color * 0.7 + roots * 0.3)
    })
}

/// The color a creature is known by in words and marks: its coat, kept
/// bright enough to read against the dark.
pub fn signature(species: Species, coat: u8) -> Vec3 {
    let color = match (species, coat) {
        (Species::Fox, 1) => Vec3::new(0.85, 0.9, 1.0),
        (Species::Fox, 2) => Vec3::new(0.7, 0.7, 0.8),
        (Species::Fox, _) => Vec3::new(1.0, 0.6, 0.25),
        (Species::Bunny, 1) => Vec3::new(0.85, 0.65, 0.5),
        (Species::Bunny, 2) => Vec3::new(0.78, 0.78, 0.85),
        (Species::Bunny, _) => Vec3::new(1.0, 0.85, 0.9),
        (Species::Penguin, _) => Vec3::new(0.6, 0.75, 1.0),
        (Species::Frog, 2) => Vec3::new(0.4, 0.65, 1.0),
        (Species::Frog, 3) => Vec3::new(1.0, 0.65, 0.75),
        (Species::Frog, _) => Vec3::new(0.55, 0.9, 0.45),
        (Species::Slime, 1) => Vec3::new(1.0, 0.6, 0.8),
        (Species::Slime, 2) => Vec3::new(0.55, 0.8, 1.0),
        (Species::Slime, 3) => Vec3::new(1.0, 0.92, 0.5),
        (Species::Slime, _) => Vec3::new(0.5, 1.0, 0.8),
        (Species::Owl, 1) => Vec3::new(0.95, 0.95, 1.0),
        (Species::Owl, _) => Vec3::new(0.85, 0.7, 0.55),
        (Species::Robot, 1) => Vec3::new(1.0, 0.45, 0.4),
        (Species::Robot, 2) => Vec3::new(1.0, 0.85, 0.35),
        (Species::Robot, _) => Vec3::new(0.4, 0.9, 0.9),
        (Species::Hedgehog, 1) => Vec3::new(0.95, 0.92, 0.85),
        (Species::Hedgehog, _) => Vec3::new(0.8, 0.65, 0.5),
        (Species::Mushroom, 1) => Vec3::new(0.85, 0.65, 0.45),
        (Species::Mushroom, 2) => Vec3::new(0.75, 0.55, 1.0),
        (Species::Mushroom, _) => Vec3::new(1.0, 0.4, 0.38),
        (Species::Axolotl, 1) => Vec3::new(1.0, 0.85, 0.5),
        (Species::Axolotl, 2) => Vec3::new(0.7, 0.65, 0.6),
        (Species::Axolotl, _) => Vec3::new(1.0, 0.7, 0.8),
        (Species::Dragon, 1) => Vec3::new(0.75, 0.6, 1.0),
        (Species::Dragon, 2) => Vec3::new(1.0, 0.5, 0.4),
        (Species::Dragon, _) => Vec3::new(0.5, 0.9, 0.55),
        (Species::Ghost, 1) => Vec3::new(0.8, 1.0, 0.9),
        (Species::Ghost, 2) => Vec3::new(0.9, 0.85, 1.0),
        (Species::Ghost, _) => Vec3::new(0.9, 0.95, 1.0),
        (Species::Jelly, 1) => Vec3::new(0.6, 0.8, 1.0),
        (Species::Jelly, 2) => Vec3::new(0.85, 0.75, 1.0),
        (Species::Jelly, 3) => Vec3::new(1.0, 0.8, 0.65),
        (Species::Jelly, _) => Vec3::new(1.0, 0.7, 0.88),
    };
    color
}

/// Where a ray meets a sphere: the distances it enters and leaves at.
fn sphere_span(origin: Vec3, direction: Vec3, center: Vec3, radius: f32) -> Option<(f32, f32)> {
    let offset = origin - center;
    let b = offset.dot(direction);
    let c = offset.length_squared() - radius * radius;
    let discriminant = b * b - c;
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let exit = -b + root;
    (exit > 0.0).then_some((-b - root, exit))
}

/// What a species' builder reports about the body it made.
#[derive(Clone, Copy, Debug, Default)]
struct Look {
    clear: f32,
    shine: f32,
    /// Light it casts on what is around it.
    light: Vec3,
    /// The top of its head, in model space.
    top: Vec3,
    /// Where a hat sits and which way is up there.
    hat: Option<Frame>,
    /// Where a scarf goes, around the neck: the middle and its radius.
    neck: Option<(Frame, f32)>,
}

/// How each part of a body moves at one moment, worked out from its motion.
struct Rig {
    time: f32,
    /// The stride's angle, a full turn per stride.
    stride: f32,
    /// From standing at 0 to striding along at 1.
    walk: f32,
    breath: f32,
    /// Squashed below 1 on landing, stretched above it in the air.
    stretch: f32,
    air: f32,
    sleep: f32,
    perk: f32,
    wag: f32,
    /// How the head turns from the body.
    head: Mat3,
    dance: f32,
    lean: f32,
    perched: f32,
    energy: f32,
    build: f32,
    lift: f32,
    sway: f32,
}

impl Rig {
    fn new(figure: &Figure) -> Self {
        let motion = figure.motion;
        let time = (motion.time.rem_euclid(3600.0)) as f32 + figure.id as f32 * 1.7;
        let walk = smoothstep(0.03, 0.35, motion.pace) * (1.0 - motion.asleep);
        let landing = (1.0 - motion.landed / 0.18).clamp(0.0, 1.0);
        let stretch = if motion.airborne {
            1.0 + 0.12 * (motion.rise / 3.0).clamp(-1.0, 1.0)
        } else {
            1.0 - 0.2 * landing * landing
        };
        let look = Vec2::new(
            motion.look.x.clamp(-0.9, 0.9),
            motion.look.y.clamp(-0.5, 0.45),
        ) * (1.0 - motion.asleep);
        let nod = -0.45 * motion.asleep;
        let breath = match motion.calm {
            Some(calm) => calm.clamp(-1.0, 1.0) * 2.0,
            None => (time * if motion.asleep > 0.5 { 1.3 } else { 2.2 }).sin(),
        };
        let stretch = stretch * (1.0 + 0.14 * motion.reach.clamp(0.0, 1.0));
        Self {
            time,
            stride: motion.stride * TAU,
            walk,
            breath,
            stretch,
            lift: motion.lift.clamp(-0.2, 0.2),
            sway: motion.sway.clamp(-0.6, 0.6)
                + 0.05 * motion.shiver.clamp(0.0, 1.0) * (time * 47.0).sin(),
            air: if motion.airborne { 1.0 } else { 0.0 },
            sleep: motion.asleep.clamp(0.0, 1.0),
            perk: motion.perk.clamp(-1.0, 1.0),
            wag: motion.wag.clamp(0.0, 1.0),
            head: Mat3::from_rotation_y(look.x) * Mat3::from_rotation_x(-(look.y + nod)),
            dance: motion.dance.clamp(0.0, 1.0),
            lean: motion.lean.clamp(-1.0, 1.0),
            perched: motion.perched.clamp(0.0, 1.0),
            energy: motion.energy.clamp(0.0, 1.0),
            build: 1.0 + 0.07 * figure.genome.build.clamp(-1.0, 1.0),
        }
    }

    /// The body's frame: bobbing with each step and breath, leaning into a
    /// stroke, and settling lower in sleep.
    fn body(&self, sink: f32) -> Frame {
        let bob = 0.012 * (self.stride * 2.0).sin().abs() * self.walk
            + 0.004 * self.breath * (1.0 - self.walk);
        let roll = 0.12 * self.lean + 0.1 * self.dance * (self.time * 6.0).sin() + self.sway;
        Frame {
            origin: Vec3::new(0.0, bob - sink * self.sleep + self.lift, 0.0),
            axes: Mat3::from_rotation_z(roll)
                * Mat3::from_diagonal(Vec3::new(
                    1.0 / self.stretch.sqrt(),
                    self.stretch,
                    1.0 / self.stretch.sqrt(),
                )),
        }
    }

    /// A tail's sway at a point along it, from the root at 0.
    fn tail_sway(&self, along: f32) -> f32 {
        let speed = 3.0 + 9.0 * self.wag;
        let size = 0.12 + 0.45 * self.wag;
        (self.time * speed - along * 1.4).sin() * size * (1.0 - 0.8 * self.sleep)
    }

    /// The angle of a leg's swing, with `offset` its share of the stride.
    fn swing(&self, offset: f32) -> f32 {
        (self.stride + offset * TAU).sin() * self.walk
    }

    /// How high a foot lifts at that point of its stride.
    fn lift(&self, offset: f32) -> f32 {
        (self.stride + offset * TAU).cos().max(0.0) * self.walk
    }

    /// The flap of wings or flippers: up and down while flying, or a happy
    /// flutter, folded at rest.
    fn flap(&self, flying: f32) -> f32 {
        let beat = (self.time * 13.0).sin();
        let flutter = (self.time * 9.0).sin() * 0.35 * self.dance;
        beat * 0.9 * flying + flutter
    }
}

/// A lighter or darker shade of `color`.
fn shade(color: Vec3, amount: f32) -> Vec3 {
    if amount >= 0.0 {
        color.lerp(Vec3::ONE, amount)
    } else {
        color * (1.0 + amount)
    }
}

/// The face plate on a head of `radius` in `head`, its middle `drop` below
/// the head's middle and `size` the face's scale.
fn plate(head: &Frame, radius: f32, drop: f32, size: f32, lit: bool) -> Plate {
    Plate {
        center: head.point(Vec3::new(0.0, -drop, 0.0)),
        right: head.axes.col(0).normalize_or(Vec3::X),
        up: head.axes.col(1).normalize_or(Vec3::Y),
        forward: head.axes.col(2).normalize_or(Vec3::Z),
        unit: radius / 0.46 * size,
        lit,
    }
}

/// Four legs under a body in `frame`: shoulders and hips at `spread` apart,
/// `front` and `back` along it, reaching the ground `stance` below the
/// body's middle. Diagonal pairs step together.
#[allow(clippy::too_many_arguments)]
fn four_legs(
    sculpt: &mut Sculpt,
    rig: &Rig,
    body: &Frame,
    spread: f32,
    front: f32,
    back: f32,
    top: f32,
    stance: f32,
    radius: f32,
    leg: Paint,
    paw: Paint,
) {
    for (index, (side, along)) in [(-1.0, front), (1.0, front), (-1.0, back), (1.0, back)]
        .into_iter()
        .enumerate()
    {
        let offset = match index {
            0 | 3 => 0.0,
            _ => 0.5,
        };
        let hip = body.point(Vec3::new(side * spread, top, along));
        let reach = 0.06 * rig.swing(offset);
        let lift = 0.05 * rig.lift(offset);
        // Tucked up in the air, folded under in sleep.
        let tuck = 0.5 * rig.air + 0.75 * rig.sleep;
        let foot_y = -stance + lift + tuck * (stance - 0.06);
        let foot = Vec3::new(
            side * (spread + 0.005),
            foot_y,
            along + reach + rig.air * 0.04 * along.signum(),
        );
        sculpt.melted(0.03, |sculpt| {
            sculpt.within(Frame::ROOT, |sculpt| {
                sculpt.cone(hip, foot + Vec3::Y * radius * 0.6, radius * 1.15, radius, leg);
            });
        });
        sculpt.melted(0.015, |sculpt| {
            sculpt.within(Frame::ROOT, |sculpt| {
                sculpt.ellipsoid(
                    foot + Vec3::new(0.0, radius * 0.45, radius * 0.4),
                    Vec3::new(radius * 1.15, radius * 0.8, radius * 1.5),
                    paw,
                );
            });
        });
    }
}

fn fox(sculpt: &mut Sculpt, rig: &Rig, coat: u8, look: &mut Look) -> Option<Plate> {
    let (fur, cream, sock, inner) = match coat {
        1 => (
            Vec3::new(0.93, 0.94, 0.97),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.72, 0.75, 0.84),
            Vec3::new(0.86, 0.72, 0.76),
        ),
        2 => (
            Vec3::new(0.4, 0.39, 0.45),
            Vec3::new(0.9, 0.9, 0.93),
            Vec3::new(0.14, 0.13, 0.16),
            Vec3::new(0.25, 0.22, 0.26),
        ),
        _ => (
            Vec3::new(0.94, 0.5, 0.2),
            Vec3::new(0.99, 0.95, 0.87),
            Vec3::new(0.26, 0.17, 0.14),
            Vec3::new(0.36, 0.2, 0.15),
        ),
    };
    let stance = 0.2;
    let body = rig.body(0.09);
    let build = rig.build;
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.06, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, 0.0, -0.02),
                Vec3::new(0.12 * build, 0.11 + 0.006 * rig.breath, 0.19),
                Paint::Plain(fur),
            );
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.035, 0.07),
                Vec3::new(0.085 * build, 0.08, 0.1),
                Paint::Plain(cream),
            );
        });
    });
    four_legs(
        sculpt,
        rig,
        &body,
        0.06,
        0.1,
        -0.12,
        -0.04,
        stance,
        0.03,
        Paint::Fade {
            top: fur,
            bottom: sock,
            middle: -0.12,
            span: 0.03,
        },
        Paint::Plain(sock),
    );
    // A bushy tail that curls up behind, wagging with its mood.
    let mut previous = body.point(Vec3::new(0.0, 0.0, -0.18));
    let mut direction = Vec3::new(0.0, 0.45, -1.0).normalize();
    let curl = 0.35 - 0.6 * rig.sleep;
    for (index, (length, radius)) in [(0.09, 0.05), (0.09, 0.068), (0.08, 0.06), (0.06, 0.04)]
        .into_iter()
        .enumerate()
    {
        let sway = rig.tail_sway(index as f32);
        direction = (Mat3::from_rotation_y(sway * 0.8) * Mat3::from_rotation_x(-curl * 0.5))
            * direction;
        if rig.sleep > 0.5 {
            direction = Mat3::from_rotation_y(0.7 * rig.sleep) * direction;
        }
        let next = previous + direction * length;
        let paint = if index == 3 {
            Paint::Plain(cream)
        } else {
            Paint::Plain(fur)
        };
        sculpt.melted(0.04, |sculpt| {
            sculpt.within(Frame::ROOT, |sculpt| {
                sculpt.cone(previous, next, radius * 0.85, radius, paint);
            });
        });
        previous = next;
    }
    let neck = Vec3::new(0.0, 0.07, 0.14);
    let head = body.child(neck, rig.head);
    let radius = 0.15;
    let center = Vec3::new(0.0, 0.08, 0.06);
    let head_frame = head.child(center, Mat3::IDENTITY);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.04, |sculpt| {
            sculpt.ellipsoid(center, Vec3::new(radius, radius * 0.9, radius * 0.92), Paint::Plain(fur));
            // Cheek ruff and a pointed muzzle.
            sculpt.ellipsoid(
                center + Vec3::new(0.0, -0.06, 0.06),
                Vec3::new(0.12, 0.06, 0.08),
                Paint::Plain(cream),
            );
            sculpt.cone(
                center + Vec3::new(0.0, -0.035, 0.08),
                center + Vec3::new(0.0, -0.05, 0.18),
                0.055,
                0.025,
                Paint::Plain(cream),
            );
        });
        sculpt.melted(0.006, |sculpt| {
            sculpt.ball(center + Vec3::new(0.0, -0.04, 0.2), 0.02, Paint::Gloss(sock * 0.6));
        });
        let droop = (-rig.perk).max(0.0) + 0.5 * rig.sleep;
        for side in [-1.0f32, 1.0] {
            let base = center + Vec3::new(side * 0.075, 0.09, -0.01);
            let tip = base
                + Vec3::new(side * (0.05 + 0.04 * droop), 0.15 - 0.07 * droop, -0.02 - 0.05 * droop)
                    * (1.0 + 0.06 * rig.perk.max(0.0));
            sculpt.melted(0.02, |sculpt| {
                sculpt.cone(base, tip, 0.055, 0.01, Paint::Fade {
                    top: sock,
                    bottom: fur,
                    middle: 0.0,
                    span: 0.0,
                });
            });
            sculpt.melted(0.004, |sculpt| {
                sculpt.cone(
                    base + Vec3::new(0.0, 0.01, 0.025),
                    tip.lerp(base, 0.25) + Vec3::new(0.0, 0.0, 0.02),
                    0.03,
                    0.006,
                    Paint::Plain(inner),
                );
            });
        }
    });
    look.top = head_frame.point(Vec3::new(0.0, radius + 0.12, 0.0));
    look.hat = Some(head_frame.child(Vec3::new(0.0, radius * 0.85, -0.02), Mat3::IDENTITY));
    look.neck = Some((body.child(Vec3::new(0.0, 0.06, 0.13), Mat3::from_rotation_x(0.6)), 0.085));
    Some(plate(&head_frame, radius, -0.025, 0.95, false))
}

fn bunny(sculpt: &mut Sculpt, rig: &Rig, coat: u8, seed: u32, look: &mut Look) -> Option<Plate> {
    let (fur, belly, inner) = match coat {
        1 => (
            Vec3::new(0.64, 0.47, 0.34),
            Vec3::new(0.93, 0.86, 0.77),
            Vec3::new(0.95, 0.7, 0.68),
        ),
        2 => (
            Vec3::new(0.64, 0.64, 0.68),
            Vec3::new(0.9, 0.9, 0.92),
            Vec3::new(0.95, 0.72, 0.75),
        ),
        3 => (
            Vec3::new(0.99, 0.94, 0.84),
            Vec3::new(1.0, 0.98, 0.94),
            Vec3::new(1.0, 0.72, 0.74),
        ),
        _ => (
            Vec3::new(0.97, 0.96, 0.95),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 0.7, 0.76),
        ),
    };
    let coat_paint = if coat == 3 {
        Paint::Spots {
            base: fur,
            spot: Vec3::new(0.55, 0.38, 0.26),
            scale: 0.09,
            seed: (seed % 97) as f32,
            glows: false,
        }
    } else {
        Paint::Plain(fur)
    };
    let stance = 0.16;
    let body = rig.body(0.03);
    // Crouched on the ground, stretched out mid-hop.
    let reach = rig.air * 0.04;
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.06, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.01, -0.03),
                Vec3::new(0.14 * rig.build, 0.13 + 0.006 * rig.breath, 0.15 + reach),
                coat_paint,
            );
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.04, 0.06),
                Vec3::new(0.09, 0.08, 0.07),
                Paint::Plain(belly),
            );
        });
        // Big hind feet and small front paws.
        for side in [-1.0f32, 1.0] {
            sculpt.melted(0.03, |sculpt| {
                sculpt.ellipsoid(
                    Vec3::new(side * 0.085, -stance + 0.03 + 0.03 * rig.air, -0.05 - reach * 2.0),
                    Vec3::new(0.045, 0.03, 0.09),
                    coat_paint,
                );
                sculpt.ellipsoid(
                    Vec3::new(side * 0.05, -stance + 0.03, 0.09 + reach),
                    Vec3::new(0.03, 0.03, 0.04),
                    Paint::Plain(belly),
                );
            });
        }
        sculpt.melted(0.02, |sculpt| {
            sculpt.ball(
                Vec3::new(0.0, 0.03 + 0.01 * rig.tail_sway(0.0), -0.17),
                0.05,
                Paint::Plain(Vec3::new(1.0, 1.0, 1.0)),
            );
        });
    });
    let head = body.child(Vec3::new(0.0, 0.1, 0.06), rig.head);
    let radius = 0.13;
    let center = Vec3::new(0.0, 0.07, 0.04);
    let head_frame = head.child(center, Mat3::IDENTITY);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.05, |sculpt| {
            sculpt.ellipsoid(center, Vec3::new(radius * 1.05, radius * 0.95, radius), coat_paint);
            for side in [-1.0f32, 1.0] {
                sculpt.ball(
                    center + Vec3::new(side * 0.06, -0.055, 0.07),
                    0.05,
                    Paint::Plain(belly),
                );
            }
        });
        sculpt.melted(0.005, |sculpt| {
            sculpt.ellipsoid(
                center + Vec3::new(0.0, -0.025, radius * 0.98),
                Vec3::new(0.018, 0.013, 0.012),
                Paint::Gloss(Vec3::new(1.0, 0.55, 0.62)),
            );
        });
        // Long ears: up and alert, or flopped back when sad or sleepy.
        let droop = ((-rig.perk).max(0.0) * 0.8 + rig.sleep * 0.9).min(1.2);
        for side in [-1.0f32, 1.0] {
            let swing = 0.12 * (rig.time * 1.7 + side).sin() * (1.0 - rig.sleep);
            let tilt = Mat3::from_rotation_x(-0.25 - 1.15 * droop)
                * Mat3::from_rotation_z(-side * (0.18 + 0.2 * droop) + swing * 0.3);
            let base = center + Vec3::new(side * 0.05, 0.1, -0.02);
            let ear = head.child(base, tilt);
            sculpt.within(ear, |sculpt| {
                sculpt.melted(0.03, |sculpt| {
                    sculpt.turned(
                        Vec3::new(0.0, 0.13, 0.0),
                        Vec3::new(0.042, 0.14, 0.025),
                        Mat3::IDENTITY,
                        coat_paint,
                    );
                });
                sculpt.melted(0.003, |sculpt| {
                    sculpt.turned(
                        Vec3::new(0.0, 0.13, 0.012),
                        Vec3::new(0.024, 0.11, 0.016),
                        Mat3::IDENTITY,
                        Paint::Plain(inner),
                    );
                });
            });
        }
    });
    look.top = head_frame.point(Vec3::new(0.0, radius + 0.2, 0.0));
    look.hat = Some(head_frame.child(Vec3::new(0.0, radius * 0.8, 0.02), Mat3::IDENTITY));
    look.neck = Some((body.child(Vec3::new(0.0, 0.08, 0.05), Mat3::from_rotation_x(0.3)), 0.09));
    Some(plate(&head_frame, radius, -0.01, 0.92, false))
}

fn penguin(sculpt: &mut Sculpt, rig: &Rig, coat: u8, look: &mut Look) -> Option<Plate> {
    let (back, front, head_color) = match coat {
        1 => (
            Vec3::new(0.62, 0.64, 0.69),
            Vec3::new(0.93, 0.94, 0.96),
            Vec3::new(0.62, 0.64, 0.69),
        ),
        2 => (
            Vec3::new(0.76, 0.77, 0.81),
            Vec3::new(0.98, 0.98, 1.0),
            Vec3::new(0.16, 0.16, 0.19),
        ),
        _ => (
            Vec3::new(0.2, 0.24, 0.33),
            Vec3::new(0.98, 0.98, 1.0),
            Vec3::new(0.2, 0.24, 0.33),
        ),
    };
    let orange = Vec3::new(1.0, 0.62, 0.22);
    let stance = 0.2;
    // A waddle rocks the body side to side with each step.
    let waddle = Mat3::from_rotation_z(0.14 * (rig.stride).sin() * rig.walk);
    let mut body = rig.body(0.05);
    body.axes = body.axes * waddle;
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.05, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.02, 0.0),
                Vec3::new(0.15 * rig.build, 0.17 + 0.005 * rig.breath, 0.14),
                Paint::Plain(back),
            );
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.04, 0.05),
                Vec3::new(0.12 * rig.build, 0.13, 0.1),
                Paint::Plain(front),
            );
        });
        // Flippers flap with joy.
        let flap = 0.25 + 0.6 * rig.flap(0.0).abs() + 0.5 * rig.air;
        for side in [-1.0f32, 1.0] {
            let flipper = Frame {
                origin: Vec3::new(side * 0.13, 0.03, -0.01),
                axes: Mat3::from_rotation_z(side * flap),
            };
            sculpt.within(flipper, |sculpt| {
                sculpt.melted(0.02, |sculpt| {
                    sculpt.ellipsoid(
                        Vec3::new(side * 0.015, -0.07, 0.0),
                        Vec3::new(0.025, 0.08, 0.05),
                        Paint::Plain(back),
                    );
                });
            });
        }
    });
    for side in [-1.0f32, 1.0] {
        let lift = 0.03 * rig.lift(if side < 0.0 { 0.0 } else { 0.5 });
        sculpt.melted(0.01, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(side * 0.06, -stance + 0.015 + lift, 0.06),
                Vec3::new(0.045, 0.015, 0.06),
                Paint::Plain(orange),
            );
        });
    }
    let head = body.child(Vec3::new(0.0, 0.12, 0.0), rig.head);
    let radius = 0.13;
    let center = Vec3::new(0.0, 0.05, 0.01);
    let head_frame = head.child(center, Mat3::IDENTITY);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.06, |sculpt| {
            sculpt.ball(center, radius, Paint::Plain(head_color));
        });
        sculpt.melted(0.03, |sculpt| {
            sculpt.ellipsoid(
                center + Vec3::new(0.0, -0.01, 0.06),
                Vec3::new(0.1, 0.085, 0.07),
                Paint::Plain(front),
            );
        });
        sculpt.melted(0.008, |sculpt| {
            sculpt.cone(
                center + Vec3::new(0.0, -0.045, radius * 0.92),
                center + Vec3::new(0.0, -0.06, radius + 0.045),
                0.025,
                0.006,
                Paint::Plain(orange * 0.9),
            );
        });
        if coat == 1 {
            // A fluffy tuft on top of a chick's head.
            sculpt.melted(0.01, |sculpt| {
                sculpt.cone(
                    center + Vec3::new(0.0, radius * 0.9, 0.0),
                    center + Vec3::new(0.02, radius + 0.05, -0.02),
                    0.025,
                    0.006,
                    Paint::Plain(back),
                );
            });
        }
    });
    look.top = head_frame.point(Vec3::new(0.0, radius + 0.03, 0.0));
    look.hat = Some(head_frame.child(Vec3::new(0.0, radius * 0.88, 0.0), Mat3::IDENTITY));
    look.neck = Some((body.child(Vec3::new(0.0, 0.1, 0.0), Mat3::IDENTITY), 0.11));
    Some(plate(&head_frame, radius, -0.01, 0.8, false))
}

fn frog(sculpt: &mut Sculpt, rig: &Rig, coat: u8, seed: u32, look: &mut Look) -> Option<Plate> {
    let seed = (seed % 89) as f32;
    let (skin, belly) = match coat {
        1 => (
            Paint::Spots {
                base: Vec3::new(0.66, 0.88, 0.3),
                spot: Vec3::new(0.22, 0.45, 0.16),
                scale: 0.07,
                seed,
                glows: false,
            },
            Vec3::new(0.95, 0.96, 0.75),
        ),
        2 => (
            Paint::Spots {
                base: Vec3::new(0.25, 0.55, 0.98),
                spot: Vec3::new(0.06, 0.07, 0.12),
                scale: 0.065,
                seed,
                glows: false,
            },
            Vec3::new(0.55, 0.75, 1.0),
        ),
        3 => (
            Paint::Plain(Vec3::new(1.0, 0.63, 0.72)),
            Vec3::new(1.0, 0.9, 0.9),
        ),
        _ => (
            Paint::Plain(Vec3::new(0.42, 0.78, 0.36)),
            Vec3::new(0.92, 0.95, 0.7),
        ),
    };
    let stance = 0.12;
    let body = rig.body(0.02);
    let reach = rig.air;
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.06, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, 0.0, -0.02),
                Vec3::new(0.15 * rig.build, 0.09 + 0.005 * rig.breath, 0.14 + 0.03 * reach),
                skin,
            );
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.04, 0.03),
                Vec3::new(0.11, 0.05, 0.1),
                Paint::Plain(belly),
            );
        });
        // Folded hind legs at the sides, which kick out behind in a hop.
        for side in [-1.0f32, 1.0] {
            let kick = Vec3::new(0.0, 0.03 * reach, -0.12 * reach);
            sculpt.melted(0.03, |sculpt| {
                sculpt.ellipsoid(
                    Vec3::new(side * 0.13, -0.05, -0.07) + kick * 0.5,
                    Vec3::new(0.05, 0.05, 0.08),
                    skin,
                );
                sculpt.ellipsoid(
                    Vec3::new(side * 0.15, -stance + 0.015, -0.01) + kick,
                    Vec3::new(0.05, 0.015, 0.07),
                    skin,
                );
                sculpt.cone(
                    Vec3::new(side * 0.07, -0.03, 0.08),
                    Vec3::new(side * 0.09, -stance + 0.02, 0.12 + 0.04 * reach),
                    0.025,
                    0.02,
                    skin,
                );
            });
        }
    });
    let head = body.child(Vec3::new(0.0, 0.03, 0.08), rig.head);
    let radius = 0.13;
    let center = Vec3::new(0.0, 0.02, 0.03);
    let head_frame = head.child(center, Mat3::IDENTITY);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.05, |sculpt| {
            sculpt.ellipsoid(center, Vec3::new(0.15, 0.08, 0.11), skin);
            // The eyes sit up on two round bumps.
            for side in [-1.0f32, 1.0] {
                sculpt.ball(center + Vec3::new(side * 0.075, 0.075, 0.035), 0.05, skin);
            }
        });
    });
    look.top = head_frame.point(Vec3::new(0.0, 0.13, 0.0));
    look.hat = Some(head_frame.child(Vec3::new(0.0, 0.1, 0.0), Mat3::IDENTITY));
    look.neck = None;
    let mut face = plate(&head_frame, radius, -0.065, 0.56, false);
    face.center += face.forward * 0.02;
    Some(face)
}

fn slime(sculpt: &mut Sculpt, rig: &Rig, coat: u8, look: &mut Look) -> Option<Plate> {
    let color = match coat {
        1 => Vec3::new(1.0, 0.55, 0.76),
        2 => Vec3::new(0.5, 0.76, 1.0),
        3 => Vec3::new(1.0, 0.92, 0.45),
        _ => Vec3::new(0.45, 0.96, 0.74),
    };
    let stance = 0.13;
    // All wobble: it squashes and stretches as it bounces and breathes.
    let wobble = 1.0 + 0.04 * (rig.time * 3.0).sin() * (1.0 - rig.sleep);
    let squash = rig.stretch * wobble * (1.0 - 0.25 * rig.sleep);
    let flat = 1.0 / squash.sqrt();
    let body = Frame {
        origin: Vec3::new(0.0, (squash - 1.0) * 0.1 + rig.lift, 0.0),
        axes: Mat3::from_rotation_z(0.1 * rig.lean + rig.sway),
    };
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.07, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.005, 0.0),
                Vec3::new(0.17 * flat * rig.build, 0.13 * squash, 0.16 * flat * rig.build),
                Paint::Plain(color),
            );
            sculpt.ball(
                Vec3::new(0.02 * (rig.time * 1.3).sin(), 0.11 * squash, -0.02),
                0.055,
                Paint::Plain(color),
            );
            sculpt.ellipsoid(
                Vec3::new(0.0, -stance + 0.03, 0.0),
                Vec3::new(0.19 * flat, 0.03, 0.18 * flat),
                Paint::Plain(color),
            );
        });
        // A bright core inside.
        sculpt.melted(0.0, |sculpt| {
            sculpt.ball(Vec3::new(0.0, -0.01, -0.02), 0.06, Paint::Glow(color * 0.6));
        });
    });
    let head = body.child(Vec3::new(0.0, 0.0, 0.0), rig.head);
    look.clear = 0.45;
    look.light = color * 0.05;
    look.top = Vec3::new(0.0, 0.16 * squash, 0.0);
    look.hat = Some(body.child(Vec3::new(0.0, 0.12 * squash, 0.0), Mat3::IDENTITY));
    let mut face = plate(&head, 0.15, -0.01, 0.75, false);
    face.center.y = -0.005;
    Some(face)
}

fn owl(sculpt: &mut Sculpt, rig: &Rig, coat: u8, seed: u32, look: &mut Look) -> Option<Plate> {
    let (feather, disk, belly) = match coat {
        1 => (
            Paint::Spots {
                base: Vec3::new(0.97, 0.97, 0.98),
                spot: Vec3::new(0.25, 0.25, 0.28),
                scale: 0.05,
                seed: (seed % 83) as f32,
                glows: false,
            },
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.98, 0.98, 1.0),
        ),
        2 => (
            Paint::Plain(Vec3::new(0.55, 0.56, 0.6)),
            Vec3::new(0.86, 0.86, 0.88),
            Vec3::new(0.78, 0.78, 0.8),
        ),
        _ => (
            Paint::Plain(Vec3::new(0.6, 0.43, 0.3)),
            Vec3::new(0.94, 0.86, 0.72),
            Vec3::new(0.86, 0.75, 0.6),
        ),
    };
    let beak = Vec3::new(0.95, 0.75, 0.3);
    let stance = 0.18;
    let flying = 1.0 - rig.perched;
    let body = rig.body(0.03);
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.07, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.02, 0.0),
                Vec3::new(0.15 * rig.build, 0.16 + 0.005 * rig.breath, 0.14),
                feather,
            );
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.05, 0.06),
                Vec3::new(0.11, 0.1, 0.08),
                Paint::Plain(belly),
            );
        });
        let flap = rig.flap(flying);
        for side in [-1.0f32, 1.0] {
            let open = 0.15 + 0.9 * flying + 0.3 * rig.dance;
            let wing = Frame {
                origin: Vec3::new(side * 0.12, 0.05, -0.01),
                axes: Mat3::from_rotation_z(side * (open + flap)),
            };
            sculpt.within(wing, |sculpt| {
                sculpt.melted(0.03, |sculpt| {
                    sculpt.ellipsoid(
                        Vec3::new(side * 0.02, -0.09, -0.01),
                        Vec3::new(0.03, 0.11, 0.08),
                        feather,
                    );
                });
            });
        }
    });
    for side in [-1.0f32, 1.0] {
        sculpt.melted(0.01, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(side * 0.05, -stance + 0.015, 0.05),
                Vec3::new(0.035, 0.015, 0.04),
                Paint::Plain(beak),
            );
        });
    }
    let head = body.child(Vec3::new(0.0, 0.1, 0.0), rig.head);
    let radius = 0.135;
    let center = Vec3::new(0.0, 0.07, 0.01);
    let head_frame = head.child(center, Mat3::IDENTITY);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.07, |sculpt| {
            sculpt.ellipsoid(center, Vec3::new(radius * 1.05, radius * 0.92, radius), feather);
        });
        sculpt.melted(0.02, |sculpt| {
            for side in [-1.0f32, 1.0] {
                sculpt.ellipsoid(
                    center + Vec3::new(side * 0.055, -0.005, 0.085),
                    Vec3::new(0.065, 0.07, 0.04),
                    Paint::Plain(disk),
                );
            }
        });
        sculpt.melted(0.005, |sculpt| {
            sculpt.cone(
                center + Vec3::new(0.0, -0.03, radius * 0.95),
                center + Vec3::new(0.0, -0.075, radius * 0.98),
                0.018,
                0.004,
                Paint::Plain(beak),
            );
        });
        let perk = 0.3 + 0.3 * rig.perk;
        for side in [-1.0f32, 1.0] {
            sculpt.melted(0.02, |sculpt| {
                sculpt.cone(
                    center + Vec3::new(side * 0.08, 0.07, -0.01),
                    center + Vec3::new(side * (0.12 + 0.02 * perk), 0.13 + 0.03 * perk, -0.03),
                    0.03,
                    0.008,
                    feather,
                );
            });
        }
    });
    look.top = head_frame.point(Vec3::new(0.0, radius + 0.05, 0.0));
    look.hat = Some(head_frame.child(Vec3::new(0.0, radius * 0.85, 0.0), Mat3::IDENTITY));
    look.neck = Some((body.child(Vec3::new(0.0, 0.08, 0.0), Mat3::IDENTITY), 0.12));
    Some(plate(&head_frame, radius, 0.0, 0.8, false))
}

fn robot(sculpt: &mut Sculpt, rig: &Rig, coat: u8, glow: Vec3, look: &mut Look) -> Option<Plate> {
    let (shell, accent) = match coat {
        1 => (Vec3::new(0.96, 0.9, 0.78), Vec3::new(0.9, 0.32, 0.3)),
        2 => (Vec3::new(1.0, 0.82, 0.32), Vec3::new(0.4, 0.43, 0.5)),
        _ => (Vec3::new(0.92, 0.94, 0.96), Vec3::new(0.25, 0.75, 0.78)),
    };
    let screen = Vec3::new(0.05, 0.07, 0.1);
    let stance = 0.22;
    // One wheel, which turns with the ground covered.
    let wheel = Frame {
        origin: Vec3::new(0.0, -stance + 0.065, 0.0),
        axes: Mat3::from_rotation_z(PI * 0.5) * Mat3::from_rotation_y(-rig.stride),
    };
    sculpt.within(wheel, |sculpt| {
        sculpt.melted(0.0, |sculpt| {
            sculpt.ring(Vec3::ZERO, Mat3::IDENTITY, 0.045, 0.022, Paint::Plain(Vec3::splat(0.2)));
            sculpt.block(
                Vec3::ZERO,
                Vec3::new(0.05, 0.012, 0.012),
                0.005,
                Paint::Plain(accent),
            );
        });
    });
    let tilt = Mat3::from_rotation_x(0.12 * (rig.walk - 0.5 * rig.sleep))
        * Mat3::from_rotation_z(0.12 * rig.lean);
    let body = Frame {
        origin: Vec3::new(0.0, 0.006 * rig.breath - 0.04 * rig.sleep + rig.lift, 0.0),
        axes: tilt * Mat3::from_rotation_z(rig.sway),
    };
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.02, |sculpt| {
            sculpt.block(
                Vec3::new(0.0, -0.06, 0.0),
                Vec3::new(0.095, 0.075, 0.075),
                0.035,
                Paint::Plain(shell),
            );
            sculpt.ball(Vec3::new(0.0, -0.06, 0.07), 0.025, Paint::Glow(accent * 0.8));
        });
        // Little arms that wave when it's happy.
        for side in [-1.0f32, 1.0] {
            let wave = 0.3 + 0.9 * rig.dance * (rig.time * 8.0 + side).sin().abs()
                + 0.3 * rig.perk.max(0.0);
            let shoulder = Vec3::new(side * 0.1, -0.04, 0.0);
            let hand = shoulder + Vec3::new(side * 0.06 * wave.cos().abs(), -0.07 + 0.09 * wave, 0.02);
            sculpt.melted(0.01, |sculpt| {
                sculpt.cone(shoulder, hand, 0.014, 0.012, Paint::Plain(accent));
                sculpt.ball(hand, 0.022, Paint::Plain(shell));
            });
        }
    });
    let head = body.child(Vec3::new(0.0, 0.05, 0.0), rig.head);
    let center = Vec3::new(0.0, 0.1, 0.0);
    let head_frame = head.child(center, Mat3::IDENTITY);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.015, |sculpt| {
            sculpt.cone(Vec3::new(0.0, -0.02, 0.0), Vec3::new(0.0, 0.03, 0.0), 0.025, 0.025, Paint::Plain(Vec3::splat(0.35)));
            sculpt.block(center, Vec3::new(0.14, 0.1, 0.1), 0.045, Paint::Plain(shell));
        });
        sculpt.melted(0.0, |sculpt| {
            sculpt.block(
                center + Vec3::new(0.0, 0.0, 0.07),
                Vec3::new(0.115, 0.075, 0.035),
                0.035,
                Paint::Gloss(screen),
            );
            for side in [-1.0f32, 1.0] {
                sculpt.ball(center + Vec3::new(side * 0.14, 0.0, 0.0), 0.028, Paint::Plain(accent));
            }
        });
        let sway = 0.15 * (rig.time * 2.0).sin() + 0.3 * rig.dance * (rig.time * 7.0).sin();
        let tip = center + Vec3::new(sway * 0.12, 0.2, -0.01);
        sculpt.melted(0.006, |sculpt| {
            sculpt.cone(center + Vec3::new(0.0, 0.09, -0.01), tip, 0.008, 0.006, Paint::Plain(Vec3::splat(0.4)));
            sculpt.ball(tip, 0.026, Paint::Glow(glow * (0.5 + 0.5 * rig.energy)));
        });
        look.top = head_frame.point(Vec3::new(0.0, 0.22, 0.0));
    });
    look.light = glow * 0.04;
    look.hat = Some(head_frame.child(Vec3::new(0.0, 0.1, 0.0), Mat3::IDENTITY));
    look.neck = Some((body.child(Vec3::new(0.0, 0.02, 0.0), Mat3::IDENTITY), 0.06));
    let mut face = plate(&head_frame, 0.12, 0.0, 0.55, true);
    face.center = head_frame.point(Vec3::new(0.0, 0.0, 0.1));
    Some(face)
}

fn hedgehog(sculpt: &mut Sculpt, rig: &Rig, coat: u8, look: &mut Look) -> Option<Plate> {
    let (spines, tips, face) = match coat {
        1 => (
            Vec3::new(0.94, 0.92, 0.88),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(1.0, 0.93, 0.9),
        ),
        _ => (
            Vec3::new(0.42, 0.31, 0.23),
            Vec3::new(0.86, 0.76, 0.6),
            Vec3::new(0.97, 0.88, 0.76),
        ),
    };
    let stance = 0.12;
    let body = rig.body(0.02);
    // Curled into a ball asleep; a little dome of spines awake.
    let curl = rig.sleep;
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.04, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.01, -0.02),
                Vec3::new(0.14 * rig.build, 0.1 + 0.004 * rig.breath + 0.02 * curl, 0.16 - 0.03 * curl),
                Paint::Spots {
                    base: spines,
                    spot: tips,
                    scale: 0.035,
                    seed: 3.0,
                    glows: false,
                },
            );
        });
        // Spines: a ring of little points over the back.
        sculpt.melted(0.02, |sculpt| {
            for index in 0..9 {
                let angle = (index as f32 / 8.0 - 0.5) * 2.4;
                let along = (index % 3) as f32 * 0.07 - 0.07;
                let base = Vec3::new(angle.sin() * 0.11, 0.06 * angle.cos() + 0.02, along - 0.03);
                let tip = base * 1.0 + Vec3::new(angle.sin() * 0.05, 0.05, -0.04);
                sculpt.cone(base, tip, 0.03, 0.005, Paint::Plain(spines));
            }
        });
        sculpt.melted(0.03, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.06, 0.05),
                Vec3::new(0.1, 0.05, 0.09),
                Paint::Plain(face),
            );
            for side in [-1.0f32, 1.0] {
                for along in [0.07f32, -0.08] {
                    let lift = 0.02 * rig.lift(if (side > 0.0) == (along > 0.0) { 0.0 } else { 0.5 });
                    sculpt.ball(
                        Vec3::new(side * 0.07, -stance + 0.02 + lift + 0.03 * curl, along),
                        0.022,
                        Paint::Plain(face * 0.85),
                    );
                }
            }
        });
    });
    let head = body.child(Vec3::new(0.0, -0.02, 0.1), rig.head);
    let center = Vec3::new(0.0, 0.0, 0.03);
    let head_frame = head.child(center, Mat3::IDENTITY);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.04, |sculpt| {
            sculpt.ball(center, 0.085, Paint::Plain(face));
            sculpt.cone(
                center + Vec3::new(0.0, -0.02, 0.04),
                center + Vec3::new(0.0, -0.035, 0.13),
                0.05,
                0.018,
                Paint::Plain(face),
            );
        });
        sculpt.melted(0.004, |sculpt| {
            sculpt.ball(center + Vec3::new(0.0, -0.035, 0.145), 0.018, Paint::Gloss(Vec3::splat(0.08)));
            for side in [-1.0f32, 1.0] {
                sculpt.ball(center + Vec3::new(side * 0.06, 0.06, -0.01), 0.025, Paint::Plain(face * 0.9));
            }
        });
    });
    look.top = body.point(Vec3::new(0.0, 0.15, 0.0));
    look.hat = Some(body.child(Vec3::new(0.0, 0.12, 0.02), Mat3::IDENTITY));
    look.neck = None;
    Some(plate(&head_frame, 0.085, -0.012, 0.85, false))
}

fn mushroom(sculpt: &mut Sculpt, rig: &Rig, coat: u8, seed: u32, look: &mut Look) -> Option<Plate> {
    let seed = (seed % 79) as f32;
    let (cap, stem, glowing) = match coat {
        1 => (
            Paint::Spots {
                base: Vec3::new(0.64, 0.43, 0.27),
                spot: Vec3::new(0.93, 0.83, 0.64),
                scale: 0.07,
                seed,
                glows: false,
            },
            Vec3::new(0.95, 0.9, 0.8),
            false,
        ),
        2 => (
            Paint::Spots {
                base: Vec3::new(0.58, 0.34, 0.84),
                spot: Vec3::new(0.65, 1.0, 0.92),
                scale: 0.07,
                seed,
                glows: true,
            },
            Vec3::new(0.9, 0.88, 0.96),
            true,
        ),
        _ => (
            Paint::Spots {
                base: Vec3::new(0.93, 0.22, 0.2),
                spot: Vec3::new(1.0, 0.98, 0.95),
                scale: 0.07,
                seed,
                glows: false,
            },
            Vec3::new(0.98, 0.94, 0.85),
            false,
        ),
    };
    let stance = 0.2;
    let waddle = Mat3::from_rotation_z(0.12 * rig.stride.sin() * rig.walk);
    let mut body = rig.body(0.06);
    body.axes = body.axes * waddle;
    for side in [-1.0f32, 1.0] {
        let lift = 0.03 * rig.lift(if side < 0.0 { 0.0 } else { 0.5 });
        let swing = 0.03 * rig.swing(if side < 0.0 { 0.0 } else { 0.5 });
        sculpt.melted(0.01, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(side * 0.05, -stance + 0.02 + lift, 0.02 + swing),
                Vec3::new(0.035, 0.022, 0.05),
                Paint::Plain(stem * 0.8),
            );
        });
    }
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.04, |sculpt| {
            sculpt.cone(
                Vec3::new(0.0, -0.15, 0.0),
                Vec3::new(0.0, 0.04, 0.0),
                0.095 * rig.build,
                0.08 * rig.build,
                Paint::Plain(stem),
            );
        });
        for side in [-1.0f32, 1.0] {
            let wave = 0.4 * rig.dance * (rig.time * 7.0 + side).sin();
            sculpt.melted(0.01, |sculpt| {
                sculpt.cone(
                    Vec3::new(side * 0.085, -0.03, 0.0),
                    Vec3::new(side * 0.13, -0.08 + 0.1 * wave.abs(), 0.02),
                    0.016,
                    0.014,
                    Paint::Plain(stem),
                );
            });
        }
    });
    let head = body.child(Vec3::new(0.0, 0.05, 0.0), rig.head);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.03, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, 0.08, 0.0),
                Vec3::new(0.21, 0.13, 0.21),
                cap,
            );
            sculpt.ellipsoid(
                Vec3::new(0.0, 0.03, 0.0),
                Vec3::new(0.19, 0.035, 0.19),
                Paint::Plain(stem * 0.9),
            );
        });
    });
    if glowing {
        // Its spots shine like foxfire.
        look.light = Vec3::new(0.4, 0.9, 0.8) * 0.06;
    }
    look.top = head.point(Vec3::new(0.0, 0.22, 0.0));
    look.hat = Some(head.child(Vec3::new(0.0, 0.19, 0.0), Mat3::IDENTITY));
    look.neck = Some((body.child(Vec3::new(0.0, -0.04, 0.0), Mat3::IDENTITY), 0.095));
    let face_frame = body.child(Vec3::new(0.0, -0.065, 0.0), rig.head);
    let mut face = plate(&face_frame, 0.09, 0.0, 0.9, false);
    face.center = face_frame.point(Vec3::new(0.0, 0.0, 0.05));
    Some(face)
}

fn axolotl(sculpt: &mut Sculpt, rig: &Rig, coat: u8, seed: u32, look: &mut Look) -> Option<Plate> {
    let (skin, gills) = match coat {
        1 => (
            Paint::Plain(Vec3::new(1.0, 0.85, 0.5)),
            Vec3::new(1.0, 0.5, 0.3),
        ),
        2 => (
            Paint::Spots {
                base: Vec3::new(0.45, 0.45, 0.4),
                spot: Vec3::new(0.25, 0.25, 0.22),
                scale: 0.05,
                seed: (seed % 71) as f32,
                glows: false,
            },
            Vec3::new(0.58, 0.36, 0.42),
        ),
        _ => (
            Paint::Plain(Vec3::new(1.0, 0.75, 0.8)),
            Vec3::new(1.0, 0.36, 0.52),
        ),
    };
    let stance = 0.1;
    let body = rig.body(0.01);
    // Its body and tail swish side to side as it crawls.
    let swish = 0.18 * rig.stride.sin() * rig.walk;
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.05, |sculpt| {
            sculpt.turned(
                Vec3::new(0.0, 0.0, -0.03),
                Vec3::new(0.09 * rig.build, 0.075 + 0.004 * rig.breath, 0.17),
                Mat3::from_rotation_y(swish * 0.5),
                skin,
            );
            let mut previous = Vec3::new(0.0, 0.0, -0.17);
            for index in 0..3 {
                let sway = rig.tail_sway(index as f32) * 0.8 - swish;
                let next = previous + Mat3::from_rotation_y(sway) * Vec3::new(0.0, 0.0, -0.09);
                sculpt.turned(
                    (previous + next) * 0.5,
                    Vec3::new(0.02, 0.05 - 0.01 * index as f32, 0.06),
                    Mat3::from_rotation_y(sway),
                    skin,
                );
                previous = next;
            }
        });
    });
    four_legs(
        sculpt, rig, &body, 0.075, 0.09, -0.1, -0.02, stance, 0.018, skin, skin,
    );
    let head = body.child(Vec3::new(0.0, 0.02, 0.13), rig.head);
    let center = Vec3::new(0.0, 0.01, 0.05);
    let head_frame = head.child(center, Mat3::IDENTITY);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.04, |sculpt| {
            sculpt.ellipsoid(center, Vec3::new(0.13, 0.085, 0.1), skin);
        });
        // Three feathery gills on each side, fanned out.
        for side in [-1.0f32, 1.0] {
            for index in 0..3 {
                let angle = 0.5 + index as f32 * 0.55;
                let flutter = 0.08 * (rig.time * 3.0 + index as f32).sin();
                let base = center + Vec3::new(side * 0.1, 0.02, -0.03);
                let tip = base
                    + Vec3::new(
                        side * (angle + flutter).cos().abs() * 0.08,
                        (angle + flutter - 0.6).sin() * 0.09,
                        -0.05,
                    );
                sculpt.melted(0.01, |sculpt| {
                    sculpt.cone(base, tip, 0.016, 0.022, Paint::Plain(gills));
                });
            }
        }
    });
    look.top = head_frame.point(Vec3::new(0.0, 0.1, 0.0));
    look.hat = Some(head_frame.child(Vec3::new(0.0, 0.08, 0.0), Mat3::IDENTITY));
    look.neck = None;
    Some(plate(&head_frame, 0.1, -0.005, 0.85, false))
}

fn dragon(sculpt: &mut Sculpt, rig: &Rig, coat: u8, look: &mut Look) -> Option<Plate> {
    let (scales, belly, wings) = match coat {
        1 => (
            Vec3::new(0.6, 0.46, 0.86),
            Vec3::new(0.98, 0.86, 0.72),
            Vec3::new(0.42, 0.31, 0.66),
        ),
        2 => (
            Vec3::new(0.9, 0.36, 0.28),
            Vec3::new(1.0, 0.86, 0.56),
            Vec3::new(0.66, 0.2, 0.2),
        ),
        _ => (
            Vec3::new(0.42, 0.76, 0.46),
            Vec3::new(0.96, 0.89, 0.62),
            Vec3::new(0.3, 0.56, 0.38),
        ),
    };
    let stance = 0.2;
    let flying = 1.0 - rig.perched;
    let body = rig.body(0.08);
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.06, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, 0.0, -0.02),
                Vec3::new(0.12 * rig.build, 0.12 + 0.006 * rig.breath, 0.16),
                Paint::Plain(scales),
            );
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.04, 0.05),
                Vec3::new(0.085, 0.08, 0.09),
                Paint::Plain(belly),
            );
        });
        let flap = rig.flap(flying) + 0.2 * rig.dance * (rig.time * 9.0).sin();
        for side in [-1.0f32, 1.0] {
            let wing = Frame {
                origin: Vec3::new(side * 0.07, 0.1, -0.03),
                axes: Mat3::from_rotation_z(side * (0.5 + 0.6 * flying + flap * 0.8)),
            };
            sculpt.within(wing, |sculpt| {
                sculpt.melted(0.015, |sculpt| {
                    sculpt.turned(
                        Vec3::new(side * 0.1, 0.03, -0.02),
                        Vec3::new(0.11, 0.06, 0.012),
                        Mat3::from_rotation_y(side * 0.3),
                        Paint::Plain(wings),
                    );
                });
            });
        }
        // Little spines down the back.
        sculpt.melted(0.01, |sculpt| {
            for index in 0..3 {
                let along = 0.06 - index as f32 * 0.07;
                sculpt.cone(
                    Vec3::new(0.0, 0.1, along),
                    Vec3::new(0.0, 0.15, along - 0.02),
                    0.018,
                    0.004,
                    Paint::Plain(belly),
                );
            }
        });
    });
    four_legs(
        sculpt,
        rig,
        &body,
        0.06,
        0.09,
        -0.1,
        -0.04,
        stance,
        0.03,
        Paint::Plain(scales),
        Paint::Plain(scales * 0.85),
    );
    let mut previous = body.point(Vec3::new(0.0, -0.01, -0.16));
    for index in 0..4 {
        let sway = rig.tail_sway(index as f32);
        let direction = Mat3::from_rotation_y(sway) * Vec3::new(0.0, 0.02 - 0.04 * index as f32, -1.0).normalize();
        let next = previous + direction * 0.08;
        let radius = 0.045 - index as f32 * 0.01;
        sculpt.melted(0.03, |sculpt| {
            sculpt.cone(previous, next, radius, radius - 0.01, Paint::Plain(scales));
        });
        previous = next;
    }
    sculpt.melted(0.01, |sculpt| {
        sculpt.ellipsoid(previous, Vec3::new(0.035, 0.012, 0.035), Paint::Plain(wings));
    });
    let head = body.child(Vec3::new(0.0, 0.08, 0.12), rig.head);
    let radius = 0.13;
    let center = Vec3::new(0.0, 0.07, 0.04);
    let head_frame = head.child(center, Mat3::IDENTITY);
    sculpt.within(head, |sculpt| {
        sculpt.melted(0.04, |sculpt| {
            sculpt.ball(center, radius, Paint::Plain(scales));
            sculpt.ellipsoid(
                center + Vec3::new(0.0, -0.04, 0.09),
                Vec3::new(0.08, 0.06, 0.07),
                Paint::Plain(scales),
            );
        });
        sculpt.melted(0.01, |sculpt| {
            for side in [-1.0f32, 1.0] {
                sculpt.cone(
                    center + Vec3::new(side * 0.06, 0.09, -0.03),
                    center + Vec3::new(side * 0.08, 0.17, -0.09),
                    0.022,
                    0.006,
                    Paint::Plain(belly),
                );
                sculpt.ball(center + Vec3::new(side * 0.025, -0.02, 0.16), 0.008, Paint::Gloss(scales * 0.4));
            }
        });
    });
    look.top = head_frame.point(Vec3::new(0.0, radius + 0.06, 0.0));
    look.hat = Some(head_frame.child(Vec3::new(0.0, radius * 0.85, -0.01), Mat3::IDENTITY));
    look.neck = Some((body.child(Vec3::new(0.0, 0.06, 0.11), Mat3::from_rotation_x(0.5)), 0.08));
    Some(plate(&head_frame, radius, -0.02, 0.9, false))
}

fn ghost(sculpt: &mut Sculpt, rig: &Rig, coat: u8, glow: Vec3, look: &mut Look) -> Option<Plate> {
    let color = match coat {
        1 => Vec3::new(0.86, 1.0, 0.94),
        2 => Vec3::new(0.94, 0.9, 1.0),
        _ => Vec3::new(0.97, 0.98, 1.0),
    };
    let bob = 0.02 * (rig.time * 2.1).sin() * (1.0 - 0.6 * rig.sleep);
    let body = Frame {
        origin: Vec3::new(0.0, bob + rig.lift, 0.0),
        axes: Mat3::from_rotation_z(0.15 * rig.lean + rig.sway)
            * Mat3::from_rotation_x(0.12 * rig.walk)
            * Mat3::from_diagonal(Vec3::new(1.0, rig.stretch, 1.0)),
    };
    let head = body.child(Vec3::new(0.0, 0.06, 0.0), rig.head);
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.06, |sculpt| {
            sculpt.ball(Vec3::new(0.0, 0.08, 0.0), 0.19, Paint::Plain(color));
            sculpt.cone(
                Vec3::new(0.0, 0.04, 0.0),
                Vec3::new(0.0, -0.17, -0.02 - 0.06 * rig.walk),
                0.19,
                0.21,
                Paint::Plain(color),
            );
        });
        // Round lobes along the hem rise and fall in a wave.
        sculpt.melted(0.03, |sculpt| {
            for index in 0..7 {
                let angle = index as f32 / 7.0 * TAU;
                let wave = 0.02 * (rig.time * 3.0 - index as f32 * 1.3).sin();
                sculpt.ball(
                    Vec3::new(angle.sin() * 0.16, -0.22 + wave, angle.cos() * 0.16 - 0.03 * rig.walk),
                    0.06,
                    Paint::Plain(color),
                );
            }
        });
    });
    look.clear = 0.25;
    look.shine = 0.25;
    look.light = glow * 0.12;
    look.top = body.point(Vec3::new(0.0, 0.28, 0.0));
    look.hat = Some(body.child(Vec3::new(0.0, 0.24, 0.0), Mat3::IDENTITY));
    look.neck = Some((body.child(Vec3::new(0.0, -0.02, 0.0), Mat3::IDENTITY), 0.19));
    let mut face = plate(&head, 0.19, -0.02, 0.85, false);
    face.center = head.point(Vec3::new(0.0, 0.0, 0.0));
    Some(face)
}

fn jelly(sculpt: &mut Sculpt, rig: &Rig, coat: u8, seed: u32, look: &mut Look) -> Option<Plate> {
    let color = match coat {
        1 => Vec3::new(0.6, 0.8, 1.0),
        2 => Vec3::new(0.84, 0.74, 1.0),
        3 => Vec3::new(1.0, 0.82, 0.66),
        _ => Vec3::new(1.0, 0.7, 0.86),
    };
    // The bell pulses, and the ribbons trail behind its motion.
    let pulse = (rig.time * 2.4).sin();
    let body = Frame {
        origin: Vec3::new(0.0, 0.015 * pulse + rig.lift, 0.0),
        axes: Mat3::from_rotation_z(0.12 * rig.lean + rig.sway) * Mat3::from_rotation_x(0.2 * rig.walk),
    };
    sculpt.within(body, |sculpt| {
        sculpt.melted(0.05, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, 0.06, 0.0),
                Vec3::new(0.18 - 0.012 * pulse, 0.13 + 0.01 * pulse, 0.18 - 0.012 * pulse),
                Paint::Spots {
                    base: color,
                    spot: shade(color, 0.6),
                    scale: 0.06,
                    seed: (seed % 61) as f32,
                    glows: false,
                },
            );
            sculpt.ellipsoid(
                Vec3::new(0.0, -0.03, 0.0),
                Vec3::new(0.17, 0.04, 0.17),
                Paint::Plain(shade(color, -0.1)),
            );
        });
        sculpt.melted(0.008, |sculpt| {
            for index in 0..6 {
                let angle = index as f32 / 6.0 * TAU + 0.3;
                let root = Vec3::new(angle.sin() * 0.11, -0.04, angle.cos() * 0.11);
                let sway = (rig.time * 2.0 + index as f32).sin() * 0.03;
                let middle = root + Vec3::new(sway, -0.12, -0.03 * rig.walk);
                let end = middle + Vec3::new(-sway, -0.12, -0.05 * rig.walk);
                sculpt.cone(root, middle, 0.014, 0.011, Paint::Plain(shade(color, 0.2)));
                sculpt.cone(middle, end, 0.011, 0.005, Paint::Plain(shade(color, 0.3)));
            }
        });
    });
    look.clear = 0.5;
    look.shine = 0.3;
    look.light = color * 0.06;
    look.top = body.point(Vec3::new(0.0, 0.2, 0.0));
    look.hat = Some(body.child(Vec3::new(0.0, 0.17, 0.0), Mat3::IDENTITY));
    look.neck = None;
    let head = body.child(Vec3::new(0.0, 0.03, 0.0), rig.head);
    Some(plate(&head, 0.17, 0.0, 0.75, false))
}

/// Puts on an accessory at the crown or around the neck.
fn wear(sculpt: &mut Sculpt, accessory: Accessory, crown: Frame, neck: Option<(Frame, f32)>) {
    let tilt = crown.child(Vec3::ZERO, Mat3::from_rotation_z(-0.2));
    sculpt.melted(0.0, |sculpt| match accessory {
        Accessory::PartyHat => sculpt.within(tilt, |sculpt| {
            sculpt.cone(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.16, 0.0),
                0.065,
                0.008,
                Paint::Spots {
                    base: Vec3::new(1.0, 0.57, 0.72),
                    spot: Vec3::new(1.0, 0.95, 0.5),
                    scale: 0.04,
                    seed: 2.0,
                    glows: false,
                },
            );
            sculpt.ball(Vec3::new(0.0, 0.17, 0.0), 0.02, Paint::Plain(Vec3::new(1.0, 0.95, 0.5)));
        }),
        Accessory::WitchHat => sculpt.within(tilt, |sculpt| {
            sculpt.ellipsoid(
                Vec3::new(0.0, 0.005, 0.0),
                Vec3::new(0.12, 0.01, 0.12),
                Paint::Plain(Vec3::new(0.58, 0.48, 0.85)),
            );
            sculpt.cone(
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.03, 0.18, -0.03),
                0.07,
                0.008,
                Paint::Fade {
                    top: Vec3::new(0.58, 0.48, 0.85),
                    bottom: Vec3::new(1.0, 0.67, 0.3),
                    middle: 0.03,
                    span: 0.005,
                },
            );
        }),
        Accessory::Scarf { color } => {
            if let Some((frame, radius)) = neck {
                let palette = [
                    Vec3::new(1.0, 0.48, 0.36),
                    Vec3::new(1.0, 0.72, 0.36),
                    Vec3::new(0.95, 0.48, 0.7),
                ];
                let wool = if color < 3 {
                    palette[usize::from(color)]
                } else {
                    crate::world::acquaintance::Color::ALL
                        .get(usize::from(color - 3))
                        .map_or(palette[0], |color| color.rgb())
                };
                sculpt.within(frame, |sculpt| {
                    sculpt.ring(Vec3::ZERO, Mat3::IDENTITY, radius, 0.022, Paint::Plain(wool));
                    sculpt.cone(
                        Vec3::new(radius * 0.5, 0.0, radius * 0.8),
                        Vec3::new(radius * 0.7, -0.09, radius * 1.0),
                        0.022,
                        0.018,
                        Paint::Plain(wool * 0.92),
                    );
                });
            }
        }
        Accessory::Flower(kind) => sculpt.within(crown, |sculpt| {
            let petal = super::shapes::Material::Flower(kind).albedo(&crate::world::Plant::default());
            let middle = super::shapes::Material::Center(kind).albedo(&crate::world::Plant::default());
            let at = Vec3::new(0.08, 0.0, 0.03);
            for index in 0..5 {
                let angle = index as f32 / 5.0 * TAU;
                sculpt.ball(at + Vec3::new(angle.cos() * 0.025, angle.sin() * 0.025, 0.0), 0.02, Paint::Plain(petal));
            }
            sculpt.ball(at + Vec3::Z * 0.008, 0.015, Paint::Plain(middle));
        }),
        Accessory::Feather => sculpt.within(crown, |sculpt| {
            sculpt.cone(
                Vec3::new(0.06, -0.01, -0.02),
                Vec3::new(0.12, 0.13, -0.06),
                0.012,
                0.025,
                Paint::Plain(Vec3::new(0.72, 0.86, 1.0)),
            );
        }),
        Accessory::LuckyPouch => {
            if let Some((frame, radius)) = neck {
                sculpt.within(frame, |sculpt| {
                    sculpt.ring(Vec3::ZERO, Mat3::IDENTITY, radius, 0.006, Paint::Plain(Vec3::new(1.0, 0.86, 0.4)));
                    sculpt.ball(Vec3::new(0.0, -0.03, radius + 0.015), 0.03, Paint::Plain(Vec3::new(1.0, 0.43, 0.55)));
                });
            }
        }
        Accessory::Holly => sculpt.within(crown, |sculpt| {
            for side in [-1.0f32, 1.0] {
                sculpt.ellipsoid(
                    Vec3::new(0.07 + side * 0.025, 0.0, 0.02),
                    Vec3::new(0.03, 0.012, 0.018),
                    Paint::Plain(Vec3::new(0.4, 0.88, 0.57)),
                );
            }
            sculpt.ball(Vec3::new(0.07, 0.012, 0.035), 0.013, Paint::Plain(Vec3::new(1.0, 0.4, 0.48)));
        }),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn figure(species: Species) -> Figure {
        Figure {
            id: 1,
            species,
            genome: Genome::default(),
            position: Vec3::new(0.0, species.stance(), 0.0),
            facing: 0.0,
            size: 1.0,
            face: Face::REST,
            glow: Vec3::new(0.4, 0.9, 0.8),
            opacity: 1.0,
            accessory: None,
            motion: Motion {
                energy: 1.0,
                ..Motion::default()
            },
        }
    }

    #[test]
    fn every_body_is_seen_from_the_front_and_finite_in_any_pose() {
        for species in Species::ALL {
            for coat in 0..species.coats() {
                for (pace, asleep, airborne) in [(0.0, 0.0, false), (1.5, 0.0, true), (0.0, 1.0, false)] {
                    let mut figure = figure(species);
                    figure.genome.coat = coat;
                    figure.motion.pace = pace;
                    figure.motion.stride = 0.37;
                    figure.motion.asleep = asleep;
                    figure.motion.airborne = airborne;
                    figure.motion.rise = 2.0;
                    figure.accessory = Some(Accessory::Scarf { color: 1 });
                    let model = Model::of(&figure);
                    let eye = figure.position + Vec3::new(0.0, 0.05, 4.0);
                    let hit = model
                        .trace(eye, (figure.position - eye).normalize(), 100.0, 0.002, 0.0)
                        .unwrap_or_else(|| panic!("{species:?} {coat} {pace}"));
                    assert!(hit.distance > 3.0 && hit.distance < 4.0, "{species:?}");
                    assert!(hit.albedo.is_finite() && hit.normal.is_finite());
                    assert!(hit.normal.z > 0.0, "{species:?} {:?}", hit.normal);
                }
            }
        }
    }

    #[test]
    fn nothing_of_a_body_lies_outside_its_bound() {
        for species in Species::ALL {
            let mut figure = figure(species);
            figure.accessory = Some(Accessory::WitchHat);
            figure.motion.dance = 1.0;
            figure.motion.time = 1.3;
            let model = Model::of(&figure);
            let (center, radius) = model.bounds();
            for part in &model.parts {
                let (middle, reach) = part.bound;
                let world = figure.position + model.turn * middle * model.scale;
                assert!(world.distance(center) + reach * model.scale <= radius + 1e-3, "{species:?}");
            }
        }
    }

    #[test]
    fn a_ray_that_misses_the_bound_finds_nothing_and_a_closer_wall_hides_the_body() {
        let figure = figure(Species::Fox);
        let model = Model::of(&figure);
        let eye = Vec3::new(0.0, 0.2, 4.0);
        assert!(model.trace(eye, Vec3::X, 100.0, 0.002, 0.0).is_none());
        let toward = (figure.position - eye).normalize();
        assert!(model.trace(eye, toward, 2.0, 0.002, 0.0).is_none());
    }

    #[test]
    fn eyes_are_dark_where_the_face_is_printed() {
        let figure = figure(Species::Bunny);
        let model = Model::of(&figure);
        let plate = model.plate.unwrap();
        // Straight at the left eye's middle.
        let eye_local = plate.center + plate.right * (-0.17 * plate.unit) - plate.up * (0.03 * plate.unit);
        let target = figure.position + eye_local;
        let eye = target + Vec3::Z * 3.0;
        let hit = model.trace(eye, (target - eye).normalize(), 100.0, 0.001, 0.0).unwrap();
        assert!(hit.albedo.length() < 0.5, "{}", hit.albedo);
        let cheek = figure.position + plate.center + plate.up * (0.3 * plate.unit);
        let eye = cheek + Vec3::Z * 3.0;
        let hit = model.trace(eye, (cheek - eye).normalize(), 100.0, 0.001, 0.0).unwrap();
        assert!(hit.albedo.length() > 0.8, "{}", hit.albedo);
    }

    #[test]
    fn a_turned_body_faces_its_way() {
        let mut figure = figure(Species::Fox);
        figure.facing = PI * 0.5;
        let model = Model::of(&figure);
        // Its nose now points along +x.
        let ahead = figure.position + Vec3::new(3.0, 0.1, 0.0);
        let hit = model
            .trace(ahead, (figure.position + Vec3::Y * 0.1 - ahead).normalize(), 100.0, 0.002, 0.0)
            .unwrap();
        assert!(hit.point.x > 0.15, "{}", hit.point);
    }

    #[test]
    fn round_cones_measure_exactly_on_their_axis() {
        let a = Vec3::ZERO;
        let b = Vec3::Y;
        assert!((round_cone(Vec3::new(0.5, 0.5, 0.0), a, b, 0.2, 0.2) - 0.3).abs() < 1e-4);
        assert!((round_cone(Vec3::new(0.0, -1.0, 0.0), a, b, 0.2, 0.1) - 0.8).abs() < 1e-4);
        assert!((round_cone(Vec3::new(0.0, 2.0, 0.0), a, b, 0.2, 0.1) - 0.9).abs() < 1e-4);
        assert!(round_cone(Vec3::new(0.0, 0.5, 0.0), a, b, 0.2, 0.1) < 0.0);
    }
}
