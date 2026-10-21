use glam::Vec3;
use serde_json::{Value, json};

use crate::world::{Activity, Plant, Seen, Thing, World, flat};

/// What the ghost knows of its situation, in words. Both minds read it:
/// System One models judge words more reliably than numbers, and Claude
/// writes in the ghost's voice from it.
pub fn observe(world: &World) -> Value {
    let ghost = &world.ghost;
    let plan = &ghost.plan;
    let doing = if plan.done {
        format!("just finished {}", plan.activity.doing())
    } else if plan.activity == Activity::Hide && plan.arrived.is_some() {
        format!("{} and waiting to be found", plan.activity.doing())
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
        "outside_the_window": world.weather.words(world.time, world.is_night()),
        "sight": sight(world),
        "room": {
            "light": light(world),
            "lamp": if world.lamp.on { "on" } else { "off" },
            "plant": plant(&world.plant),
            "ball": ball(world),
            "light_motes": motes(world),
            "glass": glass(world),
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

// What the ghost sees from where it faces: what it looks at, what else is
// in view and where, and what it can't see now.
fn sight(world: &World) -> Value {
    let sight = world.sight();
    let in_view: Vec<String> = sight
        .in_view
        .iter()
        .take(6)
        .map(|seen| format!("{}: {}", thing(world, seen.thing), placement(seen)))
        .collect();
    let out_of_view: Vec<&str> = sight.out_of_view.iter().map(|thing| thing.name()).collect();
    json!({
        "facing": facing(world.ghost.heading),
        "looking_at": sight
            .looking_at
            .map_or("nothing in particular".to_owned(), |looked| thing(world, looked)),
        "in_view": in_view,
        "out_of_view": out_of_view.join(", "),
    })
}

fn thing(world: &World, thing: Thing) -> String {
    match thing {
        Thing::Viewer if world.viewer.typing => "the viewer, typing to you".to_owned(),
        Thing::Viewer if world.viewer_present() => {
            "the viewer, watching through the glass".to_owned()
        }
        Thing::Viewer => "the glass, with nobody behind it".to_owned(),
        Thing::Window => format!(
            "the window, showing {}",
            world.weather.words(world.time, world.is_night())
        ),
        Thing::Lamp => format!(
            "the floor lamp, {}",
            if world.lamp.on { "on" } else { "off" }
        ),
        Thing::Plant => format!("the plant, {}", plant(&world.plant)),
        Thing::Ball if world.ball.velocity.length() > 0.2 => "the rolling ball".to_owned(),
        Thing::Ball => "the ball".to_owned(),
        Thing::Cushion => "the cushion".to_owned(),
        Thing::Mote(_) => "a light mote".to_owned(),
    }
}

// How far a seen thing is and where it lies in the ghost's view.
fn placement(seen: &Seen) -> String {
    let side = match seen.across {
        across if across.abs() < 0.17 => "straight ahead",
        across if across <= -0.6 => "to the left",
        across if across < 0.0 => "a little to the left",
        across if across >= 0.6 => "to the right",
        _ => "a little to the right",
    };
    let height = match seen.up {
        up if up > 0.35 => ", above",
        up if up < -0.35 => ", below",
        _ => "",
    };
    let hidden = seen
        .hidden_behind
        .map(|blocker| format!(", hidden behind the {}", blocker.name()))
        .unwrap_or_default();
    format!("{}, {side}{height}{hidden}", distance_words(seen.distance))
}

fn facing(heading: Vec3) -> &'static str {
    match heading {
        heading if heading.y > 0.7 => "up at the ceiling",
        heading if heading.y < -0.7 => "down at the floor",
        heading if heading.z.abs() >= heading.x.abs() && heading.z > 0.0 => {
            "toward the glass and the viewer's side"
        }
        heading if heading.z.abs() >= heading.x.abs() => "toward the back wall and the window",
        heading if heading.x < 0.0 => "toward the left wall, by the lamp",
        _ => "toward the right wall, by the plant",
    }
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

fn glass(world: &World) -> String {
    let doodles: Vec<String> = world
        .doodles
        .iter()
        .rev()
        .filter(|doodle| doodle.opacity(world.time) > 0.0)
        .take(2)
        .map(|doodle| {
            let fading = if doodle.opacity(world.time) < 0.5 {
                ", fading"
            } else {
                ""
            };
            format!("a {} you drew in the fog{fading}", doodle.sketch.name())
        })
        .collect();
    if doodles.is_empty() {
        "clear".to_owned()
    } else {
        doodles.join("; ")
    }
}

// The motes the ghost knows of; others it hasn't seen yet.
fn motes(world: &World) -> String {
    let known = world
        .motes
        .iter()
        .filter(|mote| world.ghost.known.contains(&mote.id))
        .count();
    let count = match known {
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
    fn the_ghost_reports_what_it_sees_where() {
        let mut world = World::new(1, 12.0);
        world.ghost.position = Vec3::new(0.0, 1.2, 0.0);
        world.ghost.heading = -Vec3::Z;
        let sight = observe(&world)["sight"].clone();
        assert_eq!(sight["facing"], "toward the back wall and the window");
        assert_eq!(sight["looking_at"], "the window, showing a clear sky");
        let in_view: Vec<&str> = sight["in_view"]
            .as_array()
            .unwrap()
            .iter()
            .map(|seen| seen.as_str().unwrap())
            .collect();
        assert!(
            in_view.iter().any(
                |seen| seen.starts_with("the floor lamp, on: ") && seen.contains("to the left")
            ),
            "{in_view:?}"
        );
        assert!(sight["out_of_view"].as_str().unwrap().contains("viewer"));
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

    #[test]
    fn the_glass_reports_the_newest_pictures_and_their_fading_in_words() {
        use crate::world::{Doodle, Sketch};
        use glam::Vec2;

        let mut world = World::new(1, 12.0);
        assert_eq!(observe(&world)["room"]["glass"], "clear");
        for (index, sketch) in [Sketch::Smile, Sketch::Star, Sketch::Heart]
            .into_iter()
            .enumerate()
        {
            world.doodles.push(Doodle {
                sketch,
                center: Vec2::ZERO,
                size: 0.3,
                started: index as f64,
            });
        }
        world.time = Doodle::LIFETIME - 10.0;
        let observation = observe(&world);
        assert_eq!(
            observation["room"]["glass"],
            "a heart you drew in the fog, fading; a star you drew in the fog, fading"
        );
        assert!(
            !observation
                .to_string()
                .chars()
                .any(|character| character.is_ascii_digit())
        );
        world.time += 20.0;
        assert_eq!(observe(&world)["room"]["glass"], "clear");
    }

    #[test]
    fn a_hidden_ghost_reports_that_it_is_waiting_to_be_found() {
        let mut world = World::new(1, 12.0);
        world.begin(Activity::Hide);
        world.step(15.0);
        assert_eq!(
            observe(&world)["ghost"]["doing"],
            "hiding from the viewer and waiting to be found"
        );
        world.poke();
        assert_eq!(
            observe(&world)["ghost"]["doing"],
            "just finished hiding from the viewer"
        );
    }
}
