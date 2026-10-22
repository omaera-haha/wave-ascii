//! How a body that isn't a ghost gets about. Every creature steers a point
//! through the air the way the ghost always has, so places and plans work
//! the same for all of them; a body that keeps to the ground follows that
//! point with its feet on whatever is under it, hopping up onto the bed or
//! the sill and leaping after motes, and a flier lands when it settles.

use glam::{Vec2, Vec3};

use super::home::{
    BED, BED_HALF, BEDROOM_WINDOW_HIGH, BEDROOM_WINDOW_LOW, BENCH, EAVE, EAVE_DROP, GARDEN,
    GARDEN_HALF, HOUSE_MAX, HOUSE_MIN, RIDGE, SWING_SEAT,
};
use super::species::Gait;
use super::{Activity, Nook, ROOM_MIN, World, flat};

/// How fast a body falls, in units per second per second: floatier than
/// real, as befits small cartoon creatures.
const GRAVITY: f32 = 7.5;
/// The highest a body can leap above what it stands on.
const LEAP: f32 = 1.25;

/// How a body stands, kept from frame to frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Footing {
    /// Where the body is; for a ghost, where the point is.
    pub body: Vec3,
    /// Whether `body` has been placed yet.
    pub placed: bool,
    pub grounded: bool,
    /// Upward speed while in the air.
    pub rise: f32,
    /// How far through its stride or hop, in whole cycles.
    pub stride: f32,
    /// Seconds since it last landed, for the squash of landing.
    pub landed: f32,
    /// For a flier, from flying at 0 to settled at 1.
    pub perched: f32,
    /// Which way the body faces, as an angle about the vertical: zero
    /// faces the glass.
    pub facing: f32,
    /// How fast it turns, for leaning into turns.
    pub turning: f32,
    /// How fast its feet are going across the ground.
    pub pace: f32,
}

impl super::Ghost {
    /// Where its body is: on the ground under its point, unless it floats.
    pub fn body(&self) -> Vec3 {
        if self.footing.placed {
            self.footing.body
        } else {
            self.position
        }
    }
}

/// What a body stands on below `point`, at the highest top no higher than
/// the point itself: the floor or lawn, or the bed, the cushion, a sill,
/// the bench, the swing, the garden bed, or the roof.
pub fn support(point: Vec3, nook: Vec3) -> f32 {
    let at = flat(point);
    let within = |center: Vec3, half: Vec2| {
        let offset = (at - flat(center)).abs();
        offset.x <= half.x && offset.y <= half.y
    };
    let mut tops = vec![0.0];
    if at.distance(flat(nook)) < Nook::RADIUS {
        tops.push(Nook::HEIGHT);
    }
    if within(BED, Vec2::new(BED_HALF.x, BED_HALF.z)) {
        tops.push(BED_HALF.y * 2.0);
    }
    // The sills inside the back windows, where a body can sit and look out.
    if at.y < ROOM_MIN.z + 0.3 && at.y >= ROOM_MIN.z - 0.1 {
        if at.x.abs() < 0.78 {
            tops.push(1.0);
        }
        if at.x > BEDROOM_WINDOW_LOW.x && at.x < BEDROOM_WINDOW_HIGH.x {
            tops.push(BEDROOM_WINDOW_LOW.y - 0.02);
        }
    }
    if within(BENCH, Vec2::new(0.62, 0.22)) {
        tops.push(0.42);
    }
    if within(SWING_SEAT, Vec2::new(0.32, 0.14)) {
        tops.push(SWING_SEAT.y);
    }
    if within(GARDEN, Vec2::new(GARDEN_HALF.x, GARDEN_HALF.z)) {
        tops.push(GARDEN_HALF.y * 2.0);
    }
    let middle = (HOUSE_MIN.z + HOUSE_MAX.z) * 0.5;
    let depth = (HOUSE_MAX.z - HOUSE_MIN.z) * 0.5 + EAVE;
    if at.x > HOUSE_MIN.x && at.x < HOUSE_MAX.x && (at.y - middle).abs() < depth {
        tops.push(RIDGE - (RIDGE - EAVE_DROP) * (at.y - middle).abs() / depth);
    }
    tops.into_iter()
        .filter(|top| *top <= point.y + 0.05)
        .fold(0.0, f32::max)
}

impl World {
    /// Moves the body after the point it follows, and turns it to face
    /// where it is going or what it looks at.
    pub(super) fn keep_footing(&mut self, seconds: f32) {
        let size = self.size();
        let nook = self.nook.position;
        let ghost = &mut self.ghost;
        let gait = ghost.species.gait();
        let point = ghost.position;
        let footing = &mut ghost.footing;
        let stance = ghost.species.stance() * size;
        let ground = support(point, nook) + stance;
        let before = footing.body;
        if !footing.placed || flat(footing.body).distance(flat(point)) > 1.5 {
            footing.body = if gait == Gait::Float {
                point
            } else {
                Vec3::new(point.x, ground, point.z)
            };
            footing.placed = true;
            footing.grounded = true;
            footing.rise = 0.0;
        }
        let seconds = seconds.max(0.0);
        footing.landed = (footing.landed + seconds).min(10.0);
        match gait {
            Gait::Float => footing.body = point,
            Gait::Fly => {
                let settled = ghost.asleep > 0.3
                    || (ghost.velocity.length() < 0.12
                        && (ghost.plan.arrived.is_some() || ghost.plan.done));
                let goal = if settled { 1.0 } else { 0.0 };
                footing.perched += (goal - footing.perched) * (1.0 - (-seconds / 0.35).exp());
                let landed = Vec3::new(point.x, ground, point.z);
                footing.body = point.lerp(landed, footing.perched);
                footing.grounded = footing.perched > 0.95;
            }
            Gait::Walk | Gait::Hop => {
                // Over a step up ahead, it jumps a moment early rather than
                // walking into the bed's side.
                let ahead = point + Vec3::new(ghost.velocity.x, 0.0, ghost.velocity.z) * 0.22;
                let ground_ahead = support(ahead.with_y(point.y), nook) + stance;
                let mut goal = ground.max(ground_ahead);
                let horizontal = flat(ghost.velocity).length();
                let reaching = matches!(ghost.plan.activity, Activity::Chase | Activity::Pounce)
                    && !ghost.plan.done
                    && flat(ghost.plan.target).distance(flat(footing.body)) < 0.55;
                if reaching {
                    goal = goal.max(ghost.plan.target.y.min(ground + LEAP));
                }
                let dancing = ghost.plan.activity == Activity::Dance && !ghost.plan.done;
                let hopping = gait == Gait::Hop && horizontal > 0.12;
                if footing.grounded {
                    let spring = if hopping {
                        Some(1.0 + 0.4 * horizontal.min(1.5))
                    } else if dancing && footing.landed > 0.3 {
                        Some(1.9)
                    } else {
                        None
                    };
                    if goal > footing.body.y + 0.06 {
                        footing.grounded = false;
                        footing.rise = (2.0 * GRAVITY * (goal - footing.body.y + 0.08)).sqrt();
                    } else if let Some(spring) = spring.filter(|_| footing.landed > 0.08) {
                        footing.grounded = false;
                        footing.rise = spring;
                    } else {
                        // Steps down easily; a drop off an edge is a fall.
                        if footing.body.y - ground > 0.06 {
                            footing.grounded = false;
                            footing.rise = 0.0;
                        } else {
                            footing.body.y = ground;
                        }
                    }
                }
                if !footing.grounded {
                    footing.rise -= GRAVITY * seconds;
                    footing.body.y += footing.rise * seconds;
                    if footing.rise <= 0.0 && footing.body.y <= ground {
                        footing.body.y = ground;
                        footing.grounded = true;
                        footing.rise = 0.0;
                        footing.landed = 0.0;
                    }
                }
                footing.body.x = point.x;
                footing.body.z = point.z;
            }
        }
        // Strides follow the ground covered; hops and flaps keep a beat.
        let travel = flat(footing.body - before).length();
        let pace = if seconds > 0.0 { travel / seconds } else { 0.0 };
        footing.pace += (pace.min(3.0) - footing.pace) * (1.0 - (-seconds / 0.2).exp());
        let stride_length = (0.32 * size).max(0.05);
        footing.stride = (footing.stride + travel / stride_length).rem_euclid(1000.0);
        // Faces where it goes while moving, else what it looks at, else the
        // glass.
        let looking = ghost
            .look
            .map(|look| look - footing.body)
            .filter(|offset| flat(*offset).length() > 0.05);
        let toward = if pace > 0.15 {
            Some(Vec3::new(
                ghost.velocity.x,
                0.0,
                ghost.velocity.z,
            ))
        } else {
            looking
        };
        let wanted = toward
            .map(|toward| toward.x.atan2(toward.z))
            .unwrap_or(0.0);
        let mut turn = wanted - footing.facing;
        turn = (turn + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        let rate = if ghost.asleep > 0.5 { 0.0 } else { 6.0 };
        let step = turn * (1.0 - (-seconds * rate).exp());
        footing.facing = (footing.facing + step).rem_euclid(std::f32::consts::TAU);
        footing.turning = if seconds > 0.0 { step / seconds } else { 0.0 };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Species, home};

    fn walker(species: Species) -> World {
        let mut world = World::new(3, 12.0);
        world.ghost.species = species;
        world
    }

    #[test]
    fn the_floor_holds_up_a_body_and_furniture_only_from_above() {
        let nook = Vec3::new(-1.3, 0.0, 0.55);
        assert_eq!(support(Vec3::new(0.0, 1.0, 0.0), nook), 0.0);
        assert_eq!(support(BED + Vec3::Y * 0.9, nook), BED_HALF.y * 2.0);
        assert_eq!(support(BED + Vec3::Y * 0.1, nook), 0.0);
        assert_eq!(support(nook + Vec3::Y * 0.5, nook), Nook::HEIGHT);
        let roof = home::Spot::Roof.position(1.0);
        assert!((support(roof, nook) - RIDGE).abs() < 0.05);
        assert_eq!(support(Vec3::new(0.0, 1.5, ROOM_MIN.z + 0.1), nook), 1.0);
    }

    #[test]
    fn a_walker_stands_on_the_floor_under_its_point_and_jumps_onto_the_bed() {
        let mut world = walker(Species::Fox);
        world.ghost.position = Vec3::new(0.0, 1.1, 0.0);
        world.keep_footing(0.1);
        let stance = Species::Fox.stance() * world.size();
        assert!((world.ghost.body().y - stance).abs() < 1e-4);
        // Over the bed, its feet go up to the mattress, through the air.
        world.ghost.position = BED + Vec3::new(1.0, 0.9, 0.0);
        world.keep_footing(0.1);
        let mut highest: f32 = 0.0;
        let mut airborne = false;
        for _ in 0..60 {
            world.ghost.position.x = (world.ghost.position.x - 0.03).max(BED.x + 0.2);
            world.ghost.velocity = -Vec3::X;
            world.keep_footing(1.0 / 30.0);
            highest = highest.max(world.ghost.body().y);
            airborne |= !world.ghost.footing.grounded;
        }
        assert!(airborne);
        assert!(highest > BED_HALF.y * 2.0 + stance);
        assert!((world.ghost.body().y - (BED_HALF.y * 2.0 + stance)).abs() < 1e-3);
        assert!(world.ghost.footing.grounded);
    }

    #[test]
    fn a_ghost_body_is_its_point_and_a_flier_settles_when_still() {
        let mut world = walker(Species::Ghost);
        world.ghost.position = Vec3::new(0.3, 1.2, 0.1);
        world.keep_footing(0.1);
        assert_eq!(world.ghost.body(), world.ghost.position);
        let mut world = walker(Species::Owl);
        world.ghost.position = Vec3::new(0.3, 1.2, 0.1);
        world.ghost.velocity = Vec3::ZERO;
        world.ghost.plan.arrived = Some(0.0);
        for _ in 0..60 {
            world.keep_footing(0.1);
        }
        assert!(world.ghost.body().y < 0.4);
        world.ghost.velocity = Vec3::X;
        for _ in 0..30 {
            world.keep_footing(0.1);
        }
        assert!((world.ghost.body().y - 1.2).abs() < 0.05);
    }

    #[test]
    fn bodies_face_where_they_go() {
        let mut world = walker(Species::Bunny);
        world.ghost.position = Vec3::new(0.0, 1.0, 0.0);
        world.keep_footing(0.1);
        for step in 0..40 {
            world.ghost.velocity = Vec3::X;
            world.ghost.position.x += 0.02;
            let _ = step;
            world.keep_footing(0.05);
        }
        let facing = world.ghost.footing.facing;
        assert!((facing - std::f32::consts::FRAC_PI_2).abs() < 0.1, "{facing}");
    }

    #[test]
    fn bodies_stay_finite_through_long_and_wild_steps() {
        for species in Species::ALL {
            let mut world = walker(species);
            for step in 0..200 {
                world.ghost.position = Vec3::new(
                    (step as f32 * 0.37).sin() * 3.0,
                    (step as f32 * 0.21).cos().abs() * 3.0,
                    (step as f32 * 0.13).sin(),
                );
                world.ghost.velocity = Vec3::splat(step as f32 * 0.01);
                world.keep_footing(if step % 7 == 0 { 5.0 } else { 0.016 });
                assert!(world.ghost.body().is_finite());
                assert!(world.ghost.footing.facing.is_finite());
            }
        }
    }
}
