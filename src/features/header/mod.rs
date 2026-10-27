//! Local identity presentation; never changes app/device names or account data.
use serde::{Deserialize, Deserializer, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    #[default]
    Left,
    Right,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Avatar {
    #[default]
    Initials,
    Symbol,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Preferences {
    pub combined: bool,
    pub side: Side,
    pub avatar: Avatar,
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
