use jiff::civil::Date;
use serde::{Deserialize, Deserializer, Serialize};

use super::{
    Activity, EventKind, Flower, Heard, Keepsake, Occasion, Season, Sky, Thing, Understanding,
    World,
};

/// One small thing worn at a time; scarf colors index a warm palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Accessory {
    Flower(Flower),
    Scarf { color: u8 },
    PartyHat,
    WitchHat,
    Feather,
    LuckyPouch,
    Holly,
}

impl Accessory {
    pub fn words(self) -> String {
        match self {
            Self::Flower(kind) => format!("a {} from my garden", kind.name()),
            Self::Scarf { .. } => "a knitted scarf".to_owned(),
            Self::PartyHat => "a paper party hat".to_owned(),
            Self::WitchHat => "a little witch hat".to_owned(),
            Self::Feather => "a feather from my shelf".to_owned(),
            Self::LuckyPouch => "a tiny lucky pouch".to_owned(),
            Self::Holly => "a sprig of holly".to_owned(),
        }
    }

    pub fn korean(self) -> String {
        match self {
            Self::Flower(kind) => format!("내 정원에서 딴 {}", kind.korean()),
            Self::Scarf { .. } => "뜨개 목도리".to_owned(),
            Self::PartyHat => "종이 파티 모자".to_owned(),
            Self::WitchHat => "작은 마녀 모자".to_owned(),
            Self::Feather => "선반에서 가져온 깃털".to_owned(),
            Self::LuckyPouch => "작은 복주머니".to_owned(),
            Self::Holly => "호랑가시나무 가지".to_owned(),
        }
    }

    pub fn thoughts(self) -> [&'static str; 2] {
        match self {
            Self::Flower(_) => [
                "I grew this flower myself.",
                "A little garden beside my face.",
            ],
            Self::Scarf { .. } => ["This scarf is so cozy.", "A warm stitch for a cold day."],
            Self::PartyHat => ["A little paper celebration!", "My hat feels like a party."],
            Self::WitchHat => ["A tiny witch, that's me.", "My hat is ready for Halloween."],
            Self::Feather => [
                "The bird's gift goes everywhere with me.",
                "A feather tickles my cheek.",
            ],
            Self::LuckyPouch => [
                "A little pouch for good luck.",
                "May this new year be kind.",
            ],
            Self::Holly => [
                "A little green for the holidays.",
                "My holly has such bright berries.",
            ],
        }
    }

    pub fn hat(self) -> bool {
        matches!(self, Self::PartyHat | Self::WitchHat)
    }

    pub fn keepsake(self) -> Option<Keepsake> {
        (self == Self::Feather).then_some(Keepsake::Feather)
    }

    pub(super) fn valid(self, occasion: Option<Occasion>) -> bool {
        match self {
            Self::Scarf { color } => usize::from(color) < 3 + super::acquaintance::Color::ALL.len(),
            Self::PartyHat => matches!(
                occasion,
                Some(Occasion::Birthday | Occasion::NewYear | Occasion::ViewerBirthday)
            ),
            Self::WitchHat => occasion == Some(Occasion::Halloween),
            Self::LuckyPouch => occasion == Some(Occasion::Seollal),
            Self::Holly => occasion == Some(Occasion::WinterHolidays),
            _ => true,
        }
    }
}

// A damaged accessory should not make an otherwise healthy life unreadable.
pub(super) fn deserialize_accessory<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Accessory>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(serde_json::from_value::<Accessory>(value).ok())
}

#[derive(Clone, Debug, Default)]
pub(super) struct Wardrobe {
    checked: Option<(Date, Option<Occasion>, bool, bool)>,
    pending: Option<Accessory>,
    fetching: bool,
    reconsider: bool,
}

impl World {
    pub fn cold_for_scarf(&self) -> bool {
        self.weather.sky == Sky::Snow
            || (self.season() == Season::Winter && !(7.0..18.0).contains(&self.hour))
    }

    fn occasion_accessory(&self) -> Option<Accessory> {
        match self.occasion() {
            Some(Occasion::Birthday | Occasion::NewYear | Occasion::ViewerBirthday) => {
                Some(Accessory::PartyHat)
            }
            Some(Occasion::Halloween) => Some(Accessory::WitchHat),
            Some(Occasion::Seollal) => Some(Accessory::LuckyPouch),
            Some(Occasion::WinterHolidays) => Some(Accessory::Holly),
            _ => None,
        }
    }

    fn scarf_color(&mut self) -> u8 {
        self.life.viewer.color.map_or_else(
            || self.pick(&[1.0; 3]).unwrap() as u8,
            |color| {
                3 + super::acquaintance::Color::ALL
                    .iter()
                    .position(|&known| known == color)
                    .unwrap() as u8
            },
        )
    }

    fn choose_accessory(&mut self) -> Option<Accessory> {
        let nature = self.life.nature.unwrap_or_default();
        if self.bedtime() {
            return self.cold_for_scarf().then(|| Accessory::Scarf {
                color: self.scarf_color(),
            });
        }
        if let Some(accessory) = self.occasion_accessory() {
            return (self.random_unit() < 0.85 + nature.playful * 0.12 - nature.shy * 0.25)
                .then_some(accessory);
        }
        if self.cold_for_scarf() {
            return (self.random_unit() < 0.9 - nature.shy * 0.2).then(|| Accessory::Scarf {
                color: self.scarf_color(),
            });
        }
        let flowers: Vec<Flower> = self
            .seedlings
            .iter()
            .filter(|plant| plant.flowering(self.season()))
            .map(|plant| plant.kind)
            .collect();
        if flowers.is_empty() && !self.life.keepsakes.contains(&Keepsake::Feather) {
            return None;
        }
        let weights = [
            if flowers.is_empty() {
                0.0
            } else {
                0.35 + nature.tender * 0.55
            },
            if self.life.keepsakes.contains(&Keepsake::Feather) {
                0.18
            } else {
                0.0
            },
            0.45 + nature.shy * 1.2,
        ];
        match self.pick(&weights) {
            Some(0) => {
                let index = self.pick(&vec![1.0; flowers.len()])?;
                Some(Accessory::Flower(flowers[index]))
            }
            Some(1) => Some(Accessory::Feather),
            _ => None,
        }
    }

    fn accessory_available(&self, accessory: Accessory) -> bool {
        match accessory {
            Accessory::Flower(kind) => self
                .seedlings
                .iter()
                .any(|plant| plant.kind == kind && plant.flowering(self.season())),
            Accessory::Feather => self.life.keepsakes.contains(&Keepsake::Feather),
            Accessory::Scarf { .. } => self.cold_for_scarf(),
            _ => self.occasion_accessory() == Some(accessory),
        }
    }

    fn put_on(&mut self, accessory: Accessory) {
        if self.wearing == Some(accessory) {
            self.wardrobe.pending = None;
            return;
        }
        self.take_off();
        self.wearing = Some(accessory);
        self.life.wearing = self.wearing;
        self.record(EventKind::PutOn(accessory));
    }

    pub(super) fn undress_for_bed(&mut self) {
        if self.bedtime()
            && !matches!(self.wearing, Some(Accessory::Scarf { .. }) if self.cold_for_scarf())
        {
            self.take_off();
        }
    }

    pub fn take_off(&mut self) {
        self.wardrobe.pending = None;
        self.wardrobe.fetching = false;
        if let Some(accessory) = self.wearing.take() {
            self.life.wearing = None;
            self.record(EventKind::TookOff(accessory));
        }
    }

    pub(super) fn reconsider_wardrobe(&mut self) {
        self.wardrobe.reconsider = true;
    }

    /// Restore only today's choice, without rolling or counting another outfit.
    pub fn restore_wardrobe(&mut self) {
        self.wardrobe = Wardrobe::default();
        if self.life.wardrobe_day.as_deref() == Some(self.date.to_string().as_str()) {
            self.wearing = self.life.wearing;
            if self.life.wardrobe_occasion == self.occasion() {
                self.wardrobe.checked = Some((
                    self.date,
                    self.occasion(),
                    self.cold_for_scarf(),
                    self.bedtime(),
                ));
            }
        } else {
            self.wearing = None;
            self.life.wearing = None;
        }
        let missing_flower = matches!(self.wearing, Some(Accessory::Flower(kind))
            if !self.seedlings.iter().any(|plant| plant.kind == kind && plant.growth >= 1.0));
        if missing_flower
            || (self.bedtime()
                && !matches!(self.wearing, Some(Accessory::Scarf { .. }) if self.cold_for_scarf()))
        {
            self.wearing = None;
            self.life.wearing = None;
        }
    }

    pub(super) fn check_wardrobe(&mut self) {
        let context = (
            self.date,
            self.occasion(),
            self.cold_for_scarf(),
            self.bedtime(),
        );
        if self.wardrobe.checked == Some(context) && !self.wardrobe.reconsider {
            return;
        }
        let new_day = self.life.wardrobe_day.as_deref() != Some(self.date.to_string().as_str());
        let new_occasion = self.life.wardrobe_occasion != self.occasion();
        let previous = self.wardrobe.checked;
        self.wardrobe.checked = Some(context);
        self.wardrobe.reconsider = false;
        self.life.wardrobe_day = Some(self.date.to_string());
        self.life.wardrobe_occasion = self.occasion();
        let keep_scarf =
            matches!(self.wearing, Some(Accessory::Scarf { .. }) if self.cold_for_scarf());
        if !keep_scarf
            && (new_day
                || new_occasion
                || self.bedtime()
                || matches!(self.wearing, Some(Accessory::Scarf { .. })))
        {
            self.take_off();
        }
        // Keep a flower until bedtime; fresh snow and occasions can replace it.
        if self.wearing.is_some() && !new_occasion && previous.is_some_and(|old| old.2 == context.2)
        {
            return;
        }
        self.wardrobe.pending = self.choose_accessory();
        self.wardrobe.fetching = false;
    }

    pub(super) fn advance_wardrobe(&mut self) {
        if self.wardrobe.reconsider {
            self.check_wardrobe();
        }
        self.dress_when_idle();
    }

    pub(super) fn dress_when_idle(&mut self) {
        let Some(accessory) = self.wardrobe.pending else {
            return;
        };
        if self.ghost.asleep >= 0.3 || self.focus.is_some() || self.telling.is_some() {
            return;
        }
        if !self.accessory_available(accessory)
            || (self.bedtime() && !matches!(accessory, Accessory::Scarf { .. }))
        {
            self.wardrobe.pending = None;
            return;
        }
        let source = match accessory {
            Accessory::Feather => Some((
                super::SHELF_POSITION + glam::Vec3::new(0.0, 0.2, 0.35),
                Thing::Shelf,
            )),
            Accessory::Flower(kind) => self
                .seedlings
                .iter()
                .position(|plant| plant.kind == kind && plant.flowering(self.season()))
                .map(|index| {
                    (
                        super::garden::SPOTS[index] + glam::Vec3::new(0.0, 0.7, 0.35),
                        Thing::Seedlings,
                    )
                }),
            _ => None,
        };
        if self.wardrobe.fetching {
            if self.ghost.plan.activity != Activity::Explore {
                self.wardrobe.fetching = false;
                return;
            }
            if source.is_some_and(|(position, _)| {
                self.ghost.position.distance(position) < super::ARRIVED
            }) {
                self.ghost.plan.done = true;
                self.put_on(accessory);
            }
            return;
        }
        if !self.ghost.plan.done
            && !matches!(
                self.ghost.plan.activity,
                Activity::Float | Activity::Explore
            )
        {
            return;
        }
        if let Some((position, thing)) = source {
            self.begin(Activity::Explore);
            self.ghost.plan.target = position;
            self.attend(thing.name(), 20.0);
            self.wardrobe.fetching = true;
        } else {
            self.put_on(accessory);
        }
    }

    pub(super) fn hear_wardrobe(&mut self, text: &str, korean: bool) -> Option<Heard> {
        if !requested_off(text, self.wearing) {
            return None;
        }
        self.listening = super::Listening::default();
        self.record(EventKind::Heard(text.to_owned()));
        self.take_off();
        Some(Heard {
            tone: None,
            understanding: Understanding::Undressing,
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }
}

pub(super) fn requested_off(text: &str, wearing: Option<Accessory>) -> bool {
    if text.trim_end().ends_with(['?', '？']) {
        return false;
    }
    let words: Vec<String> = text
        .to_lowercase()
        .split(|character: char| !character.is_alphabetic())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect();
    if words
        .iter()
        .any(|word| matches!(word.as_str(), "dont" | "not"))
        || words.windows(2).any(|run| run == ["don", "t"])
    {
        return false;
    }
    let english = words.windows(2).any(|run| run == ["take", "off"])
        && words.iter().any(|word| match word.as_str() {
            "hat" => wearing.is_none_or(Accessory::hat),
            "scarf" => matches!(wearing, Some(Accessory::Scarf { .. })),
            "flower" => matches!(wearing, Some(Accessory::Flower(_))),
            "feather" => wearing == Some(Accessory::Feather),
            "pouch" => wearing == Some(Accessory::LuckyPouch),
            "holly" => wearing == Some(Accessory::Holly),
            "accessory" => true,
            _ => false,
        });
    let joined: String = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let korean = joined.contains("벗어")
        && !joined.contains("말")
        && !joined.contains("지마")
        && ((joined.contains("모자") && wearing.is_none_or(Accessory::hat))
            || joined.contains("목도리") && matches!(wearing, Some(Accessory::Scarf { .. }))
            || joined.contains("장식"));
    english || korean
}

#[cfg(test)]
mod tests {
    use super::super::{Life, Nature, Seedling};
    use super::*;

    fn garden(world: &mut World) {
        let mut plant = Seedling::new(Flower::Bluebell);
        plant.growth = 1.0;
        plant.age = 6.0 * 86_400.0;
        world.seedlings.push(plant);
    }

    fn prepared(seed: u64) -> World {
        let mut world = World::new(seed, 12.0);
        world.step(0.0);
        world
    }

    #[test]
    fn ordinary_days_need_a_flowering_pot_or_a_kept_feather() {
        let mut saw_flower = false;
        let mut saw_feather = false;
        for seed in 0..80 {
            let mut world = prepared(seed);
            assert_eq!(world.choose_accessory(), None);
            world.seedlings.push(Seedling::new(Flower::Poppy));
            assert_eq!(world.choose_accessory(), None);
            garden(&mut world);
            for _ in 0..5 {
                if let Some(accessory) = world.choose_accessory() {
                    assert_eq!(accessory, Accessory::Flower(Flower::Bluebell));
                    saw_flower = true;
                }
            }
            world.seedlings.clear();
            world.life.keep(Keepsake::Feather);
            for _ in 0..5 {
                if let Some(accessory) = world.choose_accessory() {
                    assert_eq!(accessory, Accessory::Feather);
                    saw_feather = true;
                }
            }
        }
        assert!(saw_flower && saw_feather);
    }

    #[test]
    fn occasions_choose_their_own_accessory_ahead_of_garden_shelf_and_cold() {
        for (date, expected) in [
            (Date::constant(2026, 10, 25), Accessory::WitchHat),
            (Date::constant(2026, 12, 24), Accessory::Holly),
            (Date::constant(2027, 1, 1), Accessory::PartyHat),
            (Date::constant(2027, 2, 7), Accessory::LuckyPouch),
        ] {
            let mut chosen = 0;
            for seed in 0..50 {
                let mut world = World::new(seed, 20.0);
                world.date = date;
                garden(&mut world);
                world.life.keep(Keepsake::Feather);
                world.weather.sky = Sky::Snow;
                if let Some(accessory) = world.choose_accessory() {
                    assert_eq!(accessory, expected);
                    chosen += 1;
                }
                world.birthday = Some(date.with().year(2025).build().unwrap());
                if let Some(accessory) = world.choose_accessory() {
                    assert_eq!(accessory, Accessory::PartyHat);
                }
            }
            assert!(chosen > 0);
        }
    }

    #[test]
    fn scarves_need_snow_or_a_cold_winter_evening_and_keep_a_seeded_color() {
        for (date, hour, sky, cold) in [
            (Date::constant(2026, 6, 21), 20.0, Sky::Clear, false),
            (Date::constant(2026, 12, 10), 12.0, Sky::Clear, false),
            (Date::constant(2026, 12, 10), 18.0, Sky::Clear, true),
            (Date::constant(2026, 12, 10), 6.0, Sky::Clear, true),
            (Date::constant(2026, 12, 10), 12.0, Sky::Snow, true),
        ] {
            let mut scarves = 0;
            for seed in 0..50 {
                let mut world = World::new(seed, hour);
                world.date = date;
                world.weather.sky = sky;
                if let Some(accessory) = world.choose_accessory() {
                    assert!(cold);
                    assert!(matches!(accessory, Accessory::Scarf { color: 0..=2 }));
                    scarves += 1;
                }
            }
            assert_eq!(scarves > 0, cold);
        }
    }

    #[test]
    fn tender_ghosts_choose_more_flowers_playful_ghosts_more_hats_and_shy_ghosts_less() {
        let count = |nature: Nature, holiday: bool| {
            (0..200)
                .filter(|seed| {
                    let mut world = World::new(*seed, 12.0);
                    world.life.nature = Some(nature);
                    garden(&mut world);
                    if holiday {
                        world.date = Date::constant(2026, 10, 25);
                    }
                    world.choose_accessory().is_some()
                })
                .count()
        };
        let ordinary = Nature::default();
        assert!(
            count(
                Nature {
                    tender: 1.0,
                    ..ordinary
                },
                false
            ) > count(
                Nature {
                    tender: 0.0,
                    ..ordinary
                },
                false
            )
        );
        assert!(
            count(
                Nature {
                    playful: 1.0,
                    ..ordinary
                },
                true
            ) > count(
                Nature {
                    playful: 0.0,
                    ..ordinary
                },
                true
            )
        );
        assert!(
            count(
                Nature {
                    shy: 1.0,
                    ..ordinary
                },
                false
            ) < count(
                Nature {
                    shy: 0.0,
                    ..ordinary
                },
                false
            )
        );
    }

    #[test]
    fn choices_advance_the_world_random_source_and_repeat_for_the_same_seed() {
        let mut first = World::new(7, 12.0);
        first.date = Date::constant(2026, 12, 10);
        first.weather.sky = Sky::Snow;
        let mut second = first.clone();
        let mut untouched = first.random.clone();
        assert_eq!(first.choose_accessory(), second.choose_accessory());
        assert_eq!(first.random.next_u64(), second.random.next_u64());
        assert_ne!(first.random.next_u64(), untouched.next_u64());
    }

    #[test]
    fn shelf_and_garden_accessories_wait_for_idle_then_visit_the_source() {
        for accessory in [Accessory::Feather, Accessory::Flower(Flower::Bluebell)] {
            let mut world = prepared(9);
            world.life.keep(Keepsake::Feather);
            garden(&mut world);
            world.begin(Activity::Dance);
            world.wardrobe.pending = Some(accessory);
            world.dress_when_idle();
            assert_eq!(world.ghost.plan.activity, Activity::Dance);
            assert!(world.wearing.is_none());
            world.begin(Activity::Float);
            world.dress_when_idle();
            assert_eq!(world.ghost.plan.activity, Activity::Explore);
            assert!(world.wearing.is_none());
            for _ in 0..250 {
                world.step(0.1);
                if world.wearing.is_some() {
                    break;
                }
            }
            assert_eq!(world.wearing, Some(accessory));
            assert_eq!(world.life.total.outfits, 1);
            assert!(
                world
                    .life
                    .today
                    .words()
                    .to_lowercase()
                    .contains("wore a little accessory")
            );
            assert_eq!(
                world
                    .life
                    .milestones
                    .iter()
                    .filter(|entry| entry.text == "I wore my first little accessory.")
                    .count(),
                1
            );
        }
    }

    #[test]
    fn focus_stories_and_sleep_defer_dressing_without_waking_or_interrupting() {
        let mut world = prepared(3);
        world.wardrobe.pending = Some(Accessory::Feather);
        world.life.keep(Keepsake::Feather);
        world.start_focus(5);
        world.dress_when_idle();
        assert_eq!(world.ghost.plan.activity, Activity::Company);
        assert!(world.wearing.is_none());
        world.focus = None;
        world.ghost.asleep = 1.0;
        world.dress_when_idle();
        assert_eq!(world.ghost.asleep, 1.0);
        assert!(world.wearing.is_none());
        world.ghost.asleep = 0.0;
        world.tell(vec!["A little tale.".into()]);
        world.dress_when_idle();
        assert!(world.telling.is_some());
        assert!(world.wearing.is_none());
    }

    #[test]
    fn flowers_last_the_day_and_switching_never_layers_accessories() {
        let mut world = prepared(1);
        garden(&mut world);
        world.put_on(Accessory::Flower(Flower::Bluebell));
        world.seedlings[0].droop = 1.0;
        world.step(0.1);
        assert_eq!(world.wearing, Some(Accessory::Flower(Flower::Bluebell)));
        world.put_on(Accessory::PartyHat);
        assert_eq!(world.wearing, Some(Accessory::PartyHat));
        assert!(world.events.windows(2).any(|events| matches!(
            events[0].kind,
            EventKind::TookOff(Accessory::Flower(_))
        ) && events[1].kind
            == EventKind::PutOn(Accessory::PartyHat)));
        world.poke();
        world.begin(Activity::Dance);
        world.step(10.0);
        assert_eq!(world.wearing, Some(Accessory::PartyHat));
    }

    #[test]
    fn bedtime_removes_accessories_but_keeps_a_scarf_while_cold() {
        for accessory in [
            Accessory::Flower(Flower::Daisy),
            Accessory::PartyHat,
            Accessory::WitchHat,
            Accessory::Feather,
            Accessory::LuckyPouch,
            Accessory::Holly,
            Accessory::Scarf { color: 0 },
        ] {
            let mut world = prepared(1);
            world.life.keep(Keepsake::Feather);
            world.put_on(accessory);
            world.hour = 22.0;
            world.tuck_in();
            assert!(world.wearing.is_none());
            assert!(
                world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::TookOff(accessory))
            );
        }
        let mut world = prepared(1);
        world.date = Date::constant(2026, 12, 10);
        world.hour = 22.0;
        world.put_on(Accessory::Scarf { color: 1 });
        world.tuck_in();
        world.step(1.0);
        assert_eq!(world.wearing, Some(Accessory::Scarf { color: 1 }));
    }

    #[test]
    fn a_borrowed_feather_leaves_the_shelf_and_returns_without_creating_gifts() {
        let mut world = prepared(1);
        world.life.keepsakes = vec![Keepsake::Feather, Keepsake::Acorn, Keepsake::Feather];
        world.put_on(Accessory::Feather);
        assert_eq!(
            world
                .life
                .shelved_keepsakes()
                .filter(|gift| **gift == Keepsake::Feather)
                .count(),
            1
        );
        assert_eq!(world.saved_life().keepsakes.len(), 3);
        world.take_off();
        assert_eq!(
            world
                .life
                .shelved_keepsakes()
                .filter(|gift| **gift == Keepsake::Feather)
                .count(),
            2
        );
        assert_eq!(world.life.total.gifts, 0);
        world.life.keepsakes = vec![Keepsake::Feather];
        world.put_on(Accessory::Feather);
        for _ in 0..12 {
            world.life.keep(Keepsake::Acorn);
        }
        assert!(world.life.keepsakes.contains(&Keepsake::Feather));
        assert_eq!(world.life.keepsakes.len(), 8);
    }

    #[test]
    fn morning_calendar_flowering_and_snow_reconsider_without_per_slice_rerolls() {
        let mut world = prepared(1);
        let mut untouched = world.random.clone();
        world.check_wardrobe();
        assert_eq!(world.random.next_u64(), untouched.next_u64());
        for event in [
            EventKind::Morning,
            EventKind::Flowered(Flower::Bluebell),
            EventKind::SnowStarted,
        ] {
            world.record(event);
            assert!(world.wardrobe.reconsider);
            world.advance_wardrobe();
            assert!(!world.wardrobe.reconsider);
        }
        world.date = Date::constant(2026, 12, 31);
        world.hour = 19.0;
        world.step(0.0);
        assert_eq!(world.life.wardrobe_occasion, Some(Occasion::WinterHolidays));
        world.hour = 20.0;
        world.step(0.0);
        assert_eq!(world.life.wardrobe_occasion, Some(Occasion::NewYear));
        assert!(
            world
                .wearing
                .is_none_or(|accessory| accessory == Accessory::PartyHat)
        );
    }

    #[test]
    fn today_survives_a_restart_without_a_reroll_and_yesterday_is_removed() {
        let mut world = prepared(4);
        world.life.keep(Keepsake::Feather);
        world.put_on(Accessory::Feather);
        let saved = serde_json::to_string(&world.saved_life()).unwrap();
        let mut restored = World::new(12, 12.0);
        restored.life = serde_json::from_str::<Life>(&saved).unwrap().sanitized();
        restored.restore_wardrobe();
        assert_eq!(restored.wearing, Some(Accessory::Feather));
        let mut untouched = restored.random.clone();
        restored.check_wardrobe();
        assert_eq!(restored.random.next_u64(), untouched.next_u64());
        assert_eq!(restored.life.total.outfits, 1);
        restored.date = restored.date.tomorrow().unwrap();
        restored.life.begin_day(&restored.date.to_string());
        restored.restore_wardrobe();
        assert!(restored.wearing.is_none());
        assert_eq!(restored.life.shelved_keepsakes().count(), 1);
    }

    #[test]
    fn old_and_damaged_saves_drop_bad_accessories_and_clamp_counts() {
        let old: Life = serde_json::from_str("{}").unwrap();
        assert!(old.wearing.is_none());
        for wearing in [
            serde_json::json!("Unknown"),
            serde_json::json!({"Flower":"unknown"}),
            serde_json::json!({"Scarf":{"color":255}}),
            serde_json::json!({"Scarf":{"color":-1}}),
            serde_json::json!("Feather"),
        ] {
            let life: Life = serde_json::from_value(serde_json::json!({
                "day":"2026-06-21", "wardrobe_day":"2026-06-21", "wearing":wearing,
                "total":{"outfits":4294967295u32}
            }))
            .unwrap();
            let life = life.sanitized();
            assert!(life.wearing.is_none());
            assert_eq!(life.total.outfits, 100_000_000);
        }
        let life = Life {
            day: "2026-06-21".into(),
            wardrobe_day: Some("2026-06-21".into()),
            wearing: Some(Accessory::WitchHat),
            ..Life::default()
        }
        .sanitized();
        assert!(life.wearing.is_none());
        let life = Life {
            day: "2026-06-21".into(),
            wardrobe_day: Some("broken".into()),
            wearing: Some(Accessory::Holly),
            ..Life::default()
        }
        .sanitized();
        assert!(life.wearing.is_none());
    }

    #[test]
    fn wearing_questions_and_take_off_requests_work_in_both_languages_without_teaching_words() {
        for text in ["what are you wearing?", "뭐 입었어?", "그거 뭐야？"] {
            let mut world = prepared(1);
            world.put_on(Accessory::WitchHat);
            let heard = world.hear(text);
            assert!(matches!(
                heard.understanding,
                Understanding::Asked {
                    question: super::super::Question::Wearing,
                    ..
                }
            ));
            assert!(heard.learned.is_empty());
        }
        for text in ["take off your hat", "모자 벗어"] {
            let mut world = prepared(1);
            world.put_on(Accessory::WitchHat);
            let heard = world.hear(text);
            assert_eq!(heard.understanding, Understanding::Undressing);
            assert!(world.wearing.is_none());
            assert!(heard.learned.is_empty());
            assert!(world.life.vocabulary.known().is_empty());
            world.step(1.0);
            assert!(world.wearing.is_none());
        }
        for text in [
            "그거 뭐야",
            "뭐 입었어",
            "I am wearing a hat",
            "takeoff your hat",
        ] {
            assert_ne!(
                super::super::Question::ask(text),
                Some(super::super::Question::Wearing)
            );
        }
        for text in [
            "don't take off your hat",
            "모자 벗어야 해?",
            "모자 벗어 주지 마",
            "take off your hat?",
        ] {
            assert!(!requested_off(text, Some(Accessory::WitchHat)));
        }
    }
}
