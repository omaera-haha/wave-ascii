use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use super::doodle::Path;
use super::recognizer::{self, Recognition};
use super::{
    Activity, EventKind, Heard, Listening, Mood, Picture, ROOM_MAX, ROOM_MIN, Thing, Understanding,
    World,
};

const MAX_POINTS: usize = 600;
const MAX_STROKES: usize = 24;

/// A name and a small, normalized picture, with pen lifts kept between strokes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LearnedPicture {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub strokes: Vec<Vec<Vec2>>,
}

impl LearnedPicture {
    pub(super) fn valid(&self) -> bool {
        valid_name(&self.name)
            && !self.strokes.is_empty()
            && self.strokes.len() <= MAX_STROKES
            && self.strokes.iter().all(|stroke| !stroke.is_empty())
            && (2..=64).contains(&self.strokes.iter().map(Vec::len).sum::<usize>())
            && self
                .strokes
                .iter()
                .flatten()
                .all(|point| point.is_finite() && point.abs().max_element() <= 1.0)
            && Path::new(self.strokes.clone()).length() > 1e-5
    }
}

/// The viewer's finger traces on the outside of the glass; these are not saved.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawing {
    pub strokes: Vec<Vec<Vec2>>,
    pub started: f64,
    pub touched: f64,
    pub seen: bool,
    pub minimum: Vec2,
    pub maximum: Vec2,
    path: Path,
    points: usize,
    down: bool,
    seen_at: Option<f64>,
    naming: bool,
    recognized: Option<Recognition>,
}

impl Drawing {
    fn new(point: Vec2, time: f64) -> Self {
        Self {
            strokes: Vec::new(),
            started: time,
            touched: time,
            seen: false,
            minimum: point,
            maximum: point,
            path: Path::new(Vec::new()),
            points: 0,
            down: false,
            seen_at: None,
            naming: false,
            recognized: None,
        }
    }

    pub(super) fn close_naming(&mut self) {
        self.naming = false;
    }

    pub fn center(&self) -> Vec2 {
        (self.minimum + self.maximum) * 0.5
    }

    pub fn opacity(&self, time: f64) -> f32 {
        ((190.0 - (time - self.touched)) / 40.0).clamp(0.0, 1.0) as f32
    }

    fn finished(&self, time: f64) -> bool {
        !self.down && time - self.touched >= 1.5 && self.path.length() > 0.04
    }

    pub fn distance(&self, point: Vec2) -> f32 {
        if point.cmplt(self.minimum - Vec2::splat(0.08)).any()
            || point.cmpgt(self.maximum + Vec2::splat(0.08)).any()
        {
            return f32::INFINITY;
        }
        self.path.distance(point, 1.0)
    }

    fn outside(&self, point: Vec2) -> f32 {
        (self.minimum - point)
            .max(point - self.maximum)
            .max(Vec2::ZERO)
            .length()
    }

    /// The names, never coordinates or recognition scores, reach the minds.
    pub fn words(&self) -> String {
        let picture = self.recognized.as_ref().map_or_else(
            || {
                if self.seen {
                    "something you can't make out".to_owned()
                } else {
                    "a picture you haven't studied yet".to_owned()
                }
            },
            |recognized| format!("a {}", recognized.name),
        );
        format!("the viewer drew {picture} on the glass")
    }
}

impl World {
    pub fn draw_on_glass(&mut self, point: Vec3) {
        self.notice_viewer();
        if !point.is_finite()
            || point.x < ROOM_MIN.x
            || point.x > ROOM_MAX.x
            || point.y < ROOM_MIN.y
            || point.y > ROOM_MAX.y
        {
            self.lift_finger();
            return;
        }
        let point = point.truncate();
        let fresh = self.drawing.as_ref().is_none_or(|drawing| {
            !drawing.down && (self.time - drawing.touched > 6.0 || drawing.outside(point) > 0.9)
        });
        if fresh {
            self.drawing = Some(Drawing::new(point, self.time));
        }
        let drawing = self.drawing.as_mut().unwrap();
        if drawing.points >= MAX_POINTS || (!drawing.down && drawing.strokes.len() >= MAX_STROKES) {
            return;
        }
        if drawing.down
            && drawing
                .strokes
                .last()
                .and_then(|stroke| stroke.last())
                .is_some_and(|last| last.distance(point) < 0.02)
        {
            return;
        }
        if !drawing.down {
            drawing.strokes.push(Vec::new());
        }
        drawing.down = true;
        drawing.strokes.last_mut().unwrap().push(point);
        drawing.points += 1;
        drawing.minimum = drawing.minimum.min(point);
        drawing.maximum = drawing.maximum.max(point);
        drawing.touched = self.time;
        drawing.seen = false;
        drawing.seen_at = None;
        drawing.naming = false;
        drawing.recognized = None;
        drawing.path = Path::new(drawing.strokes.clone());
    }

    pub fn lift_finger(&mut self) {
        self.notice_viewer();
        if let Some(drawing) = &mut self.drawing {
            if drawing.down {
                drawing.touched = self.time;
            }
            drawing.down = false;
        }
    }

    pub(super) fn look_is_open(&self) -> bool {
        self.ghost.asleep < 0.3
            && self.ghost.fade < 0.3
            && (self.ghost.plan.done
                || !matches!(self.ghost.plan.activity, Activity::Hide | Activity::Game))
            && self
                .drawing
                .as_ref()
                .is_some_and(|drawing| !drawing.seen && drawing.finished(self.time))
    }

    pub(super) fn advance_fog(&mut self) {
        if self
            .drawing
            .as_ref()
            .is_some_and(|drawing| drawing.opacity(self.time) <= 0.0)
        {
            self.drawing = None;
        }
        if !self.look_is_open()
            || self.ghost.plan.activity == Activity::Look && !self.ghost.plan.done
        {
            return;
        }
        self.attend("drawing", 8.0);
        let plan = &self.ghost.plan;
        let busy = !plan.done
            && (matches!(
                plan.activity,
                Activity::Rest | Activity::Tend | Activity::Game
            ) || plan.activity == Activity::Fetch && self.ball.held
                || plan.activity == Activity::Build && self.blocks.iter().any(|block| block.held));
        if !busy {
            self.begin(Activity::Look);
        }
    }

    pub(super) fn study_drawing(&mut self) -> Option<Vec3> {
        let Some(drawing) = self.drawing.as_ref() else {
            self.ghost.plan.done = true;
            return None;
        };
        if drawing.seen || drawing.started > self.ghost.plan.started {
            self.ghost.plan.done = true;
            return None;
        }
        if !drawing.finished(self.time) {
            self.ghost.plan.arrived = None;
            return Some(drawing.center().extend(ROOM_MAX.z));
        }
        let stayed = self.ghost.plan.stayed(self.time);
        let progress = ((stayed * 4.0).floor() / (self.ghost.plan.length * 4.0)).clamp(0.0, 1.0);
        let look = drawing.path.point_at(progress).extend(ROOM_MAX.z);
        // Attention identifies the drawing; the pen path moves the actual gaze.
        self.ghost.attention = Some((Thing::Drawing, self.time));
        if self.ghost.plan.arrived.is_some() && stayed >= self.ghost.plan.length {
            self.saw_drawing();
        }
        Some(look)
    }

    fn saw_drawing(&mut self) {
        let drawing = self.drawing.as_mut().unwrap();
        let recognition = recognizer::recognize(&drawing.strokes, &self.life.pictures);
        drawing.seen = true;
        drawing.seen_at = Some(self.time);
        drawing.naming = true;
        drawing.recognized = recognition.clone();
        let name = recognition
            .as_ref()
            .map(|recognized| recognized.name.clone());
        self.record(EventKind::SawDrawing { name });
        let korean = self.latest_korean || self.speaks_korean();
        // A flower drawn while one of its own pots is in flower.
        let season = self.season();
        let blooming = recognition
            .as_ref()
            .filter(|recognized| recognized.sketch == Some(super::Sketch::Flower))
            .and_then(|_| {
                self.seedlings
                    .iter()
                    .find(|seedling| seedling.flowering(season))
            })
            .map(|seedling| seedling.kind);
        let mut line = match &recognition {
            Some(recognized) if korean => format!(
                "{}!{}",
                recognized.korean,
                if recognized.learned {
                    " 네가 알려줬잖아."
                } else {
                    ""
                }
            ),
            Some(recognized) => {
                format!(
                    "{}!{}",
                    with_article(&recognized.name),
                    if recognized.learned {
                        " You taught me that."
                    } else {
                        ""
                    }
                )
            }
            None if korean => "뭐야?".to_owned(),
            None => "What's that?".to_owned(),
        };
        if let Some(flower) = blooming {
            line.push_str(&if korean {
                format!(" 내 {}처럼!", flower.korean())
            } else {
                format!(" Like my {}!", flower.name())
            });
        }
        self.say(&line);
        let mood = match &recognition {
            Some(recognized) if recognized.learned => Mood::Excited,
            Some(recognized) if matches!(recognized.name.as_str(), "heart" | "smile") => {
                Mood::Cheerful
            }
            _ => Mood::Curious,
        };
        self.suggest_mood(mood, 8.0);
        self.ghost.plan.done = true;
        let sketch = recognition
            .as_ref()
            .and_then(|recognized| recognized.sketch);
        let chance = answer_chance(
            self.life.nature.unwrap_or_default().playful,
            self.life.bond,
            sketch.is_some(),
        );
        if self.random.unit() >= chance {
            return;
        }
        let drawing = self.drawing.as_ref().unwrap();
        let picture = sketch.map_or_else(
            || {
                Picture::copy(
                    recognition
                        .as_ref()
                        .map_or("picture", |recognized| recognized.name.as_str())
                        .to_owned(),
                    &drawing.strokes,
                    self.life.total.watched_writings < 3,
                )
            },
            Picture::Sketch,
        );
        if let Some(center) = answer_position(drawing, &picture) {
            self.begin(Activity::Doodle);
            self.ghost.plan.picture = Some(picture);
            self.ghost.plan.target = center.extend(ROOM_MAX.z - 0.35);
            self.ghost.plan.anchor = self.ghost.plan.target;
        }
    }

    pub(super) fn name_drawing(&mut self, text: &str) -> Option<Heard> {
        let drawing = self.drawing.as_mut()?;
        if !drawing.naming || !drawing.seen_at.is_some_and(|at| self.time - at <= 60.0) {
            return None;
        }
        drawing.naming = false;
        let name = picture_name(text)?;
        // A bare word the ghost already knows asks for something; "it's a
        // fetch" would still name the picture.
        let explicit = name != text.trim().to_lowercase();
        if !explicit
            && self
                .life
                .vocabulary
                .meaning(&super::vocabulary::tokens(text))
                .is_some()
        {
            return None;
        }
        let drawing = self.drawing.as_mut()?;
        let mut strokes = recognizer::normalized(&drawing.strokes);
        for point in strokes.iter_mut().flatten() {
            *point = (*point * 1000.0).round() / 1000.0;
        }
        let picture = LearnedPicture {
            name: name.clone(),
            strokes,
        };
        if !picture.valid() {
            return None;
        }
        self.life.pictures.retain(|picture| picture.name != name);
        self.life.pictures.push(picture);
        if self.life.pictures.len() > 8 {
            self.life.pictures.remove(0);
        }
        drawing.recognized = Some(Recognition {
            name: name.clone(),
            korean: name.clone(),
            sketch: None,
            learned: true,
        });
        self.listening = Listening::default();
        self.wake();
        self.ghost.needs.change(0.0, 0.05, 0.25);
        self.notice_viewer();
        self.record(EventKind::Heard(text.to_owned()));
        self.record(EventKind::LearnedPicture(name.clone()));
        Some(Heard {
            tone: None,
            understanding: Understanding::Named { picture: name },
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean: self.latest_korean,
        })
    }
}

// "A heart", "An owl", or a name in another script as it is.
fn with_article(name: &str) -> String {
    match name.chars().next() {
        Some(first) if !first.is_ascii_alphabetic() => name.to_owned(),
        Some('a' | 'e' | 'i' | 'o' | 'u' | 'A' | 'E' | 'I' | 'O' | 'U') => format!("An {name}"),
        _ => format!("A {name}"),
    }
}

pub(super) fn answer_chance(playful: f32, bond: f32, sketch: bool) -> f32 {
    let chance = (0.35 + 0.3 * playful + 0.3 * bond).clamp(0.0, 0.9);
    if sketch { chance } else { chance * 0.35 }
}

fn answer_position(drawing: &Drawing, picture: &Picture) -> Option<Vec2> {
    let extent = picture.extent() * picture.size();
    let lower = ROOM_MIN.truncate() + extent + Vec2::splat(0.08);
    let upper = ROOM_MAX.truncate() - extent - Vec2::splat(0.08);
    let vertical = drawing.center().y.clamp(lower.y, upper.y);
    // Prefer the side with more room. Neither answer may overlap the trace.
    let left = drawing.minimum.x - extent.x - 0.12;
    let right = drawing.maximum.x + extent.x + 0.12;
    let positions = if drawing.minimum.x - ROOM_MIN.x > ROOM_MAX.x - drawing.maximum.x {
        [left, right]
    } else {
        [right, left]
    };
    positions
        .into_iter()
        .find(|&horizontal| horizontal >= lower.x && horizontal <= upper.x)
        .map(|horizontal| Vec2::new(horizontal, vertical))
}

fn valid_name(name: &str) -> bool {
    let words: Vec<&str> = name.split(' ').collect();
    (1..=2).contains(&words.len())
        && (1..=16).contains(&name.chars().count())
        && words
            .iter()
            .all(|word| !word.is_empty() && word.chars().all(char::is_alphabetic))
}

fn picture_name(text: &str) -> Option<String> {
    let text = text.trim().to_lowercase();
    let mut name = text.as_str();
    for prefix in [
        "it's ",
        "it’s ",
        "it is ",
        "that's ",
        "that’s ",
        "that is ",
        "this is ",
    ] {
        if let Some(rest) = name.strip_prefix(prefix) {
            name = rest;
            break;
        }
    }
    for article in ["a ", "an ", "the "] {
        if let Some(rest) = name.strip_prefix(article) {
            name = rest;
            break;
        }
    }
    for suffix in [" 그림이야", "그림이야", "야"] {
        if let Some(rest) = name.strip_suffix(suffix) {
            name = rest.trim();
            break;
        }
    }
    if matches!(
        name,
        "hello"
            | "hi"
            | "thanks"
            | "thank you"
            | "well done"
            | "goodnight"
            | "no"
            | "yes"
            | "안녕"
            | "고마워"
    ) {
        return None;
    }
    if name == text && super::vocabulary::read(&text, "").tone.is_some() {
        return None;
    }
    valid_name(name).then(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Random, Sketch};

    fn trace(world: &mut World, strokes: &[Vec<Vec2>]) {
        for stroke in strokes {
            for point in stroke {
                world.draw_on_glass((*point * 0.4 + Vec2::new(0.0, 1.2)).extend(ROOM_MAX.z));
            }
            world.lift_finger();
        }
    }

    fn scribble() -> Vec<Vec<Vec2>> {
        let mut random = Random::new(51);
        vec![
            (0..45)
                .map(|_| Vec2::new(random.range(-1.0, 1.0), random.range(-1.0, 1.0)))
                .collect(),
        ]
    }

    fn studied(world: &mut World, strokes: &[Vec<Vec2>]) {
        trace(world, strokes);
        world.step(15.0);
        assert!(world.drawing.as_ref().unwrap().seen);
    }

    #[test]
    fn a_finished_heart_is_studied_named_and_sometimes_answered() {
        let mut answered = false;
        let mut only_looked = false;
        for seed in 1..=12 {
            let mut world = World::new(seed, 12.0);
            trace(&mut world, &Sketch::Heart.strokes());
            world.step(1.4);
            assert!(!world.drawing.as_ref().unwrap().seen);
            world.step(0.2);
            assert_eq!(world.ghost.plan.activity, Activity::Look);
            assert!(world.options().contains(&Activity::Look));
            world.step(14.0);
            assert!(world.drawing.as_ref().unwrap().seen);
            assert!(!world.options().contains(&Activity::Look));
            assert!(world.events.iter().any(|event| event.kind
                == EventKind::SawDrawing {
                    name: Some("heart".to_owned())
                }));
            assert_eq!(world.life.total.drawings_seen, 1);
            assert!(world.life.bond > 0.005);
            if world
                .doodles
                .iter()
                .any(|doodle| doodle.picture == Picture::Sketch(Sketch::Heart))
            {
                answered = true;
            } else {
                only_looked = true;
            }
            if answered && only_looked {
                break;
            }
        }
        assert!(answered && only_looked);
        assert_eq!(answer_chance(1.0, 1.0, true), 0.9);
        assert!(answer_chance(0.5, 0.5, false) < answer_chance(0.5, 0.5, true));
    }

    #[test]
    fn a_drawn_flower_brings_its_own_flowering_pot_to_mind() {
        let said = |world: &World| {
            world
                .events
                .iter()
                .filter_map(|event| match &event.kind {
                    EventKind::Said(text) => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let mut world = World::new(1, 12.0);
        let mut tulip = crate::world::Seedling::new(crate::world::Flower::Tulip);
        tulip.growth = 1.0;
        world.seedlings.push(tulip);
        studied(&mut world, &Sketch::Flower.strokes());
        assert!(said(&world).contains(&"A flower! Like my tulip!".to_owned()));

        let mut world = World::new(1, 12.0);
        world.life.korean = 0.9;
        let mut sunflower = crate::world::Seedling::new(crate::world::Flower::Sunflower);
        sunflower.growth = 1.0;
        world.seedlings.push(sunflower);
        studied(&mut world, &Sketch::Flower.strokes());
        assert!(said(&world).contains(&"꽃! 내 해바라기처럼!".to_owned()));

        let mut world = World::new(1, 12.0);
        studied(&mut world, &Sketch::Flower.strokes());
        assert!(said(&world).contains(&"A flower!".to_owned()));
        assert_eq!(with_article("owl"), "An owl");
        assert_eq!(with_article("cat"), "A cat");
        assert_eq!(with_article("고양이"), "고양이");
    }

    #[test]
    fn a_known_command_after_a_drawing_is_obeyed_rather_than_taken_for_its_name() {
        let mut world = World::new(1, 12.0);
        world
            .life
            .vocabulary
            .teach(&["twirl".to_owned()], Activity::Dance, 4.0);
        studied(&mut world, &Sketch::Heart.strokes());
        let heard = world.hear("twirl");
        assert!(matches!(
            heard.understanding,
            Understanding::Doing {
                activity: Activity::Dance,
                ..
            }
        ));
        assert!(world.life.pictures.is_empty());

        let mut world = World::new(1, 12.0);
        world
            .life
            .vocabulary
            .teach(&["twirl".to_owned()], Activity::Dance, 4.0);
        studied(&mut world, &Sketch::Heart.strokes());
        world.hear("it's a twirl");
        assert_eq!(world.life.pictures[0].name, "twirl");
    }

    #[test]
    fn a_sleeping_ghost_leaves_the_picture_unseen_until_it_wakes() {
        let mut world = World::new(1, 23.0);
        world.tuck_in();
        trace(&mut world, &Sketch::Heart.strokes());
        world.step(15.0);
        assert!(world.ghost.asleep > 0.9);
        assert!(!world.drawing.as_ref().unwrap().seen);
        assert!(!world.options().contains(&Activity::Look));
        world.hour = 12.0;
        world.poke();
        world.step(15.0);
        assert!(world.drawing.as_ref().unwrap().seen);
    }

    #[test]
    fn studying_waits_for_important_work_and_an_active_game() {
        for activity in [
            Activity::Rest,
            Activity::Tend,
            Activity::Fetch,
            Activity::Build,
            Activity::Game,
            Activity::Hide,
        ] {
            let mut world = World::new(1, 12.0);
            world.begin(activity);
            if activity == Activity::Fetch {
                world.ball.held = true;
            }
            if activity == Activity::Build {
                world.blocks[0].held = true;
            }
            trace(&mut world, &Sketch::Heart.strokes());
            world.time = 2.0;
            world.advance_fog();
            assert_eq!(world.ghost.plan.activity, activity, "{activity:?}");
            assert!(!world.drawing.as_ref().unwrap().seen);
            if matches!(activity, Activity::Game | Activity::Hide) {
                assert!(!world.look_is_open());
            }
        }
    }

    #[test]
    fn naming_a_scribble_teaches_a_picture_and_a_single_first_milestone() {
        let mut world = World::new(1, 12.0);
        studied(&mut world, &scribble());
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::SawDrawing { name: None })
        );
        let vocabulary = world.life.vocabulary.clone();
        let heard = world.hear("it's a cat");
        assert_eq!(
            heard.understanding,
            Understanding::Named {
                picture: "cat".to_owned()
            }
        );
        assert_eq!(world.life.vocabulary, vocabulary);
        assert_eq!(world.life.total.pictures_learned, 1);
        assert!(world.life.pictures[0].valid());
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "You taught me to draw a cat.")
        );
        studied(&mut world, &scribble());
        assert_eq!(
            world
                .drawing
                .as_ref()
                .unwrap()
                .recognized
                .as_ref()
                .unwrap()
                .name,
            "cat"
        );
        assert!(
            world
                .ghost
                .speech
                .as_ref()
                .unwrap()
                .text
                .contains("You taught me that")
        );
        world.hear("a cat");
        assert_eq!(world.life.pictures.len(), 1);
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text.starts_with("You taught me to draw"))
                .count(),
            1
        );
        let saved = serde_json::to_string(&world.life).unwrap();
        let loaded: Life = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded.sanitized().pictures, world.life.pictures);
    }

    #[test]
    fn names_accept_english_and_korean_fillers_and_reject_other_messages() {
        for (text, expected) in [
            ("cat", "cat"),
            ("a cat", "cat"),
            ("it's a cat", "cat"),
            ("that's an owl", "owl"),
            ("it is a little cat", "little cat"),
            ("고양이", "고양이"),
            ("고양이야", "고양이"),
            ("고양이 그림이야", "고양이"),
        ] {
            assert_eq!(picture_name(text).as_deref(), Some(expected), "{text}");
        }
        for text in [
            "",
            "hello",
            "thank you",
            "good",
            "well done",
            "잘했어",
            "a big little cat",
            "cat123",
            "cat!",
            "abcdefghijklmnopq",
            "고양이?",
        ] {
            assert_eq!(picture_name(text), None, "{text}");
        }
        let mut world = World::new(1, 12.0);
        studied(&mut world, &scribble());
        let heard = world.hear("고양이 그림이야");
        assert!(heard.korean);
        assert_eq!(
            heard.understanding,
            Understanding::Named {
                picture: "고양이".to_owned()
            }
        );
    }

    #[test]
    fn a_wrong_recognition_can_be_corrected_and_an_expired_or_used_offer_teaches_nothing() {
        let mut world = World::new(1, 12.0);
        studied(&mut world, &Sketch::Heart.strokes());
        assert!(matches!(
            world.hear("it's a cat").understanding,
            Understanding::Named { .. }
        ));
        studied(&mut world, &Sketch::Heart.strokes());
        assert_eq!(
            world
                .drawing
                .as_ref()
                .unwrap()
                .recognized
                .as_ref()
                .unwrap()
                .name,
            "cat"
        );
        studied(&mut world, &scribble());
        world.time += 61.0;
        assert!(!matches!(
            world.hear("a dog").understanding,
            Understanding::Named { .. }
        ));
        assert_eq!(world.life.pictures.len(), 1);
        studied(&mut world, &scribble());
        world.hear("hello");
        assert!(!matches!(
            world.hear("a dog").understanding,
            Understanding::Named { .. }
        ));
        studied(&mut world, &scribble());
        world.hear("what are you doing?");
        assert!(!matches!(
            world.hear("a dog").understanding,
            Understanding::Named { .. }
        ));
    }

    #[test]
    fn new_examples_replace_known_names_and_only_the_newest_eight_are_kept() {
        let mut world = World::new(1, 12.0);
        for name in [
            "cat", "dog", "owl", "tree", "fish", "boat", "leaf", "bear", "duck",
        ] {
            studied(&mut world, &scribble());
            world.hear(name);
        }
        assert_eq!(world.life.pictures.len(), 8);
        assert_eq!(world.life.pictures[0].name, "dog");
        let previous = world.life.pictures[0].strokes.clone();
        studied(&mut world, &Sketch::Star.strokes());
        world.hear("dog");
        assert_eq!(world.life.pictures.len(), 8);
        let last = world.life.pictures.last().unwrap();
        assert_eq!(last.name, "dog");
        assert_ne!(last.strokes, previous);
    }

    #[test]
    fn old_saves_have_no_pictures_and_damaged_saved_pictures_are_dropped() {
        let old: Life = serde_json::from_str("{}").unwrap();
        assert!(old.pictures.is_empty());
        let good = LearnedPicture {
            name: "cat".to_owned(),
            strokes: recognizer::normalized(&scribble()),
        };
        let mut life = Life::default();
        life.pictures.push(good.clone());
        for invalid in [f32::NAN, f32::INFINITY, 1.01, -1.01] {
            let mut damaged = good.clone();
            damaged.strokes[0][0].x = invalid;
            life.pictures.push(damaged);
        }
        for name in ["", "cat2", "three word name", "abcdefghijklmnopq"] {
            let mut damaged = good.clone();
            damaged.name = name.to_owned();
            life.pictures.push(damaged);
        }
        let mut damaged = good.clone();
        damaged.strokes = vec![vec![Vec2::ZERO; 65]];
        life.pictures.push(damaged);
        let mut damaged = good.clone();
        damaged.strokes.clear();
        life.pictures.push(damaged);
        let mut damaged = good.clone();
        damaged.strokes = vec![vec![Vec2::ZERO; 2]];
        life.pictures.push(damaged);
        assert_eq!(life.sanitized().pictures, vec![good.clone()]);
        let missing: LearnedPicture = serde_json::from_str("{}").unwrap();
        assert!(!missing.valid());
        let mut life = Life::default();
        for name in [
            "cat", "dog", "owl", "tree", "fish", "boat", "leaf", "bear", "duck",
        ] {
            let mut picture = good.clone();
            picture.name = name.to_owned();
            life.pictures.push(picture);
        }
        life.pictures.push(good.clone());
        let life = life.sanitized();
        assert_eq!(life.pictures.len(), 8);
        assert_eq!(life.pictures.last().unwrap().name, "cat");
    }

    #[test]
    fn traces_expire_after_the_full_opacity_and_fading_intervals() {
        let mut world = World::new(1, 12.0);
        trace(&mut world, &Sketch::Heart.strokes());
        let drawing = world.drawing.as_ref().unwrap();
        assert_eq!(drawing.opacity(150.0), 1.0);
        assert_eq!(drawing.opacity(170.0), 0.5);
        world.time = 190.0;
        world.advance_fog();
        assert!(world.drawing.is_none());
    }

    #[test]
    fn nearby_points_are_skipped_and_points_and_strokes_stay_capped() {
        let mut world = World::new(1, 12.0);
        world.draw_on_glass(Vec3::new(0.0, 1.0, ROOM_MAX.z));
        world.draw_on_glass(Vec3::new(0.01, 1.0, ROOM_MAX.z));
        assert_eq!(world.drawing.as_ref().unwrap().points, 1);
        for index in 0..900 {
            world.draw_on_glass(Vec3::new((index % 2) as f32 * 0.1, 1.0, ROOM_MAX.z));
        }
        assert_eq!(world.drawing.as_ref().unwrap().points, MAX_POINTS);
        assert_eq!(world.drawing.as_ref().unwrap().strokes.len(), 1);
        world.drawing = None;
        for _ in 0..40 {
            world.draw_on_glass(Vec3::new(0.0, 1.0, ROOM_MAX.z));
            world.draw_on_glass(Vec3::new(0.1, 1.0, ROOM_MAX.z));
            world.lift_finger();
        }
        assert_eq!(world.drawing.as_ref().unwrap().strokes.len(), MAX_STROKES);
        assert_eq!(world.drawing.as_ref().unwrap().points, MAX_STROKES * 2);
    }

    #[test]
    fn a_late_or_far_new_stroke_replaces_the_picture_and_leaving_the_pane_lifts_the_finger() {
        let mut world = World::new(1, 12.0);
        trace(&mut world, &Sketch::Heart.strokes());
        world.time = 7.0;
        world.draw_on_glass(Vec3::new(0.0, 1.0, ROOM_MAX.z));
        assert_eq!(world.drawing.as_ref().unwrap().points, 1);
        world.lift_finger();
        world.draw_on_glass(Vec3::new(1.3, 1.0, ROOM_MAX.z));
        assert_eq!(world.drawing.as_ref().unwrap().points, 1);
        world.draw_on_glass(Vec3::new(3.0, 1.0, ROOM_MAX.z));
        assert!(!world.drawing.as_ref().unwrap().down);
        world.draw_on_glass(Vec3::new(1.4, 1.0, ROOM_MAX.z));
        assert_eq!(world.drawing.as_ref().unwrap().strokes.len(), 2);
    }

    #[test]
    fn answer_placement_fits_the_pane_without_overlapping_the_viewers_picture() {
        let mut world = World::new(1, 12.0);
        for horizontal in [-1.5, 0.0, 1.5] {
            world.drawing = None;
            world.draw_on_glass(Vec3::new(horizontal, 1.0, ROOM_MAX.z));
            world.draw_on_glass(Vec3::new(horizontal + 0.1, 1.2, ROOM_MAX.z));
            let drawing = world.drawing.as_ref().unwrap();
            let picture = Picture::Sketch(Sketch::Heart);
            let center = answer_position(drawing, &picture).unwrap();
            let extent = picture.extent() * picture.size();
            assert!((center - extent).cmpge(ROOM_MIN.truncate()).all());
            assert!((center + extent).cmple(ROOM_MAX.truncate()).all());
            assert!(
                center.x + extent.x < drawing.minimum.x || center.x - extent.x > drawing.maximum.x
            );
        }
    }

    #[test]
    fn drawing_recognition_and_answers_use_only_the_seeded_world_random_source() {
        let mut first = World::new(8, 12.0);
        let mut second = World::new(8, 12.0);
        for world in [&mut first, &mut second] {
            studied(world, &Sketch::Heart.strokes());
            world.hear("a cat");
            studied(world, &scribble());
            world.hear("a dog");
            world.step(10.0);
        }
        assert_eq!(first.events, second.events);
        assert_eq!(first.drawing, second.drawing);
        assert_eq!(first.doodles, second.doodles);
        assert_eq!(first.life, second.life);
        assert_eq!(first.ghost.position, second.ghost.position);
    }

    #[test]
    fn recognized_pictures_follow_the_latest_messages_language() {
        let mut world = World::new(1, 12.0);
        world.hear("안녕");
        studied(&mut world, &Sketch::Heart.strokes());
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "하트!");
        world.hear("hello");
        studied(&mut world, &Sketch::Heart.strokes());
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "A heart!");
    }

    #[test]
    fn learned_pictures_join_doodling_when_the_bond_is_warm_enough() {
        let mut world = World::new(9, 12.0);
        studied(&mut world, &scribble());
        world.hear("a cat");
        world.life.bond = 0.29;
        assert!((0..40).all(|_| !matches!(world.choose_picture(), Picture::Copy(_))));
        world.life.bond = 0.6;
        let picture = (0..40)
            .find_map(|_| match world.choose_picture() {
                picture @ Picture::Copy(_) => Some(picture),
                _ => None,
            })
            .unwrap();
        assert_eq!(picture.name(), "cat");
        world.begin(Activity::Doodle);
        world.ghost.plan.picture = Some(picture);
        world.ghost.position = world.ghost.plan.target;
        world.step(8.0);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Copied("cat".to_owned()))
        );
        assert!(world.life.total.doodles > 0);
    }

    #[test]
    fn drawing_descriptions_and_naming_replies_reach_the_minds_as_words() {
        let mut world = World::new(1, 12.0);
        studied(&mut world, &scribble());
        let observation = world.drawing.as_ref().unwrap().words();
        assert!(observation.contains("the viewer drew something you can't make out on the glass"));
        assert!(
            !observation
                .chars()
                .any(|character| character.is_ascii_digit())
        );
        let mut mind = crate::mind::Mind::new(
            &crate::store::Config::default(),
            crate::mind::Journal::default(),
            true,
        );
        mind.hear(&mut world, "it's a cat");
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "A cat! I'll remember."
        );
        assert!(
            world
                .life
                .today
                .words()
                .contains("learned new pictures from you")
        );
        studied(&mut world, &scribble());
        mind.hear(&mut world, "고양이야");
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "고양이! 기억할게."
        );
    }
}
