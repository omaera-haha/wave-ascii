#[allow(dead_code, reason = "a general client, of which the ghost uses a part")]
pub mod claude;
mod decision;
mod instinct;
#[allow(dead_code, reason = "a general client, of which the ghost uses a part")]
pub mod jev;
mod mail;
pub(crate) mod musing;
mod observation;
mod reflection;

use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::runtime::Runtime;

use crate::store::{Config, Memory, Remark, Speaker};
use crate::world::{Activity, EventKind, Heard, Question, Tone, Understanding, World};
use claude::{Cache, Claude, Effort, Reply};
use decision::Decision;
use jev::Jev;
use reflection::Reflection;

pub use observation::recent as recent_events;

/// Seconds the ghost waits for System One before deciding on instinct.
const PATIENCE: f64 = 2.5;
/// Seconds into a run before the first reflection.
const FIRST_REFLECTION: f64 = 4.0;
/// How many times less often the ghost reflects while nobody is around.
const ALONE_FACTOR: f64 = 5.0;
/// Seconds a mood suggested by a mind holds.
const MOOD_HOLDS: f32 = 90.0;
/// Seconds the ghost keeps looking at what a mind turned it to.
const LOOKS_FOR: f32 = 15.0;
/// Without System Two, the chance that setting out on something crosses
/// the ghost's mind, and the seconds between its idle musings and dreams.
const MUSES_ON_SETTING_OUT: f32 = 0.35;
const MUSES_EVERY: (f32, f32) = (50.0, 100.0);
/// A night holds many dreams, but not one every few breaths.
const DREAMS_EVERY: (f32, f32) = (45.0, 90.0);
/// Seconds until a thought it had feels fresh again. Until then it comes
/// back only rarely, and less the more recently it was had.
const THOUGHT_FRESHENS: f64 = 4.0 * 3600.0;
/// How fresh the freshest musing that fits must be for the ghost to bother
/// thinking it; with only lately-had thoughts to hand, it stays quiet.
const QUIET_BELOW: f32 = 0.2;
/// Distinct thoughts remembered before those already fresh again are let go.
const THOUGHTS_REMEMBERED: usize = 512;
/// Seconds the ghost's own words get before it brings up something new.
const SPEECH_SETTLES: f64 = 8.0;
/// Seconds a thought stays before a musing may replace it.
const THOUGHT_HOLDS: f64 = crate::world::dream::HOLDS;
const MEMORIES_KEPT: usize = 60;
const REMARKS_KEPT: usize = 40;

/// How one of the minds is doing, for the status line.
#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// There is no key for it, so the ghost goes without.
    Off,
    Ready,
    Busy,
    Failed(String),
}

/// What the ghost remembers and what has been said, kept between runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Journal {
    pub memories: Vec<Memory>,
    pub conversation: Vec<Remark>,
}

/// What a mind sends back from its background task.
enum Answer {
    Decided {
        id: u64,
        decision: Result<Decision, String>,
    },
    Requested {
        id: u64,
        activity: Result<Option<Activity>, String>,
    },
    Reflected(Result<Option<Reflection>, String>),
    LetterWritten {
        index: usize,
        original: crate::world::mail::Letter,
        lines: Result<Option<Vec<String>>, String>,
    },
}

struct HeardMessage {
    text: String,
    story: Option<(u64, bool)>,
}

/// The ghost's two minds. System One (Jev) makes the quick choices of what
/// to do next; System Two (Claude) thinks things over now and then, sets
/// goals, and talks. Without keys the ghost lives on instinct alone. Requests
/// run in the background, so the world never waits on them.
pub struct Mind {
    runtime: Option<Runtime>,
    jev: Option<Jev>,
    claude: Option<Claude>,
    sender: Sender<Answer>,
    receiver: Receiver<Answer>,
    /// System One's status.
    pub one: Status,
    /// System Two's status.
    pub two: Status,
    pub journal: Journal,
    reflect_every: f64,
    effort: Option<Effort>,
    cache: Option<Cache>,
    /// Never call a model, whatever the keys.
    offline: bool,
    /// The decision being waited on: its id and when it was asked for.
    deciding: Option<(u64, f64)>,
    next_id: u64,
    reflecting: bool,
    last_reflection: Option<f64>,
    /// When the ghost last mused on its own, when it muses next, and
    /// whether its musings are dreams.
    mused_at: f64,
    /// Its latest thoughts, newest last.
    /// When it last had each thought.
    thought_at: HashMap<String, f64>,
    next_musing: f64,
    dreaming: bool,
    /// Messages from the viewer still waiting for a reply.
    heard: VecDeque<HeardMessage>,
    /// The id of the latest message's request routing; answers to earlier
    /// messages that arrive later are dropped.
    latest_request: u64,
    /// How many times the viewer has named the ghost, and how many times
    /// when the reflection on its way was asked for: a reflection can't undo
    /// a newer name.
    renamed: u64,
    reflection_renamed: u64,
    musing_focus: bool,
    reflecting_for_viewer: bool,
    pending_story: Option<u64>,
    reflecting_story: Option<(u64, bool)>,
    said_through: Option<(f64, usize)>,
    requested_letters: Vec<crate::world::mail::Letter>,
    viewer_letters_read: u32,
}

impl Mind {
    /// A mind using the models `config` has keys for, unless `offline`.
    pub fn new(config: &Config, journal: Journal, offline: bool) -> Self {
        let (sender, receiver) = mpsc::channel();
        let mut mind = Self {
            runtime: None,
            jev: None,
            claude: None,
            sender,
            receiver,
            one: Status::Off,
            two: Status::Off,
            journal,
            reflect_every: f64::from(config.reflect_every.max(30.0)),
            effort: config.claude_effort,
            cache: config.cache,
            offline,
            deciding: None,
            next_id: 0,
            reflecting: false,
            last_reflection: None,
            mused_at: 0.0,
            thought_at: HashMap::new(),
            next_musing: f64::from(MUSES_EVERY.0) * 0.4,
            dreaming: false,
            heard: VecDeque::new(),
            latest_request: 0,
            renamed: 0,
            reflection_renamed: 0,
            musing_focus: false,
            reflecting_for_viewer: false,
            pending_story: None,
            reflecting_story: None,
            said_through: None,
            requested_letters: Vec::new(),
            viewer_letters_read: 0,
        };
        mind.reconfigure(config);
        mind
    }

    /// Starts using the keys, models, and settings in `config`, for example
    /// after the viewer changes them. Requests already out finish as they
    /// were sent.
    pub fn reconfigure(&mut self, config: &Config) {
        self.effort = config.claude_effort;
        self.cache = config.cache;
        self.reflect_every = f64::from(config.reflect_every.max(30.0));
        if self.offline {
            return;
        }
        self.jev = config
            .typesafe_key()
            .and_then(|key| Jev::new(key, config.jev_model.clone()).ok())
            .map(|jev| match &config.jev_base_url {
                Some(url) => jev.with_base_url(url.clone()),
                None => jev,
            });
        self.claude = config
            .anthropic_key()
            .and_then(|key| Claude::new(key, config.claude_model.clone()).ok())
            .map(|claude| match &config.claude_base_url {
                Some(url) => claude.with_base_url(url.clone()),
                None => claude,
            });
        if (self.jev.is_some() || self.claude.is_some()) && self.runtime.is_none() {
            self.runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .ok();
        }
        if self.runtime.is_none() {
            self.jev = None;
            self.claude = None;
        }
        self.one = if self.jev.is_some() {
            Status::Ready
        } else {
            Status::Off
        };
        self.two = if self.claude.is_some() {
            Status::Ready
        } else {
            Status::Off
        };
        self.deciding = None;
    }

    /// Whether the ghost can answer in words.
    pub fn can_talk(&self) -> bool {
        self.claude.is_some()
    }

    pub fn write_letter(&mut self, world: &World) {
        let Some(claude) = self.claude.clone() else {
            return;
        };
        let fresh = world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::WroteLetter)
            .count();
        let start = world.life.letters.len().saturating_sub(fresh);
        for (index, original) in world.life.letters.iter().enumerate().skip(start) {
            if original.read || original.claude || self.requested_letters.contains(original) {
                continue;
            }
            let original = original.clone();
            self.requested_letters.push(original.clone());
            if self.requested_letters.len() > crate::world::mail::KEPT {
                self.requested_letters.remove(0);
            }
            let claude = claude.clone();
            let request = mail::request(
                world,
                &original,
                &self.journal.memories,
                self.effort,
                self.cache,
            );
            self.spawn(async move {
                let lines = match claude.structured(&request).await {
                    Ok(Reply::Json { value, .. }) => serde_json::from_value::<mail::Written>(value)
                        .map(|written| Some(written.lines))
                        .map_err(|error| error.to_string()),
                    Ok(_) => Ok(None),
                    Err(error) => Err(error.to_string()),
                };
                Answer::LetterWritten {
                    index,
                    original,
                    lines,
                }
            });
        }
    }

    /// One frame of thinking: applies whatever the minds have answered, and
    /// asks them again when it is time.
    pub fn think(&mut self, world: &mut World) {
        self.remember_speech(world);
        // A name learned from paper supersedes a reflection already on its
        // way, just as a name given in conversation does.
        if world.life.total.viewer_letters_read != self.viewer_letters_read {
            if world.events.iter().any(|event| {
                event.kind == EventKind::ReadViewerLetter
                    && world.events.iter().any(|named| {
                        named.time == event.time && matches!(named.kind, EventKind::Named(_))
                    })
            }) {
                self.renamed += 1;
            }
            self.viewer_letters_read = world.life.total.viewer_letters_read;
        }
        self.write_letter(world);
        if world.telling.is_none() {
            self.pending_story = None;
        }
        // Messages queued for System Two go unanswered once it is turned
        // off, but a story the ghost came to the glass to tell is told
        // offline instead.
        if self.claude.is_none() && !self.heard.is_empty() {
            let story = self
                .heard
                .drain(..)
                .filter_map(|message| message.story)
                .next_back();
            if let Some((id, korean)) = story
                && self.pending_story == Some(id)
                && world
                    .telling
                    .as_ref()
                    .is_some_and(|telling| telling.lines.is_empty())
            {
                let lines = musing::story(world, korean);
                world.tell(lines);
                self.pending_story = None;
            }
        }
        while let Ok(answer) = self.receiver.try_recv() {
            self.apply(answer, world);
        }

        if world.needs_decision() {
            match (&self.jev, self.deciding) {
                (Some(jev), None) => {
                    let options: Vec<(Activity, String)> = world
                        .options()
                        .into_iter()
                        .map(|activity| (activity, world.describe(activity)))
                        .collect();
                    let questions = decision::questions(&options);
                    let state = observation::observe(world);
                    let id = self.next_id;
                    self.next_id += 1;
                    self.deciding = Some((id, world.time));
                    self.one = Status::Busy;
                    let jev = jev.clone();
                    self.spawn(async move {
                        let decision = match jev.ask(&state, &questions).await {
                            Ok(answers) => decision::read(&answers)
                                .ok_or_else(|| "Jev's answer had no activity".to_owned()),
                            Err(error) => Err(error.to_string()),
                        };
                        Answer::Decided { id, decision }
                    });
                }
                (Some(_), Some((_, asked))) if world.time - asked > PATIENCE => {
                    self.deciding = None;
                    decide(world, None);
                }
                (None, _) => decide(world, None),
                _ => {}
            }
        }

        if self.claude.is_none() {
            self.muse(world);
        }

        if let Some(claude) = &self.claude
            && !self.reflecting
        {
            let interval = if world.viewer_present() {
                self.reflect_every
            } else {
                self.reflect_every * ALONE_FACTOR
            };
            let due = match self.last_reflection {
                None => world.time > FIRST_REFLECTION,
                Some(last) => world.time - last > interval,
            };
            if (due && world.focus.is_none()) || !self.heard.is_empty() {
                let heard = self.heard.pop_front();
                self.reflecting_story = heard.as_ref().and_then(|heard| heard.story);
                let observation = heard.as_ref().map_or_else(
                    || observation::observe(world),
                    |message| observation::for_message(world, &message.text),
                );
                let request = reflection::request(
                    &observation,
                    &self.journal.memories,
                    &world.life,
                    &self.journal.conversation,
                    heard.as_ref().map(|heard| heard.text.as_str()),
                    self.effort,
                    self.cache,
                );
                self.reflecting = true;
                self.reflection_renamed = self.renamed;
                self.reflecting_for_viewer = heard.is_some();
                self.last_reflection = Some(world.time);
                self.two = Status::Busy;
                let claude = claude.clone();
                self.spawn(async move {
                    Answer::Reflected(match claude.structured(&request).await {
                        Ok(Reply::Json { value, .. }) => Ok(reflection::read(value)),
                        // A declined or cut-off reply leaves the ghost as it
                        // was.
                        Ok(Reply::Refusal { .. } | Reply::CutOff { .. }) => Ok(None),
                        Err(error) => Err(error.to_string()),
                    })
                });
            }
        }
    }

    /// The viewer said `text` to the ghost. It comes to listen, does what it
    /// was asked if System One can tell, and answers once System Two has
    /// thought about it.
    pub fn hear(&mut self, world: &mut World, text: &str) {
        self.remember_speech(world);
        let was_telling = world.telling.is_some();
        let heard = world.hear(text);
        self.journal.conversation.push(Remark {
            at: now(),
            speaker: Speaker::Viewer,
            text: text.to_owned(),
        });
        trim(&mut self.journal.conversation, REMARKS_KEPT);
        if world.ghost.asleep >= 0.3 && matches!(heard.understanding, Understanding::Airing { .. })
        {
            self.next_id += 1;
            self.latest_request = self.next_id;
            return;
        }
        if matches!(
            heard.understanding,
            Understanding::Asked {
                question: Question::ReadAloud | Question::OwnBook,
                ..
            }
        ) {
            self.next_id += 1;
            self.latest_request = self.next_id;
            if world
                .telling
                .as_ref()
                .is_some_and(|telling| telling.book.is_some())
            {
                self.pending_story = None;
            }
            return;
        }
        if matches!(
            heard.understanding,
            Understanding::WordGame
                | Understanding::Yawning
                | Understanding::OwnSong
                | Understanding::Answered(_)
                | Understanding::Unanswered(_)
                | Understanding::Hint { .. }
        ) {
            self.next_id += 1;
            self.latest_request = self.next_id;
            self.remember_speech(world);
            return;
        }
        let story = matches!(
            heard.understanding,
            Understanding::Asked {
                question: Question::Story,
                ..
            }
        ) && heard.tone != Some(Tone::Scolding);
        if story && was_telling {
            return;
        }
        let request_story = if story {
            self.next_id += 1;
            self.pending_story = Some(self.next_id);
            Some((self.next_id, heard.korean))
        } else {
            None
        };
        if self.claude.is_some() {
            self.heard.push_back(HeardMessage {
                text: text.to_owned(),
                story: request_story,
            });
        }
        // Without System One to ask, words heard before and still not
        // understood make it try something, to see if that's what they mean.
        let guessed = match heard.understanding {
            Understanding::Puzzled { again: true, .. }
                if self.jev.is_none() && !world.puzzle_active() =>
            {
                world.guess()
            }
            _ => None,
        };
        // Without System Two, it answers in the few words of its own.
        if self.claude.is_none() {
            if story {
                let lines = musing::story(world, heard.korean);
                world.tell(lines);
                self.pending_story = None;
            } else {
                let lines = musing::reply(world, &heard, guessed);
                if let Some(index) = world.pick(&vec![1.0; lines.len()]) {
                    world.say(&lines[index]);
                    if matches!(
                        heard.understanding,
                        Understanding::Asked {
                            question: Question::Away,
                            ..
                        }
                    ) && let Some(away) = &mut world.away
                    {
                        away.told = true;
                    }
                }
            }
            self.remember_speech(world);
        }
        if heard.given_name.is_some() {
            self.renamed += 1;
        }
        // Every message supersedes a request still on its way, even one that
        // asks for nothing, so a late answer can't undo a no.
        self.next_id += 1;
        self.latest_request = self.next_id;
        if let Some(jev) = &self.jev
            && world.hunt.is_none()
            && routes_to_jev(&heard)
        {
            let jev = jev.clone();
            let questions = decision::request_questions(&Activity::ALL);
            let state = serde_json::Value::String(text.to_owned());
            let id = self.latest_request;
            self.spawn(async move {
                let activity = match jev.ask(&state, &questions).await {
                    Ok(answers) => Ok(decision::read_request(&answers)),
                    Err(error) => Err(error.to_string()),
                };
                Answer::Requested { id, activity }
            });
        }
    }

    // Without System Two, the ghost still thinks to itself now and then: as
    // it sets out on something, when nothing much happens, and in its
    // dreams.
    fn muse(&mut self, world: &mut World) {
        if world.reading_viewer_letter() {
            self.mused_at = world.time;
            return;
        }
        // It tells what it did while the viewer was away once nothing else
        // it says is still showing.
        let speaking = world.asking.is_some()
            || world.telling.is_some()
            || world
                .ghost
                .speech
                .as_ref()
                .is_some_and(|line| world.time - line.time < SPEECH_SETTLES);
        if world.viewer_present()
            && world.ghost.asleep < 0.3
            && !speaking
            && world.away.as_ref().is_some_and(|away| !away.told)
        {
            let lines = musing::returned(world, world.speaks_korean());
            if let Some(index) = world.pick(&vec![1.0; lines.len()]) {
                world.say(&lines[index]);
                world.away.as_mut().unwrap().told = true;
                self.remember_speech(world);
                return;
            }
        }
        if world.reading.is_some() {
            self.mused_at = world.time;
            return;
        }
        let since = self.mused_at;
        self.mused_at = world.time;
        let asleep = world.ghost.asleep > crate::world::dream::SLEEPING;
        let focusing = world.focus.is_some();
        if self.musing_focus != focusing && !asleep {
            let remaining = (self.next_musing - world.time).max(0.0);
            self.next_musing = world.time + remaining * if focusing { 3.0 } else { 1.0 / 3.0 };
        }
        self.musing_focus = focusing;
        let (low, high) = if asleep { DREAMS_EVERY } else { MUSES_EVERY };
        let (low, high) = if world.focus.is_some() && !asleep {
            (low * 3.0, high * 3.0)
        } else {
            (low, high)
        };
        if asleep != self.dreaming {
            self.dreaming = asleep;
            self.next_musing = world.time + f64::from(low);
        }
        let began = world
            .events
            .iter()
            .rev()
            .take_while(|event| event.time > since)
            .find_map(|event| match event.kind {
                EventKind::Began(activity) => Some(activity),
                _ => None,
            });
        let passerby = (!asleep && !focusing)
            .then(|| {
                world
                    .events
                    .iter()
                    .rev()
                    .take_while(|event| event.time > since)
                    .find_map(|event| match event.kind {
                        EventKind::SawPasserby(kind) => Some(kind),
                        _ => None,
                    })
            })
            .flatten();
        let quirk = (!asleep && !focusing)
            .then(|| {
                world
                    .events
                    .iter()
                    .rev()
                    .take_while(|event| event.time > since)
                    .find_map(|event| match event.kind {
                        EventKind::Quirk(kind) => Some(kind),
                        _ => None,
                    })
            })
            .flatten();
        let friend_noticed = !asleep
            && !focusing
            && world.wearing.is_some()
            && world
                .events
                .iter()
                .rev()
                .take_while(|event| event.time > since)
                .any(|event| matches!(event.kind, EventKind::FriendArrived(_)));
        // Quirks and a friend's notice always come to mind; passersby, idle
        // musings, dreams, and setting out only when something fresh fits.
        let owned = |lines: &[&str]| -> Vec<String> {
            lines.iter().map(|line| (*line).to_owned()).collect()
        };
        let (lines, optional) = if let Some(kind) = quirk {
            (owned(kind.thoughts()), false)
        } else if friend_noticed {
            (owned(&musing::friend_notices_outfit(world)), false)
        } else if let Some(kind) = passerby {
            // The same squirrel again needn't be remarked on every time.
            (owned(&kind.thoughts()), true)
        } else if world.time >= self.next_musing {
            self.next_musing = world.time + f64::from(low + (high - low) * world.random_unit());
            let lines = if asleep {
                musing::dreams(world)
            } else {
                musing::idle(world)
            };
            (lines, true)
        } else if let Some(activity) = began
            && !asleep
            && world.random_unit() < MUSES_ON_SETTING_OUT
        {
            (owned(&musing::setting_out(world, activity)), true)
        } else {
            return;
        };
        let thinking = world
            .ghost
            .thought
            .as_ref()
            .is_some_and(|thought| world.time - thought.time < THOUGHT_HOLDS);
        if (thinking && !friend_noticed) || lines.is_empty() {
            return;
        }
        // Fresh thoughts first; a line had lately comes back rarely, the
        // less the more recently it was had.
        let weights: Vec<f32> = lines
            .iter()
            .map(|line| self.freshness(line, world.time))
            .collect();
        if optional && weights.iter().all(|weight| *weight < QUIET_BELOW) {
            return;
        }
        let weights: Vec<f32> = weights.into_iter().map(|weight| weight.max(1e-3)).collect();
        if let Some(index) = world.pick(&weights) {
            self.thought_at.insert(lines[index].clone(), world.time);
            if self.thought_at.len() > THOUGHTS_REMEMBERED {
                let now = world.time;
                self.thought_at.retain(|_, at| now - *at < THOUGHT_FRESHENS);
            }
            let line = musing::named_friend_line(world, &lines[index]);
            world.think(&line);
            if asleep {
                world.dream_of(&line);
            }
        }
    }

    // From 0 for a thought just had to 1 for one not had within the hour,
    // or never.
    fn freshness(&self, line: &str, now: f64) -> f32 {
        self.thought_at.get(line).map_or(1.0, |at| {
            (((now - at) / THOUGHT_FRESHENS).clamp(0.0, 1.0) as f32).powi(2)
        })
    }

    fn spawn(&self, task: impl Future<Output = Answer> + Send + 'static) {
        if let Some(runtime) = &self.runtime {
            let sender = self.sender.clone();
            runtime.spawn(async move {
                let _ = sender.send(task.await);
            });
        }
    }

    fn apply(&mut self, answer: Answer, world: &mut World) {
        match answer {
            Answer::LetterWritten {
                index,
                original,
                lines,
            } => {
                if let Ok(Some(lines)) = lines
                    && crate::world::mail::Letter::valid_lines(&lines)
                    && if original.lines.len() == 2 {
                        lines.len() == 2
                    } else {
                        lines.len() >= 3
                    }
                    && let Some(letter) = world.life.letters.get_mut(index)
                    && !letter.read
                    && *letter == original
                {
                    letter.lines = lines;
                    letter.claude = true;
                }
            }
            Answer::Decided { id, decision } => {
                if self.deciding.map(|(waiting, _)| waiting) != Some(id) {
                    return;
                }
                self.deciding = None;
                match decision {
                    Ok(decision) => {
                        self.one = Status::Ready;
                        if let Some(mood) = decision.mood {
                            world.suggest_mood(mood, MOOD_HOLDS);
                        }
                        if world.needs_decision() {
                            decide(world, Some(&decision));
                        }
                    }
                    Err(message) => {
                        self.one = Status::Failed(message);
                        if world.needs_decision() {
                            decide(world, None);
                        }
                    }
                }
            }
            Answer::Requested { id, .. } if id != self.latest_request => {}
            Answer::Requested {
                activity: Ok(Some(activity)),
                ..
            } => {
                self.one = Status::Ready;
                world.heed(activity);
            }
            Answer::Requested {
                activity: Ok(None), ..
            } => self.one = Status::Ready,
            Answer::Requested {
                activity: Err(message),
                ..
            } => self.one = Status::Failed(message),
            Answer::Reflected(result) => {
                self.reflecting = false;
                if let Some((id, korean)) = self.reflecting_story.take() {
                    let waiting = self.pending_story == Some(id)
                        && world
                            .telling
                            .as_ref()
                            .is_some_and(|telling| telling.lines.is_empty());
                    if waiting {
                        self.pending_story = None;
                        let lines = result
                            .as_ref()
                            .ok()
                            .and_then(|reflection| reflection.as_ref())
                            .map(|reflection| reflection::story_lines(&reflection.say))
                            .filter(|lines| !lines.is_empty())
                            .unwrap_or_else(|| musing::story(world, korean));
                        world.tell(lines);
                    }
                    match result {
                        Ok(reflection) => {
                            self.two = Status::Ready;
                            if waiting && let Some(mut reflection) = reflection {
                                reflection.say.clear();
                                self.take_in(reflection, world);
                            }
                        }
                        Err(message) => self.two = Status::Failed(message),
                    }
                    return;
                }
                match result {
                    Ok(reflection) => {
                        self.two = Status::Ready;
                        if let Some(mut reflection) = reflection {
                            if world.focus.is_some() && !self.reflecting_for_viewer {
                                reflection.say.clear();
                                reflection.write.clear();
                            }
                            self.take_in(reflection, world);
                        }
                    }
                    Err(message) => self.two = Status::Failed(message),
                }
            }
        }
    }

    // Lets the ghost act on what it thought over.
    fn take_in(&mut self, reflection: Reflection, world: &mut World) {
        if !reflection.thought.is_empty() {
            world.think(&reflection.thought);
        }
        if world.asking.is_none() && !reflection.say.is_empty() && world.ghost.asleep < 0.3 {
            world.say(&reflection.say);
            self.remember_speech(world);
        }
        if !reflection.goal.is_empty() {
            world.ghost.goal = Some(reflection.goal);
        }
        if let Some(mood) = reflection.mood {
            world.suggest_mood(mood, MOOD_HOLDS);
        }
        if !reflection.remember.is_empty()
            && !self
                .journal
                .memories
                .iter()
                .any(|memory| memory.text == reflection.remember)
        {
            self.journal.memories.push(Memory {
                at: now(),
                text: reflection.remember,
            });
            trim(&mut self.journal.memories, MEMORIES_KEPT);
        }
        if self.reflection_renamed == self.renamed {
            world.take_name(&reflection.name);
        }
        if world.asking.is_none()
            && !world.reading_viewer_letter()
            && world.comforting.is_none()
            && !reflection.write.is_empty()
        {
            world.write(&reflection.write);
        }
        if world.comforting.is_none() && !reflection.look_at.is_empty() {
            world.attend(&reflection.look_at, LOOKS_FOR);
        }
    }

    fn remember_speech(&mut self, world: &World) {
        let mut at = None;
        let mut count = 0;
        for event in &world.events {
            let EventKind::Said(text) = &event.kind else {
                continue;
            };
            if at != Some(event.time) {
                at = Some(event.time);
                count = 0;
            }
            count += 1;
            if self
                .said_through
                .is_none_or(|through| (event.time, count) > through)
            {
                self.journal.conversation.push(Remark {
                    at: now(),
                    speaker: Speaker::Ghost,
                    text: text.clone(),
                });
            }
        }
        if let Some(at) = at {
            self.said_through = Some((at, count));
        }
        trim(&mut self.journal.conversation, REMARKS_KEPT);
    }
}

impl Drop for Mind {
    fn drop(&mut self) {
        // Requests still out are abandoned rather than awaited, so quitting
        // is immediate.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

/// Starts the ghost's next activity, picked at random by the chances System
/// One gave when it answered, mixed with the pull of its own needs.
fn decide(world: &mut World, decision: Option<&Decision>) {
    let options = world.options();
    let own = instinct::weigh(world, &options);
    let weights = match decision {
        Some(decision) => instinct::blend(&decision.activities, &own),
        None => own,
    };
    let chances: Vec<f32> = weights.iter().map(|(_, chance)| *chance).collect();
    if let Some(index) = world.pick(&chances) {
        world.begin(weights[index].0);
    }
}

/// Whether to ask System One what the viewer's words ask for: a no asks
/// for nothing to be done, a question only for an answer, naming a drawing
/// only names it, and asking to focus starts a session.
fn routes_to_jev(heard: &Heard) -> bool {
    heard.tone != Some(Tone::Scolding)
        && !matches!(
            heard.understanding,
            Understanding::Asked { .. }
                | Understanding::Named { .. }
                | Understanding::Felt(_)
                | Understanding::FeltBetter
                | Understanding::Promised(_)
                | Understanding::Focusing { .. }
                | Understanding::Airing { .. }
                | Understanding::WordGame
                | Understanding::Yawning
                | Understanding::Undressing
                | Understanding::Music { .. }
                | Understanding::OwnSong
                | Understanding::Answered(_)
                | Understanding::Unanswered(_)
                | Understanding::Hint { .. }
        )
}

fn trim<Item>(items: &mut Vec<Item>, keep: usize) {
    if items.len() > keep {
        let excess = items.len() - keep;
        items.drain(..excess);
    }
}

/// Unix seconds now.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    #[test]
    fn promises_bypass_jev_but_still_queue_a_claude_reflection() {
        let config = Config {
            anthropic_api_key: Some("test-key".to_owned()),
            ..Config::default()
        };
        let mut mind = Mind::new(&config, Journal::default(), false);
        let mut world = World::new(3, 12.0);
        let heard = world.hear("see you tomorrow");
        assert!(!routes_to_jev(&heard));
        mind.hear(&mut world, "I'll be away for two weeks");
        assert_eq!(mind.heard.len(), 1);
        assert_eq!(
            mind.heard.front().unwrap().text,
            "I'll be away for two weeks"
        );
        assert_eq!(world.life.vocabulary, crate::world::Vocabulary::default());
    }

    fn offline_letter() -> crate::world::mail::Letter {
        crate::world::mail::Letter {
            day: "2026-06-21".to_owned(),
            lines: vec![
                "Dear you,".to_owned(),
                "I missed you.".to_owned(),
                "Love, Pip".to_owned(),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn a_name_learned_from_a_letter_supersedes_an_older_reflections_name() {
        let mut world = World::new(4, 12.0);
        let mut mind = offline();
        mind.reflection_renamed = mind.renamed;
        world.receive_letter("Your name is Pip.", false);
        world.ghost.position = crate::world::penpal::APPROACH;
        world.step(0.1);
        world.step(6.1);
        world.ghost.position = world.ghost.plan.target;
        world.step(0.1);
        assert_eq!(world.ghost.name, "Pip");
        mind.think(&mut world);
        mind.take_in(
            Reflection {
                name: "Old name".to_owned(),
                ..Reflection::default()
            },
            &mut world,
        );
        assert_eq!(world.ghost.name, "Pip");
    }

    #[test]
    fn claude_letter_answers_replace_only_the_same_unopened_offline_letter() {
        let original = offline_letter();
        let replacement = vec![
            "Dear you,".to_owned(),
            "The rain kept me company.".to_owned(),
            "Love, Pip".to_owned(),
        ];
        for opened in [false, true] {
            let mut world = World::new(4, 12.0);
            world.life.letters.push(original.clone());
            let mut mind = offline();
            if opened {
                world.read_letter(0);
            }
            mind.apply(
                Answer::LetterWritten {
                    index: 0,
                    original: original.clone(),
                    lines: Ok(Some(replacement.clone())),
                },
                &mut world,
            );
            assert_eq!(world.life.letters[0].claude, !opened);
            assert_eq!(
                world.life.letters[0].lines,
                if opened {
                    original.lines.clone()
                } else {
                    replacement.clone()
                }
            );
            assert!(mind.journal.conversation.is_empty());
        }
        for answer in [
            Err("offline".to_owned()),
            Ok(None),
            Ok(Some(Vec::new())),
            Ok(Some(vec!["".to_owned(); 3])),
            Ok(Some(vec!["has 2 digits".to_owned(); 3])),
        ] {
            let mut world = World::new(4, 12.0);
            world.life.letters.push(original.clone());
            let mut mind = offline();
            mind.apply(
                Answer::LetterWritten {
                    index: 0,
                    original: original.clone(),
                    lines: answer,
                },
                &mut world,
            );
            assert_eq!(world.life.letters[0], original);
        }
        let mut world = World::new(4, 12.0);
        let mut newer = original.clone();
        newer.day = "2026-06-22".to_owned();
        world.life.letters.push(newer.clone());
        offline().apply(
            Answer::LetterWritten {
                index: 0,
                original,
                lines: Ok(Some(replacement)),
            },
            &mut world,
        );
        assert_eq!(world.life.letters[0], newer);
    }

    #[test]
    fn a_letter_waiting_reaches_observations_and_offline_thoughts() {
        let mut world = World::new(1, 12.0);
        world.life.letters.push(offline_letter());
        assert!(
            observation::observe(&world)["letter"]
                .as_str()
                .unwrap()
                .contains("waits unread")
        );
        assert!(
            musing::idle(&world)
                .iter()
                .any(|line| line == "I hope they find my letter.")
        );
        world.read_letter(0);
        assert_eq!(
            observation::observe(&world)["letter"],
            "no unread letter waiting"
        );
        assert!(
            !musing::idle(&world)
                .iter()
                .any(|line| line == "I hope they find my letter.")
        );
    }

    #[test]
    fn claude_writes_a_letter_over_the_network_only_before_the_envelope_is_opened() {
        for (opened, outcome, replying) in [
            (false, "success", false),
            (true, "success", false),
            (false, "refusal", false),
            (false, "empty", false),
            (false, "error", false),
            (false, "success", true),
            (true, "success", true),
            (false, "refusal", true),
            (false, "empty", true),
            (false, "error", true),
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = format!("http://{}", listener.local_addr().unwrap());
            let (sent, received) = mpsc::channel();
            let (release, waiting) = mpsc::channel();
            let lines = vec![
                "Dear you,".to_owned(),
                "The rain kept me company.".to_owned(),
                "Love, Pip".to_owned(),
            ];
            let content = if outcome == "empty" {
                serde_json::json!({"lines": []}).to_string()
            } else {
                serde_json::json!({"lines": lines}).to_string()
            };
            let reply = serde_json::json!({
                "model": "test-model", "stop_reason": if outcome == "refusal" { "refusal" } else { "end_turn" },
                "usage": {"input_tokens": 50, "output_tokens": 30},
                "content": [{"type": "text", "text": content}]
            })
            .to_string();
            let server = std::thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                assert!(first.starts_with("POST /v1/messages "));
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header.trim().is_empty() {
                        break;
                    }
                    if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:")
                    {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                sent.send(serde_json::from_slice::<serde_json::Value>(&body).unwrap())
                    .unwrap();
                waiting.recv_timeout(Duration::from_secs(10)).unwrap();
                let status = if outcome == "error" {
                    "401 Unauthorized"
                } else {
                    "200 OK"
                };
                write!(reader.into_inner(), "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}", reply.len()).unwrap();
            });
            let config = Config {
                anthropic_api_key: Some("letter-test-key".to_owned()),
                claude_base_url: Some(address),
                ..Config::default()
            };
            let mut mind = Mind::new(&config, Journal::default(), false);
            let mut world = World::new(4, 12.0);
            let mut original = offline_letter();
            if replying {
                world.receive_letter("How are you? I am tired.", false);
                original.reply_to = Some(Box::new(world.life.received[0].clone()));
            }
            world.life.letters.push(original.clone());
            world.events.push(crate::world::Event {
                time: world.time,
                kind: EventKind::WroteLetter,
            });
            mind.write_letter(&world);
            let request = received.recv_timeout(Duration::from_secs(10)).unwrap();
            assert_eq!(
                request["output_config"]["format"]["schema"]["required"][0],
                "lines"
            );
            assert_eq!(request["messages"].as_array().unwrap().len(), 1);
            let context: serde_json::Value = serde_json::from_str(
                request["messages"][0]["content"][0]["text"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap();
            if replying {
                assert_eq!(context["viewer_letter"], "How are you? I am tired.");
                assert!(
                    request["system"][0]["text"]
                        .as_str()
                        .unwrap()
                        .contains("reply to the viewer's letter")
                );
            } else {
                assert!(context["viewer_letter"].is_null());
            }
            assert!(context.get("conversation").is_none());
            if opened {
                world.read_letter(0);
            }
            release.send(()).unwrap();
            let answer = mind.receiver.recv_timeout(Duration::from_secs(10)).unwrap();
            mind.apply(answer, &mut world);
            server.join().unwrap();
            assert_eq!(
                world.life.letters[0].lines,
                if opened || outcome != "success" {
                    original.lines
                } else {
                    lines
                }
            );
            assert_eq!(
                world.life.letters[0].claude,
                !opened && outcome == "success"
            );
            assert!(mind.journal.conversation.is_empty());
        }
    }
    #[test]
    fn a_reading_request_preserves_a_story_still_waiting_for_its_words() {
        let config = crate::store::Config::default();
        let mut world = crate::world::World::new(5, 12.0);
        let mut mind = super::Mind::new(&config, super::Journal::default(), true);
        world.hear("tell me a story");
        mind.pending_story = Some(9);
        mind.hear(&mut world, "read to me");
        assert_eq!(mind.pending_story, Some(9));
        assert!(world.telling.as_ref().unwrap().lines.is_empty());
        mind.reflecting_story = Some((9, false));
        mind.apply(super::Answer::Reflected(Ok(None)), &mut world);
        assert!(!world.telling.as_ref().unwrap().lines.is_empty());
    }

    #[test]
    fn creations_reach_questions_observations_reflections_and_named_offline_memories() {
        use crate::world::{
            Nature, Random, Season,
            creation::compose_book,
            music::{Composition, Tune, compose},
        };
        let mut world = World::new(7, 12.0);
        let notes = compose(
            &mut Random::new(9),
            crate::world::Mood::Cheerful,
            Nature::default(),
        );
        world.life.compositions.push(Composition {
            tune: Tune {
                name: "Rain Song".to_owned(),
                notes,
            },
            title: "Rain Song".to_owned(),
            korean_title: "비의 노래".to_owned(),
            day: world.date.to_string(),
        });
        let mut book = compose_book(
            &mut Random::new(4),
            &world.life,
            Season::Summer,
            world.date.to_string(),
        );
        book.pages = std::mem::take(&mut book.outline);
        book.finished = true;
        let title = book.title.0.clone();
        world.life.written.push(book);
        let observation = observation::observe(&world);
        assert_eq!(observation["room"]["own_songs"], "Rain Song");
        assert!(
            observation["room"]["own_books"]
                .as_str()
                .unwrap()
                .contains(&title)
        );
        let request = reflection::request(&observation, &[], &world.life, &[], None, None, None);
        let prompt = request
            .system
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(prompt.contains("Rain Song") && prompt.contains(&title));
        for pool in [musing::idle(&world), musing::dreams(&world)] {
            let named: Vec<_> = pool
                .iter()
                .map(|line| musing::named_friend_line(&world, line))
                .collect();
            assert!(named.iter().any(|line| line.contains("Rain Song")));
            assert!(named.iter().any(|line| line.contains(&title)));
        }
        for (question, expected) in [
            ("what songs do you know?", "Rain Song"),
            ("무슨 노래 알아?", "비의 노래"),
            ("what's your favorite book?", title.as_str()),
        ] {
            let heard = world.hear(question);
            assert!(!routes_to_jev(&heard));
            assert!(
                musing::reply(&world, &heard, None)
                    .iter()
                    .any(|line| line.contains(expected)),
                "{question}"
            );
        }
        let mut mind = offline();
        mind.hear(&mut world, "sing me your song");
        assert!(world.ghost.speech.as_ref().unwrap().text.starts_with('♪'));
        assert!(
            mind.journal
                .conversation
                .last()
                .unwrap()
                .text
                .starts_with('♪')
        );
        mind.hear(&mut world, "read me your book");
        assert!(world.telling.is_some());
        assert!(mind.pending_story.is_none());
    }

    #[test]
    fn music_lessons_and_lullabies_bypass_jev_and_reply_in_the_viewers_language() {
        for (text, reply) in [
            ("do re mi", "I'll play it back!"),
            ("도레미", "따라 쳐 볼게!"),
        ] {
            let mut world = World::new(12, 12.0);
            let heard = world.hear(text);
            assert!(!routes_to_jev(&heard));
            assert!(
                musing::reply(&world, &heard, None)
                    .iter()
                    .any(|line| line == reply)
            );
            let observation = observation::observe(&world);
            assert!(
                observation["room"]["learned_tunes"]
                    .as_str()
                    .unwrap()
                    .contains("do-re-mi")
            );
            assert!(
                musing::dreams(&world)
                    .iter()
                    .any(|line| line.contains("tune"))
            );
        }
        let mut world = World::new(13, 12.0);
        for text in ["play a lullaby", "자장가 들려줘"] {
            assert!(!routes_to_jev(&world.hear(text)));
            assert!(world.music.as_ref().unwrap().lullaby);
        }
    }

    #[test]
    fn music_questions_answer_from_the_box_and_its_known_tunes() {
        use crate::world::Question;
        let mut world = World::new(14, 12.0);
        world.hear("do re mi");
        for (text, question) in [
            ("do you like music?", Question::Music),
            ("음악 좋아해?", Question::Music),
            ("what songs do you know?", Question::Songs),
            ("무슨 노래 알아?", Question::Songs),
        ] {
            let heard = world.hear(text);
            assert!(
                matches!(heard.understanding, Understanding::Asked { question: asked, .. } if asked == question)
            );
            assert!(!routes_to_jev(&heard));
            let replies = musing::reply(&world, &heard, None);
            assert!(!replies.is_empty());
            if question == Question::Songs {
                assert!(replies.iter().any(|line| line.contains("do-re-mi")));
            }
            assert!(world.life.vocabulary.known().is_empty());
        }
        assert_eq!(
            crate::world::dream::DreamPicture::from_line("a tune of golden notes", None),
            crate::world::dream::DreamPicture::Note
        );
    }

    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    #[test]
    fn performed_yawns_do_not_route_to_system_one_or_produce_a_spoken_reply() {
        let mut mind = offline();
        let mut world = World::new(1, 12.0);
        let heard = world.hear("*yawns*");
        assert!(!routes_to_jev(&heard));
        mind.hear(&mut world, "하암");
        assert!(world.ghost.speech.is_none());
        world.step(3.0);
        assert_eq!(
            world.quirk.map(|spell| spell.kind),
            Some(crate::world::Quirk::Yawn)
        );
    }

    #[test]
    fn wardrobe_requests_stay_in_code_and_friend_arrivals_notice_an_outfit() {
        let mut world = World::new(1, 12.0);
        world.wearing = Some(crate::world::Accessory::WitchHat);
        for message in [
            "take off your hat",
            "모자 벗어",
            "what are you wearing?",
            "뭐 입었어?",
            "그거 뭐야?",
        ] {
            assert!(!routes_to_jev(&world.hear(message)));
        }
        let mut mind = Mind::new(&Config::default(), Journal::default(), true);
        world.wearing = Some(crate::world::Accessory::Scarf { color: 0 });
        world.time = 1.0;
        world.think("Already thinking.");
        world.events.push(crate::world::Event {
            time: world.time,
            kind: EventKind::FriendArrived("Pip".into()),
        });
        mind.muse(&mut world);
        assert!(world.ghost.thought.as_ref().unwrap().text.contains("scarf"));
    }

    #[test]
    fn puzzle_requests_and_guesses_bypass_jev_and_keep_the_worlds_reply() {
        let mut mind = offline();
        let mut world = World::new(5, 12.0);
        assert!(!routes_to_jev(&World::new(5, 12.0).hear("word game")));
        mind.hear(&mut world, "word game");
        assert!(world.puzzle_active());
        world.step(30.0);
        let letter = world
            .puzzle
            .as_ref()
            .unwrap()
            .word
            .chars()
            .next()
            .unwrap()
            .to_string();
        let mut same = world.clone();
        assert!(!routes_to_jev(&same.hear(&letter)));
        mind.hear(&mut world, &letter);
        assert!(world.puzzle.as_ref().unwrap().shown() > 0);
        let speech = world.ghost.speech.as_ref().unwrap().text.as_str();
        assert!(speech == "Yes! That fits!" || speech == "You found my word!");
        assert_eq!(mind.journal.conversation.last().unwrap().text, speech);
    }

    #[test]
    fn focus_requests_bypass_jev_and_reply_offline_in_the_viewers_language() {
        let config = Config {
            typesafe_api_key: Some("test-key".to_owned()),
            jev_base_url: Some("http://127.0.0.1:9".to_owned()),
            ..Config::default()
        };
        for (text, reply) in [
            ("focus 40", "Let's do it. I'll be right here."),
            ("40분 집중", "좋아, 옆에 있을게."),
        ] {
            let mut mind = Mind::new(&config, Journal::default(), false);
            let mut world = World::new(3, 12.0);
            assert!(!routes_to_jev(&World::new(3, 12.0).hear(text)));
            mind.hear(&mut world, text);
            assert_eq!(world.ghost.speech.as_ref().unwrap().text, reply);
            assert_eq!(world.focus.as_ref().unwrap().length, 2400);
        }
    }

    #[test]
    fn focus_slows_idle_musings_and_quietly_holds_timer_reflections() {
        let mut mind = offline();
        let mut world = World::new(1, 12.0);
        world.start_focus(25);
        mind.next_musing = 100.0;
        mind.muse(&mut world);
        assert_eq!(mind.next_musing, 300.0);
        world.time = 301.0;
        mind.muse(&mut world);
        assert!(mind.next_musing >= world.time + f64::from(MUSES_EVERY.0 * 3.0));
        let config = Config {
            anthropic_api_key: Some("test-key".to_owned()),
            claude_base_url: Some("http://127.0.0.1:9".to_owned()),
            ..Config::default()
        };
        let mut mind = Mind::new(&config, Journal::default(), false);
        mind.think(&mut world);
        assert!(!mind.reflecting);
        assert!(mind.last_reflection.is_none());
        mind.apply(
            Answer::Reflected(Ok(Some(Reflection {
                say: "Hello!".to_owned(),
                ..Reflection::default()
            }))),
            &mut world,
        );
        assert!(world.ghost.speech.is_none());
        mind.hear(&mut world, "hi");
        mind.think(&mut world);
        assert!(mind.reflecting);
        assert!(mind.reflecting_for_viewer);
    }

    #[test]
    fn feelings_bypass_jev_reply_in_the_viewers_language_and_still_reach_reflection() {
        for text in [
            "I'm tired",
            "I'm Stressed",
            "I'm so happy!",
            "오늘 너무 힘들었어",
            "나는 뿌듯해",
        ] {
            let mut world = World::new(3, 12.0);
            let heard = world.hear(text);
            assert!(!routes_to_jev(&heard));
            let mut world = World::new(3, 12.0);
            let mut mind = offline();
            mind.hear(&mut world, text);
            let reply = &world.ghost.speech.as_ref().unwrap().text;
            assert!(musing::reply(&world, &heard, None).contains(reply));
            assert_eq!(world.life.today.praises, 0);
            assert_eq!(world.life.total.feelings_heard, 1);
            let config = Config {
                anthropic_api_key: Some("test-key".to_owned()),
                ..Config::default()
            };
            let mut mind = Mind::new(&config, Journal::default(), false);
            mind.hear(&mut World::new(3, 12.0), text);
            assert_eq!(mind.heard.front().unwrap().text, text);
        }
    }

    #[test]
    fn feeling_replies_leave_sleepers_quiet_and_reflections_preserve_the_comfort_plan_and_gaze() {
        let mut mind = offline();
        let mut world = World::new(3, 23.0);
        world.tuck_in();
        mind.hear(&mut world, "I'm sad");
        assert!(world.ghost.speech.is_none());
        assert_eq!(world.ghost.asleep, 1.0);
        mind.take_in(
            Reflection {
                say: "I'm here.".to_owned(),
                ..Reflection::default()
            },
            &mut world,
        );
        assert!(world.ghost.speech.is_none());
        let mut world = World::new(3, 12.0);
        mind.hear(&mut world, "I'm stressed");
        let plan = world.ghost.plan.clone();
        mind.take_in(
            Reflection {
                say: "I'm here.".to_owned(),
                write: "HI".to_owned(),
                look_at: "lamp".to_owned(),
                ..Reflection::default()
            },
            &mut world,
        );
        assert_eq!(world.ghost.plan, plan);
        assert!(world.ghost.attention.is_none());
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "I'm here.");
    }

    const DECISION: &str = r#"{"model": "jev-1.13.0", "answers": {
        "activity": {"type": "choice", "choice": "play", "confidence": 0.8,
            "probabilities": {"play": 0.9, "float": 0.1}},
        "mood": {"type": "choice", "choice": "cheerful", "confidence": 0.9,
            "probabilities": {"cheerful": 0.95, "content": 0.05}}},
        "usage": {"input_tokens": 100, "output_tokens": 3}}"#;
    const REQUEST: &str = r#"{"model": "jev-1.13.0", "answers": {
        "request": {"type": "choice", "choice": "play", "confidence": 0.9,
            "probabilities": {"play": 0.95, "none": 0.05}}},
        "usage": {"input_tokens": 40, "output_tokens": 1}}"#;
    const REFLECTION: &str = r#"{"id": "msg_1", "type": "message", "role": "assistant",
        "model": "claude-opus-5-5",
        "content": [{"type": "thinking", "thinking": "", "signature": "x"},
            {"type": "text", "text": "{\"thought\": \"A visitor!\", \"say\": \"Hello there!\", \"goal\": \"play with the ball\", \"mood\": \"cheerful\", \"remember\": \"\", \"name\": \"\", \"look_at\": \"\", \"write\": \"\"}"}],
        "stop_reason": "end_turn", "stop_details": null,
        "usage": {"input_tokens": 900, "output_tokens": 60}}"#;

    /// Each request's path and body, as the mock server saw them.
    type Requests = Arc<Mutex<Vec<(String, String)>>>;

    // A server on a local port that answers both APIs with canned replies
    // and keeps each request's path and body.
    fn serve() -> (String, Requests) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&requests);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                let mut reader = BufReader::new(stream);
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let path = first.split_whitespace().nth(1).unwrap_or("").to_owned();
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    if header.trim().is_empty() {
                        break;
                    }
                    if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:")
                    {
                        length = value.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let body = String::from_utf8(body).unwrap();
                let reply = match path.as_str() {
                    "/v1/systemone" if body.contains("\"request\"") => REQUEST,
                    "/v1/systemone" => DECISION,
                    _ => REFLECTION,
                };
                log.lock().unwrap().push((path, body));
                let mut stream = reader.into_inner();
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                    reply.len()
                );
            }
        });
        (address, requests)
    }

    #[test]
    fn both_minds_answer_over_the_network_while_the_world_goes_on() {
        let (address, requests) = serve();
        let config = Config {
            anthropic_api_key: Some("test-claude-key".into()),
            typesafe_api_key: Some("test-jev-key".into()),
            claude_base_url: Some(address.clone()),
            jev_base_url: Some(address),
            ..Config::default()
        };
        let mut mind = Mind::new(&config, Journal::default(), false);
        let mut world = World::new(5, 12.0);
        world.ghost.plan.done = true;
        mind.think(&mut world);
        assert_eq!(mind.one, Status::Busy);
        mind.hear(&mut world, "please play with the ball");

        let start = Instant::now();
        while world.ghost.speech.is_none() || world.ghost.plan.activity != Activity::Play {
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "{:?} {:?}",
                mind.one,
                mind.two
            );
            std::thread::sleep(Duration::from_millis(20));
            world.step(0.02);
            mind.think(&mut world);
        }
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Hello there!");
        assert_eq!(world.ghost.goal.as_deref(), Some("play with the ball"));
        assert_eq!(mind.journal.conversation.len(), 2);
        assert_eq!(mind.two, Status::Ready);

        let requests = requests.lock().unwrap();
        let reflection = requests
            .iter()
            .find(|(path, _)| path == "/v1/messages")
            .map(|(_, body)| body.as_str())
            .unwrap();
        assert!(reflection.contains("please play with the ball"));
        assert!(reflection.contains("\"fallbacks\":\"default\""));
        assert!(reflection.contains("\"ttl\":\"1h\""));
        assert!(reflection.contains("\"effort\":\"low\""));
        let decisions = requests
            .iter()
            .filter(|(path, _)| path == "/v1/systemone")
            .count();
        assert_eq!(decisions, 2);
    }

    fn offline() -> Mind {
        Mind::new(&Config::default(), Journal::default(), true)
    }

    #[test]
    fn dreams_avoid_lines_it_had_lately_while_others_fit() {
        let mut mind = offline();
        let mut world = World::new(3, 2.0);
        world.ghost.asleep = 1.0;
        let pool = musing::dreams(&world).len();
        let rounds = pool.min(16);
        assert!(rounds >= 3, "{pool}");
        let mut thoughts = Vec::new();
        for _ in 0..rounds {
            world.time += 1.0;
            world.ghost.thought = None;
            mind.dreaming = true;
            mind.next_musing = world.time;
            mind.muse(&mut world);
            thoughts.push(world.ghost.thought.as_ref().unwrap().text.clone());
        }
        let mut distinct = thoughts.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), thoughts.len(), "{thoughts:?}");
    }

    #[test]
    fn with_only_lately_had_musings_to_hand_the_ghost_stays_quiet_until_they_freshen() {
        let mut mind = offline();
        let mut world = World::new(3, 2.0);
        world.ghost.asleep = 1.0;
        let pool = musing::dreams(&world);
        for line in &pool {
            mind.thought_at.insert(line.clone(), world.time);
        }
        world.time += 60.0;
        mind.dreaming = true;
        mind.next_musing = world.time;
        mind.muse(&mut world);
        assert!(world.ghost.thought.is_none());
        // An hour later they are fresh again.
        world.time += THOUGHT_FRESHENS;
        mind.next_musing = world.time;
        mind.muse(&mut world);
        let thought = &world.ghost.thought.as_ref().unwrap().text;
        assert!(
            pool.iter()
                .any(|line| musing::named_friend_line(&world, line) == *thought),
            "{thought}"
        );
        assert_eq!(mind.freshness("never thought", world.time), 1.0);
        let had = &pool[0];
        mind.thought_at.insert(had.clone(), world.time);
        assert_eq!(mind.freshness(had, world.time), 0.0);
        assert!((mind.freshness(had, world.time + THOUGHT_FRESHENS / 2.0) - 0.25).abs() < 1e-6);
    }

    fn waiting_for_story(text: &str) -> (Mind, World) {
        let mut mind = offline();
        mind.claude = Some(Claude::new("test-key", "test-model").unwrap());
        let mut world = World::new(3, 12.0);
        world.notice_viewer();
        mind.hear(&mut world, text);
        mind.reflecting_story = mind.heard.pop_front().unwrap().story;
        mind.reflecting = true;
        (mind, world)
    }

    #[test]
    fn claudes_story_is_told_sentence_by_sentence_and_remembered_once() {
        let (mut mind, mut world) = waiting_for_story("tell me a story");
        let reflection = Reflection {
            say: "A light woke. You came! It had a home…".to_owned(),
            ..Reflection::default()
        };
        mind.apply(Answer::Reflected(Ok(Some(reflection))), &mut world);
        assert_eq!(
            observation::observe(&world)["ghost"]["doing"],
            "telling the viewer a story"
        );
        assert_eq!(
            world.telling.as_ref().unwrap().lines,
            ["A light woke.", "You came!", "It had a home…"]
        );
        assert!(world.ghost.speech.is_none());
        assert_eq!(mind.journal.conversation.len(), 1);
        mind.claude = None;
        for expected in ["A light woke.", "You came!", "It had a home…"] {
            world.step(0.1);
            mind.think(&mut world);
            mind.think(&mut world);
            assert_eq!(mind.journal.conversation.last().unwrap().text, expected);
            let pause = world.telling.as_ref().unwrap().next_at - world.time;
            world.step((pause - 0.05) as f32);
        }
        world.step(0.1);
        mind.think(&mut world);
        assert!(world.telling.is_none());
        assert_eq!(world.life.total.stories, 1);
        assert_eq!(mind.journal.conversation.len(), 4);
    }

    #[test]
    fn failed_declined_and_empty_story_reflections_fall_back_in_the_asked_language() {
        for result in [
            Err("unavailable".to_owned()),
            Ok(None),
            Ok(Some(Reflection::default())),
        ] {
            let (mut mind, mut world) = waiting_for_story("이야기 들려줘");
            mind.apply(Answer::Reflected(result), &mut world);
            let lines = &world.telling.as_ref().unwrap().lines;
            assert!((3..=5).contains(&lines.len()));
            assert!(
                lines
                    .iter()
                    .all(|line| line.chars().any(|letter| ('가'..='힣').contains(&letter)))
            );
            assert!(mind.pending_story.is_none());
        }
    }

    #[test]
    fn late_story_replies_cannot_restart_an_interrupted_or_newer_story() {
        let (mut mind, mut world) = waiting_for_story("tell me a story");
        let reflection = Reflection {
            say: "Old tale. Old light. Old home.".to_owned(),
            ..Reflection::default()
        };
        world.poke();
        mind.apply(Answer::Reflected(Ok(Some(reflection.clone()))), &mut world);
        assert!(world.telling.is_none());
        assert!(world.ghost.speech.is_none());

        let (mut mind, mut world) = waiting_for_story("tell me a story");
        world.poke();
        mind.hear(&mut world, "이야기 들려줘");
        let current = mind.pending_story;
        mind.apply(Answer::Reflected(Ok(Some(reflection))), &mut world);
        assert_eq!(mind.pending_story, current);
        assert!(world.telling.as_ref().unwrap().lines.is_empty());
        mind.reflecting_story = mind.heard.pop_front().unwrap().story;
        mind.apply(Answer::Reflected(Err("unavailable".to_owned())), &mut world);
        assert!(!world.telling.as_ref().unwrap().lines.is_empty());
    }

    #[test]
    fn another_story_request_is_ignored_while_new_messages_are_answered_normally() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        world.notice_viewer();
        mind.hear(&mut world, "tell me a story");
        let telling = world.telling.clone();
        mind.hear(&mut world, "story please");
        assert_eq!(world.telling, telling);
        assert!(mind.heard.is_empty());
        mind.hear(&mut world, "what's your name?");
        assert!(
            world
                .ghost
                .speech
                .as_ref()
                .unwrap()
                .text
                .contains(&world.ghost.name)
        );
        assert_eq!(world.telling, telling);
    }

    #[test]
    fn a_story_queued_for_claude_is_told_offline_once_claude_is_turned_off() {
        let mut mind = offline();
        mind.claude = Some(Claude::new("test-key", "test-model").unwrap());
        let mut world = World::new(3, 12.0);
        world.notice_viewer();
        mind.hear(&mut world, "tell me a story");
        assert!(world.telling.as_ref().unwrap().lines.is_empty());
        mind.claude = None;
        mind.think(&mut world);
        assert!(mind.heard.is_empty());
        assert!((3..=5).contains(&world.telling.as_ref().unwrap().lines.len()));
    }

    #[test]
    fn the_return_line_waits_for_a_story_and_fresh_words_to_finish() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        world.notice_viewer();
        let mut away = crate::world::away::story(
            "2026-06-21T12:00:00".parse().unwrap(),
            "2026-06-21T16:00:00".parse().unwrap(),
            1,
            &world.life,
            None,
        );
        away.happenings = vec![crate::world::away::Happening::Waited];
        world.away = Some(away);
        world.tell(vec!["Once.".to_owned(), "Twice.".to_owned()]);
        world.step(0.1);
        mind.think(&mut world);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Once.");
        world.telling = None;
        world.step(1.0);
        mind.think(&mut world);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Once.");
        assert!(!world.away.as_ref().unwrap().told);
        world.step(SPEECH_SETTLES as f32);
        mind.think(&mut world);
        assert!(world.away.as_ref().unwrap().told);
    }

    #[test]
    fn a_return_story_is_spoken_once_in_the_viewers_language_after_presence() {
        for korean in [false, true] {
            let mut mind = offline();
            let mut world = World::new(3, 12.0);
            world.life.korean = if korean { 1.0 } else { 0.0 };
            let mut away = crate::world::away::story(
                "2026-06-21T12:00:00".parse().unwrap(),
                "2026-06-21T16:00:00".parse().unwrap(),
                1,
                &world.life,
                None,
            );
            away.happenings = vec![crate::world::away::Happening::Waited];
            world.away = Some(away);
            mind.think(&mut world);
            assert!(world.ghost.speech.is_none());
            world.notice_viewer();
            mind.think(&mut world);
            assert!(world.away.as_ref().unwrap().told);
            let said = world.ghost.speech.as_ref().unwrap().text.clone();
            assert_eq!(
                said.chars()
                    .any(|character| ('가'..='힣').contains(&character)),
                korean
            );
            assert_eq!(mind.journal.conversation.len(), 1);
            world.ghost.speech = None;
            world.step(0.1);
            mind.think(&mut world);
            assert!(world.ghost.speech.is_none());
            assert_eq!(mind.journal.conversation.len(), 1);
            mind.hear(
                &mut world,
                if korean {
                    "뭐 하고 있었어?"
                } else {
                    "What did you do while I was away?"
                },
            );
            let said = &world.ghost.speech.as_ref().unwrap().text;
            assert!(if korean {
                said.contains("유리")
            } else {
                said.contains("glass")
            });
            assert!(world.life.vocabulary.words.is_empty());
        }
    }

    #[test]
    fn without_keys_both_minds_are_off_and_the_ghost_lives_on_instinct() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        assert_eq!((&mind.one, &mind.two), (&Status::Off, &Status::Off));
        assert!(!mind.can_talk());
        world.ghost.plan.done = true;
        mind.think(&mut world);
        assert!(!world.needs_decision());
        assert!(mind.runtime.is_none());
    }

    #[test]
    fn questions_and_noes_are_answered_rather_than_sent_to_jev_as_requests() {
        let mut world = World::new(3, 12.0);
        assert!(!routes_to_jev(&world.hear("what are you doing?")));
        assert!(routes_to_jev(&world.hear("dance!")));
        assert!(!routes_to_jev(&world.hear("no")));
    }

    #[test]
    fn a_no_cancels_a_request_still_on_its_way() {
        let config = Config {
            typesafe_api_key: Some("test-key".to_owned()),
            jev_base_url: Some("http://127.0.0.1:9".to_owned()),
            ..Config::default()
        };
        let mut mind = Mind::new(&config, Journal::default(), false);
        let mut world = World::new(3, 12.0);
        mind.hear(&mut world, "dance!");
        let sent = mind.latest_request;
        mind.hear(&mut world, "no");
        world.begin(Activity::Float);
        mind.apply(
            Answer::Requested {
                id: sent,
                activity: Ok(Some(Activity::Dance)),
            },
            &mut world,
        );
        assert_eq!(world.ghost.plan.activity, Activity::Float);
    }

    #[test]
    fn a_reflection_asked_for_before_a_new_name_cannot_rename_the_ghost_back() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        mind.hear(&mut world, "your name is Pip");
        mind.reflection_renamed = mind.renamed;
        mind.hear(&mut world, "your name is Luna");
        assert_eq!(world.ghost.name, "Luna");
        let reflection = Reflection {
            name: "Pip".to_owned(),
            ..Reflection::default()
        };
        mind.apply(Answer::Reflected(Ok(Some(reflection.clone()))), &mut world);
        assert_eq!(world.ghost.name, "Luna");
        mind.reflection_renamed = mind.renamed;
        mind.apply(Answer::Reflected(Ok(Some(reflection))), &mut world);
        assert_eq!(world.ghost.name, "Pip");
    }

    #[test]
    fn a_late_decision_is_ignored() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        world.ghost.plan.done = true;
        mind.deciding = Some((7, 0.0));
        mind.apply(
            Answer::Decided {
                id: 6,
                decision: Ok(Decision {
                    activities: vec![(Activity::Play, 1.0)],
                    mood: None,
                }),
            },
            &mut world,
        );
        assert!(world.needs_decision());
        assert_eq!(mind.deciding, Some((7, 0.0)));
    }

    #[test]
    fn a_decision_starts_an_activity_it_favors() {
        let mut mind = offline();
        let mut plays = 0;
        for seed in 0..40 {
            let mut world = World::new(seed, 12.0);
            world.ghost.plan.done = true;
            mind.deciding = Some((1, 0.0));
            mind.apply(
                Answer::Decided {
                    id: 1,
                    decision: Ok(Decision {
                        activities: vec![(Activity::Play, 1.0)],
                        mood: None,
                    }),
                },
                &mut world,
            );
            assert!(!world.needs_decision());
            plays += usize::from(world.ghost.plan.activity == Activity::Play);
        }
        assert!(plays > 25, "{plays}");
    }

    #[test]
    fn a_reflection_speaks_sets_a_goal_and_remembers_once() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        let reflection = Reflection {
            thought: "What a nice day.".to_owned(),
            say: "Hi Mina!".to_owned(),
            goal: "tend the plant".to_owned(),
            mood: Some(crate::world::Mood::Cheerful),
            remember: "The person is called Mina.".to_owned(),
            name: "Pip".to_owned(),
            look_at: "window".to_owned(),
            write: "HI".to_owned(),
        };
        mind.reflecting = true;
        mind.apply(Answer::Reflected(Ok(Some(reflection.clone()))), &mut world);
        mind.apply(Answer::Reflected(Ok(Some(reflection))), &mut world);
        assert!(!mind.reflecting);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Hi Mina!");
        assert_eq!(world.ghost.goal.as_deref(), Some("tend the plant"));
        assert_eq!(world.ghost.name, "Pip");
        assert_eq!(world.ghost.plan.activity, Activity::Doodle);
        let Some(crate::world::Picture::Word(writing)) = &world.ghost.plan.picture else {
            panic!("expected writing")
        };
        assert_eq!(writing.text, "HI");
        assert_eq!(
            world.ghost.attention.map(|(thing, _)| thing),
            Some(crate::world::Thing::Window)
        );
        assert_eq!(mind.journal.memories.len(), 1);
        assert_eq!(mind.journal.conversation.len(), 2);
        world.step(0.05);
        assert_eq!(world.ghost.mood, crate::world::Mood::Cheerful);
    }

    #[test]
    fn an_answer_to_an_older_message_cannot_undo_a_newer_one() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        mind.latest_request = 2;
        world.begin(Activity::Play);
        mind.apply(
            Answer::Requested {
                id: 1,
                activity: Ok(Some(Activity::Rest)),
            },
            &mut world,
        );
        assert_eq!(world.ghost.plan.activity, Activity::Play);
        mind.apply(
            Answer::Requested {
                id: 2,
                activity: Ok(Some(Activity::Explore)),
            },
            &mut world,
        );
        assert_eq!(world.ghost.plan.activity, Activity::Explore);
    }

    #[test]
    fn a_failed_reflection_shows_in_the_status() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        mind.apply(
            Answer::Reflected(Err("Claude authentication: bad key".into())),
            &mut world,
        );
        assert_eq!(
            mind.two,
            Status::Failed("Claude authentication: bad key".into())
        );
    }

    #[test]
    fn offline_sightings_bring_a_thought_and_focus_keeps_it_quiet() {
        for focusing in [false, true] {
            for kind in crate::world::Passerby::ALL {
                let mut mind = offline();
                let mut world = World::new(7, 12.0);
                world.time = 10.0;
                if focusing {
                    world.start_focus(25);
                }
                world.events.clear();
                world.events.push(crate::world::Event {
                    time: world.time,
                    kind: EventKind::SawPasserby(kind),
                });
                mind.next_musing = f64::INFINITY;
                mind.muse(&mut world);
                if focusing {
                    assert!(world.ghost.thought.is_none());
                } else {
                    assert!(
                        kind.thoughts()
                            .contains(&world.ghost.thought.as_ref().unwrap().text.as_str())
                    );
                }
            }
        }
    }

    #[test]
    fn without_system_two_the_ghost_muses_and_dreams_on_its_own() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        world.ghost.needs.energy = 1.0;
        for _ in 0..(150 * 20) {
            world.step(0.05);
            mind.think(&mut world);
        }
        let mused = world.ghost.thought.clone().unwrap();
        assert!(!mused.text.starts_with('…'), "{}", mused.text);

        world.ghost.needs.energy = 0.2;
        world.begin(Activity::Rest);
        for _ in 0..(60 * 20) {
            world.step(0.05);
            mind.think(&mut world);
        }
        assert!(world.ghost.asleep > 0.9);
        let dream = world.ghost.thought.clone().unwrap();
        assert!(dream.text.starts_with('…'), "{}", dream.text);
        assert!(dream.time > mused.time);
    }

    #[test]
    fn a_dream_musing_starts_a_picture_with_its_thought_but_held_thoughts_defer_it() {
        let mut mind = offline();
        let mut world = World::new(3, 23.0);
        world.tuck_in();
        mind.dreaming = true;
        mind.next_musing = 0.0;
        world.think("Still thinking.");
        mind.muse(&mut world);
        assert!(world.dream.is_none());
        world.ghost.thought = None;
        mind.next_musing = world.time;
        mind.muse(&mut world);
        let dream = world.dream.unwrap();
        let thought = world.ghost.thought.as_ref().unwrap();
        assert_eq!(dream.started, thought.time);
        assert_eq!(
            dream.picture,
            crate::world::dream::DreamPicture::from_line(&thought.text, None)
        );
        world.poke();
        assert!(world.dream.is_none());
        world.ghost.thought = None;
        mind.next_musing = world.time;
        mind.muse(&mut world);
        assert!(world.dream.is_none());
    }

    #[test]
    fn without_system_one_a_repeated_new_word_makes_the_ghost_try_something() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        world.notice_viewer();
        mind.hear(&mut world, "wiggle");
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        mind.hear(&mut world, "wiggle");
        assert!(!matches!(
            world.ghost.plan.activity,
            Activity::Visit | Activity::Float
        ));
        let said = &world.ghost.speech.as_ref().unwrap().text;
        assert!(
            ["Like this?", "Is this “wiggle”?", "Wiggle… this?"].contains(&said.as_str()),
            "{said}"
        );
    }

    #[test]
    fn hearing_the_viewer_is_written_in_the_journal() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        mind.hear(&mut world, "hello");
        let conversation = &mind.journal.conversation;
        assert_eq!(conversation[0].text, "hello");
        assert_eq!(conversation[0].speaker, Speaker::Viewer);
        // Without System Two, it answers a greeting in its own few words.
        assert_eq!(conversation[1].speaker, Speaker::Ghost);
        assert_eq!(
            world.ghost.speech.as_ref().map(|line| &line.text),
            Some(&conversation[1].text)
        );
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
    }
}

#[cfg(test)]
mod acquaintance_tests {
    use super::*;
    use crate::world::acquaintance::{Birthday, Color, Topic};

    #[test]
    fn viewer_answers_stay_in_code_and_keep_the_worlds_reply() {
        for (text, expected) in [
            ("blue", "Blue! I'll remember."),
            ("파랑", "파란색! 기억할게."),
            ("blurble", "Hmm, I'll ask again another day."),
        ] {
            let mut world = World::new(6, 12.0);
            world.asking = Some((Topic::Color, world.time + 180.0));
            let heard = world.hear(text);
            assert!(!routes_to_jev(&heard));
            assert_eq!(
                musing::reply(&world, &heard, None),
                vec![expected.to_owned()]
            );
            let mut world = World::new(6, 12.0);
            world.asking = Some((Topic::Color, world.time + 180.0));
            let mut mind = Mind::new(&Config::default(), Journal::default(), true);
            mind.hear(&mut world, text);
            assert_eq!(world.ghost.speech.as_ref().unwrap().text, expected);
            assert!(mind.heard.is_empty());
        }
    }

    #[test]
    fn viewer_memory_questions_are_answered_in_both_languages() {
        for (text, topic, korean) in [
            ("what's my favorite color?", Topic::Color, false),
            ("내가 좋아하는 색이 뭐야?", Topic::Color, true),
            ("do you know my birthday?", Topic::Birthday, false),
            ("when is my birthday?", Topic::Birthday, false),
            ("내 생일 언제야?", Topic::Birthday, true),
            ("what's my favorite season?", Topic::Season, false),
            ("what's my favorite animal?", Topic::Animal, false),
            ("what's my favorite food?", Topic::Food, false),
        ] {
            let mut world = World::new(6, 12.0);
            let heard = world.hear(text);
            assert!(
                matches!(heard.understanding, Understanding::Asked { question: Question::Viewer(asked), .. } if asked == topic),
                "{text}"
            );
            assert!(!routes_to_jev(&heard));
            let reply = musing::reply(&world, &heard, None);
            assert_eq!(
                reply,
                vec![
                    if korean {
                        "아직 말 안 해 줬어!"
                    } else {
                        "You haven't told me yet!"
                    }
                    .to_owned()
                ]
            );
            world.life.viewer.color = Some(Color::Blue);
            world.life.viewer.birthday = Some(Birthday { month: 3, day: 3 });
            world.life.viewer.season = Some(crate::world::Season::Spring);
            world.life.viewer.animal = Some("cats".to_owned());
            world.life.viewer.food = Some("kimchi".to_owned());
            let reply = musing::reply(&world, &heard, None).join(" ");
            assert!(
                reply.contains(&world.life.viewer.answer(topic, korean).unwrap())
                    || reply.contains("Blue")
                    || reply.contains("Spring")
                    || reply.contains("Cats")
                    || reply.contains("Kimchi")
            );
            assert!(!reply.chars().any(char::is_numeric));
        }
    }

    #[test]
    fn viewer_facts_reach_observations_and_reflections_without_losing_presence() {
        let mut world = World::new(6, 12.0);
        world.life.viewer.color = Some(Color::Blue);
        world.life.viewer.birthday = Some(Birthday { month: 3, day: 3 });
        world.life.viewer_name = Some("Sam".to_owned());
        let observation = observation::observe(&world);
        assert!(observation["viewer"].as_str().is_some());
        let facts = observation["viewer_facts"].as_str().unwrap();
        assert_eq!(
            facts,
            "their favorite color is blue; their birthday is March third"
        );
        let conversation = [Remark {
            at: 0,
            speaker: Speaker::Viewer,
            text: "3월 3일".to_owned(),
        }];
        let request = reflection::request(
            &observation,
            &[],
            &world.life,
            &conversation,
            Some("March 3rd"),
            None,
            None,
        );
        for block in &request.messages[0].content {
            assert!(block.text.contains("my birthday is March third"));
            assert!(!block.text.contains("3월") && !block.text.contains("3rd"));
        }
        assert!(
            request
                .system
                .iter()
                .any(|block| block.text.contains(facts) && block.text.contains("Sam"))
        );
    }

    #[test]
    fn offline_thoughts_and_dreams_recall_the_viewers_animal_and_food_as_written() {
        let mut world = World::new(6, 12.0);
        world.life.viewer.animal = Some("고양이".to_owned());
        world.life.viewer.food = Some("kimchi".to_owned());
        for pool in [musing::idle(&world), musing::dreams(&world)] {
            let lines = pool
                .iter()
                .map(|line| musing::named_friend_line(&world, line))
                .collect::<Vec<_>>()
                .join(" ");
            assert!(lines.contains("고양이"));
            assert!(lines.contains("kimchi"));
            assert!(!lines.contains("{animal}") && !lines.contains("{food}"));
        }
    }

    #[test]
    fn treasure_hints_keep_the_worlds_acknowledgement_and_never_route_to_jev() {
        let mut world = World::new(1, 12.0);
        world.start_hiding().unwrap();
        world.hide_treasure(glam::Vec3::new(-1.8, 0.0, -1.2));
        let heard = world.hear("warmer");
        assert!(!routes_to_jev(&heard));
        let mut mind = Mind::new(&Config::default(), Journal::default(), true);
        mind.hear(&mut world, "따뜻해");
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "따뜻해? 이쪽이구나!"
        );
        assert_eq!(world.ghost.plan.activity, Activity::Seek);
    }
}
