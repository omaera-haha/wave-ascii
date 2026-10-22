//! The cottage around the room, and the yard around the cottage.
//!
//! The room keeps its box. Walls get thickness, a back window, and a door
//! in the right wall. Fenced lawn all around: a big tree behind the house,
//! and the side yard on the right with a swing, garden bed, birdbath,
//! bench, lantern, and mailbox.

use glam::{Vec2, Vec3};

use crate::sdf::smoothstep;

use super::{ROOM_MAX, ROOM_MIN, World, flat};

/// Wall and ceiling thickness.
pub const WALL: f32 = 0.12;
/// Outside of the walls. The glass front has no thickness.
pub const HOUSE_MIN: Vec3 = Vec3::new(ROOM_MIN.x - WALL, 0.0, ROOM_MIN.z - WALL);
pub const HOUSE_MAX: Vec3 = Vec3::new(ROOM_MAX.x + WALL, ROOM_MAX.y + WALL, ROOM_MAX.z);

/// A low gable roof over it all: its ridge runs left to right.
pub const RIDGE: f32 = 3.0;
pub const EAVE_DROP: f32 = 2.47;
/// How far the roof reaches past the walls at the front and back, and at
/// the gable ends.
pub const EAVE: f32 = 0.26;
pub const GABLE: f32 = 0.18;
/// The chimney on the back slope: its middle, half width, and top.
pub const CHIMNEY: Vec3 = Vec3::new(-1.25, 0.0, -0.78);
pub const CHIMNEY_HALF: f32 = 0.17;
pub const CHIMNEY_TOP: f32 = 3.42;

/// Door in the right wall, (z, y) corners.
pub const DOOR_LOW: Vec2 = Vec2::new(0.42, 0.0);
pub const DOOR_HIGH: Vec2 = Vec2::new(1.12, 1.78);
/// Where the ghost lines up on each side of the door.
pub const DOOR_INSIDE: Vec3 = Vec3::new(ROOM_MAX.x - 0.42, 0.95, 0.77);
pub const DOOR_OUTSIDE: Vec3 = Vec3::new(HOUSE_MAX.x + 0.5, 0.95, 0.77);

/// Fenced lawn, (x, z) corners. The front stays open toward the viewer.
pub const LOT_MIN: Vec2 = Vec2::new(-6.6, -5.8);
pub const LOT_MAX: Vec2 = Vec2::new(8.8, 3.4);

/// Big tree behind the house; its crown shows over the roofline and its
/// low branch through the window.
pub const TREE: Vec3 = Vec3::new(0.95, 0.0, -4.4);
pub const TREE_CROWN: Vec3 = Vec3::new(0.85, 3.45, -4.3);
pub const TREE_CROWN_RADIUS: f32 = 1.6;
/// The low branch's tip, where the bird perches, seen through the window.
pub const PERCH: Vec3 = Vec3::new(-0.35, 1.32, -3.55);
/// Outer sill under the back window, where gifts and the bird sit.
pub const SILL: Vec3 = Vec3::new(0.0, 1.04, HOUSE_MIN.z - 0.08);
pub const SILL_HALF: Vec3 = Vec3::new(0.86, 0.025, 0.09);

/// Swing set: middle of the top bar, and the seat at rest.
pub const SWING_TOP: Vec3 = Vec3::new(5.0, 2.05, -1.35);
pub const SWING_SEAT: Vec3 = Vec3::new(5.0, 0.5, -1.35);
pub const SWING_HALF_WIDTH: f32 = 0.85;
/// Raised garden bed: middle on the ground, half size.
pub const GARDEN: Vec3 = Vec3::new(4.15, 0.0, 1.05);
pub const GARDEN_HALF: Vec3 = Vec3::new(0.86, 0.12, 0.3);
pub const BIRDBATH: Vec3 = Vec3::new(6.85, 0.0, 0.55);
pub const BENCH: Vec3 = Vec3::new(7.2, 0.0, -2.75);
pub const LANTERN: Vec3 = Vec3::new(3.05, 0.0, 2.2);
pub const LANTERN_HEIGHT: f32 = 1.25;
pub const MAILBOX: Vec3 = Vec3::new(8.0, 0.0, 2.95);
/// Clothesline posts in the left yard.
pub const CLOTHESLINE: [Vec3; 2] = [Vec3::new(-5.6, 0.0, -1.0), Vec3::new(-3.0, 0.0, -2.1)];
pub const CLOTHESLINE_HEIGHT: f32 = 1.55;
/// Bushes: ground center and radius.
pub const BUSHES: [(Vec3, f32); 6] = [
    (Vec3::new(-0.9, 0.0, -3.0), 0.6),
    (Vec3::new(-1.75, 0.0, -2.35), 0.42),
    (Vec3::new(-4.4, 0.0, -4.7), 0.7),
    (Vec3::new(8.0, 0.0, -4.9), 0.6),
    (Vec3::new(3.0, 0.0, -4.95), 0.55),
    (Vec3::new(-5.9, 0.0, 2.4), 0.5),
];

/// Places the ghost likes in the yard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    Swing,
    Garden,
    Birdbath,
    Bench,
    Lawn,
    Fence,
    /// Up on the roof's ridge, for the stars.
    Roof,
}

impl Spot {
    pub const ALL: [Self; 7] = [
        Self::Swing,
        Self::Garden,
        Self::Birdbath,
        Self::Bench,
        Self::Lawn,
        Self::Fence,
        Self::Roof,
    ];

    /// Where the ghost floats for this spot, for a ghost of `size`.
    pub fn position(self, size: f32) -> Vec3 {
        match self {
            Self::Swing => SWING_SEAT + Vec3::Y * (0.34 * size),
            Self::Garden => GARDEN + Vec3::new(0.0, 0.8, -0.62),
            Self::Birdbath => BIRDBATH + Vec3::new(-0.5, 0.95, 0.2),
            Self::Bench => BENCH + Vec3::new(-0.32, 0.42 + 0.34 * size, 0.08),
            Self::Lawn => Vec3::new(5.3, 0.32 * size + 0.06, 1.55),
            Self::Fence => Vec3::new(7.6, 1.05, -5.2),
            Self::Roof => Vec3::new(0.55, RIDGE + 0.36 * size, (HOUSE_MIN.z + HOUSE_MAX.z) * 0.5),
        }
    }

    /// What the ghost looks at from the spot.
    pub fn looks_at(self) -> Vec3 {
        match self {
            Self::Swing => SWING_SEAT + Vec3::new(0.0, 0.6, 4.0),
            Self::Garden => GARDEN + Vec3::Y * 0.2,
            Self::Birdbath => BIRDBATH + Vec3::Y * 0.8,
            Self::Bench => TREE_CROWN,
            Self::Lawn => Vec3::new(4.0, 7.0, -14.0),
            Self::Fence => Vec3::new(10.0, 1.6, -24.0),
            Self::Roof => Vec3::new(0.0, 9.0, -20.0),
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Self::Swing => "swinging on the swing in the yard",
            Self::Garden => "looking over the garden bed",
            Self::Birdbath => "peering into the birdbath",
            Self::Bench => "sitting on the bench in the yard",
            Self::Lawn => "lying on the lawn watching the sky",
            Self::Fence => "leaning on the back fence looking at the hills",
            Self::Roof => "sitting up on the roof watching the stars",
        }
    }
}

/// Inside the room, walls excluded.
pub fn indoors(point: Vec3) -> bool {
    point.cmpge(ROOM_MIN).all() && point.cmple(ROOM_MAX).all()
}

/// Inside the cottage's footprint, walls included.
pub fn in_house(point: Vec3) -> bool {
    over_house(point) && point.y < HOUSE_MAX.y + 0.1
}

/// Above the cottage's footprint, at any height.
fn over_house(point: Vec3) -> bool {
    point.x >= HOUSE_MIN.x
        && point.x <= HOUSE_MAX.x
        && point.z >= HOUSE_MIN.z
        && point.z <= HOUSE_MAX.z
}

/// Inside the cottage's footprint or within `margin` of its walls.
pub fn near_house(point: Vec3, margin: f32) -> bool {
    point.x >= HOUSE_MIN.x - margin
        && point.x <= HOUSE_MAX.x + margin
        && point.z >= HOUSE_MIN.z - margin
        && point.z <= HOUSE_MAX.z + margin
}

/// Whether a line of sight from `eye` to `target` gets through the walls:
/// both on the same side of them, or through the back window, the glass
/// front, or the door while it stands open.
pub fn seen_through(eye: Vec3, target: Vec3, door: f32) -> bool {
    if in_house(eye) == in_house(target) {
        return true;
    }
    let along = target - eye;
    let crossing = |axis: usize, plane: f32| -> Option<Vec3> {
        if along[axis].abs() < 1e-6 {
            return None;
        }
        let fraction = (plane - eye[axis]) / along[axis];
        (0.0..=1.0)
            .contains(&fraction)
            .then(|| eye + along * fraction)
    };
    let window = crossing(2, ROOM_MIN.z).is_some_and(|point| {
        let at = Vec2::new(point.x, point.y);
        at.cmpge(Vec2::new(-0.75, 1.0)).all() && at.cmple(Vec2::new(0.75, 2.0)).all()
    });
    let glass = crossing(2, ROOM_MAX.z)
        .is_some_and(|point| point.x > ROOM_MIN.x && point.x < ROOM_MAX.x && point.y < ROOM_MAX.y);
    let doorway = door > 0.3
        && crossing(0, ROOM_MAX.x).is_some_and(|point| {
            let at = Vec2::new(point.z, point.y);
            at.cmpge(DOOR_LOW).all() && at.cmple(DOOR_HIGH).all()
        });
    window || glass || doorway
}

/// On the fenced lawn.
pub fn on_lot(point: Vec3) -> bool {
    let at = flat(point);
    at.cmpge(LOT_MIN).all() && at.cmple(LOT_MAX).all()
}

/// The box the ghost keeps inside while out in the yard.
pub fn yard_bounds() -> (Vec3, Vec3) {
    (
        Vec3::new(LOT_MIN.x + 0.35, 0.0, LOT_MIN.y + 0.35),
        Vec3::new(LOT_MAX.x - 0.35, RIDGE + 1.0, LOT_MAX.y - 0.35),
    )
}

/// Next place to head for from `from` toward `to`: through the door when
/// one is in the house and the other out.
pub fn waypoint(from: Vec3, to: Vec3) -> Vec3 {
    let inside = in_house(from);
    // Up to the roof and down again outside the walls, never through
    // the eaves.
    let aloft = RIDGE + 0.55;
    let roof = |point: Vec3| over_house(point) && point.y > HOUSE_MAX.y;
    if !inside && !roof(from) && roof(to) && from.y < aloft - 0.1 {
        // Out of the doorway first, then straight up beside the wall.
        if in_doorway(from) {
            return Vec3::new(HOUSE_MAX.x + 1.0, from.y, from.z);
        }
        return Vec3::new(from.x, aloft, from.z);
    }
    if roof(from) && !roof(to) {
        let off = Vec3::new(HOUSE_MAX.x + 0.6, from.y.max(aloft), from.z);
        return if over_house(from) || from.y > aloft - 0.2 && !over_house(to) && !in_house(to) {
            off
        } else {
            to
        };
    }
    if !inside && !over_house(from) && from.y > HOUSE_MAX.y && !roof(to) && in_house(to) {
        // Coming down from up high beside the house: down to the door.
        return Vec3::new(from.x, DOOR_OUTSIDE.y, from.z);
    }
    if inside == in_house(to) {
        return if inside { to } else { around(from, to) };
    }
    let (near, far) = if inside {
        (DOOR_INSIDE, DOOR_OUTSIDE)
    } else {
        (DOOR_OUTSIDE, DOOR_INSIDE)
    };
    // Lined up with the door: cross. Otherwise line up first.
    let aligned = (from.z - near.z).abs() < 0.22 && (from.y - near.y).abs() < 0.45;
    let close = (from.x - near.x).abs() < 0.3 || (inside == (from.x > near.x));
    if aligned && close {
        far
    } else if inside {
        near
    } else {
        around(from, near)
    }
}

/// Out in the yard, the next place on the way to `to` that skirts the
/// cottage: `to` itself when the house isn't in the way, else the first
/// corner of the shortest way around it.
fn around(from: Vec3, to: Vec3) -> Vec3 {
    let clear = HOUSE_MAX.y + 0.3;
    if from.y > clear && to.y > clear {
        return to;
    }
    // The house and its eaves with room to spare, and corners just past
    // them, in order around it.
    let low = Vec2::new(HOUSE_MIN.x - EAVE, HOUSE_MIN.z - EAVE);
    let high = Vec2::new(HOUSE_MAX.x + EAVE, HOUSE_MAX.z + EAVE);
    let start = flat(from);
    let end = flat(to);
    if !crosses(start, end, low, high) {
        return to;
    }
    let reach = 0.25;
    // Hugging the wall, as just out of the door: step away from it first.
    if start.cmpgt(low).all() && start.cmplt(high).all() {
        let gaps = [
            start.x - low.x,
            high.x - start.x,
            start.y - low.y,
            high.y - start.y,
        ];
        let side = (0..4)
            .min_by(|first, second| gaps[*first].total_cmp(&gaps[*second]))
            .unwrap_or(1);
        let out = match side {
            0 => Vec2::new(low.x - reach, start.y),
            1 => Vec2::new(high.x + reach, start.y),
            2 => Vec2::new(start.x, low.y - reach),
            _ => Vec2::new(start.x, high.y + reach),
        };
        return Vec3::new(out.x, from.y, out.y);
    }
    let corners = [
        Vec2::new(low.x - reach, low.y - reach),
        Vec2::new(high.x + reach, low.y - reach),
        Vec2::new(high.x + reach, high.y + reach),
        Vec2::new(low.x - reach, high.y + reach),
    ];
    let seen = |point: Vec2, corner: Vec2| !crosses(point, corner, low, high);
    // From each corner the start can reach, around either way to a corner
    // that sees the end.
    let mut best: Option<(f32, Vec2)> = None;
    for first in 0..4 {
        if !seen(start, corners[first]) {
            continue;
        }
        for step in [1, 3] {
            let mut cost = start.distance(corners[first]);
            let mut index = first;
            for _ in 0..4 {
                if seen(corners[index], end) {
                    let total = cost + corners[index].distance(end);
                    if best.is_none_or(|(best, _)| total < best) {
                        // Standing on the first corner already, aim past it.
                        let next = if corners[first].distance(start) < 0.05 && index != first {
                            corners[(first + step) % 4]
                        } else {
                            corners[first]
                        };
                        best = Some((total, next));
                    }
                    break;
                }
                let next = (index + step) % 4;
                cost += corners[index].distance(corners[next]);
                index = next;
            }
        }
    }
    best.map_or(to, |(_, corner)| {
        if corner.distance(start) < 0.05 {
            to
        } else {
            Vec3::new(corner.x, (from.y + to.y) * 0.5, corner.y)
        }
    })
}

/// Whether the segment from `start` to `end` passes through the inside of
/// the rectangle from `low` to `high`.
fn crosses(start: Vec2, end: Vec2, low: Vec2, high: Vec2) -> bool {
    let along = end - start;
    let (mut enter, mut exit) = (0.0_f32, 1.0_f32);
    for axis in 0..2 {
        if along[axis].abs() < 1e-6 {
            if start[axis] <= low[axis] || start[axis] >= high[axis] {
                return false;
            }
            continue;
        }
        let first = (low[axis] - start[axis]) / along[axis];
        let second = (high[axis] - start[axis]) / along[axis];
        enter = enter.max(first.min(second));
        exit = exit.min(first.max(second));
    }
    exit - enter > 1e-4
}

/// In the doorway, where the wall lets the ghost through.
pub fn in_doorway(point: Vec3) -> bool {
    point.z > DOOR_LOW.x + 0.05
        && point.z < DOOR_HIGH.x - 0.05
        && point.y < DOOR_HIGH.y
        && point.x > DOOR_INSIDE.x - 0.4
        && point.x < HOUSE_MAX.x + 0.15
}

/// Yard things the ghost steers around: center, radius, top. The garden
/// bed is low enough to float over.
pub fn yard_obstacles() -> [(Vec3, f32, f32); 5] {
    [
        (BIRDBATH, 0.36, 0.95),
        (LANTERN, 0.14, LANTERN_HEIGHT + 0.2),
        (MAILBOX, 0.2, 1.15),
        // Low enough to sit on.
        (BENCH, 0.55, 0.5),
        (TREE, 0.4, 2.4),
    ]
}

impl World {
    /// How far the door stands open: it swings out as the ghost (or its
    /// friend) comes near it.
    pub fn door_open(&self) -> f32 {
        let middle = Vec3::new(
            HOUSE_MAX.x - WALL * 0.5,
            0.9,
            (DOOR_LOW.x + DOOR_HIGH.x) * 0.5,
        );
        let near = |position: Vec3| 1.0 - smoothstep(0.55, 1.05, position.distance(middle));
        let friend = self
            .friend
            .as_ref()
            .map_or(0.0, |friend| near(friend.position));
        near(self.ghost.position).max(friend)
    }

    /// Where the camera should look: the room, or the ghost out in the yard.
    pub fn view_focus(&self, home: Vec3) -> Vec3 {
        let ghost = self.ghost.position;
        if in_house(ghost) {
            return home;
        }
        Vec3::new(
            (ghost.x - 0.7).clamp(home.x - 3.0, 5.6),
            home.y.max(ghost.y - 1.1),
            (ghost.z * 0.55).clamp(-1.6, 1.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk(mut position: Vec3, goal: Vec3) -> (Vec3, bool) {
        let mut crossed = false;
        for _ in 0..4000 {
            let next = waypoint(position, goal);
            let offset = next - position;
            if offset.length() < 0.02 {
                break;
            }
            let was_inside = in_house(position);
            position += offset.clamp_length_max(0.02);
            if was_inside != in_house(position) {
                assert!(in_doorway(position), "went through the wall at {position}");
                crossed = true;
            }
        }
        (position, crossed)
    }

    #[test]
    fn a_trip_from_the_room_to_the_swing_goes_out_through_the_door() {
        let goal = Spot::Swing.position(1.0);
        let (position, crossed) = walk(Vec3::new(-1.0, 1.0, -0.5), goal);
        assert!(crossed);
        assert!(position.distance(goal) < 0.05);
    }

    #[test]
    fn coming_back_in_also_uses_the_door() {
        let (position, crossed) = walk(Spot::Fence.position(1.0), Vec3::new(0.0, 1.2, 0.0));
        assert!(crossed);
        assert!(indoors(position));
    }

    #[test]
    fn the_way_up_to_the_roof_and_back_in_stays_clear_of_the_walls_and_eaves() {
        let roof = Spot::Roof.position(1.0);
        let start = Vec3::new(-1.0, 1.0, -0.5);
        let mut position = start;
        for goal in [roof, start] {
            for _ in 0..8000 {
                let next = waypoint(position, goal);
                let offset = next - position;
                if offset.length() < 0.02 {
                    break;
                }
                let was_inside = in_house(position);
                position += offset.clamp_length_max(0.02);
                if was_inside != in_house(position) {
                    assert!(in_doorway(position), "through the wall at {position}");
                }
                // Never inside the roof: above the walls, over the house,
                // it stays above the ridge.
                if over_house(position) && position.y > HOUSE_MAX.y {
                    assert!(position.y > RIDGE, "through the roof at {position}");
                }
            }
            assert!(position.distance(goal) < 0.05, "{position} vs {goal}");
        }
    }

    #[test]
    fn trips_across_the_yard_go_around_the_house_not_through_it() {
        let size = 1.0;
        let left = Vec3::new(-4.5, 0.9, 1.0);
        let behind = Vec3::new(-1.0, 0.9, -3.4);
        for (start, goal) in [
            (Spot::Swing.position(size), left),
            (left, Spot::Birdbath.position(size)),
            (behind, Spot::Lawn.position(size)),
            (left, Vec3::new(0.0, 1.2, 0.0)),
        ] {
            let mut position = start;
            for _ in 0..20000 {
                let next = waypoint(position, goal);
                let offset = next - position;
                if offset.length() < 0.02 {
                    break;
                }
                let was_inside = in_house(position);
                position += offset.clamp_length_max(0.02);
                if !was_inside && in_house(position) {
                    assert!(in_doorway(position), "into the wall at {position}");
                }
            }
            assert!(position.distance(goal) < 0.05, "{start} to {goal} stopped at {position}");
        }
    }

    #[test]
    fn yard_spots_are_on_the_lot_and_outside_the_house() {
        let (low, high) = yard_bounds();
        for spot in Spot::ALL {
            let position = spot.position(1.0);
            assert!(on_lot(position), "{spot:?}");
            assert!(!in_house(position), "{spot:?}");
            assert!(
                position.cmpge(low).all() && position.cmple(high).all(),
                "{spot:?}"
            );
        }
    }
}
