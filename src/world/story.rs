use super::{Activity, EventKind, Mood, Thing, World};

#[derive(Clone, Debug, PartialEq)]
pub struct Telling {
    pub lines: Vec<String>,
    pub next: usize,
    pub next_at: f64,
    pub bedtime: bool,
}

impl World {
    /// Empty lines hold its place at the glass while a mind finds a story.
    pub fn tell(&mut self, lines: Vec<String>) {
        if self
            .telling
            .as_ref()
            .is_some_and(|telling| !telling.lines.is_empty())
        {
            return;
        }
        if self.telling.is_none() {
            self.notice_viewer();
        }
        let bedtime = self
            .telling
            .as_ref()
            .map_or_else(|| self.bedtime(), |telling| telling.bedtime);
        let lines: Vec<String> = lines
            .into_iter()
            .map(|line| line.trim().to_owned())
            .filter(|line| !line.is_empty())
            .collect();
        self.wake();
        if self.ghost.plan.activity != Activity::Visit || self.ghost.plan.done {
            self.begin(Activity::Visit);
        }
        self.telling = Some(Telling {
            next_at: if lines.is_empty() {
                f64::INFINITY
            } else {
                self.time
            },
            lines,
            next: 0,
            bedtime,
        });
    }

    pub(super) fn advance_story(&mut self) {
        if self.telling.is_none() {
            return;
        }
        if !self.viewer_present() || self.ghost.asleep >= 0.3 {
            self.telling = None;
            return;
        }
        if self.ghost.plan.activity != Activity::Visit || self.ghost.plan.done {
            self.begin(Activity::Visit);
        }
        self.ghost.attention = Some((Thing::Viewer, self.time + 1.0));
        let telling = self.telling.as_mut().unwrap();
        if self.time < telling.next_at {
            return;
        }
        if let Some(line) = telling.lines.get(telling.next).cloned() {
            telling.next += 1;
            telling.next_at = self.time + (3.5 + line.chars().count() as f64 * 0.05).min(7.0);
            self.say(&line);
            return;
        }
        let bedtime = telling.bedtime || self.bedtime();
        self.telling = None;
        self.record(EventKind::ToldStory);
        if bedtime {
            self.say(self.spoken("…goodnight.", "…잘 자."));
            self.suggest_mood(Mood::Sleepy, 8.0);
            self.begin(Activity::Rest);
            self.ghost.plan.overnight = true;
        } else {
            self.suggest_mood(Mood::Cheerful, 8.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listening(hour: f32) -> World {
        let mut world = World::new(7, hour);
        world.hear("tell me a story");
        world
    }

    #[test]
    fn sentences_are_spoken_in_order_with_time_to_read_and_count_only_on_completion() {
        let mut world = listening(12.0);
        world.tell(vec![
            "A light.".to_owned(),
            "A little light found a home in the glass box.".to_owned(),
            "The end.".to_owned(),
        ]);
        let mut pauses = Vec::new();
        for index in 0..3 {
            world.step(0.1);
            let telling = world.telling.as_ref().unwrap();
            assert_eq!(telling.next, index + 1);
            assert_eq!(
                world.ghost.speech.as_ref().unwrap().text,
                telling.lines[index]
            );
            let pause = telling.next_at - world.time;
            assert!((3.5..=7.0).contains(&pause));
            pauses.push(pause);
            assert_eq!(world.life.total.stories, 0);
            world.step((pause - 0.15) as f32);
            assert_eq!(world.telling.as_ref().unwrap().next, index + 1);
            world.step(0.06);
        }
        world.step(0.1);
        assert!(world.telling.is_none());
        assert!(pauses[1] > pauses[0]);
        assert_eq!(world.life.total.stories, 1);
        assert_eq!(world.life.today.stories, 1);
        assert!(
            world
                .life
                .today
                .words()
                .to_lowercase()
                .contains("told you a story")
        );
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|entry| entry.text == "I told you my first story.")
        );
        world.step(30.0);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::ToldStory)
                .count(),
            1
        );
    }

    #[test]
    fn pokes_scolding_leaving_and_sleep_interrupt_without_counting() {
        for interruption in 0..4 {
            let mut world = listening(12.0);
            world.tell(vec!["A light.".to_owned(); 3]);
            world.step(0.1);
            match interruption {
                0 => world.poke(),
                1 => {
                    world.hear("no");
                }
                2 => {
                    world.look_away(true);
                    world.time += 20.0;
                    world.step(0.1);
                }
                _ => {
                    world.hour = 23.0;
                    world.tuck_in();
                }
            }
            assert!(world.telling.is_none());
            world.step(20.0);
            assert_eq!(world.life.total.stories, 0);
        }
    }

    #[test]
    fn messages_and_new_requests_leave_the_current_story_running_at_the_glass() {
        let mut world = listening(12.0);
        world.tell(vec!["A light.".to_owned(); 5]);
        world.step(0.1);
        let telling = world.telling.clone();
        world.hear("tell me a story");
        world.hear("hello");
        world.tell(vec!["Another story.".to_owned()]);
        world.begin(Activity::Dance);
        assert!(!world.challenge());
        assert!(!world.write("HI"));
        assert_eq!(world.telling, telling);
        world.step(15.0);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(!world.needs_decision());
        assert!(!world.ghost.plan.done);
        assert_eq!(
            world.ghost.attention.map(|(thing, _)| thing),
            Some(Thing::Viewer)
        );
    }

    #[test]
    fn a_bedtime_story_says_goodnight_and_sleeps_overnight_on_the_cushion() {
        let mut world = listening(23.0);
        world.life.korean = 1.0;
        world.tell(vec!["작은 빛이 집을 찾았어.".to_owned(); 3]);
        world.step(20.0);
        assert!(world.telling.is_none());
        assert_eq!(world.life.total.stories, 1);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "…잘 자.");
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert!(world.ghost.plan.overnight);
        world.step(40.0);
        assert_eq!(world.ghost.asleep, 1.0);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::FellAsleep)
        );
    }
}
