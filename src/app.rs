use std::io;

use glam::{Vec2, Vec3};

use crate::album::Album;
use crate::canvas::{Canvas, Style};
use crate::color::ColorMode;
use crate::face;
use crate::frame::{CELL_ASPECT, Grid};
use crate::mind::{self, Journal, Mind};
use crate::portrait::{self, Handling};
use crate::postcard::{self, Postcard};
use crate::render::creature::{Figure, Model};
use crate::render::ghost::{Aura, Body, Pose, friend_color};
use crate::render::sky::Outside;
use crate::render::{self, Camera, FRIEND_SCALE, Scene, View};
use crate::settings::{Settings, cache_name, effort_name};
use crate::store::{Config, Paths, Save};
use crate::terminal::{Input, Key, Mouse, MouseAction};
use crate::ui;
use crate::world::{Ball, Mood, Nature, Needs, ROOM_MAX, ROOM_MIN, World};

/// Seconds after the ghost first answers in its own words that the viewer
/// hears a Claude key would let it talk.
const HINT_AFTER: f64 = 6.0;
// Radians per arrow key press, and per cell dragged.
const TURN: f32 = 0.12;
const DRAG_TURN: f32 = 0.03;
/// Seconds between saves while the app runs.
const SAVE_EVERY: f64 = 60.0;
/// How long the ghost holds its smile before a posed picture is taken.
const POSING: f32 = 2.0;

/// What the main loop should do after an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    Continue,
    /// The style or color mode changed, so the next draw must be complete.
    Restyle,
    Quit,
}

/// What the keyboard is doing: watching the ghost, or typing into a line or
/// a panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Watching,
    /// Typing a message to the ghost.
    Talking,
    /// Changing the keys, models, effort, and caching.
    Settings,
    Help,
    /// Reading what the ghost remembers and what was said.
    Journal,
    /// The ghost's life at a glance: its bond, habits, and days.
    Life,
    /// Leafing through the pictures the ghost kept.
    Album,
    Letter,
    Writing,
    /// Choosing a newcomer to come and live in the cottage.
    Adopt,
}

/// The adoption panel: which kind is picked, and its name once chosen.
#[derive(Clone, Debug, Default)]
pub struct Adoption {
    pub index: usize,
    /// The name being typed, once a kind is chosen.
    pub naming: Option<Line>,
    /// Whether it is the stranger at the glass being named, not a newcomer.
    pub stranger: bool,
    /// How many cards fit across, as last drawn, for moving up and down.
    pub across: usize,
}

/// The most characters a line holds; more is cut off as it is typed or
/// pasted.
const LINE_LIMIT: usize = 2000;

/// A line of text being edited, with a cursor between characters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Line {
    text: String,
    /// The cursor's position, in characters.
    cursor: usize,
}

impl Line {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Inserts `text` at the cursor, with line breaks and other control
    /// characters turned into spaces, up to `LINE_LIMIT` characters.
    pub fn insert(&mut self, text: &str) {
        let index = self.byte_index();
        let room = LINE_LIMIT.saturating_sub(self.text.chars().count());
        let clean: String = text
            .chars()
            .take(room)
            .map(|character| {
                if character.is_control() {
                    ' '
                } else {
                    character
                }
            })
            .collect();
        self.text.insert_str(index, &clean);
        self.cursor += clean.chars().count();
    }

    /// Applies an editing key, returning whether it was one.
    pub fn edit(&mut self, key: &Key) -> bool {
        let length = self.text.chars().count();
        match key {
            Key::Character(character) => self.insert(&character.to_string()),
            Key::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                let index = self.byte_index();
                self.text.remove(index);
            }
            Key::Delete if self.cursor < length => {
                let index = self.byte_index();
                self.text.remove(index);
            }
            Key::Left => self.cursor = self.cursor.saturating_sub(1),
            Key::Right => self.cursor = (self.cursor + 1).min(length),
            Key::Home | Key::Control('a') => self.cursor = 0,
            Key::End | Key::Control('e') => self.cursor = length,
            Key::Control('u') => *self = Self::default(),
            Key::Backspace | Key::Delete => {}
            _ => return false,
        }
        true
    }

    /// The text, trimmed, leaving the line empty.
    pub fn take(&mut self) -> String {
        let text = self.text.trim().to_owned();
        *self = Self::default();
        text
    }

    fn byte_index(&self) -> usize {
        self.text
            .char_indices()
            .nth(self.cursor)
            .map_or(self.text.len(), |(index, _)| index)
    }
}

/// A line holding `text`, with the cursor at its end.
impl From<&str> for Line {
    fn from(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            cursor: text.chars().count(),
        }
    }
}

/// The app: the world and the ghost's mind, how the viewer looks at them,
/// and what the keys and mouse are doing.
pub struct App {
    pub world: World,
    /// The mind of the creature the viewer is with.
    pub mind: Mind,
    /// The minds of its housemates.
    pub minds: Vec<(u32, Mind)>,
    /// When each housemate came to live here, in Unix seconds.
    borns: Vec<(u32, i64)>,
    /// When the viewer last turned to another creature, for showing whom.
    pub selected_at: f64,
    pub camera: Camera,
    pub style: Style,
    pub color: ColorMode,
    pub paused: bool,
    pub mode: Mode,
    /// Lines above the journal's bottom; zero follows its newest lines.
    pub journal_scroll: usize,
    /// Lines scrolled past at the top of the life panel.
    pub life_scroll: usize,
    pub talk: Line,
    /// The settings panel's rows while it is open.
    pub settings: Settings,
    pub config: Config,
    paths: Option<Paths>,
    /// Unix seconds when the ghost first appeared.
    born: i64,
    /// Whether the world's hour follows the clock.
    live_clock: bool,
    /// A short message for the viewer, and when it was posted.
    notice: Option<(String, f64)>,
    /// The size in cells of the area the scene is drawn in, and the view
    /// it was drawn with, for placing the mouse.
    area: (usize, usize),
    /// The last screen size, so scrolling uses the same bounds as drawing.
    screen_size: (usize, usize),
    view: Option<View>,
    /// Where the pointer is on screen, in scene space, and when it moved.
    pointer: Option<(Vec2, f64)>,
    /// Where the left button went down, and whether it has dragged since.
    press: Option<(usize, usize)>,
    dragged: bool,
    /// While a press that began on the ghost strokes it instead of turning
    /// the view: the last point of the stroke, in scene space.
    stroke: Option<Vec2>,
    /// Which way the latest stroke went across the screen, from -1 to 1,
    /// so the ghost leans into the hand.
    stroke_side: f32,
    /// The aura's color, eased toward the color of the ghost's mood.
    aura: Vec3,
    last_save: f64,
    /// Whether the viewer has been told, this run, that a Claude key lets
    /// the ghost talk, and when to tell them.
    hinted_claude: bool,
    hint_claude_at: Option<f64>,
    /// The last watching frame's scene and words, before its status bar.
    postcard: Option<Postcard>,
    /// Seconds until the picture the ghost is posing for is taken.
    posing: Option<f32>,
    /// Whether the terminal has the focus, as far as it reports.
    focused: bool,
    pub drawing_mode: bool,
    drawing_right: bool,
    pub album: Album,
    pub letter_index: usize,
    pub draft: crate::draft::Draft,
    pub sent_letters: bool,
    /// Lines scrolled past in an open letter taller than the screen.
    pub letter_scroll: usize,
    letter_read: bool,
    letter_reply_at: Option<f64>,
    /// Each housemate's glow, eased toward the color of its mood.
    glows: Vec<(u32, Vec3)>,
    /// The creatures as the last frame showed them, for finding them under
    /// the pointer.
    shown: Vec<Figure>,
    pub adoption: Adoption,
    /// The creature under the pointer, if any.
    pub hover: Option<u32>,
    /// Whether the camera stays close on the creature the viewer is with.
    pub follow: bool,
    /// When the stranger the viewer was last told about came.
    announced_stranger: Option<f64>,
    /// The symbols rising from each creature's head this frame.
    puffs: Vec<(u32, Vec<crate::face::emote::Puff>)>,
}

impl App {
    pub fn new(
        world: World,
        mut mind: Mind,
        config: Config,
        style: Style,
        color: ColorMode,
    ) -> Self {
        mind.write_letter(&world);
        let aura = mood_color(world.ghost.mood);
        let host = world.host();
        let others: Vec<u32> = world
            .roster()
            .into_iter()
            .filter(|id| *id != host)
            .collect();
        let minds = others
            .iter()
            .map(|id| (*id, mind.sibling(Journal::default())))
            .collect();
        let now = mind::now();
        let borns = others.iter().map(|id| (*id, now)).collect();
        Self {
            world,
            mind,
            minds,
            borns,
            selected_at: f64::NEG_INFINITY,
            camera: Camera::default(),
            style,
            color,
            paused: false,
            mode: Mode::Watching,
            journal_scroll: 0,
            life_scroll: 0,
            talk: Line::default(),
            settings: Settings::from(&config),
            config,
            paths: None,
            born: mind::now(),
            live_clock: false,
            notice: None,
            area: (0, 0),
            screen_size: (0, 0),
            view: None,
            pointer: None,
            press: None,
            dragged: false,
            stroke: None,
            stroke_side: 0.0,
            aura,
            last_save: 0.0,
            hinted_claude: false,
            hint_claude_at: None,
            postcard: None,
            posing: None,
            focused: true,
            drawing_mode: false,
            drawing_right: false,
            album: Album::default(),
            letter_index: 0,
            draft: crate::draft::Draft::default(),
            sent_letters: false,
            letter_scroll: 0,
            letter_read: false,
            letter_reply_at: None,
            glows: Vec::new(),
            shown: Vec::new(),
            adoption: Adoption::default(),
            hover: None,
            follow: false,
            announced_stranger: None,
            puffs: Vec::new(),
        }
    }

    /// Keeps the ghost's life in `paths` and follows the clock: a living
    /// ghost rather than a picture of one.
    pub fn live(mut self, paths: Option<Paths>, born: i64) -> Self {
        self.paths = paths;
        if let Some(paths) = &self.paths {
            let milestones = self
                .world
                .life
                .milestones
                .iter()
                .map(|entry| entry.text.clone())
                .collect();
            self.album = Album::load_remembering(paths, mind::now(), milestones);
        }
        self.born = born;
        self.world.birthday = jiff::Timestamp::from_second(born)
            .ok()
            .map(|timestamp| timestamp.to_zoned(jiff::tz::TimeZone::system()).date());
        self.live_clock = true;
        (self.world.date, self.world.hour) = local_time();
        self.world.tuck_in();
        for id in self.world.roster() {
            if id != self.world.host() {
                self.world.with_creature(id, World::tuck_in);
            }
        }
        self
    }

    /// Moves the world and the mind forward by `seconds`.
    pub fn advance(&mut self, seconds: f32) {
        if self.paths.is_some() {
            self.album.observe(&self.world);
            self.album.elapse(seconds);
        }
        // The picture itself is taken by the next watching frame; a portrait
        // nobody returns to watch is let go.
        if let Some(left) = self.posing.as_mut() {
            *left -= seconds;
            if *left <= -POSING * 5.0 {
                self.posing = None;
            }
        }
        if self.paused {
            return;
        }
        if self.live_clock {
            (self.world.date, self.world.hour) = local_time();
        }
        self.world.viewer.typing = self.mode == Mode::Talking && !self.talk.text().is_empty();
        self.world.step(seconds);
        self.mind.think(&mut self.world);
        for (id, mind) in &mut self.minds {
            self.world.with_creature(*id, |world| mind.think(world));
        }
        // The view follows the creature out to the yard and back in, or stays
        // close on it.
        let focus = if self.follow {
            self.world.ghost.body() + Vec3::Y * 0.25 * self.world.size()
        } else {
            self.world.view_focus(render::Camera::home())
        };
        self.camera.follow(focus, seconds);
        if !matches!(self.mode, Mode::Letter | Mode::Writing)
            && self
                .letter_reply_at
                .is_some_and(|time| self.world.time >= time)
        {
            self.letter_reply_at = None;
            // A question still waiting for its answer keeps the floor, and
            // so does comfort for how the viewer feels.
            if self.world.ghost.asleep < 0.3
                && self.world.focus.is_none()
                && self.world.telling.is_none()
                && (self.world.asking.is_none() && !self.world.awaiting_plan_answer())
                && self.world.comforting.is_none()
                && !self.world.reading_viewer_letter()
            {
                self.world.attend("viewer", 8.0);
                self.world.say(if self.world.speaks_korean() {
                    "마음에 들었어?"
                } else {
                    "Did you like it?"
                });
            }
        }
        self.world.celebrate_wish();
        if let Some(notice) = self.world.share_wish() {
            self.post(notice);
        }
        if let Some(stranger) = self.world.stranger
            && self.announced_stranger != Some(stranger.came)
        {
            self.announced_stranger = Some(stranger.came);
            self.post(format!(
                "A little {} is peeking in through the glass. Click it, or say “come in”, to ask it to stay.",
                stranger.species.noun()
            ));
        }
        self.world.share_anniversary();
        if self.paths.is_some() {
            self.album.observe(&self.world);
        }
        if self
            .hint_claude_at
            .is_some_and(|time| self.world.time >= time)
        {
            self.hint_claude_at = None;
            self.post("It talks in a few words of its own. With a Claude key (press k), it can really talk.");
        }
        let ease = 1.0 - (-1.5 * seconds).exp();
        let mut color = mood_color(self.world.ghost.mood);
        if let Some((favorite, strength)) = self.world.viewer_blush() {
            color = color.lerp(favorite, strength);
        }
        self.aura = self.aura.lerp(color, ease);
        let host = self.world.host();
        self.glows
            .retain(|(id, _)| self.world.creature(*id).is_some());
        for ghost in self.world.creatures() {
            if ghost.id == host {
                continue;
            }
            let color = mood_color(ghost.mood);
            match self.glows.iter_mut().find(|(id, _)| *id == ghost.id) {
                Some((_, glow)) => *glow = glow.lerp(color, ease),
                None => self.glows.push((ghost.id, color)),
            }
        }
        if self.live_clock && self.world.time - self.last_save > SAVE_EVERY {
            self.last_save = self.world.time;
            if let Err(error) = self.save() {
                self.post(format!("Couldn't save the household: {error}"));
            }
        }
    }

    /// Writes the ghost's life to disk, if it has a place there.
    pub fn save(&self) -> io::Result<()> {
        let Some(paths) = &self.paths else {
            return Ok(());
        };
        let world = &self.world;
        let save = Save {
            version: Save::VERSION,
            name: world.ghost.name.clone(),
            born: self.born,
            last_seen: mind::now(),
            needs: world.ghost.needs,
            plant: world.plant,
            lamp_on: world.lamp.on,
            goal: world.ghost.goal.clone(),
            memories: self.mind.journal.memories.clone(),
            conversation: self.mind.journal.conversation.clone(),
            life: world.saved_life(),
            seedlings: world.seedlings.clone(),
            room: Some(world.saved_room()),
            household: self.household(),
        };
        crate::store::save_json(&paths.save, &save)
    }

    /// Everyone but the creature the viewer is with, as a save keeps them.
    fn household(&self) -> crate::store::Household {
        let world = &self.world;
        crate::store::Household {
            id: world.ghost.id,
            species: world.ghost.species,
            genome: world.ghost.genome,
            housemates: world
                .kept_housemates()
                .into_iter()
                .map(|kept| {
                    let journal = self
                        .minds
                        .iter()
                        .find(|(id, _)| *id == kept.id)
                        .map(|(_, mind)| mind.journal.clone())
                        .unwrap_or_default();
                    crate::store::Housemate {
                        id: kept.id,
                        name: kept.name,
                        species: kept.species,
                        genome: kept.genome,
                        born: self
                            .borns
                            .iter()
                            .find(|(id, _)| *id == kept.id)
                            .map_or(self.born, |(_, born)| *born),
                        needs: kept.needs,
                        goal: kept.goal,
                        memories: journal.memories,
                        conversation: journal.conversation,
                        personal: kept.personal,
                    }
                })
                .collect(),
        }
    }

    /// Turns to creature `id`: the camera, the talk, the games, and the
    /// status line follow it. Returns whether there is such a creature.
    pub fn select(&mut self, id: u32) -> bool {
        let host = self.world.host();
        if id == host {
            return true;
        }
        let Some(index) = self.minds.iter().position(|(other, _)| *other == id) else {
            return false;
        };
        if !self.world.select(id) {
            return false;
        }
        let (_, mind) = self.minds.remove(index);
        let previous = std::mem::replace(&mut self.mind, mind);
        self.minds.push((host, previous));
        self.minds.sort_by_key(|(id, _)| *id);
        if let Some(index) = self.borns.iter().position(|(other, _)| *other == id) {
            let (_, born) = self.borns.remove(index);
            self.borns
                .push((host, std::mem::replace(&mut self.born, born)));
        }
        let glow = self
            .glows
            .iter()
            .find(|(other, _)| *other == id)
            .map_or(mood_color(self.world.ghost.mood), |(_, glow)| *glow);
        self.glows.retain(|(other, _)| *other != id);
        self.glows
            .push((host, std::mem::replace(&mut self.aura, glow)));
        self.selected_at = self.world.time;
        self.stroke = None;
        self.drawing_mode = false;
        true
    }

    /// Turns to the next creature in the household, or the one before.
    pub fn select_next(&mut self, forward: bool) {
        let roster = self.world.roster();
        if roster.len() < 2 {
            return;
        }
        let at = roster
            .iter()
            .position(|&id| id == self.world.host())
            .unwrap_or(0);
        let next = if forward {
            (at + 1) % roster.len()
        } else {
            (at + roster.len() - 1) % roster.len()
        };
        self.select(roster[next]);
    }

    /// A new creature of `species` comes to live here. Returns its name, or
    /// `None` when the cottage is full.
    pub fn adopt(&mut self, species: crate::world::Species, name: Option<&str>) -> Option<String> {
        let taken: Vec<String> = self
            .world
            .creatures()
            .map(|ghost| ghost.name.clone())
            .collect();
        let name = match name.map(str::trim).filter(|name| !name.is_empty()) {
            Some(name) => name
                .chars()
                .filter(|character| !character.is_control())
                .take(24)
                .collect(),
            None => {
                let mut random = crate::world::Random::new(
                    self.world.time.to_bits() ^ u64::from(self.world.household_size() as u32) << 40,
                );
                crate::world::species::pick_name(&mut random, &taken)
            }
        };
        let seed = mind::now() as u64 ^ self.world.time.to_bits();
        let id = self.world.adopt(species, &name, seed)?;
        self.minds.push((id, self.mind.sibling(Journal::default())));
        self.borns.push((id, mind::now()));
        Some(name)
    }

    /// Opens the adoption cards on the stranger at the glass, ready to name.
    fn welcome_stranger(&mut self) {
        let Some(stranger) = self.world.stranger else {
            return;
        };
        if self.world.household_size() >= crate::world::household::MOST {
            self.post("The cottage is full.");
            return;
        }
        self.adoption.index = crate::world::Species::ALL
            .iter()
            .position(|species| *species == stranger.species)
            .unwrap_or(0);
        let taken: Vec<String> = self
            .world
            .creatures()
            .map(|ghost| ghost.name.clone())
            .collect();
        let mut random = crate::world::Random::new(mind::now() as u64 ^ 0x5157);
        let name = crate::world::species::pick_name(&mut random, &taken);
        self.adoption.naming = Some(Line::from(name.as_str()));
        self.adoption.stranger = true;
        self.mode = Mode::Adopt;
    }

    /// Asks the stranger at the glass in to live here, under `name` or one
    /// it is given. Returns its name.
    pub fn invite_stranger(&mut self, name: Option<&str>) -> Option<String> {
        let taken: Vec<String> = self
            .world
            .creatures()
            .map(|ghost| ghost.name.clone())
            .collect();
        let name: String = match name.map(str::trim).filter(|name| !name.is_empty()) {
            Some(name) => name
                .chars()
                .filter(|character| !character.is_control())
                .take(24)
                .collect(),
            None => {
                let mut random = crate::world::Random::new(mind::now() as u64 ^ 0x5157);
                crate::world::species::pick_name(&mut random, &taken)
            }
        };
        let seed = mind::now() as u64 ^ self.world.time.to_bits();
        let id = self.world.invite_stranger(&name, seed)?;
        self.minds.push((id, self.mind.sibling(Journal::default())));
        self.borns.push((id, mind::now()));
        Some(name)
    }

    /// Two newcomers arrive at the door for a creature that has lived alone,
    /// as in a life saved before there were households.
    pub fn welcome_newcomers(&mut self) {
        let mut random = crate::world::Random::new(mind::now() as u64);
        let host = self.world.ghost.species;
        let kinds: Vec<_> = crate::world::Species::starters(&mut random)
            .into_iter()
            .filter(|species| *species != host)
            .take(2)
            .collect();
        let mut names = Vec::new();
        for species in kinds {
            if let Some(name) = self.adopt(species, None) {
                names.push(format!("{name} the {}", species.noun()));
            }
        }
        if !names.is_empty() {
            let host = self.world.ghost.name.clone();
            self.post(format!(
                "New friends came to live with {host}: {}. Tab turns to each.",
                crate::world::together::list(&names)
            ));
        }
    }

    /// Sets each housemate's journal from its saved life, and when it came.
    pub fn remember_housemates(&mut self, saved: &[crate::store::Housemate]) {
        for housemate in saved {
            if let Some((_, mind)) = self.minds.iter_mut().find(|(id, _)| *id == housemate.id) {
                mind.journal = Journal {
                    memories: housemate.memories.clone(),
                    conversation: housemate.conversation.clone(),
                };
            }
            if let Some((_, born)) = self.borns.iter_mut().find(|(id, _)| *id == housemate.id) {
                *born = housemate.born;
            }
        }
    }

    pub fn input(&mut self, input: Input) -> Response {
        match input {
            Input::Key(key) => self.press_key(key),
            Input::Paste(text) => {
                match self.mode {
                    Mode::Talking => self.talk.insert(&text),
                    Mode::Writing => self.draft.insert(&text),
                    Mode::Settings => self.settings.paste(&text),
                    _ => {}
                }
                Response::Continue
            }
            Input::Mouse(mouse) => self.mouse(mouse),
            Input::Resize { columns, rows } => {
                self.shake(usize::from(columns), usize::from(rows));
                Response::Restyle
            }
            Input::Focus(focused) => {
                self.focused = focused;
                self.world.look_away(!focused);
                Response::Continue
            }
        }
    }

    /// A line left in the terminal after quitting: how the ghost sees the
    /// viewer off, from its sleep, mood, and bond.
    pub fn farewell(&self) -> String {
        let world = &self.world;
        let ghost = &world.ghost;
        let name: String = ghost
            .name
            .chars()
            .filter(|character| !character.is_control())
            .collect();
        if let Some((promise, korean)) = world.promised_this_session() {
            return if korean {
                let subject = if name.chars().last().is_some_and(|last| {
                    ('가'..='힣').contains(&last)
                        && (u32::from(last) - u32::from('가')).is_multiple_of(28)
                }) {
                    "가"
                } else {
                    "이"
                };
                format!("{name}{subject} {} 기다릴게요.", promise.kind.korean())
            } else {
                format!("{name} will be waiting for you {}.", promise.kind.words())
            };
        }
        if world.speaks_korean() {
            return self.farewell_in_korean(&name);
        }
        let line = if world.taking_break() {
            "waves goodbye. We worked well together. Enjoy your break!"
        } else if ghost.asleep > 0.5 {
            "is asleep on its cushion. Sweet dreams."
        } else {
            match ghost.mood {
                Mood::Grumpy => "pretends not to watch you go.",
                Mood::Lonely => "watches you go. Come back soon?",
                Mood::Sleepy => "yawns and waves goodbye.",
                _ if world.bedtime() => "waves goodnight from the glass.",
                Mood::Excited | Mood::Cheerful if world.life.bond >= 0.55 => {
                    "presses against the glass and waves until you're gone."
                }
                _ if world.life.bond >= 0.55 => "waves from the glass. See you soon.",
                _ => "waves goodbye from the glass.",
            }
        };
        format!("{name} {line}")
    }

    // The same farewell for a viewer who talks to it in Korean.
    fn farewell_in_korean(&self, name: &str) -> String {
        let world = &self.world;
        let ghost = &world.ghost;
        // 이 follows a last syllable with a final consonant, 가 any other.
        let subject = if name
            .chars()
            .last()
            .filter(|last| ('가'..='힣').contains(last))
            .is_some_and(|last| (u32::from(last) - u32::from('가')) % 28 != 0)
        {
            "이"
        } else {
            "가"
        };
        let line = if world.taking_break() {
            "손을 흔들어요. 함께 열심히 했어. 푹 쉬어!"
        } else if ghost.asleep > 0.5 {
            "방석 위에서 자고 있어요. 좋은 꿈 꿔."
        } else {
            match ghost.mood {
                Mood::Grumpy => "가는 걸 못 본 척해요.",
                Mood::Lonely => "가는 모습을 바라봐요. 또 올 거지?",
                Mood::Sleepy => "하품하며 손을 흔들어요.",
                _ if world.bedtime() => "유리 너머에서 잘 자라고 손을 흔들어요.",
                Mood::Excited | Mood::Cheerful if world.life.bond >= 0.55 => {
                    "유리에 바짝 붙어 안 보일 때까지 손을 흔들어요."
                }
                _ if world.life.bond >= 0.55 => "유리 너머에서 손을 흔들어요. 또 만나!",
                _ => "유리 너머에서 손을 흔들어요.",
            }
        };
        format!("{name}{subject} {line}")
    }

    /// Whether the terminal has the focus; while it is elsewhere, frames
    /// can come more slowly.
    pub fn focused(&self) -> bool {
        self.focused
    }

    /// Reading panels leave mouse selection and the wheel to the terminal.
    pub fn wants_mouse(&self) -> bool {
        !matches!(
            self.mode,
            Mode::Journal | Mode::Help | Mode::Life | Mode::Album | Mode::Letter | Mode::Writing
        )
    }

    fn press_key(&mut self, key: Key) -> Response {
        self.world.notice_viewer();
        if key == Key::Control('c') {
            return Response::Quit;
        }
        match self.mode {
            Mode::Talking => {
                match key {
                    Key::Enter => {
                        let text = self.talk.take();
                        self.mode = Mode::Watching;
                        if !text.is_empty() {
                            // Calling a housemate by name turns to it.
                            if let Some(id) = self.world.addressed(&text) {
                                self.select(id);
                            }
                            // Asking the stranger at the glass in.
                            if self.world.stranger.is_some_and(|stranger| {
                                stranger.leaving.is_none() && stranger.looking_in(self.world.time)
                            }) && crate::world::stranger::invitation(&text)
                            {
                                let species = self.world.stranger.unwrap().species;
                                if let Some(name) = self.invite_stranger(None) {
                                    self.post(format!(
                                        "{name} the {} is coming in to live with you!",
                                        species.noun()
                                    ));
                                }
                            }
                            self.mind.hear(&mut self.world, &text);
                            if crate::world::household::to_everyone(&text) {
                                for (id, mind) in &mut self.minds {
                                    self.world.with_creature(*id, |world| {
                                        world.overhear(&text);
                                        mind.overhear(world, &text);
                                    });
                                }
                            }
                            // Its own few words answer right away; the
                            // hint about Claude comes once, after them.
                            if !self.mind.can_talk() && !self.hinted_claude {
                                self.hinted_claude = true;
                                self.hint_claude_at = Some(self.world.time + HINT_AFTER);
                            }
                        }
                    }
                    Key::Escape => self.mode = Mode::Watching,
                    key => {
                        self.talk.edit(&key);
                    }
                }
                Response::Continue
            }
            Mode::Settings => {
                match key {
                    Key::Enter => self.save_settings(),
                    Key::Escape => self.mode = Mode::Watching,
                    key => {
                        self.settings.press(&key);
                    }
                }
                Response::Continue
            }
            Mode::Life
                if matches!(
                    key,
                    Key::Up | Key::Down | Key::PageUp | Key::PageDown | Key::Home | Key::End
                ) =>
            {
                let (columns, rows) = self.screen_size;
                let (total, page) = ui::life_extent(self, columns, rows);
                let limit = total.saturating_sub(page);
                let scroll = self.life_scroll.min(limit);
                self.life_scroll = match key {
                    Key::Up => scroll.saturating_sub(1),
                    Key::Down => scroll + 1,
                    Key::PageUp => scroll.saturating_sub(page),
                    Key::PageDown => scroll + page,
                    Key::Home => 0,
                    _ => limit,
                }
                .min(limit);
                Response::Continue
            }
            Mode::Help | Mode::Life => {
                if matches!(key, Key::Character('q')) {
                    return Response::Quit;
                }
                self.mode = Mode::Watching;
                Response::Continue
            }
            Mode::Journal => self.scroll_journal(key),
            Mode::Album => {
                let oldest = self.album.pages.len().saturating_sub(1);
                self.album.index = self.album.index.min(oldest);
                match key {
                    Key::Character('q') => return Response::Quit,
                    Key::Escape | Key::Character('b') => self.mode = Mode::Watching,
                    Key::Left => self.album.index = self.album.index.saturating_sub(1),
                    Key::Right => self.album.index = (self.album.index + 1).min(oldest),
                    Key::Home => self.album.index = 0,
                    Key::End => self.album.index = oldest,
                    _ => {}
                }
                Response::Continue
            }
            Mode::Writing => {
                match key {
                    Key::Escape => self.mode = Mode::Watching,
                    Key::Control('s') if !self.draft.is_empty() => {
                        let korean = crate::world::penpal::korean(self.draft.text())
                            || (!self.draft.text().chars().any(char::is_alphabetic)
                                && self.world.speaks_korean());
                        if self.world.receive_letter(self.draft.text(), korean) {
                            self.draft = crate::draft::Draft::default();
                            self.mode = Mode::Watching;
                        }
                    }
                    key => self.draft.edit(&key),
                }
                Response::Continue
            }
            Mode::Letter => {
                match key {
                    Key::Character('w') => {
                        self.mode = Mode::Writing;
                        self.letter_reply_at = None;
                    }
                    Key::Character('v')
                        if !self.world.life.received.is_empty()
                            && !self.world.life.letters.is_empty() =>
                    {
                        self.sent_letters = !self.sent_letters;
                        self.letter_index = if self.sent_letters {
                            self.world.life.received.len()
                        } else {
                            self.world.life.letters.len()
                        }
                        .saturating_sub(1);
                        self.letter_scroll = 0;
                        if !self.sent_letters {
                            self.letter_read |= self.world.read_letter(self.letter_index);
                        }
                    }
                    Key::Character('q') => return Response::Quit,
                    Key::Escape | Key::Enter | Key::Character('e') => {
                        self.mode = Mode::Watching;
                        if self.letter_read {
                            self.letter_reply_at = Some(self.world.time + 1.0);
                            self.letter_read = false;
                        }
                    }
                    Key::Left | Key::Right => {
                        let newest = if self.sent_letters {
                            self.world.life.received.len()
                        } else {
                            self.world.life.letters.len()
                        }
                        .saturating_sub(1);
                        self.letter_index = if key == Key::Left {
                            self.letter_index.saturating_sub(1)
                        } else {
                            (self.letter_index + 1).min(newest)
                        };
                        self.letter_scroll = 0;
                        if !self.sent_letters {
                            self.letter_read |= self.world.read_letter(self.letter_index);
                        }
                    }
                    // Drawing clamps the scroll to what the screen can show.
                    Key::Up => self.letter_scroll = self.letter_scroll.saturating_sub(1),
                    Key::Down => self.letter_scroll = self.letter_scroll.saturating_add(1).min(64),
                    _ => {}
                }
                Response::Continue
            }
            Mode::Adopt => {
                self.press_adopt(key);
                Response::Continue
            }
            Mode::Watching => self.command(key),
        }
    }

    fn press_adopt(&mut self, key: Key) {
        let kinds = crate::world::Species::ALL.len();
        if let Some(name) = &mut self.adoption.naming {
            match key {
                Key::Enter => {
                    let name = name.take();
                    let species = crate::world::Species::ALL[self.adoption.index.min(kinds - 1)];
                    self.mode = Mode::Watching;
                    self.adoption.naming = None;
                    if std::mem::take(&mut self.adoption.stranger)
                        && self
                            .world
                            .stranger
                            .is_some_and(|stranger| stranger.species == species)
                    {
                        match self.invite_stranger(Some(&name)) {
                            Some(name) => self.post(format!(
                                "{name} the {} is coming in to live with you!",
                                species.noun()
                            )),
                            None => self.post("The cottage is full."),
                        }
                        return;
                    }
                    match self.adopt(species, Some(&name)) {
                        Some(name) => self.post(format!(
                            "{name} the {} is at the door! Tab turns to them.",
                            species.noun()
                        )),
                        None => self.post("The cottage is full."),
                    }
                }
                Key::Escape => {
                    self.adoption.naming = None;
                    self.adoption.stranger = false;
                }
                key => {
                    name.edit(&key);
                }
            }
            return;
        }
        let across = self.adoption.across.max(1);
        let index = self.adoption.index.min(kinds - 1);
        match key {
            Key::Escape | Key::Character('n') => self.mode = Mode::Watching,
            Key::Left => self.adoption.index = index.saturating_sub(1),
            Key::Right | Key::Tab => self.adoption.index = (index + 1).min(kinds - 1),
            Key::Up => self.adoption.index = index.saturating_sub(across),
            Key::Down => self.adoption.index = (index + across).min(kinds - 1),
            Key::Enter if self.world.household_size() < crate::world::household::MOST => {
                let taken: Vec<String> = self
                    .world
                    .creatures()
                    .map(|ghost| ghost.name.clone())
                    .collect();
                let mut random = crate::world::Random::new(mind::now() as u64 ^ index as u64);
                let suggestion = crate::world::species::pick_name(&mut random, &taken);
                self.adoption.naming = Some(Line::from(suggestion.as_str()));
            }
            _ => {}
        }
    }

    fn command(&mut self, key: Key) -> Response {
        match key {
            Key::Tab | Key::Character(']') => self.select_next(true),
            Key::BackTab | Key::Character('[') => self.select_next(false),
            Key::Character('n') => {
                self.adoption.naming = None;
                self.adoption.stranger = false;
                self.mode = Mode::Adopt;
            }
            Key::Character('v') => {
                self.follow = !self.follow;
                self.post(if self.follow {
                    "The view stays close on them (v to step back)."
                } else {
                    "The view steps back to the room."
                });
            }
            Key::Character('x') => self.toggle_hiding(),
            Key::Escape if self.world.hunt_counting() => self.toggle_hiding(),
            Key::Character('f') => self.toggle_drawing(),
            Key::Escape if self.drawing_mode => self.toggle_drawing(),
            Key::Character('q') | Key::Escape => return Response::Quit,
            Key::Enter | Key::Character('t') => self.mode = Mode::Talking,
            Key::Character('k') => {
                self.settings = Settings::from(&self.config);
                self.mode = Mode::Settings;
            }
            Key::Character('?' | 'h') => self.mode = Mode::Help,
            Key::Character('j') => {
                self.journal_scroll = 0;
                self.mode = Mode::Journal;
            }
            Key::Character('l') => {
                self.life_scroll = 0;
                self.mode = Mode::Life;
            }
            Key::Character('b') => {
                self.album.index = 0;
                self.mode = Mode::Album;
            }
            Key::Character('e') => self.open_letter(),
            Key::Character('p') => {
                if self.save_postcard() {
                    self.world.photographed(false);
                }
            }
            Key::Character('P') => self.pose_for_postcard(),
            Key::Character('o') => self.world.toggle_focus(),
            Key::Character('g') => {
                if !self.world.challenge() {
                    self.post("It's asleep.");
                }
            }
            Key::Character(character @ '1'..='9') => {
                self.world.mark(character as usize - '1' as usize);
            }
            Key::Character('m') => {
                self.style = self.style.next();
                self.post(format!("Style: {}", self.style.label()));
                return Response::Restyle;
            }
            Key::Character('c') => {
                self.color = self.color.next();
                self.post(format!("Color: {}", self.color.label()));
                return Response::Restyle;
            }
            Key::Left | Key::Character('a') => self.camera.turn(-TURN, 0.0),
            Key::Right | Key::Character('d') => self.camera.turn(TURN, 0.0),
            Key::Up | Key::Character('w') => self.camera.turn(0.0, TURN),
            Key::Down | Key::Character('s') => self.camera.turn(0.0, -TURN),
            Key::Character('+' | '=') => self.camera.zoom_by(1.1),
            Key::Character('-' | '_') => self.camera.zoom_by(1.0 / 1.1),
            Key::Character('r') => self.camera = Camera::default(),
            Key::Character(' ') => self.paused = !self.paused,
            _ => {}
        }
        if self.mode != Mode::Watching {
            self.world.lift_finger();
            self.drawing_right = false;
            self.press = None;
            self.stroke = None;
        }
        Response::Continue
    }

    fn open_letter(&mut self) {
        let Some(index) = self
            .world
            .unread_letter()
            .or_else(|| self.world.life.letters.len().checked_sub(1))
        else {
            // Before the ghost has written, the viewer's own letters are the
            // ones to browse; with none either, a fresh page.
            if let Some(newest) = self.world.life.received.len().checked_sub(1) {
                self.sent_letters = true;
                self.letter_index = newest;
                self.letter_scroll = 0;
                self.letter_read = false;
                self.mode = Mode::Letter;
            } else {
                self.mode = Mode::Writing;
            }
            return;
        };
        self.sent_letters = false;
        self.letter_index = index;
        self.letter_scroll = 0;
        self.letter_read = self.world.read_letter(index);
        self.mode = Mode::Letter;
        self.world.lift_finger();
        self.press = None;
        self.stroke = None;
        self.drawing_right = false;
    }

    fn toggle_hiding(&mut self) {
        if self.world.hunt_counting() {
            self.world.cancel_hunt();
            self.post("Treasure hiding cancelled.");
        } else {
            match self.world.start_hiding() {
                Ok(()) => {
                    self.drawing_mode = false;
                    self.world.lift_finger();
                    self.press = None;
                    self.stroke = None;
                    self.post("click the floor to hide a treasure · Esc cancels");
                }
                Err(message) => self.post(message),
            }
        }
    }

    fn toggle_drawing(&mut self) {
        if self.world.hunt.is_some() {
            return;
        }
        self.drawing_mode = !self.drawing_mode;
        self.world.lift_finger();
        self.press = None;
        self.stroke = None;
        self.drawing_right = false;
        self.post(if self.drawing_mode {
            "Drawing in the fog: drag on the glass. f or Esc to stop."
        } else {
            "Finished drawing in the fog."
        });
    }

    fn trace_glass(&mut self, point: Option<Vec2>) {
        match point
            .zip(self.view)
            .and_then(|(point, view)| view.on_glass(point))
        {
            Some(point) => self.world.draw_on_glass(point),
            None => self.world.lift_finger(),
        }
    }

    /// Returns whether a postcard was saved.
    fn save_postcard(&mut self) -> bool {
        let Some(paths) = &self.paths else {
            self.post("Postcards need a saved life");
            return false;
        };
        let Some(postcard) = &self.postcard else {
            return false;
        };
        let (message, saved) = match postcard.save(paths) {
            Ok(path) => (
                format!("Saved a postcard: {}", postcard::display_path(&path)),
                true,
            ),
            Err(error) => (format!("Couldn't save a postcard: {error}"), false),
        };
        self.post(message);
        saved
    }

    // A portrait: the ghost comes to the glass and smiles, and the picture
    // is taken a moment later. A sleeper or a paused room is taken as it is.
    fn pose_for_postcard(&mut self) {
        if self.paths.is_none() {
            self.post("Postcards need a saved life");
        } else if self.posing.is_none() && !self.paused && self.world.pose_for_photo() {
            self.posing = Some(POSING);
            self.post("Say cheese…");
        } else if self.posing.is_none() && self.save_postcard() {
            self.world.photographed(false);
        }
    }

    pub fn doing(&self) -> String {
        if let Some(focus) = &self.world.focus {
            return format!(
                "focusing · {} min left",
                focus.minutes_left(self.world.time)
            );
        }
        if self.world.ghost.asleep > 0.5 {
            "asleep on the cushion".to_owned()
        } else if self.world.ghost.plan.activity == crate::world::Activity::Friend
            && let Some(friend) = &self.world.life.friend
        {
            format!("playing with {}", friend.name)
        } else if self.world.ghost.plan.activity == crate::world::Activity::Air {
            self.world.air_doing().to_owned()
        } else if self.world.ghost.plan.activity == crate::world::Activity::Yard {
            self.world.outing_words().to_owned()
        } else if self.world.ghost.plan.activity == crate::world::Activity::Together
            && !self.world.ghost.plan.done
            && let Some(partner) = self
                .world
                .ghost
                .plan
                .with
                .and_then(|id| self.world.creature(id))
        {
            match self.world.ghost.plan.pastime {
                Some(pastime) => pastime.doing(&partner.name),
                None => format!("with {}", partner.name),
            }
        } else {
            self.world
                .ghost
                .plan
                .activity
                .doing_as(self.world.ghost.species)
                .to_owned()
        }
    }

    /// The ghost's name and activity for the terminal window title.
    pub fn title(&self) -> String {
        // A title is an escape sequence, so nothing in it may control the
        // terminal.
        let doing = if let Some(focus) = &self.world.focus {
            format!(
                "focusing with you · {} min left",
                focus.minutes_left(self.world.time)
            )
        } else {
            self.doing()
        };
        format!("{} · {}", self.world.ghost.name, doing)
            .chars()
            .filter(|character| !character.is_control())
            .collect()
    }

    fn scroll_journal(&mut self, key: Key) -> Response {
        match key {
            Key::Character('q') => return Response::Quit,
            Key::Escape | Key::Enter | Key::Character('j') => self.mode = Mode::Watching,
            Key::Up | Key::Down | Key::PageUp | Key::PageDown | Key::Home | Key::End => {
                let journal = ui::JournalPanel::new(self, self.screen_size.0, self.screen_size.1);
                let limit = journal.max_scroll();
                self.journal_scroll = self.journal_scroll.min(limit);
                self.journal_scroll = match key {
                    Key::Up => self.journal_scroll.saturating_add(1),
                    Key::Down => self.journal_scroll.saturating_sub(1),
                    Key::PageUp => self.journal_scroll.saturating_add(journal.height),
                    Key::PageDown => self.journal_scroll.saturating_sub(journal.height),
                    Key::Home => limit,
                    Key::End => 0,
                    _ => self.journal_scroll,
                }
                .min(limit);
            }
            _ => {}
        }
        Response::Continue
    }

    // Saves the settings panel and starts using it at once: the next
    // requests go out with the new keys, models, effort, and caching.
    fn save_settings(&mut self) {
        let settings = std::mem::replace(&mut self.settings, Settings::from(&self.config));
        settings.apply(&mut self.config);
        self.mode = Mode::Watching;
        self.mind.reconfigure(&self.config);
        for (_, mind) in &mut self.minds {
            mind.reconfigure_beside(&self.config, &self.mind);
        }
        let saved = match &self.paths {
            Some(paths) => self.config.save(&paths.config),
            None => Ok(()),
        };
        let on = |on: bool| if on { "on" } else { "off" };
        let message = match saved {
            Ok(()) => format!(
                "Saved. Claude {} ({}), effort {}, cache {}. Jev {} ({}).",
                self.config.claude_model,
                on(self.mind.can_talk()),
                effort_name(self.config.claude_effort),
                cache_name(self.config.cache),
                self.config.jev_model,
                on(self.mind.one != crate::mind::Status::Off),
            ),
            Err(error) => format!("Couldn't save the settings: {error}"),
        };
        self.post(message);
    }

    /// Applies a mouse event. Dragging turns the view, except that a drag
    /// begun on the ghost strokes it, and the wheel zooms. A click on the
    /// ghost pokes it, on the ball flicks it, and anywhere else in the room
    /// drops in a light mote.
    pub fn mouse(&mut self, mouse: Mouse) -> Response {
        if !self.wants_mouse() {
            return Response::Continue;
        }
        self.world.notice_viewer();
        let point = self.scene_point(mouse.column, mouse.row);
        match mouse.action {
            MouseAction::ScrollUp => self.camera.zoom_by(1.1),
            MouseAction::ScrollDown => self.camera.zoom_by(1.0 / 1.1),
            MouseAction::Move => {}
            MouseAction::DrawPress | MouseAction::DrawDrag | MouseAction::DrawRelease => {
                if self.mode == Mode::Watching && self.world.hunt.is_none() {
                    match mouse.action {
                        MouseAction::DrawPress => {
                            self.world.lift_finger();
                            self.drawing_right = true;
                            self.trace_glass(point);
                        }
                        MouseAction::DrawDrag if self.drawing_right => self.trace_glass(point),
                        MouseAction::DrawRelease if self.drawing_right => {
                            self.trace_glass(point);
                            self.world.lift_finger();
                            self.drawing_right = false;
                        }
                        _ => {}
                    }
                }
            }
            MouseAction::Press => {
                self.press = Some((mouse.column, mouse.row));
                self.dragged = false;
                self.stroke = point.filter(|point| {
                    if self.drawing_mode && self.mode == Mode::Watching {
                        return false;
                    }
                    self.view.is_some_and(|view| {
                        self.creature_at(view, *point) == Some(self.world.host())
                            && !self.touches_friend(view, *point)
                    })
                });
            }
            MouseAction::Drag => {
                if self.drawing_mode && self.mode == Mode::Watching {
                    if let Some((column, row)) = self.press
                        && (mouse.column != column || mouse.row != row)
                    {
                        if !self.dragged {
                            self.trace_glass(self.scene_point(column, row));
                        }
                        self.dragged = true;
                    }
                    if self.dragged {
                        self.trace_glass(point);
                    }
                } else if let (Some(last), Some(point), Some(view)) =
                    (self.stroke, point, self.view)
                {
                    if point != last {
                        self.dragged = true;
                        self.stroke = Some(point);
                        if self.creature_at(view, point) == Some(self.world.host()) {
                            self.world.pet(point.distance(last));
                            let across = point.x - last.x;
                            self.stroke_side = if across.abs() > 1e-4 {
                                across.signum()
                            } else {
                                0.0
                            };
                        }
                    }
                } else if self.stroke.is_none()
                    && let Some((column, row)) = self.press
                {
                    let across = mouse.column as f32 - column as f32;
                    let down = mouse.row as f32 - row as f32;
                    if across != 0.0 || down != 0.0 {
                        self.camera
                            .turn(-across * DRAG_TURN, down * DRAG_TURN * 2.0);
                        self.press = Some((mouse.column, mouse.row));
                        self.dragged = true;
                    }
                }
            }
            MouseAction::Release => {
                if self.drawing_mode && self.mode == Mode::Watching && self.dragged {
                    self.trace_glass(point);
                    self.world.lift_finger();
                }
                self.stroke = None;
                if self.press.take().is_some()
                    && !self.dragged
                    && let Some(point) = point
                {
                    self.click(point);
                }
            }
        }

        self.pointer = point.map(|point| (point, self.world.time));
        self.hover = point
            .zip(self.view)
            .and_then(|(point, view)| self.creature_at(view, point));
        let pointer = point
            .zip(self.view)
            .and_then(|(point, view)| view.on_glass(point));
        self.world.point_at(pointer);
        Response::Continue
    }

    fn touches_friend(&self, view: View, point: Vec2) -> bool {
        self.world.friend.as_ref().is_some_and(|friend| {
            friend.visibility() > 0.0
                && view.touches_ghost(point, friend.position, FRIEND_SCALE)
                && (self.creature_at(view, point).is_none()
                    || (friend.position - self.world.ghost.body()).dot(view.forward) < 0.0)
        })
    }

    /// The creature under `point` on the screen, the nearest if several are,
    /// allowing half a cell either way since they are small.
    pub fn creature_at(&self, view: View, point: Vec2) -> Option<u32> {
        if self.shown.is_empty() {
            return None;
        }
        let models: Vec<Model> = self.shown.iter().map(Model::of).collect();
        let (columns, rows) = self.area;
        let cell = Grid::new(columns.max(1), rows.max(1), CELL_ASPECT).scale();
        [
            Vec2::ZERO,
            Vec2::new(cell, 0.0),
            Vec2::new(-cell, 0.0),
            Vec2::new(0.0, cell * 1.5),
            Vec2::new(0.0, -cell * 1.5),
        ]
        .into_iter()
        .find_map(|offset| {
            let direction = view.ray(point + offset * 0.5);
            models
                .iter()
                .filter_map(|model| {
                    model
                        .trace(view.eye, direction, f32::INFINITY, 0.004, 0.0)
                        .map(|hit| (model.id, hit.distance))
                })
                .min_by(|first, second| first.1.total_cmp(&second.1))
                .map(|(id, _)| id)
        })
    }

    // A resize shakes the box like a snow globe, by how much the screen
    // changed, while the room is on screen and running.
    fn shake(&mut self, columns: usize, rows: usize) {
        let (old_columns, old_rows) = self.screen_size;
        let watching = matches!(self.mode, Mode::Watching | Mode::Talking);
        if old_columns > 0 && old_rows > 0 && !self.paused && watching {
            let change = |new: usize, old: usize| (new as f32 - old as f32) / old as f32;
            self.world
                .jostle(change(columns, old_columns), change(rows, old_rows));
        }
        self.screen_size = (columns, rows);
    }

    fn click(&mut self, point: Vec2) {
        let Some(view) = self.view else {
            return;
        };
        if let Some(game) = &self.world.game
            && game.outcome.is_none()
            && let Some(cell) = view
                .on_glass(point)
                .and_then(|point| game.cell_at(point.truncate()))
        {
            self.world.mark(cell);
            return;
        }
        if self.touches_friend(view, point) {
            self.world.poke_friend();
            return;
        }
        if let Some(id) = self.creature_at(view, point) {
            if id == portrait::STRANGER {
                self.welcome_stranger();
            } else if id == self.world.host() {
                self.world.poke();
            } else {
                self.select(id);
            }
            return;
        }
        if self.world.pop_bubble(view.eye, view.ray(point)) {
            return;
        }
        if self.world.hunt_counting() {
            if let Some(spot) = view
                .on_plane(point, 0.0)
                .filter(|spot| spot.cmpge(ROOM_MIN).all() && spot.cmple(ROOM_MAX).all())
                && self.world.hide_treasure(spot)
            {
                self.notice = None;
            }
            return;
        }
        let direction = view.ray(point);
        let block = self
            .world
            .blocks
            .iter()
            .enumerate()
            .filter_map(|(index, block)| {
                let offset = view.eye - block.position;
                let along = -offset.dot(direction);
                let miss = (offset + direction * along).length();
                (along > 0.0 && miss < crate::world::blocks::RADIUS).then_some((index, along))
            })
            .min_by(|first, second| first.1.total_cmp(&second.1));
        let offset = view.eye - self.world.ball.position;
        let along = -offset.dot(direction);
        let miss = (offset + direction * along).length();
        let ball = (along > 0.0 && miss < Ball::RADIUS * 1.6).then_some(along);
        let music_box = crate::world::music::hit_box(view.eye, direction);
        // Whichever the ray meets first takes the click.
        let nearest = [block.map(|(_, distance)| distance), ball, music_box]
            .into_iter()
            .flatten()
            .min_by(f32::total_cmp);
        if let Some(nearest) = nearest {
            if let Some((index, _)) = block.filter(|(_, distance)| *distance == nearest) {
                self.world.nudge_block(index, direction);
            } else if ball == Some(nearest) {
                self.world.kick_ball(direction);
            } else {
                self.world.wind_music_box();
            }
            return;
        }
        if view.on_window(point).is_some() {
            self.world.ask_window(!self.world.window_open);
            return;
        }
        if let Some(thing) = self.world.yard_thing_at(view.eye, direction) {
            use crate::world::outing::YardThing;
            if thing == YardThing::Mailbox {
                if self.world.unread_letter().is_some() {
                    self.open_letter();
                    return;
                }
            } else {
                self.world.click_yard(thing);
                return;
            }
        }
        // A mote appears where the ray meets the floor, a little above it,
        // or behind the glass if the ray misses the floor.
        let floor = view
            .on_plane(point, 0.0)
            .filter(|spot| spot.cmpge(ROOM_MIN).all() && spot.cmple(ROOM_MAX).all());
        if self.world.unread_letter().is_some() && floor.is_some_and(crate::world::mail::hit) {
            self.open_letter();
            return;
        }
        // Out on the lawn, too: the ghost goes out to catch it.
        let lawn = view
            .on_plane(point, 0.0)
            .filter(|spot| {
                crate::world::home::on_lot(*spot) && !crate::world::home::in_house(*spot)
            })
            .filter(|_| floor.is_none() && view.on_glass(point).is_none());
        let spot = floor
            .or(lawn)
            .map(|spot| spot + Vec3::Y * 0.9)
            .or_else(|| view.on_glass(point).map(|spot| spot - Vec3::Z * 0.5));
        if let Some(spot) = spot {
            self.world.drop_mote(spot);
        }
    }

    // The scene-space point under a cell of the scene area.
    fn scene_point(&self, column: usize, row: usize) -> Option<Vec2> {
        let (columns, rows) = self.area;
        (column < columns && row < rows).then(|| {
            Grid::new(columns, rows, CELL_ASPECT)
                .point(Vec2::new(column as f32 + 0.5, row as f32 + 0.5))
        })
    }

    /// Posts a short message for the viewer under the scene.
    pub fn post(&mut self, message: impl Into<String>) {
        self.notice = Some((message.into(), self.world.time));
    }

    pub fn notice(&self) -> Option<&str> {
        if self.world.hunt_counting() {
            return Some("click the floor to hide a treasure · Esc cancels");
        }
        self.notice
            .as_ref()
            .filter(|(_, time)| self.world.time - time < 8.0)
            .map(|(message, _)| message.as_str())
    }

    /// Draws the whole screen, `columns` by `rows`: the scene above two
    /// lines of words, and any open panel over it.
    pub fn frame(&mut self, columns: usize, rows: usize) -> Canvas {
        self.screen_size = (columns, rows);
        if self.paths.is_some() {
            self.album.observe(&self.world);
        }
        if self.mode == Mode::Album {
            let mut screen = Canvas::blank(columns, rows);
            ui::album_panel(self, &mut screen);
            return screen;
        }
        let scene_rows = rows.saturating_sub(2).max(1);
        self.area = (columns, scene_rows);
        let grid = self.style.grid(columns, scene_rows);
        let view = self.camera.view(grid);
        self.view = Some(view);
        let samples = if self.style == Style::Braille { 1 } else { 2 };
        let frame = render::render(&self.compose(view), grid, samples);

        let mut screen = Canvas::blank(columns, rows);
        screen.paste(&self.style.canvas(&frame, self.color), 0, 0);
        ui::draw_moment(self, &mut screen, &view, scene_rows);
        if self.mode == Mode::Watching && self.paths.is_some() {
            let postcard_rows = rows.min(scene_rows + 1);
            let mut canvas = Canvas::blank(columns, postcard_rows);
            canvas.paste(&screen, 0, 0);
            let colored = if self.color == ColorMode::Plain && self.style == Style::Blocks {
                let mut canvas = Canvas::blank(columns, postcard_rows);
                canvas.paste(&self.style.canvas(&frame, ColorMode::TrueColor), 0, 0);
                ui::draw_moment(self, &mut canvas, &view, scene_rows);
                Some(canvas)
            } else {
                None
            };
            if !self.paused
                && let Err(error) = self.album.capture(
                    self.paths.as_ref().unwrap(),
                    &self.world,
                    colored.as_ref().unwrap_or(&canvas),
                    mind::now(),
                )
            {
                self.post(format!("Couldn't keep an album moment: {error}"));
            }
            self.postcard = Some(Postcard::new(canvas, colored, &self.world, &self.doing()));
            if self.posing.is_some_and(|left| left <= 0.0) {
                self.posing = None;
                if self.save_postcard() {
                    self.world.photographed(true);
                }
            }
        }
        ui::draw(self, &mut screen, scene_rows);
        screen
    }

    /// Everything one frame shows from `view`.
    fn compose(&mut self, view: View) -> Scene<'_> {
        let figures = self.figures(&view);
        let world = &self.world;
        let ghost = &world.ghost;
        let quirk = world.active_quirk();
        let comfort_breath = world.comfort_breath();
        let energy = ghost.needs.energy;
        let aura = Aura {
            color: self.aura.lerp(
                Vec3::new(0.45, 0.7, 1.0),
                quirk
                    .filter(|spell| spell.kind == crate::world::Quirk::Shiver)
                    .map_or(0.0, |spell| 0.5 * spell.envelope(world.time)),
            ),
            visibility: 1.0 - ghost.fade,
            brightness: (0.25 + 0.75 * energy)
                * world.cold_glow()
                * comfort_breath.map_or(1.0, |phase| 0.9 + 0.1 * phase),
            beat: beat(ghost.mood) * (0.6 + 0.4 * energy),
        };
        let friend =
            world
                .friend
                .as_ref()
                .zip(world.life.friend.as_ref())
                .map(|(friend, companion)| {
                    let time = world.time + f64::from(friend.phase);
                    let slot = (time / 4.0).floor() as u64;
                    let blink_at =
                        0.5 + ((slot.wrapping_mul(0x9E37_79B9) >> 12) % 250) as f64 / 100.0;
                    let blink =
                        (1.0 - ((time.rem_euclid(4.0) - blink_at) / 0.12).abs()).max(0.0) as f32;
                    let poked = (1.0 - (world.time - friend.poked_at) / 1.2).clamp(0.0, 1.0) as f32;
                    let leaving = friend.state == crate::world::friend::State::Leaving;
                    Body {
                        accessory: None,
                        position: friend.position,
                        scale: FRIEND_SCALE,
                        pose: Pose::new(
                            time,
                            friend.velocity.dot(view.right) / FRIEND_SCALE,
                            friend.velocity.dot(view.up) / FRIEND_SCALE,
                            0.0,
                        )
                        .in_breeze(world.breeze_at(friend.position).dot(view.right), 0.0),
                        face: face::Face {
                            shut: [blink; 2],
                            widen: poked,
                            smile: if poked > 0.5 {
                                0.0
                            } else if friend.state == crate::world::friend::State::Playing
                                || leaving
                                || poked > 0.0
                            {
                                0.75
                            } else {
                                0.0
                            },
                            gaze: if leaving {
                                Vec2::new((time as f32 * 6.0).sin() * 0.5, 0.0)
                            } else {
                                Vec2::new(
                                    friend.heading.dot(view.right),
                                    friend.heading.dot(view.up),
                                ) * 0.5
                            },
                            ..face::Face::REST
                        },
                        aura: Aura {
                            color: friend_color(companion.hue),
                            visibility: 1.0,
                            brightness: 0.75,
                            beat: 0.8,
                        },
                        opacity: friend.visibility(),
                    }
                });
        Scene {
            world,
            outside: Outside {
                hour: world.hour,
                rain: world.weather.rain(world.time),
                storm: world.weather.storm,
                flash: world.weather.flash(world.time),
                last_flash: world.weather.last_flash,
                rainbow: world.weather.rainbow(world.time),
                season: world.season(),
                snow: world.weather.snow(world.time),
                snow_cover: world.weather.snow_cover,
                moon_phase: world.moon_phase(),
                fireworks: world.fireworks(),
                bird: world
                    .visitors
                    .bird
                    .as_ref()
                    .map(|bird| bird.picture(world.time)),
                cat: world
                    .visitors
                    .cat
                    .as_ref()
                    .map(|cat| cat.picture(world.time)),
                time: world.time,
                sill: world.life.sill,
                meteor: world
                    .shooting_star()
                    .map(|meteor| meteor.streak(world.time)),
                constellations: crate::render::sky::StarSegment::from_constellations(
                    &world.life.constellations,
                ),
                constellation_base: 0.018,
                constellation_glow: world.constellation_glow() * 0.085,
                wind: world.wind.sideways(world.time),
                window_lift: world.window_lift(),
                wind_travel: world.wind.travel,
                passersby: world.passersby(),
            },
            view,
            aura,
            figures,
            friend,
        }
    }

    /// Every creature as it looks this frame, the one the viewer is with
    /// first.
    pub fn figures(&mut self, view: &View) -> Vec<Figure> {
        let host = self.world.host();
        let ids = self.world.roster();
        let mut figures = Vec::with_capacity(ids.len());
        for id in ids {
            let handling = Handling {
                pointer: self.pointer.filter(|_| id == host),
                stroke_side: if id == host { self.stroke_side } else { 0.0 },
                glow: if id == host {
                    self.aura
                } else {
                    self.glows
                        .iter()
                        .find(|(other, _)| *other == id)
                        .map_or(Vec3::new(0.35, 0.86, 0.8), |(_, glow)| *glow)
                },
            };
            if let Some((figure, puffs)) = self.world.with_creature(id, |world| {
                (
                    portrait::figure(world, view, &handling),
                    crate::face::emote::puffs(world),
                )
            }) {
                figures.push(figure);
                self.puffs.retain(|(other, _)| *other != id);
                self.puffs.push((id, puffs));
            }
        }
        let roster = self.world.roster();
        self.puffs.retain(|(id, _)| roster.contains(id));
        figures.sort_by_key(|figure| figure.id != host);
        if let Some(stranger) = &self.world.stranger {
            figures.push(portrait::stranger(stranger, self.world.time));
        }
        self.shown = figures.clone();
        figures
    }

    /// The scene as plain pixels, `width` by `height`, for checking how it
    /// looks without a terminal.
    pub fn picture(&mut self, width: usize, height: usize) -> crate::frame::Frame {
        let grid = crate::frame::Grid::new(width, height, 1.0);
        let view = self.camera.view(grid);
        render::render(&self.compose(view), grid, 1)
    }

    /// The creatures as the last frame showed them.
    pub fn shown(&self) -> &[Figure] {
        &self.shown
    }

    /// The symbols rising from creature `id`'s head this frame.
    pub fn puffs(&self, id: u32) -> &[crate::face::emote::Puff] {
        self.puffs
            .iter()
            .find(|(other, _)| *other == id)
            .map_or(&[], |(_, puffs)| puffs)
    }

    /// The glow of creature `id`'s mood.
    pub fn glow(&self, id: u32) -> Vec3 {
        if id == self.world.host() {
            return self.aura;
        }
        self.glows
            .iter()
            .find(|(other, _)| *other == id)
            .map_or(Vec3::new(0.35, 0.86, 0.8), |(_, glow)| *glow)
    }

    pub fn born(&self) -> i64 {
        self.born
    }

    pub fn config_path(&self) -> Option<&std::path::Path> {
        self.paths.as_ref().map(|paths| paths.config.as_path())
    }

    /// The color the ghost glows right now.
    pub fn aura(&self) -> Vec3 {
        self.aura
    }
}

/// The color of the ghost's glow in each mood.
pub fn mood_color(mood: Mood) -> Vec3 {
    match mood {
        Mood::Content => Vec3::new(0.35, 0.86, 0.8),
        Mood::Cheerful => Vec3::new(1.0, 0.82, 0.35),
        Mood::Excited => Vec3::new(1.0, 0.45, 0.8),
        Mood::Curious => Vec3::new(0.35, 0.8, 1.0),
        Mood::Sleepy => Vec3::new(0.45, 0.45, 1.0),
        Mood::Lonely => Vec3::new(0.55, 0.66, 0.95),
        Mood::Bored => Vec3::new(0.66, 0.62, 0.82),
        Mood::Grumpy => Vec3::new(1.0, 0.4, 0.25),
    }
}

// Beats per second of the aura in each mood, before energy.
fn beat(mood: Mood) -> f32 {
    match mood {
        Mood::Excited => 2.2,
        Mood::Grumpy => 1.8,
        Mood::Cheerful => 1.4,
        Mood::Sleepy => 0.5,
        _ => 1.0,
    }
}

/// The local hour of the day, from 0 to 24.
pub fn local_hour() -> f32 {
    local_time().1
}

fn local_time() -> (jiff::civil::Date, f32) {
    let now = jiff::Zoned::now();
    (
        now.date(),
        f32::from(now.hour()) + f32::from(now.minute()) / 60.0 + f32::from(now.second()) / 3600.0,
    )
}

/// Starts the world from a saved life, catching up on the time the app was
/// closed: the plant grew thirsty and the ghost lonely while nobody came.
///
/// Saved values are kept within their ranges and the furniture stays where
/// the room puts it, so a hand-edited or damaged save can't break the world.
pub fn restore(world: &mut World, save: &Save, now: i64) -> Journal {
    world.away = None;
    let name: String = save
        .name
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(24)
        .collect();
    if !name.is_empty() {
        world.ghost.name = name;
    }
    let needs = save.needs;
    world.ghost.needs = Needs {
        energy: needs.energy.clamp(0.0, 1.0),
        fun: needs.fun.clamp(0.0, 1.0),
        company: needs.company.clamp(0.0, 1.0),
    };
    world.plant = save.plant.sanitized(world.plant.position);
    world.seedlings = save
        .seedlings
        .iter()
        .take(crate::world::garden::SPOTS.len())
        .cloned()
        .map(crate::world::Seedling::sanitized)
        .collect();
    world.lamp.on = save.lamp_on;
    world.ghost.goal = save.goal.clone();
    world.life = save.life.clone().sanitized();
    world
        .life
        .nature
        .get_or_insert_with(|| Nature::from_birth(save.born));
    let timezone = jiff::tz::TimeZone::system();
    let local = jiff::Timestamp::from_second(save.last_seen)
        .ok()
        .zip(jiff::Timestamp::from_second(now).ok())
        .map(|(left, now)| {
            (
                left.to_zoned(timezone.clone()).datetime(),
                now.to_zoned(timezone.clone()).datetime(),
            )
        });
    world.birthday = jiff::Timestamp::from_second(save.born)
        .ok()
        .map(|born| born.to_zoned(timezone).date());
    let seed = (save.born as u64).rotate_left(23) ^ save.last_seen as u64;
    let story = local.map(|(left, now)| {
        world.date = now.date();
        world.hour = f32::from(now.hour())
            + f32::from(now.minute()) / 60.0
            + f32::from(now.second()) / 3600.0;
        let story = crate::world::away::story(left, now, seed, &world.life, world.birthday);
        world.begin_day();
        story.write_diary(&mut world.life, left.date(), now.date());
        story
    });
    if let Some(room) = &save.room {
        world.restore_room(room);
    }
    let away = now.saturating_sub(save.last_seen).max(0) as f32;
    world.life.miss_at(away, world.hour);
    world.elapse_cold(f64::from(away), false);
    world.age_garden(away);
    world
        .ghost
        .needs
        .change(0.0, 0.0, -(away / 3600.0 * 0.1).min(0.5));
    if away > 1800.0 {
        world
            .ghost
            .needs
            .change(0.0, -(away / 86_400.0 * 0.15).min(0.5), 0.0);
        if let Some(story) = story.filter(|story| !story.happenings.is_empty()) {
            if story.rested {
                world.ghost.needs.energy = world.ghost.needs.energy.max(0.8);
            }
            world.remember_away(story, seed);
        }
        if now.saturating_sub(save.born) >= 86_400 {
            world.write_return_letter(away);
        }
        world.record_return(away);
    }
    world.deliver_replies(true, 0.0);
    world.restore_wardrobe();
    Journal {
        memories: save.memories.clone(),
        conversation: save.conversation.clone(),
    }
}

/// Brings back the rest of the household from a save made `now`, after
/// `restore` brought back the creature it is mostly about. Each housemate
/// catches up on the time away by itself. A life saved before there were
/// households gains two newcomers at the door.
pub fn restore_household(world: &mut World, save: &Save, now: i64) {
    let household = &save.household;
    world.ghost.id = household.id;
    world.reserve_id(household.id);
    world.ghost.species = household.species;
    world.ghost.genome = household.genome.sanitized(household.species);
    let timezone = jiff::tz::TimeZone::system();
    let local = |seconds: i64| {
        jiff::Timestamp::from_second(seconds)
            .ok()
            .map(|stamp| stamp.to_zoned(timezone.clone()))
    };
    let away = now.saturating_sub(save.last_seen).max(0) as f32;
    for housemate in &household.housemates {
        let birthday = local(housemate.born).map(|zoned| zoned.date());
        let Some(id) = world.welcome_back(
            housemate.id,
            &housemate.name,
            housemate.species,
            housemate.genome,
            birthday,
            housemate.needs,
            housemate.goal.clone(),
            housemate.personal.clone(),
        ) else {
            continue;
        };
        let left = local(save.last_seen).map(|zoned| zoned.datetime());
        let today = local(now).map(|zoned| zoned.datetime());
        let seed = (housemate.born as u64).rotate_left(17) ^ save.last_seen as u64;
        world.with_creature(id, |world| {
            if let (Some(left), Some(today)) = (left, today) {
                let story =
                    crate::world::away::story(left, today, seed, &world.life, world.birthday);
                world.begin_day();
                story.write_diary(&mut world.life, left.date(), today.date());
            }
            let routine = (world.life.arrivals, world.life.routine_days);
            world.life.miss_at(away, world.hour);
            (world.life.arrivals, world.life.routine_days) = routine;
            world.elapse_cold(f64::from(away), false);
            world
                .ghost
                .needs
                .change(0.0, 0.0, -(away / 3600.0 * 0.1).min(0.5));
            if away > 1800.0 {
                world
                    .ghost
                    .needs
                    .change(0.0, -(away / 86_400.0 * 0.15).min(0.5), 0.0);
                world.record_return(away);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    /// Prints what a household does over a simulated afternoon and night,
    /// for watching how they live together.
    #[test]
    #[ignore]
    fn watch_a_household() {
        use crate::world::{EventKind, Species};
        let hours: f64 = std::env::var("WATCH_HOURS")
            .ok()
            .and_then(|hours| hours.parse().ok())
            .unwrap_or(2.0);
        let mut world = World::new(11, 15.0);
        world.found_household(&[Species::Fox, Species::Bunny, Species::Owl, Species::Robot]);
        let mind = Mind::new(&Config::default(), Journal::default(), true);
        let mut app = App::new(
            world,
            mind,
            Config::default(),
            Style::Braille,
            ColorMode::Plain,
        );
        let mut last: Vec<(u32, String)> = Vec::new();
        let mut counts: std::collections::BTreeMap<String, u32> = Default::default();
        let steps = (hours * 3600.0 / 0.1) as usize;
        for step in 0..steps {
            app.world.hour = (15.0 + step as f32 * 0.1 / 3600.0) % 24.0;
            app.world.viewer.active_at = if step % 6000 < 3000 {
                app.world.time
            } else {
                app.world.viewer.active_at
            };
            app.advance(0.1);
            for id in app.world.roster() {
                let ghost = app.world.creature(id).unwrap();
                let doing = if ghost.asleep > 0.5 {
                    "asleep".to_owned()
                } else {
                    format!(
                        "{:?}{}",
                        ghost.plan.activity,
                        ghost
                            .plan
                            .pastime
                            .map_or(String::new(), |p| format!("/{p:?}"))
                    )
                };
                match last.iter_mut().find(|(other, _)| *other == id) {
                    Some((_, before)) if *before == doing => {}
                    Some((_, before)) => {
                        *before = doing.clone();
                        *counts.entry(doing.clone()).or_default() += 1;
                        if std::env::var_os("WATCH_VERBOSE").is_some() {
                            println!(
                                "{:>7.0}s h{:>5.2} {} {} at {:.1?} body {:.2?}",
                                app.world.time,
                                app.world.hour,
                                ghost.name,
                                doing,
                                ghost.position,
                                ghost.body()
                            );
                        }
                    }
                    None => last.push((id, doing)),
                }
            }
        }
        println!("{counts:#?}");
        for id in app.world.roster() {
            app.world.with_creature(id, |world| {
                let together = world.events.iter().filter(|event| matches!(event.kind, EventKind::Together { .. })).count();
                println!(
                    "{} the {:?}: bond {:.2}, fondness {:?}, together events kept {together}, energy {:.2} fun {:.2} company {:.2}, at {:.1?}",
                    world.ghost.name,
                    world.ghost.species,
                    world.life.bond,
                    world.life.fondness.iter().map(|fondness| (fondness.name.clone(), (fondness.warmth * 100.0).round() / 100.0, fondness.times)).collect::<Vec<_>>(),
                    world.ghost.needs.energy,
                    world.ghost.needs.fun,
                    world.ghost.needs.company,
                    world.ghost.position,
                );
            });
        }
    }

    #[test]
    fn quitting_echoes_only_a_promise_made_this_session_in_its_original_language() {
        let mut app = app();
        app.world.ghost.name = "Pip".to_owned();
        app.world.hear("see you tomorrow");
        assert_eq!(app.farewell(), "Pip will be waiting for you tomorrow.");
        app.world.hear("내일 봐");
        assert_eq!(app.farewell(), "Pip이 내일 기다릴게요.");
        app.world.ghost.name = "뽀삐".to_owned();
        assert_eq!(app.farewell(), "뽀삐가 내일 기다릴게요.");
        let mut loaded = World::new(4, 12.0);
        loaded.life = app.world.life.clone();
        assert!(loaded.promised_this_session().is_none());
    }

    #[test]
    fn restoring_a_promised_trip_protects_the_bond_writes_a_letter_and_resolves_on_arrival_once() {
        let mut original = World::new(3, 12.0);
        original.hear("back in a week");
        original.life.bond = 0.6;
        original.life.closest = 0.6;
        let mut save = garden_save(&original);
        save.born = local_seconds("2026-05-01T12:00:00");
        let mut restored = World::new(4, 12.0);
        restore(&mut restored, &save, local_seconds("2026-06-28T12:00:00"));
        assert_eq!(restored.life.bond, save.life.bond);
        assert!(restored.life.promise.is_some());
        assert!(
            restored
                .life
                .letters
                .last()
                .unwrap()
                .lines
                .iter()
                .any(|line| line == "I hope your trip was lovely.")
        );
        restored.notice_viewer();
        restored.step(0.1);
        assert!(restored.life.promise.is_none());
        assert_eq!(restored.life.total.promises_kept, 1);
        restored.notice_viewer();
        restored.step(0.1);
        assert_eq!(restored.life.total.promises_kept, 1);
        assert!(restored.life.bond > save.life.bond);
    }

    fn letter(day: &str, read: bool) -> crate::world::mail::Letter {
        crate::world::mail::Letter {
            day: day.to_owned(),
            lines: vec!["Dear you,".to_owned(), "Love, Pip".to_owned()],
            read,
            ..Default::default()
        }
    }

    #[test]
    fn sent_letters_follow_the_written_language_even_after_other_language_conversations() {
        for (text, established, expected) in [
            ("Hello!", 1.0, false),
            ("안녕!", 0.0, true),
            ("♥", 1.0, true),
        ] {
            let mut app = app();
            app.world.life.korean = established;
            app.input(Input::Key(Key::Character('e')));
            app.input(Input::Paste(text.to_owned()));
            app.input(Input::Key(Key::Control('s')));
            assert_eq!(app.world.life.received[0].korean, expected);
        }
    }

    #[test]
    fn the_life_panel_scrolls_when_taller_than_the_screen_and_other_keys_close_it() {
        let mut app = app();
        app.screen_size = (90, 12);
        app.input(Input::Key(Key::Character('l')));
        assert_eq!(app.mode, Mode::Life);
        let (total, page) = ui::life_extent(&app, 90, 12);
        assert!(total > page, "{total} {page}");
        app.input(Input::Key(Key::Down));
        assert_eq!(app.life_scroll, 1);
        app.input(Input::Key(Key::Up));
        app.input(Input::Key(Key::Up));
        assert_eq!(app.life_scroll, 0);
        app.input(Input::Key(Key::End));
        assert_eq!(app.life_scroll, total - page);
        app.input(Input::Key(Key::PageDown));
        assert_eq!(app.life_scroll, total - page);
        app.input(Input::Key(Key::Home));
        assert_eq!(app.life_scroll, 0);
        assert_eq!(app.mode, Mode::Life);
        app.input(Input::Key(Key::Escape));
        assert_eq!(app.mode, Mode::Watching);
        app.input(Input::Key(Key::Character('l')));
        assert_eq!(app.life_scroll, 0);
        assert_eq!(app.input(Input::Key(Key::Character('q'))), Response::Quit);
    }

    #[test]
    fn writing_routes_q_enter_paste_send_and_cancel_and_keeps_the_unsaved_draft() {
        let mut app = app();
        app.input(Input::Key(Key::Character('e')));
        assert_eq!(app.mode, Mode::Writing);
        assert!(!app.wants_mouse());
        assert_eq!(
            app.input(Input::Key(Key::Character('q'))),
            Response::Continue
        );
        app.input(Input::Key(Key::Enter));
        app.input(Input::Paste("안녕\nfriend\u{1b}".to_owned()));
        assert_eq!(app.draft.text(), "q\n안녕\nfriend");
        app.input(Input::Key(Key::Escape));
        assert_eq!(app.mode, Mode::Watching);
        assert!(app.world.life.received.is_empty());
        // While watching, `w` still turns the view; the letters reopen it.
        app.input(Input::Key(Key::Character('w')));
        assert_eq!(app.mode, Mode::Watching);
        app.input(Input::Key(Key::Character('e')));
        assert_eq!(app.mode, Mode::Writing);
        assert_eq!(app.draft.text(), "q\n안녕\nfriend");
        app.input(Input::Key(Key::Control('s')));
        assert_eq!(app.mode, Mode::Watching);
        assert!(app.draft.is_empty());
        assert_eq!(app.world.life.received[0].lines, ["q", "안녕", "friend"]);
        assert!(app.world.life.received[0].korean);
        // Before the ghost writes back, its letters open on the viewer's own.
        app.input(Input::Key(Key::Character('e')));
        assert_eq!(app.mode, Mode::Letter);
        assert!(app.sent_letters);
        assert_eq!(app.letter_index, 0);
        app.input(Input::Key(Key::Character('w')));
        app.input(Input::Key(Key::Control('s')));
        assert_eq!(app.mode, Mode::Writing);
        assert_eq!(app.world.life.received.len(), 1);
        app.input(Input::Key(Key::Escape));
        app.world.life.letters.push(letter("2026-06-21", true));
        app.input(Input::Key(Key::Character('e')));
        app.input(Input::Key(Key::Character('v')));
        assert!(app.sent_letters);
        let count = app.world.life.total.letters_read;
        app.input(Input::Key(Key::Right));
        assert_eq!(app.world.life.total.letters_read, count);
        app.input(Input::Key(Key::Character('w')));
        assert_eq!(app.mode, Mode::Writing);
        assert!(!app.wants_mouse());
    }

    #[test]
    fn the_letter_key_opens_the_newest_unread_letter_and_closes_with_text_panel_keys() {
        for close in [Key::Character('e'), Key::Escape, Key::Enter] {
            let mut app = app();
            app.world.life.letters = vec![letter("2026-06-20", false), letter("2026-06-21", true)];
            assert_eq!(
                app.input(Input::Key(Key::Character('e'))),
                Response::Continue
            );
            assert_eq!(app.mode, Mode::Letter);
            assert_eq!(app.letter_index, 0);
            assert!(!app.wants_mouse());
            app.frame(100, 32);
            assert!(app.world.life.letters[0].read);
            assert_eq!(app.world.life.total.letters_read, 1);
            let plan = app.world.ghost.plan.clone();
            let camera = app.camera;
            let paused = app.paused;
            for key in [
                Key::Character(' '),
                Key::Character('g'),
                Key::Character('j'),
                Key::Up,
                Key::Home,
            ] {
                app.input(Input::Key(key));
            }
            app.input(Input::Paste("hello".to_owned()));
            assert_eq!(app.world.ghost.plan, plan);
            assert_eq!(app.camera, camera);
            assert_eq!(app.paused, paused);
            assert_eq!(app.mode, Mode::Letter);
            app.input(Input::Key(close));
            assert_eq!(app.mode, Mode::Watching);
            assert!(app.wants_mouse());
            app.input(Input::Key(Key::Character('e')));
            assert_eq!(app.letter_index, 1);
            assert_eq!(app.world.life.total.letters_read, 1);
            assert_eq!(app.input(Input::Key(Key::Character('q'))), Response::Quit);
        }
    }

    #[test]
    fn left_and_right_page_through_letters_and_mark_each_unread_one_once() {
        let mut app = app();
        app.world.life.letters = vec![
            letter("2026-06-19", false),
            letter("2026-06-20", false),
            letter("2026-06-21", false),
        ];
        app.input(Input::Key(Key::Character('e')));
        assert_eq!(app.letter_index, 2);
        for _ in 0..4 {
            app.input(Input::Key(Key::Left));
        }
        assert_eq!(app.letter_index, 0);
        for _ in 0..4 {
            app.input(Input::Key(Key::Right));
        }
        assert_eq!(app.letter_index, 2);
        assert!(app.world.life.letters.iter().all(|letter| letter.read));
        assert_eq!(app.world.life.total.letters_read, 3);
        assert_eq!(
            app.world
                .events
                .iter()
                .filter(|event| event.kind == EventKind::ReadLetter)
                .count(),
            3
        );
    }

    #[test]
    fn up_and_down_scroll_a_letter_and_paging_starts_the_next_at_its_top() {
        let mut app = app();
        app.world.life.letters = vec![letter("2026-06-20", false), letter("2026-06-21", false)];
        app.input(Input::Key(Key::Character('e')));
        assert_eq!(app.letter_scroll, 0);
        app.input(Input::Key(Key::Up));
        assert_eq!(app.letter_scroll, 0);
        app.input(Input::Key(Key::Down));
        app.input(Input::Key(Key::Down));
        assert_eq!(app.letter_scroll, 2);
        assert_eq!(app.mode, Mode::Letter);
        app.input(Input::Key(Key::Left));
        assert_eq!(app.letter_scroll, 0);
        // A tiny screen still draws without trouble at any scroll.
        app.letter_scroll = 40;
        app.frame(12, 6);
        app.frame(60, 8);
    }

    #[test]
    fn the_letter_key_opens_a_writing_page_when_there_are_no_letters() {
        let mut app = app();
        app.input(Input::Key(Key::Character('e')));
        assert_eq!(app.mode, Mode::Writing);
        assert_eq!(app.notice(), None);
        assert!(!app.wants_mouse());
    }

    #[test]
    fn clicking_an_unread_envelope_opens_it_without_dropping_a_mote() {
        let mut app = app();
        app.world.ghost.position = Vec3::new(-1.0, 1.5, 0.0);
        app.world.life.letters.push(letter("2026-06-21", false));
        let point = app
            .view
            .unwrap()
            .project(crate::world::mail::POSITION)
            .unwrap();
        app.click(point);
        assert_eq!(app.mode, Mode::Letter);
        assert!(app.world.life.letters[0].read);
        assert!(app.world.motes.is_empty());
        app.input(Input::Key(Key::Escape));
        app.click(point);
        assert_eq!(app.mode, Mode::Watching);
        assert_eq!(app.world.motes.len(), 1);
    }

    #[test]
    fn the_ghost_waits_until_after_the_letter_closes_to_ask_about_it_without_waking() {
        for (korean, asleep) in [(false, false), (true, false), (false, true)] {
            let mut app = app();
            app.world.life.korean = if korean { 1.0 } else { 0.0 };
            if asleep {
                app.world.hour = 23.0;
                app.world.tuck_in();
            }
            app.world.life.letters.push(letter("2026-06-21", false));
            app.input(Input::Key(Key::Character('e')));
            app.advance(3.0);
            assert!(
                !app.world
                    .ghost
                    .speech
                    .as_ref()
                    .is_some_and(|speech| speech.text == "Did you like it?"
                        || speech.text == "마음에 들었어?")
            );
            app.input(Input::Key(Key::Escape));
            app.advance(1.1);
            if asleep {
                assert!(app.world.ghost.asleep > 0.5);
            } else {
                assert_eq!(
                    app.world.ghost.speech.as_ref().unwrap().text,
                    if korean {
                        "마음에 들었어?"
                    } else {
                        "Did you like it?"
                    }
                );
            }
        }
    }

    #[test]
    fn restore_writes_one_letter_per_long_return_and_keeps_it_in_the_save() {
        for hours in [5, 6, 48] {
            let mut world = World::new(11, 12.0);
            let save = garden_save(&world);
            let returned = save.last_seen + hours * 3600;
            restore(&mut world, &save, returned);
            let expected = usize::from(hours >= 6);
            assert_eq!(world.life.letters.len(), expected);
            let mut app = App::new(
                world,
                Mind::new(&Config::default(), Journal::default(), true),
                Config::default(),
                Style::Ascii,
                ColorMode::Plain,
            );
            // Only the letter about time away; a Sunday's weekly letter is
            // counted elsewhere.
            app.world.life.weekly_letter = Some(app.world.date.to_string());
            app.advance(10.0);
            assert_eq!(app.world.life.letters.len(), expected);
            let mut saved = garden_save(&app.world);
            saved.last_seen = returned;
            let saved: Save =
                serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
            restore(&mut app.world, &saved, returned + 30);
            assert_eq!(app.world.life.letters.len(), expected);
        }
    }

    #[test]
    fn restoring_delivers_a_due_penpal_reply_once_and_resumes_an_unread_paper() {
        let mut world = World::new(11, 12.0);
        world.receive_letter("How are you?", false);
        world.ghost.position = crate::world::penpal::APPROACH;
        world.step(0.1);
        world.step(6.1);
        world.ghost.position = world.ghost.plan.target;
        world.step(0.1);
        assert!(world.life.received[0].read);
        let save = garden_save(&world);
        let mut restored = World::new(11, 12.0);
        restore(&mut restored, &save, save.last_seen + 3600);
        assert!(restored.life.received[0].answered);
        assert_eq!(restored.life.letters.len(), 1);
        assert!(restored.life.letters[0].reply_to.is_some());
        let mut saved_again = garden_save(&restored);
        saved_again.last_seen = save.last_seen + 3600;
        restore(&mut restored, &saved_again, saved_again.last_seen + 1);
        assert_eq!(restored.life.letters.len(), 1);
        let mut waiting = World::new(11, 12.0);
        waiting.receive_letter("Hello!", false);
        let save = garden_save(&waiting);
        restore(&mut restored, &save, save.last_seen + 1);
        assert!(restored.waiting_letter().is_some());
        restored.ghost.plan.done = true;
        restored.step(0.1);
        assert!(restored.reading_viewer_letter());
    }

    #[test]
    fn restore_does_not_ask_a_ghost_under_a_day_old_to_write() {
        for birth in ["2026-06-21T02:00:00", "2026-06-20T23:00:00"] {
            let mut world = World::new(1, 12.0);
            let mut save = garden_save(&world);
            save.born = local_seconds(birth);
            restore(&mut world, &save, local_seconds("2026-06-21T20:00:00"));
            assert!(world.life.letters.is_empty());
        }
    }

    #[test]
    fn clicking_the_music_box_winds_it_without_dropping_a_mote_or_waking_a_sleeper() {
        let mut app = app();
        let point = app
            .view
            .unwrap()
            .project(crate::world::music::POSITION + Vec3::Y * 0.13)
            .unwrap();
        app.click(point);
        assert!(app.world.music.is_some());
        assert_eq!(app.world.ghost.plan.activity, Activity::Music);
        assert!(app.world.motes.is_empty());
        app.world.hour = 23.0;
        app.world.tuck_in();
        let plan = app.world.ghost.plan.clone();
        app.click(point);
        assert!(app.world.music.as_ref().unwrap().lullaby);
        assert_eq!(app.world.ghost.asleep, 1.0);
        assert_eq!(app.world.ghost.plan, plan);
        assert!(app.world.motes.is_empty());
    }

    #[test]
    fn a_click_goes_to_the_music_box_when_it_stands_in_front_of_the_ball() {
        let mut app = app();
        let view = app.view.unwrap();
        let target = crate::world::music::POSITION + Vec3::Y * 0.13;
        let point = view.project(target).unwrap();
        // The ball waits behind the box along the same ray.
        let behind = target + (target - view.eye).normalize() * 0.6;
        app.world.ball = Ball::at(behind.x, behind.z);
        app.world.ball.position.y = behind.y;
        app.click(point);
        assert!(app.world.music.is_some());
        assert_eq!(app.world.ball.velocity, Vec3::ZERO);
    }

    use super::*;
    use crate::store::{Config, Memory};
    use crate::world::{Activity, EventKind};

    #[test]
    fn restoring_keeps_todays_accessory_but_returns_yesterdays_feather() {
        use crate::world::{Accessory, Keepsake};
        let mut world = World::new(1, 12.0);
        world.date = jiff::civil::Date::constant(2026, 6, 21);
        world.step(0.0);
        world.life.keepsakes.push(Keepsake::Feather);
        world.life.wearing = Some(Accessory::Feather);
        world.wearing = world.life.wearing;
        let save = Save {
            version: Save::VERSION,
            name: "Boo".into(),
            born: local_seconds("2026-05-21T12:00:00"),
            last_seen: local_seconds("2026-06-21T12:00:00"),
            needs: world.ghost.needs,
            plant: world.plant,
            lamp_on: world.lamp.on,
            goal: None,
            memories: Vec::new(),
            conversation: Vec::new(),
            life: world.saved_life(),
            seedlings: Vec::new(),
            room: None,
            household: Default::default(),
        };
        let save: Save = serde_json::from_str(&serde_json::to_string(&save).unwrap()).unwrap();
        let mut restored = World::new(2, 12.0);
        restore(&mut restored, &save, save.last_seen + 60);
        assert_eq!(restored.wearing, Some(Accessory::Feather));
        restored.step(0.0);
        assert_eq!(restored.wearing, Some(Accessory::Feather));
        assert_eq!(restored.life.shelved_keepsakes().count(), 0);
        assert!(
            !restored
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::PutOn(_)))
        );
        restore(&mut restored, &save, local_seconds("2026-06-22T12:00:00"));
        assert!(restored.wearing.is_none());
        assert_eq!(restored.life.shelved_keepsakes().count(), 1);
    }

    #[test]
    fn the_focus_key_starts_and_ends_company_and_titles_change_only_each_minute() {
        let mut app = app();
        app.input(Input::Key(Key::Character('o')));
        assert_eq!(app.world.focus.as_ref().unwrap().length, 1500);
        assert_eq!(app.world.ghost.plan.activity, Activity::Company);
        let title = app.title();
        assert!(title.contains("focusing with you · 25 min left"));
        app.input(Input::Focus(false));
        app.world.step(30.0);
        assert!(app.world.viewer_present());
        assert_eq!(app.title(), title);
        app.world.step(31.0);
        assert!(app.title().contains("24 min left"));
        app.input(Input::Focus(true));
        app.input(Input::Key(Key::Character('o')));
        assert!(app.world.focus.is_none());
        assert_eq!(app.world.life.total.focus_minutes, 1);
        assert_eq!(app.world.life.total.focus_sessions, 0);
    }

    fn app() -> App {
        let world = World::new(1, 12.0);
        let mind = Mind::new(&Config::default(), Journal::default(), true);
        let mut app = App::new(
            world,
            mind,
            Config::default(),
            Style::Ascii,
            ColorMode::Plain,
        );
        app.frame(100, 32);
        app
    }

    fn fill_journal(app: &mut App) {
        app.mind.journal.memories = (0..80)
            .map(|number| Memory {
                at: 0,
                text: format!("Memory {number}"),
            })
            .collect();
    }

    fn type_text(app: &mut App, text: &str) {
        for character in text.chars() {
            app.input(Input::Key(Key::Character(character)));
        }
    }

    fn click(app: &mut App, column: usize, row: usize) {
        for action in [MouseAction::Press, MouseAction::Release] {
            app.input(Input::Mouse(Mouse {
                column,
                row,
                action,
            }));
        }
    }

    // The cell where a world point appears.
    fn cell_of(app: &App, position: Vec3) -> (usize, usize) {
        let (columns, rows) = app.area;
        let point = app.view.unwrap().project(position).unwrap();
        let pixel = Grid::new(columns, rows, CELL_ASPECT)
            .to_pixels()
            .transform_point2(point);
        (pixel.x as usize, pixel.y as usize)
    }

    #[test]
    fn clicking_the_swing_sends_the_ghost_out_and_shaking_the_tree_shoos_its_bird() {
        let mut yard = app();
        yard.world.ghost.needs.energy = 1.0;
        // Look at the yard, as when the ghost is out there.
        yard.camera.target = Vec3::new(4.3, 1.3, -0.7);
        yard.frame(100, 32);
        let (column, row) = cell_of(&yard, yard.world.swing_seat() + Vec3::Y * 0.3);
        click(&mut yard, column, row);
        assert_eq!(yard.world.ghost.plan.activity, Activity::Yard);
        assert_eq!(
            yard.world.yard_spot(),
            Some(crate::world::home::Spot::Swing)
        );

        let mut shaken = app();
        let mut bird =
            crate::world::visitor::Bird::new(-10.0, 1.0, &mut crate::world::Random::new(2));
        bird.perch = crate::world::Perch::Tree;
        shaken.world.visitors.bird = Some(bird);
        // The crown shows above the roof from farther back.
        shaken.camera.zoom = 1.0;
        shaken.camera.target = Vec3::new(0.0, 1.3, 0.0);
        shaken.frame(100, 32);
        let crown = crate::world::home::TREE_CROWN + Vec3::Y * 1.0;
        let (column, row) = cell_of(&shaken, crown);
        click(&mut shaken, column, row);
        assert!(shaken.world.tree_shake().is_some());
        assert!(
            shaken
                .world
                .events
                .iter()
                .any(|event| event.kind == EventKind::BirdFlewOff { startled: false })
        );
        assert!(shaken.world.motes.is_empty());
    }

    #[test]
    fn window_clicks_request_the_sash_and_sleeping_ghosts_ignore_them() {
        let mut app = app();
        let (column, row) = cell_of(&app, crate::world::WINDOW_CENTER);
        click(&mut app, column, row);
        assert_eq!(app.world.ghost.plan.activity, Activity::Air);
        assert_eq!(app.world.ghost.plan.window, Some(true));
        assert!(app.world.motes.is_empty());
        assert_eq!(app.doing(), "opening the window for fresh air");
        app.world.ghost.position = app.world.ghost.plan.target;
        app.world.step(1.2);
        assert!(app.world.window_open);
        app.world.ghost.position = Vec3::new(-1.2, 1.0, 0.5);
        click(&mut app, column, row);
        assert_eq!(app.world.ghost.plan.window, Some(false));
        assert_eq!(app.doing(), "closing the window");
        app.world.hour = 23.0;
        app.world.tuck_in();
        let plan = app.world.ghost.plan.clone();
        click(&mut app, column, row);
        assert_eq!(app.world.ghost.asleep, 1.0);
        assert_eq!(app.world.ghost.plan, plan);
        assert!(app.world.motes.is_empty());
    }

    #[test]
    fn a_full_moon_evening_brings_a_visiting_friend() {
        let mut app = (1..=6)
            .map(|seed| {
                let mut world = World::new(seed, 20.0);
                world.date = jiff::civil::Date::constant(2026, 9, 27);
                world.step(0.0);
                let mind = Mind::new(&Config::default(), Journal::default(), true);
                let mut app = App::new(
                    world,
                    mind,
                    Config::default(),
                    Style::Ascii,
                    ColorMode::Plain,
                );
                // Until the friend has come and played a little.
                for _ in 0..24000 {
                    app.advance(0.05);
                    if app.world.friend.is_some() && app.world.life.total.friend_games > 0 {
                        break;
                    }
                }
                app
            })
            .find(|app| app.world.friend.is_some() && app.world.life.total.friend_visits == 1)
            .expect("one of a few full-moon evenings brings the friend");
        assert!(app.world.life.total.friend_games > 0);
        let name = app.world.life.friend.as_ref().unwrap().name.clone();
        app.world.begin(Activity::Friend);
        assert!(app.title().contains(&format!("playing with {name}")));
        app.world.ghost.position = Vec3::new(-0.8, 1.1, 0.0);
        app.world.friend.as_mut().unwrap().position = Vec3::new(0.6, 1.4, 0.3);
        let view = Camera::default().view(Grid::cells(100, 32));
        app.view = Some(view);
        let point = view
            .project(app.world.friend.as_ref().unwrap().position)
            .unwrap();
        let bond = app.world.life.bond;
        app.click(point);
        assert_eq!(app.world.life.bond, bond);
        assert!(
            app.world
                .events
                .iter()
                .any(|event| event.kind == EventKind::PokedFriend)
        );
    }

    #[test]
    fn lines_edit_unicode_text_at_the_cursor() {
        let mut line = Line::default();
        line.insert("안녕 boo");
        line.edit(&Key::Home);
        line.edit(&Key::Delete);
        line.edit(&Key::End);
        line.edit(&Key::Backspace);
        line.insert("!\n");
        assert_eq!(line.text(), "녕 bo! ");
        assert_eq!(line.cursor(), 6);
        assert_eq!(line.take(), "녕 bo!");
        assert_eq!(line.text(), "");
    }

    #[test]
    fn a_huge_paste_is_cut_to_the_line_limit() {
        let mut line = Line::default();
        line.insert(&"ab".repeat(1_000_000));
        line.insert("more");
        assert_eq!(line.text().chars().count(), LINE_LIMIT);
        assert_eq!(line.cursor(), LINE_LIMIT);
    }

    #[test]
    fn talking_sends_the_message_to_the_ghost() {
        let mut app = app();
        app.input(Input::Key(Key::Enter));
        assert_eq!(app.mode, Mode::Talking);
        type_text(&mut app, "go play");
        app.input(Input::Key(Key::Enter));
        assert_eq!(app.mode, Mode::Watching);
        assert!(matches!(
            app.world.events.iter().rev().find(|event| matches!(event.kind, EventKind::Heard(_))),
            Some(event) if event.kind == EventKind::Heard("go play".into())
        ));
        // Its own words come first; the hint about Claude follows once.
        assert!(app.world.ghost.speech.is_some());
        assert_eq!(app.notice(), None);
        app.advance(HINT_AFTER as f32 + 0.1);
        assert!(app.notice().unwrap().contains("Claude key"));
        app.input(Input::Key(Key::Enter));
        type_text(&mut app, "hello again");
        app.input(Input::Key(Key::Enter));
        app.advance(10.0);
        app.advance(HINT_AFTER as f32 + 0.1);
        assert_eq!(app.notice(), None);
    }

    #[test]
    fn typed_letters_are_text_while_talking_not_commands() {
        let mut app = app();
        app.input(Input::Key(Key::Character('t')));
        type_text(&mut app, "quit");
        assert_eq!(app.mode, Mode::Talking);
        assert_eq!(app.talk.text(), "quit");
        assert_eq!(app.input(Input::Key(Key::Escape)), Response::Continue);
        assert_eq!(app.mode, Mode::Watching);
        assert_eq!(app.input(Input::Key(Key::Character('q'))), Response::Quit);
    }

    #[test]
    fn saving_preserves_bird_trust_the_shelf_the_sill_and_a_carried_gift() {
        use crate::world::Keepsake;

        let directory =
            std::env::temp_dir().join(format!("wave-ascii-app-bird-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let paths = Paths::in_directory(&directory);
        let mut app = app();
        app.paths = Some(paths.clone());
        app.world.life.bird_trust = 0.72;
        app.world.life.keepsakes = vec![Keepsake::Feather, Keepsake::Acorn];
        app.world.life.sill = Some(Keepsake::Pebble);
        app.save().unwrap();
        let saved = crate::store::load_json::<Save>(&paths.save)
            .unwrap()
            .unwrap();
        let mut world = World::new(2, 12.0);
        restore(&mut world, &saved, saved.last_seen);
        assert_eq!(world.life.bird_trust, 0.72);
        assert_eq!(world.life.keepsakes, [Keepsake::Feather, Keepsake::Acorn]);
        assert_eq!(world.life.sill, Some(Keepsake::Pebble));

        app.world.life.sill = None;
        app.world.ghost.keepsake = Some(Keepsake::Pebble);
        app.save().unwrap();
        let saved = crate::store::load_json::<Save>(&paths.save)
            .unwrap()
            .unwrap();
        restore(&mut world, &saved, saved.last_seen);
        assert_eq!(
            world.life.keepsakes,
            [Keepsake::Feather, Keepsake::Acorn, Keepsake::Pebble]
        );
        assert_eq!(world.life.sill, None);
        assert_eq!(world.life.total.gifts, 1);

        let mut old = serde_json::to_value(&saved).unwrap();
        for field in ["bird_trust", "keepsakes", "sill"] {
            old["life"].as_object_mut().unwrap().remove(field);
        }
        let old: Save = serde_json::from_value(old).unwrap();
        assert_eq!(old.life.bird_trust, 0.0);
        assert!(old.life.keepsakes.is_empty());
        assert_eq!(old.life.sill, None);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn p_saves_the_last_watching_frame_and_posts_its_path() {
        let directory =
            std::env::temp_dir().join(format!("wave-ascii-app-postcard-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let paths = Paths::in_directory(&directory);
        let mut app = app();
        app.paths = Some(paths.clone());
        app.world.date = jiff::civil::Date::constant(2026, 10, 21);
        app.world.hour = 15.0 + 30.0 / 60.0 + 12.0 / 3600.0;
        app.world.begin(Activity::Window);
        let screen = app.frame(40, 12);
        // Inputs may arrive before the next draw. Keep the moment the viewer saw.
        app.world.hour = 17.0;
        app.world.begin(Activity::Play);
        app.input(Input::Key(Key::Character('p')));
        let path = paths.postcards().join("2026-10-21-153012.txt");
        assert_eq!(
            app.notice(),
            Some(format!("Saved a postcard: {}", postcard::display_path(&path)).as_str())
        );
        let saved = std::fs::read_to_string(&path).unwrap();
        let caption = "Boo · 21 October 2026, 15:30 · gazing out of the window\n";
        let displayed = screen.postcard_text();
        let without_status: String = displayed.split_inclusive('\n').take(11).collect();
        assert_eq!(saved, format!("{without_status}{caption}"));
        assert!(
            std::fs::read_to_string(path.with_extension("ans"))
                .unwrap()
                .ends_with(&format!("{}\x1b[0m\n", caption.trim_end()))
        );

        app.mode = Mode::Talking;
        app.input(Input::Key(Key::Character('p')));
        assert_eq!(app.talk.text(), "p");
        assert_eq!(std::fs::read_dir(paths.postcards()).unwrap().count(), 2);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn shift_p_poses_the_ghost_and_takes_the_picture_after_its_smile() {
        let directory =
            std::env::temp_dir().join(format!("wave-ascii-app-portrait-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let paths = Paths::in_directory(&directory);
        let mut app = app();
        app.paths = Some(paths.clone());
        app.world.begin(Activity::Explore);
        app.frame(40, 12);
        app.input(Input::Key(Key::Character('P')));
        assert_eq!(app.notice(), Some("Say cheese…"));
        assert_eq!(app.world.ghost.plan.activity, Activity::Visit);
        assert!(!paths.postcards().exists());
        // A second press while it smiles takes nothing extra.
        app.input(Input::Key(Key::Character('P')));
        for _ in 0..30 {
            app.advance(0.05);
            app.frame(40, 12);
        }
        assert!(!paths.postcards().exists());
        for _ in 0..12 {
            app.advance(0.05);
            app.frame(40, 12);
        }
        assert_eq!(std::fs::read_dir(paths.postcards()).unwrap().count(), 2);
        assert_eq!(app.world.life.total.portraits, 1);

        // Opening a panel during the smile waits for the room to be seen.
        app.input(Input::Key(Key::Character('P')));
        app.input(Input::Key(Key::Character('j')));
        for _ in 0..50 {
            app.advance(0.05);
            app.frame(40, 12);
        }
        assert_eq!(app.world.life.total.portraits, 1);
        app.input(Input::Key(Key::Escape));
        app.frame(40, 12);
        assert_eq!(app.world.life.total.portraits, 2);
        app.input(Input::Key(Key::Character('p')));
        assert_eq!(app.world.life.total.photos, 3);
        assert_eq!(app.world.life.total.portraits, 2);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn p_reports_missing_saved_lives_and_file_errors() {
        let mut app = app();
        app.input(Input::Key(Key::Character('p')));
        assert_eq!(app.notice(), Some("Postcards need a saved life"));

        let directory = std::env::temp_dir().join(format!(
            "wave-ascii-app-postcard-error-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let paths = Paths::in_directory(&directory);
        std::fs::write(paths.postcards(), "a file is in the way").unwrap();
        app.paths = Some(paths);
        app.frame(20, 8);
        app.input(Input::Key(Key::Character('p')));
        assert!(
            app.notice()
                .unwrap()
                .starts_with("Couldn't save a postcard: ")
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    fn album_app(label: &str) -> (App, Paths, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "wave-ascii-app-album-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let paths = Paths::in_directory(&directory);
        let mut app = app();
        app.paths = Some(paths.clone());
        app.world.events.clear();
        app.world.events.push(crate::world::Event {
            time: app.world.time,
            kind: EventKind::Milestone("A moment to keep.".to_owned()),
        });
        (app, paths, directory)
    }

    #[test]
    fn album_captures_wait_for_an_unpaused_watching_frame() {
        for (label, mode, paused) in [
            ("help", Mode::Help, false),
            ("journal", Mode::Journal, false),
            ("life", Mode::Life, false),
            ("book", Mode::Album, false),
            ("settings", Mode::Settings, false),
            ("talking", Mode::Talking, false),
            ("paused", Mode::Watching, true),
        ] {
            let (mut app, paths, directory) = album_app(label);
            app.frame(24, 10);
            assert!(app.album.pages.is_empty());
            app.world.time += 2.0;
            app.mode = mode;
            app.paused = paused;
            app.advance(0.0);
            app.frame(24, 10);
            assert!(app.album.pages.is_empty(), "{label}");
            assert!(!paths.album().exists());
            app.paused = false;
            app.mode = Mode::Watching;
            app.frame(24, 10);
            assert_eq!(app.album.pages.len(), 1, "{label}");
            assert_eq!(app.album.pages[0].caption, "A moment to keep.");
            assert_eq!(Album::load(&paths, mind::now()).pages, app.album.pages);
            std::fs::remove_dir_all(directory).unwrap();
        }
    }

    #[test]
    fn album_captures_expire_while_paused_or_reading_and_need_a_saved_life() {
        for (label, mode, paused) in [
            ("expires-paused", Mode::Watching, true),
            ("expires-panel", Mode::Album, false),
        ] {
            let (mut app, paths, directory) = album_app(label);
            app.mode = mode;
            app.paused = paused;
            app.advance(61.0);
            app.mode = Mode::Watching;
            app.paused = false;
            app.world.time += 2.0;
            app.frame(24, 10);
            assert!(app.album.pages.is_empty());
            assert!(!paths.album().exists());
            let _ = std::fs::remove_dir_all(directory);
        }
        let mut app = app();
        app.world.events.push(crate::world::Event {
            time: app.world.time,
            kind: EventKind::Milestone("A moment without a save.".to_owned()),
        });
        app.advance(3.0);
        app.frame(24, 10);
        assert!(app.album.pages.is_empty());
    }

    #[test]
    fn album_keys_flip_jump_close_and_quit_and_a_still_turns_the_view() {
        let mut app = app();
        for captured in (0..4).rev() {
            app.album.pages.push(crate::album::Page::new(
                &Canvas::blank(2, 2),
                &app.world,
                captured,
                "Kept.".to_owned(),
            ));
        }
        app.input(Input::Key(Key::Character('b')));
        assert_eq!(app.mode, Mode::Album);
        assert!(!app.wants_mouse());
        for (columns, rows) in [(0, 0), (1, 1), (12, 4), (100, 32)] {
            app.frame(columns, rows);
        }
        let yaw = app.camera.yaw;
        for (key, expected) in [
            (Key::Left, 0),
            (Key::Right, 1),
            (Key::Right, 2),
            (Key::Home, 0),
            (Key::End, 3),
            (Key::Right, 3),
            (Key::Left, 2),
        ] {
            assert_eq!(app.input(Input::Key(key)), Response::Continue);
            assert_eq!(app.album.index, expected);
        }
        app.input(Input::Key(Key::Character('a')));
        assert_eq!(app.mode, Mode::Album);
        assert_eq!(app.camera.yaw, yaw);
        assert_eq!(app.input(Input::Key(Key::Character('q'))), Response::Quit);
        for key in [Key::Escape, Key::Character('b')] {
            app.input(Input::Key(key));
            assert_eq!(app.mode, Mode::Watching);
            assert!(app.wants_mouse());
            app.input(Input::Key(Key::Character('b')));
            assert_eq!(app.album.index, 0);
        }
        app.input(Input::Key(Key::Escape));
        app.input(Input::Key(Key::Character('a')));
        assert!(app.camera.yaw < yaw);
    }

    #[test]
    fn an_empty_album_opens_closes_and_handles_all_navigation_keys() {
        let mut app = app();
        app.input(Input::Key(Key::Character('b')));
        assert_eq!(app.mode, Mode::Album);
        for key in [Key::Left, Key::Right, Key::Home, Key::End] {
            assert_eq!(app.input(Input::Key(key)), Response::Continue);
            assert_eq!(app.album.index, 0);
        }
        for (columns, rows) in [(0, 0), (1, 1), (12, 4), (100, 32)] {
            app.frame(columns, rows);
        }
        assert_eq!(app.input(Input::Key(Key::Character('q'))), Response::Quit);
        assert_eq!(app.input(Input::Key(Key::Escape)), Response::Continue);
        assert_eq!(app.mode, Mode::Watching);
        app.input(Input::Key(Key::Character('b')));
        app.input(Input::Key(Key::Character('b')));
        assert_eq!(app.mode, Mode::Watching);
    }

    #[test]
    fn the_title_follows_the_name_activity_and_sleep() {
        let mut app = app();
        for (activity, doing) in [
            (Activity::Chase, "chasing a light mote"),
            (Activity::Window, "gazing out of the window"),
            (Activity::Rest, "resting on the cushion"),
        ] {
            app.world.begin(activity);
            assert_eq!(app.title(), format!("Boo · {doing}"));
        }
        app.world.ghost.asleep = 1.0;
        assert_eq!(app.title(), "Boo · asleep on the cushion");
        app.world.ghost.name = "Pip".into();
        assert_eq!(app.title(), "Pip · asleep on the cushion");
        // A name can't end the title's escape sequence and send its own.
        app.world.ghost.name = "Pip\u{7}\u{1b}[2J".into();
        assert_eq!(app.title(), "Pip[2J · asleep on the cushion");
    }

    #[test]
    fn keys_are_entered_by_typing_or_pasting_and_kept_in_the_settings() {
        let mut app = app();
        app.input(Input::Key(Key::Character('k')));
        app.input(Input::Paste("  sk-ant-test-key-12345\n".into()));
        app.input(Input::Key(Key::Tab));
        type_text(&mut app, "ts-key-67890-abcdef");
        app.input(Input::Key(Key::Enter));
        assert_eq!(app.mode, Mode::Watching);
        assert_eq!(
            app.config.anthropic_api_key.as_deref(),
            Some("sk-ant-test-key-12345")
        );
        assert_eq!(
            app.config.typesafe_api_key.as_deref(),
            Some("ts-key-67890-abcdef")
        );

        app.input(Input::Key(Key::Character('k')));
        type_text(&mut app, "-");
        app.input(Input::Key(Key::Enter));
        assert_eq!(app.config.anthropic_api_key, None);
        assert!(app.config.typesafe_api_key.is_some());
        // Started offline, the ghost stays offline whatever keys it gets.
        assert_eq!(app.mind.one, crate::mind::Status::Off);
    }

    #[test]
    fn the_model_effort_and_cache_change_while_the_app_runs() {
        let mut app = app();
        app.input(Input::Key(Key::Character('k')));
        for key in [
            Key::Down,
            Key::Down,
            Key::Left,
            Key::Left,
            Key::Down,
            Key::Right,
            Key::Down,
            Key::Left,
        ] {
            app.input(Input::Key(key));
        }
        app.input(Input::Key(Key::Enter));
        assert_eq!(app.mode, Mode::Watching);
        assert_eq!(app.config.claude_model, "claude-haiku-5-5");
        assert_eq!(
            app.config.claude_effort,
            Some(crate::mind::claude::Effort::Medium)
        );
        assert_eq!(
            app.config.cache,
            Some(crate::mind::claude::Cache::FiveMinutes)
        );
        let notice = app.notice().unwrap();
        assert!(
            notice.contains("claude-haiku-5-5") && notice.contains("effort medium"),
            "{notice}"
        );

        // Escape leaves the settings as they were.
        app.input(Input::Key(Key::Character('k')));
        app.input(Input::Key(Key::Down));
        app.input(Input::Key(Key::Down));
        app.input(Input::Key(Key::Right));
        app.input(Input::Key(Key::Escape));
        assert_eq!(app.config.claude_model, "claude-haiku-5-5");
    }

    #[test]
    fn clicking_the_ghost_pokes_it_and_clicking_the_floor_drops_a_mote() {
        let mut app = app();
        let (column, row) = cell_of(&app, app.world.ghost.position);
        click(&mut app, column, row);
        assert!(matches!(
            app.world.events.last().unwrap().kind,
            EventKind::Poked { count: 1 }
        ));

        let motes = app.world.motes.len();
        let (column, row) = cell_of(&app, Vec3::new(-0.3, 0.0, 0.9));
        click(&mut app, column, row);
        assert_eq!(app.world.motes.len(), motes + 1);
        assert_eq!(
            app.world.events.last().unwrap().kind,
            EventKind::DroppedMote
        );
    }

    #[test]
    fn the_game_key_challenges_and_digits_place_crosses_only_when_allowed() {
        use crate::world::Mark;

        let mut app = app();
        app.input(Input::Key(Key::Character('1')));
        assert!(app.world.game.is_none());
        app.world.ghost.asleep = 1.0;
        app.input(Input::Key(Key::Character('g')));
        assert!(app.world.game.is_none());
        assert_eq!(app.notice(), Some("It's asleep."));
        app.world.ghost.asleep = 0.0;
        app.input(Input::Key(Key::Character('g')));
        assert_eq!(app.world.game.as_ref().unwrap().first, Mark::Cross);
        app.input(Input::Key(Key::Character('1')));
        assert_eq!(app.world.game.as_ref().unwrap().cells[0], None);
        for _ in 0..200 {
            if app.world.game.as_ref().unwrap().grid_ready(app.world.time) {
                break;
            }
            app.world.step(0.05);
        }
        app.input(Input::Key(Key::Character('9')));
        assert_eq!(app.world.game.as_ref().unwrap().cells[8], Some(Mark::Cross));
        app.input(Input::Key(Key::Character('8')));
        assert_eq!(app.world.game.as_ref().unwrap().cells[7], None);
    }

    #[test]
    fn board_clicks_place_crosses_and_swallow_unavailable_cells_even_over_the_ghost() {
        use crate::world::Mark;

        let mut app = app();
        app.world.challenge();
        let point = app
            .world
            .game
            .as_ref()
            .unwrap()
            .cell_center(0)
            .extend(ROOM_MAX.z);
        let (column, row) = cell_of(&app, point);
        let motes = app.world.motes.len();
        click(&mut app, column, row);
        assert_eq!(app.world.motes.len(), motes);
        assert_eq!(app.world.game.as_ref().unwrap().cells[0], None);
        for _ in 0..200 {
            if app.world.game.as_ref().unwrap().grid_ready(app.world.time) {
                break;
            }
            app.world.step(0.05);
        }
        app.world.ghost.position = point - Vec3::Z * 0.3;
        app.world.events.clear();
        let motes = app.world.motes.len();
        click(&mut app, column, row);
        assert_eq!(app.world.game.as_ref().unwrap().cells[0], Some(Mark::Cross));
        click(&mut app, column, row);
        let other = app
            .world
            .game
            .as_ref()
            .unwrap()
            .cell_center(1)
            .extend(ROOM_MAX.z);
        let (column, row) = cell_of(&app, other);
        click(&mut app, column, row);
        assert_eq!(app.world.game.as_ref().unwrap().cells[1], None);
        assert_eq!(app.world.motes.len(), motes);
        assert!(
            !app.world.events.iter().any(|event| matches!(
                event.kind,
                EventKind::Poked { .. } | EventKind::DroppedMote
            ))
        );
        app.world.step(0.6);
        click(&mut app, column, row);
        assert_eq!(app.world.game.as_ref().unwrap().cells[1], None);
    }

    #[test]
    fn clicking_the_ball_flicks_it() {
        let mut app = app();
        let (column, row) = cell_of(&app, app.world.ball.position);
        click(&mut app, column, row);
        assert_eq!(app.world.events.last().unwrap().kind, EventKind::KickedBall);
        assert!(app.world.ball.velocity.length() > 1.0);
    }

    #[test]
    fn a_click_goes_to_the_ball_in_front_rather_than_a_block_behind_it() {
        let mut app = app();
        app.world.ball.position = Vec3::new(0.0, Ball::RADIUS, 0.5);
        let behind = app.world.blocks[0].position;
        let toward = (app.world.ball.position - app.view.unwrap().eye).normalize();
        app.world.blocks[0].position = app.world.ball.position + toward * 0.3;
        app.world.blocks[0].position.y = behind.y;
        let block = app.world.blocks[0].position;
        let (column, row) = cell_of(&app, app.world.ball.position);
        click(&mut app, column, row);
        assert_eq!(app.world.events.last().unwrap().kind, EventKind::KickedBall);
        assert_eq!(app.world.blocks[0].position, block);
        assert_eq!(app.world.blocks[0].velocity, Vec3::ZERO);
    }

    #[test]
    fn dragging_turns_the_view_instead_of_clicking() {
        let mut app = app();
        let yaw = app.camera.yaw;
        let events = app.world.events.len();
        for (column, action) in [
            (50, MouseAction::Press),
            (55, MouseAction::Drag),
            (60, MouseAction::Drag),
            (60, MouseAction::Release),
        ] {
            app.input(Input::Mouse(Mouse {
                column,
                row: 10,
                action,
            }));
        }
        assert!(app.camera.yaw < yaw);
        assert_eq!(app.world.events.len(), events);
    }

    #[test]
    fn dragging_across_the_ghost_strokes_it_instead_of_turning_the_view() {
        let mut app = app();
        let (yaw, pitch) = (app.camera.yaw, app.camera.pitch);
        let (column, row) = cell_of(&app, app.world.ghost.position);
        for (column, action) in [
            (column, MouseAction::Press),
            (column + 1, MouseAction::Drag),
            (column + 2, MouseAction::Drag),
            (column + 1, MouseAction::Drag),
            (column, MouseAction::Drag),
            (column + 1, MouseAction::Drag),
            (column + 2, MouseAction::Drag),
            (column + 2, MouseAction::Release),
        ] {
            app.input(Input::Mouse(Mouse {
                column,
                row,
                action,
            }));
        }
        assert_eq!((app.camera.yaw, app.camera.pitch), (yaw, pitch));
        assert!(app.world.petting() > 0.35, "{}", app.world.petting());
        assert!(
            app.world
                .events
                .iter()
                .any(|event| event.kind == EventKind::Petted)
        );
        assert!(
            !app.world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::Poked { .. }))
        );
        assert!(!app.world.viewer.pointer_is_lively());
    }

    #[test]
    fn the_pointer_over_the_room_touches_the_glass_and_the_eyes_follow_it() {
        let mut app = app();
        let (column, row) = cell_of(&app, Vec3::new(1.2, 1.9, ROOM_MAX.z));
        app.input(Input::Mouse(Mouse {
            column,
            row,
            action: MouseAction::Move,
        }));
        let pointer = app.world.viewer.pointer.unwrap();
        assert!((pointer.z - ROOM_MAX.z).abs() < 1e-3);
        let view = app.view.unwrap();
        let center = view.project(app.world.ghost.body());
        let handling = Handling {
            pointer: app.pointer,
            stroke_side: 0.0,
            glow: Vec3::ONE,
        };
        let (gaze, firmness) = portrait::gaze(&app.world, &view, center, &handling).unwrap();
        assert!(gaze.x > 0.0 && gaze.y > 0.0 && firmness > 0.5);
    }

    #[test]
    fn quick_mouse_moves_across_the_room_make_the_pointer_lively() {
        let mut app = app();
        for across in [-0.6, 0.6, -0.6] {
            let (column, row) = cell_of(&app, Vec3::new(across, 1.2, ROOM_MAX.z));
            app.mouse(Mouse {
                column,
                row,
                action: MouseAction::Move,
            });
            app.world.step(0.05);
        }
        assert!(app.world.viewer.liveliness > 0.8);
        assert!(app.world.viewer.pointer_is_lively());
        assert!(app.world.options().contains(&Activity::Pounce));
    }

    #[test]
    fn the_wheel_zooms_and_resizing_asks_for_a_full_redraw() {
        let mut app = app();
        app.input(Input::Mouse(Mouse {
            column: 10,
            row: 10,
            action: MouseAction::ScrollUp,
        }));
        assert!(app.camera.zoom > 1.0);
        assert_eq!(
            app.input(Input::Resize {
                columns: 80,
                rows: 24
            }),
            Response::Restyle
        );
    }

    #[test]
    fn resizing_jostles_the_box_only_when_watching_a_known_size() {
        let mut app = app();
        let resize = |columns, rows| Input::Resize { columns, rows };
        app.screen_size = (0, 0);
        app.input(resize(80, 24));
        assert_eq!(app.world.jostle.velocity, Vec3::ZERO);
        app.frame(100, 30);
        app.input(resize(60, 30));
        assert!(app.world.jostle.velocity.x < 0.0);
        assert_eq!(app.screen_size, (60, 30));

        let mut app = self::app();
        app.frame(100, 30);
        app.paused = true;
        app.input(resize(60, 30));
        assert_eq!(app.world.jostle.velocity, Vec3::ZERO);
        app.paused = false;
        app.mode = Mode::Journal;
        app.input(resize(140, 30));
        assert_eq!(app.world.jostle.velocity, Vec3::ZERO);
    }

    #[test]
    fn the_farewell_follows_sleep_mood_and_bond_and_keeps_the_name_safe() {
        let mut app = app();
        app.world.ghost.name = "Pip\u{1b}[2J".into();
        app.world.ghost.mood = Mood::Content;
        assert_eq!(app.farewell(), "Pip[2J waves goodbye from the glass.");
        app.world.ghost.name = "Pip".into();
        app.world.life.bond = 0.8;
        app.world.ghost.mood = Mood::Cheerful;
        assert!(app.farewell().contains("waves until you're gone"));
        app.world.ghost.mood = Mood::Grumpy;
        assert!(app.farewell().contains("pretends not to watch"));
        app.world.ghost.asleep = 1.0;
        assert_eq!(
            app.farewell(),
            "Pip is asleep on its cushion. Sweet dreams."
        );
    }

    #[test]
    fn a_korean_speaking_viewer_is_seen_off_in_korean_with_the_right_particle() {
        let mut app = app();
        app.world.life.korean = 0.8;
        app.world.ghost.mood = Mood::Content;
        app.world.ghost.name = "뽀삐".into();
        assert_eq!(app.farewell(), "뽀삐가 유리 너머에서 손을 흔들어요.");
        app.world.ghost.name = "콩".into();
        assert_eq!(app.farewell(), "콩이 유리 너머에서 손을 흔들어요.");
        app.world.ghost.name = "Pip".into();
        app.world.ghost.asleep = 1.0;
        assert_eq!(app.farewell(), "Pip가 방석 위에서 자고 있어요. 좋은 꿈 꿔.");
    }

    #[test]
    fn a_terminal_left_in_the_background_soon_counts_the_viewer_away() {
        let mut app = app();
        app.input(Input::Key(Key::Character('r')));
        app.advance(1.0);
        assert!(app.world.viewer_present());
        assert_eq!(app.input(Input::Focus(false)), Response::Continue);
        assert!(!app.focused());
        app.advance(10.0);
        assert!(app.world.viewer_present(), "a glance away is no leaving");
        app.advance(15.0);
        assert!(!app.world.viewer_present());
        let left = |app: &App| {
            app.world
                .events
                .iter()
                .any(|event| event.kind == EventKind::ViewerLeft)
        };
        assert!(!left(&app), "a few minutes elsewhere end no visit");
        for _ in 0..31 {
            app.advance(10.0);
        }
        assert!(left(&app));
        app.input(Input::Focus(true));
        assert!(app.focused());
        assert!(app.world.viewer_present());
        // A key while elsewhere, such as a wheel turned over the window,
        // brings the viewer back too.
        app.input(Input::Focus(false));
        app.advance(25.0);
        assert!(!app.world.viewer_present());
        app.input(Input::Key(Key::Character('r')));
        assert!(app.world.viewer_present());
    }

    #[test]
    fn a_saved_life_comes_back_older_after_time_away() {
        let mut world = World::new(1, 12.0);
        let mut save = Save {
            version: Save::VERSION,
            name: "Pip".into(),
            born: 0,
            last_seen: local_seconds("2026-10-20T12:00:00"),
            seedlings: Vec::new(),
            room: None,
            household: Default::default(),
            needs: crate::world::Needs::default(),
            plant: crate::world::Plant {
                position: world.plant.position,
                growth: 0.5,
                thirst: 0.0,
                ..Default::default()
            },
            lamp_on: false,
            goal: Some("tend the plant".into()),
            memories: Vec::new(),
            conversation: Vec::new(),
            life: crate::world::Life {
                bond: 0.6,
                closest: 0.6,
                day: "2026-10-20".into(),
                ..Default::default()
            },
        };
        for _ in 0..3 {
            save.life.note(&EventKind::AteMote);
        }
        restore(&mut world, &save, save.last_seen + 86_400 * 3);
        assert_eq!(world.life.nature, Some(Nature::from_birth(save.born)));
        let mut another = World::new(2, 12.0);
        restore(&mut another, &save, save.last_seen + 86_400 * 3);
        assert_eq!(another.life.nature, world.life.nature);
        save.life.nature = Some(Nature {
            playful: 0.9,
            shy: 0.7,
            ..Nature::default()
        });
        let saved: Save = serde_json::from_str(&serde_json::to_string(&save).unwrap()).unwrap();
        restore(&mut another, &saved, saved.last_seen);
        assert_eq!(another.life.nature, save.life.nature);
        assert_eq!(world.ghost.name, "Pip");
        assert_eq!(world.plant.thirst, 1.0);
        assert!(world.ghost.needs.company < save.needs.company);
        assert!(!world.lamp.on);
        assert!(matches!(
            world.events.last().unwrap().kind,
            EventKind::Returned { .. }
        ));
        assert!(world.life.bond < 0.6 && world.life.bond >= 0.3);
        world.life.begin_day("2026-10-23");
        assert!(
            world.life.diary[0]
                .text
                .starts_with("Ate three light motes")
        );
    }

    #[test]
    fn a_saved_life_with_impossible_values_is_brought_back_in_range() {
        let mut world = World::new(1, 12.0);
        let layout = world.plant.position;
        let save = Save {
            version: Save::VERSION,
            name: "   ".into(),
            born: 0,
            last_seen: 1000,
            seedlings: Vec::new(),
            room: None,
            household: Default::default(),
            needs: crate::world::Needs {
                energy: 99.0,
                fun: -3.0,
                company: 0.5,
            },
            plant: crate::world::Plant {
                position: Vec3::new(1.0e9, -4.0, 7.0),
                growth: 1.0e6,
                thirst: -5.0,
                ..Default::default()
            },
            lamp_on: true,
            goal: None,
            memories: Vec::new(),
            conversation: Vec::new(),
            life: crate::world::Life {
                bond: -2.0,
                closest: f32::INFINITY,
                bird_trust: 99.0,
                keepsakes: vec![crate::world::Keepsake::Feather; 12],
                nature: Some(Nature {
                    playful: 99.0,
                    curious: -2.0,
                    ..Nature::default()
                }),
                ..Default::default()
            },
        };
        restore(&mut world, &save, 1000);
        assert_eq!(world.ghost.name, "Boo");
        assert_eq!(
            (world.ghost.needs.energy, world.ghost.needs.fun),
            (1.0, 0.0)
        );
        assert_eq!(world.plant.position, layout);
        assert_eq!((world.plant.growth, world.plant.thirst), (1.0, 0.0));
        assert!((0.0..=1.0).contains(&world.life.bond));
        assert_eq!(world.life.nature.unwrap().playful, 1.0);
        assert_eq!(world.life.nature.unwrap().curious, 0.0);
        assert_eq!(world.life.bird_trust, 1.0);
        assert_eq!(world.life.keepsakes.len(), 8);
    }

    #[test]
    fn only_the_journal_and_help_leave_the_mouse_to_the_terminal() {
        let mut app = app();
        for (mode, wants_mouse) in [
            (Mode::Watching, true),
            (Mode::Talking, true),
            (Mode::Settings, true),
            (Mode::Journal, false),
            (Mode::Help, false),
            (Mode::Life, false),
        ] {
            app.mode = mode;
            assert_eq!(app.wants_mouse(), wants_mouse, "{mode:?}");
        }
    }

    #[test]
    fn the_journal_opens_and_reopens_at_the_bottom_and_follows_new_lines() {
        let mut app = app();
        fill_journal(&mut app);
        app.input(Input::Key(Key::Character('j')));
        assert_eq!(app.mode, Mode::Journal);
        assert_eq!(app.journal_scroll, 0);
        app.input(Input::Key(Key::Home));
        assert!(app.journal_scroll > 0);
        app.input(Input::Key(Key::Escape));
        app.input(Input::Key(Key::Character('j')));
        assert_eq!(app.journal_scroll, 0);

        app.mind.journal.memories.push(Memory {
            at: 0,
            text: "A new memory".into(),
        });
        app.frame(100, 32);
        assert_eq!(app.journal_scroll, 0);
    }

    #[test]
    fn journal_keys_scroll_by_lines_and_pages_and_clamp_at_both_ends() {
        let mut app = app();
        fill_journal(&mut app);
        app.input(Input::Key(Key::Character('j')));
        let journal = ui::JournalPanel::new(&app, 100, 32);
        let page = journal.height;
        let limit = journal.max_scroll();
        assert!(limit > page + 1);
        for (key, scroll) in [
            (Key::Up, 1),
            (Key::Down, 0),
            (Key::Down, 0),
            (Key::Up, 1),
            (Key::PageUp, page + 1),
            (Key::PageDown, 1),
            (Key::Home, limit),
            (Key::Up, limit),
            (Key::PageUp, limit),
            (Key::Down, limit - 1),
            (Key::End, 0),
            (Key::Down, 0),
            (Key::PageDown, 0),
        ] {
            assert_eq!(app.input(Input::Key(key.clone())), Response::Continue);
            assert_eq!(app.mode, Mode::Journal);
            assert_eq!(app.journal_scroll, scroll, "{key:?}");
        }
    }

    #[test]
    fn the_journal_closes_only_with_its_close_keys_and_quits_with_q() {
        let mut app = app();
        for close in [Key::Escape, Key::Enter, Key::Character('j')] {
            app.input(Input::Key(Key::Character('j')));
            assert!(!app.wants_mouse());
            for ignored in [
                Key::Character('x'),
                Key::Character(' '),
                Key::Character('?'),
                Key::Left,
                Key::Right,
                Key::Delete,
            ] {
                assert_eq!(app.input(Input::Key(ignored)), Response::Continue);
                assert_eq!(app.mode, Mode::Journal);
            }
            assert_eq!(app.input(Input::Key(close)), Response::Continue);
            assert_eq!(app.mode, Mode::Watching);
            assert!(app.wants_mouse());
        }
        app.input(Input::Key(Key::Character('j')));
        assert_eq!(app.input(Input::Key(Key::Character('q'))), Response::Quit);
    }

    #[test]
    fn resizing_and_changing_journal_content_clamps_the_scroll() {
        let mut app = app();
        fill_journal(&mut app);
        app.input(Input::Key(Key::Character('j')));
        app.input(Input::Key(Key::Home));
        assert!(app.journal_scroll > 0);
        app.frame(100, 200);
        assert_eq!(app.journal_scroll, 0);
        app.frame(100, 32);
        app.input(Input::Key(Key::Home));
        assert!(app.journal_scroll > 0);
        app.mind.journal.memories.clear();
        app.frame(100, 32);
        assert_eq!(app.journal_scroll, 0);
    }

    #[test]
    fn the_journal_draws_and_scrolls_on_tiny_screens_without_panicking() {
        let mut app = app();
        fill_journal(&mut app);
        app.input(Input::Key(Key::Character('j')));
        for (columns, rows) in [(20, 6), (1, 1)] {
            app.frame(columns, rows);
            for key in [
                Key::Home,
                Key::Up,
                Key::PageUp,
                Key::Down,
                Key::PageDown,
                Key::End,
            ] {
                app.input(Input::Key(key));
                app.frame(columns, rows);
                assert!(
                    app.journal_scroll <= ui::JournalPanel::new(&app, columns, rows).max_scroll()
                );
            }
        }
    }

    #[test]
    fn the_life_panel_opens_with_l_and_closes_on_other_keys_or_quits_with_q() {
        let mut app = app();
        app.input(Input::Key(Key::Character('l')));
        assert_eq!(app.mode, Mode::Life);
        assert!(!app.wants_mouse());
        for (columns, rows) in [(100, 32), (20, 6), (1, 1)] {
            app.frame(columns, rows);
        }
        // Arrows scroll it; anything else closes it.
        app.input(Input::Key(Key::Down));
        assert_eq!(app.mode, Mode::Life);
        app.input(Input::Key(Key::Character('x')));
        assert_eq!(app.mode, Mode::Watching);
        app.input(Input::Key(Key::Character('l')));
        assert_eq!(app.input(Input::Key(Key::Character('q'))), Response::Quit);
    }

    #[test]
    fn help_closes_on_any_key_quits_with_q_and_space_pauses_the_world() {
        let mut app = app();
        app.input(Input::Key(Key::Character('?')));
        assert_eq!(app.mode, Mode::Help);
        app.input(Input::Key(Key::Character('x')));
        assert_eq!(app.mode, Mode::Watching);
        app.input(Input::Key(Key::Character(' ')));
        let time = app.world.time;
        app.advance(1.0);
        assert_eq!(app.world.time, time);
        assert!(app.world.ghost.plan.activity == Activity::Float || app.paused);
        app.input(Input::Key(Key::Character('?')));
        assert_eq!(app.input(Input::Key(Key::Character('q'))), Response::Quit);
    }

    #[test]
    fn a_live_ghost_takes_its_birth_date_from_the_saved_timestamp_in_local_time() {
        let born = 1_790_000_000;
        let expected = jiff::Timestamp::from_second(born)
            .unwrap()
            .to_zoned(jiff::tz::TimeZone::system())
            .date();
        let app = app().live(None, born);
        assert_eq!(app.world.birthday, Some(expected));
    }

    #[test]
    fn clicking_a_loose_block_flicks_it_and_clicking_a_tower_topples_it() {
        let mut app = app();
        let spot = Vec3::new(-0.45, 0.0, 0.65);
        app.world.blocks[0] = crate::world::Block::at(spot, 0);
        let (column, row) = cell_of(&app, app.world.blocks[0].position);
        let motes = app.world.motes.len();
        click(&mut app, column, row);
        assert!(app.world.blocks[0].velocity.length() > 0.5);
        assert_eq!(app.world.motes.len(), motes);

        app.world.tower = Some(spot);
        for (level, block) in app.world.blocks.iter_mut().take(3).enumerate() {
            block.position = spot + Vec3::Y * ((level as f32 + 0.5) * crate::world::blocks::EDGE);
            block.stacked = true;
            block.velocity = Vec3::ZERO;
        }
        let (column, row) = cell_of(&app, app.world.blocks[1].position);
        click(&mut app, column, row);
        assert!(app.world.tower.is_none());
        assert!(app.world.blocks.iter().all(|block| !block.stacked));
        assert_eq!(app.world.life.total.toppled, 1);
        assert_eq!(app.world.motes.len(), motes);
    }

    #[test]
    fn right_drags_and_finger_mode_draw_without_orbiting_or_stroking() {
        for right in [true, false] {
            let mut app = app();
            let camera = (app.camera.yaw, app.camera.pitch);
            if !right {
                assert_eq!(
                    app.input(Input::Key(Key::Character('f'))),
                    Response::Continue
                );
            }
            let (column, row) = cell_of(&app, Vec3::new(0.0, 1.2, ROOM_MAX.z));
            let actions = if right {
                [
                    MouseAction::DrawPress,
                    MouseAction::DrawDrag,
                    MouseAction::DrawDrag,
                    MouseAction::DrawRelease,
                ]
            } else {
                [
                    MouseAction::Press,
                    MouseAction::Drag,
                    MouseAction::Drag,
                    MouseAction::Release,
                ]
            };
            for (offset, action) in [0, 1, 2, 2].into_iter().zip(actions) {
                app.input(Input::Mouse(Mouse {
                    column: column + offset,
                    row,
                    action,
                }));
            }
            let drawing = app.world.drawing.as_ref().unwrap();
            assert_eq!(drawing.strokes.len(), 1);
            assert!(drawing.strokes[0].len() >= 2);
            assert_eq!((app.camera.yaw, app.camera.pitch), camera);
            assert_eq!(app.world.petting(), 0.0);
        }
    }

    fn local_seconds(text: &str) -> i64 {
        text.parse::<jiff::civil::DateTime>()
            .unwrap()
            .to_zoned(jiff::tz::TimeZone::system())
            .unwrap()
            .timestamp()
            .as_second()
    }

    fn garden_save(world: &World) -> Save {
        Save {
            version: Save::VERSION,
            name: world.ghost.name.clone(),
            born: 0,
            last_seen: local_seconds("2026-06-21T12:00:00"),
            needs: world.ghost.needs,
            plant: world.plant,
            lamp_on: world.lamp.on,
            goal: None,
            memories: Vec::new(),
            conversation: Vec::new(),
            life: world.life.clone(),
            seedlings: world.seedlings.clone(),
            room: Some(world.saved_room()),
            household: Default::default(),
        }
    }

    #[test]
    fn finger_mode_toggles_and_escape_leaves_it_without_quitting() {
        let mut app = app();
        app.input(Input::Key(Key::Character('f')));
        assert!(app.drawing_mode);
        assert!(app.notice().unwrap().contains("Drawing in the fog"));
        assert_eq!(app.input(Input::Key(Key::Escape)), Response::Continue);
        assert!(!app.drawing_mode);
        app.input(Input::Key(Key::Character('f')));
        app.input(Input::Key(Key::Character('f')));
        assert!(!app.drawing_mode);
        assert_eq!(app.input(Input::Key(Key::Escape)), Response::Quit);
    }

    #[test]
    fn a_left_click_in_finger_mode_still_pokes_the_ghost_without_drawing() {
        let mut app = app();
        app.input(Input::Key(Key::Character('f')));
        let (column, row) = cell_of(&app, app.world.ghost.position);
        click(&mut app, column, row);
        assert!(app.world.drawing.is_none());
        assert!(
            app.world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::Poked { .. }))
        );
    }

    #[test]
    fn saved_flowers_seeds_and_relative_seedling_ages_round_trip() {
        let mut world = World::new(1, 12.0);
        world.plant.growth = 1.0;
        world.plant.blossom = 0.7;
        world.plant.bloom_age = 2500.0;
        world.plant.seeds = 2;
        world.plant.droop = 0.1;
        let mut seedling = crate::world::Seedling::new(crate::world::Flower::Tulip);
        seedling.age = 3.0 * 86_400.0;
        seedling.growth = 0.65;
        seedling.thirst = 0.75;
        seedling.droop = 0.2;
        world.seedlings.push(seedling);
        let save = garden_save(&world);
        let encoded = serde_json::to_string(&save).unwrap();
        let decoded: Save = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, save);
        let mut restored = World::new(2, 12.0);
        restore(&mut restored, &decoded, decoded.last_seen);
        assert_eq!(restored.plant, world.plant);
        assert_eq!(restored.seedlings, world.seedlings);
        assert!(restored.petals.is_empty());
    }

    #[test]
    fn damaged_garden_values_are_clamped_and_only_three_pots_are_restored() {
        let mut world = World::new(1, 12.0);
        let mut save = garden_save(&world);
        save.plant.blossom = 9.0;
        save.plant.growth = 1.0;
        save.plant.bloom_age = f32::NEG_INFINITY;
        save.plant.seeds = u32::MAX;
        save.plant.droop = f32::NAN;
        let mut seedling = crate::world::Seedling::new(crate::world::Flower::Daisy);
        seedling.growth = 9.0;
        seedling.thirst = -2.0;
        seedling.droop = f32::INFINITY;
        seedling.age = f64::NAN;
        save.seedlings = vec![seedling; 5];
        restore(&mut world, &save, save.last_seen);
        assert_eq!(
            (
                world.plant.blossom,
                world.plant.bloom_age,
                world.plant.seeds,
                world.plant.droop
            ),
            (1.0, 0.0, 3, 0.0)
        );
        assert_eq!(world.seedlings.len(), 3);
        for plant in &world.seedlings {
            assert_eq!(
                (plant.growth, plant.thirst, plant.droop, plant.age),
                (1.0, 0.0, 0.0, 0.0)
            );
        }
    }

    #[test]
    fn restore_keeps_the_saved_day_then_writes_skipped_days_in_order_and_in_words() {
        let mut world = World::new(1, 12.0);
        let mut save = garden_save(&world);
        save.last_seen = local_seconds("2026-06-21T12:00:00");
        save.life.begin_day("2026-06-21");
        save.life.note(&EventKind::AteMote);
        let total = save.life.total.clone();
        let skills = save.life.skills.clone();
        restore(&mut world, &save, local_seconds("2026-06-25T12:00:00"));
        assert_eq!(
            world
                .life
                .diary
                .iter()
                .map(|entry| entry.day.as_str())
                .collect::<Vec<_>>(),
            ["2026-06-21", "2026-06-22", "2026-06-23", "2026-06-24"]
        );
        assert!(world.life.diary[0].text.starts_with("Ate"));
        for entry in &world.life.diary[1..] {
            assert!(entry.text.starts_with("While you were away:"));
            assert!(
                !entry
                    .text
                    .chars()
                    .any(|character| character.is_ascii_digit())
            );
        }
        assert_eq!(world.life.total.motes, total.motes);
        assert_eq!(world.life.total.towers, total.towers);
        assert_eq!(world.life.skills, skills);
        assert_eq!(world.life.day, "2026-06-25");
        assert_eq!(world.date.to_string(), "2026-06-25");
        assert_eq!(world.hour, 12.0);
    }

    #[test]
    fn a_long_absence_keeps_the_diary_capped_and_compresses_old_quiet_days() {
        let mut world = World::new(1, 12.0);
        let mut save = garden_save(&world);
        save.life.day = "2026-06-21".to_owned();
        save.life.note(&EventKind::AteMote);
        let start = jiff::civil::Date::constant(2026, 4, 1);
        for offset in 0..60 {
            save.life.diary.push(crate::world::Entry {
                day: start
                    .checked_add(jiff::Span::new().days(offset))
                    .unwrap()
                    .to_string(),
                text: "A quiet day.".to_owned(),
                korean: None,
            });
        }
        restore(&mut world, &save, local_seconds("2027-06-21T12:00:00"));
        assert_eq!(world.life.diary.len(), 60);
        assert!(
            world
                .life
                .diary
                .windows(2)
                .all(|pair| pair[0].day < pair[1].day)
        );
        let recent = &world.life.diary[52..];
        assert!(recent[0].text.contains("many quiet days passed"));
        assert_eq!(recent[0].day, "2027-06-13");
        assert_eq!(recent[7].day, "2027-06-20");
        assert!(recent.iter().all(|entry| {
            !entry
                .text
                .chars()
                .any(|character| character.is_ascii_digit())
        }));
    }

    #[test]
    fn a_night_away_restores_energy_and_a_brief_absence_preserves_the_room() {
        let mut world = World::new(1, 12.0);
        world.ball = Ball::at(-0.5, 0.9);
        let mut save = garden_save(&world);
        save.needs.energy = 0.1;
        save.last_seen = local_seconds("2026-06-21T21:00:00");
        let mut brief = World::new(99, 12.0);
        restore(&mut brief, &save, save.last_seen + 1800);
        assert_eq!(brief.saved_room(), save.room.clone().unwrap());
        assert!(brief.away.is_none());
        restore(&mut world, &save, local_seconds("2026-06-22T08:00:00"));
        assert!(world.ghost.needs.energy >= 0.8);
        assert!(world.ghost.needs.company < save.needs.company);
        assert!(world.ghost.needs.fun < save.needs.fun);
        assert_eq!(world.away.as_ref().unwrap().nights, 1);
    }

    #[test]
    fn time_away_ages_the_garden_without_water_and_bounds_long_absences() {
        let mut world = World::new(1, 12.0);
        world.plant.thirst = 0.0;
        world.plant.growth = 1.0;
        let mut seedling = crate::world::Seedling::new(crate::world::Flower::Bluebell);
        seedling.age = 3.0 * 86_400.0;
        seedling.growth = 0.5;
        world.seedlings.push(seedling);
        let save = garden_save(&world);
        restore(&mut world, &save, save.last_seen + 300);
        assert!(world.plant.blossom > 0.0 && world.plant.blossom < 0.1);
        assert!(world.seedlings[0].growth > 0.5 && world.seedlings[0].growth < 0.51);
        assert_eq!(world.seedlings[0].age, save.seedlings[0].age + 300.0);
        assert!(
            (world.seedlings[0].thirst - 300.0 / crate::world::Plant::THIRST_TIME).abs() < 1e-5
        );
        let mut old = World::new(2, 12.0);
        restore(&mut old, &save, save.last_seen + 86_400 * 30);
        let mut ancient = World::new(3, 12.0);
        restore(&mut ancient, &save, i64::MAX);
        assert_eq!(old.seedlings, ancient.seedlings);
        assert_eq!(old.plant, ancient.plant);
        assert_eq!(old.seedlings[0].thirst, 1.0);
        assert_eq!(old.seedlings[0].droop, 1.0);
        // It grows only while the last watering keeps it moist.
        assert!(old.seedlings[0].growth < 0.52);
        assert_eq!(old.seedlings.len(), 1);
        assert!(old.petals.is_empty());
        let mut winter = World::new(1, 12.0);
        let mut winter_save = save.clone();
        winter_save.last_seen = local_seconds("2026-12-10T12:00:00");
        restore(&mut winter, &winter_save, winter_save.last_seen + 300);
        assert_eq!(winter.plant.blossom, 0.0);
        assert_eq!(winter.seedlings[0].growth, 0.5);
    }
    #[test]
    fn sharing_a_wish_posts_its_hint_and_saves_the_day_it_was_said() {
        use crate::world::{Wish, Wishing};
        let mut app = app();
        app.world.life.wish = Some(Wishing {
            wish: Wish::Game,
            day: app.world.date.to_string(),
            said: None,
        });
        app.world.notice_viewer();
        app.world.begin(Activity::Visit);
        app.world.ghost.plan.arrived = Some(app.world.time);
        app.advance(0.0);
        assert!(app.notice().unwrap().contains("press g"));
        assert_eq!(
            app.world.life.wish.as_ref().unwrap().said.as_deref(),
            Some(app.world.date.to_string().as_str())
        );
        let spoken = app
            .world
            .events
            .iter()
            .filter(|event| matches!(&event.kind, EventKind::Said(text) if text.contains("I wish")))
            .count();
        app.advance(0.0);
        assert_eq!(
            app.world
                .events
                .iter()
                .filter(
                    |event| matches!(&event.kind, EventKind::Said(text) if text.contains("I wish"))
                )
                .count(),
            spoken
        );
    }

    #[test]
    fn hiding_mode_uses_floor_clicks_keeps_pokes_and_cancels_with_escape_or_x() {
        let mut app = app();
        for key in [Key::Escape, Key::Character('x')] {
            app.input(Input::Key(Key::Character('x')));
            assert!(app.world.hunt_counting());
            assert!(app.wants_mouse());
            assert_eq!(
                app.notice(),
                Some("click the floor to hide a treasure · Esc cancels")
            );
            assert_eq!(app.input(Input::Key(key)), Response::Continue);
            assert!(app.world.hunt.is_none());
        }
        app.input(Input::Key(Key::Character('x')));
        let (column, row) = cell_of(&app, app.world.ghost.position);
        click(&mut app, column, row);
        assert!(app.world.hunt_counting());
        assert!(
            app.world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::Poked { .. }))
        );
        assert!(app.world.motes.is_empty());
        app.click(Vec2::new(100.0, 100.0));
        assert!(app.world.hunt_counting());
        let floor = Vec3::new(-0.5, 0.0, 0.8);
        app.click(app.view.unwrap().project(floor).unwrap());
        assert!(!app.world.hunt_counting());
        assert!(app.world.hunt.is_some());
        assert_eq!(app.world.ghost.plan.activity, Activity::Seek);
        assert!(app.world.motes.is_empty());
    }

    #[test]
    fn hiding_refuses_sleep_work_and_existing_games_with_a_notice() {
        let mut app = app();
        app.world.ghost.asleep = 0.9;
        app.input(Input::Key(Key::Character('x')));
        assert!(app.world.hunt.is_none());
        assert!(app.notice().unwrap().contains("asleep"));
        app.world.ghost.asleep = 0.0;
        app.world.start_focus(25);
        app.input(Input::Key(Key::Character('x')));
        assert!(app.world.hunt.is_none());
        assert!(app.notice().unwrap().contains("focus"));
        app.world.toggle_focus();
        app.world.challenge();
        app.input(Input::Key(Key::Character('x')));
        assert!(app.world.hunt.is_none());
        assert!(app.notice().unwrap().contains("game"));
    }

    fn soap(position: Vec3) -> crate::world::bubbles::Bubble {
        crate::world::bubbles::Bubble {
            position,
            velocity: Vec3::ZERO,
            radius: 0.1,
            seed: 1.0,
            born: 0.0,
            lifetime: 12.0,
            popping: None,
        }
    }

    #[test]
    fn clicking_a_bubble_pops_the_nearest_shell_before_the_ball_or_blocks() {
        for blocks in [false, true] {
            let mut app = app();
            let view = app.view.unwrap();
            app.world.ghost.position = Vec3::new(-1.5, 1.6, -0.7);
            let target = if blocks {
                app.world.blocks[0].position
            } else {
                app.world.ball.position
            };
            let point = view.project(target).unwrap();
            let direction = view.ray(point);
            let distance = (target - view.eye).dot(direction);
            let nearest = view.eye + direction * (distance - 1.0);
            let farther = view.eye + direction * (distance - 0.4);
            app.world.bubbles = vec![soap(farther), soap(nearest)];
            app.click(point);
            assert!(app.world.bubbles[1].popping.is_some());
            assert!(app.world.bubbles[0].popping.is_none());
            assert!(app.world.motes.is_empty());
            assert!(
                app.world
                    .blocks
                    .iter()
                    .all(|block| block.velocity == Vec3::ZERO)
            );
            assert!(!app.world.events.iter().any(|event| matches!(
                event.kind,
                EventKind::KickedBall | EventKind::Toppled { .. }
            )));
        }
    }

    #[test]
    fn bubble_clicks_leave_the_ghost_and_active_board_cells_first() {
        let mut app = app();
        let view = app.view.unwrap();
        let point = view.project(app.world.ghost.position).unwrap();
        let direction = view.ray(point);
        let distance = (app.world.ghost.position - view.eye).dot(direction);
        app.world.bubbles = vec![soap(view.eye + direction * (distance + 0.5))];
        app.click(point);
        assert!(app.world.bubbles[0].popping.is_none());
        assert!(
            app.world
                .events
                .iter()
                .any(|event| matches!(event.kind, EventKind::Poked { .. }))
        );

        app.world.ghost.position = Vec3::new(-1.5, 0.8, 0.0);
        app.world.game = Some(crate::world::Game::new(
            Vec2::new(0.0, 1.5),
            crate::world::Mark::Cross,
            0.0,
        ));
        let glass = Vec3::new(0.0, 1.5, ROOM_MAX.z);
        let point = view.project(glass).unwrap();
        app.world.bubbles = vec![soap(glass - Vec3::Z * 0.2)];
        app.click(point);
        assert!(app.world.bubbles[0].popping.is_none());
        assert!(app.world.motes.is_empty());
    }
    #[test]
    fn restoring_a_cold_counts_the_absence_once_and_remembers_recovery() {
        let mut original = World::new(3, 12.0);
        original.begin_life(&original.date.to_string());
        original.life.cold = Some(crate::world::cold::Cold {
            day: original.date.to_string(),
            hour: 11.0,
            severity: 0.5,
            care: vec![crate::world::cold::Care::Words],
            remaining: 20.0 * 3600.0,
        });
        original.life.last_cold = Some(original.date.to_string());
        let save = garden_save(&original);
        let mut restored = World::new(4, 12.0);
        restore(&mut restored, &save, save.last_seen + 3600);
        assert_eq!(
            restored.life.cold.as_ref().unwrap().remaining,
            19.0 * 3600.0
        );
        assert_eq!(
            restored.life.cold.as_ref().unwrap().care,
            [crate::world::cold::Care::Words]
        );
        let mut restored = World::new(4, 12.0);
        restore(&mut restored, &save, save.last_seen + 25 * 3600);
        assert!(restored.life.cold.is_none());
        assert_eq!(restored.life.total.recoveries, 1);
        assert!(restored.life.cold_recovered);
        let mut recovered = save;
        recovered.life = restored.saved_life();
        recovered.last_seen += 25 * 3600;
        restore(&mut restored, &recovered, recovered.last_seen + 3600);
        assert_eq!(restored.life.total.recoveries, 1);
    }
}
