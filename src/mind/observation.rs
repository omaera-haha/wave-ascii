use glam::Vec3;
use serde_json::{Value, json};

use crate::world::{Activity, Perch, Plant, Seen, Sky, Thing, World, calendar::moon_words, flat};

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
            "nature": world.life.nature.unwrap_or_default().words(),
            "energy": ghost.needs.energy_words(),
            "fun": ghost.needs.fun_words(),
            "company": ghost.needs.company_words(),
            "feeling": ghost.mood.name(),
            "awake": awake(world),
            "doing": doing,
            "goal": ghost.goal.as_deref().unwrap_or("none yet"),
            "loves": world.life.favorite().map_or("nothing in particular yet", Activity::doing),
        },
        "time_of_day": time_of_day(world.hour),
        "season": world.season().name(),
        "occasion": world.occasion_words(),
        "outside_the_window": outside_the_window(world),
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
        "bond_with_viewer": world.life.bond_words(),
        "visiting_routine": world.life.usual_time().map(|usual| {
            let mut words = format!("the viewer usually comes {usual}");
            if !world.viewer_present() && world.life.expectation(world.hour) >= 0.5 {
                words.push_str("; it is about the time the viewer usually comes");
            }
            words
        }).unwrap_or_else(|| "not known yet".to_owned()),
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
        .map(|event| {
            format!(
                "{} ({})",
                world.event_words(event),
                ago(world.time - event.time)
            )
        })
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

fn awake(world: &World) -> &'static str {
    let ghost = &world.ghost;
    match ghost.asleep {
        asleep if asleep < 0.3 && world.drowsy() => {
            "awake but drowsy after being woken in the night"
        }
        asleep if asleep < 0.3 => "awake",
        asleep if asleep < 0.9 => "drifting off to sleep",
        _ if ghost.plan.activity == Activity::Rest && !ghost.plan.done => {
            if ghost.plan.overnight {
                "asleep for the night and dreaming"
            } else {
                "napping and dreaming"
            }
        }
        _ => "fast asleep and dreaming",
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

fn outside_the_window(world: &World) -> String {
    let weather = &world.weather;
    let mut outside = weather.words(world.time, world.is_night()).to_owned();
    if world.is_night()
        && matches!(weather.sky, Sky::Clear | Sky::Rainbow)
        && let Some(moon) = moon_words(world.moon_phase())
    {
        outside.push_str(" with ");
        outside.push_str(moon);
    }
    if weather.sky != Sky::Snow && weather.snow_cover > 0.5 {
        outside.push_str(", the hills white with snow");
    }
    if world.fireworks() {
        outside.push_str(", fireworks bursting in the sky");
    }
    outside
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
        Thing::Window => format!("the window, showing {}", outside_the_window(world)),
        Thing::Lamp => format!(
            "the floor lamp, {}",
            if world.lamp.on { "on" } else { "off" }
        ),
        Thing::Plant => format!("the plant, {}", plant(&world.plant)),
        Thing::Ball if world.ball.held => "the ball, held by you".to_owned(),
        Thing::Ball if world.ball.velocity.length() > 0.2 => "the rolling ball".to_owned(),
        Thing::Ball => "the ball".to_owned(),
        Thing::Cushion => "the cushion".to_owned(),
        Thing::Decoration => world
            .decoration()
            .map_or("a decoration", |decoration| decoration.words())
            .to_owned(),
        Thing::Mote(_) => "a light mote".to_owned(),
        Thing::Bird => {
            let place = match world.visitors.bird.as_ref() {
                Some(bird) if bird.perched(world.time) => match bird.perch {
                    Perch::Tree => "perched on the tree",
                    Perch::Sill => "on the sill",
                },
                _ => "flying",
            };
            format!("a little bird outside the window, {place}")
        }
        Thing::Moth => "a moth fluttering around the lamp".to_owned(),
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
    if world.ball.held {
        return "in your arms".to_owned();
    }
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

fn viewer(world: &World) -> String {
    let words = if world.viewer.typing {
        "typing a message to the ghost"
    } else if world.viewer.pointer.is_some() {
        "watching through the glass"
    } else if world.viewer_present() {
        "nearby"
    } else {
        "away"
    };
    if world.viewer.pointer.is_some() {
        let motion = if world.viewer.pointer_is_lively() {
            "darting about"
        } else {
            "still"
        };
        format!("{words}, pointer {motion}")
    } else {
        words.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::EventKind;
    use glam::Vec3;

    #[test]
    fn learned_visiting_hours_and_waiting_are_described_in_words() {
        let mut world = World::new(1, 20.0);
        assert_eq!(observe(&world)["visiting_routine"], "not known yet");
        for day in 1..=3 {
            world.life.begin_day(&format!("2026-06-{day:02}"));
            world.life.arrive(20.0);
        }
        world.date = jiff::civil::Date::constant(2026, 6, 4);
        world.life.begin_day(&world.date.to_string());
        world.begin(Activity::Wait);
        let observation = observe(&world);
        assert_eq!(
            observation["ghost"]["doing"],
            "waiting at the glass for the viewer"
        );
        assert_eq!(
            observation["visiting_routine"],
            "the viewer usually comes in the evening; it is about the time the viewer usually comes"
        );
        assert!(
            !observation
                .to_string()
                .chars()
                .any(|character| character.is_ascii_digit())
        );
        world.notice_viewer();
        world.step(0.1);
        let observation = observe(&world);
        assert_eq!(
            observation["visiting_routine"],
            "the viewer usually comes in the evening"
        );
        assert!(
            !observation
                .to_string()
                .chars()
                .any(|character| character.is_ascii_digit())
        );
        for event in [EventKind::ExpectedArrival, EventKind::MissedVisit] {
            assert!(
                !event
                    .words()
                    .chars()
                    .any(|character| character.is_ascii_digit())
            );
        }
    }

    #[test]
    fn the_observation_describes_the_room_in_words_not_numbers() {
        let mut world = World::new(1, 22.0);
        world.drop_mote(Vec3::new(0.0, 1.0, 0.0));
        world.poke();
        let observation = observe(&world);
        assert_eq!(observation["time_of_day"], "night");
        assert_eq!(observation["season"], "summer");
        assert_eq!(
            observation["ghost"]["nature"],
            world.life.nature.unwrap().words()
        );
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
    fn the_viewers_pointer_can_be_darting_or_still_in_words() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        world.point_at(Some(Vec3::new(-0.6, 1.2, 1.4)));
        assert_eq!(
            observe(&world)["viewer"],
            "watching through the glass, pointer still"
        );
        world.viewer.typing = true;
        assert_eq!(
            observe(&world)["viewer"],
            "typing a message to the ghost, pointer still"
        );
        world.viewer.typing = false;
        world.point_at(Some(Vec3::new(0.6, 1.2, 1.4)));
        world.begin(Activity::Pounce);
        world.ghost.position = world.ghost.plan.target;
        world.step(0.05);
        assert_eq!(
            observe(&world)["viewer"],
            "watching through the glass, pointer darting about"
        );
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Pounced)
        );
        assert!(
            !observe(&world)
                .to_string()
                .chars()
                .any(|character| character.is_ascii_digit())
        );
        world.step(3.0);
        assert_eq!(
            observe(&world)["viewer"],
            "watching through the glass, pointer still"
        );
    }

    #[test]
    fn storms_and_thunder_reach_the_minds_in_words() {
        let mut world = World::new(1, 12.0);
        world.weather.sky = Sky::Rain;
        world.weather.storm = true;
        world.weather.until = 300.0;
        world.weather.thunder_at = Some(0.05);
        world.events.push(crate::world::Event {
            time: 0.0,
            kind: crate::world::EventKind::StormStarted,
        });
        world.step(0.1);
        let observation = observe(&world);
        assert_eq!(observation["outside_the_window"], "a thunderstorm");
        let events = recent(&world, 8);
        assert!(events.contains(&"a thunderstorm rolled in (just now)".to_owned()));
        assert!(events.contains(&"thunder rumbled outside (just now)".to_owned()));
        assert!(
            !observation
                .to_string()
                .chars()
                .any(|character| character.is_ascii_digit())
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
    fn the_ghost_tells_how_close_it_is_to_the_viewer_and_what_it_loves() {
        let mut world = World::new(1, 12.0);
        let observation = observe(&world);
        assert_eq!(observation["bond_with_viewer"], "barely knows them");
        assert_eq!(observation["ghost"]["loves"], "nothing in particular yet");
        assert_eq!(observation["ghost"]["awake"], "awake");
        world.life.bond = 0.6;
        for _ in 0..8 {
            world
                .life
                .note(&crate::world::EventKind::Began(Activity::Tend));
        }
        world.ghost.asleep = 1.0;
        let observation = observe(&world);
        assert_eq!(observation["bond_with_viewer"], "close to them");
        assert_eq!(observation["ghost"]["loves"], "tending the plant");
        assert_eq!(observation["ghost"]["awake"], "fast asleep and dreaming");
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
    fn a_held_ball_is_described_in_words_in_the_ghosts_arms() {
        let mut world = World::new(1, 12.0);
        world.begin(Activity::Fetch);
        world.ball.held = true;
        world.ball.velocity = Vec3::X;
        let observation = observe(&world);
        assert_eq!(observation["room"]["ball"], "in your arms");
        assert_eq!(thing(&world, Thing::Ball), "the ball, held by you");
        let text = observation.to_string();
        assert!(
            !text.chars().any(|character| character.is_ascii_digit()),
            "{text}"
        );
    }

    #[test]
    fn visitors_and_their_events_are_described_only_in_words() {
        use crate::world::{
            Event, EventKind, Random,
            visitor::{Bird, Moth},
        };

        let mut world = World::new(1, 12.0);
        let mut random = Random::new(1);
        let mut bird = Bird::new(0.0, &mut random);
        for (time, perch, words) in [
            (1.0, Perch::Tree, "flying"),
            (5.0, Perch::Tree, "perched on the tree"),
            (5.0, Perch::Sill, "on the sill"),
        ] {
            bird.perch = perch;
            world.visitors.bird = Some(bird);
            world.time = time;
            world.ghost.heading = (bird.position(time) - world.ghost.position).normalize();
            let observation = observe(&world).to_string();
            assert!(observation.contains(&format!("a little bird outside the window, {words}")));
            assert!(
                !observation
                    .chars()
                    .any(|character| character.is_ascii_digit())
            );
        }
        let mut moth = Moth::new(world.time, &mut random);
        moth.position = world.lamp.bulb() + Vec3::new(0.3, 0.15, 0.3);
        world.visitors.moth = Some(moth);
        world.ghost.heading = (moth.position - world.ghost.position).normalize();
        for kind in [
            EventKind::BirdLanded,
            EventKind::BirdSang,
            EventKind::BirdFlewOff { startled: true },
            EventKind::MothArrived,
            EventKind::MothLeft,
            EventKind::Noticed("a moth at the lamp".to_owned()),
        ] {
            world.events.push(Event {
                time: world.time,
                kind,
            });
        }
        let observation = observe(&world).to_string();
        assert!(observation.contains("a moth fluttering around the lamp"));
        assert!(
            !observation
                .chars()
                .any(|character| character.is_ascii_digit()),
            "{observation}"
        );
    }

    #[test]
    fn hours_name_the_time_of_day() {
        assert_eq!(time_of_day(6.0), "early morning");
        assert_eq!(time_of_day(13.0), "afternoon");
        assert_eq!(time_of_day(23.5), "night");
        assert_eq!(time_of_day(2.0), "night");
    }

    #[test]
    fn night_sleep_naps_and_drowsy_interruptions_are_described_without_digits() {
        let mut world = World::new(1, 23.0);
        world.tuck_in();
        assert_eq!(
            observe(&world)["ghost"]["awake"],
            "asleep for the night and dreaming"
        );
        world.poke();
        assert_eq!(
            observe(&world)["ghost"]["awake"],
            "awake but drowsy after being woken in the night"
        );
        let text = observe(&world).to_string();
        assert!(
            !text.chars().any(|character| character.is_ascii_digit()),
            "{text}"
        );
        world.hour = 14.0;
        world.ghost.position = world.nook.bed();
        world.ghost.needs.energy = 0.4;
        world.begin(Activity::Rest);
        world.step(4.0);
        assert_eq!(observe(&world)["ghost"]["awake"], "napping and dreaming");
    }

    #[test]
    fn seasons_snow_and_visible_moons_are_described_without_digits() {
        let mut world = World::new(1, 22.0);
        world.date = jiff::civil::Date::constant(2026, 3, 3);
        assert_eq!(observe(&world)["season"], "spring");
        assert_eq!(
            outside_the_window(&world),
            "a clear night sky with a full moon"
        );
        assert!(thing(&world, Thing::Window).contains("a full moon"));
        world.date = jiff::civil::Date::constant(2026, 2, 17);
        assert_eq!(outside_the_window(&world), "a clear night sky");
        world.weather.sky = Sky::Snow;
        world.weather.snow_cover = 0.8;
        assert_eq!(observe(&world)["season"], "winter");
        assert_eq!(outside_the_window(&world), "snow falling, the hills white");
        assert!(thing(&world, Thing::Window).contains("snow falling"));
        assert!(
            !observe(&world)
                .to_string()
                .chars()
                .any(|character| character.is_ascii_digit())
        );
        world.weather.sky = Sky::Clear;
        assert!(outside_the_window(&world).contains("hills white with snow"));
        world.date = jiff::civil::Date::constant(2026, 3, 3);
        world.hour = 12.0;
        assert!(!outside_the_window(&world).contains("moon"));
        world.hour = 22.0;
        world.weather.sky = Sky::Rain;
        assert!(!outside_the_window(&world).contains("moon"));
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

    #[test]
    fn occasions_and_decorations_are_named_in_words_without_digits() {
        use jiff::civil::Date;

        let mut world = World::new(1, 22.0);
        for (date, birthday, occasion, decoration) in [
            (
                Date::constant(2026, 10, 24),
                None,
                "Halloween",
                "a glowing jack-o'-lantern",
            ),
            (
                Date::constant(2026, 12, 24),
                None,
                "the winter holidays",
                "a little tree with twinkling lights",
            ),
            (
                Date::constant(2026, 12, 31),
                None,
                "New Year's Eve",
                "a little tree with twinkling lights",
            ),
            (
                Date::constant(2027, 1, 1),
                None,
                "New Year's Day",
                "a little tree with twinkling lights",
            ),
            (
                Date::constant(2027, 2, 14),
                None,
                "Valentine's Day",
                "a decoration",
            ),
            (
                Date::constant(2028, 3, 1),
                Some(Date::constant(2026, 3, 1)),
                "its second birthday",
                "a birthday cake with candles",
            ),
            (
                Date::constant(2026, 6, 21),
                None,
                "an ordinary day",
                "a decoration",
            ),
        ] {
            world.date = date;
            world.birthday = birthday;
            world.step(0.05);
            assert_eq!(observe(&world)["occasion"], occasion);
            assert_eq!(thing(&world, Thing::Decoration), decoration);
            let text = observe(&world).to_string();
            assert!(
                !text.chars().any(|character| character.is_ascii_digit()),
                "{text}"
            );
        }
        world.date = Date::constant(2026, 12, 31);
        world.hour = 23.5;
        world.step(0.05);
        assert!(outside_the_window(&world).contains("fireworks"));
        world.date = Date::constant(2027, 1, 1);
        world.hour = 0.0;
        world.step(0.05);
        assert!(recent(&world, 8).contains(&"a new year began (just now)".to_owned()));
        assert!(
            !observe(&world)
                .to_string()
                .chars()
                .any(|character| character.is_ascii_digit())
        );
    }
}
