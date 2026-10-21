use std::env;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::mind::claude::{Cache, Effort};
use crate::world::{Needs, Plant};

const APP: &str = "wave-ascii";

/// Where the app keeps its settings and the ghost's life between runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Paths {
    pub config: PathBuf,
    pub save: PathBuf,
}

impl Paths {
    /// `$XDG_CONFIG_HOME/wave-ascii/config.json` and
    /// `$XDG_DATA_HOME/wave-ascii/ghost.json`, under `~/.config` and
    /// `~/.local/share` when those aren't set.
    pub fn from_environment() -> Option<Self> {
        let home = env::var_os("HOME").map(PathBuf::from);
        let base = |variable: &str, fallback: &str| {
            env::var_os(variable)
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .or_else(|| home.as_ref().map(|home| home.join(fallback)))
        };
        Some(Self {
            config: base("XDG_CONFIG_HOME", ".config")?
                .join(APP)
                .join("config.json"),
            save: base("XDG_DATA_HOME", ".local/share")?
                .join(APP)
                .join("ghost.json"),
        })
    }

    /// Both files in one directory, for tests.
    #[cfg(test)]
    pub fn in_directory(directory: &Path) -> Self {
        Self {
            config: directory.join("config.json"),
            save: directory.join("ghost.json"),
        }
    }
}

/// Settings, including the API keys the ghost's minds use.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub anthropic_api_key: Option<String>,
    pub typesafe_api_key: Option<String>,
    pub claude_model: String,
    /// How hard Claude thinks; `null` leaves it to the model.
    pub claude_effort: Option<Effort>,
    /// How long Claude keeps the unchanging start of each request cached;
    /// `null` turns caching off.
    pub cache: Option<Cache>,
    pub jev_model: String,
    /// Seconds between the ghost's reflections while the viewer is around.
    pub reflect_every: f32,
    /// Other addresses for the APIs, such as a proxy or gateway.
    pub claude_base_url: Option<String>,
    pub jev_base_url: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            anthropic_api_key: None,
            typesafe_api_key: None,
            claude_model: "claude-opus-5-5".to_owned(),
            claude_effort: Some(Effort::Low),
            cache: Some(Cache::OneHour),
            jev_model: "jev-latest".to_owned(),
            reflect_every: 180.0,
            claude_base_url: None,
            jev_base_url: None,
        }
    }
}

// Keys never appear in debug output or logs.
impl fmt::Debug for Config {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Config")
            .field(
                "anthropic_api_key",
                &self.anthropic_api_key.as_deref().map(mask),
            )
            .field(
                "typesafe_api_key",
                &self.typesafe_api_key.as_deref().map(mask),
            )
            .field("claude_model", &self.claude_model)
            .field("claude_effort", &self.claude_effort)
            .field("cache", &self.cache)
            .field("jev_model", &self.jev_model)
            .field("reflect_every", &self.reflect_every)
            .field("claude_base_url", &self.claude_base_url)
            .field("jev_base_url", &self.jev_base_url)
            .finish()
    }
}

impl Config {
    /// The settings saved at `path`, or the defaults when there are none.
    pub fn load(path: &Path) -> io::Result<Self> {
        Ok(load_json(path)?.unwrap_or_default())
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        save_json(path, self)
    }

    /// The key to use for Claude: `ANTHROPIC_API_KEY` when it is set, else
    /// the saved one.
    pub fn anthropic_key(&self) -> Option<String> {
        key_from("ANTHROPIC_API_KEY", &self.anthropic_api_key)
    }

    /// The key to use for Jev: `TYPESAFE_API_KEY` when it is set, else the
    /// saved one.
    pub fn typesafe_key(&self) -> Option<String> {
        key_from("TYPESAFE_API_KEY", &self.typesafe_api_key)
    }
}

fn key_from(variable: &str, saved: &Option<String>) -> Option<String> {
    env::var(variable)
        .ok()
        .or_else(|| saved.clone())
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
}

/// A key shortened to its first and last few characters, enough to tell
/// keys apart without showing them.
pub fn mask(key: &str) -> String {
    let characters: Vec<char> = key.chars().collect();
    if characters.len() <= 10 {
        return "•".repeat(characters.len());
    }
    let start: String = characters[..6].iter().collect();
    let end: String = characters[characters.len() - 4..].iter().collect();
    format!("{start}…{end}")
}

/// The ghost's life as kept between runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Save {
    pub version: u32,
    pub name: String,
    /// Unix seconds when the ghost first appeared.
    pub born: i64,
    /// Unix seconds when the app last closed or saved.
    pub last_seen: i64,
    pub needs: Needs,
    pub plant: Plant,
    pub lamp_on: bool,
    pub goal: Option<String>,
    pub memories: Vec<Memory>,
    pub conversation: Vec<Remark>,
}

impl Save {
    pub const VERSION: u32 = 1;
}

/// Something the ghost decided to remember.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    /// Unix seconds.
    pub at: i64,
    pub text: String,
}

/// One turn of the conversation between the viewer and the ghost.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Remark {
    /// Unix seconds.
    pub at: i64,
    pub speaker: Speaker,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Speaker {
    Viewer,
    Ghost,
}

/// The value saved at `path`, or `None` when there is no file.
pub fn load_json<T: DeserializeOwned>(path: &Path) -> io::Result<Option<T>> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Writes `value` to `path` readable only by its owner, replacing the old
/// file only once the new one is complete.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory)?;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    }
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    let partial = path.with_extension("json.partial");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&partial)?;
    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let directory =
            env::temp_dir().join(format!("wave-ascii-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        directory
    }

    #[test]
    fn settings_round_trip_through_a_private_file() {
        let directory = scratch("settings");
        let paths = Paths::in_directory(&directory.join("nested"));
        assert_eq!(Config::load(&paths.config).unwrap(), Config::default());

        let config = Config {
            anthropic_api_key: Some("sk-ant-api03-secret-key-1234".to_owned()),
            ..Config::default()
        };
        config.save(&paths.config).unwrap();
        assert_eq!(Config::load(&paths.config).unwrap(), config);
        let mode = fs::metadata(&paths.config).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        let directory_mode = fs::metadata(paths.config.parent().unwrap())
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(directory_mode & 0o777, 0o700);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn missing_settings_fields_take_their_defaults() {
        let directory = scratch("partial");
        let path = directory.join("config.json");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, r#"{"typesafe_api_key": "ts-key"}"#).unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.typesafe_api_key.as_deref(), Some("ts-key"));
        assert_eq!(config.claude_model, "claude-opus-5-5");
        assert_eq!(config.cache, Some(Cache::OneHour));
        fs::write(&path, r#"{"claude_effort": "high", "cache": null}"#).unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.claude_effort, Some(Effort::High));
        assert_eq!(config.cache, None);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn keys_never_show_in_debug_output() {
        let config = Config {
            anthropic_api_key: Some("sk-ant-api03-very-secret-value-9876".to_owned()),
            typesafe_api_key: Some("ts-also-very-secret-5555".to_owned()),
            ..Config::default()
        };
        let debug = format!("{config:?}");
        assert!(!debug.contains("very-secret"), "{debug}");
        assert!(debug.contains("sk-ant…9876"));
        assert_eq!(mask("short"), "•••••");
    }

    #[test]
    fn blank_saved_keys_count_as_none() {
        assert_eq!(
            key_from("WAVE_ASCII_TEST_UNSET_VARIABLE", &Some("  ".to_owned())),
            None
        );
        assert_eq!(
            key_from("WAVE_ASCII_TEST_UNSET_VARIABLE", &Some(" key ".to_owned())).as_deref(),
            Some("key")
        );
    }

    #[test]
    fn a_damaged_file_is_an_error_rather_than_a_fresh_start() {
        let directory = scratch("damaged");
        let path = directory.join("ghost.json");
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, "{not json").unwrap();
        assert!(load_json::<Save>(&path).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
}
