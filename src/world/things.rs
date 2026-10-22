use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// The inside of the glass box: the floor is at y = 0 and the glass front
/// faces +z, toward the viewer.
pub const ROOM_MIN: Vec3 = Vec3::new(-2.0, 0.0, -1.4);
pub const ROOM_MAX: Vec3 = Vec3::new(2.0, 2.4, 1.4);
/// The middle of the window in the back wall.
pub const WINDOW_CENTER: Vec3 = Vec3::new(0.0, 1.5, ROOM_MIN.z);
/// The keepsake shelf on the back wall, left of the window and above the lamp.
pub const SHELF_POSITION: Vec3 = Vec3::new(-1.35, 1.87, -1.27);
/// Where the bird leaves gifts on the outer sill, just beyond the pane.
pub const SILL_POSITION: Vec3 = Vec3::new(0.42, 1.06, ROOM_MIN.z - 0.02);

/// A floor lamp whose bulb lights the room when it is on.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lamp {
    pub position: Vec3,
    pub on: bool,
}

impl Lamp {
    pub const HEIGHT: f32 = 1.35;
    pub const BASE_RADIUS: f32 = 0.18;

    pub fn bulb(&self) -> Vec3 {
        self.position + Vec3::Y * Self::HEIGHT
    }
}

/// A ball that rolls on the floor when nudged, or rides in the ghost's arms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ball {
    pub position: Vec3,
    pub velocity: Vec3,
    pub held: bool,
}

impl Ball {
    pub const RADIUS: f32 = 0.14;
    /// How fast a dropped ball falls, in units per second squared.
    const GRAVITY: f32 = 9.0;

    pub fn at(x: f32, z: f32) -> Self {
        Self {
            position: Vec3::new(x, Self::RADIUS, z),
            velocity: Vec3::ZERO,
            held: false,
        }
    }

    /// Rolls the ball for `seconds`, dropping it to the floor if it was let
    /// go above it, slowing it with friction, and bouncing it off the floor,
    /// the walls, and `obstacles`, given as floor circles of center and
    /// radius. Returns the speed of the hardest bounce, or 0.
    pub fn roll(&mut self, seconds: f32, obstacles: &[(Vec3, f32)]) -> f32 {
        if self.held {
            return 0.0;
        }
        let mut bounce: f32 = 0.0;
        let grounded = self.position.y <= Self::RADIUS && self.velocity.y <= 0.0;
        if grounded {
            self.velocity.y = 0.0;
        } else {
            self.velocity.y -= Self::GRAVITY * seconds;
        }
        self.position += self.velocity * seconds;
        if self.position.y <= Self::RADIUS {
            self.position.y = Self::RADIUS;
            // A bounce must outrun what one step of gravity adds, or the
            // ball would hop forever on long steps.
            if self.velocity.y < -0.6 - Self::GRAVITY * seconds {
                bounce = bounce.max(-self.velocity.y);
                self.velocity.y *= -0.45;
            } else {
                self.velocity.y = 0.0;
            }
        }
        let friction = (-0.9 * seconds).exp();
        self.velocity.x *= friction;
        self.velocity.z *= friction;
        if self.velocity.length_squared() < 1e-4 && self.position.y <= Self::RADIUS {
            self.velocity = Vec3::ZERO;
        }

        for axis in [0, 2] {
            let low = ROOM_MIN[axis] + Self::RADIUS;
            let high = ROOM_MAX[axis] - Self::RADIUS;
            if self.position[axis] < low && self.velocity[axis] < 0.0 {
                bounce = bounce.max(-self.velocity[axis]);
                self.position[axis] = low;
                self.velocity[axis] *= -0.7;
            } else if self.position[axis] > high && self.velocity[axis] > 0.0 {
                bounce = bounce.max(self.velocity[axis]);
                self.position[axis] = high;
                self.velocity[axis] *= -0.7;
            }
        }

        for (center, radius) in obstacles {
            let offset = flat(self.position - *center);
            let reach = radius + Self::RADIUS;
            let distance = offset.length();
            if distance < reach && distance > 1e-5 {
                let normal = Vec3::new(offset.x / distance, 0.0, offset.y / distance);
                self.position += normal * (reach - distance);
                let toward = self.velocity.dot(normal);
                if toward < 0.0 {
                    bounce = bounce.max(-toward);
                    self.velocity -= normal * toward * 1.7;
                }
            }
        }
        // An obstacle can push a ball lying against a wall into it; the
        // wall holds it in the room.
        for axis in [0, 2] {
            let low = ROOM_MIN[axis] + Self::RADIUS;
            let high = ROOM_MAX[axis] - Self::RADIUS;
            if self.position[axis] < low {
                self.position[axis] = low;
                self.velocity[axis] = self.velocity[axis].max(0.0);
            } else if self.position[axis] > high {
                self.position[axis] = high;
                self.velocity[axis] = self.velocity[axis].min(0.0);
            }
        }
        bounce
    }
}

/// A potted plant that grows a little each time it is tended, and gets
/// thirsty as time passes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Plant {
    pub position: Vec3,
    /// From a sprout at 0 to grown at 1; it flowers from `BLOOM`.
    pub growth: f32,
    /// From just watered at 0 to wilting at 1.
    pub thirst: f32,
    #[serde(default)]
    pub blossom: f32,
    /// Seconds into a spell; negative while resting between spells.
    #[serde(default)]
    pub bloom_age: f32,
    #[serde(default)]
    pub seeds: u32,
    #[serde(default)]
    pub droop: f32,
}

impl Plant {
    pub const POT_RADIUS: f32 = 0.2;
    pub const POT_HEIGHT: f32 = 0.26;
    pub const BLOOM: f32 = 0.8;
    /// Seconds for a watered plant to grow fully thirsty: a few waterings
    /// a day keep it well.
    pub const THIRST_TIME: f32 = 10_800.0;

    pub fn height(&self) -> f32 {
        Self::POT_HEIGHT + 0.2 + 0.6 * self.growth
    }

    pub fn top(&self) -> Vec3 {
        self.position + Vec3::new(0.24 * self.droop, self.height() - 0.28 * self.droop, 0.0)
    }

    pub fn blooming(&self) -> bool {
        self.blossom > 0.5
    }
}

/// A cushion on the floor where the ghost sleeps.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nook {
    pub position: Vec3,
}

impl Nook {
    pub const RADIUS: f32 = 0.42;
    pub const HEIGHT: f32 = 0.14;

    /// Where the ghost floats while it sleeps.
    pub fn bed(&self, size: f32) -> Vec3 {
        self.position + Vec3::Y * (Self::HEIGHT + 0.38 * size)
    }
}

/// A drifting speck of light the ghost can catch and eat for energy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mote {
    pub id: u32,
    pub position: Vec3,
    pub born: f64,
    /// Sets this mote's path apart from the others'.
    pub seed: f32,
}

impl Mote {
    pub const LIFETIME: f64 = 150.0;

    /// Drifts the mote along its wandering path, keeping it in the room.
    pub fn drift(&mut self, time: f64, seconds: f32, breeze: Vec3) {
        let phase = time as f32 + self.seed * 10.0;
        let velocity = Vec3::new(
            (phase * 0.7).sin(),
            -0.25 + 0.6 * (phase * 0.45 + self.seed).sin(),
            (phase * 0.6 + self.seed * 2.0).cos(),
        ) * 0.14;
        let outdoors = !super::home::in_house(self.position);
        self.position += (velocity + breeze) * seconds;
        if outdoors {
            let (low, high) = super::home::yard_bounds();
            let moved = self.position.clamp(low + Vec3::Y * 0.3, high);
            // Out on the lawn it drifts about but never in through a wall.
            if super::home::in_house(moved) {
                self.position = (self.position - velocity * seconds).clamp(low, high);
            } else {
                self.position = moved;
            }
            return;
        }
        let margin = Vec3::new(0.25, 0.3, 0.25);
        self.position = self
            .position
            .clamp(ROOM_MIN + margin, ROOM_MAX - Vec3::new(0.25, 0.25, 0.25));
    }

    /// How brightly it glows, fading in when it appears and out before it
    /// is gone, with a twinkle.
    pub fn glow(&self, time: f64) -> f32 {
        let age = (time - self.born) as f32;
        let left = (Self::LIFETIME as f32 - age).max(0.0);
        let fade = (age / 2.0).min(1.0) * (left / 12.0).min(1.0);
        fade * (0.8 + 0.2 * (time as f32 * 5.0 + self.seed * 20.0).sin())
    }
}

/// A point's position on the floor plane.
pub fn flat(point: Vec3) -> Vec2 {
    Vec2::new(point.x, point.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_obstacle_pushing_a_ball_at_the_wall_leaves_it_in_the_room() {
        for (wall, inward) in [(ROOM_MAX.x, -1.0), (ROOM_MIN.x, 1.0)] {
            let mut ball = Ball::at(wall + inward * Ball::RADIUS, 0.0);
            let block = Vec3::new(wall + inward * (Ball::RADIUS + 0.1), 0.09, 0.0);
            ball.roll(0.1, &[(block, 0.16)]);
            assert!(
                (ROOM_MIN.x + Ball::RADIUS..=ROOM_MAX.x - Ball::RADIUS).contains(&ball.position.x)
            );
            assert!(ball.velocity.x * inward >= 0.0);
        }
    }

    #[test]
    fn a_rolling_ball_stays_in_the_room_and_comes_to_rest() {
        let mut ball = Ball::at(0.0, 0.0);
        ball.velocity = Vec3::new(5.0, 0.0, -3.0);
        let mut bounced = false;
        for _ in 0..3000 {
            bounced |= ball.roll(0.01, &[]) > 0.0;
            assert!(ball.position.x.abs() <= ROOM_MAX.x - Ball::RADIUS + 1e-4);
            assert!(ball.position.z.abs() <= ROOM_MAX.z - Ball::RADIUS + 1e-4);
        }
        assert!(bounced);
        assert_eq!(ball.velocity, Vec3::ZERO);
    }

    #[test]
    fn the_ball_bounces_off_an_obstacle_instead_of_passing_through() {
        let mut ball = Ball::at(-1.0, 0.0);
        ball.velocity = Vec3::new(2.0, 0.0, 0.0);
        let post = (Vec3::new(0.0, 0.0, 0.0), 0.3);
        for _ in 0..200 {
            ball.roll(0.01, &[post]);
            assert!(flat(ball.position - post.0).length() >= 0.3 + Ball::RADIUS - 1e-4);
        }
        assert!(ball.velocity.x <= 0.0);
    }

    #[test]
    fn a_ball_let_go_in_the_air_falls_bounces_and_settles_on_the_floor() {
        let mut ball = Ball::at(0.0, 0.0);
        ball.position.y = 0.8;
        let mut bounced = false;
        let mut lowest = f32::INFINITY;
        for _ in 0..300 {
            bounced |= ball.roll(0.01, &[]) > 0.0;
            lowest = lowest.min(ball.position.y);
        }
        assert!(bounced);
        assert_eq!(lowest, Ball::RADIUS);
        assert_eq!(ball.position.y, Ball::RADIUS);
        assert_eq!(ball.velocity, Vec3::ZERO);
    }

    #[test]
    fn a_dropped_ball_settles_even_with_long_steps() {
        for step in [0.01, 0.05, 0.1] {
            let mut ball = Ball::at(0.0, 0.0);
            ball.position.y = 0.18;
            ball.velocity.y = 0.25;
            for _ in 0..(5.0 / step) as usize {
                ball.roll(step, &[]);
            }
            assert_eq!(ball.position.y, Ball::RADIUS, "{step}");
            assert_eq!(ball.velocity, Vec3::ZERO, "{step}");
        }
    }

    #[test]
    fn a_held_ball_does_not_roll_or_fall() {
        let mut ball = Ball::at(0.0, 0.0);
        ball.held = true;
        ball.position.y = 0.8;
        ball.velocity = Vec3::new(0.5, 0.1, -0.3);
        let before = ball;
        assert_eq!(ball.roll(0.1, &[]), 0.0);
        assert_eq!(ball, before);
    }

    #[test]
    fn motes_drift_but_stay_inside_and_fade_at_both_ends_of_their_life() {
        let mut mote = Mote {
            id: 1,
            position: Vec3::new(1.9, 2.3, 1.3),
            born: 0.0,
            seed: 0.4,
        };
        for step in 0..2000 {
            mote.drift(step as f64 * 0.05, 0.05, Vec3::new(0.17, 0.015, 0.36));
            assert!(mote.position.cmpge(ROOM_MIN).all() && mote.position.cmple(ROOM_MAX).all());
        }
        assert_eq!(mote.glow(0.0), 0.0);
        assert!(mote.glow(60.0) > 0.5);
        assert_eq!(mote.glow(Mote::LIFETIME), 0.0);
    }
}
