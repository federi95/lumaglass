//! Local Home preferences, independent of TV services and app installation state.
use crate::{i18n::Language, model::valid_app_id};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GreetingMode {
    #[default]
    Timed,
    Static,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GreetingSize {
    Small,
    Medium,
    #[default]
    Large,
}
impl GreetingSize {
    pub fn pixels(self) -> f32 {
        match self {
            Self::Small => 63.2,
            Self::Medium => 76.8,
            Self::Large => 90.4,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Small => "homeSmall",
            Self::Medium => "homeMedium",
            Self::Large => "homeLarge",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Small => Self::Medium,
            Self::Medium => Self::Large,
            Self::Large => Self::Small,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", content = "id", rename_all = "lowercase")]
pub enum StartupFocus {
    #[default]
    Auto,
    App(String),
    Input(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    #[serde(deserialize_with = "enabled")]
    pub show_greeting: bool,
    #[serde(deserialize_with = "fallback")]
    pub greeting_mode: GreetingMode,
    #[serde(deserialize_with = "fallback")]
    pub greeting_size: GreetingSize,
    #[serde(deserialize_with = "fallback")]
    pub custom_greeting: String,
    #[serde(deserialize_with = "enabled")]
    pub show_date: bool,
    #[serde(deserialize_with = "fallback")]
    pub compact_date: bool,
    #[serde(deserialize_with = "enabled")]
    pub show_weekday: bool,
    #[serde(deserialize_with = "enabled")]
    pub show_inputs: bool,
    #[serde(deserialize_with = "fallback")]
    pub inputs_first: bool,
    #[serde(deserialize_with = "fallback")]
    pub favorites_dock: bool,
    #[serde(deserialize_with = "fallback")]
    pub greeting_first: bool,
    #[serde(deserialize_with = "fallback")]
    pub startup_focus: StartupFocus,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            show_greeting: true,
            greeting_mode: GreetingMode::Timed,
            greeting_size: GreetingSize::Large,
            custom_greeting: String::new(),
            show_date: true,
            compact_date: false,
            show_weekday: true,
            show_inputs: true,
            inputs_first: false,
            favorites_dock: false,
            greeting_first: false,
            startup_focus: StartupFocus::Auto,
        }
    }
}
impl Preferences {
    pub fn sanitize(&mut self) {
        self.custom_greeting = clean_greeting(&self.custom_greeting);
        if let StartupFocus::App(id) | StartupFocus::Input(id) = &self.startup_focus
            && (!valid_app_id(id) || crate::scroll_preview::is_placeholder(id))
        {
            self.startup_focus = StartupFocus::Auto;
        }
    }
    pub fn greeting(&self, language: Language, hour: u8, name: &str) -> String {
        if self.greeting_mode == GreetingMode::Static && !self.custom_greeting.is_empty() {
            self.custom_greeting.clone()
        } else {
            language.greeting(hour, name)
        }
    }
}

pub fn clean_greeting(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(80)
        .collect::<String>()
        .trim()
        .into()
}

fn enabled<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    Ok(serde_json::Value::deserialize(deserializer)?
        .as_bool()
        .unwrap_or(true))
}
fn fallback<'de, D: Deserializer<'de>, T: serde::de::DeserializeOwned + Default>(
    deserializer: D,
) -> Result<T, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(deserializer)?).unwrap_or_default())
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Preferences, D::Error> {
    let mut preferences: Preferences = fallback(deserializer)?;
    preferences.sanitize();
    Ok(preferences)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Setting {
    ShowGreeting,
    GreetingMode,
    GreetingText,
    GreetingSize,
    ShowDate,
    DateFormat,
    ShowWeekday,
    ShowInputs,
    GreetingFirst,
    InputsFirst,
    StartupFocus,
    FavoritesDock,
}
impl Setting {
    pub const ALL: [Self; 12] = [
        Self::FavoritesDock,
        Self::ShowGreeting,
        Self::GreetingMode,
        Self::GreetingText,
        Self::GreetingSize,
        Self::ShowDate,
        Self::DateFormat,
        Self::ShowWeekday,
        Self::ShowInputs,
        Self::GreetingFirst,
        Self::InputsFirst,
        Self::StartupFocus,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::FavoritesDock => "homeFavoritesDock",
            Self::ShowGreeting => "homeShowGreeting",
            Self::GreetingMode => "homeGreetingMode",
            Self::GreetingText => "homeGreetingText",
            Self::GreetingSize => "homeGreetingSize",
            Self::ShowDate => "homeShowDate",
            Self::DateFormat => "homeDateFormat",
            Self::ShowWeekday => "homeShowWeekday",
            Self::ShowInputs => "homeShowInputs",
            Self::GreetingFirst => "homeGreetingFirst",
            Self::InputsFirst => "homeInputsFirst",
            Self::StartupFocus => "homeStartupFocus",
        }
    }
}
