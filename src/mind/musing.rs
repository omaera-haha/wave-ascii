use crate::world::{Activity, Mood, Sky, World};

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
        Activity::Rest if world.is_night() => {
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
        Activity::Dance => vec!["I feel like dancing.", "Wheee!"],
        Activity::Window => match world.weather.sky {
            Sky::Rainbow => vec!["Look, a rainbow!", "All the colors at once."],
            Sky::Rain => vec!["Listen to the rain.", "Pitter-patter."],
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
    if world.plant.thirst > 0.7 {
        lines.push("The plant looks thirsty.");
    }
    if world.plant.blooming() {
        lines.push("The flowers smell nice.");
    }
    match world.weather.sky {
        Sky::Rain => lines.extend(["The rain sounds cozy.", "Drip, drop."]),
        Sky::Rainbow => lines.push("That rainbow is still there!"),
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
            Activity::Dance => "I've been working on a new twirl.",
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

/// What it dreams of while it sleeps, mostly of what happened today.
pub fn dreams(world: &World) -> Vec<&'static str> {
    let today = &world.life.today;
    let mut lines = Vec::new();
    if today.motes > 0 {
        lines.extend([
            "…a whole sky of light motes, and all of them mine…",
            "…chasing a mote that keeps getting bigger…",
        ]);
    }
    if today.nudges > 0 || today.fetches > 0 {
        lines.push("…the ball rolls on and on, out into the hills…");
    }
    if today.dances > 0 {
        lines.push("…dancing with the light motes, round and round…");
    }
    if today.waterings > 0 {
        lines.push("…the plant grows right up through the ceiling…");
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
    fn dreams_are_woven_from_the_day() {
        let mut world = World::new(1, 23.0);
        assert!(dreams(&world).contains(&"…zzz…"));
        world.life.note(&EventKind::NudgedBall);
        assert_eq!(
            dreams(&world),
            ["…the ball rolls on and on, out into the hills…"]
        );
    }
}
