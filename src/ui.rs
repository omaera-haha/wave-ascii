use glam::Vec3;
use unicode_width::UnicodeWidthChar;

use crate::app::{App, Mode, mood_color};
use crate::canvas::{Canvas, Cell};
use crate::color::Rgb;
use crate::frame::{CELL_ASPECT, Grid};
use crate::mind::Status;
use crate::render::{GHOST_SCALE, View};
use crate::settings::{Field, cache_name, effort_name};
use crate::store::{Speaker, mask};

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

/// Draws the words around the scene: the ghost's speech bubble, the line
/// under the scene, the status bar, and any open panel.
pub fn draw(app: &App, screen: &mut Canvas, view: &View, scene_rows: usize) {
    speech_bubble(app, screen, view, scene_rows);
    words_line(app, screen, scene_rows);
    status_line(app, screen, scene_rows + 1);
    match app.mode {
        Mode::Help => help_panel(screen),
        Mode::Settings => settings_panel(app, screen),
        Mode::Journal => journal_panel(app, screen),
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

// The ghost's latest words in a bubble above its head.
fn speech_bubble(app: &App, screen: &mut Canvas, view: &View, scene_rows: usize) {
    let world = &app.world;
    let Some(speech) = &world.ghost.speech else {
        return;
    };
    if world.time - speech.time > SPEECH_SHOWS {
        return;
    }
    let head = world.ghost.position + Vec3::Y * 0.8 * GHOST_SCALE;
    let Some(point) = view.project(head) else {
        return;
    };
    let columns = screen.columns();
    let pixel = Grid::new(columns, scene_rows, CELL_ASPECT)
        .to_pixels()
        .transform_point2(point);
    let lines = wrap(&speech.text, 34.min(columns.saturating_sub(4)));
    let inner = lines.iter().map(|line| width(line)).max().unwrap_or(0);
    let box_width = inner + 2;
    if lines.is_empty() || box_width > columns {
        return;
    }
    let anchor = pixel.x.round().clamp(0.0, columns as f32 - 1.0) as usize;
    let tail_row = (pixel.y.round() as isize - 1).max(lines.len() as isize);
    let top = tail_row as usize - lines.len();
    let left = anchor
        .saturating_sub(box_width / 2)
        .min(columns - box_width);
    for (offset, line) in lines.iter().enumerate() {
        let row = top + offset;
        if row >= scene_rows {
            return;
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
        // Scroll so the cursor stays in view.
        let mut start = 0;
        while start < app.talk.cursor()
            && width(&text[start..app.talk.cursor()].iter().collect::<String>()) >= room
        {
            start += 1;
        }
        let before: String = text[start..app.talk.cursor()].iter().collect();
        let after: String = text[app.talk.cursor()..].iter().collect();
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
        None => "t to talk · click the ghost to poke it, the floor to drop a light mote · drag to look around"
            .to_owned(),
    };
    write(screen, 1, row, columns, &line, DIM, None);
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
    let doing = if ghost.asleep > 0.5 {
        "asleep"
    } else {
        ghost.plan.activity.doing()
    };
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
    let columns = screen.columns();
    let rows = screen.rows();
    let inner = lines
        .iter()
        .map(|(line, _)| width(line))
        .max()
        .unwrap_or(0)
        .max(width(title) + 4)
        .min(columns.saturating_sub(6));
    let box_width = inner + 4;
    let height = lines.len() + 2;
    if box_width > columns || height > rows {
        return;
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
}

fn help_panel(screen: &mut Canvas) {
    let lines = [
        "A ghost lives in this glass box. Jev (System One) makes its quick",
        "choices, and Claude (System Two) gives it thoughts and words.",
        "Without keys it lives on instinct. Press k to add keys.",
        "Models, effort, and caching change in the same place, at once.",
        "",
        "t, Enter   talk to it          click ghost   poke it",
        "k          keys and models     click ball    flick it",
        "j          journal             click room    drop a light mote",
        "arrows     turn the view       drag          turn the view",
        "+ -        zoom                wheel         zoom",
        "m  c       style, color        space         pause",
        "r          reset the view      q, Esc        quit",
        "",
        "Any key closes this.",
    ];
    let lines: Vec<(String, Rgb)> = lines
        .iter()
        .map(|line| {
            (
                (*line).to_owned(),
                if line.starts_with("Any") { DIM } else { TEXT },
            )
        })
        .collect();
    panel(screen, "Help", &lines);
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

fn journal_panel(app: &App, screen: &mut Canvas) {
    let ghost = &app.world.ghost;
    let mut lines = vec![(
        format!("{} · together since {}", ghost.name, date(app.born())),
        TITLE,
    )];
    lines.push((String::new(), TEXT));
    lines.push(("Remembers".to_owned(), ACCENT));
    let memories = &app.mind.journal.memories;
    if memories.is_empty() {
        lines.push(("  nothing yet".to_owned(), DIM));
    }
    for memory in memories.iter().rev().take(6).rev() {
        lines.push((format!("  {}", memory.text), TEXT));
    }
    lines.push((String::new(), TEXT));
    lines.push(("Said lately".to_owned(), ACCENT));
    let conversation = &app.mind.journal.conversation;
    if conversation.is_empty() {
        lines.push(("  nothing yet".to_owned(), DIM));
    }
    for remark in conversation.iter().rev().take(8).rev() {
        let speaker = match remark.speaker {
            Speaker::Viewer => "you",
            Speaker::Ghost => ghost.name.as_str(),
        };
        lines.push((format!("  {speaker}: {}", remark.text), TEXT));
    }
    lines.push((String::new(), TEXT));
    lines.push(("Lately".to_owned(), ACCENT));
    for event in crate::mind::recent_events(&app.world, 5) {
        lines.push((format!("  {event}"), TEXT));
    }
    if let Status::Failed(message) = &app.mind.two {
        lines.push((String::new(), TEXT));
        lines.push((format!("Claude: {message}"), BAD));
    }
    if let Status::Failed(message) = &app.mind.one {
        lines.push((format!("Jev: {message}"), BAD));
    }
    let limit = screen.rows().saturating_sub(4);
    lines.truncate(limit);
    panel(screen, "Journal", &lines);
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
    fn meters_show_five_steps() {
        assert_eq!(meter(0.0), "▱▱▱▱▱");
        assert_eq!(meter(0.62), "▰▰▰▱▱");
        assert_eq!(meter(1.0), "▰▰▰▰▰");
    }
}
