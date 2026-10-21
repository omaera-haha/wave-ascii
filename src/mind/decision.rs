use super::jev::{Answers, Choice, Questions};
use crate::world::{Activity, Mood};

/// Below this confidence, Jev's reading of a mood or a request is left
/// unused.
const CONFIDENT: f64 = 0.5;

/// A quick read of the moment: the chances of each option being the right
/// thing to do next, and how the ghost feels if that is clear.
#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    pub activities: Vec<(Activity, f32)>,
    pub mood: Option<Mood>,
}

/// Questions for what to do next among `options`, each with what doing it
/// would mean now, and for the mood.
pub fn questions(options: &[(Activity, String)]) -> Questions {
    Questions::new()
        .with(
            "activity",
            Choice::new(
                "Which should the ghost do next? Weigh its needs, its goal, what it loves \
                 doing, how close it feels to the viewer, the time of day, and what is \
                 happening around it.",
                options
                    .iter()
                    .map(|(activity, description)| (activity.name(), description.clone())),
            ),
        )
        .with(
            "mood",
            Choice::new(
                "Which word best describes how the ghost feels right now?",
                Mood::ALL
                    .iter()
                    .map(|mood| (mood.name(), mood.description())),
            ),
        )
}

/// The decision in Jev's answers to `questions`.
pub fn read(answers: &Answers) -> Option<Decision> {
    let activity = answers.choice("activity")?;
    let activities = activity
        .probabilities
        .iter()
        .filter_map(|(name, chance)| Some((Activity::parse(name)?, *chance as f32)))
        .collect::<Vec<_>>();
    if activities.is_empty() {
        return None;
    }
    let mood = answers
        .choice("mood")
        .filter(|answer| answer.confidence >= CONFIDENT)
        .and_then(|answer| Mood::parse(&answer.choice));
    Some(Decision { activities, mood })
}

/// A question for whether the viewer's message, given as the state, asks
/// the ghost to do one of `options`.
pub fn request_questions(options: &[Activity]) -> Questions {
    let mut criteria = vec![(
        "none",
        "The message asks for nothing the ghost can do, or is just talk.",
    )];
    criteria.extend(
        options
            .iter()
            .map(|activity| (activity.name(), request(*activity))),
    );
    Questions::new().with(
        "request",
        Choice::new(
            "The state is a message the viewer typed to a small ghost living in a glass box. \
             Does it ask the ghost to do one of these things?",
            criteria,
        ),
    )
}

/// The activity the viewer asked for, if Jev is sure enough.
pub fn read_request(answers: &Answers) -> Option<Activity> {
    answers
        .choice("request")
        .filter(|answer| answer.confidence >= CONFIDENT)
        .and_then(|answer| Activity::parse(&answer.choice))
}

fn request(activity: Activity) -> &'static str {
    match activity {
        Activity::Float => "Asks it to stay still, calm down, or float in place.",
        Activity::Explore => "Asks it to look around or explore the room.",
        Activity::Chase => "Asks it to catch or eat a light mote.",
        Activity::Rest => "Asks it to sleep, rest, or go to bed.",
        Activity::Play => "Asks it to play, or to push or kick the ball.",
        Activity::Tend => "Asks it to water or look after the plant.",
        Activity::Lamp => "Asks it to switch the lamp on or off.",
        Activity::Visit => "Asks it to come closer, come here, or come to the glass.",
        Activity::Wait => "Asks it to wait for them or stay by the glass until they come back.",
        Activity::Window => {
            "Asks it to look out of the window, or at the sky, the rain, or a rainbow."
        }
        Activity::Fetch => "Asks it to fetch, catch, or bring back the ball.",
        Activity::Pounce => {
            "Asks it to chase or catch the viewer's finger, cursor, or pointer on the glass."
        }
        Activity::Dance => "Asks it to dance, spin, twirl, or do a trick.",
        Activity::Hide => "Asks it to hide, or to play hide-and-seek.",
        Activity::Game => {
            "Asks it to play noughts and crosses, tic-tac-toe, or a game on the glass."
        }
        Activity::Doodle => "Asks it to draw, doodle, or write something on the glass.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn answers(value: serde_json::Value) -> Answers {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn options_go_to_jev_in_the_order_given() {
        let options = vec![
            (Activity::Visit, "Go to the glass.".to_owned()),
            (Activity::Chase, "Catch a mote.".to_owned()),
        ];
        let body = serde_json::to_string(&questions(&options)).unwrap();
        let visit = body.find("\"visit\"").unwrap();
        let chase = body.find("\"chase\"").unwrap();
        assert!(visit < chase, "{body}");
    }

    #[test]
    fn a_decision_reads_chances_and_a_confident_mood() {
        let decided = read(&answers(json!({
            "model": "jev-1.13.0",
            "answers": {
                "activity": {"type": "choice", "choice": "play", "confidence": 0.4,
                    "probabilities": {"play": 0.6, "rest": 0.3, "unknown": 0.1}},
                "mood": {"type": "choice", "choice": "bored", "confidence": 0.8,
                    "probabilities": {"bored": 0.9, "content": 0.1}}
            },
            "usage": {"input_tokens": 300, "output_tokens": 4}
        })))
        .unwrap();
        assert_eq!(
            decided.activities,
            vec![(Activity::Play, 0.6), (Activity::Rest, 0.3)]
        );
        assert_eq!(decided.mood, Some(Mood::Bored));
    }

    #[test]
    fn an_unsure_mood_is_left_out() {
        let decided = read(&answers(json!({
            "model": "jev-1.13.0",
            "answers": {
                "activity": {"type": "choice", "choice": "rest", "confidence": 0.9,
                    "probabilities": {"rest": 1.0}},
                "mood": {"type": "choice", "choice": "grumpy", "confidence": 0.2,
                    "probabilities": {"grumpy": 0.3, "content": 0.25}}
            },
            "usage": {"input_tokens": 300, "output_tokens": 4}
        })))
        .unwrap();
        assert_eq!(decided.mood, None);
    }

    #[test]
    fn a_clear_request_names_an_activity_and_talk_names_none() {
        let reply = |choice: &str, confidence: f64| {
            answers(json!({
                "model": "jev-1.13.0",
                "answers": {"request": {"type": "choice", "choice": choice,
                    "confidence": confidence, "probabilities": {choice: 0.9}}},
                "usage": {"input_tokens": 80, "output_tokens": 2}
            }))
        };
        assert_eq!(read_request(&reply("play", 0.8)), Some(Activity::Play));
        assert_eq!(read_request(&reply("fetch", 0.8)), Some(Activity::Fetch));
        assert_eq!(read_request(&reply("dance", 0.8)), Some(Activity::Dance));
        assert_eq!(read_request(&reply("none", 0.9)), None);
        assert_eq!(read_request(&reply("play", 0.3)), None);
        let body = serde_json::to_string(&request_questions(&[Activity::Rest])).unwrap();
        assert!(body.find("\"none\"").unwrap() < body.find("\"rest\"").unwrap());
    }

    #[test]
    fn every_request_keeps_the_activity_order() {
        let body = serde_json::to_string(&request_questions(&Activity::ALL)).unwrap();
        let mut previous = body.find("\"none\"").unwrap();
        for activity in Activity::ALL {
            let position = body.find(&format!("\"{}\"", activity.name())).unwrap();
            assert!(position > previous, "{body}");
            previous = position;
        }
    }

    #[test]
    fn requests_can_ask_for_hiding_and_drawing_on_the_glass() {
        let body =
            serde_json::to_string(&request_questions(&[Activity::Hide, Activity::Doodle])).unwrap();
        assert!(body.contains("Asks it to hide, or to play hide-and-seek."));
        assert!(body.contains("Asks it to draw, doodle, or write something on the glass."));
        assert!(body.find("\"hide\"").unwrap() < body.find("\"doodle\"").unwrap());
        for activity in [Activity::Hide, Activity::Doodle] {
            let answer = answers(json!({
                "model": "jev-1.13.0",
                "answers": {"request": {"type": "choice", "choice": activity.name(),
                    "confidence": 0.9, "probabilities": {activity.name(): 1.0}}},
                "usage": {"input_tokens": 80, "output_tokens": 2}
            }));
            assert_eq!(read_request(&answer), Some(activity));
        }
    }
}
