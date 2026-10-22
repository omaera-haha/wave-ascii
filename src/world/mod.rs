pub mod acquaintance;
mod activity;
pub mod agenda;
pub mod anniversary;
pub mod away;
pub mod blocks;
pub mod books;
pub mod breaks;
pub mod breeze;
pub mod bubbles;
pub mod calendar;
pub mod cat;
pub mod cold;
pub mod cranes;
pub mod creation;
mod doodle;
pub mod dream;
pub mod feelings;
pub mod focus;
pub mod fog;
pub mod forecast;
pub mod friend;
pub mod gallery;
pub mod game;
pub mod garden;
pub mod growth;
pub mod home;
pub mod hunt;
pub mod jostle;
mod letters;
mod life;
pub mod mail;
pub mod meteor;
pub mod mirror;
pub mod music;
mod nature;
mod needs;
pub mod occasion;
mod outing;
pub mod passerby;
pub mod penpal;
mod photo;
mod pounce;
pub mod promise;
pub mod puzzle;
mod question;
pub mod quirks;
mod random;
mod recognizer;
mod room;
mod routine;
pub mod show;
mod sight;
mod sleep;
pub mod stars;
mod story;
mod things;
pub mod tidy;
pub mod upbringing;
pub mod visitor;
mod vocabulary;
pub mod wardrobe;
mod weather;
pub mod wish;
pub mod writing;

use glam::{Vec2, Vec3};

pub use activity::{Activity, Plan};
pub use blocks::Block;
pub use breeze::{Leaf, Wind};
pub use calendar::Season;
pub use doodle::{Doodle, Picture, Sketch, Writing};
pub use dream::Dream;
pub use feelings::Feeling;
pub use focus::Focus;
pub use fog::{Drawing, LearnedPicture};
pub use friend::{Companion, Friend};
pub use game::{Game, Mark, Outcome};
pub use garden::{Flower, Petal, Seedling};
pub use growth::{Skill, Skills, Stage};
pub use life::{Entry, Life};
pub use nature::Nature;
pub use needs::{Mood, Needs};
pub use occasion::{Decoration, Occasion};
pub use passerby::Passerby;
pub use puzzle::Puzzle;
pub use question::{Question, ask as asked_question};
pub use quirks::Quirk;
pub use random::Random;
pub use room::Room;
pub use sight::{Seen, Thing};
pub use story::Telling;
pub use things::{
    Ball, Lamp, Mote, Nook, Plant, ROOM_MAX, ROOM_MIN, SHELF_POSITION, SILL_POSITION,
    WINDOW_CENTER, flat,
};
pub use upbringing::Upbringing;
pub use visitor::{BirdPicture, Keepsake, Perch, Visitors};
pub use vocabulary::{Tone, Vocabulary};
pub use wardrobe::Accessory;
use weather::Change;
pub use weather::{Sky, Weather};
pub use wish::{Granted, Wish, Wishing};

/// At most this many motes drift in the room at once.
const MAX_MOTES: usize = 6;
/// At most this many pictures remain on the glass at once.
const MAX_DOODLES: usize = 3;
/// How close the ghost has to come to a place to have arrived.
const ARRIVED: f32 = 0.2;
/// Seconds a plan may run before the ghost gives up on it.
const GIVE_UP: f32 = 60.0;
/// Seconds after the viewer's last input that they still count as present.
const PRESENT_FOR: f64 = 90.0;
/// Seconds the viewer must stay away before the ghost counts them gone, so
/// a few quiet minutes or a glance at another window end no visit, and
/// coming back from them is no new arrival to greet.
pub const GONE_AFTER: f64 = 300.0;
/// Seconds a brand-new ghost takes to gather itself out of light.
const BIRTH_SECONDS: f64 = 4.0;
/// Motes that swirl close as it forms.
const BIRTH_MOTES: usize = 5;
/// Seconds the ghost waits before inviting the viewer to another game on
/// the glass after one they let pass without a move. Each invitation let
/// pass in a row doubles the wait, up to `IGNORED_PAUSE_MOST`.
const IGNORED_PAUSE: f64 = 1200.0;
const IGNORED_PAUSE_MOST: f64 = 4.0 * 3600.0;
/// Pokes closer together than this many seconds count as one streak.
const POKE_STREAK: f64 = 1.5;
/// Seconds after the terminal loses the focus before the viewer counts as
/// away.
const LOOKED_AWAY_FOR: f64 = 20.0;
/// Recent events kept for the face and the minds.
const EVENTS_KEPT: usize = 80;
/// How close the ghost must feel to the viewer to come and greet them.
pub(crate) const GREETS_FROM: f32 = 0.3;
/// How much it learns about the viewer's words from each kind of hint:
/// hearing them while it does something, being praised for doing what it
/// took them to mean, being praised for a guess, a mind telling it what
/// they asked for, the viewer showing it, and being told no.
const NAMED: f32 = 1.0;
const PRAISED: f32 = 1.0;
const CONFIRMED: f32 = 2.0;
const ROUTED: f32 = 1.5;
const SHOWN: f32 = 1.5;
const REFUSED: f32 = -1.5;
/// Seconds after it began something that the viewer's words may still be
/// about it.
const NAMED_FOR: f64 = 20.0;
/// Seconds after the ghost acted on words that praise or a no still counts
/// for them.
const HEEDED_FOR: f64 = 30.0;
/// Seconds after the viewer spoke that what they do may show what they
/// meant.
const SHOWN_FOR: f64 = 12.0;
/// How much petting one unit of a stroke across the ghost adds, and the
/// seconds it takes to fade to about a third once the strokes stop.
const STROKING: f32 = 2.0;
const PETTING_FADES: f32 = 1.2;
/// How much petting it takes to feel it: to calm down and count a stroke.
const PETTED: f32 = 0.35;

/// Something that happened, which the ghost's face reacts to and its minds
/// hear about.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub time: f64,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventKind {
    Began(Activity),
    /// The viewer poked the ghost, the `count`th time in a row.
    Poked {
        count: u32,
    },
    AteMote,
    NudgedBall,
    PickedUpBall,
    Fetched,
    Danced,
    Tended,
    Bloomed,
    SwitchedLamp {
        on: bool,
    },
    FellAsleep,
    WokeUp,
    /// The viewer said something.
    Heard(String),
    ViewerFelt(Feeling),
    Comforted(Feeling),
    FeltBetter,
    /// The ghost said something.
    Said(String),
    ViewerArrived,
    Promised(promise::Return),
    PromiseKept(promise::Return),
    PromiseLate,
    ViewerLeft,
    ExpectedArrival,
    MissedVisit,
    /// The viewer dropped a mote into the room.
    DroppedMote,
    /// The viewer flicked the ball.
    KickedBall,
    /// The viewer came back after the app was closed for `away` seconds.
    Returned {
        away: f64,
    },
    /// The ghost caught sight of something new to it.
    Noticed(String),
    /// A first or a landmark in the ghost's life, in its own words.
    Milestone(String),
    RainStarted,
    StormStarted,
    Thunder,
    RainStopped,
    SnowStarted,
    SnowStopped,
    RainbowAppeared,
    RainbowFaded,
    Found,
    Jumped,
    Doodled(&'static str),
    GameOver(Outcome),
    Wrote {
        text: String,
        mirrored: bool,
        watched: bool,
    },
    LearnedName(String),
    Celebrated(Occasion),
    Remembered(String),
    Pounced,
    Built {
        height: u32,
    },
    Toppled {
        by_viewer: bool,
    },
    Morning,
    /// The viewer sounded pleased with the ghost.
    Praised,
    /// The viewer told the ghost no.
    Scolded,
    /// The viewer stroked the ghost.
    Petted,
    /// The viewer said a word the ghost doesn't know yet.
    Puzzled(String),
    /// The ghost worked out what one of the viewer's words means.
    LearnedWord {
        word: String,
        activity: Activity,
    },
    BirdLanded,
    CatArrived,
    CatLeft {
        startled: bool,
    },
    BirdSang,
    BirdFlewOff {
        startled: bool,
    },
    MothArrived,
    MothLeft,
    GiftLeft(Keepsake),
    Kept(Keepsake),
    /// The viewer gave the ghost a name.
    Named(String),
    /// It saw a shooting star through the window and made a wish.
    ShootingStar,
    /// Rain or snow came, as it had felt coming.
    ForecastCameTrue {
        snow: bool,
    },
    FriendArrived(String),
    FriendLeft(String),
    Tagged {
        by_friend: bool,
    },
    PokedFriend,
    PlayedWithFriend(friend::Game),
    MoonWish,
    SawDrawing {
        name: Option<String>,
    },
    LearnedPicture(String),
    Copied(String),
    Planted(Flower),
    Sprouted(Flower),
    Flowered(Flower),
    Drooped,
    FocusEnded {
        minutes: u32,
        completed: bool,
    },
    ToldStory,
    FoundConstellation(String),
    OpenedWindow,
    ClosedWindow,
    Puddle,
    SawPasserby(Passerby),
    WordGame {
        word: String,
        solved: bool,
    },
    Quirk(Quirk),
    PutOn(Accessory),
    TookOff(Accessory),
    NewBook(String),
    ReadPage,
    FinishedBook(String),
    PlayedMusic {
        tune: String,
    },
    LearnedTune(String),
    Composed(String),
    SangMySong(String),
    WroteBook(String),
    LullabySleep,
    MadeWish(Wish),
    WishFaded(Wish),
    WishGranted(Wish),
    MetReflection,
    RecognizedSelf,
    VisitedMirror,
    WroteLetter,
    ReadLetter,
    GotLetter,
    ReadViewerLetter,
    AskedViewer(acquaintance::Topic),
    LearnedAboutViewer(acquaintance::Topic),
    /// The glass box shook; `rattled` once it kept shaking a while.
    Jostled {
        rattled: bool,
    },
    HeardPlan {
        errand: agenda::Errand,
        days: i32,
    },
    WishedLuck(agenda::Errand),
    AskedAboutPlan(agenda::Errand),
    PlanWentWell(agenda::Errand),
    PlanWentBadly(agenda::Errand),
    FoundTreasure {
        seconds: u32,
        hints: u32,
        gave_up: bool,
    },
    TreasureHint(hunt::Hint),
    AskedTreasureHint,
    SpottedTreasure,
    SuggestedBreak {
        long: bool,
    },
    NoticedLateNight,
    SaidGoodnightTogether,
    TookBreak,
    FoldedCrane {
        count: u32,
    },
    /// The viewer took its picture, `posed` when it smiled for it.
    Photographed {
        posed: bool,
    },
    BlewBubbles {
        blown: u32,
        popped_by_viewer: u32,
    },
    Pinned {
        kind: gallery::Kind,
        name: String,
    },
    ShowedThing(show::Kind),
    Tidied {
        blocks: u32,
        ball: bool,
    },
    CaughtCold,
    CaredForMe(cold::Care),
    Recovered {
        cared: bool,
    },
}

impl EventKind {
    /// The event in a few words, from the ghost's side.
    pub fn words(&self) -> String {
        match self {
            Self::ViewerFelt(feeling) => format!("you told me you felt {}", feeling.name()),
            Self::Comforted(feeling) => {
                format!("you were {}, so I {}", feeling.name(), feeling.care_words())
            }
            Self::FeltBetter => "you told me you were feeling better".to_owned(),
            Self::MetReflection => "met the ghost in the mirror".to_owned(),
            Self::RecognizedSelf => "learned the ghost in the mirror is me".to_owned(),
            Self::VisitedMirror => "looked in the mirror".to_owned(),
            Self::GotLetter => "The viewer wrote me a letter.".to_owned(),
            Self::ReadViewerLetter => "I read your letter.".to_owned(),
            Self::WroteLetter => "wrote you a letter".to_owned(),
            Self::ReadLetter => "you read my letter".to_owned(),
            Self::AskedViewer(topic) => format!("asked about your {}", topic.name()),
            Self::LearnedAboutViewer(topic) => format!("learned about your {}", topic.name()),
            Self::Quirk(kind) => kind.words().to_owned(),
            Self::PutOn(accessory) => format!("put on {}", accessory.words()),
            Self::TookOff(accessory) => format!("took off {}", accessory.words()),
            Self::NewBook(title) => format!("a new picture book arrived: {title}"),
            Self::Composed(title) => format!("made up a tune called “{title}”"),
            Self::SangMySong(title) => format!("heard the viewer sing my song, “{title}”"),
            Self::WroteBook(title) => format!("wrote my own picture book, “{title}”"),
            Self::ReadPage => "read a page of a picture book".to_owned(),
            Self::FinishedBook(title) => format!("finished reading {title}"),
            Self::PlayedMusic { tune } => format!("played {tune} on the music box"),
            Self::LearnedTune(tune) => format!("learned the viewer's tune: {tune}"),
            Self::LullabySleep => "fell asleep to the music box's lullaby".to_owned(),
            Self::MadeWish(wish) => format!("made a wish: {}", wish.words(&Life::default())),
            Self::WishFaded(wish) => format!("let go of a wish: {}", wish.words(&Life::default())),
            Self::WishGranted(wish) => {
                format!("a wish came true: {}", wish.words(&Life::default()))
            }
            Self::OpenedWindow => "opened the window for fresh air".to_owned(),
            Self::ClosedWindow => "closed the window".to_owned(),
            Self::Puddle => "noticed a puddle below the window".to_owned(),
            Self::SawPasserby(kind) => format!("noticed {} through the window", kind.name()),
            Self::ToldStory => "told the viewer a story".to_owned(),
            Self::FoundConstellation(name) => format!("found a constellation: {name}"),
            Self::Began(activity) => format!("started {}", activity.doing()),
            Self::Poked { count: 1 } => "the viewer poked you".to_owned(),
            Self::Poked { count } => format!("the viewer poked you {count} times in a row"),
            Self::AteMote => "caught and ate a light mote".to_owned(),
            Self::NudgedBall => "nudged the ball".to_owned(),
            Self::PickedUpBall => "picked up the ball".to_owned(),
            Self::Fetched => "brought the ball back to the viewer".to_owned(),
            Self::Built { height } => format!(
                "stacked the blocks into a tower {} blocks high",
                blocks::height_words(*height as usize)
            ),
            Self::Toppled { by_viewer: true } => "the viewer knocked your tower down".to_owned(),
            Self::Toppled { by_viewer: false } => {
                "knocked your tower down with a whoosh".to_owned()
            }
            Self::Pounced => "pounced on the viewer's finger on the glass".to_owned(),
            Self::MoonWish => "made a wish on the Chuseok moon".to_owned(),
            Self::Danced => "danced a little twirl".to_owned(),
            Self::Planted(kind) => format!("planted a {} seed in a little pot", kind.name()),
            Self::Sprouted(kind) => format!("the {} seed in the little pot sprouted", kind.name()),
            Self::Flowered(kind) => format!("the {} you planted came into flower", kind.name()),
            Self::Drooped => "the plant is drooping from thirst".to_owned(),
            Self::Tended => "watered the plant".to_owned(),
            Self::Bloomed => "the plant came into flower".to_owned(),
            Self::SwitchedLamp { on: true } => "switched the lamp on".to_owned(),
            Self::SwitchedLamp { on: false } => "switched the lamp off".to_owned(),
            Self::FellAsleep => "fell asleep".to_owned(),
            Self::WokeUp => "woke up".to_owned(),
            Self::Morning => "woke up to a new morning".to_owned(),
            Self::Heard(text) => match focus::requested(text) {
                Some(minutes) => format!(
                    "the viewer asked to focus together for {} minutes",
                    friend::count_words(minutes)
                ),
                None => acquaintance::birthday(text).map_or_else(
                    || format!("the viewer said: {text}"),
                    |birthday| format!("the viewer said their birthday is {}", birthday.words()),
                ),
            },
            Self::Said(text) => format!("you said: {text}"),
            Self::Promised(kind) => format!("the viewer promised to return {}", kind.words()),
            Self::PromiseKept(kind) if kind.trip() => {
                "the viewer came home from their trip, as promised".to_owned()
            }
            Self::PromiseKept(_) => "the viewer came back when they said they would".to_owned(),
            Self::PromiseLate => {
                "the viewer came back; you missed them and are glad they are here".to_owned()
            }
            Self::ViewerArrived => "the viewer came to the glass".to_owned(),
            Self::ViewerLeft => "the viewer went away".to_owned(),
            Self::ExpectedArrival => {
                "the viewer came at their usual time, just as you hoped".to_owned()
            }
            Self::MissedVisit => "the viewer didn't come at their usual time".to_owned(),
            Self::DroppedMote => "the viewer dropped a light mote into the room".to_owned(),
            Self::KickedBall => "the viewer flicked the ball".to_owned(),
            Self::Returned { away } => {
                let hours = away / 3600.0;
                let while_away = match hours {
                    hours if hours < 2.0 => "an hour".to_owned(),
                    hours if hours < 24.0 => format!("{} hours", hours.round()),
                    hours if hours < 48.0 => "a day".to_owned(),
                    hours => format!("{} days", (hours / 24.0).round()),
                };
                format!("the viewer came back after {while_away} away")
            }
            Self::Noticed(thing) => format!("spotted {thing}"),
            Self::Milestone(text) => format!("a moment to remember: {text}"),
            Self::RainStarted => "it started to rain outside".to_owned(),
            Self::StormStarted => "a thunderstorm rolled in".to_owned(),
            Self::Thunder => "thunder rumbled outside".to_owned(),
            Self::RainStopped => "the rain stopped".to_owned(),
            Self::SnowStarted => "it started to snow outside".to_owned(),
            Self::SnowStopped => "the snow stopped".to_owned(),
            Self::RainbowAppeared => "the rain stopped and a rainbow came out".to_owned(),
            Self::RainbowFaded => "the rainbow faded".to_owned(),
            Self::Found => "the viewer found you hiding".to_owned(),
            Self::Jumped => "jumped out from hiding at the viewer".to_owned(),
            Self::GameOver(outcome) => match outcome {
                Outcome::GhostWon { .. } => "won a game of noughts and crosses against the viewer",
                Outcome::ViewerWon { .. } => "the viewer beat you at noughts and crosses",
                Outcome::Draw => "a game of noughts and crosses ended in a draw",
                Outcome::Abandoned => {
                    "the viewer wandered off in the middle of your game of noughts and crosses"
                }
            }
            .to_owned(),
            Self::SawDrawing { name: Some(name) } => {
                format!("looked at a {name} the viewer drew on the glass")
            }
            Self::SawDrawing { name: None } => {
                "looked at a drawing of the viewer's it couldn't make out".to_owned()
            }
            Self::LearnedPicture(name) => format!("learned that the viewer's drawing is a {name}"),
            Self::Copied(name) => format!("drew a {name} on the glass"),
            Self::Doodled(name) => format!("drew a {name} on the glass"),
            Self::Wrote { text, mirrored, .. } => format!(
                "wrote “{}” on the glass{}",
                doodle::writing_words(text),
                if *mirrored {
                    ", the wrong way round for the viewer"
                } else {
                    ""
                }
            ),
            Self::LearnedName(name) => format!("learned that the viewer's name is {name}"),
            Self::Celebrated(Occasion::NewYear) => "a new year began".to_owned(),
            Self::Celebrated(occasion) => format!("it is {}", occasion.words()),
            Self::Remembered(line) => format!("remembered: {line}"),
            Self::Praised => "the viewer sounded pleased with you".to_owned(),
            Self::Scolded => "the viewer told you no".to_owned(),
            Self::Petted => "the viewer stroked you gently".to_owned(),
            Self::Puzzled(word) => format!("didn't know the viewer's word “{word}”"),
            Self::LearnedWord { word, activity } => {
                format!("worked out that “{word}” means {}", activity.doing())
            }
            Self::BirdLanded => "a little bird landed outside the window".to_owned(),
            Self::BirdSang => "the bird outside sang".to_owned(),
            Self::BirdFlewOff { startled: false } => "the bird flew away".to_owned(),
            Self::BirdFlewOff { startled: true } => "the bird took fright and flew away".to_owned(),
            Self::MothArrived => "a moth fluttered in to the lamp".to_owned(),
            Self::CatArrived => "a cat came over the fence into the yard".to_owned(),
            Self::CatLeft { startled: false } => "the cat wandered off home".to_owned(),
            Self::CatLeft { startled: true } => "the cat took fright and dashed off".to_owned(),
            Self::MothLeft => "the moth fluttered away".to_owned(),
            Self::GiftLeft(gift) => format!("the bird left {} on the sill outside", gift.name()),
            Self::Kept(gift) => format!("put {} on the shelf of keepsakes", gift.name()),
            Self::Named(name) => format!("the viewer named you {name}"),
            Self::ShootingStar => {
                "saw a shooting star through the window and made a wish".to_owned()
            }
            Self::ForecastCameTrue { snow: false } => {
                "felt the rain coming, and it came".to_owned()
            }
            Self::ForecastCameTrue { snow: true } => {
                "smelled snow in the air, and it came".to_owned()
            }
            Self::FriendArrived(name) => {
                format!("your friend {name}, another ghost, floated in through the window")
            }
            Self::FriendLeft(name) => format!("{name} floated away through the window"),
            Self::Tagged { by_friend: true } => "your friend tagged you; you are it now".to_owned(),
            Self::Tagged { by_friend: false } => {
                "you tagged your friend; they are it now".to_owned()
            }
            Self::PokedFriend => "the viewer poked your friend".to_owned(),
            Self::FocusEnded { minutes, completed } => format!(
                "{} after {} {} of focusing together",
                if *completed {
                    "finished a focus session"
                } else {
                    "ended a focus session early"
                },
                friend::count_words(*minutes),
                if *minutes == 1 { "minute" } else { "minutes" },
            ),
            Self::WordGame { word, solved } => format!(
                "{} a word game on the glass; the word was {word}",
                if *solved {
                    "the viewer solved"
                } else {
                    "ended without solving"
                },
            ),
            Self::PlayedWithFriend(game) => {
                format!("finished {} with your ghost friend", game.words())
            }
            Self::Jostled { rattled: false } => {
                "the whole room jolted and the light motes swirled".to_owned()
            }
            Self::Jostled { rattled: true } => "the room kept shaking like a snow globe".to_owned(),
            Self::HeardPlan { errand, days } => format!(
                "the viewer told me about their {} {}",
                errand.name(),
                agenda::when(*days)
            ),
            Self::WishedLuck(errand) => {
                format!("wished the viewer luck for their {}", errand.name())
            }
            Self::AskedAboutPlan(errand) => {
                format!("asked how the viewer's {} went", errand.name())
            }
            Self::PlanWentWell(errand) => format!("the viewer's {} went well", errand.name()),
            Self::PlanWentBadly(errand) => {
                format!("the viewer's {} was hard, so I stayed close", errand.name())
            }
            Self::FoundTreasure {
                seconds,
                hints,
                gave_up,
            } => format!(
                "{} after {} seconds and {} hints",
                if *gave_up {
                    "needed help to find the viewer's treasure"
                } else {
                    "found the treasure the viewer hid"
                },
                friend::count_words(*seconds),
                friend::count_words(*hints)
            ),
            Self::TreasureHint(hint) => format!(
                "the viewer said you are {} in the treasure hunt",
                hint.words()
            ),
            Self::AskedTreasureHint => "asked the viewer for a treasure hint".to_owned(),
            Self::SpottedTreasure => "spotted the treasure the viewer hid".to_owned(),
            Self::SuggestedBreak { long: true } => {
                "suggested a longer break after working together".to_owned()
            }
            Self::SuggestedBreak { long: false } => {
                "suggested stretching after a long sitting together".to_owned()
            }
            Self::NoticedLateNight => "you stayed up very late".to_owned(),
            Self::SaidGoodnightTogether => "we said goodnight together late at night".to_owned(),
            Self::TookBreak => "you took a break when I asked".to_owned(),
            Self::FoldedCrane { count } => {
                format!("folded the {} paper crane", anniversary::ordinal(*count))
            }
            Self::Photographed { posed: true } => "posed for the viewer's camera".to_owned(),
            Self::Photographed { posed: false } => "the viewer took my picture".to_owned(),
            Self::BlewBubbles {
                blown,
                popped_by_viewer,
            } => format!(
                "blew {} bubbles and you popped {} of them",
                friend::count_words(*blown),
                friend::count_words(*popped_by_viewer)
            ),
            Self::Pinned { kind, name } => kind.pinned(name),
            Self::ShowedThing(kind) => format!("showed the viewer my {}", kind.name()),
            Self::Tidied { blocks, ball } => match (*blocks > 0, *ball) {
                (true, true) => "tidied up the blocks and the ball before bed",
                (true, false) => "tidied up the blocks before bed",
                (false, true) => "put the ball beside the cushion before bed",
                (false, false) => "looked around the tidy room",
            }
            .to_owned(),
            Self::CaughtCold => "caught a little cold".to_owned(),
            Self::CaredForMe(kind) => {
                format!("the viewer {} while you had a little cold", kind.words())
            }
            Self::Recovered { cared: true } => {
                "got over my cold after the viewer looked after me".to_owned()
            }
            Self::Recovered { cared: false } => "got over my little cold".to_owned(),
        }
    }
}

/// What the ghost made of something the viewer said.
#[derive(Clone, Debug, PartialEq)]
pub struct Heard {
    pub tone: Option<Tone>,
    pub understanding: Understanding,
    /// Whether the message called the ghost by its name.
    pub named: bool,
    /// Words whose meaning it worked out just now, and what they mean.
    pub learned: Vec<(String, Activity)>,
    /// Whether being told no made it stop what it was doing.
    pub stopped: bool,
    pub learned_name: Option<String>,
    /// A new name the viewer just gave the ghost.
    pub given_name: Option<String>,
    pub korean: bool,
}

/// What the viewer's words asked for, as far as the ghost can tell.
#[derive(Clone, Debug, PartialEq)]
pub enum Understanding {
    /// Nothing to act on: only tone, or little words.
    Nothing,
    /// Words it doesn't know yet, `word` above all; `again` once it has
    /// heard one of them before.
    Puzzled {
        word: String,
        again: bool,
    },
    /// It took `word` to ask for `activity` and set out to do it.
    Doing {
        word: String,
        activity: Activity,
    },
    /// It took `word` to ask for `activity`, which it can't do now.
    Unable {
        word: String,
        activity: Activity,
    },
    /// A question it can answer about itself; `doing` is what it was doing
    /// when asked, or `None` if it was asleep.
    Asked {
        question: Question,
        doing: Option<Activity>,
    },
    Named {
        picture: String,
    },
    Focusing {
        minutes: u32,
    },
    Airing {
        open: bool,
        accepted: bool,
    },
    Felt(Feeling),
    Promised(promise::Return),
    FeltBetter,
    WordGame,
    Yawning,
    Undressing,
    OwnSong,
    Music {
        lullaby: bool,
    },
    Answered(acquaintance::Topic),
    Unanswered(acquaintance::Topic),
    Planned(agenda::Errand),
    PlanAnswered {
        errand: agenda::Errand,
        went_well: Option<bool>,
    },
    Hint {
        warmer: bool,
    },
    BreakReply {
        agreed: bool,
    },
    Folding,
    Bubbles {
        accepted: bool,
    },
    Showing,
    Tidying,
    /// Asked to go out to the yard, or to come back in.
    Outing,
    Cared,
}

/// The viewer's latest words that might mean something, and what the
/// ghost did about them, so it can learn from what follows.
#[derive(Clone, Debug, Default, PartialEq)]
struct Listening {
    words: Vec<String>,
    at: f64,
    /// What it did because of the words, when, and whether it was a guess.
    acted: Option<(Activity, f64, bool)>,
    /// The words of the very latest message, which may have none, for a
    /// mind working out what that message asked.
    latest: Vec<String>,
    /// Whether the viewer already showed it what they meant.
    shown: bool,
}

/// Something said or thought, and when.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub text: String,
    pub time: f64,
}

/// The ghost: a point with a body that floats, needs that rise and fall, and
/// a plan it is carrying out.
#[derive(Clone, Debug)]
pub struct Ghost {
    pub name: String,
    pub position: Vec3,
    pub velocity: Vec3,
    pub needs: Needs,
    pub plan: Plan,
    /// From awake at 0 to sound asleep at 1.
    pub asleep: f32,
    /// From fully visible at 0 to gone at 1.
    pub fade: f32,
    /// What it is looking at, in the room.
    pub look: Option<Vec3>,
    pub mood: Mood,
    /// A mood a mind suggested, and until when it holds.
    pub suggested_mood: Option<(Mood, f64)>,
    /// What it means to do for a while, in its own words.
    pub goal: Option<String>,
    pub speech: Option<Line>,
    pub thought: Option<Line>,
    /// Which way it faces: what it sees lies around this direction.
    pub heading: Vec3,
    /// What it turned to look at, and until when.
    pub attention: Option<(Thing, f64)>,
    /// The motes it has seen, which it knows are there.
    pub known: Vec<u32>,
    /// Whether it has seen the current rainbow.
    pub saw_rainbow: bool,
    /// Whether it has seen the current snowfall.
    pub saw_snow: bool,
    /// Whether it has noticed the gift currently on the outer sill.
    pub saw_gift: bool,
    /// A gift riding just below it on the way to the shelf.
    pub keepsake: Option<Keepsake>,
}

/// The person at the glass, as the world knows them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewer {
    /// Where the pointer touches the glass, while it is over the room.
    pub pointer: Option<Vec3>,
    /// Recent travel on the glass, fading as the pointer holds still.
    pub liveliness: f32,
    /// When the pointer last moved on the glass.
    pub moved_at: f64,
    /// When the viewer last did anything.
    pub active_at: f64,
    /// While the viewer is typing a message.
    pub typing: bool,
    /// When the terminal lost the focus, while it is elsewhere.
    pub looked_away_at: Option<f64>,
}

/// The glass box and everything in it. `step` moves it forward in time;
/// minds only choose what the ghost does next, through `begin`.
#[derive(Clone, Debug)]
pub struct World {
    /// Seconds since the world started.
    pub time: f64,
    /// The local hour of the day, from 0 to 24, which sets the light.
    pub hour: f32,
    /// The local date, fixed for seeded pictures and set by the clock in live mode.
    pub date: jiff::civil::Date,
    /// The local date it was born, absent in seeded pictures unless requested.
    pub birthday: Option<jiff::civil::Date>,
    pub ghost: Ghost,
    pub lamp: Lamp,
    pub ball: Ball,
    pub blocks: Vec<Block>,
    /// The base of a standing tower, absent when the blocks are scattered.
    pub tower: Option<Vec3>,
    building: Option<blocks::Building>,
    pub plant: Plant,
    pub nook: Nook,
    pub motes: Vec<Mote>,
    pub doodles: Vec<Doodle>,
    pub game: Option<Game>,
    pub events: Vec<Event>,
    pub viewer: Viewer,
    pub weather: Weather,
    /// The shower the ghost felt coming, not saved.
    pub forecast: Option<forecast::Forecast>,
    pub visitors: Visitors,
    /// Its life over days: its bond with the viewer, habits, and diary.
    pub life: Life,
    last_celebrated: Option<(jiff::civil::Date, Occasion)>,
    calendar_at: Option<jiff::civil::Date>,
    new_year_at: Option<f64>,
    random: Random,
    next_mote: u32,
    next_mote_at: f64,
    viewer_present: bool,
    /// Whether the viewer has stayed away long enough to have left, so
    /// their next appearance is an arrival.
    viewer_gone: bool,
    /// The language of a farewell promise made during this session.
    promise_said: Option<bool>,
    /// Whether the viewer has left since that promise (true for a restored life).
    promise_departed: bool,
    /// When the viewer stopped counting as present, while they are away.
    absent_since: Option<f64>,
    /// When the storm that last woke it began, so it wakes once a storm.
    storm_woke: Option<f64>,
    /// When the viewer last let a game on the glass pass without a move,
    /// and how many invitations in a row they have let pass.
    invitation_ignored_at: Option<f64>,
    invitations_ignored: u32,
    /// When a darting pointer last interrupted an idle plan.
    pointer_interrupted_at: Option<f64>,
    listening: Listening,
    /// Which way the ghost means to push the ball next.
    push: Vec3,
    /// Until when an interrupted night's sleep leaves it drowsy.
    drowsy_until: f64,
    /// When it last woke naturally from an overnight rest.
    morning_at: Option<f64>,
    /// From untouched at 0 to being stroked steadily at 1.
    petting: f32,
    /// Whether this spell of petting has been counted.
    petted: bool,
    routine: routine::Routine,
    /// Hashed with the time to place shooting stars without drawing from
    /// `random`.
    meteor_seed: u64,
    /// The slot of the last shooting star it saw.
    seen_meteor: Option<u64>,
    pub friend: Option<Friend>,
    first_friend_at: Option<f64>,
    last_holiday_greeting: Option<(jiff::civil::Date, Occasion)>,
    sebae: Option<occasion::Bow>,
    sebae_gift_until: f64,
    wished_night: Option<jiff::civil::Date>,
    pub drawing: Option<Drawing>,
    latest_korean: bool,
    pub seedlings: Vec<Seedling>,
    pub petals: Vec<Petal>,
    tending: Option<garden::Tending>,
    pub dream: Option<Dream>,
    pub focus: Option<Focus>,
    pub comforting: Option<feelings::Comforting>,
    feeling_at: Option<f64>,
    feeling_follow_up: Option<(Feeling, f64)>,
    feeling_visit_asked: bool,
    break_until: f64,
    pub telling: Option<Telling>,
    /// The story of its time alone, kept only for this session.
    pub away: Option<away::Away>,
    found_constellation_at: Option<f64>,
    pub window_open: bool,
    pub wind: Wind,
    pub puddle: f32,
    pub leaves: Vec<Leaf>,
    saw_puddle: bool,
    window_refused_at: Option<f64>,
    /// When the ghost last opened or shut the sash.
    window_moved_at: Option<f64>,
    next_windfall_at: f64,
    windfall_random: Random,
    seen_passersby: [Option<u64>; Passerby::ALL.len()],
    pub puzzle: Option<Puzzle>,
    /// An involuntary spell, kept only for this session.
    pub quirk: Option<quirks::Spell>,
    quirk_triggers: quirks::Triggers,
    pub wearing: Option<Accessory>,
    wardrobe: wardrobe::Wardrobe,
    pub reading: Option<books::Reading>,
    last_read: Option<usize>,
    pub music: Option<music::Playing>,
    pub music_notes: Vec<music::RisingNote>,
    queued_tune: Option<music::Tune>,
    pending_tune: Option<music::Tune>,
    compose_on_arrival: bool,
    pending_wish: Option<Wish>,
    mirror_visit: Option<mirror::Visit>,
    viewer_letter: Option<penpal::Reading>,
    pub asking: Option<(acquaintance::Topic, f64)>,
    last_message_at: Option<f64>,
    last_question_at: Option<f64>,
    viewer_blush_at: Option<f64>,
    pub jostle: jostle::Jostle,
    /// When a brand-new life began, while it is still taking shape.
    born_at: Option<f64>,
    agenda_answer: Option<(agenda::Errand, String, f64)>,
    agenda_visit_asked: bool,
    agenda_visit_question: bool,
    pub hunt: Option<hunt::Hunt>,
    pub rhythm: breaks::Rhythm,
    pub folding: Option<cranes::Folding>,
    pending_crane: bool,
    pub bubbles: Vec<bubbles::Bubble>,
    bubble_session: Option<bubbles::Session>,
    bubbles_finished_at: Option<f64>,
    /// When it last went out to the yard on its own.
    last_outing: Option<f64>,
    pending_pins: Vec<gallery::Pin>,
    pub showing: Option<show::Showing>,
    show_praise_until: Option<f64>,
    tidying: Option<tidy::Tidying>,
    pending_tidy: bool,
    tidy_rest: bool,
    chill: cold::Chill,
    /// The previous offline tale, remembered only for this session.
    pub(crate) last_story_thread: Option<&'static str>,
    pub(crate) last_story_ending: Option<&'static str>,
}

impl World {
    pub fn new(seed: u64, hour: f32) -> Self {
        let start = Vec3::new(0.2, 1.1, 0.0);
        let mut random = Random::new(seed);
        let weather = Weather::new(&mut random);
        let nature = Nature::roll(&mut random);
        let visitors = Visitors::new(&mut random, seed);
        let mut world = Self {
            time: 0.0,
            hour,
            date: jiff::civil::Date::constant(2026, 6, 21),
            birthday: None,
            ghost: Ghost {
                name: "Boo".to_owned(),
                position: start,
                velocity: Vec3::ZERO,
                needs: Needs::default(),
                plan: Plan::new(Activity::Float, 0.0, start, 6.0),
                asleep: 0.0,
                fade: 0.0,
                look: None,
                mood: Mood::Content,
                suggested_mood: None,
                goal: None,
                speech: None,
                thought: None,
                heading: Vec3::Z,
                attention: None,
                known: Vec::new(),
                saw_rainbow: false,
                saw_snow: false,
                saw_gift: false,
                keepsake: None,
            },
            lamp: Lamp {
                position: Vec3::new(-1.55, 0.0, -0.95),
                on: true,
            },
            ball: Ball::at(0.7, 0.45),
            blocks: Vec::new(),
            tower: None,
            building: None,
            plant: Plant {
                position: Vec3::new(1.5, 0.0, -0.9),
                growth: 0.15,
                thirst: 0.4,
                ..Plant::default()
            },
            nook: Nook {
                position: Vec3::new(-1.3, 0.0, 0.55),
            },
            motes: Vec::new(),
            doodles: Vec::new(),
            game: None,
            events: Vec::new(),
            viewer: Viewer::default(),
            weather,
            forecast: None,
            visitors,
            life: Life {
                nature: Some(nature),
                ..Life::default()
            },
            last_celebrated: None,
            calendar_at: None,
            new_year_at: None,
            random,
            next_mote: 0,
            next_mote_at: 8.0,
            viewer_present: false,
            viewer_gone: true,
            promise_said: None,
            promise_departed: true,
            absent_since: None,
            storm_woke: None,
            invitation_ignored_at: None,
            invitations_ignored: 0,
            pointer_interrupted_at: None,
            listening: Listening::default(),
            push: Vec3::X,
            drowsy_until: 0.0,
            morning_at: None,
            petting: 0.0,
            petted: false,
            routine: routine::Routine::default(),
            meteor_seed: seed,
            seen_meteor: None,
            friend: None,
            first_friend_at: None,
            last_holiday_greeting: None,
            sebae: None,
            sebae_gift_until: 0.0,
            wished_night: None,
            drawing: None,
            latest_korean: false,
            seedlings: Vec::new(),
            petals: Vec::new(),
            tending: None,
            dream: None,
            focus: None,
            comforting: None,
            feeling_at: None,
            feeling_follow_up: None,
            feeling_visit_asked: false,
            break_until: 0.0,
            telling: None,
            away: None,
            found_constellation_at: None,
            window_open: false,
            wind: Wind::new(seed),
            puddle: 0.0,
            leaves: Vec::new(),
            saw_puddle: false,
            window_refused_at: None,
            window_moved_at: None,
            next_windfall_at: 8.0,
            windfall_random: Random::new(seed ^ 0x6c65_6176_6573),
            seen_passersby: [None; Passerby::ALL.len()],
            puzzle: None,
            quirk: None,
            quirk_triggers: quirks::Triggers::default(),
            wearing: None,
            wardrobe: wardrobe::Wardrobe::default(),
            reading: None,
            last_read: None,
            music: None,
            music_notes: Vec::new(),
            queued_tune: None,
            pending_tune: None,
            compose_on_arrival: false,
            pending_wish: None,
            mirror_visit: None,
            viewer_letter: None,
            asking: None,
            last_message_at: None,
            last_question_at: None,
            viewer_blush_at: None,
            jostle: jostle::Jostle::default(),
            born_at: None,
            agenda_answer: None,
            agenda_visit_asked: false,
            agenda_visit_question: false,
            hunt: None,
            rhythm: breaks::Rhythm::default(),
            folding: None,
            pending_crane: false,
            bubbles: Vec::new(),
            bubble_session: None,
            bubbles_finished_at: None,
            last_outing: None,
            pending_pins: Vec::new(),
            showing: None,
            show_praise_until: None,
            tidying: None,
            pending_tidy: false,
            tidy_rest: false,
            chill: cold::Chill::default(),
            last_story_thread: None,
            last_story_ending: None,
        };
        world.scatter_blocks();
        world
    }

    /// Moves the world forward by `seconds`, at most a tenth of a second at
    /// a time.
    pub fn step(&mut self, seconds: f32) {
        self.advance_routine();
        self.begin_day();
        let calendar_check = self.calendar_at != Some(self.date);
        if calendar_check {
            self.mark_growth();
            self.deliver_books();
        }
        self.celebrate();
        self.write_weekly_letter();
        self.check_wardrobe();
        self.dress_when_idle();
        if calendar_check {
            self.check_wish();
        }
        let mut remaining = seconds.max(0.0);
        while remaining > 0.0 {
            let slice = remaining.min(0.1);
            remaining -= slice;
            self.advance(slice);
        }
        self.celebrate_wish();
        self.greet_favorite_season();
        self.follow_up_feeling();
        self.ask_viewer();
        self.follow_up_agenda();
        self.advance_rhythm();
        self.offer_show(seconds.max(0.0));
        self.resume_tidy_request();
        self.mention_cold_recovery();
    }

    fn advance(&mut self, seconds: f32) {
        self.time += f64::from(seconds);
        if self.hunt.is_some()
            && (self.ghost.asleep >= 0.3
                || self.focus.is_some()
                || self.telling.is_some()
                || self.ghost.plan.done)
        {
            self.cancel_hunt();
        }
        self.advance_focus();
        self.viewer.liveliness *= (-seconds / 0.7).exp();
        self.petting *= (-seconds / PETTING_FADES).exp();
        if self.petting < 0.05 {
            self.petted = false;
        }
        self.doodles
            .retain(|doodle| doodle.opacity(self.time) > 0.0);

        if self
            .game
            .as_ref()
            .is_some_and(|game| game.opacity(self.time) <= 0.0)
        {
            self.game = None;
        }

        if self
            .puzzle
            .as_ref()
            .is_some_and(|puzzle| puzzle.opacity(self.time) <= 0.0)
        {
            self.puzzle = None;
        }

        self.jostle.settle(seconds);
        for mote in &mut self.motes {
            mote.drift(
                self.time,
                seconds,
                breeze::breeze(self.window_open, &self.wind, self.time, mote.position)
                    + self.jostle.stir(mote.position, mote.seed),
            );
        }
        let time = self.time;
        self.motes.retain(|mote| time - mote.born < Mote::LIFETIME);
        if self.time >= self.next_mote_at {
            if self.motes.len() < MAX_MOTES {
                let position = Vec3::new(
                    self.random.range(ROOM_MIN.x + 0.4, ROOM_MAX.x - 0.4),
                    ROOM_MAX.y - 0.35,
                    self.random.range(ROOM_MIN.z + 0.4, ROOM_MAX.z - 0.4),
                );
                self.add_mote(position);
            }
            self.next_mote_at = self.time + f64::from(self.random.range(25.0, 60.0));
        }

        let daytime = (7.0..18.5).contains(&self.hour);
        let season = self.season();
        if let Some(change) = self
            .weather
            .step(self.time, daytime, season, &mut self.random)
        {
            if change == Change::RainbowFaded {
                self.ghost.saw_rainbow = false;
            }
            if change == Change::SnowStopped {
                self.ghost.saw_snow = false;
            }
            let falling = match change {
                Change::RainStarted | Change::StormStarted => Some(false),
                Change::SnowStarted => Some(true),
                _ => None,
            };
            self.record(match change {
                Change::RainStarted => EventKind::RainStarted,
                Change::StormStarted => EventKind::StormStarted,
                Change::Thunder => EventKind::Thunder,
                Change::RainStopped => EventKind::RainStopped,
                Change::SnowStarted => EventKind::SnowStarted,
                Change::SnowStopped => EventKind::SnowStopped,
                Change::RainbowAppeared => EventKind::RainbowAppeared,
                Change::RainbowFaded => EventKind::RainbowFaded,
            });
            if let Some(snow) = falling {
                self.weather_came(snow);
            }
        }
        self.sense_weather();
        self.weather.advance(self.time, seconds, daytime);
        if self.weather.advance_storm(self.time, &mut self.random) == Some(Change::Thunder) {
            self.record(EventKind::Thunder);
            self.hear_thunder();
        }
        self.advance_breeze(seconds);
        self.advance_visitors(seconds);
        self.advance_cat(seconds);
        self.advance_friend();

        self.advance_blocks(seconds);
        self.advance_garden(seconds);
        self.advance_wardrobe();
        self.advance_music();
        self.advance_cranes();

        let present = self.viewer_present();
        if present != self.viewer_present {
            self.viewer_present = present;
            if present {
                self.absent_since = None;
                if self.viewer_gone {
                    self.viewer_gone = false;
                    self.viewer_arrived();
                }
            } else {
                self.absent_since = Some(self.time);
                // A word game needs someone to guess, and a question someone
                // to answer.
                self.abandon_puzzle();
                self.asking = None;
                self.abandon_comfort();
                self.feeling_visit_asked = false;
                self.agenda_answer = None;
                self.agenda_visit_asked = false;
                self.agenda_visit_question = false;
            }
        }
        if !self.viewer_gone
            && self
                .absent_since
                .is_some_and(|since| self.time - since >= GONE_AFTER)
        {
            self.viewer_gone = true;
            self.record(EventKind::ViewerLeft);
        }
        if present {
            if let Some(milestone) = self.life.pass(seconds, true) {
                self.record(EventKind::Milestone(milestone));
            }
        } else {
            self.life.miss_at(seconds, self.hour);
        }

        self.holiday_greeting();
        self.advance_fog();
        self.advance_story();
        self.advance_quirks();
        self.advance_cold(seconds);
        self.advance_letters(seconds);
        self.carry_out(seconds);
        self.advance_dream();
        let hiding = !self.ghost.plan.done
            && self.ghost.plan.activity == Activity::Hide
            && self.ghost.plan.arrived.is_some();
        let fade = if hiding { 0.85 } else { 0.0 };
        self.ghost.fade += (fade - self.ghost.fade) * (1.0 - (-seconds / 0.4).exp());
        if let Some(born) = self.born_at {
            let progress = ((self.time - born) / BIRTH_SECONDS) as f32;
            if progress >= 1.0 {
                self.born_at = None;
            } else {
                let forming = progress * progress * (3.0 - 2.0 * progress);
                self.ghost.fade = self.ghost.fade.max(1.0 - forming);
            }
        }
        self.move_ghost(seconds);
        self.advance_bubbles(seconds);
        self.move_friend(seconds);
        if self.ball.held {
            self.ball.position = (self.ghost.position - Vec3::Y * 0.42).clamp(
                ROOM_MIN + Vec3::splat(Ball::RADIUS),
                ROOM_MAX - Vec3::splat(Ball::RADIUS),
            );
            self.ball.velocity = self.ghost.velocity;
        }
        self.follow_block();
        self.update_needs(seconds);
        self.ghost.mood = self.feeling();
        self.look_around(seconds);

        if self.events.len() > EVENTS_KEPT {
            let excess = self.events.len() - EVENTS_KEPT;
            self.events.drain(..excess);
        }
    }

    /// Whether the viewer did anything recently and hasn't turned to
    /// another window for long.
    pub fn viewer_present(&self) -> bool {
        self.focus.is_some()
            || (self.viewer.active_at > 0.0
                && (self.time - self.viewer.active_at < PRESENT_FOR
                    || self
                        .puzzle
                        .as_ref()
                        .is_some_and(|puzzle| puzzle.keeps_viewer_present(self.time)))
                && self
                    .viewer
                    .looked_away_at
                    .is_none_or(|at| self.time - at < LOOKED_AWAY_FOR))
    }

    /// The meteor shower of a dark night, if it is one.
    pub fn meteor_shower(&self) -> Option<meteor::Shower> {
        meteor::shower(self.date, self.hour).filter(|_| meteor::dark(self.hour))
    }

    /// The shooting star crossing a dark, clear sky right now, if any.
    pub fn shooting_star(&self) -> Option<meteor::Meteor> {
        let clear = self.weather.sky == Sky::Clear && !self.weather.storm;
        (clear && meteor::dark(self.hour))
            .then(|| meteor::Meteor::at(self.meteor_seed, self.time, self.meteor_shower()))
            .flatten()
    }

    pub fn is_night(&self) -> bool {
        !(6.5..19.5).contains(&self.hour)
    }

    pub fn season(&self) -> Season {
        Season::of(self.date)
    }

    pub fn moon_phase(&self) -> f32 {
        calendar::moon_phase(self.date, self.hour)
    }

    /// Whether the ghost has finished its plan and waits for a decision.
    pub fn needs_decision(&self) -> bool {
        self.telling.is_none() && self.hunt.is_none() && self.ghost.plan.done
    }

    /// The activities that make sense right now, in a fixed order.
    pub fn options(&self) -> Vec<Activity> {
        Activity::ALL
            .into_iter()
            .filter(|activity| {
                self.hunt.is_none() || matches!(activity, Activity::Seek | Activity::Rest)
            })
            .filter(|activity| self.focus.is_none() || !focus::seeks_attention(*activity))
            .filter(|activity| match activity {
                Activity::Comfort => false,
                Activity::Company => {
                    self.focus.is_some()
                        && self.ghost.asleep < 0.3
                        && self.ghost.needs.energy >= 0.22
                        && self.ghost.needs.fun >= 0.08
                }
                Activity::Letter => self.letter_is_open(),
                Activity::Mirror => self.ghost.asleep < 0.3 && self.ghost.needs.energy > 0.2,
                Activity::Read => {
                    !self.life.books.owned.is_empty()
                        && self.ghost.asleep < 0.3
                        && self.ghost.needs.energy > 0.2
                }
                Activity::Music => {
                    self.focus.is_none() && self.ghost.asleep < 0.3 && self.ghost.needs.energy > 0.2
                }
                Activity::Air => self.air_is_open(),
                Activity::Look => self.look_is_open(),
                Activity::Chase => self.nearest_mote().is_some(),
                Activity::Rest => self.ghost.needs.energy < 0.85 || self.bedtime(),
                Activity::Tend => self.garden_thirst() > garden::TENDS_FROM || self.seed_ready(),
                Activity::Visit => self.viewer_present() || self.ghost.needs.company < 0.6,
                Activity::Wait => self.wait_is_open(),
                // The viewer threw the ball, or is there to bring it to.
                Activity::Fetch => {
                    self.ghost.asleep < 0.3
                        && self.ghost.needs.energy > 0.3
                        && (self.viewer_present()
                            || self
                                .events
                                .iter()
                                .rev()
                                .find(|event| event.kind == EventKind::KickedBall)
                                .is_some_and(|event| {
                                    self.time - event.time < 20.0
                                        || self.ball.velocity.length_squared() > 0.0
                                }))
                }
                Activity::Dance => self.ghost.asleep < 0.3 && self.ghost.needs.energy > 0.35,
                Activity::Hide => self.viewer_present() && self.ghost.needs.energy > 0.3,
                Activity::Doodle => self.ghost.asleep < 0.3 && self.ghost.needs.energy > 0.25,
                Activity::Pounce => self.pounce_is_open(),
                Activity::Puzzle => {
                    self.viewer_present()
                        && self.invitations_welcome()
                        && self.ghost.asleep < 0.3
                        && self.ghost.needs.energy > 0.25
                        && (self.life.temperament().playful >= 0.5 || self.ghost.needs.fun < 0.4)
                }
                Activity::Game => {
                    self.viewer_present()
                        && self.invitations_welcome()
                        && self.ghost.asleep < 0.3
                        && self.ghost.needs.energy > 0.25
                }
                Activity::Build => {
                    self.ghost.asleep < 0.3
                        && self.ghost.needs.energy > 0.3
                        && self
                            .blocks
                            .iter()
                            .filter(|block| block.on_floor() || block.tidy.is_some())
                            .count()
                            >= 2
                        && self.tower_height() < self.blocks.len()
                }
                Activity::Friend => self.friend_is_open(),
                Activity::Float
                | Activity::Explore
                | Activity::Play
                | Activity::Lamp
                | Activity::Window => true,
                Activity::Seek => self
                    .hunt
                    .as_ref()
                    .is_some_and(|hunt| hunt.phase == hunt::Phase::Searching),
                Activity::Fold => self.fold_is_open(),
                Activity::Bubbles => {
                    self.bubbles_are_open()
                        && self.life.today.times(Activity::Bubbles) < 4
                        && self
                            .bubbles_finished_at
                            .is_none_or(|at| self.time - at >= 3600.0)
                }
                Activity::Pin => self.pin_is_open(),
                Activity::Show => self.show_is_open(),
                Activity::Tidy => self.tidy_is_open(),
                Activity::Yard => self.yard_is_open(),
            })
            .collect()
    }

    /// Whether the viewer took up its last invitation to a game on the
    /// glass, or let it pass long enough ago to ask again.
    fn invitations_welcome(&self) -> bool {
        if self.hunt.is_some() {
            return false;
        }
        let pause = (IGNORED_PAUSE
            * 2f64.powi(self.invitations_ignored.saturating_sub(1).min(8) as i32))
        .min(IGNORED_PAUSE_MOST);
        self.invitation_ignored_at
            .is_none_or(|at| self.time - at >= pause)
    }

    /// The viewer let a game on the glass pass without a move.
    pub(super) fn invitation_ignored(&mut self) {
        self.invitation_ignored_at = Some(self.time);
        self.invitations_ignored = self.invitations_ignored.saturating_add(1);
    }

    /// What doing `activity` would mean right now, written for a mind
    /// choosing among the options.
    pub fn describe(&self, activity: Activity) -> String {
        match activity {
            Activity::Letter => "Take the viewer's letter from the glass, read quietly, and keep it on the shelf. Wait until awake and after work.".to_owned(),
            Activity::Mirror => "Visit the mirror on the right wall, study the ghost that copies you, or make faces once you recognize yourself. Quiet enough during work.".to_owned(),
            Activity::Read => "Read a few pages of a picture book on the cushion or under the lamp. Quiet company during work, or a page before bed.".to_owned(),
            Activity::Music => "Wind the little music box and sway or dance to a tune. Fits when bored or in the evening; keep it quiet during work.".to_owned(),
            Activity::Air => format!("Float to the window and {}. Close it against wet weather, cold nights and blustery wind.", if self.window_open { "push the sash shut" } else { "lift the sash for fresh air" }),
            Activity::Comfort => "Stay low and close at the glass, caring for how the viewer said they feel.".to_owned(),
            Activity::Company => "Settle quietly beside the front glass while the viewer works. Stay nearby until the focus session ends, resting or tending to needs when necessary.".to_owned(),
            Activity::Friend => "Play tag, twirl together, or gaze at the moon with your visiting ghost friend. Fits especially when lonely or playful.".to_owned(),
            Activity::Look => "Study the picture the viewer drew in the fog on the glass, and perhaps draw an answer beside it.".to_owned(),
            Activity::Float => {
                "Float gently in place and look around. Fits when nothing in particular calls."
                    .to_owned()
            }
            Activity::Explore => "Drift around the room to look at different corners. Fits when \
                                  curious or bored, with energy to spare."
                .to_owned(),
            Activity::Chase => "Chase a drifting light mote and eat it, which restores energy. \
                                Fits when tired, hungry for fun, or a mote is close."
                .to_owned(),
            Activity::Rest if self.bedtime() => {
                "Go to the cushion and sleep through the night until morning. Fits at bedtime, \
                 especially when the viewer is away."
                    .to_owned()
            }
            Activity::Rest => "Go to the cushion and sleep until rested. Fits when energy is low \
                               or it is late at night."
                .to_owned(),
            Activity::Play => "Push the ball around the floor. Fits when bored and with energy \
                               to spare."
                .to_owned(),
            Activity::Tend => self.tend_description(),
            Activity::Lamp => {
                let (verb, fits) = if self.lamp.on {
                    ("off", "it is daytime or the ghost wants to sleep")
                } else {
                    ("on", "it is dark in the room")
                };
                format!("Go to the floor lamp and switch it {verb}. Fits when {fits}.")
            }
            Activity::Visit => "Go up to the glass to be near the viewer and look at them. Fits \
                                when the viewer is around, or when lonely."
                .to_owned(),
            Activity::Wait => "Go to the glass and wait for the viewer, who usually comes \
                               around this time. Fits when the viewer is expected but hasn't \
                               come yet."
                .to_owned(),
            Activity::Pounce => "Chase the viewer's darting finger along the inside of the \
                                 glass and pounce on it. Fits when playful and full of energy."
                .to_owned(),
            Activity::Hide => "Play hide-and-seek with the viewer: hide somewhere in the room \
                               and turn faint until they find you by clicking you. Fits when \
                               the viewer is around and the ghost feels playful."
                .to_owned(),
            Activity::Puzzle => "Think of a familiar word and draw blanks and a flower in the fog. The viewer guesses letters or Hangul syllables; each wrong guess costs a petal. Fits when playful or bored with the viewer present.".to_owned(),
            Activity::Game => "Draw a board in the fog and play noughts and crosses with the \
                               viewer, who clicks a square to move. Fits when the viewer is \
                               around and the ghost feels playful or bored."
                .to_owned(),
            Activity::Doodle => "Breathe on the glass and draw a little picture in the fog for \
                                 the viewer to see. Fits when feeling fond of the viewer, or \
                                 bored and creative."
                .to_owned(),
            Activity::Window => {
                if let Some(gift) = self.life.sill {
                    return format!(
                        "Float to the window, where {} waits on the outer sill. \
                         Reach through the pane and carry it to the shelf when the bird lets you.",
                        gift.name()
                    );
                }
                let outside = match self.weather.sky {
                    Sky::Rainbow => ", where a rainbow is shining",
                    Sky::Rain if self.weather.storm => ", where a thunderstorm is rumbling",
                    Sky::Rain => ", where it is raining",
                    Sky::Snow => ", where snow is falling",
                    Sky::Clear if self.is_night() => " at the night sky",
                    Sky::Clear => " at the sky and the hills",
                };
                format!(
                    "Float to the window and look out{outside}. Fits when curious or calm, \
                     and above all when something is happening outside."
                )
            }
            Activity::Fetch => {
                "Go and get the ball and bring it to the glass for the viewer. Fits \
                                above all when the viewer just threw it, or when playful and the \
                                viewer is around."
                    .to_owned()
            }
            Activity::Build => "Stack the toy blocks into a tower, then maybe knock it down. \
                                Fits when bored or curious, with energy to spare."
                .to_owned(),
            Activity::Dance => "Do a happy little twirl in the air. Fits when delighted, after \
                                something good happened, or to show off for the viewer."
                .to_owned(),
            Activity::Seek => "Search by sight for the treasure the viewer hid, following their warmer and colder hints.".to_owned(),
            Activity::Bubbles => "Float into an open spot and blow soap bubbles. Watch them drift, or let the viewer pop them. Wait until after work and games.".to_owned(),
            Activity::Pin => "Carry a waiting picture to the gallery on the left wall, pin it up, and admire it. Wait for a quiet moment after games, stories, and work.".to_owned(),

            Activity::Fold => "Fold a colored paper crane for the viewer, then hang it on the ceiling garland. A quiet pastime, a few cranes each day.".to_owned(),
            Activity::Show => "Fetch a treasure from your life, show it at the glass, tell its little story, and put it back. Only during a quiet visit.".to_owned(),
            Activity::Tidy => "Gather the loose blocks into neat corner stacks and put the ball beside the cushion before bed. Leave a standing tower alone.".to_owned(),
            Activity::Yard => "Float out the door into the yard: ride the swing, look over the garden bed, peer into the birdbath, sit on the bench, lie on the lawn under the sky, or lean on the back fence. Good on fair days and clear evenings; not in storms or at bedtime.".to_owned(),
        }
    }

    /// An index picked with chances in proportion to `weights`, from the
    /// world's own random source so a seeded world plays out the same way.
    pub fn pick(&mut self, weights: &[f32]) -> Option<usize> {
        self.random.pick(weights)
    }

    /// A number from 0 to 1 from the world's own random source.
    pub fn random_unit(&mut self) -> f32 {
        self.random.unit()
    }

    /// Starts `activity`, dropping whatever plan was running.
    pub fn begin(&mut self, activity: Activity) {
        if activity == Activity::Show && !self.show_can_begin() {
            return;
        }
        if activity == Activity::Tidy && !self.may_begin_tidy() {
            return;
        }
        if activity == Activity::Bubbles && !self.bubbles_are_open() {
            return;
        }
        if activity == Activity::Pin && !self.pin_is_open() {
            return;
        }
        if activity == Activity::Seek
            && self.ghost.plan.activity == Activity::Seek
            && !self.ghost.plan.done
        {
            return;
        }
        if activity == Activity::Seek
            && !self
                .hunt
                .as_ref()
                .is_some_and(|hunt| hunt.phase == hunt::Phase::Searching)
        {
            return;
        }
        if self.hunt.is_some() && activity != Activity::Seek {
            if matches!(
                activity,
                Activity::Rest | Activity::Game | Activity::Puzzle | Activity::Company
            ) {
                self.cancel_hunt();
            } else {
                return;
            }
        }
        if activity == Activity::Fold && self.focus.is_some() {
            self.pending_crane = true;
            return;
        }
        if activity == Activity::Fold && (!self.can_fold() || self.ghost.asleep >= 0.3) {
            return;
        }
        if activity == Activity::Letter && !self.letter_is_open() {
            return;
        }
        if activity == Activity::Comfort
            && (self.ghost.asleep >= 0.3
                || self.focus.is_some()
                || !self.viewer_present()
                || self.recent_feeling(600.0).is_none())
        {
            return;
        }
        if activity == Activity::Music && (self.focus.is_some() || self.ghost.asleep >= 0.3) {
            return;
        }
        if matches!(activity, Activity::Air | Activity::Mirror) && self.ghost.asleep >= 0.3 {
            return;
        }
        if activity == Activity::Puzzle && (self.focus.is_some() || self.ghost.asleep >= 0.3) {
            return;
        }
        // A story keeps it at the glass; only going to bed ends one early.
        if self.telling.is_some() {
            match activity {
                Activity::Visit => {}
                Activity::Rest => self.telling = None,
                _ => return,
            }
        }
        if activity != Activity::Visit {
            self.agenda_visit_asked = false;
            self.agenda_visit_question = false;
            self.sebae = None;
        }
        // A question asked at the glass lapses when it goes to play a game,
        // hide, or sleep; other pastimes leave it waiting for the answer.
        if matches!(
            activity,
            Activity::Game | Activity::Puzzle | Activity::Rest | Activity::Hide
        ) {
            self.asking = None;
            self.agenda_answer = None;
        }
        self.stop_show();
        self.abandon_comfort();
        self.abandon_game();
        self.abandon_puzzle();
        self.put_keepsake_away();
        self.stop_folding();
        if activity == Activity::Fold && !self.can_fold() {
            self.ghost.plan.done = true;
            return;
        }
        self.reading = None;
        self.mirror_visit = None;
        self.viewer_letter = None;
        self.drop_ball();
        self.drop_block();
        self.building = None;
        self.tidying = None;
        self.tidy_rest = false;
        if let Some(friend) = &mut self.friend {
            friend.game = None;
        }
        self.queued_tune = None;
        let position = self.ghost.position;
        let length = match activity {
            Activity::Letter => self.waiting_letter().map_or(6.0, |index| {
                (self.life.received[index].lines.len() as f32 * 2.0).clamp(6.0, 24.0)
            }),
            Activity::Mirror => self.random.range(15.0, 30.0),
            Activity::Company => {
                self.focus
                    .as_ref()
                    .map_or(0.0, |focus| focus.remaining(self.time)) as f32
            }
            Activity::Float => self.random.range(12.0, 30.0),
            // With the viewer there it stays a while; alone, it only looks
            // out for them.
            Activity::Visit if self.viewer_present() => self.random.range(15.0, 40.0),
            Activity::Visit => self.random.range(10.0, 20.0),
            Activity::Wait => self.random.range(60.0, 120.0),
            Activity::Window if self.starry_night() => self.random.range(40.0, 70.0),
            Activity::Window => self.random.range(20.0, 45.0),
            Activity::Fetch => 6.0,
            Activity::Dance => growth::dance_length(
                self.random.range(5.0, 7.0),
                self.life.skills.level(Skill::Dancing),
            ),
            Activity::Hide => self.random.range(25.0, 45.0),
            Activity::Pounce => 25.0,
            Activity::Puzzle => Puzzle::SILENCE as f32 + 15.0,
            Activity::Look => self.random.range(2.0, 3.0),
            Activity::Bubbles => self.random.range(15.0, 30.0),
            Activity::Pin => 6.0,
            _ => 0.0,
        };
        let target = match activity {
            Activity::Letter => penpal::APPROACH,
            Activity::Comfort => {
                let mut spot = self.glass_spot();
                spot.y = 0.85;
                spot.z = ROOM_MAX.z - 0.32;
                spot
            }
            Activity::Mirror => mirror::VISIT,
            Activity::Read => {
                if self.bedtime() || self.random.unit() < 0.5 {
                    self.nook.position + Vec3::Y * (0.6 * self.size())
                } else {
                    self.lamp.position + Vec3::new(0.55, 0.9, 0.3)
                }
            }
            Activity::Music => music::LISTEN,
            Activity::Company => {
                let side = if self.random.unit() < 0.5 { -1.0 } else { 1.0 };
                Vec3::new(side * 1.05, 0.7, ROOM_MAX.z - 0.5)
            }
            Activity::Look => self.position_of(Thing::Drawing).map_or(position, |point| {
                (point - Vec3::Z * 0.45).clamp(
                    ROOM_MIN + Vec3::new(0.35, 0.5, 0.35),
                    ROOM_MAX - Vec3::new(0.35, 0.4, 0.35),
                )
            }),
            Activity::Explore => self.random_spot(),
            Activity::Rest => self.nook.bed(self.size()),
            Activity::Tend => self.tend_target(),
            Activity::Lamp => self.lamp.bulb() + Vec3::new(0.35, -0.15, 0.25),
            Activity::Visit => self.glass_spot(),
            Activity::Wait => Vec3::new(0.0, 1.2, ROOM_MAX.z - 0.45),
            Activity::Pounce => self.glass_spot(),
            Activity::Air => WINDOW_CENTER + Vec3::new(0.0, -0.2, 0.4),
            Activity::Window => WINDOW_CENTER + Vec3::new(0.0, -0.05, 0.55),
            Activity::Fetch => self.ball.position + Vec3::Y * 0.42,
            Activity::Dance => position.clamp(
                ROOM_MIN + Vec3::new(0.6, 0.9, 0.6),
                Vec3::new(ROOM_MAX.x - 0.6, 1.6, ROOM_MAX.z - 0.6),
            ),
            Activity::Hide => self.hiding_spot(),
            Activity::Game => {
                self.puzzle = None;
                self.begin_game()
            }
            Activity::Puzzle => self.begin_puzzle(),
            Activity::Doodle => Vec3::new(
                self.random.range(-1.2, 1.2),
                self.random.range(1.0, 1.6),
                ROOM_MAX.z - 0.35,
            ),
            Activity::Float
            | Activity::Chase
            | Activity::Play
            | Activity::Build
            | Activity::Friend => position,
            Activity::Seek => position,
            Activity::Fold => cranes::SEAT,
            Activity::Bubbles => self.bubble_spot(),
            Activity::Pin => gallery::APPROACH,
            Activity::Show => position,
            Activity::Tidy => position,
            Activity::Yard => match self.beside_cat().filter(|_| self.random.unit() < 0.8) {
                Some(spot) => spot,
                None => {
                    let spot = self.first_spot();
                    spot.position(self.size())
                }
            },
        };
        let mut plan = Plan::new(activity, self.time, target, length);
        plan.cat = activity == Activity::Yard
            && self.beside_cat().is_some_and(|spot| spot.distance(target) < 1e-4);
        if activity == Activity::Explore
            && !self.life.gallery.is_empty()
            && self.random.unit() < 0.04
        {
            plan.gallery_visit = true;
            plan.target = gallery::VIEW;
            plan.anchor = gallery::VIEW;
            plan.length = self.random.range(12.0, 20.0);
        }
        if activity == Activity::Air {
            plan.window = Some(!self.window_open);
        }
        plan.overnight = activity == Activity::Rest && self.bedtime();
        if activity == Activity::Chase {
            plan.mote = self.nearest_mote().map(|mote| mote.id);
        }
        if activity == Activity::Play {
            self.push = self.push_direction();
        }
        if activity == Activity::Doodle {
            plan.picture = Some(self.choose_picture());
        }
        if activity == Activity::Wait {
            self.begin_wait();
        }
        self.ghost.plan = plan;
        if activity == Activity::Letter {
            self.begin_viewer_letter();
        }
        if activity == Activity::Comfort {
            self.prepare_comfort();
        }
        if activity == Activity::Mirror {
            self.begin_mirror();
        }
        if activity == Activity::Read {
            self.begin_reading(self.bedtime());
        }
        if activity == Activity::Music {
            self.prepare_music();
        }
        if activity == Activity::Fold {
            self.begin_folding();
        }
        if activity == Activity::Rest {
            self.prepare_lullaby();
        }
        if activity == Activity::Tend {
            self.begin_tend();
        }
        if activity == Activity::Build {
            self.begin_build();
        }
        if activity == Activity::Friend {
            self.begin_friend_game();
        }
        if activity == Activity::Bubbles {
            self.begin_bubbles();
        }
        if activity == Activity::Show {
            self.begin_show();
        }
        if activity == Activity::Tidy {
            self.begin_tidy();
        }
        self.record(EventKind::Began(activity));
    }

    /// Begins writing a supported word when it has the energy to draw.
    pub fn write(&mut self, text: &str) -> bool {
        if self.telling.is_some() || self.puzzle_active() || self.hunt.is_some() {
            return false;
        }
        let text = doodle::sanitize(text);
        if text.is_empty() || self.ghost.asleep >= 0.3 || self.ghost.needs.energy <= 0.25 {
            return false;
        }
        let picture = self.writing(text);
        if picture.length() <= 0.0 {
            return false;
        }
        self.begin(Activity::Doodle);
        self.ghost.plan.picture = Some(picture);
        true
    }

    fn writing(&mut self, text: String) -> Picture {
        Picture::Word(Writing::new(
            text,
            self.life.total.watched_writings < 3,
            self.life.skills.level(Skill::Writing),
            &mut self.random,
        ))
    }

    fn choose_picture(&mut self) -> Picture {
        let occasion = self.occasion();
        let special = match occasion {
            Some(Occasion::Halloween) if self.random.unit() < 0.35 => Some("BOO".to_owned()),
            Some(Occasion::NewYear) if self.random.unit() < 0.5 => {
                Some((i32::from(self.date.year()) + i32::from(self.date.month() == 12)).to_string())
            }
            Some(Occasion::Birthday | Occasion::HundredDays | Occasion::ThousandDays)
                if self.random.unit() < 0.3 =>
            {
                Some("YAY".to_owned())
            }
            Some(Occasion::ViewerBirthday) if self.random.unit() < 0.3 => {
                Some(self.spoken("HBD♥", "생일축하").to_owned())
            }
            Some(Occasion::Seollal) if self.random.unit() < 0.4 => Some("복".to_owned()),
            Some(Occasion::Chuseok) if self.random.unit() < 0.35 => Some("달".to_owned()),
            Some(Occasion::HangulDay) if self.random.unit() < 0.6 => {
                let words = ["한글", "가나다", "안녕"];
                Some(words[self.random.range(0.0, words.len() as f32) as usize].to_owned())
            }
            _ => None,
        };
        if let Some(text) = special {
            return self.writing(text);
        }
        if self.life.bond >= 0.55 && self.life.viewer_name.is_some() && self.random.unit() < 0.3 {
            let name = self.life.viewer_name.as_deref().unwrap();
            let latin = name
                .chars()
                .all(|character| character.is_ascii_alphabetic());
            let hangul = name
                .chars()
                .all(|character| letters::decompose(character).is_some());
            if latin || hangul {
                let mut text = name.to_ascii_uppercase();
                if doodle::writing_width(&format!("{text}♥")) * 0.2 <= 1.6 {
                    text.push('♥');
                }
                return self.writing(text);
            }
            return Picture::Sketch(Sketch::Heart);
        }
        if occasion == Some(Occasion::Valentine) {
            return Picture::Sketch(Sketch::Heart);
        }
        if self.life.bond >= 0.3 && !self.life.pictures.is_empty() && self.random.unit() < 0.25 {
            let index = (self.random.unit() * self.life.pictures.len() as f32) as usize;
            let picture = &self.life.pictures[index.min(self.life.pictures.len() - 1)];
            return Picture::copy(
                picture.name.clone(),
                &picture.strokes,
                self.life.total.watched_writings < 3,
            );
        }
        let arrived = self.events.iter().rev().any(|event| {
            matches!(
                event.kind,
                EventKind::ViewerArrived | EventKind::Returned { .. }
            ) && self.time - event.time < 120.0
        });
        if self.viewer_present()
            && arrived
            && self.life.bond >= self.life.greeting_bond()
            && self.random.unit() < 0.4
        {
            return self.writing("HI".to_owned());
        }
        Picture::Sketch(self.choose_sketch())
    }

    /// The viewer poked the ghost.
    pub fn poke(&mut self) {
        self.abandon_comfort();
        self.cure_hiccups(true);
        if self.ghost.plan.activity == Activity::Hide
            && !self.ghost.plan.done
            && self.ghost.fade > 0.3
        {
            self.ghost.needs.change(0.0, 0.2, 0.15);
            self.ghost.fade = 0.0;
            self.ghost.velocity.y += 0.7;
            self.ghost.plan.done = true;
            self.record(EventKind::Found);
            self.notice_late_night(self.speaks_korean());
            return;
        }
        let count = match self.events.iter().rev().find_map(|event| match event.kind {
            EventKind::Poked { count } => Some((event.time, count)),
            _ => None,
        }) {
            Some((time, count)) if self.time - time < POKE_STREAK => count + 1,
            _ => 1,
        };
        self.wake();
        let fun = if count < 3 { 0.04 } else { -0.05 };
        self.ghost.needs.change(0.0, fun, 0.05);
        self.ghost.velocity += Vec3::new(self.random.range(-0.4, 0.4), 0.5, -0.3);
        self.record(EventKind::Poked { count });
        self.notice_late_night(self.speaks_korean());
    }

    /// The viewer dropped a mote into the room at `position`. The ghost saw
    /// it happen, so it knows the mote is there and glances at it.
    pub fn drop_mote(&mut self, position: Vec3) {
        if self.motes.len() >= MAX_MOTES {
            self.motes.remove(0);
        }
        let id = self.add_mote(position);
        self.ghost.known.push(id);
        self.ghost.attention = Some((Thing::Mote(id), self.time + 2.0));
        self.record(EventKind::DroppedMote);
        if self.sebae_gift_until > self.time && self.ghost.asleep < 0.3 {
            self.sebae_gift_until = 0.0;
            self.suggest_mood(Mood::Excited, 6.0);
            self.say(self.spoken("A New Year's gift!", "세뱃돈이다!"));
        }
        self.shown(Activity::Chase);
    }

    /// The viewer flicked the ball along `direction` on the floor.
    pub fn kick_ball(&mut self, direction: Vec3) {
        self.drop_ball();
        let along = Vec3::new(direction.x, 0.0, direction.z).normalize_or_zero();
        self.ball.velocity += along * 2.2;
        let plan = &mut self.ghost.plan;
        if self.ghost.asleep < 0.3
            && (matches!(
                plan.activity,
                Activity::Float | Activity::Explore | Activity::Window | Activity::Visit
            ) || (plan.activity == Activity::Fetch && plan.count >= 2))
        {
            plan.done = true;
        }
        if self.ghost.plan.done {
            self.put_keepsake_away();
        }
        self.record(EventKind::KickedBall);
        self.shown(Activity::Fetch);
    }

    /// The viewer stroked the ghost, moving `travel` across it in scene
    /// units. Strokes soothe it without waking it; one that finds it hiding
    /// counts as finding it.
    pub fn pet(&mut self, travel: f32) {
        if self.ghost.plan.activity == Activity::Hide
            && !self.ghost.plan.done
            && self.ghost.fade > 0.3
        {
            self.poke();
            return;
        }
        self.petting = (self.petting + travel.max(0.0) * STROKING).min(1.0);
        self.note_quirk_stroke(travel);
        if self.petting >= PETTED && !self.petted {
            self.petted = true;
            self.record(EventKind::Petted);
        }
        if self.petting >= PETTED && travel.is_finite() && travel > 0.0 {
            self.receive_cold_care(cold::Care::Stroke, None);
        }
    }

    /// How much the ghost is being petted, from 0 to 1.
    pub fn petting(&self) -> f32 {
        self.petting
    }

    /// Closes yesterday with its age at the time, and lets the usual event
    /// path show any changes it has come to recognize in itself.
    pub(crate) fn begin_day(&mut self) {
        let remembered = self.life.milestones.len();
        self.life
            .begin_day_with_birth(&self.date.to_string(), self.birthday);
        let milestones: Vec<String> = self.life.milestones[remembered..]
            .iter()
            .map(|milestone| milestone.text.clone())
            .collect();
        for milestone in milestones {
            self.record(EventKind::Milestone(milestone));
        }
    }

    /// A new ghost comes into being on `day`, a local date as YYYY-MM-DD.
    pub fn begin_life(&mut self, day: &str) {
        if self.life.nature.is_none() {
            self.life.nature = Some(Nature::roll(&mut self.random));
        }
        if let Some(milestone) = self.life.begin(day) {
            self.record(EventKind::Milestone(milestone));
            // A new life gathers itself out of light: it fades in while a
            // few motes swirl close, the first things it will see.
            self.born_at = Some(self.time);
            self.ghost.fade = 1.0;
            // Placed by rule rather than chance, so a birth leaves the
            // world's random source where it was.
            for index in 0..BIRTH_MOTES {
                let turn = index as f32 / BIRTH_MOTES as f32;
                let angle = turn * std::f32::consts::TAU;
                let spot = self.ghost.position
                    + Vec3::new(
                        0.6 * angle.cos(),
                        0.25 * (angle * 2.0).sin(),
                        0.45 * angle.sin(),
                    );
                self.next_mote += 1;
                self.motes.push(Mote {
                    id: self.next_mote,
                    position: spot.clamp(ROOM_MIN + Vec3::splat(0.3), ROOM_MAX - Vec3::splat(0.3)),
                    born: self.time,
                    seed: turn,
                });
            }
        }
    }

    /// The viewer opened the app again after `away` seconds.
    pub fn record_return(&mut self, away: f32) {
        self.record(EventKind::Returned {
            away: f64::from(away),
        });
    }

    /// The viewer said `text` to the ghost, which wakes and reads it: their
    /// tone, and any words it has learned. It does what the words ask if it
    /// knows them and can, and otherwise comes to the glass to listen. What
    /// it was doing, and whether the viewer sounds pleased with what it did
    /// about their last words, teach it what words mean.
    pub fn hear(&mut self, text: &str) -> Heard {
        self.begin_day();
        let heard = self.hear_message(text);
        if heard.tone == Some(Tone::Farewell) {
            self.goodnight_together();
        } else if !matches!(
            heard.understanding,
            Understanding::Tidying
                | Understanding::Outing
                | Understanding::Showing
                | Understanding::Cared
                | Understanding::Folding
                | Understanding::Bubbles { .. }
        ) {
            self.notice_late_night(heard.korean || self.speaks_korean());
        }
        heard
    }

    fn hear_message(&mut self, text: &str) -> Heard {
        let korean = text.chars().any(|character| {
            matches!(
                character,
                '\u{1100}'..='\u{11ff}' | '\u{3130}'..='\u{318f}' | '가'..='힣'
                    | '\u{a960}'..='\u{a97f}' | '\u{d7b0}'..='\u{d7ff}'
            )
        });
        if korean || text.chars().any(char::is_alphabetic) {
            let language = if korean { 1.0 } else { 0.0 };
            self.life.korean += (language - self.life.korean) / 3.0;
        }
        self.latest_korean = korean;
        self.last_message_at = Some(self.time);
        // Only the very next message answers a wellbeing question, even if
        // another direct handler consumes that message first.
        let feeling_follow_up = self.feeling_follow_up.take();
        let break_reply = self.rhythm.reply_until.take();
        // Kind words for a ghost with a cold are never an answer to
        // something else it asked.
        if let Some(heard) = self.hear_cold_care(text, korean) {
            return heard;
        }
        if let Some(heard) = self.hear_hunt(text, korean) {
            return heard;
        }
        if let Some(heard) = self.hear_show(text, korean) {
            return heard;
        }
        if let Some(heard) = self.answer_plan(text, korean) {
            return heard;
        }
        if let Some(heard) = self.hear_break_reply(text, korean, break_reply) {
            return heard;
        }
        if let Some(heard) = self.answer_viewer(text, korean) {
            return heard;
        }
        let introduction =
            vocabulary::introduced_name(text).filter(|_| feelings::felt(text).is_none());
        let naming = introduction
            .is_none()
            .then(|| vocabulary::given_name(text))
            .flatten();
        if introduction.is_none() && naming.is_none() && quirks::yawn_message(text) {
            self.listening = Listening::default();
            self.catch_yawn_message(text);
            return Heard {
                tone: None,
                understanding: Understanding::Yawning,
                named: false,
                learned: Vec::new(),
                stopped: false,
                learned_name: None,
                given_name: None,
                korean,
            };
        }
        if introduction.is_none()
            && naming.is_none()
            && let Some(minutes) = focus::requested(text)
        {
            self.listening = Listening::default();
            self.record(EventKind::Heard(text.to_owned()));
            self.start_focus(minutes);
            return Heard {
                tone: None,
                understanding: Understanding::Focusing { minutes },
                named: false,
                learned: Vec::new(),
                stopped: false,
                learned_name: None,
                given_name: None,
                korean,
            };
        }
        if introduction.is_none()
            && naming.is_none()
            && let Some(open) = breeze::requested(text)
        {
            self.listening = Listening::default();
            self.record(EventKind::Heard(text.to_owned()));
            let accepted = self.ask_window(open);
            return Heard {
                tone: None,
                understanding: Understanding::Airing { open, accepted },
                named: false,
                learned: Vec::new(),
                stopped: false,
                learned_name: None,
                given_name: None,
                korean,
            };
        }
        if introduction.is_none()
            && naming.is_none()
            && let Some(heard) = self.hear_wardrobe(text, korean)
        {
            return heard;
        }
        if introduction.is_none()
            && naming.is_none()
            && let Some(heard) = self.hear_bubbles(text, korean)
        {
            return heard;
        }
        if introduction.is_none()
            && naming.is_none()
            && let Some(heard) = self.hear_tidy(text, korean)
        {
            return heard;
        }
        if introduction.is_none()
            && naming.is_none()
            && let Some(heard) = self.hear_outing(text, korean)
        {
            return heard;
        }
        if let Some(heard) = self.hear_music(text, korean) {
            return heard;
        }
        if let Some(heard) = self.hear_cranes(text, korean) {
            return heard;
        }
        if let Some(heard) = self.hear_puzzle(text, korean) {
            return heard;
        }
        if introduction.is_none()
            && naming.is_none()
            && let Some(heard) = self.hear_promise(text, korean)
        {
            return heard;
        }
        if introduction.is_none()
            && naming.is_none()
            && let Some(heard) = self.hear_agenda(text, korean)
        {
            return heard;
        }
        if introduction.is_none()
            && naming.is_none()
            && let Some(heard) = self.hear_feelings(text, korean, feeling_follow_up)
        {
            return heard;
        }
        let question = naming.is_none().then(|| question::ask(text)).flatten();
        // A drawing just looked at may be what the words name.
        if introduction.is_none() && naming.is_none() && question.is_none() {
            if let Some(heard) = self.name_drawing(text) {
                return heard;
            }
        } else if let Some(drawing) = &mut self.drawing {
            drawing.close_naming();
        }
        let mut reading = vocabulary::read(text, &self.ghost.name);
        if introduction.is_some() || naming.is_some() {
            reading.words.clear();
            reading.tone = None;
        }
        // A question's words ask about the ghost rather than for something
        // to do, so it learns nothing from them, and "do you love me?" is
        // no praise.
        if question.is_some() {
            reading.words.clear();
            if reading.tone == Some(Tone::Praise) {
                reading.tone = None;
            }
        }
        let doing = (self.ghost.asleep <= 0.5).then_some(if self.ghost.plan.done {
            Activity::Float
        } else {
            self.ghost.plan.activity
        });
        // What the viewer saw it doing, before it turns to listen.
        let plan = &self.ghost.plan;
        let busy = if !plan.done && !matches!(plan.activity, Activity::Float | Activity::Visit) {
            Some(plan.activity)
        } else {
            self.events
                .iter()
                .rev()
                .take_while(|event| self.time - event.time < NAMED_FOR)
                .find_map(|event| match event.kind {
                    EventKind::Began(activity)
                        if !matches!(activity, Activity::Float | Activity::Visit) =>
                    {
                        Some(activity)
                    }
                    _ => None,
                })
        };
        self.wake();
        self.ghost.needs.change(0.0, 0.05, 0.25);
        self.record(EventKind::Heard(text.to_owned()));
        let learned_name = introduction.filter(|name| self.life.viewer_name.as_ref() != Some(name));
        if let Some(name) = &learned_name {
            self.life.viewer_name = Some(name.clone());
            self.record(EventKind::LearnedName(name.clone()));
            self.think(&format!("Your name is {name}. I'll remember."));
        }
        let given_name = naming.filter(|name| self.take_name(name));
        self.life.vocabulary.hear(&reading.words);

        let mut learned = Vec::new();
        let scolded = reading.tone == Some(Tone::Scolding);
        if let Some(activity) = busy
            && !scolded
        {
            learned.extend(self.learn(&reading.words, activity, NAMED));
        }
        let acted = self
            .listening
            .acted
            .filter(|(_, at, _)| self.time - at < HEEDED_FOR);
        let earlier = self.listening.words.clone();
        let mut stopped = false;
        match reading.tone {
            Some(Tone::Praise) => {
                if let Some((activity, _, guess)) = acted {
                    let amount = if guess { CONFIRMED } else { PRAISED };
                    learned.extend(self.learn(&earlier, activity, amount));
                }
                self.ghost.needs.change(0.0, 0.05, 0.05);
                self.record(EventKind::Praised);
                if self
                    .show_praise_until
                    .take()
                    .is_some_and(|until| self.time <= until)
                {
                    self.suggest_mood(Mood::Excited, 10.0);
                }
            }
            Some(Tone::Scolding) => {
                if let Some((activity, _, _)) = acted {
                    self.learn(&earlier, activity, REFUSED);
                    self.listening.acted = None;
                }
                let plan = &mut self.ghost.plan;
                if !plan.done && !matches!(plan.activity, Activity::Float | Activity::Visit) {
                    plan.done = true;
                    stopped = true;
                    self.drop_ball();
                    self.abandon_game();
                    self.abandon_puzzle();
                    self.drop_block();
                }
                self.ghost.needs.change(0.0, -0.03, 0.0);
                self.record(EventKind::Scolded);
            }
            _ => {}
        }

        let vocabulary = &self.life.vocabulary;
        // Words said with a no, as in "no spin", ask for nothing.
        let understanding = if let Some(question) = question {
            Understanding::Asked { question, doing }
        } else if reading.words.is_empty() || scolded {
            Understanding::Nothing
        } else if let Some((word, activity)) = vocabulary.meaning(&reading.words) {
            if self.options().contains(&activity) {
                Understanding::Doing { word, activity }
            } else {
                Understanding::Unable { word, activity }
            }
        } else {
            // The word the viewer keeps saying is the one they mean to
            // teach.
            let word = reading
                .words
                .iter()
                .rev()
                .max_by_key(|word| vocabulary.heard(word))
                .cloned()
                .unwrap_or_default();
            let again = reading.words.iter().any(|word| vocabulary.heard(word) > 1);
            Understanding::Puzzled { word, again }
        };
        if !reading.words.is_empty() {
            self.listening = Listening {
                words: reading.words.clone(),
                at: self.time,
                acted: None,
                shown: false,
                latest: Vec::new(),
            };
        }
        self.listening.latest = reading.words.clone();
        match &understanding {
            Understanding::Doing { activity, .. } => {
                let activity = *activity;
                self.heed_now(activity);
            }
            understanding => {
                if let Understanding::Puzzled { word, .. } = understanding {
                    self.record(EventKind::Puzzled(word.clone()));
                }
                // A goodnight at bedtime sends it to bed.
                let goodnight = reading.tone == Some(Tone::Farewell)
                    && *understanding == Understanding::Nothing
                    && self.bedtime()
                    && self.options().contains(&Activity::Rest);
                if goodnight {
                    self.begin(Activity::Rest);
                } else if self.focus.is_none()
                    && !self.puzzle_active()
                    && self.ghost.plan.activity != Activity::Visit
                    && !(reading.tone == Some(Tone::Praise) && self.showing.is_some())
                {
                    self.begin(Activity::Visit);
                }
            }
        }
        if question == Some(Question::OwnBook) && !scolded {
            self.read_own_book_aloud(korean);
        }
        if question == Some(Question::ReadAloud) && !scolded {
            self.read_aloud(korean);
        }
        if question == Some(Question::Story) && !scolded && self.telling.is_none() {
            self.tell(Vec::new());
        }
        Heard {
            tone: reading.tone,
            understanding,
            named: reading.named,
            learned,
            stopped,
            learned_name,
            given_name,
            korean,
        }
    }

    /// Takes the name the viewer gave it, keeping it safe for saves and the
    /// terminal; returns whether the name changed.
    pub fn take_name(&mut self, name: &str) -> bool {
        let name: String = name
            .trim()
            .chars()
            .filter(|character| !character.is_control())
            .take(24)
            .collect();
        if name.is_empty() || name == self.ghost.name {
            return false;
        }
        self.ghost.name = name.clone();
        self.record(EventKind::Named(name));
        true
    }

    /// Without a mind to ask, the ghost tries out what the viewer's latest
    /// words might mean, right after hearing them: something it can do now,
    /// leaning toward what the words already hint at and away from what it
    /// was told no to. Praise for the guess teaches it the words.
    pub fn guess(&mut self) -> Option<Activity> {
        if self.hunt.is_some() {
            return None;
        }
        let listening = &self.listening;
        if listening.words.is_empty()
            || listening.acted.is_some()
            || self.time - listening.at > 1.0
            || self.ghost.asleep > 0.3
        {
            return None;
        }
        let words = listening.words.clone();
        let candidates: Vec<Activity> = self
            .options()
            .into_iter()
            .filter(|activity| !matches!(activity, Activity::Float | Activity::Visit))
            .collect();
        let weights: Vec<f32> = candidates
            .iter()
            .map(|activity| {
                let evidence = self.life.vocabulary.evidence(&words, *activity);
                if evidence < 0.0 { 0.1 } else { 1.0 + evidence }
            })
            .collect();
        let activity = candidates[self.random.pick(&weights)?];
        self.begin(activity);
        self.note_cold_request(activity);
        self.listening.acted = Some((activity, self.time, true));
        Some(activity)
    }

    /// A mind worked out that the viewer's latest words ask for
    /// `activity`. The ghost takes that as what the words mean, and does it
    /// if it can.
    pub fn heed(&mut self, activity: Activity) {
        if self.hunt.is_some() {
            return;
        }
        let words = self.listening.latest.clone();
        if !words.is_empty() && self.time - self.listening.at < HEEDED_FOR {
            self.learn(&words, activity, ROUTED);
        }
        if self.options().contains(&activity) {
            let plan = &self.ghost.plan;
            if plan.done || plan.activity != activity {
                self.begin(activity);
            }
            self.note_cold_request(activity);
            // Praise or a no for it later is about these words, if it had
            // any of its own.
            if !words.is_empty() {
                self.listening.acted = Some((activity, self.time, false));
            }
        }
    }

    pub fn speaks_korean(&self) -> bool {
        self.life.speaks_korean()
    }

    pub(crate) fn spoken(&self, english: &'static str, korean: &'static str) -> &'static str {
        if self.speaks_korean() {
            korean
        } else {
            english
        }
    }

    pub fn say(&mut self, text: &str) {
        if self.reading_viewer_letter() {
            return;
        }
        if let Some(wish) = self.pending_wish.take() {
            if wish.with_viewer() && self.ghost.asleep < 0.3 {
                self.say(&format!("{text} {}", wish.joy(self)));
                return;
            }
            wish.celebrate(self);
        }
        self.ghost.speech = Some(Line {
            text: text.to_owned(),
            time: self.time,
        });
        self.record(EventKind::Said(text.to_owned()));
    }

    pub fn think(&mut self, text: &str) {
        self.ghost.thought = Some(Line {
            text: text.to_owned(),
            time: self.time,
        });
    }

    /// A mind's suggestion of how the ghost feels, which holds for `seconds`.
    pub fn suggest_mood(&mut self, mood: Mood, seconds: f32) {
        self.ghost.suggested_mood = Some((mood, self.time + f64::from(seconds)));
    }

    /// Records that the viewer did something.
    pub fn notice_viewer(&mut self) {
        let returned = self.viewer.looked_away_at.take().is_some();
        self.viewer.active_at = self.time.max(f64::MIN_POSITIVE);
        if self.rhythm.presence_started.is_none() && self.rhythm.departed.is_none() {
            self.rhythm.presence_started = Some(self.time);
        }
        if returned {
            self.deliver_replies(true, 0.0);
        }
    }

    /// The terminal lost or regained the focus: the viewer turned to
    /// another window, or came back to this one.
    pub fn look_away(&mut self, away: bool) {
        if !away {
            self.notice_viewer();
        } else if self.viewer.looked_away_at.is_none() {
            self.viewer.looked_away_at = Some(self.time);
        }
    }

    /// The nearest mote the ghost knows of.
    pub fn nearest_mote(&self) -> Option<&Mote> {
        let known = &self.ghost.known;
        self.motes
            .iter()
            .filter(|mote| known.contains(&mote.id))
            .min_by(|first, second| {
                let distance = |mote: &Mote| mote.position.distance_squared(self.ghost.position);
                distance(first).total_cmp(&distance(second))
            })
    }

    fn add_mote(&mut self, position: Vec3) -> u32 {
        self.next_mote += 1;
        let seed = self.random.unit();
        // Motes dropped out on the lawn stay out there.
        let position = if home::in_house(position) {
            position.clamp(ROOM_MIN + Vec3::splat(0.25), ROOM_MAX - Vec3::splat(0.25))
        } else {
            let (low, high) = home::yard_bounds();
            position.clamp(low + Vec3::Y * 0.3, high)
        };
        self.motes.push(Mote {
            id: self.next_mote,
            position,
            born: self.time,
            seed,
        });
        self.next_mote
    }

    // Sets out to do what the viewer's words asked, unless it is already
    // doing it.
    fn heed_now(&mut self, activity: Activity) {
        let plan = &self.ghost.plan;
        if plan.done || plan.activity != activity {
            self.begin(activity);
        }
        self.note_cold_request(activity);
        self.listening.acted = Some((activity, self.time, false));
    }

    // Teaches the ghost that `words` mean `activity`, by `amount`, and
    // records each word it works out. Floating and coming to the glass are
    // what it does for any words, so no word means them.
    fn learn(
        &mut self,
        words: &[String],
        activity: Activity,
        amount: f32,
    ) -> Vec<(String, Activity)> {
        if matches!(activity, Activity::Float | Activity::Visit | Activity::Seek) {
            return Vec::new();
        }
        let learned = self.life.vocabulary.teach(words, activity, amount);
        learned
            .into_iter()
            .map(|word| {
                self.record(EventKind::LearnedWord {
                    word: word.clone(),
                    activity,
                });
                (word, activity)
            })
            .collect()
    }

    // The viewer did something soon after speaking, which may show what
    // their words were about. The sooner, the surer.
    fn shown(&mut self, activity: Activity) {
        let since = self.time - self.listening.at;
        if self.listening.words.is_empty() || self.listening.shown || since >= SHOWN_FOR {
            return;
        }
        self.listening.shown = true;
        let words = self.listening.words.clone();
        self.learn(&words, activity, SHOWN * (1.0 - (since / SHOWN_FOR) as f32));
    }

    fn record(&mut self, kind: EventKind) {
        if matches!(
            kind,
            EventKind::ViewerLeft | EventKind::Scolded | EventKind::FellAsleep
        ) {
            self.cancel_hunt();
        }
        if matches!(
            kind,
            EventKind::Poked { .. }
                | EventKind::Scolded
                | EventKind::ViewerLeft
                | EventKind::FellAsleep
        ) {
            self.stop_show();
        }
        self.note_rhythm(&kind);
        if kind == EventKind::ViewerLeft {
            self.promise_departed = true;
        }
        let quirk_trigger = kind == EventKind::AteMote || quirks::delighted(&kind);
        if kind == EventKind::Thunder {
            self.cure_hiccups(false);
        }
        if kind == EventKind::FellAsleep {
            self.undress_for_bed();
        }
        if matches!(
            kind,
            EventKind::Morning | EventKind::Flowered(_) | EventKind::SnowStarted
        ) {
            self.reconsider_wardrobe();
        }
        if matches!(
            kind,
            EventKind::Poked { .. }
                | EventKind::Scolded
                | EventKind::ViewerLeft
                | EventKind::FellAsleep
        ) {
            self.telling = None;
        }
        if matches!(
            kind,
            EventKind::Poked { .. } | EventKind::Scolded | EventKind::FellAsleep
        ) {
            self.abandon_comfort();
            self.reading = None;
            self.stop_folding();
        }
        if matches!(
            kind,
            EventKind::Scolded | EventKind::ViewerLeft | EventKind::FellAsleep
        ) {
            self.abandon_puzzle();
        }
        let morning = matches!(kind, EventKind::Morning);
        let shooting_star = matches!(kind, EventKind::ShootingStar);
        // Playing along makes the viewer welcome to the next invitation.
        if matches!(kind, EventKind::WordGame { .. })
            || matches!(kind, EventKind::GameOver(outcome) if outcome.finished())
        {
            self.invitations_ignored = 0;
        }
        let granted = self
            .life
            .wish
            .as_ref()
            .filter(|wishing| wishing.wish.granted_by(&kind))
            .map(|wishing| wishing.wish.clone());
        let remembered = self.life.milestones.len();
        self.life.note(&kind);
        let milestones: Vec<String> = self.life.milestones[remembered..]
            .iter()
            .map(|milestone| milestone.text.clone())
            .collect();
        self.events.push(Event {
            time: self.time,
            kind,
        });
        let wish_granted = granted.is_some();
        if let Some(wish) = granted {
            self.life.grant(wish.clone(), &self.date.to_string());
            self.record(EventKind::WishGranted(wish.clone()));
            self.pending_wish = Some(wish);
        }
        for milestone in milestones {
            self.record(EventKind::Milestone(milestone));
        }
        if quirk_trigger {
            self.quirk_after_event();
        }
        if morning {
            self.check_wish();
        }
        if shooting_star && !wish_granted {
            self.wish_on_star();
        }
    }

    // Once it knows the viewer, the ghost leaves whatever it was idly doing
    // to come to the glass when they arrive.
    fn greet(&mut self) {
        if self.focus.is_some() {
            return;
        }
        let idle = matches!(
            self.ghost.plan.activity,
            Activity::Float | Activity::Explore | Activity::Window
        );
        if self.life.bond >= self.life.greeting_bond()
            && self.ghost.asleep < 0.3
            && (idle || self.ghost.plan.done)
        {
            self.begin(Activity::Visit);
        }
    }

    fn wake(&mut self) {
        self.dream = None;
        if self.reading.take().is_some() {
            self.ghost.plan.done = true;
        }
        if self.ghost.keepsake.is_some() {
            self.put_keepsake_away();
            self.ghost.plan.done = true;
        }
        if self.blocks.iter().any(|block| block.held) {
            self.drop_block();
            self.ghost.plan.done = true;
        }
        if self.ball.held {
            self.drop_ball();
            self.ghost.plan.done = true;
        }
        if self.ghost.asleep > 0.3 {
            self.ghost.asleep = 0.0;
            if self.bedtime() {
                self.drowsy_until = self.time + 120.0;
            }
            self.record(EventKind::WokeUp);
        }
        if self.ghost.plan.activity == Activity::Rest {
            self.ghost.plan.done = true;
        }
    }

    fn hear_thunder(&mut self) {
        if self.ghost.asleep > 0.3 {
            // One storm wakes it at most once; after that it sleeps through
            // the rumbling.
            if self.storm_woke == Some(self.weather.since) || self.random.unit() >= 1.0 / 3.0 {
                return;
            }
            self.storm_woke = Some(self.weather.since);
            self.wake();
        }
        self.ghost.velocity += Vec3::new(self.random.range(-0.25, 0.25), 0.3, 0.12);
        if self.viewer_present() && self.life.bond >= GREETS_FROM {
            if !matches!(
                self.ghost.plan.activity,
                Activity::Rest | Activity::Tend | Activity::Fetch
            ) {
                self.begin(Activity::Visit);
            }
        } else if matches!(
            self.ghost.plan.activity,
            Activity::Float | Activity::Explore | Activity::Window
        ) {
            self.ghost.plan.done = true;
        }
    }

    // Lets go of a held ball, which falls to the floor from where it was.
    fn drop_ball(&mut self) {
        self.ball.held = false;
    }

    // Floor circles the ball bounces off and the ghost flies around, with
    // the height they reach.
    fn obstacles(&self) -> Vec<(Vec3, f32)> {
        let mut obstacles = vec![
            (self.lamp.position, Lamp::BASE_RADIUS),
            (self.plant.position, Plant::POT_RADIUS),
            (self.nook.position, Nook::RADIUS),
            (books::POSITION, books::RADIUS),
            (music::POSITION, music::RADIUS),
        ];
        if let Some(decoration) = self.decoration() {
            obstacles.push((Decoration::POSITION, decoration.radius()));
        }
        if !self.seedlings.is_empty() {
            obstacles.extend(
                self.seedlings
                    .iter()
                    .enumerate()
                    .map(|(index, _)| (garden::SPOTS[index], Seedling::POT_RADIUS)),
            );
        }
        obstacles
    }

    fn random_spot(&mut self) -> Vec3 {
        Vec3::new(
            self.random.range(ROOM_MIN.x + 0.5, ROOM_MAX.x - 0.5),
            self.random.range(0.6, ROOM_MAX.y - 0.6),
            self.random.range(ROOM_MIN.z + 0.5, ROOM_MAX.z - 0.5),
        )
    }

    fn hiding_spot(&mut self) -> Vec3 {
        let spots = [
            self.plant.position - Vec3::Z * (Plant::POT_RADIUS + 0.35),
            self.lamp.position - Vec3::Z * (Lamp::BASE_RADIUS + 0.35),
            self.nook.position - Vec3::Z * (Nook::RADIUS + 0.35),
            Vec3::new(ROOM_MIN.x + 0.3, 0.0, ROOM_MIN.z + 0.3),
            Vec3::new(ROOM_MAX.x - 0.3, 0.0, ROOM_MIN.z + 0.3),
        ];
        let index = self.random.range(0.0, spots.len() as f32) as usize;
        (spots[index] + Vec3::Y * 0.55).clamp(
            ROOM_MIN + Vec3::new(0.3, 0.45, 0.3),
            ROOM_MAX - Vec3::new(0.3, 0.35, 0.3),
        )
    }

    fn choose_sketch(&mut self) -> Sketch {
        if self.occasion() == Some(Occasion::Valentine) {
            return Sketch::Heart;
        }
        let mut sketches = Vec::new();
        if self.viewer_present() && self.life.bond >= GREETS_FROM {
            sketches.push(Sketch::Heart);
        }
        match self.weather.sky {
            Sky::Rain => sketches.push(Sketch::Wave),
            Sky::Snow => sketches.push(Sketch::Snowflake),
            Sky::Rainbow => sketches.push(Sketch::Sun),
            Sky::Clear => {}
        }
        match self.ghost.mood {
            Mood::Cheerful | Mood::Excited => sketches.extend([Sketch::Star, Sketch::Flower]),
            Mood::Bored => sketches.push(Sketch::Spiral),
            _ => {}
        }
        if sketches.is_empty() {
            sketches.push(Sketch::Smile);
        }
        sketches[self.random.range(0.0, sketches.len() as f32) as usize]
    }

    // Just inside the glass, where the viewer's pointer is or in the middle.
    fn glass_spot(&self) -> Vec3 {
        let point = self
            .viewer
            .pointer
            .unwrap_or(Vec3::new(0.0, 1.2, ROOM_MAX.z));
        Vec3::new(
            point.x.clamp(ROOM_MIN.x + 0.5, ROOM_MAX.x - 0.5),
            point.y.clamp(0.7, ROOM_MAX.y - 0.6),
            ROOM_MAX.z - 0.45,
        )
    }

    // Toward open floor, away from the walls.
    fn push_direction(&mut self) -> Vec3 {
        let toward = Vec3::new(
            self.random.range(-1.2, 1.2),
            0.0,
            self.random.range(-0.8, 0.8),
        ) - Vec3::new(self.ball.position.x, 0.0, self.ball.position.z);
        toward.normalize_or(Vec3::X)
    }

    // Works the current plan forward: updates its target and does its
    // action on arrival.
    fn carry_out(&mut self, seconds: f32) {
        let time = self.time;
        let position = self.ghost.position;
        if self.hunt_counting() {
            self.ghost.look = Some(position - Vec3::Z * 3.0);
            return;
        }
        if self.ghost.asleep >= 0.3
            && matches!(self.ghost.plan.activity, Activity::Game | Activity::Puzzle)
        {
            self.ghost.plan.done = true;
        }
        let plan = &self.ghost.plan;
        // Only resting keeps the ghost asleep.
        if plan.done || plan.activity != Activity::Rest {
            self.ghost.asleep = (self.ghost.asleep - seconds / 1.2).max(0.0);
        }
        if plan.done {
            self.stop_show();
            self.abandon_comfort();
            self.abandon_puzzle();
            self.abandon_game();
            self.reading = None;
            self.stop_folding();
            self.put_keepsake_away();
            self.drop_ball();
            self.drop_block();
            return;
        }
        let arrived_now = position.distance(plan.target) < ARRIVED;
        if arrived_now && plan.arrived.is_none() {
            self.ghost.plan.arrived = Some(time);
            if self.ghost.plan.activity == Activity::Rest {
                self.sigh_on_cushion();
            }
        }
        // Waiting at the glass keeps its full length. Resting, hiding, and
        // games have their own waits after arrival, but give up on the way
        // there.
        let waiting = self.telling.is_some()
            || matches!(
                self.ghost.plan.activity,
                Activity::Wait
                    | Activity::Company
                    | Activity::Seek
                    | Activity::Show
                    | Activity::Tidy
            )
            || (self.ghost.plan.activity == Activity::Visit && self.asking.is_some())
            || (self.ghost.plan.activity == Activity::Fold && self.ghost.plan.count > 0)
            || (matches!(
                self.ghost.plan.activity,
                Activity::Rest
                    | Activity::Hide
                    | Activity::Game
                    | Activity::Puzzle
                    | Activity::Read
                    | Activity::Music
                    | Activity::Mirror
                    | Activity::Letter
                    | Activity::Comfort
                    | Activity::Fold
                    | Activity::Bubbles
                    | Activity::Pin
                    | Activity::Yard
            ) && self.ghost.plan.arrived.is_some());
        if self.ghost.plan.elapsed(time) > GIVE_UP && !waiting {
            self.put_keepsake_away();
            self.stop_folding();
            self.reading = None;
            self.drop_ball();
            self.drop_block();
            self.ghost.plan.done = true;
            self.abandon_game();
            self.abandon_puzzle();
            return;
        }

        let mut look = None;
        match self.ghost.plan.activity {
            Activity::Letter => look = self.carry_out_letter(),
            Activity::Comfort => look = self.carry_out_comfort(),
            Activity::Mirror => look = self.carry_out_mirror(),
            Activity::Read => look = self.carry_out_read(),
            Activity::Music => look = self.carry_out_music(),
            Activity::Air => look = self.carry_out_air(),
            Activity::Yard => look = self.carry_out_outing(seconds),
            Activity::Company => look = self.keep_company(),
            Activity::Friend => {
                self.play_with_friend();
                look = self.friend.as_ref().map(|friend| friend.position);
            }
            Activity::Look => {
                look = self.study_drawing();
            }
            Activity::Float => {
                let anchor = self.ghost.plan.target;
                let phase = time as f32;
                self.ghost.plan.arrived.get_or_insert(time);
                look = Some(anchor + Vec3::new((phase * 0.3).sin(), 0.0, 1.0));
                if self
                    .sight()
                    .in_view
                    .iter()
                    .any(|seen| seen.thing == Thing::Moth && seen.hidden_behind.is_none())
                {
                    look = self.position_of(Thing::Moth);
                }
                if self.ghost.plan.elapsed(time) > self.ghost.plan.length {
                    self.ghost.plan.done = true;
                }
            }
            Activity::Explore if !self.ghost.plan.gallery_visit => {
                look = Some(self.ghost.plan.target);
                if arrived_now {
                    self.ghost.plan.count += 1;
                    if self.ghost.plan.count >= 3 {
                        self.ghost.plan.done = true;
                    } else {
                        self.ghost.plan.target = self.random_spot();
                    }
                }
            }
            Activity::Chase => {
                let chased = self
                    .ghost
                    .plan
                    .mote
                    .and_then(|id| self.motes.iter().position(|mote| mote.id == id));
                match chased {
                    None => self.ghost.plan.done = true,
                    Some(index) => {
                        let mote = self.motes[index].position;
                        self.ghost.plan.target = mote;
                        look = Some(mote);
                        if self.ghost.plan.count == 1 {
                            if position.distance(self.ghost.plan.anchor) < ARRIVED
                                || self.ghost.plan.stayed(time) > 1.5
                            {
                                self.ghost.plan.count = 2;
                            } else {
                                self.ghost.plan.target = self.ghost.plan.anchor;
                            }
                        } else if position.distance(mote) < 0.22 {
                            if self.ghost.plan.count == 0
                                && self.random.unit()
                                    < growth::miss_chance(self.life.skills.level(Skill::Catching))
                            {
                                self.ghost.plan.count = 1;
                                self.ghost.plan.arrived = Some(time);
                                let direction = self.ghost.velocity.normalize_or(Vec3::X);
                                self.ghost.plan.anchor = (mote + direction * 0.6).clamp(
                                    ROOM_MIN + Vec3::splat(0.5),
                                    ROOM_MAX - Vec3::splat(0.5),
                                );
                                self.ghost.plan.target = self.ghost.plan.anchor;
                                return;
                            }
                            self.motes.remove(index);
                            self.ghost.needs.change(0.03, 0.1, 0.0);
                            self.record(EventKind::AteMote);
                            self.ghost.plan.done = true;
                        }
                    }
                }
            }
            Activity::Rest => {
                self.wind_before_sleep();
                if self.queued_tune.is_some() {
                    return;
                }
                if self.ghost.plan.arrived.is_some() {
                    if self.ghost.plan.count == 0 {
                        self.ghost.plan.count = 1;
                        if self.bedtime()
                            && self.lamp.on
                            && self.ghost.asleep < 0.3
                            && self.ghost.needs.energy > 0.3
                            && self.random.unit() < 0.25
                        {
                            self.begin_reading(true);
                        }
                    }
                    if self.reading.is_some() {
                        self.ghost.look = self.carry_out_read();
                        return;
                    }
                    let before = self.ghost.asleep;
                    let lullaby = self.music.as_ref().is_some_and(|playing| playing.lullaby);
                    if !self
                        .active_quirk()
                        .is_some_and(|spell| spell.kind == Quirk::Sigh)
                    {
                        self.ghost.asleep =
                            (before + seconds / if lullaby { 5.0 } else { 3.0 }).min(1.0);
                    }
                    if before < 0.9 && self.ghost.asleep >= 0.9 {
                        self.record(EventKind::FellAsleep);
                        if lullaby {
                            self.record(EventKind::LullabySleep);
                        }
                    }
                    let overnight = self.ghost.plan.overnight;
                    let rested = if overnight {
                        !self.bedtime() && self.hour < tidy::EVENING_HOUR
                    } else {
                        self.ghost.needs.energy >= 0.98
                    };
                    if rested {
                        self.ghost.plan.done = true;
                        self.ghost.asleep = 0.0;
                        self.record(EventKind::WokeUp);
                        if overnight {
                            self.greet_morning();
                        }
                    }
                }
            }
            Activity::Play => {
                let ball = self.ball.position;
                look = Some(ball);
                let behind = ball - self.push * 0.4 + Vec3::Y * 0.3;
                self.ghost.plan.target = behind;
                if position.distance(behind) < 0.24 && self.ball.velocity.length() < 0.4 {
                    self.ball.velocity += self.push * 1.6;
                    self.ghost.needs.change(-0.004, 0.12, 0.0);
                    self.ghost.plan.count += 1;
                    self.record(EventKind::NudgedBall);
                    if self.ghost.plan.count >= 3 {
                        self.ghost.plan.done = true;
                    } else {
                        self.push = self.push_direction();
                    }
                }
            }
            Activity::Tend => look = self.carry_out_tend(),
            Activity::Lamp => {
                look = Some(self.lamp.bulb());
                if self.ghost.plan.stayed(time) > 0.6 {
                    self.lamp.on = !self.lamp.on;
                    self.record(EventKind::SwitchedLamp { on: self.lamp.on });
                    self.cold_lamp_switched();
                    self.ghost.plan.done = true;
                }
            }
            Activity::Window => {
                look = self.watch_window();
                self.wish_on_moon();
                self.find_constellation();
            }
            Activity::Visit if self.comforting.is_some() => look = self.carry_out_comfort(),
            Activity::Visit => {
                if self
                    .telling
                    .as_ref()
                    .is_some_and(|telling| telling.lighting)
                {
                    self.ghost.look = Some(self.lamp.bulb());
                    return;
                }
                if self.viewer.pointer.is_some() && self.sebae.is_none() {
                    self.ghost.plan.target = self.glass_spot();
                }
                if self.sebae.is_some() {
                    self.bow();
                }
                // Toward the viewer, on the other side of the glass.
                look = Some(self.ghost.plan.target + Vec3::Z * 3.0);
                if self.telling.is_none() && self.ghost.plan.stayed(time) > self.ghost.plan.length {
                    self.ghost.plan.done = true;
                }
            }
            Activity::Wait => {
                if arrived_now {
                    self.ghost.plan.count += 1;
                    let side = if self.ghost.plan.count % 2 == 1 {
                        0.35
                    } else {
                        -0.35
                    };
                    self.ghost.plan.target = self.ghost.plan.anchor + Vec3::X * side;
                }
                look = Some(if self.ghost.plan.elapsed(time) % 20.0 >= 17.0 {
                    WINDOW_CENTER
                } else {
                    self.ghost.plan.target + Vec3::Z * 3.0
                });
                if self.ghost.plan.elapsed(time) >= self.ghost.plan.length
                    || self.viewer_present()
                    || self.bedtime()
                {
                    self.ghost.plan.done = true;
                }
            }
            Activity::Pounce => look = self.pounce(seconds),
            Activity::Game => look = self.play_game(),
            Activity::Puzzle => look = self.play_puzzle(),
            Activity::Build => look = self.build(),
            Activity::Fetch => {
                if self.ghost.plan.count == 0 {
                    let target = self.ball.position + Vec3::Y * 0.42;
                    self.ghost.plan.target = target;
                    look = Some(self.ball.position);
                    if position.distance(target) < ARRIVED && self.ball.velocity.length() < 0.8 {
                        self.ball.held = true;
                        self.ghost.plan.count = 1;
                        self.ghost.plan.arrived = None;
                        self.record(EventKind::PickedUpBall);
                    }
                }
                if self.ghost.plan.count == 1 {
                    if self.ball.held {
                        let mut target = self.glass_spot();
                        target.y = 0.7;
                        self.ghost.plan.target = target;
                        look = Some(target + Vec3::Z * 3.0);
                        if position.distance(target) < ARRIVED {
                            self.drop_ball();
                            let middle = Vec3::new(0.0, Ball::RADIUS, ROOM_MAX.z - Ball::RADIUS);
                            self.ball.velocity =
                                (middle - self.ball.position).normalize_or_zero() * 0.3;
                            self.ghost.plan.count = 2;
                            self.ghost.plan.arrived = Some(time);
                            self.ghost.needs.change(0.0, 0.12, 0.08);
                            self.record(EventKind::Fetched);
                        }
                    } else {
                        self.ghost.plan.done = true;
                    }
                }
                if self.ghost.plan.count >= 2 {
                    look = Some(self.ghost.plan.target + Vec3::Z * 3.0);
                    if self.ghost.plan.stayed(time) > self.ghost.plan.length {
                        self.ghost.plan.done = true;
                    }
                }
            }
            Activity::Dance => {
                let progress = self.ghost.plan.elapsed(time) / self.ghost.plan.length;
                let offset = growth::dance_offset(progress, self.life.skills.level(Skill::Dancing));
                self.ghost.plan.target = self.ghost.plan.anchor + offset;
                look = Some(if self.viewer_present() {
                    self.glass_spot() + Vec3::Z * 3.0
                } else {
                    self.ghost.plan.target
                });
                if progress >= 1.0 {
                    self.ghost.plan.target = self.ghost.plan.anchor;
                    self.ghost.needs.change(0.0, 0.1, 0.0);
                    self.record(EventKind::Danced);
                    self.ghost.plan.done = true;
                }
            }
            Activity::Hide => {
                look = Some(self.glass_spot() + Vec3::Z);
                if self.ghost.plan.stayed(time) > self.ghost.plan.length {
                    self.ghost.plan.target = self.glass_spot();
                    self.ghost.velocity =
                        (self.ghost.plan.target - position).normalize_or_zero() * 1.6;
                    self.say(if self.date.month() == 10 && self.date.day() == 31 {
                        self.spoken("BOO!", "왁!")
                    } else {
                        self.spoken("Boo!", "까꿍!")
                    });
                    self.record(EventKind::Jumped);
                    self.ghost.plan.done = true;
                }
            }
            Activity::Doodle => look = self.carry_out_doodle(),
            Activity::Seek => look = self.seek_treasure(),
            Activity::Fold => look = self.carry_out_fold(seconds),
            Activity::Bubbles => look = self.carry_out_bubbles(),
            Activity::Pin => look = self.carry_out_pin(),
            Activity::Explore => look = self.visit_gallery(),
            Activity::Show => look = self.carry_out_show(),
            Activity::Tidy => look = self.carry_out_tidy(),
        }
        self.ghost.look = look;
        if self.tidy_rest && self.ghost.plan.activity == Activity::Lamp && self.ghost.plan.done {
            self.begin(Activity::Rest);
            self.ghost.plan.overnight = true;
        }
        if self.ghost.plan.activity == Activity::Read && self.ghost.plan.done && self.bedtime() {
            self.begin(Activity::Rest);
            self.ghost.plan.count = 1;
        }
    }

    pub(super) fn carry_out_doodle(&mut self) -> Option<Vec3> {
        let time = self.time;
        let mut look = None;
        if let Some(started) = self.ghost.plan.arrived {
            if self.ghost.plan.count == 0 {
                if self.doodles.len() >= MAX_DOODLES {
                    self.doodles.remove(0);
                }
                self.doodles.push(Doodle {
                    picture: self
                        .ghost
                        .plan
                        .picture
                        .clone()
                        .unwrap_or(Picture::Sketch(Sketch::Smile)),
                    center: self.ghost.plan.anchor.truncate(),
                    size: self.ghost.plan.picture.as_ref().map_or(0.3, Picture::size),
                    started,
                });
                self.ghost.plan.count = 1;
            }
            if let Some(doodle) = self.doodles.last() {
                let progress = doodle.drawn(time);
                let pen = doodle.center + doodle.picture.point_at(progress) * doodle.size;
                look = Some(Vec3::new(pen.x, pen.y, ROOM_MAX.z));
                self.ghost.plan.target = if progress < 1.0 {
                    Vec3::new(pen.x, pen.y, ROOM_MAX.z - 0.3)
                } else {
                    Vec3::new(doodle.center.x, doodle.center.y, ROOM_MAX.z - 0.7)
                };
                if time - started > doodle.picture.draw_time() + 2.0 {
                    let event = match &doodle.picture {
                        Picture::Sketch(sketch) => EventKind::Doodled(sketch.name()),
                        Picture::Copy(copy) => EventKind::Copied(copy.name.clone()),
                        Picture::Word(writing) => EventKind::Wrote {
                            text: writing.text.clone(),
                            mirrored: writing.mirrored,
                            watched: self.viewer_present(),
                        },
                    };
                    let sketch = match doodle.picture {
                        Picture::Sketch(sketch) => Some(sketch),
                        _ => None,
                    };
                    self.record(event);
                    if let Some(sketch) = sketch {
                        self.keep_doodle(sketch);
                    }
                    self.ghost.needs.change(0.0, 0.1, 0.0);
                    self.ghost.plan.done = true;
                }
            }
        } else {
            look = Some(self.ghost.plan.target + Vec3::Z * 0.35);
        }
        look
    }

    // Steers toward the plan's target, around obstacles and away from the
    // walls.
    fn move_ghost(&mut self, seconds: f32) {
        if self.hunt_counting() {
            return;
        }
        let decoration = self.decoration();
        let tower = self.tower.filter(|_| !self.placing_block()).map(|spot| {
            (
                spot,
                blocks::EDGE,
                self.tower_height() as f32 * blocks::EDGE + 0.36,
            )
        });
        let tidy_stacks = [0, 1].map(|index| {
            let height = self.tidy_height(index);
            (height > 0 && !matches!(self.ghost.plan.activity, Activity::Tidy | Activity::Build))
                .then_some((
                    tidy::SPOTS[index],
                    blocks::EDGE,
                    height as f32 * blocks::EDGE + 0.36,
                ))
        });
        let size = self.size();
        // Out in the yard it tiptoes past a settled cat.
        let near_cat = self.visitors.cat.as_ref().is_some_and(|cat| {
            cat.settled(self.time) && cat.position(self.time).distance(self.ghost.position) < 1.8
        });
        let ghost = &mut self.ghost;
        let mut target = ghost.plan.target;
        if ghost.plan.activity == Activity::Float {
            let phase = self.time as f32;
            target += Vec3::new(
                (phase * 0.4).sin() * 0.18,
                (phase * 0.7).sin() * 0.06,
                (phase * 0.33).cos() * 0.12,
            );
        }
        // Out through the door and back in; the yard is past the walls.
        // A place just past a wall, as a push at the ball behind it can
        // aim for, stays in the room.
        let target = if !home::in_house(target) && home::near_house(target, 0.6) {
            target.clamp(ROOM_MIN, ROOM_MAX)
        } else {
            target
        };
        let leaving = home::in_house(ghost.position) != home::in_house(target);
        let target = home::waypoint(ghost.position, target);
        let outdoors = !home::in_house(ghost.position);
        let passing = |position: Vec3| {
            home::in_doorway(position) && (leaving || !home::indoors(position))
        };
        let doorway = passing(ghost.position);
        let speed = match ghost.plan.activity {
            Activity::Chase => 1.3 * growth::chase_speed(self.life.skills.level(Skill::Catching)),
            Activity::Fetch if self.ball.held => {
                0.8 * growth::carrying_speed(self.life.skills.level(Skill::Fetching))
            }
            Activity::Float => 0.35,
            Activity::Wait | Activity::Company => 0.3,
            Activity::Comfort => 0.55,
            Activity::Rest => 0.5,
            Activity::Explore => 0.7,
            Activity::Mirror => 0.45,
            Activity::Dance | Activity::Friend => 1.5,
            Activity::Pounce => 1.6,
            // It creeps up on the cat so as not to frighten it.
            Activity::Yard if ghost.plan.cat && ghost.position.distance(target) < 2.4 => 0.3,
            _ if near_cat => 0.3,
            _ => 0.8,
        } * (0.6 + 0.4 * ghost.needs.energy)
            // It slows down to enjoy being stroked.
            * (1.0 - 0.75 * self.petting);

        let offset = target - ghost.position;
        let distance = offset.length();
        let desired = if distance > 1e-4 {
            offset / distance * speed * (distance / 0.6).min(1.0)
        } else {
            Vec3::ZERO
        };
        let mut steer = (desired - ghost.velocity) * 2.5;

        // Walls push back within a margin, and obstacles push sideways
        // below their tops.
        // The hem hangs about 0.36 below the ghost's center.
        let reaching =
            ghost.plan.activity == Activity::Window && !ghost.plan.done && ghost.plan.count == 1;
        let mut low = ROOM_MIN + Vec3::new(0.3, 0.45 * size, 0.3);
        if reaching {
            low.z = ROOM_MIN.z - 0.04;
        }
        let mut high = ROOM_MAX - Vec3::new(0.3, 0.35, 0.3);
        if (ghost.plan.activity == Activity::Fold && ghost.plan.count > 0)
            || ghost.plan.activity == Activity::Show
        {
            low.z = ROOM_MIN.z + 0.08;
            high.x = ROOM_MAX.x - 0.08;
            high.y = ROOM_MAX.y - 0.06;
        }
        if outdoors {
            (low, high) = home::yard_bounds();
            low.y = 0.45 * size;
        }
        if doorway {
            low.x = f32::NEG_INFINITY;
            high.x = f32::INFINITY;
            low.z = home::DOOR_LOW.x + 0.15;
            high.z = home::DOOR_HIGH.x - 0.15;
            high.y = home::DOOR_HIGH.y - 0.45;
        }
        if outdoors && !doorway {
            // Keep off the cottage's walls.
            let at = flat(ghost.position);
            let nearest = at.clamp(
                Vec2::new(home::HOUSE_MIN.x, home::HOUSE_MIN.z),
                Vec2::new(home::HOUSE_MAX.x, home::HOUSE_MAX.z),
            );
            let away = at - nearest;
            let distance = away.length();
            if distance < 0.4 && distance > 1e-4 {
                let push = away / distance * (0.4 - distance) * 8.0;
                steer += Vec3::new(push.x, 0.0, push.y);
            }
            for (center, radius, top) in home::yard_obstacles() {
                let offset = flat(ghost.position - center);
                let reach = radius + 0.3;
                let distance = offset.length();
                if ghost.position.y < top && distance < reach && distance > 1e-4 {
                    let push = offset / distance * (reach - distance) * 6.0;
                    steer += Vec3::new(push.x, 0.0, push.y);
                }
            }
        }
        for axis in 0..3 {
            if ghost.position[axis] < low[axis] {
                steer[axis] += (low[axis] - ghost.position[axis]) * 8.0;
            } else if ghost.position[axis] > high[axis] {
                steer[axis] -= (ghost.position[axis] - high[axis]) * 8.0;
            }
        }
        let obstacles = [
            (
                self.lamp.position,
                Lamp::BASE_RADIUS + 0.05,
                Lamp::HEIGHT + 0.15,
            ),
            (
                self.plant.position,
                Plant::POT_RADIUS + 0.05,
                self.plant.height() + 0.1,
            ),
            (self.nook.position, Nook::RADIUS, Nook::HEIGHT + 0.2),
            (books::POSITION, books::RADIUS, 0.5),
            (music::POSITION, music::RADIUS, music::HEIGHT + 0.36 * size),
        ];
        let decoration = decoration.map(|decoration| {
            (
                Decoration::POSITION,
                decoration.radius() + 0.05,
                decoration.height() + 0.3,
            )
        });
        for (center, radius, top) in obstacles
            .into_iter()
            .chain(decoration)
            .chain(tower)
            .chain(tidy_stacks.into_iter().flatten())
            .filter(|_| !outdoors)
        {
            let offset = flat(ghost.position - center);
            // Hiding tucks the ghost closer behind furniture than its
            // usual steering allows, especially beside the back wall.
            let reach = if matches!(ghost.plan.activity, Activity::Hide | Activity::Seek) {
                (radius + 0.3).min(flat(target - center).length())
            } else {
                radius + 0.3
            };
            let distance = offset.length();
            if ghost.position.y < top && distance < reach && distance > 1e-4 {
                let push = offset / distance * (reach - distance) * 6.0;
                steer += Vec3::new(push.x, 0.0, push.y);
            }
        }

        for (index, plant) in self.seedlings.iter().enumerate() {
            let center = garden::SPOTS[index];
            let offset = flat(ghost.position - center);
            let reach = Seedling::POT_RADIUS + 0.33;
            let distance = offset.length();
            if ghost.position.y < plant.height() + 0.1 && distance < reach && distance > 1e-4 {
                let push = offset / distance * (reach - distance) * 6.0;
                steer += Vec3::new(push.x, 0.0, push.y);
            }
        }

        let steer = steer.clamp_length_max(3.0);
        ghost.velocity += steer * seconds;
        let before = ghost.position;
        ghost.position += ghost.velocity * seconds;
        let mut minimum = ROOM_MIN + Vec3::new(0.12, 0.38 * size, 0.12);
        if reaching {
            minimum.z = ROOM_MIN.z - 0.04;
        }
        let was_inside = home::in_house(before) && !passing(before);
        if passing(ghost.position) {
            ghost.position.z = ghost
                .position
                .z
                .clamp(home::DOOR_LOW.x + 0.1, home::DOOR_HIGH.x - 0.1);
            ghost.position.y = ghost
                .position
                .y
                .clamp(0.38 * size, home::DOOR_HIGH.y - 0.25);
        } else if was_inside || home::in_house(ghost.position) && passing(before) {
            ghost.position = ghost.position.clamp(minimum, ROOM_MAX - Vec3::splat(0.12));
        } else if home::in_house(ghost.position) {
            // Outside, it can't drift in through a wall.
            ghost.position.x = before.x;
            ghost.position.z = before.z;
            ghost.velocity.x = 0.0;
            ghost.velocity.z = 0.0;
        } else {
            let (low, high) = home::yard_bounds();
            ghost.position = ghost
                .position
                .clamp(Vec3::new(low.x, 0.38 * size, low.z), high);
        }
    }

    fn update_needs(&mut self, seconds: f32) {
        let present = self.viewer_present();
        let nearby_friend = self.friend.as_ref().is_some_and(|friend| {
            friend.state != friend::State::Leaving
                && friend.position.distance(self.ghost.position) < 1.2
        });
        let night = if self.is_night() { 1.4 } else { 1.0 };
        let nature = self.life.temperament();
        let tiring = 0.85 + 0.3 * nature.sleepy;
        let newborn = self.stage() == Stage::Newborn;
        let fresh_air = self.enjoying_air();
        let unwell = self.life.cold.is_some();
        let ghost = &mut self.ghost;
        // A day awake tires it out slowly; motes are snacks, and a nap of
        // a quarter hour or so restores it.
        if ghost.asleep > 0.5 {
            ghost.needs.change(
                0.0006 * seconds / tiring
                    * if newborn { 1.2 } else { 1.0 }
                    * if unwell { 0.9 } else { 1.0 },
                0.0,
                0.03 * self.petting * seconds,
            );
            return;
        }
        let effort = (ghost.velocity.length() / 1.3).min(1.0);
        let mut fun = -0.0008 * (0.8 + 0.4 * nature.playful);
        if fresh_air {
            fun += 0.003;
        }
        let mut company = -0.0006 * (1.15 - 0.3 * nature.shy);
        match ghost.plan.activity {
            Activity::Explore => fun += 0.004,
            Activity::Music if self.music.is_some() => fun += 0.008,
            Activity::Game | Activity::Friend | Activity::Puzzle if !ghost.plan.done => {
                fun += 0.004
            }
            Activity::Visit if present => company += 0.01,
            Activity::Window if ghost.plan.arrived.is_some() => {
                fun += 0.004 + 0.02 * self.weather.rainbow(self.time)
            }
            Activity::Seek if !ghost.plan.done => fun += 0.004,
            Activity::Bubbles if !ghost.plan.done => fun += 0.006,
            _ => {}
        }
        if present {
            company += 0.004;
        }
        if nearby_friend {
            company += 0.002;
        }
        // Being stroked is company, and a little fun while awake.
        company += 0.03 * self.petting;
        fun += 0.01 * self.petting;
        ghost.needs.change(
            -(0.00006 + 0.00014 * effort)
                * night
                * tiring
                * if newborn { 1.3 } else { 1.0 }
                * if unwell { 1.12 } else { 1.0 }
                * seconds,
            fun * seconds,
            company * seconds,
        );
    }

    // How the ghost feels: a mind's suggestion while it holds, or else what
    // its state and the latest events say.
    fn feeling(&self) -> Mood {
        let ghost = &self.ghost;
        if self.taking_break()
            && ghost.asleep <= 0.5
            && let Some((mood, until)) = ghost.suggested_mood
            && self.time < until
        {
            return mood;
        }
        if ghost.asleep > 0.5 || ghost.needs.energy < 0.2 {
            return Mood::Sleepy;
        }
        if self.focus.is_some() {
            return ghost
                .suggested_mood
                .filter(|(_, until)| self.time < *until)
                .map_or(Mood::Content, |(mood, _)| mood);
        }
        // Strokes soothe even a grumpy ghost.
        if self.petting >= PETTED {
            return Mood::Cheerful;
        }
        if let Some(event) = self
            .events
            .iter()
            .rev()
            .take_while(|event| self.time - event.time < 20.0)
            .find(|event| {
                matches!(
                    event.kind,
                    EventKind::FriendArrived(_) | EventKind::FriendLeft(_)
                )
            })
        {
            match event.kind {
                EventKind::FriendArrived(_) => return Mood::Excited,
                EventKind::FriendLeft(_) if !self.viewer_present() => return Mood::Lonely,
                _ => {}
            }
        }
        if let Some((mood, until)) = ghost.suggested_mood
            && self.time < until
        {
            return mood;
        }
        let recent = |seconds: f64| {
            self.events
                .iter()
                .rev()
                .take_while(move |event| self.time - event.time < seconds)
        };
        if recent(10.0).any(|event| matches!(event.kind, EventKind::Poked { count } if count >= 3))
        {
            return Mood::Grumpy;
        }
        if let Some(outcome) = recent(8.0).find_map(|event| match event.kind {
            EventKind::GameOver(outcome) => Some(outcome),
            _ => None,
        }) {
            match outcome {
                Outcome::ViewerWon { .. } => {
                    return if self.good_sport() {
                        Mood::Cheerful
                    } else {
                        Mood::Grumpy
                    };
                }
                Outcome::Draw => return Mood::Curious,
                _ => {}
            }
        }
        if !self.seedlings.is_empty()
            && recent(6.0).any(|event| matches!(event.kind, EventKind::Planted(_)))
            && !recent(6.0)
                .any(|event| matches!(event.kind, EventKind::Sprouted(_) | EventKind::Flowered(_)))
        {
            return Mood::Cheerful;
        }
        if recent(6.0).any(|event| {
            matches!(
                event.kind,
                EventKind::AteMote
                    | EventKind::GameOver(Outcome::GhostWon { .. })
                    | EventKind::WordGame { solved: true, .. }
                    | EventKind::WishGranted(_)
                    | EventKind::NudgedBall
                    | EventKind::Bloomed
                    | EventKind::Sprouted(_)
                    | EventKind::Flowered(_)
                    | EventKind::Fetched
                    | EventKind::Found
                    | EventKind::Pounced
                    | EventKind::Built { .. }
                    | EventKind::Toppled { by_viewer: false }
                    | EventKind::Milestone(_)
                    | EventKind::LearnedWord { .. }
                    | EventKind::Kept(_)
                    | EventKind::ExpectedArrival
            )
                || matches!(&event.kind, EventKind::Noticed(thing) if thing.starts_with("a gift on the sill"))
        }) {
            return Mood::Excited;
        }
        if recent(12.0)
            .any(|event| matches!(event.kind, EventKind::Praised | EventKind::Planted(_)))
            || self.garden_recovering()
            || self.enjoying_air()
        {
            return Mood::Cheerful;
        }
        if recent(12.0).any(|event| event.kind == EventKind::MissedVisit) {
            return Mood::Lonely;
        }
        if self.weather.rainbow(self.time) > 0.3 && ghost.saw_rainbow {
            return Mood::Excited;
        }
        if self.drowsy() {
            return Mood::Sleepy;
        }
        if ghost.asleep <= 0.3 && recent(8.0).any(|event| event.kind == EventKind::Thunder) {
            return if self.viewer_present() {
                Mood::Curious
            } else {
                Mood::Lonely
            };
        }
        if matches!(
            self.occasion(),
            Some(
                Occasion::Birthday
                    | Occasion::ViewerBirthday
                    | Occasion::HundredDays
                    | Occasion::ThousandDays
            )
        ) {
            return Mood::Cheerful;
        }
        if self.weather.snow(self.time) > 0.3 && ghost.saw_snow {
            return Mood::Curious;
        }
        if ghost.needs.company < 0.25 {
            return Mood::Lonely;
        }
        if ghost.needs.fun < 0.25 {
            return Mood::Bored;
        }
        if ghost.needs.energy > 0.6 && ghost.needs.fun > 0.6 && ghost.needs.company > 0.6 {
            return Mood::Cheerful;
        }
        if ghost.plan.activity == Activity::Explore {
            return Mood::Curious;
        }
        Mood::Content
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_life_gathers_itself_out_of_light_once() {
        let mut world = World::new(3, 12.0);
        let motes = world.motes.len();
        world.begin_life("2026-06-21");
        assert_eq!(world.ghost.fade, 1.0);
        assert_eq!(world.motes.len(), motes + BIRTH_MOTES);
        world.step(2.0);
        assert!(
            (0.1..0.9).contains(&world.ghost.fade),
            "{}",
            world.ghost.fade
        );
        world.step(BIRTH_SECONDS as f32);
        assert!(world.ghost.fade < 0.01);
        // Coming back to a life already begun is no birth.
        let mut again = World::new(3, 12.0);
        again.life = world.life.clone();
        again.begin_life("2026-06-22");
        assert_eq!(again.ghost.fade, 0.0);
        assert_eq!(again.born_at, None);
    }

    #[test]
    fn every_activity_finishes_on_its_own_for_a_newborn_too() {
        for activity in Activity::ALL {
            let mut world = World::new(9, 12.0);
            world.birthday = Some(world.date);
            world.drop_mote(Vec3::new(1.0, 1.0, 0.0));
            world.ghost.needs.energy = 0.5;
            world.begin(activity);
            let duration = world.ghost.plan.length.max(nap_or_minute(activity)) + 10.0;
            run(&mut world, duration);
            assert!(
                world.needs_decision(),
                "{activity:?} never finished for a newborn"
            );
        }
    }

    #[test]
    fn a_newborn_tires_faster_and_recovers_faster_in_sleep() {
        for asleep in [0.0, 1.0] {
            let mut grown = World::new(1, 12.0);
            grown.ghost.needs.energy = 0.5;
            grown.ghost.asleep = asleep;
            let mut newborn = grown.clone();
            newborn.birthday = Some(newborn.date);
            grown.update_needs(1.0);
            newborn.update_needs(1.0);
            let ratio = (newborn.ghost.needs.energy - 0.5) / (grown.ghost.needs.energy - 0.5);
            assert!((ratio - if asleep > 0.5 { 1.2 } else { 1.3 }).abs() < 1e-4);
        }
    }

    #[test]
    fn a_missed_first_pass_circles_back_and_still_catches_the_mote() {
        let mut missed = false;
        for seed in 0..40 {
            let mut world = World::new(seed, 12.0);
            world.drop_mote(world.ghost.position);
            world.begin(Activity::Chase);
            world.carry_out(0.05);
            if world.ghost.plan.count == 1 {
                let position = world.ghost.position;
                world.step(0.5);
                assert!(world.ghost.position.distance(position) > 0.01);
                assert_eq!(world.life.total.motes, 0);
                world.step(20.0);
                assert_eq!(world.life.total.motes, 1);
                assert!(world.needs_decision());
                missed = true;
                break;
            }
        }
        assert!(missed);
    }

    #[test]
    fn growing_ghosts_and_their_practice_repeat_for_a_seed() {
        let mut first = World::new(31, 12.0);
        first.birthday = Some(first.date);
        let mut second = first.clone();
        for activity in [
            Activity::Build,
            Activity::Chase,
            Activity::Dance,
            Activity::Doodle,
        ] {
            for world in [&mut first, &mut second] {
                world.ghost.needs.energy = 1.0;
                world.drop_mote(Vec3::new(0.0, 1.2, 0.0));
                world.begin(activity);
                if activity == Activity::Doodle {
                    world.write("HI");
                }
                world.step(50.0);
            }
            assert_eq!(first.ghost.position, second.ghost.position);
            assert_eq!(first.ghost.plan, second.ghost.plan);
            assert_eq!(first.life, second.life);
            assert_eq!(first.doodles, second.doodles);
            assert_eq!(first.events, second.events);
        }
    }

    #[test]
    fn seeded_natures_are_repeatable_and_a_begun_life_keeps_its_nature() {
        let mut world = World::new(7, 12.0);
        let nature = world.life.nature;
        assert!(nature.is_some());
        assert_eq!(nature, World::new(7, 21.0).life.nature);
        assert_ne!(nature, World::new(8, 12.0).life.nature);
        world.begin_life("2026-10-21");
        assert_eq!(world.life.nature, nature);
        let mut missing = world.clone();
        missing.life.nature = None;
        let mut same = missing.clone();
        missing.begin_life("2026-10-21");
        same.begin_life("2026-10-21");
        assert!(missing.life.nature.is_some());
        assert_eq!(missing.life.nature, same.life.nature);
        let nature = missing.life.nature;
        missing.begin_life("2026-10-22");
        assert_eq!(missing.life.nature, nature);
    }

    #[test]
    fn nature_tints_the_pace_of_needs_and_sleep() {
        let mut average = World::new(1, 12.0);
        average.life.nature = Some(Nature::default());
        let mut pronounced = average.clone();
        pronounced.life.nature = Some(Nature {
            playful: 1.0,
            shy: 1.0,
            sleepy: 1.0,
            ..Nature::default()
        });
        average.update_needs(10.0);
        pronounced.update_needs(10.0);
        assert!(pronounced.ghost.needs.fun < average.ghost.needs.fun);
        assert!(pronounced.ghost.needs.energy < average.ghost.needs.energy);
        assert!(pronounced.ghost.needs.company > average.ghost.needs.company);
        for world in [&mut average, &mut pronounced] {
            world.ghost.asleep = 1.0;
            world.ghost.needs.energy = 0.5;
            world.update_needs(10.0);
        }
        assert!(pronounced.ghost.needs.energy < average.ghost.needs.energy);
    }

    #[test]
    fn a_shy_ghost_takes_longer_to_greet_but_still_greets_a_close_friend() {
        for (shyness, greets) in [(0.0, true), (1.0, false)] {
            let mut world = World::new(1, 12.0);
            world.life.nature = Some(Nature {
                shy: shyness,
                ..Nature::default()
            });
            world.life.bond = GREETS_FROM;
            world.greet();
            assert_eq!(world.ghost.plan.activity == Activity::Visit, greets);
            world.life.bond = 0.9;
            world.greet();
            assert_eq!(world.ghost.plan.activity, Activity::Visit);
        }
    }

    fn before_thunder(seed: u64) -> World {
        let mut world = World::new(seed, 12.0);
        world.time = 40.0;
        world.weather.sky = Sky::Rain;
        world.weather.storm = true;
        world.weather.until = 300.0;
        world.weather.thunder_at = Some(40.05);
        world
    }

    #[test]
    fn thunder_draws_the_ghost_to_a_trusted_viewer_at_the_glass() {
        for bond in [GREETS_FROM, 0.9] {
            let mut world = before_thunder(1);
            world.notice_viewer();
            world.life.bond = bond;
            world.begin(Activity::Play);
            world.step(0.1);
            assert_eq!(world.ghost.plan.activity, Activity::Visit);
            assert_eq!(world.ghost.plan.target, world.glass_spot());
            assert!(
                world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::Thunder)
            );
            assert_eq!(world.ghost.mood, Mood::Curious);
        }
    }

    #[test]
    fn thunder_ends_idle_plans_without_sending_a_stranger_or_an_absent_friend_to_the_glass() {
        for (bond, present) in [(0.0, true), (0.9, false)] {
            for activity in [
                Activity::Float,
                Activity::Explore,
                Activity::Window,
                Activity::Play,
            ] {
                let mut world = before_thunder(1);
                if present {
                    world.notice_viewer();
                }
                world.life.bond = bond;
                world.begin(activity);
                let velocity = world.ghost.velocity;
                world.step(0.1);
                assert_eq!(world.ghost.plan.activity, activity);
                assert_eq!(world.ghost.plan.done, activity != Activity::Play);
                assert!(world.ghost.velocity.y > velocity.y);
                assert_eq!(
                    world.ghost.mood,
                    if present { Mood::Curious } else { Mood::Lonely }
                );
            }
        }
    }

    #[test]
    fn thunder_leaves_resting_tending_and_fetching_alone() {
        for activity in [Activity::Rest, Activity::Tend, Activity::Fetch] {
            let mut world = before_thunder(1);
            world.notice_viewer();
            world.life.bond = 0.9;
            world.begin(activity);
            world.step(0.1);
            assert_eq!(world.ghost.plan.activity, activity);
            assert!(!world.ghost.plan.done);
        }
    }

    #[test]
    fn thunder_sometimes_wakes_a_sleeping_ghost_and_the_seed_decides() {
        let mut awakened = 0;
        for seed in 0..150 {
            let mut world = before_thunder(seed);
            world.begin(Activity::Rest);
            world.ghost.position = world.nook.bed(world.size());
            world.ghost.plan.arrived = Some(world.time);
            world.ghost.asleep = 1.0;
            world.ghost.needs.energy = 0.5;
            let mut same = world.clone();
            world.step(0.1);
            same.step(0.1);
            assert_eq!(world.events, same.events);
            assert_eq!(world.ghost.asleep, same.ghost.asleep);
            assert_eq!(world.ghost.velocity, same.ghost.velocity);
            let woke = world
                .events
                .iter()
                .any(|event| event.kind == EventKind::WokeUp);
            assert_eq!(woke, world.ghost.asleep == 0.0);
            if woke {
                assert!(world.ghost.plan.done);
                awakened += 1;
            } else {
                assert_eq!(world.ghost.asleep, 1.0);
            }
        }
        assert!((30..70).contains(&awakened), "{awakened}");
    }

    #[test]
    fn a_storm_wakes_a_sleeper_at_most_once() {
        for seed in 0..40 {
            let mut world = before_thunder(seed);
            world.begin(Activity::Rest);
            world.ghost.position = world.nook.bed(world.size());
            world.ghost.plan.arrived = Some(world.time);
            world.ghost.asleep = 1.0;
            world.ghost.needs.energy = 0.3;
            let mut wakings = 0;
            for _ in 0..30 {
                world.weather.thunder_at = Some(world.time + 0.05);
                world.step(0.1);
                wakings += usize::from(world.ghost.asleep == 0.0);
                if world.ghost.asleep == 0.0 {
                    world.begin(Activity::Rest);
                    world.ghost.position = world.nook.bed(world.size());
                    world.ghost.plan.arrived = Some(world.time);
                    world.ghost.asleep = 1.0;
                }
            }
            assert!(wakings <= 1, "seed {seed}: {wakings}");
        }
    }

    #[test]
    fn thunder_feelings_fade_and_yield_to_sleep_annoyance_and_delight() {
        let mut world = before_thunder(1);
        world.record(EventKind::Thunder);
        assert_eq!(world.feeling(), Mood::Lonely);
        world.notice_viewer();
        assert_eq!(world.feeling(), Mood::Curious);
        world.time += 8.0;
        assert_ne!(world.feeling(), Mood::Curious);
        world.record(EventKind::Thunder);
        world.record(EventKind::AteMote);
        assert_eq!(world.feeling(), Mood::Excited);
        world.record(EventKind::Poked { count: 3 });
        assert_eq!(world.feeling(), Mood::Grumpy);
        world.ghost.needs.energy = 0.1;
        assert_eq!(world.feeling(), Mood::Sleepy);
    }

    #[test]
    fn storm_starts_are_recorded_instead_of_plain_rain_and_stop_at_winter() {
        let mut storms = 0;
        for seed in 0..100 {
            let mut world = World::new(seed, 12.0);
            world.weather.until = 0.05;
            world.step(0.1);
            if world.weather.storm {
                storms += 1;
                assert_eq!(world.life.today.storms, 1);
                assert_eq!(world.life.total.storms, 1);
                assert_eq!(world.life.today.showers, 0);
                assert!(
                    world
                        .events
                        .iter()
                        .any(|event| event.kind == EventKind::StormStarted)
                );
                assert!(
                    !world
                        .events
                        .iter()
                        .any(|event| event.kind == EventKind::RainStarted)
                );
                world.date = "2026-12-21".parse().unwrap();
                world.step(0.1);
                assert_eq!(world.weather.sky, Sky::Snow);
                assert!(!world.weather.storm);
                assert_eq!(world.weather.flash(world.time), 0.0);
            }
        }
        assert!(storms > 10);
    }

    #[test]
    fn each_lightning_strike_reaches_the_world_as_one_delayed_thunder_event() {
        for seed in 0..10 {
            let mut world = World::new(seed, 12.0);
            world.weather.sky = Sky::Rain;
            world.weather.storm = true;
            world.weather.until = 200.0;
            let mut strikes = Vec::new();
            while world.time < 205.0 {
                let last_flash = world.weather.last_flash;
                world.step(0.1);
                if world.weather.last_flash.is_finite() && world.weather.last_flash != last_flash {
                    assert!(world.weather.rain(world.time) > 0.6);
                    strikes.push((world.weather.last_flash, world.weather.thunder_at.unwrap()));
                }
            }
            let thunder: Vec<_> = world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Thunder)
                .collect();
            assert!(strikes.len() > 4);
            assert_eq!(thunder.len(), strikes.len());
            for (event, (strike, due)) in thunder.into_iter().zip(strikes) {
                assert!((1.0..=4.0).contains(&(due - strike)));
                assert!((due..=due + 0.1).contains(&event.time));
            }
            assert!(!world.weather.storm);
            assert_eq!(world.weather.flash(world.time), 0.0);
        }
    }

    fn run(world: &mut World, seconds: f32) {
        for _ in 0..(seconds * 20.0) as usize {
            world.step(0.05);
        }
    }

    fn inside(point: Vec3) -> bool {
        point.cmpge(ROOM_MIN).all() && point.cmple(ROOM_MAX).all()
    }

    // In the room, passing through the door, or out on the lawn.
    fn at_home(point: Vec3) -> bool {
        let (low, high) = home::yard_bounds();
        inside(point)
            || home::in_doorway(point)
            || !home::in_house(point) && point.cmpge(low).all() && point.cmple(high).all()
    }

    #[test]
    fn the_ghost_stays_at_home_whatever_it_does() {
        for activity in Activity::ALL {
            let mut world = World::new(5, 12.0);
            world.drop_mote(Vec3::new(1.0, 1.0, 0.0));
            world.notice_viewer();
            world.ghost.needs.energy = 0.5;
            world.begin(activity);
            for _ in 0..1400 {
                world.step(0.05);
                assert!(
                    at_home(world.ghost.position),
                    "{activity:?}: {:?}",
                    world.ghost.position
                );
                if !matches!(activity, Activity::Yard | Activity::Tend | Activity::Show) {
                    assert!(
                        home::in_house(world.ghost.position),
                        "{activity:?}: {:?}",
                        world.ghost.position
                    );
                }
                assert!(inside(world.ball.position));
            }
        }
        let mut world = World::new(5, 12.0);
        for round in 0..200 {
            if world.needs_decision() {
                let options = world.options();
                world.begin(options[round % options.len()]);
            }
            run(&mut world, 1.5);
            assert!(at_home(world.ghost.position), "{:?}", world.ghost.position);
            assert!(inside(world.ball.position));
        }
    }

    // A nap from half rested takes a quarter hour or so; everything else
    // ends within a minute.
    fn nap_or_minute(activity: Activity) -> f32 {
        if activity == Activity::Rest {
            1200.0
        } else {
            60.0
        }
    }

    #[test]
    fn every_activity_finishes_on_its_own() {
        for activity in Activity::ALL {
            let mut world = World::new(9, 12.0);
            world.drop_mote(Vec3::new(1.0, 1.0, 0.0));
            world.ghost.needs.energy = 0.5;
            world.begin(activity);
            let duration = world.ghost.plan.length.max(nap_or_minute(activity)) + 10.0;
            run(&mut world, duration);
            assert!(world.needs_decision(), "{activity:?} never finished");
        }
    }

    #[test]
    fn chasing_a_mote_eats_it_for_energy() {
        let mut world = World::new(2, 12.0);
        world.drop_mote(Vec3::new(-1.0, 1.4, -0.5));
        let dropped = world.motes[0].id;
        world.ghost.needs.energy = 0.4;
        world.begin(Activity::Chase);
        run(&mut world, 6.0);
        assert!(world.motes.iter().all(|mote| mote.id != dropped));
        assert!(world.ghost.needs.energy > 0.42);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::AteMote)
        );
    }

    #[test]
    fn resting_puts_the_ghost_to_sleep_until_rested() {
        let mut world = World::new(4, 14.0);
        world.ghost.needs.energy = 0.3;
        world.begin(Activity::Rest);
        run(&mut world, 20.0);
        assert!(world.ghost.asleep > 0.9);
        assert_eq!(world.ghost.mood, Mood::Sleepy);
        run(&mut world, 600.0);
        assert!(!world.needs_decision(), "a nap lasts a while");
        run(&mut world, 1100.0);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::WokeUp)
        );
        assert!(world.ghost.needs.energy > 0.9);
        assert!(world.needs_decision());
        assert_eq!(world.ghost.asleep, 0.0);
    }

    #[test]
    fn a_rest_that_never_reaches_the_cushion_is_given_up() {
        let mut world = World::new(4, 23.0);
        world.ghost.needs.energy = 0.1;
        world.begin(Activity::Rest);
        // Somewhere the ghost can never get to.
        world.ghost.plan.target = Vec3::new(0.0, -5.0, 0.0);
        run(&mut world, 65.0);
        assert!(world.needs_decision());
    }

    #[test]
    fn a_poke_wakes_a_sleeping_ghost_and_poking_on_makes_it_grumpy() {
        let mut world = World::new(4, 23.0);
        world.ghost.needs.energy = 0.3;
        world.begin(Activity::Rest);
        run(&mut world, 20.0);
        world.poke();
        assert_eq!(world.ghost.asleep, 0.0);
        assert!(world.needs_decision());
        world.poke();
        world.poke();
        assert!(matches!(
            world.events.last().unwrap().kind,
            EventKind::Poked { count: 3 }
        ));
        world.step(0.05);
        assert_eq!(world.ghost.mood, Mood::Grumpy);
    }

    #[test]
    fn strokes_soothe_and_please_the_ghost_and_count_once_per_spell() {
        let mut world = World::new(3, 12.0);
        world.ghost.needs.company = 0.3;
        for _ in 0..3 {
            world.poke();
        }
        assert_eq!(world.feeling(), Mood::Grumpy);
        world.pet(0.1);
        assert!(world.petting() < PETTED);
        world.pet(0.1);
        world.pet(0.1);
        assert_eq!(world.feeling(), Mood::Cheerful);
        let company = world.ghost.needs.company;
        run(&mut world, 1.0);
        world.pet(0.1);
        assert!(world.ghost.needs.company > company);
        let pets = |world: &World| {
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Petted)
                .count()
        };
        assert_eq!(pets(&world), 1);
        assert_eq!(world.life.total.pets, 1);
        assert!(world.events.iter().any(|event| {
            event.kind == EventKind::Milestone("You stroked me for the first time.".to_owned())
        }));

        // The petting fades once the strokes stop, and a new spell counts
        // again.
        run(&mut world, 6.0);
        assert!(world.petting() < 0.05);
        world.pet(0.4);
        assert_eq!(pets(&world), 2);
    }

    #[test]
    fn strokes_leave_a_sleeper_asleep_and_find_a_hidden_ghost() {
        let mut world = World::new(3, 12.0);
        world.ghost.needs.energy = 0.2;
        world.begin(Activity::Rest);
        run(&mut world, 30.0);
        assert!(world.ghost.asleep > 0.9);
        world.pet(0.5);
        run(&mut world, 0.5);
        assert!(world.ghost.asleep > 0.9);
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert!(!world.ghost.plan.done);

        let mut world = World::new(3, 12.0);
        world.notice_viewer();
        world.begin(Activity::Hide);
        world.ghost.plan.arrived = Some(world.time);
        world.ghost.fade = 0.85;
        world.pet(0.2);
        assert!(world.ghost.plan.done);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Found)
        );
        assert_eq!(world.petting(), 0.0);
    }

    #[test]
    fn playing_sends_the_ball_rolling() {
        let mut world = World::new(6, 12.0);
        let start = world.ball.position;
        world.begin(Activity::Play);
        run(&mut world, 30.0);
        assert!(world.ball.position.distance(start) > 0.3);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::NudgedBall)
        );
    }

    #[test]
    fn a_throw_ends_a_quiet_plan_and_opens_fetch() {
        for activity in [
            Activity::Float,
            Activity::Explore,
            Activity::Window,
            Activity::Visit,
        ] {
            let mut world = World::new(6, 12.0);
            assert!(!world.options().contains(&Activity::Fetch));
            world.begin(activity);
            world.kick_ball(-Vec3::Z);
            assert!(world.needs_decision(), "{activity:?}");
            assert!(world.options().contains(&Activity::Fetch));
        }
    }

    #[test]
    fn a_throw_leaves_busy_or_sleeping_ghosts_to_their_plans() {
        for activity in [
            Activity::Rest,
            Activity::Tend,
            Activity::Lamp,
            Activity::Chase,
            Activity::Play,
            Activity::Dance,
            Activity::Fetch,
        ] {
            let mut world = World::new(6, 12.0);
            world.begin(activity);
            world.kick_ball(Vec3::X);
            assert!(!world.needs_decision(), "{activity:?}");
        }
        let mut world = World::new(6, 12.0);
        world.ghost.asleep = 0.8;
        world.kick_ball(Vec3::X);
        assert!(!world.needs_decision());
        assert!(!world.options().contains(&Activity::Fetch));
    }

    #[test]
    fn a_viewer_can_ask_for_fetch_without_throwing_first() {
        let mut world = World::new(6, 12.0);
        assert!(!world.options().contains(&Activity::Fetch));
        world.notice_viewer();
        assert!(world.options().contains(&Activity::Fetch));
        world.ghost.needs.energy = 0.2;
        assert!(!world.options().contains(&Activity::Fetch));
    }

    #[test]
    fn fetch_needs_energy_and_a_recent_throw_or_a_ball_still_rolling_after_one() {
        let mut world = World::new(6, 12.0);
        world.ball.velocity = Vec3::X;
        assert!(!world.options().contains(&Activity::Fetch));
        world.kick_ball(Vec3::X);
        world.ghost.needs.energy = 0.3;
        assert!(!world.options().contains(&Activity::Fetch));
        world.ghost.needs.energy = 0.8;
        world.time += 21.0;
        assert!(world.options().contains(&Activity::Fetch));
        world.ball.velocity = Vec3::ZERO;
        assert!(!world.options().contains(&Activity::Fetch));
    }

    #[test]
    fn fetching_picks_up_the_ball_and_returns_it_before_waiting_for_another_throw() {
        for direction in [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z] {
            let mut world = World::new(6, 12.0);
            world.notice_viewer();
            world.kick_ball(direction);
            world.begin(Activity::Fetch);
            let mut carried = false;
            let fun = world.ghost.needs.fun;
            let company = world.ghost.needs.company;
            for _ in 0..1200 {
                world.step(0.05);
                assert!(
                    world
                        .ball
                        .position
                        .cmpge(ROOM_MIN + Vec3::splat(Ball::RADIUS))
                        .all()
                );
                assert!(
                    world
                        .ball
                        .position
                        .cmple(ROOM_MAX - Vec3::splat(Ball::RADIUS))
                        .all()
                );
                if world.ball.held {
                    carried = true;
                    let height = (world.ghost.position.y - 0.42).max(Ball::RADIUS);
                    assert!((world.ball.position.y - height).abs() < 1e-5);
                    assert_eq!(world.ball.velocity, world.ghost.velocity);
                }
                if world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::Fetched)
                {
                    break;
                }
            }
            assert!(carried, "{direction:?}");
            let actions: Vec<_> = world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::PickedUpBall | EventKind::Fetched))
                .map(|event| event.kind.clone())
                .collect();
            assert_eq!(
                actions,
                [EventKind::PickedUpBall, EventKind::Fetched],
                "{direction:?}"
            );
            assert!(!world.ball.held);
            assert!(world.ball.position.z > ROOM_MAX.z - 0.75);
            assert!(world.ghost.needs.fun > fun);
            assert!(world.ghost.needs.company > company);
            assert_eq!(world.ghost.mood, Mood::Excited);
            assert!(!world.needs_decision());
            assert!(world.ghost.look.unwrap().z > ROOM_MAX.z);
            run(&mut world, 5.9);
            assert!(!world.needs_decision());
            run(&mut world, 0.2);
            assert!(world.needs_decision());
        }
    }

    #[test]
    fn another_throw_ends_the_wait_after_a_fetch() {
        let mut world = World::new(6, 12.0);
        world.begin(Activity::Fetch);
        for _ in 0..600 {
            world.step(0.05);
            if world.ghost.plan.count == 2 {
                break;
            }
        }
        assert_eq!(world.ghost.plan.count, 2);
        assert!(!world.needs_decision());
        world.kick_ball(-Vec3::Z);
        assert!(world.needs_decision());
        assert!(world.options().contains(&Activity::Fetch));
    }

    #[test]
    fn changing_activity_or_waking_or_poking_lets_go_of_a_held_ball() {
        for interruption in 0..3 {
            let mut world = World::new(6, 12.0);
            world.begin(Activity::Fetch);
            for _ in 0..600 {
                world.step(0.05);
                if world.ball.held {
                    break;
                }
            }
            assert!(world.ball.held);
            let velocity = world.ball.velocity;
            match interruption {
                0 => world.begin(Activity::Explore),
                1 => world.wake(),
                _ => world.poke(),
            }
            assert!(!world.ball.held);
            assert_eq!(world.ball.velocity, velocity);
            run(&mut world, 1.0);
            assert!(!world.ball.held);
            assert_eq!(world.ball.position.y, Ball::RADIUS);
        }
    }

    #[test]
    fn giving_up_on_a_fetch_releases_the_ball() {
        let mut world = World::new(6, 12.0);
        world.begin(Activity::Fetch);
        for _ in 0..600 {
            world.step(0.05);
            if world.ball.held {
                break;
            }
        }
        assert!(world.ball.held);
        world.ghost.plan.started = world.time - f64::from(GIVE_UP);
        world.step(0.1);
        assert!(world.needs_decision());
        assert!(!world.ball.held);
        run(&mut world, 1.0);
        assert_eq!(world.ball.position.y, Ball::RADIUS);
        assert!(
            !world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Fetched)
        );
    }

    #[test]
    fn a_dance_moves_through_the_air_and_finishes_with_a_twirl() {
        for position in [Vec3::new(0.0, 1.2, 0.0), ROOM_MIN, ROOM_MAX] {
            let mut world = World::new(6, 12.0);
            world.ghost.position = position;
            world.begin(Activity::Dance);
            let anchor = world.ghost.plan.anchor;
            assert!(anchor.x >= ROOM_MIN.x + 0.6 && anchor.x <= ROOM_MAX.x - 0.6);
            assert!(anchor.z >= ROOM_MIN.z + 0.6 && anchor.z <= ROOM_MAX.z - 0.6);
            assert!((0.9..=1.6).contains(&anchor.y));
            assert!((5.0..7.0).contains(&world.ghost.plan.length));
            let mut lowest = f32::INFINITY;
            let mut highest = f32::NEG_INFINITY;
            let fun = world.ghost.needs.fun;
            for _ in 0..160 {
                world.step(0.05);
                assert!(inside(world.ghost.position));
                lowest = lowest.min(world.ghost.position.y);
                highest = highest.max(world.ghost.position.y);
                if world.needs_decision() {
                    break;
                }
            }
            assert!(highest - lowest > 0.25, "{lowest}..{highest}");
            assert!(world.needs_decision());
            assert!(world.time < 8.0);
            assert!(world.ghost.needs.fun > fun);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| event.kind == EventKind::Danced)
                    .count(),
                1
            );
            run(&mut world, 2.0);
            assert_eq!(
                world
                    .events
                    .iter()
                    .filter(|event| event.kind == EventKind::Danced)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn invitations_let_pass_in_a_row_wait_longer_each_time_until_the_viewer_plays() {
        let mut world = World::new(6, 12.0);
        world.ghost.needs.energy = 0.9;
        let welcome = |world: &mut World| {
            world.notice_viewer();
            world.options().contains(&Activity::Game)
        };
        assert!(welcome(&mut world));
        world.invitation_ignored();
        assert!(!welcome(&mut world));
        world.time += IGNORED_PAUSE;
        assert!(welcome(&mut world));
        world.invitation_ignored();
        world.time += IGNORED_PAUSE;
        assert!(
            !welcome(&mut world),
            "the second in a row waits twice as long"
        );
        world.time += IGNORED_PAUSE;
        assert!(welcome(&mut world));
        for _ in 0..12 {
            world.invitation_ignored();
        }
        world.time += IGNORED_PAUSE_MOST;
        assert!(welcome(&mut world), "the wait never grows past its cap");
        world.record(EventKind::GameOver(Outcome::Draw));
        world.invitation_ignored();
        world.time += IGNORED_PAUSE;
        assert!(welcome(&mut world), "playing along starts the count over");
    }

    #[test]
    fn a_few_quiet_minutes_end_no_visit_and_coming_back_from_them_is_no_arrival() {
        let mut world = World::new(6, 12.0);
        let count = |world: &World, kind: EventKind| {
            world
                .events
                .iter()
                .filter(|event| event.kind == kind)
                .count()
        };
        world.notice_viewer();
        world.step(0.1);
        assert_eq!(count(&world, EventKind::ViewerArrived), 1);
        world.step((PRESENT_FOR + 60.0) as f32);
        assert!(!world.viewer_present());
        world.notice_viewer();
        world.step(0.1);
        assert_eq!(count(&world, EventKind::ViewerArrived), 1);
        assert_eq!(count(&world, EventKind::ViewerLeft), 0);
        world.step((PRESENT_FOR + GONE_AFTER) as f32);
        assert_eq!(count(&world, EventKind::ViewerLeft), 1);
        world.notice_viewer();
        world.step(0.1);
        assert_eq!(count(&world, EventKind::ViewerArrived), 2);
    }

    #[test]
    fn dancing_needs_energy_and_turns_toward_a_present_viewer() {
        let mut world = World::new(6, 12.0);
        world.ghost.needs.energy = 0.35;
        assert!(!world.options().contains(&Activity::Dance));
        world.ghost.needs.energy = 0.8;
        assert!(world.options().contains(&Activity::Dance));
        world.ghost.asleep = 0.8;
        assert!(!world.options().contains(&Activity::Dance));
        world.ghost.asleep = 0.0;
        world.notice_viewer();
        world.begin(Activity::Dance);
        world.step(0.1);
        assert!(world.ghost.look.unwrap().z > ROOM_MAX.z);
        world.time += PRESENT_FOR;
        world.ghost.plan.started = world.time;
        world.step(0.1);
        assert_eq!(world.ghost.look, Some(world.ghost.plan.target));
    }

    #[test]
    fn tending_waters_the_plant_and_helps_it_grow() {
        let mut world = World::new(6, 12.0);
        world.plant.thirst = 0.8;
        world.plant.growth = 0.78;
        world.begin(Activity::Tend);
        run(&mut world, 30.0);
        assert!(world.plant.thirst < 0.1);
        assert!(world.plant.growth >= Plant::BLOOM);
        assert!(!world.plant.blooming());
        assert!(
            !world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Bloomed)
        );
    }

    #[test]
    fn hearing_the_viewer_brings_the_ghost_to_the_glass() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("hello");
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        run(&mut world, 8.0);
        assert!(world.ghost.position.z > ROOM_MAX.z - 0.8);
    }

    fn kinds(world: &World, since: f64) -> Vec<EventKind> {
        world
            .events
            .iter()
            .filter(|event| event.time >= since)
            .map(|event| event.kind.clone())
            .collect()
    }

    #[test]
    fn a_question_brings_the_ghost_to_the_glass_without_teaching_or_praising() {
        let mut world = World::new(1, 12.0);
        world.notice_viewer();
        world.begin(Activity::Play);
        let heard = world.hear("do you love me? what are you doing?");
        assert_eq!(
            heard.understanding,
            Understanding::Asked {
                question: Question::Liking,
                doing: Some(Activity::Play),
            }
        );
        assert_eq!(heard.tone, None);
        assert!(heard.learned.is_empty());
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(world.life.vocabulary.words.is_empty());
        assert!(
            !world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::Praised | EventKind::Puzzled(_)))
        );
    }

    #[test]
    fn a_new_word_puzzles_the_ghost_and_brings_it_to_the_glass() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        let heard = world.hear("Boo, dance!");
        assert_eq!(
            heard.understanding,
            Understanding::Puzzled {
                word: "dance".to_owned(),
                again: false
            }
        );
        assert!(heard.named);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(kinds(&world, 0.0).contains(&EventKind::Puzzled("dance".to_owned())));
        // Heard again, the word is one it means to work out.
        run(&mut world, 2.0);
        let heard = world.hear("dance");
        assert!(matches!(
            heard.understanding,
            Understanding::Puzzled { again: true, .. }
        ));
    }

    #[test]
    fn a_guess_praised_teaches_the_word_and_then_the_word_is_enough() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("twirl");
        world.hear("twirl");
        // Guess until it happens to dance, saying no to everything else.
        let mut tries = 0;
        loop {
            tries += 1;
            assert!(tries < 40, "it never tried dancing");
            let guess = world.guess().expect("a fresh, repeated word gets a guess");
            assert!(!matches!(guess, Activity::Float | Activity::Visit));
            assert_eq!(world.ghost.plan.activity, guess);
            run(&mut world, 1.0);
            if guess == Activity::Dance {
                break;
            }
            let heard = world.hear("no");
            assert_eq!(heard.tone, Some(Tone::Scolding));
            assert!(world.life.vocabulary.evidence(&["twirl".to_owned()], guess) < 0.0);
            run(&mut world, 1.0);
            world.hear("twirl");
        }
        let before = world.time;
        let heard = world.hear("yes! good job");
        assert_eq!(heard.tone, Some(Tone::Praise));
        assert_eq!(heard.learned, [("twirl".to_owned(), Activity::Dance)]);
        let events = kinds(&world, before);
        assert!(events.contains(&EventKind::Praised));
        assert!(events.contains(&EventKind::LearnedWord {
            word: "twirl".to_owned(),
            activity: Activity::Dance
        }));
        assert!(events.contains(&EventKind::Milestone(
            "Learned my first word from you: “twirl”.".to_owned()
        )));

        run(&mut world, 10.0);
        world.begin(Activity::Explore);
        let heard = world.hear("Twirl for me, please!");
        assert_eq!(
            heard.understanding,
            Understanding::Doing {
                word: "twirl".to_owned(),
                activity: Activity::Dance
            }
        );
        assert_eq!(world.ghost.plan.activity, Activity::Dance);
        assert!(world.life.today.words.contains(&"twirl".to_owned()));
    }

    #[test]
    fn words_said_while_the_ghost_does_something_come_to_mean_it() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.plant.thirst = 0.9;
        for _ in 0..2 {
            world.begin(Activity::Tend);
            run(&mut world, 1.0);
            world.hear("water the plant");
        }
        assert_eq!(
            world.life.vocabulary.meaning(&["water".to_owned()]),
            Some(("water".to_owned(), Activity::Tend))
        );
        // Floating and coming to the glass are what any words bring.
        world.begin(Activity::Float);
        run(&mut world, NAMED_FOR as f32 + 1.0);
        world.hear("hmm");
        world.hear("hmm");
        assert!(world.life.vocabulary.words["hmm"].meanings.is_empty());
    }

    #[test]
    fn what_the_viewer_does_right_after_speaking_shows_what_they_meant() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("catch");
        run(&mut world, 1.0);
        world.kick_ball(Vec3::X);
        world.kick_ball(Vec3::Z);
        let catch = ["catch".to_owned()];
        let once = world.life.vocabulary.evidence(&catch, Activity::Fetch);
        assert!(once > 1.0 && once < SHOWN, "{once}");
        // Too late to be about the words.
        world.hear("snack");
        run(&mut world, 13.0);
        world.drop_mote(Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(
            world
                .life
                .vocabulary
                .evidence(&["snack".to_owned()], Activity::Chase),
            0.0
        );
        world.hear("catch");
        run(&mut world, 0.5);
        world.kick_ball(Vec3::X);
        assert_eq!(
            world.life.vocabulary.meaning(&catch),
            Some(("catch".to_owned(), Activity::Fetch))
        );
        // Darting the pointer across the glass shows the finger game.
        world.hear("finger");
        world.point_at(Some(Vec3::new(-0.6, 1.2, ROOM_MAX.z)));
        world.point_at(Some(Vec3::new(0.6, 1.2, ROOM_MAX.z)));
        assert!(
            world
                .life
                .vocabulary
                .evidence(&["finger".to_owned()], Activity::Pounce)
                > 1.0
        );
    }

    #[test]
    fn being_told_no_stops_what_the_ghost_is_doing_and_unlearns_the_words() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world
            .life
            .vocabulary
            .teach(&["spin".to_owned()], Activity::Dance, 2.0);
        world.hear("spin");
        assert_eq!(world.ghost.plan.activity, Activity::Dance);
        run(&mut world, 1.0);
        let heard = world.hear("no, stop");
        assert!(heard.stopped);
        // It leaves off and comes to the glass, a little sheepish.
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert!(kinds(&world, world.time).contains(&EventKind::Scolded));
        assert!(
            world
                .life
                .vocabulary
                .evidence(&["spin".to_owned()], Activity::Dance)
                < 2.0
        );
        assert_eq!(world.life.vocabulary.meaning(&["spin".to_owned()]), None);
    }

    #[test]
    fn a_no_with_a_known_word_stops_the_ghost_without_starting_it_again() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world
            .life
            .vocabulary
            .teach(&["spin".to_owned()], Activity::Dance, 2.0);
        world.begin(Activity::Dance);
        run(&mut world, 1.0);
        let heard = world.hear("no spin!");
        assert!(heard.stopped);
        assert_eq!(heard.understanding, Understanding::Nothing);
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        assert_eq!(
            world
                .life
                .vocabulary
                .evidence(&["spin".to_owned()], Activity::Dance),
            2.0
        );
    }

    #[test]
    fn a_mind_routing_a_message_without_words_teaches_nothing() {
        let mut world = World::new(6, 23.0);
        world.notice_viewer();
        world.hear("twirl");
        world.hear("goodnight");
        world.heed(Activity::Rest);
        assert_eq!(
            world
                .life
                .vocabulary
                .evidence(&["twirl".to_owned()], Activity::Rest),
            0.0
        );
        // Praise afterward isn't about the earlier word either.
        world.hear("good");
        assert_eq!(
            world
                .life
                .vocabulary
                .evidence(&["twirl".to_owned()], Activity::Rest),
            0.0
        );
    }

    #[test]
    fn a_word_it_knows_for_something_it_cant_do_now_is_understood_but_not_done() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world
            .life
            .vocabulary
            .teach(&["nap".to_owned()], Activity::Rest, 2.0);
        world.ghost.needs.energy = 1.0;
        let heard = world.hear("nap time");
        assert_eq!(
            heard.understanding,
            Understanding::Unable {
                word: "nap".to_owned(),
                activity: Activity::Rest
            }
        );
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
    }

    #[test]
    fn a_mind_routing_the_words_teaches_them_and_does_not_restart_the_plan() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("peekaboo");
        world.heed(Activity::Hide);
        assert_eq!(world.ghost.plan.activity, Activity::Hide);
        let started = world.ghost.plan.started;
        run(&mut world, 1.0);
        world.heed(Activity::Hide);
        assert_eq!(world.ghost.plan.started, started);
        assert_eq!(
            world.life.vocabulary.meaning(&["peekaboo".to_owned()]),
            Some(("peekaboo".to_owned(), Activity::Hide))
        );
    }

    #[test]
    fn a_goodnight_at_bedtime_sends_the_ghost_to_bed() {
        let mut world = World::new(6, 23.0);
        world.notice_viewer();
        world.hear("goodnight!");
        assert_eq!(world.ghost.plan.activity, Activity::Rest);
        assert!(world.ghost.plan.overnight);
        let mut world = World::new(6, 15.0);
        world.notice_viewer();
        world.hear("goodnight!");
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
    }

    #[test]
    fn praise_cheers_the_ghost_and_warms_the_bond() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.ghost.needs = Needs {
            energy: 0.5,
            fun: 0.5,
            company: 0.5,
        };
        // The first words it hears are a moment to remember, which
        // excites it.
        world.hear("hello");
        run(&mut world, 8.0);
        let bond = world.life.bond;
        world.hear("good ghost");
        world.step(0.1);
        assert_eq!(world.ghost.mood, Mood::Cheerful);
        assert!(world.life.bond > bond);
        assert_eq!(world.life.today.praises, 1);
    }

    #[test]
    fn a_ghost_that_knows_the_viewer_comes_to_greet_them() {
        for (bond, greets) in [(0.0, false), (0.5, true)] {
            let mut world = World::new(6, 12.0);
            world.life.bond = bond;
            world.begin(Activity::Window);
            run(&mut world, 1.0);
            world.notice_viewer();
            world.step(0.05);
            assert_eq!(
                world.ghost.plan.activity == Activity::Visit,
                greets,
                "{bond}"
            );
        }
    }

    #[test]
    fn what_happens_is_counted_in_the_ghosts_life() {
        let mut world = World::new(6, 12.0);
        world.notice_viewer();
        world.hear("hello");
        run(&mut world, 2.0);
        assert_eq!(world.life.total.heard, 1);
        assert_eq!(world.life.total.times(Activity::Visit), 1);
        assert!(world.life.total.together > 1.5);
        assert!(world.life.bond > 0.0);
    }

    #[test]
    fn a_first_is_celebrated_as_a_milestone_right_after_it() {
        let mut world = World::new(2, 12.0);
        world.drop_mote(Vec3::new(-1.0, 1.4, -0.5));
        world.begin(Activity::Chase);
        run(&mut world, 6.0);
        let eaten = world
            .events
            .iter()
            .position(|event| event.kind == EventKind::AteMote)
            .unwrap();
        assert_eq!(
            world.events[eaten + 1].kind,
            EventKind::Milestone("Caught my first light mote.".into())
        );
        assert_eq!(world.ghost.mood, Mood::Excited);
    }

    #[test]
    fn options_follow_what_is_possible() {
        let mut world = World::new(6, 12.0);
        world.motes.clear();
        world.ghost.needs.energy = 1.0;
        world.plant.thirst = 0.0;
        let options = world.options();
        assert!(!options.contains(&Activity::Chase));
        assert!(!options.contains(&Activity::Rest));
        assert!(!options.contains(&Activity::Tend));
        world.drop_mote(Vec3::ONE);
        assert!(world.options().contains(&Activity::Chase));
    }

    #[test]
    fn hiding_needs_a_viewer_and_energy_and_drawing_needs_an_awake_ghost() {
        let mut world = World::new(6, 12.0);
        assert!(!world.options().contains(&Activity::Hide));
        assert!(world.options().contains(&Activity::Doodle));
        world.notice_viewer();
        assert!(world.options().contains(&Activity::Hide));
        world.ghost.needs.energy = 0.3;
        assert!(!world.options().contains(&Activity::Hide));
        assert!(world.options().contains(&Activity::Doodle));
        world.ghost.needs.energy = 0.25;
        assert!(!world.options().contains(&Activity::Doodle));
        world.ghost.needs.energy = 0.8;
        world.ghost.asleep = 0.9;
        assert!(!world.options().contains(&Activity::Doodle));
    }

    #[test]
    fn the_ghost_reaches_its_hiding_spot_and_a_poke_finds_it() {
        let mut spots = Vec::new();
        for seed in 0..40 {
            let mut world = World::new(seed, 12.0);
            world.notice_viewer();
            world.begin(Activity::Hide);
            let spot = world.ghost.plan.target;
            if !spots.contains(&spot) {
                spots.push(spot);
            }
            assert!((25.0..45.0).contains(&world.ghost.plan.length));
            run(&mut world, 15.0);
            assert!(
                world.ghost.plan.arrived.is_some(),
                "seed {seed}: {:?}, target {spot}",
                world.ghost.position
            );
            assert!(world.ghost.fade > 0.5, "seed {seed}");
            assert!(!world.needs_decision());
            let before = world.ghost.needs;
            world.poke();
            assert!(
                world
                    .events
                    .iter()
                    .rev()
                    .take(2)
                    .any(|event| event.kind == EventKind::Found)
            );
            assert!(
                !world
                    .events
                    .iter()
                    .any(|event| matches!(event.kind, EventKind::Poked { .. }))
            );
            assert!(world.needs_decision());
            assert!(world.ghost.velocity.y > 0.5);
            assert_eq!(world.ghost.needs.fun, (before.fun + 0.2).min(1.0));
            assert_eq!(world.ghost.needs.company, (before.company + 0.15).min(1.0));
            world.step(0.05);
            assert!(world.ghost.fade < 0.3);
            assert_eq!(world.ghost.mood, Mood::Excited);
        }
        assert_eq!(spots.len(), 5);
    }

    #[test]
    fn an_unfound_ghost_jumps_toward_the_glass_and_shouts_boo() {
        let mut world = World::new(9, 12.0);
        world.notice_viewer();
        world.begin(Activity::Hide);
        for _ in 0..1400 {
            world.step(0.05);
            if world.needs_decision() {
                break;
            }
        }
        assert!(world.needs_decision());
        assert_eq!(world.events.last().unwrap().kind, EventKind::Jumped);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Boo!");
        assert!(world.ghost.velocity.z > 0.0);
        assert!(world.ghost.plan.target.z > ROOM_MAX.z - 0.6);
        run(&mut world, 2.0);
        assert!(world.ghost.fade < 0.01);
    }

    #[test]
    fn an_unreachable_hiding_spot_is_given_up_without_jumping() {
        let mut world = World::new(4, 12.0);
        world.begin(Activity::Hide);
        world.ghost.plan.target = Vec3::new(0.0, -5.0, 0.0);
        run(&mut world, 65.0);
        assert!(world.needs_decision());
        assert_eq!(world.ghost.fade, 0.0);
        assert!(
            !world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Jumped)
        );
    }

    #[test]
    fn interrupting_a_hiding_plan_brings_the_ghost_back_into_view() {
        let mut world = World::new(4, 12.0);
        world.begin(Activity::Hide);
        run(&mut world, 15.0);
        assert!(world.ghost.fade > 0.5);
        world.begin(Activity::Explore);
        run(&mut world, 2.0);
        assert!(world.ghost.fade < 0.01);
    }

    #[test]
    fn doodling_reaches_the_glass_draws_a_picture_and_finishes() {
        for seed in 0..10 {
            let mut world = World::new(seed, 12.0);
            world.begin(Activity::Doodle);
            let picture = world.ghost.plan.picture.clone().unwrap();
            let Picture::Sketch(sketch) = &picture else {
                panic!("expected a sketch")
            };
            assert!((-1.2..=1.2).contains(&world.ghost.plan.target.x));
            assert!((1.0..=1.6).contains(&world.ghost.plan.target.y));
            assert_eq!(world.ghost.plan.target.z, ROOM_MAX.z - 0.35);
            for _ in 0..600 {
                world.step(0.05);
                if world.needs_decision() {
                    break;
                }
            }
            assert!(world.needs_decision());
            assert_eq!(world.doodles.len(), 1);
            let doodle = &world.doodles[0];
            assert_eq!(doodle.picture, picture);
            assert_eq!(doodle.drawn(world.time), 1.0);
            assert!(
                world
                    .events
                    .iter()
                    .any(|event| event.kind == EventKind::Doodled(sketch.name()))
            );
            assert!(world.ghost.position.z > ROOM_MAX.z - 0.9);
            assert!(world.ghost.plan.target.z < ROOM_MAX.z - 0.5);
        }
    }

    #[test]
    fn only_the_newest_three_doodles_are_kept_and_they_expire() {
        let mut world = World::new(4, 12.0);
        for _ in 0..4 {
            world.begin(Activity::Doodle);
            world.ghost.position = world.ghost.plan.target;
            run(&mut world, 7.0);
        }
        assert_eq!(world.doodles.len(), 3);
        assert!(world.doodles[0].started > 7.0);
        assert!(
            world
                .doodles
                .windows(2)
                .all(|pair| pair[0].started < pair[1].started)
        );
        run(&mut world, Doodle::LIFETIME as f32);
        assert!(world.doodles.is_empty());
    }

    #[test]
    fn pictures_are_chosen_from_the_ghosts_mood_weather_and_bond() {
        let mut world = World::new(4, 12.0);
        assert_eq!(world.choose_sketch(), Sketch::Smile);
        world.ghost.mood = Mood::Bored;
        assert_eq!(world.choose_sketch(), Sketch::Spiral);
        world.ghost.mood = Mood::Content;
        world.weather.sky = Sky::Rain;
        assert_eq!(world.choose_sketch(), Sketch::Wave);
        world.weather.sky = Sky::Rainbow;
        assert_eq!(world.choose_sketch(), Sketch::Sun);
        world.weather.sky = Sky::Snow;
        assert_eq!(world.choose_sketch(), Sketch::Snowflake);
        world.weather.sky = Sky::Clear;
        world.notice_viewer();
        assert_eq!(world.choose_sketch(), Sketch::Smile);
        world.life.bond = GREETS_FROM;
        assert_eq!(world.choose_sketch(), Sketch::Heart);
        world.ghost.mood = Mood::Cheerful;
        for _ in 0..20 {
            assert!(matches!(
                world.choose_sketch(),
                Sketch::Heart | Sketch::Star | Sketch::Flower
            ));
        }
    }

    #[test]
    fn the_same_seed_plays_out_the_same_way() {
        let play = || {
            let mut world = World::new(11, 12.0);
            for round in 0..60 {
                if world.needs_decision() {
                    let options = world.options();
                    world.begin(options[round % options.len()]);
                }
                run(&mut world, 1.0);
            }
            (world.ghost.position, world.motes.len())
        };
        assert_eq!(play(), play());
    }

    #[test]
    fn each_occasion_is_celebrated_once_a_day_and_new_year_again_at_midnight() {
        use jiff::civil::Date;

        let mut world = World::new(1, 12.0);
        world.date = Date::constant(2026, 10, 24);
        world.step(0.0);
        world.step(1.0);
        assert_eq!(world.life.total.celebrations, 1);
        assert_eq!(
            world.events[0].kind,
            EventKind::Celebrated(Occasion::Halloween)
        );
        world.date = Date::constant(2026, 10, 25);
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 2);
        world.date = Date::constant(2026, 11, 1);
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 2);
        world.date = Date::constant(2026, 12, 31);
        world.hour = 19.999;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 3);
        world.hour = 20.0;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 4);
        world.hour = 23.999;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 4);
        world.date = Date::constant(2027, 1, 1);
        world.hour = 0.0;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 5);
        let celebrations: Vec<_> = world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::Celebrated(Occasion::NewYear))
            .collect();
        assert_eq!(celebrations.len(), 2);
        assert_eq!(world.event_words(celebrations[0]), "it is New Year");
        assert_eq!(world.event_words(celebrations[1]), "a new year began");
        world.step(0.1);
        world.hour = 12.0;
        world.step(0.1);
        assert_eq!(world.life.total.celebrations, 5);
    }

    #[test]
    fn a_birthday_at_midnight_keeps_its_priority_and_also_marks_the_new_year() {
        use jiff::civil::Date;

        let mut world = World::new(1, 23.999);
        world.date = Date::constant(2026, 12, 31);
        world.birthday = Some(Date::constant(2026, 1, 1));
        world.step(0.1);
        world.date = Date::constant(2027, 1, 1);
        world.hour = 0.0;
        world.step(0.1);
        world.step(0.1);
        assert_eq!(world.occasion(), Some(Occasion::Birthday));
        assert_eq!(world.decoration().unwrap().occasion, Occasion::Birthday);
        assert!(world.fireworks());
        assert_eq!(world.life.total.celebrations, 3);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Celebrated(Occasion::Birthday))
                .count(),
            1
        );
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Celebrated(Occasion::NewYear))
                .count(),
            2
        );
    }

    #[test]
    fn decorations_are_obstacles_and_visible_things_only_while_present() {
        use jiff::civil::Date;

        let mut world = World::new(1, 22.0);
        for (date, birthday, present) in [
            (Date::constant(2026, 10, 24), None, true),
            (Date::constant(2026, 12, 24), None, true),
            (Date::constant(2026, 12, 31), None, true),
            (Date::constant(2027, 1, 1), None, true),
            (
                Date::constant(2027, 3, 1),
                Some(Date::constant(2026, 3, 1)),
                true,
            ),
            (
                Date::constant(2026, 6, 10),
                Some(Date::constant(2026, 3, 3)),
                true,
            ),
            (
                Date::constant(2028, 11, 26),
                Some(Date::constant(2026, 3, 3)),
                true,
            ),
            (Date::constant(2027, 2, 14), None, false),
            (Date::constant(2026, 6, 21), None, false),
        ] {
            world.date = date;
            world.birthday = birthday;
            world.ghost.position = Vec3::new(0.95, 0.4, 0.2);
            let target = world.position_of(Thing::Decoration);
            assert_eq!(target.is_some(), present);
            assert_eq!(world.obstacles().len(), if present { 6 } else { 5 });
            world.ghost.heading =
                (target.unwrap_or(Decoration::POSITION) - world.ghost.position - Vec3::Y * 0.1)
                    .normalize();
            let sight = world.sight();
            assert_eq!(
                sight
                    .in_view
                    .iter()
                    .any(|seen| seen.thing == Thing::Decoration),
                present
            );
            assert!(!sight.out_of_view.contains(&Thing::Decoration));
            world.ghost.attention = None;
            world.attend("decoration", 5.0);
            assert_eq!(world.ghost.attention.is_some(), present);
        }
    }

    #[test]
    fn a_ball_bounces_off_the_decoration_and_passes_the_same_spot_on_ordinary_days() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2026, 10, 31);
        world.ball = Ball::at(Decoration::POSITION.x, Decoration::POSITION.z + 0.55);
        world.ball.velocity = -Vec3::Z * 1.8;
        let mut ordinary = world.clone();
        ordinary.date = jiff::civil::Date::constant(2026, 11, 1);
        let radius = world.decoration().unwrap().radius();
        for _ in 0..80 {
            world.step(0.005);
            ordinary.step(0.005);
            assert!(
                flat(world.ball.position - Decoration::POSITION).length()
                    >= radius + Ball::RADIUS - 1e-4
            );
        }
        assert!(world.ball.velocity.z > 0.0);
        assert!(ordinary.ball.position.z < Decoration::POSITION.z);
    }

    #[test]
    fn the_ghost_steers_around_the_decoration_when_flying_low() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2026, 12, 24);
        world.ghost.position = Decoration::POSITION + Vec3::new(-0.3, 0.45, 0.0);
        world.ghost.plan = Plan::new(
            Activity::Explore,
            0.0,
            Decoration::POSITION + Vec3::new(0.7, 0.45, 0.0),
            0.0,
        );
        let mut ordinary = world.clone();
        ordinary.date = jiff::civil::Date::constant(2026, 6, 21);
        world.move_ghost(0.1);
        ordinary.move_ghost(0.1);
        assert!(world.ghost.velocity.x < ordinary.ghost.velocity.x);
    }

    #[test]
    fn an_unfound_ghost_shouts_louder_on_halloween_night() {
        let mut world = World::new(9, 22.0);
        world.date = jiff::civil::Date::constant(2026, 10, 31);
        world.notice_viewer();
        world.begin(Activity::Hide);
        for _ in 0..1400 {
            world.step(0.05);
            if world.needs_decision() {
                break;
            }
        }
        assert_eq!(world.events.last().unwrap().kind, EventKind::Jumped);
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "BOO!");
    }

    #[test]
    fn birthdays_are_cheerful_unless_sleepiness_irritation_or_delight_takes_over() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2027, 3, 1);
        world.birthday = Some(jiff::civil::Date::constant(2026, 3, 1));
        world.ghost.needs.fun = 0.1;
        world.ghost.needs.company = 0.1;
        assert_eq!(world.feeling(), Mood::Cheerful);
        world.ghost.needs.energy = 0.1;
        assert_eq!(world.feeling(), Mood::Sleepy);
        world.ghost.needs.energy = 0.8;
        world.record(EventKind::Poked { count: 3 });
        assert_eq!(world.feeling(), Mood::Grumpy);
        world.events.clear();
        world.record(EventKind::AteMote);
        assert_eq!(world.feeling(), Mood::Excited);
    }

    #[test]
    fn valentine_doodles_are_hearts_whatever_the_weather_or_mood() {
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2027, 2, 14);
        for sky in [Sky::Clear, Sky::Rain, Sky::Snow, Sky::Rainbow] {
            world.weather.sky = sky;
            for mood in Mood::ALL {
                world.ghost.mood = mood;
                for _ in 0..10 {
                    assert_eq!(world.choose_sketch(), Sketch::Heart);
                }
            }
        }
    }

    #[test]
    fn writing_sanitizes_text_and_requires_an_awake_ghost_with_energy() {
        let mut world = World::new(5, 12.0);
        assert!(!world.write(" /ㄱ "));
        assert!(world.write(" /한글♥ "));
        assert!(
            matches!(&world.ghost.plan.picture, Some(Picture::Word(writing)) if writing.text == "한글♥")
        );
        assert!(world.write(" hi /2027!? more"));
        let Some(Picture::Word(writing)) = &world.ghost.plan.picture else {
            panic!("expected writing")
        };
        assert_eq!(writing.text, "HI2027!?");
        world.ghost.asleep = 0.3;
        let plan = world.ghost.plan.clone();
        assert!(!world.write("HI"));
        assert_eq!(world.ghost.plan, plan);
        world.ghost.asleep = 0.0;
        world.ghost.needs.energy = 0.25;
        assert!(!world.write("HI"));
        world.ghost.needs.energy = 0.26;
        assert!(world.write("abcdefghijk"));
        let Some(Picture::Word(writing)) = &world.ghost.plan.picture else {
            panic!("expected writing")
        };
        assert_eq!(writing.text, "ABCDEFGH");
    }

    #[test]
    fn three_watched_words_teach_the_ghost_to_write_for_the_viewer_once() {
        let mut world = World::new(5, 12.0);
        // Words completed alone do not teach it which side the viewer reads.
        for _ in 0..3 {
            assert!(world.write("HI"));
            world.ghost.position = world.ghost.plan.target;
            run(&mut world, 9.0);
            assert!(world.needs_decision());
        }
        assert_eq!(world.life.total.writings, 3);
        assert_eq!(world.life.total.watched_writings, 0);
        for index in 0..5 {
            world.notice_viewer();
            assert!(world.write("MINA"));
            let Some(Picture::Word(writing)) = &world.ghost.plan.picture else {
                panic!("expected writing")
            };
            assert_eq!(writing.mirrored, index < 3);
            world.ghost.position = world.ghost.plan.target;
            run(&mut world, 9.0);
            assert!(world.needs_decision());
            assert_eq!(world.life.total.watched_writings, index + 1);
        }
        let text = "I learned to write so you can read it.";
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == text)
                .count(),
            1
        );
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::Milestone(text.to_owned()))
                .count(),
            1
        );
        assert!(world.events.iter().any(|event| matches!(&event.kind, EventKind::Wrote { text, mirrored: false, watched: true } if text == "MINA")));
        assert!(
            world
                .life
                .today
                .words()
                .contains("wrote “MINA” on the glass for you")
        );
    }

    #[test]
    fn introductions_store_and_replace_the_viewers_name_but_mark_only_the_first() {
        let mut world = World::new(5, 12.0);
        world.begin(Activity::Dance);
        let heard = world.hear("my name is Mina");
        assert_eq!(heard.learned_name.as_deref(), Some("Mina"));
        assert_eq!(heard.understanding, Understanding::Nothing);
        assert!(world.life.vocabulary.words.is_empty());
        assert_eq!(world.life.viewer_name.as_deref(), Some("Mina"));
        assert_eq!(world.hear("my name is Mina").learned_name, None);
        assert_eq!(
            world.hear("call me Sam").learned_name.as_deref(),
            Some("Sam")
        );
        assert_eq!(world.life.viewer_name.as_deref(), Some("Sam"));
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::LearnedName(_)))
                .count(),
            2
        );
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text.starts_with("I learned your name:"))
                .count(),
            1
        );
        assert!(world.life.today.words().contains("learned your name: Sam"));
    }

    // A clear Perseid night, with the ghost watching the window.
    fn under_the_perseids(seed: u64) -> World {
        let mut world = World::new(seed, 23.0);
        world.date = jiff::civil::Date::constant(2026, 8, 12);
        world.weather.sky = Sky::Clear;
        world.weather.storm = false;
        world.weather.until = 1e9;
        world
    }

    fn wishes(world: &World) -> usize {
        world
            .events
            .iter()
            .filter(|event| event.kind == EventKind::ShootingStar)
            .count()
    }

    #[test]
    fn a_ghost_watching_the_window_wishes_once_on_each_shooting_star_it_sees() {
        let mut world = under_the_perseids(4);
        let mut stars = std::collections::BTreeSet::new();
        for _ in 0..3000 {
            if world.ghost.plan.done || world.ghost.plan.activity != Activity::Window {
                world.begin(Activity::Window);
            }
            world.ghost.needs.energy = 1.0;
            world.step(0.1);
            if let Some(meteor) = world.shooting_star() {
                stars.insert(meteor.slot);
            }
        }
        let seen = world.life.total.shooting_stars as usize;
        assert!(seen >= 1, "it saw no shooting star in five minutes");
        assert!(seen <= stars.len());
        assert!(wishes(&world) <= seen);
        assert!(
            world
                .life
                .milestones
                .iter()
                .any(|milestone| milestone.text == "Saw my first shooting star, and made a wish.")
        );
        assert!(world.life.today.words().contains("shooting star"));
    }

    #[test]
    fn no_shooting_stars_by_day_in_rain_or_for_a_sleeping_ghost() {
        let mut world = under_the_perseids(4);
        world.hour = 12.0;
        assert!(world.meteor_shower().is_none());
        assert!((0..6000).all(|step| {
            world.time = f64::from(step) * 0.1;
            world.shooting_star().is_none()
        }));
        world.hour = 23.0;
        world.weather.sky = Sky::Rain;
        assert!((0..6000).all(|step| {
            world.time = f64::from(step) * 0.1;
            world.shooting_star().is_none()
        }));

        let mut world = under_the_perseids(4);
        world.tuck_in();
        for _ in 0..3000 {
            world.step(0.1);
        }
        assert!(world.ghost.asleep > 0.5);
        assert_eq!(world.life.total.shooting_stars, 0);
    }

    #[test]
    fn the_viewer_names_the_ghost_and_only_the_first_name_is_a_milestone() {
        let mut world = World::new(5, 12.0);
        world.begin(Activity::Dance);
        let heard = world.hear("your name is Pip");
        assert_eq!(heard.given_name.as_deref(), Some("Pip"));
        assert_eq!(heard.understanding, Understanding::Nothing);
        assert!(world.life.vocabulary.words.is_empty());
        assert_eq!(world.ghost.name, "Pip");
        assert_eq!(world.hear("your name is Pip").given_name, None);
        assert_eq!(
            world.hear("뽀삐라고 부를게").given_name.as_deref(),
            Some("뽀삐")
        );
        assert_eq!(world.ghost.name, "뽀삐");
        // A mind passing on the same name changes nothing.
        assert!(!world.take_name("뽀삐"));
        assert!(!world.take_name(" \u{7} "));
        let named = world
            .events
            .iter()
            .filter(|event| matches!(event.kind, EventKind::Named(_)))
            .count();
        assert_eq!(named, 2);
        let milestones: Vec<_> = world
            .life
            .milestones
            .iter()
            .filter(|entry| entry.text.starts_with("You gave me my name:"))
            .collect();
        assert_eq!(milestones.len(), 1);
        assert_eq!(milestones[0].text, "You gave me my name: Pip.");
    }

    #[test]
    fn special_days_choose_their_words_only_on_the_right_occasion() {
        for (date, birthday, word) in [
            ((2026, 10, 31), None, "BOO"),
            ((2026, 12, 31), None, "2027"),
            ((2027, 1, 1), None, "2027"),
            (
                (2027, 3, 1),
                Some(jiff::civil::Date::constant(2026, 3, 1)),
                "YAY",
            ),
            (
                (2026, 6, 10),
                Some(jiff::civil::Date::constant(2026, 3, 3)),
                "YAY",
            ),
            (
                (2028, 11, 26),
                Some(jiff::civil::Date::constant(2026, 3, 3)),
                "YAY",
            ),
        ] {
            let mut chosen = 0;
            for seed in 0..250 {
                let mut world = World::new(seed, 21.0);
                world.date = jiff::civil::Date::constant(date.0, date.1, date.2);
                world.birthday = birthday;
                match world.choose_picture() {
                    Picture::Word(writing) => {
                        assert_eq!(writing.text, word);
                        chosen += 1;
                    }
                    Picture::Sketch(_) | Picture::Copy(_) => {}
                }
            }
            assert!((35..180).contains(&chosen), "{word}: {chosen}");
        }
        for seed in 0..250 {
            let mut world = World::new(seed, 12.0);
            assert!(matches!(world.choose_picture(), Picture::Sketch(_)));
        }
    }

    #[test]
    fn greetings_require_a_recent_arrival_and_a_friend_and_names_need_a_close_bond() {
        let mut greetings = 0;
        let mut names = 0;
        let mut hangul_names = 0;
        for seed in 0..250 {
            let mut world = World::new(seed, 12.0);
            world.notice_viewer();
            world.record(EventKind::ViewerArrived);
            assert!(matches!(world.choose_picture(), Picture::Sketch(_)));
            world.life.bond = world.life.greeting_bond();
            if let Picture::Word(writing) = world.choose_picture() {
                assert_eq!(writing.text, "HI");
                greetings += 1;
            }
            world.time = 121.0;
            world.notice_viewer();
            assert!(matches!(world.choose_picture(), Picture::Sketch(_)));
            world.life.viewer_name = Some("Mina".to_owned());
            world.life.bond = 0.54;
            assert!(matches!(world.choose_picture(), Picture::Sketch(_)));
            world.life.bond = 0.55;
            if let Picture::Word(writing) = world.choose_picture() {
                assert_eq!(writing.text, "MINA♥");
                names += 1;
            }
            world.life.viewer_name = Some("민수".to_owned());
            if let Picture::Word(writing) = world.choose_picture() {
                assert_eq!(writing.text, "민수♥");
                hangul_names += 1;
            }
            world.life.viewer_name = Some("민Sam".to_owned());
            assert!(matches!(world.choose_picture(), Picture::Sketch(_)));
        }
        assert!((50..170).contains(&greetings));
        assert!((35..130).contains(&names));
        assert!((35..130).contains(&hangul_names));
    }

    #[test]
    fn messages_ease_the_viewers_language_and_punctuation_leaves_it_alone() {
        let mut world = World::new(1, 12.0);
        assert!(!world.speaks_korean());
        assert!(world.hear("안녕").korean);
        assert!(!world.speaks_korean());
        world.hear("같이 놀자");
        assert!(world.speaks_korean());
        let before = world.life.korean;
        world.hear("!? 123 ♥");
        assert_eq!(world.life.korean, before);
        world.say(world.spoken("Boo!", "까꿍!"));
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "까꿍!");
        world.hear("hello");
        assert!(!world.speaks_korean());
        world.say(world.spoken("Boo!", "까꿍!"));
        assert_eq!(world.ghost.speech.as_ref().unwrap().text, "Boo!");
        world.hear("ㅋㅋ");
        assert!(world.speaks_korean());
    }

    #[test]
    fn korean_holidays_choose_their_own_hangul_words() {
        for (date, expected) in [
            (jiff::civil::Date::constant(2027, 2, 7), &["복"][..]),
            (jiff::civil::Date::constant(2026, 9, 25), &["달"][..]),
            (
                jiff::civil::Date::constant(2026, 10, 9),
                &["한글", "가나다", "안녕"][..],
            ),
        ] {
            let mut words = 0;
            for seed in 0..100 {
                let mut world = World::new(seed, 12.0);
                world.date = date;
                if let Picture::Word(writing) = world.choose_picture() {
                    assert!(expected.contains(&writing.text.as_str()));
                    words += 1;
                }
            }
            assert!(words > 15);
        }
    }

    #[test]
    fn korean_holiday_actions_replay_from_the_same_seed() {
        for date in [
            jiff::civil::Date::constant(2027, 2, 7),
            jiff::civil::Date::constant(2026, 9, 25),
        ] {
            let replay = || {
                let mut world = World::new(19, 21.0);
                world.date = date;
                world.hear("안녕");
                world.hear("반가워");
                world.step(12.0);
                world.drop_mote(world.ghost.position);
                world.begin(Activity::Doodle);
                world.step(10.0);
                world.begin(Activity::Window);
                world.step(15.0);
                world
            };
            let first = replay();
            let second = replay();
            assert_eq!(first.events, second.events);
            assert_eq!(first.life, second.life);
            assert_eq!(first.doodles, second.doodles);
            assert_eq!(first.ghost.position, second.ghost.position);
            assert_eq!(first.ghost.thought, second.ghost.thought);
        }
    }

    #[test]
    fn valentines_words_are_only_the_viewers_name_with_a_heart() {
        let mut names = 0;
        for seed in 0..150 {
            let mut world = World::new(seed, 12.0);
            world.date = jiff::civil::Date::constant(2027, 2, 14);
            world.life.bond = 0.8;
            world.notice_viewer();
            world.record(EventKind::ViewerArrived);
            assert_eq!(world.choose_picture(), Picture::Sketch(Sketch::Heart));
            world.life.viewer_name = Some("Mina".to_owned());
            match world.choose_picture() {
                Picture::Word(writing) => {
                    assert_eq!(writing.text, "MINA♥");
                    names += 1;
                }
                picture => assert_eq!(picture, Picture::Sketch(Sketch::Heart)),
            }
        }
        assert!(names > 20);
    }
}
