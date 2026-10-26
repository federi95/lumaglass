//! Typed local appearance settings and shared card geometry.
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    Compact,
    #[default]
    Standard,
    Comfortable,
}
#[derive(Clone, Copy)]
pub struct Cards {
    pub columns: usize,
    pub width: f32,
    pub height: f32,
    pub pitch: f32,
}
impl Density {
    pub const fn columns(self) -> usize {
        match self {
            Self::Compact => 8,
            Self::Standard => 6,
            Self::Comfortable => 5,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Compact => "appearanceCompact",
            Self::Standard => "appearanceStandard",
            Self::Comfortable => "appearanceComfortable",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Compact => Self::Standard,
            Self::Standard => Self::Comfortable,
            Self::Comfortable => Self::Compact,
        }
    }
    pub const fn home(self) -> Cards {
        self.cards(1664., 25.6, 256.)
    }
    pub const fn apps(self) -> Cards {
        self.cards(1752., 26.4, 270.)
    }
    const fn cards(self, available: f32, gap: f32, standard_width: f32) -> Cards {
        let columns = self.columns();
        let width = (available - (columns - 1) as f32 * gap) / columns as f32;
        Cards {
            columns,
            width,
            height: 154.4 * width / standard_width,
            pitch: width + gap,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    #[serde(deserialize_with = "fallback")]
    pub density: Density,
}
fn fallback<'de, D: Deserializer<'de>, T: serde::de::DeserializeOwned + Default>(
    deserializer: D,
) -> Result<T, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(deserializer)?).unwrap_or_default())
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Preferences, D::Error> {
    fallback(deserializer)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Setting {
    Density,
}
impl Setting {
    pub const ALL: [Self; 1] = [Self::Density];
    pub fn key(self) -> &'static str {
        match self {
            Self::Density => "appearanceDensity",
        }
    }
}
