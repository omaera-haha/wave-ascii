mod app;
mod aurora;
mod canvas;
mod color;
mod drawing;
mod frame;
mod lava;
mod menu;
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

use app::{App, Response};
use canvas::Style;
use color::ColorMode;
use menu::{Entry, Menu};
use scene::{Scene, View};
use terminal::{Size, Terminal};

const HELP: &str = "\
Usage: wave-ascii [OPTIONS] [SCENE | FILE]...

{scenes}
Files:  .svg .svgz drawings; .glb .gltf .obj .stl models
The first scene or file named is shown first; files join the scene list.
With none named, the scene menu opens at the start.

Options:
  --style ascii|blocks|braille   how pixels become characters (default ascii)
  --color auto|truecolor|256|never
                                 auto follows NO_COLOR and COLORTERM, and is
                                 plain when output is piped
  --time SECONDS                 moment of the frame printed when piped
                                 (default 0)
  -h, --help                     show this help

Keys:
  Enter        scene menu            m   next style
  Tab, S-Tab   next, previous scene  c   next color mode
  1-9          scene by number       r   reset view
  arrows/WASD  turn                  space   pause
  + / -        zoom                  q, Esc, Ctrl+C   quit

In the scene menu, arrows, Home, End, and Page Up and Down move through the
list while the highlighted scene plays beside it. Enter shows it, and Esc goes
back to the scene the menu opened on.

Piped output produces one frame, sized by COLUMNS and LINES.";

struct Options {
    scenes: Vec<Scene>,
    /// The first scene named on the command line, if any.
    first: Option<usize>,
    style: Style,
    color: Option<ColorMode>,
    time: f32,
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
    let mut time = 0.0;
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
            "--time" => {
                let value = value()?;
                time = value
                    .parse::<f32>()
                    .ok()
                    .filter(|seconds| seconds.is_finite())
                    .ok_or_else(|| {
                        invalid_input(format!("--time needs a number of seconds, not {value:?}"))
                    })?;
            }
            flag if flag.starts_with('-') => {
                return Err(invalid_input(format!(
                    "unknown option {flag:?}; see --help"
                )));
            }
            scene_name => {
                let index = match Scene::names().position(|known| known == scene_name) {
                    Some(index) => index,
                    None if Scene::is_loadable(Path::new(scene_name)) => {
                        scenes.push(Scene::load(Path::new(scene_name))?);
                        scenes.len() - 1
                    }
                    None => {
                        return Err(invalid_input(format!(
                            "unknown scene {scene_name:?}; see --help for the scenes, or use a supported file"
                        )));
                    }
                };
                first.get_or_insert(index);
            }
        }
    }

    Ok(Some(Options {
        scenes,
        first,
        style,
        color,
        time,
    }))
}

// The built-in scene names for the help, by group, wrapped to 80 columns.
fn scene_list() -> String {
    const INDENT: usize = 13;
    let mut list = String::from("Scenes:\n");
    for (group, names) in Scene::groups() {
        let mut line = format!("  {:<width$}", format!("{group}:"), width = INDENT - 2);
        for (index, name) in names.iter().enumerate() {
            let separator = if index + 1 < names.len() { "," } else { "" };
            if line.len() > INDENT && line.len() + 1 + name.len() + separator.len() > 79 {
                list.push_str(line.trim_end());
                list.push('\n');
                line = " ".repeat(INDENT);
            } else if index > 0 {
                line.push(' ');
            }
            line.push_str(name);
            line.push_str(separator);
        }
        list.push_str(&line);
        list.push('\n');
    }
    list.trim_end().to_owned()
}

fn run() -> io::Result<()> {
    let Some(Options {
        scenes,
        first,
        style,
        color,
        mut time,
    }) = parse_options()?
    else {
        println!("{}", HELP.replace("{scenes}", &scene_list()));
        return Ok(());
    };

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        let size = Size::from_environment();
        let color = color.unwrap_or(ColorMode::Plain);
        let frame = scenes[first.unwrap_or(0)].render(
            style.grid(size.columns, size.rows),
            time,
            View::default(),
        );
        io::stdout()
            .lock()
            .write_all(style.canvas(&frame, color).ansi(color).as_bytes())?;
        return Ok(());
    }

    // Files named on the command line follow the built-in scenes.
    let builtin_count = Scene::names().count();
    let entries = scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| Entry {
            label: scene.label().to_owned(),
            group: if index < builtin_count {
                scene.group()
            } else {
                "Files"
            },
        })
        .collect();
    let mut app = App::new(
        Menu::new(entries),
        first.unwrap_or(0),
        style,
        color.unwrap_or_else(ColorMode::detect),
    );
    // Without a scene to start on, start by showing what there is.
    if first.is_none() {
        app.menu.open(0);
    }

    let mut terminal = Terminal::open()?;
    let mut size = terminal.size();
    let mut last_frame = Instant::now();
    let mut last_size_check = last_frame;
    let frame_duration = Duration::from_secs_f64(1.0 / 30.0);

    loop {
        let frame_start = Instant::now();
        let mut restyled = false;
        for key in terminal.keys()? {
            match app.press(key) {
                Response::Quit => return Ok(()),
                Response::Restyle => restyled = true,
                Response::Continue => {}
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
        if !app.paused {
            time += (now - last_frame).as_secs_f32();
        }
        last_frame = now;

        // Leave a column for wrapping and a row for the status line.
        let (columns, rows) = (size.columns.saturating_sub(1), size.rows.saturating_sub(1));
        let (style, color, view) = (app.style, app.color, app.view);
        let scene = &scenes[app.scene];
        let draw = |columns, rows| {
            style.canvas(&scene.render(style.grid(columns, rows), time, view), color)
        };
        let canvas = if app.menu.is_open() {
            app.menu.draw(columns, rows, draw)
        } else {
            draw(columns, rows)
        };
        terminal.draw(&canvas, color, &app.status(scene.label()))?;
        thread::sleep(frame_duration.saturating_sub(frame_start.elapsed()));
    }
}
