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
mod peek;
mod postcard;
mod render;
mod sdf;
mod settings;
mod store;
mod terminal;
mod ui;
mod world;

use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use glam::Vec3;

use app::{App, Response};
use canvas::Style;
use color::ColorMode;
use mind::{Journal, Mind};
use store::{Config, Paths, Save};
use terminal::{Size, Terminal};
use world::World;

/// Frames per second while the terminal is in the background.
const BACKGROUND_FPS: f64 = 24.0;
static ALBUM_EXPORTS: AtomicU64 = AtomicU64::new(0);

const HELP: &str = "\
Usage: wave-ascii [OPTIONS]

A ghost lives in a little cottage in your terminal: a room behind a glass
front, and a fenced yard with a tree, a swing, and a garden bed under a real
sky. It floats about, chases light motes, plays fetch, stacks toy blocks,
tends its plant and goes out to its garden, swings in the yard, watches the
bird and the sky, plays hide-and-seek and noughts and crosses, draws and
writes in the fog on the glass, sleeps at night, and watches you. Over days it grows up, grows close to you, learns your words and
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
  --peek                         print one line about what the saved ghost is
                                 probably doing, for a prompt or status bar
  --album-html PATH              export the saved album as one HTML memory book
  --force                        allow replacing an existing HTML export
  -h, --help                     show this help

Keys:
  t, Enter     talk to the ghost      k        API keys and models
  arrows/WASD  turn the view          j        journal (arrows scroll)
  + / -        zoom                   l        its life and growth
  m, c         style, color mode      ?        help
  r            reset the view         space    pause
  p            save a postcard        b        album of moments
  P            ask it to pose, then save a postcard
  g            noughts and crosses    1-9      choose a square
  o            focus together for twenty-five minutes (o again ends it)
  f            draw in the fog with left drags (f or Esc stops)
  x            hide a treasure: click the floor (x or Esc cancels)
  e            letters, or write when empty (left/right browse)
  w (letters)  write; Ctrl+S sends, Esc keeps draft; v browses sent
  q, Esc       quit

Mouse: click the ghost to poke it (or find it when it hides), the ball to
flick it, a block to nudge it or a tower to topple it, the window to ask it
to open or shut, the music box to wind it, a soap bubble to pop it, an envelope
to read it, the lawn to drop a light mote out in the yard, and elsewhere to drop
one in the room. Ask for “bubbles!” to start, or ask it to go outside.
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
    peek: bool,
    album_html: Option<PathBuf>,
    force: bool,
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
        peek: false,
        album_html: None,
        force: false,
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
            "--peek" => options.peek = true,
            "--album-html" => options.album_html = Some(PathBuf::from(value()?)),
            "--force" => options.force = true,
            other => {
                return Err(invalid_input(format!(
                    "unknown argument {other:?}; see --help"
                )));
            }
        }
    }
    if options.force && options.album_html.is_none() {
        return Err(invalid_input("--force needs --album-html PATH".to_owned()));
    }
    if options.peek && options.album_html.is_some() {
        return Err(invalid_input(
            "choose either --peek or --album-html PATH".to_owned(),
        ));
    }
    Ok(Some(options))
}

fn run() -> io::Result<()> {
    let Some(options) = parse_options()? else {
        println!("{HELP}");
        return Ok(());
    };

    let paths = Paths::from_environment();
    if let Some(path) = &options.album_html {
        println!("{}", export_album(paths.as_ref(), path, options.force)?);
        return Ok(());
    }
    if options.peek {
        return print_peek(paths.as_ref());
    }

    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return print_frame(&options);
    }

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
    // For checking looks: set the weather for the preview.
    if let Ok(weather) = env::var("WAVE_ASCII_WEATHER") {
        let sky = match weather.as_str() {
            "rain" | "storm" => world::Sky::Rain,
            "snow" => world::Sky::Snow,
            "rainbow" => world::Sky::Rainbow,
            _ => world::Sky::Clear,
        };
        world.weather.sky = sky;
        world.weather.storm = weather == "storm";
        world.weather.since = -100.0;
        world.weather.until = 1e9;
        if sky == world::Sky::Snow {
            world.weather.snow_cover = 0.9;
            world.weather.settled_snow = Some(-100.0);
        }
    }
    if let Ok(perch) = env::var("WAVE_ASCII_BIRD") {
        let mut random = world::Random::new(3);
        let mut bird = world::visitor::Bird::new(-10.0, 1.0, &mut random);
        bird.perch = if perch == "sill" {
            world::Perch::Sill
        } else {
            world::Perch::Tree
        };
        world.visitors.bird = Some(bird);
    }
    world.step(0.0);
    let mind = Mind::new(&Config::default(), Journal::default(), true);
    let mut app = App::new(world, mind, Config::default(), options.style, color);
    // For checking looks: start the preview with the ghost at an activity.
    if let Some(activity) = env::var("WAVE_ASCII_ACTIVITY")
        .ok()
        .and_then(|name| world::Activity::parse(&name))
    {
        app.world.begin(activity);
    }
    // Counted in whole steps so long previews don't drift with rounding.
    let time = f64::from(options.time);
    let steps = (time / 0.05 - 1e-9).ceil().max(0.0) as u64;
    for step in 0..steps {
        app.advance((time - step as f64 * 0.05).min(0.05) as f32);
    }
    // For checking looks without a terminal: the raw pixels as a PPM image.
    if let Some(path) = env::var_os("WAVE_ASCII_IMAGE") {
        let picture = app.picture(size.columns * 4, size.rows * 8);
        let mut bytes = format!("P6 {} {} 255\n", picture.width(), picture.height()).into_bytes();
        for row in 0..picture.height() {
            for column in 0..picture.width() {
                let rgb = picture.shade(column, row).rgb().clamp(Vec3::ZERO, Vec3::ONE);
                bytes.extend(rgb.to_array().map(|channel| (channel.powf(1.0 / 2.2) * 255.0) as u8));
            }
        }
        return fs::write(path, bytes);
    }
    let canvas = app.frame(size.columns, size.rows);
    io::stdout().lock().write_all(canvas.ansi(color).as_bytes())
}

// One line about the saved ghost, read without touching anything.
fn print_peek(paths: Option<&Paths>) -> io::Result<()> {
    let save: Option<Save> = match paths {
        Some(paths) => store::load_json(&paths.save)?,
        None => None,
    };
    let line = match save {
        Some(save) => {
            let now = jiff::Zoned::now();
            let away = now.timestamp().as_second().saturating_sub(save.last_seen);
            peek::glance(&save, now.datetime(), away.max(0))
        }
        None => peek::nobody().to_owned(),
    };
    println!("{line}");
    Ok(())
}

fn seed() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |duration| duration.as_nanos() as u64)
}

fn export_album(paths: Option<&Paths>, path: &Path, force: bool) -> io::Result<String> {
    let Some(paths) = paths else {
        return Ok(peek::nobody().to_owned());
    };
    let Some(save): Option<Save> = store::load_json(&paths.save)? else {
        return Ok(peek::nobody().to_owned());
    };
    let milestones: Vec<_> = save
        .life
        .milestones
        .iter()
        .map(|milestone| milestone.text.clone())
        .collect();
    let pages = album::Album::read_only(paths, &milestones);
    if pages.is_empty() {
        return Ok("The ghost hasn't kept any moments yet.".to_owned());
    }
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let destination = fs::canonicalize(directory.unwrap_or(Path::new(".")))?.join(
        path.file_name()
            .ok_or_else(|| invalid_input("--album-html needs a filename".to_owned()))?,
    );
    let existing = fs::canonicalize(path).ok();
    if fs::canonicalize(&paths.save)
        .is_ok_and(|save| save == destination || existing.as_ref() == Some(&save))
        || fs::canonicalize(paths.album()).is_ok_and(|album| destination.starts_with(album))
    {
        return Err(invalid_input(
            "choose an HTML path outside the saved life and album".to_owned(),
        ));
    }
    write_album_html(path, &album::html(&save.name, &pages), force)?;
    Ok(format!("Wrote {} pages to {}", pages.len(), path.display()))
}

fn write_album_html(path: &Path, contents: &str, force: bool) -> io::Result<()> {
    let exists = || {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "{} already exists; use --force to replace it",
                path.display()
            ),
        )
    };
    if !force && path.try_exists()? {
        return Err(exists());
    }
    let partial = path.with_extension(format!(
        "html.{}-{}.partial",
        std::process::id(),
        ALBUM_EXPORTS.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .open(&partial)?;
    let written = (|| {
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        if force {
            fs::rename(&partial, path)
        } else {
            // Like album captures, publish without replacing a concurrent writer.
            fs::hard_link(&partial, path).map_err(|error| {
                if error.kind() == io::ErrorKind::AlreadyExists {
                    exists()
                } else {
                    error
                }
            })
        }
    })();
    let _ = fs::remove_file(partial);
    written
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

    fn scratch(name: &str) -> PathBuf {
        let directory = env::temp_dir().join(format!(
            "wave-ascii-album-html-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    fn saved_album(directory: &Path, pages: usize) -> Paths {
        let paths = Paths::in_directory(directory);
        let world = World::new(1, 12.0);
        let save = Save {
            version: Save::VERSION,
            name: "Pip".to_owned(),
            born: 100,
            last_seen: 200,
            needs: world.ghost.needs,
            plant: world.plant,
            lamp_on: false,
            goal: None,
            memories: Vec::new(),
            conversation: Vec::new(),
            life: world.saved_life(),
            seedlings: Vec::new(),
            room: None,
        };
        store::save_json(&paths.save, &save).unwrap();
        for number in 0..pages {
            let page = album::Page::new(
                &canvas::Canvas::blank(2, 1),
                &world,
                100 + number as i64,
                format!("Memory {number}."),
            );
            store::save_json(&paths.album().join(format!("page-{number}.json")), &page).unwrap();
        }
        paths
    }

    #[test]
    fn album_export_reports_its_page_count_without_loading_config_or_changing_the_life() {
        let directory = scratch("report");
        let paths = saved_album(&directory, 2);
        fs::write(&paths.config, "invalid config, which must never be loaded").unwrap();
        let save = fs::read(&paths.save).unwrap();
        let first = fs::read(paths.album().join("page-0.json")).unwrap();
        let second = fs::read(paths.album().join("page-1.json")).unwrap();
        let path = directory.join("memory book.html");
        assert_eq!(
            export_album(Some(&paths), &path, false).unwrap(),
            format!("Wrote 2 pages to {}", path.display())
        );
        let output = fs::read_to_string(&path).unwrap();
        assert_eq!(output.matches("<figure>").count(), 2);
        assert!(output.find("Memory 0.").unwrap() < output.find("Memory 1.").unwrap());
        assert_eq!(fs::read(&paths.save).unwrap(), save);
        assert_eq!(fs::read(paths.album().join("page-0.json")).unwrap(), first);
        assert_eq!(fs::read(paths.album().join("page-1.json")).unwrap(), second);
        assert_eq!(fs::read_dir(paths.album()).unwrap().count(), 2);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 4);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn album_export_refuses_to_replace_a_file_unless_forced() {
        let directory = scratch("force");
        let paths = saved_album(&directory, 1);
        let path = directory.join("album.html");
        fs::write(&path, "earlier export").unwrap();
        let error = export_album(Some(&paths), &path, false).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains("--force"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "earlier export");
        assert_eq!(
            export_album(Some(&paths), &path, true).unwrap(),
            format!("Wrote 1 pages to {}", path.display())
        );
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .starts_with("<!doctype html>")
        );
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 3);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_missing_life_or_an_empty_album_writes_nothing_even_when_forced() {
        let directory = scratch("empty");
        let paths = Paths::in_directory(&directory);
        let path = directory.join("album.html");
        assert_eq!(export_album(None, &path, false).unwrap(), peek::nobody());
        assert_eq!(
            export_album(Some(&paths), &path, false).unwrap(),
            peek::nobody()
        );
        assert!(!directory.exists());
        let paths = saved_album(&directory, 0);
        assert_eq!(
            export_album(Some(&paths), &path, false).unwrap(),
            "The ghost hasn't kept any moments yet."
        );
        assert!(!path.exists());
        assert!(!paths.album().exists());
        fs::write(&path, "earlier export").unwrap();
        export_album(Some(&paths), &path, true).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "earlier export");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn even_a_forced_export_cannot_replace_the_saved_life_or_album() {
        let directory = scratch("source");
        let paths = saved_album(&directory, 1);
        let page = paths.album().join("page-0.json");
        for path in [&paths.save, &page, &paths.album().join("export.html")] {
            let before = fs::read(path).ok();
            let error = export_album(Some(&paths), path, true).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert_eq!(fs::read(path).ok(), before);
        }
        let alias = directory.join("life-link.json");
        std::os::unix::fs::symlink(&paths.save, &alias).unwrap();
        assert!(export_album(Some(&paths), &alias, true).is_err());
        assert!(alias.is_symlink());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_failed_publication_removes_its_temporary_file() {
        let directory = scratch("failure");
        let paths = saved_album(&directory, 1);
        let path = directory.join("album.html");
        fs::create_dir(&path).unwrap();
        assert!(export_album(Some(&paths), &path, true).is_err());
        assert!(path.is_dir());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 3);
        fs::remove_dir_all(directory).unwrap();
    }
}
