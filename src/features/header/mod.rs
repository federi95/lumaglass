//! Local identity presentation; never changes app/device names or account data.
use serde::{Deserialize, Deserializer, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    #[default]
    Right,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Avatar {
    #[default]
    Initials,
    Symbol,
    Sun,
    Moon,
    Gaming,
    Sparkles,
    Cat,
    Robot,
    Leaf,
}
impl Avatar {
    pub const ALL: [Self; 9] = [
        Self::Initials,
        Self::Symbol,
        Self::Sun,
        Self::Moon,
        Self::Gaming,
        Self::Sparkles,
        Self::Cat,
        Self::Robot,
        Self::Leaf,
    ];
    pub fn next(self) -> Self {
        Self::ALL[(Self::ALL.iter().position(|v| *v == self).unwrap_or(0) + 1) % Self::ALL.len()]
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Initials => "avatarInitials",
            Self::Symbol => "avatarSymbol",
            Self::Sun => "avatarSun",
            Self::Moon => "avatarMoon",
            Self::Gaming => "avatarGaming",
            Self::Sparkles => "avatarSparkles",
            Self::Cat => "avatarCat",
            Self::Robot => "avatarRobot",
            Self::Leaf => "avatarLeaf",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            Self::Initials | Self::Symbol => "user",
            Self::Sun => "weather-sun",
            Self::Moon => "weather-moon",
            Self::Gaming => "gaming",
            Self::Sparkles => "sparkles",
            Self::Cat => "avatar-cat",
            Self::Robot => "avatar-robot",
            Self::Leaf => "avatar-leaf",
        }
    }
    pub fn color(self) -> u32 {
        match self {
            Self::Sun => 0xffd584,
            Self::Moon => 0xb7c5ff,
            Self::Gaming => 0x8edac3,
            Self::Sparkles => 0xedb6e6,
            Self::Cat => 0xf2c49d,
            Self::Robot => 0x9edcec,
            Self::Leaf => 0xb5d9a0,
            _ => 0xc9b6ff,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub combined: bool,
    pub side: Side,
    pub avatar: Avatar,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            combined: true,
            side: Side::Right,
            avatar: Avatar::default(),
        }
    }
}
pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Preferences, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(d)?).unwrap_or_default())
}
pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|part| part.chars().find(|c| c.is_alphanumeric()))
        .take(2)
        .flat_map(char::to_uppercase)
        .take(2)
        .collect()
}
