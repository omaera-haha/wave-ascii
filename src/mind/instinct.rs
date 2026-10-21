use crate::world::{Activity, World, flat};

/// How much the ghost's needs, surroundings, and habits pull it toward each
/// of the activities open to it, as chances that add up to 1. This is the mind it
/// has without any model, and the weight it gives its own needs when a model
/// does choose.
pub fn weigh(world: &World, options: &[Activity]) -> Vec<(Activity, f32)> {
    let needs = world.ghost.needs;
    let night = world.is_night();
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
            Activity::Rest => 2.2 * (1.0 - needs.energy).powi(2) + if night { 0.5 } else { 0.0 },
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
                0.2 + 0.5 * (1.0 - needs.fun)
                    + 0.6 * outside.rain(world.time)
                    + 2.5 * outside.rainbow(world.time)
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
    use crate::world::EventKind;
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
    fn blending_mostly_follows_the_model() {
        let own = [(Activity::Rest, 0.9), (Activity::Play, 0.1)];
        let model = [(Activity::Rest, 0.0), (Activity::Play, 1.0)];
        let blended = blend(&model, &own);
        assert!(chance(&blended, Activity::Play) > chance(&blended, Activity::Rest));
        let total: f32 = blended.iter().map(|(_, chance)| chance).sum();
        assert!((total - 1.0).abs() < 1e-5);
    }
}
