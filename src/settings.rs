use crate::app::Line;
use crate::mind::claude::{Cache, Effort};
use crate::store::Config;
use crate::terminal::Key;

/// Models the arrow keys step through; any other name can be typed.
pub const CLAUDE_MODELS: [&str; 4] = [
    "claude-haiku-5-5",
    "claude-sonnet-5-5",
    "claude-opus-5-5",
    "claude-fable-5-1",
];
pub const JEV_MODELS: [&str; 2] = ["jev-latest", "jev-preview"];
const EFFORTS: [Option<Effort>; 6] = [
    None,
    Some(Effort::Low),
    Some(Effort::Medium),
    Some(Effort::High),
    Some(Effort::XHigh),
    Some(Effort::Max),
];
const CACHES: [Option<Cache>; 3] = [None, Some(Cache::FiveMinutes), Some(Cache::OneHour)];

/// One row of the settings panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    ClaudeKey,
    JevKey,
    ClaudeModel,
    Effort,
    Cache,
    JevModel,
}

impl Field {
    pub const ALL: [Self; 6] = [
        Self::ClaudeKey,
        Self::JevKey,
        Self::ClaudeModel,
        Self::Effort,
        Self::Cache,
        Self::JevModel,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ClaudeKey => "Claude key",
            Self::JevKey => "Jev key",
            Self::ClaudeModel => "Claude model",
            Self::Effort => "Effort",
            Self::Cache => "Cache",
            Self::JevModel => "Jev model",
        }
    }
}

/// The settings panel while it is open: keys, models, effort, and caching.
/// Nothing changes until it is saved with `apply`.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    /// Keys start empty; an empty field keeps the saved key.
    pub claude_key: Line,
    pub jev_key: Line,
    pub claude_model: Line,
    pub jev_model: Line,
    pub effort: Option<Effort>,
    pub cache: Option<Cache>,
    pub focus: Field,
}

impl Settings {
    pub fn from(config: &Config) -> Self {
        Self {
            claude_key: Line::default(),
            jev_key: Line::default(),
            claude_model: Line::from(config.claude_model.as_str()),
            jev_model: Line::from(config.jev_model.as_str()),
            effort: config.claude_effort,
            cache: config.cache,
            focus: Field::ClaudeKey,
        }
    }

    /// Applies a key: Up, Down, and Tab move between rows, Left and Right
    /// change a model, the effort, or the cache, and the rest edit text.
    /// Returns whether the key was used.
    pub fn press(&mut self, key: &Key) -> bool {
        let step = match key {
            Key::Left => -1,
            Key::Right => 1,
            _ => 0,
        };
        match key {
            Key::Up | Key::BackTab => self.focus = shift(&Field::ALL, self.focus, -1),
            Key::Down | Key::Tab => self.focus = shift(&Field::ALL, self.focus, 1),
            Key::Left | Key::Right => match self.focus {
                Field::ClaudeModel => step_model(&mut self.claude_model, &CLAUDE_MODELS, step),
                Field::JevModel => step_model(&mut self.jev_model, &JEV_MODELS, step),
                Field::Effort => self.effort = shift(&EFFORTS, self.effort, step),
                Field::Cache => self.cache = shift(&CACHES, self.cache, step),
                Field::ClaudeKey | Field::JevKey => {
                    return self.line().is_some_and(|line| line.edit(key));
                }
            },
            _ => return self.line().is_some_and(|line| line.edit(key)),
        }
        true
    }

    /// Pastes `text` into the row being edited, if it takes text.
    pub fn paste(&mut self, text: &str) {
        if let Some(line) = self.line() {
            line.insert(text.trim());
        }
    }

    /// Writes the settings into `config`. A key field left empty keeps its
    /// key, and a lone "-" removes it; an empty model name keeps the model.
    pub fn apply(mut self, config: &mut Config) {
        for (field, key) in [
            (&mut self.claude_key, &mut config.anthropic_api_key),
            (&mut self.jev_key, &mut config.typesafe_api_key),
        ] {
            match field.take().as_str() {
                "" => {}
                "-" => *key = None,
                text => *key = Some(text.to_owned()),
            }
        }
        for (field, model) in [
            (&mut self.claude_model, &mut config.claude_model),
            (&mut self.jev_model, &mut config.jev_model),
        ] {
            let name = field.take();
            if !name.is_empty() {
                *model = name;
            }
        }
        config.claude_effort = self.effort;
        config.cache = self.cache;
    }

    // The text the focused row edits, if it takes text.
    fn line(&mut self) -> Option<&mut Line> {
        match self.focus {
            Field::ClaudeKey => Some(&mut self.claude_key),
            Field::JevKey => Some(&mut self.jev_key),
            Field::ClaudeModel => Some(&mut self.claude_model),
            Field::JevModel => Some(&mut self.jev_model),
            Field::Effort | Field::Cache => None,
        }
    }
}

/// The effort as the panel names it.
pub fn effort_name(effort: Option<Effort>) -> &'static str {
    effort.map_or("model's default", Effort::name)
}

/// The cache lifetime as the panel names it.
pub fn cache_name(cache: Option<Cache>) -> &'static str {
    match cache {
        None => "off",
        Some(Cache::FiveMinutes) => "5 minutes",
        Some(Cache::OneHour) => "1 hour",
    }
}

// The item `step` places from `current`, wrapping around; an item not in
// the list counts as the first.
fn shift<Item: PartialEq + Copy>(items: &[Item], current: Item, step: isize) -> Item {
    let index = items.iter().position(|item| *item == current).unwrap_or(0) as isize;
    items[(index + step).rem_euclid(items.len() as isize) as usize]
}

fn step_model(line: &mut Line, models: &[&str], step: isize) {
    let index = models.iter().position(|model| *model == line.text());
    let next = match index {
        Some(index) => (index as isize + step).rem_euclid(models.len() as isize) as usize,
        // A typed name steps to the first or last preset.
        None if step > 0 => 0,
        None => models.len() - 1,
    };
    *line = Line::from(models[next]);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(settings: &mut Settings, keys: &[Key]) {
        for key in keys {
            settings.press(key);
        }
    }

    #[test]
    fn arrows_step_through_models_effort_and_cache_and_wrap() {
        let mut config = Config::default();
        let mut settings = Settings::from(&config);
        press(&mut settings, &[Key::Down, Key::Down]);
        assert_eq!(settings.focus, Field::ClaudeModel);
        press(&mut settings, &[Key::Left, Key::Left]);
        assert_eq!(settings.claude_model.text(), "claude-haiku-5-5");
        press(&mut settings, &[Key::Left]);
        assert_eq!(settings.claude_model.text(), "claude-fable-5-1");
        press(
            &mut settings,
            &[Key::Right, Key::Down, Key::Left, Key::Down, Key::Right],
        );
        assert_eq!(settings.effort, None);
        assert_eq!(settings.cache, None);
        press(&mut settings, &[Key::Down, Key::Right]);
        assert_eq!(settings.jev_model.text(), "jev-preview");
        settings.apply(&mut config);
        assert_eq!(config.claude_model, "claude-haiku-5-5");
        assert_eq!(config.claude_effort, None);
        assert_eq!(config.cache, None);
        assert_eq!(config.jev_model, "jev-preview");
    }

    #[test]
    fn any_model_name_can_be_typed() {
        let mut config = Config::default();
        let mut settings = Settings::from(&config);
        settings.focus = Field::ClaudeModel;
        settings.press(&Key::Control('u'));
        for character in "claude-sonnet-5".chars() {
            settings.press(&Key::Character(character));
        }
        settings.apply(&mut config);
        assert_eq!(config.claude_model, "claude-sonnet-5");
    }

    #[test]
    fn keys_are_kept_replaced_or_removed() {
        let mut config = Config {
            anthropic_api_key: Some("old-claude".into()),
            typesafe_api_key: Some("old-jev".into()),
            ..Config::default()
        };
        let mut settings = Settings::from(&config);
        settings.paste(" new-claude \n");
        settings.focus = Field::JevKey;
        settings.paste("-");
        settings.apply(&mut config);
        assert_eq!(config.anthropic_api_key.as_deref(), Some("new-claude"));
        assert_eq!(config.typesafe_api_key, None);

        let unchanged = config.clone();
        Settings::from(&config).apply(&mut config);
        assert_eq!(config, unchanged);
    }

    #[test]
    fn up_from_the_first_row_wraps_to_the_last() {
        let mut settings = Settings::from(&Config::default());
        settings.press(&Key::Up);
        assert_eq!(settings.focus, Field::JevModel);
    }
}
