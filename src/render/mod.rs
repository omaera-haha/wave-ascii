mod bubbles;
mod dream;
pub mod ghost;
mod shapes;
pub mod window;

use std::f32::consts::PI;

use glam::{Vec2, Vec3};

use crate::face::Face;
use crate::frame::{Frame, Grid, Shade};
use crate::sdf::{capsule, coverage, mix, smoothstep};
use crate::world::mirror;
use crate::world::{Ball, Decoration, Occasion, ROOM_MAX, ROOM_MIN, World};
use ghost::{Aura, BOUNDS, Body, Pose};
use shapes::{Furniture, Material, decoration_light, sphere_span};

/// World units per unit of the ghost's own space.
pub const GHOST_SCALE: f32 = 0.45;
pub const FRIEND_SCALE: f32 = GHOST_SCALE * 0.75;
/// The point the camera circles and looks at.
const TARGET: Vec3 = Vec3::new(0.0, 1.05, 0.0);
/// One over the tangent of half the vertical field of view, about 50 degrees.
const FOCAL: f32 = 2.1;
const EDGE_COLOR: Vec3 = Vec3::new(0.62, 0.72, 1.0);
const WALL_COLOR: Vec3 = Vec3::new(0.78, 0.82, 0.96);
const MOTE_COLOR: Vec3 = Vec3::new(1.0, 0.9, 0.58);
const LAMP_LIGHT: Vec3 = Vec3::new(1.0, 0.78, 0.5);

/// Where the viewer looks from: turned around the room by `yaw`, raised by
/// `pitch`, and moved in by `zoom`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.3,
            zoom: 1.0,
        }
    }
}

impl Camera {
    pub fn turn(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch = (self.pitch + pitch).clamp(-0.15, 1.3);
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.5, 3.0);
    }

    /// The view on a screen sampled by `grid`. Narrow screens move the
    /// camera back so the whole room still fits across.
    pub fn view(&self, grid: Grid) -> View {
        let fit = (1.75 / grid.half_width().max(0.1)).max(1.0);
        let distance = 5.2 * fit / self.zoom;
        let around = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        let eye = TARGET + around * distance;
        let forward = (TARGET - eye).normalize();
        let right = forward.cross(Vec3::Y).normalize();
        View {
            eye,
            forward,
            right,
            up: right.cross(forward),
        }
    }
}

/// A camera placed in the room: where it is and which ways are forward,
/// right, and up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub eye: Vec3,
    pub forward: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

impl View {
    /// The direction of the ray through `point` of the screen, in the
    /// grid's scene space.
    pub fn ray(&self, point: Vec2) -> Vec3 {
        (self.forward * FOCAL + self.right * point.x + self.up * point.y).normalize()
    }

    /// Where `position` appears on the screen, in the grid's scene space, or
    /// `None` when it is behind the camera.
    pub fn project(&self, position: Vec3) -> Option<Vec2> {
        let offset = position - self.eye;
        let depth = offset.dot(self.forward);
        (depth > 0.05)
            .then(|| Vec2::new(offset.dot(self.right), offset.dot(self.up)) * FOCAL / depth)
    }

    /// Where the ray through `point` of the screen meets the plane at
    /// height `height`, if it does in front of the camera.
    pub fn on_plane(&self, point: Vec2, height: f32) -> Option<Vec3> {
        let direction = self.ray(point);
        let distance = (height - self.eye.y) / direction.y;
        (distance.is_finite() && distance > 0.0).then(|| self.eye + direction * distance)
    }

    /// Where the ray through `point` of the screen meets the glass front of
    /// the room, if it does.
    pub fn on_glass(&self, point: Vec2) -> Option<Vec3> {
        let direction = self.ray(point);
        let distance = (ROOM_MAX.z - self.eye.z) / direction.z;
        let hit = self.eye + direction * distance;
        let inside = (ROOM_MIN.x..=ROOM_MAX.x).contains(&hit.x)
            && (ROOM_MIN.y..=ROOM_MAX.y).contains(&hit.y);
        (distance.is_finite() && distance > 0.0 && inside).then_some(hit)
    }

    /// Where the ray meets the window rectangle on the back wall.
    pub fn on_window(&self, point: Vec2) -> Option<Vec3> {
        let direction = self.ray(point);
        if direction.z.abs() < 1e-6 {
            return None;
        }
        let distance = (ROOM_MIN.z - self.eye.z) / direction.z;
        let position = self.eye + direction * distance;
        (distance > 0.0
            && position.truncate().cmpge(window::WINDOW_LOW).all()
            && position.truncate().cmple(window::WINDOW_HIGH).all())
        .then_some(position)
    }

    /// Whether the ray meets the ghost's billboard inside its outline's bounds.
    pub fn touches_ghost(&self, point: Vec2, ghost: Vec3, scale: f32) -> bool {
        self.billboard(self.ray(point), ghost, scale)
            .is_some_and(|(_, local)| local.abs().cmple(Vec2::new(0.6, 0.75)).all())
    }

    /// Where a ray meets the ghost's billboard: the distance along the ray
    /// and the point in the ghost's own space.
    fn billboard(&self, direction: Vec3, ghost: Vec3, scale: f32) -> Option<(f32, Vec2)> {
        self.billboard_from(self.eye, direction, ghost, scale)
    }

    fn mirrored(&self) -> Self {
        Self {
            eye: mirror::reflect_point(self.eye),
            forward: mirror::reflect_direction(self.forward),
            // Reflecting every axis would flip the basis's handedness and
            // cancel the mirror's left-right reversal on billboards, so the
            // right axis turns back to keep it.
            right: -mirror::reflect_direction(self.right),
            up: mirror::reflect_direction(self.up),
        }
    }

    fn billboard_from(
        &self,
        origin: Vec3,
        direction: Vec3,
        ghost: Vec3,
        scale: f32,
    ) -> Option<(f32, Vec2)> {
        let along = direction.dot(self.forward);
        let distance = (ghost - origin).dot(self.forward) / along;
        if along <= 0.0 || distance <= 0.0 {
            return None;
        }
        let offset = origin + direction * distance - ghost;
        Some((
            distance,
            Vec2::new(offset.dot(self.right), offset.dot(self.up)) / scale,
        ))
    }
}

/// Everything one frame shows.
pub struct Scene<'a> {
    pub world: &'a World,
    pub outside: window::Outside,
    pub view: View,
    pub face: Face,
    pub aura: Aura,
    pub pose: Pose,
    pub friend: Option<Body>,
}

/// Renders the scene, averaging `samples` by `samples` rays a pixel or so
/// for smooth edges.
pub fn render(scene: &Scene, grid: Grid, samples: usize) -> Frame {
    if samples > 1 {
        let (fine, across, down) = grid.refine(samples);
        return render(scene, fine, 1).downsample(across, down);
    }
    let ghost_scale = GHOST_SCALE * scene.world.size();
    let dream = dream::Bubble::new(scene, ghost_scale, grid.scale());
    let mut bodies = [
        Some(Body {
            position: scene.world.ghost.position,
            scale: ghost_scale,
            pose: scene.pose,
            face: scene.face,
            aura: scene.aura,
            opacity: 1.0,
            accessory: scene.world.wearing,
        }),
        scene.friend,
    ];
    if let [Some(first), Some(second)] = bodies
        && first.position.dot(scene.view.forward) < second.position.dot(scene.view.forward)
    {
        bodies.swap(0, 1);
    }
    let lights = Lights::of(scene);
    let world = scene.world;
    let curtain_gust = if world.window_open {
        world.wind.gust(world.time)
    } else {
        0.0
    };
    let shown_token = world.shown_token();
    let shown_book = shown_token.and_then(|(token, position)| match token {
        crate::world::show::Token::Book(index) => Some((index, position, 0.0)),
        _ => None,
    });
    let held_book = world.held_book().or(shown_book);
    let garland = shapes::Garland::of(world);
    let mut book_pile: Vec<_> = world
        .life
        .written
        .iter()
        .enumerate()
        .map(|(index, _)| crate::world::books::OwnedBook {
            index: crate::world::books::LIBRARY.len() + index,
            ..Default::default()
        })
        .collect();
    book_pile.extend(world.life.books.owned.iter().cloned());
    let mut furniture = vec![
        Furniture::Books(&book_pile, held_book.map(|(index, _, _)| index)),
        Furniture::Curtain(-1.0, curtain_gust),
        Furniture::Curtain(1.0, curtain_gust),
        Furniture::Lamp(world.lamp),
        if world.window_open {
            Furniture::SwayingPlant(world.plant, world.breeze_at(world.plant.top()))
        } else {
            Furniture::Plant(world.plant)
        },
        Furniture::Nook(world.nook),
        Furniture::Shelf(
            &world.life.keepsakes,
            world.wearing.and_then(|accessory| accessory.keepsake()),
            shown_token.and_then(|(token, _)| match token {
                crate::world::show::Token::Keepsake(_, index) => Some(index),
                _ => None,
            }),
        ),
        Furniture::Mirror,
        Furniture::MusicBox(
            world.music.is_some(),
            world
                .music
                .as_ref()
                .map_or(0.0, |playing| playing.beat(world.time) * PI),
        ),
    ];
    furniture.push(Furniture::Gallery(&world.life.gallery));
    if let Some(position) = world.carried_pin() {
        furniture.push(Furniture::GalleryCard(position));
    }
    if let Some((position, brightness)) = world.treasure() {
        let twinkle = 0.75 + 0.25 * (world.time as f32 * 4.0).sin().powi(8);
        let light = if world.lamp.on { 1.0 } else { 0.3 };
        furniture.push(Furniture::Treasure {
            position,
            radius: crate::world::hunt::RADIUS * brightness.min(1.0).sqrt().max(0.01),
            brightness: brightness * twinkle * light,
        });
    }
    if !world.life.cranes.colors.is_empty() {
        furniture.push(Furniture::Cranes(&garland));
    }
    if let Some((token, position)) = shown_token {
        let rotation = glam::Quat::from_rotation_y(-scene.view.right.z.atan2(scene.view.right.x));
        let position = position - scene.view.forward * 0.2 * world.size();
        match token {
            crate::world::show::Token::Keepsake(gift, _) => {
                furniture.push(Furniture::Keepsake(gift, position))
            }
            crate::world::show::Token::Crane(color, _) => furniture.push(Furniture::FoldingPaper(
                crate::world::cranes::Paper {
                    color,
                    progress: 1.0,
                },
                position,
                rotation,
            )),
            crate::world::show::Token::Song => {
                furniture.push(Furniture::SongSheet(position, rotation))
            }
            crate::world::show::Token::Book(_) => {}
        }
    }
    if let Some((paper, position)) = world.held_paper() {
        furniture.push(Furniture::FoldingPaper(
            paper,
            position - scene.view.forward * 0.2 * world.size(),
            glam::Quat::from_rotation_y(-scene.view.right.z.atan2(scene.view.right.x)),
        ));
    }
    if let Some(position) = world.viewer_paper() {
        furniture.push(Furniture::ViewerLetter(position));
    }
    if world.unread_letter().is_some() {
        furniture.push(Furniture::Envelope);
    }
    if let Some((index, position, turning)) = held_book {
        furniture.push(Furniture::OpenBook {
            index,
            position: position - scene.view.forward * 0.2 * world.size(),
            size: world.size(),
            rotation: glam::Quat::from_rotation_y(-scene.view.right.z.atan2(scene.view.right.x)),
            turning,
            ink: (index >= crate::world::books::LIBRARY.len()).then(|| {
                if let Some(reading) = &world.reading
                    && reading.writing
                {
                    let started = reading
                        .turned_at
                        .or(world.ghost.plan.arrived)
                        .unwrap_or(world.time);
                    let ends = reading.next_at.unwrap_or(started + 20.0);
                    ((world.time - started) / (ends - started).max(0.1)).clamp(0.0, 1.0) as f32
                } else {
                    1.0
                }
            }),
        });
    }
    if let Some(decoration) = world.decoration() {
        furniture.push(Furniture::Decoration(decoration, world.time));
    }
    if let Some(gift) = world.ghost.keepsake {
        furniture.push(Furniture::Keepsake(
            gift,
            world.ghost.position - Vec3::Y * 0.5,
        ));
    }
    furniture.extend(world.music_notes.iter().map(|note| {
        Furniture::MusicNote(
            note.position(world.time),
            note.flagged,
            (note.fade(world.time) * 255.0) as u8,
        )
    }));
    furniture.extend(world.blocks.iter().copied().map(Furniture::Block));
    furniture.extend(world.seedlings.iter().enumerate().map(|(index, plant)| {
        Furniture::Seedling(plant, crate::world::garden::SPOTS[index], world.season())
    }));
    furniture.extend(
        world
            .petals
            .iter()
            .copied()
            .map(|petal| Furniture::Petal(petal, world.time)),
    );
    if world.puddle > 0.0 {
        furniture.push(Furniture::Puddle(world.puddle));
    }
    furniture.extend(
        world
            .leaves
            .iter()
            .copied()
            .map(|leaf| Furniture::Leaf(leaf, world.time)),
    );
    if let Some(seed) = world.carried_seed() {
        furniture.push(Furniture::Seed(seed));
    }
    // How wide one pixel is, in radians.
    let spread = grid.scale() / FOCAL;
    let mut reflected_bodies = bodies;
    let mirrored_view = scene.view.mirrored();
    if let [Some(first), Some(second)] = reflected_bodies
        && first.position.dot(mirrored_view.forward) < second.position.dot(mirrored_view.forward)
    {
        reflected_bodies.swap(0, 1);
    }
    let shells: Vec<_> = world
        .bubbles
        .iter()
        .filter_map(|bubble| bubbles::Shell::new(bubble, scene.view, world.time, grid.scale()))
        .collect();
    if let Some(position) = world.bubble_wand() {
        furniture.push(Furniture::BubbleWand(position));
    }
    let tracer = Tracer {
        scene,
        lights: &lights,
        furniture: &furniture,
        bodies: [bodies, reflected_bodies],
        spread,
        bubbles: &shells,
    };
    Frame::sample(grid, |point| {
        let direction = scene.view.ray(point);
        let dream = dream.as_ref().filter(|bubble| bubble.contains(point));
        let color = tracer.trace(scene.view.eye, direction, scene.view, 0.0, dream, false);
        Shade::from_rgb(color.min(Vec3::ONE))
    })
}

struct Tracer<'a> {
    scene: &'a Scene<'a>,
    lights: &'a Lights,
    furniture: &'a [Furniture<'a>],
    bodies: [[Option<Body>; 2]; 2],
    spread: f32,
    bubbles: &'a [bubbles::Shell],
}

impl Tracer<'_> {
    fn trace(
        &self,
        origin: Vec3,
        direction: Vec3,
        view: View,
        traveled: f32,
        dream: Option<&dream::Bubble>,
        reflected: bool,
    ) -> Vec3 {
        let scene = self.scene;
        let lights = self.lights;
        let furniture = self.furniture;
        let bodies = &self.bodies[usize::from(reflected)];
        let spread = self.spread;
        let world = scene.world;
        let eye = origin;
        let room = room_exit(eye, direction);
        let mut limit = room.map_or(f32::INFINITY, |hit| hit.distance);
        let mut color = room.map_or(Vec3::ZERO, |hit| {
            if reflected && hit.axis == 2 && hit.high {
                Vec3::ZERO
            } else {
                room_color(
                    &hit,
                    lights,
                    (traveled + hit.distance) * spread,
                    &scene.outside,
                )
            }
        });

        if let Some((enter, _)) = sphere_span(eye, direction, world.ball.position, Ball::RADIUS)
            && enter > 0.0
            && enter < limit
        {
            limit = enter;
            let point = eye + direction * enter;
            let normal = (point - world.ball.position) / Ball::RADIUS;
            color = Material::Ball.albedo(&world.plant) * lights.at(point, normal);
        }
        for piece in furniture {
            if let Some((distance, material)) = piece.trace(eye, direction, limit) {
                limit = distance;
                let point = eye + direction * distance;
                let normal = piece.normal(point);
                let albedo = material.albedo(&world.plant);
                color = albedo * lights.at(point, normal)
                    + material.emission(world.time, &world.plant)
                    + match material {
                        Material::Water => {
                            Vec3::new(0.4, 0.55, 0.7)
                                * (0.2 + 0.6 * (1.0 - normal.dot(-direction).abs()).powi(3))
                        }
                        Material::Bulb if world.lamp.on => albedo * 1.4,
                        Material::Lampshade if world.lamp.on => LAMP_LIGHT * 0.35,
                        Material::Treasure => {
                            let brightness = if let Furniture::Treasure { brightness, .. } = piece {
                                *brightness
                            } else {
                                0.0
                            };
                            albedo * brightness * 1.2
                        }
                        _ => Vec3::ZERO,
                    };
            }
        }
        if !reflected
            && let Some(hit) = room
            && hit.axis == 0
            && hit.high
            && mirror::contains(hit.point)
            && limit == hit.distance
        {
            let reflected_direction = mirror::reflect_direction(direction);
            let reflected_color = self.trace(
                hit.point - Vec3::X * 1e-4,
                reflected_direction,
                view.mirrored(),
                traveled + hit.distance,
                None,
                true,
            );
            let sheen = 0.025 + 0.055 * (1.0 - direction.x.abs()).powi(3);
            color =
                reflected_color * Vec3::new(0.86, 0.9, 0.95) + Vec3::new(0.55, 0.7, 0.9) * sheen;
        }
        if let Some(moth) = &world.visitors.moth
            && let Some((distance, local)) =
                view.billboard_from(eye, direction, moth.position, GHOST_SCALE)
            && distance < limit
        {
            let point = local * GHOST_SCALE;
            let pixel = ((traveled + distance) * spread).max(1e-5);
            // Nothing of the moth, outspread wings and their soft edge
            // included, reaches farther; the wings' distance is scaled down,
            // so their edge widens with the pixel.
            let coverage = if point.length() < MOTH_REACH + 1.6 * pixel {
                moth_coverage(point, pixel, moth.flap(world.time))
            } else {
                0.0
            };
            if coverage > 0.0 {
                let warm = if world.lamp.on {
                    (1.0 - moth.position.distance(world.lamp.bulb()) / 0.7).max(0.0)
                } else {
                    0.0
                };
                let albedo = Vec3::new(0.72, 0.72, 0.68);
                let light = lights.at(moth.position, -direction).max(Vec3::splat(0.35));
                color = color.lerp(albedo * light + LAMP_LIGHT * warm * 0.45, coverage);
                if coverage > 0.5 {
                    limit = distance;
                }
            }
        }

        // The glass, with whatever is drawn on it, lies over what is behind it,
        // and under the ghost when the ghost is nearer.
        let glass = (!reflected)
            .then(|| glass_distance(eye, direction))
            .flatten()
            .filter(|distance| *distance <= limit + 1e-3);
        let soap = (!reflected && !self.bubbles.is_empty())
            .then(|| bubbles::hits(self.bubbles, view, direction, limit, spread * FOCAL));
        let mut soap = soap
            .iter()
            .flat_map(|hits| hits.iter().flatten())
            .peekable();
        let mut glazed = false;
        let mut dream = dream.and_then(|bubble| {
            view.billboard_from(eye, direction, bubble.position, bubble.scale)
                .filter(|(distance, _)| *distance < limit)
                .map(|(distance, local)| (distance, local, bubble))
        });
        let mut overlay = |distance: f32, glow: ghost::Glow| {
            if let Some(glass) = glass
                && glass > distance
                && !glazed
            {
                color = glass_color(scene, eye, direction, spread, glass, color);
                glazed = true;
            }
            color = glow.over(color);
        };
        for body in bodies.iter().flatten() {
            if body.opacity <= 0.0 {
                continue;
            }
            let through_window = body.position.z < ROOM_MIN.z
                && room.is_some_and(|hit| {
                    hit.axis == 2 && !hit.high && (limit - hit.distance).abs() < 1e-4
                });
            let Some((distance, local)) =
                view.billboard_from(eye, direction, body.position, body.scale)
            else {
                continue;
            };
            if (distance >= limit && !through_window)
                || local.cmplt(BOUNDS[0]).any()
                || local.cmpgt(BOUNDS[1]).any()
            {
                continue;
            }
            if let Some((distance, local, bubble)) =
                dream.take_if(|(dream_distance, _, _)| *dream_distance >= distance)
            {
                overlay(
                    distance,
                    bubble.shade(local, (traveled + distance) * spread / bubble.scale),
                );
            }
            while soap
                .peek()
                .is_some_and(|(bubble_distance, _)| *bubble_distance >= distance)
            {
                let &(bubble_distance, glow) = soap.next().unwrap();
                overlay(bubble_distance, glow);
            }
            let pixel = (traveled + distance) * spread / body.scale;
            overlay(distance, ghost::shade(local, pixel, world.time, body));
        }
        if let Some((distance, local, bubble)) = dream {
            overlay(
                distance,
                bubble.shade(local, (traveled + distance) * spread / bubble.scale),
            );
        }
        for &(distance, glow) in soap {
            overlay(distance, glow);
        }
        if let Some(glass) = glass
            && !glazed
        {
            color = glass_color(scene, eye, direction, spread, glass, color);
        }

        // Light motes and the lamp's bulb glow in front of whatever is behind.
        for mote in &world.motes {
            color += mote_color(world)
                * sprite(
                    eye,
                    direction,
                    mote.position,
                    limit,
                    spread,
                    0.035,
                    traveled,
                )
                * mote.glow(world.time);
        }
        if world.lamp.on {
            color += LAMP_LIGHT
                * sprite(
                    eye,
                    direction,
                    world.lamp.bulb(),
                    limit + 0.3,
                    spread,
                    0.12,
                    traveled,
                )
                * 0.3;
        }
        color
    }
}

/// How far the moth reaches from its center, wings spread.
const MOTH_REACH: f32 = 0.13;

fn moth_coverage(point: Vec2, pixel: f32, flap: f32) -> f32 {
    let wing_width = (0.025 + 0.035 * flap.abs()).max(pixel.min(0.09) * 0.8);
    let mut distance = capsule(point, Vec2::new(0.0, -0.023), Vec2::new(0.0, 0.023), 0.014);
    for side in [-1.0, 1.0] {
        let center = Vec2::new(side * wing_width * 0.7, flap * 0.022);
        let wing = ((point - center) / Vec2::new(wing_width, 0.028)).length() - 1.0;
        distance = distance.min(wing * wing_width.min(0.028));
    }
    coverage(distance, pixel * 0.6)
}

fn mote_color(world: &World) -> Vec3 {
    if world.occasion() == Some(Occasion::Valentine) {
        Vec3::new(1.0, 0.5, 0.75)
    } else {
        MOTE_COLOR
    }
}

// How far along the ray it meets the glass, if it does.
fn glass_distance(eye: Vec3, direction: Vec3) -> Option<f32> {
    let distance = (ROOM_MAX.z - eye.z) / direction.z;
    if !distance.is_finite() || distance <= 0.0 {
        return None;
    }
    let hit = eye + direction * distance;
    ((ROOM_MIN.x..=ROOM_MAX.x).contains(&hit.x) && (ROOM_MIN.y..=ROOM_MAX.y).contains(&hit.y))
        .then_some(distance)
}

// The fog and its strokes on the glass, which the ray meets at `distance`,
// laid over `color`.
fn glass_color(
    scene: &Scene,
    origin: Vec3,
    direction: Vec3,
    spread: f32,
    distance: f32,
    mut color: Vec3,
) -> Vec3 {
    if scene.world.doodles.is_empty()
        && scene.world.game.is_none()
        && scene.world.drawing.is_none()
        && scene.world.puzzle.is_none()
    {
        return color;
    }
    let hit = origin + direction * distance;
    let point = Vec2::new(hit.x, hit.y);
    let footprint = distance * spread / direction.z.abs().max(0.01);
    let frost = Vec3::new(0.8, 0.92, 1.0);
    for doodle in &scene.world.doodles {
        let offset = point - doodle.center;
        let radius = doodle.picture.extent().max(Vec2::splat(0.02)) * doodle.size * 1.3;
        let mist_edge = footprint.max(radius.min_element() * 0.2);
        if offset.abs().cmpgt(radius + Vec2::splat(mist_edge)).any() {
            continue;
        }
        let opacity = doodle.opacity(scene.world.time);
        let mist = coverage(
            ((offset / radius).length() - 1.0) * radius.min_element(),
            mist_edge,
        );
        color = color.lerp(frost, mist * 0.12 * opacity);
        let stroke = doodle
            .picture
            .distance(offset / doodle.size, doodle.drawn(scene.world.time))
            * doodle.size;
        let stroke = coverage(stroke - 0.0125, footprint * 0.5);
        color = color.lerp(frost, stroke * 0.85 * opacity);
    }
    if let Some(game) = &scene.world.game {
        let offset = (point - game.center).abs();
        let radius = game.cell_size * 1.5;
        let mist_edge = footprint.max(game.cell_size * 0.25);
        if offset.max_element() <= radius + mist_edge {
            let time = scene.world.time;
            let opacity = game.opacity(time);
            let mist = coverage(offset.max_element() - radius, mist_edge);
            color = color.lerp(frost, mist * 0.12 * opacity);
            let stroke = coverage(game.distance(point, time) - 0.0125, footprint * 0.5);
            color = color.lerp(frost, stroke * 0.85 * opacity);
            let winning = coverage(game.winning_distance(point, time) - 0.015, footprint * 0.5);
            color = color.lerp(Vec3::ONE, winning * 0.95 * opacity);
        }
    }
    if let Some(puzzle) = &scene.world.puzzle {
        let offset = (point - puzzle.center).abs();
        if offset
            .cmple(crate::world::Puzzle::EXTENT + Vec2::splat(footprint))
            .all()
        {
            let opacity = puzzle.opacity(scene.world.time);
            let stroke = coverage(
                puzzle.distance(point, scene.world.time) - 0.0125,
                footprint * 0.5,
            );
            let ink = if puzzle.solved { Vec3::ONE } else { frost };
            color = color.lerp(ink, stroke * 0.95 * opacity);
        }
    }
    if let Some(drawing) = &scene.world.drawing {
        let offset = point - drawing.center();
        let radius = ((drawing.maximum - drawing.minimum) * 0.5).max(Vec2::splat(0.04));
        let mist_edge = footprint.max(0.06);
        if offset.abs().cmple(radius + Vec2::splat(mist_edge)).all() {
            let opacity = drawing.opacity(scene.world.time);
            let frost = Vec3::new(1.0, 0.96, 0.88);
            let mist = coverage((offset.abs() - radius).max_element(), mist_edge);
            color = color.lerp(frost, mist * 0.1 * opacity);
            let stroke = coverage(drawing.distance(point) - 0.018, footprint * 0.5);
            color = color.lerp(frost, stroke * 0.9 * opacity);
        }
    }
    color
}

// How brightly a glowing point lights the ray, falling off with how far the
// ray passes from it.
fn sprite(
    eye: Vec3,
    direction: Vec3,
    center: Vec3,
    limit: f32,
    spread: f32,
    size: f32,
    traveled: f32,
) -> f32 {
    let along = (center - eye).dot(direction);
    if along <= 0.0 || along > limit + 0.05 {
        return 0.0;
    }
    let miss = (eye + direction * along).distance_squared(center);
    let width = size.max((traveled + along) * spread * 1.2);
    (-miss / (width * width)).exp()
}

/// Where a ray leaves the room: rays pass through the near walls as through
/// glass and show the inside of the far ones.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RoomHit {
    distance: f32,
    point: Vec3,
    /// The axis the wall faces along: 0 for the sides, 1 for the floor and
    /// ceiling, 2 for the back and the glass front.
    axis: usize,
    /// Whether the wall is at the high end of its axis.
    high: bool,
}

fn room_exit(origin: Vec3, direction: Vec3) -> Option<RoomHit> {
    let inverse = direction.recip();
    let first = (ROOM_MIN - origin) * inverse;
    let second = (ROOM_MAX - origin) * inverse;
    let enter = first.min(second).max_element().max(0.0);
    let exits = first.max(second);
    let distance = exits.min_element();
    // A ray along a wall's plane gives no number; it misses.
    if distance.is_nan() || distance <= enter {
        return None;
    }
    let axis = (0..3)
        .min_by(|first, second| exits[*first].total_cmp(&exits[*second]))
        .unwrap_or(0);
    Some(RoomHit {
        distance,
        point: origin + direction * distance,
        axis,
        high: direction[axis] > 0.0,
    })
}

// The inside of a wall: a lattice of dots, densest on the floor, edged by
// dashed lines along the box's corners. The glass shows only its edges.
fn room_color(hit: &RoomHit, lights: &Lights, footprint: f32, outside: &window::Outside) -> Vec3 {
    let (first, second) = match hit.axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let position = Vec2::new(hit.point[first], hit.point[second]);
    let low = Vec2::new(ROOM_MIN[first], ROOM_MIN[second]);
    let high = Vec2::new(ROOM_MAX[first], ROOM_MAX[second]);
    let mut normal = Vec3::ZERO;
    normal[hit.axis] = if hit.high { -1.0 } else { 1.0 };
    let light = lights.at(hit.point, normal);
    if hit.axis == 2
        && !hit.high
        && let Some(window) = window::window(position, footprint, outside)
    {
        let frame_color = Vec3::new(0.62, 0.46, 0.32);
        return window.sky * (1.0 - window.frame) + frame_color * light * window.frame;
    }

    let floor = hit.axis == 1 && !hit.high;
    let glass = hit.axis == 2 && hit.high;
    let (spacing, strength) = match (floor, glass, hit.axis == 1) {
        (true, _, _) => (0.2, 1.0),
        (_, true, _) => (1.0, 0.0),
        (_, _, true) => (0.3, 0.3),
        _ => (0.3, 0.55),
    };
    let radius = 0.018;
    let nearest = (position / spacing).round() * spacing;
    let sharp = coverage(position.distance(nearest) - radius, footprint * 0.6);
    // Far away the dots blur into their average brightness.
    let average = PI * radius * radius / (spacing * spacing) * 2.5;
    let dots = mix(
        sharp,
        average,
        smoothstep(spacing * 0.15, spacing * 0.5, footprint),
    );
    let mut color = WALL_COLOR * light * (0.03 + 0.9 * dots * strength);

    // The wall's own border is an edge of the box.
    let border = (position - low).min(high - position);
    let edge = border.min_element();
    let width = 0.008_f32.max(footprint * 0.5);
    let line = coverage(edge - width, footprint * 0.5);
    if line > 0.0 {
        let along = if border.x < border.y {
            position.y
        } else {
            position.x
        };
        let dash = coverage(
            ((along / 0.09).fract() - 0.5).abs() * 0.09 - 0.022,
            footprint * 0.5,
        );
        let dash = mix(dash, 1.0, smoothstep(0.02, 0.05, footprint));
        let brightness = 0.3 + 0.5 * light.max_element().min(1.0);
        color = color.lerp(EDGE_COLOR * brightness, line * dash);
    }
    color
}

/// The light in the room: daylight and the window, the lamp, and the glow of
/// the ghost and the motes.
struct Lights {
    ambient: Vec3,
    points: Vec<Light>,
}

struct Light {
    position: Vec3,
    color: Vec3,
    /// How fast the light fades with the square of distance.
    falloff: f32,
}

impl Lights {
    fn of(scene: &Scene) -> Self {
        let world = scene.world;
        let size = world.size();
        let mut points = Vec::new();
        let window_center = (window::WINDOW_LOW + window::WINDOW_HIGH) * 0.5;
        points.push(Light {
            position: Vec3::new(window_center.x, window_center.y, ROOM_MIN.z + 0.35),
            color: window::light_through(&scene.outside),
            falloff: 0.35,
        });
        if world.lamp.on {
            points.push(Light {
                position: world.lamp.bulb() - Vec3::Y * 0.1,
                color: LAMP_LIGHT * 1.1,
                falloff: 0.6,
            });
        }
        points.push(Light {
            position: world.ghost.position,
            color: scene.aura.color
                * (0.2 + 0.25 * scene.aura.brightness)
                * scene.aura.visibility.clamp(0.0, 1.0)
                * size,
            falloff: 5.0,
        });
        if let Some(body) = &scene.friend {
            points.push(Light {
                position: body.position,
                color: body.aura.color * 0.15 * body.aura.visibility * body.opacity,
                falloff: 6.0,
            });
        }
        if let Some(decoration) = world.decoration() {
            points.push(Light {
                position: Decoration::POSITION + Vec3::Y * decoration.height() * 0.65,
                color: decoration_light(decoration, world.time),
                falloff: 3.0,
            });
        }
        for mote in &world.motes {
            points.push(Light {
                position: mote.position,
                color: mote_color(world) * 0.25 * mote.glow(world.time),
                falloff: 10.0,
            });
        }
        Self {
            ambient: daylight(world.hour) * (1.0 - scene.outside.rain.clamp(0.0, 1.0) / 3.0),
            points,
        }
    }

    fn at(&self, point: Vec3, normal: Vec3) -> Vec3 {
        self.points.iter().fold(self.ambient, |total, light| {
            let offset = light.position - point;
            let distance_squared = offset.length_squared();
            let facing = normal
                .dot(offset / distance_squared.sqrt().max(1e-4))
                .max(0.0);
            total + light.color * (0.25 + 0.75 * facing) / (1.0 + light.falloff * distance_squared)
        })
    }
}

/// The light from outside at `hour`: bright by day, warm at dawn and dusk,
/// and dim blue at night.
fn daylight(hour: f32) -> Vec3 {
    let sun = smoothstep(5.5, 8.0, hour) * (1.0 - smoothstep(18.0, 20.5, hour));
    let dusk =
        (1.0 - (hour - 7.0).abs() / 1.5).max(0.0) + (1.0 - (hour - 19.2).abs() / 1.5).max(0.0);
    Vec3::new(0.05, 0.06, 0.12).lerp(Vec3::new(0.5, 0.53, 0.58), sun)
        + Vec3::new(0.22, 0.1, 0.02) * dusk.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Style;

    fn grid() -> Grid {
        Style::Ascii.grid(100, 30)
    }

    fn mirror_scene(world: &World) -> Scene<'_> {
        Scene {
            world,
            outside: window::Outside::clear(world.hour, world.time),
            view: Camera::default().view(grid()),
            face: Face::REST,
            aura: Aura {
                color: Vec3::ONE,
                visibility: 1.0,
                brightness: 0.7,
                beat: 1.0,
            },
            pose: Pose::new(world.time, 0.0, 0.0, 0.0),
            friend: None,
        }
    }

    fn resident(scene: &Scene) -> Body {
        Body {
            position: scene.world.ghost.position,
            scale: GHOST_SCALE,
            pose: scene.pose,
            face: scene.face,
            aura: scene.aura,
            opacity: 1.0,
            accessory: None,
        }
    }

    #[test]
    fn a_ray_from_the_default_camera_reaches_the_mirror_center() {
        let view = Camera::default().view(grid());
        let direction = (mirror::CENTER - view.eye).normalize();
        let hit = room_exit(view.eye, direction).unwrap();
        assert_eq!(hit.axis, 0);
        assert!(hit.high);
        assert!(hit.point.distance(mirror::CENTER) < 1e-5);
        assert!(mirror::contains(hit.point));
        for offset in [Vec3::Y * 0.61, Vec3::Z * 0.39] {
            assert!(!mirror::contains(mirror::CENTER + offset));
        }
    }

    #[test]
    fn mirrored_billboards_use_the_reflected_eye_and_basis_and_total_path() {
        let view = Camera::default().view(grid());
        let mirrored = view.mirrored();
        assert_eq!(mirrored.eye, mirror::reflect_point(view.eye));
        for (basis, reflected) in [
            (view.forward, mirrored.forward),
            (-view.right, mirrored.right),
            (view.up, mirrored.up),
        ] {
            assert_eq!(reflected, mirror::reflect_direction(basis));
            assert!((reflected.length() - 1.0).abs() < 1e-6);
        }
        // The basis keeps its handedness, so a billboard seen in the mirror
        // shows its left and right swapped, as a reflection should.
        let handedness = |right: Vec3, up: Vec3, forward: Vec3| right.cross(up).dot(forward);
        assert!(
            handedness(view.right, view.up, view.forward)
                * handedness(mirrored.right, mirrored.up, mirrored.forward)
                > 0.0
        );
        assert!(mirrored.right.dot(mirrored.up).abs() < 1e-6);
        let direction = (mirror::CENTER - view.eye).normalize();
        let reflected = mirror::reflect_direction(direction);
        let position = mirror::CENTER + reflected * 0.25;
        let scale = 0.1;
        let local = Vec2::new(0.2, -0.3);
        let surface = position + (mirrored.right * local.x + mirrored.up * local.y) * scale;
        let reflected = (surface - mirrored.eye).normalize();
        let incoming = mirror::reflect_direction(reflected);
        let hit = room_exit(view.eye, incoming).unwrap();
        assert!(mirror::contains(hit.point));
        let (from_eye, eye_local) = mirrored.billboard(reflected, position, scale).unwrap();
        let (from_glass, glass_local) = mirrored
            .billboard_from(hit.point, reflected, position, scale)
            .unwrap();
        assert!(eye_local.distance(local) < 1e-4);
        assert!(glass_local.distance(local) < 1e-4);
        assert!((hit.distance + from_glass - from_eye).abs() < 1e-5);
        assert!(mirrored.forward.dot((mirrored.eye - position).normalize()) < 0.0);
    }

    #[test]
    fn rays_away_from_the_mirror_match_the_original_trace() {
        let world = World::new(1, 12.0);
        let scene = mirror_scene(&world);
        let bodies = [Some(resident(&scene)), None];
        let furniture = [
            Furniture::Lamp(world.lamp),
            Furniture::Plant(world.plant),
            Furniture::Nook(world.nook),
        ];
        let lights = Lights::of(&scene);
        let tracer = Tracer {
            bubbles: &[],
            scene: &scene,
            lights: &lights,
            furniture: &furniture,
            bodies: [bodies, bodies],
            spread: 0.01,
        };
        // Computed with the original trace before separating the origin from the camera.
        for (target, original) in [
            (
                Vec3::new(-1.0, 1.0, -1.5),
                Vec3::new(0.030333366, 0.031090869, 0.035526168),
            ),
            (
                Vec3::new(-1.0, 0.0, 0.2),
                Vec3::new(0.55302423, 0.5093097, 0.90395874),
            ),
            (
                world.ghost.position,
                Vec3::new(0.7468277, 0.76198936, 0.7901465),
            ),
            (
                world.ball.position,
                Vec3::new(0.5958956, 0.28548053, 0.24318193),
            ),
            (
                Vec3::new(-2.0, 1.0, 0.0),
                Vec3::new(0.0240388, 0.025569687, 0.030685458),
            ),
        ] {
            let direction = (target - scene.view.eye).normalize();
            assert!(
                tracer
                    .trace(scene.view.eye, direction, scene.view, 0.0, None, false)
                    .distance(original)
                    < 1e-6
            );
        }
    }

    #[test]
    fn reflected_small_faded_and_visiting_ghosts_are_finite() {
        let world = World::new(1, 21.0);
        let scene = mirror_scene(&world);
        let lights = Lights::of(&scene);
        let incoming = (mirror::CENTER - scene.view.eye).normalize();
        let position = mirror::CENTER + mirror::reflect_direction(incoming) * 0.25;
        for scale in [0.01, 0.15, GHOST_SCALE] {
            for visibility in [0.0, 0.1, 1.0] {
                for has_friend in [false, true] {
                    let body = Body {
                        position,
                        scale,
                        aura: Aura {
                            visibility,
                            ..scene.aura
                        },
                        ..resident(&scene)
                    };
                    let friend = has_friend.then_some(Body {
                        position: position - Vec3::X * 0.08,
                        scale: scale * 0.75,
                        opacity: 0.4,
                        ..body
                    });
                    let bodies = [Some(body), friend];
                    let furniture = [
                        Furniture::Mirror,
                        Furniture::Lamp(world.lamp),
                        Furniture::MusicBox(false, 0.0),
                    ];
                    let tracer = Tracer {
                        bubbles: &[],
                        scene: &scene,
                        lights: &lights,
                        furniture: &furniture,
                        bodies: [bodies, bodies],
                        spread: 0.003,
                    };
                    for vertical in -4..=4 {
                        for across in -4..=4 {
                            let target = mirror::CENTER
                                + Vec3::new(0.0, vertical as f32 * 0.1, across as f32 * 0.07);
                            let direction = (target - scene.view.eye).normalize();
                            assert!(
                                tracer
                                    .trace(scene.view.eye, direction, scene.view, 0.0, None, false)
                                    .is_finite()
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn reflected_rays_skip_front_glass_drawings_and_the_viewers_side_is_dark() {
        use crate::world::{Doodle, Picture, Sketch};
        let mut world = World::new(1, 12.0);
        let ray = Vec3::Z;
        let origin = mirror::CENTER - Vec3::X * 0.01;
        let view = Camera::default().view(grid()).mirrored();
        let empty = |world: &World| {
            let scene = mirror_scene(world);
            let lights = Lights::of(&scene);
            let tracer = Tracer {
                bubbles: &[],
                scene: &scene,
                lights: &lights,
                furniture: &[],
                bodies: [[None; 2]; 2],
                spread: 0.01,
            };
            tracer.trace(origin, ray, view, 1.0, None, true)
        };
        world.lamp.on = false;
        world.motes.clear();
        assert_eq!(empty(&world), Vec3::ZERO);
        world.doodles.push(Doodle {
            picture: Picture::Sketch(Sketch::Heart),
            center: origin.truncate(),
            size: 0.3,
            started: 0.0,
        });
        assert_eq!(empty(&world), Vec3::ZERO);
    }

    #[test]
    fn hit_testing_uses_the_same_body_scale_as_the_billboard() {
        let view = Camera::default().view(grid());
        let mut world = World::new(1, 0.0);
        world.birthday = Some(world.date);
        let ghost = world.ghost.position;
        let center = view.project(ghost).unwrap();
        let edge = view
            .project(ghost + view.right * GHOST_SCALE * 0.52)
            .unwrap();
        assert!(view.touches_ghost(center, ghost, GHOST_SCALE * world.size()));
        assert!(!view.touches_ghost(edge, ghost, GHOST_SCALE * world.size()));
        assert!(view.touches_ghost(edge, ghost, GHOST_SCALE));
        let (_, local) = view
            .billboard(view.ray(edge), ghost, GHOST_SCALE * world.size())
            .unwrap();
        assert!((local.x - 0.52 / world.size()).abs() < 1e-4);
    }

    #[test]
    fn nothing_of_the_moth_lies_beyond_the_bound_that_skips_it() {
        for pixel in [0.001, 0.01, 0.05, 0.2] {
            for flap in [-1.0, -0.4, 0.0, 0.6, 1.0] {
                for step in 0..64 {
                    let angle = step as f32 / 64.0 * std::f32::consts::TAU;
                    let point = Vec2::from_angle(angle) * (MOTH_REACH + 1.6 * pixel);
                    assert_eq!(moth_coverage(point, pixel, flap), 0.0, "{pixel} {flap}");
                }
            }
        }
    }

    #[test]
    fn a_projected_point_lies_on_the_ray_through_its_pixel() {
        let view = Camera::default().view(grid());
        let point = Vec3::new(0.7, 1.4, -0.3);
        let screen = view.project(point).unwrap();
        let direction = view.ray(screen);
        let toward = (point - view.eye).normalize();
        assert!(direction.dot(toward) > 0.99999);
    }

    #[test]
    fn the_whole_room_fits_on_screen_from_the_default_camera() {
        for (columns, rows) in [(100, 30), (80, 24), (60, 30), (200, 50)] {
            let grid = Style::Ascii.grid(columns, rows);
            let view = Camera::default().view(grid);
            for corner in 0..8 {
                let point = Vec3::new(
                    if corner & 1 == 0 {
                        ROOM_MIN.x
                    } else {
                        ROOM_MAX.x
                    },
                    if corner & 2 == 0 {
                        ROOM_MIN.y
                    } else {
                        ROOM_MAX.y
                    },
                    if corner & 4 == 0 {
                        ROOM_MIN.z
                    } else {
                        ROOM_MAX.z
                    },
                );
                let screen = view.project(point).unwrap();
                assert!(
                    screen.x.abs() < grid.half_width(),
                    "{columns}x{rows} {point}"
                );
                assert!(screen.y.abs() < 1.0, "{columns}x{rows} {point}");
            }
        }
    }

    #[test]
    fn rays_from_the_front_leave_through_the_far_walls_not_the_glass() {
        let view = Camera::default().view(grid());
        for column in -10..=10 {
            for row in -10..=10 {
                let point = Vec2::new(column as f32 * 0.08, row as f32 * 0.06);
                if let Some(hit) = room_exit(view.eye, view.ray(point)) {
                    assert!(!(hit.axis == 2 && hit.high), "{point}");
                }
            }
        }
    }

    #[test]
    fn a_ray_through_the_window_from_the_default_camera_hits_the_back_wall() {
        let view = Camera::default().view(grid());
        let center = (window::WINDOW_LOW + window::WINDOW_HIGH) * 0.5;
        let point = Vec3::new(center.x, center.y, ROOM_MIN.z);
        let screen = view.project(point).unwrap();
        let hit = room_exit(view.eye, view.ray(screen)).unwrap();
        assert_eq!(hit.axis, 2);
        assert!(!hit.high);
        assert!(hit.point.distance(point) < 1e-4);
        assert!(
            window::window(
                Vec2::new(hit.point.x, hit.point.y),
                0.01,
                &window::Outside::clear(12.0, 0.0),
            )
            .is_some()
        );
    }

    #[test]
    fn pointing_at_the_floor_and_the_glass_finds_points_on_them() {
        let view = Camera::default().view(grid());
        let floor = view.on_plane(Vec2::new(0.0, -0.5), 0.0).unwrap();
        assert!(floor.y.abs() < 1e-4);
        let glass = view.on_glass(Vec2::new(0.1, 0.1)).unwrap();
        assert!((glass.z - ROOM_MAX.z).abs() < 1e-4);
        assert!(view.on_glass(Vec2::new(5.0, 0.0)).is_none());
    }

    #[test]
    fn the_ghost_is_touched_where_it_appears_on_screen() {
        let view = Camera::default().view(grid());
        let ghost = Vec3::new(0.4, 1.1, 0.2);
        let screen = view.project(ghost).unwrap();
        assert!(view.touches_ghost(screen, ghost, GHOST_SCALE));
        assert!(!view.touches_ghost(screen + Vec2::new(0.6, 0.0), ghost, GHOST_SCALE));
    }

    #[test]
    fn the_friend_is_touched_where_its_scaled_billboard_is_projected() {
        let grid = Grid::cells(100, 32);
        for yaw in [0.0, 1.2, 3.0] {
            let view = Camera {
                yaw,
                ..Camera::default()
            }
            .view(grid);
            let position = Vec3::new(-0.6, 1.4, -0.4);
            let point = view.project(position).unwrap();
            assert!(view.touches_ghost(point, position, FRIEND_SCALE));
            let outside = view
                .project(position + view.right * FRIEND_SCALE * 0.7)
                .unwrap();
            assert!(!view.touches_ghost(outside, position, FRIEND_SCALE));
            assert!(view.touches_ghost(outside, position, GHOST_SCALE));
        }
    }

    #[test]
    fn night_is_darker_than_noon_and_dusk_is_warmer() {
        assert!(daylight(2.0).max_element() < daylight(12.0).max_element() * 0.3);
        let dusk = daylight(19.2);
        assert!(dusk.x > dusk.z);
    }

    #[test]
    fn rays_meet_the_glass_only_within_its_pane_and_ahead_of_the_eye() {
        let front = Vec3::new(0.0, 1.0, 5.0);
        let distance = glass_distance(front, -Vec3::Z).unwrap();
        assert!((distance - (5.0 - ROOM_MAX.z)).abs() < 1e-5);
        assert_eq!(glass_distance(front, Vec3::Z), None);
        assert_eq!(glass_distance(front, Vec3::X), None);
        let wide = (Vec3::new(ROOM_MAX.x + 1.0, 1.0, ROOM_MAX.z) - front).normalize();
        assert_eq!(glass_distance(front, wide), None);
        let behind = Vec3::new(0.0, 1.0, -5.0);
        let far = glass_distance(behind, Vec3::Z).unwrap();
        assert!((far - (5.0 + ROOM_MAX.z)).abs() < 1e-5);
    }

    #[test]
    fn fading_ghosts_and_doodles_render_finite_values_from_either_side_of_the_glass() {
        use crate::world::{Doodle, Sketch};

        let grid = Grid::cells(40, 16);
        let mut world = World::new(1, 12.0);
        for sketch in Sketch::ALL {
            world.doodles = vec![Doodle {
                picture: crate::world::Picture::Sketch(sketch),
                center: Vec2::new(0.0, 1.2),
                size: 0.3,
                started: 0.0,
            }];
            for time in [0.0, 2.0, 6.0, Doodle::LIFETIME - 10.0, Doodle::LIFETIME] {
                world.time = time;
                for yaw in [0.0, PI * 0.5, PI, PI * 1.5] {
                    let scene = Scene {
                        world: &world,
                        outside: window::Outside::clear(world.hour, time),
                        view: Camera {
                            yaw,
                            ..Camera::default()
                        }
                        .view(grid),
                        face: Face::REST,
                        aura: Aura {
                            color: Vec3::ONE,
                            visibility: 0.15,
                            brightness: 0.7,
                            beat: 1.0,
                        },
                        pose: Pose::new(time, 0.0, 0.0, 0.0),
                        friend: None,
                    };
                    let frame = render(&scene, grid, 1);
                    for row in 0..frame.height() {
                        for column in 0..frame.width() {
                            let shade = frame.shade(column, row);
                            assert!(shade.brightness.is_finite());
                            assert!(shade.color.is_finite());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn occasions_with_each_decoration_and_fireworks_render_finite_frames() {
        use jiff::civil::Date;

        let grid = Grid::cells(48, 18);
        let mut world = World::new(1, 23.75);
        for (date, birthday) in [
            (Date::constant(2026, 10, 31), None),
            (Date::constant(2026, 12, 24), None),
            (Date::constant(2026, 12, 31), None),
            (Date::constant(2027, 1, 1), None),
            (Date::constant(2027, 2, 14), None),
            (Date::constant(2027, 3, 1), Some(Date::constant(2026, 3, 1))),
            (Date::constant(2032, 3, 1), Some(Date::constant(2026, 3, 1))),
        ] {
            world.date = date;
            world.birthday = birthday;
            for time in [0.5, 1.0, 4.2, 1e9, f64::MAX] {
                world.time = time;
                for yaw in [0.0, PI * 0.5, PI] {
                    let scene = Scene {
                        world: &world,
                        outside: window::Outside {
                            fireworks: world.fireworks(),
                            ..window::Outside::clear(world.hour, time)
                        },
                        view: Camera {
                            yaw,
                            ..Camera::default()
                        }
                        .view(grid),
                        face: Face::REST,
                        aura: Aura {
                            color: Vec3::ONE,
                            visibility: 1.0,
                            brightness: 0.7,
                            beat: 1.0,
                        },
                        pose: Pose::new(0.0, 0.0, 0.0, 0.0),
                        friend: None,
                    };
                    assert!(window::light_through(&scene.outside).is_finite());
                    let frame = render(&scene, grid, 1);
                    for row in 0..frame.height() {
                        for column in 0..frame.width() {
                            let shade = frame.shade(column, row);
                            assert!(shade.brightness.is_finite());
                            assert!(shade.color.is_finite());
                        }
                    }
                }
            }
        }
    }
}
