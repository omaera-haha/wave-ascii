mod bubbles;
mod dream;
pub mod ghost;
mod shapes;
pub mod sky;
pub mod window;
mod yard;

use std::f32::consts::PI;

use glam::{Vec2, Vec3};

use crate::face::Face;
use crate::frame::{Frame, Grid, Shade};
use crate::sdf::{capsule, coverage, mix, smoothstep};
use crate::world::home::{self, DOOR_HIGH, DOOR_LOW};
use crate::world::mirror;
use crate::world::{Ball, Decoration, Occasion, ROOM_MAX, ROOM_MIN, World};
use ghost::{Aura, BOUNDS, Body, Pose};
use shapes::{Furniture, Material, decoration_light, sphere_span};

/// World units per unit of the ghost's own space.
pub const GHOST_SCALE: f32 = 0.45;
pub const FRIEND_SCALE: f32 = GHOST_SCALE * 0.75;
/// The point the camera circles and looks at, unless it follows the ghost
/// out into the yard.
const TARGET: Vec3 = Vec3::new(0.0, 1.3, 0.0);
/// One over the tangent of half the vertical field of view, about 50 degrees.
const FOCAL: f32 = 2.1;
const FRAME_COLOR: Vec3 = Vec3::new(0.62, 0.46, 0.32);
/// The room's walls, floor, and ceiling are drawn dimmer than the things in
/// it, so they stand out in glyphs.
const ROOM_EXPOSURE: f32 = 0.95;
const TRIM_COLOR: Vec3 = Vec3::new(0.86, 0.84, 0.78);
const MOTE_COLOR: Vec3 = Vec3::new(1.0, 0.9, 0.58);
const LAMP_LIGHT: Vec3 = Vec3::new(1.0, 0.78, 0.5);

/// Where the viewer looks from: turned around `target` by `yaw`, raised by
/// `pitch`, and moved in by `zoom`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub target: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.15,
            zoom: 1.0,
            target: TARGET,
        }
    }
}

impl Camera {
    pub fn turn(&mut self, yaw: f32, pitch: f32) {
        self.yaw += yaw;
        self.pitch = (self.pitch + pitch).clamp(-0.15, 1.3);
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.4, 3.0);
    }

    /// Eases the target toward `focus` over about a second.
    pub fn follow(&mut self, focus: Vec3, seconds: f32) {
        let amount = 1.0 - (-seconds.max(0.0) * 1.4).exp();
        self.target = self.target.lerp(focus, amount);
    }

    /// The room's middle, where the camera looks when the ghost is in.
    pub fn home() -> Vec3 {
        TARGET
    }

    /// The view on a screen sampled by `grid`. Narrow screens move the
    /// camera back so the whole room still fits across.
    pub fn view(&self, grid: Grid) -> View {
        let fit = (1.75 / grid.half_width().max(0.1)).max(1.0);
        let distance = 5.8 * fit / self.zoom;
        let around = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        );
        let eye = self.target + around * distance;
        let forward = (self.target - eye).normalize();
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
    pub outside: sky::Outside,
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
    render_with(scene, grid, true)
}

// With `cull`, each ray from the eye tests only what its screen tile may
// show; without, everything, which looks the same but takes longer.
fn render_with(scene: &Scene, grid: Grid, cull: bool) -> Frame {
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
    furniture.push(Furniture::Clock(world.hour));
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
    if let Some(gift) = scene.outside.sill {
        furniture.push(Furniture::Keepsake(
            gift,
            home::SILL + Vec3::new(0.42, home::SILL_HALF.y, 0.0),
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
    if let Some((can, landing, pour)) = world.watering() {
        let toward = Vec3::new(landing.x - can.x, 0.0, landing.z - can.z);
        let turn = (-toward.z).atan2(toward.x);
        let rotation =
            glam::Quat::from_rotation_y(turn) * glam::Quat::from_rotation_z(-0.85 * pour);
        furniture.push(Furniture::WateringCan(can, rotation));
        if pour > 0.6 {
            let spout = can + rotation * Vec3::new(0.22, 0.08, 0.0);
            for index in 0..6 {
                let fall = ((world.time * 1.6 + index as f64 / 6.0).fract()) as f32;
                let drop = spout.lerp(landing, fall) - Vec3::Y * 0.12 * fall * (1.0 - fall);
                furniture.push(Furniture::Droplet(drop));
            }
        }
    }
    if let Some(position) = world.bubble_wand() {
        furniture.push(Furniture::BubbleWand(position));
    }
    // Whatever is out in the yard is lit by the sky and traced with it.
    let (indoor, outdoor): (Vec<_>, Vec<_>) = furniture
        .into_iter()
        .partition(|piece| home::in_house(piece.bounds().0));
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
    let mut glows = Vec::new();
    for body in bodies.iter().flatten() {
        if !home::in_house(body.position) {
            glows.push((
                body.position,
                body.aura.color
                    * (0.2 + 0.25 * body.aura.brightness)
                    * body.aura.visibility.clamp(0.0, 1.0)
                    * body.opacity,
                4.0,
            ));
        }
    }
    let outside_ghost = (!home::in_house(world.ghost.position)).then_some(world.ghost.position);
    let yard = yard::Yard::new(yard::Setting {
        outside: &scene.outside,
        plant: &world.plant,
        furniture: &outdoor,
        view: scene.view,
        glows,
        ghost: outside_ghost,
        lantern: sky::Sky::at_hour(world.hour).day < 0.5,
        swing: world.swing_angle(),
        flag: world.unread_letter().is_some(),
        gust: world.wind.gust(world.time),
        // Puddles linger while the rainbow shines after the rain.
        wet: scene
            .outside
            .rain
            .max(scene.outside.rainbow * 0.7)
            .clamp(0.0, 1.0),
        door: world.door_open(),
        smoke: match world.season() {
            crate::world::Season::Winter => true,
            crate::world::Season::Autumn => !(8.0..17.0).contains(&world.hour),
            _ => false,
        },
        spill: if world.lamp.on {
            LAMP_LIGHT * 0.32 * (1.0 - sky::Sky::at_hour(world.hour).day)
        } else {
            Vec3::ZERO
        },
        occasion: world.occasion(),
        // Looking down from above the ridge while the ghost is in, the roof
        // comes off like a dollhouse's.
        roofless: home::in_house(world.ghost.position) && scene.view.eye.y > home::RIDGE,
        snow_ghost: world.snow_ghost_shape(),
        shaken: world.tree_shake(),
    });
    let tiles = Tiles::new(grid, scene.view, &indoor, &yard, &outdoor);
    let tracer = Tracer {
        scene,
        lights: &lights,
        furniture: &indoor,
        tiles: cull.then_some(&tiles),
        yard: &yard,
        door: world.door_open(),
        solid: !home::in_house(world.ghost.position),
        bodies: [bodies, reflected_bodies],
        spread,
        bubbles: &shells,
    };
    Frame::sample(grid, |point| {
        let direction = scene.view.ray(point);
        let dream = dream.as_ref().filter(|bubble| bubble.contains(point));
        let tile = cull.then(|| tiles.at(point));
        let color = tracer.trace_from(
            scene.view.eye,
            direction,
            scene.view,
            0.0,
            dream,
            false,
            tile,
        );
        // A gentle lift of the mid-tones, which glyphs otherwise lose: about
        // the 0.85th power, from square roots, which are quicker.
        let color = color.clamp(Vec3::ZERO, Vec3::ONE);
        let roots = Vec3::new(color.x.sqrt(), color.y.sqrt(), color.z.sqrt());
        Shade::from_rgb(color * 0.7 + roots * 0.3)
    })
}

/// What each patch of the screen may show: for each tile, the indoor
/// furniture, yard pieces, and outdoor things whose bounds cover it, so a
/// ray from the eye tests only those.
struct Tiles {
    size: f32,
    left: f32,
    top: f32,
    columns: usize,
    rows: usize,
    furniture: Vec<Vec<u16>>,
    pieces: Vec<Vec<u16>>,
    outdoor: Vec<Vec<u16>>,
}

impl Tiles {
    fn new(
        grid: Grid,
        view: View,
        indoor: &[Furniture],
        yard: &yard::Yard,
        outdoor: &[Furniture],
    ) -> Self {
        let size = 0.1;
        let half_width = grid.half_width();
        let columns = ((2.0 * half_width / size).ceil() as usize).max(1);
        let rows = ((2.0 / size).ceil() as usize).max(1);
        let mut tiles = Self {
            size,
            left: -half_width,
            top: 1.0,
            columns,
            rows,
            furniture: vec![Vec::new(); columns * rows],
            pieces: vec![Vec::new(); columns * rows],
            outdoor: vec![Vec::new(); columns * rows],
        };
        let margin = grid.scale() * 2.0;
        let spheres = |list: &[Furniture]| -> Vec<(Vec3, Vec3)> {
            list.iter()
                .map(|piece| {
                    let (center, radius) = piece.bounds();
                    (center, Vec3::splat(radius))
                })
                .collect()
        };
        let boxes: [Vec<(Vec3, Vec3)>; 3] = [
            spheres(indoor),
            yard.piece_boxes().collect(),
            spheres(outdoor),
        ];
        for (kind, boxes) in boxes.iter().enumerate() {
            for (index, &(center, half)) in boxes.iter().enumerate() {
                let (low, high) = match screen_box(view, center, half) {
                    Some((low, high)) => (low - Vec2::splat(margin), high + Vec2::splat(margin)),
                    None => (Vec2::splat(-1e9), Vec2::splat(1e9)),
                };
                let first = ((low.x - tiles.left) / size).floor().max(0.0) as usize;
                let last =
                    (((high.x - tiles.left) / size).floor().max(0.0) as usize).min(columns - 1);
                let top = ((tiles.top - high.y) / size).floor().max(0.0) as usize;
                let bottom = (((tiles.top - low.y) / size).floor().max(0.0) as usize).min(rows - 1);
                if first > last || top > bottom || low.x > half_width || high.y < -1.0 {
                    continue;
                }
                for row in top..=bottom {
                    for column in first..=last {
                        let tile = row * columns + column;
                        let list = match kind {
                            0 => &mut tiles.furniture[tile],
                            1 => &mut tiles.pieces[tile],
                            _ => &mut tiles.outdoor[tile],
                        };
                        list.push(index as u16);
                    }
                }
            }
        }
        tiles
    }

    /// The tile under a screen point.
    fn at(&self, point: Vec2) -> usize {
        let column = (((point.x - self.left) / self.size).max(0.0) as usize).min(self.columns - 1);
        let row = (((self.top - point.y) / self.size).max(0.0) as usize).min(self.rows - 1);
        row * self.columns + column
    }
}

/// Where a box appears on the screen, as the corners of a rectangle
/// around it, or `None` when part of it is behind the eye.
fn screen_box(view: View, center: Vec3, half: Vec3) -> Option<(Vec2, Vec2)> {
    let mut low = Vec2::splat(f32::INFINITY);
    let mut high = Vec2::splat(f32::NEG_INFINITY);
    for corner in 0..8 {
        let sign = Vec3::new(
            if corner & 1 == 0 { -1.0 } else { 1.0 },
            if corner & 2 == 0 { -1.0 } else { 1.0 },
            if corner & 4 == 0 { -1.0 } else { 1.0 },
        );
        let offset = center + half * sign - view.eye;
        let depth = offset.dot(view.forward);
        if depth < 0.05 {
            return None;
        }
        let point = Vec2::new(offset.dot(view.right), offset.dot(view.up)) * FOCAL / depth;
        low = low.min(point);
        high = high.max(point);
    }
    Some((low, high))
}

struct Tracer<'a> {
    scene: &'a Scene<'a>,
    lights: &'a Lights,
    furniture: &'a [Furniture<'a>],
    /// Screen tiles for rays from the eye; mirrored rays test everything.
    tiles: Option<&'a Tiles>,
    yard: &'a yard::Yard<'a>,
    /// How far the door stands open, 0 to 1.
    door: f32,
    /// While the ghost is out, the walls stand whole and only the glass
    /// front, the window, and the open door show inside; while it is in,
    /// the near walls are cut away to watch it.
    solid: bool,
    bodies: [[Option<Body>; 2]; 2],
    spread: f32,
    bubbles: &'a [bubbles::Shell],
}

impl Tracer<'_> {
    #[cfg(test)]
    fn trace(
        &self,
        origin: Vec3,
        direction: Vec3,
        view: View,
        traveled: f32,
        dream: Option<&dream::Bubble>,
        reflected: bool,
    ) -> Vec3 {
        self.trace_from(origin, direction, view, traveled, dream, reflected, None)
    }

    /// Traces a ray; `tile`, for rays from the eye, limits what it tests to
    /// what that patch of the screen may show.
    #[allow(clippy::too_many_arguments)]
    fn trace_from(
        &self,
        origin: Vec3,
        direction: Vec3,
        view: View,
        traveled: f32,
        dream: Option<&dream::Bubble>,
        reflected: bool,
        tile: Option<usize>,
    ) -> Vec3 {
        let tile = tile.zip(self.tiles);
        let cull = tile.map(|(tile, tiles)| yard::Cull {
            pieces: &tiles.pieces[tile],
            furniture: &tiles.outdoor[tile],
        });
        let scene = self.scene;
        let lights = self.lights;
        let furniture = self.furniture;
        let bodies = &self.bodies[usize::from(reflected)];
        let spread = self.spread;
        let world = scene.world;
        let eye = origin;
        let room = room_exit(eye, direction);
        let mut limit = f32::INFINITY;
        // Outdoors before the room, or all the way when the ray misses it.
        let before = room.map_or(f32::INFINITY, |hit| hit.enter);
        let front = self.yard.trace_among(
            eye,
            direction,
            0.0,
            before,
            room.is_some() && !self.solid,
            traveled,
            spread,
            cull,
        );
        let mut color = match (front, room) {
            (Some((distance, shade)), _) => {
                limit = distance;
                shade
            }
            (None, Some(hit)) => {
                let footprint = (traveled + hit.distance) * spread;
                match opening(&hit, self.door, scene.outside.window_lift, footprint) {
                    Some((frame, sheen, trim)) => {
                        let (far, outdoors) = self
                            .yard
                            .trace_among(
                                eye,
                                direction,
                                hit.distance + 1e-4,
                                f32::INFINITY,
                                false,
                                traveled,
                                spread,
                                cull,
                            )
                            .unwrap_or_else(|| {
                                (f32::INFINITY, self.yard.background(direction, spread))
                            });
                        let mut normal = Vec3::ZERO;
                        normal[hit.axis] = if hit.high { -1.0 } else { 1.0 };
                        let wood = if trim { TRIM_COLOR } else { FRAME_COLOR };
                        let glass = outdoors.lerp(Vec3::new(0.76, 0.87, 1.0), sheen);
                        limit = if frame > 0.5 { hit.distance } else { far };
                        glass * (1.0 - frame) + wood * lights.at(hit.point, normal) * frame
                    }
                    None => {
                        limit = hit.distance;
                        room_color(&hit, lights, footprint)
                    }
                }
            }
            (None, None) => self.yard.background(direction, spread),
        };

        if let Some((enter, _)) = sphere_span(eye, direction, world.ball.position, Ball::RADIUS)
            && enter > 0.0
            && enter < limit
        {
            limit = enter;
            let point = eye + direction * enter;
            let normal = (point - world.ball.position) / Ball::RADIUS;
            color = Material::Ball.albedo(&world.plant) * lights.at(point, normal);
        }
        let pieces: &mut dyn Iterator<Item = &Furniture> = match tile {
            Some((tile, tiles)) => &mut tiles.furniture[tile]
                .iter()
                .map(|&index| &furniture[usize::from(index)]),
            None => &mut furniture.iter(),
        };
        for piece in pieces {
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
            let reflected_color = self.trace_from(
                hit.point - Vec3::X * 1e-4,
                reflected_direction,
                view.mirrored(),
                traveled + hit.distance,
                None,
                true,
                None,
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
        // Rain and snow fall in front of everything but what is nearer.
        let mut nearest = limit;
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
            let Some((distance, local)) =
                view.billboard_from(eye, direction, body.position, body.scale)
            else {
                continue;
            };
            if distance >= limit || local.cmplt(BOUNDS[0]).any() || local.cmpgt(BOUNDS[1]).any() {
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
            let glow = ghost::shade(local, pixel, world.time, body);
            overlay(distance, glow);
            nearest = nearest.min(distance);
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
        color += self.yard.sparkles(eye, direction, limit, traveled, spread);
        color = self
            .yard
            .smoke(eye, direction, limit, traveled, spread, color);
        let shelter = match (front, room) {
            (None, Some(hit)) => hit.enter,
            _ => 0.0,
        };
        self.yard
            .weather(eye, direction, shelter, nearest, traveled, spread, color)
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
    let spread = miss / (width * width);
    // Past a dozen widths' worth it rounds to nothing.
    if spread > 12.0 { 0.0 } else { (-spread).exp() }
}

/// Where a ray leaves the room: rays pass through the near walls as through
/// glass and show the inside of the far ones.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RoomHit {
    /// Where the ray comes into the room, 0 when it starts inside.
    enter: f32,
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
        enter,
        distance,
        point: origin + direction * distance,
        axis,
        high: direction[axis] > 0.0,
    })
}

/// Whether the ray leaves the room through a gap to the outdoors: the
/// window, the open door, or the glass front. Gives how much frame covers
/// the point, the glass's sheen, and whether the frame is painted trim.
fn opening(hit: &RoomHit, door: f32, lift: f32, footprint: f32) -> Option<(f32, f32, bool)> {
    match (hit.axis, hit.high) {
        (2, false) => window::pane(hit.point.truncate(), footprint, lift)
            .map(|pane| (pane.frame, pane.sheen, false)),
        (2, true) => Some((0.0, 0.0, false)),
        (0, true) if door > 0.02 => {
            let at = Vec2::new(hit.point.z, hit.point.y);
            if at.cmpge(DOOR_LOW).all() && at.cmple(DOOR_HIGH).all() {
                let edge = (at - DOOR_LOW).min(DOOR_HIGH - at);
                let edge = edge.x.min(if at.y > 0.05 { edge.y } else { 1.0 });
                Some((coverage(edge - 0.05, footprint * 0.5 + 1e-4), 0.0, true))
            } else {
                None
            }
        }
        _ => None,
    }
}

// The inside of a wall: floorboards, wallpaper over a painted wainscot,
// a beamed ceiling, and the door in the right wall.
fn room_color(hit: &RoomHit, lights: &Lights, footprint: f32) -> Vec3 {
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
    let footprint = footprint.max(1e-4);
    let albedo = match (hit.axis, hit.high) {
        (1, false) => {
            let shadow = lights.ghost.map_or(1.0, |ghost| {
                let across = Vec2::new(hit.point.x - ghost.x, hit.point.z - ghost.z).length();
                1.0 - 0.35 * (-(across * across) / (0.06 * ghost.y.max(0.3))).exp()
            });
            floorboards(hit.point, footprint) * shadow
        }
        (1, true) => ceiling(hit.point, footprint),
        (0, true) if door_panel(hit.point, footprint).is_some() => {
            door_panel(hit.point, footprint).unwrap()
        }
        _ => wall(hit.point, hit.axis, footprint),
    };
    // Corners gather a little shadow.
    let border = (position - low).min(high - position).min_element();
    albedo * light * (0.72 + 0.28 * smoothstep(0.0, 0.3, border)) * ROOM_EXPOSURE
}

/// A round woven rug in the middle of the floor: its middle and radii.
const RUG: Vec2 = Vec2::new(-0.15, 0.3);
const RUG_RADII: Vec2 = Vec2::new(1.05, 0.62);

fn floorboards(point: Vec3, footprint: f32) -> Vec3 {
    let boards = planks(point, footprint);
    let at = (Vec2::new(point.x, point.z) - RUG) / RUG_RADII;
    let reach = at.length();
    if reach > 1.05 {
        return boards;
    }
    // Rings of color, fading to the average when they're too fine.
    let ring = (reach * 7.0).floor();
    let fade = 1.0 - smoothstep(0.03, 0.09, footprint);
    let colors = [
        Vec3::new(0.78, 0.5, 0.42),
        Vec3::new(0.86, 0.76, 0.58),
        Vec3::new(0.56, 0.62, 0.7),
    ];
    let average = (colors[0] + colors[1] + colors[2]) / 3.0;
    let woven = colors[ring as usize % 3].lerp(average, 1.0 - fade);
    let edge = coverage((reach - 1.0) * RUG_RADII.min_element(), footprint * 0.5);
    boards.lerp(woven * 0.85, edge)
}

fn planks(point: Vec3, footprint: f32) -> Vec3 {
    const WIDTH: f32 = 0.17;
    const LENGTH: f32 = 1.3;
    let row = (point.z / WIDTH).floor();
    let shifted = point.x + sky::scramble(Vec2::new(row, 3.0)) * LENGTH;
    let plank = (shifted / LENGTH).floor();
    let seed = sky::scramble(Vec2::new(row, plank));
    let fade = 1.0 - smoothstep(0.015, 0.06, footprint);
    let grain = crate::render::sky::noise(Vec2::new(shifted * 6.0, row * 7.3 + point.z * 30.0));
    let base = Vec3::new(0.64, 0.46, 0.31) * (0.84 + 0.22 * seed + 0.12 * (grain - 0.5) * fade);
    let across = (0.5 - ((point.z / WIDTH).fract() - 0.5).abs()) * WIDTH;
    let along = (0.5 - ((shifted / LENGTH).fract() - 0.5).abs()) * LENGTH;
    let seam = coverage(across.min(along) - 0.004, footprint * 0.5);
    base * (1.0 - 0.5 * seam * mix(0.4, 1.0, fade))
}

fn ceiling(point: Vec3, footprint: f32) -> Vec3 {
    let beam = (0.5 - ((point.x / 0.8 + 0.5).fract() - 0.5).abs()) * 0.8;
    let beam = coverage(0.05 - beam, footprint * 0.5);
    Vec3::new(0.72, 0.7, 0.66).lerp(Vec3::new(0.5, 0.37, 0.27), beam)
}

fn wall(point: Vec3, axis: usize, footprint: f32) -> Vec3 {
    let along = if axis == 0 { point.z } else { point.x };
    let height = point.y;
    let edge = footprint * 0.5;
    let fade = 1.0 - smoothstep(0.015, 0.07, footprint);
    // Wallpaper with soft stripes and little dots between them.
    let stripe = (0.5 - ((along / 0.3).fract() - 0.5).abs()) * 0.3;
    let stripe = coverage(stripe - 0.012, edge);
    let dot = Vec2::new(
        ((along / 0.3 + 0.5).fract() - 0.5) * 0.3,
        ((height / 0.3).fract() - 0.5) * 0.3,
    );
    let dot = coverage(dot.length() - 0.014, edge);
    let paper = Vec3::new(0.74, 0.69, 0.62) * (1.0 - (0.07 * stripe + 0.06 * dot) * fade);
    // Painted wainscot with grooves, a rail on top, a skirting board below.
    let groove = (0.5 - ((along / 0.18).fract() - 0.5).abs()) * 0.18;
    let groove = coverage(groove - 0.004, edge) * fade;
    let panel = Vec3::new(0.52, 0.64, 0.57) * (1.0 - 0.2 * groove);
    let rail = Vec3::new(0.8, 0.77, 0.7);
    let skirting = Vec3::new(0.42, 0.31, 0.23);
    let mut color = panel;
    color = color.lerp(rail, coverage(0.8 - height, edge));
    color = color.lerp(paper, coverage(0.84 - height, edge));
    color.lerp(skirting, coverage(height - 0.1, edge))
}

/// The closed door on the inside of the right wall, if the point is on it.
fn door_panel(point: Vec3, footprint: f32) -> Option<Vec3> {
    let at = Vec2::new(point.z, point.y);
    let low = DOOR_LOW - Vec2::new(0.06, 0.0);
    let high = DOOR_HIGH + Vec2::splat(0.06);
    if at.cmplt(low).any() || at.cmpgt(high).any() {
        return None;
    }
    let edge = footprint * 0.5;
    let inside = (at - DOOR_LOW).min(DOOR_HIGH - at).min_element();
    let trim = coverage(inside, edge);
    let leaf = Vec3::new(0.56, 0.38, 0.27);
    let middle = (DOOR_LOW + DOOR_HIGH) * 0.5;
    let mut inset = f32::INFINITY;
    for center in [
        Vec2::new(middle.x, DOOR_LOW.y + 0.5),
        Vec2::new(middle.x, DOOR_LOW.y + 1.28),
    ] {
        let half = Vec2::new(0.22, if center.y < 1.0 { 0.32 } else { 0.36 });
        let offset = (at - center).abs() - half;
        inset = inset.min(offset.max_element());
    }
    let groove = coverage(inset.abs() - 0.008, edge);
    let knob = coverage(at.distance(Vec2::new(DOOR_LOW.x + 0.09, 0.95)) - 0.03, edge);
    let color = (leaf * (1.0 - 0.3 * groove)).lerp(Vec3::new(0.85, 0.7, 0.4), knob);
    Some(color.lerp(TRIM_COLOR * 0.85, trim))
}

/// The light in the room: daylight and the window, the sun's patch through
/// the panes, the lamp, and the glow of the ghost and the motes.
struct Lights {
    ambient: Vec3,
    points: Vec<Light>,
    sun: Vec3,
    sunlight: Vec3,
    lift: f32,
    /// The ghost floating in the room, for its soft shadow on the floor.
    ghost: Option<Vec3>,
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
            color: sky::light_through(&scene.outside),
            falloff: 0.35,
        });
        if world.lamp.on {
            points.push(Light {
                position: world.lamp.bulb() - Vec3::Y * 0.1,
                color: LAMP_LIGHT * 1.1,
                falloff: 0.6,
            });
        }
        if home::in_house(world.ghost.position) {
            points.push(Light {
                position: world.ghost.position,
                color: scene.aura.color
                    * (0.2 + 0.25 * scene.aura.brightness)
                    * scene.aura.visibility.clamp(0.0, 1.0)
                    * size,
                falloff: 5.0,
            });
        }
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
            sun: scene.outside.sunlight_direction(),
            sunlight: scene.outside.sunlight(),
            lift: scene.outside.window_lift,
            ghost: home::in_house(world.ghost.position)
                .then_some(world.ghost.position)
                .filter(|_| world.ghost.fade < 0.9),
        }
    }

    fn at(&self, point: Vec3, normal: Vec3) -> Vec3 {
        let mut total = self.points.iter().fold(self.ambient, |total, light| {
            let offset = light.position - point;
            let distance_squared = offset.length_squared();
            let facing = normal
                .dot(offset / distance_squared.sqrt().max(1e-4))
                .max(0.0);
            total + light.color * (0.25 + 0.75 * facing) / (1.0 + light.falloff * distance_squared)
        });
        // Sunshine through the window panes, mullions and all.
        let facing = normal.dot(self.sun);
        if facing > 0.0 && self.sun.z < -1e-3 && self.sunlight.max_element() > 1e-3 {
            let along = (ROOM_MIN.z - point.z) / self.sun.z;
            let through = point + self.sun * along;
            if along > 0.0
                && let Some(pane) = window::pane(through.truncate(), 0.02, self.lift)
            {
                total += self.sunlight * facing * (1.0 - pane.frame) * 0.9;
            }
        }
        total
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
            outside: sky::Outside::clear(world.hour, world.time),
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

    fn yard<'a>(scene: &'a Scene) -> yard::Yard<'a> {
        yard::Yard::new(yard::Setting {
            outside: &scene.outside,
            plant: &scene.world.plant,
            furniture: &[],
            view: scene.view,
            glows: Vec::new(),
            ghost: None,
            lantern: false,
            swing: 0.0,
            flag: false,
            gust: 0.0,
            wet: 0.0,
            door: 0.0,
            smoke: false,
            spill: Vec3::ZERO,
            occasion: None,
            roofless: false,
            snow_ghost: None,
            shaken: None,
        })
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
    fn screen_tiles_skip_only_what_could_not_be_seen() {
        let mut world = World::new(3, 15.0);
        world.date = jiff::civil::Date::constant(2026, 10, 30);
        world.visitors.next_cat = 0.0;
        world.step(30.0);
        for (yaw, target) in [
            (0.0, Camera::home()),
            (0.6, Vec3::new(4.3, 1.2, -0.7)),
            (2.6, Camera::home()),
        ] {
            let grid = Grid::new(160, 96, 1.0);
            let scene = Scene {
                view: Camera {
                    yaw,
                    target,
                    zoom: 0.7,
                    ..Camera::default()
                }
                .view(grid),
                ..mirror_scene(&world)
            };
            let culled = render_with(&scene, grid, true);
            let full = render_with(&scene, grid, false);
            for row in 0..grid.height {
                for column in 0..grid.width {
                    let difference =
                        culled.shade(column, row).rgb() - full.shade(column, row).rgb();
                    assert!(
                        difference.abs().max_element() < 1e-4,
                        "{yaw} {column} {row}"
                    );
                }
            }
        }
    }

    #[test]
    fn rays_through_the_room_and_out_to_the_yard_give_finite_colors() {
        let world = World::new(1, 12.0);
        let scene = mirror_scene(&world);
        let bodies = [Some(resident(&scene)), None];
        let furniture = [
            Furniture::Lamp(world.lamp),
            Furniture::Plant(world.plant),
            Furniture::Nook(world.nook),
        ];
        let lights = Lights::of(&scene);
        let yard = yard(&scene);
        let tracer = Tracer {
            bubbles: &[],
            scene: &scene,
            lights: &lights,
            furniture: &furniture,
            tiles: None,
            yard: &yard,
            door: 1.0,
            solid: false,
            bodies: [bodies, bodies],
            spread: 0.01,
        };
        for target in [
            Vec3::new(-1.0, 1.0, -1.5),
            Vec3::new(0.0, 1.5, -1.4),
            Vec3::new(2.0, 0.9, 0.8),
            world.ghost.position,
            world.ball.position,
            Vec3::new(6.0, 0.5, 0.0),
            Vec3::new(0.0, 8.0, -20.0),
        ] {
            let direction = (target - scene.view.eye).normalize();
            let color = tracer.trace(scene.view.eye, direction, scene.view, 0.0, None, false);
            assert!(color.is_finite(), "{target}");
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
                    let yard = yard(&scene);
                    let tracer = Tracer {
                        bubbles: &[],
                        scene: &scene,
                        lights: &lights,
                        furniture: &furniture,
                        tiles: None,
                        yard: &yard,
                        door: 0.0,
                        solid: false,
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
    fn reflected_rays_skip_front_glass_drawings() {
        use crate::world::{Doodle, Picture, Sketch};
        let mut world = World::new(1, 12.0);
        let ray = Vec3::Z;
        let origin = mirror::CENTER - Vec3::X * 0.01;
        let view = Camera::default().view(grid()).mirrored();
        let empty = |world: &World| {
            let scene = mirror_scene(world);
            let lights = Lights::of(&scene);
            let yard = yard(&scene);
            let tracer = Tracer {
                bubbles: &[],
                scene: &scene,
                lights: &lights,
                furniture: &[],
                tiles: None,
                yard: &yard,
                door: 0.0,
                solid: false,
                bodies: [[None; 2]; 2],
                spread: 0.01,
            };
            tracer.trace(origin, ray, view, 1.0, None, true)
        };
        world.lamp.on = false;
        world.motes.clear();
        let before = empty(&world);
        assert!(before.is_finite());
        world.doodles.push(Doodle {
            picture: Picture::Sketch(Sketch::Heart),
            center: origin.truncate(),
            size: 0.3,
            started: 0.0,
        });
        assert_eq!(empty(&world), before);
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
        assert!(window::pane(Vec2::new(hit.point.x, hit.point.y), 0.01, 0.0).is_some());
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
                        outside: sky::Outside::clear(world.hour, time),
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
                        outside: sky::Outside {
                            fireworks: world.fireworks(),
                            ..sky::Outside::clear(world.hour, time)
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
                    assert!(sky::light_through(&scene.outside).is_finite());
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

#[cfg(test)]
mod bench {
    use super::*;

    #[test]
    #[ignore]
    fn bench_frames() {
        let hour: f32 = std::env::var("BENCH_HOUR")
            .ok()
            .and_then(|hour| hour.parse().ok())
            .unwrap_or(13.0);
        let mut world = World::new(1, hour);
        world.date = jiff::civil::Date::constant(2026, 10, 22);
        world.step(0.0);
        let grid = Grid::new(400, 240, 1.0);
        for (name, camera) in [
            ("room", Camera::default()),
            (
                "yard",
                Camera {
                    target: Vec3::new(4.3, 1.2, -0.7),
                    ..Camera::default()
                },
            ),
        ] {
            let scene = Scene {
                world: &world,
                outside: sky::Outside {
                    season: world.season(),
                    ..sky::Outside::clear(hour, 10.0)
                },
                view: camera.view(grid),
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
            let start = std::time::Instant::now();
            for _ in 0..10 {
                std::hint::black_box(render(&scene, grid, 1));
            }
            println!("{name}: {:?} per frame", start.elapsed() / 10);
        }
    }
}
