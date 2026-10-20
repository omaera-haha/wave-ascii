use crate::canvas::{Canvas, Cell};
use crate::color::Rgb;
use crate::terminal::Key;

// Plain output drops these colors, so the highlighted entry also gets a marker.
const PANEL: Rgb = Rgb([17, 19, 27]);
const BORDER: Rgb = Rgb([74, 82, 108]);
const TITLE: Rgb = Rgb([238, 240, 246]);
const HEADING: Rgb = Rgb([126, 154, 214]);
const TEXT: Rgb = Rgb([196, 201, 213]);
const SHORTCUT: Rgb = Rgb([106, 114, 134]);
const HIGHLIGHT: Rgb = Rgb([255, 178, 94]);
const HIGHLIGHT_TEXT: Rgb = Rgb([28, 20, 12]);
const MARKER: char = '▸';
// An entry row is ` ▸ 1  label ` between the borders.
const ENTRY_PADDING: usize = 9;
const MINIMUM_WIDTH: usize = 18;

/// A scene as the menu lists it.
pub struct Entry {
    pub label: String,
    /// Consecutive entries in the same group share one heading.
    pub group: &'static str,
}

/// What a key did to the open menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// The highlight moved to this entry, which plays as a preview.
    Highlighted(usize),
    /// The entry was picked, and the menu closed.
    Chosen(usize),
    /// The menu closed without a choice, going back to the entry it opened on.
    Dismissed(usize),
    /// The menu has no use for the key, or is closed.
    Ignored,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    Gap,
    Heading(&'static str),
    Entry(usize),
}

/// A list of scenes in a panel at the left of the screen, browsed with the
/// arrow keys rather than searched. The highlighted scene plays beside it.
pub struct Menu {
    entries: Vec<Entry>,
    lines: Vec<Line>,
    open: bool,
    selected: usize,
    original: usize,
    // The first list line in view and how many lines fit, as of the last draw.
    scroll: usize,
    page: usize,
}

impl Menu {
    pub fn new(entries: Vec<Entry>) -> Self {
        let mut lines = Vec::new();
        for (index, entry) in entries.iter().enumerate() {
            if index == 0 || entries[index - 1].group != entry.group {
                if index > 0 {
                    lines.push(Line::Gap);
                }
                lines.push(Line::Heading(entry.group));
            }
            lines.push(Line::Entry(index));
        }
        Self {
            entries,
            lines,
            open: false,
            selected: 0,
            original: 0,
            scroll: 0,
            page: 10,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the menu with `current` highlighted.
    pub fn open(&mut self, current: usize) {
        self.open = !self.entries.is_empty();
        self.selected = current.min(self.entries.len().saturating_sub(1));
        self.original = self.selected;
    }

    pub fn handle(&mut self, key: Key) -> Action {
        if !self.open {
            return Action::Ignored;
        }
        let count = self.entries.len();
        let last = count - 1;
        self.selected = match key {
            Key::Up | Key::BackTab | Key::Character(b'k' | b'w' | b'p') => {
                (self.selected + last) % count
            }
            Key::Down | Key::Character(b'\t' | b'j' | b's' | b'n') => (self.selected + 1) % count,
            Key::Home | Key::Character(b'g') => 0,
            Key::End | Key::Character(b'G') => last,
            Key::PageUp => self.selected.saturating_sub(self.page),
            Key::PageDown => (self.selected + self.page).min(last),
            Key::Enter => {
                self.open = false;
                return Action::Chosen(self.selected);
            }
            Key::Character(digit @ b'1'..=b'9') if usize::from(digit - b'1') < count => {
                self.open = false;
                self.selected = usize::from(digit - b'1');
                return Action::Chosen(self.selected);
            }
            Key::Escape => {
                self.open = false;
                self.selected = self.original;
                return Action::Dismissed(self.original);
            }
            _ => return Action::Ignored,
        };
        Action::Highlighted(self.selected)
    }

    /// Columns the panel takes from a screen `columns` wide: enough for the
    /// longest label, but no more than half a wide screen.
    pub fn width(&self, columns: usize) -> usize {
        let longest = self
            .entries
            .iter()
            .map(|entry| entry.label.chars().count().max(entry.group.chars().count()))
            .max()
            .unwrap_or(0);
        (longest + ENTRY_PADDING)
            .max(MINIMUM_WIDTH)
            .min((columns / 2).max(MINIMUM_WIDTH))
            .min(columns)
    }

    /// Lays out a `columns` x `rows` screen with the panel at the left and the
    /// canvas from `preview`, given the remaining columns and rows, beside it.
    pub fn draw(
        &mut self,
        columns: usize,
        rows: usize,
        preview: impl FnOnce(usize, usize) -> Canvas,
    ) -> Canvas {
        let width = self.width(columns);
        let mut screen = Canvas::blank(columns, rows);
        screen.paste(&preview(columns - width, rows), width, 0);
        if width >= 2 && rows >= 2 {
            self.draw_panel(&mut screen, width, rows);
        }
        screen
    }

    fn draw_panel(&mut self, screen: &mut Canvas, width: usize, rows: usize) {
        let visible = rows - 2;
        self.page = visible.max(1);
        self.scroll_to_selection(visible);
        let right = width - 1;
        let bottom = rows - 1;

        for row in 0..rows {
            for column in 0..width {
                let glyph = match (column, row) {
                    (0, 0) => '╭',
                    (column, 0) if column == right => '╮',
                    (0, row) if row == bottom => '╰',
                    (column, row) if column == right && row == bottom => '╯',
                    (0, _) => '│',
                    (column, _) if column == right => '│',
                    (_, 0) => '─',
                    (_, row) if row == bottom => '─',
                    _ => ' ',
                };
                screen.set(column, row, paint(glyph, BORDER, PANEL));
            }
        }
        write(screen, 2, 0, right, " Scenes ", TITLE, PANEL);
        let position = format!(" {}/{} ", self.selected + 1, self.entries.len());
        let position_column = width.saturating_sub(position.chars().count() + 2).max(1);
        write(
            screen,
            position_column,
            bottom,
            right,
            &position,
            SHORTCUT,
            PANEL,
        );
        if self.scroll > 0 {
            screen.set(right - 1, 0, paint('▲', HEADING, PANEL));
        }
        if self.scroll + visible < self.lines.len() {
            screen.set(1, bottom, paint('▼', HEADING, PANEL));
        }

        let lines = self.lines.iter().skip(self.scroll).take(visible);
        for (row, line) in (1..).zip(lines) {
            match *line {
                Line::Gap => {}
                Line::Heading(group) => {
                    write(screen, 2, row, right, &group.to_uppercase(), HEADING, PANEL);
                }
                Line::Entry(index) => {
                    let selected = index == self.selected;
                    let (text, shortcut, background) = if selected {
                        (HIGHLIGHT_TEXT, HIGHLIGHT_TEXT, HIGHLIGHT)
                    } else {
                        (TEXT, SHORTCUT, PANEL)
                    };
                    for column in 1..right {
                        screen.set(column, row, paint(' ', text, background));
                    }
                    if selected {
                        screen.set(2, row, paint(MARKER, text, background));
                    }
                    if index < 9 {
                        let digit = char::from(b'1' + index as u8);
                        screen.set(4, row, paint(digit, shortcut, background));
                    }
                    let room = right.saturating_sub(8);
                    let label = fit(&self.entries[index].label, room);
                    write(screen, 7, row, right, &label, text, background);
                }
            }
        }
    }

    // Scrolls just enough to show the highlighted entry, along with the
    // heading above it when it is the first of its group.
    fn scroll_to_selection(&mut self, visible: usize) {
        let Some(line) = self
            .lines
            .iter()
            .position(|line| *line == Line::Entry(self.selected))
        else {
            return;
        };
        if visible == 0 {
            return;
        }
        let mut top = line;
        while top > 0 && matches!(self.lines[top - 1], Line::Heading(_)) {
            top -= 1;
        }
        if top < self.scroll {
            self.scroll = top;
        }
        if line >= self.scroll + visible {
            self.scroll = line + 1 - visible;
        }
        self.scroll = self.scroll.min(self.lines.len().saturating_sub(visible));
    }
}

fn paint(glyph: char, foreground: Rgb, background: Rgb) -> Cell {
    Cell {
        glyph,
        foreground: Some(foreground),
        background: Some(background),
    }
}

// Writes `text` from `column` on `row`, stopping before column `end`.
fn write(
    screen: &mut Canvas,
    column: usize,
    row: usize,
    end: usize,
    text: &str,
    foreground: Rgb,
    background: Rgb,
) {
    for (column, glyph) in (column..end).zip(text.chars()) {
        screen.set(column, row, paint(glyph, foreground, background));
    }
}

// Shortens `text` to `room` characters, ending with an ellipsis if cut.
fn fit(text: &str, room: usize) -> String {
    if text.chars().count() <= room {
        text.to_owned()
    } else {
        let mut shortened: String = text.chars().take(room.saturating_sub(1)).collect();
        shortened.push('…');
        shortened
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(groups: &[(&'static str, usize)]) -> Menu {
        let mut entries = Vec::new();
        for (group, count) in groups {
            for _ in 0..*count {
                entries.push(Entry {
                    label: format!("{group} {}", entries.len() + 1),
                    group,
                });
            }
        }
        Menu::new(entries)
    }

    fn screen(menu: &mut Menu, columns: usize, rows: usize) -> Vec<String> {
        menu.draw(columns, rows, Canvas::blank)
            .text()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn highlighted_row(lines: &[String]) -> Option<&str> {
        lines
            .iter()
            .map(String::as_str)
            .find(|line| line.contains(MARKER))
    }

    #[test]
    fn a_closed_menu_ignores_keys() {
        let mut menu = menu(&[("Scenes", 3)]);
        assert!(!menu.is_open());
        assert_eq!(menu.handle(Key::Down), Action::Ignored);
        assert_eq!(menu.handle(Key::Enter), Action::Ignored);
    }

    #[test]
    fn opening_highlights_the_current_scene() {
        let mut menu = menu(&[("Scenes", 5)]);
        menu.open(3);
        assert!(menu.is_open());
        assert_eq!(menu.handle(Key::Enter), Action::Chosen(3));
        assert!(!menu.is_open());
    }

    #[test]
    fn up_and_down_wrap_around_the_list() {
        let mut menu = menu(&[("Scenes", 3)]);
        menu.open(0);
        assert_eq!(menu.handle(Key::Up), Action::Highlighted(2));
        assert_eq!(menu.handle(Key::Down), Action::Highlighted(0));
        assert_eq!(menu.handle(Key::Down), Action::Highlighted(1));
    }

    #[test]
    fn letter_keys_and_tab_also_move_the_highlight() {
        let mut menu = menu(&[("Scenes", 4)]);
        menu.open(0);
        for (key, expected) in [
            (Key::Character(b'j'), 1),
            (Key::Character(b's'), 2),
            (Key::Character(b'\t'), 3),
            (Key::Character(b'n'), 0),
            (Key::Character(b'k'), 3),
            (Key::Character(b'w'), 2),
            (Key::BackTab, 1),
            (Key::Character(b'p'), 0),
        ] {
            assert_eq!(menu.handle(key), Action::Highlighted(expected), "{key:?}");
        }
    }

    #[test]
    fn home_and_end_jump_to_the_ends_without_wrapping_pages() {
        let mut menu = menu(&[("Scenes", 30)]);
        menu.open(4);
        assert_eq!(menu.handle(Key::End), Action::Highlighted(29));
        assert_eq!(menu.handle(Key::PageDown), Action::Highlighted(29));
        assert_eq!(menu.handle(Key::Home), Action::Highlighted(0));
        assert_eq!(menu.handle(Key::PageUp), Action::Highlighted(0));
        assert_eq!(menu.handle(Key::Character(b'G')), Action::Highlighted(29));
        assert_eq!(menu.handle(Key::Character(b'g')), Action::Highlighted(0));
    }

    #[test]
    fn page_keys_move_by_the_rows_that_fit() {
        let mut menu = menu(&[("Scenes", 30)]);
        menu.open(0);
        // Ten rows leave eight for the list between the borders.
        screen(&mut menu, 60, 10);
        assert_eq!(menu.handle(Key::PageDown), Action::Highlighted(8));
        assert_eq!(menu.handle(Key::PageDown), Action::Highlighted(16));
        assert_eq!(menu.handle(Key::PageUp), Action::Highlighted(8));
    }

    #[test]
    fn digits_pick_one_of_the_first_nine_entries() {
        let mut menu = menu(&[("Scenes", 12)]);
        menu.open(0);
        assert_eq!(menu.handle(Key::Character(b'7')), Action::Chosen(6));
        assert!(!menu.is_open());

        let mut short = self::menu(&[("Scenes", 3)]);
        short.open(1);
        assert_eq!(short.handle(Key::Character(b'5')), Action::Ignored);
        assert!(short.is_open());
    }

    #[test]
    fn escape_returns_to_the_scene_the_menu_opened_on() {
        let mut menu = menu(&[("Scenes", 5)]);
        menu.open(2);
        menu.handle(Key::Down);
        menu.handle(Key::Down);
        assert_eq!(menu.handle(Key::Escape), Action::Dismissed(2));
        assert!(!menu.is_open());
    }

    #[test]
    fn other_keys_are_left_to_the_caller() {
        let mut menu = menu(&[("Scenes", 5)]);
        menu.open(2);
        for key in [
            Key::Left,
            Key::Right,
            Key::Character(b'q'),
            Key::Character(b'm'),
            Key::Character(b' '),
        ] {
            assert_eq!(menu.handle(key), Action::Ignored, "{key:?}");
        }
        assert!(menu.is_open());
    }

    #[test]
    fn entries_are_listed_under_group_headings_with_shortcuts() {
        let mut menu = menu(&[("Generated", 2), ("Models", 2)]);
        menu.open(0);
        let panel = screen(&mut menu, 60, 12);
        assert!(panel[0].contains("Scenes"));
        assert!(panel[1].contains("GENERATED"));
        assert!(panel[2].contains("1  Generated 1"));
        assert!(panel[3].contains("2  Generated 2"));
        assert!(panel[5].contains("MODELS"));
        assert!(panel[6].contains("3  Models 3"));
        assert!(panel[11].contains("1/4"));
    }

    #[test]
    fn the_highlight_follows_the_selection() {
        let mut menu = menu(&[("Scenes", 4)]);
        menu.open(0);
        assert!(
            highlighted_row(&screen(&mut menu, 60, 10))
                .unwrap()
                .contains("Scenes 1")
        );
        menu.handle(Key::Down);
        menu.handle(Key::Down);
        let lines = screen(&mut menu, 60, 10);
        assert!(highlighted_row(&lines).unwrap().contains("Scenes 3"));
        assert_eq!(lines.iter().filter(|line| line.contains(MARKER)).count(), 1);
    }

    #[test]
    fn a_long_list_scrolls_to_keep_the_highlight_in_view() {
        let mut menu = menu(&[("Drawings", 15), ("Models", 15)]);
        menu.open(0);
        for _ in 0..20 {
            menu.handle(Key::Down);
            let lines = screen(&mut menu, 60, 8);
            let label = format!("{} ", menu.entries[menu.selected].label);
            let row = highlighted_row(&lines).expect("the highlight is in view");
            assert!(row.contains(&label), "{row:?} should show {label:?}");
        }
        // Back at the top, the first heading shows again.
        menu.handle(Key::Home);
        let lines = screen(&mut menu, 60, 8);
        assert!(lines[1].contains("DRAWINGS"));
        assert!(highlighted_row(&lines).unwrap().contains("Drawings 1 "));
    }

    #[test]
    fn the_preview_fills_the_columns_beside_the_panel() {
        let mut menu = menu(&[("Scenes", 3)]);
        menu.open(0);
        let width = menu.width(80);
        let mut preview_size = None;
        let screen = menu.draw(80, 20, |columns, rows| {
            preview_size = Some((columns, rows));
            let mut canvas = Canvas::blank(columns, rows);
            for row in 0..rows {
                canvas.set(
                    0,
                    row,
                    Cell {
                        glyph: '@',
                        ..Cell::BLANK
                    },
                );
            }
            canvas
        });
        assert_eq!(preview_size, Some((80 - width, 20)));
        assert_eq!((screen.columns(), screen.rows()), (80, 20));
        for line in screen.text().lines() {
            assert_eq!(line.chars().nth(width), Some('@'));
        }
    }

    #[test]
    fn long_labels_are_shortened_and_the_panel_leaves_room_for_the_preview() {
        let mut menu = Menu::new(vec![Entry {
            label: "a-very-long-file-name-that-would-cover-the-preview.glb".to_owned(),
            group: "Files",
        }]);
        menu.open(0);
        assert_eq!(menu.width(80), 40);
        let lines = screen(&mut menu, 80, 6);
        assert!(lines[2].contains('…'));
        assert!(menu.width(10) <= 10);
    }
}
