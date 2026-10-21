use std::env;
use std::io::{self, Write};
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton,
    MouseEventKind,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, SetTitle};
use crossterm::{cursor, execute};

use crate::canvas::Canvas;
use crate::color::ColorMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Character(char),
    /// A letter typed with Ctrl held, such as Ctrl+C.
    Control(char),
    Enter,
    Escape,
    Backspace,
    Delete,
    Tab,
    /// Shift+Tab.
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
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
    /// The pointer moved with no button held.
    Move,
    /// The left button went down.
    Press,
    /// The pointer moved with the left button held.
    Drag,
    /// The left button came up.
    Release,
    ScrollUp,
    ScrollDown,
    DrawPress,
    DrawDrag,
    DrawRelease,
}

/// Something that arrived from the terminal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    Key(Key),
    /// Text pasted all at once.
    Paste(String),
    Mouse(Mouse),
    /// The terminal changed size.
    Resize,
    /// The terminal window gained (true) or lost (false) the focus, in
    /// terminals that report it.
    Focus(bool),
}

impl Input {
    /// The input an event stands for, if the app uses it. Key releases and
    /// unsupported buttons are left out.
    fn from_event(event: Event) -> Option<Self> {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => {
                let control = key.modifiers.contains(KeyModifiers::CONTROL);
                Some(Self::Key(match key.code {
                    KeyCode::Char(character) if control => {
                        Key::Control(character.to_ascii_lowercase())
                    }
                    KeyCode::Char(character) => Key::Character(character),
                    KeyCode::Enter => Key::Enter,
                    KeyCode::Esc => Key::Escape,
                    KeyCode::Backspace => Key::Backspace,
                    KeyCode::Delete => Key::Delete,
                    KeyCode::Tab => Key::Tab,
                    KeyCode::BackTab => Key::BackTab,
                    KeyCode::Up => Key::Up,
                    KeyCode::Down => Key::Down,
                    KeyCode::Left => Key::Left,
                    KeyCode::Right => Key::Right,
                    KeyCode::Home => Key::Home,
                    KeyCode::End => Key::End,
                    KeyCode::PageUp => Key::PageUp,
                    KeyCode::PageDown => Key::PageDown,
                    _ => return None,
                }))
            }
            Event::Mouse(mouse) => {
                let action = match mouse.kind {
                    MouseEventKind::Moved => MouseAction::Move,
                    MouseEventKind::Down(MouseButton::Left) => MouseAction::Press,
                    MouseEventKind::Drag(MouseButton::Left) => MouseAction::Drag,
                    MouseEventKind::Up(MouseButton::Left) => MouseAction::Release,
                    MouseEventKind::Down(MouseButton::Right) => MouseAction::DrawPress,
                    MouseEventKind::Drag(MouseButton::Right) => MouseAction::DrawDrag,
                    MouseEventKind::Up(MouseButton::Right) => MouseAction::DrawRelease,
                    MouseEventKind::ScrollUp => MouseAction::ScrollUp,
                    MouseEventKind::ScrollDown => MouseAction::ScrollDown,
                    _ => return None,
                };
                Some(Self::Mouse(Mouse {
                    column: usize::from(mouse.column),
                    row: usize::from(mouse.row),
                    action,
                }))
            }
            Event::Paste(text) => Some(Self::Paste(text)),
            Event::Resize(..) => Some(Self::Resize),
            Event::FocusGained => Some(Self::Focus(true)),
            Event::FocusLost => Some(Self::Focus(false)),
            _ => None,
        }
    }
}

/// The terminal in raw mode on the alternate screen, with the cursor hidden
/// and optional mouse capture. Dropping it puts everything back, so code must let
/// it drop normally instead of calling `process::exit`.
pub struct Terminal {
    previous: Option<Canvas>,
    output: String,
    mouse_capture: bool,
    title: Option<String>,
}

impl Terminal {
    pub fn open() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let terminal = Self {
            previous: None,
            output: String::new(),
            mouse_capture: true,
            title: None,
        };
        execute!(
            io::stdout(),
            crossterm::style::Print("\x1b[22;0t"),
            EnterAlternateScreen,
            cursor::Hide,
            EnableMouseCapture,
            EnableBracketedPaste,
            EnableFocusChange,
            terminal::Clear(terminal::ClearType::All),
        )?;
        Ok(terminal)
    }

    /// Lets the terminal select text while a reading panel is open.
    pub fn set_mouse_capture(&mut self, on: bool) -> io::Result<()> {
        if self.mouse_capture != on {
            if on {
                execute!(io::stdout(), EnableMouseCapture)?;
            } else {
                execute!(io::stdout(), DisableMouseCapture)?;
            }
            self.mouse_capture = on;
        }
        Ok(())
    }

    /// Changes the window title only when the ghost's name or activity changes.
    pub fn set_title(&mut self, title: &str) -> io::Result<()> {
        if self.title.as_deref() != Some(title) {
            execute!(io::stdout(), SetTitle(title))?;
            self.title = Some(title.to_owned());
        }
        Ok(())
    }

    pub fn size(&self) -> Size {
        terminal::size()
            .ok()
            .filter(|(columns, rows)| *columns > 0 && *rows > 0)
            .map_or_else(Size::from_environment, |(columns, rows)| Size {
                columns: usize::from(columns),
                rows: usize::from(rows),
            })
    }

    /// The inputs that have arrived, waiting up to `wait` for the first.
    pub fn inputs(&mut self, wait: Duration) -> io::Result<Vec<Input>> {
        let mut inputs = Vec::new();
        let mut wait = wait;
        while event::poll(wait)? {
            inputs.extend(Input::from_event(event::read()?));
            wait = Duration::ZERO;
        }
        Ok(inputs)
    }

    /// Clears the screen and forgets what it showed, so the next draw is
    /// complete. Call it after resizing or changing how cells are encoded.
    pub fn clear(&mut self) -> io::Result<()> {
        self.previous = None;
        execute!(io::stdout(), terminal::Clear(terminal::ClearType::All))
    }

    /// Draws the canvas, rewriting only cells that changed since the last
    /// draw. The frame is wrapped in synchronized-output markers so terminals
    /// that support them never show a half-drawn frame.
    pub fn draw(&mut self, canvas: &Canvas, color: ColorMode) -> io::Result<()> {
        self.output.clear();
        self.output.push_str("\x1b[?2026h");
        canvas.redraw(self.previous.as_ref(), color, &mut self.output);
        self.output.push_str("\x1b[0m\x1b[?2026l");
        let mut stdout = io::stdout().lock();
        stdout.write_all(self.output.as_bytes())?;
        stdout.flush()?;
        self.previous = Some(canvas.clone());
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            DisableFocusChange,
            DisableBracketedPaste,
            DisableMouseCapture,
            cursor::Show,
            LeaveAlternateScreen,
            crossterm::style::Print("\x1b[23;0t"),
        );
        let _ = terminal::disable_raw_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, MouseEvent};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    fn mouse(kind: MouseEventKind) -> Event {
        Event::Mouse(MouseEvent {
            kind,
            column: 12,
            row: 4,
            modifiers: KeyModifiers::NONE,
        })
    }

    #[test]
    fn typed_characters_keep_their_case_and_unicode() {
        for character in ['a', 'Q', '한', '?'] {
            assert_eq!(
                Input::from_event(key(KeyCode::Char(character), KeyModifiers::SHIFT)),
                Some(Input::Key(Key::Character(character)))
            );
        }
    }

    #[test]
    fn letters_with_ctrl_become_control_keys() {
        assert_eq!(
            Input::from_event(key(KeyCode::Char('C'), KeyModifiers::CONTROL)),
            Some(Input::Key(Key::Control('c')))
        );
    }

    #[test]
    fn key_releases_are_ignored() {
        let mut release = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(Input::from_event(Event::Key(release)), None);
    }

    #[test]
    fn the_left_button_moves_and_wheel_are_reported_at_their_cell() {
        let cases = [
            (MouseEventKind::Moved, MouseAction::Move),
            (MouseEventKind::Down(MouseButton::Left), MouseAction::Press),
            (MouseEventKind::Drag(MouseButton::Left), MouseAction::Drag),
            (MouseEventKind::Up(MouseButton::Left), MouseAction::Release),
            (
                MouseEventKind::Down(MouseButton::Right),
                MouseAction::DrawPress,
            ),
            (
                MouseEventKind::Drag(MouseButton::Right),
                MouseAction::DrawDrag,
            ),
            (
                MouseEventKind::Up(MouseButton::Right),
                MouseAction::DrawRelease,
            ),
            (MouseEventKind::ScrollUp, MouseAction::ScrollUp),
            (MouseEventKind::ScrollDown, MouseAction::ScrollDown),
        ];
        for (kind, action) in cases {
            assert_eq!(
                Input::from_event(mouse(kind)),
                Some(Input::Mouse(Mouse {
                    column: 12,
                    row: 4,
                    action
                }))
            );
        }
        assert_eq!(
            Input::from_event(mouse(MouseEventKind::Down(MouseButton::Middle))),
            None
        );
    }

    #[test]
    fn pastes_and_resizes_come_through() {
        assert_eq!(
            Input::from_event(Event::Paste("sk-key".into())),
            Some(Input::Paste("sk-key".into()))
        );
        assert_eq!(
            Input::from_event(Event::Resize(80, 24)),
            Some(Input::Resize)
        );
    }

    #[test]
    fn focus_changes_come_through() {
        assert_eq!(
            Input::from_event(Event::FocusGained),
            Some(Input::Focus(true))
        );
        assert_eq!(
            Input::from_event(Event::FocusLost),
            Some(Input::Focus(false))
        );
    }
}
