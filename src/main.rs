mod album;
mod app;
#[cfg(test)]
mod audit;
mod canvas;
mod color;
mod draft;
mod face;
mod frame;
mod mind;
mod postcard;
mod render;
mod sdf;
mod settings;
mod store;
mod terminal;
mod ui;
mod world;

use std::env;
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use app::{App, Response};
use canvas::Style;
use color::ColorMode;
use mind::{Journal, Mind};
use store::{Config, Paths, Save};
use terminal::{Size, Terminal};
use world::World;

/// Frames per second while the terminal is in the background.
const BACKGROUND_FPS: f64 = 24.0;

const HELP: &str = "\
Usage: wave-ascii [OPTIONS]

A ghost lives in a glass box in your terminal. It floats about, chases light
motes, plays fetch, stacks toy blocks, tends a plant and its garden, watches
the bird and the sky from the window, plays hide-and-seek and noughts and
crosses, draws and writes in the fog on the glass, sleeps at night, and
watches you. Over days it grows up, grows close to you, learns your words and
your name, forms habits, and keeps a diary. Jev (TypeSafe's System One model)
makes its quick choices and Claude gives it thoughts and words; without API
keys it lives on instinct. Press k in the app to add keys, or set
ANTHROPIC_API_KEY and TYPESAFE_API_KEY.

Its life is saved between runs in ~/.local/share/wave-ascii/ghost.json, and
its settings, keys included, in ~/.config/wave-ascii/config.json.

Options:
  --style ascii|blocks|braille   how pixels become characters (default braille)
  --color auto|truecolor|256|never
                                 auto follows NO_COLOR and COLORTERM, and is
                                 plain when output is piped
  --fps N                        frames per second (default 60)
  --offline                      don't call any model, even with keys
  --time SECONDS                 when piped: how long the world runs before
                                 the frame is printed (default 0)
  --hour HOUR                    when piped: the hour of day (default 12)
  --date YYYY-MM-DD              when piped: the date (default 2026-06-21)
  --born YYYY-MM-DD              when piped: the ghost's birth date (optional)
  -h, --help                     show this help

Keys:
  t, Enter     talk to the ghost      k        API keys and models
  arrows/WASD  turn the view          j        journal (arrows scroll)
  + / -        zoom                   l        its life and growth
  m, c         style, color mode      ?        help
  r            reset the view         space    pause
  p            save a postcard        b        album of moments
  g            noughts and crosses    1-9      choose a square
  o            focus together for twenty-five minutes (o again ends it)
  f            draw in the fog with left drags (f or Esc stops)
  e            letters, or write when empty (left/right browse)
  w (letters)  write; Ctrl+S sends, Esc keeps draft; v browses sent
  q, Esc       quit

Mouse: click the ghost to poke it (or find it when it hides), the ball to
flick it, a block to nudge it or a tower to topple it, the window to ask it
to open or shut, the music box to wind it, an envelope to read it, and elsewhere to drop a light mote.
Press on the ghost and drag across it to stroke it; drag elsewhere to turn the view,
and scroll to zoom. Right-drag draws in the fog on the glass. In the
journal, help, life, letter, or album panel, the mouse selects text. In the journal,
PageUp/PageDown and Home/End also scroll; Esc, Enter, or j closes it. In the
album, left/right flip pages, Home/End jump to newest/oldest, and Esc or b
closes it; q quits.

Piped output prints one frame of a fresh ghost, sized by COLUMNS and LINES.";

struct Options {
    style: Style,
    color: Option<ColorMode>,
    frames_per_second: f64,
    offline: bool,
    time: f32,
    hour: f32,
    date: jiff::civil::Date,
    born: Option<jiff::civil::Date>,
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

fn parse_date(value: &str) -> io::Result<jiff::civil::Date> {
    let valid_format = value.len() == 10
        && value.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        });
    if valid_format && let Ok(date) = value.parse() {
        return Ok(date);
    }
    Err(invalid_input(format!(
        "--date needs a valid date in YYYY-MM-DD format, not {value:?}"
    )))
}

/// Returns `None` when help was requested.
fn parse_options() -> io::Result<Option<Options>> {
    let mut options = Options {
        style: Style::Braille,
        color: None,
        frames_per_second: 60.0,
        offline: false,
        time: 0.0,
        hour: 12.0,
        date: jiff::civil::Date::constant(2026, 6, 21),
        born: None,
    };
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
        let number = |value: String, low: f64, high: f64, what: &str| {
            value
                .parse::<f64>()
                .ok()
                .filter(|number| (low..=high).contains(number))
                .ok_or_else(|| invalid_input(format!("{name} needs {what}, not {value:?}")))
        };
        match name.as_str() {
            "-h" | "--help" => return Ok(None),
            "--style" => {
                let value = value()?;
                options.style = Style::parse(&value).ok_or_else(|| {
                    invalid_input(format!(
                        "unknown style {value:?}; use ascii, blocks, or braille"
                    ))
                })?;
            }
            "--color" => {
                let value = value()?;
                options.color = match value.as_str() {
                    "auto" => None,
                    other => Some(ColorMode::parse(other).ok_or_else(|| {
                        invalid_input(format!(
                            "unknown color mode {value:?}; use auto, truecolor, 256, or never"
                        ))
                    })?),
                };
            }
            "--fps" => {
                options.frames_per_second =
                    number(value()?, 1.0, 1000.0, "a number from 1 to 1000")?;
            }
            "--time" => {
                options.time = number(value()?, 0.0, 86_400.0, "a number of seconds")? as f32
            }
            "--hour" => options.hour = number(value()?, 0.0, 24.0, "an hour from 0 to 24")? as f32,
            "--date" => options.date = parse_date(&value()?)?,
            "--born" => {
                options.born = Some(parse_date(&value()?).map_err(|error| {
                    invalid_input(error.to_string().replace("--date", "--born"))
                })?)
            }
            "--offline" => options.offline = true,
            other => {
                return Err(invalid_input(format!(
                    "unknown argument {other:?}; see --help"
                )));
            }
        }
    }
    Ok(Some(options))
}

fn run() -> io::Result<()> {
    let Some(options) = parse_options()? else {
        println!("{HELP}");
        return Ok(());
    };

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return print_frame(&options);
    }

    let paths = Paths::from_environment();
    let config = match &paths {
        Some(paths) => Config::load(&paths.config)?,
        None => Config::default(),
    };
    let save: Option<Save> = match &paths {
        Some(paths) => store::load_json(&paths.save).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "couldn't read the ghost's life from {}: {error}",
                    paths.save.display()
                ),
            )
        })?,
        None => None,
    };
    let mut world = World::new(seed(), app::local_hour());
    world.date = jiff::Zoned::now().date();
    let now = mind::now();
    let (journal, born) = match &save {
        Some(save) => (app::restore(&mut world, save, now), save.born),
        None => {
            world.begin_life(&jiff::Zoned::now().date().to_string());
            (Journal::default(), now)
        }
    };
    let mind = Mind::new(&config, journal, options.offline);
    let color = options.color.unwrap_or_else(ColorMode::detect);
    let mut app = App::new(world, mind, config, options.style, color).live(paths, born);

    let result = run_loop(&mut app, options.frames_per_second);
    let saved = app.save();
    if result.is_ok() {
        println!("{}", app.farewell());
    }
    result.and(saved)
}

fn run_loop(app: &mut App, frames_per_second: f64) -> io::Result<()> {
    let mut terminal = Terminal::open()?;
    let frame_duration = Duration::from_secs_f64(1.0 / frames_per_second);
    // A terminal in the background still shows the ghost, at a calmer pace.
    let background_duration = frame_duration.max(Duration::from_secs_f64(1.0 / BACKGROUND_FPS));
    let mut last_frame = Instant::now();
    let mut next_frame = last_frame;
    // Whether the next draw must be complete; it waits for the next frame.
    let mut restyled = false;

    loop {
        let wait = next_frame.saturating_duration_since(Instant::now());
        for input in terminal.inputs(wait)? {
            match app.input(input) {
                Response::Quit => return Ok(()),
                Response::Restyle => restyled = true,
                Response::Continue => {}
            }
        }
        terminal.set_mouse_capture(app.wants_mouse())?;
        if Instant::now() < next_frame {
            continue;
        }
        if restyled {
            terminal.clear()?;
            restyled = false;
        }

        let now = Instant::now();
        app.advance((now - last_frame).as_secs_f32());
        last_frame = now;
        let size = terminal.size();
        let canvas = app.frame(size.columns, size.rows);
        terminal.set_title(&app.title())?;
        terminal.draw(&canvas, app.color)?;

        // Frames start on a fixed beat. When drawing falls behind, the beat
        // starts over rather than rushing to catch up.
        next_frame += if app.focused() {
            frame_duration
        } else {
            background_duration
        };
        if next_frame < Instant::now() {
            next_frame = Instant::now();
        }
    }
}

// One frame of a fresh ghost after `--time` seconds, for pipes and files.
fn print_frame(options: &Options) -> io::Result<()> {
    let size = Size::from_environment();
    let color = options.color.unwrap_or(ColorMode::Plain);
    let mut world = World::new(1, options.hour);
    world.date = options.date;
    world.birthday = options.born;
    world.step(0.0);
    let mind = Mind::new(&Config::default(), Journal::default(), true);
    let mut app = App::new(world, mind, Config::default(), options.style, color);
    // Counted in whole steps so long previews don't drift with rounding.
    let time = f64::from(options.time);
    let steps = (time / 0.05 - 1e-9).ceil().max(0.0) as u64;
    for step in 0..steps {
        app.advance((time - step as f64 * 0.05).min(0.05) as f32);
    }
    let canvas = app.frame(size.columns, size.rows);
    io::stdout().lock().write_all(canvas.ansi(color).as_bytes())
}

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |duration| duration.as_nanos() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_require_the_calendar_and_the_documented_format() {
        assert_eq!(
            parse_date("2026-12-24").unwrap(),
            jiff::civil::Date::constant(2026, 12, 24)
        );
        assert!(parse_date("2024-02-29").is_ok());
        for value in [
            "2026-02-29",
            "2026-13-01",
            "2026-04-31",
            "2026-1-01",
            "2026-01-01T12:00",
            "tomorrow",
            "",
        ] {
            let error = parse_date(value).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(error.to_string().contains("YYYY-MM-DD"));
        }
    }
}
