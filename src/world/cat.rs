//! A neighborhood cat that drops by the yard now and then: it comes over
//! the fence, settles on the bench, the lawn, the fence top, or the garden
//! bed, washes, naps, and wanders off. Rushing at it sends it running;
//! quiet company slowly earns its trust.

use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::home::{self, BENCH, GARDEN, GARDEN_HALF, LOT_MAX, LOT_MIN};
use super::{EventKind, Random, Sky, World};

/// Walking and running speeds, in units a second.
const WALK: f32 = 0.55;
const RUN: f32 = 2.6;
/// Height of the fence top the cat walks along.
const FENCE_TOP: f32 = 0.86;

/// The cat's coat; the same cat always comes back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Coat {
    Ginger,
    Gray,
    Black,
    Calico,
}

impl Coat {
    pub fn words(self) -> &'static str {
        match self {
            Self::Ginger => "ginger",
            Self::Gray => "gray",
            Self::Black => "black",
            Self::Calico => "calico",
        }
    }

    fn choose(random: &mut Random) -> Self {
        [Self::Ginger, Self::Gray, Self::Black, Self::Calico]
            [(random.unit() * 4.0) as usize % 4]
    }
}

/// Where the cat settles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rest {
    Bench,
    Lawn,
    Fence,
    Garden,
}

impl Rest {
    fn position(self) -> Vec3 {
        match self {
            Self::Bench => BENCH + Vec3::new(0.32, 0.48, 0.02),
            Self::Lawn => Vec3::new(6.35, 0.0, 1.05),
            Self::Fence => Vec3::new(7.1, FENCE_TOP, LOT_MIN.y),
            Self::Garden => {
                GARDEN + Vec3::new(GARDEN_HALF.x - 0.04, GARDEN_HALF.y * 2.0, -GARDEN_HALF.z)
            }
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Self::Bench => "on the bench",
            Self::Lawn => "on a sunny patch of lawn",
            Self::Fence => "on top of the back fence",
            Self::Garden => "on the corner of the garden bed",
        }
    }
}

/// What the cat is doing, for drawing it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pose {
    Walking,
    Sitting,
    Napping,
    Running,
}

/// How the cat looks right now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatPicture {
    pub position: Vec3,
    /// Toward +x (1) or -x (-1).
    pub facing: f32,
    pub pose: Pose,
    /// How far along its stride, in radians.
    pub step: f32,
    /// The tail's sway, from -1 to 1.
    pub tail: f32,
    pub coat: Coat,
    /// Washing a paw while it sits.
    pub washing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cat {
    pub rest: Rest,
    pub noticed: bool,
    pub coat: Coat,
    /// Where it comes over the fence, along the right side.
    entry: Vec3,
    came: f64,
    settles_at: f64,
    sleeps_at: f64,
    leaves_at: f64,
    /// When it set off, from where, and whether it ran.
    departing: Option<(f64, Vec3, bool)>,
    seed: f32,
}

impl Cat {
    fn new(time: f64, coat: Coat, random: &mut Random) -> Self {
        let rest = [Rest::Bench, Rest::Lawn, Rest::Fence, Rest::Garden]
            [(random.unit() * 4.0) as usize % 4];
        let entry = Vec3::new(LOT_MAX.x, FENCE_TOP, random.range(-3.6, -1.0));
        let mut cat = Self {
            rest,
            noticed: false,
            coat,
            entry,
            came: time,
            settles_at: 0.0,
            sleeps_at: 0.0,
            leaves_at: 0.0,
            departing: None,
            seed: random.range(0.0, TAU),
        };
        cat.settles_at = time + f64::from(length(&cat.path()) / WALK);
        cat.sleeps_at = cat.settles_at + f64::from(random.range(30.0, 70.0));
        cat.leaves_at = cat.sleeps_at + f64::from(random.range(90.0, 220.0));
        cat
    }

    /// The way in: along the fence top, or down off it, to its spot.
    fn path(&self) -> Vec<Vec3> {
        if self.rest == Rest::Fence {
            vec![
                self.entry,
                Vec3::new(LOT_MAX.x, FENCE_TOP, LOT_MIN.y),
                self.rest.position(),
            ]
        } else {
            let down = Vec3::new(LOT_MAX.x - 0.45, 0.0, self.entry.z);
            vec![self.entry, down, self.rest.position()]
        }
    }

    /// Whether it has reached its spot and isn't leaving.
    pub fn settled(&self, time: f64) -> bool {
        self.departing.is_none() && time >= self.settles_at
    }

    pub fn napping(&self, time: f64) -> bool {
        self.departing.is_none() && time >= self.sleeps_at
    }

    pub fn picture(&self, time: f64) -> CatPicture {
        let wave = |speed: f64| ((time * speed).rem_euclid(std::f64::consts::TAU) as f32 + self.seed).sin();
        let (position, pose, facing) = if let Some((start, from, startled)) = self.departing {
            let away = self.exit(from, startled);
            let speed = if startled { RUN } else { WALK };
            let (position, heading) = along(&away, ((time - start) as f32 * speed).max(0.0));
            (
                position,
                if startled { Pose::Running } else { Pose::Walking },
                heading.x.signum(),
            )
        } else {
            let walked = (time - self.came) as f32 * WALK;
            let path = self.path();
            if walked < length(&path) {
                let (position, heading) = along(&path, walked);
                (position, Pose::Walking, heading.x.signum())
            } else {
                let pose = if time >= self.sleeps_at {
                    Pose::Napping
                } else {
                    Pose::Sitting
                };
                (self.rest.position(), pose, -1.0)
            }
        };
        CatPicture {
            position,
            facing: if facing == 0.0 { -1.0 } else { facing },
            pose,
            step: ((time * 7.0).rem_euclid(std::f64::consts::TAU)) as f32,
            tail: wave(if pose == Pose::Napping { 0.4 } else { 1.3 }),
            coat: self.coat,
            washing: pose == Pose::Sitting && wave(0.21) > 0.55,
        }
    }

    pub fn position(&self, time: f64) -> Vec3 {
        self.picture(time).position
    }

    /// The way out: back along its path, or straight over the nearest
    /// fence when it bolts.
    fn exit(&self, from: Vec3, startled: bool) -> Vec<Vec3> {
        if startled {
            let side = if from.x > (LOT_MIN.x + LOT_MAX.x) * 0.5 {
                Vec3::new(LOT_MAX.x + 0.8, FENCE_TOP, from.z)
            } else {
                Vec3::new(from.x, FENCE_TOP, LOT_MIN.y - 0.8)
            };
            vec![from, side]
        } else {
            let mut path = self.path();
            path.reverse();
            path[0] = from;
            path.push(self.entry + Vec3::X * 0.8);
            path
        }
    }

    fn gone(&self, time: f64) -> bool {
        self.departing.is_some_and(|(start, from, startled)| {
            let speed = if startled { RUN } else { WALK };
            (time - start) as f32 * speed >= length(&self.exit(from, startled))
        })
    }
}

fn length(path: &[Vec3]) -> f32 {
    path.windows(2).map(|pair| pair[0].distance(pair[1])).sum()
}

/// The point `distance` along a path, and the way it heads there.
fn along(path: &[Vec3], distance: f32) -> (Vec3, Vec3) {
    let mut left = distance;
    for pair in path.windows(2) {
        let leg = pair[0].distance(pair[1]);
        let heading = (pair[1] - pair[0]).normalize_or(Vec3::X);
        if left <= leg {
            return (pair[0] + heading * left, heading);
        }
        left -= leg;
    }
    let last = path.len() - 1;
    let heading = (path[last] - path[last.saturating_sub(1)]).normalize_or(Vec3::X);
    (path[last], heading)
}

impl World {
    pub(super) fn advance_cat(&mut self, seconds: f32) {
        let time = self.time;
        let daytime = (8.0..20.0).contains(&self.hour);
        let fair = !matches!(self.weather.sky, Sky::Rain) && !self.weather.storm;
        if self.visitors.cat.is_none() && time >= self.visitors.next_cat {
            self.visitors.next_cat =
                time + f64::from(self.visitors.cat_random.range(1800.0, 4200.0) / (1.0 + self.life.cat_trust));
            if daytime && fair {
                let coat = *self
                    .life
                    .cat_coat
                    .get_or_insert_with(|| Coat::choose(&mut self.visitors.cat_random));
                let mut cat = Cat::new(time, coat, &mut self.visitors.cat_random);
                // A soft meow carries indoors; it may hear it.
                if self.ghost.asleep < 0.3 && self.visitors.cat_random.unit() < 0.7 {
                    cat.noticed = true;
                    let korean = self.speaks_korean();
                    self.think(if korean {
                        "방금 야옹 소리 들렸어?"
                    } else {
                        "Was that a meow?"
                    });
                }
                self.visitors.cat = Some(cat);
                self.record(EventKind::CatArrived);
                if let Some(milestone) = self.life.reach(
                    "A cat came to visit the yard.".to_owned(),
                    "고양이가 마당에 놀러 왔어.",
                ) {
                    self.record(EventKind::Milestone(milestone));
                }
            }
        }
        let Some(mut cat) = self.visitors.cat.take() else {
            return;
        };
        let position = cat.position(time);
        let offset = position - self.ghost.position;
        let near = offset.length();
        let outside = !home::in_house(self.ghost.position);
        let wary = 1.0 - 0.7 * self.life.cat_trust.clamp(0.0, 1.0);
        let speed = self.ghost.velocity.length();
        let toward = self.ghost.velocity.dot(offset.normalize_or_zero());
        let rushing = outside && near < 1.6 * wary && speed > 0.75 && toward > 0.45;
        let crowded = outside && near < 0.45 * wary;
        if cat.departing.is_none() && cat.settled(time) && (rushing || crowded) {
            cat.departing = Some((time, position, true));
            self.life.cat_trust = (self.life.cat_trust - 0.05).max(0.0);
            self.record(EventKind::CatLeft { startled: true });
            if self.ghost.asleep < 0.5 {
                let korean = self.speaks_korean();
                self.think(if korean {
                    "앗, 고양이가 놀랐나 봐."
                } else {
                    "Oh… the cat ran off."
                });
            }
        } else if cat.departing.is_none()
            && (time >= cat.leaves_at || !fair || self.is_night() && !daytime)
            && cat.settled(time)
        {
            cat.departing = Some((time, position, false));
            self.record(EventKind::CatLeft { startled: false });
        } else if cat.settled(time) && outside && near < 1.6 && speed < 0.35 {
            // Quiet company earns a little trust.
            let before = self.life.cat_trust;
            self.life.cat_trust = super::visitor::quiet_trust(before, seconds * 3.0);
            if before < 0.5
                && self.life.cat_trust >= 0.5
                && let Some(milestone) = self.life.reach(
                    "The cat lets me sit beside it now.".to_owned(),
                    "이제 고양이가 내 옆에 있게 해 줘.",
                )
            {
                self.record(EventKind::Milestone(milestone));
            }
        }
        if !cat.gone(time) {
            self.visitors.cat = Some(cat);
        }
    }

    /// Where to float to keep the cat company: beside it, on the house's
    /// side, at a polite distance.
    pub(super) fn beside_cat(&self) -> Option<Vec3> {
        let cat = self.visitors.cat.as_ref()?;
        if cat.departing.is_some() {
            return None;
        }
        let rest = cat.rest.position();
        let gap = if self.life.cat_trust >= 0.5 { 0.55 } else { 0.95 };
        let (low, high) = home::yard_bounds();
        let spot = rest + Vec3::new(-gap, 0.0, 0.25);
        Some(Vec3::new(
            spot.x.clamp(low.x, high.x),
            (rest.y + 0.38 * self.size()).max(0.38 * self.size() + 0.05),
            spot.z.clamp(low.z, high.z),
        ))
    }

    /// The cat in words for the minds.
    pub fn cat_words(&self) -> String {
        let Some(cat) = &self.visitors.cat else {
            return match self.life.cat_coat {
                Some(coat) => format!("the {} cat isn't visiting just now", coat.words()),
                None => "no cat has visited the yard yet".to_owned(),
            };
        };
        let coat = cat.coat.words();
        let doing = match cat.departing {
            Some((_, _, true)) => "dashing off over the fence".to_owned(),
            Some(_) => "wandering off home".to_owned(),
            None if cat.napping(self.time) => format!("napping {}", cat.rest.words()),
            None if cat.settled(self.time) => format!("sitting {}", cat.rest.words()),
            None => "padding into the yard".to_owned(),
        };
        let trust = match self.life.cat_trust {
            trust if trust < 0.2 => "wary of you",
            trust if trust < 0.5 => "getting used to you",
            _ => "your friend",
        };
        format!("a {coat} cat, {trust}, is {doing}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visit(world: &mut World) {
        world.visitors.next_cat = world.time;
        world.advance_cat(0.0);
    }

    #[test]
    fn a_cat_comes_in_settles_naps_and_wanders_off_within_its_visit() {
        let mut world = World::new(4, 12.0);
        visit(&mut world);
        let cat = world.visitors.cat.unwrap();
        let mut settled = false;
        let mut napped = false;
        for _ in 0..(600.0 / 0.5) as usize {
            world.time += 0.5;
            world.advance_cat(0.5);
            let Some(cat) = world.visitors.cat else {
                break;
            };
            let picture = cat.picture(world.time);
            assert!(picture.position.is_finite());
            settled |= picture.pose == Pose::Sitting;
            napped |= picture.pose == Pose::Napping;
        }
        assert!(settled && napped);
        assert!(world.visitors.cat.is_none());
        assert_eq!(world.life.cat_coat, Some(cat.coat));
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::CatLeft { startled: false })
        );
    }

    #[test]
    fn rushing_at_a_settled_cat_sends_it_over_the_fence_but_quiet_company_earns_trust() {
        let mut world = World::new(4, 12.0);
        visit(&mut world);
        let mut cat = world.visitors.cat.unwrap();
        cat.came -= 200.0;
        cat.settles_at -= 200.0;
        cat.sleeps_at += 1000.0;
        cat.leaves_at += 1000.0;
        world.visitors.cat = Some(cat);
        let rest = cat.rest.position();
        world.ghost.position = rest + Vec3::new(-1.2, 0.6, 0.0);
        world.ghost.velocity = Vec3::new(1.2, 0.0, 0.0);
        world.time += 0.1;
        world.advance_cat(0.1);
        assert!(world.visitors.cat.unwrap().departing.is_some());

        let mut world = World::new(4, 12.0);
        visit(&mut world);
        let mut cat = world.visitors.cat.unwrap();
        cat.came -= 200.0;
        cat.settles_at -= 200.0;
        cat.sleeps_at += 5000.0;
        cat.leaves_at += 5000.0;
        world.visitors.cat = Some(cat);
        world.ghost.position = world.beside_cat().unwrap();
        world.ghost.velocity = Vec3::ZERO;
        for _ in 0..600 {
            world.time += 1.0;
            world.advance_cat(1.0);
        }
        assert!(world.visitors.cat.unwrap().departing.is_none());
        assert!(world.life.cat_trust > 0.3);
    }

    #[test]
    fn cats_only_come_by_day_in_fair_weather() {
        let mut world = World::new(4, 23.0);
        visit(&mut world);
        assert!(world.visitors.cat.is_none());
        let mut world = World::new(4, 12.0);
        world.weather.sky = Sky::Rain;
        visit(&mut world);
        assert!(world.visitors.cat.is_none());
    }

    #[test]
    fn paths_and_exits_stay_inside_or_lead_over_the_fence() {
        let mut random = Random::new(9);
        for _ in 0..40 {
            let cat = Cat::new(0.0, Coat::Gray, &mut random);
            for point in cat.path() {
                assert!(point.x <= LOT_MAX.x + 1e-4 && point.z >= LOT_MIN.y - 1e-4);
            }
            let (end, _) = along(&cat.path(), 1e6);
            assert!(end.distance(cat.rest.position()) < 1e-4);
            let flat = glam::Vec2::new(end.x, end.z);
            assert!(flat.cmpge(LOT_MIN).all() && flat.cmple(LOT_MAX).all());
        }
    }
}
