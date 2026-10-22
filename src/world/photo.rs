use super::{Activity, EventKind, Mood, World};

impl World {
    /// The viewer is about to take its portrait: an awake ghost that isn't
    /// busy comes to the glass, turns to them, and smiles for the picture.
    /// Returns whether it is posing; a sleeper or a ghost in the middle of
    /// something is pictured as it is.
    pub fn pose_for_photo(&mut self) -> bool {
        if !self.can_be_photographed() {
            return false;
        }
        let plan = &self.ghost.plan;
        if plan.done
            || matches!(
                plan.activity,
                Activity::Float | Activity::Explore | Activity::Window | Activity::Wait
            )
        {
            self.begin(Activity::Visit);
        }
        self.attend("viewer", 4.0);
        self.suggest_mood(Mood::Cheerful, 8.0);
        let line = self.spoken("Cheese!", "김치~!");
        self.say(line);
        true
    }

    /// A picture was just taken, posed for or caught in the moment. The
    /// first one, and a caught one now and then, earns a glance and a word.
    pub fn photographed(&mut self, posed: bool) {
        let first = self.life.total.photos == 0;
        self.record(EventKind::Photographed { posed });
        if posed || !self.can_be_photographed() {
            return;
        }
        if !first && self.random.unit() >= 1.0 / 3.0 {
            return;
        }
        self.attend("viewer", 3.0);
        let nature = self.life.temperament();
        let (english, korean) = if first {
            ("Did you just take my picture?", "방금 내 사진 찍었어?")
        } else if nature.shy > 0.6 {
            ("Eep! A picture?", "앗! 사진 찍었어?")
        } else if nature.playful > 0.6 {
            ("Get my good side!", "예쁘게 찍어 줘!")
        } else {
            ("Oh! A picture of me?", "어! 내 사진이야?")
        };
        let line = self.spoken(english, korean);
        self.say(line);
    }

    // Awake and free to look at the camera: not asleep, hiding, or in the
    // middle of a story or a hunt.
    fn can_be_photographed(&self) -> bool {
        self.ghost.asleep < 0.5
            && self.telling.is_none()
            && self.hunt.is_none()
            && !(self.ghost.plan.activity == Activity::Hide && !self.ghost.plan.done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_free_ghost_poses_at_the_glass_and_a_sleeper_is_pictured_as_it_is() {
        let mut world = World::new(4, 12.0);
        world.begin(Activity::Explore);
        assert!(world.pose_for_photo());
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Cheese!");

        let mut world = World::new(4, 23.0);
        world.tuck_in();
        world.step(30.0);
        assert!(!world.pose_for_photo());
        assert!(world.ghost.asleep > 0.5);
        assert!(world.ghost.speech.is_none());
    }

    #[test]
    fn a_busy_ghost_smiles_without_leaving_what_it_is_doing() {
        let mut world = World::new(4, 12.0);
        world.begin(Activity::Build);
        assert!(world.pose_for_photo());
        assert_eq!(world.ghost.plan.activity, Activity::Build);
        world.life.korean = 1.0;
        world.pose_for_photo();
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "김치~!");
    }

    #[test]
    fn the_first_picture_is_noticed_and_every_picture_counts() {
        let mut world = World::new(4, 12.0);
        world.photographed(false);
        assert_eq!(
            world.ghost.speech.as_ref().unwrap().text,
            "Did you just take my picture?"
        );
        for _ in 0..5 {
            world.photographed(true);
        }
        assert_eq!(world.life.total.photos, 6);
        assert_eq!(world.life.total.portraits, 5);
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "You took my picture for the first time.")
        );
    }
}
