use crate::world::{
    Activity, EventKind, GREETS_FROM, Mood, Occasion, Outcome, Sky, Stage, World, flat,
};

/// How much the ghost's needs, surroundings, nature, and habits pull it toward each
/// of the activities open to it, as chances that add up to 1. This is the mind it
/// has without any model, and the weight it gives its own needs when a model
/// does choose.
pub fn weigh(world: &World, options: &[Activity]) -> Vec<(Activity, f32)> {
    let needs = world.ghost.needs;
    let night = world.is_night();
    let bedtime = world.bedtime();
    let present = world.viewer_present();
    let morning = world.morning();
    let shelter = world.weather.storm && world.life.bond < GREETS_FROM;
    let weights: Vec<f32> = options
        .iter()
        .map(|activity| match activity {
            Activity::Letter => {
                if world.letter_is_open() {
                    20.0
                } else {
                    0.0
                }
            }
            Activity::Mirror => {
                if world.ghost.asleep >= 0.3 {
                    0.0
                } else {
                    let young = if world.stage() == Stage::Grown {
                        1.0
                    } else {
                        1.5
                    };
                    let new_outfit = world.wearing.is_some()
                        && world
                            .events
                            .iter()
                            .rev()
                            .take_while(|event| world.time - event.time < 600.0)
                            .any(|event| matches!(event.kind, EventKind::PutOn(_)));
                    (0.006 + 0.009 * (1.0 - needs.fun) + if new_outfit { 0.014 } else { 0.0 })
                        * needs.energy
                        * young
                }
            }
            Activity::Read => {
                let age = if world.stage() == Stage::Newborn {
                    0.25
                } else {
                    1.0
                };
                // Rereading only old favorites draws it less than a story
                // it hasn't finished, and pages read today satisfy it.
                let books = &world.life.books.owned;
                let unread = if books.iter().any(|book| book.finished == 0) {
                    1.0
                } else {
                    0.4
                };
                let read = 1.0 / (1.0 + world.life.today.pages as f32 / 12.0);
                // A few sittings a day, more as quiet company during work.
                (0.04 + 0.1 * (1.0 - needs.fun) + if world.focus.is_some() { 0.45 } else { 0.0 })
                    * needs.energy
                    * age
                    * unread
                    * read
            }
            Activity::Music => {
                if world.focus.is_some() || world.ghost.asleep >= 0.3 {
                    0.0
                } else {
                    (0.03 + 0.1 * (1.0 - needs.fun) + if world.hour >= 18.0 { 0.06 } else { 0.0 })
                        * needs.energy
                }
            }
            Activity::Air => {
                if world.ghost.asleep >= 0.3
                    || (!world.window_open && !world.fresh_air_is_fair())
                    || (world.window_open && !world.wants_window_shut())
                {
                    0.0
                } else if world.window_open {
                    30.0
                } else {
                    0.25 + 0.8 * (1.0 - needs.fun) + 0.5 * (1.0 - needs.energy)
                }
            }
            Activity::Friend => {
                if !world.friend_is_open() {
                    0.0
                } else {
                    (5.0 + 3.0 * (1.0 - needs.company) + 2.0 * world.life.temperament().playful)
                        * needs.energy
                        * if present {
                            1.0 - 0.25 * world.life.bond
                        } else {
                            1.0
                        }
                }
            }
            Activity::Comfort => 0.0,
            Activity::Company => {
                if world.focus.is_some() {
                    10.0 * needs.energy
                } else {
                    0.0
                }
            }
            Activity::Float => 0.45,
            Activity::Look => {
                if world.options().contains(&Activity::Look) {
                    8.0
                } else {
                    0.0
                }
            }
            Activity::Explore => 0.25 + 0.9 * (1.0 - needs.fun) * needs.energy,
            Activity::Chase => {
                let close = world.nearest_mote().map_or(0.0, |mote| {
                    (1.5 - mote.position.distance(world.ghost.position)).max(0.0) * 0.3
                });
                0.2 + 1.1 * (1.0 - needs.energy) + 0.4 * (1.0 - needs.fun) + close
            }
            Activity::Rest => {
                2.2 * (1.0 - needs.energy).powi(2)
                    + if bedtime {
                        if present {
                            0.6
                        } else {
                            2.0 + world.bedtime_progress() + if world.drowsy() { 0.8 } else { 0.0 }
                        }
                    } else if night {
                        0.5
                    } else {
                        0.0
                    }
                    + if shelter { 0.6 } else { 0.0 }
                    + if world.stage() == Stage::Newborn {
                        0.4 + 0.8 * (1.0 - needs.energy)
                    } else {
                        0.0
                    }
            }
            Activity::Play => {
                let close = (1.5 - flat(world.ball.position - world.ghost.position).length())
                    .max(0.0)
                    * 0.15;
                0.1 + 0.9 * (1.0 - needs.fun) * needs.energy + close
            }
            Activity::Build => {
                (0.06
                    + 0.3 * (1.0 - needs.fun)
                    + 0.15 * world.life.temperament().curious
                    + if present { 0.1 } else { 0.0 })
                    * needs.energy
            }
            Activity::Tend => {
                1.6 * world.garden_thirst().powi(2)
                    + 4.0 * world.garden_droop()
                    + if world.seed_ready() { 0.8 } else { 0.0 }
            }
            Activity::Lamp if bedtime => {
                if world.lamp.on && !present {
                    1.2 + 2.0 * needs.energy
                } else {
                    0.04
                }
            }
            Activity::Lamp => match (night, world.lamp.on) {
                (true, false) if needs.energy > 0.3 => 0.9,
                (true, true) if needs.energy < 0.3 => 0.5,
                (false, true) => 0.35,
                // Daylight needs no lamp unless rain darkens the room.
                (false, false) if world.weather.rain(world.time) < 0.5 => 0.0,
                _ => 0.04,
            },
            Activity::Visit => {
                let present = if world.viewer_present() {
                    0.6 + 0.8 * world.life.bond
                } else {
                    0.0
                };
                // Alone, it looks out for the viewer now and then, but it
                // has a life of its own to get on with.
                present
                    + if world.viewer_present() { 1.3 } else { 0.6 } * (1.0 - needs.company)
                    + if morning && world.viewer_present() {
                        1.2
                    } else {
                        0.0
                    }
            }
            Activity::Wait => {
                let expectation = world
                    .life
                    .expectation(world.hour)
                    .max(world.life.expectation(world.hour + 1.0 / 3.0));
                (0.9 + 1.2 * expectation + 0.8 * world.life.bond)
                    * (1.0 - 0.3 * world.life.temperament().shy)
            }
            Activity::Window => {
                let outside = &world.weather;
                let weight = 0.2
                    + 0.5 * (1.0 - needs.fun)
                    + 0.6 * outside.rain(world.time)
                    + 0.6 * outside.snow(world.time)
                    + 2.5 * outside.rainbow(world.time)
                    + if morning { 1.8 } else { 0.0 }
                    + if world
                        .visitors
                        .bird
                        .as_ref()
                        .is_some_and(|bird| bird.noticed && bird.perched(world.time))
                    {
                        1.2
                    } else {
                        0.0
                    }
                    + if world.fireworks() { 1.5 } else { 0.0 }
                    + if world.meteor_shower().is_some()
                        && outside.sky == Sky::Clear
                        && !outside.storm
                    {
                        1.2
                    } else {
                        0.0
                    }
                    + if world.constellation_pending() {
                        0.35
                    } else {
                        0.0
                    }
                    + if world.focus.is_none() && world.passersby().iter().any(Option::is_some) {
                        0.4
                    } else {
                        0.0
                    }
                    + if world.chuseok_night() { 2.0 } else { 0.0 };
                weight * if shelter { 0.2 } else { 1.0 }
                    + if world.life.sill.is_some()
                        && !world
                            .visitors
                            .bird
                            .as_ref()
                            .is_some_and(|bird| bird.perched(world.time))
                    {
                        1.5
                    } else {
                        0.0
                    }
            }
            Activity::Fetch => {
                let thrown = world
                    .events
                    .iter()
                    .rev()
                    .find(|event| event.kind == EventKind::KickedBall)
                    .map_or(0.0, |event| {
                        (1.0 - (world.time - event.time) as f32 / 20.0).clamp(0.0, 1.0)
                    });
                // Unasked, it only now and then brings the viewer the ball.
                let playful = if thrown > 0.0 {
                    0.1 + 0.7 * (1.0 - needs.fun) + if world.viewer_present() { 0.5 } else { 0.0 }
                } else {
                    0.04 + 0.12 * (1.0 - needs.fun)
                };
                (1.5 * thrown + playful) * needs.energy
            }
            Activity::Pounce => {
                let annoyed = world
                    .events
                    .iter()
                    .rev()
                    .take_while(|event| world.time - event.time < 10.0)
                    .any(|event| matches!(event.kind, EventKind::Poked { count } if count >= 3));
                if !world.viewer.pointer_is_lively()
                    || world.ghost.asleep >= 0.3
                    || needs.energy <= 0.25
                    || matches!(world.ghost.mood, Mood::Sleepy | Mood::Grumpy)
                    || annoyed
                {
                    0.0
                } else {
                    let lively = world.viewer.liveliness.min(1.0);
                    let playful = if matches!(world.ghost.mood, Mood::Cheerful | Mood::Excited) {
                        0.5
                    } else {
                        0.0
                    };
                    (0.4 + 1.2 * lively + 0.8 * (1.0 - needs.fun) + playful) * needs.energy
                }
            }
            Activity::Dance => {
                let delighted = world
                    .events
                    .iter()
                    .rev()
                    .take_while(|event| world.time - event.time < 20.0)
                    .any(|event| {
                        matches!(
                            event.kind,
                            EventKind::Bloomed
                                | EventKind::Sprouted(_)
                                | EventKind::Flowered(_)
                                | EventKind::GameOver(Outcome::GhostWon { .. })
                                | EventKind::ExpectedArrival
                                | EventKind::Fetched
                                | EventKind::Milestone(_)
                                | EventKind::Celebrated(Occasion::NewYear)
                        )
                    });
                let rainbow = if world.ghost.saw_rainbow {
                    1.2 * world.weather.rainbow(world.time)
                } else {
                    0.0
                };
                let company = if world.viewer_present() && needs.company > 0.7 {
                    0.2
                } else {
                    0.0
                };
                (0.03
                    + rainbow
                    + if matches!(
                        world.occasion(),
                        Some(
                            Occasion::Birthday
                                | Occasion::ViewerBirthday
                                | Occasion::HundredDays
                                | Occasion::ThousandDays
                        )
                    ) {
                        0.6
                    } else {
                        0.0
                    }
                    + if delighted { 0.7 } else { 0.0 }
                    + company
                    + if needs.fun > 0.7 { 0.06 } else { 0.0 })
                    * needs.energy
            }
            Activity::Hide => {
                if world.viewer_present() {
                    0.1 + 0.6 * (1.0 - needs.fun) * needs.energy
                        + if world.occasion() == Some(Occasion::Halloween) {
                            0.5
                        } else {
                            0.0
                        }
                } else {
                    0.0
                }
            }
            Activity::Puzzle => {
                if !world.viewer_present() || world.focus.is_some() {
                    0.0
                } else {
                    let recent = world
                        .puzzle
                        .as_ref()
                        .and_then(|puzzle| puzzle.ended)
                        .is_some_and(|ended| world.time - ended < 40.0);
                    (0.08 + 0.25 * (1.0 - needs.fun) + 0.15 * world.life.temperament().playful)
                        * needs.energy
                        * if recent { 0.2 } else { 1.0 }
                }
            }
            Activity::Game => {
                if !world.viewer_present() {
                    0.0
                } else {
                    let recent = world
                        .game
                        .as_ref()
                        .and_then(|game| game.ended)
                        .is_some_and(|ended| world.time - ended < 20.0);
                    let playful = world.life.temperament().playful;
                    (0.15 + 0.5 * (1.0 - needs.fun) + 0.4 * playful + 0.3 * world.life.bond)
                        * needs.energy
                        * if recent { 0.3 } else { 1.0 }
                }
            }
            Activity::Doodle => {
                let fond = if world.viewer_present() && needs.company > 0.6 {
                    0.2
                } else {
                    0.0
                };
                (0.05 + 0.18 * (1.0 - needs.fun) + fond) * needs.energy
            }
            Activity::Seek => 0.0,
            Activity::Fold => {
                if !world.fold_is_open() {
                    0.0
                } else {
                    let nature = world.life.temperament();
                    let rain = if world.weather.sky != Sky::Clear {
                        1.5
                    } else {
                        1.0
                    };
                    let evening = if (17.0..22.0).contains(&world.hour) {
                        1.6
                    } else {
                        1.0
                    };
                    let caring = if world.crane_for_sad_viewer() {
                        1.7
                    } else {
                        1.0
                    };
                    0.01 * needs.energy
                        * (0.7 + nature.tender)
                        * (1.3 - 0.6 * nature.playful)
                        * rain
                        * evening
                        * caring
                }
            }
            Activity::Bubbles => {
                if !world.bubbles_are_open() {
                    0.0
                } else {
                    (0.012 + 0.035 * (1.0 - needs.fun))
                        * needs.energy
                        * if present { 1.6 } else { 1.0 }
                        * if !night && world.weather.sky == Sky::Clear {
                            1.25
                        } else {
                            1.0
                        }
                        * if world.window_open && world.wind.strength > 0.2 {
                            1.25
                        } else {
                            1.0
                        }
                }
            }
            Activity::Pin => {
                if world.pin_is_open() {
                    20.0
                } else {
                    0.0
                }
            }
            Activity::Tidy => {
                if world.tidy_is_open() {
                    let nature = world.life.temperament();
                    0.16 * needs.energy * (0.7 + nature.tender) * (1.4 - 0.6 * nature.playful)
                } else {
                    0.0
                }
            }
            // Quiet visits offer show and tell on their own timer.
            Activity::Show => 0.0,
            Activity::Together => {
                let fond = world
                    .together_partner()
                    .map_or(0.0, |partner| world.fondness_for(partner.id));
                let present = if present && !world.companion { 0.5 } else { 1.0 };
                (0.05 + 0.3 * (1.0 - needs.company) + 0.15 * (1.0 - needs.fun) + 0.15 * fond)
                    * needs.energy
                    * present
            }
            Activity::Yard => {
                if world.yard_is_open() {
                    let fair = if world.weather.sky == Sky::Clear {
                        1.0
                    } else {
                        0.6
                    };
                    let snowy = 0.5 * world.weather.snow_cover.clamp(0.0, 1.0);
                    let starry = if world.starry_night() { 0.4 } else { 0.0 };
                    let cat = if world
                        .visitors
                        .cat
                        .as_ref()
                        .is_some_and(|cat| cat.noticed && cat.settled(world.time))
                    {
                        1.6
                    } else {
                        0.0
                    };
                    (0.25 + 0.6 * (1.0 - needs.fun) + snowy + starry + cat) * needs.energy * fair
                } else {
                    0.0
                }
            }
        })
        .zip(options)
        .map(|(weight, activity)| {
            let quiet = if world.focus.is_some() {
                if crate::world::focus::seeks_attention(*activity) {
                    0.0
                } else if matches!(
                    activity,
                    Activity::Play | Activity::Build | Activity::Friend | Activity::Doodle
                ) {
                    0.15
                } else {
                    1.0
                }
            } else {
                1.0
            };
            let playful = if world.taking_break()
                && matches!(
                    activity,
                    Activity::Play
                        | Activity::Dance
                        | Activity::Game
                        | Activity::Fetch
                        | Activity::Puzzle
                ) {
                4.0
            } else {
                1.0
            };
            let wishing = if world
                .life
                .wish
                .as_ref()
                .is_some_and(|wishing| wishing.wish.serves(*activity, world))
            {
                1.4
            } else {
                1.0
            };
            let caring = if present && let Some(feeling) = world.recent_negative_feeling(1800.0) {
                match activity {
                    Activity::Pounce | Activity::Hide | Activity::Build | Activity::Music => 0.15,
                    Activity::Game | Activity::Puzzle
                        if feeling != crate::world::Feeling::Bored =>
                    {
                        0.15
                    }
                    Activity::Visit | Activity::Company => 1.35,
                    _ => 1.0,
                }
            } else {
                1.0
            };
            weight
                * caring
                * world.quirk_leaning(*activity)
                * world.cold_leaning(*activity)
                * wishing
                * quiet
                * playful
                * world.life.habit(*activity)
                * world.life.novelty(*activity)
                * world.life.temperament().leaning(*activity, world.life.bond)
        })
        .collect();
    let total: f32 = weights.iter().sum::<f32>().max(f32::MIN_POSITIVE);
    options
        .iter()
        .zip(weights)
        .map(|(activity, weight)| (*activity, weight / total))
        .collect()
}

/// Mixes a model's chances for each option with the ghost's own needs,
/// mostly trusting the model.
pub fn blend(model: &[(Activity, f32)], own: &[(Activity, f32)]) -> Vec<(Activity, f32)> {
    own.iter()
        .map(|(activity, chance)| {
            let suggested = model
                .iter()
                .find(|(candidate, _)| candidate == activity)
                .map_or(0.0, |(_, chance)| *chance);
            (*activity, 0.75 * suggested + 0.25 * chance)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_recent_negative_feeling_softens_noisy_choices_until_half_an_hour_passes() {
        let mut worried = World::new(3, 12.0);
        worried.hear("I'm sad");
        let mut ordinary = worried.clone();
        ordinary.life.feelings.clear();
        let options = [
            Activity::Visit,
            Activity::Hide,
            Activity::Build,
            Activity::Game,
        ];
        let normal = weigh(&ordinary, &options);
        let careful = weigh(&worried, &options);
        assert!(careful[0].1 > normal[0].1);
        for index in 1..options.len() {
            assert!(careful[index].1 < normal[index].1);
        }
        worried.time += 1801.0;
        worried.notice_viewer();
        assert_eq!(weigh(&worried, &options), normal);
        let mut bored = ordinary;
        bored.hear("I'm bored");
        assert!(weigh(&bored, &options)[3].1 > careful[3].1);
    }

    use super::*;

    #[test]
    fn reading_is_favored_during_focus_by_curiosity_and_tenderness_and_less_by_newborns() {
        let mut world = World::new(5, 12.0);
        let options = [Activity::Float, Activity::Read];
        let usual = chance(&weigh(&world, &options), Activity::Read);
        world.start_focus(25);
        assert!(chance(&weigh(&world, &options), Activity::Read) > usual);
        world.focus = None;
        world.life.nature = Some(crate::world::Nature {
            curious: 1.0,
            tender: 1.0,
            ..Default::default()
        });
        assert!(chance(&weigh(&world, &options), Activity::Read) > usual);
        world.birthday = Some(world.date);
        assert!(chance(&weigh(&world, &options), Activity::Read) < usual);
    }

    #[test]
    fn rereading_finished_books_and_a_day_of_pages_draw_the_ghost_to_read_less() {
        let mut world = World::new(5, 12.0);
        let options = [Activity::Float, Activity::Read];
        let usual = chance(&weigh(&world, &options), Activity::Read);
        world.life.today.pages = 24;
        let after_pages = chance(&weigh(&world, &options), Activity::Read);
        assert!(after_pages < usual);
        world.life.today.pages = 0;
        for book in &mut world.life.books.owned {
            book.finished = 1;
        }
        assert!(chance(&weigh(&world, &options), Activity::Read) < usual);
    }

    #[test]
    fn focus_favors_company_and_a_finished_session_favors_play_for_five_minutes() {
        let mut world = World::new(1, 12.0);
        world.plant.thirst = 0.7;
        world.start_focus(25);
        let options = world.options();
        let focused = weigh(&world, &options);
        assert!(chance(&focused, Activity::Company) > 0.7);
        for activity in [
            Activity::Float,
            Activity::Tend,
            Activity::Window,
            Activity::Lamp,
            Activity::Explore,
        ] {
            assert!(chance(&focused, activity) > 0.0);
        }
        world.time = 1500.0;
        world.end_focus();
        let options = [
            Activity::Float,
            Activity::Play,
            Activity::Dance,
            Activity::Game,
            Activity::Fetch,
        ];
        let playful = weigh(&world, &options);
        world.time += 300.0;
        let usual = weigh(&world, &options);
        for activity in [
            Activity::Play,
            Activity::Dance,
            Activity::Game,
            Activity::Fetch,
        ] {
            let ratio = |weights: &[(Activity, f32)]| {
                chance(weights, activity) / chance(weights, Activity::Float)
            };
            assert!(ratio(&playful) >= ratio(&usual) * 3.9, "{activity:?}");
        }
    }

    #[test]
    fn clear_chuseok_evenings_pull_the_ghost_toward_the_window() {
        let mut world = World::new(3, 21.0);
        world.date = jiff::civil::Date::constant(2026, 9, 25);
        let options = [Activity::Float, Activity::Window];
        let clear = weigh(&world, &options)[1].1;
        world.weather.sky = crate::world::Sky::Rain;
        let rain = weigh(&world, &options)[1].1;
        world.weather.sky = crate::world::Sky::Clear;
        world.hour = 12.0;
        let daytime = weigh(&world, &options)[1].1;
        world.hour = 21.0;
        world.date = jiff::civil::Date::constant(2026, 9, 28);
        let ordinary = weigh(&world, &options)[1].1;
        assert!(clear > rain && clear > daytime && clear > ordinary);
    }
    use crate::world::{Event, Sky};
    use glam::Vec3;

    fn chance(weights: &[(Activity, f32)], activity: Activity) -> f32 {
        weights
            .iter()
            .find(|(candidate, _)| *candidate == activity)
            .map_or(0.0, |(_, chance)| *chance)
    }

    #[test]
    fn a_newborn_is_more_likely_to_take_a_nap() {
        let mut world = World::new(1, 12.0);
        world.ghost.needs.energy = 0.6;
        let options = [Activity::Rest, Activity::Float];
        let grown = weigh(&world, &options)[0].1;
        world.birthday = Some(world.date);
        assert!(weigh(&world, &options)[0].1 > grown);
    }

    #[test]
    fn games_need_company_favor_play_and_pause_before_asking_again() {
        let mut world = World::new(2, 12.0);
        let options = [Activity::Float, Activity::Game];
        assert_eq!(chance(&weigh(&world, &options), Activity::Game), 0.0);
        world.notice_viewer();
        let usual = chance(&weigh(&world, &options), Activity::Game);
        world.ghost.needs.fun = 0.0;
        assert!(chance(&weigh(&world, &options), Activity::Game) > usual);
        world.ghost.needs.fun = 0.7;
        world.life.bond = 0.8;
        assert!(chance(&weigh(&world, &options), Activity::Game) > usual);
        world.life.bond = 0.0;
        world.life.nature.as_mut().unwrap().playful = 1.0;
        assert!(chance(&weigh(&world, &options), Activity::Game) > usual);
        world.begin(Activity::Game);
        world.begin(Activity::Float);
        let recent = chance(&weigh(&world, &options), Activity::Game);
        world.time += 20.1;
        assert!(chance(&weigh(&world, &options), Activity::Game) > recent);
    }

    #[test]
    fn chances_add_up_to_one() {
        let world = World::new(1, 12.0);
        let weights = weigh(&world, &world.options());
        let total: f32 = weights.iter().map(|(_, chance)| chance).sum();
        assert!((total - 1.0).abs() < 1e-5);
    }

    #[test]
    fn a_playful_nature_gives_play_a_larger_chance_in_the_same_world() {
        let mut world = World::new(1, 12.0);
        let options = world.options();
        world.life.nature = Some(crate::world::Nature {
            playful: 0.0,
            ..Default::default()
        });
        let quiet = weigh(&world, &options);
        world.life.nature.as_mut().unwrap().playful = 1.0;
        let playful = weigh(&world, &options);
        assert!(chance(&playful, Activity::Play) > chance(&quiet, Activity::Play));
    }

    #[test]
    fn a_darting_pointer_strongly_draws_a_playful_ghost_to_pounce() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        world.point_at(Some(Vec3::new(-0.6, 1.2, 1.4)));
        world.point_at(Some(Vec3::new(0.6, 1.2, 1.4)));
        let options = world.options();
        let weights = weigh(&world, &options);
        assert!(chance(&weights, Activity::Pounce) > chance(&weights, Activity::Float) * 3.0);
        assert_eq!(
            weights
                .iter()
                .max_by(|first, second| first.1.total_cmp(&second.1))
                .unwrap()
                .0,
            Activity::Pounce
        );
        world.ghost.needs.fun = 0.1;
        let bored = weigh(&world, &options);
        let pull = |weights: &[(Activity, f32)]| {
            chance(weights, Activity::Pounce) / chance(weights, Activity::Float)
        };
        assert!(pull(&bored) > pull(&weights));
        for mood in [Mood::Cheerful, Mood::Excited] {
            world.ghost.mood = mood;
            assert!(
                chance(&weigh(&world, &options), Activity::Pounce)
                    > chance(&bored, Activity::Pounce)
            );
        }
        for mood in [Mood::Sleepy, Mood::Grumpy] {
            world.ghost.mood = mood;
            assert_eq!(chance(&weigh(&world, &options), Activity::Pounce), 0.0);
        }
        world.ghost.mood = Mood::Content;
        for _ in 0..3 {
            world.poke();
        }
        assert_eq!(chance(&weigh(&world, &options), Activity::Pounce), 0.0);
        world.step(11.0);
        assert_eq!(chance(&weigh(&world, &options), Activity::Pounce), 0.0);
    }

    #[test]
    fn a_tired_ghost_leans_toward_rest_and_food() {
        let mut world = World::new(1, 23.0);
        world.drop_mote(Vec3::new(0.0, 1.0, 0.0));
        let rested = weigh(&world, &world.options());
        world.ghost.needs.energy = 0.1;
        let tired = weigh(&world, &world.options());
        assert!(chance(&tired, Activity::Rest) > chance(&rested, Activity::Rest));
        assert!(chance(&tired, Activity::Rest) > chance(&tired, Activity::Play));
        assert!(chance(&tired, Activity::Chase) > chance(&tired, Activity::Explore));
    }

    #[test]
    fn bedtime_makes_rest_a_strong_favorite_and_company_softens_the_pull() {
        let mut world = World::new(1, 23.0);
        world.ghost.needs.energy = 0.1;
        let alone = weigh(&world, &world.options());
        let rest = chance(&alone, Activity::Rest);
        for (activity, weight) in &alone {
            if *activity != Activity::Rest {
                assert!(rest > 2.0 * weight, "{activity:?}: {weight}");
            }
        }
        world.notice_viewer();
        let together = weigh(&world, &world.options());
        assert!(chance(&together, Activity::Rest) < rest);
    }

    #[test]
    fn fresh_air_draws_a_bored_or_tired_ghost_and_bad_weather_demands_shutting() {
        let mut world = World::new(7, 12.0);
        let options = [Activity::Float, Activity::Air];
        let ordinary = chance(&weigh(&world, &options), Activity::Air);
        world.ghost.needs.fun = 0.1;
        world.ghost.needs.energy = 0.3;
        assert!(chance(&weigh(&world, &options), Activity::Air) > ordinary);
        world.window_open = true;
        assert_eq!(chance(&weigh(&world, &options), Activity::Air), 0.0);
        for reason in 0..4 {
            world.hour = 12.0;
            world.weather.sky = Sky::Clear;
            world.wind.strength = 0.2;
            match reason {
                0 => world.weather.sky = Sky::Rain,
                1 => world.weather.sky = Sky::Snow,
                2 => world.hour = 22.0,
                _ => world.wind.strength = 0.9,
            }
            assert!(world.options().contains(&Activity::Air));
            assert!(chance(&weigh(&world, &options), Activity::Air) > 0.95);
            world.window_open = false;
            assert!(!world.options().contains(&Activity::Air));
            world.window_open = true;
        }
    }

    #[test]
    fn rest_grows_more_inviting_later_in_bedtime_and_after_a_quiet_interruption() {
        let mut world = World::new(1, 22.0);
        let options = [Activity::Float, Activity::Rest];
        let early = chance(&weigh(&world, &options), Activity::Rest);
        world.hour = 2.0;
        let late = chance(&weigh(&world, &options), Activity::Rest);
        assert!(late > early);
        world.tuck_in();
        world.poke();
        let drowsy = chance(&weigh(&world, &options), Activity::Rest);
        assert!(drowsy > late);
        world.notice_viewer();
        let company = chance(&weigh(&world, &options), Activity::Rest);
        assert!(company < drowsy);
        world.time += 91.0;
        assert!(!world.viewer_present());
        assert_eq!(chance(&weigh(&world, &options), Activity::Rest), drowsy);
    }

    #[test]
    fn a_bright_day_needs_no_lamp_but_a_rainy_one_might() {
        let mut world = World::new(1, 12.0);
        world.lamp.on = false;
        let options = world.options();
        assert_eq!(chance(&weigh(&world, &options), Activity::Lamp), 0.0);
        world.weather.sky = crate::world::Sky::Rain;
        world.weather.since = world.time - 100.0;
        world.weather.until = world.time + 500.0;
        assert!(chance(&weigh(&world, &options), Activity::Lamp) > 0.0);
    }

    #[test]
    fn the_bedtime_lamp_is_left_for_the_viewer_and_switched_off_when_alone() {
        let mut world = World::new(1, 22.0);
        let options = [Activity::Float, Activity::Lamp, Activity::Rest];
        let alone = weigh(&world, &options);
        assert!(chance(&alone, Activity::Lamp) > chance(&alone, Activity::Rest));
        world.notice_viewer();
        let together = weigh(&world, &options);
        assert!(chance(&together, Activity::Lamp) < chance(&alone, Activity::Lamp) * 0.1);
        world.lamp.on = false;
        assert_eq!(weigh(&world, &options), together);
        world.viewer.active_at = 0.0;
        let dark = weigh(&world, &options);
        assert!(chance(&dark, Activity::Lamp) < chance(&alone, Activity::Lamp) * 0.1);
        world.hour = 12.0;
        world.lamp.on = true;
        let daytime = weigh(&world, &options);
        world.lamp.on = false;
        assert!(
            chance(&weigh(&world, &options), Activity::Lamp) < chance(&daytime, Activity::Lamp)
        );
    }

    #[test]
    fn mornings_draw_the_ghost_to_the_window_and_to_a_present_viewer() {
        let mut world = World::new(1, 6.5);
        let options = [Activity::Float, Activity::Window, Activity::Visit];
        let ordinary = weigh(&world, &options);
        world.hour = 6.49;
        world.tuck_in();
        world.hour = 6.5;
        world.step(0.1);
        let morning = weigh(&world, &options);
        let ratio = |weights: &[(Activity, f32)], activity| {
            chance(weights, activity) / chance(weights, Activity::Float)
        };
        assert!(ratio(&morning, Activity::Window) > ratio(&ordinary, Activity::Window));
        assert!(
            (ratio(&morning, Activity::Visit) - ratio(&ordinary, Activity::Visit)).abs() < 0.001
        );
        world.notice_viewer();
        let together = weigh(&world, &options);
        world.time += 600.0;
        world.notice_viewer();
        let later = weigh(&world, &options);
        assert!(ratio(&together, Activity::Visit) > ratio(&later, Activity::Visit));
        assert!(ratio(&together, Activity::Window) > ratio(&later, Activity::Window));
    }

    #[test]
    fn a_thirsty_plant_draws_the_ghost_to_tend_it() {
        let mut world = World::new(1, 12.0);
        world.plant.thirst = 1.0;
        let weights = weigh(&world, &world.options());
        let top = weights
            .iter()
            .max_by(|first, second| first.1.total_cmp(&second.1))
            .unwrap();
        assert_eq!(top.0, Activity::Tend);
    }

    #[test]
    fn habits_tip_the_balance_toward_what_the_ghost_often_does() {
        let mut world = World::new(1, 12.0);
        let fresh = weigh(&world, &world.options());
        for _ in 0..50 {
            world.life.note(&EventKind::Began(Activity::Window));
        }
        let habitual = weigh(&world, &world.options());
        assert!(chance(&habitual, Activity::Window) > chance(&fresh, Activity::Window) * 1.15);
    }

    #[test]
    fn a_close_bond_draws_the_ghost_to_the_viewer() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        let stranger = weigh(&world, &world.options());
        world.life.bond = 0.9;
        let friend = weigh(&world, &world.options());
        assert!(chance(&friend, Activity::Visit) > chance(&stranger, Activity::Visit));
    }

    #[test]
    fn a_storm_draws_an_unfamiliar_ghost_to_the_cushion_and_away_from_the_window() {
        let mut world = World::new(1, 12.0);
        world.ghost.needs.energy = 0.7;
        world.time = 40.0;
        world.weather.sky = Sky::Rain;
        world.weather.until = 300.0;
        let options = world.options();
        let rain = weigh(&world, &options);
        world.weather.storm = true;
        let storm = weigh(&world, &options);
        assert!(chance(&storm, Activity::Rest) > chance(&rain, Activity::Rest));
        assert!(chance(&storm, Activity::Window) < chance(&rain, Activity::Window));
        assert!(chance(&storm, Activity::Rest) > chance(&storm, Activity::Window));
        world.life.bond = GREETS_FROM;
        assert_eq!(weigh(&world, &options), rain);
    }

    #[test]
    fn a_just_flicked_ball_draws_the_ghost_to_fetch_and_the_pull_fades() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        world.kick_ball(Vec3::X);
        let options = world.options();
        let thrown = weigh(&world, &options);
        let top = thrown
            .iter()
            .max_by(|first, second| first.1.total_cmp(&second.1))
            .unwrap();
        assert_eq!(top.0, Activity::Fetch);
        world.time += 20.0;
        let later = weigh(&world, &options);
        assert!(chance(&later, Activity::Fetch) < chance(&thrown, Activity::Fetch));
    }

    #[test]
    fn a_rainbow_raises_dance_only_after_the_ghost_has_seen_it() {
        let mut world = World::new(1, 12.0);
        world.time = 20.0;
        let options = world.options();
        let clear = weigh(&world, &options);
        world.weather.sky = Sky::Rainbow;
        world.weather.until = 100.0;
        let unseen = weigh(&world, &options);
        world.ghost.saw_rainbow = true;
        let seen = weigh(&world, &options);
        assert!(chance(&unseen, Activity::Dance) <= chance(&clear, Activity::Dance));
        assert!(chance(&seen, Activity::Dance) > chance(&clear, Activity::Dance));
        assert!(chance(&seen, Activity::Dance) > chance(&unseen, Activity::Dance));
    }

    #[test]
    fn a_mote_is_an_everyday_snack_and_no_reason_to_dance() {
        let mut world = World::new(1, 12.0);
        let options = world.options();
        let before = weigh(&world, &options);
        world.events.push(Event {
            time: world.time,
            kind: EventKind::AteMote,
        });
        assert_eq!(
            chance(&weigh(&world, &options), Activity::Dance),
            chance(&before, Activity::Dance)
        );
    }

    #[test]
    fn good_things_raise_dance_for_a_little_while() {
        for kind in [
            EventKind::Bloomed,
            EventKind::Fetched,
            EventKind::GameOver(Outcome::GhostWon { line: [0, 1, 2] }),
        ] {
            let mut world = World::new(1, 12.0);
            let options = world.options();
            let before = weigh(&world, &options);
            world.events.push(Event {
                time: world.time,
                kind,
            });
            let delighted = weigh(&world, &options);
            assert!(chance(&delighted, Activity::Dance) > chance(&before, Activity::Dance));
            world.time += 21.0;
            let later = weigh(&world, &options);
            assert_eq!(
                chance(&later, Activity::Dance),
                chance(&before, Activity::Dance)
            );
        }
    }

    #[test]
    fn blending_mostly_follows_the_model() {
        let own = [(Activity::Rest, 0.9), (Activity::Play, 0.1)];
        let model = [(Activity::Rest, 0.0), (Activity::Play, 1.0)];
        let blended = blend(&model, &own);
        assert!(chance(&blended, Activity::Play) > chance(&blended, Activity::Rest));
        let total: f32 = blended.iter().map(|(_, chance)| chance).sum();
        assert!((total - 1.0).abs() < 1e-5);
    }

    #[test]
    fn hiding_needs_company_and_doodling_draws_on_fondness_and_boredom() {
        let mut world = World::new(1, 12.0);
        let options = [Activity::Float, Activity::Hide, Activity::Doodle];
        let alone = weigh(&world, &options);
        assert_eq!(chance(&alone, Activity::Hide), 0.0);
        world.notice_viewer();
        world.ghost.needs.company = 0.8;
        let together = weigh(&world, &options);
        let ratio = |weights: &[(Activity, f32)], activity| {
            chance(weights, activity) / chance(weights, Activity::Float)
        };
        assert!(ratio(&together, Activity::Hide) > 0.0);
        assert!(ratio(&together, Activity::Doodle) > ratio(&alone, Activity::Doodle));
        world.ghost.needs.fun = 0.0;
        let bored = weigh(&world, &options);
        assert!(ratio(&bored, Activity::Hide) > ratio(&together, Activity::Hide));
        assert!(ratio(&bored, Activity::Doodle) > ratio(&together, Activity::Doodle));
    }

    #[test]
    fn a_perched_bird_draws_the_ghost_to_the_window_only_after_it_has_been_noticed() {
        use crate::world::{Random, visitor::Bird};

        let mut world = World::new(1, 12.0);
        let options = [Activity::Float, Activity::Window];
        let ordinary = weigh(&world, &options);
        let mut bird = Bird::new(0.0, 0.0, &mut Random::new(1));
        world.time = 5.0;
        world.visitors.bird = Some(bird);
        assert_eq!(weigh(&world, &options), ordinary);
        bird.noticed = true;
        world.visitors.bird = Some(bird);
        assert!(
            chance(&weigh(&world, &options), Activity::Window)
                > chance(&ordinary, Activity::Window)
        );
        world.time = 0.0;
        assert_eq!(weigh(&world, &options), ordinary);
    }

    #[test]
    fn a_waiting_gift_invites_a_window_visit_once_the_bird_is_gone() {
        use crate::world::{Keepsake, Random, visitor::Bird};

        let mut world = World::new(1, 12.0);
        let options = [Activity::Float, Activity::Window];
        let ordinary = weigh(&world, &options);
        world.life.sill = Some(Keepsake::Petal);
        assert!(
            chance(&weigh(&world, &options), Activity::Window)
                > chance(&ordinary, Activity::Window)
        );
        assert!(world.describe(Activity::Window).contains("a blossom petal"));
        world.time = 5.0;
        world.visitors.bird = Some(Bird::new(0.0, 0.8, &mut Random::new(1)));
        assert_eq!(weigh(&world, &options), ordinary);
    }

    #[test]
    fn special_days_draw_the_ghost_to_hiding_dancing_and_fireworks() {
        use jiff::civil::Date;

        let mut world = World::new(1, 23.5);
        world.notice_viewer();
        let options = [
            Activity::Float,
            Activity::Hide,
            Activity::Dance,
            Activity::Window,
        ];
        let ordinary = weigh(&world, &options);
        let ratio = |weights: &[(Activity, f32)], activity| {
            chance(weights, activity) / chance(weights, Activity::Float)
        };
        world.date = Date::constant(2026, 10, 31);
        assert!(ratio(&weigh(&world, &options), Activity::Hide) > ratio(&ordinary, Activity::Hide));
        world.date = Date::constant(2027, 3, 1);
        world.birthday = Some(Date::constant(2026, 3, 1));
        assert!(
            ratio(&weigh(&world, &options), Activity::Dance) > ratio(&ordinary, Activity::Dance)
        );
        for elapsed in [99, 999] {
            let born = Date::constant(2026, 3, 3);
            world.birthday = Some(born);
            world.date = born.checked_add(jiff::Span::new().days(elapsed)).unwrap();
            assert!(
                ratio(&weigh(&world, &options), Activity::Dance)
                    > ratio(&ordinary, Activity::Dance)
            );
        }
        world.date = Date::constant(2026, 12, 31);
        assert!(
            ratio(&weigh(&world, &options), Activity::Window) > ratio(&ordinary, Activity::Window)
        );
        world.hour = 23.499;
        assert_eq!(
            ratio(&weigh(&world, &options), Activity::Window),
            ratio(&ordinary, Activity::Window)
        );
    }
    #[test]
    fn a_visiting_friend_draws_play_more_when_lonely_and_less_when_tired_or_with_a_close_viewer() {
        let mut world = World::new(2, 20.0);
        world.date = jiff::civil::Date::constant(2026, 10, 25);
        world.life.wish = Some(crate::world::Wishing {
            wish: crate::world::Wish::Tower { height: 3 },
            day: world.date.to_string(),
            said: None,
        });
        world.step(0.1);
        world.invite_friend();
        let options = [Activity::Float, Activity::Friend];
        let usual = chance(&weigh(&world, &options), Activity::Friend);
        assert!(usual > 0.85);
        world.ghost.needs.company = 0.0;
        assert!(chance(&weigh(&world, &options), Activity::Friend) > usual);
        world.ghost.needs.energy = 0.2;
        assert!(chance(&weigh(&world, &options), Activity::Friend) < usual);
        world.ghost.needs.energy = 0.85;
        world.ghost.needs.company = 0.6;
        let alone = chance(&weigh(&world, &options), Activity::Friend);
        world.notice_viewer();
        world.life.bond = 0.9;
        assert!(chance(&weigh(&world, &options), Activity::Friend) < alone);
    }

    #[test]
    fn ripe_seeds_and_drooping_pots_pull_the_ghost_toward_tending() {
        let mut world = World::new(1, 12.0);
        let options = [Activity::Float, Activity::Tend];
        let ordinary = chance(&weigh(&world, &options), Activity::Tend);
        world.plant.seeds = 1;
        let planting = chance(&weigh(&world, &options), Activity::Tend);
        assert!(planting > ordinary);
        world.plant.seeds = 0;
        let mut pot = crate::world::Seedling::new(crate::world::Flower::Sunflower);
        pot.thirst = 1.0;
        world.seedlings.push(pot);
        let thirsty = chance(&weigh(&world, &options), Activity::Tend);
        world.seedlings[0].droop = 1.0;
        let urgent = chance(&weigh(&world, &options), Activity::Tend);
        assert!(urgent > thirsty);
        assert!(urgent > planting);
    }
    #[test]
    fn a_held_wish_moderately_favors_what_serves_it_and_keeps_focus_quiet() {
        use crate::world::{Wish, Wishing};
        let mut world = World::new(17, 12.0);
        world.notice_viewer();
        let options = [Activity::Float, Activity::Build];
        let before = weigh(&world, &options);
        world.life.wish = Some(Wishing {
            wish: Wish::Tower { height: 3 },
            day: world.date.to_string(),
            said: None,
        });
        let after = weigh(&world, &options);
        assert!(after[1].1 > before[1].1);
        assert!((after[1].1 / after[0].1 / (before[1].1 / before[0].1) - 1.4).abs() < 0.0001);
        world.start_focus(5);
        world.life.wish.as_mut().unwrap().wish = Wish::Game;
        assert!(!world.options().contains(&Activity::Visit));
        assert_eq!(
            weigh(&world, &[Activity::Visit, Activity::Company])[0].1,
            0.0
        );
    }

    #[test]
    fn cranes_favor_tender_patient_rainy_evenings_and_a_sad_viewer() {
        let mut world = World::new(7, 12.0);
        world.begin_life(&world.date.to_string());
        world.life.nature = Some(crate::world::Nature {
            tender: 0.5,
            playful: 0.5,
            ..Default::default()
        });
        world.weather.sky = Sky::Clear;
        let options = [Activity::Float, Activity::Fold];
        let usual = chance(&weigh(&world, &options), Activity::Fold);
        world.life.nature.as_mut().unwrap().tender = 0.9;
        world.life.nature.as_mut().unwrap().playful = 0.1;
        assert!(chance(&weigh(&world, &options), Activity::Fold) > usual);
        world.life.nature.as_mut().unwrap().tender = 0.5;
        world.life.nature.as_mut().unwrap().playful = 0.5;
        world.weather.sky = Sky::Rain;
        assert!(chance(&weigh(&world, &options), Activity::Fold) > usual);
        world.weather.sky = Sky::Clear;
        world.hour = 19.0;
        assert!(chance(&weigh(&world, &options), Activity::Fold) > usual);
        world.hour = 12.0;
        world.hear("I'm sad");
        assert!(chance(&weigh(&world, &options), Activity::Fold) > usual);
        world.birthday = Some(world.date);
        assert_eq!(chance(&weigh(&world, &options), Activity::Fold), 0.0);
    }

    #[test]
    fn bubble_instinct_favors_an_audience_sun_breeze_and_playful_curiosity_but_tires_of_repeats() {
        use crate::world::{Nature, Understanding};
        let chance = |world: &World| weigh(world, &[Activity::Float, Activity::Bubbles])[1].1;
        let mut world = World::new(3, 12.0);
        world.life.nature = Some(Nature::default());
        world.weather.sky = Sky::Rain;
        let ordinary = chance(&world);
        world.time = 1.0;
        world.notice_viewer();
        assert!(chance(&world) > ordinary);
        let audience = chance(&world);
        world.weather.sky = Sky::Clear;
        assert!(chance(&world) > audience);
        let sunny = chance(&world);
        world.window_open = true;
        world.wind.strength = 0.5;
        assert!(chance(&world) > sunny);
        let breezy = chance(&world);
        world.life.nature = Some(Nature {
            playful: 1.0,
            curious: 1.0,
            ..Nature::default()
        });
        assert!(chance(&world) > breezy);
        let fresh = chance(&world);
        for _ in 0..4 {
            world.life.note(&EventKind::Began(Activity::Bubbles));
        }
        assert!(chance(&world) < fresh);
        assert!(!world.options().contains(&Activity::Bubbles));
        // A viewer request may still start a fifth session.
        assert_eq!(
            world.hear("bubbles!").understanding,
            Understanding::Bubbles { accepted: true }
        );
    }

    #[test]
    fn a_tender_quiet_nature_favors_evening_tidying_but_low_energy_favors_sleep() {
        let mut world = World::new(7, 21.5);
        world.ghost.needs.energy = 0.8;
        let options = [Activity::Float, Activity::Rest, Activity::Tidy];
        let usual = chance(&weigh(&world, &options), Activity::Tidy);
        let nature = world.life.nature.as_mut().unwrap();
        nature.tender = 1.0;
        nature.playful = 0.0;
        let tender = chance(&weigh(&world, &options), Activity::Tidy);
        let nature = world.life.nature.as_mut().unwrap();
        nature.tender = 0.0;
        nature.playful = 1.0;
        let playful = chance(&weigh(&world, &options), Activity::Tidy);
        assert!(tender > usual && tender > playful);
        world.ghost.needs.energy = 0.1;
        assert_eq!(chance(&weigh(&world, &options), Activity::Tidy), 0.0);
        assert!(chance(&weigh(&world, &options), Activity::Rest) > 0.5);
    }

    #[test]
    fn a_little_cold_damps_play_and_favors_rest_and_reading_until_recovery() {
        let mut world = World::new(1, 12.0);
        let choices = [Activity::Play, Activity::Read, Activity::Rest];
        let healthy = weigh(&world, &choices);
        world.life.cold = Some(crate::world::cold::Cold {
            day: world.date.to_string(),
            hour: 12.0,
            severity: 0.5,
            care: Vec::new(),
            remaining: 80_000.0,
        });
        let sick = weigh(&world, &choices);
        assert!(sick[0].1 < healthy[0].1);
        assert!(sick[1].1 > healthy[1].1);
        assert!(sick[2].1 > healthy[2].1);
        world.elapse_cold(100_000.0, false);
        assert_eq!(weigh(&world, &choices), healthy);
    }
}
