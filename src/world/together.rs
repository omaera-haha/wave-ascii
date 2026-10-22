//! Housemates do things together: a game of tag, a chat, a nuzzle, a
//! little dance in a ring, or gazing out side by side. One asks, and the
//! other joins when it is free; each then plays its own part, reading where
//! the other is from what it sees of its housemates.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, Mood, Peer, Plan, ROOM_MAX, ROOM_MIN, Sky, World, home};

/// Seconds an invitation waits for its answer.
const ANSWER_WITHIN: f64 = 6.0;
/// Seconds between turns in a chat.
const TURN: f64 = 3.2;
/// How long each chaser chases in tag before they swap.
const CHASE_FOR: f64 = 5.0;

/// Something two housemates do together.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Pastime {
    Tag,
    Chat,
    Nuzzle,
    Dance,
    Gaze,
}

impl Pastime {
    pub const ALL: [Self; 5] = [Self::Tag, Self::Chat, Self::Nuzzle, Self::Dance, Self::Gaze];

    pub fn name(self) -> &'static str {
        match self {
            Self::Tag => "tag",
            Self::Chat => "chat",
            Self::Nuzzle => "nuzzle",
            Self::Dance => "dance",
            Self::Gaze => "gaze",
        }
    }

    /// Seconds it lasts once both are there.
    fn length(self) -> f32 {
        match self {
            Self::Tag => 22.0,
            Self::Chat => 15.0,
            Self::Nuzzle => 5.0,
            Self::Dance => 11.0,
            Self::Gaze => 24.0,
        }
    }

    /// What doing it with `name` was, in a few words.
    pub fn words(self, name: &str) -> String {
        match self {
            Self::Tag => format!("played tag with {name}"),
            Self::Chat => format!("had a chat with {name}"),
            Self::Nuzzle => format!("nuzzled {name}"),
            Self::Dance => format!("danced in a ring with {name}"),
            Self::Gaze => format!("gazed out at the sky with {name}"),
        }
    }

    /// Doing it with `name`, as the status line says.
    pub fn doing(self, name: &str) -> String {
        match self {
            Self::Tag => format!("playing tag with {name}"),
            Self::Chat => format!("chatting with {name}"),
            Self::Nuzzle => format!("nuzzling {name}"),
            Self::Dance => format!("dancing in a ring with {name}"),
            Self::Gaze => format!("gazing out with {name}"),
        }
    }
}

/// One housemate asking another to do something together.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Invitation {
    pub from: u32,
    pub to: u32,
    pub pastime: Pastime,
    pub at: f64,
}

/// How warmly it feels about one housemate, and how often they've played.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fondness {
    pub id: u32,
    pub name: String,
    /// From strangers at 0 to best friends at 1.
    pub warmth: f32,
    pub times: u32,
    /// The day of their latest time together, and how many that day: a
    /// friendship grows over days more than over one long afternoon.
    pub day: String,
    pub today: u32,
}

/// Lists names as "A", "A and B", "A, B, and C".
pub fn list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// Whether a housemate is free to be asked.
fn free(peer: &Peer) -> bool {
    peer.asleep < 0.3
        && peer.fade < 0.3
        && peer.with.is_none()
        && (!peer.busy
            || matches!(
                peer.activity,
                Activity::Float | Activity::Explore | Activity::Window | Activity::Visit
            ))
}

impl World {
    /// The housemate it would ask to do something together, if anyone is
    /// free: the fondest of those around, else the nearest.
    pub fn together_partner(&self) -> Option<&Peer> {
        if self.ghost.asleep >= 0.3 || self.ghost.needs.energy < 0.2 {
            return None;
        }
        let here = home::area(self.ghost.position);
        self.peers
            .iter()
            .filter(|peer| free(peer))
            .filter(|peer| {
                home::area(peer.position) == here
                    || peer.position.distance(self.ghost.position) < 4.0
            })
            .max_by(|first, second| {
                let score = |peer: &Peer| {
                    self.fondness_for(peer.id) * 2.0
                        - peer.position.distance(self.ghost.position) * 0.3
                };
                score(first).total_cmp(&score(second))
            })
    }

    /// How fond it is of housemate `id`, from 0 to 1.
    pub fn fondness_for(&self, id: u32) -> f32 {
        self.life
            .fondness
            .iter()
            .find(|fondness| fondness.id == id)
            .map_or(0.0, |fondness| fondness.warmth)
    }

    /// The housemate it is fondest of, with how fond, once there is one.
    pub fn best_friend(&self) -> Option<&Fondness> {
        self.life
            .fondness
            .iter()
            .filter(|fondness| fondness.warmth >= 0.15)
            .max_by(|first, second| first.warmth.total_cmp(&second.warmth))
    }

    fn choose_pastime(&mut self, partner: &Peer) -> Pastime {
        let night = self.is_night();
        let clear = self.weather.sky == Sky::Clear || self.weather.sky == Sky::Rainbow;
        let fond = self.fondness_for(partner.id);
        let energy = self.ghost.needs.energy;
        let lively = matches!(self.ghost.mood, Mood::Excited | Mood::Cheerful);
        let weights = [
            if energy > 0.45 { 1.2 + if lively { 0.6 } else { 0.0 } } else { 0.1 },
            1.3,
            0.4 + fond,
            if energy > 0.35 { 0.5 + if lively { 0.6 } else { 0.0 } } else { 0.0 },
            if (night && clear) || self.weather.sky == Sky::Rainbow || self.weather.sky == Sky::Snow {
                1.0
            } else {
                0.2
            },
        ];
        Pastime::ALL[self.pick(&weights).unwrap_or(1)]
    }

    /// Asks the chosen housemate to do something together, and sets off
    /// toward it.
    pub(super) fn begin_together(&mut self) -> bool {
        let Some(partner) = self.together_partner().cloned() else {
            return false;
        };
        let pastime = self.choose_pastime(&partner);
        self.invitations.retain(|invitation| invitation.from != self.ghost.id);
        self.invitations.push(Invitation {
            from: self.ghost.id,
            to: partner.id,
            pastime,
            at: self.time,
        });
        self.join(partner.id, pastime, true);
        true
    }

    fn join(&mut self, partner: u32, pastime: Pastime, asking: bool) {
        let position = self
            .peers
            .iter()
            .find(|peer| peer.id == partner)
            .map_or(self.ghost.position, |peer| peer.position);
        let mut plan = Plan::new(Activity::Together, self.time, position, pastime.length());
        // The anchor keeps the last turn of a chat said, so it starts at none.
        plan.anchor = Vec3::ZERO;
        plan.with = Some(partner);
        plan.pastime = Some(pastime);
        // The one who asked starts as "it" in tag and speaks first.
        plan.count = u32::from(!asking);
        self.ghost.plan = plan;
        self.record(EventKind::Began(Activity::Together));
    }

    /// Takes up a housemate's invitation, when free to.
    pub(super) fn answer_invitations(&mut self) {
        let id = self.ghost.id;
        let time = self.time;
        self.invitations
            .retain(|invitation| time - invitation.at < ANSWER_WITHIN);
        let Some(index) = self.invitations.iter().position(|invitation| invitation.to == id)
        else {
            return;
        };
        let invitation = self.invitations[index];
        let plan = &self.ghost.plan;
        let free = self.ghost.asleep < 0.3
            && self.ghost.fade < 0.3
            && self.focus.is_none()
            && (plan.done
                || matches!(
                    plan.activity,
                    Activity::Float | Activity::Explore | Activity::Window
                )
                || (plan.activity == Activity::Visit && (self.companion || !self.viewer_present())));
        if !free {
            return;
        }
        self.invitations.remove(index);
        self.join(invitation.from, invitation.pastime, false);
    }

    /// One moment of doing something together. Returns where it looks.
    pub(super) fn carry_out_together(&mut self) -> Option<Vec3> {
        let time = self.time;
        let plan = self.ghost.plan.clone();
        let (Some(with), Some(pastime)) = (plan.with, plan.pastime) else {
            self.ghost.plan.done = true;
            return None;
        };
        let Some(partner) = self.peers.iter().find(|peer| peer.id == with).cloned() else {
            self.ghost.plan.done = true;
            return None;
        };
        let joined = partner.with == Some(self.ghost.id);
        if !joined {
            if let Some(arrived) = plan.arrived {
                // The other finished first, or was called away.
                self.ghost.plan.done = true;
                if (time - arrived) as f32 > plan.length * 0.5 {
                    self.warm_to(&partner, pastime);
                    self.record(EventKind::Together {
                        with: partner.name.clone(),
                        what: pastime,
                    });
                }
                return Some(partner.position);
            }
            // Asked but not answered yet.
            if plan.elapsed(time) > ANSWER_WITHIN as f32 + 0.5 {
                self.ghost.plan.done = true;
                self.think_about_partner(&partner, false);
            }
            self.ghost.plan.target = partner.position;
            return Some(partner.position);
        }
        let position = self.ghost.position;
        let apart = partner.position - position;
        let near = Vec3::new(apart.x, 0.0, apart.z).length() < 0.75;
        if plan.arrived.is_none() && near {
            self.ghost.plan.arrived = Some(time);
        }
        let started = plan.arrived.unwrap_or(time);
        let since = time - started;
        let leader = plan.count == 0;
        let mut gazing = None;
        match pastime {
            Pastime::Tag => {
                let chasing = ((since / CHASE_FOR).floor() as i64 % 2 == 0) == leader;
                self.ghost.plan.target = if chasing || plan.arrived.is_none() {
                    partner.position
                } else {
                    let away = (position - partner.position).with_y(0.0).normalize_or(Vec3::X);
                    let swerve = Vec3::new(-away.z, 0.0, away.x) * (time as f32 * 1.3).sin() * 0.6;
                    self.within_reach(position + away * 1.1 + swerve)
                };
                if chasing && apart.length() < 0.32 && self.ghost.plan.count < 1000 {
                    self.hop(1.8);
                    self.ghost.plan.count += 2;
                    if self.ghost.speech.as_ref().is_none_or(|line| time - line.time > 4.0) {
                        let tag = self.spoken("Tag!", "잡았다!");
                        self.say(tag);
                    }
                }
            }
            Pastime::Chat => {
                // Facing each other a little apart.
                let toward = apart.with_y(0.0).normalize_or(Vec3::X);
                self.ghost.plan.target = self.within_reach(partner.position - toward * 0.55);
                if plan.arrived.is_some() {
                    let turn = (since / TURN).floor() as i64;
                    let mine = (turn % 2 == 0) == leader;
                    let spoken_turn = self.ghost.plan.anchor.y as i64;
                    if mine && turn < 5 && spoken_turn != turn + 1 {
                        self.ghost.plan.anchor.y = (turn + 1) as f32;
                        let line = self.chat_line(&partner, turn);
                        self.say(&line);
                    }
                }
            }
            Pastime::Nuzzle => {
                let toward = apart.with_y(0.0).normalize_or(Vec3::X);
                self.ghost.plan.target = partner.position - toward * 0.28;
            }
            Pastime::Dance => {
                let middle = (position + partner.position) * 0.5;
                let side = if leader { 0.0 } else { std::f32::consts::PI };
                let angle = since as f32 * 2.2 + side;
                self.ghost.plan.target = self.within_reach(
                    Vec3::new(middle.x, position.y, middle.z)
                        + Vec3::new(angle.cos(), 0.0, angle.sin()) * 0.38,
                );
            }
            Pastime::Gaze => {
                let outside = !home::in_house(position);
                let (spot, sky) = if outside {
                    (
                        home::Spot::Lawn.position(self.size()),
                        home::Spot::Lawn.looks_at(),
                    )
                } else if home::area(position) == home::Area::Bedroom {
                    let middle = (home::BEDROOM_WINDOW_LOW + home::BEDROOM_WINDOW_HIGH) * 0.5;
                    (
                        Vec3::new(middle.x, 1.3, ROOM_MIN.z + 0.45),
                        Vec3::new(middle.x, 2.5, ROOM_MIN.z - 6.0),
                    )
                } else {
                    (
                        Vec3::new(0.0, 1.35, ROOM_MIN.z + 0.45),
                        Vec3::new(0.0, 2.6, ROOM_MIN.z - 6.0),
                    )
                };
                let side = if leader { -0.28 } else { 0.28 };
                self.ghost.plan.target = spot + Vec3::X * side;
                if plan.arrived.is_some() && position.distance(self.ghost.plan.target) < 0.4 {
                    gazing = Some(sky);
                }
            }
        }
        if plan.arrived.is_some() && since as f32 > plan.length {
            self.ghost.plan.done = true;
            self.warm_to(&partner, pastime);
            self.record(EventKind::Together {
                with: partner.name.clone(),
                what: pastime,
            });
        } else if plan.elapsed(time) > 45.0 {
            self.ghost.plan.done = true;
        }
        gazing.or(Some(partner.position))
    }

    /// `point`, kept inside whichever room or yard it is in.
    fn within_reach(&self, point: Vec3) -> Vec3 {
        let position = self.ghost.position;
        match home::area(position) {
            home::Area::Living => point.clamp(
                ROOM_MIN + Vec3::new(0.4, 0.45, 0.4),
                ROOM_MAX - Vec3::new(0.4, 0.5, 0.4),
            ),
            home::Area::Bedroom => point.clamp(
                home::BEDROOM_MIN + Vec3::new(0.4, 0.45, 0.4),
                home::BEDROOM_MAX - Vec3::new(0.4, 0.5, 0.4),
            ),
            home::Area::Yard => {
                let (low, high) = home::yard_bounds();
                point.clamp(low + Vec3::new(0.2, 0.4, 0.2), high)
            }
        }
    }

    fn warm_to(&mut self, partner: &Peer, pastime: Pastime) {
        let gain = match pastime {
            Pastime::Nuzzle => 0.035,
            Pastime::Chat | Pastime::Gaze => 0.03,
            Pastime::Tag | Pastime::Dance => 0.025,
        };
        let day = self.date.to_string();
        let fondness = match self
            .life
            .fondness
            .iter_mut()
            .position(|fondness| fondness.id == partner.id)
        {
            Some(index) => &mut self.life.fondness[index],
            None => {
                self.life.fondness.push(Fondness {
                    id: partner.id,
                    name: partner.name.clone(),
                    ..Fondness::default()
                });
                self.life.fondness.last_mut().unwrap()
            }
        };
        fondness.name = partner.name.clone();
        if fondness.day != day {
            fondness.day = day;
            fondness.today = 0;
        }
        let gain = gain / (1.0 + fondness.today as f32 / 3.0);
        fondness.warmth = (fondness.warmth + gain * (1.0 - fondness.warmth)).min(1.0);
        fondness.times = fondness.times.saturating_add(1);
        fondness.today = fondness.today.saturating_add(1);
        self.ghost.needs.change(0.0, 0.08, 0.1);
    }

    fn think_about_partner(&mut self, partner: &Peer, joined: bool) {
        if !joined && self.ghost.asleep < 0.3 {
            let line = if self.speaks_korean() {
                format!("{}{} 바쁜가 봐.", partner.name, super::writing::particle(&partner.name, "은", "는"))
            } else {
                format!("{} must be busy.", partner.name)
            };
            self.think(&line);
        }
    }

    /// What it says on its `turn` of a chat with `partner`.
    fn chat_line(&mut self, partner: &Peer, turn: i64) -> String {
        let korean = self.speaks_korean();
        let name = &partner.name;
        let sound = self.ghost.species.sound();
        if turn == 0 {
            return if korean {
                format!("{name}, 안녕! {sound}")
            } else {
                format!("Hi, {name}! {sound}")
            };
        }
        if turn == 1 {
            let replies: [(&str, &str); 4] = [
                ("Oh, hello!", "어, 안녕!"),
                ("Hi hi!", "안녕 안녕!"),
                ("There you are!", "여기 있었구나!"),
                ("Hello, friend.", "안녕, 친구."),
            ];
            let (english, hangul) = replies[self.pick(&[1.0; 4]).unwrap_or(0)];
            return format!("{} {sound}", if korean { hangul } else { english });
        }
        if turn >= 4 {
            let partings: [(&str, &str); 3] = [
                ("Let's play again later!", "이따 또 놀자!"),
                ("You're my favorite.", "네가 제일 좋아."),
                ("Hehe.", "헤헤."),
            ];
            let (english, hangul) = partings[self.pick(&[1.0; 3]).unwrap_or(0)];
            return (if korean { hangul } else { english }).to_owned();
        }
        // The middle of a chat: about the other, the weather, or the viewer.
        let compliment = admire(partner.species, korean);
        let weather = match self.weather.sky {
            Sky::Rain if self.weather.storm => ("Thunder! Stay close to me.", "천둥이야! 내 옆에 있어."),
            Sky::Rain => ("Listen to the rain.", "빗소리 들어 봐."),
            Sky::Snow => ("Snow! Let's go out and play.", "눈이다! 나가서 놀자."),
            Sky::Rainbow => ("Did you see the rainbow?", "무지개 봤어?"),
            Sky::Clear if self.is_night() => ("The stars are out.", "별이 떴어."),
            Sky::Clear => ("It's a nice day.", "날씨 좋다."),
        };
        let viewer = if self.viewer_present() {
            ("Our friend is watching us!", "친구가 우리를 보고 있어!")
        } else {
            ("I miss our friend at the glass.", "유리창 너머 친구가 보고 싶어.")
        };
        let options = [
            compliment,
            (if korean { weather.1 } else { weather.0 }).to_owned(),
            (if korean { viewer.1 } else { viewer.0 }).to_owned(),
            if korean {
                "아까 빛 알갱이를 먹었어. 맛있었어!".to_owned()
            } else {
                "I caught a light mote earlier. Yummy!".to_owned()
            },
        ];
        let weights = [1.2, 1.0, 0.8, if self.life.today.motes > 0 { 0.8 } else { 0.0 }];
        options[self.pick(&weights).unwrap_or(0)].clone()
    }
}

/// Something nice to say about a housemate of `species`.
fn admire(species: super::Species, korean: bool) -> String {
    use super::Species::*;
    let (english, hangul) = match species {
        Ghost => ("You glow so softly.", "너는 은은하게 빛나."),
        Fox => ("Your tail is so fluffy!", "꼬리가 엄청 복슬복슬해!"),
        Bunny => ("Your ears are so long!", "귀가 엄청 길다!"),
        Penguin => ("I love how you waddle.", "뒤뚱뒤뚱 걷는 게 좋아."),
        Frog => ("How do you jump so high?", "어떻게 그렇게 높이 뛰어?"),
        Slime => ("You're so wobbly and shiny!", "말랑말랑 반짝반짝해!"),
        Owl => ("Can you teach me to fly?", "나는 법 가르쳐 줄래?"),
        Robot => ("Your antenna is blinking!", "안테나가 반짝여!"),
        Hedgehog => ("Your spines are so neat.", "가시가 정말 단정해."),
        Mushroom => ("I like your spotty hat.", "점박이 모자가 좋아."),
        Axolotl => ("Your frills are so pretty.", "아가미 장식이 예뻐."),
        Dragon => ("Can you breathe fire yet?", "이제 불 뿜을 수 있어?"),
        Jelly => ("You float like a dream.", "꿈처럼 둥둥 떠다녀."),
    };
    (if korean { hangul } else { english }).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::Species;

    fn pair() -> World {
        let mut world = World::new(11, 12.0);
        world.found_household(&[Species::Fox, Species::Bunny]);
        world
    }

    fn run(world: &mut World, seconds: f32) {
        for _ in 0..(seconds * 10.0) as usize {
            world.step(0.1);
        }
    }

    #[test]
    fn names_are_listed_the_way_people_say_them() {
        let names = |list: &[&str]| list.iter().map(|name| name.to_string()).collect::<Vec<_>>();
        assert_eq!(list(&names(&[])), "");
        assert_eq!(list(&names(&["Pip"])), "Pip");
        assert_eq!(list(&names(&["Pip", "Mochi"])), "Pip and Mochi");
        assert_eq!(list(&names(&["Pip", "Mochi", "Bean"])), "Pip, Mochi, and Bean");
    }

    #[test]
    fn an_invitation_is_taken_up_and_both_grow_fonder_after() {
        let mut world = pair();
        world.ghost.needs.energy = 0.9;
        world.with_creature(1, |world| {
            world.ghost.plan.done = true;
            world.ghost.needs.energy = 0.9;
        });
        world.look_at_housemates();
        assert!(world.begin_together());
        let pastime = world.ghost.plan.pastime.unwrap();
        run(&mut world, 1.0);
        let joined = world
            .with_creature(1, |world| {
                (world.ghost.plan.activity, world.ghost.plan.with, world.ghost.plan.pastime)
            })
            .unwrap();
        assert_eq!(joined, (Activity::Together, Some(0), Some(pastime)));
        run(&mut world, 60.0);
        assert!(world.fondness_for(1) > 0.0, "{pastime:?}");
        assert!(world.with_creature(1, |world| world.fondness_for(0)).unwrap() > 0.0);
        assert!(world.events.iter().any(|event| matches!(
            &event.kind,
            EventKind::Together { what, .. } if *what == pastime
        )));
    }

    #[test]
    fn a_chat_opens_with_a_hello_from_the_one_who_asked() {
        let mut world = pair();
        let partner = world.with_creature(1, |world| world.ghost.name.clone()).unwrap();
        world.with_creature(1, |world| world.ghost.plan.done = true);
        world.look_at_housemates();
        world.invitations.push(Invitation {
            from: 0,
            to: 1,
            pastime: Pastime::Chat,
            at: world.time,
        });
        // Wherever the partner stands, the opener is said.
        world.with_creature(1, |world| world.ghost.position.y = 1.9);
        world.look_at_housemates();
        world.join(1, Pastime::Chat, true);
        let mut said = Vec::new();
        for _ in 0..200 {
            world.step(0.1);
            if let Some(line) = &world.ghost.speech
                && said.last() != Some(&line.text)
            {
                said.push(line.text.clone());
            }
        }
        assert!(
            said.iter().any(|line| line.starts_with(&format!("Hi, {partner}!"))),
            "{said:?}"
        );
    }

    #[test]
    fn a_sleeping_housemate_lets_an_invitation_lapse() {
        let mut world = pair();
        world.hour = 23.0;
        world.with_creature(1, |world| {
            world.ghost.asleep = 1.0;
            world.ghost.plan = Plan::new(Activity::Rest, world.time, world.ghost.position, 0.0);
            world.ghost.plan.overnight = true;
        });
        world.look_at_housemates();
        assert!(world.together_partner().is_none());
        world.invitations.push(Invitation {
            from: 0,
            to: 1,
            pastime: Pastime::Chat,
            at: world.time,
        });
        world.join(1, Pastime::Chat, true);
        run(&mut world, 10.0);
        assert!(world.ghost.plan.done || world.ghost.plan.activity != Activity::Together);
        assert_eq!(world.with_creature(1, |world| world.ghost.plan.activity), Some(Activity::Rest));
    }
}
