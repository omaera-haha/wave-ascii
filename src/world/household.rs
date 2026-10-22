//! Several creatures share the cottage. The world holds one of them at a
//! time in its own fields, so everything written for one creature works for
//! each: the one the viewer is with stays in, and the others wait in
//! `housemates` as souls and take their turns in the world as companions.
//!
//! Only the creature the viewer is with runs the household: the weather, the
//! visitors, games and letters with the viewer. Companions only live their
//! own lives in it, among the same things.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::{
    Accessory, Activity, Dream, Event, EventKind, Ghost, Life, Listening, Mood, Occasion, Passerby,
    Plan, Wish, World, books, cold, garden, growth, mirror, quirks, species::Species, wardrobe,
};

/// At most this many creatures live in the cottage.
pub const MOST: usize = 6;

macro_rules! personal {
    ($($(#[$meta:meta])* $field:ident: $type:ty),* $(,)?) => {
        /// The part of a life that is one creature's own, rather than the
        /// household's: its bond with the viewer, its days, its words.
        #[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct Personal {
            $($(#[$meta])* pub $field: $type),*
        }

        impl Personal {
            fn exchange(&mut self, life: &mut Life) {
                $(std::mem::swap(&mut self.$field, &mut life.$field);)*
            }

            fn take(life: &mut Life) -> Self {
                Self { $($field: std::mem::take(&mut life.$field)),* }
            }
        }
    };
}

personal! {
    nature: Option<super::Nature>,
    upbringing: super::Upbringing,
    bond: f32,
    closest: f32,
    warmed: f32,
    total: super::life::Tally,
    today: super::life::Tally,
    day: String,
    diary: Vec<super::Entry>,
    milestones: Vec<super::Entry>,
    anniversary_said: Option<String>,
    landmark_greeted: Option<String>,
    alone: f64,
    vocabulary: super::Vocabulary,
    skills: super::Skills,
    pictures: Vec<super::LearnedPicture>,
    #[serde(deserialize_with = "super::wardrobe::deserialize_accessory")]
    wearing: Option<Accessory>,
    wardrobe_day: Option<String>,
    wardrobe_occasion: Option<Occasion>,
    tunes: Vec<super::music::Tune>,
    compositions: Vec<super::music::Composition>,
    composition_day: Option<String>,
    wish: Option<super::Wishing>,
    granted: Vec<super::Granted>,
    mirror: mirror::MirrorLife,
    yesterday: Vec<super::Entry>,
    #[serde(deserialize_with = "super::cold::deserialize")]
    cold: Option<cold::Cold>,
    last_cold: Option<String>,
    cold_recovered: bool,
    fondness: Vec<super::together::Fondness>,
}

impl Personal {
    /// Brought back in range, the way a whole saved life is.
    pub fn sanitized(mut self) -> Self {
        let mut life = Life::default();
        self.exchange(&mut life);
        let mut life = life.sanitized();
        self.exchange(&mut life);
        self
    }
}

macro_rules! soul {
    ($($field:ident: $type:ty),* $(,)?) => {
        /// One creature's own part of the world, kept aside while another
        /// creature has the world.
        #[derive(Clone, Debug)]
        pub struct Soul {
            pub personal: Personal,
            $(pub(super) $field: $type),*
        }

        impl World {
            /// Trades the creature in the world for the one in `soul`.
            fn exchange(&mut self, soul: &mut Soul) {
                soul.personal.exchange(&mut self.life);
                $(std::mem::swap(&mut self.$field, &mut soul.$field);)*
            }

            /// This world's creature, taken out to live in another world.
            fn into_soul(mut self) -> Soul {
                Soul {
                    personal: Personal::take(&mut self.life),
                    $($field: self.$field),*
                }
            }
        }
    };
}

soul! {
    birthday: Option<jiff::civil::Date>,
    ghost: Ghost,
    events: Vec<Event>,
    last_celebrated: Option<(jiff::civil::Date, Occasion)>,
    calendar_at: Option<jiff::civil::Date>,
    new_year_at: Option<f64>,
    viewer_present: bool,
    viewer_gone: bool,
    absent_since: Option<f64>,
    storm_woke: Option<f64>,
    pointer_interrupted_at: Option<f64>,
    listening: Listening,
    push: Vec3,
    drowsy_until: f64,
    morning_at: Option<f64>,
    petting: f32,
    petted: bool,
    seen_meteor: Option<u64>,
    last_holiday_greeting: Option<(jiff::civil::Date, Occasion)>,
    wished_night: Option<jiff::civil::Date>,
    tending: Option<garden::Tending>,
    dream: Option<Dream>,
    found_constellation_at: Option<f64>,
    saw_puddle: bool,
    seen_passersby: [Option<u64>; Passerby::ALL.len()],
    quirk: Option<quirks::Spell>,
    quirk_triggers: quirks::Triggers,
    wearing: Option<Accessory>,
    wardrobe: wardrobe::Wardrobe,
    reading: Option<books::Reading>,
    last_read: Option<usize>,
    compose_on_arrival: bool,
    pending_wish: Option<Wish>,
    mirror_visit: Option<mirror::Visit>,
    viewer_blush_at: Option<f64>,
    born_at: Option<f64>,
    last_outing: Option<f64>,
    chill: cold::Chill,
}

/// Another creature of the household, as the one in the world sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct Peer {
    pub id: u32,
    pub name: String,
    pub species: Species,
    pub position: Vec3,
    pub velocity: Vec3,
    pub activity: Activity,
    /// Whether it is still at its activity.
    pub busy: bool,
    pub asleep: f32,
    pub mood: Mood,
    pub size: f32,
    /// How faint it has turned, from 0 to 1.
    pub fade: f32,
    /// What it last said, and when.
    pub said: Option<(String, f64)>,
    /// Who it is playing or talking with, if anyone.
    pub with: Option<u32>,
}

impl Peer {
    fn of(ghost: &Ghost, size: f32) -> Self {
        Self {
            id: ghost.id,
            name: ghost.name.clone(),
            species: ghost.species,
            position: ghost.position,
            velocity: ghost.velocity,
            activity: ghost.plan.activity,
            busy: !ghost.plan.done,
            asleep: ghost.asleep,
            mood: ghost.mood,
            size,
            fade: ghost.fade,
            said: ghost
                .speech
                .as_ref()
                .map(|line| (line.text.clone(), line.time)),
            with: ghost.plan.with.filter(|_| !ghost.plan.done),
        }
    }

    /// How far its body reaches out from its middle.
    pub fn reach(&self) -> f32 {
        body_reach(self.species, self.size)
    }
}

/// How far a body reaches out from its middle, across the floor.
pub fn body_reach(species: Species, size: f32) -> f32 {
    let base = match species {
        Species::Ghost | Species::Jelly => 0.22,
        Species::Fox | Species::Dragon | Species::Axolotl => 0.24,
        Species::Robot | Species::Owl | Species::Penguin => 0.17,
        _ => 0.19,
    };
    base * size
}

/// Activities that only the creature the viewer is with may take up: they
/// are played with the viewer, or run things the whole house shares.
pub fn hosted(activity: Activity) -> bool {
    matches!(
        activity,
        Activity::Wait
            | Activity::Fetch
            | Activity::Hide
            | Activity::Pounce
            | Activity::Game
            | Activity::Puzzle
            | Activity::Air
            | Activity::Letter
            | Activity::Comfort
            | Activity::Seek
            | Activity::Show
            | Activity::Pin
            | Activity::Tidy
            | Activity::Look
            | Activity::Bubbles
            | Activity::Friend
            | Activity::Build
            | Activity::Fold
    )
}

/// Activities only one creature at a time can be at, since they use one
/// thing everyone shares.
pub fn exclusive(activity: Activity) -> bool {
    matches!(
        activity,
        Activity::Tend
            | Activity::Music
            | Activity::Mirror
            | Activity::Lamp
            | Activity::Doodle
            | Activity::Read
    ) || hosted(activity)
}

impl World {
    /// Every creature's id, in the order they came to live here.
    pub fn roster(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = std::iter::once(self.ghost.id)
            .chain(self.housemates.iter().map(|soul| soul.ghost.id))
            .collect();
        ids.sort_unstable();
        ids
    }

    /// How many creatures live here.
    pub fn household_size(&self) -> usize {
        1 + self.housemates.len()
    }

    /// The id of the creature the viewer is with.
    pub fn host(&self) -> u32 {
        self.ghost.id
    }

    /// The creature `id`, wherever it is.
    pub fn creature(&self, id: u32) -> Option<&Ghost> {
        if self.ghost.id == id {
            return Some(&self.ghost);
        }
        self.housemates
            .iter()
            .find(|soul| soul.ghost.id == id)
            .map(|soul| &soul.ghost)
    }

    /// Every creature, the one in the world first.
    pub fn creatures(&self) -> impl Iterator<Item = &Ghost> {
        std::iter::once(&self.ghost).chain(self.housemates.iter().map(|soul| &soul.ghost))
    }

    /// The others, as the creature in the world sees them.
    pub fn peers(&self) -> &[Peer] {
        &self.peers
    }

    /// The creature in the world, as the others see it.
    pub fn as_peer(&self) -> Peer {
        Peer::of(&self.ghost, self.size())
    }

    fn soul_peer(&self, soul: &Soul) -> Peer {
        Peer::of(
            &soul.ghost,
            growth::body_size(soul.birthday, self.date, self.hour),
        )
    }

    /// Everyone but the creature in the world and housemate `skip`.
    fn others(&self, skip: Option<usize>) -> Vec<Peer> {
        self.housemates
            .iter()
            .enumerate()
            .filter(|(index, _)| Some(*index) != skip)
            .map(|(_, soul)| self.soul_peer(soul))
            .collect()
    }

    /// Lets the creature in the world see its housemates where they are now.
    pub(super) fn look_at_housemates(&mut self) {
        self.peers = self.others(None);
    }

    /// Runs `act` with creature `id` in the world. A housemate takes its turn
    /// as a companion and goes back to waiting after.
    pub fn with_creature<R>(&mut self, id: u32, act: impl FnOnce(&mut World) -> R) -> Option<R> {
        if self.ghost.id == id {
            return Some(act(self));
        }
        let index = self.housemates.iter().position(|soul| soul.ghost.id == id)?;
        let mut peers: Vec<Peer> = std::iter::once(self.as_peer())
            .chain(self.others(Some(index)))
            .collect();
        let mut soul = self.housemates.remove(index);
        self.exchange(&mut soul);
        std::mem::swap(&mut self.peers, &mut peers);
        let companion = std::mem::replace(&mut self.companion, true);
        let result = act(self);
        self.companion = companion;
        std::mem::swap(&mut self.peers, &mut peers);
        self.exchange(&mut soul);
        self.housemates.insert(index, soul);
        Some(result)
    }

    /// Turns to creature `id`: it becomes the one in the world, the one the
    /// viewer talks and plays with. Whatever the last one was playing with
    /// the viewer ends.
    pub fn select(&mut self, id: u32) -> bool {
        if self.ghost.id == id {
            return true;
        }
        let Some(index) = self.housemates.iter().position(|soul| soul.ghost.id == id) else {
            return false;
        };
        self.hand_over();
        let mut soul = self.housemates.remove(index);
        self.exchange(&mut soul);
        self.housemates.push(soul);
        self.housemates.sort_by_key(|soul| soul.ghost.id);
        self.look_at_housemates();
        // Whatever it was at as a companion goes on; it only becomes able to
        // play with the viewer.
        true
    }

    /// Ends what the creature in the world does with the viewer or with the
    /// whole house, before another takes over.
    fn hand_over(&mut self) {
        self.cancel_hunt();
        self.stop_show();
        self.abandon_comfort();
        self.abandon_puzzle();
        self.abandon_game();
        self.telling = None;
        self.drop_ball();
        self.drop_block();
        self.stop_folding();
        self.put_keepsake_away();
        self.asking = None;
        if hosted(self.ghost.plan.activity) {
            self.ghost.plan.done = true;
        }
        self.ghost.fade = 0.0;
    }

    /// A new creature of `species` called `name` comes to live here. It
    /// arrives at the door and comes in to look around. Returns its id, or
    /// `None` when the house is full.
    pub fn adopt(&mut self, species: Species, name: &str, seed: u64) -> Option<u32> {
        if self.household_size() >= MOST {
            return None;
        }
        let id = self.next_creature;
        self.next_creature += 1;
        let mut newcomer = World::new(seed, self.hour);
        newcomer.date = self.date;
        newcomer.time = self.time;
        newcomer.birthday = Some(self.date);
        newcomer.ghost.id = id;
        newcomer.ghost.species = species;
        newcomer.ghost.genome = super::Genome::roll(species, &mut self.random);
        newcomer.ghost.name = name.to_owned();
        newcomer.ghost.position = super::home::DOOR_OUTSIDE;
        newcomer.ghost.plan = Plan::new(Activity::Explore, self.time, super::home::DOOR_INSIDE, 6.0);
        newcomer.ghost.plan.count = 2;
        newcomer.viewer_gone = self.viewer_gone;
        newcomer.viewer_present = self.viewer_present;
        newcomer.life.begin(&self.date.to_string());
        let names: Vec<String> = self.creatures().map(|ghost| ghost.name.clone()).collect();
        let soul = newcomer.into_soul();
        self.housemates.push(soul);
        self.with_creature(id, |world| {
            world.ghost.speech = None;
            world.record(EventKind::MovedIn { names });
        });
        let name = name.to_owned();
        self.record(EventKind::Newcomer { name, species });
        self.look_at_housemates();
        Some(id)
    }

    /// The household after a fresh start: `count` creatures of different
    /// kinds, the first in the world, all gathered in the living room.
    pub fn found_household(&mut self, species: &[Species]) {
        let Some((&first, rest)) = species.split_first() else {
            return;
        };
        let mut taken = Vec::new();
        self.ghost.species = first;
        self.ghost.genome = super::Genome::roll(first, &mut self.random);
        self.ghost.name = super::species::pick_name(&mut self.random, &taken);
        taken.push(self.ghost.name.clone());
        for (index, &kind) in rest.iter().enumerate() {
            let name = super::species::pick_name(&mut self.random, &taken);
            taken.push(name.clone());
            let seed = self.random.unit().to_bits() as u64 ^ (index as u64 + 1) << 32;
            let Some(id) = self.adopt(kind, &name, seed) else {
                break;
            };
            // Founders are already home rather than arriving.
            let spot = Vec3::new(-1.0 + 1.2 * index as f32, 0.9, 0.3 - 0.4 * index as f32);
            self.with_creature(id, |world| {
                world.events.clear();
                world.ghost.position = spot;
                world.ghost.plan = Plan::new(Activity::Float, world.time, spot, 4.0);
            });
        }
        self.events
            .retain(|event| !matches!(event.kind, EventKind::Newcomer { .. }));
        self.look_at_housemates();
    }

    /// Lets each housemate live through `seconds` as a companion, after the
    /// creature in the world has run the household through them.
    pub(super) fn step_housemates(&mut self, seconds: f32, news: &[EventKind]) {
        let ids: Vec<u32> = self.housemates.iter().map(|soul| soul.ghost.id).collect();
        for id in ids {
            self.with_creature(id, |world| {
                for kind in news {
                    world.record(kind.clone());
                    if *kind == EventKind::Thunder {
                        world.hear_thunder();
                    }
                }
                world.live_as_companion(seconds);
            });
        }
        self.keep_apart(seconds);
        self.look_at_housemates();
    }

    fn live_as_companion(&mut self, seconds: f32) {
        self.begin_day();
        if self.calendar_at != Some(self.date) {
            self.mark_growth();
        }
        self.celebrate();
        self.check_wardrobe();
        self.dress_when_idle();
        let mut remaining = seconds.max(0.0);
        while remaining > 0.0 {
            let slice = remaining.min(0.1);
            remaining -= slice;
            self.advance(slice);
        }
        self.celebrate_wish();
        self.mention_cold_recovery();
    }

    /// Bodies that overlap ease apart. Sleepers stay put and the awake one
    /// moves aside.
    fn keep_apart(&mut self, seconds: f32) {
        let size = self.size();
        let mut bodies: Vec<(Vec3, f32, bool)> = std::iter::once((
            self.ghost.position,
            body_reach(self.ghost.species, size),
            self.ghost.asleep > 0.5,
        ))
        .chain(self.housemates.iter().map(|soul| {
            let peer = self.soul_peer(soul);
            (peer.position, peer.reach(), peer.asleep > 0.5)
        }))
        .collect();
        let mut pushes = vec![Vec2::ZERO; bodies.len()];
        for first in 0..bodies.len() {
            for second in first + 1..bodies.len() {
                let (a, reach_a, still_a) = bodies[first];
                let (b, reach_b, still_b) = bodies[second];
                if (a.y - b.y).abs() > 0.35 {
                    continue;
                }
                let offset = Vec2::new(a.x - b.x, a.z - b.z);
                let room = reach_a + reach_b;
                let distance = offset.length();
                if distance >= room {
                    continue;
                }
                let away = if distance > 1e-4 {
                    offset / distance
                } else {
                    Vec2::new(
                        ((first * 7 + second) as f32).sin(),
                        ((first * 3 + second) as f32).cos(),
                    )
                    .normalize_or(Vec2::X)
                };
                let overlap = room - distance;
                let share = match (still_a, still_b) {
                    (true, true) => (0.5, 0.5),
                    (true, false) => (0.0, 1.0),
                    (false, true) => (1.0, 0.0),
                    (false, false) => (0.5, 0.5),
                };
                pushes[first] += away * overlap * share.0;
                pushes[second] -= away * overlap * share.1;
            }
        }
        let ease = (seconds * 6.0).min(1.0);
        for (index, push) in pushes.into_iter().enumerate() {
            if push == Vec2::ZERO {
                continue;
            }
            let shift = Vec3::new(push.x, 0.0, push.y) * ease;
            bodies[index].0 += shift;
            let ghost = if index == 0 {
                &mut self.ghost
            } else {
                &mut self.housemates[index - 1].ghost
            };
            let moved = ghost.position + shift;
            // Never through a wall.
            if super::home::area(moved) == super::home::area(ghost.position) {
                ghost.position = moved;
            }
        }
    }

    /// Whether another creature is already at `activity`, which only one
    /// can be at a time.
    pub fn claimed(&self, activity: Activity) -> bool {
        exclusive(activity)
            && self
                .peers
                .iter()
                .any(|peer| peer.busy && peer.activity == activity)
    }

    /// Where the creature's sleeping place is among the others': each has its
    /// own spot in the bed or on the cushion.
    pub fn berth(&self) -> Vec2 {
        let place = self
            .roster()
            .iter()
            .position(|&id| id == self.ghost.id)
            .unwrap_or(0);
        match place {
            0 => Vec2::ZERO,
            1 => Vec2::new(0.26, 0.24),
            2 => Vec2::new(-0.26, 0.24),
            3 => Vec2::new(0.0, 0.46),
            4 => Vec2::new(0.28, -0.12),
            _ => Vec2::new(-0.28, -0.12),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn household() -> World {
        let mut world = World::new(5, 12.0);
        world.found_household(&[Species::Fox, Species::Bunny, Species::Owl]);
        world
    }

    #[test]
    fn a_founded_household_has_every_kind_with_its_own_name() {
        let world = household();
        assert_eq!(world.household_size(), 3);
        let species: Vec<_> = world.creatures().map(|ghost| ghost.species).collect();
        assert_eq!(species, [Species::Fox, Species::Bunny, Species::Owl]);
        let mut names: Vec<_> = world.creatures().map(|ghost| ghost.name.clone()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 3);
        assert_eq!(world.roster(), [0, 1, 2]);
    }

    #[test]
    fn selecting_swaps_whole_creatures_and_keeps_their_own_lives() {
        let mut world = household();
        world.life.bond = 0.7;
        world.life.keepsakes.push(super::super::Keepsake::Feather);
        let first = world.ghost.name.clone();
        assert!(world.select(1));
        assert_eq!(world.host(), 1);
        assert_eq!(world.life.bond, 0.0);
        // The shelf belongs to the house, whoever is in the world.
        assert_eq!(world.life.keepsakes.len(), 1);
        assert!(world.select(0));
        assert_eq!(world.ghost.name, first);
        assert_eq!(world.life.bond, 0.7);
        assert!(!world.select(9));
        assert_eq!(world.roster(), [0, 1, 2]);
    }

    #[test]
    fn companions_live_on_their_own_and_never_take_up_the_viewers_games() {
        let mut world = household();
        world.viewer.active_at = 1.0;
        for _ in 0..3000 {
            world.step(0.1);
            for id in [1, 2] {
                world.with_creature(id, |world| {
                    assert!(world.companion);
                    if world.needs_decision() {
                        let options = world.options();
                        assert!(options.iter().all(|activity| !hosted(*activity)));
                        let activity = options[world.pick(&vec![1.0; options.len()]).unwrap()];
                        world.begin(activity);
                    }
                });
            }
            if world.needs_decision() {
                world.begin(Activity::Float);
            }
        }
        let (low, high) = super::super::home::yard_bounds();
        for ghost in world.creatures() {
            let at = ghost.position;
            assert!(at.is_finite() && ghost.body().is_finite());
            assert!(
                super::super::home::indoors(at)
                    || super::super::home::in_doorway(at)
                    || super::super::home::in_inner_doorway(at)
                    || (!super::super::home::in_house(at) && at.cmpge(low).all() && at.cmple(high).all()),
                "{at}"
            );
        }
    }

    #[test]
    fn a_companion_cannot_begin_what_another_is_already_doing_alone() {
        let mut world = household();
        world.begin(Activity::Mirror);
        world.look_at_housemates();
        world.with_creature(1, |world| {
            assert!(world.claimed(Activity::Mirror));
            assert!(!world.options().contains(&Activity::Mirror));
            world.begin(Activity::Mirror);
            assert_ne!(world.ghost.plan.activity, Activity::Mirror);
            world.begin(Activity::Game);
            assert_ne!(world.ghost.plan.activity, Activity::Game);
        });
    }

    #[test]
    fn overlapping_creatures_ease_apart() {
        let mut world = household();
        let spot = Vec3::new(0.0, 0.9, 0.0);
        world.ghost.position = spot;
        world.with_creature(1, |world| world.ghost.position = spot);
        for _ in 0..40 {
            world.keep_apart(0.1);
        }
        let other = world.creature(1).unwrap().position;
        assert!(world.ghost.position.distance(other) > 0.2);
    }

    #[test]
    fn the_house_holds_at_most_six() {
        let mut world = household();
        let mut adopted = 0;
        for index in 0..10 {
            if world.adopt(Species::Frog, &format!("Frog {index}"), index).is_some() {
                adopted += 1;
            }
        }
        assert_eq!(adopted, MOST - 3);
        assert_eq!(world.household_size(), MOST);
    }

    #[test]
    fn weather_and_visitors_reach_every_creature() {
        let mut world = household();
        world.weather.until = world.time + 0.05;
        world.weather.sky = super::super::Sky::Clear;
        for _ in 0..2000 {
            world.step(0.5);
            if world.weather.sky != super::super::Sky::Clear {
                break;
            }
        }
        let started = |events: &[Event]| {
            events.iter().any(|event| {
                matches!(
                    event.kind,
                    EventKind::RainStarted | EventKind::StormStarted | EventKind::SnowStarted
                )
            })
        };
        assert!(started(&world.events));
        assert!(world.with_creature(2, |world| started(&world.events)).unwrap());
    }
}
