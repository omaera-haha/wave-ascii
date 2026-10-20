use std::fmt::Write;

use glam::Vec3;

use crate::color::{ColorMode, Ground, Paint, Rgb};
use crate::frame::{CELL_ASPECT, Frame, Grid, Shade};

const RAMP: &[u8] = b" .:-=+*#%@";
// Pixels dimmer than this are left as terminal background in block mode.
const TRANSPARENT: f32 = 0.02;
// A 4x4 ordered-dither matrix, read with global pixel coordinates so the
// pattern continues across cells.
const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

/// How pixels become characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// One pixel per cell, drawn with the brightness ramp.
    Ascii,
    /// Two square pixels per cell, drawn with half blocks.
    Blocks,
    /// Eight square dots per cell, drawn with braille patterns.
    Braille,
}

impl Style {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "ascii" => Some(Self::Ascii),
            "blocks" => Some(Self::Blocks),
            "braille" => Some(Self::Braille),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ascii => "ascii",
            Self::Blocks => "blocks",
            Self::Braille => "braille",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Ascii => Self::Blocks,
            Self::Blocks => Self::Braille,
            Self::Braille => Self::Ascii,
        }
    }

    fn pixels_per_cell(self) -> (usize, usize) {
        match self {
            Self::Ascii => (1, 1),
            Self::Blocks => (1, 2),
            Self::Braille => (2, 4),
        }
    }

    /// The grid a scene must be sampled on to fill `columns` x `rows` cells.
    pub fn grid(self, columns: usize, rows: usize) -> Grid {
        let (across, down) = self.pixels_per_cell();
        Grid::new(
            columns * across,
            rows * down,
            CELL_ASPECT * down as f32 / across as f32,
        )
    }

    pub fn canvas(self, frame: &Frame, color: ColorMode) -> Canvas {
        let (across, down) = self.pixels_per_cell();
        let columns = frame.width() / across;
        let rows = frame.height() / down;
        let mut cells = Vec::with_capacity(columns * rows);
        for row in 0..rows {
            for column in 0..columns {
                cells.push(match self {
                    Self::Ascii => ascii_cell(frame, column, row),
                    Self::Blocks => block_cell(frame, column, row, color.is_colored()),
                    Self::Braille => braille_cell(frame, column, row),
                });
            }
        }
        Canvas {
            columns,
            rows,
            cells,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub glyph: char,
    pub foreground: Option<Rgb>,
    pub background: Option<Rgb>,
}

impl Cell {
    pub const BLANK: Self = Self {
        glyph: ' ',
        foreground: None,
        background: None,
    };

    fn ink(glyph: char, color: Vec3) -> Self {
        Self {
            glyph,
            foreground: Some(Rgb::from_vec3(color)),
            background: None,
        }
    }
}

// Ink color for a glyph: the square root keeps dim glyphs from vanishing,
// since the glyph's own density already darkens them.
fn ink_color(color: Vec3, brightness: f32) -> Vec3 {
    color * brightness.sqrt()
}

fn dithered(brightness: f32, x: usize, y: usize) -> bool {
    brightness > (BAYER[y % 4][x % 4] as f32 + 0.5) / 16.0
}

fn ascii_cell(frame: &Frame, column: usize, row: usize) -> Cell {
    let shade = frame.shade(column, row);
    let index = (shade.brightness * (RAMP.len() - 1) as f32).round() as usize;
    if index == 0 {
        Cell::BLANK
    } else {
        Cell::ink(
            RAMP[index] as char,
            ink_color(shade.color, shade.brightness),
        )
    }
}

fn block_cell(frame: &Frame, column: usize, row: usize, colored: bool) -> Cell {
    let top = frame.shade(column, row * 2);
    let bottom = frame.shade(column, row * 2 + 1);
    if !colored {
        let glyph = match (
            dithered(top.brightness, column, row * 2),
            dithered(bottom.brightness, column, row * 2 + 1),
        ) {
            (false, false) => ' ',
            (true, false) => '▀',
            (false, true) => '▄',
            (true, true) => '█',
        };
        return Cell {
            glyph,
            ..Cell::BLANK
        };
    }

    let pixel =
        |shade: Shade| (shade.brightness > TRANSPARENT).then(|| Rgb::from_vec3(shade.rgb()));
    match (pixel(top), pixel(bottom)) {
        (None, None) => Cell::BLANK,
        (Some(top), None) => Cell {
            glyph: '▀',
            foreground: Some(top),
            background: None,
        },
        (None, Some(bottom)) => Cell {
            glyph: '▄',
            foreground: Some(bottom),
            background: None,
        },
        (Some(top), Some(bottom)) => Cell {
            glyph: '▀',
            foreground: Some(top),
            background: Some(bottom),
        },
    }
}

fn braille_cell(frame: &Frame, column: usize, row: usize) -> Cell {
    // Bit for each dot, indexed by [dot row][dot column].
    const DOTS: [[u32; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
    let mut bits = 0;
    let mut brightness = 0.0;
    let mut light = Vec3::ZERO;
    for (dot_row, row_bits) in DOTS.iter().enumerate() {
        for (dot_column, bit) in row_bits.iter().enumerate() {
            let (x, y) = (column * 2 + dot_column, row * 4 + dot_row);
            let shade = frame.shade(x, y);
            brightness += shade.brightness;
            light += shade.rgb();
            if dithered(shade.brightness, x, y) {
                bits |= bit;
            }
        }
    }
    if bits == 0 {
        return Cell::BLANK;
    }
    let glyph = char::from_u32(0x2800 + bits).unwrap_or(' ');
    Cell::ink(glyph, ink_color(light / brightness, brightness / 8.0))
}

/// A grid of terminal cells, ready to encode.
#[derive(Clone, Debug, PartialEq)]
pub struct Canvas {
    columns: usize,
    rows: usize,
    cells: Vec<Cell>,
}

impl Canvas {
    pub fn blank(columns: usize, rows: usize) -> Self {
        Self {
            columns,
            rows,
            cells: vec![Cell::BLANK; columns * rows],
        }
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Replaces one cell. Cells outside the canvas are ignored.
    pub fn set(&mut self, column: usize, row: usize, cell: Cell) {
        if column < self.columns && row < self.rows {
            self.cells[row * self.columns + column] = cell;
        }
    }

    /// Copies `other` onto this canvas with its top-left corner at `column`
    /// and `row`, clipped to this canvas.
    pub fn paste(&mut self, other: &Canvas, column: usize, row: usize) {
        for (offset, cells) in other.cells.chunks(other.columns.max(1)).enumerate() {
            for (index, cell) in cells.iter().enumerate() {
                self.set(column + index, row + offset, *cell);
            }
        }
    }

    /// The glyphs alone, one line per row.
    pub fn text(&self) -> String {
        let mut output = String::with_capacity(self.cells.len() * 3 + self.rows);
        for row in self.cells.chunks(self.columns.max(1)) {
            output.extend(row.iter().map(|cell| cell.glyph));
            output.push('\n');
        }
        output
    }

    /// Lines with color escapes, for writing to a pipe or file.
    pub fn ansi(&self, color: ColorMode) -> String {
        if !color.is_colored() {
            return self.text();
        }
        let mut output = String::with_capacity(self.cells.len() * 24);
        for row in self.cells.chunks(self.columns.max(1)) {
            let mut pen = Pen::default();
            for cell in row {
                pen.write(cell, color, &mut output);
            }
            output.push_str("\x1b[0m\n");
        }
        output
    }

    /// Escapes that turn the screen from `previous` into this canvas, moving
    /// the cursor past unchanged cells. Without a previous canvas, every cell
    /// is drawn. Positions are absolute, starting at the top-left corner.
    pub fn redraw(&self, previous: Option<&Canvas>, color: ColorMode, output: &mut String) {
        let previous = previous
            .filter(|previous| previous.columns == self.columns && previous.rows == self.rows);
        let mut pen = Pen::default();
        output.push_str("\x1b[0m");
        for (row, cells) in self.cells.chunks(self.columns.max(1)).enumerate() {
            let mut cursor_in_place = false;
            for (column, cell) in cells.iter().enumerate() {
                let unchanged = previous
                    .is_some_and(|previous| previous.cells[row * self.columns + column] == *cell);
                if unchanged {
                    cursor_in_place = false;
                    continue;
                }
                if !cursor_in_place {
                    let _ = write!(output, "\x1b[{};{}H", row + 1, column + 1);
                    cursor_in_place = true;
                }
                pen.write(cell, color, output);
            }
        }
        output.push_str("\x1b[0m");
    }
}

/// Tracks the current terminal colors so escapes are written only when they
/// change.
#[derive(Default)]
struct Pen {
    foreground: Option<Paint>,
    background: Option<Paint>,
}

impl Pen {
    fn write(&mut self, cell: &Cell, color: ColorMode, output: &mut String) {
        let background = color.paint(cell.background);
        if background != self.background {
            Paint::write_sgr(background, Ground::Background, output);
            self.background = background;
        }
        // A space without a background shows no foreground color.
        if cell.glyph != ' ' || background.is_some() {
            let foreground = color.paint(cell.foreground);
            if foreground != self.foreground {
                Paint::write_sgr(foreground, Ground::Foreground, output);
                self.foreground = foreground;
            }
        }
        output.push(cell.glyph);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn uniform(style: Style, shade: Shade) -> Canvas {
        let grid = style.grid(3, 2);
        style.canvas(&Frame::sample(grid, |_| shade), ColorMode::TrueColor)
    }

    #[test]
    fn brightness_is_clamped_to_the_ramp() {
        assert_eq!(
            uniform(Style::Ascii, Shade::gray(-3.0)).text(),
            "   \n   \n"
        );
        assert_eq!(uniform(Style::Ascii, Shade::gray(4.0)).text(), "@@@\n@@@\n");
    }

    #[test]
    fn every_style_fills_the_requested_cells() {
        for style in [Style::Ascii, Style::Blocks, Style::Braille] {
            let canvas = uniform(style, Shade::gray(1.0));
            assert_eq!((canvas.columns(), canvas.rows()), (3, 2));
            assert!(canvas.cells.iter().all(|cell| cell.glyph != ' '));
        }
        assert_eq!(
            uniform(Style::Braille, Shade::gray(1.0)).cells[0].glyph,
            '⣿'
        );
    }

    #[test]
    fn colored_blocks_use_both_halves_of_the_cell() {
        let grid = Style::Blocks.grid(1, 1);
        let frame = Frame::build(grid.width, grid.height, |_, row| {
            Shade::new(1.0, if row == 0 { Vec3::X } else { Vec3::Z })
        });
        let cell = Style::Blocks.canvas(&frame, ColorMode::TrueColor).cells[0];
        assert_eq!(cell.glyph, '▀');
        assert_eq!(cell.foreground, Some(Rgb([255, 0, 0])));
        assert_eq!(cell.background, Some(Rgb([0, 0, 255])));
    }

    #[test]
    fn plain_output_has_no_escapes() {
        let canvas = uniform(Style::Ascii, Shade::new(0.8, Vec3::X));
        assert!(!canvas.ansi(ColorMode::Plain).contains('\x1b'));
        assert!(canvas.ansi(ColorMode::TrueColor).contains("\x1b[38;2;"));
    }

    #[test]
    fn redrawing_an_unchanged_canvas_writes_no_glyphs() {
        let canvas = uniform(Style::Ascii, Shade::new(0.8, Vec3::X));
        let mut full = String::new();
        canvas.redraw(None, ColorMode::TrueColor, &mut full);
        assert_eq!(full.matches('#').count(), 6);

        let mut diff = String::new();
        canvas.redraw(Some(&canvas), ColorMode::TrueColor, &mut diff);
        assert_eq!(diff, "\x1b[0m\x1b[0m");
    }

    // Plays escapes onto a screen of cells, understanding only what `redraw`
    // writes: cursor positions, truecolor and default colors, and glyphs.
    fn replay(escapes: &str, screen: &mut [Cell], columns: usize) {
        let mut cursor = 0;
        let mut pen = Cell::BLANK;
        let mut characters = escapes.chars().peekable();
        while let Some(character) = characters.next() {
            if character != '\x1b' {
                screen[cursor] = Cell {
                    glyph: character,
                    ..pen
                };
                cursor += 1;
                continue;
            }
            characters.next();
            let mut parameters = String::new();
            let command = loop {
                let next = characters.next().unwrap();
                if next.is_ascii_alphabetic() {
                    break next;
                }
                parameters.push(next);
            };
            let numbers: Vec<u8> = parameters.split(';').map(|n| n.parse().unwrap()).collect();
            match (command, numbers.as_slice()) {
                ('H', [row, column]) => {
                    cursor = (*row as usize - 1) * columns + *column as usize - 1;
                }
                ('m', [0]) => pen = Cell::BLANK,
                ('m', [39]) => pen.foreground = None,
                ('m', [49]) => pen.background = None,
                ('m', [38, 2, red, green, blue]) => {
                    pen.foreground = Some(Rgb([*red, *green, *blue]))
                }
                ('m', [48, 2, red, green, blue]) => {
                    pen.background = Some(Rgb([*red, *green, *blue]))
                }
                other => panic!("unexpected escape {other:?}"),
            }
        }
    }

    #[test]
    fn redrawing_over_a_previous_canvas_reproduces_the_new_one() {
        let frame = |time: f32| {
            let grid = Style::Blocks.grid(12, 5);
            Frame::sample(grid, |point| {
                let distance = (point - Vec2::new(time, 0.0)).length();
                Shade::new(1.0 - distance, Vec3::new(time, 0.5, 1.0 - time))
            })
        };
        let first = Style::Blocks.canvas(&frame(0.0), ColorMode::TrueColor);
        let second = Style::Blocks.canvas(&frame(0.4), ColorMode::TrueColor);
        assert_ne!(first, second);

        let mut screen = vec![Cell::BLANK; 12 * 5];
        for (canvas, previous) in [(&first, None), (&second, Some(&first))] {
            let mut escapes = String::new();
            canvas.redraw(previous, ColorMode::TrueColor, &mut escapes);
            replay(&escapes, &mut screen, 12);
        }
        // A space without a background shows no foreground, so ignore it there.
        let visible = |cell: &Cell| Cell {
            foreground: cell
                .foreground
                .filter(|_| cell.glyph != ' ' || cell.background.is_some()),
            ..*cell
        };
        let expected: Vec<Cell> = second.cells.iter().map(visible).collect();
        let shown: Vec<Cell> = screen.iter().map(visible).collect();
        assert_eq!(shown, expected);
    }

    #[test]
    fn colors_are_written_only_when_they_change() {
        let canvas = uniform(Style::Ascii, Shade::new(1.0, Vec3::Y));
        assert_eq!(
            canvas
                .ansi(ColorMode::TrueColor)
                .matches("\x1b[38;2;")
                .count(),
            2
        );
    }
}
