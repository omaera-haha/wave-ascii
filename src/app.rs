use std::io;

use glam::{Vec2, Vec3};

use crate::canvas::{Canvas, Style};
use crate::color::ColorMode;
use crate::face::{self, mood::Cues};
use crate::frame::{CELL_ASPECT, Grid};
use crate::mind::{self, Journal, Mind};
use crate::render::ghost::{Aura, Pose};
use crate::render::window::Outside;
use crate::render::{self, Camera, GHOST_SCALE, Scene, View};
use crate::settings::{Settings, cache_name, effort_name};
use crate::store::{Config, Paths, Save};
use crate::terminal::{Input, Key, Mouse, MouseAction};
use crate::ui;
use crate::world::{Ball, Mood, Nature, Needs, ROOM_MAX, ROOM_MIN, World};

// Radians per arrow key press, and per cell dragged.
const TURN: f32 = 0.12;
const DRAG_TURN: f32 = 0.03;
/// Seconds the eyes keep following the pointer after it stops.
const WATCH_POINTER: f64 = 4.0;
/// Seconds of events the face reacts to.
const REACTION_WINDOW: f64 = 6.0;
/// Seconds between saves while the app runs.
const SAVE_EVERY: f64 = 60.0;

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
    pub mind: Mind,
    pub camera: Camera,
    pub style: Style,
    pub color: ColorMode,
    pub paused: bool,
    pub mode: Mode,
    /// Lines above the journal's bottom; zero follows its newest lines.
    pub journal_scroll: usize,
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
    /// The aura's color, eased toward the color of the ghost's mood.
    aura: Vec3,
    last_save: f64,
}

impl App {
    pub fn new(world: World, mind: Mind, config: Config, style: Style, color: ColorMode) -> Self {
        let aura = mood_color(world.ghost.mood);
        Self {
            world,
            mind,
            camera: Camera::default(),
            style,
            color,
            paused: false,
            mode: Mode::Watching,
            journal_scroll: 0,
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
            aura,
            last_save: 0.0,
        }
    }

    /// Keeps the ghost's life in `paths` and follows the clock: a living
    /// ghost rather than a picture of one.
    pub fn live(mut self, paths: Option<Paths>, born: i64) -> Self {
        self.paths = paths;
        self.born = born;
        self.world.birthday = jiff::Timestamp::from_second(born)
            .ok()
            .map(|timestamp| timestamp.to_zoned(jiff::tz::TimeZone::system()).date());
        self.live_clock = true;
        (self.world.date, self.world.hour) = local_time();
        self
    }

    /// Moves the world and the mind forward by `seconds`.
    pub fn advance(&mut self, seconds: f32) {
        if self.paused {
            return;
        }
        if self.live_clock {
            (self.world.date, self.world.hour) = local_time();
            self.world.life.begin_day(&self.world.date.to_string());
        }
        self.world.viewer.typing = self.mode == Mode::Talking && !self.talk.text().is_empty();
        self.world.step(seconds);
        self.mind.think(&mut self.world);
        let ease = 1.0 - (-1.5 * seconds).exp();
        self.aura = self.aura.lerp(mood_color(self.world.ghost.mood), ease);
        if self.live_clock && self.world.time - self.last_save > SAVE_EVERY {
            self.last_save = self.world.time;
            if let Err(error) = self.save() {
                self.post(format!("Couldn't save the ghost: {error}"));
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
            life: world.life.clone(),
        };
        crate::store::save_json(&paths.save, &save)
    }

    pub fn input(&mut self, input: Input) -> Response {
        match input {
            Input::Key(key) => self.press_key(key),
            Input::Paste(text) => {
                match self.mode {
                    Mode::Talking => self.talk.insert(&text),
                    Mode::Settings => self.settings.paste(&text),
                    _ => {}
                }
                Response::Continue
            }
            Input::Mouse(mouse) => self.mouse(mouse),
            Input::Resize => Response::Restyle,
        }
    }

    /// Reading panels leave mouse selection and the wheel to the terminal.
    pub fn wants_mouse(&self) -> bool {
        !matches!(self.mode, Mode::Journal | Mode::Help | Mode::Life)
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
                            self.mind.hear(&mut self.world, &text);
                            if !self.mind.can_talk() {
                                self.post("The ghost listens, but can't answer in words without a Claude key (press k).");
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
            Mode::Help | Mode::Life => {
                if matches!(key, Key::Character('q')) {
                    return Response::Quit;
                }
                self.mode = Mode::Watching;
                Response::Continue
            }
            Mode::Journal => self.scroll_journal(key),
            Mode::Watching => self.command(key),
        }
    }

    fn command(&mut self, key: Key) -> Response {
        match key {
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
            Key::Character('l') => self.mode = Mode::Life,
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
        Response::Continue
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

    /// Applies a mouse event. Dragging turns the view and the wheel zooms.
    /// A click on the ghost pokes it, on the ball flicks it, and anywhere
    /// else in the room drops in a light mote.
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
            MouseAction::Press => {
                self.press = Some((mouse.column, mouse.row));
                self.dragged = false;
            }
            MouseAction::Drag => {
                if let Some((column, row)) = self.press {
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
                if self.press.take().is_some()
                    && !self.dragged
                    && let Some(point) = point
                {
                    self.click(point);
                }
            }
        }

        self.pointer = point.map(|point| (point, self.world.time));
        self.world.viewer.pointer = point
            .zip(self.view)
            .and_then(|(point, view)| view.on_glass(point));
        Response::Continue
    }

    fn click(&mut self, point: Vec2) {
        let Some(view) = self.view else {
            return;
        };
        if view.touches_ghost(point, self.world.ghost.position) {
            self.world.poke();
            return;
        }
        let direction = view.ray(point);
        let offset = view.eye - self.world.ball.position;
        let along = -offset.dot(direction);
        let miss = (offset + direction * along).length();
        if along > 0.0 && miss < Ball::RADIUS * 1.6 {
            self.world.kick_ball(direction);
            return;
        }
        // A mote appears where the ray meets the floor, a little above it,
        // or behind the glass if the ray misses the floor.
        let floor = view
            .on_plane(point, 0.0)
            .filter(|spot| spot.cmpge(ROOM_MIN).all() && spot.cmple(ROOM_MAX).all());
        let spot = floor
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
        self.notice
            .as_ref()
            .filter(|(_, time)| self.world.time - time < 8.0)
            .map(|(message, _)| message.as_str())
    }

    /// Draws the whole screen, `columns` by `rows`: the scene above two
    /// lines of words, and any open panel over it.
    pub fn frame(&mut self, columns: usize, rows: usize) -> Canvas {
        self.screen_size = (columns, rows);
        let scene_rows = rows.saturating_sub(2).max(1);
        self.area = (columns, scene_rows);
        let grid = self.style.grid(columns, scene_rows);
        let view = self.camera.view(grid);
        self.view = Some(view);

        let world = &self.world;
        let ghost = &world.ghost;
        let center = view.project(ghost.position);
        let gaze = self.gaze(&view, center);
        let reactions = world
            .events
            .iter()
            .filter(|event| world.time - event.time < REACTION_WINDOW)
            .filter_map(|event| {
                face::mood::reaction(&event.kind).map(|reaction| (event.time, reaction))
            })
            .collect();
        let face = face::mood::face(
            world.time,
            &Cues {
                mood: ghost.mood,
                asleep: ghost.asleep,
                gaze,
                reactions,
            },
        );
        let energy = ghost.needs.energy;
        let aura = Aura {
            color: self.aura,
            visibility: 1.0 - ghost.fade,
            brightness: 0.25 + 0.75 * energy,
            beat: beat(ghost.mood) * (0.6 + 0.4 * energy),
        };
        let pose = Pose::new(
            world.time,
            ghost.velocity.dot(view.right) / GHOST_SCALE,
            ghost.velocity.y / GHOST_SCALE,
            ghost.asleep,
        );
        let samples = if self.style == Style::Braille { 1 } else { 2 };
        let frame = render::render(
            &Scene {
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
                    time: world.time,
                },
                view,
                face,
                aura,
                pose,
            },
            grid,
            samples,
        );

        let mut screen = Canvas::blank(columns, rows);
        screen.paste(&self.style.canvas(&frame, self.color), 0, 0);
        ui::draw(self, &mut screen, &view, scene_rows);
        screen
    }

    // Where the eyes go: to the pointer while it moves, else the way the
    // ghost faces, so its face shows what it is looking at.
    fn gaze(&self, view: &View, center: Option<Vec2>) -> Option<(Vec2, f32)> {
        let center = center?;
        let ghost = &self.world.ghost;
        if ghost.asleep > 0.5 {
            return None;
        }
        if let Some((pointer, moved)) = self.pointer
            && self.world.time - moved < WATCH_POINTER
        {
            return Some(((pointer - center) / 0.5, 0.85));
        }
        let ahead = view.project(ghost.position + ghost.heading)?;
        Some(((ahead - center) / 0.45, 0.75))
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
    let name: String = save.name.trim().chars().take(24).collect();
    if !name.is_empty() {
        world.ghost.name = name;
    }
    let needs = save.needs;
    world.ghost.needs = Needs {
        energy: needs.energy.clamp(0.0, 1.0),
        fun: needs.fun.clamp(0.0, 1.0),
        company: needs.company.clamp(0.0, 1.0),
    };
    world.plant.growth = save.plant.growth.clamp(0.0, 1.0);
    world.plant.thirst = save.plant.thirst.clamp(0.0, 1.0);
    world.lamp.on = save.lamp_on;
    world.ghost.goal = save.goal.clone();
    world.life = save.life.clone().sanitized();
    world
        .life
        .nature
        .get_or_insert_with(|| Nature::from_birth(save.born));
    let away = (now - save.last_seen).max(0) as f32;
    world.life.miss(away);
    world.plant.thirst = (world.plant.thirst + away / crate::world::Plant::THIRST_TIME).min(1.0);
    world
        .ghost
        .needs
        .change(0.0, 0.0, -(away / 3600.0 * 0.1).min(0.5));
    if away > 1800.0 {
        world.record_return(away);
    }
    Journal {
        memories: save.memories.clone(),
        conversation: save.conversation.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Config, Memory};
    use crate::world::{Activity, EventKind};

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
        assert!(app.notice().unwrap().contains("Claude key"));
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
    fn clicking_the_ball_flicks_it() {
        let mut app = app();
        let (column, row) = cell_of(&app, app.world.ball.position);
        click(&mut app, column, row);
        assert_eq!(app.world.events.last().unwrap().kind, EventKind::KickedBall);
        assert!(app.world.ball.velocity.length() > 1.0);
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
        let center = view.project(app.world.ghost.position);
        let (gaze, firmness) = app.gaze(&view, center).unwrap();
        assert!(gaze.x > 0.0 && gaze.y > 0.0 && firmness > 0.5);
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
        assert_eq!(app.input(Input::Resize), Response::Restyle);
    }

    #[test]
    fn a_saved_life_comes_back_older_after_time_away() {
        let mut world = World::new(1, 12.0);
        let mut save = Save {
            version: Save::VERSION,
            name: "Pip".into(),
            born: 0,
            last_seen: 1000,
            needs: crate::world::Needs::default(),
            plant: crate::world::Plant {
                position: world.plant.position,
                growth: 0.5,
                thirst: 0.0,
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
        restore(&mut world, &save, 1000 + 86_400 * 3);
        assert_eq!(world.life.nature, Some(Nature::from_birth(save.born)));
        let mut another = World::new(2, 12.0);
        restore(&mut another, &save, 1000 + 86_400 * 3);
        assert_eq!(another.life.nature, world.life.nature);
        save.life.nature = Some(Nature {
            playful: 0.9,
            shy: 0.7,
            ..Nature::default()
        });
        let saved: Save = serde_json::from_str(&serde_json::to_string(&save).unwrap()).unwrap();
        restore(&mut another, &saved, 1000);
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
        assert!(world.life.diary[0].text.starts_with("Ate 3 light motes"));
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
            needs: crate::world::Needs {
                energy: 99.0,
                fun: -3.0,
                company: 0.5,
            },
            plant: crate::world::Plant {
                position: Vec3::new(1.0e9, -4.0, 7.0),
                growth: 1.0e6,
                thirst: -5.0,
            },
            lamp_on: true,
            goal: None,
            memories: Vec::new(),
            conversation: Vec::new(),
            life: crate::world::Life {
                bond: -2.0,
                closest: f32::INFINITY,
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
    fn the_life_panel_opens_with_l_and_closes_on_any_key_or_quits_with_q() {
        let mut app = app();
        app.input(Input::Key(Key::Character('l')));
        assert_eq!(app.mode, Mode::Life);
        assert!(!app.wants_mouse());
        for (columns, rows) in [(100, 32), (20, 6), (1, 1)] {
            app.frame(columns, rows);
        }
        app.input(Input::Key(Key::Down));
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
}
