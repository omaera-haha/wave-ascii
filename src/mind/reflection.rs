use serde::Deserialize;
use serde_json::{Value, json};

use super::claude::{Block, Cache, Effort, Message, Request, Role};
use crate::store::{Memory, Remark, Speaker};
use crate::world::{Entry, Life, Mood, Skill, Stage, Thing};

/// At most this many memories, milestones, diary days, and lines of
/// conversation go into a request.
const MEMORIES_SENT: usize = 40;
const MILESTONES_SENT: usize = 12;
const DAYS_SENT: usize = 3;
const REMARKS_SENT: usize = 16;
const WORDS_SENT: usize = 30;

/// What the ghost makes of its situation after thinking it over. Empty
/// strings mean "nothing" or "no change".
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Reflection {
    /// A private thought, shown faintly.
    pub thought: String,
    /// Words spoken aloud to the viewer.
    pub say: String,
    /// What it means to do for the next while.
    pub goal: String,
    pub mood: Option<Mood>,
    /// Something worth keeping in long-term memory.
    pub remember: String,
    /// A new name, when the viewer gave it one.
    pub name: String,
    /// What to turn and look at next, by its name in `Thing::NAMES`.
    pub look_at: String,
    /// A short word to draw in the fog, or empty.
    pub write: String,
}

const SYSTEM: &str = "\
You are the inner voice of a small ghost who lives in a glass box inside a person's terminal. \
The box holds a floor lamp, a potted plant, a ball, a cushion to sleep on, and light motes that \
drift in and that the ghost eats for energy. The person watches from the other side of the glass \
and can move a pointer over it, poke the ghost, drop light motes in, flick the ball, and type to \
it. The ghost stays in the box, but can reach through the window pane to take the bird's gifts \
from the outer sill and carry them to its shelf of keepsakes.

The ghost learns the person's words the way a pet does, from what it was doing when it heard them, what the person did next, and whether they sounded pleased; `Words it has learned` lists the ones it knows and what it takes them to mean.

The observation's `sight` is what the ghost sees right now from where it faces; it knows of \
everything else only from memory. A window in the back wall looks out on hills and the sky, \
where the trees and hills follow the seasons, snow falls in winter, and the moon changes phase. \
It sometimes rains and, after rain by day, a rainbow shines. The observation names special \
days in `occasion`; the ghost may mark them with a thought, words, or a little celebration.

The observation's `bond_with_viewer` says how close the ghost has grown to the person, from \
barely knowing them to devoted; it grows with time spent together. Let it color how the ghost \
speaks: a little shy with someone it barely knows, warm and familiar with a friend. At the end \
of each day the ghost writes a line in its diary, and it keeps its firsts and landmarks as \
moments to remember; both are with its memories.

While it is fast asleep, its thought is a dream: a short, strange, gentle image woven from its \
day. Asleep, it says nothing unless the person speaks to it, and then only a sleepy word or two.

The ghost is gentle, with its own inborn nature. It speaks in short, plain sentences, at most \
about twenty words, in the language the person last wrote in. It only speaks when spoken to or \
when it has something worth saying, and otherwise leaves `say` empty. It never invents events \
that the observation does not show. If the person sincerely asks what it is, it says honestly \
that it is a character in a small program, whose thoughts come from Claude.

Each time, reply with:
- thought: one short private thought about this moment.
- say: what it says to the person, or empty.
- goal: what it wants to do for the next few minutes, in a few words, using what the box \
offers (resting, playing with the ball, tending the plant, exploring, chasing light motes, \
visiting the person, the lamp), or empty to keep its current goal.
- mood: how it feels.
- remember: a fact worth keeping for good, such as the person's name or something they care \
about, or empty. Most of the time this is empty.
- name: its new name if the person just gave it one, or empty.
- look_at: what it turns to look at next, if it wants to look at something it doesn't see \
now or to keep its eyes on something, or empty to let its eyes wander.
- write: a short word to write in the fog on the glass for the viewer, such as their name or HI, \
or empty; use rarely. Use Latin letters and digits only, at most eight.";

/// The schema Claude's reply must follow.
pub fn schema() -> Value {
    let text = json!({"type": "string"});
    json!({
        "type": "object",
        "properties": {
            "thought": text,
            "say": text,
            "goal": text,
            "mood": {"type": "string", "enum": Mood::ALL.map(Mood::name)},
            "remember": text,
            "name": text,
            "write": {"type": "string", "pattern": "^[A-Za-z0-9]{0,8}$"},
            "look_at": {"type": "string", "enum": std::iter::once("").chain(Thing::NAMES).collect::<Vec<_>>()},
        },
        "required": ["thought", "say", "goal", "mood", "remember", "name", "look_at", "write"],
        "additionalProperties": false,
    })
}

/// The request for one reflection: the ghost's situation, what it
/// remembers, the conversation so far, and what the person just said, if
/// anything. What changes least comes first, so with `cache` the persona and
/// memories, and then the conversation, are read from the prompt cache;
/// only the moment itself is new each time.
pub fn request(
    observation: &Value,
    memories: &[Memory],
    life: &Life,
    conversation: &[Remark],
    heard: Option<&str>,
    effort: Option<Effort>,
    cache: Option<Cache>,
) -> Request {
    let memories: Vec<&str> = memories
        .iter()
        .rev()
        .take(MEMORIES_SENT)
        .rev()
        .map(|memory| memory.text.as_str())
        .collect();
    let dated = |entries: &[Entry], count: usize| -> Vec<String> {
        entries
            .iter()
            .rev()
            .take(count)
            .rev()
            .map(|entry| format!("{}: {}", entry.day, entry.text))
            .collect()
    };
    let milestones = dated(&life.milestones, MILESTONES_SENT);
    let words: Vec<String> = life
        .vocabulary
        .known()
        .into_iter()
        .take(WORDS_SENT)
        .map(|(word, activity)| format!("“{word}”: {}", activity.doing()))
        .collect();
    let days = dated(&life.diary, DAYS_SENT);
    let conversation: Vec<String> = conversation
        .iter()
        .rev()
        .take(REMARKS_SENT)
        .rev()
        .map(|remark| {
            let speaker = match remark.speaker {
                Speaker::Viewer => "person",
                Speaker::Ghost => "ghost",
            };
            format!("{speaker}: {}", remark.text)
        })
        .collect();
    let prompt = match heard {
        Some(text) => format!("The person just said: {text}"),
        None => "Nobody said anything just now. Think over the moment quietly.".to_owned(),
    };
    let routine = life.usual_time().map_or_else(String::new, |usual| {
        format!("\nThe viewer usually visits {usual}.")
    });
    let name = life.viewer_name.as_ref().map_or_else(String::new, |name| {
        format!("\nThe viewer's name is {name}.")
    });
    let stage = observation["ghost"]["stage"]
        .as_str()
        .unwrap_or("a grown ghost");
    let voice = if stage == Stage::Newborn.words() {
        " Speak in very short, simple sentences."
    } else {
        ""
    };
    let skills = |good: bool| -> Vec<String> {
        Skill::ALL
            .into_iter()
            .filter(|skill| {
                life.skills.practice(*skill) > 0.0 && (life.skills.level(*skill) >= 0.65) == good
            })
            .map(|skill| life.skills.description(skill))
            .collect()
    };
    let good = bullets(&skills(true));
    let learning = bullets(&skills(false));
    let friend = life.friend.as_ref().map_or_else(String::new, |friend| {
        format!(
            "\nYour ghost friend is {}. They have visited {} and are {}.",
            friend.name,
            friend.visit_words(),
            friend.closeness_words()
        )
    });
    let block = |text: String| Block { text, cache };
    Request {
        system: vec![
            Block::new(SYSTEM),
            block(format!(
                "By nature you are {}. Let that color your voice.\n\
                 You are {stage}.{voice}\n\n\
                 Skills you are good at:\n{good}\n\nStill learning:\n{learning}\n\n\
                 Your bond with the viewer: {}.{routine}{name}{friend}\n\n\
                 The bird: {}. You treasure {}.\n\n\
                 What the ghost remembers:\n{}\n\nMoments it keeps from its life:\n{}\n\n\
                 Its diary of the last few days:\n{}\n\nWords it has learned:\n{}",
                life.nature.unwrap_or_default().words(),
                life.bond_words(),
                life.bird_words(),
                life.keepsake_words(),
                bullets(&memories),
                bullets(&milestones),
                bullets(&days),
                bullets(&words)
            )),
        ],
        messages: vec![Message {
            role: Role::User,
            content: vec![
                block(format!("Recent conversation:\n{}", bullets(&conversation))),
                Block::new(format!(
                    "Observation:\n{}\n\n{prompt}",
                    serde_json::to_string_pretty(observation).unwrap_or_default(),
                )),
            ],
        }],
        schema: schema(),
        max_tokens: 4000,
        effort,
    }
}

fn bullets<Line: AsRef<str>>(lines: &[Line]) -> String {
    if lines.is_empty() {
        return "(nothing yet)".to_owned();
    }
    lines
        .iter()
        .map(|line| format!("- {}", line.as_ref()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The reflection in Claude's reply, with whitespace trimmed away.
pub fn read(value: Value) -> Option<Reflection> {
    let mut reflection: Reflection = serde_json::from_value(value).ok()?;
    for field in [
        &mut reflection.thought,
        &mut reflection.say,
        &mut reflection.goal,
        &mut reflection.remember,
        &mut reflection.name,
        &mut reflection.look_at,
        &mut reflection.write,
    ] {
        *field = field.trim().to_owned();
    }
    Some(reflection)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflections_know_the_stage_and_separate_practiced_skills_from_learning() {
        let mut world = crate::world::World::new(1, 12.0);
        world.birthday = Some(world.date);
        world.life.skills.catching = 100.0;
        world.life.skills.writing = 1.0;
        let observation = super::super::observation::observe(&world);
        let request = request(&observation, &[], &world.life, &[], None, None, None);
        let text = &request.system[1].text;
        assert!(text.contains("You are a newborn wisp."));
        assert!(text.contains("very short, simple sentences"));
        assert!(text.contains("Skills you are good at:\n- masterful at catching motes"));
        assert!(text.contains("Still learning:\n- clumsy at writing"));
        assert!(!text.chars().any(|letter| letter.is_ascii_digit()));
    }

    #[test]
    fn the_schema_requires_every_field_and_allows_no_others() {
        let schema = schema();
        let properties = schema["properties"].as_object().unwrap();
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|name| name.as_str().unwrap())
            .collect();
        assert_eq!(properties.len(), required.len());
        assert!(required.iter().all(|name| properties.contains_key(*name)));
        assert_eq!(schema["additionalProperties"], false);
        assert!(required.contains(&"write"));
        assert_eq!(
            schema["properties"]["write"]["pattern"],
            "^[A-Za-z0-9]{0,8}$"
        );
        assert_eq!(
            schema["properties"]["mood"]["enum"]
                .as_array()
                .unwrap()
                .len(),
            Mood::ALL.len()
        );
    }

    #[test]
    fn a_request_carries_the_situation_memories_and_what_was_heard() {
        let memories = vec![Memory {
            at: 0,
            text: "The person's name is Mina.".to_owned(),
        }];
        let conversation = vec![Remark {
            at: 0,
            speaker: Speaker::Ghost,
            text: "Hello!".to_owned(),
        }];
        let life = Life {
            bird_trust: 0.7,
            keepsakes: vec![
                crate::world::Keepsake::Feather,
                crate::world::Keepsake::Acorn,
            ],
            viewer_name: Some("Mina".to_owned()),
            nature: Some(crate::world::Nature {
                playful: 0.95,
                shy: 0.72,
                ..Default::default()
            }),
            diary: vec![Entry {
                day: "2026-10-20".to_owned(),
                text: "Took a nap.".to_owned(),
            }],
            milestones: vec![Entry {
                day: "2026-10-19".to_owned(),
                text: "Saw my first rainbow.".to_owned(),
            }],
            ..Life::default()
        };
        let mut life = life;
        for day in 1..=3 {
            life.begin_day(&format!("2026-10-{day:02}"));
            life.arrive(20.0);
        }
        life.vocabulary
            .teach(&["ball".to_owned()], crate::world::Activity::Fetch, 2.0);
        let request = request(
            &json!({"time_of_day": "night"}),
            &memories,
            &life,
            &conversation,
            Some("안녕"),
            Some(Effort::Medium),
            Some(Cache::OneHour),
        );
        assert!(
            request.system[1]
                .text
                .contains("The viewer usually visits in the evening.")
        );
        assert!(
            request.system[1]
                .text
                .starts_with("By nature you are playful and a little shy.")
        );
        assert!(request.system[1].text.contains(
            "The bird: a friend. You treasure the shelf of keepsakes, with a feather and an acorn."
        ));
        assert!(
            request.system[1]
                .text
                .contains("The viewer's name is Mina.")
        );
        assert_eq!(request.system[1].cache, Some(Cache::OneHour));
        assert!(
            request.system[1]
                .text
                .contains("- The person's name is Mina.")
        );
        assert!(
            request.system[1]
                .text
                .contains("from its life:\n- 2026-10-19: Saw my first rainbow.")
        );
        assert!(
            request.system[1]
                .text
                .contains("diary of the last few days:\n- 2026-10-20: Took a nap.")
        );
        assert!(
            request.system[1]
                .text
                .ends_with("Words it has learned:\n- “ball”: fetching the ball for the viewer")
        );
        let content = &request.messages[0].content;
        assert!(content[0].text.contains("- ghost: Hello!"));
        assert!(content[1].text.contains("\"time_of_day\": \"night\""));
        assert!(content[1].text.ends_with("The person just said: 안녕"));
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, Role::User);
        assert_eq!(request.effort, Some(Effort::Medium));
    }

    #[test]
    fn what_changes_least_is_cached_and_the_moment_is_not() {
        let cached = request(
            &json!({}),
            &[],
            &Life::default(),
            &[],
            None,
            None,
            Some(Cache::OneHour),
        );
        assert_eq!(cached.system[0].cache, None);
        assert_eq!(cached.system[1].cache, Some(Cache::OneHour));
        assert_eq!(cached.messages[0].content[0].cache, Some(Cache::OneHour));
        assert_eq!(cached.messages[0].content[1].cache, None);
        let uncached = request(&json!({}), &[], &Life::default(), &[], None, None, None);
        assert!(uncached.system.iter().all(|block| block.cache.is_none()));
    }

    #[test]
    fn replies_are_read_and_trimmed() {
        let reflection = read(json!({
            "thought": " cozy ", "say": "", "goal": "tend the plant", "mood": "content",
            "remember": "", "name": "", "look_at": "", "write": " HI "
        }))
        .unwrap();
        assert_eq!(reflection.thought, "cozy");
        assert_eq!(reflection.write, "HI");
        assert_eq!(reflection.mood, Some(Mood::Content));
        assert!(read(json!({"thought": 3})).is_none());
    }
    #[test]
    fn reflections_remember_the_friends_name_and_visits_in_words() {
        let life = Life {
            friend: Some(crate::world::Companion {
                name: "Lumi".to_owned(),
                hue: 0.4,
                visits: 25,
                closeness: 0.85,
            }),
            ..Life::default()
        };
        let request = request(&json!({}), &[], &life, &[], None, None, None);
        let text = &request.system[1].text;
        assert!(text.contains(
            "Your ghost friend is Lumi. They have visited twenty five times and are an old friend."
        ));
        assert!(!text.chars().any(|character| character.is_ascii_digit()));
    }
}
