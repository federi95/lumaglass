//! Opt-in foreground shortcuts; only navigation actions, never shell commands.
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Button {
    Red,
    Green,
    Yellow,
    Blue,
}
impl Button {
    pub const ALL: [Self; 4] = [Self::Red, Self::Green, Self::Yellow, Self::Blue];
    pub fn index(self) -> usize {
        match self {
            Self::Red => 0,
            Self::Green => 1,
            Self::Yellow => 2,
            Self::Blue => 3,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Red => "shortcutRed",
            Self::Green => "shortcutGreen",
            Self::Yellow => "shortcutYellow",
            Self::Blue => "shortcutBlue",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    #[default]
    None,
    Search,
    Favorites,
    Inputs,
    Apps,
    Settings,
    Recents,
    ControlCenter,
}
impl Action {
    pub fn key(self) -> &'static str {
        match self {
            Self::None => "stateOff",
            Self::Search => "shortcutSearch",
            Self::Favorites => "favorites",
            Self::Inputs => "inputs",
            Self::Apps => "apps",
            Self::Settings => "settings",
            Self::Recents => "shortcutRecents",
            Self::ControlCenter => "controlCenter",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::Search,
            Self::Search => Self::Favorites,
            Self::Favorites => Self::Inputs,
            Self::Inputs => Self::Apps,
            Self::Apps => Self::Settings,
            Self::Settings => Self::Recents,
            Self::Recents => Self::ControlCenter,
            Self::ControlCenter => Self::None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    #[serde(deserialize_with = "fallback")]
    pub enabled: bool,
    #[serde(deserialize_with = "fallback")]
    pub red: Action,
    #[serde(deserialize_with = "fallback")]
    pub green: Action,
    #[serde(deserialize_with = "fallback")]
    pub yellow: Action,
    #[serde(deserialize_with = "fallback")]
    pub blue: Action,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            enabled: false,
            red: Action::Search,
            green: Action::Favorites,
            yellow: Action::Inputs,
            blue: Action::Recents,
        }
    }
}
impl Preferences {
    pub fn get(&self, button: Button) -> Action {
        match button {
            Button::Red => self.red,
            Button::Green => self.green,
            Button::Yellow => self.yellow,
            Button::Blue => self.blue,
        }
    }
    pub fn cycle(&mut self, button: Button) {
        let value = self.get(button).next();
        match button {
            Button::Red => self.red = value,
            Button::Green => self.green = value,
            Button::Yellow => self.yellow = value,
            Button::Blue => self.blue = value,
        }
    }
}
fn fallback<'de, D: Deserializer<'de>, T: serde::de::DeserializeOwned + Default>(
    deserializer: D,
) -> Result<T, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(deserializer)?).unwrap_or_default())
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Preferences, D::Error> {
    fallback(deserializer)
}

#[derive(Default)]
pub struct Session {
    held: [bool; 4],
    observed: [bool; 4],
}
impl Session {
    pub fn observed(&self, button: Button) -> bool {
        self.observed[button.index()]
    }
    pub fn down(&mut self, button: Button, repeat: bool, active: bool) -> bool {
        let index = button.index();
        self.observed[index] |= active;
        let held = std::mem::replace(&mut self.held[index], true);
        !held && !repeat
    }
    pub fn release(&mut self, button: Button) {
        self.held[button.index()] = false;
    }
    pub fn cancel(&mut self) {
        self.held = [false; 4];
    }
}
