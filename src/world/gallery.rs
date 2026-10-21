use std::sync::OnceLock;

use glam::{Vec2, Vec3};
use jiff::civil::Date;
use serde::{Deserialize, Deserializer, Serialize};

use super::doodle::Path;
use super::{Activity, EventKind, Life, ROOM_MIN, Sketch, World, recognizer};

pub const WIDTH: f32 = 0.85;
pub const HEIGHT: f32 = 0.6;
pub const DEPTH: f32 = 0.045;
pub const FRAME_WIDTH: f32 = 0.035;
pub const CENTER: Vec3 = Vec3::new(ROOM_MIN.x + DEPTH * 0.5, 1.5, -0.15);
pub const APPROACH: Vec3 = Vec3::new(ROOM_MIN.x + 0.48, CENTER.y, CENTER.z);
pub const VIEW: Vec3 = Vec3::new(ROOM_MIN.x + 0.85, CENTER.y, CENTER.z);
const KEPT: usize = 4;
const WAITING: usize = 2;
const MAX_STROKES: usize = 12;
const MAX_POINTS: usize = 120;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    #[default]
    ViewerDrawing,
    OwnDoodle,
    Constellation,
}

impl Kind {
    pub fn words(self, name: &str) -> String {
        match self {
            Self::ViewerDrawing if name == "picture" => "a copy of the viewer's drawing".to_owned(),
            Self::ViewerDrawing => format!("a copy of the viewer's {name} drawing"),
            Self::OwnDoodle => format!("your {name} doodle"),
            Self::Constellation => format!("your constellation, {name}"),
        }
    }

    pub fn pinned(self, name: &str) -> String {
        match self {
            Self::ViewerDrawing if name == "picture" => {
                "pinned the viewer's drawing to my gallery".to_owned()
            }
            Self::ViewerDrawing => format!("pinned the viewer's {name} drawing to my gallery"),
            Self::OwnDoodle => format!("pinned my {name} doodle to my gallery"),
            Self::Constellation => format!("pinned my constellation, {name}, to my gallery"),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Pin {
    #[serde(default)]
    pub kind: Kind,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub korean: String,
    #[serde(default)]
    pub day: String,
    #[serde(default)]
    pub strokes: Vec<Vec<Vec2>>,
    #[serde(skip)]
    path: OnceLock<Path>,
    #[serde(skip)]
    source: Option<f64>,
}

impl PartialEq for Pin {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.name == other.name
            && self.korean == other.korean
            && self.day == other.day
            && self.strokes == other.strokes
    }
}

impl Pin {
    fn new(kind: Kind, name: &str, korean: &str, day: Date, strokes: &[Vec<Vec2>]) -> Self {
        let strokes: Vec<_> = strokes
            .iter()
            .filter(|stroke| {
                stroke
                    .windows(2)
                    .any(|pair| pair[0].distance_squared(pair[1]) > 1e-10)
            })
            .take(MAX_STROKES)
            .cloned()
            .collect();
        let mut strokes = recognizer::normalized(&strokes);
        for point in strokes.iter_mut().flatten() {
            *point = (*point * 1000.0).round() / 1000.0;
        }
        Self {
            kind,
            name: name.to_owned(),
            korean: korean.to_owned(),
            day: day.to_string(),
            strokes,
            ..Self::default()
        }
    }

    fn valid(&self, today: &str) -> bool {
        let named = |name: &str| {
            !name.trim().is_empty()
                && name.chars().count() <= 128
                && !name
                    .chars()
                    .any(|letter| letter.is_control() || letter.is_numeric())
        };
        named(&self.name)
            && named(&self.korean)
            && self
                .day
                .parse::<Date>()
                .is_ok_and(|day| today.parse::<Date>().is_ok_and(|today| day <= today))
            && !self.strokes.is_empty()
            && self.strokes.len() <= MAX_STROKES
            && self.strokes.iter().all(|stroke| !stroke.is_empty())
            && (2..=MAX_POINTS).contains(&self.strokes.iter().map(Vec::len).sum::<usize>())
            && self
                .strokes
                .iter()
                .flatten()
                .all(|point| point.is_finite() && point.abs().max_element() <= 1.0)
            && self.strokes.iter().any(|stroke| {
                stroke
                    .windows(2)
                    .any(|pair| pair[0].distance_squared(pair[1]) > 1e-10)
            })
    }

    /// The saved path is prepared once, then shared by all rays and frames.
    pub fn distance(&self, point: Vec2) -> f32 {
        if point.abs().max_element() > 1.05 {
            return f32::INFINITY;
        }
        self.path
            .get_or_init(|| Path::new(self.strokes.clone()))
            .distance(point, 1.0)
    }

    pub fn words(&self) -> String {
        self.kind.words(&self.name)
    }

    pub fn recall(&self) -> String {
        match self.kind {
            Kind::ViewerDrawing if self.name == "picture" => {
                "I still like the picture you drew me.".to_owned()
            }
            Kind::ViewerDrawing => format!("I still like the {} you drew me.", self.name),
            Kind::OwnDoodle => format!("I'm still proud of my {} doodle.", self.name),
            Kind::Constellation => format!("I found {} hiding in the stars.", self.name),
        }
    }

    fn korean_words(&self) -> String {
        match self.kind {
            Kind::ViewerDrawing if self.name == "picture" => "네가 그려 준 그림".to_owned(),
            Kind::ViewerDrawing => format!("네가 그려 준 {} 그림", self.korean),
            Kind::OwnDoodle => format!("내가 그린 {} 그림", self.korean),
            Kind::Constellation => format!("내가 찾은 {} 별자리", self.korean),
        }
    }
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Pin>, D::Error> {
    let values = Vec::<serde_json::Value>::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .filter_map(|value| serde_json::from_value(value).ok())
        .collect())
}

pub(super) fn sanitize(life: &mut Life) {
    life.gallery.retain(|pin| pin.valid(&life.day));
    while life.gallery.len() > KEPT {
        remove_oldest(&mut life.gallery);
    }
    for pin in &mut life.gallery {
        for point in pin.strokes.iter_mut().flatten() {
            *point = (*point * 1000.0).round() / 1000.0;
        }
        pin.path = OnceLock::new();
    }
    life.gallery.retain(|pin| pin.valid(&life.day));
    for tally in [&mut life.today, &mut life.total] {
        tally.viewer_pins = tally.viewer_pins.min(100_000_000);
        tally.doodle_pins = tally.doodle_pins.min(100_000_000);
        tally.constellation_pins = tally.constellation_pins.min(100_000_000);
        tally.pinned_doodles.retain(|name| {
            !name.trim().is_empty()
                && name.chars().count() <= 128
                && !name
                    .chars()
                    .any(|letter| letter.is_control() || letter.is_numeric())
        });
        tally.pinned_doodles.truncate(8);
    }
}

fn remove_oldest(pins: &mut Vec<Pin>) {
    let index = pins
        .iter()
        .position(|pin| pin.kind != Kind::ViewerDrawing)
        .unwrap_or(0);
    pins.remove(index);
}

impl Life {
    fn pin(&mut self, pin: Pin) {
        if self.gallery.len() >= KEPT {
            remove_oldest(&mut self.gallery);
        }
        self.gallery.push(pin);
    }

    pub fn gallery_words(&self) -> String {
        if self.gallery.is_empty() {
            "an empty gallery board, waiting for pictures".to_owned()
        } else {
            format!(
                "your gallery board with {}",
                super::life::listed(&self.gallery.iter().map(Pin::words).collect::<Vec<_>>())
            )
        }
    }

    pub fn gallery_answer(&self, korean: bool) -> String {
        let Some(newest) = self.gallery.last() else {
            return if korean {
                "아직 비어 있어. 소중한 그림을 붙여 둘 거야."
            } else {
                "It's still empty. I'll keep pictures that matter to me there."
            }
            .to_owned();
        };
        if korean {
            format!(
                "벽에 붙여 둔 그림들이야: {}. 가장 최근에 붙인 그림은 {}.",
                self.gallery
                    .iter()
                    .map(Pin::korean_words)
                    .collect::<Vec<_>>()
                    .join(", "),
                newest.korean_words()
            )
        } else {
            format!(
                "I've kept {}. The newest is {}.",
                super::life::listed(
                    &self
                        .gallery
                        .iter()
                        .map(|pin| pin.words().replacen("your ", "my ", 1))
                        .collect::<Vec<_>>()
                ),
                newest.words().replacen("your ", "my ", 1)
            )
        }
    }
}

impl World {
    pub fn pin_is_open(&self) -> bool {
        !self.pending_pins.is_empty()
            && self.ghost.asleep < 0.3
            && self.focus.is_none()
            && self.hunt.is_none()
            && self.telling.is_none()
            && !self
                .game
                .as_ref()
                .is_some_and(|game| game.outcome.is_none())
            && !self.puzzle_active()
    }

    fn queue_pin(&mut self, pin: Pin) -> bool {
        if !pin.valid(&self.date.to_string()) {
            return false;
        }
        if self.pending_pins.len() >= WAITING {
            if pin.kind != Kind::OwnDoodle
                && let Some(index) = self
                    .pending_pins
                    .iter()
                    .rposition(|pin| pin.kind == Kind::OwnDoodle)
            {
                self.pending_pins.remove(index);
            } else if pin.kind == Kind::Constellation {
                // Leave a card already being carried at the front alone.
                self.pending_pins.pop();
            } else {
                return false;
            }
        }
        self.pending_pins.push(pin);
        true
    }

    pub(super) fn keep_viewer_drawing(&mut self, name: Option<(&str, &str)>) -> bool {
        let waiting = self
            .pending_pins
            .iter()
            .filter(|pin| pin.kind == Kind::ViewerDrawing)
            .count() as u32;
        if self.life.today.viewer_pins + waiting >= 2
            || self.pending_pins.len() >= WAITING
                && !self
                    .pending_pins
                    .iter()
                    .any(|pin| pin.kind == Kind::OwnDoodle)
        {
            return false;
        }
        let first = self.life.total.viewer_pins == 0
            && !self
                .pending_pins
                .iter()
                .any(|pin| pin.kind == Kind::ViewerDrawing);
        let chance = if self.life.bond >= 0.55 {
            2.0 / 3.0
        } else {
            1.0 / 3.0
        };
        if !first && self.random.unit() >= chance {
            return false;
        }
        let Some(drawing) = &self.drawing else {
            return false;
        };
        let (name, korean) = name.unwrap_or(("picture", "그림"));
        let mut pin = Pin::new(
            Kind::ViewerDrawing,
            name,
            korean,
            self.date,
            &drawing.strokes,
        );
        pin.source = Some(drawing.started);
        self.queue_pin(pin)
    }

    pub(super) fn name_gallery_drawing(&mut self, name: &str) {
        let Some(drawing) = &self.drawing else {
            return;
        };
        for pin in self
            .pending_pins
            .iter_mut()
            .chain(self.life.gallery.iter_mut())
        {
            if pin.kind == Kind::ViewerDrawing && pin.source == Some(drawing.started) {
                pin.name = name.to_owned();
                pin.korean = name.to_owned();
            }
        }
    }

    pub(super) fn keep_doodle(&mut self, sketch: Sketch) {
        if self.life.today.doodle_pins > 0
            || self.pending_pins.len() >= WAITING
            || self
                .pending_pins
                .iter()
                .any(|pin| pin.kind == Kind::OwnDoodle)
            || self.random.unit() >= 0.2
        {
            return;
        }
        let korean = match sketch {
            Sketch::Heart => "하트",
            Sketch::Star => "별",
            Sketch::Smile => "웃는 얼굴",
            Sketch::Spiral => "소용돌이",
            Sketch::Sun => "해",
            Sketch::Flower => "꽃",
            Sketch::Wave => "물결",
            Sketch::Snowflake => "눈송이",
        };
        self.queue_pin(Pin::new(
            Kind::OwnDoodle,
            sketch.name(),
            korean,
            self.date,
            &sketch.strokes(),
        ));
    }

    pub(super) fn keep_constellation(&mut self, constellation: &super::stars::Constellation) {
        let stroke = constellation
            .stars
            .iter()
            .map(|&index| super::stars::field()[usize::from(index)].position)
            .collect();
        self.queue_pin(Pin::new(
            Kind::Constellation,
            &constellation.name,
            &constellation.korean,
            self.date,
            &[stroke],
        ));
    }

    pub(super) fn carry_out_pin(&mut self) -> Option<Vec3> {
        if self.pending_pins.is_empty() && self.ghost.plan.count == 0
            || self.focus.is_some()
            || self.hunt.is_some()
            || self.telling.is_some()
            || self.ghost.asleep >= 0.3
        {
            self.ghost.plan.done = true;
            return None;
        }
        if self.ghost.plan.arrived.is_some() {
            let stayed = self.ghost.plan.stayed(self.time);
            if stayed >= 1.0 && self.ghost.plan.count == 0 {
                let mut pin = self.pending_pins.remove(0);
                pin.day = self.date.to_string();
                let event = EventKind::Pinned {
                    kind: pin.kind,
                    name: pin.name.clone(),
                };
                self.life.pin(pin);
                self.ghost.plan.count = 1;
                self.ghost.plan.target = VIEW;
                self.record(event);
                self.ghost.needs.change(0.0, 0.04, 0.0);
            }
            if stayed >= self.ghost.plan.length {
                self.ghost.plan.done = true;
            }
        }
        Some(CENTER)
    }

    pub fn carried_pin(&self) -> Option<Vec3> {
        (self.ghost.plan.activity == Activity::Pin
            && !self.ghost.plan.done
            && self.ghost.plan.count == 0
            && !self.pending_pins.is_empty())
        .then_some(self.ghost.position - Vec3::Y * (0.43 * self.size()))
    }

    pub(super) fn visit_gallery(&mut self) -> Option<Vec3> {
        if self.ghost.plan.arrived.is_some() {
            if self.ghost.plan.count == 0 {
                self.ghost.plan.count = 1;
                let index = (self.random.unit() * self.life.gallery.len() as f32) as usize;
                if let Some(pin) = self.life.gallery.get(index) {
                    self.think(&pin.recall());
                }
            }
            if self.ghost.plan.stayed(self.time) >= self.ghost.plan.length {
                self.ghost.plan.done = true;
            }
        }
        Some(CENTER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Lamp, Nook, Question, ROOM_MAX, SHELF_POSITION, Thing};

    fn pin(kind: Kind, name: &str) -> Pin {
        Pin::new(
            kind,
            name,
            name,
            Date::constant(2026, 6, 21),
            &Sketch::Heart.strokes(),
        )
    }

    fn with_drawing(seed: u64) -> World {
        let mut world = World::new(seed, 12.0);
        world.begin_life("2026-06-21");
        for point in &Sketch::Heart.strokes()[0] {
            world.draw_on_glass((*point * 0.4 + Vec2::new(0.0, 1.2)).extend(ROOM_MAX.z));
        }
        world.lift_finger();
        world.drawing.as_mut().unwrap().seen = true;
        world
    }

    fn finish_pin(world: &mut World) {
        world.begin(Activity::Pin);
        assert_eq!(world.ghost.plan.activity, Activity::Pin);
        world.ghost.position = APPROACH;
        world.ghost.velocity = Vec3::ZERO;
        world.step(7.0);
        assert!(world.ghost.plan.done);
    }

    #[test]
    fn the_first_viewer_drawing_is_always_kept_even_without_a_name() {
        for seed in 1..=32 {
            let mut world = with_drawing(seed);
            assert!(world.keep_viewer_drawing(None));
            assert_eq!(world.pending_pins[0].kind, Kind::ViewerDrawing);
            assert_eq!(world.pending_pins[0].name, "picture");
            assert!(world.life.gallery.is_empty());
        }
    }

    #[test]
    fn closeness_raises_the_chance_and_no_more_than_two_drawings_are_pinned_a_day() {
        let mut distant = 0;
        let mut close = 0;
        for seed in 1..=120 {
            for (bond, count) in [(0.0, &mut distant), (1.0, &mut close)] {
                let mut world = with_drawing(seed);
                world.life.total.viewer_pins = 1;
                world.life.bond = bond;
                *count += u32::from(world.keep_viewer_drawing(Some(("heart", "하트"))));
            }
        }
        assert!(distant > 20 && distant < 60, "{distant}");
        assert!(close > 60 && close < 100, "{close}");
        let mut world = with_drawing(7);
        for _ in 0..40 {
            if world.keep_viewer_drawing(None) {
                finish_pin(&mut world);
            }
        }
        assert_eq!(world.life.today.viewer_pins, 2);
        assert_eq!(world.life.total.viewer_pins, 2);
        world.date = world.date.tomorrow().unwrap();
        world.step(0.1);
        assert!((0..40).any(|_| world.keep_viewer_drawing(None)));
    }

    #[test]
    fn pending_cards_count_toward_daily_limits_and_a_doodle_is_kept_only_once() {
        let mut world = with_drawing(3);
        for _ in 0..40 {
            world.keep_viewer_drawing(None);
        }
        assert_eq!(world.pending_pins.len(), 2);
        world.pending_pins.clear();
        for _ in 0..80 {
            world.keep_doodle(Sketch::Star);
        }
        assert_eq!(world.pending_pins.len(), 1);
        finish_pin(&mut world);
        for _ in 0..80 {
            world.keep_doodle(Sketch::Heart);
        }
        assert!(world.pending_pins.is_empty());
        assert_eq!(world.life.today.doodle_pins, 1);
    }

    #[test]
    fn constellations_always_queue_even_when_a_doodle_must_make_room() {
        for seed in 1..=24 {
            let mut world = with_drawing(seed);
            world.queue_pin(pin(Kind::ViewerDrawing, "heart"));
            world.queue_pin(pin(Kind::OwnDoodle, "star"));
            let constellation = super::super::stars::Constellation {
                name: "the Little Heart".to_owned(),
                korean: "작은 하트".to_owned(),
                stars: vec![0, 1, 2, 3],
                found: world.date.to_string(),
            };
            world.keep_constellation(&constellation);
            assert_eq!(world.pending_pins.len(), 2);
            assert_eq!(world.pending_pins[0].kind, Kind::ViewerDrawing);
            assert_eq!(world.pending_pins[1].kind, Kind::Constellation);
            assert!(world.pending_pins[1].valid(&world.date.to_string()));
        }
    }

    #[test]
    fn eviction_keeps_viewer_drawings_and_falls_back_to_the_oldest_when_all_are_theirs() {
        let mut life = Life::default();
        for (kind, name) in [
            (Kind::ViewerDrawing, "heart"),
            (Kind::OwnDoodle, "star"),
            (Kind::Constellation, "sun"),
            (Kind::ViewerDrawing, "flower"),
            (Kind::OwnDoodle, "wave"),
        ] {
            life.pin(pin(kind, name));
        }
        assert_eq!(
            life.gallery
                .iter()
                .map(|pin| pin.name.as_str())
                .collect::<Vec<_>>(),
            ["heart", "sun", "flower", "wave"]
        );
        life.gallery.clear();
        for name in ["heart", "star", "sun", "flower", "wave"] {
            life.pin(pin(Kind::ViewerDrawing, name));
        }
        assert_eq!(life.gallery[0].name, "star");
        assert_eq!(life.gallery.len(), 4);
    }

    #[test]
    fn compact_strokes_round_trip_and_the_cache_does_not_change_saved_state() {
        let pin = pin(Kind::ViewerDrawing, "heart");
        assert!(pin.strokes.len() <= 12);
        assert!(pin.strokes.iter().map(Vec::len).sum::<usize>() <= 120);
        for point in pin.strokes.iter().flatten() {
            assert!(
                (*point * 1000.0 - (*point * 1000.0).round())
                    .abs()
                    .max_element()
                    < 0.0001
            );
        }
        let encoded = serde_json::to_string(&pin).unwrap();
        let restored: Pin = serde_json::from_str(&encoded).unwrap();
        pin.distance(Vec2::ZERO);
        assert_eq!(pin, restored);
        assert_eq!(encoded, serde_json::to_string(&pin).unwrap());
        assert!(restored.valid("2026-06-21"));
    }

    #[test]
    fn sanitization_drops_damaged_pins_and_bounds_counts_without_random_draws() {
        let good = pin(Kind::ViewerDrawing, "heart");
        let mut damaged = Vec::new();
        for day in ["damaged", "2026-02-30", "2027-06-21"] {
            let mut pin = good.clone();
            pin.day = day.to_owned();
            damaged.push(pin);
        }
        for name in ["", "heart\n", "heart\0", "heart7"] {
            let mut pin = good.clone();
            pin.name = name.to_owned();
            damaged.push(pin);
        }
        for strokes in [
            vec![],
            vec![vec![]],
            vec![vec![Vec2::ZERO; 121]],
            vec![vec![Vec2::ZERO, Vec2::ONE]; 13],
            vec![vec![Vec2::ZERO, Vec2::splat(1.1)]],
            vec![vec![Vec2::ZERO, Vec2::NAN]],
            vec![vec![Vec2::ZERO, Vec2::ZERO]],
        ] {
            let mut pin = good.clone();
            pin.strokes = strokes;
            damaged.push(pin);
        }
        let mut life = Life {
            day: good.day.clone(),
            gallery: damaged,
            ..Life::default()
        };
        life.gallery.push(good.clone());
        life.today.viewer_pins = u32::MAX;
        life.total.constellation_pins = u32::MAX;
        let life = life.sanitized();
        assert_eq!(life.gallery, [good]);
        assert_eq!(life.today.viewer_pins, 100_000_000);
        assert_eq!(life.total.constellation_pins, 100_000_000);
        let old: Life = serde_json::from_str("{}").unwrap();
        assert!(old.gallery.is_empty());
    }

    #[test]
    fn sanitization_trims_the_board_using_the_same_eviction_rule() {
        let mut life = Life {
            day: "2026-06-21".to_owned(),
            ..Life::default()
        };
        life.gallery.push(pin(Kind::ViewerDrawing, "heart"));
        for name in ["star", "sun", "flower", "wave", "spiral"] {
            life.gallery.push(pin(Kind::OwnDoodle, name));
        }
        let life = life.sanitized();
        assert_eq!(life.gallery.len(), 4);
        assert_eq!(life.gallery[0].name, "heart");
        assert_eq!(life.gallery[1].name, "flower");
    }

    #[test]
    fn pinning_carries_a_card_survives_interruption_and_records_only_on_placement() {
        let mut world = with_drawing(5);
        world.keep_viewer_drawing(Some(("heart", "하트")));
        assert!(world.options().contains(&Activity::Pin));
        world.begin(Activity::Pin);
        assert!(world.carried_pin().is_some());
        world.begin(Activity::Float);
        assert!(world.carried_pin().is_none());
        assert_eq!(world.pending_pins.len(), 1);
        assert!(world.life.gallery.is_empty());
        let bond = world.life.bond;
        finish_pin(&mut world);
        assert!(world.pending_pins.is_empty());
        assert!(world.carried_pin().is_none());
        assert_eq!(world.life.gallery.len(), 1);
        assert_eq!(world.life.total.viewer_pins, 1);
        assert!(world.life.bond > bond);
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|mark| mark.text == "I pinned your drawing on my wall.")
        );
        assert!(
            world
                .life
                .today
                .words()
                .contains("pinned your drawing to my wall")
        );
        assert!(world.events.iter().any(|event| matches!(&event.kind, EventKind::Pinned { kind: Kind::ViewerDrawing, name } if name == "heart")));
        assert_eq!(world.ghost.plan.target, VIEW);
    }

    #[test]
    fn sleep_work_games_hunts_and_stories_leave_the_card_waiting() {
        for activity in [Activity::Rest, Activity::Game, Activity::Puzzle] {
            let mut world = with_drawing(1);
            world.queue_pin(pin(Kind::ViewerDrawing, "heart"));
            world.begin(activity);
            if activity == Activity::Rest {
                world.ghost.asleep = 1.0;
            }
            assert!(!world.pin_is_open(), "{activity:?}");
            world.begin(Activity::Pin);
            assert_eq!(world.ghost.plan.activity, activity);
            assert_eq!(world.pending_pins.len(), 1);
        }
        let mut world = with_drawing(1);
        world.queue_pin(pin(Kind::ViewerDrawing, "heart"));
        world.toggle_focus();
        assert!(!world.options().contains(&Activity::Pin));
        world.end_focus();
        assert!(world.start_hiding().is_ok());
        assert!(!world.pin_is_open());
        world.cancel_hunt();
        world.tell(vec!["Once a picture stayed on my wall.".to_owned()]);
        assert!(!world.pin_is_open());
        assert_eq!(world.pending_pins.len(), 1);
    }

    #[test]
    fn a_taught_name_reaches_the_waiting_card_and_the_pinned_copy() {
        let mut world = with_drawing(4);
        world.keep_viewer_drawing(None);
        world.name_gallery_drawing("owl");
        assert_eq!(world.pending_pins[0].name, "owl");
        finish_pin(&mut world);
        world.name_gallery_drawing("부엉이");
        assert_eq!(world.life.gallery[0].name, "부엉이");
    }

    #[test]
    fn wall_questions_name_the_newest_pin_in_both_languages_without_teaching_words() {
        let mut world = with_drawing(1);
        assert!(world.life.gallery_answer(false).contains("empty"));
        world.life.pin(pin(Kind::OwnDoodle, "star"));
        let mut newest = pin(Kind::ViewerDrawing, "heart");
        newest.korean = "하트".to_owned();
        world.life.pin(newest);
        for text in ["what's on your wall?", "벽에 뭐 붙였어?"] {
            assert_eq!(Question::ask(text), Some(Question::Gallery));
            let heard = world.hear(text);
            assert!(matches!(
                heard.understanding,
                super::super::Understanding::Asked {
                    question: Question::Gallery,
                    ..
                }
            ));
            assert!(heard.learned.is_empty());
        }
        assert!(
            world
                .life
                .gallery_answer(false)
                .contains("The newest is a copy of the viewer's heart drawing")
        );
        assert!(world.life.gallery_answer(true).contains("하트"));
        assert!(world.life.gallery_words().contains("star doodle"));
        assert_eq!(world.position_of(Thing::Gallery), Some(CENTER));
        world.ghost.position = VIEW;
        world.ghost.heading = -Vec3::X;
        assert!(
            world
                .sight()
                .in_view
                .iter()
                .any(|seen| seen.thing == Thing::Gallery && seen.hidden_behind.is_none())
        );
    }

    #[test]
    fn the_board_is_inside_the_left_wall_and_clears_existing_furniture() {
        let world = World::new(1, 12.0);
        let minimum = CENTER - Vec3::new(DEPTH * 0.5, HEIGHT * 0.5, WIDTH * 0.5);
        let maximum = CENTER + Vec3::new(DEPTH * 0.5, HEIGHT * 0.5, WIDTH * 0.5);
        assert!((minimum.x - ROOM_MIN.x).abs() < 1e-5);
        assert!(minimum.cmpge(ROOM_MIN).all());
        assert!(maximum.cmple(ROOM_MAX).all());
        let distance_to_box = |point: Vec3| {
            (minimum - point)
                .max(point - maximum)
                .max(Vec3::ZERO)
                .length()
        };
        assert!(distance_to_box(world.lamp.position + Vec3::Y * Lamp::HEIGHT) > 0.31);
        assert!(minimum.z > SHELF_POSITION.z + 0.13);
        assert!(minimum.y > super::super::books::POSITION.y + 0.3);
        assert!(minimum.y > world.nook.position.y + Nook::HEIGHT);
    }
    #[test]
    fn a_first_viewer_picture_takes_priority_over_a_waiting_doodle() {
        let mut world = with_drawing(1);
        world.queue_pin(pin(Kind::Constellation, "little star"));
        world.queue_pin(pin(Kind::OwnDoodle, "star"));
        assert!(world.keep_viewer_drawing(None));
        assert_eq!(world.pending_pins.len(), 2);
        assert!(
            world
                .pending_pins
                .iter()
                .any(|pin| pin.kind == Kind::ViewerDrawing)
        );
        assert!(
            world
                .pending_pins
                .iter()
                .any(|pin| pin.kind == Kind::Constellation)
        );
    }

    #[test]
    fn malformed_saved_cards_do_not_prevent_other_cards_from_loading() {
        let good = serde_json::to_value(pin(Kind::ViewerDrawing, "heart")).unwrap();
        let mut unknown = good.clone();
        unknown["kind"] = serde_json::json!("Unknown");
        let mut broken = good.clone();
        broken["strokes"] = serde_json::json!("not strokes");
        let life: Life = serde_json::from_value(
            serde_json::json!({"day":"2026-06-21", "gallery":[unknown, broken, good]}),
        )
        .unwrap();
        assert_eq!(life.sanitized().gallery.len(), 1);
    }

    #[test]
    fn studying_a_drawing_queues_its_copy_and_speaks_the_promise_in_both_languages() {
        for (korean, expected) in [(false, "I'll keep this one!"), (true, "이거 간직할래!")] {
            let mut world = with_drawing(1);
            world.drawing.as_mut().unwrap().seen = false;
            world.life.korean = if korean { 1.0 } else { 0.0 };
            world.step(15.0);
            assert!(world.events.iter().any(|event| matches!(&event.kind, EventKind::SawDrawing { name: Some(name) } if name == "heart")));
            assert!(
                world
                    .pending_pins
                    .iter()
                    .any(|pin| pin.kind == Kind::ViewerDrawing
                        && pin.name == "heart"
                        && pin.korean == "하트")
            );
            assert!(world.events.iter().any(
                |event| matches!(&event.kind, EventKind::Said(line) if line.contains(expected))
            ));
        }
    }

    #[test]
    fn a_gallery_visit_remembers_a_picture_and_finishes_after_looking() {
        let mut world = with_drawing(1);
        world.life.pin(pin(Kind::ViewerDrawing, "heart"));
        let mut visited = false;
        for _ in 0..200 {
            world.begin(Activity::Explore);
            if world.ghost.plan.gallery_visit {
                visited = true;
                assert_eq!(world.ghost.plan.target, VIEW);
                world.ghost.position = VIEW;
                world.ghost.velocity = Vec3::ZERO;
                world.step(21.0);
                assert!(world.ghost.plan.done);
                assert_eq!(
                    world.ghost.thought.as_ref().unwrap().text,
                    "I still like the heart you drew me."
                );
                break;
            }
        }
        assert!(visited);
    }
}
