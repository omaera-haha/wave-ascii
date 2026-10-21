mod activity;
mod life;
mod needs;
mod random;
mod sight;
mod things;
mod weather;

use glam::Vec3;

pub use activity::{Activity, Plan};
pub use life::{Entry, Life};
pub use needs::{Mood, Needs};
pub use random::Random;
pub use sight::{Seen, Thing};
pub use things::{Ball, Lamp, Mote, Nook, Plant, ROOM_MAX, ROOM_MIN, WINDOW_CENTER, flat};
use weather::Change;
pub use weather::{Sky, Weather};

/// At most this many motes drift in the room at once.
const MAX_MOTES: usize = 6;
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
const GREETS_FROM: f32 = 0.3;

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
    RainStarted,
    RainStopped,
    RainbowAppeared,
    RainbowFaded,
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
            Self::Tended => "watered the plant".to_owned(),
            Self::Bloomed => "the plant came into flower".to_owned(),
            Self::SwitchedLamp { on: true } => "switched the lamp on".to_owned(),
            Self::SwitchedLamp { on: false } => "switched the lamp off".to_owned(),
            Self::FellAsleep => "fell asleep".to_owned(),
            Self::WokeUp => "woke up".to_owned(),
            Self::Heard(text) => format!("the viewer said: {text}"),
            Self::Said(text) => format!("you said: {text}"),
            Self::ViewerArrived => "the viewer came to the glass".to_owned(),
            Self::ViewerLeft => "the viewer went away".to_owned(),
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
            Self::RainStarted => "it started to rain outside".to_owned(),
            Self::RainStopped => "the rain stopped".to_owned(),
            Self::RainbowAppeared => "the rain stopped and a rainbow came out".to_owned(),
            Self::RainbowFaded => "the rainbow faded".to_owned(),
        }
    }
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
}

/// The person at the glass, as the world knows them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewer {
    /// Where the pointer touches the glass, while it is over the room.
    pub pointer: Option<Vec3>,
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
    pub ghost: Ghost,
    pub lamp: Lamp,
    pub ball: Ball,
    pub plant: Plant,
    pub nook: Nook,
    pub motes: Vec<Mote>,
    pub events: Vec<Event>,
    pub viewer: Viewer,
    pub weather: Weather,
    /// Its life over days: its bond with the viewer, habits, and diary.
    pub life: Life,
    random: Random,
    next_mote: u32,
    next_mote_at: f64,
    viewer_present: bool,
    /// Which way the ghost means to push the ball next.
    push: Vec3,
}

impl World {
    pub fn new(seed: u64, hour: f32) -> Self {
        let start = Vec3::new(0.2, 1.1, 0.0);
        let mut random = Random::new(seed);
        let weather = Weather::new(&mut random);
        Self {
            time: 0.0,
            hour,
            ghost: Ghost {
                name: "Boo".to_owned(),
                position: start,
                velocity: Vec3::ZERO,
                needs: Needs::default(),
                plan: Plan::new(Activity::Float, 0.0, start, 6.0),
                asleep: 0.0,
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
            events: Vec::new(),
            viewer: Viewer::default(),
            weather,
            life: Life::default(),
            random,
            next_mote: 0,
            next_mote_at: 8.0,
            viewer_present: false,
            push: Vec3::X,
        }
    }

    /// Moves the world forward by `seconds`, at most a tenth of a second at
    /// a time.
    pub fn step(&mut self, seconds: f32) {
        let mut remaining = seconds.max(0.0);
        while remaining > 0.0 {
            let slice = remaining.min(0.1);
            remaining -= slice;
            self.advance(slice);
        }
    }

    fn advance(&mut self, seconds: f32) {
        self.time += f64::from(seconds);

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
        if let Some(change) = self.weather.step(self.time, daytime, &mut self.random) {
            if change == Change::RainbowFaded {
                self.ghost.saw_rainbow = false;
            }
            self.record(match change {
                Change::RainStarted => EventKind::RainStarted,
                Change::RainStopped => EventKind::RainStopped,
                Change::RainbowAppeared => EventKind::RainbowAppeared,
                Change::RainbowFaded => EventKind::RainbowFaded,
            });
        }

        let obstacles = self.obstacles();
        self.ball.roll(seconds, &obstacles);
        self.plant.thirst = (self.plant.thirst + seconds / Plant::THIRST_TIME).min(1.0);

        let present = self.viewer_present();
        if present != self.viewer_present {
            self.viewer_present = present;
            self.record(if present {
                EventKind::ViewerArrived
            } else {
                EventKind::ViewerLeft
            });
            if present {
                self.greet();
            }
        }
        self.life.pass(seconds, present);

        self.carry_out(seconds);
        self.move_ghost(seconds);
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
                Activity::Rest => self.ghost.needs.energy < 0.85,
                Activity::Tend => self.plant.thirst > 0.25,
                Activity::Visit => self.viewer_present() || self.ghost.needs.company < 0.6,
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
            Activity::Window => {
                let outside = match self.weather.sky {
                    Sky::Rainbow => ", where a rainbow is shining",
                    Sky::Rain => ", where it is raining",
                    Sky::Clear if self.is_night() => " at the night sky",
                    Sky::Clear => " at the sky and the hills",
                };
                format!(
                    "Float to the window and look out{outside}. Fits when curious or calm, \
                     and above all when something is happening outside."
                )
            }
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
        let position = self.ghost.position;
        let length = match activity {
            Activity::Float => self.random.range(5.0, 11.0),
            Activity::Visit => self.random.range(8.0, 15.0),
            Activity::Window => self.random.range(10.0, 18.0),
            _ => 0.0,
        };
        let target = match activity {
            Activity::Explore => self.random_spot(),
            Activity::Rest => self.nook.bed(),
            Activity::Tend => self.plant.top() + Vec3::new(0.0, 0.25, 0.3),
            Activity::Lamp => self.lamp.bulb() + Vec3::new(0.35, -0.15, 0.25),
            Activity::Visit => self.glass_spot(),
            Activity::Window => WINDOW_CENTER + Vec3::new(0.0, -0.05, 0.55),
            Activity::Float | Activity::Chase | Activity::Play => position,
        };
        let mut plan = Plan::new(activity, self.time, target, length);
        if activity == Activity::Chase {
            plan.mote = self.nearest_mote().map(|mote| mote.id);
        }
        if activity == Activity::Play {
            self.push = self.push_direction();
        }
        self.ghost.plan = plan;
        self.record(EventKind::Began(activity));
    }

    /// The viewer poked the ghost.
    pub fn poke(&mut self) {
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
    }

    /// The viewer flicked the ball along `direction` on the floor.
    pub fn kick_ball(&mut self, direction: Vec3) {
        let along = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
        self.ball.velocity += along * 2.2;
        self.record(EventKind::KickedBall);
    }

    /// The viewer opened the app again after `away` seconds.
    pub fn record_return(&mut self, away: f32) {
        self.record(EventKind::Returned {
            away: f64::from(away),
        });
    }

    /// The viewer said `text` to the ghost, which wakes and comes to listen.
    pub fn hear(&mut self, text: &str) {
        self.wake();
        self.ghost.needs.change(0.0, 0.05, 0.25);
        self.record(EventKind::Heard(text.to_owned()));
        if self.ghost.plan.activity != Activity::Visit {
            self.begin(Activity::Visit);
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

    fn record(&mut self, kind: EventKind) {
        self.life.note(&kind);
        self.events.push(Event {
            time: self.time,
            kind,
        });
    }

    // Once it knows the viewer, the ghost leaves whatever it was idly doing
    // to come to the glass when they arrive.
    fn greet(&mut self) {
        let idle = matches!(
            self.ghost.plan.activity,
            Activity::Float | Activity::Explore | Activity::Window
        );
        if self.life.bond >= GREETS_FROM
            && self.ghost.asleep < 0.3
            && (idle || self.ghost.plan.done)
        {
            self.begin(Activity::Visit);
        }
    }

    fn wake(&mut self) {
        if self.ghost.asleep > 0.3 {
            self.ghost.asleep = 0.0;
            self.record(EventKind::WokeUp);
        }
        if self.ghost.plan.activity == Activity::Rest {
            self.ghost.plan.done = true;
        }
    }

    // Floor circles the ball bounces off and the ghost flies around, with
    // the height they reach.
    fn obstacles(&self) -> [(Vec3, f32); 3] {
        [
            (self.lamp.position, Lamp::BASE_RADIUS),
            (self.plant.position, Plant::POT_RADIUS),
            (self.nook.position, Nook::RADIUS),
        ]
    }

    fn random_spot(&mut self) -> Vec3 {
        Vec3::new(
            self.random.range(ROOM_MIN.x + 0.5, ROOM_MAX.x - 0.5),
            self.random.range(0.6, ROOM_MAX.y - 0.6),
            self.random.range(ROOM_MIN.z + 0.5, ROOM_MAX.z - 0.5),
        )
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
        let plan = &self.ghost.plan;
        // Only resting keeps the ghost asleep.
        if plan.done || plan.activity != Activity::Rest {
            self.ghost.asleep = (self.ghost.asleep - seconds / 1.2).max(0.0);
        }
        if plan.done {
            return;
        }
        let arrived_now = position.distance(plan.target) < ARRIVED;
        if arrived_now && plan.arrived.is_none() {
            self.ghost.plan.arrived = Some(time);
        }
        // Once on the cushion, a rest lasts until the ghost is rested or
        // woken; on the way there it gives up like any plan.
        let resting =
            self.ghost.plan.activity == Activity::Rest && self.ghost.plan.arrived.is_some();
        if self.ghost.plan.elapsed(time) > GIVE_UP && !resting {
            self.ghost.plan.done = true;
            return;
        }

        let mut look = None;
        match self.ghost.plan.activity {
            Activity::Float => {
                let anchor = self.ghost.plan.target;
                let phase = time as f32;
                self.ghost.plan.arrived.get_or_insert(time);
                look = Some(anchor + Vec3::new((phase * 0.3).sin(), 0.0, 1.0));
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
                    if self.ghost.needs.energy >= 0.98 {
                        self.ghost.plan.done = true;
                        self.ghost.asleep = 0.0;
                        self.record(EventKind::WokeUp);
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
                look = Some(WINDOW_CENTER);
                if self.ghost.plan.stayed(time) > self.ghost.plan.length {
                    self.ghost.plan.done = true;
                }
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
        }
        self.ghost.look = look;
    }

    // Steers toward the plan's target, around obstacles and away from the
    // walls.
    fn move_ghost(&mut self, seconds: f32) {
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
            Activity::Rest => 0.5,
            Activity::Explore => 0.7,
            _ => 0.8,
        } * (0.6 + 0.4 * ghost.needs.energy);

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
        let low = ROOM_MIN + Vec3::new(0.3, 0.45, 0.3);
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
        for (center, radius, top) in obstacles {
            let offset = flat(ghost.position - center);
            let reach = radius + 0.3;
            let distance = offset.length();
            if ghost.position.y < top && distance < reach && distance > 1e-4 {
                let push = offset / distance * (reach - distance) * 6.0;
                steer += Vec3::new(push.x, 0.0, push.y);
            }
        }

        let steer = steer.clamp_length_max(3.0);
        ghost.velocity += steer * seconds;
        ghost.position += ghost.velocity * seconds;
        ghost.position = ghost.position.clamp(
            ROOM_MIN + Vec3::new(0.12, 0.38, 0.12),
            ROOM_MAX - Vec3::splat(0.12),
        );
    }

    fn update_needs(&mut self, seconds: f32) {
        let present = self.viewer_present();
        let night = if self.is_night() { 1.4 } else { 1.0 };
        let ghost = &mut self.ghost;
        if ghost.asleep > 0.5 {
            ghost.needs.change(0.012 * seconds, 0.0, 0.0);
            return;
        }
        let effort = (ghost.velocity.length() / 1.3).min(1.0);
        let mut fun = -0.0016;
        let mut company = -0.0012;
        match ghost.plan.activity {
            Activity::Explore => fun += 0.004,
            Activity::Visit if present => company += 0.01,
            Activity::Window if ghost.plan.arrived.is_some() => {
                fun += 0.004 + 0.02 * self.weather.rainbow(self.time)
            }
            _ => {}
        }
        if present {
            company += 0.004;
        }
        ghost.needs.change(
            -(0.0012 + 0.0025 * effort) * night * seconds,
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
        if recent(6.0).any(|event| {
            matches!(
                event.kind,
                EventKind::AteMote | EventKind::NudgedBall | EventKind::Bloomed
            )
        }) {
            return Mood::Excited;
        }
        if self.weather.rainbow(self.time) > 0.3 && ghost.saw_rainbow {
            return Mood::Excited;
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
            run(&mut world, 70.0);
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
        let mut world = World::new(4, 23.0);
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
}
