use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::canvas::Canvas;
use crate::store::{self, Paths};
use crate::world::World;

/// The last watching frame, without the status bar, and its moment in words.
pub struct Postcard {
    canvas: Canvas,
    /// Plain blocks need a colored canvas made from the same sampled frame.
    colored: Option<Canvas>,
    filename: String,
    caption: String,
}

impl Postcard {
    pub fn new(canvas: Canvas, colored: Option<Canvas>, world: &World, doing: &str) -> Self {
        // Live hours hold whole seconds in a float. Round back to the second
        // so a small representation error never changes the minute or filename.
        let seconds = (f64::from(world.hour).rem_euclid(24.0) * 3600.0).round() as u32 % 86_400;
        let hour = seconds / 3600;
        let minute = seconds / 60 % 60;
        let second = seconds % 60;
        Self {
            canvas,
            colored,
            filename: format!("{}-{hour:02}{minute:02}{second:02}", world.date),
            caption: format!(
                "{} · {}, {hour:02}:{minute:02} · {doing}",
                world.ghost.name,
                world.date.strftime("%-d %B %Y"),
            ),
        }
    }

    /// Writes a plain and a truecolor copy, keeping any earlier postcards.
    pub fn save(&self, paths: &Paths) -> io::Result<PathBuf> {
        let directory = paths.postcards();
        store::create_directory(&directory)?;
        let text = format!("{}{}\n", self.canvas.postcard_text(), self.caption);
        let ansi = format!(
            "{}{}\x1b[0m\n",
            self.colored
                .as_ref()
                .unwrap_or(&self.canvas)
                .postcard_ansi(),
            self.caption,
        );
        for number in 1_u64.. {
            let filename = if number == 1 {
                self.filename.clone()
            } else {
                format!("{}-{number}", self.filename)
            };
            let text_path = directory.join(format!("{filename}.txt"));
            let ansi_path = directory.join(format!("{filename}.ans"));
            let create = |path: &Path| {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(path)
            };
            let mut text_file = match create(&text_path) {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            };
            let mut ansi_file = match create(&ansi_path) {
                Ok(file) => file,
                Err(error) => {
                    let _ = fs::remove_file(&text_path);
                    if error.kind() == io::ErrorKind::AlreadyExists {
                        continue;
                    }
                    return Err(error);
                }
            };
            let written = (|| {
                text_file.write_all(text.as_bytes())?;
                ansi_file.write_all(ansi.as_bytes())?;
                text_file.sync_all()?;
                ansi_file.sync_all()
            })();
            if let Err(error) = written {
                let _ = fs::remove_file(&text_path);
                let _ = fs::remove_file(&ansi_path);
                return Err(error);
            }
            return Ok(text_path);
        }
        Err(io::Error::other("No postcard filenames left"))
    }
}

/// A path for notices, shortening the home directory to `~`.
pub fn display_path(path: &Path) -> String {
    shorten_home(path, env::var_os("HOME").as_deref().map(Path::new))
}

fn shorten_home(path: &Path, home: Option<&Path>) -> String {
    match home
        .filter(|home| home.is_absolute())
        .and_then(|home| path.strip_prefix(home).ok())
    {
        Some(relative) => Path::new("~").join(relative).display().to_string(),
        None => path.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn scratch(name: &str) -> PathBuf {
        let directory =
            env::temp_dir().join(format!("wave-ascii-postcard-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    fn postcard() -> Postcard {
        let mut world = World::new(1, 15.0 + 30.0 / 60.0 + 12.0 / 3600.0);
        world.date = jiff::civil::Date::constant(2026, 10, 21);
        Postcard::new(
            Canvas::blank(3, 2),
            None,
            &world,
            "gazing out of the window",
        )
    }

    #[test]
    fn postcards_create_a_private_directory_and_keep_both_files_in_the_same_second() {
        let directory = scratch("twice");
        let paths = Paths::in_directory(&directory.join("nested"));
        let postcard = postcard();
        let first = postcard.save(&paths).unwrap();
        let second = postcard.save(&paths).unwrap();
        assert_eq!(first.file_name().unwrap(), "2026-10-21-153012.txt");
        assert_eq!(second.file_name().unwrap(), "2026-10-21-153012-2.txt");
        for path in [&first, &second] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::read_to_string(path).unwrap(),
                "\n\nBoo · 21 October 2026, 15:30 · gazing out of the window\n"
            );
            assert_eq!(
                fs::read_to_string(path.with_extension("ans")).unwrap(),
                "\x1b[0m\n\x1b[0m\nBoo · 21 October 2026, 15:30 · gazing out of the window\x1b[0m\n"
            );
        }
        for path in [&directory, &directory.join("nested"), &paths.postcards()] {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn an_existing_ansi_copy_also_reserves_the_postcard_name() {
        let directory = scratch("ansi");
        let paths = Paths::in_directory(&directory);
        store::create_directory(&paths.postcards()).unwrap();
        let reserved = paths.postcards().join("2026-10-21-153012.ans");
        fs::write(&reserved, "earlier postcard").unwrap();
        let saved = postcard().save(&paths).unwrap();
        assert_eq!(saved.file_name().unwrap(), "2026-10-21-153012-2.txt");
        assert_eq!(fs::read_to_string(reserved).unwrap(), "earlier postcard");
        assert!(!paths.postcards().join("2026-10-21-153012.txt").exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn notices_shorten_only_paths_inside_the_home_directory() {
        let home = Some(Path::new("/home/ghost"));
        assert_eq!(
            shorten_home(Path::new("/home/ghost/postcards/a.txt"), home),
            "~/postcards/a.txt"
        );
        assert_eq!(
            shorten_home(Path::new("/home/ghostly/a.txt"), home),
            "/home/ghostly/a.txt"
        );
        assert_eq!(shorten_home(Path::new("/tmp/a.txt"), None), "/tmp/a.txt");
    }
}
