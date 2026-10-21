use std::f32::consts::PI;

use glam::{Quat, Vec3};

use super::things::{Nook, Plant, ROOM_MAX, WINDOW_CENTER};
use super::{Ball, EventKind, World};

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
    Mote(u32),
}

impl Thing {
    /// The names minds use for things, as in `World::attend`.
    pub const NAMES: [&str; 7] = [
        "viewer",
        "window",
        "plant",
        "ball",
        "lamp",
        "cushion",
        "light mote",
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Window => "window",
            Self::Lamp => "lamp",
            Self::Plant => "plant",
            Self::Ball => "ball",
            Self::Cushion => "cushion",
            Self::Mote(_) => "light mote",
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
            Thing::Viewer => self
                .viewer
                .pointer
                .unwrap_or(Vec3::new(0.0, 1.2, ROOM_MAX.z)),
            Thing::Window => WINDOW_CENTER,
            Thing::Lamp => self.lamp.bulb(),
            Thing::Plant => self.plant.position + Vec3::Y * self.plant.height() * 0.6,
            Thing::Ball => self.ball.position,
            Thing::Cushion => self.nook.position + Vec3::Y * 0.1,
            Thing::Mote(id) => self.motes.iter().find(|mote| mote.id == id)?.position,
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
            ]
            .into_iter()
            .find(|thing| thing.name() == name),
        };
        if let Some(thing) = thing {
            self.ghost.attention = Some((thing, self.time + f64::from(seconds)));
        }
    }

    // The furniture standing between the eye and a thing, if any, judged by
    // rough spheres around each piece.
    fn blocker(&self, eye: Vec3, target: Vec3, thing: Thing) -> Option<Thing> {
        let blockers = [
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
        ];
        let along = target - eye;
        let length = along.length();
        blockers.into_iter().find_map(|(blocker, center, radius)| {
            if blocker == thing {
                return None;
            }
            let fraction = ((center - eye).dot(along) / (length * length)).clamp(0.0, 1.0);
            let closest = eye + along * fraction;
            (fraction > 0.02 && fraction < 0.95 && closest.distance(center) < radius)
                .then_some(blocker)
        })
    }

    /// Turns the ghost's gaze toward what holds its attention: what a mind
    /// asked it to look at, else what its plan is about, else where it is
    /// going, else wherever its eyes wander. Then notes what it newly sees.
    pub(super) fn look_around(&mut self, seconds: f32) {
        let time = self.time;
        let ghost = &self.ghost;
        let attended = ghost
            .attention
            .filter(|(_, until)| *until > time)
            .and_then(|(thing, _)| self.position_of(thing));
        let wander = || {
            let phase = time as f32;
            let yaw = PI * (phase * 0.045).sin() + 0.6 * (phase * 0.21).sin();
            Vec3::new(yaw.sin(), 0.15 * (phase * 0.17).sin(), yaw.cos())
        };
        let desired = match attended.or(ghost.look) {
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
        // rainbow or snowfall is an event.
        let sight = self.sight();
        for seen in &sight.in_view {
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
        world.drop_mote(Vec3::ONE);
        for name in Thing::NAMES {
            world.ghost.attention = None;
            world.attend(name, 5.0);
            assert!(world.ghost.attention.is_some(), "{name}");
        }
    }

    #[test]
    fn snow_is_noticed_once_per_shower_only_after_the_window_is_seen() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2026, 12, 24);
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
}
