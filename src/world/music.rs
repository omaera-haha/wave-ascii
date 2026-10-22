use std::f32::consts::TAU;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{Activity, EventKind, Heard, Listening, Understanding, World};

pub const POSITION: Vec3 = Vec3::new(1.72, 0.0, 0.25);
pub const RADIUS: f32 = 0.25;
pub const HEIGHT: f32 = 0.43;
pub const LISTEN: Vec3 = Vec3::new(1.12, 0.85, 0.4);
const MAX_NOTES: usize = 24;
const SOLFEGE: [&str; 8] = ["do", "re", "mi", "fa", "sol", "la", "ti", "do"];

pub fn hit_box(origin: Vec3, direction: Vec3) -> Option<f32> {
    let low = POSITION + Vec3::new(-0.21, 0.0, -0.23);
    let high = POSITION + Vec3::new(0.27, 0.43, 0.19);
    let mut enter = 0.0_f32;
    let mut exit = f32::INFINITY;
    for axis in 0..3 {
        if direction[axis].abs() < 1e-6 {
            if origin[axis] < low[axis] || origin[axis] > high[axis] {
                return None;
            }
        } else {
            let first = (low[axis] - origin[axis]) / direction[axis];
            let second = (high[axis] - origin[axis]) / direction[axis];
            enter = enter.max(first.min(second));
            exit = exit.min(first.max(second));
        }
    }
    (exit >= enter).then_some(enter)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    #[serde(default)]
    pub pitch: i8,
    #[serde(default = "one_beat")]
    pub beats: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tune {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub notes: Vec<Note>,
}

fn one_beat() -> f32 {
    1.0
}

impl Tune {
    fn named(notes: Vec<Note>) -> Self {
        let name = notes
            .iter()
            .take(6)
            .map(|note| SOLFEGE[note.pitch as usize])
            .collect::<Vec<_>>()
            .join("-");
        Self { name, notes }
    }

    pub fn sanitized(mut self) -> Option<Self> {
        self.notes.truncate(MAX_NOTES);
        for note in &mut self.notes {
            note.pitch = note.pitch.clamp(0, 7);
            note.beats = if note.beats.is_finite() {
                note.beats.clamp(0.5, 4.0)
            } else {
                1.0
            };
        }
        if self.notes.is_empty() {
            return None;
        }
        self.name = self
            .name
            .chars()
            .filter(|character| character.is_alphabetic() || matches!(character, ' ' | '-'))
            .take(64)
            .collect();
        if self.name.trim().is_empty() {
            self.name = Self::named(self.notes.clone()).name;
        }
        Some(self)
    }
}

/// Dashes between syllables separate them; a dash before whitespace or the
/// end, and each tilde, holds the preceding note for another beat.
pub fn lesson(text: &str) -> Option<Tune> {
    let text = text.trim().to_lowercase();
    let characters: Vec<char> = text.chars().collect();
    let mut notes: Vec<Note> = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character.is_whitespace() {
            index += 1;
            continue;
        }
        if matches!(character, '-' | '~') {
            let previous = *characters.get(index.checked_sub(1)?)?;
            if previous.is_whitespace() {
                let separated = character == '-'
                    && characters[index + 1..]
                        .iter()
                        .find(|next| !next.is_whitespace())
                        .is_some_and(|next| next.is_alphabetic());
                if !separated {
                    return None;
                }
                index += 1;
                continue;
            }
            let next = characters.get(index + 1);
            if character == '~'
                || next.is_none_or(|next| next.is_whitespace() || matches!(next, '-' | '~'))
            {
                let note = notes.last_mut()?;
                note.beats = (note.beats + 1.0).min(4.0);
            }
            index += 1;
            continue;
        }
        let pitch = match character {
            '도' => 0,
            '레' => 1,
            '미' => 2,
            '파' => 3,
            '솔' => 4,
            '라' => 5,
            '시' => 6,
            _ if character.is_ascii_alphabetic() => {
                let start = index;
                while index < characters.len() && characters[index].is_ascii_alphabetic() {
                    index += 1;
                }
                let word: String = characters[start..index].iter().collect();
                index -= 1;
                match word.as_str() {
                    "do" => 0,
                    "re" => 1,
                    "mi" => 2,
                    "fa" => 3,
                    "so" | "sol" => 4,
                    "la" => 5,
                    "ti" | "si" => 6,
                    _ => return None,
                }
            }
            _ => return None,
        };
        notes.push(Note { pitch, beats: 1.0 });
        if notes.len() > MAX_NOTES {
            return None;
        }
        index += 1;
    }
    (notes.len() >= 3).then(|| Tune::named(notes))
}

/// These little melodies are composed for the room.
pub fn built_in(index: usize) -> Tune {
    let (name, pitches, lengths): (&str, &[i8], &[f32]) = match index % 4 {
        0 => (
            "little lullaby",
            &[4, 2, 3, 1, 2, 0, 1, 0],
            &[2.0, 2.0, 1.0, 1.0, 2.0, 2.0, 2.0, 4.0],
        ),
        1 => (
            "turning waltz",
            &[0, 2, 5, 4, 1, 3, 6, 4, 2, 1, 0],
            &[1.0, 1.0, 1.0, 3.0, 1.0, 1.0, 1.0, 3.0, 1.0, 1.0, 4.0],
        ),
        2 => (
            "tiny march",
            &[0, 3, 2, 4, 1, 5, 4, 2, 0],
            &[1.0, 0.5, 0.5, 2.0, 1.0, 0.5, 0.5, 1.0, 2.0],
        ),
        _ => (
            "mote sparkle",
            &[0, 2, 4, 7, 5, 6, 3, 4, 2, 0],
            &[1.0, 0.5, 0.5, 2.0, 1.0, 1.0, 0.5, 0.5, 2.0, 2.0],
        ),
    };
    Tune {
        name: name.to_owned(),
        notes: pitches
            .iter()
            .zip(lengths)
            .map(|(&pitch, &beats)| Note { pitch, beats })
            .collect(),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    pub tune: Tune,
    pub title: String,
    pub korean_title: String,
    pub day: String,
}

/// Two related phrases: a repeated opening, a wandering answer, and a tonic.
pub fn compose(random: &mut super::Random, mood: super::Mood, nature: super::Nature) -> Vec<Note> {
    let bright = matches!(mood, super::Mood::Cheerful | super::Mood::Excited);
    let quiet = matches!(
        mood,
        super::Mood::Content | super::Mood::Sleepy | super::Mood::Lonely
    );
    let length = random.range(4.0, 9.0) as usize;
    let center = if bright {
        5
    } else if quiet {
        2
    } else {
        3
    };
    let mut pitch = center;
    let mut notes: Vec<Note> = Vec::with_capacity(length * 2);
    for index in 0..length * 2 {
        if index == length * 2 - 1 {
            pitch = 0;
        } else if (length..length + 2).contains(&index) {
            pitch = notes[index - length].pitch;
        } else if index > 0 {
            let interval = match random.unit() {
                value if value < 0.8 => 1,
                value if value < 0.96 => 2,
                _ => 4,
            };
            let rising = random.unit() < if pitch < center { 0.7 } else { 0.3 };
            pitch = (pitch + if rising { interval } else { -interval }).clamp(0, 6);
        }
        let held = random.unit()
            < if quiet {
                0.55 + nature.tender * 0.15
            } else {
                0.18
            };
        let beats = if index == length * 2 - 1 {
            2.0
        } else if held {
            if quiet && random.unit() < 0.3 {
                4.0
            } else {
                2.0
            }
        } else if bright && random.unit() < 0.5 + nature.playful * 0.2 {
            0.5
        } else {
            1.0
        };
        notes.push(Note { pitch, beats });
    }
    notes
}

/// A title for a new song from what fills the ghost's life now: each fitting
/// inspiration offers a few titles, and the first one not already among its
/// songs wins, taking every inspiration's first title before any second, so
/// a beloved viewer doesn't name every song.
pub fn composition_title(world: &World) -> (String, String) {
    let life = &world.life;
    let mut inspirations: Vec<Vec<(String, String)>> = Vec::new();
    if let Some(name) = &life.viewer_name
        && life.bond >= 0.3
    {
        inspirations.push(vec![
            (
                format!("Song for {name}"),
                format!("{name}에게 바치는 노래"),
            ),
            (format!("{name}'s Lullaby"), format!("{name}의 자장가")),
            (
                format!("A Tune for {name} at the Glass"),
                format!("유리 너머 {name}에게"),
            ),
            (
                format!("{name}, Come Back Soon"),
                format!("{name}, 또 와 줘"),
            ),
        ]);
    }
    if world.weather.sky == super::Sky::Rain {
        inspirations.push(titles(&[
            ("Rain Song", "비의 노래"),
            ("Raindrops on the Pane", "창문의 빗방울"),
            ("Puddle Waltz", "물웅덩이 왈츠"),
        ]));
    } else if world.weather.sky == super::Sky::Snow {
        inspirations.push(titles(&[
            ("Snow Song", "눈의 노래"),
            ("Snowflake Waltz", "눈송이 왈츠"),
            ("The Quiet After Snow", "눈 온 뒤의 고요"),
        ]));
    }
    if let Some(friend) = &life.friend
        && super::creation::elapsed_days(life.last_friend_visit.as_deref(), world.date)
            .is_some_and(|days| (0..=1).contains(&days))
    {
        let name = &friend.name;
        inspirations.push(vec![
            (format!("{name}'s Waltz"), format!("{name}의 왈츠")),
            (
                format!("Tag with {name}"),
                format!("{name}의 술래잡기 노래"),
            ),
            (
                format!("Moon Gazing with {name}"),
                format!("{name}의 달맞이"),
            ),
        ]);
    }
    if world.is_night() {
        inspirations.push(titles(&[
            ("Moon Song", "달의 노래"),
            ("Lamplight Lullaby", "램프 불빛 자장가"),
            ("Starry Window", "별빛 창문"),
        ]));
    }
    if life.bird_trust >= 0.5 {
        inspirations.push(titles(&[
            ("Bird Song", "새의 노래"),
            ("Song for the Sill", "창턱의 노래"),
            ("Feather Waltz", "깃털 왈츠"),
        ]));
    }
    if world.hour < 10.0 {
        inspirations.push(titles(&[
            ("Morning Song", "아침의 노래"),
            ("Sunrise Stretch", "해돋이 기지개"),
        ]));
    }
    inspirations.push(titles(match world.season() {
        super::Season::Spring => &[("Blossom Song", "봄꽃의 노래"), ("Petal Dance", "꽃잎 춤")],
        super::Season::Summer => &[
            ("Little Song", "작은 노래"),
            ("Firefly Song", "반딧불이 노래"),
        ],
        super::Season::Autumn => &[
            ("Autumn Song", "가을의 노래"),
            ("Falling Leaves", "떨어지는 잎새"),
        ],
        super::Season::Winter => &[
            ("Winter Song", "겨울의 노래"),
            ("Warm Cushion Song", "따뜻한 방석의 노래"),
        ],
    }));
    let taken = |title: &(String, String)| {
        life.compositions
            .iter()
            .any(|known| known.title == title.0 || known.korean_title == title.1)
    };
    let rounds = inspirations.iter().map(Vec::len).max().unwrap_or(0);
    if let Some(title) = (0..rounds)
        .flat_map(|round| {
            inspirations
                .iter()
                .filter_map(move |titles| titles.get(round))
        })
        .find(|title| !taken(title))
    {
        return title.clone();
    }
    let (english, korean) = inspirations[0][0].clone();
    let mut title = (english.clone(), korean.clone());
    let mut repeats = 0;
    while taken(&title) {
        repeats += 1;
        title = if repeats == 1 {
            (format!("{english} Again"), format!("{korean} 또"))
        } else {
            (
                format!("{english} Again {}", super::friend::count_words(repeats)),
                format!("{korean} {}", "또".repeat(repeats as usize)),
            )
        };
    }
    title
}

fn titles(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(english, korean)| ((*english).to_owned(), (*korean).to_owned()))
        .collect()
}

pub fn own_song_requested(text: &str) -> bool {
    matches!(
        text.trim()
            .trim_end_matches(['?', '!', '.'])
            .to_lowercase()
            .as_str(),
        "sing me your song"
            | "play your song"
            | "do you have your own song"
            | "네 노래 불러줘"
            | "네가 만든 노래"
            | "네 노래 들려줘"
    )
}

impl World {
    pub fn composition_words(&self, korean: bool) -> String {
        self.life
            .compositions
            .iter()
            .map(|composition| {
                if korean {
                    composition.korean_title.as_str()
                } else {
                    composition.title.as_str()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn hum(&mut self, tune: &Tune, korean: bool) {
        let syllables = if korean {
            ["도", "레", "미", "파", "솔", "라", "시", "도"]
        } else {
            SOLFEGE
        };
        let notes = tune
            .notes
            .iter()
            .take(6)
            .map(|note| syllables[note.pitch as usize])
            .collect::<Vec<_>>()
            .join(" ");
        self.say(&format!("♪ {notes}… ♪"));
    }

    fn queue_music_request(&mut self, tune: Tune) {
        // A treasure hunt, like work, keeps the lesson for afterwards.
        if self.focus.is_none()
            && self.hunt.is_none()
            && self.ghost.asleep < 0.3
            && self.telling.is_none()
        {
            self.music = None;
            self.begin(Activity::Music);
            self.compose_on_arrival = false;
            self.queued_tune = Some(tune);
            self.pending_tune = None;
        } else {
            self.pending_tune = Some(tune);
        }
    }

    pub fn can_compose(&self) -> bool {
        self.stage() != super::Stage::Newborn
            && self.life.total.tunes_played >= super::creation::COMPOSES_AFTER_TUNES
            && self.life.diary.len() >= super::creation::COMPOSES_AFTER_DAYS
            && self.focus.is_none()
            && self.ghost.asleep < 0.3
            && self.telling.is_none()
            && super::creation::elapsed_days(self.life.composition_day.as_deref(), self.date)
                .is_none_or(|days| days >= 3)
    }

    fn try_compose(&mut self) -> bool {
        if !self.can_compose() {
            return false;
        }
        let nature = self.life.temperament();
        if self.random.unit() >= 0.25 + 0.15 * (nature.playful + nature.tender - 1.0) {
            return false;
        }
        let notes = compose(&mut self.random, self.ghost.mood, nature);
        if self
            .life
            .compositions
            .iter()
            .any(|composition| composition.tune.notes == notes)
        {
            return false;
        }
        let (title, korean_title) = composition_title(self);
        let tune = Tune {
            name: title.clone(),
            notes,
        };
        self.life.composition_day = Some(self.date.to_string());
        self.life.compositions.push(Composition {
            tune: tune.clone(),
            title: title.clone(),
            korean_title,
            day: self.date.to_string(),
        });
        if self.life.compositions.len() > 8 {
            self.life.compositions.remove(0);
        }
        self.record(EventKind::Composed(title));
        self.hum(&tune, self.speaks_korean());
        self.queued_tune = Some(tune);
        true
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Playing {
    pub tune: Tune,
    pub started: f64,
    pub tempo: f32,
    pub lullaby: bool,
    onsets: Vec<f64>,
    emitted: usize,
    beats: f64,
}

impl Playing {
    fn new(tune: Tune, started: f64, lullaby: bool, repeats: usize) -> Self {
        let mut playing = Self {
            tune,
            started,
            tempo: if lullaby { 64.0 } else { 108.0 },
            lullaby,
            onsets: Vec::new(),
            emitted: 0,
            beats: 0.0,
        };
        let mut beats = Vec::new();
        for _ in 0..repeats {
            for note in &playing.tune.notes {
                beats.push(playing.beats);
                playing.beats += f64::from(note.beats);
            }
        }
        playing.onsets = beats
            .into_iter()
            .map(|beat| playing.seconds_at(beat))
            .collect();
        playing
    }

    fn seconds_at(&self, beat: f64) -> f64 {
        let tail = (self.beats * 0.25).min(4.0);
        let progress = ((beat - (self.beats - tail)) / tail).clamp(0.0, 1.0);
        (beat + 0.35 * tail * progress * progress) * 60.0 / f64::from(self.tempo)
    }

    pub fn duration(&self) -> f64 {
        self.seconds_at(self.beats)
    }

    pub fn beat(&self, time: f64) -> f32 {
        let elapsed =
            ((time - self.started).max(0.0) * f64::from(self.tempo) / 60.0).min(self.beats + 1.4);
        let tail = (self.beats * 0.25).min(4.0);
        let straight = self.beats - tail;
        let beat = if elapsed <= straight {
            elapsed
        } else {
            straight + tail * (((1.0 + 1.4 * (elapsed - straight) / tail).sqrt() - 1.0) / 0.7)
        };
        beat as f32
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RisingNote {
    pub born: f64,
    pub pitch: i8,
    pub flagged: bool,
    pub side: f32,
}

impl RisingNote {
    pub const LIFETIME: f64 = 4.0;

    pub fn position(self, time: f64) -> Vec3 {
        let age = (time - self.born).clamp(0.0, Self::LIFETIME) as f32;
        POSITION
            + Vec3::new(
                -0.03 + self.side * age * 0.035,
                0.38 + f32::from(self.pitch) * 0.018 + age * 0.18,
                0.03 + (age * 1.4 + self.side).sin() * 0.055,
            )
    }

    pub fn fade(self, time: f64) -> f32 {
        ((Self::LIFETIME - (time - self.born)) / 1.5).clamp(0.0, 1.0) as f32
    }
}

pub fn lullaby_requested(text: &str) -> bool {
    let text = text.trim().to_lowercase();
    matches!(
        text.as_str(),
        "lullaby"
            | "play a lullaby"
            | "play a lullaby please"
            | "a lullaby please"
            | "자장가"
            | "자장가 틀어줘"
            | "자장가 들려줘"
            | "자장가 쳐줘"
    )
}

impl World {
    pub(super) fn hear_music(&mut self, text: &str, korean: bool) -> Option<Heard> {
        let tune = lesson(text);
        let lullaby = lullaby_requested(text);
        let own = own_song_requested(text);
        if tune.is_none() && !lullaby && !own {
            return None;
        }
        self.listening = Listening::default();
        self.notice_viewer();
        self.record(EventKind::Heard(text.to_owned()));
        if own {
            if let Some(composition) = self.life.compositions.last().cloned() {
                if self.focus.is_none() && self.ghost.asleep < 0.3 && self.telling.is_none() {
                    self.hum(&composition.tune, korean);
                }
                self.queue_music_request(composition.tune);
            } else if self.ghost.asleep < 0.3 {
                self.say(if korean {
                    "아직 내 노래를 만들지 못했어."
                } else {
                    "I haven't made a song of my own yet."
                });
            }
        } else if let Some(tune) = tune {
            if let Some(composition) = self
                .life
                .compositions
                .iter()
                .find(|composition| {
                    composition
                        .tune
                        .notes
                        .iter()
                        .map(|note| note.pitch)
                        .eq(tune.notes.iter().map(|note| note.pitch))
                })
                .cloned()
            {
                let mut tune = tune;
                tune.name = composition.title.clone();
                self.record(EventKind::SangMySong(composition.title));
                if self.ghost.asleep < 0.3 {
                    self.say(if korean {
                        "내 노래다! 불러줬구나!"
                    } else {
                        "That's my song! You sang it!"
                    });
                }
                self.queue_music_request(tune);
                return Some(Heard {
                    tone: None,
                    understanding: Understanding::OwnSong,
                    named: false,
                    learned: Vec::new(),
                    stopped: false,
                    learned_name: None,
                    given_name: None,
                    korean,
                });
            }
            if !self
                .life
                .tunes
                .iter()
                .any(|known| known.notes == tune.notes)
            {
                self.life.tunes.push(tune.clone());
                if self.life.tunes.len() > 8 {
                    self.life.tunes.remove(0);
                }
                self.record(EventKind::LearnedTune(tune.name.clone()));
            }
            self.queue_music_request(tune);
        } else {
            self.ask_lullaby();
        }
        Some(Heard {
            tone: None,
            understanding: if own {
                Understanding::OwnSong
            } else {
                Understanding::Music { lullaby }
            },
            named: false,
            learned: Vec::new(),
            stopped: false,
            learned_name: None,
            given_name: None,
            korean,
        })
    }

    /// A lullaby lulls a drowsing ghost deeper, and one that tips over into
    /// sleep has fallen asleep to it.
    fn deepen_sleep(&mut self) {
        let before = self.ghost.asleep;
        self.ghost.asleep = (before + 0.15).min(1.0);
        if before < 0.9 && self.ghost.asleep >= 0.9 {
            self.record(EventKind::FellAsleep);
            self.record(EventKind::LullabySleep);
        }
    }

    /// A viewer's winding is immediate. Sleepers hear only the lullaby;
    /// focus permits it only when the viewer explicitly asks for a lullaby.
    pub fn wind_music_box(&mut self) {
        if self.focus.is_some() {
            return;
        }
        let lullaby = self.ghost.asleep >= 0.3 || self.bedtime();
        if self.ghost.asleep >= 0.3 {
            self.deepen_sleep();
        } else {
            self.begin(Activity::Music);
        }
        self.compose_on_arrival = false;
        let tune = if lullaby {
            built_in(0)
        } else {
            self.queued_tune.take().unwrap_or_else(|| built_in(1))
        };
        let lullaby = lullaby || tune.name == "little lullaby";
        self.start_music(tune, lullaby, 1);
    }

    pub(super) fn play_comfort_lullaby(&mut self) {
        self.begin(Activity::Music);
        self.compose_on_arrival = false;
        self.queued_tune = Some(built_in(0));
    }

    fn ask_lullaby(&mut self) {
        if self.ghost.asleep >= 0.3 {
            self.deepen_sleep();
        } else if self.focus.is_none() {
            self.begin(Activity::Music);
        }
        self.start_music(built_in(0), true, 2);
    }

    pub(super) fn prepare_music(&mut self) {
        self.compose_on_arrival = true;
        let own_weight = if self.life.bond >= 0.3 { 2 } else { 1 };
        let choices = 4 + self.life.tunes.len() + self.life.compositions.len() * own_weight;
        let index = (self.random_unit() * choices as f32) as usize;
        self.queued_tune = Some(if index < 4 {
            built_in(index)
        } else if index < 4 + self.life.tunes.len() {
            self.life.tunes[index - 4].clone()
        } else {
            self.life.compositions
                [(index.min(choices - 1) - 4 - self.life.tunes.len()) / own_weight]
                .tune
                .clone()
        });
    }

    fn start_music(&mut self, tune: Tune, lullaby: bool, repeats: usize) {
        self.music = Some(Playing::new(tune, self.time, lullaby, repeats));
        self.advance_music();
    }

    pub(super) fn advance_music(&mut self) {
        if self.pending_tune.is_some()
            && self.focus.is_none()
            && self.hunt.is_none()
            && self.ghost.asleep < 0.3
            && self.telling.is_none()
            && (self.ghost.plan.done
                || matches!(
                    self.ghost.plan.activity,
                    Activity::Float | Activity::Visit | Activity::Company
                ))
        {
            let tune = self.pending_tune.take();
            self.music = None;
            self.begin(Activity::Music);
            self.queued_tune = tune;
            self.compose_on_arrival = false;
        }
        self.music_notes
            .retain(|note| self.time - note.born < RisingNote::LIFETIME);
        let Some(playing) = &mut self.music else {
            return;
        };
        let elapsed = self.time - playing.started;
        while playing.emitted < playing.onsets.len() && elapsed >= playing.onsets[playing.emitted] {
            let index = playing.emitted;
            let note = &playing.tune.notes[index % playing.tune.notes.len()];
            let born = playing.started + playing.onsets[index];
            if self.time - born < RisingNote::LIFETIME {
                self.music_notes.push(RisingNote {
                    born,
                    pitch: note.pitch,
                    flagged: note.beats <= 1.0,
                    side: if index.is_multiple_of(2) { -1.0 } else { 1.0 },
                });
            }
            playing.emitted += 1;
        }
        if self.music_notes.len() > 16 {
            self.music_notes.drain(..self.music_notes.len() - 16);
        }
        if elapsed >= playing.duration() {
            let tune = playing.tune.name.clone();
            self.music = None;
            self.record(EventKind::PlayedMusic { tune });
        }
    }

    pub(super) fn carry_out_music(&mut self) -> Option<Vec3> {
        if self.ghost.plan.arrived.is_some() {
            if self.ghost.plan.count == 0 && self.ghost.plan.stayed(self.time) >= 1.2 {
                if self.music.is_none() {
                    if std::mem::take(&mut self.compose_on_arrival) && self.try_compose() {
                        self.ghost.plan.arrived = Some(self.time);
                        return Some(POSITION + Vec3::Y * 0.3);
                    }
                    let tune = self.queued_tune.take().unwrap_or_else(|| built_in(0));
                    let repeats = if self.random_unit() < 0.3 { 2 } else { 1 };
                    let lullaby = tune.name == "little lullaby";
                    self.start_music(tune, lullaby, repeats);
                }
                self.ghost.plan.count = 1;
            }
            if self.ghost.plan.count > 0 {
                if let Some(playing) = &self.music {
                    let phase = playing.beat(self.time) * TAU;
                    let dancing = self.life.temperament().playful > 0.55
                        && self.ghost.needs.energy > 0.45
                        && !playing.lullaby;
                    let offset = if dancing {
                        Vec3::new(
                            (phase / 4.0).cos() * 0.18,
                            phase.sin() * 0.08,
                            (phase / 4.0).sin() * 0.18,
                        )
                    } else {
                        Vec3::new((phase / 2.0).sin() * 0.09, phase.cos() * 0.055, 0.0)
                    };
                    self.ghost.plan.target = LISTEN + offset;
                } else {
                    self.ghost.plan.done = true;
                }
            }
        }
        Some(POSITION + Vec3::Y * 0.3)
    }

    pub(super) fn prepare_lullaby(&mut self) {
        if self.bedtime()
            && self.focus.is_none()
            && self.music.is_none()
            && self.ghost.asleep < 0.3
            && self.random_unit() < 0.35
        {
            self.queued_tune = Some(built_in(0));
            self.ghost.plan.target = LISTEN;
        }
    }

    pub(super) fn wind_before_sleep(&mut self) {
        if self.queued_tune.is_some() {
            if self.ghost.plan.arrived.is_some() && self.ghost.plan.stayed(self.time) >= 1.2 {
                let tune = self.queued_tune.take().unwrap();
                self.start_music(tune, true, 2);
                self.ghost.plan.arrived = None;
                self.ghost.plan.target = self.nook.bed(self.size());
            } else {
                self.ghost.look = Some(POSITION + Vec3::Y * 0.3);
            }
        }
    }

    pub fn music_words(&self) -> String {
        self.music.as_ref().map_or_else(
            || "a wind-up music box, quiet with its lid shut".to_owned(),
            |playing| format!("the music box is playing {}", playing.tune.name),
        )
    }

    pub fn learned_tune_words(&self) -> String {
        if self.life.tunes.is_empty() {
            "no tunes learned from the viewer yet".to_owned()
        } else {
            self.life
                .tunes
                .iter()
                .map(|tune| tune.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{Life, Mood, Nature, Random, Thing};

    #[test]
    fn compositions_have_related_phrases_valid_notes_and_a_tonic_ending() {
        for seed in 0..128 {
            for mood in Mood::ALL {
                let notes = compose(&mut Random::new(seed), mood, Nature::default());
                assert_eq!(
                    notes,
                    compose(&mut Random::new(seed), mood, Nature::default())
                );
                assert!((8..=16).contains(&notes.len()));
                assert_eq!(notes.last().unwrap().pitch, 0);
                assert_eq!(notes[0].pitch, notes[notes.len() / 2].pitch);
                assert_eq!(notes[1].pitch, notes[notes.len() / 2 + 1].pitch);
                assert!(
                    notes
                        .iter()
                        .all(|note| (0..=7).contains(&note.pitch)
                            && (0.5..=4.0).contains(&note.beats))
                );
                let ordinary = notes
                    .windows(2)
                    .filter(|notes| (notes[0].pitch - notes[1].pitch).abs() <= 1)
                    .count();
                assert!(ordinary > 0);
                let playing = Playing::new(Tune::named(notes), 0.0, false, 1);
                assert!(playing.duration().is_finite() && playing.duration() > 0.0);
            }
        }
    }

    #[test]
    fn cheerful_compositions_average_higher_and_quicker_than_sleepy_ones() {
        let average = |mood| {
            let notes: Vec<Note> = (0..256)
                .flat_map(|seed| compose(&mut Random::new(seed), mood, Nature::default()))
                .collect();
            (
                notes.iter().map(|note| f32::from(note.pitch)).sum::<f32>() / notes.len() as f32,
                notes.iter().map(|note| note.beats).sum::<f32>() / notes.len() as f32,
            )
        };
        let cheerful = average(Mood::Cheerful);
        let sleepy = average(Mood::Sleepy);
        assert!(cheerful.0 > sleepy.0 + 1.0, "{cheerful:?} {sleepy:?}");
        assert!(cheerful.1 < sleepy.1);
    }

    fn my_song() -> Composition {
        let mut tune = lesson("mi sol la sol mi re do").unwrap();
        for note in &mut tune.notes {
            note.beats = 2.0;
        }
        Composition {
            tune,
            title: "Rain Song".to_owned(),
            korean_title: "비의 노래".to_owned(),
            day: "2026-06-21".to_owned(),
        }
    }

    #[test]
    fn titles_follow_present_memories_and_stay_unique_in_both_languages() {
        let mut world = World::new(7, 12.0);
        assert_eq!(composition_title(&world).0, "Little Song");
        world.hour = 8.0;
        assert_eq!(composition_title(&world).0, "Morning Song");
        world.life.bird_trust = 0.7;
        assert_eq!(composition_title(&world).0, "Bird Song");
        world.hour = 23.0;
        assert_eq!(composition_title(&world).0, "Moon Song");
        world.life.friend = Some(crate::world::Companion {
            name: "Pell".to_owned(),
            visits: 1,
            ..Default::default()
        });
        world.life.last_friend_visit = Some(world.date.to_string());
        assert_eq!(composition_title(&world).0, "Pell's Waltz");
        world.weather.sky = crate::world::Sky::Snow;
        assert_eq!(composition_title(&world).0, "Snow Song");
        world.weather.sky = crate::world::Sky::Rain;
        world.life.compositions.push(my_song());
        // The rain's first title is taken, so the friend's turn comes before
        // the rain's second title.
        assert_eq!(
            composition_title(&world),
            ("Pell's Waltz".to_owned(), "Pell의 왈츠".to_owned())
        );
        world.life.viewer_name = Some("Mina".to_owned());
        world.life.bond = 0.3;
        assert_eq!(
            composition_title(&world),
            (
                "Song for Mina".to_owned(),
                "Mina에게 바치는 노래".to_owned()
            )
        );
        world.life.viewer_name = None;
        world.weather.sky = crate::world::Sky::Clear;
        world.date = world.date.checked_add(jiff::Span::new().days(2)).unwrap();
        assert_eq!(composition_title(&world).0, "Moon Song");
    }

    #[test]
    fn a_beloved_viewer_shares_the_titles_with_the_moment() {
        let mut world = World::new(7, 23.0);
        world.life.viewer_name = Some("Mina".to_owned());
        world.life.bond = 0.8;
        let mut seen = Vec::new();
        for _ in 0..9 {
            let (title, korean_title) = composition_title(&world);
            assert!(!seen.contains(&title), "{title} repeats");
            seen.push(title.clone());
            world.life.compositions.push(Composition {
                tune: my_song().tune,
                title,
                korean_title,
                day: "2026-06-21".to_owned(),
            });
        }
        let named = seen.iter().filter(|title| title.contains("Mina")).count();
        assert_eq!(&seen[..2], ["Song for Mina", "Moon Song"]);
        assert!(named <= 4, "{seen:?}");
        assert!(
            !seen.iter().any(|title| title.contains("Again")),
            "{seen:?}"
        );
    }

    #[test]
    fn singing_its_notes_back_ignores_holds_and_marks_the_first_time_once() {
        for (text, expected) in [
            ("mi sol la sol mi re do", "That's my song! You sang it!"),
            ("미 솔 라 솔 미 레 도", "내 노래다! 불러줬구나!"),
        ] {
            let mut world = World::new(9, 12.0);
            world.life.compositions.push(my_song());
            let heard = world.hear(text);
            assert_eq!(heard.understanding, Understanding::OwnSong);
            assert!(world.life.tunes.is_empty());
            assert_eq!(world.life.total.songs_sung_back, 1);
            assert!(world.life.bond > 0.0);
            assert_eq!(world.ghost.speech.as_ref().unwrap().text, expected);
            assert_eq!(world.queued_tune.as_ref().unwrap().name, "Rain Song");
            world.hear(text);
            assert_eq!(world.life.total.songs_sung_back, 2);
            assert_eq!(
                world
                    .life
                    .milestones
                    .iter()
                    .filter(|entry| entry.text == "You sang my song back to me.")
                    .count(),
                1
            );
        }
    }

    #[test]
    fn own_song_requests_hum_the_newest_or_explain_that_none_exists() {
        for text in [
            "sing me your song",
            "play your song",
            "do you have your own song?",
            "네 노래 불러줘",
            "네가 만든 노래",
        ] {
            let mut world = World::new(9, 12.0);
            assert_eq!(world.hear(text).understanding, Understanding::OwnSong);
            assert!(
                world.ghost.speech.as_ref().unwrap().text.contains("yet")
                    || world.ghost.speech.as_ref().unwrap().text.contains("아직")
            );
            world.life.compositions.push(my_song());
            world.hear(text);
            assert!(world.ghost.speech.as_ref().unwrap().text.starts_with('♪'));
            assert_eq!(
                world.queued_tune.as_ref().unwrap().notes,
                my_song().tune.notes
            );
            assert!(world.life.vocabulary.known().is_empty());
            assert!(!world.compose_on_arrival);
            world.start_focus(25);
            world.hear(text);
            assert!(world.pending_tune.is_some());
            assert!(world.music.is_none());
        }
    }

    #[test]
    fn composition_checks_do_not_draw_randomness_until_eligible() {
        let mut world = World::new(2, 12.0);
        let mut expected = world.random.clone();
        assert!(!world.try_compose());
        assert_eq!(world.random.next_u64(), expected.next_u64());
        super::super::creation::tests::ready_to_compose(&mut world);
        assert!(world.can_compose());
        world.life.composition_day = Some(world.date.to_string());
        assert!(!world.can_compose());
        world.date = world.date.checked_add(jiff::Span::new().days(2)).unwrap();
        assert!(!world.can_compose());
        world.date = world.date.checked_add(jiff::Span::new().days(1)).unwrap();
        assert!(world.can_compose());
        world.birthday = Some(world.date);
        assert!(!world.can_compose());
        world.birthday = None;
        world.start_focus(25);
        assert!(!world.can_compose());
    }

    #[test]
    fn an_eligible_month_keeps_at_most_eight_songs_and_spaces_creation_by_local_days() {
        let mut world = World::new(42, 12.0);
        super::super::creation::tests::ready_to_compose(&mut world);
        let mut days = Vec::new();
        for day in 0..30 {
            for _ in 0..10 {
                if world.try_compose() {
                    days.push(day);
                }
            }
            world.date = world.date.checked_add(jiff::Span::new().days(1)).unwrap();
        }
        assert!(days.len() >= 8 && days.len() <= 10, "{days:?}");
        assert!(days.windows(2).all(|days| days[1] - days[0] >= 3));
        assert_eq!(world.life.compositions.len(), 8);
        assert_eq!(world.life.total.compositions as usize, days.len());
    }

    #[test]
    fn an_eligible_music_sitting_hums_before_winding_and_completes_playback() {
        let mut world = World::new(42, 12.0);
        super::super::creation::tests::ready_to_compose(&mut world);
        let seed = (0..100)
            .find(|seed| Random::new(*seed).unit() < 0.1)
            .unwrap();
        world.begin(Activity::Music);
        world.random = Random::new(seed);
        world.ghost.plan.arrived = Some(world.time - 2.0);
        world.carry_out_music();
        assert_eq!(world.life.total.compositions, 1);
        assert!(world.music.is_none());
        assert!(world.ghost.speech.as_ref().unwrap().text.starts_with('♪'));
        world.time += 2.0;
        world.carry_out_music();
        assert_eq!(
            world.music.as_ref().unwrap().tune.name,
            world.life.compositions[0].title
        );
        world.time += 100.0;
        world.advance_music();
        assert_eq!(
            world.life.total.tunes_played,
            super::super::creation::COMPOSES_AFTER_TUNES + 1
        );
        assert!(
            world
                .life
                .today
                .diary()
                .to_lowercase()
                .contains("made up a tune called")
        );
    }

    #[test]
    fn lessons_read_english_aliases_and_korean_with_or_without_spaces() {
        let english = lesson("DO-re MI fa so la TI").unwrap();
        assert_eq!(english, lesson("do re mi fa sol la si").unwrap());
        assert_eq!(
            english,
            lesson("do - re - mi - fa - sol - la - si").unwrap()
        );
        assert_eq!(english, lesson("도레미파솔라시").unwrap());
        assert_eq!(english, lesson("도 레 미 파 솔 라 시").unwrap());
        assert_eq!(
            english
                .notes
                .iter()
                .map(|note| note.pitch)
                .collect::<Vec<_>>(),
            (0..7).collect::<Vec<_>>()
        );
        assert_eq!(lesson("do re mi").unwrap().name, "do-re-mi");
    }

    #[test]
    fn trailing_dashes_and_tildes_hold_notes_without_adding_syllables() {
        let tune = lesson("do- re~ mi~~").unwrap();
        assert_eq!(
            tune.notes.iter().map(|note| note.beats).collect::<Vec<_>>(),
            vec![2.0, 2.0, 3.0]
        );
        assert_eq!(tune, lesson("도- 레~ 미~~").unwrap());
        assert_eq!(lesson("do-re-mi").unwrap().notes[0].beats, 1.0);
        assert_eq!(lesson("do~~~~~~ re mi").unwrap().notes[0].beats, 4.0);
        assert_eq!(lesson("do do- mi").unwrap().notes.len(), 3);
    }

    #[test]
    fn lessons_reject_ordinary_words_punctuation_and_wrong_lengths() {
        for text in [
            "",
            "do",
            "do re",
            "do re me",
            "do you like music",
            "sing do re mi",
            "도 레 미 좋아",
            "do, re, mi",
            "doremi",
            "-do re mi",
            "do re mi!",
            "do re mi -",
        ] {
            assert!(lesson(text).is_none(), "{text}");
        }
        assert!(lesson(&["do"; 24].join(" ")).is_some());
        assert!(lesson(&["do"; 25].join(" ")).is_none());
        assert!(lesson(&"도".repeat(25)).is_none());
    }

    #[test]
    fn playback_releases_each_note_on_its_beat_and_slows_only_near_the_end() {
        let mut world = World::new(1, 12.0);
        world.start_music(lesson("do re mi fa sol la ti do").unwrap(), false, 1);
        let playing = world.music.as_ref().unwrap().clone();
        let beat_seconds = 60.0 / f64::from(playing.tempo);
        assert_eq!(world.music_notes.len(), 1);
        world.time = beat_seconds - 0.001;
        world.advance_music();
        assert_eq!(world.music_notes.len(), 1);
        world.time = beat_seconds;
        world.advance_music();
        assert_eq!(world.music_notes.len(), 2);
        assert!((world.music_notes[1].born - beat_seconds).abs() < 1e-9);
        for beat in [0.0, 1.0, 4.0, 7.5, 8.0] {
            assert!(
                (f64::from(playing.beat(playing.started + playing.seconds_at(beat))) - beat).abs()
                    < 1e-5
            );
        }
        assert!((playing.onsets[2] - playing.onsets[1] - beat_seconds).abs() < 1e-9);
        assert!(playing.onsets[7] - playing.onsets[6] > beat_seconds);
        assert!(playing.duration() > 8.0 * beat_seconds);
        world.time = playing.duration();
        world.advance_music();
        world.advance_music();
        assert!(world.music.is_none());
        assert_eq!(world.life.total.tunes_played, 1);
        assert_eq!(
            world
                .events
                .iter()
                .filter(|event| matches!(event.kind, EventKind::PlayedMusic { .. }))
                .count(),
            1
        );
        assert!(world.life.today.words().contains("music box"));
    }

    #[test]
    fn rising_notes_are_bounded_rise_and_expire_after_playback() {
        let mut world = World::new(2, 12.0);
        let tune = lesson(&["do"; 24].join(" ")).unwrap();
        world.start_music(tune, false, 2);
        let first = world.music_notes[0];
        assert!(first.position(2.0).y > first.position(0.0).y);
        assert_eq!(first.fade(RisingNote::LIFETIME), 0.0);
        for _ in 0..400 {
            world.time += 0.1;
            world.advance_music();
            assert!(world.music_notes.len() <= 16);
            assert!(
                world
                    .music_notes
                    .iter()
                    .all(|note| note.position(world.time).is_finite()
                        && world.time - note.born < RisingNote::LIFETIME)
            );
        }
        assert!(world.music.is_none());
        assert!(world.music_notes.is_empty());
        assert_eq!(world.life.total.tunes_played, 1);
    }

    #[test]
    fn teaching_replays_and_keeps_one_copy_without_learning_any_words() {
        let mut world = World::new(3, 12.0);
        let heard = world.hear("do re mi");
        assert_eq!(heard.understanding, Understanding::Music { lullaby: false });
        assert!(heard.learned.is_empty());
        assert!(world.life.vocabulary.known().is_empty());
        assert_eq!(world.ghost.plan.activity, Activity::Music);
        world.ghost.position = LISTEN;
        world.step(1.5);
        assert_eq!(
            world.music.as_ref().unwrap().tune,
            lesson("do re mi").unwrap()
        );
        world.step(8.0);
        assert!(world.needs_decision());
        assert_eq!(world.life.total.tunes_played, 1);
        world.hear("도레미");
        assert_eq!(world.life.tunes.len(), 1);
        assert_eq!(world.life.total.tunes_learned, 1);
        assert_eq!(
            world
                .life
                .milestones
                .iter()
                .filter(|entry| entry.text == "You taught me my first tune.")
                .count(),
            1
        );
        assert!(world.listening.words.is_empty());
    }

    #[test]
    fn a_new_lesson_replaces_playback_and_a_story_defers_it_until_finished() {
        let mut world = World::new(15, 12.0);
        world.start_music(built_in(1), false, 2);
        world.hear("mi re do");
        assert!(world.music.is_none());
        world.ghost.position = LISTEN;
        world.step(1.5);
        assert_eq!(
            world.music.as_ref().unwrap().tune,
            lesson("mi re do").unwrap()
        );
        world.music = None;
        world.hear("tell me a story");
        world.hear("do mi sol");
        assert!(world.telling.is_some());
        assert_eq!(world.ghost.plan.activity, Activity::Visit);
        world.telling = None;
        world.step(0.1);
        assert_eq!(world.ghost.plan.activity, Activity::Music);
        assert_eq!(world.queued_tune, lesson("do mi sol"));
    }

    #[test]
    fn music_and_its_motion_repeat_for_the_same_seed_and_inputs() {
        let mut first = World::new(16, 20.0);
        let mut second = World::new(16, 20.0);
        for _ in 0..3 {
            first.begin(Activity::Music);
            second.begin(Activity::Music);
            for _ in 0..400 {
                first.step(0.1);
                second.step(0.1);
                assert_eq!(first.music, second.music);
                assert_eq!(first.music_notes, second.music_notes);
                assert_eq!(first.ghost.position, second.ghost.position);
            }
        }
        assert_eq!(first.life, second.life);
    }

    #[test]
    fn tunes_round_trip_default_in_old_lives_and_sanitize_damaged_notes() {
        let mut world = World::new(4, 12.0);
        world.hear("do re mi");
        let encoded = serde_json::to_string(&world.life).unwrap();
        let restored: Life = serde_json::from_str(&encoded).unwrap();
        assert_eq!(restored.sanitized().tunes, world.life.tunes);
        let mut old = serde_json::to_value(&world.life).unwrap();
        old.as_object_mut().unwrap().remove("tunes");
        assert!(
            serde_json::from_value::<Life>(old)
                .unwrap()
                .tunes
                .is_empty()
        );
        world.life.tunes.push(Tune {
            name: "broken\n123".to_owned(),
            notes: vec![
                Note {
                    pitch: -20,
                    beats: f32::NAN,
                },
                Note {
                    pitch: 99,
                    beats: -5.0,
                },
                Note {
                    pitch: 2,
                    beats: 99.0,
                },
            ],
        });
        world.life.tunes.push(Tune {
            name: "empty".to_owned(),
            notes: Vec::new(),
        });
        let sanitized = world.life.sanitized();
        assert_eq!(sanitized.tunes.len(), 2);
        let notes = &sanitized.tunes[1].notes;
        assert_eq!(
            notes[0],
            Note {
                pitch: 0,
                beats: 1.0
            }
        );
        assert_eq!(
            notes[1],
            Note {
                pitch: 7,
                beats: 0.5
            }
        );
        assert_eq!(notes[2].beats, 4.0);
        assert!(
            !sanitized.tunes[1]
                .name
                .chars()
                .any(|character| character.is_control() || character.is_ascii_digit())
        );
    }

    #[test]
    fn only_the_newest_eight_tunes_are_kept_and_instinct_can_choose_them() {
        let mut world = World::new(5, 12.0);
        for index in 0..10 {
            world.hear(&format!(
                "do re {} {}",
                SOLFEGE[index / 7],
                SOLFEGE[index % 7]
            ));
        }
        assert_eq!(world.life.tunes.len(), 8);
        assert_eq!(
            world.life.tunes.last().unwrap(),
            &lesson("do re re mi").unwrap()
        );
        let mut found = false;
        for _ in 0..50 {
            world.begin(Activity::Music);
            found |= world
                .life
                .tunes
                .contains(world.queued_tune.as_ref().unwrap());
        }
        assert!(found);
        world.life.tunes.extend((0..10).map(|index| Tune {
            name: "saved tune".to_owned(),
            notes: vec![Note {
                pitch: index % 7,
                beats: 1.0 + f32::from(index / 7),
            }],
        }));
        assert_eq!(world.life.sanitized().tunes.len(), 8);
    }

    #[test]
    fn music_sways_and_playful_ghosts_follow_small_loops_until_the_tune_ends() {
        let mut paths = Vec::new();
        for playful in [0.0, 1.0] {
            let mut world = World::new(6, 12.0);
            world.life.nature = Some(Nature {
                playful,
                ..Nature::default()
            });
            world.ghost.needs.energy = 0.8;
            world.begin(Activity::Music);
            world.ghost.position = LISTEN;
            world.ghost.plan.arrived = Some(0.0);
            world.ghost.plan.count = 1;
            world.start_music(built_in(1), false, 1);
            let mut targets = Vec::new();
            for _ in 0..40 {
                world.time += 0.05;
                world.carry_out_music();
                targets.push(world.ghost.plan.target);
            }
            assert!(targets.windows(2).any(|pair| pair[0].y != pair[1].y));
            paths.push(targets);
            world.time += 50.0;
            world.advance_music();
            world.carry_out_music();
            assert!(world.ghost.plan.done);
        }
        assert!(paths[0].iter().all(|target| target.z == LISTEN.z));
        assert!(
            paths[1]
                .iter()
                .any(|target| (target.z - LISTEN.z).abs() > 0.1)
        );
    }

    #[test]
    fn bedtime_sometimes_winds_a_lullaby_and_sleep_comes_before_its_end() {
        let mut slept = false;
        let mut skipped = false;
        for seed in 0..20 {
            let mut world = World::new(seed, 23.0);
            world.begin(Activity::Rest);
            if world.queued_tune.is_none() {
                skipped = true;
                continue;
            }
            world.ghost.position = LISTEN;
            world.step(1.5);
            assert!(world.music.as_ref().unwrap().lullaby);
            assert_eq!(world.ghost.plan.target, world.nook.bed(world.size()));
            world.step(28.0);
            assert!(
                world.ghost.asleep >= 0.9,
                "seed {seed}: {}",
                world.ghost.asleep
            );
            assert!(world.music.is_some());
            assert_eq!(world.life.total.lullaby_sleeps, 1);
            assert!(
                world
                    .life
                    .milestones
                    .iter()
                    .any(|entry| entry.text == "I fell asleep to my first lullaby.")
            );
            world.step(20.0);
            assert_eq!(world.life.total.lullaby_sleeps, 1);
            assert!(world.ghost.asleep >= 0.9);
            slept = true;
            break;
        }
        assert!(slept && skipped);
    }

    #[test]
    fn a_lullaby_lengthens_the_drowsy_settle_and_a_click_deepens_sleep() {
        let mut world = World::new(7, 23.0);
        world.tuck_in();
        world.ghost.asleep = 0.5;
        let plan = world.ghost.plan.clone();
        world.wind_music_box();
        assert_eq!(world.ghost.asleep, 0.65);
        assert_eq!(world.ghost.plan, plan);
        assert!(world.music.as_ref().unwrap().lullaby);
        world.step(0.5);
        assert!((world.ghost.asleep - 0.75).abs() < 0.001);
        world.step(8.0);
        assert_eq!(world.ghost.asleep, 1.0);
        assert_eq!(world.life.total.lullaby_sleeps, 1);
    }

    #[test]
    fn a_lullaby_that_tips_a_drowsing_ghost_into_sleep_marks_it_asleep() {
        let mut world = World::new(7, 23.0);
        world.tuck_in();
        world.ghost.asleep = 0.8;
        let before = world.life.total.lullaby_sleeps;
        world.wind_music_box();
        assert!(world.ghost.asleep >= 0.9);
        assert_eq!(world.life.total.lullaby_sleeps, before + 1);
        assert!(
            world
                .events
                .iter()
                .any(|event| event.kind == EventKind::FellAsleep)
        );
        // Deepening a sleep that already took hold counts nothing new.
        world.wind_music_box();
        assert_eq!(world.life.total.lullaby_sleeps, before + 1);
    }

    #[test]
    fn focus_suppresses_music_except_for_explicit_lullabies_and_defers_lessons() {
        let mut world = World::new(8, 12.0);
        world.wind_music_box();
        world.start_focus(5);
        assert!(world.music.is_none());
        assert!(!world.options().contains(&Activity::Music));
        let company = world.ghost.plan.clone();
        world.begin(Activity::Music);
        world.wind_music_box();
        assert_eq!(world.ghost.plan, company);
        assert!(world.music.is_none());
        world.hear("do re mi");
        assert_eq!(world.life.tunes.len(), 1);
        assert!(world.music.is_none());
        world.hear("play a lullaby");
        assert!(world.music.as_ref().unwrap().lullaby);
        assert_eq!(world.ghost.plan, company);
        world.music = None;
        world.end_focus();
        world.step(0.1);
        assert_eq!(world.ghost.plan.activity, Activity::Music);
        assert_eq!(world.queued_tune, lesson("do re mi"));
    }

    #[test]
    fn music_box_is_furniture_seen_and_attended_and_stops_a_rolling_ball() {
        let mut world = World::new(9, 12.0);
        assert!(world.obstacles().contains(&(POSITION, RADIUS)));
        world.ghost.position = LISTEN;
        world.ghost.heading =
            (world.position_of(Thing::MusicBox).unwrap() - LISTEN - Vec3::Y * 0.1).normalize();
        assert!(
            world
                .sight()
                .in_view
                .iter()
                .any(|seen| seen.thing == Thing::MusicBox && seen.hidden_behind.is_none())
        );
        world.attend("music box", 5.0);
        assert_eq!(world.ghost.attention.unwrap().0, Thing::MusicBox);
        world.ball = super::super::Ball::at(POSITION.x - 0.6, POSITION.z);
        world.ball.velocity = Vec3::X;
        world.step(1.0);
        assert!(world.ball.position.x < POSITION.x - RADIUS);
        assert!(hit_box(POSITION + Vec3::new(0.0, 0.1, 1.0), -Vec3::Z).is_some());
        assert!(hit_box(POSITION + Vec3::Y, Vec3::Z).is_none());
    }
}
