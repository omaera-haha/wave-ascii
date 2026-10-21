use std::io;

use glam::{Vec2, Vec3};

use crate::canvas::{Canvas, Style};
use crate::color::ColorMode;
use crate::face::{self, expression::Expression, mood::Cues};
use crate::frame::{CELL_ASPECT, Grid};
use crate::mind::{self, Journal, Mind};
use crate::postcard::{self, Postcard};
use crate::render::ghost::{Aura, Pose};
use crate::render::window::Outside;
use crate::render::{self, Camera, GHOST_SCALE, Scene, View};
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
            stroke: None,
            stroke_side: 0.0,
            aura,
            last_save: 0.0,
            hinted_claude: false,
            hint_claude_at: None,
            postcard: None,
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
        self.world.tuck_in();
        self
    }

    /// Moves the world and the mind forward by `seconds`.
    pub fn advance(&mut self, seconds: f32) {
        if self.paused {
            return;
        }
        if self.live_clock {
            (self.world.date, self.world.hour) = local_time();
        }
        self.world.viewer.typing = self.mode == Mode::Talking && !self.talk.text().is_empty();
        self.world.step(seconds);
        self.mind.think(&mut self.world);
        if self
            .hint_claude_at
            .is_some_and(|time| self.world.time >= time)
        {
            self.hint_claude_at = None;
            self.post("It talks in a few words of its own. With a Claude key (press k), it can really talk.");
        }
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
            life: world.saved_life(),
            seedlings: world.seedlings.clone(),
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
            Key::Character('p') => self.save_postcard(),
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
        Response::Continue
    }

    fn save_postcard(&mut self) {
        let Some(paths) = &self.paths else {
            self.post("Postcards need a saved life");
            return;
        };
        let Some(postcard) = &self.postcard else {
            return;
        };
        let message = match postcard.save(paths) {
            Ok(path) => format!("Saved a postcard: {}", postcard::display_path(&path)),
            Err(error) => format!("Couldn't save a postcard: {error}"),
        };
        self.post(message);
    }

    fn doing(&self) -> &'static str {
        if self.world.ghost.asleep > 0.5 {
            "asleep on the cushion"
        } else {
            self.world.ghost.plan.activity.doing()
        }
    }

    /// The ghost's name and activity for the terminal window title.
    pub fn title(&self) -> String {
        // A title is an escape sequence, so nothing in it may control the
        // terminal.
        format!("{} · {}", self.world.ghost.name, self.doing())
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
            MouseAction::Press => {
                self.press = Some((mouse.column, mouse.row));
                self.dragged = false;
                self.stroke = point.filter(|point| {
                    self.view
                        .is_some_and(|view| view.touches_ghost(*point, self.world.ghost.position))
                });
            }
            MouseAction::Drag => {
                if let (Some(last), Some(point), Some(view)) = (self.stroke, point, self.view) {
                    if point != last {
                        self.dragged = true;
                        self.stroke = Some(point);
                        if view.touches_ghost(point, self.world.ghost.position) {
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
        let pointer = point
            .zip(self.view)
            .and_then(|(point, view)| view.on_glass(point));
        self.world.point_at(pointer);
        Response::Continue
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
        if view.touches_ghost(point, self.world.ghost.position) {
            self.world.poke();
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
        // Whichever the ray meets first takes the click.
        match (block, ball) {
            (Some((index, block)), ball) if ball.is_none_or(|ball| block < ball) => {
                self.world.nudge_block(index, direction);
                return;
            }
            (_, Some(_)) => {
                self.world.kick_ball(direction);
                return;
            }
            _ => {}
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
        let mut reactions: Vec<_> = world
            .events
            .iter()
            .filter(|event| world.time - event.time < REACTION_WINDOW)
            .filter(|event| event.kind != crate::world::EventKind::BirdSang || ghost.asleep <= 0.5)
            .filter_map(|event| {
                let reaction = if matches!(
                    event.kind,
                    crate::world::EventKind::GameOver(crate::world::Outcome::ViewerWon { .. })
                ) && world.good_sport()
                {
                    Some(Expression::Smile)
                } else {
                    face::mood::reaction(&event.kind)
                };
                reaction.map(|reaction| (event.time, reaction))
            })
            .collect();
        // The first morning also earns a milestone; let the stretch take
        // precedence over reactions recorded at that same moment.
        reactions.sort_by(|(time, expression), (other_time, other_expression)| {
            time.total_cmp(other_time).then_with(|| {
                matches!(expression, Expression::Stretch)
                    .cmp(&matches!(other_expression, Expression::Stretch))
            })
        });
        let face = face::mood::face(
            world.time,
            &Cues {
                mood: ghost.mood,
                asleep: ghost.asleep,
                petting: world.petting(),
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
            ghost.velocity.dot(view.right) / GHOST_SCALE + self.stroke_side * world.petting() * 1.2,
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
                    bird: world
                        .visitors
                        .bird
                        .as_ref()
                        .map(|bird| bird.picture(world.time)),
                    time: world.time,
                    sill: world.life.sill,
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
            self.postcard = Some(Postcard::new(canvas, colored, &self.world, self.doing()));
        }
        ui::draw(self, &mut screen, scene_rows);
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
    let away = now.saturating_sub(save.last_seen).max(0) as f32;
    world.life.miss(away);
    world.age_garden(away);
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
        let center = view.project(app.world.ghost.position);
        let (gaze, firmness) = app.gaze(&view, center).unwrap();
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
            seedlings: Vec::new(),
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
            seedlings: Vec::new(),
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

    fn garden_save(world: &World) -> Save {
        Save {
            version: Save::VERSION,
            name: world.ghost.name.clone(),
            born: 0,
            last_seen: 1000,
            needs: world.ghost.needs,
            plant: world.plant,
            lamp_on: world.lamp.on,
            goal: None,
            memories: Vec::new(),
            conversation: Vec::new(),
            life: world.life.clone(),
            seedlings: world.seedlings.clone(),
        }
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
    fn time_away_ages_the_garden_without_water_and_bounds_long_absences() {
        let mut world = World::new(1, 12.0);
        world.plant.thirst = 0.0;
        world.plant.growth = 1.0;
        let mut seedling = crate::world::Seedling::new(crate::world::Flower::Bluebell);
        seedling.age = 3.0 * 86_400.0;
        seedling.growth = 0.5;
        world.seedlings.push(seedling);
        let save = garden_save(&world);
        restore(&mut world, &save, 1300);
        assert!(world.plant.blossom > 0.0 && world.plant.blossom < 0.1);
        assert!(world.seedlings[0].growth > 0.5 && world.seedlings[0].growth < 0.51);
        assert_eq!(world.seedlings[0].age, save.seedlings[0].age + 300.0);
        assert!((world.seedlings[0].thirst - 1.0 / 3.0).abs() < 1e-5);
        let mut old = World::new(2, 12.0);
        restore(&mut old, &save, save.last_seen + 86_400 * 30);
        let mut ancient = World::new(3, 12.0);
        restore(&mut ancient, &save, i64::MAX);
        assert_eq!(old.seedlings, ancient.seedlings);
        assert_eq!(old.plant, ancient.plant);
        assert_eq!(old.seedlings[0].thirst, 1.0);
        assert_eq!(old.seedlings[0].droop, 1.0);
        assert!(old.seedlings[0].growth < 0.51);
        assert_eq!(old.seedlings.len(), 1);
        assert!(old.petals.is_empty());
        let mut winter = World::new(1, 12.0);
        winter.date = jiff::civil::Date::constant(2026, 12, 10);
        restore(&mut winter, &save, 1300);
        assert_eq!(winter.plant.blossom, 0.0);
        assert_eq!(winter.seedlings[0].growth, 0.5);
    }
}
