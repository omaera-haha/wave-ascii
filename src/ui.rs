use glam::Vec3;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::app::{App, Mode, mood_color};
use crate::canvas::{Canvas, Cell};
use crate::color::Rgb;
use crate::face::emote;
use crate::frame::{CELL_ASPECT, Grid};
use crate::mind::Status;
use crate::render::{GHOST_SCALE, View};
use crate::settings::{Field, cache_name, effort_name};
use crate::store::{Speaker, mask};
use crate::world::Skill;

const PANEL: Rgb = Rgb([17, 19, 27]);
const BORDER: Rgb = Rgb([74, 82, 108]);
const TITLE: Rgb = Rgb([238, 240, 246]);
const TEXT: Rgb = Rgb([196, 201, 213]);
const DIM: Rgb = Rgb([120, 128, 150]);
const ACCENT: Rgb = Rgb([255, 178, 94]);
const BAR: Rgb = Rgb([28, 31, 42]);
const BUBBLE: Rgb = Rgb([236, 238, 245]);
const BUBBLE_TEXT: Rgb = Rgb([30, 32, 44]);
const GOOD: Rgb = Rgb([120, 220, 140]);
const BAD: Rgb = Rgb([255, 110, 100]);
/// Seconds the ghost's words stay in its speech bubble and under the scene.
const SPEECH_SHOWS: f64 = 14.0;
const THOUGHT_SHOWS: f64 = 30.0;

/// Draws the scene's words and symbols, without a status bar or panel.
pub fn draw_moment(app: &App, screen: &mut Canvas, view: &View, scene_rows: usize) {
    let bubble_right = speech_bubble(app, screen, view, scene_rows);
    emote_puffs(app, screen, view, scene_rows, bubble_right);
    words_line(app, screen, scene_rows);
}

/// Draws the status bar and any open panel over the scene.
pub fn draw(app: &mut App, screen: &mut Canvas, scene_rows: usize) {
    status_line(app, screen, scene_rows + 1);
    match app.mode {
        Mode::Help => help_panel(screen),
        Mode::Settings => settings_panel(app, screen),
        Mode::Journal => journal_panel(app, screen),
        Mode::Life => life_panel(app, screen),
        Mode::Album => album_panel(app, screen),
        Mode::Letter | Mode::Writing => letter_panel(app, screen),
        Mode::Watching | Mode::Talking => {}
    }
}

/// Writes `text` from `column` on `row`, stopping before column `end`.
/// Wide characters take two cells. Returns the column after the text.
pub fn write(
    screen: &mut Canvas,
    column: usize,
    row: usize,
    end: usize,
    text: &str,
    foreground: Rgb,
    background: Option<Rgb>,
) -> usize {
    let mut column = column;
    for character in text.chars() {
        let width = character.width().unwrap_or(0);
        if width == 0 {
            continue;
        }
        if column + width > end {
            break;
        }
        screen.set(
            column,
            row,
            Cell {
                glyph: character,
                foreground: Some(foreground),
                background,
            },
        );
        if width == 2 {
            screen.set(column + 1, row, Cell::CONTINUATION);
        }
        column += width;
    }
    column
}

/// Where to start showing `text` so that the characters before `cursor`
/// fit in fewer than `room` cells, leaving a cell for the cursor.
pub fn visible_start(text: &[char], cursor: usize, room: usize) -> usize {
    let mut start = cursor;
    let mut used = 0;
    while start > 0 {
        let next = text[start - 1].width().unwrap_or(0);
        if used + next >= room {
            break;
        }
        used += next;
        start -= 1;
    }
    start
}

/// How many cells `text` takes.
pub fn width(text: &str) -> usize {
    text.chars()
        .map(|character| character.width().unwrap_or(0))
        .sum()
}

/// `text` broken into lines at most `limit` cells wide, between words where
/// it can be and anywhere in a word too long for a line.
pub fn wrap(text: &str, limit: usize) -> Vec<String> {
    let limit = limit.max(1);
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let gap = usize::from(!line.is_empty());
        if width(&line) + gap + width(word) <= limit {
            if gap == 1 {
                line.push(' ');
            }
            line.push_str(word);
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        for character in word.chars() {
            if width(&line) + character.width().unwrap_or(0) > limit {
                lines.push(std::mem::take(&mut line));
            }
            line.push(character);
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn fill(screen: &mut Canvas, row: usize, start: usize, end: usize, background: Rgb) {
    for column in start..end {
        screen.set(
            column,
            row,
            Cell {
                glyph: ' ',
                foreground: None,
                background: Some(background),
            },
        );
    }
}

fn rgb(color: Vec3) -> Rgb {
    Rgb::from_vec3(color)
}

// The ghost's latest words in a bubble above its head. Returns the column
// just beyond the box when any of it is visible.
fn speech_bubble(app: &App, screen: &mut Canvas, view: &View, scene_rows: usize) -> Option<usize> {
    let world = &app.world;
    let speech = world.ghost.speech.as_ref()?;
    if world.time - speech.time > SPEECH_SHOWS {
        return None;
    }
    let head = world.ghost.position + Vec3::Y * 0.8 * GHOST_SCALE * world.size();
    let point = view.project(head)?;
    let columns = screen.columns();
    let scene_rows = scene_rows.min(screen.rows());
    if columns == 0 || scene_rows == 0 {
        return None;
    }
    let pixel = Grid::new(columns, scene_rows, CELL_ASPECT)
        .to_pixels()
        .transform_point2(point);
    let lines = wrap(&speech.text, 34.min(columns.saturating_sub(4)));
    let inner = lines.iter().map(|line| width(line)).max().unwrap_or(0);
    let box_width = inner + 2;
    if lines.is_empty() || box_width > columns {
        return None;
    }
    let anchor = pixel.x.round().clamp(0.0, columns as f32 - 1.0) as usize;
    let tail_row = (pixel.y.round() as isize)
        .saturating_sub(1)
        .max(lines.len() as isize);
    let top = tail_row as usize - lines.len();
    if top >= scene_rows {
        return None;
    }
    let left = anchor
        .saturating_sub(box_width / 2)
        .min(columns - box_width);
    for (offset, line) in lines.iter().enumerate() {
        let row = top + offset;
        if row >= scene_rows {
            break;
        }
        fill(screen, row, left, left + box_width, BUBBLE);
        write(
            screen,
            left + 1,
            row,
            left + box_width,
            line,
            BUBBLE_TEXT,
            Some(BUBBLE),
        );
    }
    if (tail_row as usize) < scene_rows {
        write(
            screen,
            anchor,
            tail_row as usize,
            columns,
            "▾",
            BUBBLE,
            None,
        );
    }
    Some(left + box_width)
}

// Wordless feelings drift up from the head, beside any speech bubble.
fn emote_puffs(
    app: &App,
    screen: &mut Canvas,
    view: &View,
    scene_rows: usize,
    bubble_right: Option<usize>,
) {
    let columns = screen.columns();
    let scene_rows = scene_rows.min(screen.rows());
    if columns == 0 || scene_rows == 0 {
        return;
    }
    let head = app.world.ghost.position + Vec3::Y * 0.8 * GHOST_SCALE * app.world.size();
    let Some(point) = view.project(head) else {
        return;
    };
    let pixel = Grid::new(columns, scene_rows, CELL_ASPECT)
        .to_pixels()
        .transform_point2(point);
    if !(0.0..columns as f32).contains(&pixel.x) || !(0.0..scene_rows as f32).contains(&pixel.y) {
        return;
    }
    let origin = bubble_right.map_or(pixel.x, |right| pixel.x.max(right as f32));
    for puff in emote::puffs(&app.world) {
        let column = (origin + puff.offset.x).round();
        let row = (pixel.y - puff.offset.y).round();
        if !(0.0..columns as f32).contains(&column) || !(0.0..scene_rows as f32).contains(&row) {
            continue;
        }
        let column = column as usize;
        let row = row as usize;
        let fade = ((puff.age - 2.0 / 3.0) * 3.0).clamp(0.0, 1.0);
        let color = puff.emote.color(app.aura()) * (1.0 - fade);
        let background = screen.get(column, row).and_then(|cell| cell.background);
        write(
            screen,
            column,
            row,
            columns,
            puff.glyph,
            rgb(color),
            background,
        );
    }
}

// The line under the scene: what's being typed, a notice, or the ghost's
// latest words, thought, or goal.
fn words_line(app: &App, screen: &mut Canvas, row: usize) {
    let columns = screen.columns();
    let world = &app.world;
    let ghost = &world.ghost;
    if app.mode == Mode::Talking {
        let column = write(screen, 1, row, columns, "› ", ACCENT, None);
        let room = columns.saturating_sub(column + 1);
        let text: Vec<char> = app.talk.text().chars().collect();
        let cursor = app.talk.cursor();
        let start = visible_start(&text, cursor, room);
        let before: String = text[start..cursor].iter().collect();
        let after: String = text[cursor..].iter().take(room + 1).collect();
        let column = write(screen, column, row, columns, &before, TITLE, None);
        let under: String = after.chars().next().map_or(" ".to_owned(), String::from);
        let next = write(screen, column, row, columns, &under, PANEL, Some(ACCENT));
        let rest: String = after.chars().skip(1).collect();
        write(screen, next, row, columns, &rest, TITLE, None);
        return;
    }
    if let Some(notice) = app.notice() {
        write(screen, 1, row, columns, notice, ACCENT, None);
        return;
    }
    if let Some(speech) = &ghost.speech
        && world.time - speech.time < SPEECH_SHOWS
    {
        let column = write(
            screen,
            1,
            row,
            columns,
            &format!("{}: ", ghost.name),
            rgb(app.aura()),
            None,
        );
        write(screen, column, row, columns, &speech.text, TITLE, None);
        return;
    }
    if let Some(thought) = &ghost.thought
        && world.time - thought.time < THOUGHT_SHOWS
    {
        write(
            screen,
            1,
            row,
            columns,
            &format!("… {}", thought.text),
            DIM,
            None,
        );
        return;
    }
    let line = match &ghost.goal {
        Some(goal) => format!("{} means to {goal}", ghost.name),
        None => idle_tip(world.time).to_owned(),
    };
    write(screen, 1, row, columns, &line, DIM, None);
}

/// Tips for a quiet words line, the basics first.
const TIPS: [&str; 9] = [
    "t to talk · click the ghost to poke it, the floor to drop a light mote · drag to look around",
    "drag across the ghost to stroke it · right drag draws in the fog on the glass",
    "e reads its letters, and w there writes it one back",
    "o to focus together beside your work · g for noughts and crosses on the glass",
    "tell it how you feel, like “I'm tired” or “오늘 힘들었어”, and it looks after you",
    "type do re mi to teach it a tune · “play a lullaby” winds the music box",
    "l shows its life · j its journal and diary · b the album of moments it kept",
    "p keeps a postcard of this moment · ? lists every key",
    "ask “fold a crane” or “종이학 접어줘” · little paper wishes grow above the plant",
];
/// Seconds the basics show first, and each later tip after them.
const BASICS_SHOW: f64 = 90.0;
const TIP_SHOWS: f64 = 40.0;

// The tip for a quiet words line at `time` into the session: the basics
// for a while, then the others in turn.
fn idle_tip(time: f64) -> &'static str {
    if time < BASICS_SHOW {
        return TIPS[0];
    }
    let turn = ((time - BASICS_SHOW) / TIP_SHOWS) as usize;
    TIPS[1 + turn % (TIPS.len() - 1)]
}

// The status bar: the ghost's name, mood, and needs, what it is doing, the
// state of its minds, and where to find help.
fn status_line(app: &App, screen: &mut Canvas, row: usize) {
    let columns = screen.columns();
    fill(screen, row, 0, columns, BAR);
    let ghost = &app.world.ghost;
    let mut column = write(screen, 1, row, columns, &ghost.name, TITLE, Some(BAR));
    column = write(screen, column, row, columns, " · ", DIM, Some(BAR));
    column = write(
        screen,
        column,
        row,
        columns,
        "● ",
        rgb(mood_color(ghost.mood)),
        Some(BAR),
    );
    column = write(
        screen,
        column,
        row,
        columns,
        ghost.mood.name(),
        TEXT,
        Some(BAR),
    );
    for (label, level) in [
        ("energy", ghost.needs.energy),
        ("fun", ghost.needs.fun),
        ("company", ghost.needs.company),
    ] {
        column = write(
            screen,
            column,
            row,
            columns,
            &format!("  {label} "),
            DIM,
            Some(BAR),
        );
        column = write(
            screen,
            column,
            row,
            columns,
            &meter(level),
            meter_color(level),
            Some(BAR),
        );
    }
    let doing = app.doing();
    column = write(
        screen,
        column,
        row,
        columns,
        &format!(" · {doing}"),
        TEXT,
        Some(BAR),
    );

    // The right end: the minds and the help key, kept when the rest is cut.
    let mut right = vec![
        (" jev ".to_owned(), DIM),
        status_mark(&app.mind.one),
        (
            format!(
                "  {} ",
                app.config
                    .claude_model
                    .strip_prefix("claude-")
                    .unwrap_or(&app.config.claude_model)
            ),
            DIM,
        ),
        status_mark(&app.mind.two),
    ];
    if app.paused {
        right.push(("  paused".to_owned(), ACCENT));
    }
    right.push(("  ? help ".to_owned(), DIM));
    let right_width: usize = right.iter().map(|(text, _)| width(text)).sum();
    if column + right_width <= columns {
        let mut column = columns - right_width;
        for (text, color) in right {
            column = write(screen, column, row, columns, &text, color, Some(BAR));
        }
    }
    if app.world.unread_letter().is_some() {
        let hint = " ✉ e ";
        let column = columns.saturating_sub(width(hint));
        fill(screen, row, column, columns, BAR);
        write(screen, column, row, columns, hint, ACCENT, Some(BAR));
    }
    if app.drawing_mode {
        let hint = if app.world.unread_letter().is_some() {
            " f fog · Esc stop · ✉ e "
        } else {
            " f fog · Esc stop "
        };
        let column = columns.saturating_sub(width(hint));
        fill(screen, row, column, columns, BAR);
        write(screen, column, row, columns, hint, ACCENT, Some(BAR));
    }
}

fn meter(level: f32) -> String {
    let filled = (level * 5.0).round() as usize;
    format!("{}{}", "▰".repeat(filled), "▱".repeat(5 - filled.min(5)))
}

fn meter_color(level: f32) -> Rgb {
    if level < 0.25 {
        BAD
    } else if level < 0.5 {
        ACCENT
    } else {
        GOOD
    }
}

fn status_mark(status: &Status) -> (String, Rgb) {
    match status {
        Status::Off => ("○".to_owned(), DIM),
        Status::Ready => ("●".to_owned(), GOOD),
        Status::Busy => ("◐".to_owned(), ACCENT),
        Status::Failed(_) => ("✕".to_owned(), BAD),
    }
}

/// A box in the middle of the screen with a title, holding `lines` of
/// text in the given colors.
fn panel(screen: &mut Canvas, title: &str, lines: &[(String, Rgb)]) {
    let inner = lines
        .iter()
        .map(|(line, _)| width(line))
        .max()
        .unwrap_or(0)
        .max(width(title) + 4)
        .min(screen.columns().saturating_sub(6));
    draw_panel(screen, title, lines, inner);
}

fn letter_panel(app: &App, screen: &mut Canvas) {
    if screen.columns() < 5 || screen.rows() < 4 {
        return;
    }
    let paper = Rgb([250, 240, 215]);
    let ink = Rgb([70, 56, 62]);
    let inner = 40.min(screen.columns().saturating_sub(6)).max(1);
    let writing = app.mode == Mode::Writing;
    let (mut lines, cursor) = if writing {
        let heading = if app.world.speaks_korean() {
            format!("{}에게", app.world.ghost.name)
        } else {
            format!("Dear {},", app.world.ghost.name)
        };
        let (body, (row, column)) = draft_lines(&app.draft, inner);
        let mut lines = wrap(&heading, inner);
        lines.push(String::new());
        let heading_lines = lines.len();
        lines.extend(body);
        (lines, Some((row + heading_lines, column)))
    } else if app.sent_letters {
        let Some(letter) = app.world.life.received.get(app.letter_index) else {
            return;
        };
        let mut lines = vec![format!("{} · from you", letter.date_words()), String::new()];
        for line in &letter.lines {
            lines.extend(wrap(line, inner));
        }
        (lines, None)
    } else {
        let Some(letter) = app.world.life.letters.get(app.letter_index) else {
            return;
        };
        let mut lines = vec![letter.date_words(), String::new()];
        for line in &letter.lines {
            lines.extend(wrap(line, inner));
        }
        (lines, None)
    };
    let height = if writing {
        screen.rows().min(20)
    } else {
        (lines.len() + 4).min(screen.rows())
    };
    let visible = height.saturating_sub(3);
    let scroll = if let Some((row, _)) = cursor {
        row.saturating_sub(visible.saturating_sub(1))
    } else {
        app.letter_scroll.min(lines.len().saturating_sub(visible))
    };
    let hint = if writing {
        "Enter ↵ · Ctrl+S send · Esc keep draft"
    } else {
        "←→ · ↑↓ · w write · v sent · Esc close"
    };
    let box_width = (inner + 4).min(screen.columns());
    let left = (screen.columns() - box_width) / 2;
    let top = (screen.rows() - height) / 2;
    let end = left + box_width - 2;
    for row in top..top + height {
        fill(screen, row, left, left + box_width, paper);
    }
    for (offset, line) in lines.iter().skip(scroll).take(visible).enumerate() {
        write(
            screen,
            left + 2,
            top + 1 + offset,
            end,
            line,
            ink,
            Some(paper),
        );
    }
    if let Some((row, column)) = cursor {
        let under = lines
            .get_mut(row)
            .map(|line| {
                let mut used = 0;
                line.chars()
                    .find(|character| {
                        let at = used;
                        used += character.width().unwrap_or(0);
                        at == column
                    })
                    .unwrap_or(' ')
            })
            .unwrap_or(' ');
        write(
            screen,
            left + 2 + column,
            top + 1 + row - scroll,
            end,
            &under.to_string(),
            paper,
            Some(ink),
        );
    }
    write(
        screen,
        left + 2,
        top + height - 2,
        end,
        hint,
        ink,
        Some(paper),
    );
}

/// Hard-wrap editable text without losing spaces or explicit empty lines.
/// The cursor is a character boundary; returned columns are terminal cells.
fn draft_lines(draft: &crate::draft::Draft, columns: usize) -> (Vec<String>, (usize, usize)) {
    let columns = columns.max(1);
    let mut lines = vec![String::new()];
    let mut column = 0;
    let mut cursor = (0, 0);
    let length = draft.text().chars().count();
    for (index, character) in draft.text().chars().chain(std::iter::once(' ')).enumerate() {
        let cells = character.width().unwrap_or(0);
        if character != '\n' && column + cells > columns {
            lines.push(String::new());
            column = 0;
        }
        if index == draft.cursor() {
            cursor = (lines.len() - 1, column);
        }
        if index == length {
            break;
        }
        if character == '\n' {
            lines.push(String::new());
            column = 0;
        } else {
            lines.last_mut().unwrap().push(character);
            column += cells;
        }
    }
    (lines, cursor)
}

struct PanelBounds {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

fn draw_panel(
    screen: &mut Canvas,
    title: &str,
    lines: &[(String, Rgb)],
    inner: usize,
) -> Option<PanelBounds> {
    let columns = screen.columns();
    let rows = screen.rows();
    let box_width = inner + 4;
    let height = lines.len() + 2;
    if box_width > columns || height > rows {
        return None;
    }
    let left = (columns - box_width) / 2;
    let top = rows.saturating_sub(height) / 2;
    let right = left + box_width - 1;
    for row in top..top + height {
        fill(screen, row, left, left + box_width, PANEL);
        let edge = if row == top || row == top + height - 1 {
            '─'
        } else {
            ' '
        };
        for column in left..=right {
            let glyph = match (column, row) {
                (column, row) if column == left && row == top => '╭',
                (column, row) if column == right && row == top => '╮',
                (column, row) if column == left && row == top + height - 1 => '╰',
                (column, row) if column == right && row == top + height - 1 => '╯',
                (column, _) if column == left || column == right => '│',
                _ => edge,
            };
            screen.set(
                column,
                row,
                Cell {
                    glyph,
                    foreground: Some(BORDER),
                    background: Some(PANEL),
                },
            );
        }
    }
    write(
        screen,
        left + 2,
        top,
        right,
        &format!(" {title} "),
        TITLE,
        Some(PANEL),
    );
    for (offset, (line, color)) in lines.iter().enumerate() {
        write(
            screen,
            left + 2,
            top + 1 + offset,
            right - 1,
            line,
            *color,
            Some(PANEL),
        );
    }
    Some(PanelBounds {
        left,
        top,
        right,
        bottom: top + height - 1,
    })
}

fn help_panel(screen: &mut Canvas) {
    let lines: Vec<(String, Rgb)> = help_lines(screen.rows())
        .iter()
        .map(|line| {
            (
                (*line).to_owned(),
                if line.starts_with("Any key") {
                    DIM
                } else {
                    TEXT
                },
            )
        })
        .collect();
    panel(screen, "Help", &lines);
}

/// The help that fits `rows`: short screens keep the keys and leave out the
/// words around them.
fn help_lines(rows: usize) -> Vec<&'static str> {
    let introduction = [
        "A ghost lives here. Jev chooses; Claude gives thoughts and words.",
        "Without keys (k), instinct guides it. Speak Korean to hear Korean.",
    ];
    let keys = [
        "t, Enter   talk to it           click ghost   poke or find it",
        "k          keys and models      click ball    flick it",
        "j          journal and diary    click room    drop a light mote",
        "l          life and growth      click blocks  nudge or topple",
        "p          save a postcard      click friend  make it giggle",
        "o          focus (o ends it)    drag ghost    stroke it",
        "g          noughts and crosses  1-9, click    choose a square",
        "f          draw (f/Esc stops)   right-drag    draw on glass",
        "b, e       album, letters       click window  open or shut",
        "arrows     turn the view        drag          turn the view",
        "+ -, wheel zoom                 r             reset the view",
        "m  c       style, color         space         pause",
        "?          this help            q, Esc        quit",
    ];
    let notes = [
        "Ask for a word game; type a letter to guess in talk mode.",
        "In the journal: PgUp/PgDn, Home/End · Esc, Enter, j close it.",
        "Click the music box to wind a tune; teach it do re mi.",
        "Letters: ←/→ browse · w write, Ctrl+S sends · v sent",
        "Album: ←/→, Home/End · life panel: ↑/↓ scroll",
        "Tell it how you feel, or when you'll be back.",
    ];
    let room = rows.saturating_sub(3);
    let mut lines = keys.to_vec();
    if lines.len() + notes.len() <= room {
        lines.extend(notes);
    }
    if lines.len() + introduction.len() <= room {
        lines.splice(0..0, introduction);
    }
    lines.push("Any key closes this.");
    lines
}

fn settings_panel(app: &App, screen: &mut Canvas) {
    let settings = &app.settings;
    let saved_in = app
        .config_path()
        .map_or("nowhere (this run only)".to_owned(), |path| {
            path.display().to_string()
        });
    let current = |variable: &str, saved: &Option<String>| {
        if std::env::var(variable).is_ok_and(|key| !key.trim().is_empty()) {
            format!("set by {variable}")
        } else {
            saved.as_deref().map_or("none".to_owned(), mask)
        }
    };
    // Typed keys show only their last four characters.
    let hidden = |line: &crate::app::Line| {
        let count = line.text().chars().count();
        line.text()
            .chars()
            .enumerate()
            .map(|(position, character)| {
                if position + 4 < count {
                    '•'
                } else {
                    character
                }
            })
            .collect::<String>()
    };
    let mut lines = vec![
        (format!("Saved to {saved_in}, readable only by you."), DIM),
        (String::new(), TEXT),
    ];
    for field in Field::ALL {
        let value = match field {
            Field::ClaudeKey => format!(
                "[{:<34}]  now {}",
                hidden(&settings.claude_key),
                current("ANTHROPIC_API_KEY", &app.config.anthropic_api_key)
            ),
            Field::JevKey => format!(
                "[{:<34}]  now {}",
                hidden(&settings.jev_key),
                current("TYPESAFE_API_KEY", &app.config.typesafe_api_key)
            ),
            Field::ClaudeModel => format!("‹ {} ›", settings.claude_model.text()),
            Field::Effort => format!("‹ {} ›", effort_name(settings.effort)),
            Field::Cache => format!("‹ {} ›", cache_name(settings.cache)),
            Field::JevModel => format!("‹ {} ›", settings.jev_model.text()),
        };
        let focused = settings.focus == field;
        let marker = if focused { "›" } else { " " };
        lines.push((
            format!("{marker} {:<13} {value}", field.label()),
            if focused { TITLE } else { TEXT },
        ));
    }
    lines.push((String::new(), TEXT));
    lines.push((
        "↑↓ choose · ←→ change · type or paste a key, - removes it · Enter saves · Esc cancels"
            .to_owned(),
        DIM,
    ));
    panel(screen, "Settings", &lines);
}

/// The wrapped journal and its viewport, shared by drawing and scroll keys.
pub struct JournalPanel {
    lines: Vec<(String, Rgb)>,
    inner: usize,
    pub height: usize,
}

impl JournalPanel {
    pub fn new(app: &App, columns: usize, rows: usize) -> Self {
        let inner = columns.saturating_sub(6);
        let mut lines = Vec::new();
        let mut append = |prefix: &str, text: &str, color: Rgb| {
            let indentation = width(prefix);
            // On narrow screens, where the indentation would leave no room
            // for even one wide character, the prefix wraps with the text.
            let wrapped = if indentation + 2 > inner {
                wrap(&format!("{prefix}{text}"), inner)
            } else {
                wrap(text, inner - indentation)
                    .into_iter()
                    .enumerate()
                    .map(|(index, line)| {
                        let prefix = if index == 0 {
                            prefix.to_owned()
                        } else {
                            " ".repeat(indentation)
                        };
                        format!("{prefix}{line}")
                    })
                    .collect()
            };
            if wrapped.is_empty() {
                lines.push((String::new(), color));
            } else {
                lines.extend(wrapped.into_iter().map(|line| (line, color)));
            }
        };
        let ghost = &app.world.ghost;
        append(
            "",
            &format!("{} · together since {}", ghost.name, date(app.born())),
            TITLE,
        );
        append("", "", TEXT);
        append("", "Remembers", ACCENT);
        let memories = &app.mind.journal.memories;
        if memories.is_empty() {
            append("  ", "nothing yet", DIM);
        }
        for memory in memories {
            append("  ", &memory.text, TEXT);
        }
        append("", "", TEXT);
        append("", "Moments to remember", ACCENT);
        let milestones = &app.world.life.milestones;
        if milestones.is_empty() {
            append("  ", "none yet", DIM);
        }
        for milestone in milestones {
            append(&format!("  {}  ", milestone.day), &milestone.text, TEXT);
        }
        append("", "", TEXT);
        append("", "Diary", ACCENT);
        let diary = &app.world.life.diary;
        if diary.is_empty() {
            append("  ", "no days written yet", DIM);
        }
        for entry in diary {
            append(&format!("  {}  ", entry.day), &entry.text, TEXT);
        }
        append("", "", TEXT);
        append("", "Its own picture books", ACCENT);
        let korean = app.world.speaks_korean();
        for book in &app.world.life.written {
            let (first_title, second_title) = if korean {
                (&book.title.1, &book.title.0)
            } else {
                (&book.title.0, &book.title.1)
            };
            append(
                "  ",
                &format!(
                    "{first_title} / {second_title}{}",
                    if book.finished {
                        ""
                    } else {
                        " (unfinished draft)"
                    }
                ),
                ACCENT,
            );
            for page in &book.pages {
                let (first, second) = if korean {
                    (&page.1, &page.0)
                } else {
                    (&page.0, &page.1)
                };
                append("    ", first, TEXT);
                append("    ", second, DIM);
            }
        }
        append("", "", TEXT);
        append("", "Letters", ACCENT);
        for letter in &app.world.life.letters {
            append(
                &format!("  {}  ", letter.date_words()),
                &letter.lines[0],
                TEXT,
            );
        }
        for letter in &app.world.life.received {
            append(
                &format!("  {} · from you  ", letter.date_words()),
                &letter.lines[0],
                TEXT,
            );
        }
        append("", "", TEXT);
        append("", "Said lately", ACCENT);
        let conversation = &app.mind.journal.conversation;
        if conversation.is_empty() {
            append("  ", "nothing yet", DIM);
        }
        for remark in conversation {
            let speaker = match remark.speaker {
                Speaker::Viewer => "you",
                Speaker::Ghost => ghost.name.as_str(),
            };
            append(&format!("  {speaker}: "), &remark.text, TEXT);
        }
        append("", "", TEXT);
        append("", "Lately", ACCENT);
        for event in crate::mind::recent_events(&app.world, 10) {
            append("  ", &event, TEXT);
        }
        if let Status::Failed(message) = &app.mind.two {
            append("", "", TEXT);
            append("Claude: ", message, BAD);
        }
        if let Status::Failed(message) = &app.mind.one {
            append("Jev: ", message, BAD);
        }
        let height = if inner == 0 {
            0
        } else {
            rows.saturating_sub(4).min(lines.len())
        };
        Self {
            lines,
            inner,
            height,
        }
    }

    pub fn max_scroll(&self) -> usize {
        if self.height == 0 {
            0
        } else {
            self.lines.len().saturating_sub(self.height)
        }
    }

    fn visible_range(&self, scroll: usize) -> std::ops::Range<usize> {
        let end = self.lines.len() - scroll.min(self.max_scroll());
        end.saturating_sub(self.height)..end
    }

    fn draw(&self, screen: &mut Canvas, scroll: usize) {
        if self.height == 0 {
            return;
        }
        let visible = self.visible_range(scroll);
        let Some(bounds) = draw_panel(screen, "Journal", &self.lines[visible.clone()], self.inner)
        else {
            return;
        };
        if self.max_scroll() == 0 {
            return;
        }
        let range = format!(
            " {}–{} of {} ",
            visible.start + 1,
            visible.end,
            self.lines.len()
        );
        write(
            screen,
            bounds.left + 2,
            bounds.bottom,
            bounds.right.saturating_sub(2),
            &range,
            DIM,
            Some(PANEL),
        );
        for (row, marker, more) in [
            (bounds.top, "▲", visible.start > 0),
            (bounds.bottom, "▼", visible.end < self.lines.len()),
        ] {
            if more {
                write(
                    screen,
                    bounds.right - 1,
                    row,
                    bounds.right,
                    marker,
                    ACCENT,
                    Some(PANEL),
                );
            }
        }
    }
}

// The ghost's life at a glance: its age and stage, its skills, its bond, what it
// loves doing, what it has done in all its days, its latest milestone, and
// today so far.
fn life_panel(app: &App, screen: &mut Canvas) {
    let lines = life_lines(app, screen.columns(), usize::MAX);
    let page = life_page(screen.rows());
    if lines.len() <= page {
        panel(screen, "Life", &lines);
        return;
    }
    // Taller than the screen: a window onto it, scrolled with the arrows.
    let scroll = app.life_scroll.min(lines.len() - page);
    let title = format!(
        "Life · {}–{} of {} · ↑↓",
        scroll + 1,
        scroll + page,
        lines.len()
    );
    panel(screen, &title, &lines[scroll..scroll + page]);
}

// Lines of the life panel a screen of `rows` shows at once, inside its
// border with a line to spare above and below.
fn life_page(rows: usize) -> usize {
    rows.saturating_sub(4).max(1)
}

/// How many lines the life panel holds on a screen of `columns`, and how
/// many fit at once on one of `rows`, for scrolling it.
pub fn life_extent(app: &App, columns: usize, rows: usize) -> (usize, usize) {
    (life_lines(app, columns, usize::MAX).len(), life_page(rows))
}

pub fn album_panel(app: &App, screen: &mut Canvas) {
    let columns = screen.columns();
    let rows = screen.rows();
    if columns == 0 || rows == 0 {
        return;
    }
    let Some(page) = app.album.pages.get(app.album.index) else {
        write(screen, 0, 0, columns, "Album", TITLE, None);
        write(
            screen,
            0,
            rows / 2,
            columns,
            "The ghost hasn't kept any moments yet.",
            TEXT,
            None,
        );
        write(
            screen,
            0,
            rows - 1,
            columns,
            "Esc or b closes · q quits",
            DIM,
            None,
        );
        return;
    };
    write(
        screen,
        0,
        0,
        columns,
        &format!("Album · {}", page.name),
        TITLE,
        None,
    );
    let mut caption = wrap(&page.caption, columns);
    caption.truncate(rows.saturating_sub(4));
    let picture_rows = rows.saturating_sub(caption.len() + 4);
    let picture = page.canvas();
    let width = columns.min(picture.columns());
    let height = picture_rows.min(picture.rows());
    let source_column = picture.columns().saturating_sub(width) / 2;
    let source_row = picture.rows().saturating_sub(height) / 2;
    let column = columns.saturating_sub(width) / 2;
    let row = 1 + picture_rows.saturating_sub(height) / 2;
    for down in 0..height {
        for across in 0..width {
            let cell = picture
                .get(source_column + across, source_row + down)
                .unwrap();
            // A crop must keep both halves of a wide character together.
            if (across == 0 && cell == Cell::CONTINUATION)
                || (across + 1 == width && cell.glyph.width() == Some(2))
            {
                continue;
            }
            screen.set(column + across, row + down, cell);
        }
    }
    let mut row = 1 + picture_rows;
    for line in caption {
        write(screen, 0, row, columns, &line, TEXT, None);
        row += 1;
    }
    write(screen, 0, row, columns, &page.date_words(), DIM, None);
    write(
        screen,
        0,
        row + 1,
        columns,
        &format!("{} of {}", app.album.index + 1, app.album.pages.len()),
        DIM,
        None,
    );
    write(
        screen,
        0,
        rows - 1,
        columns,
        "←/→ flip · Home/End · Esc or b closes · q quits",
        DIM,
        None,
    );
}

/// The life panel's lines for a screen of `columns` by `rows`. Sections that
/// don't fit are left out, the least needed first.
// The viewer's words the ghost knows, the most heard first, as many as fit
// a line of the life panel.
fn known_words(life: &crate::world::Life, columns: usize) -> String {
    let known = life.vocabulary.known();
    if known.is_empty() {
        return "none of your words yet".to_owned();
    }
    let room = columns.saturating_sub(24).clamp(16, 52);
    let mut line = String::new();
    for (shown, (word, _)) in known.iter().enumerate() {
        let more = format!(" and {} more", known.len() - shown);
        let next = if line.is_empty() {
            format!("“{word}”")
        } else {
            format!("{line}, “{word}”")
        };
        let rest = if shown + 1 < known.len() {
            more.len()
        } else {
            0
        };
        if !line.is_empty() && UnicodeWidthStr::width(next.as_str()) + rest > room {
            return line + &more;
        }
        line = next;
    }
    line
}

fn life_lines(app: &App, columns: usize, rows: usize) -> Vec<(String, Rgb)> {
    let life = &app.world.life;
    let ghost = &app.world.ghost;
    let days = (crate::mind::now() - app.born()).max(0) / 86_400;
    let age = match days {
        0 => "less than a day old".to_owned(),
        1 => "one day old".to_owned(),
        days => format!("{days} days old"),
    };
    let life_day = app.world.day_of_life().map_or_else(String::new, |day| {
        format!(" · its {} day", crate::world::anniversary::ordinal(day))
    });
    let mut header = vec![
        (
            format!(
                "{} · {age}{life_day} · {} · born {}",
                ghost.name,
                app.world.stage().words(),
                date(app.born())
            ),
            TITLE,
        ),
        (
            app.world.occasion().map_or_else(String::new, |_| {
                format!("Today     {}", app.world.occasion_words())
            }),
            TEXT,
        ),
        (
            format!(
                "Nature    {}{}",
                life.temperament().words(),
                life.upbringing
                    .words()
                    .map_or_else(String::new, |change| format!(", {change}"))
            ),
            TEXT,
        ),
        (
            format!(
                "Bond      {}  {}",
                meter(life.bond),
                life.bond_words().replace("them", "you")
            ),
            TEXT,
        ),
        (
            format!(
                "Together  {} in all, {} today",
                hours_and_minutes(life.total.together),
                hours_and_minutes(life.today.together)
            ),
            TEXT,
        ),
        (
            format!(
                "Loves     {}",
                life.favorite()
                    .map_or("nothing in particular yet", |activity| activity.doing())
            ),
            TEXT,
        ),
        (format!("Knows     {}", known_words(life, columns)), TEXT),
    ];
    if header.len() < rows.saturating_sub(2) {
        header.push((
            format!(
                "Album     {} moments kept · b to open",
                app.album.pages.len()
            ),
            TEXT,
        ));
    }
    if let Some(usual) = life.usual_time()
        && header.len() < rows.saturating_sub(2)
    {
        header.push((format!("Visits    usually {usual}"), TEXT));
    }
    // Longer lines continue under their text, past the label's column.
    let labeled = |label: &str, words: String| -> Vec<(String, Rgb)> {
        wrap(&words, columns.saturating_sub(20).clamp(20, 52))
            .into_iter()
            .enumerate()
            .map(|(index, line)| {
                let label = if index == 0 { label } else { "" };
                (format!("{label:<10}{line}"), TEXT)
            })
            .collect()
    };
    header.extend(labeled(
        "Wish",
        format!(
            "{} · {} came true",
            life.wish.as_ref().map_or_else(
                || "nothing just now".to_owned(),
                |wishing| wishing.wish.words(life)
            ),
            crate::world::friend::count_words(life.total.wishes_granted)
        ),
    ));
    if life.total.focus_sessions > 0 || life.total.focus_minutes > 0 {
        header.extend(labeled(
            "Focus",
            format!(
                "{} sessions · {} together",
                life.total.focus_sessions,
                hours_and_minutes(f64::from(life.total.focus_minutes) * 60.0)
            ),
        ));
    }
    let korean = app.world.speaks_korean();
    let mut viewer = Vec::new();
    if let Some(name) = &life.viewer_name {
        viewer.push(name.clone());
    }
    if korean {
        viewer.push("talks to it in Korean".to_owned());
    }
    for topic in crate::world::acquaintance::Topic::ALL {
        if let Some(answer) = life.viewer.answer(topic, false) {
            viewer.push(format!("{}: {answer}", topic.name()));
        }
    }
    if !viewer.is_empty() {
        header.extend(labeled("You", viewer.join(" · ")));
    }
    let practiced: Vec<String> = Skill::ALL
        .into_iter()
        .filter(|skill| life.skills.practice(*skill) > 0.0)
        .map(|skill| life.skills.description(skill))
        .collect();
    if !practiced.is_empty() {
        header.extend(labeled("Skills", practiced.join(", ")));
    }
    if let Some(friend) = &life.friend {
        let mut words = format!(
            "{} · {} · visited {}",
            friend.name,
            friend.closeness_words(),
            friend.visit_words()
        );
        if life.total.friend_games > 0 {
            words.push_str(&format!(
                " · played together {}",
                crate::world::friend::count_words(life.total.friend_games)
            ));
        }
        header.extend(labeled("Friend", words));
    }
    if life.mirror.sessions > 0 || life.total.met_reflection {
        header.extend(labeled("Mirror", life.mirror.words().to_owned()));
    }
    header.extend(labeled("Books", app.world.books_words()));
    if app.world.life.cranes.total > 0 || app.world.life.cranes.paper.is_some() {
        header.extend(labeled(
            "Cranes",
            format!("{} folded for you", app.world.life.cranes.total),
        ));
    }
    if !life.tunes.is_empty() || !life.compositions.is_empty() {
        let own = app.world.composition_words(false);
        let words = if own.is_empty() {
            app.world.learned_tune_words()
        } else {
            format!("{}; made up {own}", app.world.learned_tune_words())
        };
        header.extend(labeled("Tunes", words));
    }
    let garden = labeled("Garden", app.world.garden_words());
    let sky = if life.constellations.is_empty() {
        Vec::new()
    } else {
        labeled(
            "Its sky",
            life.constellations
                .iter()
                .map(|constellation| constellation.name.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        )
    };
    let total = &life.total;
    let mut bird = labeled("Bird", life.bird_words().to_owned());
    bird.extend(labeled("", life.keepsake_words()));
    let mut counts = vec![
        (String::new(), TEXT),
        ("In all its days".to_owned(), ACCENT),
    ];
    for (first, first_count, second, second_count) in [
        (
            "light motes eaten",
            total.motes,
            "ball nudges",
            total.nudges,
        ),
        ("fetches", total.fetches, "dances", total.dances),
        (
            "tunes played",
            total.tunes_played,
            "tunes learned",
            total.tunes_learned,
        ),
        ("found hiding", total.finds, "finger pounces", total.pounces),
        ("doodles", total.doodles, "words written", total.writings),
        (
            "drawings seen",
            total.drawings_seen,
            "pictures learned",
            total.pictures_learned,
        ),
        ("plant watered", total.waterings, "flowers", total.blooms),
        ("naps", total.naps, "nights slept", total.nights),
        ("messages from you", total.heard, "pokes", total.pokes),
        (
            "strokes from you",
            total.pets,
            "words learned",
            total.learned,
        ),
        (
            "storms sat out",
            total.storms,
            "rainbows seen",
            total.rainbows,
        ),
        (
            "celebrations",
            total.celebrations,
            "birds watched",
            total.birds,
        ),
        ("moths seen", total.moths, "gifts kept", total.gifts),
        ("games won", total.wins, "games lost", total.losses),
        ("games drawn", total.draws, "towers built", total.towers),
        ("tallest tower", total.tallest, "you toppled", total.toppled),
        (
            "expected visits",
            total.expected,
            "visits missed",
            total.missed,
        ),
        (
            "snowfalls seen",
            total.snowfalls,
            "shooting stars",
            total.shooting_stars,
        ),
        (
            "stories told",
            total.stories,
            "constellations",
            total.constellations,
        ),
    ] {
        counts.push((
            format!("  {first:<18}{first_count:>5}     {second:<14}{second_count:>5}"),
            TEXT,
        ));
    }
    if total.moon_wishes > 0 {
        counts.push((format!("  Moon wishes: {}", total.moon_wishes), TEXT));
    }
    let latest = match life.milestones.last() {
        Some(milestone) => vec![
            (String::new(), TEXT),
            ("Its latest moment to remember".to_owned(), ACCENT),
            (format!("  {}", milestone.text), TEXT),
        ],
        None => Vec::new(),
    };
    let mut today = vec![(String::new(), TEXT), ("Today so far".to_owned(), ACCENT)];
    let room = columns.saturating_sub(10).clamp(20, 56);
    for line in wrap(&life.today.words(), room) {
        today.push((format!("  {line}"), TEXT));
    }
    let footer = vec![
        (String::new(), TEXT),
        (
            "Its diary and moments are in the journal (j). Any key closes this.".to_owned(),
            DIM,
        ),
    ];

    // In the order they show, with the order they are kept in when space
    // runs short.
    let mut sections = [
        (header, 0),
        (garden, 2),
        (sky, 3),
        (bird, 2),
        (counts, 4),
        (latest, 3),
        (today, 1),
        (footer, 5),
    ];
    let mut room = rows.saturating_sub(2);
    let mut kept = [false; 8];
    let mut by_need: Vec<usize> = (0..sections.len()).collect();
    by_need.sort_by_key(|index| sections[*index].1);
    for index in by_need {
        let length = sections[index].0.len();
        if length <= room {
            room -= length;
            kept[index] = true;
        } else if index == 0 {
            // A long header keeps its first lines, which say who it is.
            if room > 0 {
                sections[0].0.truncate(room);
                kept[0] = true;
            }
            break;
        }
    }
    sections
        .into_iter()
        .zip(kept)
        .filter(|(_, kept)| *kept)
        .flat_map(|((lines, _), _)| lines)
        .collect()
}

fn hours_and_minutes(seconds: f64) -> String {
    let minutes = (seconds / 60.0) as u32;
    match (minutes / 60, minutes % 60) {
        (0, minutes) => format!("{minutes} min"),
        (hours, 0) => format!("{hours} h"),
        (hours, minutes) => format!("{hours} h {minutes} min"),
    }
}

fn journal_panel(app: &mut App, screen: &mut Canvas) {
    let journal = JournalPanel::new(app, screen.columns(), screen.rows());
    app.journal_scroll = app.journal_scroll.min(journal.max_scroll());
    journal.draw(screen, app.journal_scroll);
}

fn date(unix: i64) -> String {
    jiff::Timestamp::from_second(unix)
        .map(|timestamp| {
            timestamp
                .to_zoned(jiff::tz::TimeZone::system())
                .date()
                .to_string()
        })
        .unwrap_or_else(|_| "long ago".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Style;
    use crate::color::ColorMode;
    use crate::mind::{Journal, Mind};
    use crate::store::{Config, Memory, Remark};
    use crate::world::World;

    fn app() -> App {
        App::new(
            World::new(1, 12.0),
            Mind::new(&Config::default(), Journal::default(), true),
            Config::default(),
            Style::Ascii,
            ColorMode::Plain,
        )
    }

    #[test]
    fn editable_wrapping_keeps_spaces_blank_lines_and_counts_wide_cursor_cells() {
        let mut draft = crate::draft::Draft::default();
        draft.insert("안녕 ab\n\n x");
        let (lines, cursor) = draft_lines(&draft, 4);
        assert_eq!(lines, ["안녕", " ab", "", " x"]);
        assert_eq!(cursor, (3, 2));
        draft.edit(&crate::terminal::Key::Up);
        assert_eq!(draft_lines(&draft, 4).1, (2, 0));
        draft.edit(&crate::terminal::Key::Up);
        assert_eq!(draft_lines(&draft, 4).1, (1, 0));
        assert_eq!(draft_lines(&draft, 20).1, (0, 4));
        draft.edit(&crate::terminal::Key::End);
        assert_eq!(draft_lines(&draft, 4).1, (1, 3));
    }

    #[test]
    fn emotes_and_speech_handle_tiny_scenes_and_heads_outside_the_view() {
        let mut app = app();
        app.world.ghost.asleep = 1.0;
        app.world.say("A few words beside the rising symbols.");
        let view = crate::render::Camera::default().view(Grid::cells(80, 24));
        let position = app.world.ghost.position;
        for head in [
            position,
            position + Vec3::X * 100.0,
            position + Vec3::Y * 100.0,
            position - Vec3::Y * 100.0,
            view.eye - view.forward,
        ] {
            app.world.ghost.position = head;
            for columns in [0, 1, 2, 5, 80] {
                for rows in [0, 1, 2, 5, 24] {
                    let mut screen = Canvas::blank(columns, rows);
                    let scene_rows = rows.saturating_sub(2).max(1);
                    let bubble_right = speech_bubble(&app, &mut screen, &view, scene_rows);
                    emote_puffs(&app, &mut screen, &view, scene_rows, bubble_right);
                }
            }
        }
    }

    #[test]
    fn a_narrow_journal_still_shows_every_line_of_wide_text() {
        let mut app = app();
        app.mind.journal.conversation.push(Remark {
            at: 0,
            speaker: crate::store::Speaker::Viewer,
            text: "안녕하세요 반가워요".into(),
        });
        let journal = JournalPanel::new(&app, 14, 30);
        let said: Vec<&String> = journal
            .lines
            .iter()
            .map(|(line, _)| line)
            .skip_while(|line| !line.contains("you"))
            .take_while(|line| !line.is_empty())
            .collect();
        assert!(said.len() > 1, "{said:?}");
        assert!(said.iter().all(|line| !line.trim().is_empty()), "{said:?}");
        assert!(said.iter().all(|line| width(line) <= 8), "{said:?}");
    }

    #[test]
    fn a_long_journal_remark_wraps_without_losing_text_and_indents_continuations() {
        let mut app = app();
        let text = "The ghost remembers a long conversation about the light in the room";
        app.mind.journal.conversation.push(Remark {
            at: 0,
            speaker: Speaker::Viewer,
            text: text.into(),
        });
        let journal = JournalPanel::new(&app, 30, 12);
        let start = journal
            .lines
            .iter()
            .position(|(line, _)| line.starts_with("  you: "))
            .unwrap();
        let lines: Vec<&str> = journal.lines[start..]
            .iter()
            .take_while(|(line, _)| !line.is_empty())
            .map(|(line, _)| line.as_str())
            .collect();
        let indentation = width("  you: ");
        assert!(lines.len() > 2);
        assert!(lines.iter().all(|line| width(line) <= journal.inner));
        assert!(lines[1..].iter().all(|line| line.starts_with("       ")));
        assert_eq!(
            lines
                .iter()
                .map(|line| line[indentation..].trim())
                .collect::<Vec<_>>()
                .join(" "),
            text
        );
    }

    #[test]
    fn the_journal_keeps_all_entries_ten_recent_events_and_model_errors() {
        let mut app = app();
        for number in 0..20 {
            app.mind.journal.memories.push(Memory {
                at: 0,
                text: format!("Memory {number}"),
            });
            app.mind.journal.conversation.push(Remark {
                at: 0,
                speaker: Speaker::Ghost,
                text: format!("Remark {number}"),
            });
            app.world.say(&format!("Event {number}"));
        }
        app.mind.one = Status::Failed("a Jev error".into());
        app.mind.two = Status::Failed("a Claude error".into());
        let journal = JournalPanel::new(&app, 100, 32);
        let lines: Vec<&str> = journal
            .lines
            .iter()
            .map(|(line, _)| line.as_str())
            .collect();
        for number in 0..20 {
            assert!(lines.contains(&format!("  Memory {number}").as_str()));
            assert!(
                lines.contains(&format!("  {}: Remark {number}", app.world.ghost.name).as_str())
            );
        }
        let events = crate::mind::recent_events(&app.world, 10);
        assert_eq!(events.len(), 10);
        for event in events {
            assert!(lines.contains(&format!("  {event}").as_str()));
        }
        assert_eq!(
            lines.iter().filter(|line| line.contains("Event ")).count(),
            10
        );
        assert!(lines.contains(&"Claude: a Claude error"));
        assert!(lines.contains(&"Jev: a Jev error"));
        assert_eq!(journal.visible_range(0).end, journal.lines.len());
        assert_eq!(journal.visible_range(usize::MAX).start, 0);
    }

    #[test]
    fn wide_characters_take_two_cells_and_stop_at_the_end() {
        let mut screen = Canvas::blank(8, 1);
        let end = write(&mut screen, 0, 0, 7, "안녕ab", TEXT, None);
        assert_eq!(end, 6);
        assert_eq!(screen.text(), "안녕ab  \n");
        let end = write(&mut screen, 0, 0, 3, "안녕", TEXT, None);
        assert_eq!(end, 2);
    }

    #[test]
    fn wrapping_breaks_between_words_and_inside_long_ones() {
        assert_eq!(wrap("hello there you", 11), ["hello there", "you"]);
        assert_eq!(wrap("abcdefgh", 3), ["abc", "def", "gh"]);
        assert_eq!(wrap("안녕하세요", 4), ["안녕", "하세", "요"]);
        assert!(wrap("   ", 5).is_empty());
    }

    #[test]
    fn the_talk_line_scrolls_to_keep_the_cursor_in_view() {
        let text: Vec<char> = "hello 안녕하세요".chars().collect();
        assert_eq!(visible_start(&text, 3, 10), 0);
        // Ten cells before the cursor; "안녕하세요" alone takes ten.
        assert_eq!(visible_start(&text, text.len(), 11), 6);
        assert_eq!(visible_start(&text, text.len(), 10), 7);
        assert_eq!(visible_start(&text, 0, 5), 0);
    }

    #[test]
    fn a_long_sky_and_header_still_leave_the_life_panel_its_first_lines() {
        let mut app = app();
        let life = &mut app.world.life;
        life.viewer_name = Some("Somebody With A Long Name".to_owned());
        life.total.focus_sessions = 3;
        life.skills.stacking = 50.0;
        life.skills.writing = 50.0;
        for index in 0..12 {
            life.constellations
                .push(crate::world::stars::Constellation {
                    name: format!("the Very Sleepy Dancing Lost Heart Number {index}"),
                    korean: "하트".to_owned(),
                    stars: vec![0, 1, 2],
                    found: "2026-06-21".to_owned(),
                });
        }
        for rows in [24, 12, 6] {
            let lines = life_lines(&app, 80, rows);
            assert!(lines.len() + 2 <= rows, "{rows}: {}", lines.len());
            assert!(lines[0].0.starts_with("Boo"), "{rows}");
        }
    }

    #[test]
    fn a_quiet_words_line_shows_the_basics_first_then_every_other_tip_in_turn() {
        assert_eq!(idle_tip(0.0), TIPS[0]);
        assert_eq!(idle_tip(BASICS_SHOW - 0.1), TIPS[0]);
        let later: Vec<&str> = (0..TIPS.len() - 1)
            .map(|turn| idle_tip(BASICS_SHOW + TIP_SHOWS * (turn as f64 + 0.5)))
            .collect();
        assert_eq!(later, TIPS[1..]);
        assert_eq!(
            idle_tip(BASICS_SHOW + TIP_SHOWS * (TIPS.len() as f64 - 0.5)),
            TIPS[1]
        );
    }

    #[test]
    fn the_life_panel_leaves_out_what_does_not_fit_the_screen() {
        let mut app = app();
        app.world.life.milestones.push(crate::world::Entry {
            day: "2026-10-21".into(),
            text: "Saw my first rainbow.".into(),
        });
        let full = life_lines(&app, 100, 60);
        for rows in [60, 24, 16, 12, 9] {
            let lines = life_lines(&app, 100, rows);
            assert!(lines.len() + 2 <= rows, "{rows}: {}", lines.len());
            assert!(lines[0].0.starts_with("Boo"), "{rows}");
        }
        assert!(life_lines(&app, 100, 20).len() < full.len());
        assert!(
            life_lines(&app, 100, 16)
                .iter()
                .any(|(line, _)| line == "Today so far")
        );
        assert_eq!(life_lines(&app, 100, 4).len(), 2);
        assert!(life_lines(&app, 100, 2).is_empty());
    }

    #[test]
    fn help_fits_a_standard_terminal_and_keeps_its_keys_on_short_ones() {
        let full = help_lines(60);
        assert!(full.len() + 2 <= 24);
        assert_eq!(help_lines(24), full);
        for rows in [20, 17, 16] {
            let lines = help_lines(rows);
            assert!(lines.len() + 2 <= rows, "{rows}: {}", lines.len());
            assert!(lines.iter().any(|line| line.starts_with("t, Enter")));
            assert!(lines.iter().any(|line| line.starts_with("? ")));
        }
    }

    #[test]
    fn time_together_reads_in_hours_and_minutes() {
        assert_eq!(hours_and_minutes(59.0), "0 min");
        assert_eq!(hours_and_minutes(25.0 * 60.0), "25 min");
        assert_eq!(hours_and_minutes(2.0 * 3600.0), "2 h");
        assert_eq!(hours_and_minutes(4.0 * 3600.0 + 600.0), "4 h 10 min");
    }

    #[test]
    fn meters_show_five_steps() {
        assert_eq!(meter(0.0), "▱▱▱▱▱");
        assert_eq!(meter(0.62), "▰▰▰▱▱");
        assert_eq!(meter(1.0), "▰▰▰▰▰");
    }
}
