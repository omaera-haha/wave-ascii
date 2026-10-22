//! Going out to the yard: the ghost floats out the door and spends a while
//! at a couple of its favorite spots, then comes back in.

use glam::Vec3;

use super::home::{self, Spot};
use super::{Activity, EventKind, Heard, Listening, Season, Sky, Understanding, World, meteor};

/// Seconds it stays at a spot once there.
const DWELL: (f32, f32) = (14.0, 26.0);
/// Spots it visits on one outing at most.
const VISITS: u32 = 3;
/// Seconds between outings it starts on its own.
const PAUSE: f64 = 1500.0;

impl World {
    pub fn yard_is_open(&self) -> bool {
        let rain = self.weather.rain(self.time);
        self.ghost.asleep < 0.3
            && self.ghost.needs.energy > 0.3
            && !self.weather.storm
            && rain < 0.45
            && !self.bedtime()
            && self.focus.is_none()
            && self.hunt.is_none()
            && (self
                .last_outing
                .is_none_or(|at| self.time - at >= PAUSE || self.time < at)
                || self
                    .visitors
                    .cat
                    .as_ref()
                    .is_some_and(|cat| cat.noticed && cat.settled(self.time)))
    }

    /// Where to go first: the lawn for the stars at night, else a spot that
    /// suits the season and the hour.
    pub(super) fn first_spot(&mut self) -> Spot {
        let dark = meteor::dark(self.hour);
        if dark {
            let clear = self.weather.sky == Sky::Clear && !self.weather.storm;
            let pick = self.random.unit();
            return if clear && pick < 0.45 {
                Spot::Roof
            } else if pick < 0.8 {
                Spot::Lawn
            } else {
                Spot::Bench
            };
        }
        let mut weights = [
            (Spot::Swing, 1.4),
            (
                Spot::Garden,
                if self.seedlings.is_empty() { 0.4 } else { 1.0 },
            ),
            (Spot::Birdbath, 0.8),
            (Spot::Bench, 0.7),
            (Spot::Lawn, 0.8),
            (Spot::Fence, 0.6),
        ];
        if self.season() == Season::Winter {
            weights[2].1 = 0.3;
            weights[4].1 = 1.4;
        }
        let total: f32 = weights.iter().map(|(_, weight)| weight).sum();
        let mut pick = self.random.unit() * total;
        for (spot, weight) in weights {
            if pick < weight {
                return spot;
            }
            pick -= weight;
        }
        Spot::Swing
    }

    /// The spot an outing is at or heading to.
    pub fn yard_spot(&self) -> Option<Spot> {
        if self.ghost.plan.activity != Activity::Yard || self.ghost.plan.done || self.ghost.plan.cat
        {
            return None;
        }
        let size = self.size();
        let target = self.ghost.plan.anchor;
        Spot::ALL.into_iter().min_by(|first, second| {
            first
                .position(size)
                .distance_squared(target)
                .total_cmp(&second.position(size).distance_squared(target))
        })
    }

    /// Whether the ghost is sitting on the swing, swinging.
    pub fn swinging(&self) -> Option<f32> {
        (self.yard_spot() == Some(Spot::Swing))
            .then(|| self.ghost.plan.stayed(self.time))
            .filter(|stayed| *stayed > 0.0)
    }

    pub(super) fn carry_out_outing(&mut self, seconds: f32) -> Option<Vec3> {
        if self.ghost.plan.cat {
            return self.keep_cat_company(seconds);
        }
        let spot = self.yard_spot()?;
        let arrived = self.ghost.plan.arrived;
        if arrived == Some(self.time) {
            self.last_outing = Some(self.time);
            self.ghost.plan.count += 1;
            self.ghost.plan.length = self.random.range(DWELL.0, DWELL.1);
            self.arrive_at(spot);
        }
        let stayed = self.ghost.plan.stayed(self.time);
        if spot == Spot::Swing && arrived.is_some() {
            // Ride the seat as it swings.
            let seat = self.swing_seat() + Vec3::Y * (0.34 * self.size());
            self.ghost.plan.target = seat;
            if self.ghost.position.distance(seat) < 0.4 {
                self.ghost.position = self.ghost.position.lerp(seat, 0.5);
            }
        }
        if arrived.is_some() && stayed > self.ghost.plan.length {
            self.move_on(Some(spot));
        }
        self.ghost.needs.change(0.0, 0.012 * seconds, 0.0);
        Some(spot.looks_at())
    }

    /// Done with a spot: on to another one, or back inside.
    fn move_on(&mut self, from: Option<Spot>) {
        self.ghost.plan.cat = false;
        if self.ghost.plan.count >= VISITS
            || self.ghost.needs.energy < 0.3
            || self.bedtime()
            || self.random.unit() < 0.35
        {
            self.ghost.plan.done = true;
            return;
        }
        let dark = meteor::dark(self.hour);
        let next = loop {
            let index = (self.random.unit() * Spot::ALL.len() as f32) as usize;
            let choice = Spot::ALL[index.min(Spot::ALL.len() - 1)];
            if Some(choice) != from && (choice != Spot::Roof || dark) {
                break choice;
            }
        };
        let position = next.position(self.size());
        self.ghost.plan.target = position;
        self.ghost.plan.anchor = position;
        self.ghost.plan.arrived = None;
        self.ghost.plan.started = self.time;
    }

    /// Sits quietly near the cat, at the distance it allows.
    fn keep_cat_company(&mut self, seconds: f32) -> Option<Vec3> {
        let Some(spot) = self.beside_cat() else {
            self.move_on(None);
            return None;
        };
        self.ghost.plan.target = spot;
        self.ghost.plan.anchor = spot;
        let arrived = self.ghost.plan.arrived;
        if arrived == Some(self.time) {
            self.last_outing = Some(self.time);
            self.ghost.plan.count += 1;
            self.ghost.plan.length = self.random.range(35.0, 70.0);
            if self.ghost.asleep < 0.3 {
                let korean = self.speaks_korean();
                let friendly = self.life.cat_trust >= 0.5;
                self.think(match (friendly, korean) {
                    (false, false) => "Hello, kitty. I'll sit very still.",
                    (false, true) => "안녕, 고양아. 가만히 있을게.",
                    (true, false) => "There you are, my furry friend.",
                    (true, true) => "왔구나, 내 털뭉치 친구.",
                });
            }
        }
        if arrived.is_some() && self.ghost.plan.stayed(self.time) > self.ghost.plan.length {
            self.move_on(None);
        }
        self.ghost
            .needs
            .change(0.0, 0.01 * seconds, 0.008 * seconds);
        self.position_of(super::Thing::Cat)
    }

    fn arrive_at(&mut self, spot: Spot) {
        if self.ghost.asleep >= 0.3 {
            return;
        }
        let korean = self.speaks_korean();
        let dark = meteor::dark(self.hour);
        let snowy = self.weather.snow_cover > 0.3;
        let thought = match (spot, korean) {
            (Spot::Swing, false) => "Wheee! Up to the clouds!",
            (Spot::Swing, true) => "슝— 구름까지 올라가자!",
            (Spot::Garden, false) if self.seedlings.is_empty() => "This bed is waiting for seeds.",
            (Spot::Garden, true) if self.seedlings.is_empty() => "씨앗을 기다리는 화단이야.",
            (Spot::Garden, false) => "Growing, growing, little ones.",
            (Spot::Garden, true) => "쑥쑥 자라라, 얘들아.",
            (Spot::Birdbath, false) if self.season() == Season::Winter => {
                "The birdbath froze into a little mirror."
            }
            (Spot::Birdbath, true) if self.season() == Season::Winter => {
                "새 물그릇이 꽁꽁 얼어서 작은 거울이 됐어."
            }
            (Spot::Birdbath, false) => "My face wobbles in the water.",
            (Spot::Birdbath, true) => "물속 내 얼굴이 일렁거려.",
            (Spot::Bench, false) => "A good bench for sitting and thinking.",
            (Spot::Bench, true) => "앉아서 생각하기 좋은 벤치야.",
            (Spot::Lawn, false) if dark => "So many stars from out here.",
            (Spot::Lawn, true) if dark => "밖에서 보니까 별이 정말 많아.",
            (Spot::Lawn, false) if snowy => "The snow is soft as a cushion.",
            (Spot::Lawn, true) if snowy => "눈이 방석처럼 폭신해.",
            (Spot::Lawn, false) if self.weather.sky == Sky::Clear => {
                "That cloud looks like a sleepy whale."
            }
            (Spot::Lawn, true) if self.weather.sky == Sky::Clear => "저 구름, 졸린 고래 같아.",
            (Spot::Lawn, false) => "The grass smells like rain.",
            (Spot::Lawn, true) => "풀에서 비 냄새가 나.",
            (Spot::Fence, false) => "The hills go on and on.",
            (Spot::Fence, true) => "언덕이 끝없이 이어져.",
            (Spot::Roof, false) => "The roof is the best seat for the stars.",
            (Spot::Roof, true) => "지붕은 별 보기 제일 좋은 자리야.",
        };
        self.think(thought);
    }

    /// Where the swing's seat is now.
    pub fn swing_seat(&self) -> Vec3 {
        let length = home::SWING_TOP.y - home::SWING_SEAT.y;
        let (sine, cosine) = self.swing_angle().sin_cos();
        home::SWING_TOP + Vec3::new(0.0, -length * cosine, length * sine)
    }

    /// The swing's angle from hanging straight, in radians.
    pub fn swing_angle(&self) -> f32 {
        let riding = self
            .swinging()
            .map_or(0.0, |stayed| (stayed / 3.0).min(1.0));
        riding * (self.time * 1.9).sin() as f32 * 0.45
    }

    /// Asked to go out ("go outside", "마당에 나가자", "ride the swing"), or
    /// to come back in.
    pub(super) fn hear_outing(&mut self, text: &str, korean: bool) -> Option<Heard> {
        let request = requested(text)?;
        self.listening = Listening::default();
        self.record(EventKind::Heard(text.to_owned()));
        let outside = !home::in_house(self.ghost.position);
        let lines = match request {
            Request::In => {
                if self.ghost.plan.activity == Activity::Yard && !self.ghost.plan.done || outside {
                    self.ghost.plan.done = true;
                    ("Coming in!", "들어갈게!")
                } else {
                    ("I'm already inside.", "나 벌써 안에 있어.")
                }
            }
            Request::Out(spot) => {
                let rain = self.weather.rain(self.time);
                if self.ghost.asleep >= 0.3 {
                    ("Mm… tomorrow…", "음… 내일…")
                } else if self.weather.storm || rain >= 0.45 {
                    ("It's too wet out there.", "밖에 비가 너무 와.")
                } else if self.ghost.needs.energy < 0.2 {
                    ("I'm too tired to go out.", "너무 졸려서 못 나가겠어.")
                } else {
                    self.begin(Activity::Yard);
                    if let Some(spot) = spot {
                        let position = spot.position(self.size());
                        self.ghost.plan.target = position;
                        self.ghost.plan.anchor = position;
                    }
                    self.last_outing = Some(self.time);
                    match spot {
                        Some(Spot::Swing) => ("To the swing!", "그네 타러 가자!"),
                        _ => ("Out to the yard!", "마당에 나가자!"),
                    }
                }
            }
        };
        self.say(if korean { lines.1 } else { lines.0 });
        Some(Heard {
            tone: None,
            understanding: Understanding::Outing,
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    /// Where the ghost is, in or out, and how the yard looks, for the minds.
    pub fn yard_words(&self) -> String {
        let place = if home::in_doorway(self.ghost.position) && !home::indoors(self.ghost.position)
        {
            "passing through the door".to_owned()
        } else if home::in_house(self.ghost.position) {
            "indoors in its room".to_owned()
        } else if self.ghost.plan.activity == Activity::Yard && self.ghost.plan.cat {
            "out in the yard, keeping the visiting cat company".to_owned()
        } else {
            match self
                .yard_spot()
                .filter(|_| self.ghost.plan.arrived.is_some())
            {
                Some(spot) => format!("out in the yard, {}", spot.words()),
                None => "out in the yard".to_owned(),
            }
        };
        let ground = if self.weather.snow_cover > 0.4 {
            "snow covers the lawn"
        } else if self.weather.rain(self.time) > 0.2 {
            "the lawn is wet with rain"
        } else {
            "the lawn is green"
        };
        let snowman = if crate::world::passerby::Passerby::Snowman
            .at(
                self.meteor_seed,
                self.time,
                self.date,
                self.hour,
                &self.weather,
            )
            .is_some()
        {
            ", and a snowman stands on the lawn"
        } else {
            ""
        };
        format!("The ghost is {place}; {ground}{snowman}.")
    }

    /// What the ghost is doing in the yard, in words.
    pub fn outing_words(&self) -> &'static str {
        if self.ghost.plan.cat && !self.ghost.plan.done {
            return "keeping the cat company in the yard";
        }
        self.yard_spot()
            .filter(|_| self.ghost.plan.arrived.is_some())
            .map_or("heading out to the yard", Spot::words)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Request {
    Out(Option<Spot>),
    In,
}

fn requested(text: &str) -> Option<Request> {
    let lower = text.to_lowercase();
    let words: Vec<&str> = lower
        .split(|character: char| !character.is_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .collect();
    let has = |phrase: &str| {
        let phrase: Vec<&str> = phrase.split_whitespace().collect();
        words.windows(phrase.len()).any(|run| run == phrase)
    };
    if [
        "come in",
        "come inside",
        "come back in",
        "go inside",
        "stay inside",
    ]
    .iter()
    .any(|phrase| has(phrase))
        || ["들어와", "들어가", "안으로"]
            .iter()
            .any(|word| lower.contains(word))
    {
        return Some(Request::In);
    }
    if words
        .iter()
        .any(|word| matches!(*word, "not" | "dont" | "don't" | "never"))
        || lower.contains("말자")
        || lower.contains("하지 마")
        || lower.contains("나가지")
    {
        return None;
    }
    let swing =
        words.iter().any(|word| matches!(*word, "swing" | "swings")) || lower.contains("그네");
    if swing {
        return Some(Request::Out(Some(Spot::Swing)));
    }
    let english = [
        "go outside",
        "go out",
        "play outside",
        "go out to the yard",
        "to the yard",
        "in the yard",
        "the backyard",
        "outside",
    ]
    .iter()
    .any(|phrase| has(phrase));
    let korean = [
        "마당",
        "밖에 나가",
        "밖에서 놀",
        "나가자",
        "나가 놀",
        "산책",
    ]
    .iter()
    .any(|word| lower.contains(word));
    (english || korean).then_some(Request::Out(None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_outing_goes_out_the_door_visits_spots_and_comes_to_an_end() {
        let mut world = World::new(3, 11.0);
        world.ghost.needs.energy = 1.0;
        world.begin(Activity::Yard);
        let mut went_out = false;
        let mut visited = Vec::new();
        for _ in 0..(400.0 / 0.05) as usize {
            world.step(0.05);
            if !home::in_house(world.ghost.position) {
                went_out = true;
            }
            if let Some(spot) = world.yard_spot()
                && world.ghost.plan.arrived.is_some()
                && visited.last() != Some(&spot)
            {
                visited.push(spot);
            }
            if world.ghost.plan.activity != Activity::Yard || world.ghost.plan.done {
                break;
            }
        }
        assert!(went_out);
        assert!(!visited.is_empty());
        assert!(visited.len() <= VISITS as usize);
    }

    #[test]
    fn the_swing_only_moves_while_the_ghost_rides_it() {
        let mut world = World::new(3, 11.0);
        assert_eq!(world.swing_angle(), 0.0);
        world.begin(Activity::Yard);
        let seat = Spot::Swing.position(world.size());
        world.ghost.plan.target = seat;
        world.ghost.plan.anchor = seat;
        world.ghost.position = seat;
        world.ghost.plan.arrived = Some(world.time);
        world.time += 4.0;
        world.time += 0.4;
        assert!(world.swinging().is_some());
        assert!(world.swing_angle().abs() > 0.0);
        assert!(world.swing_seat().distance(home::SWING_SEAT) < 0.8);
    }

    #[test]
    fn asking_to_go_out_or_come_in_works_in_both_languages() {
        for message in [
            "let's go outside!",
            "play in the yard",
            "마당에 나가자",
            "밖에 나가서 놀자",
        ] {
            let mut world = World::new(3, 11.0);
            world.ghost.needs.energy = 1.0;
            let heard = world.hear(message);
            assert_eq!(heard.understanding, Understanding::Outing, "{message}");
            assert_eq!(world.ghost.plan.activity, Activity::Yard, "{message}");
        }
        let mut world = World::new(3, 11.0);
        world.ghost.needs.energy = 1.0;
        world.hear("ride the swing");
        assert_eq!(world.yard_spot(), Some(Spot::Swing));
        world.hear("come back in");
        assert!(world.ghost.plan.done);
        for message in ["don't go outside", "hello", "the window is open"] {
            let mut world = World::new(3, 11.0);
            assert_ne!(
                world.hear(message).understanding,
                Understanding::Outing,
                "{message}"
            );
        }
        let mut world = World::new(3, 11.0);
        world.weather.storm = true;
        world.hear("go outside");
        assert_ne!(world.ghost.plan.activity, Activity::Yard);
    }

    #[test]
    fn storms_and_bedtime_keep_it_indoors() {
        let mut world = World::new(3, 11.0);
        world.ghost.needs.energy = 1.0;
        assert!(world.yard_is_open());
        world.weather.storm = true;
        assert!(!world.yard_is_open());
        let mut world = World::new(3, 23.5);
        world.ghost.needs.energy = 1.0;
        assert!(!world.yard_is_open() || !world.bedtime());
    }

    #[test]
    fn an_outing_to_a_settled_cat_sits_beside_it_without_startling_it() {
        let mut world = World::new(3, 13.0);
        world.ghost.needs.energy = 1.0;
        world.visitors.next_cat = 0.0;
        world.step(0.05);
        let mut stayed = 0.0;
        for step in 0..(200.0 / 0.05) as usize {
            if step == 400 {
                world.begin(Activity::Yard);
            }
            world.step(0.05);
            if world.ghost.plan.activity == Activity::Yard
                && world.ghost.plan.cat
                && world.ghost.plan.arrived.is_some()
            {
                stayed += 0.05;
            }
        }
        let startled = world
            .events
            .iter()
            .any(|event| event.kind == EventKind::CatLeft { startled: true });
        assert!(!startled);
        assert!(stayed > 20.0);
    }
}
