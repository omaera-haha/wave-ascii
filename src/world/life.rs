use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, GREETS_FROM, Nature, Occasion, Outcome, Vocabulary};

/// Days of the diary kept.
const DAYS_KEPT: usize = 60;
/// Bonds at which the ghost marks how close it has grown, and what it
/// says about it.
const CLOSER: [(f32, &str); 4] = [
    (0.3, "We got to know each other."),
    (0.55, "I've grown fond of you."),
    (0.8, "We're close now, you and I."),
    (0.95, "I'm devoted to you."),
];
/// Hours together the ghost marks, and what it says about them. Milestones
/// reach the minds as events, so they are written in words, not digits.
const HOURS_MARKED: [(f64, &str); 3] = [
    (1.0, "We spent our first hour together."),
    (10.0, "We've spent ten hours together."),
    (100.0, "We've spent a hundred hours together."),
];
/// Numbers of the viewer's words it knows that the ghost marks, after the
/// first.
const WORDS_MARKED: [(usize, &str); 4] = [
    (10, "I know ten of your words now."),
    (25, "I know twenty-five of your words now."),
    (50, "I know fifty of your words now."),
    (100, "I know a hundred of your words now."),
];
/// Words learned in a day that the diary names.
const WORDS_NAMED: usize = 4;
/// Light motes eaten that the ghost marks, after the first.
const MOTES_MARKED: [(u32, &str); 4] = [
    (50, "Ate my fiftieth light mote."),
    (100, "Ate my hundredth light mote."),
    (500, "Ate my five hundredth light mote."),
    (1000, "Ate my thousandth light mote."),
];
/// How fast the bond grows for each second the viewer is around, while it is
/// new. It slows as the bond deepens: about a third after an hour together,
/// two thirds after four, and most of the way after a dozen.
const GROWING: f32 = 0.43 / 3600.0;
/// How much of the bond fades for each day the viewer stays away, after the
/// first.
const FADING: f32 = 0.03;
/// Times an activity counts as done before the first, so a new ghost has no
/// habits and a few choices don't make one.
const PRIOR: f32 = 3.0;

/// What the ghost did and what happened to it over some span of time.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tally {
    /// Light motes it caught and ate.
    pub motes: u32,
    /// Times it pushed the ball.
    pub nudges: u32,
    /// Times it brought the ball back to the viewer.
    pub fetches: u32,
    /// Times it caught the viewer's finger on the glass.
    pub pounces: u32,
    pub dances: u32,
    pub celebrations: u32,
    /// The occasions it marked, kept once each for the diary.
    pub occasions: Vec<Occasion>,
    /// Times the viewer found it hiding, and it jumped out unfound.
    pub finds: u32,
    pub jumps: u32,
    /// Pictures it drew on the glass.
    pub doodles: u32,
    /// Finished games of noughts and crosses, from the ghost's side.
    #[serde(default)]
    pub games: u32,
    #[serde(default)]
    pub wins: u32,
    #[serde(default)]
    pub losses: u32,
    #[serde(default)]
    pub draws: u32,
    pub waterings: u32,
    /// Times the plant came into flower.
    pub blooms: u32,
    pub naps: u32,
    /// Overnight rests that ended naturally in the morning.
    pub nights: u32,
    pub showers: u32,
    pub storms: u32,
    pub snowfalls: u32,
    /// Rainbows it caught sight of.
    pub rainbows: u32,
    /// Messages the viewer typed to it.
    pub heard: u32,
    /// Times the viewer sounded pleased with it.
    pub praises: u32,
    /// Times it worked out what one of the viewer's words means.
    pub learned: u32,
    /// The first few words it worked out, kept for the diary.
    pub words: Vec<String>,
    pub pokes: u32,
    /// Seconds the viewer was around, kept in double precision so years of
    /// frames still add up.
    pub together: f64,
    /// Times it set out on each activity, by the activity's name.
    pub activities: BTreeMap<String, u32>,
    /// Visits it saw at the window and the lamp.
    pub birds: u32,
    pub moths: u32,
}

impl Tally {
    /// Whether nothing at all was counted.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    fn note(&mut self, kind: &EventKind) {
        match kind {
            EventKind::Began(activity) => {
                *self
                    .activities
                    .entry(activity.name().to_owned())
                    .or_default() += 1;
            }
            EventKind::AteMote => self.motes += 1,
            EventKind::NudgedBall => self.nudges += 1,
            EventKind::Fetched => self.fetches += 1,
            EventKind::Pounced => self.pounces += 1,
            EventKind::Danced => self.dances += 1,
            EventKind::Celebrated(occasion) => {
                self.celebrations += 1;
                if !self.occasions.contains(occasion) {
                    self.occasions.push(*occasion);
                }
            }
            EventKind::Found => self.finds += 1,
            EventKind::Jumped => self.jumps += 1,
            EventKind::Doodled(_) => self.doodles += 1,
            EventKind::GameOver(outcome) if outcome.finished() => {
                self.games += 1;
                match outcome {
                    Outcome::GhostWon { .. } => self.wins += 1,
                    Outcome::ViewerWon { .. } => self.losses += 1,
                    Outcome::Draw => self.draws += 1,
                    Outcome::Abandoned => unreachable!(),
                }
            }
            EventKind::Tended => self.waterings += 1,
            EventKind::Bloomed => self.blooms += 1,
            EventKind::FellAsleep => self.naps += 1,
            EventKind::Morning => self.nights += 1,
            EventKind::RainStarted => self.showers += 1,
            EventKind::StormStarted => self.storms += 1,
            EventKind::SnowStarted => self.snowfalls += 1,
            EventKind::Noticed(thing) if thing.starts_with("the rainbow") => self.rainbows += 1,
            EventKind::Noticed(thing) if thing == "a bird at the window" => self.birds += 1,
            EventKind::Noticed(thing) if thing == "a moth at the lamp" => self.moths += 1,
            EventKind::Heard(_) => self.heard += 1,
            EventKind::Praised => self.praises += 1,
            EventKind::LearnedWord { word, .. } => {
                self.learned += 1;
                if self.words.len() < WORDS_NAMED && !self.words.contains(word) {
                    self.words.push(word.clone());
                }
            }
            EventKind::Poked { .. } => self.pokes += 1,
            _ => {}
        }
    }

    /// Times it set out on `activity`.
    pub fn times(&self, activity: Activity) -> u32 {
        self.activities.get(activity.name()).copied().unwrap_or(0)
    }

    /// The day in a few sentences, as the ghost would write it in its diary
    /// to the viewer.
    pub fn words(&self) -> String {
        let mut deeds: Vec<String> = self
            .occasions
            .iter()
            .map(|occasion| occasion.diary_words().to_owned())
            .collect();
        match self.motes {
            0 => {}
            1 => deeds.push("ate a light mote".to_owned()),
            motes => deeds.push(format!("ate {motes} light motes")),
        }
        match self.nudges {
            0 => {}
            1 => deeds.push("nudged the ball".to_owned()),
            nudges => deeds.push(format!("pushed the ball around {nudges} times")),
        }
        match self.fetches {
            0 => {}
            1 => deeds.push("fetched the ball for you".to_owned()),
            fetches => deeds.push(format!("fetched the ball for you {fetches} times")),
        }
        match self.pounces {
            0 => {}
            1 => deeds.push("pounced on your finger".to_owned()),
            pounces => deeds.push(format!("pounced on your finger {pounces} times")),
        }
        match self.dances {
            0 => {}
            1 => deeds.push("danced a twirl".to_owned()),
            dances => deeds.push(format!("danced {dances} twirls")),
        }
        match (self.finds, self.jumps) {
            (0, 0) => {}
            (0, _) => deeds.push("played hide-and-seek, and you never found me".to_owned()),
            (_, 0) => deeds.push("played hide-and-seek, and you found me".to_owned()),
            _ => deeds.push("played hide-and-seek".to_owned()),
        }
        if self.games == 1 {
            let result = if self.wins > 0 {
                "and won"
            } else if self.losses > 0 {
                "and you won"
            } else {
                "and drew"
            };
            deeds.push(format!("played noughts and crosses with you {result}"));
        } else if self.games > 1 {
            let mut results = Vec::new();
            if self.wins > 0 {
                results.push("I won".to_owned());
            }
            if self.losses > 0 {
                results.push("you won".to_owned());
            }
            if self.draws > 0 {
                results.push("we drew".to_owned());
            }
            deeds.push(format!(
                "played games of noughts and crosses with you: {}",
                listed(&results)
            ));
        }
        match self.doodles {
            0 => {}
            1 => deeds.push("drew a picture on the glass".to_owned()),
            doodles => deeds.push(format!("drew {doodles} pictures on the glass")),
        }
        match (self.waterings, self.blooms) {
            (0, 0) => {}
            (0, _) => deeds.push("saw the plant flower".to_owned()),
            (_, 0) => deeds.push("watered the plant".to_owned()),
            _ => deeds.push("watered the plant until it flowered".to_owned()),
        }
        match self.naps {
            0 => {}
            1 => deeds.push("took a nap".to_owned()),
            naps => deeds.push(format!("took {naps} naps")),
        }
        if self.nights > 0 {
            deeds.push("slept through the night".to_owned());
        }
        if self.snowfalls > 0 {
            deeds.push("watched the snow fall".to_owned());
        }
        if self.birds > 0 {
            deeds.push("watched a bird at the window".to_owned());
        }
        if self.moths > 0 {
            deeds.push("saw a moth at the lamp".to_owned());
        }
        if self.storms > 0 {
            deeds.push("sat out a thunderstorm".to_owned());
        }
        if !self.words.is_empty() {
            let quoted: Vec<String> = self.words.iter().map(|word| format!("“{word}”")).collect();
            deeds.push(match quoted.as_slice() {
                [word] => format!("learned what {word} means"),
                words => format!("learned what {} mean", listed(words)),
            });
        }
        match (self.rainbows, self.showers) {
            (0, 0) => {}
            (0, _) => deeds.push("listened to the rain".to_owned()),
            (1, _) => deeds.push("saw a rainbow".to_owned()),
            (rainbows, _) => deeds.push(format!("saw {rainbows} rainbows")),
        }
        let mut text = if deeds.is_empty() {
            "A quiet day.".to_owned()
        } else {
            format!("{}.", capitalized(&listed(&deeds)))
        };
        if self.together < 60.0 && self.heard == 0 {
            text.push_str(" You didn't come by.");
        } else {
            let talked = if self.heard > 0 {
                ", and we talked"
            } else {
                ""
            };
            text.push_str(&format!(
                " You were here {}{talked}.",
                duration_words(self.together)
            ));
        }
        if self.pokes >= 6 {
            text.push_str(" You poked me a lot.");
        }
        text
    }
}

/// A day the ghost lived, in its own words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// The local date, as YYYY-MM-DD.
    pub day: String,
    pub text: String,
}

/// The ghost's life over days: how close it has grown to the viewer, what
/// it has done, the habits that grew from that, and a diary of past days.
/// It is kept between runs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Life {
    /// The nature it was born with, absent in saves from before natures.
    #[serde(default)]
    pub nature: Option<Nature>,
    /// How close it feels to the viewer, from a stranger at 0 to devoted
    /// at 1.
    pub bond: f32,
    /// The closest it has ever felt. Half of that never fades.
    pub closest: f32,
    /// Everything since it first appeared.
    pub total: Tally,
    /// The day so far.
    pub today: Tally,
    /// Which day `today` counts, as YYYY-MM-DD, or empty before the first.
    pub day: String,
    /// A line for each past day it lived, oldest first.
    pub diary: Vec<Entry>,
    /// Its firsts and landmarks, kept for good, oldest first.
    pub milestones: Vec<Entry>,
    /// Seconds since the viewer was last around, while the app ran and
    /// while it was closed.
    pub alone: f64,
    /// The viewer's words it has picked up, and what it thinks they mean.
    pub vocabulary: Vocabulary,
}

impl Life {
    /// Counts what happened, and lets it warm or cool the bond. Returns the
    /// milestone this reached, if any.
    pub fn note(&mut self, kind: &EventKind) -> Option<String> {
        self.total.note(kind);
        self.today.note(kind);
        let bond = self.bond;
        let milestone = self.first(kind);
        let warmth = match kind {
            EventKind::Heard(_) => 0.015,
            EventKind::Praised | EventKind::LearnedWord { .. } => 0.005,
            EventKind::Fetched => 0.01,
            EventKind::Pounced => 0.003,
            EventKind::Found => 0.015,
            EventKind::ViewerArrived => 0.003,
            EventKind::DroppedMote | EventKind::KickedBall => 0.002,
            EventKind::GameOver(outcome) if outcome.finished() => 0.002,
            EventKind::Poked { count } if *count < 3 => 0.002,
            EventKind::Poked { .. } => -0.012,
            _ => 0.0,
        };
        self.warm(warmth);
        milestone.or_else(|| self.closer(bond))
    }

    /// Lets `seconds` pass, with the viewer around or not. Time together
    /// deepens the bond, more slowly the deeper it is. Returns the milestone
    /// this reached, if any.
    pub fn pass(&mut self, seconds: f32, present: bool) -> Option<String> {
        if !present {
            self.miss(seconds);
            return None;
        }
        self.alone = 0.0;
        let (bond, together) = (self.bond, self.total.together);
        self.total.together += f64::from(seconds);
        self.today.together += f64::from(seconds);
        let nature = self.nature.unwrap_or_default();
        let shyness = if self.bond < 0.3 {
            0.4 * (nature.shy - 0.5)
        } else {
            0.0
        };
        let warmth = (1.0 + 0.4 * (nature.tender - 0.5) - shyness).clamp(0.75, 1.35);
        self.warm(GROWING * (1.0 - self.bond).powi(2) * seconds * warmth);
        let milestone = HOURS_MARKED
            .into_iter()
            .find(|(hours, _)| (together..self.total.together).contains(&(hours * 3600.0)))
            .and_then(|(_, text)| self.reach(text.to_owned()));
        milestone.or_else(|| self.closer(bond))
    }

    /// The viewer stayed away for `seconds` more. After the first day alone,
    /// the bond fades a little each day, but never below half of the
    /// closest it was.
    pub fn miss(&mut self, seconds: f32) {
        let before = self.alone;
        self.alone += f64::from(seconds.max(0.0));
        let past_first = |alone: f64| (alone / 86_400.0 - 1.0).max(0.0);
        let days = past_first(self.alone) - past_first(before);
        if days > 0.0 {
            let holding = 1.15 - 0.3 * self.nature.unwrap_or_default().shy;
            self.warm(-FADING * days as f32 * holding);
        }
    }

    /// How close it needs to feel before coming to greet the viewer.
    pub fn greeting_bond(&self) -> f32 {
        let nature = self.nature.unwrap_or_default();
        GREETS_FROM
            * (1.0 + 0.4 * (nature.shy - 0.5) - 0.2 * (nature.tender - 0.5)).clamp(0.75, 1.35)
    }

    /// Starts counting `day`, a local date as YYYY-MM-DD. When it is a new
    /// day, the last one goes into the diary, unless nothing happened in it.
    pub fn begin_day(&mut self, day: &str) {
        if self.day == day {
            return;
        }
        if !self.day.is_empty() && !self.today.is_empty() {
            self.diary.push(Entry {
                day: self.day.clone(),
                text: self.today.words(),
            });
            if self.diary.len() > DAYS_KEPT {
                let excess = self.diary.len() - DAYS_KEPT;
                self.diary.drain(..excess);
            }
        }
        self.day = day.to_owned();
        self.today = Tally::default();
    }

    /// Starts a new ghost's life on `day`, a local date as YYYY-MM-DD, with
    /// the first moment it keeps.
    pub fn begin(&mut self, day: &str) -> Option<String> {
        self.begin_day(day);
        self.reach("I came into being in a glass box.".to_owned())
    }

    /// How much more, or less, than a new ghost it leans toward `activity`
    /// because of how often it chose it before: from 0.85 for what it never
    /// does to 1.3 for what it does most. Floating is no habit.
    pub fn habit(&self, activity: Activity) -> f32 {
        if activity == Activity::Float {
            return 1.0;
        }
        let kinds = Activity::ALL.len() as f32 - 1.0;
        let total: u32 = Activity::ALL
            .into_iter()
            .filter(|other| *other != Activity::Float)
            .map(|other| self.total.times(other))
            .sum();
        let share = (self.total.times(activity) as f32 + PRIOR) / (total as f32 + PRIOR * kinds);
        (0.85 + 0.15 * share * kinds).clamp(0.85, 1.3)
    }

    /// What it has done most, once it has done it a few times.
    pub fn favorite(&self) -> Option<Activity> {
        Activity::ALL
            .into_iter()
            .filter(|activity| *activity != Activity::Float)
            .map(|activity| (activity, self.total.times(activity)))
            .filter(|(_, times)| *times >= 5)
            .max_by_key(|(_, times)| *times)
            .map(|(activity, _)| activity)
    }

    /// How close it feels to the viewer, in a few words about "them".
    pub fn bond_words(&self) -> &'static str {
        match self.bond {
            bond if bond < 0.12 => "barely knows them",
            bond if bond < 0.3 => "getting to know them",
            bond if bond < 0.55 => "fond of them",
            bond if bond < 0.8 => "close to them",
            _ => "devoted to them",
        }
    }

    /// The life as saved, with values kept within their ranges, so a
    /// hand-edited or damaged save can't break the world.
    pub fn sanitized(mut self) -> Self {
        self.nature = self.nature.map(Nature::sanitized);
        let unit = |value: f32| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        self.closest = unit(self.closest);
        self.bond = unit(self.bond).max(self.closest * 0.5);
        self.closest = self.closest.max(self.bond);
        for tally in [&mut self.total, &mut self.today] {
            if !tally.together.is_finite() || tally.together < 0.0 {
                tally.together = 0.0;
            }
        }
        if !self.alone.is_finite() || self.alone < 0.0 {
            self.alone = 0.0;
        }
        self.vocabulary = self.vocabulary.sanitized();
        self
    }

    // The first time something happens, or a count it marks.
    fn first(&mut self, kind: &EventKind) -> Option<String> {
        let total = &self.total;
        if let EventKind::GameOver(outcome) = kind {
            let firsts = [
                (
                    outcome.finished() && total.games == 1,
                    "We played our first game of noughts and crosses.",
                ),
                (
                    matches!(outcome, Outcome::ViewerWon { .. }) && total.losses == 1,
                    "You beat me at noughts and crosses for the first time.",
                ),
                (
                    matches!(outcome, Outcome::GhostWon { .. }) && total.wins == 1,
                    "I won my first game of noughts and crosses.",
                ),
            ];
            let mut first = None;
            for (reached, text) in firsts {
                if reached {
                    let milestone = self.reach(text.to_owned());
                    first = first.or(milestone);
                }
            }
            return first;
        }
        let text = match kind {
            EventKind::AteMote if total.motes == 1 => "Caught my first light mote.".to_owned(),
            EventKind::AteMote => MOTES_MARKED
                .into_iter()
                .find(|(motes, _)| *motes == total.motes)?
                .1
                .to_owned(),
            EventKind::NudgedBall if total.nudges == 1 => {
                "Played with the ball for the first time.".to_owned()
            }
            EventKind::Fetched if total.fetches == 1 => {
                "Played fetch with you for the first time.".to_owned()
            }
            EventKind::Pounced if total.pounces == 1 => {
                "Caught your finger on the glass for the first time.".to_owned()
            }
            EventKind::Danced if total.dances == 1 => "Danced my first twirl.".to_owned(),
            EventKind::Found if total.finds == 1 => "You found me hiding.".to_owned(),
            EventKind::Doodled(_) if total.doodles == 1 => {
                "Drew my first picture on the glass.".to_owned()
            }
            EventKind::Bloomed if total.blooms == 1 => {
                "My plant flowered for the first time.".to_owned()
            }
            EventKind::Morning if total.nights == 1 => {
                "Slept through my first night in the glass box.".to_owned()
            }
            EventKind::Noticed(thing)
                if thing.starts_with("the rainbow") && total.rainbows == 1 =>
            {
                "Saw my first rainbow.".to_owned()
            }
            EventKind::StormStarted if total.storms == 1 => {
                "Sat out my first thunderstorm.".to_owned()
            }
            EventKind::Noticed(thing) if thing == "a bird at the window" && total.birds == 1 => {
                "Saw my first bird.".to_owned()
            }
            EventKind::Noticed(thing) if thing == "a moth at the lamp" && total.moths == 1 => {
                "A moth came to visit my lamp.".to_owned()
            }
            EventKind::SnowStarted if total.snowfalls == 1 => {
                "Saw snow fall for the first time.".to_owned()
            }
            EventKind::Celebrated(occasion) => match occasion {
                Occasion::Halloween => "My first Halloween in the glass box.",
                Occasion::WinterHolidays => "My first winter holidays.",
                Occasion::NewYear => "Saw in my first new year.",
                Occasion::Valentine => "My first Valentine's Day.",
                Occasion::Birthday => "My first birthday!",
            }
            .to_owned(),
            EventKind::Heard(_) if total.heard == 1 => {
                "You talked to me for the first time.".to_owned()
            }
            EventKind::LearnedWord { word, .. } if total.learned == 1 => {
                format!("Learned my first word from you: “{word}”.")
            }
            EventKind::LearnedWord { .. } => {
                let known = self.vocabulary.known().len();
                // Several words can be learned at once, so a mark passed
                // without being kept is kept now.
                WORDS_MARKED
                    .into_iter()
                    .filter(|(count, _)| *count <= known)
                    .find(|(_, text)| {
                        !self
                            .milestones
                            .iter()
                            .any(|milestone| milestone.text == *text)
                    })?
                    .1
                    .to_owned()
            }
            _ => return None,
        };
        self.reach(text)
    }

    // The bond grew past one of the marks since it was `before`.
    fn closer(&mut self, before: f32) -> Option<String> {
        let (_, text) = CLOSER
            .into_iter()
            .rev()
            .find(|(mark, _)| before < *mark && self.bond >= *mark)?;
        self.reach(text.to_owned())
    }

    // Keeps a milestone, once; returns it if it is new.
    fn reach(&mut self, text: String) -> Option<String> {
        if self
            .milestones
            .iter()
            .any(|milestone| milestone.text == text)
        {
            return None;
        }
        self.milestones.push(Entry {
            day: self.day.clone(),
            text: text.clone(),
        });
        Some(text)
    }

    fn warm(&mut self, amount: f32) {
        self.bond = (self.bond + amount).clamp(self.closest * 0.5, 1.0);
        self.closest = self.closest.max(self.bond);
    }
}

/// A span of time in loose words, for the diary.
fn duration_words(seconds: f64) -> String {
    let minutes = seconds / 60.0;
    match minutes {
        minutes if minutes < 5.0 => "for a few minutes".to_owned(),
        minutes if minutes < 20.0 => "for a while".to_owned(),
        minutes if minutes < 45.0 => "for about half an hour".to_owned(),
        minutes if minutes < 90.0 => "for about an hour".to_owned(),
        minutes => format!("for about {} hours", (minutes / 60.0).round()),
    }
}

/// Items joined as "a", "a and b", or "a, b, and c".
fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

fn capitalized(text: &str) -> String {
    let mut characters = text.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finished_games_count_results_warm_the_bond_and_keep_all_firsts_once() {
        let mut life = Life::default();
        let won = EventKind::GameOver(Outcome::GhostWon { line: [0, 1, 2] });
        let lost = EventKind::GameOver(Outcome::ViewerWon { line: [0, 1, 2] });
        life.note(&EventKind::GameOver(Outcome::Abandoned));
        assert_eq!(life.total.games, 0);
        assert_eq!(life.bond, 0.0);
        assert!(life.milestones.is_empty());
        life.note(&won);
        assert!(life.bond > 0.0);
        assert_eq!(life.milestones.len(), 2);
        assert!(
            life.today
                .words()
                .contains("Played noughts and crosses with you and won")
        );
        life.note(&lost);
        life.note(&EventKind::GameOver(Outcome::Draw));
        assert_eq!(
            (
                life.total.games,
                life.total.wins,
                life.total.losses,
                life.total.draws
            ),
            (3, 1, 1, 1)
        );
        assert_eq!(life.milestones.len(), 3);
        assert!(life.milestones.iter().any(|entry| entry.text == "You beat me at noughts and crosses for the first time."));
        assert!(life.milestones.iter().all(|entry| {
            !entry
                .text
                .chars()
                .any(|character| character.is_ascii_digit())
        }));
        assert!(life.today.words().contains("I won, you won, and we drew"));
        life.note(&won);
        life.note(&lost);
        assert_eq!(life.milestones.len(), 3);
        let loaded: Life = serde_json::from_str(&serde_json::to_string(&life).unwrap()).unwrap();
        assert_eq!(loaded, life);
        let old: Tally = serde_json::from_str(r#"{"doodles": 2}"#).unwrap();
        assert_eq!((old.games, old.wins, old.losses, old.draws), (0, 0, 0, 0));
    }

    #[test]
    fn mornings_count_nights_and_remember_the_first_one_once() {
        let mut life = Life::default();
        let first = "Slept through my first night in the glass box.";
        assert_eq!(life.note(&EventKind::Morning).as_deref(), Some(first));
        assert_eq!(life.today.nights, 1);
        assert_eq!(life.total.nights, 1);
        assert!(life.today.words().contains("Slept through the night"));
        assert_eq!(life.note(&EventKind::Morning), None);
        assert_eq!(life.total.nights, 2);
        assert_eq!(life.milestones.len(), 1);
    }

    #[test]
    fn old_tallies_load_without_any_nights() {
        let tally: Tally = serde_json::from_str(r#"{"naps": 2}"#).unwrap();
        assert_eq!(tally.naps, 2);
        assert_eq!(tally.nights, 0);
    }

    #[test]
    fn a_day_is_written_in_words_from_what_happened() {
        let mut day = Tally {
            motes: 4,
            nudges: 1,
            waterings: 1,
            blooms: 1,
            rainbows: 1,
            showers: 1,
            together: 3900.0,
            heard: 2,
            ..Tally::default()
        };
        assert_eq!(
            day.words(),
            "Ate 4 light motes, nudged the ball, watered the plant until it flowered, and saw a \
             rainbow. You were here for about an hour, and we talked."
        );
        day = Tally {
            fetches: 3,
            dances: 1,
            together: 1500.0,
            ..Tally::default()
        };
        assert_eq!(
            day.words(),
            "Fetched the ball for you 3 times and danced a twirl. You were here for about half \
             an hour."
        );
        day = Tally {
            naps: 2,
            showers: 1,
            ..Tally::default()
        };
        assert_eq!(
            day.words(),
            "Took 2 naps and listened to the rain. You didn't come by."
        );
        day = Tally {
            finds: 1,
            doodles: 2,
            ..Tally::default()
        };
        assert_eq!(
            day.words(),
            "Played hide-and-seek, and you found me and drew 2 pictures on the glass. You \
             didn't come by."
        );
        day = Tally {
            snowfalls: 2,
            ..Tally::default()
        };
        assert_eq!(day.words(), "Watched the snow fall. You didn't come by.");
        day = Tally {
            together: 200.0,
            pokes: 8,
            ..Tally::default()
        };
        assert_eq!(
            day.words(),
            "A quiet day. You were here for a few minutes. You poked me a lot."
        );
    }

    #[test]
    fn events_are_counted_for_the_day_and_for_all_time() {
        let mut life = Life::default();
        life.begin_day("2026-10-20");
        for kind in [
            EventKind::AteMote,
            EventKind::AteMote,
            EventKind::Began(Activity::Play),
            EventKind::NudgedBall,
        ] {
            life.note(&kind);
        }
        assert_eq!((life.today.motes, life.total.motes), (2, 2));
        assert_eq!(life.total.times(Activity::Play), 1);
        life.begin_day("2026-10-21");
        life.note(&EventKind::AteMote);
        assert_eq!((life.today.motes, life.total.motes), (1, 3));
        assert_eq!(life.diary.len(), 1);
        assert_eq!(life.diary[0].day, "2026-10-20");
        assert!(life.diary[0].text.starts_with("Ate 2 light motes"));
    }

    #[test]
    fn pounces_count_warm_the_bond_and_mark_the_first_catch() {
        let mut life = Life::default();
        life.begin_day("2026-10-20");
        let first = life.note(&EventKind::Pounced).unwrap();
        assert_eq!(first, "Caught your finger on the glass for the first time.");
        assert!(!first.chars().any(|character| character.is_ascii_digit()));
        assert_eq!((life.today.pounces, life.total.pounces), (1, 1));
        assert!((life.bond - 0.003).abs() < 1e-6);
        assert!(life.today.words().starts_with("Pounced on your finger."));
        assert_eq!(life.note(&EventKind::Pounced), None);
        assert!((life.bond - 0.006).abs() < 1e-6);
        life.begin_day("2026-10-21");
        assert_eq!((life.today.pounces, life.total.pounces), (0, 2));
        assert!(
            life.diary[0]
                .text
                .starts_with("Pounced on your finger 2 times.")
        );
        assert_eq!(serde_json::from_str::<Tally>("{}").unwrap().pounces, 0);
        let saved = serde_json::to_string(&life).unwrap();
        assert_eq!(serde_json::from_str::<Life>(&saved).unwrap(), life);
    }

    #[test]
    fn storms_count_once_and_leave_a_diary_entry_without_changing_the_bond() {
        let mut life = Life::default();
        life.begin_day("2026-06-21");
        life.note(&EventKind::StormStarted);
        for _ in 0..12 {
            life.note(&EventKind::Thunder);
        }
        assert_eq!((life.today.storms, life.total.storms), (1, 1));
        assert_eq!(life.today.showers, 0);
        assert_eq!(life.bond, 0.0);
        life.begin_day("2026-06-22");
        assert_eq!(life.today.storms, 0);
        assert_eq!(
            life.diary[0].text,
            "Sat out a thunderstorm. You didn't come by."
        );
        assert_eq!(serde_json::from_str::<Tally>("{}").unwrap().storms, 0);
    }

    #[test]
    fn the_same_day_again_changes_nothing_and_empty_days_leave_no_entry() {
        let mut life = Life::default();
        life.begin_day("2026-10-20");
        life.note(&EventKind::Tended);
        life.begin_day("2026-10-20");
        assert_eq!(life.today.waterings, 1);
        life.begin_day("2026-10-21");
        life.begin_day("2026-10-22");
        assert_eq!(life.diary.len(), 1);
        assert_eq!(life.day, "2026-10-22");
    }

    #[test]
    fn the_diary_keeps_the_latest_days() {
        let mut life = Life::default();
        for day in 0..(DAYS_KEPT + 10) {
            life.begin_day(&format!("day {day}"));
            life.note(&EventKind::AteMote);
        }
        life.begin_day("last");
        assert_eq!(life.diary.len(), DAYS_KEPT);
        assert_eq!(
            life.diary.last().unwrap().day,
            format!("day {}", DAYS_KEPT + 9)
        );
    }

    #[test]
    fn time_together_deepens_the_bond_ever_more_slowly() {
        let mut life = Life::default();
        let mut after = Vec::new();
        for hours in [1, 3, 8] {
            for _ in 0..(hours * 3600) {
                life.pass(1.0, true);
            }
            after.push(life.bond);
        }
        assert!((0.25..0.35).contains(&after[0]), "{after:?}");
        assert!((0.55..0.7).contains(&after[1]), "{after:?}");
        assert!((0.8..0.9).contains(&after[2]), "{after:?}");
        let before = life.bond;
        life.pass(3600.0, false);
        assert_eq!(life.bond, before);
        assert_eq!(life.total.together, 12.0 * 3600.0);
    }

    #[test]
    fn shy_ghosts_warm_more_slowly_at_first_and_tender_ones_more_quickly() {
        let mut average = Life::default();
        let mut shy = Life {
            nature: Some(Nature {
                shy: 1.0,
                ..Nature::default()
            }),
            ..Life::default()
        };
        let mut tender = Life {
            nature: Some(Nature {
                tender: 1.0,
                ..Nature::default()
            }),
            ..Life::default()
        };
        for _ in 0..1800 {
            for life in [&mut average, &mut shy, &mut tender] {
                life.pass(1.0, true);
            }
        }
        assert!(shy.bond < average.bond && average.bond < tender.bond);
        assert!(shy.greeting_bond() > average.greeting_bond());
        assert!(tender.greeting_bond() < average.greeting_bond());
        for life in [&mut average, &mut shy] {
            life.bond = 0.5;
            life.closest = 0.5;
        }
        average.pass(1.0, true);
        shy.pass(1.0, true);
        assert_eq!(average.bond, shy.bond);
        average.miss(3.0 * 86_400.0);
        shy.miss(3.0 * 86_400.0);
        assert!(shy.bond > average.bond);
    }

    #[test]
    fn loaded_nature_values_are_kept_in_range_and_missing_traits_are_average() {
        let life = Life {
            nature: Some(Nature {
                playful: -2.0,
                curious: 3.0,
                shy: f32::NAN,
                sleepy: f32::INFINITY,
                tender: 0.8,
            }),
            ..Life::default()
        }
        .sanitized();
        assert_eq!(
            life.nature,
            Some(Nature {
                playful: 0.0,
                curious: 1.0,
                shy: 0.5,
                sleepy: 0.5,
                tender: 0.8,
            })
        );
        let missing: Life = serde_json::from_str(r#"{"bond": 0.2}"#).unwrap();
        assert_eq!(missing.nature, None);
        let partial: Life = serde_json::from_str(r#"{"nature": {"playful": 0.8}}"#).unwrap();
        assert_eq!(
            partial.nature,
            Some(Nature {
                playful: 0.8,
                ..Nature::default()
            })
        );
    }

    #[test]
    fn days_alone_fade_the_bond_whether_the_app_runs_or_not() {
        let mut running = Life {
            bond: 0.8,
            closest: 0.8,
            ..Life::default()
        };
        let mut closed = running.clone();
        for _ in 0..(3 * 24 * 60) {
            running.pass(60.0, false);
        }
        closed.miss(86_400.0);
        closed.miss(86_400.0 * 2.0);
        assert!((running.bond - 0.74).abs() < 1e-3, "{}", running.bond);
        assert!((closed.bond - 0.74).abs() < 1e-3, "{}", closed.bond);
        running.pass(1.0, true);
        assert_eq!(running.alone, 0.0);
    }

    #[test]
    fn long_times_together_still_add_up() {
        let mut life = Life {
            total: Tally {
                together: 600_000.0,
                ..Tally::default()
            },
            ..Life::default()
        };
        for _ in 0..600 {
            life.pass(1.0 / 60.0, true);
        }
        assert!((life.total.together - 600_010.0).abs() < 1e-3);
    }

    #[test]
    fn talking_warms_the_bond_and_pestering_cools_it() {
        let mut life = Life::default();
        life.note(&EventKind::Heard("hello".into()));
        let warmed = life.bond;
        assert!(warmed > 0.0);
        for count in 1..=6 {
            life.note(&EventKind::Poked { count });
        }
        assert!(life.bond < warmed);
    }

    #[test]
    fn a_long_absence_fades_the_bond_but_never_below_half_the_closest() {
        let mut life = Life {
            bond: 0.8,
            closest: 0.8,
            ..Life::default()
        };
        life.miss(3600.0 * 20.0);
        assert_eq!(life.bond, 0.8);
        life.miss(86_400.0 * 3.0);
        // Twenty hours and then three days: almost four days alone in all.
        assert!((life.bond - 0.715).abs() < 1e-4, "{}", life.bond);
        life.miss(86_400.0 * 365.0);
        assert_eq!(life.bond, 0.4);
        assert_eq!(life.closest, 0.8);
    }

    #[test]
    fn habits_grow_from_what_the_ghost_chooses_often() {
        let mut life = Life::default();
        for activity in Activity::ALL {
            assert!((life.habit(activity) - 1.0).abs() < 1e-5, "{activity:?}");
        }
        assert_eq!(life.favorite(), None);
        for _ in 0..40 {
            life.note(&EventKind::Began(Activity::Play));
        }
        for _ in 0..6 {
            life.note(&EventKind::Began(Activity::Window));
            life.note(&EventKind::Began(Activity::Float));
        }
        assert!(life.habit(Activity::Play) > 1.2);
        assert!(life.habit(Activity::Explore) < 0.95);
        assert_eq!(life.habit(Activity::Float), 1.0);
        assert_eq!(life.favorite(), Some(Activity::Play));
    }

    #[test]
    fn a_new_life_begins_with_its_first_moment() {
        let mut life = Life::default();
        assert!(life.begin("2026-10-21").is_some());
        assert_eq!(life.day, "2026-10-21");
        assert_eq!(life.milestones[0].day, "2026-10-21");
        assert_eq!(life.begin("2026-10-21"), None);
    }

    #[test]
    fn firsts_and_landmarks_are_kept_once() {
        let mut life = Life::default();
        life.begin_day("2026-10-21");
        assert_eq!(
            life.note(&EventKind::AteMote).as_deref(),
            Some("Caught my first light mote.")
        );
        assert_eq!(life.note(&EventKind::AteMote), None);
        for _ in 2..50 {
            life.note(&EventKind::AteMote);
        }
        assert_eq!(
            life.milestones.last().unwrap().text,
            "Ate my fiftieth light mote."
        );
        assert_eq!(life.milestones[0].day, "2026-10-21");
        let reached = life.milestones.len();
        life.total.motes = 0;
        life.note(&EventKind::AteMote);
        assert_eq!(life.milestones.len(), reached);
    }

    #[test]
    fn learning_several_words_at_once_still_marks_ten_words() {
        let mut life = Life::default();
        let words: Vec<String> = [
            "ball", "dance", "nap", "water", "lamp", "hide", "draw", "spin", "look",
        ]
        .map(str::to_owned)
        .to_vec();
        for word in &words {
            let learned = life
                .vocabulary
                .teach(std::slice::from_ref(word), Activity::Play, 2.0);
            for word in learned {
                life.note(&EventKind::LearnedWord {
                    word,
                    activity: Activity::Play,
                });
            }
        }
        assert_eq!(life.vocabulary.known().len(), 9);
        let pair = ["kick".to_owned(), "roll".to_owned()];
        let mut marked = Vec::new();
        for word in life.vocabulary.teach(&pair, Activity::Play, 2.0) {
            marked.extend(life.note(&EventKind::LearnedWord {
                word,
                activity: Activity::Play,
            }));
        }
        assert_eq!(marked, ["I know ten of your words now."]);
    }

    #[test]
    fn the_first_of_each_special_day_is_a_milestone() {
        let mut life = Life::default();
        let halloween = EventKind::Celebrated(Occasion::Halloween);
        assert_eq!(
            life.note(&halloween).as_deref(),
            Some("My first Halloween in the glass box.")
        );
        assert_eq!(life.note(&halloween), None);
        assert!(
            life.note(&EventKind::Celebrated(Occasion::Birthday))
                .is_some()
        );
    }

    #[test]
    fn growing_closer_and_hours_together_are_milestones() {
        let mut life = Life::default();
        let mut reached = Vec::new();
        for _ in 0..(11 * 3600) {
            if let Some(text) = life.pass(1.0, true) {
                reached.push(text);
            }
        }
        assert_eq!(
            reached,
            [
                "We got to know each other.",
                "We spent our first hour together.",
                "I've grown fond of you.",
                "We're close now, you and I.",
                "We've spent ten hours together.",
            ]
        );
    }

    #[test]
    fn bond_words_rise_with_the_bond() {
        let words: Vec<&str> = [0.0, 0.2, 0.4, 0.7, 0.95]
            .into_iter()
            .map(|bond| {
                Life {
                    bond,
                    ..Life::default()
                }
                .bond_words()
            })
            .collect();
        assert_eq!(
            words,
            [
                "barely knows them",
                "getting to know them",
                "fond of them",
                "close to them",
                "devoted to them"
            ]
        );
    }

    #[test]
    fn a_damaged_life_is_brought_back_in_range() {
        let life = Life {
            bond: f32::NAN,
            closest: 7.0,
            total: Tally {
                together: -5.0,
                ..Tally::default()
            },
            ..Life::default()
        }
        .sanitized();
        assert_eq!((life.bond, life.closest), (0.5, 1.0));
        assert_eq!(life.total.together, 0.0);
    }

    #[test]
    fn lists_read_naturally() {
        let items = |count: usize| -> Vec<String> {
            ["a", "b", "c"][..count]
                .iter()
                .map(|item| item.to_string())
                .collect()
        };
        assert_eq!(listed(&items(1)), "a");
        assert_eq!(listed(&items(2)), "a and b");
        assert_eq!(listed(&items(3)), "a, b, and c");
    }

    #[test]
    fn visitor_sightings_are_remembered_with_firsts_and_diary_words_and_load_from_old_saves() {
        let mut life: Life = serde_json::from_str(r#"{"total":{"motes":2},"today":{}}"#).unwrap();
        assert_eq!((life.total.birds, life.total.moths), (0, 0));
        for (kind, first) in [
            (
                EventKind::Noticed("a bird at the window".to_owned()),
                "Saw my first bird.",
            ),
            (
                EventKind::Noticed("a moth at the lamp".to_owned()),
                "A moth came to visit my lamp.",
            ),
        ] {
            assert_eq!(life.note(&kind).as_deref(), Some(first));
            assert_eq!(life.note(&kind), None);
        }
        assert_eq!((life.total.birds, life.total.moths), (2, 2));
        assert_eq!((life.today.birds, life.today.moths), (2, 2));
        let words = life.today.words().to_lowercase();
        assert!(words.contains("watched a bird at the window"));
        assert!(words.contains("saw a moth at the lamp"));
        let restored: Life = serde_json::from_str(&serde_json::to_string(&life).unwrap()).unwrap();
        assert_eq!(restored, life);
    }

    #[test]
    fn celebrations_are_counted_and_named_in_the_diary() {
        let mut life = Life::default();
        life.begin_day("2026-10-31");
        for occasion in [
            Occasion::Halloween,
            Occasion::WinterHolidays,
            Occasion::NewYear,
            Occasion::Valentine,
            Occasion::Birthday,
        ] {
            life.note(&EventKind::Celebrated(occasion));
        }
        life.note(&EventKind::Celebrated(Occasion::NewYear));
        assert_eq!(life.total.celebrations, 6);
        assert_eq!(life.today.celebrations, 6);
        let words = life.today.words();
        assert!(words.contains("Celebrated Halloween"));
        assert!(words.contains("celebrated the winter holidays"));
        assert!(words.contains("celebrated Valentine's Day"));
        assert!(words.contains("had a birthday"));
        assert_eq!(words.matches("celebrated the new year").count(), 1);
        let restored: Life = serde_json::from_str(&serde_json::to_string(&life).unwrap()).unwrap();
        assert_eq!(life, restored);
        life.begin_day("2026-11-01");
        assert_eq!(life.today.celebrations, 0);
        assert_eq!(life.total.celebrations, 6);
        assert_eq!(life.diary[0].text, words);
        assert_eq!(serde_json::from_str::<Tally>("{}").unwrap().celebrations, 0);
    }
}
