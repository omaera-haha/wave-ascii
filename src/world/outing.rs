//! Going out to the yard: the ghost floats out the door and spends a while
//! at a couple of its favorite spots, then comes back in.

use glam::Vec3;

use super::home::{self, Spot};
use super::{Activity, Season, Sky, World, meteor};

/// Seconds it stays at a spot once there.
const DWELL: (f32, f32) = (14.0, 26.0);
/// Spots it visits on one outing at most.
const VISITS: u32 = 3;
/// Seconds between outings it starts on its own.
const PAUSE: f64 = 1500.0;

impl World {
    pub fn yard_is_open(&self) -> bool {
        let rain = self.weather.rain(self.time);
        self.ghost.asleep < 0.3
            && self.ghost.needs.energy > 0.3
            && !self.weather.storm
            && rain < 0.45
            && !self.bedtime()
            && self.focus.is_none()
            && self.hunt.is_none()
            && self
                .last_outing
                .is_none_or(|at| self.time - at >= PAUSE || self.time < at)
    }

    /// Where to go first: the lawn for the stars at night, else a spot that
    /// suits the season and the hour.
    pub(super) fn first_spot(&mut self) -> Spot {
        let dark = meteor::dark(self.hour);
        if dark {
            return if self.random.unit() < 0.7 {
                Spot::Lawn
            } else {
                Spot::Bench
            };
        }
        let mut weights = [
            (Spot::Swing, 1.4),
            (Spot::Garden, if self.seedlings.is_empty() { 0.4 } else { 1.0 }),
            (Spot::Birdbath, 0.8),
            (Spot::Bench, 0.7),
            (Spot::Lawn, 0.8),
            (Spot::Fence, 0.6),
        ];
        if self.season() == Season::Winter {
            weights[2].1 = 0.3;
            weights[4].1 = 1.4;
        }
        let total: f32 = weights.iter().map(|(_, weight)| weight).sum();
        let mut pick = self.random.unit() * total;
        for (spot, weight) in weights {
            if pick < weight {
                return spot;
            }
            pick -= weight;
        }
        Spot::Swing
    }

    /// The spot an outing is at or heading to.
    pub fn yard_spot(&self) -> Option<Spot> {
        if self.ghost.plan.activity != Activity::Yard || self.ghost.plan.done {
            return None;
        }
        let size = self.size();
        let target = self.ghost.plan.anchor;
        Spot::ALL
            .into_iter()
            .min_by(|first, second| {
                first
                    .position(size)
                    .distance_squared(target)
                    .total_cmp(&second.position(size).distance_squared(target))
            })
    }

    /// Whether the ghost is sitting on the swing, swinging.
    pub fn swinging(&self) -> Option<f32> {
        (self.yard_spot() == Some(Spot::Swing))
            .then(|| self.ghost.plan.stayed(self.time))
            .filter(|stayed| *stayed > 0.0)
    }

    pub(super) fn carry_out_outing(&mut self, seconds: f32) -> Option<Vec3> {
        let spot = self.yard_spot()?;
        let arrived = self.ghost.plan.arrived;
        if arrived == Some(self.time) {
            self.last_outing = Some(self.time);
            self.ghost.plan.count += 1;
            self.ghost.plan.length = self.random.range(DWELL.0, DWELL.1);
            self.arrive_at(spot);
        }
        let stayed = self.ghost.plan.stayed(self.time);
        if spot == Spot::Swing && arrived.is_some() {
            // Ride the seat as it swings.
            let seat = self.swing_seat() + Vec3::Y * (0.34 * self.size());
            self.ghost.plan.target = seat;
            if self.ghost.position.distance(seat) < 0.4 {
                self.ghost.position = self.ghost.position.lerp(seat, 0.5);
            }
        }
        if arrived.is_some() && stayed > self.ghost.plan.length {
            let visits = self.ghost.plan.count;
            if visits >= VISITS
                || self.ghost.needs.energy < 0.3
                || self.bedtime()
                || self.random.unit() < 0.35
            {
                self.ghost.plan.done = true;
            } else {
                let next = loop {
                    let index = (self.random.unit() * Spot::ALL.len() as f32) as usize;
                    let choice = Spot::ALL[index.min(Spot::ALL.len() - 1)];
                    if choice != spot {
                        break choice;
                    }
                };
                let position = next.position(self.size());
                self.ghost.plan.target = position;
                self.ghost.plan.anchor = position;
                self.ghost.plan.arrived = None;
                self.ghost.plan.started = self.time;
            }
        }
        self.ghost.needs.change(0.0, 0.012 * seconds, 0.0);
        Some(spot.looks_at())
    }

    fn arrive_at(&mut self, spot: Spot) {
        if self.ghost.asleep >= 0.3 {
            return;
        }
        let korean = self.speaks_korean();
        let dark = meteor::dark(self.hour);
        let snowy = self.weather.snow_cover > 0.3;
        let thought = match (spot, korean) {
            (Spot::Swing, false) => "Wheee! Up to the clouds!",
            (Spot::Swing, true) => "슝— 구름까지 올라가자!",
            (Spot::Garden, false) if self.seedlings.is_empty() => {
                "This bed is waiting for seeds."
            }
            (Spot::Garden, true) if self.seedlings.is_empty() => "씨앗을 기다리는 화단이야.",
            (Spot::Garden, false) => "Growing, growing, little ones.",
            (Spot::Garden, true) => "쑥쑥 자라라, 얘들아.",
            (Spot::Birdbath, false) if self.season() == Season::Winter => {
                "The birdbath froze into a little mirror."
            }
            (Spot::Birdbath, true) if self.season() == Season::Winter => {
                "새 물그릇이 꽁꽁 얼어서 작은 거울이 됐어."
            }
            (Spot::Birdbath, false) => "My face wobbles in the water.",
            (Spot::Birdbath, true) => "물속 내 얼굴이 일렁거려.",
            (Spot::Bench, false) => "A good bench for sitting and thinking.",
            (Spot::Bench, true) => "앉아서 생각하기 좋은 벤치야.",
            (Spot::Lawn, false) if dark => "So many stars from out here.",
            (Spot::Lawn, true) if dark => "밖에서 보니까 별이 정말 많아.",
            (Spot::Lawn, false) if snowy => "The snow is soft as a cushion.",
            (Spot::Lawn, true) if snowy => "눈이 방석처럼 폭신해.",
            (Spot::Lawn, false) if self.weather.sky == Sky::Clear => {
                "That cloud looks like a sleepy whale."
            }
            (Spot::Lawn, true) if self.weather.sky == Sky::Clear => "저 구름, 졸린 고래 같아.",
            (Spot::Lawn, false) => "The grass smells like rain.",
            (Spot::Lawn, true) => "풀에서 비 냄새가 나.",
            (Spot::Fence, false) => "The hills go on and on.",
            (Spot::Fence, true) => "언덕이 끝없이 이어져.",
        };
        self.think(thought);
    }

    /// Where the swing's seat is now.
    pub fn swing_seat(&self) -> Vec3 {
        let length = home::SWING_TOP.y - home::SWING_SEAT.y;
        let (sine, cosine) = self.swing_angle().sin_cos();
        home::SWING_TOP + Vec3::new(0.0, -length * cosine, length * sine)
    }

    /// The swing's angle from hanging straight, in radians.
    pub fn swing_angle(&self) -> f32 {
        let riding = self.swinging().map_or(0.0, |stayed| (stayed / 3.0).min(1.0));
        riding * (self.time * 1.9).sin() as f32 * 0.45
    }

    /// What the ghost is doing in the yard, in words.
    pub fn outing_words(&self) -> &'static str {
        self.yard_spot()
            .filter(|_| self.ghost.plan.arrived.is_some())
            .map_or("heading out to the yard", Spot::words)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_outing_goes_out_the_door_visits_spots_and_comes_to_an_end() {
        let mut world = World::new(3, 11.0);
        world.ghost.needs.energy = 1.0;
        world.begin(Activity::Yard);
        let mut went_out = false;
        let mut visited = Vec::new();
        for _ in 0..(400.0 / 0.05) as usize {
            world.step(0.05);
            if !home::in_house(world.ghost.position) {
                went_out = true;
            }
            if let Some(spot) = world.yard_spot()
                && world.ghost.plan.arrived.is_some()
                && visited.last() != Some(&spot)
            {
                visited.push(spot);
            }
            if world.ghost.plan.activity != Activity::Yard || world.ghost.plan.done {
                break;
            }
        }
        assert!(went_out);
        assert!(!visited.is_empty());
        assert!(visited.len() <= VISITS as usize);
    }

    #[test]
    fn the_swing_only_moves_while_the_ghost_rides_it() {
        let mut world = World::new(3, 11.0);
        assert_eq!(world.swing_angle(), 0.0);
        world.begin(Activity::Yard);
        let seat = Spot::Swing.position(world.size());
        world.ghost.plan.target = seat;
        world.ghost.plan.anchor = seat;
        world.ghost.position = seat;
        world.ghost.plan.arrived = Some(world.time);
        world.time += 4.0;
        world.time += 0.4;
        assert!(world.swinging().is_some());
        assert!(world.swing_angle().abs() > 0.0);
        assert!(world.swing_seat().distance(home::SWING_SEAT) < 0.8);
    }

    #[test]
    fn storms_and_bedtime_keep_it_indoors() {
        let mut world = World::new(3, 11.0);
        world.ghost.needs.energy = 1.0;
        assert!(world.yard_is_open());
        world.weather.storm = true;
        assert!(!world.yard_is_open());
        let mut world = World::new(3, 23.5);
        world.ghost.needs.energy = 1.0;
        assert!(!world.yard_is_open() || !world.bedtime());
    }
}
