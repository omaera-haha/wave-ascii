#[allow(dead_code, reason = "a general client, of which the ghost uses a part")]
pub mod claude;
mod decision;
mod instinct;
#[allow(dead_code, reason = "a general client, of which the ghost uses a part")]
pub mod jev;
mod observation;
mod reflection;

use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::runtime::Runtime;

use crate::store::{Config, Memory, Remark, Speaker};
use crate::world::{Activity, World};
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
    /// Messages from the viewer still waiting for a reply.
    heard: VecDeque<String>,
    /// The id of the latest message's request routing; answers to earlier
    /// messages that arrive later are dropped.
    latest_request: u64,
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
            heard: VecDeque::new(),
            latest_request: 0,
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

    /// One frame of thinking: applies whatever the minds have answered, and
    /// asks them again when it is time.
    pub fn think(&mut self, world: &mut World) {
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
            if due || !self.heard.is_empty() {
                let heard = self.heard.pop_front();
                let request = reflection::request(
                    &observation::observe(world),
                    &self.journal.memories,
                    &self.journal.conversation,
                    heard.as_deref(),
                    self.effort,
                    self.cache,
                );
                self.reflecting = true;
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
        world.hear(text);
        self.journal.conversation.push(Remark {
            at: now(),
            speaker: Speaker::Viewer,
            text: text.to_owned(),
        });
        trim(&mut self.journal.conversation, REMARKS_KEPT);
        if self.claude.is_some() {
            self.heard.push_back(text.to_owned());
        }
        if let Some(jev) = &self.jev {
            let jev = jev.clone();
            let questions = decision::request_questions(&Activity::ALL);
            let state = serde_json::Value::String(text.to_owned());
            self.next_id += 1;
            let id = self.next_id;
            self.latest_request = id;
            self.spawn(async move {
                let activity = match jev.ask(&state, &questions).await {
                    Ok(answers) => Ok(decision::read_request(&answers)),
                    Err(error) => Err(error.to_string()),
                };
                Answer::Requested { id, activity }
            });
        }
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
                if world.options().contains(&activity) {
                    world.begin(activity);
                }
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
                match result {
                    Ok(reflection) => {
                        self.two = Status::Ready;
                        if let Some(reflection) = reflection {
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
        if !reflection.say.is_empty() {
            world.say(&reflection.say);
            self.journal.conversation.push(Remark {
                at: now(),
                speaker: Speaker::Ghost,
                text: reflection.say.clone(),
            });
            trim(&mut self.journal.conversation, REMARKS_KEPT);
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
        let name: String = reflection.name.chars().take(24).collect();
        if !name.is_empty() {
            world.ghost.name = name;
        }
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
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

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
            {"type": "text", "text": "{\"thought\": \"A visitor!\", \"say\": \"Hello there!\", \"goal\": \"play with the ball\", \"mood\": \"cheerful\", \"remember\": \"\", \"name\": \"\"}"}],
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
        };
        mind.reflecting = true;
        mind.apply(Answer::Reflected(Ok(Some(reflection.clone()))), &mut world);
        mind.apply(Answer::Reflected(Ok(Some(reflection))), &mut world);
        assert!(!mind.reflecting);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Hi Mina!");
        assert_eq!(world.ghost.goal.as_deref(), Some("tend the plant"));
        assert_eq!(world.ghost.name, "Pip");
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
    fn hearing_the_viewer_is_written_in_the_journal() {
        let mut mind = offline();
        let mut world = World::new(3, 12.0);
        mind.hear(&mut world, "hello");
        assert_eq!(mind.journal.conversation.last().unwrap().text, "hello");
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
    }
}
