use serde::{Deserialize, Serialize};

/// What the ghost needs, each from 0 (in need) to 1 (satisfied).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Needs {
    /// Falls while it is awake and moving; sleep and light motes restore it.
    pub energy: f32,
    /// Falls while nothing happens; play, exploring, and attention restore it.
    pub fun: f32,
    /// Falls while it is alone; the viewer's attention restores it.
    pub company: f32,
}

impl Default for Needs {
    fn default() -> Self {
        Self {
            energy: 0.85,
            fun: 0.7,
            company: 0.6,
        }
    }
}

impl Needs {
    /// Adds the changes and keeps every need between 0 and 1.
    pub fn change(&mut self, energy: f32, fun: f32, company: f32) {
        self.energy = (self.energy + energy).clamp(0.0, 1.0);
        self.fun = (self.fun + fun).clamp(0.0, 1.0);
        self.company = (self.company + company).clamp(0.0, 1.0);
    }

    pub fn energy_words(&self) -> &'static str {
        words(
            self.energy,
            ["exhausted", "tired", "rested", "full of energy"],
        )
    }

    pub fn fun_words(&self) -> &'static str {
        words(
            self.fun,
            ["very bored", "a little bored", "entertained", "delighted"],
        )
    }

    pub fn company_words(&self) -> &'static str {
        words(
            self.company,
            ["very lonely", "a little lonely", "content", "well loved"],
        )
    }
}

// Four words for the quarters of a level from 0 to 1. System One models
// judge words better than numbers.
fn words(level: f32, quarters: [&'static str; 4]) -> &'static str {
    quarters[((level * 4.0) as usize).min(3)]
}

/// How the ghost feels, which shows in its face and the color of its glow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mood {
    #[default]
    Content,
    Cheerful,
    Excited,
    Curious,
    Sleepy,
    Lonely,
    Bored,
    Grumpy,
}

impl Mood {
    pub const ALL: [Self; 8] = [
        Self::Content,
        Self::Cheerful,
        Self::Excited,
        Self::Curious,
        Self::Sleepy,
        Self::Lonely,
        Self::Bored,
        Self::Grumpy,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Cheerful => "cheerful",
            Self::Excited => "excited",
            Self::Curious => "curious",
            Self::Sleepy => "sleepy",
            Self::Lonely => "lonely",
            Self::Bored => "bored",
            Self::Grumpy => "grumpy",
        }
    }

    /// When the mood fits, written for a model choosing among moods.
    pub fn description(self) -> &'static str {
        match self {
            Self::Content => "Calm and at ease; nothing much is needed or happening.",
            Self::Cheerful => "Happy and light, after something nice happened or with needs met.",
            Self::Excited => "Thrilled by something happening right now, like a catch or a game.",
            Self::Curious => "Interested in something new or unexplained around it.",
            Self::Sleepy => "Low on energy or it is late; wants to rest.",
            Self::Lonely => "Has been alone too long and wants the viewer's attention.",
            Self::Bored => "Nothing interesting has happened for a while.",
            Self::Grumpy => "Annoyed, for example after being poked again and again.",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mood| mood.name() == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn needs_stay_between_zero_and_one() {
        let mut needs = Needs::default();
        needs.change(5.0, -5.0, 0.1);
        assert_eq!((needs.energy, needs.fun), (1.0, 0.0));
    }

    #[test]
    fn levels_read_as_words_from_low_to_high() {
        let needs = Needs {
            energy: 0.05,
            fun: 0.5,
            company: 1.0,
        };
        assert_eq!(needs.energy_words(), "exhausted");
        assert_eq!(needs.fun_words(), "entertained");
        assert_eq!(needs.company_words(), "well loved");
    }

    #[test]
    fn mood_names_round_trip() {
        for mood in Mood::ALL {
            assert_eq!(Mood::parse(mood.name()), Some(mood));
        }
        assert_eq!(Mood::parse("furious"), None);
    }
}
