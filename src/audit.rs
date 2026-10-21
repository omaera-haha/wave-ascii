//! A behavior audit for tuning the ghost's rhythm: it lives a few days
//! offline beside a simulated viewer, who comes by in the morning, at
//! lunch, and in the evening, and does something every few minutes. The
//! report counts activities, events, naps, repeated thoughts, and the
//! diary, so pacing problems (a hundred naps a day, rain every twenty
//! minutes) show as numbers. Run it with
//! `cargo test --release audit -- --ignored --nocapture`; the full
//! timeline goes to `wave-ascii-audit-SEED.log` in the temporary directory.
use std::collections::BTreeMap;

use glam::Vec3;

use crate::mind::{Journal, Mind};
use crate::store::Config;
use crate::world::{EventKind, ROOM_MAX, World};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn present(hour: f64) -> bool {
    (7.5..8.5).contains(&hour) || (12.0..13.0).contains(&hour) || (18.0..23.5).contains(&hour)
}

const MESSAGES: [&str; 14] = [
    "hello",
    "good job",
    "let's play",
    "how are you?",
    "what are you doing?",
    "my name is Sam",
    "go to sleep",
    "no",
    "tell me a story",
    "안녕",
    "뭐해?",
    "let's play a word game",
    "I love you",
    "bye",
];

fn run(seed: u64, days: f64, born: Option<jiff::civil::Date>, korean: bool) {
    let start = jiff::civil::date(2026, 10, 21).at(7, 0, 0, 0);
    let mut world = World::new(seed, 7.0);
    world.date = start.date();
    world.birthday = born;
    world.begin_life(&start.date().to_string());
    let mut mind = Mind::new(&Config::default(), Journal::default(), true);
    let mut rng = Rng(seed ^ 0xABCDEF);
    let step = 0.1f32;
    let total = days * 86400.0;
    let mut seen = 0.0f64;
    let mut events: BTreeMap<String, u32> = BTreeMap::new();
    let mut shares: BTreeMap<String, f64> = BTreeMap::new();
    let mut hourly: BTreeMap<u32, BTreeMap<String, f64>> = BTreeMap::new();
    let mut thoughts: BTreeMap<String, u32> = BTreeMap::new();
    let mut speeches: BTreeMap<String, u32> = BTreeMap::new();
    let mut last_thought = String::new();
    let mut last_speech = String::new();
    let mut switches = 0u32;
    let mut durations: BTreeMap<String, (u32, f64)> = BTreeMap::new();
    let mut noticed: BTreeMap<String, u32> = BTreeMap::new();
    let mut short_plans: BTreeMap<String, u32> = BTreeMap::new();
    let mut plan_started = 0.0f64;
    let mut last_activity = world.ghost.plan.activity;
    let mut was_present = true;
    let mut next_action = 0.0f64;
    let mut log: Vec<String> = Vec::new();
    let mut focus_day = None;
    let mut feeling_day = None;
    let mut thought_count = 0u32;
    let mut speech_count = 0u32;
    let mut max_speed = 0.0f32;
    let mut outside = 0u32;
    let mut time = 0.0f64;
    let mut by_day: Vec<String> = Vec::new();
    while time < total {
        let clock = start
            .checked_add(jiff::SignedDuration::from_secs_f64(time))
            .unwrap();
        if clock.date() != world.date {
            by_day.push(format!(
                "  {}: bond {:.2}, {} milestones, {} books owned",
                world.date,
                world.life.bond,
                world.life.milestones.len(),
                world.life.books.owned.len(),
            ));
        }
        world.date = clock.date();
        let hour = f64::from(clock.hour())
            + f64::from(clock.minute()) / 60.0
            + f64::from(clock.second()) / 3600.0;
        world.hour = hour as f32;
        let here = present(hour);
        if here != was_present {
            world.look_away(!here);
            log.push(format!(
                "{clock} viewer {}",
                if here { "arrives" } else { "leaves" }
            ));
            was_present = here;
        }
        if here && time >= next_action {
            next_action = time + 60.0 + rng.next() * 240.0;
            world.notice_viewer();
            let roll = rng.next();
            let pointer = Vec3::new(
                (rng.next() as f32 - 0.5) * 3.0,
                0.5 + rng.next() as f32 * 1.5,
                ROOM_MAX.z,
            );
            world.point_at(Some(pointer));
            if roll < 0.05 {
                world.poke();
                log.push(format!("{clock} viewer pokes"));
            } else if roll < 0.15 {
                for _ in 0..8 {
                    world.pet(0.3);
                    world.step(0.1);
                }
            } else if roll < 0.35 {
                world.drop_mote(pointer - Vec3::Z * 0.6);
            } else if roll < 0.45 {
                world.kick_ball(Vec3::new(rng.next() as f32 - 0.5, 0.0, -0.5));
            } else if roll < 0.65 {
                let mut text = MESSAGES[(rng.next() * MESSAGES.len() as f64) as usize];
                if korean && rng.next() < 0.7 {
                    text = ["안녕", "뭐해?", "잘했어", "놀자", "이야기 해줘", "사랑해"]
                        [(rng.next() * 6.0) as usize];
                }
                mind.hear(&mut world, text);
                log.push(format!("{clock} viewer says {text:?}"));
            } else if roll < 0.7 && world.challenge() {
                log.push(format!("{clock} viewer challenges"));
            }
            if world.game.is_some() {
                for cell in 0..9 {
                    if world.mark(cell) {
                        break;
                    }
                }
            }
            if focus_day != Some(clock.date()) && (12.1..12.2).contains(&hour) {
                focus_day = Some(clock.date());
                world.toggle_focus();
                log.push(format!("{clock} viewer starts focus"));
            }
        } else if here && rng.next() < 0.002 {
            world.notice_viewer();
        }
        // One disclosure per evening exercises care and next-day follow-ups
        // without changing the simulated viewer's ordinary action rate.
        if here && feeling_day != Some(clock.date()) && (18.2..18.3).contains(&hour) {
            feeling_day = Some(clock.date());
            let message = if korean {
                "오늘 너무 힘들었어"
            } else {
                "I'm stressed out"
            };
            mind.hear(&mut world, message);
            log.push(format!("{clock} viewer says {message:?}"));
        }
        world.step(step);
        mind.think(&mut world);
        // What the app adds to each frame of a living world.
        world.celebrate_wish();
        world.share_wish();
        world.share_anniversary();
        time += f64::from(step);
        let activity = world.ghost.plan.activity;
        let name = activity.name().to_owned();
        *shares.entry(name.clone()).or_default() += f64::from(step);
        *hourly
            .entry(hour as u32)
            .or_default()
            .entry(name.clone())
            .or_default() += f64::from(step);
        if activity != last_activity {
            let lasted = time - plan_started;
            let entry = durations
                .entry(last_activity.name().to_owned())
                .or_insert((0u32, 0.0f64));
            entry.0 += 1;
            entry.1 += lasted;
            switches += 1;
            if time - plan_started < 3.0 {
                *short_plans
                    .entry(format!("{} -> {}", last_activity.name(), name))
                    .or_default() += 1;
            }
            plan_started = time;
            last_activity = activity;
        }
        let speed = world.ghost.velocity.length();
        max_speed = max_speed.max(speed);
        if !world.ghost.position.is_finite()
            || world
                .ghost
                .position
                .cmplt(crate::world::ROOM_MIN - Vec3::splat(0.6))
                .any()
            || world
                .ghost
                .position
                .cmpgt(ROOM_MAX + Vec3::splat(0.6))
                .any()
        {
            outside += 1;
        }
        if let Some(thought) = &world.ghost.thought
            && thought.text != last_thought
        {
            last_thought = thought.text.clone();
            thought_count += 1;
            *thoughts.entry(thought.text.clone()).or_default() += 1;
            log.push(format!("{clock} thinks {:?}", thought.text));
        }
        if let Some(speech) = &world.ghost.speech
            && speech.text != last_speech
        {
            last_speech = speech.text.clone();
            speech_count += 1;
            *speeches.entry(speech.text.clone()).or_default() += 1;
            log.push(format!("{clock} says {:?}", speech.text));
        }
        for event in world.events.iter().filter(|event| event.time > seen) {
            let key = match &event.kind {
                EventKind::Began(activity) => format!("Began({})", activity.name()),
                EventKind::Milestone(text) => {
                    log.push(format!("{clock} MILESTONE {text}"));
                    "Milestone".to_owned()
                }
                EventKind::Noticed(thing) => {
                    *noticed.entry(thing.clone()).or_default() += 1;
                    "Noticed".to_owned()
                }
                other => {
                    let text = format!("{other:?}");
                    let head = text.split(['(', ' ', '{']).next().unwrap_or("").to_owned();
                    if !matches!(
                        other,
                        EventKind::AteMote | EventKind::Tagged { .. } | EventKind::BirdSang
                    ) {
                        log.push(format!("{clock} event {}", other.words()));
                    }
                    head
                }
            };
            *events.entry(key).or_default() += 1;
        }
        if let Some(last) = world.events.last() {
            seen = seen.max(last.time);
        }
    }
    println!("==== seed {seed} days {days} born {born:?} korean {korean}");
    println!(
        "switches {switches} ({:.1}/h), thoughts {thought_count}, speeches {speech_count}, max speed {max_speed:.2}, outside steps {outside}",
        f64::from(switches) / (days * 24.0)
    );
    println!("-- activity shares");
    let mut shares: Vec<_> = shares.into_iter().collect();
    shares.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (name, seconds) in &shares {
        println!("  {name:10} {:5.1}%", seconds / total * 100.0);
    }
    println!("-- mean durations");
    for (name, (count, seconds)) in &durations {
        println!(
            "  {name:10} {count:6} x {:7.1} s",
            seconds / f64::from(*count)
        );
    }
    println!("-- noticed");
    for (name, count) in &noticed {
        println!("  {count:6} {name}");
    }
    println!("-- by hour (top three)");
    for (hour, map) in &hourly {
        let mut list: Vec<_> = map.iter().collect();
        list.sort_by(|a, b| b.1.total_cmp(a.1));
        let sum: f64 = list.iter().map(|(_, s)| **s).sum();
        let top: Vec<String> = list
            .iter()
            .take(3)
            .map(|(n, s)| format!("{n} {:.0}%", **s / sum * 100.0))
            .collect();
        println!("  {hour:02}: {}", top.join(", "));
    }
    println!("-- quick switches (<3 s)");
    let mut quick: Vec<_> = short_plans.into_iter().collect();
    quick.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (pair, count) in quick.iter().take(15) {
        println!("  {count:5} {pair}");
    }
    println!("-- events");
    for (name, count) in &events {
        println!("  {count:6} {name}");
    }
    println!("-- most repeated thoughts");
    let mut list: Vec<_> = thoughts.into_iter().collect();
    list.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (text, count) in list.iter().take(25) {
        println!("  {count:4} {text}");
    }
    println!("-- most repeated speech");
    let mut list: Vec<_> = speeches.into_iter().collect();
    list.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for (text, count) in list.iter().take(25) {
        println!("  {count:4} {text}");
    }
    println!("-- diary");
    for entry in &world.life.diary {
        println!("  {}: {}", entry.day, entry.text);
    }
    println!("-- milestones");
    for entry in &world.life.milestones {
        println!("  {}: {}", entry.day, entry.text);
    }
    println!("-- by day");
    for line in &by_day {
        println!("{line}");
    }
    println!("bond {:.2} needs {:?}", world.life.bond, world.ghost.needs);
    let path = std::env::temp_dir().join(format!("wave-ascii-audit-{seed}.log"));
    std::fs::write(&path, log.join("\n")).unwrap();
    println!("timeline: {}", path.display());
}

/// Days each audit lives: three, or `AUDIT_DAYS` for a longer look.
fn days() -> f64 {
    std::env::var("AUDIT_DAYS")
        .ok()
        .and_then(|days| days.parse().ok())
        .filter(|days: &f64| *days > 0.0)
        .unwrap_or(3.0)
}

#[test]
#[ignore]
fn audit_a_grown_ghost_over_three_days() {
    run(7, days(), None, false);
}

#[test]
#[ignore]
fn audit_a_newborn_spoken_to_in_korean_over_three_days() {
    run(11, days(), Some(jiff::civil::date(2026, 10, 20)), true);
}
