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
        if tune.is_none() && !lullaby {
            return None;
        }
        self.listening = Listening::default();
        self.notice_viewer();
        self.record(EventKind::Heard(text.to_owned()));
        if let Some(tune) = tune {
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
            if self.focus.is_none() && self.ghost.asleep < 0.3 && self.telling.is_none() {
                self.music = None;
                self.begin(Activity::Music);
                self.queued_tune = Some(tune);
                self.pending_tune = None;
            } else {
                self.pending_tune = Some(tune);
            }
        } else {
            self.ask_lullaby();
        }
        Some(Heard {
            tone: None,
            understanding: Understanding::Music { lullaby },
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
        let tune = if lullaby {
            built_in(0)
        } else {
            self.queued_tune.take().unwrap_or_else(|| built_in(1))
        };
        let lullaby = lullaby || tune.name == "little lullaby";
        self.start_music(tune, lullaby, 1);
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
        let choices = 4 + self.life.tunes.len();
        let index = (self.random_unit() * choices as f32) as usize;
        self.queued_tune = Some(if index < 4 {
            built_in(index)
        } else {
            self.life.tunes[index.min(choices - 1) - 4].clone()
        });
    }

    fn start_music(&mut self, tune: Tune, lullaby: bool, repeats: usize) {
        self.music = Some(Playing::new(tune, self.time, lullaby, repeats));
        self.advance_music();
    }

    pub(super) fn advance_music(&mut self) {
        if self.pending_tune.is_some()
            && self.focus.is_none()
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
                    let dancing = self.life.nature.unwrap_or_default().playful > 0.55
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
    use crate::world::{Life, Nature, Thing};

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
