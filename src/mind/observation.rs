use serde_json::{Value, json};

use crate::world::{Activity, Plant, World, flat};

/// What the ghost knows of its situation, in words. Both minds read it:
/// System One models judge words more reliably than numbers, and Claude
/// writes in the ghost's voice from it.
pub fn observe(world: &World) -> Value {
    let ghost = &world.ghost;
    let plan = &ghost.plan;
    let doing = if plan.done {
        format!("just finished {}", plan.activity.doing())
    } else {
        plan.activity.doing().to_owned()
    };
    json!({
        "ghost": {
            "name": ghost.name,
            "energy": ghost.needs.energy_words(),
            "fun": ghost.needs.fun_words(),
            "company": ghost.needs.company_words(),
            "feeling": ghost.mood.name(),
            "doing": doing,
            "goal": ghost.goal.as_deref().unwrap_or("none yet"),
        },
        "time_of_day": time_of_day(world.hour),
        "room": {
            "light": light(world),
            "lamp": if world.lamp.on { "on" } else { "off" },
            "plant": plant(&world.plant),
            "ball": ball(world),
            "light_motes": motes(world),
        },
        "viewer": viewer(world),
        "recent_events": recent(world, 8),
    })
}

/// The latest events, newest last, with how long ago each happened.
pub fn recent(world: &World, count: usize) -> Vec<String> {
    let mut lines: Vec<String> = world
        .events
        .iter()
        .rev()
        .filter(|event| !matches!(event.kind, crate::world::EventKind::Began(Activity::Float)))
        .take(count)
        .map(|event| format!("{} ({})", event.kind.words(), ago(world.time - event.time)))
        .collect();
    lines.reverse();
    lines
}

pub fn time_of_day(hour: f32) -> &'static str {
    match hour {
        hour if (5.0..8.0).contains(&hour) => "early morning",
        hour if (8.0..12.0).contains(&hour) => "morning",
        hour if (12.0..17.0).contains(&hour) => "afternoon",
        hour if (17.0..21.0).contains(&hour) => "evening",
        _ => "night",
    }
}

fn ago(seconds: f64) -> &'static str {
    match seconds {
        seconds if seconds < 15.0 => "just now",
        seconds if seconds < 90.0 => "a minute ago",
        seconds if seconds < 600.0 => "a few minutes ago",
        _ => "a while ago",
    }
}

fn distance_words(distance: f32) -> &'static str {
    match distance {
        distance if distance < 0.8 => "close by",
        distance if distance < 2.0 => "across the room",
        _ => "far away",
    }
}

fn light(world: &World) -> &'static str {
    match (world.is_night(), world.lamp.on) {
        (false, _) => "daylight",
        (true, true) => "lamplight in the dark",
        (true, false) => "dark",
    }
}

fn plant(plant: &Plant) -> String {
    let size = match plant.growth {
        growth if plant.blooming() && growth >= 0.95 => "fully grown and flowering",
        _ if plant.blooming() => "flowering",
        growth if growth < 0.3 => "a small sprout",
        growth if growth < 0.6 => "a young plant",
        _ => "a leafy plant",
    };
    let water = match plant.thirst {
        thirst if thirst < 0.3 => "well watered",
        thirst if thirst < 0.7 => "a little dry",
        _ => "very thirsty and drooping",
    };
    format!("{size}, {water}")
}

fn ball(world: &World) -> String {
    let distance = flat(world.ball.position - world.ghost.position).length();
    let motion = if world.ball.velocity.length() > 0.2 {
        "rolling"
    } else {
        "resting on the floor"
    };
    format!("{motion}, {}", distance_words(distance))
}

fn motes(world: &World) -> String {
    let count = match world.motes.len() {
        0 => return "none".to_owned(),
        1 => "one light mote drifting".to_owned(),
        count => format!("{count} light motes drifting"),
    };
    let nearest = world
        .nearest_mote()
        .map(|mote| mote.position.distance(world.ghost.position))
        .unwrap_or(f32::INFINITY);
    format!("{count}, the nearest {}", distance_words(nearest))
}

fn viewer(world: &World) -> &'static str {
    if world.viewer.typing {
        "typing a message to the ghost"
    } else if world.viewer.pointer.is_some() && world.viewer_present() {
        "watching through the glass, pointer moving"
    } else if world.viewer_present() {
        "nearby"
    } else {
        "away"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn the_observation_describes_the_room_in_words_not_numbers() {
        let mut world = World::new(1, 22.0);
        world.drop_mote(Vec3::new(0.0, 1.0, 0.0));
        world.poke();
        let observation = observe(&world);
        assert_eq!(observation["time_of_day"], "night");
        assert_eq!(observation["room"]["light"], "lamplight in the dark");
        assert!(
            observation["room"]["light_motes"]
                .as_str()
                .unwrap()
                .starts_with("one light mote")
        );
        let events: Vec<&str> = observation["recent_events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|event| event.as_str().unwrap())
            .collect();
        assert!(
            events.contains(&"the viewer poked you (just now)"),
            "{events:?}"
        );
        let text = observation.to_string();
        assert!(
            !text.chars().any(|character| character.is_ascii_digit()),
            "{text}"
        );
    }

    #[test]
    fn the_plant_reads_by_size_and_water() {
        let thirsty = Plant {
            position: Vec3::ZERO,
            growth: 0.85,
            thirst: 0.9,
        };
        assert_eq!(plant(&thirsty), "flowering, very thirsty and drooping");
    }

    #[test]
    fn hours_name_the_time_of_day() {
        assert_eq!(time_of_day(6.0), "early morning");
        assert_eq!(time_of_day(13.0), "afternoon");
        assert_eq!(time_of_day(23.5), "night");
        assert_eq!(time_of_day(2.0), "night");
    }
}
