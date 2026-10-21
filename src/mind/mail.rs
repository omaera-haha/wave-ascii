use serde::Deserialize;
use serde_json::json;

use super::claude::{Block, Cache, Effort, Message, Request, Role};
use crate::store::Memory;
use crate::world::{World, mail::Letter};

#[derive(Deserialize)]
pub struct Written {
    pub lines: Vec<String>,
}

pub fn request(
    world: &World,
    letter: &Letter,
    memories: &[Memory],
    effort: Option<Effort>,
    cache: Option<Cache>,
) -> Request {
    let mut persona = Block::new(format!(
        "You are {}, a small ghost living in a glass room. Write a letter you left for the viewer while they were away. By nature you are {}. Your bond: {}. You are {}. Use {}. Return only JSON with lines. Write three to six short lines, each at most forty display columns where possible and never over eighty. Use words, never digits. Start with the supplied salutation and end with the supplied signature. Use only the supplied happenings and memories, no invented events. Include a warm personal line. A newborn writes just two very simple babbling lines, ending with its signature. This letter is separate from conversation; do not answer any past messages.",
        world.ghost.name,
        world.life.nature.unwrap_or_default().words(),
        world.life.bond_words(),
        world.stage().words(),
        if letter.korean { "Korean" } else { "English" }
    ));
    persona.cache = cache;
    let days: Vec<&str> = world
        .away
        .as_ref()
        .map(|away| {
            away.days
                .iter()
                .filter_map(|day| {
                    world
                        .life
                        .diary
                        .iter()
                        .find(|entry| entry.day == day.date.to_string())
                        .map(|entry| entry.text.as_str())
                })
                .collect()
        })
        .unwrap_or_default();
    let happenings: Vec<&str> = world
        .away
        .as_ref()
        .map(|away| {
            away.ranked()
                .into_iter()
                .take(3)
                .map(|happening| happening.lines(letter.korean)[0])
                .collect()
        })
        .unwrap_or_default();
    let memories: Vec<&str> = memories
        .iter()
        .rev()
        .take(40)
        .map(|memory| memory.text.as_str())
        .collect();
    Request {
        system: vec![persona],
        messages: vec![Message::text(
            Role::User,
            json!({
                "offline_letter": letter.lines,
                "away_happenings": happenings,
                "diary_days_while_away": days,
                "memories": memories,
                "wish": world.wish_observation(),
                "shelf": world.life.keepsake_words(),
                "books_read": world.life.books.titles(true, letter.korean),
            })
            .to_string(),
        )],
        schema: json!({
            "type": "object",
            // Structured outputs reject item counts other than zero or one,
            // so the prompt asks for the length and `Mind::apply` checks it.
            "properties": {"lines": {"type": "array", "items": {"type": "string"}}},
            "required": ["lines"],
            "additionalProperties": false,
        }),
        max_tokens: 700,
        effort,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, away};

    #[test]
    fn a_letter_request_uses_absence_diary_memories_and_language_without_conversation() {
        let mut world = World::new(1, 12.0);
        world.life.korean = 1.0;
        let left = "2026-06-18T12:00:00".parse().unwrap();
        let now = "2026-06-21T12:00:00".parse().unwrap();
        let story = away::story(left, now, 1, &Life::default(), None);
        story.write_diary(&mut world.life, left.date(), now.date());
        world.away = Some(story);
        world.write_return_letter(86400.0);
        let letter = &world.life.letters[0];
        let request = request(
            &world,
            letter,
            &[Memory {
                at: 0,
                text: "Sam likes flowers.".to_owned(),
            }],
            Some(Effort::Low),
            Some(Cache::OneHour),
        );
        assert!(request.system[0].text.contains("Korean"));
        assert!(
            crate::mind::reflection::tests::supported_by_structured_outputs(&request.schema),
            "{}",
            request.schema
        );
        assert_eq!(request.messages.len(), 1);
        let context: serde_json::Value =
            serde_json::from_str(&request.messages[0].content[0].text).unwrap();
        assert!(!context["away_happenings"].as_array().unwrap().is_empty());
        assert!(
            !context["diary_days_while_away"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(context["memories"][0], "Sam likes flowers.");
        assert!(context.get("conversation").is_none());
        assert_eq!(request.effort, Some(Effort::Low));
        assert_eq!(request.system[0].cache, Some(Cache::OneHour));
    }
}
