use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use super::{Ball, Block, ROOM_MAX, ROOM_MIN, World, blocks::EDGE};

/// A resting toy layout. Invalid snapshots are discarded as a whole so a
/// fallback block cannot collide with a restored one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Room {
    #[serde(default)]
    pub ball: [f32; 3],
    #[serde(default)]
    pub blocks: Vec<Cube>,
    #[serde(default)]
    pub tower: Option<[f32; 3]>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cube {
    #[serde(default)]
    pub position: [f32; 3],
    #[serde(default)]
    pub orientation: [f32; 4],
    #[serde(default)]
    pub color: u8,
    #[serde(default)]
    pub stacked: bool,
}

fn inside(position: Vec3, margin: f32) -> bool {
    position.is_finite()
        && [0, 2].into_iter().all(|axis| {
            position[axis] >= ROOM_MIN[axis] + margin && position[axis] <= ROOM_MAX[axis] - margin
        })
        && (0.0..=ROOM_MAX.y).contains(&position.y)
}

fn overlaps(first: &Block, second: &Block) -> bool {
    let first_axes = [Vec3::X, Vec3::Y, Vec3::Z].map(|axis| first.orientation * axis);
    let second_axes = [Vec3::X, Vec3::Y, Vec3::Z].map(|axis| second.orientation * axis);
    let offset = second.position - first.position;
    // Separating axes also accept touching tower levels and yawed floor cubes.
    first_axes
        .into_iter()
        .chain(second_axes)
        .chain(first_axes.into_iter().flat_map(|first| {
            second_axes
                .into_iter()
                .map(move |second| first.cross(second))
        }))
        .filter(|axis| axis.length_squared() > 1e-8)
        .all(|axis| {
            let axis = axis.normalize();
            let reach = first_axes
                .into_iter()
                .chain(second_axes)
                .map(|edge| edge.dot(axis).abs() * EDGE * 0.5)
                .sum::<f32>();
            offset.dot(axis).abs() < reach - 0.001
        })
}

impl World {
    /// Saving never moves the live toys or advances the world's random source.
    pub fn saved_room(&self) -> Room {
        let position = if self.ball.held {
            self.ghost.position
        } else {
            self.ball.position
        };
        let ball = Ball::at(
            position
                .x
                .clamp(ROOM_MIN.x + Ball::RADIUS, ROOM_MAX.x - Ball::RADIUS),
            position
                .z
                .clamp(ROOM_MIN.z + Ball::RADIUS, ROOM_MAX.z - Ball::RADIUS),
        );
        let mut blocks: Vec<Block> = self
            .blocks
            .iter()
            .map(|block| {
                let mut resting = *block;
                if !resting.stacked {
                    if resting.held {
                        resting.position = self.ghost.position;
                    }
                    resting.orientation = resting.resting_orientation();
                    let margin = super::blocks::RADIUS;
                    resting.position.x = resting
                        .position
                        .x
                        .clamp(ROOM_MIN.x + margin, ROOM_MAX.x - margin);
                    resting.position.z = resting
                        .position
                        .z
                        .clamp(ROOM_MIN.z + margin, ROOM_MAX.z - margin);
                    resting.position.y = EDGE * 0.5;
                }
                resting
            })
            .collect();
        for index in 0..blocks.len() {
            if blocks[index].stacked
                || blocks
                    .iter()
                    .enumerate()
                    .all(|(other, block)| other == index || !overlaps(&blocks[index], block))
            {
                continue;
            }
            // A carried cube above a tower needs the nearest free floor spot,
            // otherwise saving mid-placement would invalidate the whole room.
            let origin = blocks[index].position;
            'search: for ring in 1..=20 {
                for direction in 0..32 {
                    let angle = direction as f32 * std::f32::consts::TAU / 32.0;
                    let mut candidate = blocks[index];
                    candidate.position =
                        origin + Vec3::new(angle.cos(), 0.0, angle.sin()) * (ring as f32 * EDGE);
                    if inside(candidate.position, super::blocks::RADIUS)
                        && blocks
                            .iter()
                            .enumerate()
                            .all(|(other, block)| other == index || !overlaps(&candidate, block))
                    {
                        blocks[index] = candidate;
                        break 'search;
                    }
                }
            }
        }
        Room {
            ball: ball.position.to_array(),
            blocks: blocks
                .iter()
                .map(|block| Cube {
                    position: block.position.to_array(),
                    orientation: block.orientation.to_array(),
                    color: block.color,
                    stacked: block.stacked,
                })
                .collect(),
            tower: self.tower.map(|base| base.to_array()),
        }
    }

    pub fn restore_room(&mut self, room: &Room) {
        let restored = self.checked_room(room);
        let (ball, blocks, tower) = restored.unwrap_or_else(|| {
            let seeded = World::new(self.meteor_seed, self.hour);
            (seeded.ball, seeded.blocks, None)
        });
        self.ball = ball;
        self.blocks = blocks;
        self.tower = tower;
    }

    fn checked_room(&self, room: &Room) -> Option<(Ball, Vec<Block>, Option<Vec3>)> {
        let ball = Vec3::from_array(room.ball);
        if !inside(ball, Ball::RADIUS)
            || (ball.y - Ball::RADIUS).abs() > 0.001
            || room.blocks.len() != 5
        {
            return None;
        }
        let tower = room.tower.map(Vec3::from_array);
        if tower.is_some_and(|base| !inside(base, super::blocks::RADIUS) || base.y.abs() > 0.001) {
            return None;
        }
        let mut blocks = Vec::new();
        let mut colors = [false; 5];
        for cube in &room.blocks {
            let position = Vec3::from_array(cube.position);
            let orientation = Quat::from_array(cube.orientation);
            if !inside(position, super::blocks::RADIUS)
                || !orientation.is_finite()
                || (orientation.length_squared() - 1.0).abs() > 0.001
                || cube.color >= 5
                || colors[cube.color as usize]
            {
                return None;
            }
            colors[cube.color as usize] = true;
            let mut block = Block::at(position, cube.color);
            block.position = position;
            block.orientation = orientation.normalize();
            block.stacked = cube.stacked;
            if block.orientation.angle_between(block.resting_orientation()) > 0.005 {
                return None;
            }
            if cube.stacked {
                let base = tower?;
                if super::flat(position - base).length() > 0.06 {
                    return None;
                }
            } else if (position.y - EDGE * 0.5).abs() > 0.001 {
                return None;
            }
            if blocks.iter().any(|other| overlaps(&block, other)) {
                return None;
            }
            blocks.push(block);
        }
        let mut levels: Vec<f32> = blocks
            .iter()
            .filter(|block| block.stacked)
            .map(|block| block.position.y)
            .collect();
        levels.sort_by(f32::total_cmp);
        if tower.is_some() != !levels.is_empty()
            || levels
                .iter()
                .enumerate()
                .any(|(index, height)| (*height - (index as f32 + 0.5) * EDGE).abs() > 0.001)
        {
            return None;
        }
        Some((Ball::at(ball.x, ball.z), blocks, tower))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resting_toys_and_a_standing_tower_round_trip() {
        let mut world = World::new(7, 12.0);
        world.ball = Ball::at(-0.4, 0.9);
        let base = Vec3::new(0.0, 0.0, 0.4);
        for (index, block) in world.blocks.iter_mut().enumerate() {
            block.position = if index < 3 {
                base + Vec3::Y * ((index as f32 + 0.5) * EDGE)
            } else {
                Vec3::new(-0.8 + index as f32 * 0.5, EDGE * 0.5, -0.4)
            };
            block.orientation = Quat::from_rotation_y(0.1);
            block.stacked = index < 3;
        }
        world.tower = Some(base);
        let room = world.saved_room();
        let encoded = serde_json::to_string(&room).unwrap();
        let mut restored = World::new(9, 12.0);
        restored.restore_room(&serde_json::from_str(&encoded).unwrap());
        assert_eq!(restored.saved_room(), room);
        assert_eq!(restored.tower_height(), 3);
        restored.topple(Vec3::X);
        assert!(restored.tower.is_none());
        assert!(restored.blocks.iter().all(|block| !block.stacked));
    }

    #[test]
    fn held_and_moving_toys_are_saved_at_rest() {
        let mut world = World::new(7, 12.0);
        world.ghost.position = Vec3::new(0.0, 1.2, 0.8);
        world.ball.held = true;
        world.ball.velocity = Vec3::ONE;
        world.blocks[0].held = true;
        world.blocks[1].position.y = 1.0;
        world.blocks[1].orientation = Quat::from_rotation_x(0.6);
        world.blocks[1].velocity = Vec3::ONE;
        world.blocks[1].spin = Vec3::ONE;
        let room = world.saved_room();
        assert_eq!(room.ball, [0.0, Ball::RADIUS, 0.8]);
        assert_eq!(room.blocks[0].position, [0.0, EDGE * 0.5, 0.8]);
        assert_eq!(room.blocks[1].position[1], EDGE * 0.5);
        let mut restored = World::new(7, 12.0);
        restored.restore_room(&room);
        assert_eq!(restored.saved_room(), room);
        assert_eq!(restored.ball.velocity, Vec3::ZERO);
        assert!(
            restored.blocks.iter().all(|block| !block.held
                && block.velocity == Vec3::ZERO
                && block.spin == Vec3::ZERO)
        );
        assert!(world.ball.held && world.blocks[0].held);
        world.ball.held = false;
        assert_eq!(world.saved_room().ball, [0.7, Ball::RADIUS, 0.45]);
    }

    #[test]
    fn saving_during_a_placement_preserves_the_tower_and_finds_floor_space() {
        let mut world = World::new(7, 12.0);
        let base = Vec3::new(0.0, 0.0, 0.5);
        world.tower = Some(base);
        world.blocks[0].position = base + Vec3::Y * EDGE * 0.5;
        world.blocks[0].stacked = true;
        world.blocks[1].held = true;
        world.ghost.position = base + Vec3::Y;
        let room = world.saved_room();
        let mut restored = World::new(9, 12.0);
        restored.restore_room(&room);
        assert_eq!(restored.tower, Some(base));
        assert_eq!(restored.tower_height(), 1);
        assert!(!restored.blocks[1].held);
        assert!((restored.blocks[1].position.y - EDGE * 0.5).abs() < 0.001);
        assert!(!overlaps(&restored.blocks[0], &restored.blocks[1]));
    }

    #[test]
    fn corrupt_layouts_fall_back_to_the_seeded_toys() {
        let mut world = World::new(5, 12.0);
        let seeded = world.saved_room();
        for corruption in 0..8 {
            let mut room = seeded.clone();
            match corruption {
                0 => room.ball[0] = f32::NAN,
                1 => room.blocks.pop().map(|_| ()).unwrap(),
                2 => room.blocks[0].position[2] = 100.0,
                3 => room.blocks[0].orientation = [0.0; 4],
                4 => room.blocks[0].color = 5,
                5 => room.blocks[1].position = room.blocks[0].position,
                6 => room.tower = Some([f32::INFINITY, 0.0, 0.0]),
                _ => room.blocks[0].stacked = true,
            }
            world.ball = Ball::at(-0.5, -0.5);
            world.restore_room(&room);
            assert_eq!(world.saved_room(), seeded, "corruption {corruption}");
        }
    }
}
