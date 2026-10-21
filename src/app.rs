use glam::Vec2;

use crate::canvas::Style;
use crate::color::ColorMode;
use crate::frame::{CELL_ASPECT, Grid};
use crate::menu::{Action, Menu};
use crate::scene::{Click, Pointer, View, Viewer};
use crate::terminal::{Key, Mouse, MouseAction};

// Radians per arrow key press.
const TURN: f32 = 0.12;
// Clicks this many seconds apart or less come in a row.
const CLICK_GAP: f32 = 1.0;
// The pointer is still once it has rested this many seconds.
const STILL_AFTER: f32 = 1.0;

/// What the main loop should do after a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    Continue,
    /// The style or color mode changed, so the next draw must be complete.
    Restyle,
    Quit,
}

/// Everything the keys and mouse control.
pub struct App {
    pub scene: usize,
    pub view: View,
    pub paused: bool,
    pub style: Style,
    pub color: ColorMode,
    pub menu: Menu,
    /// Seconds on the scene clock, which stops while paused.
    pub time: f32,
    pub viewer: Viewer,
    // The size of the area the scene and menu are drawn in, as of the last
    // draw.
    columns: usize,
    rows: usize,
}

impl App {
    pub fn new(menu: Menu, scene: usize, style: Style, color: ColorMode, time: f32) -> Self {
        Self {
            scene,
            view: View::default(),
            paused: false,
            style,
            color,
            menu,
            time,
            viewer: Viewer {
                active_at: time,
                shown_at: time,
                ..Viewer::default()
            },
            columns: 0,
            rows: 0,
        }
    }

    /// Sets the size of the area the scene and menu are drawn in, which
    /// places the mouse.
    pub fn resize(&mut self, columns: usize, rows: usize) {
        self.columns = columns;
        self.rows = rows;
    }

    /// Applies a key. An open menu sees it first and keeps the keys it uses.
    pub fn press(&mut self, key: Key) -> Response {
        self.notice();
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

    /// Applies a mouse event. The wheel works like the up and down arrows.
    /// Over the open menu, a click picks a scene; over the scene, the pointer
    /// and clicks are passed on to it.
    pub fn mouse(&mut self, mouse: Mouse) -> Response {
        self.notice();
        let panel = if self.menu.is_open() {
            self.menu.width(self.columns)
        } else {
            0
        };
        let on_scene = (panel..self.columns).contains(&mouse.column) && mouse.row < self.rows;
        let position = on_scene.then(|| {
            Grid::new(self.columns - panel, self.rows, CELL_ASPECT).point(Vec2::new(
                (mouse.column - panel) as f32 + 0.5,
                mouse.row as f32 + 0.5,
            ))
        });

        match mouse.action {
            MouseAction::ScrollUp => return self.press(Key::Up),
            MouseAction::ScrollDown => return self.press(Key::Down),
            MouseAction::Move => {}
            MouseAction::Press => {
                if let Some(position) = position {
                    let count = match self.viewer.click {
                        Some(click) if self.time - click.time <= CLICK_GAP => click.count + 1,
                        _ => 1,
                    };
                    self.viewer.click = Some(Click {
                        position,
                        time: self.time,
                        count,
                    });
                } else if mouse.column < panel
                    && let Some(index) = self.menu.entry_at(mouse.row)
                {
                    self.menu.choose(index);
                    self.show(index);
                }
            }
        }

        self.viewer.pointer = position.map(|position| {
            let since = match self.viewer.pointer {
                Some(pointer) if self.time - pointer.moved_at < STILL_AFTER => pointer.since,
                _ => self.time,
            };
            Pointer {
                position,
                moved_at: self.time,
                since,
            }
        });
        Response::Continue
    }

    // Records that the viewer did something now, and how long it had been
    // since they last did.
    fn notice(&mut self) {
        if self.time > self.viewer.active_at {
            self.viewer.quiet = self.time - self.viewer.active_at;
        }
        self.viewer.active_at = self.time;
    }

    // A different scene starts from the default view, without the clicks
    // made on the last one.
    fn show(&mut self, index: usize) {
        if index != self.scene {
            self.scene = index;
            self.view = View::default();
            self.viewer.click = None;
            self.viewer.shown_at = self.time;
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
        let mut app = App::new(
            Menu::new(entries),
            0,
            Style::Ascii,
            ColorMode::TrueColor,
            0.0,
        );
        app.resize(81, 21);
        app
    }

    fn mouse(column: usize, row: usize, action: MouseAction) -> Mouse {
        Mouse {
            column,
            row,
            action,
        }
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

    #[test]
    fn clicking_a_menu_entry_shows_it_and_closes_the_menu() {
        let mut app = app(12);
        app.press(Key::Enter);
        // Row 1 holds the group heading and row 2 the first entry.
        app.mouse(mouse(3, 1, MouseAction::Press));
        assert!(app.menu.is_open());
        app.mouse(mouse(3, 4, MouseAction::Press));
        assert_eq!(app.scene, 2);
        assert!(!app.menu.is_open());
        assert_eq!(app.viewer.click, None);
    }

    #[test]
    fn the_wheel_browses_the_menu_and_turns_the_view_like_the_arrows() {
        let mut app = app(12);
        app.mouse(mouse(40, 10, MouseAction::ScrollUp));
        assert!(app.view.pitch > 0.0);
        app.press(Key::Enter);
        app.mouse(mouse(40, 10, MouseAction::ScrollDown));
        assert_eq!(app.scene, 1);
        assert!(app.menu.is_open());
    }

    #[test]
    fn the_pointer_is_placed_in_the_space_of_the_scene_beside_the_menu() {
        let mut app = app(12);
        app.mouse(mouse(40, 10, MouseAction::Move));
        assert_eq!(app.viewer.pointer.unwrap().position, Vec2::ZERO);
        app.mouse(mouse(80, 0, MouseAction::Move));
        let corner = app.viewer.pointer.unwrap().position;
        assert!(corner.x > 1.0 && corner.y > 0.9, "{corner}");

        app.press(Key::Enter);
        let panel = app.menu.width(81);
        app.mouse(mouse(panel + (81 - panel) / 2, 10, MouseAction::Move));
        assert_eq!(app.viewer.pointer.unwrap().position, Vec2::ZERO);
        // Over the menu or the status line, the pointer is off the scene.
        app.mouse(mouse(2, 10, MouseAction::Move));
        assert_eq!(app.viewer.pointer, None);
        app.mouse(mouse(40, 21, MouseAction::Move));
        assert_eq!(app.viewer.pointer, None);
    }

    #[test]
    fn a_moving_pointer_keeps_the_time_it_started_until_it_rests() {
        let mut app = app(3);
        for time in [1.0, 1.5, 2.2] {
            app.time = time;
            app.mouse(mouse(10, 10, MouseAction::Move));
        }
        let pointer = app.viewer.pointer.unwrap();
        assert_eq!((pointer.since, pointer.moved_at), (1.0, 2.2));
        app.time = 5.0;
        app.mouse(mouse(11, 10, MouseAction::Move));
        assert_eq!(app.viewer.pointer.unwrap().since, 5.0);
    }

    #[test]
    fn quick_clicks_on_the_scene_count_up_and_a_pause_starts_over() {
        let mut app = app(3);
        for time in [1.0, 1.4, 2.1] {
            app.time = time;
            app.mouse(mouse(40, 10, MouseAction::Press));
        }
        let click = app.viewer.click.unwrap();
        assert_eq!(
            (click.count, click.time, click.position),
            (3, 2.1, Vec2::ZERO)
        );
        app.time = 4.0;
        app.mouse(mouse(40, 10, MouseAction::Press));
        assert_eq!(app.viewer.click.unwrap().count, 1);
    }

    #[test]
    fn any_input_records_how_long_the_viewer_was_quiet() {
        let mut app = app(3);
        app.time = 50.0;
        app.press(Key::Character(b'x'));
        app.mouse(mouse(40, 10, MouseAction::Move));
        assert_eq!((app.viewer.active_at, app.viewer.quiet), (50.0, 50.0));
        app.time = 51.5;
        app.mouse(mouse(41, 10, MouseAction::Move));
        assert_eq!((app.viewer.active_at, app.viewer.quiet), (51.5, 1.5));
    }

    #[test]
    fn showing_another_scene_forgets_its_clicks_and_notes_when_it_was_shown() {
        let mut app = app(3);
        app.time = 7.0;
        app.mouse(mouse(40, 10, MouseAction::Press));
        app.press(Key::Character(b'r'));
        assert!(app.viewer.click.is_some());
        app.press(Key::Character(b'2'));
        assert_eq!(app.viewer.click, None);
        assert_eq!(app.viewer.shown_at, 7.0);
    }
}
