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

    /// Whether there's snow enough, and a snowfall it hasn't yet made a
    /// snow ghost for.
    fn snow_ghost_due(&self) -> bool {
        self.weather.snow_cover > 0.4
            && self.weather.settled_snow.is_some_and(|spell| {
                self.snow_ghost.is_none_or(|(made, progress)| made != spell || progress < 1.0)
            })
    }

    /// How the snow ghost on the lawn stands: how built and how melted.
    pub fn snow_ghost_shape(&self) -> Option<(f32, f32)> {
        let (spell, progress) = self.snow_ghost?;
        (self.weather.settled_snow == Some(spell) && self.weather.snow_cover > 0.05)
            .then(|| (progress, (1.0 - self.weather.snow_cover).clamp(0.0, 1.0)))
    }

    /// Where to go first: the lawn for the stars at night, else a spot that
    /// suits the season and the hour.
    pub(super) fn first_spot(&mut self) -> Spot {
        if self.snow_ghost_due() && self.random.unit() < 0.8 {
            return Spot::SnowGhost;
        }
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
        if spot == Spot::SnowGhost
            && arrived.is_some()
            && let Some(spell) = self.weather.settled_snow
        {
            // Pat by pat, it shapes a little ghost out of snow.
            let (made, progress) = self.snow_ghost.unwrap_or((spell, 0.0));
            let before = if made == spell { progress } else { 0.0 };
            let progress = (before + seconds / 22.0).min(1.0);
            self.snow_ghost = Some((spell, progress));
            self.ghost.plan.length = self.ghost.plan.length.max(stayed + 1.0);
            if before < 1.0 && progress >= 1.0 {
                self.ghost.plan.length = stayed + 6.0;
                let korean = self.speaks_korean();
                self.think(if korean {
                    "나랑 똑같은 눈 유령이야!"
                } else {
                    "A snow ghost, just like me!"
                });
                if let Some(milestone) = self.life.reach(
                    "Made my first snow ghost.".to_owned(),
                    "처음으로 눈 유령을 만들었어.",
                ) {
                    self.record(EventKind::Milestone(milestone));
                }
            }
        }
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
            let fits = match choice {
                Spot::Roof => dark,
                Spot::SnowGhost => self.snow_ghost_due(),
                _ => true,
            };
            if Some(choice) != from && fits {
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
            (Spot::SnowGhost, false) => "I'll make a ghost out of snow!",
            (Spot::SnowGhost, true) => "눈으로 유령을 만들 거야!",
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
            Request::Out(spot) => self.go_out(spot),
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

    /// Sets out for the yard, or a spot in it, when it can: its answer.
    fn go_out(&mut self, spot: Option<Spot>) -> (&'static str, &'static str) {
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
                self.ghost.plan.cat = false;
                self.ghost.plan.arrived = None;
            }
            self.last_outing = Some(self.time);
            match spot {
                Some(Spot::Swing) => ("To the swing!", "그네 타러 가자!"),
                _ => ("Out to the yard!", "마당에 나가자!"),
            }
        }
    }

    /// The yard thing a ray from `eye` along `direction` meets first, past
    /// anything of the house in the way.
    pub fn yard_thing_at(&self, eye: Vec3, direction: Vec3) -> Option<YardThing> {
        let meet = |center: Vec3, radius: f32| {
            let offset = eye - center;
            let along = -offset.dot(direction);
            let miss = (offset + direction * along).length();
            (along > 0.0 && miss < radius).then_some(along)
        };
        let mut things = vec![
            (YardThing::Swing, meet(self.swing_seat() + Vec3::Y * 0.4, 0.55)),
            (YardThing::Mailbox, meet(home::MAILBOX + Vec3::Y * 1.05, 0.3)),
            (
                YardThing::Tree,
                meet(home::TREE_CROWN, home::TREE_CROWN_RADIUS * 0.9)
                    .or_else(|| meet(home::TREE + Vec3::Y * 1.1, 0.45)),
            ),
        ];
        if let Some(cat) = &self.visitors.cat {
            things.push((YardThing::Cat, meet(cat.position(self.time) + Vec3::Y * 0.2, 0.35)));
        }
        // The house hides what is behind it.
        let house = home::house_entry(eye, direction);
        things
            .into_iter()
            .filter_map(|(thing, distance)| distance.map(|distance| (thing, distance)))
            .filter(|(_, distance)| house.is_none_or(|house| *distance < house))
            .min_by(|first, second| first.1.total_cmp(&second.1))
            .map(|(thing, _)| thing)
    }

    /// The viewer clicked something in the yard. The mailbox is the app's.
    pub fn click_yard(&mut self, thing: YardThing) {
        self.notice_viewer();
        let korean = self.speaks_korean();
        match thing {
            YardThing::Swing => {
                let lines = self.go_out(Some(Spot::Swing));
                self.say(if korean { lines.1 } else { lines.0 });
            }
            YardThing::Cat => {
                if self.ghost.asleep < 0.3
                    && let Some(spot) = self.beside_cat()
                {
                    self.begin(Activity::Yard);
                    self.ghost.plan.target = spot;
                    self.ghost.plan.anchor = spot;
                    self.ghost.plan.cat = true;
                    self.ghost.plan.arrived = None;
                    self.last_outing = Some(self.time);
                    self.think(if korean { "고양이다!" } else { "The cat!" });
                }
            }
            YardThing::Tree => {
                self.tree_shaken = Some(self.time);
                if self
                    .visitors
                    .bird
                    .as_ref()
                    .is_some_and(|bird| bird.perch == super::Perch::Tree)
                {
                    self.shoo_bird();
                }
            }
            YardThing::Mailbox => {}
        }
    }

    /// Seconds since the viewer last shook the tree, while its leaves fall.
    pub fn tree_shake(&self) -> Option<f32> {
        let since = (self.time - self.tree_shaken?) as f32;
        (0.0..6.0).contains(&since).then_some(since)
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

/// Something out in the yard the viewer can click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YardThing {
    Swing,
    Cat,
    Tree,
    Mailbox,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Request {
    Out(Option<Spot>),
    In,
}

fn requested(text: &str) -> Option<Request> {
    let lower = text.to_lowercase().replace(['\u{2019}', '\u{2018}'], "'");
    let words: Vec<&str> = lower
        .split(|character: char| !character.is_alphanumeric() && character != '\'')
        .filter(|word| !word.is_empty())
        .collect();
    let has = |phrase: &str| {
        let phrase: Vec<&str> = phrase.split_whitespace().collect();
        words.windows(phrase.len()).any(|run| run == phrase)
    };
    let ends = |phrase: &str| {
        let phrase: Vec<&str> = phrase.split_whitespace().collect();
        words.ends_with(&phrase)
    };
    // Saying no, or telling of their own comings and goings, asks nothing.
    let refused = words.iter().any(|word| {
        matches!(
            *word,
            "not" | "dont" | "don't" | "never" | "can't" | "cant" | "cannot" | "won't" | "shouldn't"
        )
    }) || ["말자", "하지 마", "나가지", "안 돼", "안돼", "안 되", "안되"]
        .iter()
        .any(|word| lower.contains(word));
    let themselves = words
        .iter()
        .any(|word| *word == "i" || word.starts_with("i'") || *word == "im")
        || ["갈게", "다녀올", "갔다 올", "나갈 거", "나가볼게", "나갔다", "들어와서", "들어왔"]
            .iter()
            .any(|word| lower.contains(word));
    if refused || themselves {
        return None;
    }
    if ["come in", "come inside", "come back in", "come back inside", "come in now"]
        .iter()
        .any(|phrase| ends(phrase))
        || has("stay inside")
        || ["이제 들어와", "안으로 들어와", "그만 들어와", "들어와 들어와"]
            .iter()
            .any(|word| lower.contains(word))
        || lower.trim_end_matches(['!', '.', '~', ' ']).ends_with("들어와")
            && !lower.contains("잘 들어와")
    {
        return Some(Request::In);
    }
    let swing = [
        "ride the swing",
        "on the swing",
        "go on the swing",
        "go swing",
        "go swinging",
        "swing on the swing",
    ]
    .iter()
    .any(|phrase| has(phrase))
        || lower.contains("그네 타")
        || lower.contains("그네타");
    if swing {
        return Some(Request::Out(Some(Spot::Swing)));
    }
    let english = [
        "go outside",
        "go out and play",
        "go out to play",
        "go play outside",
        "play outside",
        "play in the yard",
        "go to the yard",
        "go into the yard",
        "out to the yard",
        "let's go out",
        "lets go out",
    ]
    .iter()
    .any(|phrase| has(phrase));
    let korean = [
        "마당에 나가",
        "마당에서 놀",
        "마당 가자",
        "마당으로 가",
        "밖에 나가자",
        "밖에 나가서 놀",
        "밖에서 놀자",
        "나가 놀자",
        "나가서 놀자",
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
            "그네 타자",
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
        for message in [
            "don't go outside",
            "hello",
            "the window is open",
            "it's cold outside",
            "I'll go out for dinner later",
            "나 밖에 나가볼게",
            "집에 들어가는 중",
            "don\u{2019}t go outside",
            "you can't go outside",
            "밖에 나가면 안 돼",
            "what's outside the window?",
            "the cat is in the yard",
            "mood swings",
            "잘 들어가",
            "집에 들어와서 너무 피곤해",
            "can I come in?",
            "this will come in handy",
            "don't come in",
        ] {
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

    #[test]
    fn motes_dropped_anywhere_on_the_lawn_get_caught_around_the_house() {
        for spot in [
            Vec3::new(-4.0, 0.9, 0.0),
            Vec3::new(0.0, 0.9, -3.0),
            Vec3::new(0.0, 0.9, 2.6),
            Vec3::new(-1.5, 0.9, 2.8),
            Vec3::new(2.4, 0.9, 0.0),
            Vec3::new(0.0, 0.9, 1.75),
            Vec3::new(6.0, 0.9, 0.0),
        ] {
            let mut world = World::new(3, 12.0);
            world.ghost.needs.energy = 1.0;
            world.motes.clear();
            world.drop_mote(spot);
            world.begin(Activity::Chase);
            let mut ate = false;
            for _ in 0..(90.0 / 0.05) as usize {
                world.step(0.05);
                if world.events.iter().any(|event| event.kind == EventKind::AteMote) {
                    ate = true;
                    break;
                }
                if world.ghost.plan.done && !world.motes.is_empty() {
                    world.begin(Activity::Chase);
                }
            }
            assert!(ate, "{spot}: ghost at {}", world.ghost.position);
        }
    }

    #[test]
    fn every_yard_spot_is_reachable_from_the_room() {
        for spot in Spot::ALL {
            let mut world = World::new(3, if spot == Spot::Roof { 22.0 } else { 12.0 });
            world.ghost.needs.energy = 1.0;
            world.begin(Activity::Yard);
            let position = spot.position(world.size());
            world.ghost.plan.target = position;
            world.ghost.plan.anchor = position;
            world.ghost.plan.cat = false;
            let mut arrived = false;
            for _ in 0..(60.0 / 0.05) as usize {
                world.step(0.05);
                if world.ghost.plan.arrived.is_some() {
                    arrived = true;
                    break;
                }
            }
            assert!(arrived, "{spot:?}: ghost at {}", world.ghost.position);
        }
    }

    #[test]
    fn asking_for_the_swing_wins_over_a_visit_to_the_cat() {
        for seed in 0..12 {
            let mut world = World::new(seed, 13.0);
            world.ghost.needs.energy = 1.0;
            world.visitors.next_cat = 0.0;
            world.step(30.0);
            world.hear("ride the swing");
            assert_eq!(world.yard_spot(), Some(Spot::Swing), "seed {seed}");
        }
    }
}
