//! Now and then a little stranger wanders round the back of the yard, hops
//! up onto the outer sill, and peers in through the window at the
//! household, then wanders off again, unless the viewer invites it to stay.

use glam::Vec3;

use super::home::{SILL, SILL_HALF};
use super::species::{Genome, Species};
use super::{EventKind, Sky, World, household};

/// Seconds it takes to walk up to the glass, and back away.
const WALK: f64 = 7.0;
/// Seconds it stays looking in, when nobody invites it.
const STAYS: (f32, f32) = (150.0, 240.0);
/// How big a stranger is, against those who live here.
pub const SIZE: f32 = 0.85;
/// Seconds between strangers, at the soonest and latest.
const BETWEEN: (f32, f32) = (2400.0, 6000.0);

/// A creature from somewhere else, visiting the front of the yard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stranger {
    pub species: Species,
    pub genome: Genome,
    pub came: f64,
    pub leaves_at: f64,
    /// Where it comes from and goes back to, at the edge of the lawn.
    pub from: Vec3,
    /// Where it sits on the sill, looking in through the window.
    pub spot: Vec3,
    /// When it set off home, once it has.
    pub leaving: Option<f64>,
}

impl Stranger {
    /// How far along its walk up to the glass (0 to 1) or away from it.
    fn along(&self, time: f64) -> f32 {
        match self.leaving {
            Some(left) => 1.0 - ((time - left) / WALK).clamp(0.0, 1.0) as f32,
            None => ((time - self.came) / WALK).clamp(0.0, 1.0) as f32,
        }
    }

    /// Where its body is: walking along the ground to below the sill,
    /// hopping up onto it, or sitting there shifting a little.
    pub fn position(&self, time: f64) -> Vec3 {
        let along = self.along(time);
        let below = Vec3::new(self.spot.x, self.from.y, self.spot.z - 0.35);
        if along < 0.8 {
            let walk = along / 0.8;
            return self.from.lerp(below, walk * walk * (3.0 - 2.0 * walk));
        }
        let hop = ((along - 0.8) / 0.2).min(1.0);
        let fidget = if along >= 1.0 {
            Vec3::new(((time - self.came) * 0.4).sin() as f32 * 0.08, 0.0, 0.0)
        } else {
            Vec3::ZERO
        };
        below.lerp(self.spot, hop) + Vec3::Y * 0.35 * (hop * std::f32::consts::PI).sin() + fidget
    }

    /// How fast it is going, for its stride.
    pub fn pace(&self, time: f64) -> f32 {
        let along = self.along(time);
        if along > 0.0 && along < 0.8 {
            self.from.distance(self.spot) / (WALK as f32 * 0.8) * 1.2
        } else {
            0.0
        }
    }

    /// Which way it faces: along its walk, else in through the window.
    pub fn facing(&self, time: f64) -> f32 {
        let along = self.along(time);
        if along > 0.0 && along < 0.8 {
            let toward = if self.leaving.is_some() {
                self.from - self.spot
            } else {
                self.spot - self.from
            };
            toward.x.atan2(toward.z)
        } else {
            0.0
        }
    }

    pub fn gone(&self, time: f64) -> bool {
        self.leaving.is_some_and(|left| time - left > WALK)
    }

    /// Whether it is at the glass, looking in.
    pub fn looking_in(&self, time: f64) -> bool {
        self.leaving.is_none() && time - self.came >= WALK
    }
}

impl World {
    /// Brings a stranger by now and then, while there is room for one more
    /// and the viewer is around to see it, and sends it home after a while.
    pub(super) fn advance_stranger(&mut self) {
        let time = self.time;
        if let Some(mut stranger) = self.stranger {
            if stranger.leaving.is_none() && time >= stranger.leaves_at {
                stranger.leaving = Some(time);
                self.stranger = Some(stranger);
                if self.ghost.asleep < 0.3 {
                    let line = if self.speaks_korean() {
                        format!("잘 가, 꼬마 {}!", stranger.species.noun_korean())
                    } else {
                        format!("Bye, little {}!", stranger.species.noun())
                    };
                    self.think(&line);
                }
            } else if stranger.gone(time) {
                self.stranger = None;
            }
            return;
        }
        let fair = self.weather.sky != Sky::Rain && !self.weather.storm;
        let daytime = (8.0..19.0).contains(&self.hour);
        if time < self.next_stranger {
            return;
        }
        self.next_stranger = time + f64::from(self.random.range(BETWEEN.0, BETWEEN.1));
        if !fair
            || !daytime
            || !self.viewer_present()
            || self.household_size() >= household::MOST
            || self.focus.is_some()
        {
            return;
        }
        let kinds: Vec<Species> = Species::ALL
            .into_iter()
            .filter(|species| self.creatures().all(|ghost| ghost.species != *species))
            .collect();
        let Some(index) = self.pick(&vec![1.0; kinds.len()]) else {
            return;
        };
        let species = kinds[index];
        let side = if self.random.unit() < 0.5 { -1.0 } else { 1.0 };
        // Strangers are a little smaller than those who live here.
        let stance = species.stance() * SIZE;
        let spot = Vec3::new(
            SILL.x + self.random.range(-0.45, 0.45),
            SILL.y + SILL_HALF.y + stance,
            SILL.z,
        );
        let stranger = Stranger {
            species,
            genome: Genome::roll(species, &mut self.random),
            came: time,
            leaves_at: time + f64::from(self.random.range(STAYS.0, STAYS.1)) + WALK,
            from: Vec3::new(side * 5.5, stance, SILL.z - 1.6),
            spot,
            leaving: None,
        };
        self.stranger = Some(stranger);
        self.record(EventKind::StrangerCame(species));
        if self.ghost.asleep < 0.3 {
            let line = if self.speaks_korean() {
                format!("저기 꼬마 {}가 우리를 보고 있어!", species.noun_korean())
            } else {
                format!("A little {} is peeking in at us!", species.noun())
            };
            self.say(&line);
            self.attend("viewer", 4.0);
        }
    }

    /// Lets a stranger come by at the next chance: for previews.
    pub fn expect_stranger(&mut self) {
        self.next_stranger = self.time;
    }

    /// The viewer invites the stranger at the glass to live here, with
    /// `name`. Returns its id, or `None` when nobody is there or the house is
    /// full.
    pub fn invite_stranger(&mut self, name: &str, seed: u64) -> Option<u32> {
        let stranger = self.stranger.take()?;
        let at = stranger.position(self.time);
        let Some(id) = self.adopt(stranger.species, name, seed) else {
            self.stranger = Some(stranger);
            return None;
        };
        let genome = stranger.genome;
        // It is already in the yard, and goes round to the door.
        self.with_creature(id, |world| {
            world.ghost.genome = genome;
            world.ghost.position = Vec3::new(at.x, 0.9, at.z);
            world.ghost.footing = Default::default();
        });
        Some(id)
    }
}

/// Whether a message asks the stranger in.
pub fn invitation(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "come in",
        "come inside",
        "welcome",
        "stay with us",
        "live with us",
        "들어와",
        "어서 와",
        "어서와",
        "같이 살자",
    ]
    .iter()
    .any(|words| lower.contains(words))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stranger_walks_up_looks_in_and_goes_home_unless_invited() {
        let mut world = World::new(4, 12.0);
        world.viewer.active_at = 1.0;
        world.next_stranger = 0.0;
        world.time = 1.0;
        world.viewer.active_at = world.time;
        world.advance_stranger();
        let stranger = world.stranger.expect("a stranger came by");
        assert_ne!(stranger.species, world.ghost.species);
        assert!(stranger.position(world.time).distance(stranger.from) < 1e-3);
        assert!(stranger.position(world.time + WALK).distance(stranger.spot) < 0.2);
        assert!(stranger.looking_in(world.time + WALK + 1.0));
        world.time = stranger.leaves_at + 0.1;
        world.advance_stranger();
        assert!(world.stranger.unwrap().leaving.is_some());
        world.time += WALK + 1.0;
        world.advance_stranger();
        assert!(world.stranger.is_none());
    }

    #[test]
    fn an_invited_stranger_comes_to_live_here_with_its_own_coat() {
        let mut world = World::new(4, 12.0);
        world.next_stranger = 0.0;
        world.time = 1.0;
        world.viewer.active_at = world.time;
        world.advance_stranger();
        let stranger = world.stranger.unwrap();
        assert!(invitation("Come in, little one!"));
        assert!(invitation("들어와!"));
        assert!(!invitation("hello"));
        let id = world.invite_stranger("Pip", 9).unwrap();
        assert!(world.stranger.is_none());
        let newcomer = world.creature(id).unwrap();
        assert_eq!(newcomer.species, stranger.species);
        assert_eq!(newcomer.genome, stranger.genome);
        assert_eq!(world.household_size(), 2);
    }

    #[test]
    fn no_stranger_comes_to_a_full_house_or_while_nobody_watches() {
        let mut world = World::new(4, 12.0);
        world.next_stranger = 0.0;
        world.time = 1.0;
        world.advance_stranger();
        assert!(world.stranger.is_none());
        for index in 0..5 {
            world.adopt(Species::Frog, &format!("Frog {index}"), index);
        }
        world.next_stranger = 0.0;
        world.viewer.active_at = world.time;
        world.advance_stranger();
        assert!(world.stranger.is_none());
    }
}
