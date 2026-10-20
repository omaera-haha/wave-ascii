use crate::canvas::Style;
use crate::color::ColorMode;
use crate::menu::{Action, Menu};
use crate::scene::View;
use crate::terminal::Key;

// Radians per arrow key press.
const TURN: f32 = 0.12;

/// What the main loop should do after a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    Continue,
    /// The style or color mode changed, so the next draw must be complete.
    Restyle,
    Quit,
}

/// Everything the keys control.
pub struct App {
    pub scene: usize,
    pub view: View,
    pub paused: bool,
    pub style: Style,
    pub color: ColorMode,
    pub menu: Menu,
}

impl App {
    pub fn new(menu: Menu, scene: usize, style: Style, color: ColorMode) -> Self {
        Self {
            scene,
            view: View::default(),
            paused: false,
            style,
            color,
            menu,
        }
    }

    /// Applies a key. An open menu sees it first and keeps the keys it uses.
    pub fn press(&mut self, key: Key) -> Response {
        match self.menu.handle(key) {
            Action::Highlighted(index) | Action::Chosen(index) | Action::Dismissed(index) => {
                self.show(index);
                return Response::Continue;
            }
            Action::Ignored => {}
        }

        let count = self.menu.len();
        match key {
            Key::Character(b'q' | b'Q' | 3) | Key::Escape => return Response::Quit,
            Key::Enter => self.menu.open(self.scene),
            Key::Character(digit @ b'1'..=b'9') if usize::from(digit - b'1') < count => {
                self.show(usize::from(digit - b'1'));
            }
            Key::Character(b'\t' | b'n') => self.show((self.scene + 1) % count),
            Key::BackTab | Key::Character(b'p') => self.show((self.scene + count - 1) % count),
            Key::Character(b'm') => {
                self.style = self.style.next();
                return Response::Restyle;
            }
            Key::Character(b'c') => {
                self.color = self.color.next();
                return Response::Restyle;
            }
            Key::Left | Key::Character(b'a') => self.view.yaw -= TURN,
            Key::Right | Key::Character(b'd') => self.view.yaw += TURN,
            Key::Up | Key::Character(b'w') => self.view.pitch += TURN,
            Key::Down | Key::Character(b's') => self.view.pitch -= TURN,
            Key::Character(b'+' | b'=') => self.view.zoom = (self.view.zoom * 1.1).min(8.0),
            Key::Character(b'-' | b'_') => self.view.zoom = (self.view.zoom / 1.1).max(0.25),
            Key::Character(b'r') => self.view = View::default(),
            Key::Character(b' ') => self.paused = !self.paused,
            _ => {}
        }
        Response::Continue
    }

    // A different scene starts from the default view.
    fn show(&mut self, index: usize) {
        if index != self.scene {
            self.scene = index;
            self.view = View::default();
        }
    }

    /// The status line, led by the keys for finding scenes.
    pub fn status(&self, label: &str) -> String {
        let pause = if self.paused { "play" } else { "pause" };
        if self.menu.is_open() {
            format!(
                "{label} · ↑↓ browse · enter show · esc back · m {} · c {} · space {pause} · q quit",
                self.style.label(),
                self.color.label(),
            )
        } else {
            format!(
                "{label} · enter scenes · tab next · m {} · c {} · arrows turn · +/- zoom · space {pause} · q quit",
                self.style.label(),
                self.color.label(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::Entry;

    fn app(count: usize) -> App {
        let entries = (0..count)
            .map(|index| Entry {
                label: format!("Scene {}", index + 1),
                group: "Scenes",
            })
            .collect();
        App::new(Menu::new(entries), 0, Style::Ascii, ColorMode::TrueColor)
    }

    fn press(app: &mut App, keys: &[Key]) -> Vec<Response> {
        keys.iter().map(|key| app.press(*key)).collect()
    }

    #[test]
    fn enter_opens_the_menu_on_the_current_scene() {
        let mut app = app(12);
        app.press(Key::Character(b'4'));
        assert_eq!(app.press(Key::Enter), Response::Continue);
        assert!(app.menu.is_open());
        assert!(app.status("Scene 4").contains("enter show"));
        app.press(Key::Down);
        assert_eq!(app.scene, 4);
    }

    #[test]
    fn browsing_previews_each_scene_and_enter_keeps_it() {
        let mut app = app(12);
        press(&mut app, &[Key::Enter, Key::Down, Key::Down]);
        assert_eq!(app.scene, 2);
        assert!(app.menu.is_open());
        press(&mut app, &[Key::End]);
        assert_eq!(app.scene, 11);
        press(&mut app, &[Key::Enter]);
        assert_eq!(app.scene, 11);
        assert!(!app.menu.is_open());
    }

    #[test]
    fn escape_closes_the_menu_and_restores_the_scene_before_quitting() {
        let mut app = app(12);
        app.press(Key::Character(b'3'));
        let responses = press(&mut app, &[Key::Enter, Key::Down, Key::Down, Key::Escape]);
        assert!(
            responses
                .iter()
                .all(|response| *response == Response::Continue)
        );
        assert_eq!(app.scene, 2);
        assert!(!app.menu.is_open());
        assert_eq!(app.press(Key::Escape), Response::Quit);
    }

    #[test]
    fn quit_keys_work_while_the_menu_is_open() {
        for key in [Key::Character(b'q'), Key::Character(3)] {
            let mut app = app(3);
            app.press(Key::Enter);
            assert_eq!(app.press(key), Response::Quit, "{key:?}");
        }
    }

    #[test]
    fn arrows_browse_in_the_menu_and_turn_the_view_outside_it() {
        let mut app = app(5);
        app.press(Key::Up);
        assert_eq!(app.scene, 0);
        assert!(app.view.pitch > 0.0);

        app.press(Key::Enter);
        app.press(Key::Up);
        assert_eq!(app.scene, 4);
        assert_eq!(app.view, View::default());
        // Left and right still turn the preview.
        app.press(Key::Right);
        assert!(app.view.yaw > 0.0);
        assert!(app.menu.is_open());
    }

    #[test]
    fn digits_in_the_menu_choose_and_close_it() {
        let mut app = app(12);
        press(&mut app, &[Key::Enter, Key::Character(b'9')]);
        assert_eq!(app.scene, 8);
        assert!(!app.menu.is_open());
    }

    #[test]
    fn tab_and_shift_tab_cycle_through_every_scene() {
        let mut app = app(12);
        press(&mut app, &[Key::BackTab]);
        assert_eq!(app.scene, 11);
        press(&mut app, &[Key::Character(b'\t'), Key::Character(b'\t')]);
        assert_eq!(app.scene, 1);
        press(&mut app, &[Key::Character(b'p'), Key::Character(b'n')]);
        assert_eq!(app.scene, 1);
    }

    #[test]
    fn changing_scenes_resets_the_view() {
        let mut app = app(3);
        press(&mut app, &[Key::Left, Key::Character(b'+')]);
        assert_ne!(app.view, View::default());
        app.press(Key::Character(b'2'));
        assert_eq!(app.view, View::default());
    }

    #[test]
    fn style_and_color_keys_ask_for_a_complete_redraw_even_in_the_menu() {
        let mut app = app(3);
        app.press(Key::Enter);
        assert_eq!(app.press(Key::Character(b'm')), Response::Restyle);
        assert_eq!(app.style, Style::Blocks);
        assert_eq!(app.press(Key::Character(b'c')), Response::Restyle);
        assert_eq!(app.color, ColorMode::Palette);
        assert!(app.menu.is_open());
    }

    #[test]
    fn the_status_line_names_the_menu_key_first() {
        let app = app(3);
        let status = app.status("Ocean");
        assert!(status.starts_with("Ocean · enter scenes"));
    }
}
