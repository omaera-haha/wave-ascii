use std::collections::VecDeque;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthChar;

use crate::canvas::{Canvas, Cell};
use crate::color::Rgb;
use crate::store::{self, Paths};
use crate::world::{EventKind, Outcome, World};

/// Ordinary pages kept, the newest.
const PAGES_KEPT: usize = 60;
/// Milestone pages kept besides them. Each milestone is reached only once,
/// so a life's firsts stay, from the day it came into being.
const MILESTONES_KEPT: usize = 120;
const CAPTURE_DELAY: f64 = 2.0;
const CAPTURE_EXPIRES: f64 = 60.0;
const CAPTURE_INTERVAL: f64 = 300.0;
const MAX_CELLS: usize = 262_144;
static WRITES: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct ColorRun {
    length: usize,
    foreground: Option<[u8; 3]>,
    background: Option<[u8; 3]>,
}

/// A self-contained picture; colors stay in truecolor until terminal output.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Page {
    format: String,
    pub captured: i64,
    pub date: String,
    pub caption: String,
    pub name: String,
    columns: usize,
    rows: usize,
    characters: String,
    colors: Vec<ColorRun>,
    daily: Option<String>,
    /// Whether it keeps a milestone, which outlasts ordinary pages.
    milestone: bool,
}

impl Page {
    pub fn new(canvas: &Canvas, world: &World, captured: i64, caption: String) -> Self {
        let mut page = Self {
            format: "wave-ascii-album-v1".to_owned(),
            captured,
            date: world.date.to_string(),
            caption,
            name: world.ghost.name.clone(),
            columns: canvas.columns(),
            rows: canvas.rows(),
            ..Self::default()
        };
        for row in 0..page.rows {
            for column in 0..page.columns {
                let cell = canvas.get(column, row).unwrap();
                page.characters.push(cell.glyph);
                let foreground = cell.foreground.map(|color| color.0);
                let background = cell.background.map(|color| color.0);
                if let Some(run) = page.colors.last_mut()
                    && run.foreground == foreground
                    && run.background == background
                {
                    run.length += 1;
                } else {
                    page.colors.push(ColorRun {
                        length: 1,
                        foreground,
                        background,
                    });
                }
            }
        }
        page
    }

    fn valid(&self) -> bool {
        let Some(count) = self.columns.checked_mul(self.rows) else {
            return false;
        };
        if self.format != "wave-ascii-album-v1"
            || count == 0
            || count > MAX_CELLS
            || self.columns > 4096
            || self.rows > 4096
            || self.date.parse::<jiff::civil::Date>().is_err()
            || jiff::Timestamp::from_second(self.captured).is_err()
            || self.characters.chars().count() != count
            || self.colors.iter().any(|run| run.length == 0)
            || self
                .colors
                .iter()
                .try_fold(0_usize, |sum, run| sum.checked_add(run.length))
                != Some(count)
            || [&self.caption, &self.name]
                .iter()
                .any(|text| text.len() > 4096 || text.chars().any(char::is_control))
            || self.daily.as_ref().is_some_and(|key| key.len() > 100)
        {
            return false;
        }
        let mut continuation = false;
        for (index, character) in self.characters.chars().enumerate() {
            if continuation {
                if character != Cell::CONTINUATION.glyph {
                    return false;
                }
                continuation = false;
            } else {
                match character.width() {
                    Some(1) if !character.is_control() => {}
                    Some(2) if index % self.columns + 1 < self.columns => continuation = true,
                    _ => return false,
                }
            }
        }
        !continuation
    }

    pub fn canvas(&self) -> Canvas {
        let mut canvas = Canvas::blank(self.columns, self.rows);
        let mut characters = self.characters.chars();
        let mut index = 0;
        for run in &self.colors {
            for _ in 0..run.length {
                let Some(glyph) = characters.next() else {
                    return canvas;
                };
                canvas.set(
                    index % self.columns,
                    index / self.columns,
                    Cell {
                        glyph,
                        foreground: run.foreground.map(Rgb),
                        background: run.background.map(Rgb),
                    },
                );
                index += 1;
            }
        }
        canvas
    }

    pub fn date_words(&self) -> String {
        self.date
            .parse::<jiff::civil::Date>()
            .map(|date| date.strftime("%-d %B %Y").to_string())
            .unwrap_or_default()
    }

    fn save(&self, paths: &Paths, filename: &str) -> io::Result<PathBuf> {
        let directory = paths.album();
        store::create_directory(&directory)?;
        let prefix = format!("{filename}-");
        let first_number = fs::read_dir(&directory)?
            .flatten()
            .filter_map(|file| {
                let name = file.file_name();
                let stem = name.to_str()?.strip_suffix(".json")?;
                if stem == filename {
                    Some(1)
                } else {
                    stem.strip_prefix(&prefix)?.parse::<u64>().ok()
                }
            })
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| io::Error::other("No album filenames left"))?;
        let partial = directory.join(format!(
            ".{}-{}.partial",
            std::process::id(),
            WRITES.fetch_add(1, Ordering::Relaxed)
        ));
        let written = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&partial)?;
            serde_json::to_writer(&mut file, self)?;
            file.flush()?;
            file.sync_all()?;
            for number in first_number.. {
                let suffix = if number == 1 {
                    String::new()
                } else {
                    format!("-{number}")
                };
                let path = directory.join(format!("{filename}{suffix}.json"));
                // Linking publishes the complete file atomically without replacing a collision.
                match fs::hard_link(&partial, &path) {
                    Ok(()) => return Ok(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(error),
                }
            }
            Err(io::Error::other("No album filenames left"))
        })();
        let _ = fs::remove_file(partial);
        written
    }
}

struct Capture {
    time: f64,
    expires: f64,
    caption: String,
    milestone: bool,
    daily: Option<String>,
}

#[derive(Default)]
pub struct Album {
    pub pages: Vec<Page>,
    pub index: usize,
    pending: VecDeque<Capture>,
    elapsed: f64,
    last_capture: Option<f64>,
    seen: Option<(f64, usize)>,
    day: String,
    daily: Vec<String>,
    /// The life's milestones, which mark pages saved before pages knew
    /// whether they kept one.
    milestones: Vec<String>,
}

impl Album {
    #[cfg(test)]
    pub fn load(paths: &Paths, now: i64) -> Self {
        Self::load_remembering(paths, now, Vec::new())
    }

    /// Loads the album for a life whose `milestones` say which older pages,
    /// saved without the mark, keep one.
    pub fn load_remembering(paths: &Paths, now: i64, milestones: Vec<String>) -> Self {
        let mut album = Self {
            milestones,
            ..Self::default()
        };
        let mut entries = entries(&paths.album());
        prune(&mut entries, &album.milestones);
        album.pages = entries.into_iter().map(|(_, page)| page).collect();
        album.last_capture = album
            .pages
            .first()
            .map(|page| -(now.saturating_sub(page.captured).max(0) as f64));
        album
    }

    /// All valid saved pages, oldest first, without pruning files on disk.
    pub fn read_only(paths: &Paths, milestones: &[String]) -> Vec<Page> {
        entries(&paths.album())
            .into_iter()
            .rev()
            .map(|(_, mut page)| {
                page.milestone |= milestones.contains(&page.caption);
                page
            })
            .collect()
    }

    /// App time keeps expiry moving even when the simulation is paused.
    pub fn elapse(&mut self, seconds: f32) {
        if seconds.is_finite() && seconds > 0.0 {
            self.elapsed += f64::from(seconds);
        }
        self.pending
            .retain(|capture| self.elapsed <= capture.expires);
    }

    pub fn observe(&mut self, world: &World) {
        let day = world.date.to_string();
        if self.day != day {
            self.day = day;
            self.daily = self
                .pages
                .iter()
                .filter_map(|page| page.daily.clone())
                .filter(|key| key.starts_with(&self.day))
                .collect();
        }
        let previous = self.seen;
        let mut time = f64::NEG_INFINITY;
        let mut count = 0;
        for event in &world.events {
            if event.time == time {
                count += 1;
            } else {
                time = event.time;
                count = 1;
            }
            if previous.is_some_and(|(seen_time, seen_count)| {
                time < seen_time || (time == seen_time && count <= seen_count)
            }) {
                continue;
            }
            self.seen = Some((time, count));
            let Some((caption, daily)) = caption(&event.kind) else {
                continue;
            };
            let daily = daily.map(|key| format!("{}:{key}", self.day));
            if let Some(key) = &daily {
                if self.daily.contains(key) {
                    continue;
                }
                self.daily.push(key.clone());
            }
            let milestone = matches!(event.kind, EventKind::Milestone(_));
            // A first already earns a milestone; keep its own words on one picture.
            if milestone
                && let Some(capture) = self
                    .pending
                    .iter_mut()
                    .rev()
                    .find(|capture| capture.time == time && !capture.milestone)
            {
                capture.caption = caption;
                capture.milestone = true;
                continue;
            }
            if !milestone
                && (self
                    .last_capture
                    .is_some_and(|last| self.elapsed - last < CAPTURE_INTERVAL)
                    || self.pending.iter().any(|capture| !capture.milestone))
            {
                continue;
            }
            self.pending.push_back(Capture {
                time,
                expires: self.elapsed + CAPTURE_EXPIRES,
                caption,
                milestone,
                daily,
            });
        }
    }

    /// Called only with a fresh, unpaused watching frame, before UI panels.
    pub fn capture(
        &mut self,
        paths: &Paths,
        world: &World,
        canvas: &Canvas,
        now: i64,
    ) -> io::Result<()> {
        while self
            .pending
            .front()
            .is_some_and(|capture| world.time >= capture.time + CAPTURE_DELAY)
        {
            let capture = self.pending.pop_front().unwrap();
            if self.elapsed > capture.expires
                || (!capture.milestone
                    && self
                        .last_capture
                        .is_some_and(|last| self.elapsed - last < CAPTURE_INTERVAL))
            {
                continue;
            }
            let mut page = Page::new(canvas, world, now, capture.caption);
            page.daily = capture.daily;
            page.milestone = capture.milestone;
            if !page.valid() {
                continue;
            }
            let seconds = (f64::from(world.hour).rem_euclid(24.0) * 3600.0).round() as u32 % 86_400;
            let filename = format!(
                "{}-{:02}{:02}{:02}",
                world.date,
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            );
            page.save(paths, &filename)?;
            self.last_capture = Some(self.elapsed);
            let mut saved = entries(&paths.album());
            prune(&mut saved, &self.milestones);
            self.pages = saved.into_iter().map(|(_, page)| page).collect();
            self.index = self.index.min(self.pages.len().saturating_sub(1));
        }
        Ok(())
    }
}

fn entries(directory: &Path) -> Vec<(PathBuf, Page)> {
    let mut entries = Vec::new();
    if let Ok(files) = fs::read_dir(directory) {
        for file in files.flatten() {
            let path = file.path();
            if path.extension().is_none_or(|extension| extension != "json")
                || !file.file_type().is_ok_and(|kind| kind.is_file())
                || !file
                    .metadata()
                    .is_ok_and(|metadata| metadata.len() <= 32 * 1024 * 1024)
            {
                continue;
            }
            if let Ok(bytes) = fs::read(&path)
                && let Ok(page) = serde_json::from_slice::<Page>(&bytes)
                && page.valid()
            {
                entries.push((path, page));
            }
        }
    }
    entries.sort_by(|(path, page), (other_path, other)| {
        other
            .captured
            .cmp(&page.captured)
            .then_with(|| filename_order(other_path).cmp(&filename_order(path)))
    });
    entries
}

fn filename_order(path: &Path) -> (&str, u64) {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    match stem.rsplit_once('-') {
        Some((base, suffix)) if base.len() == 17 => (base, suffix.parse().unwrap_or(1)),
        _ => (stem, 1),
    }
}

// Keeps the newest ordinary pages and milestone pages, each up to its own
// limit, and deletes the rest. Entries come newest first.
fn prune(entries: &mut Vec<(PathBuf, Page)>, life_milestones: &[String]) {
    let (mut ordinary, mut milestones) = (0, 0);
    entries.retain(|(path, page)| {
        let kept = if page.milestone || life_milestones.contains(&page.caption) {
            milestones += 1;
            milestones <= MILESTONES_KEPT
        } else {
            ordinary += 1;
            ordinary <= PAGES_KEPT
        };
        if !kept {
            let _ = fs::remove_file(path);
        }
        kept
    });
}

fn caption(kind: &EventKind) -> Option<(String, Option<String>)> {
    let (caption, daily) = match kind {
        // Counting motes eaten is no moment to keep a picture of.
        EventKind::Milestone(words) if words.starts_with("Ate my ") => return None,
        EventKind::Milestone(words) => (words.clone(), None),
        EventKind::WishGranted(wish) => (
            format!(
                "My wish came true: {}.",
                wish.words(&crate::world::Life::default())
            ),
            None,
        ),
        EventKind::Celebrated(crate::world::Occasion::ViewerBirthday) => (
            "Your birthday, together.".to_owned(),
            Some("occasion:your birthday".to_owned()),
        ),
        EventKind::Celebrated(occasion) => (
            format!("{} with you.", occasion.name()),
            Some(format!("occasion:{}", occasion.name())),
        ),
        EventKind::Noticed(thing) if thing.starts_with("the rainbow") => (
            "A rainbow at my window.".to_owned(),
            Some("rainbow".to_owned()),
        ),
        EventKind::Noticed(thing) if thing == "the falling snow" => (
            "Snow falling beyond my window.".to_owned(),
            Some("snow".to_owned()),
        ),
        EventKind::Composed(title) => (format!("I made up a song: {title}."), None),
        EventKind::SangMySong(title) => (format!("You sang my song back to me: {title}."), None),
        EventKind::WroteBook(title) => (format!("I wrote a picture book: {title}."), None),
        EventKind::RecognizedSelf => ("The ghost in the mirror is me!".to_owned(), None),
        EventKind::ShootingStar => ("I wished on a shooting star.".to_owned(), None),
        EventKind::FinishedBook(title) => (format!("I finished reading {title}."), None),
        EventKind::LearnedTune(tune) => (format!("You taught me a tune: {tune}."), None),
        EventKind::FoundConstellation(name) => (format!("I found a constellation: {name}."), None),
        EventKind::Built { height: 5.. } => ("My tower reached five blocks high.".to_owned(), None),
        EventKind::GameOver(Outcome::GhostWon { .. }) => {
            ("I won our game on the glass!".to_owned(), None)
        }
        EventKind::FriendArrived(name) => (format!("{name} came to visit me."), None),
        EventKind::Bloomed => ("My plant opened its flowers.".to_owned(), None),
        EventKind::Flowered(flower) => (format!("My {} opened its flowers.", flower.name()), None),
        EventKind::ReadViewerLetter => ("I read your letter.".to_owned(), None),
        EventKind::Comforted(_) => (
            "I stayed close to you.".to_owned(),
            Some("comfort".to_owned()),
        ),
        EventKind::PromiseKept(kind) if kind.trip() => ("You came home.".to_owned(), None),
        EventKind::ReadLetter => ("You read my letter.".to_owned(), None),
        EventKind::ToldStory => ("I told you a whole story.".to_owned(), None),
        EventKind::FocusEnded {
            completed: true, ..
        } => ("We finished our work together.".to_owned(), None),
        EventKind::PlanWentWell(errand) => (format!("Your {} went well!", errand.name()), None),
        EventKind::FoundTreasure { gave_up: false, .. } => {
            ("I found the treasure you hid.".to_owned(), None)
        }
        EventKind::FoldedCrane { count: 100 } => {
            ("A hundred paper cranes for you.".to_owned(), None)
        }
        EventKind::FoldedCrane { count: 1000 } => (
            "A thousand paper cranes, and a wish for you.".to_owned(),
            None,
        ),
        EventKind::Pinned { kind, name } => (format!("I {}.", kind.pinned(name)), None),
        _ => return None,
    };
    Some((caption, daily))
}

/// A self-contained memory book. Equal capture times retain their input order.
pub fn html(name: &str, pages: &[Page]) -> String {
    let capacity = 4096
        + name.len() * 6
        + pages
            .iter()
            .map(|page| {
                page.characters.len() + page.colors.len() * 80 + page.caption.len() * 6 + 256
            })
            .sum::<usize>();
    let mut output = String::with_capacity(capacity);
    output.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>");
    escape_html(&mut output, name);
    output.push_str("&#39;s album</title>\n<style>\n\
        :root { color-scheme: dark; }\n\
        * { box-sizing: border-box; }\n\
        body { margin: 0; background: #211d1b; color: #eee2d1; font-family: Georgia, serif; }\n\
        main { width: fit-content; max-width: 100%; margin: auto; padding: 2rem 1rem; }\n\
        h1 { margin: 0 0 2rem; font-weight: normal; text-align: center; }\n\
        figure { margin: 0 0 2rem; padding: 1rem; background: #2b2521; border: 1px solid #514337; border-radius: .4rem; }\n\
        pre { margin: 0; padding: 1rem; overflow-x: auto; background: #181615; color: #eee2d1; font-family: \"DejaVu Sans Mono\", \"Noto Sans Mono\", \"Noto Sans Mono CJK KR\", \"Cascadia Mono\", Menlo, Consolas, monospace; font-size: 12px; line-height: 1; font-variant-ligatures: none; white-space: pre; }\n\
        figcaption { max-width: 70ch; margin: 1rem auto 0; overflow-wrap: anywhere; }\n\
        .caption { margin: 0 0 .5rem; }\n\
        .date { color: #c7b59f; font-size: .85rem; }\n\
        .milestone { margin-left: .75rem; color: #e6bd7d; font-size: .8rem; }\n\
        @media print {\n\
          :root { color-scheme: light; }\n\
          body { background: white; color: #211d1b; }\n\
          main { padding: 0; }\n\
          figure { break-inside: avoid; page-break-inside: avoid; background: white; border-color: #ccc; }\n\
          pre { overflow: visible; font-size: 6pt; print-color-adjust: exact; -webkit-print-color-adjust: exact; }\n\
          .date, .milestone { color: #514337; }\n\
        }\n\
        </style>\n</head>\n<body>\n<main>\n<h1>");
    escape_html(&mut output, name);
    output.push_str("&#39;s album</h1>\n");
    let mut ordered: Vec<_> = pages.iter().collect();
    ordered.sort_by_key(|page| page.captured);
    for page in ordered {
        output.push_str("<figure>\n<pre>");
        let colors = page
            .colors
            .iter()
            .flat_map(|run| std::iter::repeat_n((run.foreground, run.background), run.length));
        let mut cells = page.characters.chars().zip(colors);
        let mut previous = None;
        for row in 0..page.rows {
            for _ in 0..page.columns {
                let Some((character, colors)) = cells.next() else {
                    break;
                };
                if character.is_control() {
                    continue;
                }
                if previous != Some(colors) {
                    if previous.is_some() {
                        output.push_str("</span>");
                    }
                    output.push_str("<span style=\"color:");
                    html_color(&mut output, colors.0, "inherit");
                    output.push_str(";background:");
                    html_color(&mut output, colors.1, "transparent");
                    output.push_str("\">");
                    previous = Some(colors);
                }
                escape_character(&mut output, character);
            }
            if row + 1 < page.rows {
                output.push('\n');
            }
        }
        if previous.is_some() {
            output.push_str("</span>");
        }
        output.push_str("</pre>\n<figcaption><p class=\"caption\">");
        escape_html(&mut output, &page.caption);
        output.push_str("</p><span class=\"date\">");
        escape_html(&mut output, &page.name);
        output.push_str(" · ");
        escape_html(&mut output, &page.date_words());
        output.push_str("</span>");
        if page.milestone {
            output.push_str("<span class=\"milestone\">✦ Milestone</span>");
        }
        output.push_str("</figcaption>\n</figure>\n");
    }
    output.push_str("</main>\n</body>\n</html>\n");
    output
}

fn html_color(output: &mut String, color: Option<[u8; 3]>, fallback: &str) {
    if let Some([red, green, blue]) = color {
        let _ = write!(output, "#{red:02x}{green:02x}{blue:02x}");
    } else {
        output.push_str(fallback);
    }
}

fn escape_html(output: &mut String, text: &str) {
    for character in text.chars() {
        escape_character(output, character);
    }
}

fn escape_character(output: &mut String, character: char) {
    match character {
        '&' => output.push_str("&amp;"),
        '<' => output.push_str("&lt;"),
        '>' => output.push_str("&gt;"),
        '"' => output.push_str("&quot;"),
        '\'' => output.push_str("&#39;"),
        character if !character.is_control() => output.push(character),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn kept_trips_schedule_an_ordinary_album_moment_and_everyday_promises_do_not() {
        use crate::world::promise::Return;
        assert_eq!(
            caption(&EventKind::PromiseKept(Return::Week)),
            Some(("You came home.".to_owned(), None))
        );
        assert!(caption(&EventKind::PromiseKept(Return::Tomorrow)).is_none());
        assert!(caption(&EventKind::Promised(Return::Week)).is_none());
        assert!(caption(&EventKind::PromiseLate).is_none());
    }

    #[test]
    fn completed_comfort_uses_one_ordinary_album_trigger_each_local_day() {
        let directory = scratch("comfort");
        let paths = Paths::in_directory(&directory);
        let mut album = Album::default();
        let mut world = World::new(1, 12.0);
        moment(&mut world, EventKind::Comforted(crate::world::Feeling::Sad));
        album.observe(&world);
        assert_eq!(album.pending.len(), 1);
        assert!(!album.pending[0].milestone);
        world.time += 2.0;
        album.elapse(2.0);
        album
            .capture(&paths, &world, &Canvas::blank(1, 1), 100)
            .unwrap();
        let mut album = Album::load(&paths, 500);
        world.time += 400.0;
        moment(
            &mut world,
            EventKind::Comforted(crate::world::Feeling::Tired),
        );
        album.observe(&world);
        assert!(album.pending.is_empty());
        world.date = world.date.tomorrow().unwrap();
        moment(
            &mut world,
            EventKind::Comforted(crate::world::Feeling::Tired),
        );
        album.observe(&world);
        assert_eq!(album.pending.len(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    use super::*;
    use crate::world::{Event, Occasion};
    use std::os::unix::fs::PermissionsExt;

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("wave-ascii-album-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    fn moment(world: &mut World, kind: EventKind) {
        world.events.push(Event {
            time: world.time,
            kind,
        });
    }

    #[test]
    fn page_encoding_round_trips_wide_characters_continuations_and_color_runs() {
        let mut canvas = Canvas::blank(5, 2);
        let colored = Cell {
            glyph: '한',
            foreground: Some(Rgb([21, 87, 245])),
            background: Some(Rgb([7, 15, 31])),
        };
        canvas.set(0, 0, colored);
        canvas.set(1, 0, Cell::CONTINUATION);
        canvas.set(
            4,
            1,
            Cell {
                glyph: '▀',
                ..colored
            },
        );
        let world = World::new(1, 12.0);
        let page = Page::new(&canvas, &world, 100, "My picture.".to_owned());
        let encoded = serde_json::to_vec(&page).unwrap();
        let decoded: Page = serde_json::from_slice(&encoded).unwrap();
        assert!(decoded.valid());
        assert_eq!(decoded, page);
        assert_eq!(decoded.canvas(), canvas);
        assert_eq!(decoded.colors[1].length, 8);
        assert_eq!(decoded.date_words(), "21 June 2026");
    }

    #[test]
    fn loading_skips_missing_damaged_foreign_and_unsafe_files() {
        let directory = scratch("damaged");
        let paths = Paths::in_directory(&directory);
        assert!(Album::load(&paths, 100).pages.is_empty());
        let page = Page::new(
            &Canvas::blank(2, 2),
            &World::new(1, 12.0),
            100,
            "Kept.".to_owned(),
        );
        let first = page.save(&paths, "2026-06-21-120000").unwrap();
        let second = page.save(&paths, "2026-06-21-120000").unwrap();
        assert_ne!(first, second);
        fs::write(paths.album().join("broken.json"), "{").unwrap();
        fs::write(
            paths.album().join("foreign.json"),
            "{\"name\":\"elsewhere\"}",
        )
        .unwrap();
        fs::write(
            paths.album().join("partial.partial"),
            serde_json::to_vec(&page).unwrap(),
        )
        .unwrap();
        let mut damaged = page.clone();
        damaged.colors[0].length = usize::MAX;
        damaged.save(&paths, "overflow").unwrap();
        damaged = page.clone();
        damaged.characters = "\u{1b}[2J".to_owned();
        damaged.save(&paths, "controls").unwrap();
        damaged = page.clone();
        damaged.characters = "한   ".to_owned();
        damaged.save(&paths, "split-wide-character").unwrap();
        assert_eq!(Album::load(&paths, 100).pages, vec![page.clone(), page]);
        assert_eq!(
            fs::metadata(paths.album()).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(first).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(
            fs::read_dir(paths.album()).unwrap().all(|file| !file
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with('.'))
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn loading_keeps_the_newest_sixty_pages_and_deletes_only_older_valid_pages() {
        let directory = scratch("retention");
        let paths = Paths::in_directory(&directory);
        let world = World::new(1, 12.0);
        for captured in 0..65 {
            Page::new(
                &Canvas::blank(1, 1),
                &world,
                captured,
                "A moment.".to_owned(),
            )
            .save(&paths, &format!("page-{captured}"))
            .unwrap();
        }
        fs::write(paths.album().join("foreign.json"), "{}").unwrap();
        let album = Album::load(&paths, 1000);
        assert_eq!(album.pages.len(), 60);
        assert_eq!(album.pages.first().unwrap().captured, 64);
        assert_eq!(album.pages.last().unwrap().captured, 5);
        assert_eq!(fs::read_dir(paths.album()).unwrap().count(), 61);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn pages_in_the_same_second_stay_in_capture_order_and_retention_runs_on_capture() {
        let directory = scratch("same-second");
        let paths = Paths::in_directory(&directory);
        let mut world = World::new(1, 12.0);
        let canvas = Canvas::blank(1, 1);
        let mut album = Album::default();
        for number in 0..62 {
            moment(
                &mut world,
                EventKind::Milestone(format!("Moment {number}.")),
            );
        }
        album.observe(&world);
        world.time = 2.0;
        album.elapse(2.0);
        album.capture(&paths, &world, &canvas, 100).unwrap();
        assert_eq!(album.pages.len(), 62);
        assert_eq!(album.pages[0].caption, "Moment 61.");
        assert_eq!(album.pages[61].caption, "Moment 0.");
        assert_eq!(fs::read_dir(paths.album()).unwrap().count(), 62);
        assert_eq!(Album::load(&paths, 100).pages, album.pages);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn counted_motes_make_no_picture_but_a_first_mote_does() {
        assert!(
            caption(&EventKind::Milestone(
                "Ate my hundredth light mote.".to_owned()
            ))
            .is_none()
        );
        assert!(
            caption(&EventKind::Milestone(
                "Caught my first light mote.".to_owned()
            ))
            .is_some()
        );
    }

    #[test]
    fn pages_saved_before_the_mark_still_keep_the_lifes_milestones() {
        let directory = scratch("legacy-milestones");
        let paths = Paths::in_directory(&directory);
        let world = World::new(1, 12.0);
        let canvas = Canvas::blank(1, 1);
        Page::new(
            &canvas,
            &world,
            0,
            "I came into being in a glass box.".to_owned(),
        )
        .save(&paths, "page-first")
        .unwrap();
        for captured in 1..=(PAGES_KEPT as i64 + 5) {
            Page::new(&canvas, &world, captured, "A moment.".to_owned())
                .save(&paths, &format!("page-{captured}"))
                .unwrap();
        }
        let milestones = vec!["I came into being in a glass box.".to_owned()];
        let album = Album::load_remembering(&paths, 1000, milestones);
        assert_eq!(album.pages.len(), PAGES_KEPT + 1);
        assert_eq!(
            album.pages.last().unwrap().caption,
            "I came into being in a glass box."
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn milestone_pages_outlast_ordinary_ones_up_to_their_own_limit() {
        let directory = scratch("milestones-kept");
        let paths = Paths::in_directory(&directory);
        let world = World::new(1, 12.0);
        let canvas = Canvas::blank(1, 1);
        // The first moment of a life, then a long run of ordinary pages.
        let mut first = Page::new(&canvas, &world, 0, "I came into being.".to_owned());
        first.milestone = true;
        first.save(&paths, "page-first").unwrap();
        for captured in 1..=(PAGES_KEPT as i64 + 10) {
            Page::new(&canvas, &world, captured, "A moment.".to_owned())
                .save(&paths, &format!("page-{captured}"))
                .unwrap();
        }
        let album = Album::load(&paths, 1000);
        assert_eq!(album.pages.len(), PAGES_KEPT + 1);
        assert_eq!(album.pages.last().unwrap().caption, "I came into being.");
        assert_eq!(album.pages[PAGES_KEPT - 1].captured, 11);
        // Milestones beyond their own limit go, oldest first.
        for captured in 0..(MILESTONES_KEPT as i64 + 1) {
            let mut page = Page::new(
                &canvas,
                &world,
                2000 + captured,
                "Another first.".to_owned(),
            );
            page.milestone = true;
            page.save(&paths, &format!("first-{captured}")).unwrap();
        }
        let album = Album::load(&paths, 5000);
        assert_eq!(
            album.pages.iter().filter(|page| page.milestone).count(),
            MILESTONES_KEPT
        );
        assert!(
            album
                .pages
                .iter()
                .all(|page| page.caption != "I came into being.")
        );
        assert_eq!(
            album.pages.iter().filter(|page| !page.milestone).count(),
            PAGES_KEPT
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn the_ordinary_capture_cooldown_survives_restarting_the_app() {
        let directory = scratch("restart");
        let paths = Paths::in_directory(&directory);
        let mut world = World::new(1, 12.0);
        Page::new(&Canvas::blank(1, 1), &world, 100, "A moment.".to_owned())
            .save(&paths, "2026-06-21-120000")
            .unwrap();
        let mut album = Album::load(&paths, 200);
        moment(&mut world, EventKind::ShootingStar);
        album.observe(&world);
        assert!(album.pending.is_empty());
        world.time = 200.0;
        album.elapse(200.0);
        moment(&mut world, EventKind::ShootingStar);
        album.observe(&world);
        assert_eq!(album.pending.len(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn milestones_wait_two_seconds_and_are_observed_only_once() {
        let directory = scratch("delay");
        let paths = Paths::in_directory(&directory);
        let mut album = Album::default();
        let mut world = World::new(1, 12.0);
        moment(
            &mut world,
            EventKind::Milestone("My first moment.".to_owned()),
        );
        album.observe(&world);
        album.observe(&world);
        let canvas = Canvas::blank(2, 2);
        album.capture(&paths, &world, &canvas, 100).unwrap();
        assert!(album.pages.is_empty());
        world.time = 1.99;
        album.elapse(1.99);
        album.capture(&paths, &world, &canvas, 101).unwrap();
        assert!(album.pages.is_empty());
        world.time = 2.0;
        album.elapse(0.01);
        album.capture(&paths, &world, &canvas, 102).unwrap();
        assert_eq!(album.pages.len(), 1);
        assert_eq!(album.pages[0].caption, "My first moment.");
        assert_eq!(album.pages[0].canvas(), canvas);
        assert!(album.pending.is_empty());
        album.observe(&world);
        assert!(album.pending.is_empty());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn ordinary_moments_are_rate_limited_but_every_milestone_gets_a_picture() {
        let directory = scratch("rate");
        let paths = Paths::in_directory(&directory);
        let mut album = Album::default();
        let mut world = World::new(1, 12.0);
        let canvas = Canvas::blank(1, 1);
        moment(&mut world, EventKind::ShootingStar);
        moment(
            &mut world,
            EventKind::Milestone("My first star.".to_owned()),
        );
        album.observe(&world);
        assert_eq!(album.pending.len(), 1);
        world.time = 2.0;
        album.elapse(2.0);
        album.capture(&paths, &world, &canvas, 100).unwrap();
        assert_eq!(album.pages[0].caption, "My first star.");
        moment(&mut world, EventKind::ToldStory);
        moment(
            &mut world,
            EventKind::Milestone("My first story.".to_owned()),
        );
        moment(
            &mut world,
            EventKind::Milestone("We grew closer.".to_owned()),
        );
        album.observe(&world);
        assert_eq!(album.pending.len(), 2);
        world.time = 4.0;
        album.elapse(2.0);
        album.capture(&paths, &world, &canvas, 102).unwrap();
        assert_eq!(album.pages.len(), 3);
        moment(&mut world, EventKind::Bloomed);
        album.observe(&world);
        assert!(album.pending.is_empty());
        world.time = 304.0;
        album.elapse(300.0);
        moment(&mut world, EventKind::Bloomed);
        album.observe(&world);
        assert_eq!(album.pending.len(), 1);
        world.time = 306.0;
        album.elapse(2.0);
        album.capture(&paths, &world, &canvas, 404).unwrap();
        assert_eq!(album.pages.len(), 4);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn a_capture_expires_after_a_minute_even_when_world_time_stands_still() {
        let mut album = Album::default();
        let mut world = World::new(1, 12.0);
        moment(&mut world, EventKind::Milestone("A moment.".to_owned()));
        album.observe(&world);
        album.elapse(60.0);
        assert_eq!(album.pending.len(), 1);
        album.elapse(0.01);
        assert!(album.pending.is_empty());
        assert_eq!(world.time, 0.0);
    }

    #[test]
    fn occasions_and_weather_are_firsts_of_the_day_and_remain_so_after_loading() {
        let directory = scratch("daily");
        let paths = Paths::in_directory(&directory);
        let mut album = Album::default();
        let mut world = World::new(1, 12.0);
        moment(&mut world, EventKind::Celebrated(Occasion::Seollal));
        album.observe(&world);
        world.time = 2.0;
        album.elapse(2.0);
        album
            .capture(&paths, &world, &Canvas::blank(1, 1), 100)
            .unwrap();
        let mut album = Album::load(&paths, 500);
        album.observe(&world);
        assert!(album.pending.is_empty());
        world.date = world.date.checked_add(jiff::Span::new().days(1)).unwrap();
        moment(&mut world, EventKind::Celebrated(Occasion::Seollal));
        album.observe(&world);
        assert_eq!(album.pending.len(), 1);
        moment(
            &mut world,
            EventKind::Noticed("the rainbow through the window".to_owned()),
        );
        moment(
            &mut world,
            EventKind::Noticed("the falling snow".to_owned()),
        );
        album.observe(&world);
        assert_eq!(album.daily.len(), 3);
        assert_eq!(album.pending.len(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn only_the_selected_memorable_events_make_album_captions() {
        for kind in [
            EventKind::Composed("Rain Song".to_owned()),
            EventKind::SangMySong("Rain Song".to_owned()),
            EventKind::WroteBook("The Little Ghost and the Bird".to_owned()),
            EventKind::ShootingStar,
            EventKind::WishGranted(crate::world::Wish::Rainbow),
            EventKind::RecognizedSelf,
            EventKind::FoundConstellation("Sleepy Heart".to_owned()),
            EventKind::Built { height: 5 },
            EventKind::GameOver(Outcome::GhostWon { line: [0, 1, 2] }),
            EventKind::FriendArrived("Pip".to_owned()),
            EventKind::Bloomed,
            EventKind::Flowered(crate::world::Flower::Tulip),
            EventKind::ToldStory,
            EventKind::FocusEnded {
                minutes: 25,
                completed: true,
            },
        ] {
            let (words, _) = caption(&kind).unwrap();
            assert!(!words.chars().any(|character| character.is_ascii_digit()));
        }
        for kind in [
            EventKind::Built { height: 4 },
            EventKind::FocusEnded {
                minutes: 5,
                completed: false,
            },
            EventKind::GameOver(Outcome::Abandoned),
            EventKind::RainStarted,
        ] {
            assert!(caption(&kind).is_none());
        }
    }

    #[test]
    fn good_news_about_a_plan_is_an_ordinary_album_moment() {
        use crate::world::agenda::Errand;
        let (words, daily) = caption(&EventKind::PlanWentWell(Errand::Interview)).unwrap();
        assert_eq!(words, "Your interview went well!");
        assert!(daily.is_none());
        assert!(caption(&EventKind::WishedLuck(Errand::Interview)).is_none());
        assert!(caption(&EventKind::PlanWentBadly(Errand::Interview)).is_none());
    }

    #[test]
    fn treasure_finds_make_album_moments_but_revealed_treasures_do_not() {
        assert!(
            caption(&EventKind::FoundTreasure {
                seconds: 18,
                hints: 2,
                gave_up: false
            })
            .is_some()
        );
        assert!(
            caption(&EventKind::FoundTreasure {
                seconds: 243,
                hints: 0,
                gave_up: true
            })
            .is_none()
        );
    }

    #[test]
    fn html_keeps_memories_in_order_with_escaped_text_and_merged_wide_character_runs() {
        let mut canvas = Canvas::blank(8, 2);
        let ink = Cell {
            glyph: ' ',
            foreground: Some(Rgb([21, 87, 245])),
            background: Some(Rgb([7, 15, 31])),
        };
        for row in 0..2 {
            for column in 0..8 {
                canvas.set(column, row, ink);
            }
        }
        for (column, glyph) in ['한', '\0', '&', '<', '>', '"', '\'', 'A']
            .into_iter()
            .enumerate()
        {
            canvas.set(
                column,
                0,
                if glyph == '\0' {
                    Cell::CONTINUATION
                } else {
                    Cell { glyph, ..ink }
                },
            );
        }
        let world = World::new(1, 12.0);
        let mut older = Page::new(
            &canvas,
            &world,
            100,
            "First 뽀삐 & < > \" '\n\u{1b}".to_owned(),
        );
        older.name = "Old & < > \" '\r".to_owned();
        older.milestone = true;
        let mut newer = Page::new(
            &Canvas::blank(1, 1),
            &world,
            200,
            "Later <script>alert('hi')</script>".to_owned(),
        );
        newer.name = "New Pip".to_owned();
        let output = html("Pip & < > \" '\t", &[newer, older]);
        assert!(output.contains("<title>Pip &amp; &lt; &gt; &quot; &#39;&#39;s album</title>"));
        assert!(output.contains("First 뽀삐 &amp; &lt; &gt; &quot; &#39;</p>"));
        assert!(output.contains("Old &amp; &lt; &gt; &quot; &#39; · 21 June 2026"));
        assert!(output.contains("Later &lt;script&gt;alert(&#39;hi&#39;)&lt;/script&gt;"));
        assert!(output.find("First 뽀삐").unwrap() < output.find("Later &lt;script&gt;").unwrap());
        assert!(output.contains("<pre><span style=\"color:#1557f5;background:#070f1f\">한&amp;&lt;&gt;&quot;&#39;A\n        </span></pre>"));
        assert_eq!(output.matches('한').count(), 1);
        assert_eq!(output.matches("<span style=").count(), 2);
        assert_eq!(output.matches("✦ Milestone").count(), 1);
        assert_eq!(output.matches("<figure>").count(), 2);
        assert!(!output.contains(['\0', '\r', '\t', '\u{1b}']));
        assert!(!output.contains("<script"));
        assert!(!output.contains("src="));
        assert!(!output.contains("href="));
        for tag in [
            "html",
            "head",
            "title",
            "style",
            "body",
            "main",
            "h1",
            "figure",
            "pre",
            "figcaption",
            "p",
            "span",
        ] {
            assert_eq!(
                output.matches(&format!("<{tag}>")).count()
                    + output.matches(&format!("<{tag} ")).count(),
                output.matches(&format!("</{tag}>")).count(),
                "unbalanced {tag}"
            );
        }
        let mut nesting = Vec::new();
        for tag in output.split('<').skip(1) {
            let tag = tag.split_once('>').unwrap().0;
            let name = tag.split_whitespace().next().unwrap();
            if let Some(name) = name.strip_prefix('/') {
                assert_eq!(nesting.pop(), Some(name));
            } else if name != "!doctype" && name != "meta" {
                nesting.push(name);
            }
        }
        assert!(nesting.is_empty());
    }

    #[test]
    fn html_resets_missing_colors_and_strips_control_cells() {
        let mut canvas = Canvas::blank(4, 1);
        canvas.set(
            0,
            0,
            Cell {
                glyph: 'a',
                foreground: Some(Rgb([1, 2, 3])),
                background: None,
            },
        );
        canvas.set(
            1,
            0,
            Cell {
                glyph: 'b',
                foreground: None,
                background: Some(Rgb([4, 5, 6])),
            },
        );
        canvas.set(
            2,
            0,
            Cell {
                glyph: '\u{1b}',
                ..Cell::BLANK
            },
        );
        let page = Page::new(&canvas, &World::new(1, 12.0), 100, "Quiet.".to_owned());
        let output = html("Pip", &[page]);
        assert!(output.contains("<span style=\"color:#010203;background:transparent\">a</span>"));
        assert!(output.contains("<span style=\"color:inherit;background:#040506\">b</span>"));
        assert!(output.contains("<span style=\"color:inherit;background:transparent\"> </span>"));
        assert!(!output.contains('\u{1b}'));
    }

    #[test]
    fn reading_for_export_keeps_every_file_and_remembers_older_milestones() {
        let directory = scratch("read-only");
        let paths = Paths::in_directory(&directory);
        let world = World::new(1, 12.0);
        let canvas = Canvas::blank(1, 1);
        let first = Page::new(&canvas, &world, 0, "My first memory.".to_owned());
        first.save(&paths, "2026-06-21-120000").unwrap();
        for captured in 1..=PAGES_KEPT as i64 {
            Page::new(&canvas, &world, captured, "An ordinary moment.".to_owned())
                .save(&paths, &format!("page-{captured}"))
                .unwrap();
        }
        Page::new(
            &canvas,
            &world,
            PAGES_KEPT as i64,
            "Later in the same second.".to_owned(),
        )
        .save(&paths, "page-60")
        .unwrap();
        fs::write(paths.album().join("damaged.json"), "{").unwrap();
        let snapshot = || {
            let mut files: Vec<_> = fs::read_dir(paths.album())
                .unwrap()
                .map(|file| {
                    let path = file.unwrap().path();
                    let bytes = fs::read(&path).unwrap();
                    (path, bytes)
                })
                .collect();
            files.sort();
            files
        };
        let before = snapshot();
        let pages = Album::read_only(&paths, std::slice::from_ref(&first.caption));
        assert_eq!(pages.len(), PAGES_KEPT + 2);
        assert_eq!(pages[0].caption, first.caption);
        assert!(pages[0].milestone);
        assert_eq!(pages.last().unwrap().caption, "Later in the same second.");
        assert_eq!(snapshot(), before);
        assert_eq!(html("Pip", &pages).matches("<figure>").count(), pages.len());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn pinning_a_picture_schedules_an_ordinary_album_moment() {
        let moment = caption(&EventKind::Pinned {
            kind: crate::world::gallery::Kind::ViewerDrawing,
            name: "heart".to_owned(),
        })
        .unwrap();
        assert!(moment.0.contains("heart"));
        assert_eq!(moment.1, None);
    }
}
