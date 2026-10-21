use crate::world::{Activity, Heard, Mood, Occasion, Sky, Tone, Understanding, World};

// Thoughts the ghost has on its own while it has no System Two to think
// with: a few words that fit the moment, so a ghost without keys still has
// an inner life. Each function gives every line that fits; the caller picks
// one.

/// What crosses its mind as it sets out to do `activity`.
pub fn setting_out(world: &World, activity: Activity) -> Vec<&'static str> {
    let present = world.viewer_present();
    match activity {
        Activity::Float => Vec::new(),
        Activity::Explore => vec![
            "I wonder what's in that corner.",
            "Let's have a look around.",
            "Did something move over there?",
        ],
        Activity::Chase => vec![
            "A light mote! Mine.",
            "That one looks tasty.",
            "Come here, little light.",
        ],
        Activity::Rest if world.bedtime() => {
            vec!["Time for bed.", "Goodnight, little room."]
        }
        Activity::Rest => vec![
            "Time for a little nap.",
            "My cushion is calling.",
            "Just resting my eyes…",
        ],
        Activity::Play => vec![
            "Ball time!",
            "Where shall I push it this time?",
            "Watch this.",
        ],
        Activity::Tend if world.plant.thirst > 0.7 => {
            vec![
                "Oh no, the plant is drooping.",
                "Hold on, plant, I'm coming!",
            ]
        }
        Activity::Tend => vec!["A little water for you, plant.", "Grow, little plant."],
        Activity::Lamp if world.lamp.on && world.is_night() => vec!["Lights out."],
        Activity::Lamp if world.lamp.on => vec!["We don't need the lamp by day."],
        Activity::Lamp => vec!["It's getting dark in here.", "Let there be light."],
        Activity::Visit if present && world.life.bond >= 0.3 => {
            vec!["There you are!", "I'll come say hi.", "Hello, you."]
        }
        Activity::Visit if present => vec!["Someone's at the glass.", "Who's there?"],
        Activity::Visit => vec!["Is anyone out there?", "Maybe someone will come by."],
        Activity::Fetch => vec!["I'll get it!", "Wait for me, ball!"],
        Activity::Pounce => vec![
            "What's that? I'll get it!",
            "Your finger! I'm coming.",
            "Gotcha… almost.",
        ],
        Activity::Hide => vec!["Ready or not…", "Bet you can't find me."],
        Activity::Doodle if world.life.bond >= 0.3 => {
            vec!["I'll draw you something.", "This one's for you."]
        }
        Activity::Doodle => vec!["Let me draw something.", "Hm, what shall I draw?"],
        Activity::Dance => vec!["I feel like dancing.", "Wheee!"],
        Activity::Window if world.morning() => morning(world),
        Activity::Window
            if world
                .visitors
                .bird
                .as_ref()
                .is_some_and(|bird| bird.perched(world.time)) =>
        {
            vec!["Hello, little bird.", "Don't fly away…"]
        }
        Activity::Window => match world.weather.sky {
            Sky::Rainbow => vec!["Look, a rainbow!", "All the colors at once."],
            Sky::Rain if world.weather.storm => vec!["Whoa, look at that!"],
            Sky::Rain => vec!["Listen to the rain.", "Pitter-patter."],
            Sky::Snow => vec!["It's snowing!", "Every flake is different, they say."],
            Sky::Clear if world.weather.snow_cover > 0.5 => {
                vec!["The hills are all white.", "I wish I could touch the snow."]
            }
            Sky::Clear if world.is_night() => {
                vec!["So many stars tonight.", "Where does the moon go by day?"]
            }
            Sky::Clear => vec!["I like watching the clouds.", "The hills look soft today."],
        },
    }
}

/// What it muses about while nothing in particular happens.
pub fn idle(world: &World) -> Vec<&'static str> {
    let ghost = &world.ghost;
    let needs = ghost.needs;
    let present = world.viewer_present();
    let mut lines = Vec::new();
    if let Some(nature) = world.life.nature {
        if nature.shy >= 0.7 && present && world.life.bond < 0.3 {
            lines.push("Is it okay if I watch you?");
        }
        if nature.curious >= 0.7 {
            lines.push("What's behind the window, I wonder?");
        }
        if nature.playful >= 0.7 {
            lines.push("Someone throw the ball!");
        }
        if nature.sleepy >= 0.7 {
            lines.push("Yawn… perhaps a little lie down.");
        }
        if nature.tender >= 0.7 {
            lines.push("I hope the plant is happy.");
        }
    }
    if world.morning() {
        lines.extend(morning(world));
    }
    if world.petting() >= 0.35 {
        lines.extend(["Mmm… right there.", "That's nice…", "Don't stop."]);
    }
    if world.bedtime() && present {
        lines.extend(["I should be asleep…", "Just a little longer."]);
    }
    if world
        .visitors
        .moth
        .as_ref()
        .is_some_and(|moth| moth.noticed)
    {
        lines.extend(["A moth! It loves the lamp.", "Fly carefully, little moth."]);
    }
    match world.occasion() {
        Some(Occasion::Halloween) => lines.extend([
            "Boo! Did I scare you?",
            "The pumpkin is grinning at me.",
            "A ghost is very good at Halloween.",
        ]),
        Some(Occasion::WinterHolidays) => lines.extend([
            "The little tree is so pretty.",
            "A tiny holiday in a tiny room.",
        ]),
        Some(Occasion::NewYear) => {
            lines.extend(["Happy new year!", "I wonder what this year will bring."])
        }
        Some(Occasion::Valentine) => lines.extend([
            "A heart on the glass, just for you.",
            "Even the light motes look fond today.",
        ]),
        Some(Occasion::Birthday) => lines.extend([
            "It's my birthday!",
            "Another year in my glass box.",
            "I made a wish. It's a secret.",
        ]),
        None => {}
    }
    if needs.energy < 0.3 {
        lines.extend(["I'm getting sleepy.", "Running low on light."]);
    }
    if needs.fun < 0.3 {
        lines.extend(["Nothing ever happens in here.", "So bored…"]);
    }
    if needs.company < 0.3 && !present {
        lines.extend([
            "It's quiet without anyone around.",
            "I wonder when they'll be back.",
        ]);
    }
    if present && world.life.bond >= 0.55 {
        lines.extend(["I'm glad you're here.", "You always come back."]);
    } else if present && world.life.bond < 0.12 {
        lines.extend(["Who is that, out there?", "They're looking at me."]);
    }
    if world.viewer.pointer_is_lively() && needs.energy > 0.25 {
        lines.push("Your finger keeps getting away!");
    }
    if world.plant.thirst > 0.7 {
        lines.push("The plant looks thirsty.");
    }
    if world.plant.blooming() {
        lines.push("The flowers smell nice.");
    }
    match world.weather.sky {
        Sky::Rain if world.weather.storm => {
            lines.extend(["I don't like thunder.", "Is it over yet?"]);
            if present && world.life.bond >= 0.3 {
                lines.push("I'm glad you're here when it thunders.");
            }
        }
        Sky::Rain => lines.extend(["The rain sounds cozy.", "Drip, drop."]),
        Sky::Rainbow => lines.push("That rainbow is still there!"),
        Sky::Snow => lines.extend(["Snow, snow, snow.", "It looks cold out there."]),
        Sky::Clear => {}
    }
    if world.is_night() && !world.lamp.on {
        lines.push("It's so dark.");
    }
    if matches!(ghost.mood, Mood::Cheerful | Mood::Excited) {
        lines.extend(["What a nice day.", "La la la."]);
    }
    if let Some(favorite) = world.life.favorite() {
        lines.push(match favorite {
            Activity::Play => "I could play with that ball all day.",
            Activity::Chase => "I hope another mote drifts in soon.",
            Activity::Tend => "My plant is the best plant.",
            Activity::Window => "I wonder what's past those hills.",
            Activity::Visit => "I like it at the glass.",
            Activity::Rest => "Naps are the best part of the day.",
            Activity::Explore => "I know every corner of this room.",
            Activity::Lamp => "I like the lamp's warm light.",
            Activity::Float => "Floating is nice.",
            Activity::Fetch => "Throw the ball again! Please?",
            Activity::Pounce => "Move your finger again!",
            Activity::Dance => "I've been working on a new twirl.",
            Activity::Hide => "Nobody hides like a ghost.",
            Activity::Doodle => "I should draw on the glass again.",
        });
    }
    if lines.is_empty() {
        lines.extend([
            "Hm.",
            "A ghost in a glass box. That's me.",
            "I wonder what the viewer is doing.",
            "The glass is cool.",
        ]);
    }
    lines
}

fn morning(world: &World) -> Vec<&'static str> {
    let mut lines = vec!["Good morning!", "The sun's up.", "What a night."];
    if let Some(thought) = &world.ghost.thought
        && thought.text.starts_with('…')
    {
        if thought.text.contains("ball") {
            lines.push("That ball rolled all the way to the hills.");
        } else if thought.text.contains("mote") {
            lines.push("There were so many light motes.");
        } else if thought.text.contains("plant") {
            lines.push("The plant was as tall as the room.");
        } else if thought.text.contains("rainbow") {
            lines.push("A rainbow makes a very good slide.");
        } else if thought.text.contains("glass") {
            lines.push("For a moment, there was no glass.");
        }
    }
    lines
}

/// What it dreams of while it sleeps, mostly of what happened today.
pub fn dreams(world: &World) -> Vec<&'static str> {
    let today = &world.life.today;
    let mut lines = Vec::new();
    match world.occasion() {
        Some(Occasion::Halloween) => lines.extend([
            "…a pumpkin so big I can hide inside its grin…",
            "…all the pumpkins shout boo, and I laugh…",
        ]),
        Some(Occasion::WinterHolidays) => lines.extend([
            "…the little tree grows stars instead of needles…",
            "…a present wrapped in moonlight…",
        ]),
        Some(Occasion::NewYear) => lines.extend([
            "…fireworks turn into light motes I can catch…",
            "…a new year opens like a window…",
        ]),
        Some(Occasion::Valentine) => lines.extend([
            "…pink lights leaving hearts wherever they drift…",
            "…a heart drawn on the moon…",
        ]),
        Some(Occasion::Birthday) => lines.extend([
            "…a cake as big as the room, and a wish for every candle…",
            "…everyone sings, even the plant…",
        ]),
        None => {}
    }
    if today.motes > 0 {
        lines.extend([
            "…a whole sky of light motes, and all of them mine…",
            "…chasing a mote that keeps getting bigger…",
        ]);
    }
    if today.birds > 0 {
        lines.push("…a little bird, singing…");
    }
    if today.nudges > 0 || today.fetches > 0 {
        lines.push("…the ball rolls on and on, out into the hills…");
    }
    if today.doodles > 0 {
        lines.push("…drawing pictures in the clouds…");
    }
    if today.finds > 0 {
        lines.push("…hiding somewhere nobody will ever look…");
    }
    if today.dances > 0 {
        lines.push("…dancing with the light motes, round and round…");
    }
    if today.pounces > 0 {
        lines.push("…a finger of light on the glass, always just out of reach…");
    }
    if today.pets > 0 {
        lines.push("…a warm hand, stroking softly, and the glass is gone…");
    }
    if !today.words.is_empty() {
        lines.push("…your words drift by like light motes, and I catch one…");
    }
    if today.waterings > 0 {
        lines.push("…the plant grows right up through the ceiling…");
    }
    if today.snowfalls > 0 {
        lines.push("…the snow falls inside the glass, soft as light…");
    }
    if today.storms > 0 {
        lines.push("…thunder far away, like a drum…");
    }
    if today.rainbows > 0 {
        lines.push("…sliding down a rainbow…");
    } else if today.showers > 0 {
        lines.push("…rain, but every drop is a little light…");
    }
    if today.heard > 0 || today.together > 600.0 {
        lines.push("…someone at the glass, waving…");
    }
    if lines.is_empty() {
        lines.extend([
            "…floating, floating…",
            "…a room with no glass at all…",
            "…zzz…",
        ]);
    }
    lines
}

/// What it says back when the viewer talks to it without System Two: a
/// few words about what it made of their message. It names the word it
/// understood or wonders about, so the viewer can see it learning.
pub fn reply(world: &World, heard: &Heard, guessed: Option<Activity>) -> Vec<String> {
    let lines = |items: &[&str]| items.iter().map(|line| (*line).to_owned()).collect();
    if let Some((word, _)) = heard.learned.last() {
        return vec![
            format!("So “{word}” means this!"),
            format!("Oh! “{word}”! I get it now."),
        ];
    }
    if guessed.is_some()
        && let Understanding::Puzzled { word, .. } = &heard.understanding
    {
        return vec![
            "Like this?".to_owned(),
            format!("Is this “{word}”?"),
            format!("{}… this?", capitalized(word)),
        ];
    }
    match &heard.understanding {
        Understanding::Doing { word, .. } => {
            let word = capitalized(word);
            return vec![
                format!("{word}? Okay!"),
                format!("{word}! I know that one."),
                format!("{word}! Watch me."),
            ];
        }
        Understanding::Unable { activity, .. } => return unable(world, *activity),
        _ => {}
    }
    let bond = world.life.bond;
    match heard.tone {
        Some(Tone::Scolding) if heard.stopped => return lines(&["Oh… okay, I'll stop.", "Sorry…"]),
        Some(Tone::Scolding) => return lines(&["Oh… sorry.", "Okay, okay…"]),
        Some(Tone::Praise) if bond < 0.12 => return lines(&["Oh! Thank you.", "Hehe."]),
        Some(Tone::Praise) => return lines(&["Hehe!", "Yay!", "You think so?"]),
        _ => {}
    }
    if let Understanding::Puzzled { word, again } = &heard.understanding {
        return if *again {
            vec![
                format!("“{word}” again… what does it mean?"),
                format!("Hm, “{word}”… show me?"),
            ]
        } else {
            vec![
                format!("“{word}”?"),
                format!("What's “{word}”?"),
                format!("Hm? {}?", capitalized(word)),
            ]
        };
    }
    match heard.tone {
        Some(Tone::Farewell) if world.is_night() => lines(&["Goodnight!", "Sleep well."]),
        Some(Tone::Farewell) => lines(&["Bye… come back soon.", "See you!"]),
        Some(Tone::Greeting) if bond < 0.12 => lines(&["Oh! Hello…", "Hi there."]),
        Some(Tone::Greeting) if bond < 0.55 => lines(&["Hello!", "Hi! It's you."]),
        Some(Tone::Greeting) => lines(&["Hi! You're here!", "Hello, you!"]),
        _ if heard.named => lines(&["Yes? That's me!", "You called?"]),
        _ => Vec::new(),
    }
}

// Why it can't do what the viewer asked for just now.
fn unable(world: &World, activity: Activity) -> Vec<String> {
    let line = match activity {
        Activity::Rest => "I'm not sleepy!",
        Activity::Chase if world.motes.is_empty() => "I don't see any light motes.",
        Activity::Chase => "I don't know where a light mote is.",
        Activity::Tend => "The plant doesn't need water yet.",
        Activity::Hide if !world.viewer_present() => "Nobody's here to find me.",
        _ if world.ghost.needs.energy < 0.4 => "I'm too tired for that.",
        _ => "Not right now.",
    };
    vec![line.to_owned()]
}

fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    characters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(characters).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::EventKind;

    #[test]
    fn every_activity_but_floating_comes_with_a_thought() {
        let mut world = World::new(1, 12.0);
        for activity in Activity::ALL {
            assert_eq!(
                setting_out(&world, activity).is_empty(),
                activity == Activity::Float,
                "{activity:?}"
            );
        }
        world.notice_viewer();
        world.life.bond = 0.6;
        assert!(setting_out(&world, Activity::Visit).contains(&"There you are!"));
    }

    #[test]
    fn idle_thoughts_follow_the_ghosts_needs_and_never_run_dry() {
        let mut world = World::new(1, 12.0);
        assert!(!idle(&world).is_empty());
        world.ghost.needs.energy = 0.1;
        world.plant.thirst = 0.9;
        let lines = idle(&world);
        assert!(lines.contains(&"I'm getting sleepy."));
        assert!(lines.contains(&"The plant looks thirsty."));
    }

    #[test]
    fn idle_thoughts_follow_only_pronounced_traits_and_shyness_needs_a_stranger() {
        use crate::world::Nature;

        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        world.life.nature = Some(Nature::default());
        let average = idle(&world);
        world.life.nature = Some(Nature {
            playful: 1.0,
            curious: 1.0,
            shy: 1.0,
            sleepy: 1.0,
            tender: 1.0,
        });
        let pronounced = idle(&world);
        for line in [
            "Is it okay if I watch you?",
            "What's behind the window, I wonder?",
            "Someone throw the ball!",
            "Yawn… perhaps a little lie down.",
            "I hope the plant is happy.",
        ] {
            assert!(!average.contains(&line));
            assert!(pronounced.contains(&line));
        }
        world.life.bond = 0.5;
        assert!(!idle(&world).contains(&"Is it okay if I watch you?"));
        world.life.bond = 0.0;
        world.viewer.active_at = 0.0;
        assert!(!idle(&world).contains(&"Is it okay if I watch you?"));
    }

    #[test]
    fn dreams_are_woven_from_the_day() {
        let mut world = World::new(1, 23.0);
        assert!(dreams(&world).contains(&"…zzz…"));
        world.life.note(&EventKind::NudgedBall);
        assert_eq!(
            dreams(&world),
            ["…the ball rolls on and on, out into the hills…"]
        );
    }

    #[test]
    fn company_at_bedtime_brings_a_few_reluctant_goodnights() {
        let mut world = World::new(1, 23.0);
        assert!(!idle(&world).contains(&"Just a little longer."));
        world.notice_viewer();
        assert!(idle(&world).contains(&"I should be asleep…"));
        assert!(idle(&world).contains(&"Just a little longer."));
        world.hour = 12.0;
        assert!(!idle(&world).contains(&"Just a little longer."));
    }

    #[test]
    fn morning_thoughts_greet_the_day_and_recall_the_last_dream() {
        let mut world = World::new(1, 6.49);
        world.tuck_in();
        world.think("…the ball rolls on and on, out into the hills…");
        world.hour = 6.5;
        world.step(0.1);
        for lines in [idle(&world), setting_out(&world, Activity::Window)] {
            assert!(lines.contains(&"Good morning!"));
            assert!(lines.contains(&"The sun's up."));
            assert!(lines.contains(&"What a night."));
            assert!(lines.contains(&"That ball rolled all the way to the hills."));
        }
        world.time += 600.0;
        assert!(!idle(&world).contains(&"Good morning!"));
        assert!(!setting_out(&world, Activity::Window).contains(&"Good morning!"));
    }

    #[test]
    fn storms_bring_worries_reassurance_from_a_friend_and_dreams_afterward() {
        let mut world = World::new(1, 12.0);
        world.weather.sky = Sky::Rain;
        world.weather.storm = true;
        assert!(setting_out(&world, Activity::Window).contains(&"Whoa, look at that!"));
        assert!(idle(&world).contains(&"I don't like thunder."));
        assert!(idle(&world).contains(&"Is it over yet?"));
        let reassured = "I'm glad you're here when it thunders.";
        assert!(!idle(&world).contains(&reassured));
        world.notice_viewer();
        assert!(!idle(&world).contains(&reassured));
        world.life.bond = 0.3;
        assert!(idle(&world).contains(&reassured));
        world.viewer.active_at = 0.0;
        assert!(!idle(&world).contains(&reassured));
        world.life.note(&EventKind::StormStarted);
        world.weather.storm = false;
        assert!(dreams(&world).contains(&"…thunder far away, like a drum…"));
        assert!(!idle(&world).contains(&"I don't like thunder."));
    }

    #[test]
    fn every_occasion_brings_its_own_thoughts_and_dreams() {
        use jiff::civil::Date;

        let mut world = World::new(1, 22.0);
        let ordinary_thoughts = idle(&world);
        let ordinary_dreams = dreams(&world);
        for (month, day, birthday) in [
            (10, 31, None),
            (12, 24, None),
            (12, 31, None),
            (2, 14, None),
            (3, 1, Some(Date::constant(2026, 3, 1))),
        ] {
            world.date = Date::new(2027, month, day).unwrap();
            world.birthday = birthday;
            assert!(
                idle(&world)
                    .iter()
                    .any(|line| !ordinary_thoughts.contains(line))
            );
            assert!(
                dreams(&world)
                    .iter()
                    .any(|line| !ordinary_dreams.contains(line))
            );
        }
    }

    #[test]
    fn replies_name_the_word_the_ghost_understood_or_wonders_about() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        let heard = world.hear("dance");
        let lines = reply(&world, &heard, None);
        assert!(!lines.is_empty());
        assert!(
            lines
                .iter()
                .all(|line| line.contains("dance") || line.contains("Dance"))
        );

        world
            .life
            .vocabulary
            .teach(&["spin".to_owned()], Activity::Dance, 2.0);
        let heard = world.hear("spin!");
        assert!(
            reply(&world, &heard, None)
                .iter()
                .all(|line| line.starts_with("Spin"))
        );

        let heard = world.hear("hmm hmm");
        let guessed = world.guess();
        assert!(guessed.is_some());
        assert!(reply(&world, &heard, guessed).contains(&"Like this?".to_owned()));
    }

    #[test]
    fn tone_alone_gets_an_answer_and_little_words_get_none() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        for text in ["hello", "good job", "no", "bye", "Boo?"] {
            let heard = world.hear(text);
            assert!(!reply(&world, &heard, None).is_empty(), "{text}");
        }
        let heard = world.hear("is it");
        assert!(reply(&world, &heard, None).is_empty());
    }
}
