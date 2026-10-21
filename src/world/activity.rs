use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::Sketch;

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
}

impl Activity {
    pub const ALL: [Self; 16] = [
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
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|activity| activity.name() == name)
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
    pub sketch: Option<Sketch>,
    /// How many times it has done the activity's action, such as nudging
    /// the ball or picking a new spot to explore.
    pub count: u32,
    /// Seconds it means to keep at an open-ended activity.
    pub length: f32,
    pub done: bool,
    /// A rest that lasts until morning, even when energy is full.
    pub overnight: bool,
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
            sketch: None,
            count: 0,
            length,
            done: false,
            overnight: false,
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
