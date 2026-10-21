use crate::world::{Activity, EventKind, GREETS_FROM, World, flat};

/// How much the ghost's needs, surroundings, and habits pull it toward each
/// of the activities open to it, as chances that add up to 1. This is the mind it
/// has without any model, and the weight it gives its own needs when a model
/// does choose.
pub fn weigh(world: &World, options: &[Activity]) -> Vec<(Activity, f32)> {
    let needs = world.ghost.needs;
    let night = world.is_night();
    let shelter = world.weather.storm && world.life.bond < GREETS_FROM;
    let weights: Vec<f32> = options
        .iter()
        .map(|activity| match activity {
            Activity::Float => 0.45,
            Activity::Explore => 0.25 + 0.9 * (1.0 - needs.fun) * needs.energy,
            Activity::Chase => {
                let close = world.nearest_mote().map_or(0.0, |mote| {
                    (1.5 - mote.position.distance(world.ghost.position)).max(0.0) * 0.3
                });
                0.2 + 1.1 * (1.0 - needs.energy) + 0.4 * (1.0 - needs.fun) + close
            }
            Activity::Rest => {
                2.2 * (1.0 - needs.energy).powi(2)
                    + if night { 0.5 } else { 0.0 }
                    + if shelter { 0.6 } else { 0.0 }
            }
            Activity::Play => {
                let close = (1.5 - flat(world.ball.position - world.ghost.position).length())
                    .max(0.0)
                    * 0.15;
                0.2 + 1.2 * (1.0 - needs.fun) * needs.energy + close
            }
            Activity::Tend => 1.6 * world.plant.thirst.powi(2),
            Activity::Lamp => match (night, world.lamp.on) {
                (true, false) if needs.energy > 0.3 => 0.9,
                (true, true) if needs.energy < 0.3 => 0.5,
                (false, true) => 0.35,
                _ => 0.04,
            },
            Activity::Visit => {
                let present = if world.viewer_present() {
                    0.6 + 0.8 * world.life.bond
                } else {
                    0.0
                };
                present + 1.3 * (1.0 - needs.company)
            }
            Activity::Window => {
                let outside = &world.weather;
                let weight = 0.2
                    + 0.5 * (1.0 - needs.fun)
                    + 0.6 * outside.rain(world.time)
                    + 0.6 * outside.snow(world.time)
                    + 2.5 * outside.rainbow(world.time);
                weight * if shelter { 0.2 } else { 1.0 }
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
                                | EventKind::Fetched
                                | EventKind::AteMote
                                | EventKind::Milestone(_)
                        )
                    });
                let rainbow = if world.ghost.saw_rainbow {
                    1.2 * world.weather.rainbow(world.time)
                } else {
                    0.0
                };
                let company = if world.viewer_present() && needs.company > 0.7 {
                    0.4
                } else {
                    0.0
                };
                (0.1 + rainbow
                    + if delighted { 0.7 } else { 0.0 }
                    + company
                    + if needs.fun > 0.7 { 0.3 } else { 0.0 })
                    * needs.energy
            }
            Activity::Hide => {
                if world.viewer_present() {
                    0.15 + 0.7 * (1.0 - needs.fun) * needs.energy
                } else {
                    0.0
                }
            }
            Activity::Doodle => {
                let fond = if world.viewer_present() && needs.company > 0.6 {
                    0.4
                } else {
                    0.0
                };
                (0.12 + 0.35 * (1.0 - needs.fun) + fond) * needs.energy
            }
        })
        .zip(options)
        .map(|(weight, activity)| weight * world.life.habit(*activity))
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
    use super::*;
    use crate::world::{Event, Sky};
    use glam::Vec3;

    fn chance(weights: &[(Activity, f32)], activity: Activity) -> f32 {
        weights
            .iter()
            .find(|(candidate, _)| *candidate == activity)
            .map_or(0.0, |(_, chance)| *chance)
    }

    #[test]
    fn chances_add_up_to_one() {
        let world = World::new(1, 12.0);
        let weights = weigh(&world, &world.options());
        let total: f32 = weights.iter().map(|(_, chance)| chance).sum();
        assert!((total - 1.0).abs() < 1e-5);
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
    fn good_things_raise_dance_for_a_little_while() {
        for kind in [EventKind::Bloomed, EventKind::Fetched, EventKind::AteMote] {
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
}
