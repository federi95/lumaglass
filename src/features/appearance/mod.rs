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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    #[default]
    Neutral,
    Blue,
    Teal,
    Violet,
    Amber,
    Rose,
}
impl Accent {
    pub fn rgb(self) -> u32 {
        match self {
            Self::Neutral => 0xf0f0f0,
            Self::Blue => 0x9fc5ff,
            Self::Teal => 0x8cdcd3,
            Self::Violet => 0xc9b6ff,
            Self::Amber => 0xf3cf83,
            Self::Rose => 0xf2afc7,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Neutral => "accentNeutral",
            Self::Blue => "accentBlue",
            Self::Teal => "accentTeal",
            Self::Violet => "accentViolet",
            Self::Amber => "accentAmber",
            Self::Rose => "accentRose",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Neutral => Self::Blue,
            Self::Blue => Self::Teal,
            Self::Teal => Self::Violet,
            Self::Violet => Self::Amber,
            Self::Amber => Self::Rose,
            Self::Rose => Self::Neutral,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Dimming {
    #[default]
    Off,
    Low,
    Medium,
    High,
}
impl Dimming {
    pub fn alpha(self) -> f32 {
        match self {
            Self::Off => 0.,
            Self::Low => 0.2,
            Self::Medium => 0.4,
            Self::High => 0.6,
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "stateOff",
            Self::Low => "20%",
            Self::Medium => "40%",
            Self::High => "60%",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Low,
            Self::Low => Self::Medium,
            Self::Medium => Self::High,
            Self::High => Self::Off,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Blur {
    #[default]
    Off,
    Soft,
    Strong,
}
impl Blur {
    pub fn suffix(self) -> &'static str {
        match self {
            Self::Off => "",
            Self::Soft => "-blur-soft",
            Self::Strong => "-blur-strong",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "stateOff",
            Self::Soft => "appearanceBlurSoft",
            Self::Strong => "appearanceBlurStrong",
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Soft,
            Self::Soft => Self::Strong,
            Self::Strong => Self::Off,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    #[serde(deserialize_with = "fallback")]
    pub density: Density,
    #[serde(deserialize_with = "fallback")]
    pub accent: Accent,
    #[serde(deserialize_with = "fallback")]
    pub dimming: Dimming,
    #[serde(deserialize_with = "fallback")]
    pub blur: Blur,
    #[serde(default = "enabled", deserialize_with = "glass_fallback")]
    pub liquid_glass: bool,
}
fn enabled() -> bool {
    true
}
fn glass_fallback<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(serde_json::from_value(serde_json::Value::deserialize(d)?).unwrap_or(true))
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            density: Density::default(),
            accent: Accent::default(),
            dimming: Dimming::default(),
            blur: Blur::default(),
            liquid_glass: true,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Setting {
    LiquidGlass,
    Density,
    Accent,
    Dimming,
    Blur,
}
impl Setting {
    pub const ALL: [Self; 5] = [
        Self::Density,
        Self::Accent,
        Self::Dimming,
        Self::Blur,
        Self::LiquidGlass,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::LiquidGlass => "liquidGlass",
            Self::Density => "appearanceDensity",
            Self::Accent => "appearanceAccent",
            Self::Dimming => "appearanceDimming",
            Self::Blur => "appearanceBlur",
        }
    }
}
