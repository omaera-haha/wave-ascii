use std::f32::consts::PI;

use glam::{Quat, Vec3};

use super::things::{Nook, Plant, ROOM_MAX, WINDOW_CENTER};
use super::{Ball, Decoration, EventKind, SHELF_POSITION, World};

/// Half the ghost's field of view: it sees what lies within this angle of
/// where it faces.
const FIELD_OF_VIEW: f32 = 1.05;
/// Within this angle of where it faces, a thing is what it looks at.
const FOCUS: f32 = 0.3;
/// Radians a second the ghost turns its gaze.
const TURN_RATE: f32 = 2.5;

/// Something in the room the ghost can see and turn to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Thing {
    /// The person behind the glass, or the glass itself when nobody is there.
    Viewer,
    Window,
    Lamp,
    Plant,
    Ball,
    Cushion,
    Blocks,
    Decoration,
    Mote(u32),
    Bird,
    Moth,
    Shelf,
    Friend,
    Drawing,
    Seedlings,
    Books,
    MusicBox,
}

impl Thing {
    /// The names minds use for things, as in `World::attend`.
    pub const NAMES: [&str; 17] = [
        "viewer",
        "window",
        "plant",
        "ball",
        "lamp",
        "cushion",
        "light mote",
        "decoration",
        "bird",
        "moth",
        "shelf",
        "blocks",
        "friend",
        "drawing",
        "pots",
        "books",
        "music box",
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Window => "window",
            Self::Lamp => "lamp",
            Self::Plant => "plant",
            Self::Ball => "ball",
            Self::Cushion => "cushion",
            Self::Blocks => "blocks",
            Self::Decoration => "decoration",
            Self::Mote(_) => "light mote",
            Self::Bird => "bird",
            Self::Moth => "moth",
            Self::Shelf => "shelf",
            Self::Friend => "friend",
            Self::Drawing => "drawing",
            Self::Seedlings => "pots",
            Self::Books => "books",
            Self::MusicBox => "music box",
        }
    }
}

/// One thing in the ghost's view: how far it is, how far off to the right
/// (positive) or left of where it faces and how far up, in radians, and
/// what stands in the way, if anything.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub thing: Thing,
    pub distance: f32,
    pub across: f32,
    pub up: f32,
    /// The angle between where the ghost faces and the thing, in radians.
    pub off_center: f32,
    pub hidden_behind: Option<Thing>,
}

/// What the ghost can see from where it faces right now.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sight {
    /// The visible thing nearest the middle of its view, if close to it.
    pub looking_at: Option<Thing>,
    /// Things in its field of view, nearest the middle first.
    pub in_view: Vec<Seen>,
    /// The fixed things of the room it can't see now.
    pub out_of_view: Vec<Thing>,
}

impl World {
    /// Where a thing is, if it is still in the room.
    pub fn position_of(&self, thing: Thing) -> Option<Vec3> {
        Some(match thing {
            Thing::MusicBox => super::music::POSITION + Vec3::Y * 0.2,
            Thing::Viewer => self
                .viewer
                .pointer
                .unwrap_or(Vec3::new(0.0, 1.2, ROOM_MAX.z)),
            Thing::Window => WINDOW_CENTER,
            Thing::Lamp => self.lamp.bulb(),
            Thing::Plant => self.plant.position + Vec3::Y * self.plant.height() * 0.6,
            Thing::Ball => self.ball.position,
            Thing::Blocks => match self.tower {
                Some(spot) => {
                    spot + Vec3::Y * (self.tower_height() as f32 * super::blocks::EDGE * 0.5)
                }
                None => {
                    self.blocks
                        .iter()
                        .filter(|block| !block.held && !block.stacked)
                        .min_by(|first, second| {
                            first
                                .position
                                .distance_squared(self.ghost.position)
                                .total_cmp(&second.position.distance_squared(self.ghost.position))
                        })?
                        .position
                }
            },
            Thing::Books => super::books::POSITION + Vec3::Y * 0.1,
            Thing::Cushion => self.nook.position + Vec3::Y * 0.1,
            Thing::Decoration => {
                let decoration = self.decoration()?;
                Decoration::POSITION + Vec3::Y * decoration.height() * 0.5
            }
            Thing::Mote(id) => self.motes.iter().find(|mote| mote.id == id)?.position,
            Thing::Bird => self.visitors.bird.as_ref()?.position(self.time),
            Thing::Moth => self.visitors.moth.as_ref()?.position,
            Thing::Shelf => SHELF_POSITION,
            Thing::Friend => self.friend.as_ref()?.position,
            Thing::Drawing => self.drawing.as_ref()?.center().extend(ROOM_MAX.z),
            Thing::Seedlings => {
                let (index, plant) =
                    self.seedlings
                        .iter()
                        .enumerate()
                        .min_by(|(first, _), (second, _)| {
                            super::garden::SPOTS[*first]
                                .distance_squared(self.ghost.position)
                                .total_cmp(
                                    &super::garden::SPOTS[*second]
                                        .distance_squared(self.ghost.position),
                                )
                        })?;
                super::garden::SPOTS[index] + Vec3::Y * plant.height() * 0.5
            }
        })
    }

    /// What the ghost sees from where it faces.
    pub fn sight(&self) -> Sight {
        let ghost = &self.ghost;
        let eye = ghost.position + Vec3::Y * 0.1;
        let heading = ghost.heading;
        let right = heading.cross(Vec3::Y).normalize_or(Vec3::X);
        let fixed = [
            Thing::Viewer,
            Thing::Window,
            Thing::Plant,
            Thing::Ball,
            Thing::Lamp,
            Thing::Cushion,
            Thing::Blocks,
            Thing::Decoration,
            Thing::Bird,
            Thing::Moth,
            Thing::Shelf,
            Thing::Friend,
            Thing::Drawing,
            Thing::Seedlings,
            Thing::Books,
            Thing::MusicBox,
        ];
        let motes = self.motes.iter().map(|mote| Thing::Mote(mote.id));
        let mut in_view = Vec::new();
        let mut out_of_view = Vec::new();
        for thing in fixed.into_iter().chain(motes) {
            let Some(position) = self.position_of(thing) else {
                continue;
            };
            let offset = position - eye;
            let distance = offset.length();
            let direction = offset / distance.max(1e-4);
            if direction.angle_between(heading) > FIELD_OF_VIEW {
                if !matches!(thing, Thing::Mote(_)) {
                    out_of_view.push(thing);
                }
                continue;
            }
            let off_center = direction.angle_between(heading);
            let flat = (direction - Vec3::Y * direction.y).normalize_or(heading);
            in_view.push(Seen {
                off_center,
                thing,
                distance,
                across: flat.dot(right).clamp(-1.0, 1.0).asin(),
                up: direction.y.clamp(-1.0, 1.0).asin() - heading.y.clamp(-1.0, 1.0).asin(),
                hidden_behind: self.blocker(eye, position, thing),
            });
        }
        // Side and height angles distort when the ghost looks steeply up or
        // down, so the true angle from its heading decides what is central.
        in_view.sort_by(|first, second| first.off_center.total_cmp(&second.off_center));
        let looking_at = in_view
            .iter()
            .find(|seen| seen.hidden_behind.is_none())
            .filter(|seen| seen.off_center < FOCUS)
            .map(|seen| seen.thing);
        Sight {
            looking_at,
            in_view,
            out_of_view,
        }
    }

    /// Has the ghost turn to `thing` for `seconds`, for example because a
    /// mind chose to look at it. Names are those of `Thing::NAMES`; a light
    /// mote is the nearest one it knows of.
    pub fn attend(&mut self, name: &str, seconds: f32) {
        let thing = match name {
            "light mote" => self.nearest_mote().map(|mote| Thing::Mote(mote.id)),
            name => [
                Thing::Viewer,
                Thing::Window,
                Thing::Plant,
                Thing::Ball,
                Thing::Lamp,
                Thing::Cushion,
                Thing::Blocks,
                Thing::Decoration,
                Thing::Bird,
                Thing::Moth,
                Thing::Shelf,
                Thing::Friend,
                Thing::Drawing,
                Thing::Seedlings,
                Thing::Books,
                Thing::MusicBox,
            ]
            .into_iter()
            .find(|thing| thing.name() == name && self.position_of(*thing).is_some()),
        };
        if let Some(thing) = thing {
            self.ghost.attention = Some((thing, self.time + f64::from(seconds)));
        }
    }

    // The furniture standing between the eye and a thing, if any, judged by
    // rough spheres around each piece.
    fn blocker(&self, eye: Vec3, target: Vec3, thing: Thing) -> Option<Thing> {
        let blockers = [
            (
                Thing::MusicBox,
                super::music::POSITION + Vec3::Y * 0.16,
                0.22,
            ),
            (Thing::Lamp, self.lamp.bulb() + Vec3::Y * 0.06, 0.2),
            (
                Thing::Plant,
                self.plant.position + Vec3::Y * self.plant.height() * 0.5,
                Plant::POT_RADIUS + 0.08,
            ),
            (
                Thing::Cushion,
                self.nook.position + Vec3::Y * 0.07,
                Nook::RADIUS * 0.8,
            ),
            (Thing::Ball, self.ball.position, Ball::RADIUS),
            (Thing::Books, super::books::POSITION + Vec3::Y * 0.09, 0.16),
            (Thing::Shelf, SHELF_POSITION - Vec3::X * 0.32, 0.12),
            (Thing::Shelf, SHELF_POSITION, 0.12),
            (Thing::Shelf, SHELF_POSITION + Vec3::X * 0.32, 0.12),
        ];
        let along = target - eye;
        let length = along.length();
        let decoration = self.decoration().map(|decoration| {
            (
                Thing::Decoration,
                Decoration::POSITION + Vec3::Y * decoration.height() * 0.5,
                decoration.radius(),
            )
        });
        let tower = self.tower.filter(|_| self.tower_height() >= 3).map(|spot| {
            (
                Thing::Blocks,
                spot,
                self.tower_height() as f32 * super::blocks::EDGE,
            )
        });
        if let Some((_, spot, height)) = tower
            && thing != Thing::Blocks
        {
            let floor_along = super::flat(along);
            let fraction = (super::flat(spot - eye).dot(floor_along)
                / floor_along.length_squared().max(1e-6))
            .clamp(0.0, 1.0);
            let closest = eye + along * fraction;
            if fraction > 0.02
                && fraction < 0.95
                && closest.y >= 0.0
                && closest.y <= height
                && super::flat(closest - spot).length() < super::blocks::EDGE
            {
                return Some(Thing::Blocks);
            }
        }
        let blocked =
            blockers
                .into_iter()
                .chain(decoration)
                .find_map(|(blocker, center, radius)| {
                    if blocker == thing {
                        return None;
                    }
                    let fraction = ((center - eye).dot(along) / (length * length)).clamp(0.0, 1.0);
                    let closest = eye + along * fraction;
                    (fraction > 0.02 && fraction < 0.95 && closest.distance(center) < radius)
                        .then_some(blocker)
                });
        if blocked.is_some() || self.seedlings.is_empty() || thing == Thing::Seedlings {
            return blocked;
        }
        self.seedlings
            .iter()
            .enumerate()
            .find_map(|(index, plant)| {
                let center = super::garden::SPOTS[index] + Vec3::Y * plant.height() * 0.5;
                let fraction = ((center - eye).dot(along) / (length * length)).clamp(0.0, 1.0);
                let closest = eye + along * fraction;
                (fraction > 0.02
                    && fraction < 0.95
                    && closest.distance(center) < super::Seedling::POT_RADIUS + 0.03)
                    .then_some(Thing::Seedlings)
            })
    }

    /// Turns the ghost's gaze toward what holds its attention: what a mind
    /// asked it to look at, else what its plan is about, else where it is
    /// going, else wherever its eyes wander. Then notes what it newly sees.
    pub(super) fn look_around(&mut self, seconds: f32) {
        let time = self.time;
        let following = (self.ghost.plan.activity == super::Activity::Window
            && self.focus.is_none())
        .then(|| self.passersby().into_iter().flatten().next())
        .flatten()
        .map(super::passerby::Passing::window_position);
        let ghost = &self.ghost;
        let attended = ghost
            .attention
            .filter(|(_, until)| *until > time)
            .and_then(|(thing, _)| {
                if thing == Thing::Window {
                    following.or_else(|| self.position_of(thing))
                } else {
                    self.position_of(thing)
                }
            });
        let wander = || {
            let phase = time as f32;
            let yaw = PI * (phase * 0.045).sin() + 0.6 * (phase * 0.21).sin();
            Vec3::new(yaw.sin(), 0.15 * (phase * 0.17).sin(), yaw.cos())
        };
        let desired = match attended.or(following).or(ghost.look) {
            Some(target) => (target - ghost.position).normalize_or(ghost.heading),
            None if ghost.velocity.length() > 0.15 => ghost.velocity.normalize(),
            None => wander().normalize(),
        };
        let angle = ghost.heading.angle_between(desired);
        if angle > 1e-4 {
            let axis = ghost
                .heading
                .cross(desired)
                .try_normalize()
                .unwrap_or(Vec3::Y);
            let step = angle.min(TURN_RATE * seconds);
            self.ghost.heading = (Quat::from_axis_angle(axis, step) * ghost.heading).normalize();
        }
        if self.ghost.attention.is_some_and(|(_, until)| until <= time) {
            self.ghost.attention = None;
        }

        // Motes it sees become known to it, and the first sight of a
        // rainbow, snowfall, or visitor is an event. Asleep, it sees
        // nothing.
        let sight = if self.ghost.asleep < 0.5 {
            self.sight()
        } else {
            Sight::default()
        };
        if sight
            .in_view
            .iter()
            .any(|seen| seen.thing == Thing::Window && seen.hidden_behind.is_none())
        {
            self.notice_passersby();
        }
        // A shooting star counts once it has begun to streak.
        let shooting_star = self
            .shooting_star()
            .filter(|meteor| self.seen_meteor != Some(meteor.slot) && meteor.progress(time) > 0.1);
        for seen in &sight.in_view {
            if seen.thing == Thing::Window
                && seen.hidden_behind.is_none()
                && !self.ghost.saw_gift
                && let Some(gift) = self.life.sill
            {
                self.ghost.saw_gift = true;
                self.record(EventKind::Noticed(format!(
                    "a gift on the sill outside: {}",
                    gift.name()
                )));
            }
            match seen.thing {
                Thing::Mote(id)
                    if seen.hidden_behind.is_none() && !self.ghost.known.contains(&id) =>
                {
                    self.ghost.known.push(id);
                    self.record(EventKind::Noticed("a light mote".to_owned()));
                }
                Thing::Window
                    if seen.hidden_behind.is_none()
                        && self.weather.snow(time) > 0.3
                        && !self.ghost.saw_snow =>
                {
                    self.ghost.saw_snow = true;
                    self.record(EventKind::Noticed("the falling snow".to_owned()));
                }
                Thing::Window
                    if seen.hidden_behind.is_none()
                        && self.weather.rainbow(time) > 0.3
                        && !self.ghost.saw_rainbow =>
                {
                    self.ghost.saw_rainbow = true;
                    self.record(EventKind::Noticed(
                        "the rainbow through the window".to_owned(),
                    ));
                }
                Thing::Window
                    if seen.hidden_behind.is_none()
                        && let Some(meteor) = shooting_star
                        && self.seen_meteor != Some(meteor.slot) =>
                {
                    self.seen_meteor = Some(meteor.slot);
                    self.record(EventKind::ShootingStar);
                    self.wish();
                }
                Thing::Bird if seen.hidden_behind.is_none() => {
                    if let Some(bird) = &mut self.visitors.bird
                        && bird.perched(time)
                        && !bird.noticed
                    {
                        bird.noticed = true;
                        self.record(EventKind::Noticed("a bird at the window".to_owned()));
                    }
                }
                Thing::Moth if seen.hidden_behind.is_none() => {
                    if let Some(moth) = &mut self.visitors.moth
                        && !moth.noticed
                    {
                        moth.noticed = true;
                        self.record(EventKind::Noticed("a moth at the lamp".to_owned()));
                    }
                }
                _ => {}
            }
        }
        let motes = &self.motes;
        self.ghost
            .known
            .retain(|id| motes.iter().any(|mote| mote.id == *id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Mood, Sky, World};

    fn facing(world: &mut World, direction: Vec3) {
        world.ghost.heading = direction.normalize();
    }

    #[test]
    fn the_ghost_sees_what_it_faces_and_not_what_is_behind_it() {
        let mut world = World::new(1, 12.0);
        world.ghost.position = Vec3::new(0.0, 1.2, 0.6);
        facing(&mut world, -Vec3::Z);
        let sight = world.sight();
        let seen: Vec<Thing> = sight.in_view.iter().map(|seen| seen.thing).collect();
        assert!(seen.contains(&Thing::Window), "{seen:?}");
        assert_eq!(sight.looking_at, Some(Thing::Window));
        assert!(sight.out_of_view.contains(&Thing::Viewer));

        facing(&mut world, Vec3::Z);
        let sight = world.sight();
        assert_eq!(sight.looking_at, Some(Thing::Viewer));
        assert!(sight.out_of_view.contains(&Thing::Window));
    }

    #[test]
    fn looking_steeply_up_does_not_put_the_edge_of_view_in_focus() {
        let mut world = World::new(1, 12.0);
        world.ghost.position = Vec3::new(0.0, 0.45, -0.9);
        facing(&mut world, Vec3::new(0.0, 0.866, 0.5));
        let sight = world.sight();
        let window = sight
            .in_view
            .iter()
            .find(|seen| seen.thing == Thing::Window)
            .unwrap();
        assert!(window.off_center > 0.9, "{window:?}");
        assert_ne!(sight.looking_at, Some(Thing::Window));
    }

    #[test]
    fn things_to_the_left_and_right_have_their_side() {
        let mut world = World::new(1, 12.0);
        world.ghost.position = Vec3::new(0.0, 1.2, 0.8);
        facing(&mut world, -Vec3::Z);
        let sight = world.sight();
        let lamp = sight
            .in_view
            .iter()
            .find(|seen| seen.thing == Thing::Lamp)
            .unwrap();
        let plant = sight
            .in_view
            .iter()
            .find(|seen| seen.thing == Thing::Plant)
            .unwrap();
        assert!(
            lamp.across < 0.0 && plant.across > 0.0,
            "{lamp:?} {plant:?}"
        );
    }

    #[test]
    fn furniture_hides_what_is_behind_it() {
        let mut world = World::new(1, 12.0);
        // Looking along the floor at the ball from behind the plant.
        world.ball.position = world.plant.position + Vec3::new(-1.2, 0.0, 0.0);
        world.ball.position.y = Ball::RADIUS;
        world.ghost.position = world.plant.position + Vec3::new(0.6, 0.15, 0.0);
        let toward = world.ball.position - world.ghost.position;
        facing(&mut world, toward);
        let sight = world.sight();
        let ball = sight
            .in_view
            .iter()
            .find(|seen| seen.thing == Thing::Ball)
            .unwrap();
        assert_eq!(ball.hidden_behind, Some(Thing::Plant));
    }

    #[test]
    fn the_gaze_turns_smoothly_toward_what_holds_attention() {
        let mut world = World::new(1, 12.0);
        world.ghost.plan.done = true;
        facing(&mut world, Vec3::Z);
        world.attend("window", 10.0);
        world.step(0.1);
        let partway = world.ghost.heading;
        assert!(partway.angle_between(Vec3::Z) > 0.1);
        assert!(partway.angle_between(Vec3::Z) < 0.3);
        for _ in 0..40 {
            world.step(0.05);
        }
        assert_eq!(world.sight().looking_at, Some(Thing::Window));
    }

    #[test]
    fn a_mote_becomes_known_once_seen_and_only_known_motes_can_be_chased() {
        let mut world = World::new(1, 12.0);
        world.ghost.plan.done = true;
        world.ghost.position = Vec3::new(0.0, 1.2, 0.0);
        world.motes.clear();
        world.next_mote_at = f64::INFINITY;
        world.add_mote(Vec3::new(0.0, 1.4, 1.1));
        facing(&mut world, -Vec3::Z);
        world.ghost.attention = Some((Thing::Window, 100.0));
        world.step(0.05);
        assert!(world.ghost.known.is_empty());
        assert!(!world.options().contains(&crate::world::Activity::Chase));

        world.attend("viewer", 10.0);
        for _ in 0..30 {
            world.step(0.05);
        }
        assert_eq!(world.ghost.known.len(), 1);
        assert!(world.options().contains(&crate::world::Activity::Chase));
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Noticed("a light mote".into()))
        );
    }

    #[test]
    fn unknown_names_are_ignored_when_attending() {
        let mut world = World::new(1, 12.0);
        world.attend("the moon", 5.0);
        assert_eq!(world.ghost.attention, None);
        world.attend("plant", 5.0);
        assert_eq!(
            world.ghost.attention.map(|(thing, _)| thing),
            Some(Thing::Plant)
        );
    }

    #[test]
    fn every_name_minds_may_use_is_a_thing() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2026, 10, 31);
        world.drop_mote(Vec3::ONE);
        world.visitors.bird = Some(super::super::visitor::Bird::new(
            0.0,
            0.0,
            &mut super::super::Random::new(1),
        ));
        world.visitors.moth = Some(super::super::visitor::Moth::new(
            0.0,
            &mut super::super::Random::new(1),
        ));
        world.invite_friend();
        world.draw_on_glass(Vec3::new(0.0, 1.2, ROOM_MAX.z));
        world
            .seedlings
            .push(super::super::Seedling::new(super::super::Flower::Daisy));
        for name in Thing::NAMES {
            world.ghost.attention = None;
            world.attend(name, 5.0);
            assert!(world.ghost.attention.is_some(), "{name}");
        }
    }

    #[test]
    fn a_sleeping_ghost_notices_nothing_until_it_wakes() {
        let mut world = World::new(1, 23.0);
        world.date = jiff::civil::Date::constant(2026, 12, 10);
        world.time = 100.0;
        world.weather.sky = Sky::Snow;
        world.weather.since = 0.0;
        world.weather.until = 1000.0;
        world.tuck_in();
        world.attend("window", 100.0);
        let toward = WINDOW_CENTER - world.ghost.position;
        facing(&mut world, toward);
        world.step(1.0);
        assert!(world.ghost.asleep > 0.9);
        assert!(!world.ghost.saw_snow);
        world.poke();
        world.attend("window", 100.0);
        world.step(1.0);
        assert!(world.ghost.saw_snow);
    }

    #[test]
    fn snow_is_noticed_once_per_shower_only_after_the_window_is_seen() {
        let mut world = World::new(1, 12.0);
        // A winter day with no holiday to get excited about.
        world.date = jiff::civil::Date::constant(2026, 12, 10);
        world.time = 100.0;
        world.weather.sky = Sky::Snow;
        world.weather.since = 0.0;
        world.weather.until = 1000.0;
        world.ghost.plan.done = true;
        world.ghost.position = Vec3::new(0.0, 1.2, 0.0);
        world.attend("viewer", 100.0);
        facing(&mut world, Vec3::Z);
        world.step(0.1);
        assert!(!world.ghost.saw_snow);
        assert!(world.weather.snow_cover > 0.0);
        // The first scarf has its own milestone; isolate the snow reaction.
        world.events.clear();
        world.attend("window", 100.0);
        facing(&mut world, -Vec3::Z);
        world.step(0.1);
        assert!(world.ghost.saw_snow);
        world.step(0.1);
        assert_eq!(world.ghost.mood, Mood::Curious);
        let noticed = EventKind::Noticed("the falling snow".to_owned());
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == noticed)
                .count(),
            1
        );
        world.weather.until = world.time;
        world.step(0.1);
        assert!(!world.ghost.saw_snow);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::SnowStopped)
        );
        world.weather.until = world.time;
        world.step(0.1);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::SnowStarted)
        );
        world.step(20.0);
        assert!(world.ghost.saw_snow);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == noticed)
                .count(),
            2
        );
    }

    #[test]
    fn the_decoration_hides_what_stands_behind_it_only_while_present() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2026, 10, 31);
        let eye = Decoration::POSITION + Vec3::new(0.0, 0.18, 1.0);
        let target = Decoration::POSITION + Vec3::new(0.0, 0.18, -0.24);
        assert_eq!(
            world.blocker(eye, target, Thing::Ball),
            Some(Thing::Decoration)
        );
        assert_eq!(world.blocker(eye, target, Thing::Decoration), None);
        world.date = jiff::civil::Date::constant(2026, 11, 1);
        assert_eq!(world.blocker(eye, target, Thing::Ball), None);
    }

    #[test]
    fn blocks_can_hold_attention_and_a_tall_tower_hides_things_beyond_it() {
        let mut world = World::new(1, 12.0);
        let nearest = world
            .blocks
            .iter()
            .min_by(|first, second| {
                first
                    .position
                    .distance_squared(world.ghost.position)
                    .total_cmp(&second.position.distance_squared(world.ghost.position))
            })
            .unwrap()
            .position;
        assert_eq!(world.position_of(Thing::Blocks), Some(nearest));
        world.tower = Some(Vec3::ZERO);
        for (level, block) in world.blocks.iter_mut().take(3).enumerate() {
            block.position = Vec3::Y * ((level as f32 + 0.5) * super::super::blocks::EDGE);
            block.stacked = true;
        }
        world.attend("blocks", 3.0);
        assert_eq!(world.ghost.attention, Some((Thing::Blocks, 3.0)));
        let eye = Vec3::new(0.0, 0.25, 1.0);
        let target = Vec3::new(0.0, 0.25, -1.0);
        assert_eq!(
            world.blocker(eye, target, Thing::Plant),
            Some(Thing::Blocks)
        );
        assert_eq!(
            world.blocker(eye + Vec3::Y, target + Vec3::Y, Thing::Plant),
            None
        );
        assert_eq!(world.blocker(eye, target, Thing::Blocks), None);
        world.blocks[2].stacked = false;
        assert_eq!(world.blocker(eye, target, Thing::Plant), None);
    }
}
