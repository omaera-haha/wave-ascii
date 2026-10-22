use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::Picture;

/// Something the ghost can set out to do. The world carries it out; minds
/// only choose which.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Activity {
    Float,
    Explore,
    Chase,
    Rest,
    Play,
    Tend,
    Lamp,
    Visit,
    Window,
    Fetch,
    Dance,
    Hide,
    Doodle,
    Pounce,
    Game,
    Wait,
    Build,
    Friend,
    Look,
    Company,
    Air,
    Puzzle,
    Read,
    Music,
    Mirror,
    Letter,
    Comfort,
    Seek,
    Fold,
    Bubbles,
    Pin,
    Show,
    Tidy,
    Yard,
    Together,
}

impl Activity {
    pub const ALL: [Self; 35] = [
        Self::Float,
        Self::Explore,
        Self::Chase,
        Self::Rest,
        Self::Play,
        Self::Tend,
        Self::Lamp,
        Self::Visit,
        Self::Window,
        Self::Fetch,
        Self::Dance,
        Self::Hide,
        Self::Doodle,
        Self::Pounce,
        Self::Game,
        Self::Wait,
        Self::Build,
        Self::Friend,
        Self::Look,
        Self::Company,
        Self::Air,
        Self::Puzzle,
        Self::Read,
        Self::Music,
        Self::Mirror,
        Self::Letter,
        Self::Comfort,
        Self::Seek,
        Self::Fold,
        Self::Bubbles,
        Self::Pin,
        Self::Show,
        Self::Tidy,
        Self::Yard,
        Self::Together,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Explore => "explore",
            Self::Chase => "chase",
            Self::Rest => "rest",
            Self::Play => "play",
            Self::Tend => "tend",
            Self::Lamp => "lamp",
            Self::Visit => "visit",
            Self::Window => "window",
            Self::Fetch => "fetch",
            Self::Dance => "dance",
            Self::Hide => "hide",
            Self::Doodle => "doodle",
            Self::Pounce => "pounce",
            Self::Game => "game",
            Self::Wait => "wait",
            Self::Build => "build",
            Self::Friend => "friend",
            Self::Look => "look",
            Self::Company => "company",
            Self::Air => "air",
            Self::Puzzle => "puzzle",
            Self::Read => "read",
            Self::Music => "music",
            Self::Mirror => "mirror",
            Self::Letter => "letter",
            Self::Comfort => "comfort",
            Self::Seek => "seek",
            Self::Fold => "fold",
            Self::Bubbles => "bubbles",
            Self::Pin => "pin",
            Self::Show => "show",
            Self::Tidy => "tidy",
            Self::Yard => "yard",
            Self::Together => "together",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|activity| activity.name() == name)
    }

    /// The activity as it reads for a creature of `species`, which floats,
    /// flies, or keeps to the ground.
    pub fn doing_as(self, species: super::Species) -> &'static str {
        match (self, species.gait()) {
            (Self::Float, super::Gait::Walk | super::Gait::Hop) => "pottering about",
            (Self::Float, super::Gait::Fly) => "fluttering about",
            _ => self.doing(),
        }
    }

    /// The activity as the status line and the minds' notes describe it.
    pub fn doing(self) -> &'static str {
        match self {
            Self::Float => "floating about",
            Self::Explore => "exploring the room",
            Self::Chase => "chasing a light mote",
            Self::Rest => "resting on the cushion",
            Self::Play => "playing with the ball",
            Self::Tend => "tending the plant",
            Self::Lamp => "seeing to the lamp",
            Self::Visit => "visiting the viewer at the glass",
            Self::Window => "gazing out of the window",
            Self::Fetch => "fetching the ball for the viewer",
            Self::Dance => "dancing a little twirl",
            Self::Hide => "hiding from the viewer",
            Self::Doodle => "drawing on the glass",
            Self::Pounce => "chasing the viewer's finger on the glass",
            Self::Game => "playing noughts and crosses with the viewer on the glass",
            Self::Wait => "waiting at the glass for the viewer",
            Self::Build => "stacking the toy blocks into a tower",
            Self::Friend => "playing with its ghost friend",
            Self::Look => "looking at what the viewer drew on the glass",
            Self::Air => "seeing to the window",
            Self::Company => "keeping the viewer company while they work",
            Self::Puzzle => "playing a word-guessing game with the viewer on the glass",
            Self::Read => "reading a picture book",
            Self::Music => "listening to the music box",
            Self::Mirror => "looking in the mirror",
            Self::Letter => "reading your letter",
            Self::Comfort => "caring for how the viewer feels",
            Self::Seek => "searching for the viewer's treasure",
            Self::Fold => "folding a paper crane",
            Self::Bubbles => "blowing soap bubbles",
            Self::Pin => "pinning a picture to the gallery wall",
            Self::Show => "showing the viewer a treasure",
            Self::Tidy => "tidying the blocks into neat stacks before bed",
            Self::Yard => "out in the yard",
            Self::Together => "doing something with a housemate",
        }
    }
}

/// What the ghost is doing now and how far along it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub activity: Activity,
    pub started: f64,
    /// Where it is heading.
    pub target: Vec3,
    /// The center of an activity's path.
    pub anchor: Vec3,
    /// When it reached the place where the activity happens.
    pub arrived: Option<f64>,
    /// The mote it is chasing.
    pub mote: Option<u32>,
    /// The picture it means to draw on the glass.
    pub picture: Option<Picture>,
    /// How many times it has done the activity's action, such as nudging
    /// the ball or picking a new spot to explore.
    pub count: u32,
    /// Seconds it means to keep at an open-ended activity.
    pub length: f32,
    pub done: bool,
    /// A rest that lasts until morning, even when energy is full.
    pub overnight: bool,
    pub window: Option<bool>,
    pub gallery_visit: bool,
    /// An outing to keep the visiting cat company.
    pub cat: bool,
    /// The housemate it is doing this with, if any, and what.
    pub with: Option<u32>,
    pub pastime: Option<super::together::Pastime>,
}

impl Plan {
    pub fn new(activity: Activity, started: f64, target: Vec3, length: f32) -> Self {
        Self {
            activity,
            started,
            target,
            anchor: target,
            arrived: None,
            mote: None,
            picture: None,
            count: 0,
            length,
            done: false,
            overnight: false,
            window: None,
            gallery_visit: false,
            cat: false,
            with: None,
            pastime: None,
        }
    }

    pub fn elapsed(&self, time: f64) -> f32 {
        (time - self.started) as f32
    }

    /// Seconds since it arrived, or 0 before then.
    pub fn stayed(&self, time: f64) -> f32 {
        self.arrived.map_or(0.0, |arrived| (time - arrived) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_names_round_trip() {
        for activity in Activity::ALL {
            assert_eq!(Activity::parse(activity.name()), Some(activity));
        }
    }
}
