use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::friend::count_words;
use super::{
    Activity, EventKind, GREETS_FROM, Keepsake, LearnedPicture, Nature, Occasion, Outcome,
    Upbringing, Vocabulary,
};

/// Days of the diary kept.
const DAYS_KEPT: usize = 60;
/// Bonds at which the ghost marks how close it has grown, and what it
/// says about it.
const CLOSER: [(f32, &str, &str); 4] = [
    (
        0.3,
        "We got to know each other.",
        "우리는 서로를 알아 갔어.",
    ),
    (0.55, "I've grown fond of you.", "네가 점점 좋아졌어."),
    (
        0.8,
        "We're close now, you and I.",
        "너와 나는 이제 가까운 사이야.",
    ),
    (
        0.95,
        "I'm devoted to you.",
        "내 마음은 언제나 네 곁에 있어.",
    ),
];
/// Hours together the ghost marks, and what it says about them. Milestones
/// reach the minds as events, so they are written in words, not digits.
const HOURS_MARKED: [(f64, &str, &str); 3] = [
    (
        1.0,
        "We spent our first hour together.",
        "처음으로 한 시간을 같이 보냈어.",
    ),
    (
        10.0,
        "We've spent ten hours together.",
        "열 시간을 같이 보냈어.",
    ),
    (
        100.0,
        "We've spent a hundred hours together.",
        "백 시간을 같이 보냈어.",
    ),
];
/// Numbers of the viewer's words it knows that the ghost marks, after the
/// first.
const WORDS_MARKED: [(usize, &str, &str); 4] = [
    (
        10,
        "I know ten of your words now.",
        "네 말 열 개를 알게 됐어.",
    ),
    (
        25,
        "I know twenty-five of your words now.",
        "네 말 스물다섯 개를 알게 됐어.",
    ),
    (
        50,
        "I know fifty of your words now.",
        "네 말 쉰 개를 알게 됐어.",
    ),
    (
        100,
        "I know a hundred of your words now.",
        "네 말 백 개를 알게 됐어.",
    ),
];
/// Deeds a diary entry keeps, the ones that stood out most.
const DIARY_DEEDS: usize = 6;
/// Words learned in a day that the diary names.
const WORDS_NAMED: usize = 4;
/// Light motes eaten that the ghost marks, after the first.
const MOTES_MARKED: [(u32, &str, &str); 4] = [
    (
        50,
        "Ate my fiftieth light mote.",
        "빛 알갱이 쉰 개를 먹었어.",
    ),
    (
        100,
        "Ate my hundredth light mote.",
        "빛 알갱이 백 개를 먹었어.",
    ),
    (
        500,
        "Ate my five hundredth light mote.",
        "빛 알갱이 오백 개를 먹었어.",
    ),
    (
        1000,
        "Ate my thousandth light mote.",
        "빛 알갱이 천 개를 먹었어.",
    ),
];
/// How much each second with the viewer around warms the bond, before the
/// day's warmth wears off and a deeper bond slows it.
const GROWING: f32 = 0.26 / 3600.0;
/// The warmth one day can hold. Closeness gathered in a day wears off as it
/// gathers, so hours spread over many days count for more than one long
/// day: with company most of each day, the ghost gets to know the viewer on
/// the first day, grows fond within a few, is close after about a week and
/// a half, and devoted only after many weeks.
const DAILY_WARMTH: f32 = 0.45;
/// How much of the bond fades for each day the viewer stays away, after the
/// first.
const FADING: f32 = 0.03;
/// Times an activity counts as done before the first, so a new ghost has no
/// habits and a few choices don't make one.
const PRIOR: f32 = 3.0;
/// How much yesterday's visiting routine still counts today.
const ROUTINE_DECAY: f32 = 0.9;

/// How much something the ghost did stands out in its day, from the
/// everyday to what it will remember for good.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Salience {
    Routine,
    Small,
    Notable,
    Special,
    Momentous,
}

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
    pub anniversaries: u32,
    /// Recalled lines kept for the diary, bounded like other deed details.
    pub anniversary_words: Vec<String>,
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
    #[serde(default)]
    pub writings: u32,
    /// Completed words with the viewer there to watch.
    #[serde(default)]
    pub watched_writings: u32,
    #[serde(default)]
    pub written_words: Vec<String>,
    #[serde(default)]
    pub viewer_names: Vec<String>,
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
    /// Times the viewer told it no.
    pub scolds: u32,
    /// Times it worked out what one of the viewer's words means.
    pub learned: u32,
    /// The first few words it worked out, kept for the diary.
    pub words: Vec<String>,
    pub pokes: u32,
    /// Times the viewer stroked it.
    pub pets: u32,
    /// Seconds the viewer was around, kept in double precision so years of
    /// frames still add up.
    pub together: f64,
    /// Times it set out on each activity, by the activity's name.
    pub activities: BTreeMap<String, u32>,
    /// Visits it saw at the window and the lamp.
    pub birds: u32,
    pub moths: u32,
    /// Gifts from the bird that made it onto the shelf.
    #[serde(default)]
    pub gifts: u32,
    /// The kinds it kept, once each, for the diary.
    #[serde(default)]
    pub keepsakes: Vec<Keepsake>,
    #[serde(default)]
    pub expected: u32,
    #[serde(default)]
    pub missed: u32,
    pub promises_made: u32,
    pub promises_kept: u32,
    /// Kept trip promises are remembered separately for diary salience.
    pub trips_kept: u32,
    /// Towers built, highest tower, and towers the viewer knocked down.
    #[serde(default)]
    pub towers: u32,
    #[serde(default)]
    pub tallest: u32,
    #[serde(default)]
    pub toppled: u32,
    /// Shooting stars it saw and wished on.
    #[serde(default)]
    pub shooting_stars: u32,
    /// Rain and snow it felt coming before they fell.
    #[serde(default)]
    pub forecasts: u32,
    #[serde(default)]
    pub snow_forecasts: u32,
    #[serde(default)]
    pub friend_visits: u32,
    #[serde(default)]
    pub friend_games: u32,
    #[serde(default)]
    pub friend_name: Option<String>,
    #[serde(default)]
    pub friend_played: Vec<super::friend::Game>,
    #[serde(default)]
    pub moon_wishes: u32,
    #[serde(default)]
    pub drawings_seen: u32,
    #[serde(default)]
    pub pictures_learned: u32,
    #[serde(default)]
    pub planted: u32,
    #[serde(default)]
    pub sprouted: u32,
    #[serde(default)]
    pub flowered: u32,
    #[serde(default)]
    pub drooped: u32,
    #[serde(default)]
    pub planted_kinds: Vec<super::Flower>,
    #[serde(default)]
    pub sprouted_kinds: Vec<super::Flower>,
    #[serde(default)]
    pub flowered_kinds: Vec<super::Flower>,
    #[serde(default)]
    pub focus_sessions: u32,
    #[serde(default)]
    pub focus_minutes: u32,
    #[serde(default)]
    pub stories: u32,
    #[serde(default)]
    pub constellations: u32,
    #[serde(default)]
    pub airings: u32,
    #[serde(default)]
    pub passersby: u32,
    #[serde(default)]
    pub passerby_kinds: Vec<super::Passerby>,
    #[serde(default)]
    pub word_games: u32,
    #[serde(default)]
    pub words_guessed: u32,
    #[serde(default)]
    pub quirks: BTreeMap<super::Quirk, u32>,
    pub outfits: u32,
    pub pages: u32,
    #[serde(default)]
    pub books_finished: u32,
    #[serde(default)]
    pub books_received: u32,
    pub tunes_played: u32,
    #[serde(default)]
    pub compositions: u32,
    #[serde(default)]
    pub composition_titles: Vec<String>,
    #[serde(default)]
    pub songs_sung_back: u32,
    #[serde(default)]
    pub books_written: u32,
    #[serde(default)]
    pub written_titles: Vec<String>,
    #[serde(default)]
    pub tunes_learned: u32,
    #[serde(default)]
    pub lullaby_sleeps: u32,
    pub names: u32,
    #[serde(default)]
    pub wishes_made: u32,
    #[serde(default)]
    pub wishes_granted: u32,
    /// Granted wishes that involved the viewer.
    pub viewer_wishes_granted: u32,
    #[serde(default)]
    pub wishes_faded: u32,
    #[serde(default)]
    pub wish_words: Vec<String>,
    #[serde(default)]
    pub mirror_visits: u32,
    #[serde(default)]
    pub met_reflection: bool,
    #[serde(default)]
    pub recognized_self: bool,
    pub letters_written: u32,
    #[serde(default)]
    pub letters_read: u32,
    #[serde(default)]
    pub letters_received: u32,
    #[serde(default)]
    pub viewer_letters_read: u32,
    pub learned_about: u32,
    pub comforts: u32,
    /// First lasting changes it recognized, kept among the day's deeds.
    pub temperament_changes: Vec<String>,
    pub feelings_heard: u32,
    pub feelings: Vec<super::Feeling>,
    pub comforted_feelings: Vec<super::Feeling>,
    pub felt_better: u32,
    #[serde(default)]
    pub plans_heard: u32,
    #[serde(default)]
    pub luck_wished: u32,
    #[serde(default)]
    pub plans_went_well: u32,
    #[serde(default)]
    pub plans_went_badly: u32,
    #[serde(default)]
    pub heard_plans: Vec<super::agenda::Errand>,
    #[serde(default)]
    pub wished_plans: Vec<super::agenda::Errand>,
    #[serde(default)]
    pub good_plans: Vec<super::agenda::Errand>,
    #[serde(default)]
    pub hard_plans: Vec<super::agenda::Errand>,
    #[serde(default)]
    pub hunts: u32,
    #[serde(default)]
    pub treasures_found: u32,
    #[serde(default)]
    pub treasure_hints: u32,
    #[serde(default)]
    pub fastest_treasure: Option<u32>,
    #[serde(default)]
    pub breaks_suggested: u32,
    #[serde(default)]
    pub breaks_taken: u32,
    #[serde(default)]
    pub late_nights: u32,
    #[serde(default)]
    pub goodnights_together: u32,
    #[serde(default)]
    pub cranes: u32,
    #[serde(default)]
    pub cranes_for_sad_viewer: u32,
    /// Pictures the viewer took of it, and how many it posed for.
    #[serde(default)]
    pub photos: u32,
    #[serde(default)]
    pub portraits: u32,
    #[serde(default)]
    pub bubble_sessions: u32,
    #[serde(default)]
    pub bubbles_blown: u32,
    #[serde(default)]
    pub bubbles_popped: u32,
    #[serde(default)]
    pub viewer_pins: u32,
    #[serde(default)]
    pub doodle_pins: u32,
    #[serde(default)]
    pub constellation_pins: u32,
    #[serde(default)]
    pub pinned_doodles: Vec<String>,
    #[serde(default)]
    /// Paired names and remembered deeds supplied by their typed events.
    pub korean_details: BTreeMap<String, String>,
}

impl Tally {
    /// Whether nothing at all was counted.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    fn note(&mut self, kind: &EventKind) {
        match kind {
            EventKind::Promised(_) => self.promises_made = self.promises_made.saturating_add(1),
            EventKind::PromiseKept(kind) => {
                self.promises_kept = self.promises_kept.saturating_add(1);
                if kind.trip() {
                    self.trips_kept = self.trips_kept.saturating_add(1);
                }
            }
            EventKind::ViewerFelt(feeling) => {
                self.feelings_heard = self.feelings_heard.saturating_add(1);
                if !self.feelings.contains(feeling) {
                    self.feelings.push(*feeling);
                }
            }
            EventKind::Comforted(feeling) => {
                self.comforts = self.comforts.saturating_add(1);
                if !self.comforted_feelings.contains(feeling) {
                    self.comforted_feelings.push(*feeling);
                }
            }
            EventKind::FeltBetter => self.felt_better = self.felt_better.saturating_add(1),
            EventKind::Composed(title) => {
                self.compositions = self.compositions.saturating_add(1);
                if self.composition_titles.len() < 8 {
                    self.composition_titles.push(title.clone());
                }
            }
            EventKind::SangMySong(_) => {
                self.songs_sung_back = self.songs_sung_back.saturating_add(1)
            }
            EventKind::WroteBook(title) => {
                self.books_written = self.books_written.saturating_add(1);
                if self.written_titles.len() < 6 {
                    self.written_titles.push(title.clone());
                }
            }
            EventKind::VisitedMirror => self.mirror_visits = self.mirror_visits.saturating_add(1),
            EventKind::MetReflection => self.met_reflection = true,
            EventKind::RecognizedSelf => self.recognized_self = true,
            EventKind::GotLetter => self.letters_received = self.letters_received.saturating_add(1),
            EventKind::ReadViewerLetter => {
                self.viewer_letters_read = self.viewer_letters_read.saturating_add(1)
            }
            EventKind::WroteLetter => self.letters_written = self.letters_written.saturating_add(1),
            EventKind::ReadLetter => self.letters_read = self.letters_read.saturating_add(1),
            EventKind::LearnedAboutViewer(_) => {
                self.learned_about = self.learned_about.saturating_add(1)
            }
            EventKind::Quirk(quirk) => {
                let count = self.quirks.entry(*quirk).or_default();
                *count = count.saturating_add(1);
            }
            EventKind::PutOn(_) => self.outfits = self.outfits.saturating_add(1),
            EventKind::NewBook(_) => self.books_received = self.books_received.saturating_add(1),
            EventKind::ReadPage => self.pages = self.pages.saturating_add(1),
            EventKind::FinishedBook(_) => {
                self.books_finished = self.books_finished.saturating_add(1)
            }
            EventKind::PlayedMusic { .. } => {
                self.tunes_played = self.tunes_played.saturating_add(1)
            }
            EventKind::LearnedTune(_) => self.tunes_learned = self.tunes_learned.saturating_add(1),
            EventKind::LullabySleep => self.lullaby_sleeps = self.lullaby_sleeps.saturating_add(1),
            EventKind::Named(_) => self.names = self.names.saturating_add(1),
            EventKind::MadeWish(_) | EventKind::WishGranted(_) | EventKind::WishFaded(_) => {
                match kind {
                    EventKind::MadeWish(_) => self.wishes_made = self.wishes_made.saturating_add(1),
                    EventKind::WishGranted(_) => {
                        self.wishes_granted = self.wishes_granted.saturating_add(1);
                        if let EventKind::WishGranted(wish) = kind
                            && wish.with_viewer()
                        {
                            self.viewer_wishes_granted =
                                self.viewer_wishes_granted.saturating_add(1);
                        }
                    }
                    _ => self.wishes_faded = self.wishes_faded.saturating_add(1),
                }
                if self.wish_words.len() < 12 {
                    let english = match kind {
                        EventKind::MadeWish(wish) => {
                            format!("made a wish {}", wish.words(&Life::default()))
                        }
                        _ => kind.words(),
                    };
                    let korean = match kind {
                        EventKind::MadeWish(wish) => {
                            format!("소원을 빌었어: {}", wish.korean(&Life::default()))
                        }
                        EventKind::WishGranted(wish) => {
                            format!("소원이 이루어졌어: {}", wish.korean(&Life::default()))
                        }
                        EventKind::WishFaded(wish) => {
                            format!("소원을 살며시 보내 줬어: {}", wish.korean(&Life::default()))
                        }
                        _ => unreachable!(),
                    };
                    self.remember_writing(english.clone(), korean);
                    self.wish_words.push(english);
                }
            }
            EventKind::SawPasserby(passerby) => {
                self.passersby = self.passersby.saturating_add(1);
                if !self.passerby_kinds.contains(passerby) {
                    self.passerby_kinds.push(*passerby);
                }
            }
            EventKind::FocusEnded { minutes, completed } => {
                self.focus_minutes = self.focus_minutes.saturating_add(*minutes);
                self.focus_sessions = self.focus_sessions.saturating_add(u32::from(*completed));
            }
            EventKind::OpenedWindow => self.airings = self.airings.saturating_add(1),
            EventKind::ToldStory => self.stories = self.stories.saturating_add(1),
            EventKind::FoundConstellation(_) => {
                self.constellations = self.constellations.saturating_add(1);
            }
            EventKind::Began(activity) => {
                if *activity == Activity::Seek {
                    self.hunts = self.hunts.saturating_add(1);
                }
                let count = self
                    .activities
                    .entry(activity.name().to_owned())
                    .or_default();
                *count = count.saturating_add(1);
            }
            EventKind::AteMote => self.motes = self.motes.saturating_add(1),
            EventKind::NudgedBall => self.nudges = self.nudges.saturating_add(1),
            EventKind::Fetched => self.fetches = self.fetches.saturating_add(1),
            EventKind::Pounced => self.pounces = self.pounces.saturating_add(1),
            EventKind::Danced => self.dances = self.dances.saturating_add(1),
            EventKind::MoonWish => self.moon_wishes = self.moon_wishes.saturating_add(1),
            EventKind::Built { height } => {
                self.towers = self.towers.saturating_add(1);
                self.tallest = self.tallest.max(*height);
            }
            EventKind::Toppled { by_viewer: true } => self.toppled = self.toppled.saturating_add(1),
            EventKind::Remembered(line) => {
                self.anniversaries = self.anniversaries.saturating_add(1);
                if self.anniversary_words.len() < 12 {
                    self.anniversary_words.push(line.clone());
                }
            }
            EventKind::Celebrated(occasion) => {
                self.celebrations = self.celebrations.saturating_add(1);
                if !self.occasions.contains(occasion) {
                    self.occasions.push(*occasion);
                }
            }
            EventKind::Found => self.finds = self.finds.saturating_add(1),
            EventKind::Jumped => self.jumps = self.jumps.saturating_add(1),
            EventKind::Doodled(_) | EventKind::Copied(_) => {
                self.doodles = self.doodles.saturating_add(1)
            }
            EventKind::SawDrawing { .. } => {
                self.drawings_seen = self.drawings_seen.saturating_add(1)
            }
            EventKind::LearnedPicture(_) => {
                self.pictures_learned = self.pictures_learned.saturating_add(1)
            }
            EventKind::WordGame { solved, .. } => {
                self.word_games = self.word_games.saturating_add(1);
                if *solved {
                    self.words_guessed = self.words_guessed.saturating_add(1);
                }
            }
            EventKind::GameOver(outcome) if outcome.finished() => {
                self.games = self.games.saturating_add(1);
                match outcome {
                    Outcome::GhostWon { .. } => self.wins = self.wins.saturating_add(1),
                    Outcome::ViewerWon { .. } => self.losses = self.losses.saturating_add(1),
                    Outcome::Draw => self.draws = self.draws.saturating_add(1),
                    Outcome::Abandoned => unreachable!(),
                }
            }
            EventKind::Wrote { text, watched, .. } => {
                self.writings = self.writings.saturating_add(1);
                if *watched {
                    self.watched_writings = self.watched_writings.saturating_add(1);
                }
                if self.written_words.len() < WORDS_NAMED && !self.written_words.contains(text) {
                    self.written_words.push(text.clone());
                }
            }
            EventKind::LearnedName(name) => {
                if self.viewer_names.len() < WORDS_NAMED && !self.viewer_names.contains(name) {
                    self.viewer_names.push(name.clone());
                }
            }
            EventKind::Planted(kind) => {
                self.planted = self.planted.saturating_add(1);
                if !self.planted_kinds.contains(kind) {
                    self.planted_kinds.push(*kind);
                }
            }
            EventKind::Sprouted(kind) => {
                self.sprouted = self.sprouted.saturating_add(1);
                if !self.sprouted_kinds.contains(kind) {
                    self.sprouted_kinds.push(*kind);
                }
            }
            EventKind::Flowered(kind) => {
                self.flowered = self.flowered.saturating_add(1);
                if !self.flowered_kinds.contains(kind) {
                    self.flowered_kinds.push(*kind);
                }
            }
            EventKind::Drooped => self.drooped = self.drooped.saturating_add(1),
            EventKind::Tended => self.waterings = self.waterings.saturating_add(1),
            EventKind::Bloomed => self.blooms = self.blooms.saturating_add(1),
            EventKind::FellAsleep => self.naps = self.naps.saturating_add(1),
            EventKind::Morning => self.nights = self.nights.saturating_add(1),
            EventKind::RainStarted => self.showers = self.showers.saturating_add(1),
            EventKind::StormStarted => self.storms = self.storms.saturating_add(1),
            EventKind::SnowStarted => self.snowfalls = self.snowfalls.saturating_add(1),
            EventKind::Noticed(thing) if thing.starts_with("the rainbow") => {
                self.rainbows = self.rainbows.saturating_add(1)
            }
            EventKind::ShootingStar => {
                self.shooting_stars = self.shooting_stars.saturating_add(1);
            }
            EventKind::ForecastCameTrue { snow: false } => {
                self.forecasts = self.forecasts.saturating_add(1)
            }
            EventKind::ForecastCameTrue { snow: true } => {
                self.snow_forecasts = self.snow_forecasts.saturating_add(1)
            }
            EventKind::Noticed(thing) if thing == "a bird at the window" => {
                self.birds = self.birds.saturating_add(1)
            }
            EventKind::Noticed(thing) if thing == "a moth at the lamp" => {
                self.moths = self.moths.saturating_add(1)
            }
            EventKind::Heard(_) => self.heard = self.heard.saturating_add(1),
            EventKind::Scolded => self.scolds = self.scolds.saturating_add(1),
            EventKind::Praised => self.praises = self.praises.saturating_add(1),
            EventKind::LearnedWord { word, .. } => {
                self.learned = self.learned.saturating_add(1);
                if self.words.len() < WORDS_NAMED && !self.words.contains(word) {
                    self.words.push(word.clone());
                }
            }
            EventKind::Poked { .. } => self.pokes = self.pokes.saturating_add(1),
            EventKind::Petted => self.pets = self.pets.saturating_add(1),
            EventKind::Kept(gift) => {
                self.gifts = self.gifts.saturating_add(1);
                if !self.keepsakes.contains(gift) {
                    self.keepsakes.push(*gift);
                }
            }
            EventKind::ExpectedArrival => self.expected = self.expected.saturating_add(1),
            EventKind::MissedVisit => self.missed = self.missed.saturating_add(1),
            EventKind::FriendArrived(name) => {
                self.friend_visits = self.friend_visits.saturating_add(1);
                self.friend_name = Some(name.clone());
            }
            EventKind::PlayedWithFriend(game) => {
                self.friend_games = self.friend_games.saturating_add(1);
                if !self.friend_played.contains(game) {
                    self.friend_played.push(*game);
                }
            }
            EventKind::HeardPlan { errand, .. } => {
                self.plans_heard = self.plans_heard.saturating_add(1);
                if !self.heard_plans.contains(errand) {
                    self.heard_plans.push(*errand);
                }
            }
            EventKind::WishedLuck(errand) => {
                self.luck_wished = self.luck_wished.saturating_add(1);
                if !self.wished_plans.contains(errand) {
                    self.wished_plans.push(*errand);
                }
            }
            EventKind::PlanWentWell(errand) => {
                self.plans_went_well = self.plans_went_well.saturating_add(1);
                if !self.good_plans.contains(errand) {
                    self.good_plans.push(*errand);
                }
            }
            EventKind::PlanWentBadly(errand) => {
                self.plans_went_badly = self.plans_went_badly.saturating_add(1);
                if !self.hard_plans.contains(errand) {
                    self.hard_plans.push(*errand);
                }
            }
            EventKind::FoundTreasure {
                seconds,
                gave_up: false,
                ..
            } => {
                self.treasures_found = self.treasures_found.saturating_add(1);
                self.fastest_treasure = Some(
                    self.fastest_treasure
                        .map_or(*seconds, |fastest| fastest.min(*seconds)),
                );
            }
            EventKind::TreasureHint(_) => {
                self.treasure_hints = self.treasure_hints.saturating_add(1)
            }
            EventKind::SuggestedBreak { .. } => {
                self.breaks_suggested = self.breaks_suggested.saturating_add(1)
            }
            EventKind::TookBreak => self.breaks_taken = self.breaks_taken.saturating_add(1),
            EventKind::NoticedLateNight => self.late_nights = self.late_nights.saturating_add(1),
            EventKind::SaidGoodnightTogether => {
                self.goodnights_together = self.goodnights_together.saturating_add(1)
            }
            EventKind::FoldedCrane { .. } => {
                self.cranes = self.cranes.saturating_add(1).min(super::cranes::MAXIMUM)
            }
            EventKind::Photographed { posed } => {
                self.photos = self.photos.saturating_add(1);
                if *posed {
                    self.portraits = self.portraits.saturating_add(1);
                }
            }
            EventKind::BlewBubbles {
                blown,
                popped_by_viewer,
            } => {
                self.bubble_sessions = self.bubble_sessions.saturating_add(1);
                self.bubbles_blown = self.bubbles_blown.saturating_add(*blown);
                self.bubbles_popped = self.bubbles_popped.saturating_add(*popped_by_viewer);
            }
            EventKind::Pinned { kind, name } => {
                if *kind == super::gallery::Kind::OwnDoodle
                    && self.pinned_doodles.len() < 8
                    && !self.pinned_doodles.contains(name)
                {
                    self.pinned_doodles.push(name.clone());
                }
                let count = match kind {
                    super::gallery::Kind::ViewerDrawing => &mut self.viewer_pins,
                    super::gallery::Kind::OwnDoodle => &mut self.doodle_pins,
                    super::gallery::Kind::Constellation => &mut self.constellation_pins,
                };
                *count = count.saturating_add(1).min(100_000_000);
            }
            _ => {}
        }
    }

    /// Times it set out on `activity`.
    pub fn times(&self, activity: Activity) -> u32 {
        self.activities.get(activity.name()).copied().unwrap_or(0)
    }

    /// Everything that happened in a few sentences, as the ghost would tell
    /// the viewer what it has been up to.
    pub fn words(&self) -> String {
        let deeds: Vec<String> = self.deeds().into_iter().map(|(_, deed, _)| deed).collect();
        self.written(&deeds)
    }

    /// The day as the ghost writes it in its diary: only what stood out
    /// most, the most notable first, then how the viewer spent it.
    pub fn diary(&self) -> String {
        let mut deeds = self.deeds();
        // A stable sort keeps deeds of equal weight in their usual order.
        deeds.sort_by(|(first, _, _), (second, _, _)| second.cmp(first));
        deeds.truncate(DIARY_DEEDS);
        let deeds: Vec<String> = deeds.into_iter().map(|(_, deed, _)| deed).collect();
        self.written(&deeds)
    }

    pub fn words_korean(&self) -> String {
        let deeds = self
            .deeds()
            .into_iter()
            .map(
                |(_, english, korean)| {
                    if korean.is_empty() { english } else { korean }
                },
            )
            .collect::<Vec<_>>();
        self.written_korean(&deeds)
    }

    pub fn diary_korean(&self) -> String {
        let deeds = self
            .top_deeds("", DIARY_DEEDS)
            .into_iter()
            .map(|entry| entry.in_language(true).to_owned())
            .collect::<Vec<_>>();
        self.written_korean(&deeds)
    }

    pub(super) fn top_deeds(&self, day: &str, limit: usize) -> Vec<Entry> {
        let mut deeds = self.deeds();
        deeds.sort_by(|(first, _, _), (second, _, _)| second.cmp(first));
        deeds
            .into_iter()
            .take(limit)
            .map(|(_, text, korean)| Entry {
                day: day.to_owned(),
                text,
                korean: (!korean.is_empty()).then_some(korean),
            })
            .collect()
    }

    pub fn recall(&self, korean: bool) -> String {
        let deeds = self
            .top_deeds("", 3)
            .iter()
            .map(|entry| entry.in_language(korean).to_owned())
            .collect::<Vec<_>>();
        recalled_deeds(&deeds, korean)
    }

    pub(super) fn remember_writing(&mut self, english: String, korean: String) {
        if self.korean_details.len() >= 64 && !self.korean_details.contains_key(&english) {
            self.korean_details.pop_first();
        }
        self.korean_details.insert(english, korean);
    }

    fn korean_detail<'a>(&'a self, text: &'a str) -> &'a str {
        self.korean_details.get(text).map_or(text, String::as_str)
    }

    // Each deed keeps its salience and both languages. An empty Korean side
    // falls back to English, including deeds added by newer features.
    fn deeds(&self) -> Vec<(Salience, String, String)> {
        let mut deeds: Vec<(Salience, String, String)> = self
            .occasions
            .iter()
            .map(|occasion| {
                (
                    Salience::Momentous,
                    occasion.diary_words().to_owned(),
                    format!(
                        "{}{} 기념했어",
                        super::writing::occasion_korean(*occasion),
                        super::writing::particle(
                            super::writing::occasion_korean(*occasion),
                            "을",
                            "를"
                        )
                    ),
                )
            })
            .collect();
        deeds.extend(self.temperament_changes.iter().map(|line| {
            (
                Salience::Momentous,
                line.trim_end_matches('.').to_owned(),
                super::upbringing::korean_change(line).to_owned(),
            )
        }));
        if self.scolds > 0 {
            deeds.push((
                Salience::Small,
                "you told me no".to_owned(),
                "안 된다는 말을 들었어".to_owned(),
            ));
        }
        if self.treasures_found > 0 {
            deeds.push((
                Salience::Notable,
                if self.fastest_treasure.is_some_and(|seconds| seconds < 20) {
                    "found your treasure in no time".to_owned()
                } else {
                    "found the treasure you hid".to_owned()
                },
                if self.fastest_treasure.is_some_and(|seconds| seconds < 20) {
                    "네 보물을 금세 찾았어"
                } else {
                    "네가 숨긴 보물을 찾았어"
                }
                .to_owned(),
            ));
        } else if self.hunts > 0 {
            deeds.push((
                Salience::Small,
                "searched for the treasure you hid".to_owned(),
                "네가 숨긴 보물을 찾아다녔어".to_owned(),
            ));
        }
        if self.treasure_hints > 0 {
            deeds.push((
                Salience::Small,
                "followed your warmer and colder hints".to_owned(),
                "가깝고 멀다는 네 힌트를 따라갔어".to_owned(),
            ));
        }
        if self.promises_made > 0 {
            deeds.push((
                Salience::Small,
                "you told me when you would return".to_owned(),
                "언제 돌아올지 말해 줬어".to_owned(),
            ));
        }
        if self.promises_kept > self.trips_kept {
            deeds.push((
                Salience::Notable,
                "you came back when you said you would".to_owned(),
                "약속한 때에 돌아와 줬어".to_owned(),
            ));
        }
        if self.trips_kept > 0 {
            deeds.push((
                Salience::Special,
                "you came home from your trip".to_owned(),
                "네가 여행을 마치고 돌아왔어".to_owned(),
            ));
        }
        deeds.extend(self.anniversary_words.iter().map(|line| {
            let korean = self.korean_details.get(line).cloned().unwrap_or_default();
            let mut letters = line.trim_end_matches('.').chars();
            let line = letters.next().map_or_else(String::new, |first| {
                first.to_lowercase().collect::<String>() + letters.as_str()
            });
            (
                Salience::Notable,
                format!("remembered {}", line.replacen(", ", ", when ", 1)),
                korean,
            )
        }));
        if self.anniversaries > 0 && self.anniversary_words.is_empty() {
            deeds.push((
                Salience::Notable,
                "remembered a special day from my life".to_owned(),
                "내 삶의 특별했던 날을 떠올렸어".to_owned(),
            ));
        }
        for feeling in &self.comforted_feelings {
            let words = if matches!(feeling, super::Feeling::Stressed | super::Feeling::Angry) {
                "we breathed together".to_owned()
            } else {
                format!("you were {}, so I {}", feeling.name(), feeling.care_words())
            };
            deeds.push((Salience::Special, words, super::writing::comfort(*feeling)));
        }
        for feeling in &self.feelings {
            if !self.comforted_feelings.contains(feeling) {
                deeds.push((
                    Salience::Notable,
                    format!("you told me you were {}", feeling.name()),
                    format!("‘{}’ 하고 말해 줬어", feeling.korean()),
                ));
            }
        }
        if self.felt_better > 0 {
            deeds.push((
                Salience::Special,
                "you told me you were feeling better".to_owned(),
                "기분이 나아졌다고 말해 줬어".to_owned(),
            ));
        }
        deeds.extend(super::Quirk::ALL.into_iter().filter_map(|kind| {
            self.quirks
                .get(&kind)
                .filter(|count| **count > 0)
                .map(|count| {
                    (
                        Salience::Small,
                        kind.diary(*count),
                        super::writing::quirk(kind, *count),
                    )
                })
        }));
        if self.letters_received > 0 {
            deeds.push((
                Salience::Special,
                "received your letter".to_owned(),
                "네 편지를 받았어".to_owned(),
            ));
        }
        if self.viewer_letters_read > 0 {
            deeds.push((
                Salience::Special,
                "read your letter".to_owned(),
                "네 편지를 읽었어".to_owned(),
            ));
        }
        for title in &self.composition_titles {
            deeds.push((
                Salience::Special,
                format!("made up a tune called “{title}”"),
                format!(
                    "《{}》{} 곡을 만들었어",
                    self.korean_detail(title),
                    super::writing::particle(self.korean_detail(title), "이라는", "라는")
                ),
            ));
        }
        for title in &self.written_titles {
            deeds.push((
                Salience::Momentous,
                format!("wrote my own picture book, “{title}”"),
                format!(
                    "내 그림책 《{}》{} 썼어",
                    self.korean_detail(title),
                    super::writing::particle(self.korean_detail(title), "을", "를")
                ),
            ));
        }
        if self.songs_sung_back > 0 {
            deeds.push((
                Salience::Special,
                "heard you sing my own song back to me".to_owned(),
                "네가 내 노래를 다시 불러 줬어".to_owned(),
            ));
        }
        if self.letters_written > 0 {
            deeds.push((
                Salience::Special,
                "wrote you a letter".to_owned(),
                "네게 편지를 썼어".to_owned(),
            ));
        }
        if self.letters_read > 0 {
            deeds.push((
                Salience::Special,
                "watched you read my letter".to_owned(),
                "네가 내 편지를 읽는 모습을 봤어".to_owned(),
            ));
        }
        if self.outfits > 0 {
            deeds.push((
                Salience::Small,
                "wore a little accessory".to_owned(),
                "작은 장신구를 달았어".to_owned(),
            ));
        }
        if self.pages > 0 {
            deeds.push((
                Salience::Notable,
                format!(
                    "read {} {}",
                    count_words(self.pages),
                    if self.pages == 1 { "page" } else { "pages" }
                ),
                format!("책을 {}쪽 읽었어", self.pages),
            ));
        }
        if self.books_finished > 0 {
            deeds.push((
                Salience::Special,
                format!(
                    "finished {} picture {}",
                    count_words(self.books_finished),
                    if self.books_finished == 1 {
                        "book"
                    } else {
                        "books"
                    }
                ),
                format!("그림책 {}권을 끝까지 읽었어", self.books_finished),
            ));
        }
        if self.books_received > 0 {
            deeds.push((
                Salience::Special,
                format!(
                    "received {} new picture {}",
                    count_words(self.books_received),
                    if self.books_received == 1 {
                        "book"
                    } else {
                        "books"
                    }
                ),
                format!("새 그림책 {}권을 받았어", self.books_received),
            ));
        }
        deeds.extend(self.wish_words.iter().map(|words| {
            let salience = if words.starts_with("made a wish") {
                Salience::Notable
            } else if words.contains("came true") {
                Salience::Special
            } else {
                Salience::Small
            };
            (
                salience,
                words.clone(),
                self.korean_details.get(words).cloned().unwrap_or_default(),
            )
        }));
        if self.focus_sessions > 0 {
            deeds.push((
                Salience::Special,
                format!(
                    "kept you company through {} focus {}",
                    count_words(self.focus_sessions),
                    if self.focus_sessions == 1 {
                        "session"
                    } else {
                        "sessions"
                    },
                ),
                format!(
                    "네가 집중하는 동안 {}번 곁에 있어 줬어",
                    self.focus_sessions
                ),
            ));
        }
        if self.airings > 0 {
            deeds.push((
                Salience::Small,
                "opened the window for fresh air".to_owned(),
                "창문을 열어 바깥 공기를 마셨어".to_owned(),
            ));
        }
        if self.focus_minutes > 0 {
            deeds.push((
                Salience::Small,
                format!(
                    "focused together for {} {}",
                    count_words(self.focus_minutes),
                    if self.focus_minutes == 1 {
                        "minute"
                    } else {
                        "minutes"
                    }
                ),
                format!("{}분 동안 같이 집중했어", self.focus_minutes),
            ));
        }
        if let Some(name) = &self.friend_name {
            let visit = match self.friend_visits {
                0 => String::new(),
                1 => format!("{name} came to visit"),
                2 => format!("{name} visited twice"),
                count => format!("{name} visited {} times", count_words(count)),
            };
            if !visit.is_empty() {
                let games: Vec<String> = self
                    .friend_played
                    .iter()
                    .map(|game| game.words().to_owned())
                    .collect();
                deeds.push((
                    Salience::Special,
                    if games.is_empty() {
                        visit
                    } else {
                        format!("{visit} and we played {}", listed(&games))
                    },
                    super::writing::friend_visit(self, name),
                ));
            }
        }
        match self.motes {
            0 => {}
            1 => deeds.push((
                Salience::Routine,
                "ate a light mote".to_owned(),
                "빛 알갱이 하나를 먹었어".to_owned(),
            )),
            motes => deeds.push((
                Salience::Routine,
                format!("ate {} light motes", count_words(motes)),
                format!("빛 알갱이 {motes}개를 먹었어"),
            )),
        }
        match self.nudges {
            0 => {}
            1 => deeds.push((
                Salience::Routine,
                "nudged the ball".to_owned(),
                "공을 밀었어".to_owned(),
            )),
            nudges => deeds.push((
                Salience::Routine,
                format!("pushed the ball around {} times", count_words(nudges)),
                format!("공을 {nudges}번 굴렸어"),
            )),
        }
        match self.fetches {
            0 => {}
            1 => deeds.push((
                Salience::Notable,
                "fetched the ball for you".to_owned(),
                "네게 공을 가져다줬어".to_owned(),
            )),
            fetches => deeds.push((
                Salience::Notable,
                format!("fetched the ball for you {} times", count_words(fetches)),
                format!("네게 공을 {fetches}번 가져다줬어"),
            )),
        }
        match self.pounces {
            0 => {}
            1 => deeds.push((
                Salience::Small,
                "pounced on your finger".to_owned(),
                "네 손가락에 폴짝 뛰어들었어".to_owned(),
            )),
            pounces => deeds.push((
                Salience::Small,
                format!("pounced on your finger {} times", count_words(pounces)),
                format!("네 손가락에 {pounces}번 폴짝 뛰어들었어"),
            )),
        }
        let tower = match self.towers {
            0 => None,
            1 => Some("built a tower of blocks".to_owned()),
            towers => Some(format!("built {} towers of blocks", count_words(towers))),
        };
        if let Some(mut tower) = tower {
            match self.toppled {
                0 => {}
                1 => tower.push_str(", and you knocked one down"),
                toppled => {
                    tower.push_str(&format!(", and you knocked {} down", count_words(toppled)))
                }
            }
            deeds.push((
                Salience::Notable,
                tower,
                if self.toppled > 0 {
                    format!(
                        "블록 탑을 {}개 쌓았고, 네가 {}개를 무너뜨렸어",
                        self.towers, self.toppled
                    )
                } else {
                    format!("블록 탑을 {}개 쌓았어", self.towers)
                },
            ));
        } else if self.toppled > 0 {
            deeds.push((
                Salience::Small,
                "watched you knock my tower down".to_owned(),
                "네가 내 탑을 무너뜨리는 모습을 봤어".to_owned(),
            ));
        }
        if self.learned_about > 0 {
            deeds.push((
                Salience::Special,
                "learned a little more about you".to_owned(),
                "너에 대해 조금 더 알게 됐어".to_owned(),
            ));
        }
        if self.moon_wishes > 0 {
            deeds.push((
                Salience::Special,
                "made a wish on the Chuseok moon".to_owned(),
                "추석 달을 보며 소원을 빌었어".to_owned(),
            ));
        }
        if self.constellations > 0 {
            deeds.push((
                Salience::Special,
                if self.constellations == 1 {
                    "found a picture in the stars".to_owned()
                } else {
                    "found new constellations in the stars".to_owned()
                },
                "별들 사이에서 새로운 그림을 찾았어".to_owned(),
            ));
        }
        match self.dances {
            0 => {}
            1 => deeds.push((
                Salience::Small,
                "danced a twirl".to_owned(),
                "빙글 돌며 춤췄어".to_owned(),
            )),
            dances => deeds.push((
                Salience::Small,
                format!("danced {} twirls", count_words(dances)),
                format!("{dances}번 빙글 돌며 춤췄어"),
            )),
        }
        match (self.finds, self.jumps) {
            (0, 0) => {}
            (0, _) => deeds.push((
                Salience::Notable,
                "played hide-and-seek, and you never found me".to_owned(),
                "숨바꼭질을 했는데 네가 나를 못 찾았어".to_owned(),
            )),
            (_, 0) => deeds.push((
                Salience::Notable,
                "played hide-and-seek, and you found me".to_owned(),
                "숨바꼭질을 했고 네가 나를 찾았어".to_owned(),
            )),
            _ => deeds.push((
                Salience::Notable,
                "played hide-and-seek".to_owned(),
                "숨바꼭질을 했어".to_owned(),
            )),
        }
        if self.games == 1 {
            let result = if self.wins > 0 {
                "and won"
            } else if self.losses > 0 {
                "and you won"
            } else {
                "and drew"
            };
            deeds.push((
                Salience::Notable,
                format!("played noughts and crosses with you {result}"),
                if self.wins > 0 {
                    "너랑 틱택토를 해서 이겼어"
                } else if self.losses > 0 {
                    "너랑 틱택토를 했고 네가 이겼어"
                } else {
                    "너랑 틱택토를 해서 비겼어"
                }
                .to_owned(),
            ));
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
            deeds.push((
                Salience::Notable,
                format!(
                    "played games of noughts and crosses with you: {}",
                    listed(&results)
                ),
                format!(
                    "너랑 틱택토를 했어. 나는 {}번, 너는 {}번 이겼고, {}번 비겼어",
                    self.wins, self.losses, self.draws
                ),
            ));
        }
        match self.doodles {
            0 => {}
            1 => deeds.push((
                Salience::Small,
                "drew a picture on the glass".to_owned(),
                "유리에 그림을 그렸어".to_owned(),
            )),
            doodles => deeds.push((
                Salience::Small,
                format!("drew {} pictures on the glass", count_words(doodles)),
                format!("유리에 그림을 {doodles}개 그렸어"),
            )),
        }
        if self.drawings_seen > 0 {
            deeds.push((
                Salience::Notable,
                "looked at the pictures you drew on the glass".to_owned(),
                "네가 유리에 그린 그림을 봤어".to_owned(),
            ));
        }
        if self.pictures_learned > 0 {
            deeds.push((
                Salience::Special,
                "learned new pictures from you".to_owned(),
                "네게 새로운 그림을 배웠어".to_owned(),
            ));
        }
        for word in &self.written_words {
            deeds.push((
                Salience::Notable,
                format!(
                    "wrote “{}” on the glass for you",
                    super::doodle::writing_words(word)
                ),
                format!(
                    "네가 읽을 수 있게 유리에 ‘{word}’{} 썼어",
                    super::writing::particle(word, "이라고", "라고")
                ),
            ));
        }
        if self.writings > 0 && self.written_words.is_empty() {
            deeds.push((
                Salience::Small,
                "wrote words on the glass for you".to_owned(),
                "네가 읽을 수 있게 유리에 글씨를 썼어".to_owned(),
            ));
        }
        for name in &self.viewer_names {
            deeds.push((
                Salience::Momentous,
                format!("learned your name: {name}"),
                format!("네 이름을 알게 됐어: {name}"),
            ));
        }
        match (self.waterings, self.blooms) {
            (0, 0) => {}
            (0, _) => deeds.push((
                Salience::Notable,
                "saw the plant flower".to_owned(),
                "화분에 꽃이 피는 걸 봤어".to_owned(),
            )),
            (_, 0) => deeds.push((
                Salience::Small,
                "watered the plant".to_owned(),
                "화분에 물을 줬어".to_owned(),
            )),
            _ => deeds.push((
                Salience::Notable,
                "watered the plant until it flowered".to_owned(),
                "화분에 물을 주고 꽃이 피는 걸 봤어".to_owned(),
            )),
        }
        for kind in &self.planted_kinds {
            deeds.push((
                Salience::Special,
                format!("planted a {} seed", kind.name()),
                format!("{} 씨앗을 심었어", kind.korean()),
            ));
        }
        for kind in &self.sprouted_kinds {
            deeds.push((
                Salience::Special,
                format!("saw the {} sprout", kind.name()),
                format!("{} 싹이 나는 걸 봤어", kind.korean()),
            ));
        }
        for kind in &self.flowered_kinds {
            deeds.push((
                Salience::Momentous,
                format!("the {} came into flower", kind.name()),
                format!("{} 꽃이 피었어", kind.korean()),
            ));
        }
        if self.drooped > 0 {
            deeds.push((
                Salience::Notable,
                "noticed the plant drooping from thirst".to_owned(),
                "목말라 시든 화분을 발견했어".to_owned(),
            ));
        }
        match self.naps {
            0 => {}
            1 => deeds.push((
                Salience::Small,
                "took a nap".to_owned(),
                "낮잠을 잤어".to_owned(),
            )),
            naps => deeds.push((
                Salience::Small,
                format!("took {} naps", count_words(naps)),
                format!("낮잠을 {naps}번 잤어"),
            )),
        }
        if self.nights > 0 {
            deeds.push((
                Salience::Small,
                "slept through the night".to_owned(),
                "밤새 푹 잤어".to_owned(),
            ));
        }
        if self.snowfalls > 0 {
            deeds.push((
                Salience::Special,
                "watched the snow fall".to_owned(),
                "눈이 내리는 걸 봤어".to_owned(),
            ));
        }
        if self.birds > 0 {
            deeds.push((
                Salience::Small,
                "watched a bird at the window".to_owned(),
                "창가에서 새를 구경했어".to_owned(),
            ));
        }
        if self.gifts > 0 {
            let gifts: Vec<String> = self
                .keepsakes
                .iter()
                .map(|gift| gift.name().to_owned())
                .collect();
            let gifts = if gifts.is_empty() {
                "a little gift".to_owned()
            } else {
                listed(&gifts)
            };
            deeds.push((
                Salience::Special,
                if self.gifts == 1 {
                    format!("the bird brought me {gifts}")
                } else {
                    format!("the bird brought me gifts, including {gifts}")
                },
                format!(
                    "새가 선물을 가져다줬어: {}",
                    self.keepsakes
                        .iter()
                        .map(|gift| gift.korean())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        if self.moths > 0 {
            deeds.push((
                Salience::Small,
                "saw a moth at the lamp".to_owned(),
                "등불 곁에서 나방을 봤어".to_owned(),
            ));
        }
        if self.expected > 0 {
            deeds.push((
                Salience::Notable,
                "you came at your usual time".to_owned(),
                "네가 늘 오는 시간에 왔어".to_owned(),
            ));
        }
        if self.missed > 0 {
            deeds.push((
                Salience::Notable,
                "waited for you at your usual time, but you didn't come".to_owned(),
                "네가 늘 오는 시간에 기다렸지만 오지 않았어".to_owned(),
            ));
        }
        if self.storms > 0 {
            deeds.push((
                Salience::Notable,
                "sat out a thunderstorm".to_owned(),
                "천둥이 치는 동안 가만히 기다렸어".to_owned(),
            ));
        }
        if self.forecasts > 0 {
            deeds.push((
                Salience::Notable,
                "felt the rain coming before it fell".to_owned(),
                "비가 오기 전에 느꼈어".to_owned(),
            ));
        }
        if self.snow_forecasts > 0 {
            deeds.push((
                Salience::Notable,
                "smelled the snow coming before it fell".to_owned(),
                "눈이 오기 전에 냄새를 맡았어".to_owned(),
            ));
        }
        if !self.words.is_empty() {
            let quoted: Vec<String> = self.words.iter().map(|word| format!("“{word}”")).collect();
            deeds.push((
                Salience::Special,
                match quoted.as_slice() {
                    [word] => format!("learned what {word} means"),
                    words => format!("learned what {} mean", listed(words)),
                },
                format!("{}의 뜻을 배웠어", quoted.join(", ")),
            ));
        }
        if self.passersby > 0 {
            if self.passerby_kinds.is_empty() {
                deeds.push((
                    Salience::Notable,
                    "watched passing life outside the window".to_owned(),
                    "창밖으로 지나가는 친구들을 구경했어".to_owned(),
                ));
            } else {
                let kinds: Vec<String> = self
                    .passerby_kinds
                    .iter()
                    .map(|kind| kind.name().to_owned())
                    .collect();
                deeds.push((
                    Salience::Notable,
                    format!("saw {} through the window", listed(&kinds)),
                    format!(
                        "창밖에서 {} 구경했어",
                        self.passerby_kinds
                            .iter()
                            .map(|kind| format!(
                                "{}{}",
                                super::writing::passerby(*kind),
                                super::writing::particle(
                                    super::writing::passerby(*kind),
                                    "을",
                                    "를"
                                )
                            ))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
            }
        }
        match self.shooting_stars {
            0 => {}
            1 => deeds.push((
                Salience::Notable,
                "wished on a shooting star".to_owned(),
                "별똥별에 소원을 빌었어".to_owned(),
            )),
            stars => deeds.push((
                Salience::Notable,
                format!("wished on {} shooting stars", count_words(stars)),
                format!("별똥별 {stars}개에 소원을 빌었어"),
            )),
        }
        if self.word_games > 0 {
            deeds.push((
                Salience::Notable,
                format!(
                    "played {} word {} with you; you guessed {}",
                    count_words(self.word_games),
                    if self.word_games == 1 {
                        "game"
                    } else {
                        "games"
                    },
                    count_words(self.words_guessed)
                ),
                format!(
                    "너랑 낱말 놀이를 {}번 했고, 네가 {}개를 맞혔어",
                    self.word_games, self.words_guessed
                ),
            ));
        }
        if self.tunes_played > 0 {
            deeds.push((
                Salience::Small,
                format!(
                    "played {} {} on the music box",
                    count_words(self.tunes_played),
                    if self.tunes_played == 1 {
                        "tune"
                    } else {
                        "tunes"
                    }
                ),
                format!("오르골로 노래를 {}곡 틀었어", self.tunes_played),
            ));
        }
        if self.tunes_learned > 0 {
            deeds.push((
                Salience::Special,
                "learned a tune from you".to_owned(),
                "네게 노래를 배웠어".to_owned(),
            ));
        }
        if self.mirror_visits > 0 {
            deeds.push((
                Salience::Small,
                "looked in the mirror".to_owned(),
                "거울을 들여다봤어".to_owned(),
            ));
        }
        if self.met_reflection {
            deeds.push((
                Salience::Special,
                "met the ghost in the mirror".to_owned(),
                "거울 속 유령을 만났어".to_owned(),
            ));
        }
        if self.recognized_self {
            deeds.push((
                Salience::Momentous,
                "learned the ghost in the mirror is me".to_owned(),
                "거울 속 유령이 나라는 걸 알게 됐어".to_owned(),
            ));
        }
        if self.lullaby_sleeps > 0 {
            deeds.push((
                Salience::Small,
                "fell asleep to a lullaby".to_owned(),
                "자장가를 들으며 잠들었어".to_owned(),
            ));
        }
        match (self.photos, self.portraits) {
            (0, _) => {}
            (1, 1) => deeds.push((
                Salience::Small,
                "posed for your camera".to_owned(),
                "네 카메라 앞에서 포즈를 잡았어".to_owned(),
            )),
            (1, _) => deeds.push((
                Salience::Small,
                "you took my picture".to_owned(),
                "네가 내 사진을 찍어 줬어".to_owned(),
            )),
            (count, _) => deeds.push((
                Salience::Small,
                format!("you took {} pictures of me", count_words(count)),
                format!("네가 내 사진을 {count}장 찍어 줬어"),
            )),
        }
        match self.stories {
            0 => {}
            1 => deeds.push((
                Salience::Special,
                "told you a story".to_owned(),
                "네게 이야기를 들려줬어".to_owned(),
            )),
            count => deeds.push((
                Salience::Special,
                format!("told you {} stories", count_words(count)),
                format!("네게 이야기를 {count}번 들려줬어"),
            )),
        }
        match (self.rainbows, self.showers) {
            (0, 0) => {}
            (0, _) => deeds.push((
                Salience::Routine,
                "listened to the rain".to_owned(),
                "빗소리를 들었어".to_owned(),
            )),
            (1, _) => deeds.push((
                Salience::Special,
                "saw a rainbow".to_owned(),
                "무지개를 봤어".to_owned(),
            )),
            (rainbows, _) => deeds.push((
                Salience::Special,
                format!("saw {} rainbows", count_words(rainbows)),
                format!("무지개를 {rainbows}번 봤어"),
            )),
        }
        deeds.extend(self.heard_plans.iter().map(|errand| {
            (
                Salience::Small,
                format!("you told me about your {}", errand.name()),
                format!("네 {} 이야기를 들었어", errand.korean()),
            )
        }));
        deeds.extend(self.wished_plans.iter().map(|errand| {
            (
                Salience::Notable,
                format!("wished you luck for your {}", errand.name()),
                format!("네 {} 잘 되길 빌었어", errand.korean()),
            )
        }));
        deeds.extend(self.good_plans.iter().map(|errand| {
            (
                Salience::Special,
                format!("your {} went well", errand.name()),
                format!("네 {} 잘 됐어", errand.korean()),
            )
        }));
        deeds.extend(self.hard_plans.iter().map(|errand| {
            (
                Salience::Notable,
                format!("your {} was hard, so I stayed close", errand.name()),
                format!("네 {} 힘들었다고 해서 곁에 있었어", errand.korean()),
            )
        }));
        if self.breaks_suggested > 0 {
            deeds.push((
                Salience::Small,
                format!(
                    "suggested {} {}",
                    count_words(self.breaks_suggested),
                    if self.breaks_suggested == 1 {
                        "break"
                    } else {
                        "breaks"
                    }
                ),
                format!("쉬자고 {}번 말했어", self.breaks_suggested),
            ));
        }
        if self.breaks_taken > 0 {
            deeds.push((
                Salience::Notable,
                "you took a break when I asked".to_owned(),
                "내 말에 네가 잠깐 쉬었어".to_owned(),
            ));
        }
        if self.late_nights > 0 {
            deeds.push((
                Salience::Small,
                "you stayed up very late".to_owned(),
                "네가 아주 늦게까지 깨어 있었어".to_owned(),
            ));
        }
        if self.goodnights_together > 0 {
            deeds.push((
                Salience::Notable,
                "we said goodnight together late at night".to_owned(),
                "늦은 밤에 같이 잘 자라고 인사했어".to_owned(),
            ));
        }
        if self.cranes > 0 {
            deeds.push((
                Salience::Small,
                if self.cranes == 1 {
                    "folded a paper crane".to_owned()
                } else {
                    format!("folded {} paper cranes", count_words(self.cranes))
                },
                format!("종이학을 {}마리 접었어", self.cranes),
            ));
        }
        if self.cranes_for_sad_viewer > 0 {
            deeds.push((
                Salience::Notable,
                "folded a crane for you when you were sad".to_owned(),
                "네가 슬플 때 종이학을 접어 줬어".to_owned(),
            ));
        }
        if self.bubble_sessions > 0 {
            deeds.push(if self.bubbles_popped > 0 {
                (
                    Salience::Notable,
                    "blew bubbles and you popped them".to_owned(),
                    "비눗방울을 불었고 네가 톡톡 터뜨려 줬어".to_owned(),
                )
            } else {
                (
                    Salience::Small,
                    "blew bubbles".to_owned(),
                    "비눗방울을 불었어".to_owned(),
                )
            });
        }
        if self.viewer_pins > 0 {
            deeds.push((
                Salience::Notable,
                "pinned your drawing to my wall".to_owned(),
                "네 그림을 내 벽에 붙였어".to_owned(),
            ));
        }
        if self.doodle_pins > 0 {
            if self.pinned_doodles.is_empty() {
                deeds.push((
                    Salience::Small,
                    "pinned my doodle to the wall".to_owned(),
                    "내 그림을 벽에 붙였어".to_owned(),
                ));
            } else {
                for name in &self.pinned_doodles {
                    deeds.push((
                        Salience::Small,
                        format!("pinned my {name} doodle to the wall"),
                        format!("내가 그린 {} 그림을 벽에 붙였어", sketch_korean(name)),
                    ));
                }
            }
        }
        if self.constellation_pins > 0 {
            deeds.push((
                Salience::Small,
                "pinned my constellation to the wall".to_owned(),
                "내가 찾은 별자리를 벽에 붙였어".to_owned(),
            ));
        }
        deeds
    }

    fn written_korean(&self, deeds: &[String]) -> String {
        let mut text = super::writing::sentence(deeds);
        if self.together < 60.0 && self.heard == 0 && self.expected == 0 {
            text.push_str(" 오늘은 네가 오지 않았어.");
        } else {
            text.push_str(&format!(
                " 오늘은 {} 같이 있었{}.",
                super::writing::duration(self.together),
                if self.heard > 0 {
                    "고, 이야기도 나눴어"
                } else {
                    "어"
                }
            ));
        }
        text.push_str(match self.pets {
            0 => "",
            1 => " 한 번 쓰다듬어 줬어.",
            2..=4 => " 몇 번 쓰다듬어 줬어.",
            _ => " 많이 쓰다듬어 줬어.",
        });
        if self.pokes >= 6 {
            text.push_str(" 콕콕 많이 찔렀어.");
        }
        text
    }

    // The deeds as a sentence, then the viewer's part in the day.
    fn written(&self, deeds: &[String]) -> String {
        let mut text = if deeds.is_empty() {
            "A quiet day.".to_owned()
        } else {
            format!("{}.", capitalized(&listed(deeds)))
        };
        if self.together < 60.0 && self.heard == 0 && self.expected == 0 {
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
        match self.pets {
            0 => {}
            1 => text.push_str(" You stroked me once."),
            pets if pets < 5 => text.push_str(" You stroked me a few times."),
            _ => text.push_str(" You stroked me so often."),
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
    #[serde(default)]
    pub korean: Option<String>,
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
    /// The small changes that shared days have made to its inborn nature.
    pub upbringing: Upbringing,
    /// How close it feels to the viewer, from a stranger at 0 to devoted
    /// at 1.
    pub bond: f32,
    /// The closest it has ever felt. Half of that never fades.
    pub closest: f32,
    /// Warmth the bond gathered today, which makes more of it harder to
    /// gather.
    #[serde(default)]
    pub warmed: f32,
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
    /// Last local days of anniversary speech and a life-landmark greeting.
    pub anniversary_said: Option<String>,
    pub landmark_greeted: Option<String>,
    /// Seconds since the viewer was last around, while the app ran and
    /// while it was closed.
    pub alone: f64,
    /// The viewer's most recent promise of when to return.
    #[serde(default, deserialize_with = "super::promise::deserialize")]
    pub promise: Option<super::promise::Promise>,
    /// The viewer's words it has picked up, and what it thinks they mean.
    pub vocabulary: Vocabulary,
    /// How safe the bird feels with the ghost, from wary to trusting.
    #[serde(default)]
    pub bird_trust: f32,
    /// The shelf's newest eight gifts, oldest first.
    #[serde(default)]
    pub keepsakes: Vec<Keepsake>,
    /// A gift still waiting outside, even while the app is closed.
    #[serde(default)]
    pub sill: Option<Keepsake>,
    /// The local day, as YYYY-MM-DD, the bird last decided whether to
    /// leave a gift, and whether one is still to come that day.
    #[serde(default)]
    pub gift_decision: Option<(String, bool)>,
    /// The day a gift was last left, as saves briefly kept it before
    /// `gift_decision`; read once and folded into it.
    #[serde(default, skip_serializing)]
    pub gift_day: Option<String>,
    /// The local evening the friend last decided whether to come, and
    /// whether it would.
    #[serde(default)]
    pub friend_decision: Option<(String, bool)>,
    /// Arrivals at each local hour, with older days gradually fading.
    #[serde(default)]
    pub arrivals: [f32; 24],
    /// Hours counted today, at most once each.
    #[serde(default)]
    pub arrived_hours: u32,
    /// Different days on which the viewer has arrived.
    #[serde(default)]
    pub days_seen: u32,
    /// Days of learning, decayed in step with the arrivals.
    #[serde(default)]
    pub routine_days: f32,
    /// The last arrival's local day and hour, including an early arrival
    /// while the ghost was already waiting.
    #[serde(default)]
    pub last_arrival: Option<(String, f32)>,
    #[serde(default)]
    pub viewer_name: Option<String>,
    /// Completed practice, migrated from lifetime totals in older saves.
    #[serde(default = "super::Skills::from_old_save")]
    pub skills: super::Skills,
    #[serde(default)]
    pub friend: Option<super::Companion>,
    /// The share of the viewer's speech that is Korean, eased with each message.
    #[serde(default)]
    pub korean: f32,
    #[serde(default)]
    pub pictures: Vec<LearnedPicture>,
    #[serde(default)]
    pub constellations: Vec<super::stars::Constellation>,
    /// The evening of its last search, including an unsuccessful one.
    #[serde(default)]
    pub constellation_night: Option<String>,
    #[serde(default, deserialize_with = "super::wardrobe::deserialize_accessory")]
    pub wearing: Option<super::Accessory>,
    #[serde(default)]
    pub wardrobe_day: Option<String>,
    #[serde(default)]
    pub wardrobe_occasion: Option<Occasion>,
    #[serde(default)]
    pub books: super::books::Books,
    #[serde(default)]
    pub tunes: Vec<super::music::Tune>,
    #[serde(default)]
    pub compositions: Vec<super::music::Composition>,
    #[serde(default)]
    pub composition_day: Option<String>,
    #[serde(default)]
    pub written: Vec<super::creation::WrittenBook>,
    #[serde(default)]
    pub written_day: Option<String>,
    #[serde(default)]
    pub last_friend_visit: Option<String>,
    #[serde(default)]
    pub wish: Option<super::Wishing>,
    #[serde(default)]
    pub granted: Vec<super::Granted>,
    #[serde(default)]
    pub mirror: super::mirror::MirrorLife,
    pub letters: Vec<super::mail::Letter>,
    #[serde(default)]
    pub received: Vec<super::penpal::ViewerLetter>,
    pub viewer: super::acquaintance::Acquaintance,
    pub feelings: Vec<super::feelings::FeelingNote>,
    #[serde(default, deserialize_with = "super::agenda::deserialize")]
    pub agenda: Vec<super::agenda::Engagement>,
    #[serde(default)]
    pub late_night_said: Option<String>,
    #[serde(default)]
    pub goodnight_together_said: Option<String>,
    #[serde(default)]
    pub rhythm_day: Option<String>,
    #[serde(default)]
    pub rhythm_suggestions: u8,
    #[serde(default)]
    pub cranes: super::cranes::Cranes,
    #[serde(default, deserialize_with = "super::gallery::deserialize")]
    pub gallery: Vec<super::gallery::Pin>,
    #[serde(default)]
    /// The previous day's three most notable deeds, kept without parsing prose.
    pub yesterday: Vec<Entry>,
}

impl Life {
    pub(super) fn remember_day(&mut self, entry: Entry, deeds: Vec<Entry>) {
        if self
            .yesterday
            .first()
            .is_none_or(|previous| previous.day <= entry.day)
        {
            self.yesterday = deeds;
        }
        self.diary.push(entry);
        if self.diary.len() > DAYS_KEPT {
            self.diary.drain(..self.diary.len() - DAYS_KEPT);
        }
    }
    /// Quiet company slowly earns the bird's trust, marking friendship once.
    pub fn watch_bird(&mut self, seconds: f32) -> Option<String> {
        let before = self.bird_trust;
        self.bird_trust = super::visitor::quiet_trust(before, seconds);
        if before < 0.5 && self.bird_trust >= 0.5 {
            self.reach(
                "The bird trusts me now.".to_owned(),
                "이제 새가 나를 믿어 줘.",
            )
        } else {
            None
        }
    }

    /// Places a gift on the shelf, making room for the newest eight.
    pub fn keep(&mut self, gift: Keepsake) {
        self.keepsakes.push(gift);
        if self.keepsakes.len() > 8 {
            let protect_feather = self.wearing == Some(super::Accessory::Feather)
                && self
                    .keepsakes
                    .iter()
                    .filter(|gift| **gift == Keepsake::Feather)
                    .count()
                    == 1;
            let index = self
                .keepsakes
                .iter()
                .position(|gift| !protect_feather || *gift != Keepsake::Feather)
                .unwrap_or(0);
            self.keepsakes.remove(index);
        }
    }

    /// The shelf borrows one feather for the ghost without losing the gift.
    pub fn shelved_keepsakes(&self) -> impl Iterator<Item = &Keepsake> {
        let mut worn = self.wearing.and_then(super::Accessory::keepsake);
        self.keepsakes.iter().filter(move |gift| {
            if Some(**gift) == worn {
                worn = None;
                false
            } else {
                true
            }
        })
    }

    /// How the bird feels about the ghost, in its own words.
    pub fn bird_words(&self) -> &'static str {
        match self.bird_trust {
            trust if trust < 0.2 => "wary of me",
            trust if trust < 0.5 => "getting used to me",
            _ => "a friend",
        }
    }

    pub fn keepsake_words(&self) -> String {
        match self.keepsake_list() {
            Some(list) => format!("the shelf of keepsakes, with {list}"),
            None => "an empty shelf".to_owned(),
        }
    }

    /// The keepsakes on the shelf, alike ones counted together in words,
    /// such as "three feathers and an acorn", or `None` while it is empty.
    pub fn keepsake_list(&self) -> Option<String> {
        const NUMBERS: [&str; 9] = [
            "", "one", "two", "three", "four", "five", "six", "seven", "eight",
        ];
        let mut kinds: Vec<(Keepsake, usize)> = Vec::new();
        for gift in self.shelved_keepsakes() {
            match kinds.iter_mut().find(|(kind, _)| kind == gift) {
                Some((_, count)) => *count += 1,
                None => kinds.push((*gift, 1)),
            }
        }
        let names: Vec<String> = kinds
            .into_iter()
            .map(|(gift, count)| match count {
                1 => gift.name().to_owned(),
                count => format!(
                    "{} {}",
                    NUMBERS.get(count).copied().unwrap_or("many"),
                    gift.plural()
                ),
            })
            .collect();
        (!names.is_empty()).then(|| listed(&names))
    }

    /// Counts what happened, and lets it warm or cool the bond. Returns the
    /// milestone this reached, if any.
    pub fn note(&mut self, kind: &EventKind) -> Option<String> {
        if matches!(kind, EventKind::FriendArrived(_)) {
            self.last_friend_visit = Some(self.day.clone());
        }
        let title = match kind {
            EventKind::Composed(title) => self
                .compositions
                .iter()
                .find(|song| song.title == *title)
                .map(|song| (title.clone(), song.korean_title.clone())),
            EventKind::WroteBook(title) => self
                .written
                .iter()
                .find(|book| book.title.0 == *title)
                .map(|book| (title.clone(), book.title.1.clone())),
            EventKind::FoundConstellation(name) => self
                .constellations
                .iter()
                .find(|stars| stars.name == *name)
                .map(|stars| (name.clone(), stars.korean.clone())),
            _ => None,
        };
        if let Some((english, korean)) = title {
            self.today.remember_writing(english.clone(), korean.clone());
            self.total.remember_writing(english, korean);
        }
        self.skills.migrate(&self.total);
        self.total.note(kind);
        self.today.note(kind);
        if matches!(kind, EventKind::FoldedCrane { .. })
            && self
                .feelings
                .iter()
                .any(|note| note.day == self.day && note.feeling == super::Feeling::Sad)
        {
            self.today.cranes_for_sad_viewer = self
                .today
                .cranes_for_sad_viewer
                .saturating_add(1)
                .min(super::cranes::MAXIMUM);
            self.total.cranes_for_sad_viewer = self
                .total
                .cranes_for_sad_viewer
                .saturating_add(1)
                .min(super::cranes::MAXIMUM);
        }
        if let Some(friend) = &mut self.friend {
            match kind {
                EventKind::FriendArrived(_) => {
                    friend.visits = friend.visits.saturating_add(1);
                    friend.closeness = (friend.closeness + 0.05).min(1.0);
                }
                EventKind::PlayedWithFriend(_) => {
                    friend.closeness = (friend.closeness + 0.025).min(1.0)
                }
                _ => {}
            }
        }
        let bond = self.bond;
        let practiced = self.skills.note(kind);
        let milestone = self.first(kind);
        let skill_milestone = practiced.and_then(|skill| {
            (self.skills.level(skill) >= 0.65)
                .then(|| self.reach(skill.milestone().to_owned(), skill.milestone_korean()))
                .flatten()
        });
        let milestone = milestone.or(skill_milestone);
        let friendship = match &self.friend {
            Some(friend)
                if friend.closeness >= 0.5
                    && matches!(
                        kind,
                        EventKind::FriendArrived(_) | EventKind::PlayedWithFriend(_)
                    ) =>
            {
                Some((
                    format!("{} and I are good friends now.", friend.name),
                    format!("친구 {}, 이제 아주 친해졌어.", friend.name),
                ))
            }
            _ => None,
        }
        .and_then(|(text, korean)| self.reach(text, korean));
        let milestone = milestone.or(friendship);
        let milestone = if matches!(kind, EventKind::Built { height } if *height >= 5) {
            let tallest = self.reach(
                "I built a tower five blocks high.".to_owned(),
                "블록 다섯 개 높이의 탑을 쌓았어.",
            );
            milestone.or(tallest)
        } else {
            milestone
        };
        if matches!(kind, EventKind::Planted(_)) && self.total.planted >= 3 {
            self.reach(
                "My garden has three little pots.".to_owned(),
                "내 정원에 작은 화분이 세 개 생겼어.",
            );
        }
        let warmth = match kind {
            EventKind::PromiseKept(kind) if kind.trip() => 0.02,
            EventKind::PromiseKept(_) => 0.01,
            EventKind::WishGranted(wish) if wish.with_viewer() => 0.01,
            EventKind::FocusEnded {
                completed: true, ..
            } => 0.01,
            EventKind::GotLetter => 0.008,
            EventKind::ToldStory | EventKind::ReadLetter => 0.005,
            EventKind::LearnedAboutViewer(_) => 0.005,
            EventKind::SangMySong(_) => 0.01,
            EventKind::LearnedTune(_) => 0.005,
            EventKind::WordGame { .. } => 0.003,
            EventKind::SawDrawing { name: Some(name) } if name == "heart" => 0.005,
            EventKind::LearnedPicture(_) => 0.005,
            EventKind::Heard(_) => 0.015,
            EventKind::Praised | EventKind::LearnedWord { .. } | EventKind::LearnedName(_) => 0.005,
            EventKind::Named(_) => 0.01,
            EventKind::Comforted(_) => 0.006,
            EventKind::FeltBetter => 0.004,
            EventKind::Fetched => 0.01,
            EventKind::Petted => 0.004,
            EventKind::Pounced => 0.003,
            EventKind::Toppled { by_viewer: true } => {
                if self.temperament().playful >= 0.5 || self.bond >= 0.55 {
                    0.003
                } else {
                    -0.003
                }
            }
            EventKind::Found => 0.015,
            EventKind::ViewerArrived => 0.003,
            EventKind::DroppedMote | EventKind::KickedBall => 0.002,
            EventKind::GameOver(outcome) if outcome.finished() => 0.002,
            EventKind::Poked { count } if *count < 3 => 0.002,
            EventKind::Poked { .. } => -0.012,
            EventKind::WishedLuck(_) => 0.003,
            EventKind::PlanWentWell(_) => 0.008,
            EventKind::FoundTreasure { gave_up: false, .. } => 0.004,
            EventKind::TookBreak | EventKind::SaidGoodnightTogether => 0.004,
            EventKind::BlewBubbles {
                popped_by_viewer, ..
            } if *popped_by_viewer > 0 => 0.003,
            EventKind::Pinned {
                kind: super::gallery::Kind::ViewerDrawing,
                ..
            } => 0.005,
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
        let nature = self.temperament();
        let shyness = if self.bond < 0.3 {
            0.4 * (nature.shy - 0.5)
        } else {
            0.0
        };
        let warmth = (1.0 + 0.4 * (nature.tender - 0.5) - shyness).clamp(0.75, 1.35);
        self.warm(GROWING * seconds * warmth);
        let milestone = HOURS_MARKED
            .into_iter()
            .find(|(hours, _, _)| (together..self.total.together).contains(&(hours * 3600.0)))
            .and_then(|(_, text, korean)| self.reach(text.to_owned(), korean));
        milestone.or_else(|| self.closer(bond))
    }

    /// The viewer stayed away for `seconds` more. After the first day alone,
    /// the bond fades a little each day, but never below half of the
    /// closest it was.
    pub fn miss(&mut self, seconds: f32) {
        self.miss_at(seconds, 0.0);
    }

    /// The same rule with the local hour, so a restore spanning the grace
    /// deadline fades only the part of the absence after that deadline.
    pub fn miss_at(&mut self, seconds: f32, hour: f32) {
        let before = self.alone;
        self.alone += f64::from(seconds.max(0.0));
        let past_first = |alone: f64| (alone / 86_400.0 - 1.0).max(0.0);
        let mut days = past_first(self.alone) - past_first(before);
        if let Some(promise) = self.promise.filter(|promise| promise.valid())
            && let Ok(today) = self.day.parse::<jiff::civil::Date>()
        {
            // Deadlines cover whole local days, including one day of grace.
            // A long restore must not retroactively fade the protected days.
            let deadline = promise.back_by.checked_add(jiff::Span::new().days(2));
            let overdue = deadline.map_or(0.0, |deadline| {
                today.duration_since(deadline).as_secs_f64() + f64::from(hour) * 3600.0
            });
            days = days.min((overdue / 86_400.0).max(0.0));
        }
        if days > 0.0 {
            let holding = 1.15 - 0.3 * self.temperament().shy;
            self.warm(-FADING * days as f32 * holding);
        }
    }

    /// How close it needs to feel before coming to greet the viewer.
    pub fn greeting_bond(&self) -> f32 {
        let nature = self.temperament();
        GREETS_FROM
            * (1.0 + 0.4 * (nature.shy - 0.5) - 0.2 * (nature.tender - 0.5)).clamp(0.75, 1.35)
    }

    /// Learns an arrival at this local hour, once per hour per day. Returns
    /// the moment it first learns the viewer's routine, if reached.
    pub fn arrive(&mut self, hour: f32) -> Option<String> {
        if !hour.is_finite() {
            return None;
        }
        let hour = hour.rem_euclid(24.0);
        self.last_arrival = Some((self.day.clone(), hour));
        let mask = 1 << hour as usize;
        if self.arrived_hours & mask != 0 {
            return None;
        }
        if self.arrived_hours == 0 {
            self.days_seen = self.days_seen.saturating_add(1);
        }
        self.arrived_hours |= mask;
        self.arrivals[hour as usize] += 1.0;
        self.routine_days = self.routine_days.max(1.0);
        self.usual_time()?;
        self.reach(
            "I've learned when you usually come.".to_owned(),
            "네가 늘 언제 오는지 알게 됐어.",
        )
    }

    /// How strongly the viewer is expected at this hour, with half of
    /// each neighboring hour's evidence, wrapping around midnight.
    pub fn expectation(&self, hour: f32) -> f32 {
        if self.days_seen < 3 || self.routine_days <= 0.0 || !hour.is_finite() {
            return 0.0;
        }
        let hour = hour.rem_euclid(24.0) as usize;
        let weight = self.arrivals[hour]
            + 0.5 * (self.arrivals[(hour + 23) % 24] + self.arrivals[(hour + 1) % 24]);
        (weight / self.routine_days).clamp(0.0, 1.0)
    }

    /// The part of day the viewer most often comes, once it has seen them
    /// on enough different days to know a routine.
    pub fn usual_time(&self) -> Option<&'static str> {
        if self.days_seen < 3 {
            return None;
        }
        let (hour, _) = self
            .arrivals
            .iter()
            .enumerate()
            .filter(|(_, weight)| **weight > 0.0)
            .max_by(|first, second| first.1.total_cmp(second.1))?;
        Some(match hour {
            5..8 => "in the early morning",
            8..11 => "in the morning",
            11..14 => "around midday",
            14..17 => "in the afternoon",
            17..22 => "in the evening",
            _ => "late at night",
        })
    }

    /// Starts counting `day`, a local date as YYYY-MM-DD. When it is a new
    /// day, the last one goes into the diary, unless nothing happened in it.
    pub fn begin_day(&mut self, day: &str) {
        self.begin_day_with_birth(day, None);
    }

    pub(super) fn begin_day_with_birth(&mut self, day: &str, birthday: Option<jiff::civil::Date>) {
        if self.day == day {
            return;
        }
        // A clock that went back keeps the later day: beginning an earlier
        // one would throw away today's tally and warmth, and make what was
        // kept today look like it came from the future.
        if let (Ok(current), Ok(next)) = (
            self.day.parse::<jiff::civil::Date>(),
            day.parse::<jiff::civil::Date>(),
        ) && next < current
        {
            return;
        }
        let days = self
            .day
            .parse::<jiff::civil::Date>()
            .ok()
            .zip(day.parse::<jiff::civil::Date>().ok())
            .map_or(0, |(before, after)| {
                (after - before).get_days().clamp(0, 365)
            });
        self.raise_through(day, birthday);
        let decay = ROUTINE_DECAY.powi(days);
        for weight in &mut self.arrivals {
            *weight *= decay;
        }
        self.routine_days = if self.day.is_empty() {
            1.0
        } else {
            self.routine_days * decay + (1.0 - decay) / (1.0 - ROUTINE_DECAY)
        };
        self.arrived_hours = 0;
        if !self.day.is_empty() && !self.today.is_empty() {
            self.diary.push(Entry {
                day: self.day.clone(),
                text: self.today.diary(),
                korean: Some(self.today.diary_korean()),
            });
            if self.diary.len() > DAYS_KEPT {
                let excess = self.diary.len() - DAYS_KEPT;
                self.diary.drain(..excess);
            }
        }
        self.yesterday = self.today.top_deeds(&self.day, 3);
        self.day = day.to_owned();
        self.today = Tally::default();
        self.warmed = 0.0;
    }

    /// Starts a new ghost's life on `day`, a local date as YYYY-MM-DD, with
    /// the first moment it keeps.
    pub fn begin(&mut self, day: &str) -> Option<String> {
        self.begin_day(day);
        self.reach(
            "I came into being in a glass box.".to_owned(),
            "유리 상자 안에서 태어났어.",
        )
    }

    /// How much more, or less, than a new ghost it leans toward `activity`
    /// because of how often it chose it before: from 0.85 for what it never
    /// does to 1.3 for what it does most. Floating is no habit.
    pub fn habit(&self, activity: Activity) -> f32 {
        if matches!(
            activity,
            Activity::Float | Activity::Comfort | Activity::Fold
        ) {
            return 1.0;
        }
        // Comfort follows a message; folding already has its daily allowance.
        let kinds = Activity::ALL.len() as f32 - 3.0;
        let total: u32 = Activity::ALL
            .into_iter()
            .filter(|other| !matches!(other, Activity::Float | Activity::Comfort | Activity::Fold))
            .map(|other| self.total.times(other))
            .sum();
        let share = (self.total.times(activity) as f32 + PRIOR) / (total as f32 + PRIOR * kinds);
        (0.85 + 0.15 * share * kinds).clamp(0.85, 1.3)
    }

    /// How fresh a pastime still feels today. Done often already today, it
    /// draws the ghost less, so a day holds some variety; what it needs, and
    /// what the viewer starts, stay as they are.
    pub fn novelty(&self, activity: Activity) -> f32 {
        let enough = match activity {
            Activity::Mirror => 4.0,
            Activity::Music => 5.0,
            Activity::Doodle | Activity::Hide | Activity::Build => 8.0,
            Activity::Dance | Activity::Explore | Activity::Play => 16.0,
            Activity::Bubbles => 1.5,
            _ => return 1.0,
        };
        1.0 / (1.0 + self.today.times(activity) as f32 / enough)
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
        super::gallery::sanitize(&mut self);
        for entry in self
            .diary
            .iter_mut()
            .chain(&mut self.milestones)
            .chain(&mut self.yesterday)
        {
            entry.korean = entry
                .korean
                .take()
                .filter(|text| super::writing::valid(text));
        }
        self.yesterday.retain(|entry| {
            super::writing::valid(&entry.text)
                && entry
                    .day
                    .parse::<jiff::civil::Date>()
                    .ok()
                    .zip(self.day.parse::<jiff::civil::Date>().ok())
                    .is_some_and(|(day, today)| day < today)
        });
        self.yesterday.truncate(3);
        for tally in [&mut self.today, &mut self.total] {
            tally.korean_details.retain(|english, korean| {
                super::writing::valid(english) && super::writing::valid(korean)
            });
            while tally.korean_details.len() > 64 {
                tally.korean_details.pop_last();
            }
        }
        super::breaks::sanitize(&mut self);
        self.cranes.sanitize(&self.day);
        for (tally, maximum) in [
            (&mut self.today, 3),
            (&mut self.total, super::cranes::MAXIMUM),
        ] {
            tally.cranes = tally.cranes.min(maximum);
            tally.cranes_for_sad_viewer = tally.cranes_for_sad_viewer.min(tally.cranes);
        }
        self.promise = self.promise.filter(|promise| {
            promise.valid()
                && self
                    .day
                    .parse::<jiff::civil::Date>()
                    .is_ok_and(|day| promise.said <= day)
        });
        self.anniversary_said = self
            .anniversary_said
            .filter(|day| day.parse::<jiff::civil::Date>().is_ok());
        self.landmark_greeted = self
            .landmark_greeted
            .filter(|day| day.parse::<jiff::civil::Date>().is_ok());
        for tally in [&mut self.today, &mut self.total] {
            tally.hunts = tally.hunts.min(1_000_000);
            tally.treasures_found = tally.treasures_found.min(tally.hunts);
            tally.treasure_hints = tally.treasure_hints.min(10_000_000);
            tally.fastest_treasure = tally
                .fastest_treasure
                .filter(|seconds| (1..=240).contains(seconds) && tally.treasures_found > 0);
            tally.anniversary_words.retain(|line| {
                !line.trim().is_empty()
                    && line.chars().count() <= 256
                    && !line.chars().any(char::is_control)
            });
            tally.anniversary_words.truncate(12);
            tally
                .temperament_changes
                .retain(|text| super::upbringing::valid_change(text));
            tally.temperament_changes.truncate(5);
        }
        super::agenda::sanitize(&mut self.agenda, &self.day);
        self.feelings.retain(|note| note.valid(&self.day));
        if self.feelings.len() > 12 {
            self.feelings.drain(..self.feelings.len() - 12);
        }
        super::creation::sanitize(&mut self);
        self.mirror.sanitize(&self.day);
        self.total.met_reflection |= self.mirror.sessions > 0 || self.mirror.recognizes_self();
        self.total.recognized_self |= self.mirror.recognizes_self();
        self.received.retain(super::penpal::ViewerLetter::valid);
        for letter in &mut self.received {
            letter.sanitize();
        }
        if self.received.len() > super::mail::KEPT {
            self.received
                .drain(..self.received.len() - super::mail::KEPT);
        }
        for letter in &mut self.letters {
            letter.sanitize();
        }
        self.letters.retain(super::mail::Letter::valid);
        if self.letters.len() > super::mail::KEPT {
            self.letters.drain(..self.letters.len() - super::mail::KEPT);
        }
        self.viewer.sanitize();
        self.books.sanitize();
        self.tunes = self
            .tunes
            .into_iter()
            .filter_map(super::music::Tune::sanitized)
            .collect();
        self.tunes.reverse();
        let mut kept = Vec::new();
        for tune in self.tunes {
            if kept.len() < 8
                && !kept
                    .iter()
                    .any(|known: &super::music::Tune| known.notes == tune.notes)
            {
                kept.push(tune);
            }
        }
        kept.reverse();
        self.tunes = kept;
        self.constellations
            .retain(super::stars::Constellation::valid);
        if self.constellations.len() > 12 {
            self.constellations.drain(..self.constellations.len() - 12);
        }
        self.constellation_night = self
            .constellation_night
            .filter(|night| night.parse::<jiff::civil::Date>().is_ok());
        self.skills = self.skills.sanitized(&self.total);
        self.nature = self.nature.map(Nature::sanitized);
        self.upbringing.sanitize(&self.day);
        self.pictures.retain(LearnedPicture::valid);
        // A newer example replaces an older example with the same name.
        let mut names = std::collections::BTreeSet::new();
        self.pictures.reverse();
        self.pictures
            .retain(|picture| names.insert(picture.name.clone()));
        self.pictures.truncate(8);
        self.pictures.reverse();
        self.viewer_name = self.viewer_name.filter(|name| {
            (2..=16).contains(&name.chars().count()) && name.chars().all(char::is_alphabetic)
        });
        let unit = |value: f32| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        self.friend = self.friend.filter(|friend| {
            !friend.name.trim().is_empty()
                && friend.name.chars().count() <= 24
                && !friend.name.chars().any(char::is_control)
        });
        if let Some(friend) = &mut self.friend {
            friend.hue = unit(friend.hue);
            friend.closeness = unit(friend.closeness);
            friend.visits = friend.visits.min(1_000_000);
        }
        self.wish = self.wish.take().filter(|wishing| {
            wishing.wish.valid(&self) && wishing.day.parse::<jiff::civil::Date>().is_ok()
        });
        if let Some(wishing) = &mut self.wish {
            wishing.said = wishing
                .said
                .take()
                .filter(|day| day.parse::<jiff::civil::Date>().is_ok());
        }
        let mut granted = std::mem::take(&mut self.granted);
        granted.retain(|granted| {
            granted.wish.valid(&self) && granted.day.parse::<jiff::civil::Date>().is_ok()
        });
        if granted.len() > 12 {
            granted.drain(..granted.len() - 12);
        }
        self.granted = granted;
        if self.total.names == 0
            && self
                .milestones
                .iter()
                .any(|entry| entry.text.starts_with("You gave me my name: "))
        {
            self.total.names = 1;
        }
        self.closest = unit(self.closest);
        self.bird_trust = unit(self.bird_trust);
        self.korean = unit(self.korean);
        if self.keepsakes.len() > 8 {
            self.keepsakes.drain(..self.keepsakes.len() - 8);
        }
        self.wardrobe_day = self
            .wardrobe_day
            .filter(|day| day.parse::<jiff::civil::Date>().is_ok() && day == &self.day);
        self.wearing = self.wearing.filter(|accessory| {
            accessory.valid(self.wardrobe_occasion)
                && self.wardrobe_day.is_some()
                && (accessory.keepsake().is_none() || self.keepsakes.contains(&Keepsake::Feather))
        });

        self.bond = unit(self.bond).max(self.closest * 0.5);
        self.closest = self.closest.max(self.bond);
        for tally in [&mut self.total, &mut self.today] {
            tally.quirks.retain(|_, count| {
                *count = (*count).min(100_000_000);
                *count > 0
            });
            tally.letters_written = tally.letters_written.min(100_000_000);
            tally.letters_received = tally.letters_received.min(100_000_000);
            tally.viewer_letters_read = tally.viewer_letters_read.min(100_000_000);
            tally.letters_read = tally.letters_read.min(100_000_000);
            tally.outfits = tally.outfits.min(100_000_000);
            tally.pages = tally.pages.min(100_000_000);
            tally.books_finished = tally.books_finished.min(100_000_000);
            tally.books_received = tally.books_received.min(12);
            tally.compositions = tally.compositions.min(100_000_000);
            tally.books_written = tally.books_written.min(100_000_000);
            tally.songs_sung_back = tally.songs_sung_back.min(100_000_000);
            tally
                .composition_titles
                .retain(|title| super::creation::valid_text(title, 80));
            tally.composition_titles.truncate(8);
            tally
                .written_titles
                .retain(|title| super::creation::valid_text(title, 80));
            tally.written_titles.truncate(6);
            tally.tunes_played = tally.tunes_played.min(100_000_000);
            tally.tunes_learned = tally.tunes_learned.min(100_000_000);
            tally.mirror_visits = tally.mirror_visits.min(100_000_000);
            tally.lullaby_sleeps = tally.lullaby_sleeps.min(100_000_000);
            tally.learned_about = tally.learned_about.min(100_000_000);
            tally.names = tally.names.min(100_000_000);
            tally.wishes_made = tally.wishes_made.min(100_000_000);
            tally.wishes_granted = tally.wishes_granted.min(100_000_000);
            tally.viewer_wishes_granted = tally.viewer_wishes_granted.min(tally.wishes_granted);
            tally.scolds = tally.scolds.min(100_000_000);
            tally.wishes_faded = tally.wishes_faded.min(100_000_000);
            tally.wish_words.retain(|words| {
                words.chars().count() <= 256 && !words.chars().any(char::is_control)
            });
            tally.wish_words.truncate(12);
            tally.passersby = tally.passersby.min(100_000_000);
            let mut kinds = Vec::new();
            for kind in &tally.passerby_kinds {
                if !kinds.contains(kind) {
                    kinds.push(*kind);
                }
            }
            tally.passerby_kinds = kinds;
            tally.comforts = tally.comforts.min(100_000_000);
            tally.feelings_heard = tally.feelings_heard.min(100_000_000);
            tally.felt_better = tally.felt_better.min(100_000_000);
            let mut feelings = Vec::new();
            tally.feelings.retain(|feeling| {
                if feelings.contains(feeling) {
                    false
                } else {
                    feelings.push(*feeling);
                    true
                }
            });
            let mut comforts = Vec::new();
            tally.comforted_feelings.retain(|feeling| {
                if comforts.contains(feeling) {
                    false
                } else {
                    comforts.push(*feeling);
                    true
                }
            });
            tally.focus_sessions = tally.focus_sessions.min(1_000_000);
            tally.focus_minutes = tally.focus_minutes.min(100_000_000);
            tally.stories = tally.stories.min(100_000_000);
            tally.photos = tally.photos.min(100_000_000);
            tally.portraits = tally.portraits.min(tally.photos);
            tally.airings = tally.airings.min(100_000_000);
            tally.word_games = tally.word_games.min(100_000_000);
            tally.words_guessed = tally.words_guessed.min(tally.word_games);
            tally.friend_visits = tally.friend_visits.min(1_000_000);
            tally.friend_games = tally.friend_games.min(100_000_000);
            tally.friend_name = tally.friend_name.take().filter(|name| {
                !name.trim().is_empty()
                    && name.chars().count() <= 24
                    && !name.chars().any(char::is_control)
            });
            tally.friend_played.truncate(3);
            tally.tallest = tally.tallest.min(5);
            tally.planted_kinds.truncate(5);
            tally.sprouted_kinds.truncate(5);
            tally.flowered_kinds.truncate(5);
            tally.watched_writings = tally.watched_writings.min(tally.writings);
            tally.written_words.truncate(WORDS_NAMED);
            for word in &mut tally.written_words {
                *word = word
                    .chars()
                    .filter(|character| {
                        character.is_ascii_uppercase()
                            || character.is_ascii_digit()
                            || super::letters::decompose(*character).is_some()
                            || matches!(character, '!' | '?' | '♥')
                    })
                    .take(17)
                    .collect();
            }
            tally.written_words.retain(|word| !word.is_empty());
            tally.viewer_names.retain(|name| {
                (2..=16).contains(&name.chars().count()) && name.chars().all(char::is_alphabetic)
            });
            tally.viewer_names.truncate(WORDS_NAMED);
            tally.plans_heard = tally.plans_heard.min(100_000_000);
            tally.luck_wished = tally.luck_wished.min(100_000_000);
            tally.plans_went_well = tally.plans_went_well.min(100_000_000);
            tally.plans_went_badly = tally.plans_went_badly.min(100_000_000);
            tally.bubble_sessions = tally.bubble_sessions.min(1_000_000);
            tally.bubbles_blown = tally.bubbles_blown.min(tally.bubble_sessions * 10);
            tally.bubbles_popped = tally.bubbles_popped.min(tally.bubbles_blown);
            for errands in [
                &mut tally.heard_plans,
                &mut tally.wished_plans,
                &mut tally.good_plans,
                &mut tally.hard_plans,
            ] {
                let mut seen = Vec::new();
                errands.retain(|errand| {
                    if seen.contains(errand) {
                        false
                    } else {
                        seen.push(*errand);
                        true
                    }
                });
            }
            if !tally.together.is_finite() || tally.together < 0.0 {
                tally.together = 0.0;
            }
        }
        if !self.alone.is_finite() || self.alone < 0.0 {
            self.alone = 0.0;
        }
        if let Some(day) = self.gift_day.take()
            && self.gift_decision.is_none()
        {
            self.gift_decision = Some((day, false));
        }
        for decision in [&mut self.gift_decision, &mut self.friend_decision] {
            *decision = decision
                .take()
                .filter(|(day, _)| day.parse::<jiff::civil::Date>().is_ok());
        }
        self.warmed = if self.warmed.is_finite() {
            self.warmed.clamp(0.0, DAILY_WARMTH)
        } else {
            0.0
        };
        self.vocabulary = self.vocabulary.sanitized();
        self.routine_days = if self.routine_days.is_finite() {
            self.routine_days.clamp(0.0, 11.0)
        } else {
            0.0
        };
        for weight in &mut self.arrivals {
            *weight = if weight.is_finite() {
                weight.clamp(0.0, self.routine_days)
            } else {
                0.0
            };
        }
        self.arrived_hours &= (1 << 24) - 1;
        self.last_arrival = self.last_arrival.filter(|(day, hour)| {
            day.parse::<jiff::civil::Date>().is_ok()
                && hour.is_finite()
                && (0.0..24.0).contains(hour)
        });
        self
    }

    // The first time something happens, or a count it marks.
    fn first(&mut self, kind: &EventKind) -> Option<String> {
        if let EventKind::PromiseKept(kind) = kind {
            let first = self.reach(
                "You came back when you said you would.".to_owned(),
                "돌아온다고 약속한 때에 와 줬어.",
            );
            let trip = kind
                .trip()
                .then(|| {
                    self.reach(
                        "You came home from a trip.".to_owned(),
                        "네가 여행을 마치고 돌아왔어.",
                    )
                })
                .flatten();
            return first.or(trip);
        }
        if matches!(kind, EventKind::MetReflection) {
            return self.reach(
                "I met a ghost in the mirror.".to_owned(),
                "거울 속 유령을 만났어.",
            );
        }
        if matches!(kind, EventKind::RecognizedSelf) {
            return self.reach(
                "I know the ghost in the mirror is me.".to_owned(),
                "거울 속 유령이 나라는 걸 알게 됐어.",
            );
        }
        if let EventKind::LearnedAboutViewer(topic) = kind {
            let first = self.reach(
                "You told me about yourself.".to_owned(),
                "너에 대해 이야기해 줬어.",
            );
            let birthday = (*topic == super::acquaintance::Topic::Birthday)
                .then(|| {
                    self.reach(
                        "I know your birthday now.".to_owned(),
                        "이제 네 생일을 알아.",
                    )
                })
                .flatten();
            return first.or(birthday);
        }
        if matches!(kind, EventKind::LearnedTune(_)) {
            return self.reach(
                "You taught me my first tune.".to_owned(),
                "처음으로 내게 노래를 가르쳐 줬어.",
            );
        }
        if matches!(kind, EventKind::LullabySleep) {
            return self.reach(
                "I fell asleep to my first lullaby.".to_owned(),
                "처음으로 자장가를 들으며 잠들었어.",
            );
        }
        if matches!(kind, EventKind::WishGranted(_)) {
            let mut first = None;
            for (count, text, korean) in [
                (1, "My first wish came true.", "첫 소원이 이루어졌어."),
                (
                    5,
                    "Five of my wishes have come true.",
                    "소원 다섯 개가 이루어졌어.",
                ),
            ] {
                if self.total.wishes_granted >= count {
                    first = first.or(self.reach(text.to_owned(), korean));
                }
            }
            return first;
        }
        if let EventKind::SawPasserby(passerby) = kind {
            return self.reach(
                format!("I saw {} for the first time.", passerby.name()),
                format!(
                    "처음으로 {}{} 봤어.",
                    super::writing::passerby(*passerby),
                    super::writing::particle(super::writing::passerby(*passerby), "을", "를")
                ),
            );
        }
        if let EventKind::FocusEnded { completed, .. } = kind {
            let reached = [
                (
                    *completed && self.total.focus_sessions >= 1,
                    "We finished our first focus session together.",
                    "처음으로 같이 집중하는 시간을 끝냈어.",
                ),
                (
                    *completed && self.total.focus_sessions >= 5,
                    "We have finished five focus sessions together.",
                    "같이 집중하는 시간을 다섯 번 끝냈어.",
                ),
                (
                    self.total.focus_minutes >= 600,
                    "We have focused together for ten hours.",
                    "열 시간 동안 같이 집중했어.",
                ),
            ];
            let mut first = None;
            for (reached, text, korean) in reached {
                if reached {
                    let milestone = self.reach(text.to_owned(), korean);
                    first = first.or(milestone);
                }
            }
            return first;
        }
        if let EventKind::FriendArrived(name) = kind {
            return match self.friend.as_ref().map_or(0, |friend| friend.visits) {
                1 => self.reach(
                    format!("I met another ghost tonight. Their name is {name}."),
                    format!("오늘 밤 유령 친구를 만났어. 이름은 {name}."),
                ),
                5 => self.reach(
                    format!("{name} has come to see me five times now."),
                    format!("친구 {name}, 벌써 다섯 번이나 놀러 왔어."),
                ),
                _ => None,
            };
        }
        let total = &self.total;
        if let EventKind::GameOver(outcome) = kind {
            let firsts = [
                (
                    outcome.finished() && total.games == 1,
                    "We played our first game of noughts and crosses.",
                    "처음으로 같이 틱택토를 했어.",
                ),
                (
                    matches!(outcome, Outcome::ViewerWon { .. }) && total.losses == 1,
                    "You beat me at noughts and crosses for the first time.",
                    "네가 처음으로 틱택토에서 나를 이겼어.",
                ),
                (
                    matches!(outcome, Outcome::GhostWon { .. }) && total.wins == 1,
                    "I won my first game of noughts and crosses.",
                    "처음으로 틱택토에서 이겼어.",
                ),
            ];
            let mut first = None;
            for (reached, text, korean) in firsts {
                if reached {
                    let milestone = self.reach(text.to_owned(), korean);
                    first = first.or(milestone);
                }
            }
            return first;
        }
        let (text, korean) = match kind {
            EventKind::Comforted(_) if total.comforts == 1 => (
                "I comforted you for the first time.".to_owned(),
                "처음으로 네 마음을 달래 줬어.".to_owned(),
            ),
            EventKind::WordGame { solved: true, .. } if total.words_guessed == 1 => (
                "You guessed my secret word for the first time.".to_owned(),
                "네가 처음으로 내 비밀 낱말을 맞혔어.".to_owned(),
            ),
            EventKind::Composed(title) if total.compositions == 1 => (
                format!("I made up a tune of my own: {title}."),
                format!("내 노래를 만들었어: {}.", self.today.korean_detail(title)),
            ),
            EventKind::SangMySong(_) if total.songs_sung_back == 1 => (
                "You sang my song back to me.".to_owned(),
                "네가 내 노래를 다시 불러 줬어.".to_owned(),
            ),
            EventKind::WroteBook(title) if total.books_written == 1 => (
                format!("I wrote a picture book of my own: {title}."),
                format!("내 그림책을 썼어: {}.", self.today.korean_detail(title)),
            ),
            EventKind::FinishedBook(_) if total.books_finished == 1 => (
                "I finished my first picture book.".to_owned(),
                "첫 그림책을 끝까지 읽었어.".to_owned(),
            ),
            EventKind::GotLetter if total.letters_received == 1 => (
                "You wrote me my first letter.".to_owned(),
                "네가 내게 첫 편지를 써 줬어.".to_owned(),
            ),
            EventKind::WroteLetter if total.letters_written == 1 => (
                "I wrote you my first letter.".to_owned(),
                "네게 첫 편지를 썼어.".to_owned(),
            ),
            EventKind::ToldStory if total.stories == 1 => (
                "I told you my first story.".to_owned(),
                "네게 처음으로 이야기를 들려줬어.".to_owned(),
            ),
            EventKind::LearnedPicture(name) if total.pictures_learned == 1 => (
                format!("You taught me to draw a {name}."),
                format!("네게 그림 그리는 법을 배웠어: {name}."),
            ),
            EventKind::Planted(_) if total.planted == 1 => (
                "I planted my first seed.".to_owned(),
                "첫 씨앗을 심었어.".to_owned(),
            ),
            EventKind::Sprouted(_) if total.sprouted == 1 => (
                "Something is growing in the little pot!".to_owned(),
                "작은 화분에서 싹이 났어!".to_owned(),
            ),
            EventKind::Flowered(_) if total.flowered == 1 => (
                "The first flower I grew from a seed is blooming.".to_owned(),
                "씨앗부터 키운 첫 꽃이 피었어.".to_owned(),
            ),
            EventKind::Kept(_) if total.gifts == 1 => (
                "The bird brought me a gift.".to_owned(),
                "새가 선물을 가져다줬어.".to_owned(),
            ),
            EventKind::AteMote if total.motes == 1 => (
                "Caught my first light mote.".to_owned(),
                "첫 빛 알갱이를 잡았어.".to_owned(),
            ),
            EventKind::AteMote => {
                let (_, english, korean) = MOTES_MARKED
                    .into_iter()
                    .find(|(motes, _, _)| *motes == total.motes)?;
                (english.to_owned(), korean.to_owned())
            }
            EventKind::NudgedBall if total.nudges == 1 => (
                "Played with the ball for the first time.".to_owned(),
                "처음으로 공을 가지고 놀았어.".to_owned(),
            ),
            EventKind::Fetched if total.fetches == 1 => (
                "Played fetch with you for the first time.".to_owned(),
                "처음으로 네게 공을 가져다줬어.".to_owned(),
            ),
            EventKind::Pounced if total.pounces == 1 => (
                "Caught your finger on the glass for the first time.".to_owned(),
                "처음으로 유리 너머 네 손가락을 잡았어.".to_owned(),
            ),
            EventKind::Built { .. } if total.towers == 1 => (
                "I built my first tower of blocks.".to_owned(),
                "첫 블록 탑을 쌓았어.".to_owned(),
            ),
            EventKind::Danced if total.dances == 1 => (
                "Danced my first twirl.".to_owned(),
                "처음으로 빙글 돌며 춤췄어.".to_owned(),
            ),
            EventKind::Found if total.finds == 1 => (
                "You found me hiding.".to_owned(),
                "숨어 있던 나를 네가 찾았어.".to_owned(),
            ),
            EventKind::Doodled(_) | EventKind::Copied(_) if total.doodles == 1 => (
                "Drew my first picture on the glass.".to_owned(),
                "유리에 첫 그림을 그렸어.".to_owned(),
            ),
            EventKind::Wrote {
                mirrored: false, ..
            } => (
                "I learned to write so you can read it.".to_owned(),
                "네가 읽을 수 있게 글씨 쓰는 법을 배웠어.".to_owned(),
            ),
            EventKind::LearnedName(name)
                if !self
                    .milestones
                    .iter()
                    .any(|milestone| milestone.text.starts_with("I learned your name: ")) =>
            {
                (
                    format!("I learned your name: {name}."),
                    format!("네 이름을 알게 됐어: {name}."),
                )
            }
            EventKind::Named(name)
                if !self
                    .milestones
                    .iter()
                    .any(|milestone| milestone.text.starts_with("You gave me my name: ")) =>
            {
                (
                    format!("You gave me my name: {name}."),
                    format!("네가 내게 이름을 지어 줬어: {name}."),
                )
            }
            EventKind::Bloomed if total.blooms == 1 => (
                "My plant flowered for the first time.".to_owned(),
                "내 화분에 처음으로 꽃이 피었어.".to_owned(),
            ),
            EventKind::Morning if total.nights == 1 => (
                "Slept through my first night in the glass box.".to_owned(),
                "유리 상자에서 첫 밤을 푹 잤어.".to_owned(),
            ),
            EventKind::Noticed(thing)
                if thing.starts_with("the rainbow") && total.rainbows == 1 =>
            {
                (
                    "Saw my first rainbow.".to_owned(),
                    "첫 무지개를 봤어.".to_owned(),
                )
            }
            EventKind::ShootingStar if total.shooting_stars == 1 => (
                "Saw my first shooting star, and made a wish.".to_owned(),
                "첫 별똥별을 보고 소원을 빌었어.".to_owned(),
            ),
            EventKind::ForecastCameTrue { snow: false } if total.forecasts == 1 => (
                "I felt the rain coming before it fell.".to_owned(),
                "비가 오기 전에 느꼈어.".to_owned(),
            ),
            EventKind::ForecastCameTrue { snow: true } if total.snow_forecasts == 1 => (
                "I smelled the snow coming before it fell.".to_owned(),
                "눈이 오기 전에 냄새를 맡았어.".to_owned(),
            ),
            EventKind::PutOn(_) if total.outfits == 1 => (
                "I wore my first little accessory.".to_owned(),
                "처음으로 작은 장신구를 달았어.".to_owned(),
            ),
            EventKind::FoundConstellation(name) if total.constellations == 1 => (
                format!("I found my first constellation: {name}."),
                format!("첫 별자리를 찾았어: {}.", self.today.korean_detail(name)),
            ),
            EventKind::StormStarted if total.storms == 1 => (
                "Sat out my first thunderstorm.".to_owned(),
                "첫 천둥 소리를 들으며 기다렸어.".to_owned(),
            ),
            EventKind::Noticed(thing) if thing == "a bird at the window" && total.birds == 1 => (
                "Saw my first bird.".to_owned(),
                "처음으로 새를 봤어.".to_owned(),
            ),
            EventKind::Noticed(thing) if thing == "a moth at the lamp" && total.moths == 1 => (
                "A moth came to visit my lamp.".to_owned(),
                "나방이 내 등불에 놀러 왔어.".to_owned(),
            ),
            EventKind::SnowStarted if total.snowfalls == 1 => (
                "Saw snow fall for the first time.".to_owned(),
                "처음으로 눈이 내리는 걸 봤어.".to_owned(),
            ),
            EventKind::Celebrated(occasion) => {
                let (english, korean) = match occasion {
                    Occasion::Halloween => (
                        "My first Halloween in the glass box.",
                        "유리 상자에서 첫 할로윈을 보냈어.",
                    ),
                    Occasion::WinterHolidays => {
                        ("My first winter holidays.", "첫 겨울 명절을 보냈어.")
                    }
                    Occasion::NewYear => ("Saw in my first new year.", "첫 새해를 맞았어."),
                    Occasion::Valentine => {
                        ("My first Valentine's Day.", "첫 밸런타인데이를 보냈어.")
                    }
                    Occasion::Birthday => ("My first birthday!", "첫 생일을 맞았어!"),
                    Occasion::ViewerBirthday => (
                        "Celebrated your birthday together for the first time.",
                        "처음으로 네 생일을 같이 축하했어.",
                    ),
                    Occasion::HundredDays => (
                        "My hundredth day in the glass box.",
                        "유리 상자에서 백일을 맞았어.",
                    ),
                    Occasion::ThousandDays => (
                        "My thousandth day in the glass box.",
                        "유리 상자에서 천일을 맞았어.",
                    ),
                    Occasion::Seollal => ("Celebrated my first Seollal.", "첫 설날을 보냈어."),
                    Occasion::Chuseok => (
                        "Celebrated my first Chuseok under the harvest moon.",
                        "첫 추석 달을 보며 명절을 보냈어.",
                    ),
                    Occasion::HangulDay => (
                        "My first Hangul Day, drawing letters in the fog.",
                        "첫 한글날에 김 서린 유리에 글씨를 그렸어.",
                    ),
                };
                (english.to_owned(), korean.to_owned())
            }
            EventKind::Petted if total.pets == 1 => (
                "You stroked me for the first time.".to_owned(),
                "네가 처음으로 나를 쓰다듬어 줬어.".to_owned(),
            ),
            EventKind::Heard(_) if total.heard == 1 => (
                "You talked to me for the first time.".to_owned(),
                "네가 처음으로 내게 말을 걸어 줬어.".to_owned(),
            ),
            EventKind::LearnedWord { word, .. } if total.learned == 1 => (
                format!("Learned my first word from you: “{word}”."),
                format!("네게 첫 낱말을 배웠어: ‘{word}’."),
            ),
            EventKind::LearnedWord { .. } => {
                let known = self.vocabulary.known().len();
                // Several words can be learned at once, so a mark passed
                // without being kept is kept now.
                let (_, english, korean) = WORDS_MARKED
                    .into_iter()
                    .filter(|(count, _, _)| *count <= known)
                    .find(|(_, text, _)| {
                        !self
                            .milestones
                            .iter()
                            .any(|milestone| milestone.text == *text)
                    })?;
                (english.to_owned(), korean.to_owned())
            }
            EventKind::PlanWentWell(errand) if total.plans_went_well == 1 => (
                format!("You told me your {} went well.", errand.name()),
                format!("네 {} 잘 됐다고 말해 줬어.", errand.korean()),
            ),
            EventKind::FoundTreasure { gave_up: false, .. } if total.treasures_found == 1 => (
                "I found the treasure you hid for the first time.".to_owned(),
                "처음으로 네가 숨긴 보물을 찾았어.".to_owned(),
            ),
            EventKind::TookBreak if total.breaks_taken == 1 => (
                "You took a break when I asked.".to_owned(),
                "내 말에 네가 잠깐 쉬었어.".to_owned(),
            ),
            EventKind::FoldedCrane { count } if super::cranes::LANDMARKS.contains(count) => {
                if *count == 1 {
                    (
                        "I folded my first paper crane for you.".to_owned(),
                        "너를 위해 첫 종이학을 접었어.".to_owned(),
                    )
                } else if *count == 1000 {
                    (
                        "I folded a thousand paper cranes and made a wish for you.".to_owned(),
                        "종이학 천 마리를 접고 너를 위해 소원을 빌었어.".to_owned(),
                    )
                } else {
                    (
                        format!("I folded {} paper cranes for you.", count_words(*count)),
                        format!("너를 위해 종이학 {count}마리를 접었어."),
                    )
                }
            }
            EventKind::BlewBubbles {
                popped_by_viewer, ..
            } if *popped_by_viewer > 0 => (
                "You popped my bubbles for the first time.".to_owned(),
                "처음으로 네가 내 비눗방울을 터뜨려 줬어.".to_owned(),
            ),
            EventKind::Pinned {
                kind: super::gallery::Kind::ViewerDrawing,
                ..
            } if total.viewer_pins == 1 => (
                "I pinned your drawing on my wall.".to_owned(),
                "네 그림을 내 벽에 처음 붙였어.".to_owned(),
            ),
            EventKind::Photographed { .. } if total.photos == 1 => (
                "You took my picture for the first time.".to_owned(),
                "네가 처음으로 내 사진을 찍어 줬어.".to_owned(),
            ),
            _ => return None,
        };
        self.reach(text, korean)
    }

    // The bond grew past one of the marks since it was `before`.
    fn closer(&mut self, before: f32) -> Option<String> {
        let (_, text, korean) = CLOSER
            .into_iter()
            .rev()
            .find(|(mark, _, _)| before < *mark && self.bond >= *mark)?;
        self.reach(text.to_owned(), korean)
    }

    // Keeps a milestone, once; returns it if it is new.
    pub(super) fn reach(&mut self, text: String, korean: impl Into<String>) -> Option<String> {
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
            korean: Some(korean.into()),
        });
        Some(text)
    }

    // A moment's warmth fills only what room the day has left for it, and
    // a deeper bond takes it in more slowly; coolness lands in full.
    fn warm(&mut self, amount: f32) {
        let change = if amount > 0.0 {
            let fresh = amount * (1.0 - self.warmed / DAILY_WARMTH).max(0.0);
            self.warmed = (self.warmed + fresh).min(DAILY_WARMTH);
            fresh * (1.0 - self.bond).powi(2)
        } else {
            amount
        };
        self.bond = (self.bond + change).clamp(self.closest * 0.5, 1.0);
        self.closest = self.closest.max(self.bond);
    }
}

pub(super) fn recalled_deeds(deeds: &[String], korean: bool) -> String {
    if korean {
        return super::writing::sentence(deeds);
    }
    if deeds.is_empty() {
        "We had a quiet day.".to_owned()
    } else {
        format!("{}.", capitalized(&listed(deeds)))
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
        minutes => format!(
            "for about {} hours",
            count_words((minutes / 60.0).round() as u32)
        ),
    }
}

/// Items joined as "a", "a and b", or "a, b, and c".
pub(super) fn listed(items: &[String]) -> String {
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

/// A doodle's name as the diary writes it in Korean.
fn sketch_korean(name: &str) -> &str {
    match name {
        "heart" => "하트",
        "star" => "별",
        "smile" => "웃는 얼굴",
        "spiral" => "소용돌이",
        "sun" => "해",
        "flower" => "꽃",
        "wave" => "물결",
        "snowflake" => "눈송이",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_diary_ranks_a_kept_trip_above_an_everyday_return() {
        let mut tally = Tally::default();
        tally.note(&EventKind::PromiseKept(
            super::super::promise::Return::Tomorrow,
        ));
        tally.note(&EventKind::PromiseKept(super::super::promise::Return::Week));
        let deeds = tally.deeds();
        assert!(deeds.contains(&(
            Salience::Notable,
            "you came back when you said you would".to_owned(),
            "약속한 때에 돌아와 줬어".to_owned()
        )));
        assert!(deeds.contains(&(
            Salience::Special,
            "you came home from your trip".to_owned(),
            "네가 여행을 마치고 돌아왔어".to_owned()
        )));
        assert_eq!(tally.promises_kept, 2);
        assert_eq!(tally.trips_kept, 1);
    }

    #[test]
    fn completed_care_warms_the_bond_and_diary_distinguishes_shared_and_completed_feelings() {
        use crate::world::Feeling;
        let mut life = Life::default();
        life.begin_day("2026-06-21");
        life.note(&EventKind::ViewerFelt(Feeling::Sad));
        life.note(&EventKind::ViewerFelt(Feeling::Sad));
        life.note(&EventKind::ViewerFelt(Feeling::Happy));
        assert_eq!(life.today.feelings_heard, 3);
        assert_eq!(life.today.feelings, vec![Feeling::Sad, Feeling::Happy]);
        assert!(!life.today.words().contains("drew you a heart"));
        assert!(life.today.deeds().contains(&(
            Salience::Notable,
            "you told me you were happy".to_owned(),
            "‘행복해’ 하고 말해 줬어".to_owned()
        )));
        let before = life.bond;
        assert_eq!(
            life.note(&EventKind::Comforted(Feeling::Sad)).as_deref(),
            Some("I comforted you for the first time.")
        );
        assert!((life.bond - before - 0.006).abs() < 0.0002);
        assert!(life.today.deeds().contains(&(
            Salience::Special,
            "you were sad, so I drew you a heart".to_owned(),
            "‘슬퍼’ 하고 말해서 하트를 그려 줬어".to_owned()
        )));
        let before = life.bond;
        life.note(&EventKind::FeltBetter);
        // A day's warmth so far and the bond already gathered take a
        // little off.
        assert!((life.bond - before - 0.004).abs() < 0.0002);
        assert!(life.note(&EventKind::Comforted(Feeling::Sad)).is_none());
        life.begin_day("2026-06-22");
        assert!(life.today.feelings.is_empty());
        assert!(life.diary.last().unwrap().text.contains("drew you a heart"));
    }

    use super::*;

    #[test]
    fn focus_counts_load_from_old_saves_clamp_and_keep_all_landmarks_once() {
        let mut life: Life = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(life.total.focus_sessions, 0);
        assert_eq!(life.total.focus_minutes, 0);
        for _ in 0..5 {
            life.note(&EventKind::FocusEnded {
                minutes: 120,
                completed: true,
            });
        }
        assert_eq!(life.total.focus_sessions, 5);
        assert_eq!(life.total.focus_minutes, 600);
        for text in [
            "We finished our first focus session together.",
            "We have finished five focus sessions together.",
            "We have focused together for ten hours.",
        ] {
            assert_eq!(
                life.milestones
                    .iter()
                    .filter(|entry| entry.text == text)
                    .count(),
                1
            );
        }
        let marked = life.milestones.len();
        life.note(&EventKind::FocusEnded {
            minutes: 10,
            completed: false,
        });
        assert_eq!(life.milestones.len(), marked);
        assert_eq!(life.total.focus_sessions, 5);
        assert_eq!(life.total.focus_minutes, 610);
        let words = life.total.words();
        assert!(
            words
                .to_lowercase()
                .contains("kept you company through five focus sessions")
        );
        assert!(!words.chars().any(|character| character.is_ascii_digit()));
        let loaded: Life = serde_json::from_str(&serde_json::to_string(&life).unwrap()).unwrap();
        assert_eq!(loaded.total.focus_minutes, 610);
        life.total.focus_sessions = u32::MAX;
        life.today.focus_minutes = u32::MAX;
        let life = life.sanitized();
        assert_eq!(life.total.focus_sessions, 1_000_000);
        assert_eq!(life.today.focus_minutes, 100_000_000);
    }

    #[test]
    fn completed_stories_warm_the_bond_keep_one_first_and_load_in_old_lives() {
        let mut life = Life::default();
        life.begin_day("2026-06-21");
        assert_eq!(
            life.note(&EventKind::ToldStory).as_deref(),
            Some("I told you my first story.")
        );
        assert!(life.bond > 0.0);
        life.note(&EventKind::ToldStory);
        assert_eq!(life.total.stories, 2);
        assert_eq!(life.today.stories, 2);
        assert_eq!(life.milestones.len(), 1);
        assert!(life.today.words().contains("two stories"));
        assert!(
            !life
                .today
                .words()
                .chars()
                .any(|letter| letter.is_ascii_digit())
        );
        let encoded = serde_json::to_value(&life).unwrap();
        assert_eq!(
            serde_json::from_value::<Life>(encoded.clone())
                .unwrap()
                .sanitized(),
            life
        );
        let mut old = encoded;
        for tally in ["total", "today"] {
            old[tally].as_object_mut().unwrap().remove("stories");
        }
        let restored: Life = serde_json::from_value(old).unwrap();
        assert_eq!(restored.total.stories, 0);
        assert_eq!(restored.today.stories, 0);
        life.total.stories = u32::MAX;
        life.today.stories = u32::MAX;
        life.note(&EventKind::ToldStory);
        assert_eq!(life.total.stories, u32::MAX);
        let clean = life.sanitized();
        assert_eq!(clean.total.stories, 100_000_000);
        assert_eq!(clean.today.stories, 100_000_000);
    }

    #[test]
    fn the_viewers_language_round_trips_defaults_and_clamps_saved_values() {
        let mut life = Life {
            korean: 0.8,
            ..Life::default()
        };
        life.note(&EventKind::Wrote {
            text: "한글".to_owned(),
            mirrored: false,
            watched: true,
        });
        let encoded = serde_json::to_value(&life).unwrap();
        let restored: Life = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(restored.sanitized(), life);
        let mut old = encoded;
        old.as_object_mut().unwrap().remove("korean");
        assert_eq!(serde_json::from_value::<Life>(old).unwrap().korean, 0.0);
        for (value, expected) in [
            (-1.0, 0.0),
            (2.0, 1.0),
            (f32::NAN, 0.0),
            (f32::INFINITY, 0.0),
        ] {
            assert_eq!(
                Life {
                    korean: value,
                    ..Life::default()
                }
                .sanitized()
                .korean,
                expected
            );
        }
    }

    fn visits(life: &mut Life, hour: f32, days: std::ops::RangeInclusive<i8>) {
        for day in days {
            life.begin_day(&format!("2026-06-{day:02}"));
            life.arrive(hour);
        }
    }

    #[test]
    fn three_days_of_arrivals_teach_the_hour_and_its_neighbors() {
        let mut life = Life::default();
        visits(&mut life, 20.25, 1..=2);
        assert_eq!(life.expectation(20.0), 0.0);
        assert_eq!(life.usual_time(), None);
        life.begin_day("2026-06-03");
        assert_eq!(
            life.arrive(20.75).as_deref(),
            Some("I've learned when you usually come.")
        );
        assert!((life.expectation(20.0) - 1.0).abs() < 1e-5);
        assert!((life.expectation(19.0) - 0.5).abs() < 1e-5);
        assert!((life.expectation(21.0) - 0.5).abs() < 1e-5);
        assert_eq!(life.expectation(12.0), 0.0);
        life.begin_day("2026-06-04");
        assert!(life.expectation(20.0) >= 0.5);
        assert_eq!(life.arrive(20.0), None);
        assert_eq!(life.milestones.len(), 1);
    }

    #[test]
    fn an_hour_counts_once_each_day_and_a_day_seen_counts_once() {
        let mut life = Life::default();
        life.begin_day("2026-06-01");
        for hour in [20.0, 20.5, 20.9, 21.0, 21.5] {
            life.arrive(hour);
        }
        assert_eq!(life.arrivals[20], 1.0);
        assert_eq!(life.arrivals[21], 1.0);
        assert_eq!(life.days_seen, 1);
        life.begin_day("2026-06-01");
        life.arrive(20.0);
        assert_eq!(life.arrivals[20], 1.0);
        life.begin_day("2026-06-02");
        life.arrive(20.0);
        assert_eq!(life.days_seen, 2);
        assert!((life.arrivals[20] - 1.9).abs() < 1e-5);
    }

    #[test]
    fn expectations_wrap_around_midnight() {
        let mut life = Life::default();
        visits(&mut life, 0.1, 1..=3);
        assert!((life.expectation(23.9) - 0.5).abs() < 1e-5);
        assert!((life.expectation(24.0) - 1.0).abs() < 1e-5);
        assert!((life.expectation(1.0) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn daily_decay_lets_a_new_routine_overtake_the_old_one() {
        let mut life = Life::default();
        visits(&mut life, 20.0, 1..=14);
        assert_eq!(life.usual_time(), Some("in the evening"));
        visits(&mut life, 9.0, 15..=28);
        assert_eq!(life.usual_time(), Some("in the morning"));
        assert!(life.expectation(9.0) > 0.75);
        assert!(life.expectation(20.0) < 0.25);
    }

    #[test]
    fn days_away_decay_as_if_each_new_day_had_been_seen() {
        let mut daily = Life::default();
        visits(&mut daily, 20.0, 1..=3);
        let mut away = daily.clone();
        for day in 4..=17 {
            daily.begin_day(&format!("2026-06-{day:02}"));
        }
        away.begin_day("2026-06-17");
        assert!((away.arrivals[20] - daily.arrivals[20]).abs() < 1e-5);
        assert!((away.routine_days - daily.routine_days).abs() < 1e-5);
        assert!(away.expectation(20.0) < 0.1);
        assert_eq!(away.arrived_hours, 0);
        away.begin_day("9999-12-31");
        assert!(away.routine_days.is_finite());
        assert!((away.routine_days - 10.0).abs() < 1e-4);
    }

    #[test]
    fn usual_times_name_the_part_of_day_in_words() {
        for (hour, words) in [
            (6.0, "in the early morning"),
            (9.0, "in the morning"),
            (12.0, "around midday"),
            (15.0, "in the afternoon"),
            (20.0, "in the evening"),
            (23.0, "late at night"),
        ] {
            let mut life = Life::default();
            visits(&mut life, hour, 1..=3);
            assert_eq!(life.usual_time(), Some(words));
        }
    }

    #[test]
    fn saved_routines_round_trip_and_older_lives_start_without_one() {
        let mut life = Life::default();
        visits(&mut life, 20.0, 1..=3);
        life.note(&EventKind::ExpectedArrival);
        life.note(&EventKind::MissedVisit);
        let saved = serde_json::to_string(&life).unwrap();
        let mut loaded: Life = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded, life);
        loaded.arrive(20.5);
        assert_eq!(loaded.arrivals, life.arrivals);
        assert!(
            loaded
                .today
                .words()
                .to_lowercase()
                .contains("you came at your usual time")
        );
        assert!(
            loaded
                .today
                .words()
                .contains("waited for you at your usual time, but you didn't come")
        );
        let old: Life = serde_json::from_str(r#"{"bond":0.5,"day":"2026-06-03"}"#).unwrap();
        assert_eq!(old.arrivals, [0.0; 24]);
        assert_eq!(old.days_seen, 0);
        assert_eq!(old.usual_time(), None);
        assert_eq!(old.expectation(20.0), 0.0);
        assert_eq!(old.last_arrival, None);
    }

    #[test]
    fn loaded_routine_values_are_kept_in_range() {
        let mut life = Life {
            routine_days: f32::INFINITY,
            arrived_hours: u32::MAX,
            last_arrival: Some(("2026-06-03".to_owned(), f32::NAN)),
            ..Life::default()
        };
        life.arrivals[0] = f32::NAN;
        life.arrivals[1] = -1.0;
        life.arrivals[2] = 1000.0;
        let clean = life.sanitized();
        assert_eq!(clean.arrivals, [0.0; 24]);
        assert_eq!(clean.routine_days, 0.0);
        assert_eq!(clean.arrived_hours, (1 << 24) - 1);
        assert_eq!(clean.last_arrival, None);
        let clean = Life {
            routine_days: 1000.0,
            arrivals: [1000.0; 24],
            ..Life::default()
        }
        .sanitized();
        assert_eq!(clean.routine_days, 11.0);
        assert_eq!(clean.arrivals, [11.0; 24]);
    }

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
    fn the_shelf_keeps_the_newest_eight_gifts_and_restores_safely() {
        let mut life = Life::default();
        life.keep(Keepsake::Petal);
        life.keep(Keepsake::Pebble);
        for _ in 0..7 {
            life.keep(Keepsake::Feather);
        }
        assert_eq!(life.keepsakes.len(), 8);
        assert_eq!(life.keepsakes[0], Keepsake::Pebble);
        life.keepsakes.insert(0, Keepsake::Acorn);
        life.bird_trust = 8.0;
        let life = life.sanitized();
        assert_eq!(life.bird_trust, 1.0);
        assert_eq!(life.keepsakes.len(), 8);
        assert_eq!(life.keepsakes[0], Keepsake::Pebble);
        for trust in [f32::NAN, f32::INFINITY, -1.0] {
            let life = Life {
                bird_trust: trust,
                ..Life::default()
            }
            .sanitized();
            assert_eq!(life.bird_trust, 0.0);
        }
    }

    #[test]
    fn friendship_and_the_first_kept_gift_are_milestones_once() {
        let mut life = Life {
            bird_trust: 0.499,
            ..Life::default()
        };
        assert_eq!(
            life.watch_bird(10.0).as_deref(),
            Some("The bird trusts me now.")
        );
        life.bird_trust = 0.499;
        assert_eq!(life.watch_bird(10.0), None);
        assert_eq!(life.note(&EventKind::GiftLeft(Keepsake::Feather)), None);
        assert_eq!(life.total.gifts, 0);
        assert_eq!(
            life.note(&EventKind::Kept(Keepsake::Feather)).as_deref(),
            Some("The bird brought me a gift.")
        );
        assert_eq!(life.note(&EventKind::Kept(Keepsake::Acorn)), None);
        assert_eq!(life.total.gifts, 2);
        let words = life.today.words();
        assert!(words.contains("a feather and an acorn"));
        assert!(!words.chars().any(|character| character.is_ascii_digit()));
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
            "Ate four light motes, nudged the ball, watered the plant until it flowered, and saw a \
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
            "Fetched the ball for you three times and danced a twirl. You were here for about half \
             an hour."
        );
        day = Tally {
            naps: 2,
            showers: 1,
            ..Tally::default()
        };
        assert_eq!(
            day.words(),
            "Took two naps and listened to the rain. You didn't come by."
        );
        day = Tally {
            finds: 1,
            doodles: 2,
            ..Tally::default()
        };
        assert_eq!(
            day.words(),
            "Played hide-and-seek, and you found me and drew two pictures on the glass. You \
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
    fn a_damaged_archived_letter_or_long_anniversary_line_cannot_break_the_next_save() {
        let json = r#"{
            "day": "2026-10-21",
            "letters": [{
                "day": "2026-10-21",
                "lines": ["Dear you,", "Thank you for your letter."],
                "korean": false,
                "read": false,
                "reply_to": {
                    "day": "2026-10-20",
                    "lines": ["Hello"],
                    "korean": false,
                    "answered": false,
                    "read": true,
                    "reply": {"arrival": "2026-10-20T10:00:00", "morning": "2026-10-21T07:00:00",
                              "due": "2026-10-20T10:30:00", "remaining": 1e100}
                }
            }],
            "today": {"anniversary_words": ["A week ago today, I saw my first bird.", "", "x\u0007"]}
        }"#;
        let life: Life = serde_json::from_str(json).unwrap();
        let life = life.sanitized();
        assert_eq!(life.letters.len(), 1);
        let saved = serde_json::to_string(&life).unwrap();
        let again: Life = serde_json::from_str(&saved).unwrap();
        assert_eq!(again.letters, life.letters);
        assert_eq!(life.today.anniversary_words.len(), 1);
    }

    #[test]
    fn a_clock_that_goes_back_keeps_the_later_day_and_its_tally() {
        let mut life = Life::default();
        life.begin_day("2026-10-21");
        life.note(&EventKind::AteMote);
        life.pass(600.0, true);
        let warmed = life.warmed;
        life.begin_day("2026-10-20");
        assert_eq!(life.day, "2026-10-21");
        assert_eq!(life.today.motes, 1);
        assert_eq!(life.warmed, warmed);
        assert!(life.diary.is_empty());
        life.begin_day("2026-10-22");
        assert_eq!(life.diary.len(), 1);
    }

    #[test]
    fn a_save_from_before_gift_decisions_still_keeps_todays_gift_given() {
        let life: Life = serde_json::from_str(r#"{"gift_day": "2026-10-21"}"#).unwrap();
        let life = life.sanitized();
        assert_eq!(life.gift_decision, Some(("2026-10-21".to_owned(), false)));
        assert!(!serde_json::to_string(&life).unwrap().contains("gift_day"));
    }

    #[test]
    fn a_diary_entry_keeps_only_what_stood_out_most_first() {
        let day = Tally {
            motes: 300,
            nudges: 120,
            dances: 40,
            doodles: 30,
            naps: 4,
            pounces: 12,
            showers: 1,
            rainbows: 1,
            stories: 1,
            pages: 20,
            towers: 9,
            occasions: vec![Occasion::Halloween],
            viewer_names: vec!["Sam".to_owned()],
            together: 3900.0,
            heard: 3,
            ..Tally::default()
        };
        let diary = day.diary();
        assert!(
            diary.starts_with("Celebrated Halloween, learned your name: Sam, "),
            "{diary}"
        );
        for kept in ["told you a story", "saw a rainbow", "read twenty pages"] {
            assert!(diary.contains(kept), "{diary}");
        }
        for left in ["light motes", "the ball", "twirls", "naps"] {
            assert!(!diary.contains(left), "{diary}");
            assert!(day.words().contains(left));
        }
        assert!(diary.ends_with("You were here for about an hour, and we talked."));
        // A small day keeps everything, the most notable first.
        let quiet = Tally {
            motes: 4,
            naps: 1,
            ..Tally::default()
        };
        assert_eq!(
            quiet.diary(),
            "Took a nap and ate four light motes. You didn't come by."
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
        assert!(life.diary[0].text.starts_with("Ate two light motes"));
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
        assert!((life.bond - 0.006).abs() < 1e-4);
        life.begin_day("2026-10-21");
        assert_eq!((life.today.pounces, life.total.pounces), (0, 2));
        assert!(
            life.diary[0]
                .text
                .starts_with("Pounced on your finger two times.")
        );
        assert_eq!(serde_json::from_str::<Tally>("{}").unwrap().pounces, 0);
        let saved = serde_json::to_string(&life).unwrap();
        assert_eq!(serde_json::from_str::<Life>(&saved).unwrap(), life);
    }

    #[test]
    fn counts_at_their_limit_from_a_damaged_save_stay_there() {
        let mut tally = Tally {
            motes: u32::MAX,
            moon_wishes: u32::MAX,
            planted: u32::MAX,
            ..Tally::default()
        };
        tally.activities.insert("dance".to_owned(), u32::MAX);
        for kind in [
            EventKind::AteMote,
            EventKind::MoonWish,
            EventKind::Planted(super::super::Flower::Daisy),
            EventKind::Began(Activity::Dance),
        ] {
            tally.note(&kind);
        }
        assert_eq!(tally.motes, u32::MAX);
        assert_eq!(tally.moon_wishes, u32::MAX);
        assert_eq!(tally.planted, u32::MAX);
        assert_eq!(tally.activities["dance"], u32::MAX);
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
        // Company through most of each day.
        let mut life = Life::default();
        let mut after = Vec::new();
        for day in 1..=60 {
            life.begin_day(&format!(
                "2026-{:02}-{:02}",
                1 + (day - 1) / 28,
                1 + (day - 1) % 28
            ));
            for _ in 0..(8 * 60) {
                life.pass(60.0, true);
            }
            after.push(life.bond);
        }
        assert!((0.3..0.4).contains(&after[0]), "{after:?}");
        assert!(after[1] < 0.55 && after[4] >= 0.55, "{after:?}");
        assert!(after[7] < 0.8 && after[13] >= 0.8, "{after:?}");
        assert!(after[27] < 0.95 && after[59] >= 0.95, "{after:?}");
        let gains: Vec<f32> = after.windows(2).map(|pair| pair[1] - pair[0]).collect();
        assert!(gains.windows(2).all(|pair| pair[1] < pair[0]), "{gains:?}");
        let before = life.bond;
        life.pass(3600.0, false);
        assert_eq!(life.bond, before);
        assert_eq!(life.total.together, 60.0 * 8.0 * 3600.0);
    }

    #[test]
    fn hours_spread_over_days_bring_the_ghost_closer_than_one_long_day() {
        let mut long_day = Life::default();
        long_day.begin_day("2026-10-01");
        for _ in 0..12 {
            long_day.pass(3600.0, true);
        }
        let mut spread = Life::default();
        for day in 1..=6 {
            spread.begin_day(&format!("2026-10-{day:02}"));
            spread.pass(3600.0, true);
            spread.pass(3600.0, true);
        }
        assert_eq!(long_day.total.together, spread.total.together);
        assert!(long_day.bond < 0.4, "{}", long_day.bond);
        assert!(
            spread.bond > long_day.bond + 0.2,
            "{} {}",
            spread.bond,
            long_day.bond
        );
        // A day's warmth from talk and play wears off the same way.
        let mut chatty = Life::default();
        for _ in 0..200 {
            chatty.note(&EventKind::Heard("hello".into()));
        }
        assert!(chatty.bond < DAILY_WARMTH, "{}", chatty.bond);
        assert!(chatty.warmed <= DAILY_WARMTH);
        chatty.begin_day("2026-10-02");
        assert_eq!(chatty.warmed, 0.0);
        let before = chatty.bond;
        chatty.note(&EventKind::Heard("hello".into()));
        assert!(chatty.bond - before > 0.005);
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
            life.warmed = 0.0;
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
    fn pastimes_done_often_today_draw_the_ghost_less_until_the_next_day() {
        let mut life = Life::default();
        life.begin_day("2026-10-20");
        assert_eq!(life.novelty(Activity::Music), 1.0);
        for _ in 0..5 {
            life.note(&EventKind::Began(Activity::Music));
            life.note(&EventKind::Began(Activity::Rest));
            life.note(&EventKind::Began(Activity::Chase));
        }
        assert!((life.novelty(Activity::Music) - 0.5).abs() < 1e-6);
        assert_eq!(life.novelty(Activity::Rest), 1.0);
        assert_eq!(life.novelty(Activity::Chase), 1.0);
        life.begin_day("2026-10-21");
        assert_eq!(life.novelty(Activity::Music), 1.0);
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
        for day in 1..=20 {
            life.begin_day(&format!("2026-10-{day:02}"));
            for _ in 0..(6 * 60) {
                if let Some(text) = life.pass(60.0, true) {
                    reached.push(text);
                }
            }
        }
        assert_eq!(
            reached,
            [
                "We spent our first hour together.",
                "We got to know each other.",
                "We've spent ten hours together.",
                "I've grown fond of you.",
                "We're close now, you and I.",
                "We've spent a hundred hours together.",
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
            Occasion::Seollal,
            Occasion::Chuseok,
            Occasion::HangulDay,
        ] {
            life.note(&EventKind::Celebrated(occasion));
        }
        life.note(&EventKind::Celebrated(Occasion::NewYear));
        assert_eq!(life.total.celebrations, 9);
        assert_eq!(life.today.celebrations, 9);
        let words = life.today.words();
        assert!(words.contains("Celebrated Halloween"));
        assert!(words.contains("celebrated the winter holidays"));
        assert!(words.contains("celebrated Valentine's Day"));
        assert!(words.contains("had a birthday"));
        assert!(words.contains("celebrated Seollal"));
        assert!(words.contains("celebrated Chuseok"));
        assert!(words.contains("celebrated Hangul Day"));
        for occasion in [Occasion::Seollal, Occasion::Chuseok] {
            assert!(life.note(&EventKind::Celebrated(occasion)).is_none());
            assert_eq!(
                life.milestones
                    .iter()
                    .filter(|entry| entry.text.contains(occasion.name()))
                    .count(),
                1
            );
        }
        assert_eq!(words.matches("celebrated the new year").count(), 1);
        let restored: Life = serde_json::from_str(&serde_json::to_string(&life).unwrap()).unwrap();
        assert_eq!(life, restored);
        life.begin_day("2026-11-01");
        assert_eq!(life.today.celebrations, 0);
        assert_eq!(life.total.celebrations, 11);
        assert!(
            life.diary[0]
                .text
                .starts_with("Celebrated Halloween, celebrated the winter")
        );
        assert_eq!(serde_json::from_str::<Tally>("{}").unwrap().celebrations, 0);
    }

    #[test]
    fn towers_count_height_and_viewer_demolitions_and_keep_both_first_and_tallest_milestones() {
        let mut life = Life::default();
        assert_eq!(
            life.note(&EventKind::Built { height: 5 }).as_deref(),
            Some("I built my first tower of blocks.")
        );
        assert!(
            life.milestones
                .iter()
                .any(|entry| entry.text == "I built a tower five blocks high.")
        );
        life.note(&EventKind::Built { height: 3 });
        life.note(&EventKind::Toppled { by_viewer: false });
        life.note(&EventKind::Toppled { by_viewer: true });
        assert_eq!(life.total.towers, 2);
        assert_eq!(life.total.tallest, 5);
        assert_eq!(life.today, life.total);
        assert_eq!(life.total.toppled, 1);
        assert_eq!(life.milestones.len(), 2);
        assert!(
            life.today
                .words()
                .contains("Built two towers of blocks, and you knocked one down")
        );
        assert!(life.milestones.iter().all(|entry| {
            !entry
                .text
                .chars()
                .any(|character| character.is_ascii_digit())
        }));
        let saved = serde_json::to_string(&life).unwrap();
        assert_eq!(serde_json::from_str::<Life>(&saved).unwrap(), life);
        let older: Life = serde_json::from_str(r#"{"total":{"motes":2},"today":{}}"#).unwrap();
        assert_eq!(
            (older.total.towers, older.total.tallest, older.total.toppled),
            (0, 0, 0)
        );
        life.total.tallest = u32::MAX;
        assert_eq!(life.sanitized().total.tallest, 5);
    }

    #[test]
    fn names_and_writing_are_backward_compatible_and_survive_a_saved_life() {
        let old: Life = serde_json::from_str(r#"{"bond":0.6,"total":{"doodles":3}}"#).unwrap();
        assert_eq!(old.viewer_name, None);
        assert_eq!(old.total.writings, 0);
        assert_eq!(old.total.watched_writings, 0);
        let mut life = old;
        life.viewer_name = Some("민수".to_owned());
        life.note(&EventKind::LearnedName("민수".to_owned()));
        for _ in 0..3 {
            life.note(&EventKind::Wrote {
                text: "HI".to_owned(),
                mirrored: true,
                watched: true,
            });
        }
        let saved = serde_json::to_string(&life).unwrap();
        let loaded: Life = serde_json::from_str(&saved).unwrap();
        assert_eq!(loaded.clone().sanitized(), life);
        assert_eq!(loaded.total.watched_writings, 3);
        assert_eq!(loaded.viewer_name.as_deref(), Some("민수"));
    }

    #[test]
    fn damaged_saved_names_and_writing_counts_are_brought_back_in_range() {
        let life = Life {
            viewer_name: Some("Mina123\u{1b}".to_owned()),
            total: Tally {
                writings: 2,
                watched_writings: 900,
                written_words: vec!["HI\u{1b}".to_owned(); 20],
                viewer_names: vec!["bad\nname".to_owned(); 10],
                ..Default::default()
            },
            ..Default::default()
        }
        .sanitized();
        assert_eq!(life.viewer_name, None);
        assert_eq!(life.total.watched_writings, 2);
        assert_eq!(life.total.written_words, vec!["HI"; WORDS_NAMED]);
        assert!(life.total.viewer_names.is_empty());
    }
    #[test]
    fn viewer_art_is_notable_and_a_named_doodle_is_a_small_diary_deed() {
        let mut tally = Tally::default();
        tally.note(&EventKind::Pinned {
            kind: super::super::gallery::Kind::ViewerDrawing,
            name: "heart".to_owned(),
        });
        tally.note(&EventKind::Pinned {
            kind: super::super::gallery::Kind::OwnDoodle,
            name: "star".to_owned(),
        });
        let deeds = tally.deeds();
        assert!(deeds.contains(&(
            Salience::Notable,
            "pinned your drawing to my wall".to_owned(),
            "네 그림을 내 벽에 붙였어".to_owned()
        )));
        assert!(deeds.contains(&(
            Salience::Small,
            "pinned my star doodle to the wall".to_owned(),
            "내가 그린 별 그림을 벽에 붙였어".to_owned()
        )));
        assert_eq!(tally.viewer_pins, 1);
        assert_eq!(tally.doodle_pins, 1);
    }

    #[test]
    fn every_deed_pairs_its_english_with_korean_including_all_saved_details() {
        use crate::world::{Feeling, Flower, Passerby, Quirk, Wish, agenda::Errand, friend::Game};
        for count in [1, 2, 6] {
            let mut saved = serde_json::to_value(Tally::default()).unwrap();
            for value in saved.as_object_mut().unwrap().values_mut() {
                if value.is_number() {
                    *value = serde_json::json!(count);
                }
                if value.is_boolean() {
                    *value = serde_json::json!(true);
                }
            }
            let mut tally: Tally = serde_json::from_value(saved).unwrap();
            tally.occasions = vec![
                Occasion::Halloween,
                Occasion::WinterHolidays,
                Occasion::NewYear,
                Occasion::Valentine,
                Occasion::Birthday,
                Occasion::Seollal,
                Occasion::Chuseok,
                Occasion::HangulDay,
                Occasion::ViewerBirthday,
                Occasion::HundredDays,
                Occasion::ThousandDays,
            ];
            tally.anniversary_words =
                vec!["A week ago today, I learned your name: Mina.".to_owned()];
            tally.korean_details.insert(
                tally.anniversary_words[0].clone(),
                "일주일 전 오늘, 네 이름을 알게 됐어: Mina".to_owned(),
            );
            tally.temperament_changes = vec!["I'm not so shy anymore.".to_owned()];
            tally.words = vec!["fetch".to_owned()];
            tally.written_words = vec!["MINA".to_owned()];
            tally.viewer_names = vec!["Mina".to_owned()];
            tally.friend_name = Some("Pell".to_owned());
            tally.friend_played = vec![Game::Tag, Game::Twirl, Game::MoonGazing];
            tally.keepsakes = vec![Keepsake::Feather, Keepsake::Acorn];
            tally.planted_kinds = vec![
                Flower::Daisy,
                Flower::Tulip,
                Flower::Sunflower,
                Flower::Bluebell,
                Flower::Poppy,
            ];
            tally.sprouted_kinds = tally.planted_kinds.clone();
            tally.flowered_kinds = tally.planted_kinds.clone();
            tally.passerby_kinds = Passerby::ALL.to_vec();
            tally.quirks = Quirk::ALL.into_iter().map(|quirk| (quirk, count)).collect();
            tally.feelings = vec![
                Feeling::Happy,
                Feeling::Sad,
                Feeling::Tired,
                Feeling::Lonely,
                Feeling::Stressed,
                Feeling::Angry,
                Feeling::Unwell,
                Feeling::Bored,
                Feeling::Proud,
            ];
            tally.comforted_feelings = tally.feelings.clone();
            tally.composition_titles = vec!["My Tune".to_owned()];
            tally.written_titles = vec!["My Book".to_owned()];
            tally
                .korean_details
                .insert("My Tune".to_owned(), "작은 노래".to_owned());
            tally
                .korean_details
                .insert("My Book".to_owned(), "작은 책".to_owned());
            tally.heard_plans = vec![
                Errand::Exam,
                Errand::Interview,
                Errand::Presentation,
                Errand::Meeting,
                Errand::Appointment,
                Errand::Date,
                Errand::Match,
                Errand::Performance,
                Errand::Deadline,
                Errand::Party,
                Errand::Moving,
                Errand::FirstDay,
                Errand::Result,
            ];
            tally.wished_plans = tally.heard_plans.clone();
            tally.good_plans = tally.heard_plans.clone();
            tally.hard_plans = tally.heard_plans.clone();
            tally.fastest_treasure = Some(10);
            tally.note(&EventKind::MadeWish(Wish::Book { index: 0 }));
            tally.note(&EventKind::WishGranted(Wish::Rainbow));
            tally.note(&EventKind::WishFaded(Wish::Snow));
            for (_, english, korean) in tally.deeds() {
                assert!(!english.is_empty());
                assert!(!korean.is_empty(), "{english}");
                let prose = ["Mina", "Pell", "MINA", "fetch"]
                    .iter()
                    .fold(korean.clone(), |prose, name| prose.replace(name, ""));
                assert!(
                    !prose.chars().any(|letter| letter.is_ascii_alphabetic()),
                    "{english}: {korean}"
                );
            }
            // Feelings merely heard use their own paired phrases as well.
            tally.comforted_feelings.clear();
            for (_, english, korean) in tally.deeds() {
                assert!(!korean.is_empty(), "{english}");
            }
        }
    }

    #[test]
    fn days_keep_both_languages_and_recall_only_the_top_three_deeds() {
        let mut life = Life::default();
        life.begin("2026-10-20");
        for event in [
            EventKind::AteMote,
            EventKind::Petted,
            EventKind::Built { height: 3 },
            EventKind::LearnedName("Mina".to_owned()),
            EventKind::GotLetter,
        ] {
            life.note(&event);
        }
        life.pass(7.0 * 3600.0, true);
        life.note(&EventKind::Heard("hello".to_owned()));
        life.begin_day("2026-10-21");
        assert_eq!(life.yesterday.len(), 3);
        assert!(life.diary[0].text.contains("Mina"));
        assert!(
            life.diary[0]
                .korean
                .as_ref()
                .unwrap()
                .contains("일곱 시간쯤 같이 있었고, 이야기도 나눴어")
        );
        assert!(
            life.recall_yesterday("2026-10-21".parse().unwrap(), false)
                .contains("Mina")
        );
        assert!(
            life.recall_yesterday("2026-10-21".parse().unwrap(), true)
                .contains("네 이름")
        );
        assert!(
            !life
                .recall_yesterday("2026-10-21".parse().unwrap(), true)
                .contains("빛 알갱이")
        );
        let again: Life = serde_json::from_str(&serde_json::to_string(&life).unwrap()).unwrap();
        assert_eq!(again, life);
        life.begin_day("2026-10-23");
        assert_eq!(
            life.recall_yesterday("2026-10-23".parse().unwrap(), true),
            "조용한 하루였어."
        );
    }

    #[test]
    fn old_writing_and_deeds_without_a_translation_fall_back_to_english() {
        let old: Entry =
            serde_json::from_str(r#"{"day":"2026-10-20","text":"A quiet day."}"#).unwrap();
        assert_eq!(old.korean, None);
        assert_eq!(old.in_language(true), "A quiet day.");
        let tally = Tally {
            wish_words: vec!["a future deed".to_owned()],
            anniversary_words: vec!["A week ago today, I found a treasure.".to_owned()],
            ..Default::default()
        };
        assert!(tally.words_korean().contains("a future deed"));
        assert!(tally.words_korean().contains("remembered a week ago today"));
        assert!(tally.diary_korean().contains("a future deed"));
        assert_eq!(
            serde_json::from_str::<Life>("{}").unwrap().yesterday,
            Vec::new()
        );
    }

    #[test]
    fn damaged_korean_is_dropped_without_losing_the_english_entry() {
        for korean in [
            "".to_owned(),
            "  ".to_owned(),
            "안녕\n친구".to_owned(),
            "가".repeat(super::super::writing::TEXT_LIMIT + 1),
        ] {
            let entry = Entry {
                day: "2026-10-20".to_owned(),
                text: "A quiet day.".to_owned(),
                korean: Some(korean),
            };
            let life = Life {
                day: "2026-10-21".to_owned(),
                diary: vec![entry.clone()],
                milestones: vec![entry.clone()],
                yesterday: vec![entry],
                ..Default::default()
            }
            .sanitized();
            for entry in life
                .diary
                .iter()
                .chain(&life.milestones)
                .chain(&life.yesterday)
            {
                assert_eq!(entry.korean, None);
                assert_eq!(entry.text, "A quiet day.");
            }
        }
        let valid = Entry {
            day: "2026-10-20".to_owned(),
            text: "A quiet day.".to_owned(),
            korean: Some("조용한 하루였어.".to_owned()),
        };
        let life = Life {
            day: "2026-10-21".to_owned(),
            diary: vec![valid.clone()],
            yesterday: vec![
                valid.clone(),
                Entry {
                    day: "broken".to_owned(),
                    ..valid.clone()
                },
                Entry {
                    day: "2026-10-22".to_owned(),
                    ..valid
                },
            ],
            ..Default::default()
        }
        .sanitized();
        assert_eq!(life.yesterday.len(), 1);
        assert_eq!(life.diary[0].in_language(true), "조용한 하루였어.");
    }

    #[test]
    fn firsts_landmarks_growth_skills_and_upbringing_keep_korean_beside_english_events() {
        use crate::world::{Feeling, Skill, Wish, World};
        let mut life = Life::default();
        life.begin("2026-10-20");
        let events = [
            EventKind::AteMote,
            EventKind::Petted,
            EventKind::GotLetter,
            EventKind::LearnedName("Mina".to_owned()),
            EventKind::Named("Pip".to_owned()),
            EventKind::Composed("My Tune".to_owned()),
            EventKind::WroteBook("My Book".to_owned()),
            EventKind::Comforted(Feeling::Sad),
            EventKind::WishGranted(Wish::Rainbow),
            EventKind::FoldedCrane { count: 1 },
            EventKind::FinishedBook("Book".to_owned()),
            EventKind::ForecastCameTrue { snow: false },
            EventKind::MetReflection,
            EventKind::Celebrated(Occasion::HundredDays),
        ];
        for event in events {
            let english = life.note(&event).unwrap();
            let entry = life
                .milestones
                .iter()
                .find(|entry| entry.text == english)
                .unwrap();
            assert!(
                entry.korean.as_ref().is_some_and(|text| !text.is_empty()),
                "{english}"
            );
            assert!(
                !english
                    .chars()
                    .any(|letter| ('가'..='힣').contains(&letter))
            );
        }
        life.pass(3600.0, true);
        life.watch_bird(100_000.0);
        life.upbringing.drift = [0.15; 5];
        life.begin_day("2026-10-21");
        assert!(
            life.milestones
                .iter()
                .any(|entry| entry.text.contains("playful"))
        );
        let mut world = World::new(7, 12.0);
        world.birthday = Some(world.date.checked_add(jiff::Span::new().days(-40)).unwrap());
        world.mark_growth();
        for skill in Skill::ALL {
            world
                .life
                .reach(skill.milestone().to_owned(), skill.milestone_korean());
        }
        for entry in life.milestones.iter().chain(&world.life.milestones) {
            assert!(
                entry.korean.as_ref().is_some_and(|text| !text.is_empty()),
                "{}",
                entry.text
            );
        }
        let count = life.milestones.len();
        assert!(
            life.reach(
                "I came into being in a glass box.".to_owned(),
                "다른 한국어".to_owned()
            )
            .is_none()
        );
        assert_eq!(life.milestones.len(), count);
    }
    #[test]
    fn creation_and_constellation_milestones_use_saved_korean_titles() {
        use crate::world::{
            creation::WrittenBook,
            music::{Composition, Note, Tune},
            stars::Constellation,
        };
        let mut life = Life::default();
        life.begin_day("2026-10-21");
        life.compositions.push(Composition {
            title: "My Tune".to_owned(),
            korean_title: "나의 노래".to_owned(),
            day: life.day.clone(),
            tune: Tune {
                name: "My Tune".to_owned(),
                notes: vec![Note {
                    pitch: 0,
                    beats: 1.0,
                }],
            },
        });
        life.written.push(WrittenBook {
            title: ("My Book".to_owned(), "나의 책".to_owned()),
            ..Default::default()
        });
        life.constellations.push(Constellation {
            name: "Little Heart".to_owned(),
            korean: "작은 하트".to_owned(),
            ..Default::default()
        });
        for (event, title) in [
            (EventKind::Composed("My Tune".to_owned()), "나의 노래"),
            (EventKind::WroteBook("My Book".to_owned()), "나의 책"),
            (
                EventKind::FoundConstellation("Little Heart".to_owned()),
                "작은 하트",
            ),
        ] {
            let english = life.note(&event).unwrap();
            assert!(
                life.milestones
                    .iter()
                    .find(|entry| entry.text == english)
                    .unwrap()
                    .in_language(true)
                    .contains(title)
            );
        }
        assert!(life.today.words_korean().contains("나의 노래"));
        assert!(life.today.words_korean().contains("나의 책"));
        assert!(life.today.words().contains("My Tune"));
    }

    #[test]
    fn paired_deed_details_are_bounded_and_sanitized() {
        let mut life = Life {
            day: "2026-10-21".to_owned(),
            ..Default::default()
        };
        for index in 0..100 {
            life.today
                .remember_writing(format!("title {index}"), "이름".to_owned());
        }
        assert_eq!(life.today.korean_details.len(), 64);
        life.total
            .korean_details
            .insert("damaged".to_owned(), "잘못된\n글".to_owned());
        life = life.sanitized();
        assert!(life.total.korean_details.is_empty());
        assert_eq!(life.today.korean_details.len(), 64);
    }
}
