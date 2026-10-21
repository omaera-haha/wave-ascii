use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};

use crate::canvas::Canvas;
use crate::color::ColorMode;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub columns: usize,
    pub rows: usize,
}

impl Size {
    pub fn from_environment() -> Self {
        fn dimension(name: &str, fallback: usize) -> usize {
            env::var(name)
                .ok()
                .and_then(|value| value.parse().ok())
                .filter(|value| *value > 0)
                .unwrap_or(fallback)
        }
        Self {
            columns: dimension("COLUMNS", 80),
            rows: dimension("LINES", 24),
        }
    }

    fn parse(output: &str) -> Option<Self> {
        let mut dimensions = output.split_whitespace();
        let rows = dimensions.next()?.parse().ok()?;
        let columns = dimensions.next()?.parse().ok()?;
        (rows > 0 && columns > 0).then_some(Self { columns, rows })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Character(u8),
    Enter,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    /// Shift+Tab.
    BackTab,
    Escape,
}

/// What a mouse did, at a cell counted from 0 at the top left of the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mouse {
    pub column: usize,
    pub row: usize,
    pub action: MouseAction,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAction {
    /// The pointer moved, with or without a button held.
    Move,
    /// The left button went down.
    Press,
    ScrollUp,
    ScrollDown,
}

impl Mouse {
    // Reads an SGR report, `ESC [ < button ; column ; row M`, from its
    // parameters after the `<` and its final byte, which is `m` for a
    // release. Buttons other than the left one and the wheel are left out.
    fn parse(parameters: &[u8], last: u8) -> Option<Self> {
        let mut numbers = std::str::from_utf8(parameters)
            .ok()?
            .split(';')
            .map(|number| number.parse::<usize>().ok());
        let (button, column, row) = (numbers.next()??, numbers.next()??, numbers.next()??);
        let action = if button & 64 != 0 {
            match button & 3 {
                0 => MouseAction::ScrollUp,
                1 => MouseAction::ScrollDown,
                _ => return None,
            }
        } else if button & 32 != 0 {
            MouseAction::Move
        } else if last == b'M' && button & 3 == 0 {
            MouseAction::Press
        } else {
            return None;
        };
        Some(Self {
            column: column.checked_sub(1)?,
            row: row.checked_sub(1)?,
            action,
        })
    }
}

/// Something that arrived from the terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Key(Key),
    Mouse(Mouse),
}

impl Input {
    /// Reads the inputs in `bytes`, and how many bytes they took. A sequence
    /// cut off at the end is left unread for the next read to finish.
    // Raw mode delivers arrows as `ESC [ A`-style sequences, so a lone ESC is
    // the Escape key only when no sequence follows it in the same read.
    fn parse(bytes: &[u8]) -> (Vec<Self>, usize) {
        let mut inputs = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            let start = index;
            let byte = bytes[index];
            index += 1;
            if byte != 27 || !matches!(bytes.get(index), Some(b'[' | b'O')) {
                inputs.push(Self::Key(match byte {
                    27 => Key::Escape,
                    // Raw mode turns off the translation of Enter's CR to LF.
                    b'\r' | b'\n' => Key::Enter,
                    _ => Key::Character(byte),
                }));
                continue;
            }
            // Read parameters up to the final byte of the sequence.
            index += 1;
            let parameters_start = index;
            while index < bytes.len() && !(0x40..=0x7e).contains(&bytes[index]) {
                index += 1;
            }
            let Some(&last) = bytes.get(index) else {
                return (inputs, start);
            };
            let parameters = &bytes[parameters_start..index];
            index += 1;
            // Terminals without the SGR mouse encoding send three raw bytes
            // after `ESC [ M`, which must not be read as keys.
            if bytes[start + 1] == b'[' && last == b'M' && parameters.is_empty() {
                if index + 3 > bytes.len() {
                    return (inputs, start);
                }
                index += 3;
                continue;
            }
            if let Some(report) = parameters.strip_prefix(b"<") {
                inputs.extend(Mouse::parse(report, last).map(Self::Mouse));
                continue;
            }
            // Keys with modifiers add parameters after a `;`, so only the
            // first counts.
            let parameter = parameters.split(|byte| *byte == b';').next();
            let key = match (last, parameter) {
                (b'A', _) => Some(Key::Up),
                (b'B', _) => Some(Key::Down),
                (b'C', _) => Some(Key::Right),
                (b'D', _) => Some(Key::Left),
                (b'H', _) | (b'~', Some(b"1" | b"7")) => Some(Key::Home),
                (b'F', _) | (b'~', Some(b"4" | b"8")) => Some(Key::End),
                (b'~', Some(b"5")) => Some(Key::PageUp),
                (b'~', Some(b"6")) => Some(Key::PageDown),
                (b'Z', _) => Some(Key::BackTab),
                _ => None,
            };
            inputs.extend(key.map(Self::Key));
        }
        (inputs, index)
    }
}

pub struct Terminal {
    tty: File,
    saved_mode: String,
    // The start of a sequence that the last read cut off.
    unread: Vec<u8>,
    previous: Option<Canvas>,
    output: String,
}

impl Terminal {
    pub fn open() -> io::Result<Self> {
        let tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
        let saved_mode = stty(&tty, &["-g"])?;
        stty(&tty, &["raw", "-echo", "min", "0", "time", "0"])?;
        let terminal = Self {
            tty,
            saved_mode,
            unread: Vec::new(),
            previous: None,
            output: String::new(),
        };
        let mut output = io::stdout().lock();
        // Report every mouse move, not only drags, in the SGR encoding.
        output.write_all(b"\x1b[?1049h\x1b[?25l\x1b[?1003h\x1b[?1006h\x1b[2J\x1b[H")?;
        output.flush()?;
        Ok(terminal)
    }

    pub fn size(&self) -> Size {
        stty(&self.tty, &["size"])
            .ok()
            .and_then(|output| Size::parse(&output))
            .unwrap_or_else(Size::from_environment)
    }

    pub fn inputs(&mut self) -> io::Result<Vec<Input>> {
        let mut buffer = [0; 1024];
        let count = self.tty.read(&mut buffer)?;
        self.unread.extend_from_slice(&buffer[..count]);
        let (inputs, read) = Input::parse(&self.unread);
        self.unread.drain(..read);
        // No sequence the app reads is this long, so this one never ends.
        if self.unread.len() > 32 {
            self.unread.clear();
        }
        Ok(inputs)
    }

    /// Clears the screen and forgets what it showed, so the next draw is
    /// complete. Call it after resizing or changing how cells are encoded.
    pub fn clear(&mut self) -> io::Result<()> {
        self.previous = None;
        io::stdout().lock().write_all(b"\x1b[2J\x1b[H")
    }

    /// Draws the canvas, rewriting only cells that changed since the last
    /// draw, with the status line below it. The frame is wrapped in
    /// synchronized-output markers so terminals that support them never show
    /// a half-drawn frame.
    pub fn draw(&mut self, canvas: &Canvas, color: ColorMode, status: &str) -> io::Result<()> {
        // The status is padded by a space on each side.
        let status: String = status
            .chars()
            .take(canvas.columns().saturating_sub(2))
            .collect();
        self.output.clear();
        self.output.push_str("\x1b[?2026h");
        canvas.redraw(self.previous.as_ref(), color, &mut self.output);
        self.output.push_str(&format!(
            "\x1b[{};1H\x1b[0;7m {status} \x1b[0m\x1b[K\x1b[?2026l",
            canvas.rows() + 1
        ));

        let mut stdout = io::stdout().lock();
        stdout.write_all(self.output.as_bytes())?;
        stdout.flush()?;
        self.previous = Some(canvas.clone());
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = stty(&self.tty, &[self.saved_mode.trim()]);
        let mut output = io::stdout().lock();
        let _ = output.write_all(b"\x1b[?1006l\x1b[?1003l\x1b[?25h\x1b[?1049l");
        let _ = output.flush();
    }
}

fn stty(tty: &File, arguments: &[&str]) -> io::Result<String> {
    let output = Command::new("stty")
        .args(arguments)
        .stdin(Stdio::from(tty.try_clone()?))
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "stty failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(bytes: &[u8]) -> Vec<Key> {
        let (inputs, read) = Input::parse(bytes);
        assert_eq!(read, bytes.len(), "{bytes:?}");
        inputs
            .into_iter()
            .map(|input| match input {
                Input::Key(key) => key,
                Input::Mouse(mouse) => panic!("{mouse:?} in {bytes:?}"),
            })
            .collect()
    }

    #[test]
    fn arrow_sequences_are_not_mistaken_for_escape() {
        assert_eq!(
            keys(b"\x1b[A\x1bOD\x1b[1;5Cq\x1b"),
            [
                Key::Up,
                Key::Left,
                Key::Right,
                Key::Character(b'q'),
                Key::Escape
            ]
        );
        assert_eq!(keys(b"\x1b\x1b[B"), [Key::Escape, Key::Down]);
    }

    #[test]
    fn enter_arrives_as_a_carriage_return_in_raw_mode() {
        assert_eq!(keys(b"\r"), [Key::Enter]);
        assert_eq!(keys(b"\n"), [Key::Enter]);
        assert_eq!(keys(b"\t"), [Key::Character(b'\t')]);
    }

    #[test]
    fn home_and_end_are_read_in_every_common_encoding() {
        for sequence in [&b"\x1b[H"[..], b"\x1bOH", b"\x1b[1~", b"\x1b[7~"] {
            assert_eq!(keys(sequence), [Key::Home], "{sequence:?}");
        }
        for sequence in [&b"\x1b[F"[..], b"\x1bOF", b"\x1b[4~", b"\x1b[8~"] {
            assert_eq!(keys(sequence), [Key::End], "{sequence:?}");
        }
    }

    #[test]
    fn page_keys_and_shift_tab_are_recognized_with_or_without_modifiers() {
        assert_eq!(
            keys(b"\x1b[5~\x1b[6~\x1b[5;5~\x1b[Z"),
            [Key::PageUp, Key::PageDown, Key::PageUp, Key::BackTab]
        );
    }

    #[test]
    fn unknown_sequences_are_skipped_whole() {
        // F5 and Insert, then a key the app knows.
        assert_eq!(keys(b"\x1b[15~\x1b[2~j"), [Key::Character(b'j')]);
    }

    #[test]
    fn a_sequence_cut_off_at_the_end_of_a_read_waits_for_the_rest() {
        assert_eq!(
            Input::parse(b"q\x1b[5"),
            (vec![Input::Key(Key::Character(b'q'))], 1)
        );
        assert_eq!(keys(b"\x1b[5~"), [Key::PageUp]);
        let (inputs, read) = Input::parse(b"\x1b[<35;12");
        assert!(inputs.is_empty());
        assert_eq!(read, 0);
    }

    #[test]
    fn sgr_mouse_reports_become_moves_presses_and_scrolls_at_cells_from_zero() {
        let mouse = |column, row, action| {
            Input::Mouse(Mouse {
                column,
                row,
                action,
            })
        };
        let (inputs, _) =
            Input::parse(b"\x1b[<35;12;5M\x1b[<0;1;1M\x1b[<32;3;4Mj\x1b[<64;7;8M\x1b[<65;7;8M");
        assert_eq!(
            inputs,
            [
                mouse(11, 4, MouseAction::Move),
                mouse(0, 0, MouseAction::Press),
                mouse(2, 3, MouseAction::Move),
                Input::Key(Key::Character(b'j')),
                mouse(6, 7, MouseAction::ScrollUp),
                mouse(6, 7, MouseAction::ScrollDown),
            ]
        );
    }

    #[test]
    fn releases_and_other_buttons_are_skipped() {
        // A left release, a right press, a middle press, and a sideways
        // scroll.
        let bytes = b"\x1b[<0;5;5m\x1b[<2;5;5M\x1b[<1;5;5M\x1b[<66;5;5M";
        assert_eq!(Input::parse(bytes), (Vec::new(), bytes.len()));
    }

    #[test]
    fn legacy_mouse_reports_are_skipped_rather_than_read_as_keys() {
        // A move to column 81 and row 2 encodes them as `q` and `"`.
        assert_eq!(keys(b"\x1b[MCq\"j"), [Key::Character(b'j')]);
        assert_eq!(Input::parse(b"\x1b[MCq"), (Vec::new(), 0));
    }

    #[test]
    fn stty_rows_and_columns_are_read_in_the_correct_order() {
        let size = Size::parse("24 80\n").unwrap();
        assert_eq!((size.columns, size.rows), (80, 24));
        assert!(Size::parse("0 0").is_none());
        assert!(Size::parse("not a size").is_none());
    }
}
