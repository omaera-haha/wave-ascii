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

impl Key {
    // Raw mode delivers arrows as `ESC [ A`-style sequences, so a lone ESC is
    // the Escape key only when no sequence follows it in the same read.
    fn parse(bytes: &[u8]) -> Vec<Self> {
        let mut keys = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            let byte = bytes[index];
            index += 1;
            if byte != 27 || !matches!(bytes.get(index), Some(b'[' | b'O')) {
                keys.push(match byte {
                    27 => Self::Escape,
                    // Raw mode turns off the translation of Enter's CR to LF.
                    b'\r' | b'\n' => Self::Enter,
                    _ => Self::Character(byte),
                });
                continue;
            }
            // Read parameters up to the final byte of the sequence. Keys with
            // modifiers add parameters after a `;`, so only the first counts.
            index += 1;
            let start = index;
            while index < bytes.len() && !(0x40..=0x7e).contains(&bytes[index]) {
                index += 1;
            }
            let parameter = bytes[start..index].split(|byte| *byte == b';').next();
            let key = match (bytes.get(index), parameter) {
                (Some(b'A'), _) => Some(Self::Up),
                (Some(b'B'), _) => Some(Self::Down),
                (Some(b'C'), _) => Some(Self::Right),
                (Some(b'D'), _) => Some(Self::Left),
                (Some(b'H'), _) | (Some(b'~'), Some(b"1" | b"7")) => Some(Self::Home),
                (Some(b'F'), _) | (Some(b'~'), Some(b"4" | b"8")) => Some(Self::End),
                (Some(b'~'), Some(b"5")) => Some(Self::PageUp),
                (Some(b'~'), Some(b"6")) => Some(Self::PageDown),
                (Some(b'Z'), _) => Some(Self::BackTab),
                _ => None,
            };
            keys.extend(key);
            index += 1;
        }
        keys
    }
}

pub struct Terminal {
    tty: File,
    saved_mode: String,
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
            previous: None,
            output: String::new(),
        };
        let mut output = io::stdout().lock();
        output.write_all(b"\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H")?;
        output.flush()?;
        Ok(terminal)
    }

    pub fn size(&self) -> Size {
        stty(&self.tty, &["size"])
            .ok()
            .and_then(|output| Size::parse(&output))
            .unwrap_or_else(Size::from_environment)
    }

    pub fn keys(&mut self) -> io::Result<Vec<Key>> {
        let mut keys = [0; 64];
        let count = self.tty.read(&mut keys)?;
        Ok(Key::parse(&keys[..count]))
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
        let _ = output.write_all(b"\x1b[?25h\x1b[?1049l");
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

    #[test]
    fn arrow_sequences_are_not_mistaken_for_escape() {
        assert_eq!(
            Key::parse(b"\x1b[A\x1bOD\x1b[1;5Cq\x1b"),
            [
                Key::Up,
                Key::Left,
                Key::Right,
                Key::Character(b'q'),
                Key::Escape
            ]
        );
        assert_eq!(Key::parse(b"\x1b\x1b[B"), [Key::Escape, Key::Down]);
    }

    #[test]
    fn enter_arrives_as_a_carriage_return_in_raw_mode() {
        assert_eq!(Key::parse(b"\r"), [Key::Enter]);
        assert_eq!(Key::parse(b"\n"), [Key::Enter]);
        assert_eq!(Key::parse(b"\t"), [Key::Character(b'\t')]);
    }

    #[test]
    fn home_and_end_are_read_in_every_common_encoding() {
        for sequence in [&b"\x1b[H"[..], b"\x1bOH", b"\x1b[1~", b"\x1b[7~"] {
            assert_eq!(Key::parse(sequence), [Key::Home], "{sequence:?}");
        }
        for sequence in [&b"\x1b[F"[..], b"\x1bOF", b"\x1b[4~", b"\x1b[8~"] {
            assert_eq!(Key::parse(sequence), [Key::End], "{sequence:?}");
        }
    }

    #[test]
    fn page_keys_and_shift_tab_are_recognized_with_or_without_modifiers() {
        assert_eq!(
            Key::parse(b"\x1b[5~\x1b[6~\x1b[5;5~\x1b[Z"),
            [Key::PageUp, Key::PageDown, Key::PageUp, Key::BackTab]
        );
    }

    #[test]
    fn unknown_sequences_are_skipped_whole() {
        // F5 and Insert, then a key the app knows.
        assert_eq!(Key::parse(b"\x1b[15~\x1b[2~j"), [Key::Character(b'j')]);
    }

    #[test]
    fn a_sequence_cut_off_at_the_end_of_a_read_produces_no_key() {
        assert_eq!(Key::parse(b"q\x1b["), [Key::Character(b'q')]);
        assert_eq!(Key::parse(b"\x1b[5"), []);
    }

    #[test]
    fn stty_rows_and_columns_are_read_in_the_correct_order() {
        let size = Size::parse("24 80\n").unwrap();
        assert_eq!((size.columns, size.rows), (80, 24));
        assert!(Size::parse("0 0").is_none());
        assert!(Size::parse("not a size").is_none());
    }
}
