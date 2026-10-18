mod canvas;
mod color;
mod drawing;
mod frame;
mod mesh;
mod model;
mod ocean;
mod scene;
mod terminal;

use std::env;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use canvas::Style;
use color::ColorMode;
use scene::{Scene, View};
use terminal::{Key, Size, Terminal};

const HELP: &str = "\
Usage: wave-ascii [OPTIONS] [SCENE | FILE]...

Scenes: 2d, 3d, ocean, knot, wave, duck
Files:  .svg .svgz drawings; .glb .gltf .obj .stl models
The first scene or file named is shown first; files join the scene list.

Options:
  --style ascii|blocks|braille   how pixels become characters (default ascii)
  --color auto|truecolor|256|never
                                 auto follows NO_COLOR and COLORTERM, and is
                                 plain when output is piped
  -h, --help                     show this help

Keys:
  1-9 or Tab   change scene          m   next style
  arrows/WASD  turn                  c   next color mode
  + / -        zoom                  r   reset view
  space        pause                 q, Esc, Ctrl+C   quit

Piped output produces one frame at time 0, sized by COLUMNS and LINES.";

// Radians per arrow key press.
const TURN: f32 = 0.12;

struct Options {
    scenes: Vec<Scene>,
    first: usize,
    style: Style,
    color: Option<ColorMode>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("wave-ascii: {error}");
            ExitCode::FAILURE
        }
    }
}

fn invalid_input(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// Returns `None` when help was requested.
fn parse_options() -> io::Result<Option<Options>> {
    let mut scenes = Scene::builtins();
    let mut first = None;
    let mut style = Style::Ascii;
    let mut color = None;
    let mut arguments = env::args().skip(1);

    while let Some(argument) = arguments.next() {
        let (name, inline_value) = match argument.split_once('=') {
            Some((name, value)) if name.starts_with("--") => {
                (name.to_owned(), Some(value.to_owned()))
            }
            _ => (argument.clone(), None),
        };
        let mut value = || {
            inline_value
                .clone()
                .or_else(|| arguments.next())
                .ok_or_else(|| invalid_input(format!("{name} needs a value")))
        };
        match name.as_str() {
            "-h" | "--help" => return Ok(None),
            "--style" => {
                let value = value()?;
                style = Style::parse(&value).ok_or_else(|| {
                    invalid_input(format!(
                        "unknown style {value:?}; use ascii, blocks, or braille"
                    ))
                })?;
            }
            "--color" => {
                let value = value()?;
                color = match value.as_str() {
                    "auto" => None,
                    other => Some(ColorMode::parse(other).ok_or_else(|| {
                        invalid_input(format!(
                            "unknown color mode {value:?}; use auto, truecolor, 256, or never"
                        ))
                    })?),
                };
            }
            flag if flag.starts_with('-') => {
                return Err(invalid_input(format!(
                    "unknown option {flag:?}; see --help"
                )));
            }
            scene_name => {
                let index = match Scene::NAMES.iter().position(|known| *known == scene_name) {
                    Some(index) => index,
                    None if Scene::is_loadable(Path::new(scene_name)) => {
                        scenes.push(Scene::load(Path::new(scene_name))?);
                        scenes.len() - 1
                    }
                    None => {
                        return Err(invalid_input(format!(
                            "unknown scene {scene_name:?}; use {}, or a supported file",
                            Scene::NAMES.join(", ")
                        )));
                    }
                };
                first.get_or_insert(index);
            }
        }
    }

    Ok(Some(Options {
        scenes,
        first: first.unwrap_or(0),
        style,
        color,
    }))
}

fn run() -> io::Result<()> {
    let Some(Options {
        scenes,
        first,
        mut style,
        color,
    }) = parse_options()?
    else {
        println!("{HELP}");
        return Ok(());
    };

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        let size = Size::from_environment();
        let color = color.unwrap_or(ColorMode::Plain);
        let frame = scenes[first].render(style.grid(size.columns, size.rows), 0.0, View::default());
        io::stdout()
            .lock()
            .write_all(style.canvas(&frame, color).ansi(color).as_bytes())?;
        return Ok(());
    }

    let mut color = color.unwrap_or_else(ColorMode::detect);
    let mut index = first;
    let mut view = View::default();
    let mut paused = false;
    let mut time = 0.0;

    let mut terminal = Terminal::open()?;
    let mut size = terminal.size();
    let mut last_frame = Instant::now();
    let mut last_size_check = last_frame;
    let frame_duration = Duration::from_secs_f64(1.0 / 30.0);

    loop {
        let frame_start = Instant::now();
        let mut restyled = false;
        for key in terminal.keys()? {
            match key {
                Key::Character(b'q' | b'Q' | 3) | Key::Escape => return Ok(()),
                Key::Character(digit @ b'1'..=b'9') if usize::from(digit - b'1') < scenes.len() => {
                    index = usize::from(digit - b'1');
                    view = View::default();
                }
                Key::Character(b'\t' | b'n') => {
                    index = (index + 1) % scenes.len();
                    view = View::default();
                }
                Key::Character(b'm') => {
                    style = style.next();
                    restyled = true;
                }
                Key::Character(b'c') => {
                    color = color.next();
                    restyled = true;
                }
                Key::Left | Key::Character(b'a') => view.yaw -= TURN,
                Key::Right | Key::Character(b'd') => view.yaw += TURN,
                Key::Up | Key::Character(b'w') => view.pitch += TURN,
                Key::Down | Key::Character(b's') => view.pitch -= TURN,
                Key::Character(b'+' | b'=') => view.zoom = (view.zoom * 1.1).min(8.0),
                Key::Character(b'-' | b'_') => view.zoom = (view.zoom / 1.1).max(0.25),
                Key::Character(b'r') => view = View::default(),
                Key::Character(b' ') => paused = !paused,
                _ => {}
            }
        }

        if last_size_check.elapsed() >= Duration::from_millis(250) {
            let current = terminal.size();
            if current != size {
                size = current;
                restyled = true;
            }
            last_size_check = Instant::now();
        }
        if restyled {
            terminal.clear()?;
        }

        let now = Instant::now();
        if !paused {
            time += (now - last_frame).as_secs_f32();
        }
        last_frame = now;

        // Leave a column for wrapping and a row for the status line.
        let grid = style.grid(size.columns.saturating_sub(1), size.rows.saturating_sub(1));
        let scene = &scenes[index];
        let frame = scene.render(grid, time, view);
        let status = format!(
            "{} · 1-{} scene · m {} · c {} · arrows turn · +/- zoom · space {} · q quit",
            scene.label(),
            scenes.len().min(9),
            style.label(),
            color.label(),
            if paused { "play" } else { "pause" },
        );
        terminal.draw(&style.canvas(&frame, color), color, &status)?;
        thread::sleep(frame_duration.saturating_sub(frame_start.elapsed()));
    }
}
