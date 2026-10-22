use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, Plant, Random, Season, World};

const HOUR: f32 = 3600.0;
const DAY: f32 = 24.0 * HOUR;
const REST: f32 = DAY * 0.5;
/// How dry a plant must be before the ghost sets out to water the garden.
pub const TENDS_FROM: f32 = 0.6;
/// How dry a plant must be to be watered on a round already under way.
const ROUND_THIRST: f32 = 0.2;

/// Where the seedlings grow in the raised bed out in the yard, set so a
/// pot's height down from each is the soil.
pub const SPOTS: [Vec3; 3] = [
    Vec3::new(
        super::home::GARDEN.x - 0.52,
        super::home::GARDEN_HALF.y * 2.0 - 0.04 - Seedling::POT_HEIGHT,
        super::home::GARDEN.z,
    ),
    Vec3::new(
        super::home::GARDEN.x,
        super::home::GARDEN_HALF.y * 2.0 - 0.04 - Seedling::POT_HEIGHT,
        super::home::GARDEN.z,
    ),
    Vec3::new(
        super::home::GARDEN.x + 0.52,
        super::home::GARDEN_HALF.y * 2.0 - 0.04 - Seedling::POT_HEIGHT,
        super::home::GARDEN.z,
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Flower {
    Daisy,
    Tulip,
    Sunflower,
    Bluebell,
    Poppy,
}

impl Flower {
    pub fn name(self) -> &'static str {
        match self {
            Self::Daisy => "daisy",
            Self::Tulip => "tulip",
            Self::Sunflower => "sunflower",
            Self::Bluebell => "bluebell",
            Self::Poppy => "poppy",
        }
    }

    /// Its name in Korean.
    pub fn korean(self) -> &'static str {
        match self {
            Self::Daisy => "데이지",
            Self::Tulip => "튤립",
            Self::Sunflower => "해바라기",
            Self::Bluebell => "초롱꽃",
            Self::Poppy => "양귀비",
        }
    }

    fn choose(random: &mut Random) -> Self {
        let kinds = [
            Self::Daisy,
            Self::Tulip,
            Self::Sunflower,
            Self::Bluebell,
            Self::Poppy,
        ];
        kinds[random.pick(&[1.0; 5]).unwrap()]
    }
}

/// A pot's position is its place in `World::seedlings`, never a saved coordinate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Seedling {
    pub kind: Flower,
    pub growth: f32,
    pub thirst: f32,
    pub droop: f32,
    /// Seconds since planting, kept relative to the plant rather than a session.
    pub age: f64,
    /// A watering's growth waits until a drooping stem has lifted.
    #[serde(default)]
    pending_growth: bool,
}

impl Seedling {
    pub const POT_RADIUS: f32 = 0.11;
    pub const POT_HEIGHT: f32 = 0.13;
    pub const SPROUT: f32 = 0.08;
    pub const BUD: f32 = 0.75;

    pub fn new(kind: Flower) -> Self {
        Self {
            kind,
            growth: 0.0,
            thirst: 0.0,
            droop: 0.0,
            age: 0.0,
            pending_growth: false,
        }
    }

    pub fn sanitized(mut self) -> Self {
        self.growth = unit(self.growth);
        self.thirst = unit(self.thirst);
        self.droop = unit(self.droop);
        self.age = if self.age.is_finite() {
            self.age.clamp(0.0, 100.0 * 366.0 * f64::from(DAY))
        } else {
            0.0
        };
        self
    }

    pub fn height(&self) -> f32 {
        let grown = match self.kind {
            Flower::Sunflower => 0.65,
            Flower::Bluebell => 0.42,
            _ => 0.36,
        };
        Self::POT_HEIGHT + grown * self.growth
    }

    pub fn flowering(&self, season: Season) -> bool {
        self.growth >= 1.0 && self.droop == 0.0 && season != Season::Winter
    }

    pub fn words(&self, season: Season) -> String {
        let kind = self.kind.name();
        let stage = if self.growth < Self::SPROUT {
            format!("a {kind} seed in bare soil")
        } else if self.growth < 0.35 {
            format!("a {kind} sprout")
        } else if self.growth < Self::BUD {
            format!("a leafy young {kind}")
        } else if self.growth < 1.0 {
            format!("a {kind} in bud")
        } else if season == Season::Winter {
            format!("a {kind} resting for winter")
        } else if self.droop > 0.0 {
            format!("a {kind} with closed flowers")
        } else {
            format!("a {kind} in flower")
        };
        if self.droop > 0.0 {
            format!("{stage}, drooping")
        } else if self.thirst > 0.7 {
            format!("{stage}, thirsty")
        } else {
            stage
        }
    }

    fn grow(&mut self, amount: f32) {
        // Water cannot hurry a seed straight into a flower in one sitting.
        let maturity = (self.age / f64::from(4.0 * DAY)).min(1.0) as f32;
        self.growth = (self.growth + amount)
            .min(1.0)
            .min(maturity.max(self.growth));
    }

    fn advance(&mut self, seconds: f32, season: Season, drying: f32) {
        self.age = (self.age + f64::from(seconds)).min(100.0 * 366.0 * f64::from(DAY));
        let growing = care(&mut self.thirst, &mut self.droop, seconds, drying);
        if season != Season::Winter {
            self.grow(growing / (6.0 * DAY) * spring(season));
            if self.pending_growth && self.droop == 0.0 && self.thirst < 0.6 {
                self.grow(0.04 * spring(season));
                self.pending_growth = false;
            }
        }
    }

    fn water(&mut self, season: Season) {
        self.thirst = 0.0;
        if season != Season::Winter {
            if self.droop == 0.0 {
                self.grow(0.04 * spring(season));
                self.pending_growth = false;
            } else {
                self.pending_growth = true;
            }
        }
    }
}

/// A fallen flower petal; ephemeral and never saved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Petal {
    pub position: Vec3,
    pub velocity: Vec3,
    pub spin: f32,
    pub dropped: f64,
}

impl Petal {
    pub const LIFETIME: f64 = 60.0;

    fn drift(&mut self, seconds: f32, breeze: Vec3) {
        self.spin += seconds * 1.7;
        self.velocity.y = (self.velocity.y - seconds * 0.12).max(-0.12);
        self.position +=
            (self.velocity + breeze * if self.position.y > 0.02 { 1.5 } else { 0.1 }) * seconds;
        self.position = self.position.clamp(
            super::ROOM_MIN + Vec3::splat(0.01),
            super::ROOM_MAX - Vec3::splat(0.01),
        );
        if self.position.y < 0.012 {
            self.position.y = 0.012;
            self.velocity *= (-seconds * 4.0).exp();
        }
    }
}

fn unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn spring(season: Season) -> f32 {
    if season == Season::Spring { 1.2 } else { 1.0 }
}

// Returns well-watered time after any droop has lifted. Plants droop from
// thirst at `drying` times the daytime pace: not at all by night, while the
// ghost sleeps, and at half pace over a long absence of days and nights.
fn care(thirst: &mut f32, droop: &mut f32, seconds: f32, drying: f32) -> f32 {
    let wet = ((0.6 - *thirst) * Plant::THIRST_TIME).clamp(0.0, seconds);
    let dry = (seconds - (1.0 - *thirst) * Plant::THIRST_TIME).max(0.0);
    let recovery = *droop * 60.0;
    *droop = (*droop - wet / 60.0).max(0.0);
    *droop = (*droop + dry * drying / (6.0 * HOUR)).min(1.0);
    *thirst = (*thirst + seconds / Plant::THIRST_TIME).min(1.0);
    (wet - recovery).max(0.0)
}

impl Plant {
    pub fn sanitized(mut self, position: Vec3) -> Self {
        self.position = position;
        self.growth = unit(self.growth);
        self.thirst = unit(self.thirst);
        self.blossom = unit(self.blossom);
        self.droop = unit(self.droop);
        self.seeds = self.seeds.min(3);
        self.bloom_age = if self.bloom_age.is_finite() {
            self.bloom_age.clamp(-REST, DAY + HOUR)
        } else {
            0.0
        };
        if self.growth < Self::BLOOM {
            self.blossom = 0.0;
            self.bloom_age = 0.0;
        }
        self
    }

    fn water(&mut self, season: Season) {
        self.thirst = 0.0;
        if self.droop == 0.0 && season != Season::Winter {
            self.growth = (self.growth + 0.05 * spring(season)).min(1.0);
        }
    }

    /// Advances the plant without any random draws. Returns bloom, droop, and
    /// petal crossings; the world records events and gives petals their drift.
    fn advance(&mut self, seconds: f32, season: Season, drying: f32) -> (bool, bool, u32) {
        if self.thirst >= 1.0 && self.blossom == 0.0 {
            let before = self.droop;
            self.droop = (self.droop + seconds * drying / (6.0 * HOUR)).min(1.0);
            // A resting plant goes on resting; one about to open waits for water.
            if self.bloom_age < 0.0 {
                self.bloom_age = (self.bloom_age + seconds).min(0.0);
            }
            return (false, before <= 0.5 && self.droop > 0.5, 0);
        }
        let before_droop = self.droop;
        let growing = care(&mut self.thirst, &mut self.droop, seconds, drying);
        let drooped = before_droop <= 0.5 && self.droop > 0.5;
        if season != Season::Winter {
            self.growth = (self.growth + growing * 0.003 / HOUR * spring(season)).min(1.0);
        }
        let before = self.blossom;
        let age = self.bloom_age;
        // Open flowers outlast a dry spell; only real drooping closes them.
        if season == Season::Winter || self.droop > 0.5 {
            self.blossom = (self.blossom - seconds / HOUR).max(0.0);
            if self.blossom < 1e-6 {
                self.blossom = 0.0;
            }
            self.bloom_age = -REST;
        } else if self.growth >= Self::BLOOM {
            if self.bloom_age < 0.0 {
                self.bloom_age = (self.bloom_age + seconds).min(0.0);
            } else if growing > 0.0 || self.bloom_age >= HOUR {
                self.bloom_age += if self.bloom_age < HOUR {
                    growing
                } else {
                    seconds
                };
                if self.bloom_age >= DAY + HOUR {
                    self.seeds = (self.seeds + 1).min(3);
                    self.blossom = 0.0;
                    self.bloom_age = -REST;
                } else if self.bloom_age >= DAY {
                    self.blossom = (1.0 - (self.bloom_age - DAY) / HOUR).clamp(0.0, 1.0);
                } else {
                    self.blossom = (self.bloom_age / HOUR).min(1.0);
                }
            }
        }
        let bloomed = age <= HOUR * 0.5 && self.bloom_age > HOUR * 0.5 && self.blooming();
        let petals = ((before * 5.0).ceil() - (self.blossom * 5.0).ceil()).max(0.0) as u32;
        (bloomed, drooped, petals)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Tending {
    Water(Option<usize>),
    Seed,
    Plant(usize),
}

impl World {
    pub fn garden_words(&self) -> String {
        let descriptions: Vec<String> = self
            .seedlings
            .iter()
            .map(|plant| plant.words(self.season()))
            .collect();
        let contents = match descriptions.as_slice() {
            [] => return "nothing planted in the garden bed yet".to_owned(),
            [one] => one.clone(),
            [one, two] => format!("{one} and {two}"),
            [one, two, three, ..] => format!("{one}, {two}, and {three}"),
        };
        let count = match descriptions.len() {
            1 => "one plant in the garden bed",
            2 => "two plants in the garden bed",
            _ => "three plants in the garden bed",
        };
        format!("{count}: {contents}")
    }

    pub fn seed_ready(&self) -> bool {
        self.plant.seeds > 0 && self.seedlings.len() < SPOTS.len()
    }

    pub fn garden_thirst(&self) -> f32 {
        self.seedlings
            .iter()
            .fold(self.plant.thirst, |thirst, plant| thirst.max(plant.thirst))
    }

    pub fn garden_droop(&self) -> f32 {
        self.seedlings
            .iter()
            .fold(self.plant.droop, |droop, plant| droop.max(plant.droop))
    }

    pub fn garden_recovering(&self) -> bool {
        (self.plant.droop > 0.0 && self.plant.thirst < 0.3)
            || self
                .seedlings
                .iter()
                .any(|plant| plant.droop > 0.0 && plant.thirst < 0.3)
    }

    /// The watering can while it waters: where the can is, where the water
    /// lands, and how far it is tipped, from 0 upright to 1 pouring.
    pub fn watering(&self) -> Option<(Vec3, Vec3, f32)> {
        let Some(Tending::Water(plant)) = self.tending else {
            return None;
        };
        if self.ghost.plan.done || self.ghost.plan.activity != Activity::Tend {
            return None;
        }
        let landing = match plant {
            Some(index) => SPOTS[index] + Vec3::Y * (self.seedlings[index].height() + 0.02),
            None => self.plant.top(),
        };
        // Held out at its side, toward the front, so it shows beside it.
        let size = self.size();
        let can = self.ghost.position + Vec3::new(0.36, -0.12, 0.12) * size;
        let pour = (self.ghost.plan.stayed(self.time) / 0.8).clamp(0.0, 1.0)
            * (1.0 - ((self.ghost.plan.stayed(self.time) - 4.6) / 0.4).clamp(0.0, 1.0));
        Some((can, landing, pour))
    }

    pub fn carried_seed(&self) -> Option<Vec3> {
        (!self.ghost.plan.done
            && self.ghost.plan.activity == Activity::Tend
            && matches!(self.tending, Some(Tending::Plant(_))))
        .then_some(self.ghost.position - Vec3::Y * 0.42)
    }

    fn thirstiest(&self) -> Option<usize> {
        let mut thirst = self.plant.thirst;
        let mut chosen = None;
        for (index, plant) in self.seedlings.iter().enumerate() {
            if plant.thirst > thirst {
                thirst = plant.thirst;
                chosen = Some(index);
            }
        }
        chosen
    }

    /// The thirstiest plant still worth watering on a round already under
    /// way, if any.
    fn thirsty_for_round(&self) -> Option<Option<usize>> {
        let plant = self.thirstiest();
        let thirst = plant.map_or(self.plant.thirst, |index| self.seedlings[index].thirst);
        (thirst > ROUND_THIRST).then_some(plant)
    }

    pub(super) fn tend_target(&self) -> Vec3 {
        self.water_target(self.thirstiest())
    }

    fn water_target(&self, plant: Option<usize>) -> Vec3 {
        match plant {
            Some(index) => {
                SPOTS[index] + Vec3::Y * (self.seedlings[index].height() + 0.35) + Vec3::Z * 0.3
            }
            None => self.plant.top() + Vec3::new(0.0, 0.25, 0.3),
        }
    }

    pub fn tend_description(&self) -> String {
        let plant = self.thirstiest().map_or_else(
            || "the potted plant".to_owned(),
            |index| format!("the {} in the garden bed", self.seedlings[index].kind.name()),
        );
        let seeds = if self.seed_ready() {
            " A ripe seed is ready to plant out in the garden bed."
        } else {
            ""
        };
        format!("Water and care for {plant}, the thirstiest plant.{seeds}")
    }

    pub(super) fn begin_tend(&mut self) {
        self.tending = Some(Tending::Water(self.thirstiest()));
    }

    pub(super) fn carry_out_tend(&mut self) -> Option<Vec3> {
        let stayed = self.ghost.plan.stayed(self.time);
        let season = self.season();
        match self.tending? {
            Tending::Water(plant) => {
                let target = self.water_target(plant);
                if stayed > 5.0 {
                    match plant {
                        Some(index) => {
                            let before = self.seedlings[index].growth;
                            self.seedlings[index].water(season);
                            self.note_growth(index, before);
                        }
                        None => self.plant.water(season),
                    }
                    self.ghost.needs.change(0.0, 0.08, 0.0);
                    self.record(EventKind::Tended);
                    // One round waters every plant that has begun to dry,
                    // rather than a trip for each as it crosses the line.
                    if let Some(next) = self.thirsty_for_round() {
                        self.tending = Some(Tending::Water(next));
                        self.ghost.plan.target = self.water_target(next);
                        self.ghost.plan.arrived = None;
                        self.ghost.plan.started = self.time;
                    } else if self.seed_ready() && self.random.unit() < 0.65 {
                        self.tending = Some(Tending::Seed);
                        self.ghost.plan.target = self.plant.top() + Vec3::new(0.0, 0.25, 0.3);
                        self.ghost.plan.arrived = None;
                    } else {
                        self.ghost.plan.done = true;
                    }
                }
                Some(target - Vec3::Y * 0.25)
            }
            Tending::Seed => {
                if !self.seed_ready() {
                    self.ghost.plan.done = true;
                } else if stayed > 1.0 {
                    let spot = self.seedlings.len();
                    self.tending = Some(Tending::Plant(spot));
                    self.ghost.plan.target = SPOTS[spot] + Vec3::new(0.0, 0.72, 0.2);
                    self.ghost.plan.arrived = None;
                }
                Some(self.plant.top())
            }
            Tending::Plant(spot) => {
                if stayed > 3.0 {
                    if self.seed_ready() && spot == self.seedlings.len() {
                        let kind = Flower::choose(&mut self.random);
                        self.plant.seeds -= 1;
                        self.seedlings.push(Seedling::new(kind));
                        self.record(EventKind::Planted(kind));
                    }
                    self.ghost.plan.done = true;
                }
                Some(SPOTS[spot])
            }
        }
    }

    fn note_growth(&mut self, index: usize, before: f32) {
        let plant = &self.seedlings[index];
        let kind = plant.kind;
        let sprouted = before < Seedling::SPROUT && plant.growth >= Seedling::SPROUT;
        let flowered = before < 1.0 && plant.growth >= 1.0;
        if sprouted {
            self.record(EventKind::Sprouted(kind));
        }
        if flowered {
            self.record(EventKind::Flowered(kind));
        }
    }

    pub(super) fn advance_garden(&mut self, seconds: f32) {
        if self.plant.thirst >= 1.0
            && self.plant.droop >= 1.0
            && self.plant.blossom == 0.0
            && self.seedlings.is_empty()
            && self.petals.is_empty()
        {
            return;
        }
        let season = self.season();
        let drying = if self.is_night() { 0.0 } else { 1.0 };
        let (bloomed, drooped, petals) = self.plant.advance(seconds, season, drying);
        if bloomed {
            self.record(EventKind::Bloomed);
        }
        if drooped {
            self.record(EventKind::Drooped);
            if self.ghost.asleep < 0.3 {
                self.think("Oh no, the plant is drooping.");
            }
        }
        for _ in 0..petals.min(12usize.saturating_sub(self.petals.len()) as u32) {
            self.petals.push(Petal {
                position: self.plant.top(),
                velocity: Vec3::new(
                    self.random.range(-0.025, 0.025),
                    -0.03,
                    self.random.range(-0.025, 0.025),
                ),
                spin: self.random.range(0.0, std::f32::consts::TAU),
                dropped: self.time,
            });
        }
        if !self.petals.is_empty() {
            for petal in &mut self.petals {
                petal.drift(
                    seconds,
                    super::breeze::breeze(self.window_open, &self.wind, self.time, petal.position)
                        + self.jostle.stir(petal.position, petal.spin),
                );
            }
            self.petals
                .retain(|petal| self.time - petal.dropped < Petal::LIFETIME);
        }
        for index in 0..self.seedlings.len() {
            let before = self.seedlings[index].growth;
            self.seedlings[index].advance(seconds, season, drying);
            self.note_growth(index, before);
        }
    }

    /// Only the garden ages during an absence. Minute slices bound work even
    /// for an old save; nothing is watered, moved, or planted while away.
    pub fn age_garden(&mut self, seconds: f32) {
        // A remembered absence cannot make a wish come true.
        let wish = self.life.wish.take();
        let mut remaining = seconds.clamp(0.0, 7.0 * DAY);
        let season = self.season();
        while remaining > 0.0 {
            let slice = remaining.min(60.0);
            remaining -= slice;
            let (bloomed, drooped, _) = self.plant.advance(slice, season, 0.5);
            if bloomed {
                self.record(EventKind::Bloomed);
            }
            if drooped {
                self.record(EventKind::Drooped);
            }
            for index in 0..self.seedlings.len() {
                let before = self.seedlings[index].growth;
                self.seedlings[index].advance(slice, season, 0.5);
                self.note_growth(index, before);
            }
        }
        self.life.wish = wish;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Mood, ROOM_MAX, ROOM_MIN, Thing};

    fn care_for(plant: &mut Plant, seconds: f32, season: Season) -> (u32, u32) {
        let mut remaining = seconds;
        let mut blooms = 0;
        let mut petals = 0;
        while remaining > 0.0 {
            let slice = remaining.min(60.0);
            remaining -= slice;
            plant.thirst = 0.0;
            let (bloomed, _, dropped) = plant.advance(slice, season, 1.0);
            blooms += u32::from(bloomed);
            petals += dropped;
        }
        (blooms, petals)
    }

    fn grown() -> Plant {
        Plant {
            growth: 1.0,
            ..Plant::default()
        }
    }

    #[test]
    fn watered_flowers_open_hold_fade_set_a_seed_and_rest_before_opening_again() {
        let mut plant = grown();
        assert_eq!(care_for(&mut plant, HOUR, Season::Summer).0, 1);
        assert!(plant.blooming());
        assert_eq!(plant.blossom, 1.0);
        assert_eq!(care_for(&mut plant, DAY - HOUR, Season::Summer).0, 0);
        assert_eq!(plant.blossom, 1.0);
        assert_eq!(care_for(&mut plant, HOUR * 0.5, Season::Summer).0, 0);
        assert!((plant.blossom - 0.5).abs() < 0.01);
        assert_eq!(plant.seeds, 0);
        care_for(&mut plant, HOUR * 0.5, Season::Summer);
        assert_eq!(plant.blossom, 0.0);
        assert_eq!(plant.seeds, 1);
        assert_eq!(plant.bloom_age, -REST);
        care_for(&mut plant, REST, Season::Summer);
        assert_eq!(plant.blossom, 0.0);
        assert_eq!(care_for(&mut plant, HOUR, Season::Summer).0, 1);
        plant.seeds = 3;
        care_for(&mut plant, DAY, Season::Summer);
        assert_eq!(plant.seeds, 3);
    }

    #[test]
    fn winter_closes_open_flowers_without_starting_another_spell_or_setting_seeds() {
        let mut plant = grown();
        care_for(&mut plant, HOUR, Season::Summer);
        care_for(&mut plant, HOUR, Season::Winter);
        assert_eq!(plant.blossom, 0.0);
        assert_eq!(care_for(&mut plant, DAY * 2.0, Season::Winter).0, 0);
        assert_eq!(plant.seeds, 0);
        assert!(plant.bloom_age < 0.0);
    }

    #[test]
    fn a_plant_watered_by_day_and_left_overnight_still_sets_seeds() {
        let mut plant = grown();
        for day in 0..4 {
            for minute in 0..24 * 60 {
                let hour = minute as f32 / 60.0;
                let awake = (6.5..22.0).contains(&hour);
                if awake {
                    plant.water(Season::Summer);
                }
                let night = !(6.5..19.5).contains(&hour);
                plant.advance(60.0, Season::Summer, if night { 0.0 } else { 1.0 });
            }
            assert!(plant.droop < 0.5, "day {day}");
        }
        assert!(plant.seeds >= 1, "four ordinary days set no seed");
    }

    #[test]
    fn a_short_dry_spell_leaves_open_flowers_and_a_resting_plant_keeps_resting() {
        let mut plant = grown();
        plant.bloom_age = 3.0 * HOUR;
        plant.blossom = 1.0;
        plant.thirst = 1.0;
        plant.advance(2.0 * HOUR, Season::Summer, 1.0);
        assert!(plant.droop > 0.0 && plant.droop <= 0.5);
        assert_eq!(plant.blossom, 1.0);
        assert!(plant.bloom_age > 3.0 * HOUR);
        let mut resting = grown();
        resting.bloom_age = -REST;
        resting.thirst = 1.0;
        resting.advance(HOUR, Season::Summer, 0.0);
        assert_eq!(resting.bloom_age, -REST + HOUR);
        assert_eq!(resting.droop, 0.0);
    }

    #[test]
    fn only_full_thirst_causes_droop_and_water_lifts_it_over_a_minute() {
        let mut plant = Plant {
            thirst: 0.9,
            growth: 0.5,
            ..Plant::default()
        };
        plant.advance(60.0, Season::Summer, 1.0);
        assert_eq!(plant.droop, 0.0);
        plant.thirst = 1.0;
        let growth = plant.growth;
        plant.advance(6.0 * HOUR, Season::Summer, 1.0);
        assert_eq!(plant.droop, 1.0);
        assert_eq!(plant.growth, growth);
        plant.water(Season::Spring);
        assert_eq!(plant.growth, growth);
        plant.advance(30.0, Season::Summer, 1.0);
        assert!((plant.droop - 0.5).abs() < 1e-5);
        assert_eq!(plant.growth, growth);
        plant.advance(30.0, Season::Summer, 1.0);
        assert_eq!(plant.droop, 0.0);
    }

    #[test]
    fn spring_speeds_watered_growth_and_winter_leaves_it_resting() {
        let mut summer = Plant::default();
        let mut spring = summer;
        let mut winter = summer;
        summer.water(Season::Summer);
        spring.water(Season::Spring);
        winter.water(Season::Winter);
        assert!(spring.growth > summer.growth);
        assert_eq!(winter.growth, 0.0);
        let before = summer.growth;
        care_for(&mut summer, HOUR, Season::Summer);
        assert!(summer.growth > before);
    }

    fn finish_tend(world: &mut World) {
        for _ in 0..2400 {
            world.step(0.1);
            if world.ghost.plan.done {
                return;
            }
            let (low, high) = super::super::home::yard_bounds();
            assert!(world.ghost.position.cmpge(ROOM_MIN.min(low)).all());
            assert!(world.ghost.position.cmple(ROOM_MAX.max(high)).all());
        }
        panic!("tending did not finish");
    }

    fn plant_one(world: &mut World) {
        let before = world.seedlings.len();
        for _ in 0..12 {
            world.begin(Activity::Tend);
            finish_tend(world);
            if world.seedlings.len() > before {
                return;
            }
        }
        panic!("the ghost never chose to plant its seed");
    }

    #[test]
    fn tending_plants_ripe_seeds_and_marks_the_first_seed_and_full_garden_once() {
        let mut world = World::new(7, 12.0);
        world.plant.seeds = 3;
        for _ in 0..3 {
            plant_one(&mut world);
        }
        assert_eq!(world.plant.seeds, 0);
        assert_eq!(world.life.total.planted, 3);
        assert_eq!(world.life.today.planted, 3);
        assert_eq!(world.seedlings.len(), 3);
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == "I planted my first seed.")
                .count(),
            1
        );
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == "My garden has three little pots.")
                .count(),
            1
        );
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::Planted(_)))
                .count(),
            3
        );
        assert_eq!(world.ghost.mood, Mood::Cheerful);
        world.plant.seeds = 1;
        world.begin(Activity::Tend);
        finish_tend(&mut world);
        assert_eq!(world.plant.seeds, 1);
        assert_eq!(world.seedlings.len(), 3);
        assert_eq!(world.life.total.planted, 3);
    }

    #[test]
    fn interrupting_a_carried_seed_leaves_it_with_the_main_plant() {
        let mut world = World::new(8, 12.0);
        world.plant.seeds = 1;
        world.begin(Activity::Tend);
        world.tending = Some(Tending::Plant(0));
        assert!(world.carried_seed().is_some());
        world.begin(Activity::Explore);
        world.step(0.1);
        assert!(world.carried_seed().is_none());
        assert!(world.seedlings.is_empty());
        assert_eq!(world.plant.seeds, 1);
        assert_eq!(world.life.total.planted, 0);
    }

    #[test]
    fn tending_starts_at_the_thirstiest_pot_and_waters_the_drying_ones_on_its_round() {
        let mut world = World::new(9, 12.0);
        world.plant.thirst = 0.3;
        world.seedlings = vec![
            Seedling::new(Flower::Daisy),
            Seedling::new(Flower::Tulip),
            Seedling::new(Flower::Poppy),
        ];
        world.seedlings[0].thirst = 0.5;
        world.seedlings[1].thirst = 0.95;
        world.seedlings[2].thirst = 0.1;
        assert_eq!(world.thirstiest(), Some(1));
        assert!(world.options().contains(&Activity::Tend));
        assert!(world.describe(Activity::Tend).contains("tulip"));
        world.begin(Activity::Tend);
        assert_eq!(world.tending, Some(Tending::Water(Some(1))));
        finish_tend(&mut world);
        assert!(world.seedlings[1].thirst < 0.1);
        assert!(world.seedlings[0].thirst < 0.1);
        assert!(world.plant.thirst < 0.1);
        assert!(world.seedlings[2].thirst >= 0.1);
        let watered = world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Tended)
            .count();
        assert_eq!(watered, 3);
    }

    #[test]
    fn a_garden_only_a_little_dry_waits_for_its_round() {
        let mut world = World::new(9, 12.0);
        world.plant.thirst = TENDS_FROM - 0.05;
        world.seedlings = vec![Seedling::new(Flower::Daisy)];
        world.seedlings[0].thirst = 0.3;
        assert!(!world.options().contains(&Activity::Tend));
        world.seedlings[0].thirst = TENDS_FROM + 0.05;
        assert!(world.options().contains(&Activity::Tend));
    }

    #[test]
    fn seedlings_need_days_and_care_and_cross_each_growth_event_only_once() {
        let mut world = World::new(10, 12.0);
        world.seedlings.push(Seedling::new(Flower::Poppy));
        for _ in 0..30 {
            world.seedlings[0].water(Season::Summer);
        }
        assert_eq!(world.seedlings[0].growth, 0.0);
        for _ in 0..25 {
            for _ in 0..5 * 60 {
                world.time += 60.0;
                world.advance_garden(60.0);
            }
            let before = world.seedlings[0].growth;
            world.seedlings[0].water(Season::Summer);
            world.note_growth(0, before);
        }
        world.advance_garden(60.0);
        assert_eq!(world.seedlings[0].growth, 1.0);
        assert_eq!(
            (world.life.total.sprouted, world.life.total.flowered),
            (1, 1)
        );
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::Sprouted(_)))
                .count(),
            1
        );
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::Flowered(_)))
                .count(),
            1
        );
        assert!(
            world
                .life
                .today
                .words()
                .contains("the poppy came into flower")
        );
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "Something is growing in the little pot!")
        );
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "The first flower I grew from a seed is blooming.")
        );
        world.seedlings[0].thirst = 1.0;
        world.seedlings[0].advance(6.0 * HOUR, Season::Summer, 1.0);
        assert_eq!(world.seedlings[0].droop, 1.0);
        let growth = world.seedlings[0].growth;
        world.seedlings[0].water(Season::Summer);
        assert_eq!(world.seedlings[0].growth, growth);
        world.seedlings[0].advance(60.0, Season::Summer, 1.0);
        assert_eq!(world.seedlings[0].droop, 0.0);
        assert!(!world.seedlings[0].flowering(Season::Winter));
        assert!(world.seedlings[0].flowering(Season::Summer));
    }

    #[test]
    fn drooping_is_noted_once_until_the_plant_recovers_and_droops_again() {
        let mut world = World::new(11, 12.0);
        world.plant.thirst = 1.0;
        world.advance_garden(4.0 * HOUR);
        world.advance_garden(HOUR);
        assert_eq!(world.life.total.drooped, 1);
        world.plant.water(Season::Summer);
        world.advance_garden(60.0);
        world.plant.thirst = 1.0;
        world.advance_garden(4.0 * HOUR);
        assert_eq!(world.life.total.drooped, 2);
    }

    #[test]
    fn falling_petals_are_capped_and_expire_and_pots_are_obstacles_and_visible_things() {
        let mut world = World::new(12, 12.0);
        world.plant.growth = 1.0;
        for _ in 0..8 {
            world.plant.blossom = 1.0;
            world.plant.bloom_age = DAY + HOUR - 60.0;
            world.plant.thirst = 0.0;
            world.advance_garden(60.0);
            assert!(world.petals.len() <= 12);
        }
        assert_eq!(world.petals.len(), 12);
        world.time += Petal::LIFETIME;
        world.advance_garden(0.1);
        assert!(world.petals.is_empty());
        assert_eq!(world.position_of(Thing::Seedlings), None);
        world.seedlings.push(Seedling::new(Flower::Bluebell));
        assert!(
            world
                .obstacles()
                .contains(&(SPOTS[0], Seedling::POT_RADIUS))
        );
        world.attend("garden", 20.0);
        assert!(world.position_of(Thing::Seedlings).is_some());
        assert_eq!(
            world.ghost.attention.map(|(thing, _)| thing),
            Some(Thing::Seedlings)
        );
        world.date = jiff::civil::Date::constant(2026, 10, 31);
        for spot in SPOTS {
            for (position, radius) in world
                .obstacles()
                .into_iter()
                .filter(|(position, _)| !SPOTS.contains(position))
            {
                assert!(
                    super::super::flat(spot - position).length() > radius + Seedling::POT_RADIUS
                );
            }
        }
    }

    #[test]
    fn the_same_seed_grows_the_same_garden_and_drops_the_same_petals() {
        let mut first = World::new(13, 12.0);
        let mut second = World::new(13, 12.0);
        for world in [&mut first, &mut second] {
            world.plant.seeds = 3;
            for _ in 0..3 {
                plant_one(world);
            }
            world.plant.growth = 1.0;
            world.plant.blossom = 1.0;
            world.plant.bloom_age = DAY + HOUR - 60.0;
            world.plant.thirst = 0.0;
            world.advance_garden(60.0);
        }
        assert_eq!(first.seedlings, second.seedlings);
        assert_eq!(first.plant, second.plant);
        assert_eq!(first.petals, second.petals);
        assert_eq!(first.events, second.events);
    }
}
