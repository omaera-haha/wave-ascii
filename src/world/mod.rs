mod activity;
pub mod calendar;
mod doodle;
pub mod game;
mod life;
mod nature;
mod needs;
pub mod occasion;
mod pounce;
mod question;
mod random;
mod routine;
mod sight;
mod sleep;
mod things;
pub mod visitor;
mod vocabulary;
mod weather;

use glam::{Vec2, Vec3};

pub use activity::{Activity, Plan};
pub use calendar::Season;
pub use doodle::{Doodle, Sketch};
pub use game::{Game, Mark, Outcome};
pub use life::{Entry, Life};
pub use nature::Nature;
pub use needs::{Mood, Needs};
pub use occasion::{Decoration, Occasion};
pub use question::Question;
pub use random::Random;
pub use sight::{Seen, Thing};
pub use things::{
    Ball, Lamp, Mote, Nook, Plant, ROOM_MAX, ROOM_MIN, SHELF_POSITION, SILL_POSITION,
    WINDOW_CENTER, flat,
};
pub use visitor::{BirdPicture, Keepsake, Perch, Visitors};
pub use vocabulary::{Tone, Vocabulary};
use weather::Change;
pub use weather::{Sky, Weather};

/// At most this many motes drift in the room at once.
const MAX_MOTES: usize = 6;
/// At most this many pictures remain on the glass at once.
const MAX_DOODLES: usize = 3;
/// How close the ghost has to come to a place to have arrived.
const ARRIVED: f32 = 0.2;
/// Seconds a plan may run before the ghost gives up on it.
const GIVE_UP: f32 = 60.0;
/// Seconds after the viewer's last input that they still count as present.
const PRESENT_FOR: f64 = 90.0;
/// Pokes closer together than this many seconds count as one streak.
const POKE_STREAK: f64 = 1.5;
/// Recent events kept for the face and the minds.
const EVENTS_KEPT: usize = 80;
/// How close the ghost must feel to the viewer to come and greet them.
pub(crate) const GREETS_FROM: f32 = 0.3;
/// How much it learns about the viewer's words from each kind of hint:
/// hearing them while it does something, being praised for doing what it
/// took them to mean, being praised for a guess, a mind telling it what
/// they asked for, the viewer showing it, and being told no.
const NAMED: f32 = 1.0;
const PRAISED: f32 = 1.0;
const CONFIRMED: f32 = 2.0;
const ROUTED: f32 = 1.5;
const SHOWN: f32 = 1.5;
const REFUSED: f32 = -1.5;
/// Seconds after it began something that the viewer's words may still be
/// about it.
const NAMED_FOR: f64 = 20.0;
/// Seconds after the ghost acted on words that praise or a no still counts
/// for them.
const HEEDED_FOR: f64 = 30.0;
/// Seconds after the viewer spoke that what they do may show what they
/// meant.
const SHOWN_FOR: f64 = 12.0;
/// How much petting one unit of a stroke across the ghost adds, and the
/// seconds it takes to fade to about a third once the strokes stop.
const STROKING: f32 = 2.0;
const PETTING_FADES: f32 = 1.2;
/// How much petting it takes to feel it: to calm down and count a stroke.
const PETTED: f32 = 0.35;

/// Something that happened, which the ghost's face reacts to and its minds
/// hear about.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub time: f64,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    Began(Activity),
    /// The viewer poked the ghost, the `count`th time in a row.
    Poked {
        count: u32,
    },
    AteMote,
    NudgedBall,
    PickedUpBall,
    Fetched,
    Danced,
    Tended,
    Bloomed,
    SwitchedLamp {
        on: bool,
    },
    FellAsleep,
    WokeUp,
    /// The viewer said something.
    Heard(String),
    /// The ghost said something.
    Said(String),
    ViewerArrived,
    ViewerLeft,
    ExpectedArrival,
    MissedVisit,
    /// The viewer dropped a mote into the room.
    DroppedMote,
    /// The viewer flicked the ball.
    KickedBall,
    /// The viewer came back after the app was closed for `away` seconds.
    Returned {
        away: f64,
    },
    /// The ghost caught sight of something new to it.
    Noticed(String),
    /// A first or a landmark in the ghost's life, in its own words.
    Milestone(String),
    RainStarted,
    StormStarted,
    Thunder,
    RainStopped,
    SnowStarted,
    SnowStopped,
    RainbowAppeared,
    RainbowFaded,
    Found,
    Jumped,
    Doodled(&'static str),
    GameOver(Outcome),
    Celebrated(Occasion),
    Pounced,
    Morning,
    /// The viewer sounded pleased with the ghost.
    Praised,
    /// The viewer told the ghost no.
    Scolded,
    /// The viewer stroked the ghost.
    Petted,
    /// The viewer said a word the ghost doesn't know yet.
    Puzzled(String),
    /// The ghost worked out what one of the viewer's words means.
    LearnedWord {
        word: String,
        activity: Activity,
    },
    BirdLanded,
    BirdSang,
    BirdFlewOff {
        startled: bool,
    },
    MothArrived,
    MothLeft,
    GiftLeft(Keepsake),
    Kept(Keepsake),
}

impl EventKind {
    /// The event in a few words, from the ghost's side.
    pub fn words(&self) -> String {
        match self {
            Self::Began(activity) => format!("started {}", activity.doing()),
            Self::Poked { count: 1 } => "the viewer poked you".to_owned(),
            Self::Poked { count } => format!("the viewer poked you {count} times in a row"),
            Self::AteMote => "caught and ate a light mote".to_owned(),
            Self::NudgedBall => "nudged the ball".to_owned(),
            Self::PickedUpBall => "picked up the ball".to_owned(),
            Self::Fetched => "brought the ball back to the viewer".to_owned(),
            Self::Pounced => "pounced on the viewer's finger on the glass".to_owned(),
            Self::Danced => "danced a little twirl".to_owned(),
            Self::Tended => "watered the plant".to_owned(),
            Self::Bloomed => "the plant came into flower".to_owned(),
            Self::SwitchedLamp { on: true } => "switched the lamp on".to_owned(),
            Self::SwitchedLamp { on: false } => "switched the lamp off".to_owned(),
            Self::FellAsleep => "fell asleep".to_owned(),
            Self::WokeUp => "woke up".to_owned(),
            Self::Morning => "woke up to a new morning".to_owned(),
            Self::Heard(text) => format!("the viewer said: {text}"),
            Self::Said(text) => format!("you said: {text}"),
            Self::ViewerArrived => "the viewer came to the glass".to_owned(),
            Self::ViewerLeft => "the viewer went away".to_owned(),
            Self::ExpectedArrival => {
                "the viewer came at their usual time, just as you hoped".to_owned()
            }
            Self::MissedVisit => "the viewer didn't come at their usual time".to_owned(),
            Self::DroppedMote => "the viewer dropped a light mote into the room".to_owned(),
            Self::KickedBall => "the viewer flicked the ball".to_owned(),
            Self::Returned { away } => {
                let hours = away / 3600.0;
                let while_away = match hours {
                    hours if hours < 2.0 => "an hour".to_owned(),
                    hours if hours < 24.0 => format!("{} hours", hours.round()),
                    hours if hours < 48.0 => "a day".to_owned(),
                    hours => format!("{} days", (hours / 24.0).round()),
                };
                format!("the viewer came back after {while_away} away")
            }
            Self::Noticed(thing) => format!("spotted {thing}"),
            Self::Milestone(text) => format!("a moment to remember: {text}"),
            Self::RainStarted => "it started to rain outside".to_owned(),
            Self::StormStarted => "a thunderstorm rolled in".to_owned(),
            Self::Thunder => "thunder rumbled outside".to_owned(),
            Self::RainStopped => "the rain stopped".to_owned(),
            Self::SnowStarted => "it started to snow outside".to_owned(),
            Self::SnowStopped => "the snow stopped".to_owned(),
            Self::RainbowAppeared => "the rain stopped and a rainbow came out".to_owned(),
            Self::RainbowFaded => "the rainbow faded".to_owned(),
            Self::Found => "the viewer found you hiding".to_owned(),
            Self::Jumped => "jumped out from hiding at the viewer".to_owned(),
            Self::GameOver(outcome) => match outcome {
                Outcome::GhostWon { .. } => "won a game of noughts and crosses against the viewer",
                Outcome::ViewerWon { .. } => "the viewer beat you at noughts and crosses",
                Outcome::Draw => "a game of noughts and crosses ended in a draw",
                Outcome::Abandoned => {
                    "the viewer wandered off in the middle of your game of noughts and crosses"
                }
            }
            .to_owned(),
            Self::Doodled(name) => format!("drew a {name} on the glass"),
            Self::Celebrated(Occasion::NewYear) => "a new year began".to_owned(),
            Self::Celebrated(occasion) => format!("it is {}", occasion.words()),
            Self::Praised => "the viewer sounded pleased with you".to_owned(),
            Self::Scolded => "the viewer told you no".to_owned(),
            Self::Petted => "the viewer stroked you gently".to_owned(),
            Self::Puzzled(word) => format!("didn't know the viewer's word “{word}”"),
            Self::LearnedWord { word, activity } => {
                format!("worked out that “{word}” means {}", activity.doing())
            }
            Self::BirdLanded => "a little bird landed outside the window".to_owned(),
            Self::BirdSang => "the bird outside sang".to_owned(),
            Self::BirdFlewOff { startled: false } => "the bird flew away".to_owned(),
            Self::BirdFlewOff { startled: true } => "the bird took fright and flew away".to_owned(),
            Self::MothArrived => "a moth fluttered in to the lamp".to_owned(),
            Self::MothLeft => "the moth fluttered away".to_owned(),
            Self::GiftLeft(gift) => format!("the bird left {} on the sill outside", gift.name()),
            Self::Kept(gift) => format!("put {} on the shelf of keepsakes", gift.name()),
        }
    }
}

/// What the ghost made of something the viewer said.
#[derive(Clone, Debug, PartialEq)]
pub struct Heard {
    pub tone: Option<Tone>,
    pub understanding: Understanding,
    /// Whether the message called the ghost by its name.
    pub named: bool,
    /// Words whose meaning it worked out just now, and what they mean.
    pub learned: Vec<(String, Activity)>,
    /// Whether being told no made it stop what it was doing.
    pub stopped: bool,
}

/// What the viewer's words asked for, as far as the ghost can tell.
#[derive(Clone, Debug, PartialEq)]
pub enum Understanding {
    /// Nothing to act on: only tone, or little words.
    Nothing,
    /// Words it doesn't know yet, `word` above all; `again` once it has
    /// heard one of them before.
    Puzzled { word: String, again: bool },
    /// It took `word` to ask for `activity` and set out to do it.
    Doing { word: String, activity: Activity },
    /// It took `word` to ask for `activity`, which it can't do now.
    Unable { word: String, activity: Activity },
    /// A question it can answer about itself; `doing` is what it was doing
    /// when asked, or `None` if it was asleep.
    Asked {
        question: Question,
        doing: Option<Activity>,
    },
}

/// The viewer's latest words that might mean something, and what the
/// ghost did about them, so it can learn from what follows.
#[derive(Clone, Debug, Default, PartialEq)]
struct Listening {
    words: Vec<String>,
    at: f64,
    /// What it did because of the words, when, and whether it was a guess.
    acted: Option<(Activity, f64, bool)>,
    /// The words of the very latest message, which may have none, for a
    /// mind working out what that message asked.
    latest: Vec<String>,
    /// Whether the viewer already showed it what they meant.
    shown: bool,
}

/// Something said or thought, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub time: f64,
}

/// The ghost: a point with a body that floats, needs that rise and fall, and
/// a plan it is carrying out.
#[derive(Clone, Debug)]
pub struct Ghost {
    pub name: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub needs: Needs,
    pub plan: Plan,
    /// From awake at 0 to sound asleep at 1.
    pub asleep: f32,
    /// From fully visible at 0 to gone at 1.
    pub fade: f32,
    /// What it is looking at, in the room.
    pub look: Option<Vec3>,
    pub mood: Mood,
    /// A mood a mind suggested, and until when it holds.
    pub suggested_mood: Option<(Mood, f64)>,
    /// What it means to do for a while, in its own words.
    pub goal: Option<String>,
    pub speech: Option<Line>,
    pub thought: Option<Line>,
    /// Which way it faces: what it sees lies around this direction.
    pub heading: Vec3,
    /// What it turned to look at, and until when.
    pub attention: Option<(Thing, f64)>,
    /// The motes it has seen, which it knows are there.
    pub known: Vec<u32>,
    /// Whether it has seen the current rainbow.
    pub saw_rainbow: bool,
    /// Whether it has seen the current snowfall.
    pub saw_snow: bool,
    /// Whether it has noticed the gift currently on the outer sill.
    pub saw_gift: bool,
    /// A gift riding just below it on the way to the shelf.
    pub keepsake: Option<Keepsake>,
}

/// The person at the glass, as the world knows them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewer {
    /// Where the pointer touches the glass, while it is over the room.
    pub pointer: Option<Vec3>,
    /// Recent travel on the glass, fading as the pointer holds still.
    pub liveliness: f32,
    /// When the pointer last moved on the glass.
    pub moved_at: f64,
    /// When the viewer last did anything.
    pub active_at: f64,
    /// While the viewer is typing a message.
    pub typing: bool,
}

/// The glass box and everything in it. `step` moves it forward in time;
/// minds only choose what the ghost does next, through `begin`.
#[derive(Clone, Debug)]
pub struct World {
    /// Seconds since the world started.
    pub time: f64,
    /// The local hour of the day, from 0 to 24, which sets the light.
    pub hour: f32,
    /// The local date, fixed for seeded pictures and set by the clock in live mode.
    pub date: jiff::civil::Date,
    /// The local date it was born, absent in seeded pictures unless requested.
    pub birthday: Option<jiff::civil::Date>,
    pub ghost: Ghost,
    pub lamp: Lamp,
    pub ball: Ball,
    pub plant: Plant,
    pub nook: Nook,
    pub motes: Vec<Mote>,
    pub doodles: Vec<Doodle>,
    pub game: Option<Game>,
    pub events: Vec<Event>,
    pub viewer: Viewer,
    pub weather: Weather,
    pub visitors: Visitors,
    /// Its life over days: its bond with the viewer, habits, and diary.
    pub life: Life,
    last_celebrated: Option<(jiff::civil::Date, Occasion)>,
    calendar_at: Option<jiff::civil::Date>,
    new_year_at: Option<f64>,
    random: Random,
    next_mote: u32,
    next_mote_at: f64,
    viewer_present: bool,
    /// When a darting pointer last interrupted an idle plan.
    pointer_interrupted_at: Option<f64>,
    listening: Listening,
    /// Which way the ghost means to push the ball next.
    push: Vec3,
    /// Until when an interrupted night's sleep leaves it drowsy.
    drowsy_until: f64,
    /// When it last woke naturally from an overnight rest.
    morning_at: Option<f64>,
    /// From untouched at 0 to being stroked steadily at 1.
    petting: f32,
    /// Whether this spell of petting has been counted.
    petted: bool,
    routine: routine::Routine,
}

impl World {
    pub fn new(seed: u64, hour: f32) -> Self {
        let start = Vec3::new(0.2, 1.1, 0.0);
        let mut random = Random::new(seed);
        let weather = Weather::new(&mut random);
        let nature = Nature::roll(&mut random);
        let visitors = Visitors::new(&mut random);
        Self {
            time: 0.0,
            hour,
            date: jiff::civil::Date::constant(2026, 6, 21),
            birthday: None,
            ghost: Ghost {
                name: "Boo".to_owned(),
                position: start,
                velocity: Vec3::ZERO,
                needs: Needs::default(),
                plan: Plan::new(Activity::Float, 0.0, start, 6.0),
                asleep: 0.0,
                fade: 0.0,
                look: None,
                mood: Mood::Content,
                suggested_mood: None,
                goal: None,
                speech: None,
                thought: None,
                heading: Vec3::Z,
                attention: None,
                known: Vec::new(),
                saw_rainbow: false,
                saw_snow: false,
                saw_gift: false,
                keepsake: None,
            },
            lamp: Lamp {
                position: Vec3::new(-1.55, 0.0, -0.95),
                on: true,
            },
            ball: Ball::at(0.7, 0.45),
            plant: Plant {
                position: Vec3::new(1.5, 0.0, -0.9),
                growth: 0.15,
                thirst: 0.4,
            },
            nook: Nook {
                position: Vec3::new(-1.3, 0.0, 0.55),
            },
            motes: Vec::new(),
            doodles: Vec::new(),
            game: None,
            events: Vec::new(),
            viewer: Viewer::default(),
            weather,
            visitors,
            life: Life {
                nature: Some(nature),
                ..Life::default()
            },
            last_celebrated: None,
            calendar_at: None,
            new_year_at: None,
            random,
            next_mote: 0,
            next_mote_at: 8.0,
            viewer_present: false,
            pointer_interrupted_at: None,
            listening: Listening::default(),
            push: Vec3::X,
            drowsy_until: 0.0,
            morning_at: None,
            petting: 0.0,
            petted: false,
            routine: routine::Routine::default(),
        }
    }

    /// Moves the world forward by `seconds`, at most a tenth of a second at
    /// a time.
    pub fn step(&mut self, seconds: f32) {
        self.advance_routine();
        self.life.begin_day(&self.date.to_string());
        self.celebrate();
        let mut remaining = seconds.max(0.0);
        while remaining > 0.0 {
            let slice = remaining.min(0.1);
            remaining -= slice;
            self.advance(slice);
        }
    }

    fn advance(&mut self, seconds: f32) {
        self.time += f64::from(seconds);
        self.viewer.liveliness *= (-seconds / 0.7).exp();
        self.petting *= (-seconds / PETTING_FADES).exp();
        if self.petting < 0.05 {
            self.petted = false;
        }
        self.doodles
            .retain(|doodle| doodle.opacity(self.time) > 0.0);

        if self
            .game
            .as_ref()
            .is_some_and(|game| game.opacity(self.time) <= 0.0)
        {
            self.game = None;
        }

        for mote in &mut self.motes {
            mote.drift(self.time, seconds);
        }
        let time = self.time;
        self.motes.retain(|mote| time - mote.born < Mote::LIFETIME);
        if self.time >= self.next_mote_at {
            if self.motes.len() < MAX_MOTES {
                let position = Vec3::new(
                    self.random.range(ROOM_MIN.x + 0.4, ROOM_MAX.x - 0.4),
                    ROOM_MAX.y - 0.35,
                    self.random.range(ROOM_MIN.z + 0.4, ROOM_MAX.z - 0.4),
                );
                self.add_mote(position);
            }
            self.next_mote_at = self.time + f64::from(self.random.range(25.0, 60.0));
        }

        let daytime = (7.0..18.5).contains(&self.hour);
        let season = self.season();
        if let Some(change) = self
            .weather
            .step(self.time, daytime, season, &mut self.random)
        {
            if change == Change::RainbowFaded {
                self.ghost.saw_rainbow = false;
            }
            if change == Change::SnowStopped {
                self.ghost.saw_snow = false;
            }
            self.record(match change {
                Change::RainStarted => EventKind::RainStarted,
                Change::StormStarted => EventKind::StormStarted,
                Change::Thunder => EventKind::Thunder,
                Change::RainStopped => EventKind::RainStopped,
                Change::SnowStarted => EventKind::SnowStarted,
                Change::SnowStopped => EventKind::SnowStopped,
                Change::RainbowAppeared => EventKind::RainbowAppeared,
                Change::RainbowFaded => EventKind::RainbowFaded,
            });
        }
        self.weather.advance(self.time, seconds, daytime);
        if self.weather.advance_storm(self.time, &mut self.random) == Some(Change::Thunder) {
            self.record(EventKind::Thunder);
            self.hear_thunder();
        }
        self.advance_visitors(seconds);

        let obstacles = self.obstacles();
        self.ball.roll(seconds, &obstacles);
        self.plant.thirst = (self.plant.thirst + seconds / Plant::THIRST_TIME).min(1.0);

        let present = self.viewer_present();
        if present != self.viewer_present {
            self.viewer_present = present;
            if present {
                self.viewer_arrived();
            } else {
                self.record(EventKind::ViewerLeft);
            }
        }
        if let Some(milestone) = self.life.pass(seconds, present) {
            self.record(EventKind::Milestone(milestone));
        }

        self.carry_out(seconds);
        let hiding = !self.ghost.plan.done
            && self.ghost.plan.activity == Activity::Hide
            && self.ghost.plan.arrived.is_some();
        let fade = if hiding { 0.85 } else { 0.0 };
        self.ghost.fade += (fade - self.ghost.fade) * (1.0 - (-seconds / 0.4).exp());
        self.move_ghost(seconds);
        if self.ball.held {
            self.ball.position = (self.ghost.position - Vec3::Y * 0.42).clamp(
                ROOM_MIN + Vec3::splat(Ball::RADIUS),
                ROOM_MAX - Vec3::splat(Ball::RADIUS),
            );
            self.ball.velocity = self.ghost.velocity;
        }
        self.update_needs(seconds);
        self.ghost.mood = self.feeling();
        self.look_around(seconds);

        if self.events.len() > EVENTS_KEPT {
            let excess = self.events.len() - EVENTS_KEPT;
            self.events.drain(..excess);
        }
    }

    /// Whether the viewer did anything recently.
    pub fn viewer_present(&self) -> bool {
        self.viewer.active_at > 0.0 && self.time - self.viewer.active_at < PRESENT_FOR
    }

    pub fn is_night(&self) -> bool {
        !(6.5..19.5).contains(&self.hour)
    }

    pub fn season(&self) -> Season {
        Season::of(self.date)
    }

    pub fn moon_phase(&self) -> f32 {
        calendar::moon_phase(self.date, self.hour)
    }

    /// Whether the ghost has finished its plan and waits for a decision.
    pub fn needs_decision(&self) -> bool {
        self.ghost.plan.done
    }

    /// The activities that make sense right now, in a fixed order.
    pub fn options(&self) -> Vec<Activity> {
        Activity::ALL
            .into_iter()
            .filter(|activity| match activity {
                Activity::Chase => self.nearest_mote().is_some(),
                Activity::Rest => self.ghost.needs.energy < 0.85 || self.bedtime(),
                Activity::Tend => self.plant.thirst > 0.25,
                Activity::Visit => self.viewer_present() || self.ghost.needs.company < 0.6,
                Activity::Wait => self.wait_is_open(),
                // The viewer threw the ball, or is there to bring it to.
                Activity::Fetch => {
                    self.ghost.asleep < 0.3
                        && self.ghost.needs.energy > 0.3
                        && (self.viewer_present()
                            || self
                                .events
                                .iter()
                                .rev()
                                .find(|event| event.kind == EventKind::KickedBall)
                                .is_some_and(|event| {
                                    self.time - event.time < 20.0
                                        || self.ball.velocity.length_squared() > 0.0
                                }))
                }
                Activity::Dance => self.ghost.asleep < 0.3 && self.ghost.needs.energy > 0.35,
                Activity::Hide => self.viewer_present() && self.ghost.needs.energy > 0.3,
                Activity::Doodle => self.ghost.asleep < 0.3 && self.ghost.needs.energy > 0.25,
                Activity::Pounce => self.pounce_is_open(),
                Activity::Game => {
                    self.viewer_present()
                        && self.ghost.asleep < 0.3
                        && self.ghost.needs.energy > 0.25
                }
                Activity::Float
                | Activity::Explore
                | Activity::Play
                | Activity::Lamp
                | Activity::Window => true,
            })
            .collect()
    }

    /// What doing `activity` would mean right now, written for a mind
    /// choosing among the options.
    pub fn describe(&self, activity: Activity) -> String {
        match activity {
            Activity::Float => {
                "Float gently in place and look around. Fits when nothing in particular calls."
                    .to_owned()
            }
            Activity::Explore => "Drift around the room to look at different corners. Fits when \
                                  curious or bored, with energy to spare."
                .to_owned(),
            Activity::Chase => "Chase a drifting light mote and eat it, which restores energy. \
                                Fits when tired, hungry for fun, or a mote is close."
                .to_owned(),
            Activity::Rest if self.bedtime() => {
                "Go to the cushion and sleep through the night until morning. Fits at bedtime, \
                 especially when the viewer is away."
                    .to_owned()
            }
            Activity::Rest => "Go to the cushion and sleep until rested. Fits when energy is low \
                               or it is late at night."
                .to_owned(),
            Activity::Play => "Push the ball around the floor. Fits when bored and with energy \
                               to spare."
                .to_owned(),
            Activity::Tend => {
                "Water and care for the potted plant. Fits when the plant is thirsty.".to_owned()
            }
            Activity::Lamp => {
                let (verb, fits) = if self.lamp.on {
                    ("off", "it is daytime or the ghost wants to sleep")
                } else {
                    ("on", "it is dark in the room")
                };
                format!("Go to the floor lamp and switch it {verb}. Fits when {fits}.")
            }
            Activity::Visit => "Go up to the glass to be near the viewer and look at them. Fits \
                                when the viewer is around, or when lonely."
                .to_owned(),
            Activity::Wait => "Go to the glass and wait for the viewer, who usually comes \
                               around this time. Fits when the viewer is expected but hasn't \
                               come yet."
                .to_owned(),
            Activity::Pounce => "Chase the viewer's darting finger along the inside of the \
                                 glass and pounce on it. Fits when playful and full of energy."
                .to_owned(),
            Activity::Hide => "Play hide-and-seek with the viewer: hide somewhere in the room \
                               and turn faint until they find you by clicking you. Fits when \
                               the viewer is around and the ghost feels playful."
                .to_owned(),
            Activity::Game => "Draw a board in the fog and play noughts and crosses with the \
                               viewer, who clicks a square to move. Fits when the viewer is \
                               around and the ghost feels playful or bored."
                .to_owned(),
            Activity::Doodle => "Breathe on the glass and draw a little picture in the fog for \
                                 the viewer to see. Fits when feeling fond of the viewer, or \
                                 bored and creative."
                .to_owned(),
            Activity::Window => {
                if let Some(gift) = self.life.sill {
                    return format!(
                        "Float to the window, where {} waits on the outer sill. \
                         Reach through the pane and carry it to the shelf when the bird lets you.",
                        gift.name()
                    );
                }
                let outside = match self.weather.sky {
                    Sky::Rainbow => ", where a rainbow is shining",
                    Sky::Rain if self.weather.storm => ", where a thunderstorm is rumbling",
                    Sky::Rain => ", where it is raining",
                    Sky::Snow => ", where snow is falling",
                    Sky::Clear if self.is_night() => " at the night sky",
                    Sky::Clear => " at the sky and the hills",
                };
                format!(
                    "Float to the window and look out{outside}. Fits when curious or calm, \
                     and above all when something is happening outside."
                )
            }
            Activity::Fetch => {
                "Go and get the ball and bring it to the glass for the viewer. Fits \
                                above all when the viewer just threw it, or when playful and the \
                                viewer is around."
                    .to_owned()
            }
            Activity::Dance => "Do a happy little twirl in the air. Fits when delighted, after \
                                something good happened, or to show off for the viewer."
                .to_owned(),
        }
    }

    /// An index picked with chances in proportion to `weights`, from the
    /// world's own random source so a seeded world plays out the same way.
    pub fn pick(&mut self, weights: &[f32]) -> Option<usize> {
        self.random.pick(weights)
    }

    /// A number from 0 to 1 from the world's own random source.
    pub fn random_unit(&mut self) -> f32 {
        self.random.unit()
    }

    /// Starts `activity`, dropping whatever plan was running.
    pub fn begin(&mut self, activity: Activity) {
        self.abandon_game();
        self.put_keepsake_away();
        self.drop_ball();
        let position = self.ghost.position;
        let length = match activity {
            Activity::Float => self.random.range(5.0, 11.0),
            Activity::Visit => self.random.range(8.0, 15.0),
            Activity::Wait => self.random.range(60.0, 120.0),
            Activity::Window => self.random.range(10.0, 18.0),
            Activity::Fetch => 6.0,
            Activity::Dance => self.random.range(5.0, 7.0),
            Activity::Hide => self.random.range(25.0, 45.0),
            Activity::Pounce => 25.0,
            _ => 0.0,
        };
        let target = match activity {
            Activity::Explore => self.random_spot(),
            Activity::Rest => self.nook.bed(),
            Activity::Tend => self.plant.top() + Vec3::new(0.0, 0.25, 0.3),
            Activity::Lamp => self.lamp.bulb() + Vec3::new(0.35, -0.15, 0.25),
            Activity::Visit => self.glass_spot(),
            Activity::Wait => Vec3::new(0.0, 1.2, ROOM_MAX.z - 0.45),
            Activity::Pounce => self.glass_spot(),
            Activity::Window => WINDOW_CENTER + Vec3::new(0.0, -0.05, 0.55),
            Activity::Fetch => self.ball.position + Vec3::Y * 0.42,
            Activity::Dance => position.clamp(
                ROOM_MIN + Vec3::new(0.6, 0.9, 0.6),
                Vec3::new(ROOM_MAX.x - 0.6, 1.6, ROOM_MAX.z - 0.6),
            ),
            Activity::Hide => self.hiding_spot(),
            Activity::Game => self.begin_game(),
            Activity::Doodle => Vec3::new(
                self.random.range(-1.2, 1.2),
                self.random.range(1.0, 1.6),
                ROOM_MAX.z - 0.35,
            ),
            Activity::Float | Activity::Chase | Activity::Play => position,
        };
        let mut plan = Plan::new(activity, self.time, target, length);
        plan.overnight = activity == Activity::Rest && self.bedtime();
        if activity == Activity::Chase {
            plan.mote = self.nearest_mote().map(|mote| mote.id);
        }
        if activity == Activity::Play {
            self.push = self.push_direction();
        }
        if activity == Activity::Doodle {
            plan.sketch = Some(self.choose_sketch());
        }
        if activity == Activity::Wait {
            self.begin_wait();
        }
        self.ghost.plan = plan;
        self.record(EventKind::Began(activity));
    }

    /// The viewer poked the ghost.
    pub fn poke(&mut self) {
        if self.ghost.plan.activity == Activity::Hide
            && !self.ghost.plan.done
            && self.ghost.fade > 0.3
        {
            self.ghost.needs.change(0.0, 0.2, 0.15);
            self.ghost.fade = 0.0;
            self.ghost.velocity.y += 0.7;
            self.ghost.plan.done = true;
            self.record(EventKind::Found);
            return;
        }
        let count = match self.events.iter().rev().find_map(|event| match event.kind {
            EventKind::Poked { count } => Some((event.time, count)),
            _ => None,
        }) {
            Some((time, count)) if self.time - time < POKE_STREAK => count + 1,
            _ => 1,
        };
        self.wake();
        let fun = if count < 3 { 0.04 } else { -0.05 };
        self.ghost.needs.change(0.0, fun, 0.05);
        self.ghost.velocity += Vec3::new(self.random.range(-0.4, 0.4), 0.5, -0.3);
        self.record(EventKind::Poked { count });
    }

    /// The viewer dropped a mote into the room at `position`. The ghost saw
    /// it happen, so it knows the mote is there and glances at it.
    pub fn drop_mote(&mut self, position: Vec3) {
        if self.motes.len() >= MAX_MOTES {
            self.motes.remove(0);
        }
        let id = self.add_mote(position);
        self.ghost.known.push(id);
        self.ghost.attention = Some((Thing::Mote(id), self.time + 2.0));
        self.record(EventKind::DroppedMote);
        self.shown(Activity::Chase);
    }

    /// The viewer flicked the ball along `direction` on the floor.
    pub fn kick_ball(&mut self, direction: Vec3) {
        self.drop_ball();
        let along = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
        self.ball.velocity += along * 2.2;
        let plan = &mut self.ghost.plan;
        if self.ghost.asleep < 0.3
            && (matches!(
                plan.activity,
                Activity::Float | Activity::Explore | Activity::Window | Activity::Visit
            ) || (plan.activity == Activity::Fetch && plan.count >= 2))
        {
            plan.done = true;
        }
        if self.ghost.plan.done {
            self.put_keepsake_away();
        }
        self.record(EventKind::KickedBall);
        self.shown(Activity::Fetch);
    }

    /// The viewer stroked the ghost, moving `travel` across it in scene
    /// units. Strokes soothe it without waking it; one that finds it hiding
    /// counts as finding it.
    pub fn pet(&mut self, travel: f32) {
        if self.ghost.plan.activity == Activity::Hide
            && !self.ghost.plan.done
            && self.ghost.fade > 0.3
        {
            self.poke();
            return;
        }
        self.petting = (self.petting + travel.max(0.0) * STROKING).min(1.0);
        if self.petting >= PETTED && !self.petted {
            self.petted = true;
            self.record(EventKind::Petted);
        }
    }

    /// How much the ghost is being petted, from 0 to 1.
    pub fn petting(&self) -> f32 {
        self.petting
    }

    /// A new ghost comes into being on `day`, a local date as YYYY-MM-DD.
    pub fn begin_life(&mut self, day: &str) {
        if self.life.nature.is_none() {
            self.life.nature = Some(Nature::roll(&mut self.random));
        }
        if let Some(milestone) = self.life.begin(day) {
            self.record(EventKind::Milestone(milestone));
        }
    }

    /// The viewer opened the app again after `away` seconds.
    pub fn record_return(&mut self, away: f32) {
        self.record(EventKind::Returned {
            away: f64::from(away),
        });
    }

    /// The viewer said `text` to the ghost, which wakes and reads it: their
    /// tone, and any words it has learned. It does what the words ask if it
    /// knows them and can, and otherwise comes to the glass to listen. What
    /// it was doing, and whether the viewer sounds pleased with what it did
    /// about their last words, teach it what words mean.
    pub fn hear(&mut self, text: &str) -> Heard {
        let mut reading = vocabulary::read(text, &self.ghost.name);
        // A question's words ask about the ghost rather than for something
        // to do, so it learns nothing from them, and "do you love me?" is
        // no praise.
        let question = question::ask(text);
        if question.is_some() {
            reading.words.clear();
            if reading.tone == Some(Tone::Praise) {
                reading.tone = None;
            }
        }
        let doing = (self.ghost.asleep <= 0.5).then_some(if self.ghost.plan.done {
            Activity::Float
        } else {
            self.ghost.plan.activity
        });
        // What the viewer saw it doing, before it turns to listen.
        let plan = &self.ghost.plan;
        let busy = if !plan.done && !matches!(plan.activity, Activity::Float | Activity::Visit) {
            Some(plan.activity)
        } else {
            self.events
                .iter()
                .rev()
                .take_while(|event| self.time - event.time < NAMED_FOR)
                .find_map(|event| match event.kind {
                    EventKind::Began(activity)
                        if !matches!(activity, Activity::Float | Activity::Visit) =>
                    {
                        Some(activity)
                    }
                    _ => None,
                })
        };
        self.wake();
        self.ghost.needs.change(0.0, 0.05, 0.25);
        self.record(EventKind::Heard(text.to_owned()));
        self.life.vocabulary.hear(&reading.words);

        let mut learned = Vec::new();
        let scolded = reading.tone == Some(Tone::Scolding);
        if let Some(activity) = busy
            && !scolded
        {
            learned.extend(self.learn(&reading.words, activity, NAMED));
        }
        let acted = self
            .listening
            .acted
            .filter(|(_, at, _)| self.time - at < HEEDED_FOR);
        let earlier = self.listening.words.clone();
        let mut stopped = false;
        match reading.tone {
            Some(Tone::Praise) => {
                if let Some((activity, _, guess)) = acted {
                    let amount = if guess { CONFIRMED } else { PRAISED };
                    learned.extend(self.learn(&earlier, activity, amount));
                }
                self.ghost.needs.change(0.0, 0.05, 0.05);
                self.record(EventKind::Praised);
            }
            Some(Tone::Scolding) => {
                if let Some((activity, _, _)) = acted {
                    self.learn(&earlier, activity, REFUSED);
                    self.listening.acted = None;
                }
                let plan = &mut self.ghost.plan;
                if !plan.done && !matches!(plan.activity, Activity::Float | Activity::Visit) {
                    plan.done = true;
                    stopped = true;
                    self.drop_ball();
                    self.abandon_game();
                }
                self.ghost.needs.change(0.0, -0.03, 0.0);
                self.record(EventKind::Scolded);
            }
            _ => {}
        }

        let vocabulary = &self.life.vocabulary;
        // Words said with a no, as in "no spin", ask for nothing.
        let understanding = if let Some(question) = question {
            Understanding::Asked { question, doing }
        } else if reading.words.is_empty() || scolded {
            Understanding::Nothing
        } else if let Some((word, activity)) = vocabulary.meaning(&reading.words) {
            if self.options().contains(&activity) {
                Understanding::Doing { word, activity }
            } else {
                Understanding::Unable { word, activity }
            }
        } else {
            // The word the viewer keeps saying is the one they mean to
            // teach.
            let word = reading
                .words
                .iter()
                .rev()
                .max_by_key(|word| vocabulary.heard(word))
                .cloned()
                .unwrap_or_default();
            let again = reading.words.iter().any(|word| vocabulary.heard(word) > 1);
            Understanding::Puzzled { word, again }
        };
        if !reading.words.is_empty() {
            self.listening = Listening {
                words: reading.words.clone(),
                at: self.time,
                acted: None,
                shown: false,
                latest: Vec::new(),
            };
        }
        self.listening.latest = reading.words.clone();
        match &understanding {
            Understanding::Doing { activity, .. } => {
                let activity = *activity;
                self.heed_now(activity);
            }
            understanding => {
                if let Understanding::Puzzled { word, .. } = understanding {
                    self.record(EventKind::Puzzled(word.clone()));
                }
                // A goodnight at bedtime sends it to bed.
                let goodnight = reading.tone == Some(Tone::Farewell)
                    && *understanding == Understanding::Nothing
                    && self.bedtime()
                    && self.options().contains(&Activity::Rest);
                if goodnight {
                    self.begin(Activity::Rest);
                } else if self.ghost.plan.activity != Activity::Visit {
                    self.begin(Activity::Visit);
                }
            }
        }
        Heard {
            tone: reading.tone,
            understanding,
            named: reading.named,
            learned,
            stopped,
        }
    }

    /// Without a mind to ask, the ghost tries out what the viewer's latest
    /// words might mean, right after hearing them: something it can do now,
    /// leaning toward what the words already hint at and away from what it
    /// was told no to. Praise for the guess teaches it the words.
    pub fn guess(&mut self) -> Option<Activity> {
        let listening = &self.listening;
        if listening.words.is_empty()
            || listening.acted.is_some()
            || self.time - listening.at > 1.0
            || self.ghost.asleep > 0.3
        {
            return None;
        }
        let words = listening.words.clone();
        let candidates: Vec<Activity> = self
            .options()
            .into_iter()
            .filter(|activity| !matches!(activity, Activity::Float | Activity::Visit))
            .collect();
        let weights: Vec<f32> = candidates
            .iter()
            .map(|activity| {
                let evidence = self.life.vocabulary.evidence(&words, *activity);
                if evidence < 0.0 { 0.1 } else { 1.0 + evidence }
            })
            .collect();
        let activity = candidates[self.random.pick(&weights)?];
        self.begin(activity);
        self.listening.acted = Some((activity, self.time, true));
        Some(activity)
    }

    /// A mind worked out that the viewer's latest words ask for
    /// `activity`. The ghost takes that as what the words mean, and does it
    /// if it can.
    pub fn heed(&mut self, activity: Activity) {
        let words = self.listening.latest.clone();
        if !words.is_empty() && self.time - self.listening.at < HEEDED_FOR {
            self.learn(&words, activity, ROUTED);
        }
        if self.options().contains(&activity) {
            let plan = &self.ghost.plan;
            if plan.done || plan.activity != activity {
                self.begin(activity);
            }
            // Praise or a no for it later is about these words, if it had
            // any of its own.
            if !words.is_empty() {
                self.listening.acted = Some((activity, self.time, false));
            }
        }
    }

    pub fn say(&mut self, text: &str) {
        self.ghost.speech = Some(Line {
            text: text.to_owned(),
            time: self.time,
        });
        self.record(EventKind::Said(text.to_owned()));
    }

    pub fn think(&mut self, text: &str) {
        self.ghost.thought = Some(Line {
            text: text.to_owned(),
            time: self.time,
        });
    }

    /// A mind's suggestion of how the ghost feels, which holds for `seconds`.
    pub fn suggest_mood(&mut self, mood: Mood, seconds: f32) {
        self.ghost.suggested_mood = Some((mood, self.time + f64::from(seconds)));
    }

    /// Records that the viewer did something.
    pub fn notice_viewer(&mut self) {
        self.viewer.active_at = self.time.max(f64::MIN_POSITIVE);
    }

    /// The nearest mote the ghost knows of.
    pub fn nearest_mote(&self) -> Option<&Mote> {
        let known = &self.ghost.known;
        self.motes
            .iter()
            .filter(|mote| known.contains(&mote.id))
            .min_by(|first, second| {
                let distance = |mote: &Mote| mote.position.distance_squared(self.ghost.position);
                distance(first).total_cmp(&distance(second))
            })
    }

    fn add_mote(&mut self, position: Vec3) -> u32 {
        self.next_mote += 1;
        let seed = self.random.unit();
        self.motes.push(Mote {
            id: self.next_mote,
            position: position.clamp(ROOM_MIN + Vec3::splat(0.25), ROOM_MAX - Vec3::splat(0.25)),
            born: self.time,
            seed,
        });
        self.next_mote
    }

    // Sets out to do what the viewer's words asked, unless it is already
    // doing it.
    fn heed_now(&mut self, activity: Activity) {
        let plan = &self.ghost.plan;
        if plan.done || plan.activity != activity {
            self.begin(activity);
        }
        self.listening.acted = Some((activity, self.time, false));
    }

    // Teaches the ghost that `words` mean `activity`, by `amount`, and
    // records each word it works out. Floating and coming to the glass are
    // what it does for any words, so no word means them.
    fn learn(
        &mut self,
        words: &[String],
        activity: Activity,
        amount: f32,
    ) -> Vec<(String, Activity)> {
        if matches!(activity, Activity::Float | Activity::Visit) {
            return Vec::new();
        }
        let learned = self.life.vocabulary.teach(words, activity, amount);
        learned
            .into_iter()
            .map(|word| {
                self.record(EventKind::LearnedWord {
                    word: word.clone(),
                    activity,
                });
                (word, activity)
            })
            .collect()
    }

    // The viewer did something soon after speaking, which may show what
    // their words were about. The sooner, the surer.
    fn shown(&mut self, activity: Activity) {
        let since = self.time - self.listening.at;
        if self.listening.words.is_empty() || self.listening.shown || since >= SHOWN_FOR {
            return;
        }
        self.listening.shown = true;
        let words = self.listening.words.clone();
        self.learn(&words, activity, SHOWN * (1.0 - (since / SHOWN_FOR) as f32));
    }

    fn record(&mut self, kind: EventKind) {
        let remembered = self.life.milestones.len();
        self.life.note(&kind);
        let milestones: Vec<String> = self.life.milestones[remembered..]
            .iter()
            .map(|milestone| milestone.text.clone())
            .collect();
        self.events.push(Event {
            time: self.time,
            kind,
        });
        for milestone in milestones {
            self.record(EventKind::Milestone(milestone));
        }
    }

    // Once it knows the viewer, the ghost leaves whatever it was idly doing
    // to come to the glass when they arrive.
    fn greet(&mut self) {
        let idle = matches!(
            self.ghost.plan.activity,
            Activity::Float | Activity::Explore | Activity::Window
        );
        if self.life.bond >= self.life.greeting_bond()
            && self.ghost.asleep < 0.3
            && (idle || self.ghost.plan.done)
        {
            self.begin(Activity::Visit);
        }
    }

    fn wake(&mut self) {
        if self.ghost.keepsake.is_some() {
            self.put_keepsake_away();
            self.ghost.plan.done = true;
        }
        if self.ball.held {
            self.drop_ball();
            self.ghost.plan.done = true;
        }
        if self.ghost.asleep > 0.3 {
            self.ghost.asleep = 0.0;
            if self.bedtime() {
                self.drowsy_until = self.time + 120.0;
            }
            self.record(EventKind::WokeUp);
        }
        if self.ghost.plan.activity == Activity::Rest {
            self.ghost.plan.done = true;
        }
    }

    fn hear_thunder(&mut self) {
        if self.ghost.asleep > 0.3 {
            if self.random.unit() >= 1.0 / 3.0 {
                return;
            }
            self.wake();
        }
        self.ghost.velocity += Vec3::new(self.random.range(-0.25, 0.25), 0.3, 0.12);
        if self.viewer_present() && self.life.bond >= GREETS_FROM {
            if !matches!(
                self.ghost.plan.activity,
                Activity::Rest | Activity::Tend | Activity::Fetch
            ) {
                self.begin(Activity::Visit);
            }
        } else if matches!(
            self.ghost.plan.activity,
            Activity::Float | Activity::Explore | Activity::Window
        ) {
            self.ghost.plan.done = true;
        }
    }

    // Lets go of a held ball, which falls to the floor from where it was.
    fn drop_ball(&mut self) {
        self.ball.held = false;
    }

    // Floor circles the ball bounces off and the ghost flies around, with
    // the height they reach.
    fn obstacles(&self) -> Vec<(Vec3, f32)> {
        let mut obstacles = vec![
            (self.lamp.position, Lamp::BASE_RADIUS),
            (self.plant.position, Plant::POT_RADIUS),
            (self.nook.position, Nook::RADIUS),
        ];
        if let Some(decoration) = self.decoration() {
            obstacles.push((Decoration::POSITION, decoration.radius()));
        }
        obstacles
    }

    fn random_spot(&mut self) -> Vec3 {
        Vec3::new(
            self.random.range(ROOM_MIN.x + 0.5, ROOM_MAX.x - 0.5),
            self.random.range(0.6, ROOM_MAX.y - 0.6),
            self.random.range(ROOM_MIN.z + 0.5, ROOM_MAX.z - 0.5),
        )
    }

    fn hiding_spot(&mut self) -> Vec3 {
        let spots = [
            self.plant.position - Vec3::Z * (Plant::POT_RADIUS + 0.35),
            self.lamp.position - Vec3::Z * (Lamp::BASE_RADIUS + 0.35),
            self.nook.position - Vec3::Z * (Nook::RADIUS + 0.35),
            Vec3::new(ROOM_MIN.x + 0.3, 0.0, ROOM_MIN.z + 0.3),
            Vec3::new(ROOM_MAX.x - 0.3, 0.0, ROOM_MIN.z + 0.3),
        ];
        let index = self.random.range(0.0, spots.len() as f32) as usize;
        (spots[index] + Vec3::Y * 0.55).clamp(
            ROOM_MIN + Vec3::new(0.3, 0.45, 0.3),
            ROOM_MAX - Vec3::new(0.3, 0.35, 0.3),
        )
    }

    fn choose_sketch(&mut self) -> Sketch {
        if self.occasion() == Some(Occasion::Valentine) {
            return Sketch::Heart;
        }
        let mut sketches = Vec::new();
        if self.viewer_present() && self.life.bond >= GREETS_FROM {
            sketches.push(Sketch::Heart);
        }
        match self.weather.sky {
            Sky::Rain => sketches.push(Sketch::Wave),
            Sky::Snow => sketches.push(Sketch::Snowflake),
            Sky::Rainbow => sketches.push(Sketch::Sun),
            Sky::Clear => {}
        }
        match self.ghost.mood {
            Mood::Cheerful | Mood::Excited => sketches.extend([Sketch::Star, Sketch::Flower]),
            Mood::Bored => sketches.push(Sketch::Spiral),
            _ => {}
        }
        if sketches.is_empty() {
            sketches.push(Sketch::Smile);
        }
        sketches[self.random.range(0.0, sketches.len() as f32) as usize]
    }

    // Just inside the glass, where the viewer's pointer is or in the middle.
    fn glass_spot(&self) -> Vec3 {
        let point = self
            .viewer
            .pointer
            .unwrap_or(Vec3::new(0.0, 1.2, ROOM_MAX.z));
        Vec3::new(
            point.x.clamp(ROOM_MIN.x + 0.5, ROOM_MAX.x - 0.5),
            point.y.clamp(0.7, ROOM_MAX.y - 0.6),
            ROOM_MAX.z - 0.45,
        )
    }

    // Toward open floor, away from the walls.
    fn push_direction(&mut self) -> Vec3 {
        let toward = Vec3::new(
            self.random.range(-1.2, 1.2),
            0.0,
            self.random.range(-0.8, 0.8),
        ) - Vec3::new(self.ball.position.x, 0.0, self.ball.position.z);
        toward.normalize_or(Vec3::X)
    }

    // Works the current plan forward: updates its target and does its
    // action on arrival.
    fn carry_out(&mut self, seconds: f32) {
        let time = self.time;
        let position = self.ghost.position;
        if self.ghost.asleep >= 0.3 && self.ghost.plan.activity == Activity::Game {
            self.ghost.plan.done = true;
        }
        let plan = &self.ghost.plan;
        // Only resting keeps the ghost asleep.
        if plan.done || plan.activity != Activity::Rest {
            self.ghost.asleep = (self.ghost.asleep - seconds / 1.2).max(0.0);
        }
        if plan.done {
            self.abandon_game();
            self.put_keepsake_away();
            self.drop_ball();
            return;
        }
        let arrived_now = position.distance(plan.target) < ARRIVED;
        if arrived_now && plan.arrived.is_none() {
            self.ghost.plan.arrived = Some(time);
        }
        // Waiting at the glass keeps its full length. Resting, hiding, and
        // games have their own waits after arrival, but give up on the way
        // there.
        let waiting = self.ghost.plan.activity == Activity::Wait
            || (matches!(
                self.ghost.plan.activity,
                Activity::Rest | Activity::Hide | Activity::Game
            ) && self.ghost.plan.arrived.is_some());
        if self.ghost.plan.elapsed(time) > GIVE_UP && !waiting {
            self.put_keepsake_away();
            self.drop_ball();
            self.ghost.plan.done = true;
            self.abandon_game();
            return;
        }

        let mut look = None;
        match self.ghost.plan.activity {
            Activity::Float => {
                let anchor = self.ghost.plan.target;
                let phase = time as f32;
                self.ghost.plan.arrived.get_or_insert(time);
                look = Some(anchor + Vec3::new((phase * 0.3).sin(), 0.0, 1.0));
                if self
                    .sight()
                    .in_view
                    .iter()
                    .any(|seen| seen.thing == Thing::Moth && seen.hidden_behind.is_none())
                {
                    look = self.position_of(Thing::Moth);
                }
                if self.ghost.plan.elapsed(time) > self.ghost.plan.length {
                    self.ghost.plan.done = true;
                }
            }
            Activity::Explore => {
                look = Some(self.ghost.plan.target);
                if arrived_now {
                    self.ghost.plan.count += 1;
                    if self.ghost.plan.count >= 3 {
                        self.ghost.plan.done = true;
                    } else {
                        self.ghost.plan.target = self.random_spot();
                    }
                }
            }
            Activity::Chase => {
                let chased = self
                    .ghost
                    .plan
                    .mote
                    .and_then(|id| self.motes.iter().position(|mote| mote.id == id));
                match chased {
                    None => self.ghost.plan.done = true,
                    Some(index) => {
                        let mote = self.motes[index].position;
                        self.ghost.plan.target = mote;
                        look = Some(mote);
                        if position.distance(mote) < 0.22 {
                            self.motes.remove(index);
                            self.ghost.needs.change(0.25, 0.1, 0.0);
                            self.record(EventKind::AteMote);
                            self.ghost.plan.done = true;
                        }
                    }
                }
            }
            Activity::Rest => {
                if self.ghost.plan.arrived.is_some() {
                    let before = self.ghost.asleep;
                    self.ghost.asleep = (before + seconds / 3.0).min(1.0);
                    if before < 0.9 && self.ghost.asleep >= 0.9 {
                        self.record(EventKind::FellAsleep);
                    }
                    let overnight = self.ghost.plan.overnight;
                    let rested = if overnight {
                        !self.bedtime()
                    } else {
                        self.ghost.needs.energy >= 0.98
                    };
                    if rested {
                        self.ghost.plan.done = true;
                        self.ghost.asleep = 0.0;
                        self.record(EventKind::WokeUp);
                        if overnight {
                            self.greet_morning();
                        }
                    }
                }
            }
            Activity::Play => {
                let ball = self.ball.position;
                look = Some(ball);
                let behind = ball - self.push * 0.4 + Vec3::Y * 0.3;
                self.ghost.plan.target = behind;
                if position.distance(behind) < 0.24 && self.ball.velocity.length() < 0.4 {
                    self.ball.velocity += self.push * 1.6;
                    self.ghost.needs.change(-0.02, 0.12, 0.0);
                    self.ghost.plan.count += 1;
                    self.record(EventKind::NudgedBall);
                    if self.ghost.plan.count >= 3 {
                        self.ghost.plan.done = true;
                    } else {
                        self.push = self.push_direction();
                    }
                }
            }
            Activity::Tend => {
                look = Some(self.plant.top());
                if self.ghost.plan.stayed(time) > 5.0 {
                    let was_blooming = self.plant.blooming();
                    self.plant.thirst = 0.0;
                    self.plant.growth = (self.plant.growth + 0.05).min(1.0);
                    self.ghost.needs.change(0.0, 0.08, 0.0);
                    self.record(EventKind::Tended);
                    if !was_blooming && self.plant.blooming() {
                        self.record(EventKind::Bloomed);
                    }
                    self.ghost.plan.done = true;
                }
            }
            Activity::Lamp => {
                look = Some(self.lamp.bulb());
                if self.ghost.plan.stayed(time) > 0.6 {
                    self.lamp.on = !self.lamp.on;
                    self.record(EventKind::SwitchedLamp { on: self.lamp.on });
                    self.ghost.plan.done = true;
                }
            }
            Activity::Window => {
                look = self.watch_window();
            }
            Activity::Visit => {
                if self.viewer.pointer.is_some() {
                    self.ghost.plan.target = self.glass_spot();
                }
                // Toward the viewer, on the other side of the glass.
                look = Some(self.ghost.plan.target + Vec3::Z * 3.0);
                if self.ghost.plan.stayed(time) > self.ghost.plan.length {
                    self.ghost.plan.done = true;
                }
            }
            Activity::Wait => {
                if arrived_now {
                    self.ghost.plan.count += 1;
                    let side = if self.ghost.plan.count % 2 == 1 {
                        0.35
                    } else {
                        -0.35
                    };
                    self.ghost.plan.target = self.ghost.plan.anchor + Vec3::X * side;
                }
                look = Some(if self.ghost.plan.elapsed(time) % 20.0 >= 17.0 {
                    WINDOW_CENTER
                } else {
                    self.ghost.plan.target + Vec3::Z * 3.0
                });
                if self.ghost.plan.elapsed(time) >= self.ghost.plan.length
                    || self.viewer_present()
                    || self.bedtime()
                {
                    self.ghost.plan.done = true;
                }
            }
            Activity::Pounce => look = self.pounce(seconds),
            Activity::Game => look = self.play_game(),
            Activity::Fetch => {
                if self.ghost.plan.count == 0 {
                    let target = self.ball.position + Vec3::Y * 0.42;
                    self.ghost.plan.target = target;
                    look = Some(self.ball.position);
                    if position.distance(target) < ARRIVED && self.ball.velocity.length() < 0.8 {
                        self.ball.held = true;
                        self.ghost.plan.count = 1;
                        self.ghost.plan.arrived = None;
                        self.record(EventKind::PickedUpBall);
                    }
                }
                if self.ghost.plan.count == 1 {
                    if self.ball.held {
                        let mut target = self.glass_spot();
                        target.y = 0.7;
                        self.ghost.plan.target = target;
                        look = Some(target + Vec3::Z * 3.0);
                        if position.distance(target) < ARRIVED {
                            self.drop_ball();
                            let middle = Vec3::new(0.0, Ball::RADIUS, ROOM_MAX.z - Ball::RADIUS);
                            self.ball.velocity =
                                (middle - self.ball.position).normalize_or_zero() * 0.3;
                            self.ghost.plan.count = 2;
                            self.ghost.plan.arrived = Some(time);
                            self.ghost.needs.change(0.0, 0.12, 0.08);
                            self.record(EventKind::Fetched);
                        }
                    } else {
                        self.ghost.plan.done = true;
                    }
                }
                if self.ghost.plan.count >= 2 {
                    look = Some(self.ghost.plan.target + Vec3::Z * 3.0);
                    if self.ghost.plan.stayed(time) > self.ghost.plan.length {
                        self.ghost.plan.done = true;
                    }
                }
            }
            Activity::Dance => {
                let progress = self.ghost.plan.elapsed(time) / self.ghost.plan.length;
                let offset = if progress < 0.8 {
                    let phase = progress / 0.8 * std::f32::consts::TAU * 2.0;
                    Vec3::new(phase.sin(), -phase.cos(), 0.0) * 0.3
                } else {
                    let bounce = ((progress - 0.8) / 0.2).min(1.0);
                    Vec3::Y * (-0.3 * (1.0 - bounce) + 0.15 * (bounce * std::f32::consts::PI).sin())
                };
                self.ghost.plan.target = self.ghost.plan.anchor + offset;
                look = Some(if self.viewer_present() {
                    self.glass_spot() + Vec3::Z * 3.0
                } else {
                    self.ghost.plan.target
                });
                if progress >= 1.0 {
                    self.ghost.plan.target = self.ghost.plan.anchor;
                    self.ghost.needs.change(0.0, 0.1, 0.0);
                    self.record(EventKind::Danced);
                    self.ghost.plan.done = true;
                }
            }
            Activity::Hide => {
                look = Some(self.glass_spot() + Vec3::Z);
                if self.ghost.plan.stayed(time) > self.ghost.plan.length {
                    self.ghost.plan.target = self.glass_spot();
                    self.ghost.velocity =
                        (self.ghost.plan.target - position).normalize_or_zero() * 1.6;
                    self.say(if self.date.month() == 10 && self.date.day() == 31 {
                        "BOO!"
                    } else {
                        "Boo!"
                    });
                    self.record(EventKind::Jumped);
                    self.ghost.plan.done = true;
                }
            }
            Activity::Doodle => {
                if let Some(started) = self.ghost.plan.arrived {
                    if self.ghost.plan.count == 0 {
                        if self.doodles.len() >= MAX_DOODLES {
                            self.doodles.remove(0);
                        }
                        self.doodles.push(Doodle {
                            sketch: self.ghost.plan.sketch.unwrap_or(Sketch::Smile),
                            center: Vec2::new(position.x, position.y),
                            size: 0.3,
                            started,
                        });
                        self.ghost.plan.count = 1;
                    }
                    if let Some(doodle) = self.doodles.last().copied() {
                        let progress = doodle.drawn(time);
                        let pen = doodle.center + doodle.sketch.point_at(progress) * doodle.size;
                        look = Some(Vec3::new(pen.x, pen.y, ROOM_MAX.z));
                        self.ghost.plan.target = if progress < 1.0 {
                            Vec3::new(pen.x, pen.y, ROOM_MAX.z - 0.3)
                        } else {
                            Vec3::new(doodle.center.x, doodle.center.y, ROOM_MAX.z - 0.7)
                        };
                        if time - started > Doodle::DRAW_TIME + 2.0 {
                            self.record(EventKind::Doodled(doodle.sketch.name()));
                            self.ghost.needs.change(0.0, 0.1, 0.0);
                            self.ghost.plan.done = true;
                        }
                    }
                } else {
                    look = Some(self.ghost.plan.target + Vec3::Z * 0.35);
                }
            }
        }
        self.ghost.look = look;
    }

    // Steers toward the plan's target, around obstacles and away from the
    // walls.
    fn move_ghost(&mut self, seconds: f32) {
        let decoration = self.decoration();
        let ghost = &mut self.ghost;
        let mut target = ghost.plan.target;
        if ghost.plan.activity == Activity::Float {
            let phase = self.time as f32;
            target += Vec3::new(
                (phase * 0.4).sin() * 0.18,
                (phase * 0.7).sin() * 0.06,
                (phase * 0.33).cos() * 0.12,
            );
        }
        let speed = match ghost.plan.activity {
            Activity::Chase => 1.3,
            Activity::Float => 0.35,
            Activity::Wait => 0.3,
            Activity::Rest => 0.5,
            Activity::Explore => 0.7,
            Activity::Dance => 1.5,
            Activity::Pounce => 1.6,
            _ => 0.8,
        } * (0.6 + 0.4 * ghost.needs.energy)
            // It slows down to enjoy being stroked.
            * (1.0 - 0.75 * self.petting);

        let offset = target - ghost.position;
        let distance = offset.length();
        let desired = if distance > 1e-4 {
            offset / distance * speed * (distance / 0.6).min(1.0)
        } else {
            Vec3::ZERO
        };
        let mut steer = (desired - ghost.velocity) * 2.5;

        // Walls push back within a margin, and obstacles push sideways
        // below their tops.
        // The hem hangs about 0.36 below the ghost's center.
        let reaching =
            ghost.plan.activity == Activity::Window && !ghost.plan.done && ghost.plan.count == 1;
        let mut low = ROOM_MIN + Vec3::new(0.3, 0.45, 0.3);
        if reaching {
            low.z = ROOM_MIN.z - 0.04;
        }
        let high = ROOM_MAX - Vec3::new(0.3, 0.35, 0.3);
        for axis in 0..3 {
            if ghost.position[axis] < low[axis] {
                steer[axis] += (low[axis] - ghost.position[axis]) * 8.0;
            } else if ghost.position[axis] > high[axis] {
                steer[axis] -= (ghost.position[axis] - high[axis]) * 8.0;
            }
        }
        let obstacles = [
            (
                self.lamp.position,
                Lamp::BASE_RADIUS + 0.05,
                Lamp::HEIGHT + 0.15,
            ),
            (
                self.plant.position,
                Plant::POT_RADIUS + 0.05,
                self.plant.height() + 0.1,
            ),
            (self.nook.position, Nook::RADIUS, Nook::HEIGHT + 0.2),
        ];
        let decoration = decoration.map(|decoration| {
            (
                Decoration::POSITION,
                decoration.radius() + 0.05,
                decoration.height() + 0.3,
            )
        });
        for (center, radius, top) in obstacles.into_iter().chain(decoration) {
            let offset = flat(ghost.position - center);
            // Hiding tucks the ghost closer behind furniture than its
            // usual steering allows, especially beside the back wall.
            let reach = if ghost.plan.activity == Activity::Hide {
                (radius + 0.3).min(flat(target - center).length())
            } else {
                radius + 0.3
            };
            let distance = offset.length();
            if ghost.position.y < top && distance < reach && distance > 1e-4 {
                let push = offset / distance * (reach - distance) * 6.0;
                steer += Vec3::new(push.x, 0.0, push.y);
            }
        }

        let steer = steer.clamp_length_max(3.0);
        ghost.velocity += steer * seconds;
        ghost.position += ghost.velocity * seconds;
        let mut minimum = ROOM_MIN + Vec3::new(0.12, 0.38, 0.12);
        if reaching {
            minimum.z = ROOM_MIN.z - 0.04;
        }
        ghost.position = ghost.position.clamp(minimum, ROOM_MAX - Vec3::splat(0.12));
    }

    fn update_needs(&mut self, seconds: f32) {
        let present = self.viewer_present();
        let night = if self.is_night() { 1.4 } else { 1.0 };
        let nature = self.life.nature.unwrap_or_default();
        let tiring = 0.85 + 0.3 * nature.sleepy;
        let ghost = &mut self.ghost;
        if ghost.asleep > 0.5 {
            ghost
                .needs
                .change(0.012 * seconds / tiring, 0.0, 0.03 * self.petting * seconds);
            return;
        }
        let effort = (ghost.velocity.length() / 1.3).min(1.0);
        let mut fun = -0.0016 * (0.8 + 0.4 * nature.playful);
        let mut company = -0.0012 * (1.15 - 0.3 * nature.shy);
        match ghost.plan.activity {
            Activity::Explore => fun += 0.004,
            Activity::Game if !ghost.plan.done => fun += 0.004,
            Activity::Visit if present => company += 0.01,
            Activity::Window if ghost.plan.arrived.is_some() => {
                fun += 0.004 + 0.02 * self.weather.rainbow(self.time)
            }
            _ => {}
        }
        if present {
            company += 0.004;
        }
        // Being stroked is company, and a little fun while awake.
        company += 0.03 * self.petting;
        fun += 0.01 * self.petting;
        ghost.needs.change(
            -(0.0012 + 0.0025 * effort) * night * tiring * seconds,
            fun * seconds,
            company * seconds,
        );
    }

    // How the ghost feels: a mind's suggestion while it holds, or else what
    // its state and the latest events say.
    fn feeling(&self) -> Mood {
        let ghost = &self.ghost;
        if ghost.asleep > 0.5 || ghost.needs.energy < 0.2 {
            return Mood::Sleepy;
        }
        // Strokes soothe even a grumpy ghost.
        if self.petting >= PETTED {
            return Mood::Cheerful;
        }
        if let Some((mood, until)) = ghost.suggested_mood
            && self.time < until
        {
            return mood;
        }
        let recent = |seconds: f64| {
            self.events
                .iter()
                .rev()
                .take_while(move |event| self.time - event.time < seconds)
        };
        if recent(10.0).any(|event| matches!(event.kind, EventKind::Poked { count } if count >= 3))
        {
            return Mood::Grumpy;
        }
        if let Some(outcome) = recent(8.0).find_map(|event| match event.kind {
            EventKind::GameOver(outcome) => Some(outcome),
            _ => None,
        }) {
            match outcome {
                Outcome::ViewerWon { .. } => {
                    return if self.good_sport() {
                        Mood::Cheerful
                    } else {
                        Mood::Grumpy
                    };
                }
                Outcome::Draw => return Mood::Curious,
                _ => {}
            }
        }
        if recent(6.0).any(|event| {
            matches!(
                event.kind,
                EventKind::AteMote
                    | EventKind::GameOver(Outcome::GhostWon { .. })
                    | EventKind::NudgedBall
                    | EventKind::Bloomed
                    | EventKind::Fetched
                    | EventKind::Found
                    | EventKind::Pounced
                    | EventKind::Milestone(_)
                    | EventKind::LearnedWord { .. }
                    | EventKind::Kept(_)
                    | EventKind::ExpectedArrival
            )
                || matches!(&event.kind, EventKind::Noticed(thing) if thing.starts_with("a gift on the sill"))
        }) {
            return Mood::Excited;
        }
        if recent(12.0).any(|event| event.kind == EventKind::Praised) {
            return Mood::Cheerful;
        }
        if recent(12.0).any(|event| event.kind == EventKind::MissedVisit) {
            return Mood::Lonely;
        }
        if self.weather.rainbow(self.time) > 0.3 && ghost.saw_rainbow {
            return Mood::Excited;
        }
        if self.drowsy() {
            return Mood::Sleepy;
        }
        if ghost.asleep <= 0.3 && recent(8.0).any(|event| event.kind == EventKind::Thunder) {
            return if self.viewer_present() {
                Mood::Curious
            } else {
                Mood::Lonely
            };
        }
        if self.occasion() == Some(Occasion::Birthday) {
            return Mood::Cheerful;
        }
        if self.weather.snow(self.time) > 0.3 && ghost.saw_snow {
            return Mood::Curious;
        }
        if ghost.needs.company < 0.25 {
            return Mood::Lonely;
        }
        if ghost.needs.fun < 0.25 {
            return Mood::Bored;
        }
        if ghost.needs.energy > 0.6 && ghost.needs.fun > 0.6 && ghost.needs.company > 0.6 {
            return Mood::Cheerful;
        }
        if ghost.plan.activity == Activity::Explore {
            return Mood::Curious;
        }
        Mood::Content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_natures_are_repeatable_and_a_begun_life_keeps_its_nature() {
        let mut world = World::new(7, 12.0);
        let nature = world.life.nature;
        assert!(nature.is_some());
        assert_eq!(nature, World::new(7, 21.0).life.nature);
        assert_ne!(nature, World::new(8, 12.0).life.nature);
        world.begin_life("2026-10-21");
        assert_eq!(world.life.nature, nature);
        let mut missing = world.clone();
        missing.life.nature = None;
        let mut same = missing.clone();
        missing.begin_life("2026-10-21");
        same.begin_life("2026-10-21");
        assert!(missing.life.nature.is_some());
        assert_eq!(missing.life.nature, same.life.nature);
        let nature = missing.life.nature;
        missing.begin_life("2026-10-22");
        assert_eq!(missing.life.nature, nature);
    }

    #[test]
    fn nature_tints_the_pace_of_needs_and_sleep() {
        let mut average = World::new(1, 12.0);
        average.life.nature = Some(Nature::default());
        let mut pronounced = average.clone();
        pronounced.life.nature = Some(Nature {
            playful: 1.0,
            shy: 1.0,
            sleepy: 1.0,
            ..Nature::default()
        });
        average.update_needs(10.0);
        pronounced.update_needs(10.0);
        assert!(pronounced.ghost.needs.fun < average.ghost.needs.fun);
        assert!(pronounced.ghost.needs.energy < average.ghost.needs.energy);
        assert!(pronounced.ghost.needs.company > average.ghost.needs.company);
        for world in [&mut average, &mut pronounced] {
            world.ghost.asleep = 1.0;
            world.ghost.needs.energy = 0.5;
            world.update_needs(10.0);
        }
        assert!(pronounced.ghost.needs.energy < average.ghost.needs.energy);
    }

    #[test]
    fn a_shy_ghost_takes_longer_to_greet_but_still_greets_a_close_friend() {
        for (shyness, greets) in [(0.0, true), (1.0, false)] {
            let mut world = World::new(1, 12.0);
            world.life.nature = Some(Nature {
                shy: shyness,
                ..Nature::default()
            });
            world.life.bond = GREETS_FROM;
            world.greet();
            assert_eq!(world.ghost.plan.activity == Activity::Visit, greets);
            world.life.bond = 0.9;
            world.greet();
            assert_eq!(world.ghost.plan.activity, Activity::Visit);
        }
    }

    fn before_thunder(seed: u64) -> World {
        let mut world = World::new(seed, 12.0);
        world.time = 40.0;
        world.weather.sky = Sky::Rain;
        world.weather.storm = true;
        world.weather.until = 300.0;
        world.weather.thunder_at = Some(40.05);
        world
    }

    #[test]
    fn thunder_draws_the_ghost_to_a_trusted_viewer_at_the_glass() {
        for bond in [GREETS_FROM, 0.9] {
            let mut world = before_thunder(1);
            world.notice_viewer();
            world.life.bond = bond;
            world.begin(Activity::Play);
            world.step(0.1);
            assert_eq!(world.ghost.plan.activity, Activity::Visit);
            assert_eq!(world.ghost.plan.target, world.glass_spot());
            assert!(
                world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::Thunder)
            );
            assert_eq!(world.ghost.mood, Mood::Curious);
        }
    }

    #[test]
    fn thunder_ends_idle_plans_without_sending_a_stranger_or_an_absent_friend_to_the_glass() {
        for (bond, present) in [(0.0, true), (0.9, false)] {
            for activity in [
                Activity::Float,
                Activity::Explore,
                Activity::Window,
                Activity::Play,
            ] {
                let mut world = before_thunder(1);
                if present {
                    world.notice_viewer();
                }
                world.life.bond = bond;
                world.begin(activity);
                let velocity = world.ghost.velocity;
                world.step(0.1);
                assert_eq!(world.ghost.plan.activity, activity);
                assert_eq!(world.ghost.plan.done, activity != Activity::Play);
                assert!(world.ghost.velocity.y > velocity.y);
                assert_eq!(
                    world.ghost.mood,
                    if present { Mood::Curious } else { Mood::Lonely }
                );
            }
        }
    }

    #[test]
    fn thunder_leaves_resting_tending_and_fetching_alone() {
        for activity in [Activity::Rest, Activity::Tend, Activity::Fetch] {
            let mut world = before_thunder(1);
            world.notice_viewer();
            world.life.bond = 0.9;
            world.begin(activity);
            world.step(0.1);
            assert_eq!(world.ghost.plan.activity, activity);
            assert!(!world.ghost.plan.done);
        }
    }

    #[test]
    fn thunder_sometimes_wakes_a_sleeping_ghost_and_the_seed_decides() {
        let mut awakened = 0;
        for seed in 0..150 {
            let mut world = before_thunder(seed);
            world.begin(Activity::Rest);
            world.ghost.position = world.nook.bed();
            world.ghost.plan.arrived = Some(world.time);
            world.ghost.asleep = 1.0;
            world.ghost.needs.energy = 0.5;
            let mut same = world.clone();
            world.step(0.1);
            same.step(0.1);
            assert_eq!(world.events, same.events);
            assert_eq!(world.ghost.asleep, same.ghost.asleep);
            assert_eq!(world.ghost.velocity, same.ghost.velocity);
            let woke = world
                .events
                .iter()
                .any(|event| event.kind == EventKind::WokeUp);
            assert_eq!(woke, world.ghost.asleep == 0.0);
            if woke {
                assert!(world.ghost.plan.done);
                awakened += 1;
            } else {
                assert_eq!(world.ghost.asleep, 1.0);
            }
        }
        assert!((30..70).contains(&awakened), "{awakened}");
    }

    #[test]
    fn thunder_feelings_fade_and_yield_to_sleep_annoyance_and_delight() {
        let mut world = before_thunder(1);
        world.record(EventKind::Thunder);
        assert_eq!(world.feeling(), Mood::Lonely);
        world.notice_viewer();
        assert_eq!(world.feeling(), Mood::Curious);
        world.time += 8.0;
        assert_ne!(world.feeling(), Mood::Curious);
        world.record(EventKind::Thunder);
        world.record(EventKind::AteMote);
        assert_eq!(world.feeling(), Mood::Excited);
        world.record(EventKind::Poked { count: 3 });
        assert_eq!(world.feeling(), Mood::Grumpy);
        world.ghost.needs.energy = 0.1;
        assert_eq!(world.feeling(), Mood::Sleepy);
    }

    #[test]
    fn storm_starts_are_recorded_instead_of_plain_rain_and_stop_at_winter() {
        let mut storms = 0;
        for seed in 0..100 {
            let mut world = World::new(seed, 12.0);
            world.weather.until = 0.05;
            world.step(0.1);
            if world.weather.storm {
                storms += 1;
                assert_eq!(world.life.today.storms, 1);
                assert_eq!(world.life.total.storms, 1);
                assert_eq!(world.life.today.showers, 0);
                assert!(
                    world
                        .events
                        .iter()
                        .any(|event| event.kind == EventKind::StormStarted)
                );
                assert!(
                    !world
                        .events
                        .iter()
                        .any(|event| event.kind == EventKind::RainStarted)
                );
                world.date = "2026-12-21".parse().unwrap();
                world.step(0.1);
                assert_eq!(world.weather.sky, Sky::Snow);
                assert!(!world.weather.storm);
                assert_eq!(world.weather.flash(world.time), 0.0);
            }
        }
        assert!(storms > 10);
    }

    #[test]
    fn each_lightning_strike_reaches_the_world_as_one_delayed_thunder_event() {
        for seed in 0..10 {
            let mut world = World::new(seed, 12.0);
            world.weather.sky = Sky::Rain;
            world.weather.storm = true;
            world.weather.until = 200.0;
            let mut strikes = Vec::new();
            while world.time < 205.0 {
                let last_flash = world.weather.last_flash;
                world.step(0.1);
                if world.weather.last_flash.is_finite() && world.weather.last_flash != last_flash {
                    assert!(world.weather.rain(world.time) > 0.6);
                    strikes.push((world.weather.last_flash, world.weather.thunder_at.unwrap()));
                }
            }
            let thunder: Vec<_> = world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Thunder)
                .collect();
            assert!(strikes.len() > 4);
            assert_eq!(thunder.len(), strikes.len());
            for (event, (strike, due)) in thunder.into_iter().zip(strikes) {
                assert!((1.0..=4.0).contains(&(due - strike)));
                assert!((due..=due + 0.1).contains(&event.time));
            }
            assert!(!world.weather.storm);
            assert_eq!(world.weather.flash(world.time), 0.0);
        }
    }

    fn run(world: &mut World, seconds: f32) {
        for _ in 0..(seconds * 20.0) as usize {
            world.step(0.05);
        }
    }

    fn inside(point: Vec3) -> bool {
        point.cmpge(ROOM_MIN).all() && point.cmple(ROOM_MAX).all()
    }

    #[test]
    fn the_ghost_stays_in_the_room_whatever_it_does() {
        for activity in Activity::ALL {
            let mut world = World::new(5, 12.0);
            world.drop_mote(Vec3::new(1.0, 1.0, 0.0));
            world.notice_viewer();
            world.ghost.needs.energy = 0.5;
            world.begin(activity);
            for _ in 0..1400 {
                world.step(0.05);
                assert!(
                    inside(world.ghost.position),
                    "{activity:?}: {:?}",
                    world.ghost.position
                );
                assert!(inside(world.ball.position));
            }
        }
        let mut world = World::new(5, 12.0);
        for round in 0..200 {
            if world.needs_decision() {
                let options = world.options();
                world.begin(options[round % options.len()]);
            }
            run(&mut world, 1.5);
            assert!(inside(world.ghost.position), "{:?}", world.ghost.position);
            assert!(inside(world.ball.position));
        }
    }

    #[test]
    fn every_activity_finishes_on_its_own() {
        for activity in Activity::ALL {
            let mut world = World::new(9, 12.0);
            world.drop_mote(Vec3::new(1.0, 1.0, 0.0));
            world.ghost.needs.energy = 0.5;
            world.begin(activity);
            let duration = world.ghost.plan.length.max(60.0) + 10.0;
            run(&mut world, duration);
            assert!(world.needs_decision(), "{activity:?} never finished");
        }
    }

    #[test]
    fn chasing_a_mote_eats_it_for_energy() {
        let mut world = World::new(2, 12.0);
        world.drop_mote(Vec3::new(-1.0, 1.4, -0.5));
        let dropped = world.motes[0].id;
        world.ghost.needs.energy = 0.4;
        world.begin(Activity::Chase);
        run(&mut world, 6.0);
        assert!(world.motes.iter().all(|mote| mote.id != dropped));
        assert!(world.ghost.needs.energy > 0.55);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::AteMote)
        );
    }

    #[test]
    fn resting_puts_the_ghost_to_sleep_until_rested() {
        let mut world = World::new(4, 14.0);
        world.ghost.needs.energy = 0.3;
        world.begin(Activity::Rest);
        run(&mut world, 20.0);
        assert!(world.ghost.asleep > 0.9);
        assert_eq!(world.ghost.mood, Mood::Sleepy);
        run(&mut world, 70.0);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::WokeUp)
        );
        assert!(world.ghost.needs.energy > 0.9);
        assert!(world.needs_decision());
        assert_eq!(world.ghost.asleep, 0.0);
    }

    #[test]
    fn a_rest_that_never_reaches_the_cushion_is_given_up() {
        let mut world = World::new(4, 23.0);
        world.ghost.needs.energy = 0.1;
        world.begin(Activity::Rest);
        // Somewhere the ghost can never get to.
        world.ghost.plan.target = Vec3::new(0.0, -5.0, 0.0);
        run(&mut world, 65.0);
        assert!(world.needs_decision());
    }

    #[test]
    fn a_poke_wakes_a_sleeping_ghost_and_poking_on_makes_it_grumpy() {
        let mut world = World::new(4, 23.0);
        world.ghost.needs.energy = 0.3;
        world.begin(Activity::Rest);
        run(&mut world, 20.0);
        world.poke();
        assert_eq!(world.ghost.asleep, 0.0);
        assert!(world.needs_decision());
        world.poke();
        world.poke();
        assert!(matches!(
            world.events.last().unwrap().kind,
            EventKind::Poked { count: 3 }
        ));
        world.step(0.05);
        assert_eq!(world.ghost.mood, Mood::Grumpy);
    }

    #[test]
    fn strokes_soothe_and_please_the_ghost_and_count_once_per_spell() {
        let mut world = World::new(3, 12.0);
        world.ghost.needs.company = 0.3;
        for _ in 0..3 {
            world.poke();
        }
        assert_eq!(world.feeling(), Mood::Grumpy);
        world.pet(0.1);
        assert!(world.petting() < PETTED);
        world.pet(0.1);
        world.pet(0.1);
        assert_eq!(world.feeling(), Mood::Cheerful);
        let company = world.ghost.needs.company;
        run(&mut world, 1.0);
        world.pet(0.1);
        assert!(world.ghost.needs.company > company);
        let pets = |world: &World| {
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Petted)
                .count()
        };
        assert_eq!(pets(&world), 1);
        assert_eq!(world.life.total.pets, 1);
        assert!(world.events.iter().any(|event| {
            event.kind == EventKind::Milestone("You stroked me for the first time.".to_owned())
        }));

        // The petting fades once the strokes stop, and a new spell counts
        // again.
        run(&mut world, 6.0);
        assert!(world.petting() < 0.05);
        world.pet(0.4);
        assert_eq!(pets(&world), 2);
    }

    #[test]
    fn strokes_leave_a_sleeper_asleep_and_find_a_hidden_ghost() {
        let mut world = World::new(3, 12.0);
        world.ghost.needs.energy = 0.2;
        world.begin(Activity::Rest);
        run(&mut world, 30.0);
        assert!(world.ghost.asleep > 0.9);
        world.pet(0.5);
        run(&mut world, 0.5);
        assert!(world.ghost.asleep > 0.9);
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert!(!world.ghost.plan.done);

        let mut world = World::new(3, 12.0);
        world.notice_viewer();
        world.begin(Activity::Hide);
        world.ghost.plan.arrived = Some(world.time);
        world.ghost.fade = 0.85;
        world.pet(0.2);
        assert!(world.ghost.plan.done);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Found)
        );
        assert_eq!(world.petting(), 0.0);
    }

    #[test]
    fn playing_sends_the_ball_rolling() {
        let mut world = World::new(6, 12.0);
        let start = world.ball.position;
        world.begin(Activity::Play);
        run(&mut world, 30.0);
        assert!(world.ball.position.distance(start) > 0.3);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::NudgedBall)
        );
    }

    #[test]
    fn a_throw_ends_a_quiet_plan_and_opens_fetch() {
        for activity in [
            Activity::Float,
            Activity::Explore,
            Activity::Window,
            Activity::Visit,
        ] {
            let mut world = World::new(6, 12.0);
            assert!(!world.options().contains(&Activity::Fetch));
            world.begin(activity);
            world.kick_ball(-Vec3::Z);
            assert!(world.needs_decision(), "{activity:?}");
            assert!(world.options().contains(&Activity::Fetch));
        }
    }

    #[test]
    fn a_throw_leaves_busy_or_sleeping_ghosts_to_their_plans() {
        for activity in [
            Activity::Rest,
            Activity::Tend,
            Activity::Lamp,
            Activity::Chase,
            Activity::Play,
            Activity::Dance,
            Activity::Fetch,
        ] {
            let mut world = World::new(6, 12.0);
            world.begin(activity);
            world.kick_ball(Vec3::X);
            assert!(!world.needs_decision(), "{activity:?}");
        }
        let mut world = World::new(6, 12.0);
        world.ghost.asleep = 0.8;
        world.kick_ball(Vec3::X);
        assert!(!world.needs_decision());
        assert!(!world.options().contains(&Activity::Fetch));
    }

    #[test]
    fn a_viewer_can_ask_for_fetch_without_throwing_first() {
        let mut world = World::new(6, 12.0);
        assert!(!world.options().contains(&Activity::Fetch));
        world.notice_viewer();
        assert!(world.options().contains(&Activity::Fetch));
        world.ghost.needs.energy = 0.2;
        assert!(!world.options().contains(&Activity::Fetch));
    }

    #[test]
    fn fetch_needs_energy_and_a_recent_throw_or_a_ball_still_rolling_after_one() {
        let mut world = World::new(6, 12.0);
        world.ball.velocity = Vec3::X;
        assert!(!world.options().contains(&Activity::Fetch));
        world.kick_ball(Vec3::X);
        world.ghost.needs.energy = 0.3;
        assert!(!world.options().contains(&Activity::Fetch));
        world.ghost.needs.energy = 0.8;
        world.time += 21.0;
        assert!(world.options().contains(&Activity::Fetch));
        world.ball.velocity = Vec3::ZERO;
        assert!(!world.options().contains(&Activity::Fetch));
    }

    #[test]
    fn fetching_picks_up_the_ball_and_returns_it_before_waiting_for_another_throw() {
        for direction in [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z] {
            let mut world = World::new(6, 12.0);
            world.notice_viewer();
            world.kick_ball(direction);
            world.begin(Activity::Fetch);
            let mut carried = false;
            let fun = world.ghost.needs.fun;
            let company = world.ghost.needs.company;
            for _ in 0..1200 {
                world.step(0.05);
                assert!(
                    world
                        .ball
                        .position
                        .cmpge(ROOM_MIN + Vec3::splat(Ball::RADIUS))
                        .all()
                );
                assert!(
                    world
                        .ball
                        .position
                        .cmple(ROOM_MAX - Vec3::splat(Ball::RADIUS))
                        .all()
                );
                if world.ball.held {
                    carried = true;
                    let height = (world.ghost.position.y - 0.42).max(Ball::RADIUS);
                    assert!((world.ball.position.y - height).abs() < 1e-5);
                    assert_eq!(world.ball.velocity, world.ghost.velocity);
                }
                if world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::Fetched)
                {
                    break;
                }
            }
            assert!(carried, "{direction:?}");
            let actions: Vec<_> = world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::PickedUpBall | EventKind::Fetched))
                .map(|event| event.kind.clone())
                .collect();
            assert_eq!(
                actions,
                [EventKind::PickedUpBall, EventKind::Fetched],
                "{direction:?}"
            );
            assert!(!world.ball.held);
            assert!(world.ball.position.z > ROOM_MAX.z - 0.75);
            assert!(world.ghost.needs.fun > fun);
            assert!(world.ghost.needs.company > company);
            assert_eq!(world.ghost.mood, Mood::Excited);
            assert!(!world.needs_decision());
            assert!(world.ghost.look.unwrap().z > ROOM_MAX.z);
            run(&mut world, 5.9);
            assert!(!world.needs_decision());
            run(&mut world, 0.2);
            assert!(world.needs_decision());
        }
    }

    #[test]
    fn another_throw_ends_the_wait_after_a_fetch() {
        let mut world = World::new(6, 12.0);
        world.begin(Activity::Fetch);
        for _ in 0..600 {
            world.step(0.05);
            if world.ghost.plan.count == 2 {
                break;
            }
        }
        assert_eq!(world.ghost.plan.count, 2);
        assert!(!world.needs_decision());
        world.kick_ball(-Vec3::Z);
        assert!(world.needs_decision());
        assert!(world.options().contains(&Activity::Fetch));
    }

    #[test]
    fn changing_activity_or_waking_or_poking_lets_go_of_a_held_ball() {
        for interruption in 0..3 {
            let mut world = World::new(6, 12.0);
            world.begin(Activity::Fetch);
            for _ in 0..600 {
                world.step(0.05);
                if world.ball.held {
                    break;
                }
            }
            assert!(world.ball.held);
            let velocity = world.ball.velocity;
            match interruption {
                0 => world.begin(Activity::Explore),
                1 => world.wake(),
                _ => world.poke(),
            }
            assert!(!world.ball.held);
            assert_eq!(world.ball.velocity, velocity);
            run(&mut world, 1.0);
            assert!(!world.ball.held);
            assert_eq!(world.ball.position.y, Ball::RADIUS);
        }
    }

    #[test]
    fn giving_up_on_a_fetch_releases_the_ball() {
        let mut world = World::new(6, 12.0);
        world.begin(Activity::Fetch);
        for _ in 0..600 {
            world.step(0.05);
            if world.ball.held {
                break;
            }
        }
        assert!(world.ball.held);
        world.ghost.plan.started = world.time - f64::from(GIVE_UP);
        world.step(0.1);
        assert!(world.needs_decision());
        assert!(!world.ball.held);
        run(&mut world, 1.0);
        assert_eq!(world.ball.position.y, Ball::RADIUS);
        assert!(
            !world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Fetched)
        );
    }

    #[test]
    fn a_dance_moves_through_the_air_and_finishes_with_a_twirl() {
        for position in [Vec3::new(0.0, 1.2, 0.0), ROOM_MIN, ROOM_MAX] {
            let mut world = World::new(6, 12.0);
            world.ghost.position = position;
            world.begin(Activity::Dance);
            let anchor = world.ghost.plan.anchor;
            assert!(anchor.x >= ROOM_MIN.x + 0.6 && anchor.x <= ROOM_MAX.x - 0.6);
            assert!(anchor.z >= ROOM_MIN.z + 0.6 && anchor.z <= ROOM_MAX.z - 0.6);
            assert!((0.9..=1.6).contains(&anchor.y));
            assert!((5.0..7.0).contains(&world.ghost.plan.length));
            let mut lowest = f32::INFINITY;
            let mut highest = f32::NEG_INFINITY;
            let fun = world.ghost.needs.fun;
            for _ in 0..160 {
                world.step(0.05);
                assert!(inside(world.ghost.position));
                lowest = lowest.min(world.ghost.position.y);
                highest = highest.max(world.ghost.position.y);
                if world.needs_decision() {
                    break;
                }
            }
            assert!(highest - lowest > 0.25, "{lowest}..{highest}");
            assert!(world.needs_decision());
            assert!(world.time < 8.0);
            assert!(world.ghost.needs.fun > fun);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| event.kind == EventKind::Danced)
                    .count(),
                1
            );
            run(&mut world, 2.0);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| event.kind == EventKind::Danced)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn dancing_needs_energy_and_turns_toward_a_present_viewer() {
        let mut world = World::new(6, 12.0);
        world.ghost.needs.energy = 0.35;
        assert!(!world.options().contains(&Activity::Dance));
        world.ghost.needs.energy = 0.8;
        assert!(world.options().contains(&Activity::Dance));
        world.ghost.asleep = 0.8;
        assert!(!world.options().contains(&Activity::Dance));
        world.ghost.asleep = 0.0;
        world.notice_viewer();
        world.begin(Activity::Dance);
        world.step(0.1);
        assert!(world.ghost.look.unwrap().z > ROOM_MAX.z);
        world.time += PRESENT_FOR;
        world.ghost.plan.started = world.time;
        world.step(0.1);
        assert_eq!(world.ghost.look, Some(world.ghost.plan.target));
    }

    #[test]
    fn tending_waters_the_plant_and_helps_it_grow() {
        let mut world = World::new(6, 12.0);
        world.plant.thirst = 0.8;
        world.plant.growth = 0.78;
        world.begin(Activity::Tend);
        run(&mut world, 30.0);
        assert!(world.plant.thirst < 0.1);
        assert!(world.plant.blooming());
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Bloomed)
        );
    }

    #[test]
    fn hearing_the_viewer_brings_the_ghost_to_the_glass() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("hello");
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        run(&mut world, 8.0);
        assert!(world.ghost.position.z > ROOM_MAX.z - 0.8);
    }

    fn kinds(world: &World, since: f64) -> Vec<EventKind> {
        world
            .events
            .iter()
            .filter(|event| event.time >= since)
            .map(|event| event.kind.clone())
            .collect()
    }

    #[test]
    fn a_question_brings_the_ghost_to_the_glass_without_teaching_or_praising() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        world.begin(Activity::Play);
        let heard = world.hear("do you love me? what are you doing?");
        assert_eq!(
            heard.understanding,
            Understanding::Asked {
                question: Question::Liking,
                doing: Some(Activity::Play),
            }
        );
        assert_eq!(heard.tone, None);
        assert!(heard.learned.is_empty());
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(world.life.vocabulary.words.is_empty());
        assert!(
            !world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::Praised | EventKind::Puzzled(_)))
        );
    }

    #[test]
    fn a_new_word_puzzles_the_ghost_and_brings_it_to_the_glass() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        let heard = world.hear("Boo, dance!");
        assert_eq!(
            heard.understanding,
            Understanding::Puzzled {
                word: "dance".to_owned(),
                again: false
            }
        );
        assert!(heard.named);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(kinds(&world, 0.0).contains(&EventKind::Puzzled("dance".to_owned())));
        // Heard again, the word is one it means to work out.
        run(&mut world, 2.0);
        let heard = world.hear("dance");
        assert!(matches!(
            heard.understanding,
            Understanding::Puzzled { again: true, .. }
        ));
    }

    #[test]
    fn a_guess_praised_teaches_the_word_and_then_the_word_is_enough() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("twirl");
        world.hear("twirl");
        // Guess until it happens to dance, saying no to everything else.
        let mut tries = 0;
        loop {
            tries += 1;
            assert!(tries < 40, "it never tried dancing");
            let guess = world.guess().expect("a fresh, repeated word gets a guess");
            assert!(!matches!(guess, Activity::Float | Activity::Visit));
            assert_eq!(world.ghost.plan.activity, guess);
            run(&mut world, 1.0);
            if guess == Activity::Dance {
                break;
            }
            let heard = world.hear("no");
            assert_eq!(heard.tone, Some(Tone::Scolding));
            assert!(world.life.vocabulary.evidence(&["twirl".to_owned()], guess) < 0.0);
            run(&mut world, 1.0);
            world.hear("twirl");
        }
        let before = world.time;
        let heard = world.hear("yes! good job");
        assert_eq!(heard.tone, Some(Tone::Praise));
        assert_eq!(heard.learned, [("twirl".to_owned(), Activity::Dance)]);
        let events = kinds(&world, before);
        assert!(events.contains(&EventKind::Praised));
        assert!(events.contains(&EventKind::LearnedWord {
            word: "twirl".to_owned(),
            activity: Activity::Dance
        }));
        assert!(events.contains(&EventKind::Milestone(
            "Learned my first word from you: “twirl”.".to_owned()
        )));

        run(&mut world, 10.0);
        world.begin(Activity::Explore);
        let heard = world.hear("Twirl for me, please!");
        assert_eq!(
            heard.understanding,
            Understanding::Doing {
                word: "twirl".to_owned(),
                activity: Activity::Dance
            }
        );
        assert_eq!(world.ghost.plan.activity, Activity::Dance);
        assert!(world.life.today.words.contains(&"twirl".to_owned()));
    }

    #[test]
    fn words_said_while_the_ghost_does_something_come_to_mean_it() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.plant.thirst = 0.9;
        for _ in 0..2 {
            world.begin(Activity::Tend);
            run(&mut world, 1.0);
            world.hear("water the plant");
        }
        assert_eq!(
            world.life.vocabulary.meaning(&["water".to_owned()]),
            Some(("water".to_owned(), Activity::Tend))
        );
        // Floating and coming to the glass are what any words bring.
        world.begin(Activity::Float);
        run(&mut world, NAMED_FOR as f32 + 1.0);
        world.hear("hmm");
        world.hear("hmm");
        assert!(world.life.vocabulary.words["hmm"].meanings.is_empty());
    }

    #[test]
    fn what_the_viewer_does_right_after_speaking_shows_what_they_meant() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("catch");
        run(&mut world, 1.0);
        world.kick_ball(Vec3::X);
        world.kick_ball(Vec3::Z);
        let catch = ["catch".to_owned()];
        let once = world.life.vocabulary.evidence(&catch, Activity::Fetch);
        assert!(once > 1.0 && once < SHOWN, "{once}");
        // Too late to be about the words.
        world.hear("snack");
        run(&mut world, 13.0);
        world.drop_mote(Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(
            world
                .life
                .vocabulary
                .evidence(&["snack".to_owned()], Activity::Chase),
            0.0
        );
        world.hear("catch");
        run(&mut world, 0.5);
        world.kick_ball(Vec3::X);
        assert_eq!(
            world.life.vocabulary.meaning(&catch),
            Some(("catch".to_owned(), Activity::Fetch))
        );
        // Darting the pointer across the glass shows the finger game.
        world.hear("finger");
        world.point_at(Some(Vec3::new(-0.6, 1.2, ROOM_MAX.z)));
        world.point_at(Some(Vec3::new(0.6, 1.2, ROOM_MAX.z)));
        assert!(
            world
                .life
                .vocabulary
                .evidence(&["finger".to_owned()], Activity::Pounce)
                > 1.0
        );
    }

    #[test]
    fn being_told_no_stops_what_the_ghost_is_doing_and_unlearns_the_words() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world
            .life
            .vocabulary
            .teach(&["spin".to_owned()], Activity::Dance, 2.0);
        world.hear("spin");
        assert_eq!(world.ghost.plan.activity, Activity::Dance);
        run(&mut world, 1.0);
        let heard = world.hear("no, stop");
        assert!(heard.stopped);
        // It leaves off and comes to the glass, a little sheepish.
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(kinds(&world, world.time).contains(&EventKind::Scolded));
        assert!(
            world
                .life
                .vocabulary
                .evidence(&["spin".to_owned()], Activity::Dance)
                < 2.0
        );
        assert_eq!(world.life.vocabulary.meaning(&["spin".to_owned()]), None);
    }

    #[test]
    fn a_no_with_a_known_word_stops_the_ghost_without_starting_it_again() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world
            .life
            .vocabulary
            .teach(&["spin".to_owned()], Activity::Dance, 2.0);
        world.begin(Activity::Dance);
        run(&mut world, 1.0);
        let heard = world.hear("no spin!");
        assert!(heard.stopped);
        assert_eq!(heard.understanding, Understanding::Nothing);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert_eq!(
            world
                .life
                .vocabulary
                .evidence(&["spin".to_owned()], Activity::Dance),
            2.0
        );
    }

    #[test]
    fn a_mind_routing_a_message_without_words_teaches_nothing() {
        let mut world = World::new(6, 23.0);
        world.notice_viewer();
        world.hear("twirl");
        world.hear("goodnight");
        world.heed(Activity::Rest);
        assert_eq!(
            world
                .life
                .vocabulary
                .evidence(&["twirl".to_owned()], Activity::Rest),
            0.0
        );
        // Praise afterward isn't about the earlier word either.
        world.hear("good");
        assert_eq!(
            world
                .life
                .vocabulary
                .evidence(&["twirl".to_owned()], Activity::Rest),
            0.0
        );
    }

    #[test]
    fn a_word_it_knows_for_something_it_cant_do_now_is_understood_but_not_done() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world
            .life
            .vocabulary
            .teach(&["nap".to_owned()], Activity::Rest, 2.0);
        world.ghost.needs.energy = 1.0;
        let heard = world.hear("nap time");
        assert_eq!(
            heard.understanding,
            Understanding::Unable {
                word: "nap".to_owned(),
                activity: Activity::Rest
            }
        );
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
    }

    #[test]
    fn a_mind_routing_the_words_teaches_them_and_does_not_restart_the_plan() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("peekaboo");
        world.heed(Activity::Hide);
        assert_eq!(world.ghost.plan.activity, Activity::Hide);
        let started = world.ghost.plan.started;
        run(&mut world, 1.0);
        world.heed(Activity::Hide);
        assert_eq!(world.ghost.plan.started, started);
        assert_eq!(
            world.life.vocabulary.meaning(&["peekaboo".to_owned()]),
            Some(("peekaboo".to_owned(), Activity::Hide))
        );
    }

    #[test]
    fn a_goodnight_at_bedtime_sends_the_ghost_to_bed() {
        let mut world = World::new(6, 23.0);
        world.notice_viewer();
        world.hear("goodnight!");
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert!(world.ghost.plan.overnight);
        let mut world = World::new(6, 15.0);
        world.notice_viewer();
        world.hear("goodnight!");
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
    }

    #[test]
    fn praise_cheers_the_ghost_and_warms_the_bond() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.ghost.needs = Needs {
            energy: 0.5,
            fun: 0.5,
            company: 0.5,
        };
        // The first words it hears are a moment to remember, which
        // excites it.
        world.hear("hello");
        run(&mut world, 8.0);
        let bond = world.life.bond;
        world.hear("good ghost");
        world.step(0.1);
        assert_eq!(world.ghost.mood, Mood::Cheerful);
        assert!(world.life.bond > bond);
        assert_eq!(world.life.today.praises, 1);
    }

    #[test]
    fn a_ghost_that_knows_the_viewer_comes_to_greet_them() {
        for (bond, greets) in [(0.0, false), (0.5, true)] {
            let mut world = World::new(6, 12.0);
            world.life.bond = bond;
            world.begin(Activity::Window);
            run(&mut world, 1.0);
            world.notice_viewer();
            world.step(0.05);
            assert_eq!(
                world.ghost.plan.activity == Activity::Visit,
                greets,
                "{bond}"
            );
        }
    }

    #[test]
    fn what_happens_is_counted_in_the_ghosts_life() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("hello");
        run(&mut world, 2.0);
        assert_eq!(world.life.total.heard, 1);
        assert_eq!(world.life.total.times(Activity::Visit), 1);
        assert!(world.life.total.together > 1.5);
        assert!(world.life.bond > 0.0);
    }

    #[test]
    fn a_first_is_celebrated_as_a_milestone_right_after_it() {
        let mut world = World::new(2, 12.0);
        world.drop_mote(Vec3::new(-1.0, 1.4, -0.5));
        world.begin(Activity::Chase);
        run(&mut world, 6.0);
        let eaten = world
            .events
            .iter()
            .position(|event| event.kind == EventKind::AteMote)
            .unwrap();
        assert_eq!(
            world.events[eaten + 1].kind,
            EventKind::Milestone("Caught my first light mote.".into())
        );
        assert_eq!(world.ghost.mood, Mood::Excited);
    }

    #[test]
    fn options_follow_what_is_possible() {
        let mut world = World::new(6, 12.0);
        world.motes.clear();
        world.ghost.needs.energy = 1.0;
        world.plant.thirst = 0.0;
        let options = world.options();
        assert!(!options.contains(&Activity::Chase));
        assert!(!options.contains(&Activity::Rest));
        assert!(!options.contains(&Activity::Tend));
        world.drop_mote(Vec3::ONE);
        assert!(world.options().contains(&Activity::Chase));
    }

    #[test]
    fn hiding_needs_a_viewer_and_energy_and_drawing_needs_an_awake_ghost() {
        let mut world = World::new(6, 12.0);
        assert!(!world.options().contains(&Activity::Hide));
        assert!(world.options().contains(&Activity::Doodle));
        world.notice_viewer();
        assert!(world.options().contains(&Activity::Hide));
        world.ghost.needs.energy = 0.3;
        assert!(!world.options().contains(&Activity::Hide));
        assert!(world.options().contains(&Activity::Doodle));
        world.ghost.needs.energy = 0.25;
        assert!(!world.options().contains(&Activity::Doodle));
        world.ghost.needs.energy = 0.8;
        world.ghost.asleep = 0.9;
        assert!(!world.options().contains(&Activity::Doodle));
    }

    #[test]
    fn the_ghost_reaches_its_hiding_spot_and_a_poke_finds_it() {
        let mut spots = Vec::new();
        for seed in 0..40 {
            let mut world = World::new(seed, 12.0);
            world.notice_viewer();
            world.begin(Activity::Hide);
            let spot = world.ghost.plan.target;
            if !spots.contains(&spot) {
                spots.push(spot);
            }
            assert!((25.0..45.0).contains(&world.ghost.plan.length));
            run(&mut world, 15.0);
            assert!(
                world.ghost.plan.arrived.is_some(),
                "seed {seed}: {:?}, target {spot}",
                world.ghost.position
            );
            assert!(world.ghost.fade > 0.5, "seed {seed}");
            assert!(!world.needs_decision());
            let before = world.ghost.needs;
            world.poke();
            assert!(
                world
                    .events
                    .iter()
                    .rev()
                    .take(2)
                    .any(|event| event.kind == EventKind::Found)
            );
            assert!(
                !world
                    .events
                    .iter()
                    .any(|event| matches!(event.kind, EventKind::Poked { .. }))
            );
            assert!(world.needs_decision());
            assert!(world.ghost.velocity.y > 0.5);
            assert_eq!(world.ghost.needs.fun, (before.fun + 0.2).min(1.0));
            assert_eq!(world.ghost.needs.company, (before.company + 0.15).min(1.0));
            world.step(0.05);
            assert!(world.ghost.fade < 0.3);
            assert_eq!(world.ghost.mood, Mood::Excited);
        }
        assert_eq!(spots.len(), 5);
    }

    #[test]
    fn an_unfound_ghost_jumps_toward_the_glass_and_shouts_boo() {
        let mut world = World::new(9, 12.0);
        world.notice_viewer();
        world.begin(Activity::Hide);
        for _ in 0..1400 {
            world.step(0.05);
            if world.needs_decision() {
                break;
            }
        }
        assert!(world.needs_decision());
        assert_eq!(world.events.last().unwrap().kind, EventKind::Jumped);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Boo!");
        assert!(world.ghost.velocity.z > 0.0);
        assert!(world.ghost.plan.target.z > ROOM_MAX.z - 0.6);
        run(&mut world, 2.0);
        assert!(world.ghost.fade < 0.01);
    }

    #[test]
    fn an_unreachable_hiding_spot_is_given_up_without_jumping() {
        let mut world = World::new(4, 12.0);
        world.begin(Activity::Hide);
        world.ghost.plan.target = Vec3::new(0.0, -5.0, 0.0);
        run(&mut world, 65.0);
        assert!(world.needs_decision());
        assert_eq!(world.ghost.fade, 0.0);
        assert!(
            !world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Jumped)
        );
    }

    #[test]
    fn interrupting_a_hiding_plan_brings_the_ghost_back_into_view() {
        let mut world = World::new(4, 12.0);
        world.begin(Activity::Hide);
        run(&mut world, 15.0);
        assert!(world.ghost.fade > 0.5);
        world.begin(Activity::Explore);
        run(&mut world, 2.0);
        assert!(world.ghost.fade < 0.01);
    }

    #[test]
    fn doodling_reaches_the_glass_draws_a_picture_and_finishes() {
        for seed in 0..10 {
            let mut world = World::new(seed, 12.0);
            world.begin(Activity::Doodle);
            let sketch = world.ghost.plan.sketch.unwrap();
            assert!((-1.2..=1.2).contains(&world.ghost.plan.target.x));
            assert!((1.0..=1.6).contains(&world.ghost.plan.target.y));
            assert_eq!(world.ghost.plan.target.z, ROOM_MAX.z - 0.35);
            for _ in 0..600 {
                world.step(0.05);
                if world.needs_decision() {
                    break;
                }
            }
            assert!(world.needs_decision());
            assert_eq!(world.doodles.len(), 1);
            let doodle = world.doodles[0];
            assert_eq!(doodle.sketch, sketch);
            assert_eq!(doodle.drawn(world.time), 1.0);
            assert!(
                world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::Doodled(sketch.name()))
            );
            assert!(world.ghost.position.z > ROOM_MAX.z - 0.9);
            assert!(world.ghost.plan.target.z < ROOM_MAX.z - 0.5);
        }
    }

    #[test]
    fn only_the_newest_three_doodles_are_kept_and_they_expire() {
        let mut world = World::new(4, 12.0);
        for _ in 0..4 {
            world.begin(Activity::Doodle);
            world.ghost.position = world.ghost.plan.target;
            run(&mut world, 7.0);
        }
        assert_eq!(world.doodles.len(), 3);
        assert!(world.doodles[0].started > 7.0);
        assert!(
            world
                .doodles
                .windows(2)
                .all(|pair| pair[0].started < pair[1].started)
        );
        run(&mut world, Doodle::LIFETIME as f32);
        assert!(world.doodles.is_empty());
    }

    #[test]
    fn pictures_are_chosen_from_the_ghosts_mood_weather_and_bond() {
        let mut world = World::new(4, 12.0);
        assert_eq!(world.choose_sketch(), Sketch::Smile);
        world.ghost.mood = Mood::Bored;
        assert_eq!(world.choose_sketch(), Sketch::Spiral);
        world.ghost.mood = Mood::Content;
        world.weather.sky = Sky::Rain;
        assert_eq!(world.choose_sketch(), Sketch::Wave);
        world.weather.sky = Sky::Rainbow;
        assert_eq!(world.choose_sketch(), Sketch::Sun);
        world.weather.sky = Sky::Snow;
        assert_eq!(world.choose_sketch(), Sketch::Snowflake);
        world.weather.sky = Sky::Clear;
        world.notice_viewer();
        assert_eq!(world.choose_sketch(), Sketch::Smile);
        world.life.bond = GREETS_FROM;
        assert_eq!(world.choose_sketch(), Sketch::Heart);
        world.ghost.mood = Mood::Cheerful;
        for _ in 0..20 {
            assert!(matches!(
                world.choose_sketch(),
                Sketch::Heart | Sketch::Star | Sketch::Flower
            ));
        }
    }

    #[test]
    fn the_same_seed_plays_out_the_same_way() {
        let play = || {
            let mut world = World::new(11, 12.0);
            for round in 0..60 {
                if world.needs_decision() {
                    let options = world.options();
                    world.begin(options[round % options.len()]);
                }
                run(&mut world, 1.0);
            }
            (world.ghost.position, world.motes.len())
        };
        assert_eq!(play(), play());
    }

    #[test]
    fn each_occasion_is_celebrated_once_a_day_and_new_year_again_at_midnight() {
        use jiff::civil::Date;

        let mut world = World::new(1, 12.0);
        world.date = Date::constant(2026, 10, 24);
        world.step(0.0);
        world.step(1.0);
        assert_eq!(world.life.total.celebrations, 1);
        assert_eq!(
            world.events[0].kind,
            EventKind::Celebrated(Occasion::Halloween)
        );
        world.date = Date::constant(2026, 10, 25);
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 2);
        world.date = Date::constant(2026, 11, 1);
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 2);
        world.date = Date::constant(2026, 12, 31);
        world.hour = 19.999;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 3);
        world.hour = 20.0;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 4);
        world.hour = 23.999;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 4);
        world.date = Date::constant(2027, 1, 1);
        world.hour = 0.0;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 5);
        let celebrations: Vec<_> = world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Celebrated(Occasion::NewYear))
            .collect();
        assert_eq!(celebrations.len(), 2);
        assert_eq!(world.event_words(celebrations[0]), "it is New Year");
        assert_eq!(world.event_words(celebrations[1]), "a new year began");
        world.step(0.1);
        world.hour = 12.0;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 5);
    }

    #[test]
    fn a_birthday_at_midnight_keeps_its_priority_and_also_marks_the_new_year() {
        use jiff::civil::Date;

        let mut world = World::new(1, 23.999);
        world.date = Date::constant(2026, 12, 31);
        world.birthday = Some(Date::constant(2026, 1, 1));
        world.step(0.1);
        world.date = Date::constant(2027, 1, 1);
        world.hour = 0.0;
        world.step(0.1);
        world.step(0.1);
        assert_eq!(world.occasion(), Some(Occasion::Birthday));
        assert_eq!(world.decoration().unwrap().occasion, Occasion::Birthday);
        assert!(world.fireworks());
        assert_eq!(world.life.total.celebrations, 3);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Celebrated(Occasion::Birthday))
                .count(),
            1
        );
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Celebrated(Occasion::NewYear))
                .count(),
            2
        );
    }

    #[test]
    fn decorations_are_obstacles_and_visible_things_only_while_present() {
        use jiff::civil::Date;

        let mut world = World::new(1, 22.0);
        for (date, birthday, present) in [
            (Date::constant(2026, 10, 24), None, true),
            (Date::constant(2026, 12, 24), None, true),
            (Date::constant(2026, 12, 31), None, true),
            (Date::constant(2027, 1, 1), None, true),
            (
                Date::constant(2027, 3, 1),
                Some(Date::constant(2026, 3, 1)),
                true,
            ),
            (Date::constant(2027, 2, 14), None, false),
            (Date::constant(2026, 6, 21), None, false),
        ] {
            world.date = date;
            world.birthday = birthday;
            world.ghost.position = Vec3::new(0.95, 0.4, 0.2);
            let target = world.position_of(Thing::Decoration);
            assert_eq!(target.is_some(), present);
            assert_eq!(world.obstacles().len(), if present { 4 } else { 3 });
            world.ghost.heading =
                (target.unwrap_or(Decoration::POSITION) - world.ghost.position - Vec3::Y * 0.1)
                    .normalize();
            let sight = world.sight();
            assert_eq!(
                sight
                    .in_view
                    .iter()
                    .any(|seen| seen.thing == Thing::Decoration),
                present
            );
            assert!(!sight.out_of_view.contains(&Thing::Decoration));
            world.ghost.attention = None;
            world.attend("decoration", 5.0);
            assert_eq!(world.ghost.attention.is_some(), present);
        }
    }

    #[test]
    fn a_ball_bounces_off_the_decoration_and_passes_the_same_spot_on_ordinary_days() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2026, 10, 31);
        world.ball = Ball::at(Decoration::POSITION.x, Decoration::POSITION.z + 0.55);
        world.ball.velocity = -Vec3::Z * 1.8;
        let mut ordinary = world.clone();
        ordinary.date = jiff::civil::Date::constant(2026, 11, 1);
        let radius = world.decoration().unwrap().radius();
        for _ in 0..80 {
            world.step(0.005);
            ordinary.step(0.005);
            assert!(
                flat(world.ball.position - Decoration::POSITION).length()
                    >= radius + Ball::RADIUS - 1e-4
            );
        }
        assert!(world.ball.velocity.z > 0.0);
        assert!(ordinary.ball.position.z < Decoration::POSITION.z);
    }

    #[test]
    fn the_ghost_steers_around_the_decoration_when_flying_low() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2026, 12, 24);
        world.ghost.position = Decoration::POSITION + Vec3::new(-0.3, 0.45, 0.0);
        world.ghost.plan = Plan::new(
            Activity::Explore,
            0.0,
            Decoration::POSITION + Vec3::new(0.7, 0.45, 0.0),
            0.0,
        );
        let mut ordinary = world.clone();
        ordinary.date = jiff::civil::Date::constant(2026, 6, 21);
        world.move_ghost(0.1);
        ordinary.move_ghost(0.1);
        assert!(world.ghost.velocity.x < ordinary.ghost.velocity.x);
    }

    #[test]
    fn an_unfound_ghost_shouts_louder_on_halloween_night() {
        let mut world = World::new(9, 22.0);
        world.date = jiff::civil::Date::constant(2026, 10, 31);
        world.notice_viewer();
        world.begin(Activity::Hide);
        for _ in 0..1400 {
            world.step(0.05);
            if world.needs_decision() {
                break;
            }
        }
        assert_eq!(world.events.last().unwrap().kind, EventKind::Jumped);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "BOO!");
    }

    #[test]
    fn birthdays_are_cheerful_unless_sleepiness_irritation_or_delight_takes_over() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2027, 3, 1);
        world.birthday = Some(jiff::civil::Date::constant(2026, 3, 1));
        world.ghost.needs.fun = 0.1;
        world.ghost.needs.company = 0.1;
        assert_eq!(world.feeling(), Mood::Cheerful);
        world.ghost.needs.energy = 0.1;
        assert_eq!(world.feeling(), Mood::Sleepy);
        world.ghost.needs.energy = 0.8;
        world.record(EventKind::Poked { count: 3 });
        assert_eq!(world.feeling(), Mood::Grumpy);
        world.events.clear();
        world.record(EventKind::AteMote);
        assert_eq!(world.feeling(), Mood::Excited);
    }

    #[test]
    fn valentine_doodles_are_hearts_whatever_the_weather_or_mood() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2027, 2, 14);
        for sky in [Sky::Clear, Sky::Rain, Sky::Snow, Sky::Rainbow] {
            world.weather.sky = sky;
            for mood in Mood::ALL {
                world.ghost.mood = mood;
                for _ in 0..10 {
                    assert_eq!(world.choose_sketch(), Sketch::Heart);
                }
            }
        }
    }
}
