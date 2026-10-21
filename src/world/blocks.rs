use glam::{Quat, Vec3};

use super::{ARRIVED, Activity, Ball, EventKind, Mood, ROOM_MAX, ROOM_MIN, World, flat};

pub const EDGE: f32 = 0.18;
/// A sphere enclosing a block even while it tumbles.
pub const RADIUS: f32 = EDGE * 0.866_025_4;
const CARRY_HEIGHT: f32 = 0.42;

/// A toy cube, either loose, carried, or resting in a standing tower.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub position: Vec3,
    pub orientation: Quat,
    pub velocity: Vec3,
    pub spin: Vec3,
    pub color: u8,
    pub held: bool,
    pub stacked: bool,
}

impl Block {
    pub fn at(position: Vec3, color: u8) -> Self {
        Self {
            position: Vec3::new(position.x, EDGE * 0.5, position.z),
            orientation: Quat::IDENTITY,
            velocity: Vec3::ZERO,
            spin: Vec3::ZERO,
            color,
            held: false,
            stacked: false,
        }
    }

    fn floor_height(&self) -> f32 {
        let local_up = self.orientation.conjugate() * Vec3::Y;
        local_up.abs().element_sum() * EDGE * 0.5
    }

    // The smallest turn that brings the nearest face flat, retaining yaw.
    pub(super) fn resting_orientation(&self) -> Quat {
        let face = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z]
            .into_iter()
            .map(|axis| self.orientation * axis)
            .max_by(|first, second| first.y.total_cmp(&second.y))
            .unwrap();
        (Quat::from_rotation_arc(face, Vec3::Y) * self.orientation).normalize()
    }

    /// Whether physics has nothing left to do with it: held, stacked, or
    /// lying still on the floor.
    fn settled(&self) -> bool {
        self.held
            || self.stacked
            || (self.velocity == Vec3::ZERO
                && self.spin == Vec3::ZERO
                && self.position.y <= self.floor_height() + 0.001)
    }

    pub fn on_floor(&self) -> bool {
        !self.held
            && !self.stacked
            && self.position.y <= RADIUS + 0.01
            && self.velocity.length() < 0.25
    }

    /// Gravity, floor bounces, friction, and tumbling. The world uses short
    /// substeps so a quick ball or block cannot skip a small obstacle.
    pub fn step(&mut self, seconds: f32, obstacles: &[(Vec3, f32)]) {
        if self.held || self.stacked {
            return;
        }
        let grounded = self.position.y <= self.floor_height() + 0.001 && self.velocity.y <= 0.0;
        if grounded {
            self.velocity.y = 0.0;
        } else {
            self.velocity.y -= 9.0 * seconds;
        }
        self.position += self.velocity * seconds;
        self.orientation =
            (Quat::from_scaled_axis(self.spin * seconds) * self.orientation).normalize();
        let floor = self.floor_height();
        let landed = self.position.y <= floor + 0.001;
        if landed {
            self.position.y = floor;
            if self.velocity.y < -0.6 - 9.0 * seconds {
                self.velocity.y *= -0.3;
                self.spin += Vec3::new(self.velocity.z, 0.0, -self.velocity.x);
            } else {
                self.velocity.y = 0.0;
            }
        }
        let friction = (-if landed { 3.0 } else { 0.2 } * seconds).exp();
        self.velocity.x *= friction;
        self.velocity.z *= friction;
        self.spin *= (-if landed { 4.0 } else { 0.5 } * seconds).exp();
        if landed && self.velocity.length() < 0.12 && self.spin.length() < 0.4 {
            let resting = self.resting_orientation();
            self.orientation = self
                .orientation
                .slerp(resting, 1.0 - (-12.0 * seconds).exp());
            if self.orientation.angle_between(resting) < 0.002 {
                self.orientation = resting;
            }
            self.position.y = self.floor_height();
            if self.velocity.length() < 0.015 && self.spin.length() < 0.02 {
                self.velocity = Vec3::ZERO;
                self.spin = Vec3::ZERO;
            }
        }
        for (center, radius) in obstacles {
            let offset = flat(self.position - *center);
            let reach = radius + RADIUS;
            let distance = offset.length();
            if distance < reach {
                let across = offset.normalize_or(glam::Vec2::X);
                let normal = Vec3::new(across.x, 0.0, across.y);
                self.position += normal * (reach - distance);
                let toward = self.velocity.dot(normal);
                if toward < 0.0 {
                    self.velocity -= normal * toward * 1.4;
                }
            }
        }
        self.contain();
    }

    fn contain(&mut self) {
        for axis in [0, 2] {
            let low = ROOM_MIN[axis] + RADIUS;
            let high = ROOM_MAX[axis] - RADIUS;
            if self.position[axis] < low {
                self.position[axis] = low;
                self.velocity[axis] = self.velocity[axis].abs() * 0.4;
            } else if self.position[axis] > high {
                self.position[axis] = high;
                self.velocity[axis] = -self.velocity[axis].abs() * 0.4;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    Gather,
    Carry,
    Admire,
    Swoop,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Building {
    spot: Vec3,
    height: usize,
    stage: Stage,
}

/// The handful of heights the physical toy can reach, written for minds.
pub fn height_words(height: usize) -> &'static str {
    match height {
        0 => "no",
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        _ => "five",
    }
}

fn placement_jitter(level: f32) -> f32 {
    1.5 - level
}

fn tower_height_range(level: f32) -> (usize, usize) {
    let minimum = 2 + (level * 2.0).round() as usize;
    (minimum, minimum + 1)
}

fn accident_chance(level: f32) -> f32 {
    0.12 * (1.0 - level).powi(2)
}

fn place(block: &mut Block, spot: Vec3, height: usize, level: f32, random: &mut super::Random) {
    let jitter = Vec3::new(
        random.range(-0.018, 0.018) * placement_jitter(level),
        0.0,
        random.range(-0.018, 0.018) * placement_jitter(level),
    );
    block.position = spot + Vec3::Y * ((height as f32 + 0.5) * EDGE) + jitter;
    block.orientation = Quat::from_rotation_y(random.range(-0.18, 0.18));
    block.velocity = Vec3::ZERO;
    block.spin = Vec3::ZERO;
    block.held = false;
    block.stacked = true;
}

impl World {
    pub fn tower_height(&self) -> usize {
        self.blocks.iter().filter(|block| block.stacked).count()
    }

    pub(super) fn scatter_blocks(&mut self) {
        for color in 0..5 {
            // Rejection sampling keeps the seeded scatter clear of furniture,
            // the ball, and other blocks, with a bounded search.
            for _ in 0..200 {
                let spot = Vec3::new(
                    self.random.range(-1.5, 1.5),
                    0.0,
                    self.random.range(-0.8, 0.9),
                );
                if self
                    .obstacles()
                    .iter()
                    .all(|(center, radius)| flat(spot - *center).length() > radius + RADIUS + 0.1)
                    && flat(spot - self.ball.position).length() > Ball::RADIUS + RADIUS + 0.15
                    && self
                        .blocks
                        .iter()
                        .all(|block| flat(spot - block.position).length() > RADIUS * 2.0 + 0.05)
                {
                    let mut block = Block::at(spot, color);
                    block.orientation = Quat::from_rotation_y(self.random.range(-3.0, 3.0));
                    self.blocks.push(block);
                    break;
                }
            }
        }
    }

    pub(super) fn advance_blocks(&mut self, seconds: f32) {
        let furniture = self.obstacles();
        // Once the ball and the loose blocks lie still, clear of the
        // furniture and the tower, nothing moves until something touches
        // them and sets them moving again.
        let mut solid = furniture.clone();
        solid.extend(self.tower.map(|spot| (spot, EDGE)));
        let clear = |position: Vec3, radius: f32| {
            solid
                .iter()
                .all(|(center, reach)| flat(position - *center).length() >= reach + radius)
        };
        if self.ball.velocity == Vec3::ZERO
            && self.ball.position.y <= Ball::RADIUS
            && clear(self.ball.position, Ball::RADIUS)
            && self.blocks.iter().all(|block| {
                block.settled() && (block.held || block.stacked || clear(block.position, RADIUS))
            })
        {
            return;
        }
        let mut remaining = seconds;
        while remaining > 0.0 {
            let slice = remaining.min(0.02);
            remaining -= slice;
            if let Some(spot) = self.tower {
                let motion = self.ball.velocity;
                let toward = flat(motion) * slice;
                let offset = flat(spot - self.ball.position);
                let fraction =
                    (offset.dot(toward) / toward.length_squared().max(1e-6)).clamp(0.0, 1.0);
                if !self.ball.held
                    && flat(motion).length() > 0.6
                    && self.ball.position.y < self.tower_height() as f32 * EDGE + Ball::RADIUS
                    && (offset - toward * fraction).length() < EDGE + Ball::RADIUS
                {
                    self.topple(motion);
                }
            }
            let mut ball_obstacles = furniture.clone();
            if let Some(spot) = self.tower {
                ball_obstacles.push((spot, EDGE));
            }
            ball_obstacles.extend(
                self.blocks
                    .iter()
                    .filter(|block| {
                        !block.held && !block.stacked && block.position.y < EDGE + Ball::RADIUS
                    })
                    .map(|block| (block.position, RADIUS)),
            );
            self.ball.roll(slice, &ball_obstacles);
            let mut block_obstacles = furniture.clone();
            if let Some(spot) = self.tower {
                block_obstacles.push((spot, EDGE));
            }
            for block in &mut self.blocks {
                block.step(slice, &block_obstacles);
            }
            // A few relaxation passes separate even a heap dropped in one place.
            for _ in 0..6 {
                for first in 0..self.blocks.len() {
                    for second in first + 1..self.blocks.len() {
                        let (before, after) = self.blocks.split_at_mut(second);
                        let first = &mut before[first];
                        let second = &mut after[0];
                        if first.held || second.held || first.stacked || second.stacked {
                            continue;
                        }
                        let offset = second.position - first.position;
                        let distance = offset.length();
                        if distance < RADIUS * 2.0 {
                            let normal = offset.normalize_or(Vec3::X);
                            // Separate sideways near the floor, so resting cubes
                            // do not float on one another's collision spheres.
                            let normal = if normal.y.abs() > 0.8 {
                                Vec3::X
                            } else {
                                Vec3::new(normal.x, 0.0, normal.z).normalize_or(Vec3::X)
                            };
                            let reach = ((RADIUS * 2.0).powi(2) - offset.y.powi(2)).max(0.0).sqrt();
                            let overlap = (reach - flat(offset).length()).max(0.0);
                            first.position -= normal * overlap * 0.5;
                            second.position += normal * overlap * 0.5;
                            let toward = (second.velocity - first.velocity).dot(normal);
                            if toward < 0.0 {
                                first.velocity += normal * toward * 0.65;
                                second.velocity -= normal * toward * 0.65;
                            }
                            first.contain();
                            second.contain();
                        }
                    }
                }
            }
        }
    }

    /// Releases the standing tower in a growing spray from bottom to top.
    /// Callers record who knocked it down when that is known.
    pub fn topple(&mut self, direction: Vec3) {
        let Some(spot) = self.tower.take() else {
            return;
        };
        let along = Vec3::new(direction.x, 0.0, direction.z).normalize_or(Vec3::Z);
        for block in &mut self.blocks {
            if !block.stacked {
                continue;
            }
            let height = (block.position.y / EDGE).max(0.5);
            block.stacked = false;
            block.velocity = along * (0.35 + height * 0.28)
                + Vec3::new(
                    self.random.range(-0.25, 0.25),
                    0.5 + height * 0.12,
                    self.random.range(-0.25, 0.25),
                );
            block.spin = Vec3::new(
                self.random.range(-5.0, 5.0),
                self.random.range(-3.0, 3.0),
                self.random.range(-5.0, 5.0),
            );
        }
        if self.ghost.plan.activity == Activity::Build && self.ghost.plan.count > 0 {
            self.drop_block();
            self.ghost.plan.done = true;
        }
        self.ghost.look = Some(spot + Vec3::Y * EDGE);
    }

    /// A click shows what the viewer means by building, or demolishes a tower.
    pub fn nudge_block(&mut self, index: usize, direction: Vec3) {
        let Some(block) = self.blocks.get(index) else {
            return;
        };
        if block.stacked {
            self.topple(direction);
            self.record(EventKind::Toppled { by_viewer: true });
            let delighted =
                self.life.nature.unwrap_or_default().playful >= 0.5 || self.life.bond >= 0.55;
            let lines = if delighted {
                [
                    ("Again!", "또 하자!"),
                    ("Whoosh! Again!", "와르르! 또 하자!"),
                    ("Hee hee!", "히히!"),
                ]
            } else {
                [
                    ("My tower!", "내 탑!"),
                    ("Oh… my tower.", "아… 내 탑."),
                    ("I made that!", "내가 만든 건데!"),
                ]
            };
            let index = self.random.range(0.0, lines.len() as f32) as usize;
            self.suggest_mood(
                if delighted {
                    Mood::Excited
                } else {
                    Mood::Grumpy
                },
                if delighted { 5.0 } else { 8.0 },
            );
            self.say(self.spoken(lines[index].0, lines[index].1));
        } else if !block.held {
            let along = Vec3::new(direction.x, 0.0, direction.z).normalize_or(Vec3::Z);
            let block = &mut self.blocks[index];
            block.velocity += along * 0.85 + Vec3::Y * 0.18;
            block.spin += Vec3::Y.cross(along) * 3.0;
        }
        self.shown(Activity::Build);
    }

    pub(super) fn drop_block(&mut self) {
        for block in &mut self.blocks {
            block.held = false;
        }
    }

    pub(super) fn follow_block(&mut self) {
        for block in &mut self.blocks {
            if block.held {
                block.position = (self.ghost.position - Vec3::Y * CARRY_HEIGHT).clamp(
                    ROOM_MIN + Vec3::new(RADIUS, EDGE * 0.5, RADIUS),
                    ROOM_MAX - Vec3::splat(RADIUS),
                );
                block.velocity = self.ghost.velocity;
            }
        }
    }

    pub(super) fn begin_build(&mut self) {
        let Some(spot) = self.tower.or_else(|| self.tower_spot()) else {
            self.ghost.plan.done = true;
            return;
        };
        let available = self
            .blocks
            .iter()
            .filter(|block| block.on_floor() || block.stacked)
            .count();
        if available < 2 {
            self.ghost.plan.done = true;
            return;
        }
        let needs = self.ghost.needs;
        let (minimum, maximum) = tower_height_range(self.life.skills.level(super::Skill::Stacking));
        let height = (minimum
            + ((needs.energy + 1.0 - needs.fun) * self.random.unit() * 1.5) as usize)
            .min(maximum)
            .min(available)
            .max((self.tower_height() + 1).min(available));
        self.building = Some(Building {
            spot,
            height,
            stage: Stage::Gather,
        });
        self.ghost.plan.anchor = spot;
        self.ghost.plan.count = self.tower_height() as u32;
    }

    fn tower_spots(&self) -> Vec<Vec3> {
        // Front spots come first, with enough space for an admiring retreat
        // and for the blocks to scatter when the tower falls.
        let mut spots = Vec::new();
        for depth in [0.55, 0.25, -0.1, -0.45] {
            for across in [-0.65, 0.0, 0.65, 1.2] {
                let spot = Vec3::new(across, 0.0, depth);
                if self
                    .obstacles()
                    .iter()
                    .all(|(center, radius)| flat(spot - *center).length() > radius + 0.65)
                    && flat(spot - self.ball.position).length() > Ball::RADIUS + 0.55
                {
                    spots.push(spot);
                }
            }
            if !spots.is_empty() {
                break;
            }
        }
        spots
    }

    fn tower_spot(&mut self) -> Option<Vec3> {
        let spots = self.tower_spots();
        self.random
            .pick(&vec![1.0; spots.len()])
            .map(|index| spots[index])
    }

    pub(super) fn away_blocks(&mut self, standing: bool, random: &mut super::Random) {
        let spots = self.tower_spots();
        let base = random
            .pick(&vec![1.0; spots.len()])
            .map(|index| spots[index]);
        let level = self.life.skills.level(super::Skill::Stacking);
        let (minimum, maximum) = tower_height_range(level);
        let height = (minimum + (random.unit() * (maximum - minimum + 1) as f32) as usize)
            .min(self.blocks.len());
        self.tower = base.filter(|_| standing);
        let obstacles = self.obstacles();
        let mut blocks = Vec::new();
        if let Some(base) = self.tower {
            for index in 0..height {
                let mut block = self.blocks[index];
                place(&mut block, base, index, level, random);
                blocks.push(block);
            }
        }
        for index in blocks.len()..self.blocks.len() {
            let clear = |spot: Vec3| {
                [0, 2].into_iter().all(|axis| {
                    spot[axis] >= ROOM_MIN[axis] + RADIUS && spot[axis] <= ROOM_MAX[axis] - RADIUS
                }) && obstacles
                    .iter()
                    .all(|(center, radius)| flat(spot - *center).length() > radius + RADIUS + 0.05)
                    && flat(spot - self.ball.position).length() > Ball::RADIUS + RADIUS + 0.05
                    && blocks
                        .iter()
                        .all(|block| flat(spot - block.position).length() > RADIUS * 2.0 + 0.02)
                    && self
                        .tower
                        .is_none_or(|base| flat(spot - base).length() > EDGE + RADIUS + 0.05)
            };
            let spot = (0..200)
                .map(|_| {
                    if let Some(base) = base.filter(|_| !standing) {
                        base + Vec3::new(random.range(-0.7, 0.7), 0.0, random.range(-0.6, 0.6))
                    } else {
                        Vec3::new(random.range(-1.5, 1.5), 0.0, random.range(-0.8, 0.9))
                    }
                })
                .find(|spot| clear(*spot))
                .or_else(|| {
                    // A fixed fallback grid guarantees space even near furniture.
                    (0..10)
                        .flat_map(|across| {
                            (0..7).map(move |depth| {
                                Vec3::new(
                                    -1.6 + across as f32 * 0.35,
                                    0.0,
                                    -1.05 + depth as f32 * 0.35,
                                )
                            })
                        })
                        .find(|spot| clear(*spot))
                });
            if let Some(spot) = spot {
                let mut block = Block::at(spot, self.blocks[index].color);
                block.orientation = Quat::from_rotation_y(random.range(-3.0, 3.0));
                blocks.push(block);
            }
        }
        self.blocks = blocks;
    }

    pub(super) fn away_ball(&mut self, random: &mut super::Random) {
        let obstacles = self.obstacles();
        for _ in 0..200 {
            let ball = Ball::at(random.range(-1.5, 1.5), random.range(-0.8, 0.9));
            if obstacles.iter().all(|(center, radius)| {
                flat(ball.position - *center).length() > radius + Ball::RADIUS
            }) && self
                .blocks
                .iter()
                .all(|block| flat(ball.position - block.position).length() > RADIUS + Ball::RADIUS)
            {
                self.ball = ball;
                break;
            }
        }
    }

    pub(super) fn placing_block(&self) -> bool {
        self.ghost.plan.activity == Activity::Build
            && !self.ghost.plan.done
            && self
                .building
                .is_some_and(|building| matches!(building.stage, Stage::Carry | Stage::Swoop))
    }

    pub(super) fn build(&mut self) -> Option<Vec3> {
        let mut building = self.building?;
        let height = self.tower_height();
        let look = building.spot + Vec3::Y * (height as f32 * EDGE * 0.5);
        if height == 0 && self.ghost.plan.count > 0 {
            self.drop_block();
            self.ghost.plan.done = true;
            return Some(look);
        }
        match building.stage {
            Stage::Gather => {
                if height >= building.height {
                    building.stage = Stage::Admire;
                    self.ghost.plan.arrived = None;
                    self.ghost.plan.target = building.spot
                        + Vec3::new(0.5, (height as f32 * EDGE + 0.2).max(0.8), -0.55);
                } else if let Some(index) = self
                    .blocks
                    .iter()
                    .enumerate()
                    .filter(|(_, block)| block.on_floor())
                    .min_by(|(_, first), (_, second)| {
                        first
                            .position
                            .distance_squared(self.ghost.position)
                            .total_cmp(&second.position.distance_squared(self.ghost.position))
                    })
                    .map(|(index, _)| index)
                {
                    let block = &mut self.blocks[index];
                    let target = block.position + Vec3::Y * CARRY_HEIGHT;
                    self.ghost.plan.target = target;
                    if self.ghost.position.distance(target) < ARRIVED {
                        block.held = true;
                        block.spin = Vec3::ZERO;
                        building.stage = Stage::Carry;
                    }
                    self.building = Some(building);
                    return Some(block.position);
                } else {
                    self.ghost.plan.done = true;
                }
            }
            Stage::Carry => {
                let target =
                    building.spot + Vec3::Y * ((height as f32 + 0.5) * EDGE + CARRY_HEIGHT);
                self.ghost.plan.target = target;
                if let Some(index) = self.blocks.iter().position(|block| block.held) {
                    if self.ghost.position.distance(target) < ARRIVED {
                        let level = self.life.skills.level(super::Skill::Stacking);
                        place(
                            &mut self.blocks[index],
                            building.spot,
                            height,
                            level,
                            &mut self.random,
                        );
                        self.tower = Some(building.spot);
                        self.ghost.plan.count = height as u32 + 1;
                        building.stage = Stage::Gather;
                        if height > 0 && self.random.unit() < accident_chance(level) {
                            self.topple(self.ghost.velocity);
                            self.record(EventKind::Toppled { by_viewer: false });
                            let playful = self.life.nature.unwrap_or_default().playful >= 0.5;
                            self.suggest_mood(
                                if playful {
                                    Mood::Cheerful
                                } else {
                                    Mood::Grumpy
                                },
                                4.0,
                            );
                            self.say(if playful {
                                "Oops! Hee hee."
                            } else {
                                "Oh… my tower."
                            });
                            self.ghost.plan.done = true;
                        }
                    }
                } else {
                    self.ghost.plan.done = true;
                }
            }
            Stage::Admire => {
                if self.ghost.plan.stayed(self.time) >= 3.0 {
                    self.record(EventKind::Built {
                        height: height as u32,
                    });
                    self.ghost.needs.change(-0.003, 0.2, 0.0);
                    if !(self.viewer_present() && self.recent_negative_feeling(1800.0).is_some())
                        && self.random.unit()
                            < 0.25 + 0.5 * self.life.nature.unwrap_or_default().playful
                    {
                        building.stage = Stage::Swoop;
                        self.ghost.plan.target = building.spot + Vec3::Y * 0.5;
                    } else {
                        self.ghost.plan.done = true;
                    }
                }
            }
            Stage::Swoop => {
                if self.ghost.position.distance(self.ghost.plan.target) < ARRIVED + EDGE {
                    self.topple(self.ghost.velocity);
                    self.record(EventKind::Toppled { by_viewer: false });
                    self.ghost.needs.change(0.0, 0.12, 0.0);
                    self.ghost.plan.done = true;
                }
            }
        }
        self.building = Some(building);
        Some(look)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Nature;

    fn tower(world: &mut World, height: usize) {
        let spot = Vec3::new(0.0, 0.0, 0.3);
        world.tower = Some(spot);
        for (level, block) in world.blocks.iter_mut().take(height).enumerate() {
            block.position = spot + Vec3::Y * ((level as f32 + 0.5) * EDGE);
            block.orientation = Quat::IDENTITY;
            block.stacked = true;
        }
    }

    fn assert_settled(world: &World) {
        for (index, block) in world.blocks.iter().enumerate() {
            assert_eq!(block.velocity, Vec3::ZERO, "{index}: {block:?}");
            assert_eq!(block.spin, Vec3::ZERO, "{index}: {block:?}");
            assert!(
                (block.position.y - EDGE * 0.5).abs() < 0.001,
                "{index}: {block:?}"
            );
            let up = (block.orientation.conjugate() * Vec3::Y)
                .abs()
                .max_element();
            assert!(up > 0.9999, "{index}: {block:?}");
            for axis in [0, 2] {
                assert!(block.position[axis] >= ROOM_MIN[axis] + RADIUS - 1e-5);
                assert!(block.position[axis] <= ROOM_MAX[axis] - RADIUS + 1e-5);
            }
            for other in &world.blocks[index + 1..] {
                assert!(
                    block.position.distance(other.position) >= RADIUS * 2.0 - 0.001,
                    "{block:?} {other:?}"
                );
            }
            for (center, radius) in world.obstacles() {
                assert!(
                    flat(block.position - center).length() >= radius + RADIUS - 0.001,
                    "{block:?} {center:?}"
                );
            }
        }
    }

    #[test]
    fn practice_reduces_jitter_and_accidents_and_raises_planned_towers() {
        assert_eq!(placement_jitter(0.0), 1.5);
        assert_eq!(placement_jitter(1.0), 0.5);
        assert_eq!(tower_height_range(0.0), (2, 3));
        assert_eq!(tower_height_range(0.5), (3, 4));
        assert_eq!(tower_height_range(1.0), (4, 5));
        assert_eq!(accident_chance(0.0), 0.12);
        assert_eq!(accident_chance(1.0), 0.0);
        assert!((accident_chance(0.5) - 0.03).abs() < 1e-6);
    }

    #[test]
    fn a_clumsy_placement_can_topple_the_tower_and_nature_colors_its_reaction() {
        for playful in [0.0, 1.0] {
            let mut accident = false;
            for seed in 0..80 {
                let mut world = World::new(seed, 12.0);
                world.life.nature.as_mut().unwrap().playful = playful;
                tower(&mut world, 1);
                world.begin(Activity::Build);
                let mut building = world.building.unwrap();
                building.stage = Stage::Carry;
                world.building = Some(building);
                world.blocks[1].held = true;
                world.ghost.position = building.spot + Vec3::Y * (1.5 * EDGE + CARRY_HEIGHT);
                world.build();
                if world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::Toppled { by_viewer: false })
                {
                    assert!(world.ghost.plan.done);
                    assert!(world.tower.is_none());
                    assert!(!world.blocks.iter().any(|block| block.held));
                    assert_eq!(
                        world.ghost.suggested_mood.unwrap().0,
                        if playful == 0.0 {
                            Mood::Grumpy
                        } else {
                            Mood::Cheerful
                        }
                    );
                    accident = true;
                    break;
                }
            }
            assert!(accident);
        }
    }

    #[test]
    fn toppled_blocks_tumble_and_settle_on_faces_without_overlapping_or_leaving_the_room() {
        for seed in 0..20 {
            for step in [0.01, 0.05, 0.1] {
                let mut world = World::new(seed, 12.0);
                tower(&mut world, 5);
                world.topple(Vec3::new(2.0, 1.0, -0.6));
                assert!(world.tower.is_none());
                assert!(
                    world
                        .blocks
                        .iter()
                        .all(|block| !block.stacked && block.spin.length() > 0.0)
                );
                for _ in 0..(15.0 / step) as usize {
                    world.advance_blocks(step);
                }
                assert_settled(&world);
            }
        }
    }

    #[test]
    fn a_seeded_scatter_is_clear_and_repeats_with_all_five_blocks() {
        for seed in 0..100 {
            let world = World::new(seed, 12.0);
            assert_eq!(world.blocks.len(), 5);
            assert_eq!(world.blocks, World::new(seed, 12.0).blocks);
            assert_settled(&world);
            for block in &world.blocks {
                assert!(
                    flat(block.position - world.ball.position).length() > RADIUS + Ball::RADIUS
                );
            }
        }
    }

    #[test]
    fn building_collects_the_planned_height_admires_it_and_finishes_without_teleporting() {
        let mut left_standing = 0;
        let mut knocked_down = 0;
        for seed in 0..40 {
            let mut world = World::new(seed, 12.0);
            world.ghost.needs.energy = 1.0;
            world.ghost.needs.fun = 0.0;
            world.life.skills.stacking = 1_000_000.0;
            world.begin(Activity::Build);
            let building = world.building.unwrap();
            assert!((3..=5).contains(&building.height));
            let mut carried = false;
            let mut admired = None;
            for _ in 0..1200 {
                let before = world.ghost.position;
                world.step(0.05);
                assert!(before.distance(world.ghost.position) < 0.2);
                carried |= world.blocks.iter().any(|block| block.held);
                if world.building.unwrap().stage == Stage::Admire {
                    let height = world.tower_height();
                    assert_eq!(height, building.height);
                    let mut stacked: Vec<_> =
                        world.blocks.iter().filter(|block| block.stacked).collect();
                    stacked.sort_by(|first, second| first.position.y.total_cmp(&second.position.y));
                    for (level, block) in stacked.iter().enumerate() {
                        assert!((block.position.y - (level as f32 + 0.5) * EDGE).abs() < 1e-5);
                        assert!(flat(block.position - building.spot).length() < 0.03);
                    }
                    if world.ghost.plan.arrived.is_some() {
                        admired.get_or_insert(world.time);
                    }
                }
                if world.needs_decision() {
                    break;
                }
            }
            assert!(carried);
            assert!(world.needs_decision(), "seed {seed}");
            let built = world
                .events
                .iter()
                .find(|event| matches!(event.kind, EventKind::Built { .. }))
                .unwrap_or_else(|| {
                    panic!(
                        "seed {seed}, plan {:?}, building {:?}, tower {}",
                        world.ghost.plan,
                        world.building,
                        world.tower_height()
                    )
                });
            assert_eq!(
                built.kind,
                EventKind::Built {
                    height: building.height as u32
                }
            );
            assert!(built.time - admired.unwrap() >= 2.9);
            assert_eq!(world.life.today.towers, 1);
            assert_eq!(world.ghost.mood, Mood::Excited);
            assert!(!world.blocks.iter().any(|block| block.held));
            if world.tower.is_some() {
                left_standing += 1;
            } else {
                knocked_down += 1;
                assert!(
                    world
                        .events
                        .iter()
                        .any(|event| event.kind == EventKind::Toppled { by_viewer: false })
                );
            }
        }
        assert!(left_standing > 0 && knocked_down > 0);
    }

    #[test]
    fn a_partial_tower_keeps_its_spot_and_can_be_completed() {
        let mut world = World::new(2, 12.0);
        tower(&mut world, 2);
        let spot = world.tower;
        world.begin(Activity::Build);
        assert_eq!(Some(world.building.unwrap().spot), spot);
        for _ in 0..1200 {
            world.step(0.05);
            if world.needs_decision() {
                break;
            }
        }
        assert_eq!(world.life.today.towers, 1);
        assert!(world.life.today.tallest >= 3);
    }

    #[test]
    fn a_fast_ball_topples_the_tower_and_a_slow_or_carried_ball_does_not() {
        for (speed, held, should_fall) in
            [(2.0, false, true), (0.5, false, false), (2.0, true, false)]
        {
            let mut world = World::new(1, 12.0);
            tower(&mut world, 5);
            world.ball = Ball::at(-0.36, 0.3);
            world.ball.velocity = Vec3::X * speed;
            world.ball.held = held;
            world.advance_blocks(0.1);
            assert_eq!(world.tower.is_none(), should_fall);
            if !held {
                assert!(world.ball.velocity.x < 0.0);
            }
        }
    }

    #[test]
    fn the_ball_bounces_off_a_loose_block() {
        let mut world = World::new(1, 12.0);
        world.blocks = vec![Block::at(Vec3::ZERO, 0)];
        world.ball = Ball::at(-0.35, 0.0);
        world.ball.velocity = Vec3::X * 2.0;
        world.advance_blocks(0.1);
        assert!(world.ball.velocity.x < 0.0);
        assert!(
            flat(world.ball.position - world.blocks[0].position).length()
                >= Ball::RADIUS + RADIUS - 0.001
        );
    }

    #[test]
    fn a_click_topples_a_tower_and_nature_or_bond_decides_the_reaction() {
        for (playful, bond, mood) in [
            (0.8, 0.0, Mood::Excited),
            (0.2, 0.7, Mood::Excited),
            (0.2, 0.0, Mood::Grumpy),
        ] {
            let mut world = World::new(1, 12.0);
            world.life.nature = Some(Nature {
                playful,
                ..Nature::default()
            });
            world.life.bond = bond;
            tower(&mut world, 5);
            world.nudge_block(2, Vec3::X);
            assert!(world.tower.is_none());
            assert_eq!(world.life.today.toppled, 1);
            assert!(
                world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::Toppled { by_viewer: true })
            );
            assert_eq!(world.ghost.suggested_mood.unwrap().0, mood);
            assert!(world.ghost.speech.is_some());
            world.step(0.1);
            assert_eq!(world.ghost.mood, mood);
        }
    }

    #[test]
    fn interrupting_or_giving_up_on_building_releases_the_carried_block_to_land() {
        for interruption in 0..4 {
            let mut world = World::new(1, 12.0);
            world.begin(Activity::Build);
            for _ in 0..600 {
                world.step(0.05);
                if world.blocks.iter().any(|block| block.held) {
                    break;
                }
            }
            let index = world.blocks.iter().position(|block| block.held).unwrap();
            match interruption {
                0 => world.begin(Activity::Float),
                1 => {
                    world.poke();
                }
                2 => {
                    world.hear("no");
                }
                _ => {
                    world.ghost.plan.started = world.time - 61.0;
                    world.step(0.05);
                }
            }
            assert!(!world.blocks[index].held);
            for _ in 0..200 {
                world.advance_blocks(0.05);
            }
            assert!((world.blocks[index].position.y - EDGE * 0.5).abs() < 0.001);
        }
    }

    #[test]
    fn building_requires_energy_wakefulness_and_two_loose_blocks() {
        let mut world = World::new(1, 12.0);
        assert!(world.options().contains(&Activity::Build));
        world.ghost.needs.energy = 0.3;
        assert!(!world.options().contains(&Activity::Build));
        world.ghost.needs.energy = 0.8;
        world.ghost.asleep = 0.3;
        assert!(!world.options().contains(&Activity::Build));
        world.ghost.asleep = 0.0;
        tower(&mut world, 4);
        assert!(!world.options().contains(&Activity::Build));
        world.blocks[3] = Block::at(Vec3::new(0.9, 0.0, -0.4), 3);
        assert!(world.options().contains(&Activity::Build));
        tower(&mut world, 5);
        assert!(!world.options().contains(&Activity::Build));
    }

    #[test]
    fn block_motion_and_building_repeat_for_the_same_seed() {
        let mut first = World::new(7, 12.0);
        let mut second = first.clone();
        for world in [&mut first, &mut second] {
            world.begin(Activity::Build);
        }
        for _ in 0..1200 {
            first.step(0.05);
            second.step(0.05);
            assert_eq!(first.blocks, second.blocks);
            assert_eq!(first.tower, second.tower);
            assert_eq!(first.ghost.plan, second.ghost.plan);
            assert_eq!(first.events, second.events);
        }
    }
    #[test]
    fn fast_tumbling_blocks_bounce_off_walls_and_furniture_and_a_heap_separates() {
        for step in [0.01, 0.1] {
            let mut world = World::new(3, 12.0);
            for block in &mut world.blocks {
                block.position = Vec3::new(-0.5, 0.7, 0.4);
                block.velocity = Vec3::new(7.0, 2.0, -5.0);
                block.spin = Vec3::new(8.0, -4.0, 6.0);
            }
            for _ in 0..(15.0 / step) as usize {
                world.advance_blocks(step);
            }
            assert_settled(&world);
        }
    }

    #[test]
    fn a_tower_spot_leaves_clearance_from_walls_furniture_and_the_ball() {
        for ball_spot in [
            Vec3::ZERO,
            Vec3::new(0.7, 0.0, 0.45),
            Vec3::new(-0.7, 0.0, 0.6),
        ] {
            let mut world = World::new(1, 12.0);
            world.date = jiff::civil::Date::constant(2026, 12, 24);
            world.ball.position = ball_spot + Vec3::Y * Ball::RADIUS;
            let spot = world.tower_spot().unwrap();
            assert!(spot.z > 0.0);
            for axis in [0, 2] {
                assert!(spot[axis] - ROOM_MIN[axis] > 0.65);
                assert!(ROOM_MAX[axis] - spot[axis] > 0.65);
            }
            assert!(flat(spot - world.ball.position).length() > Ball::RADIUS + 0.55);
            assert!(
                world
                    .obstacles()
                    .iter()
                    .all(|(center, radius)| flat(spot - *center).length() > radius + 0.65)
            );
        }
    }
    #[test]
    fn a_first_tower_five_high_announces_both_milestones() {
        let mut world = World::new(1, 12.0);
        world.record(EventKind::Built { height: 5 });
        let milestones: Vec<_> = world
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                EventKind::Milestone(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            milestones,
            [
                "I built my first tower of blocks.",
                "I built a tower five blocks high."
            ]
        );
    }
}
