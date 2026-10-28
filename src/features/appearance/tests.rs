use super::*;

#[test]
fn legacy_glass_choices_and_unrelated_appearance_settings_survive_loading() {
    for (value, expected) in [
        ("false", Glass::Off),
        ("true", Glass::Full),
        (r#""off""#, Glass::Off),
        (r#""light""#, Glass::Light),
        (r#""full""#, Glass::Full),
        (r#""future-mode""#, Glass::Full),
        ("null", Glass::Full),
        ("12", Glass::Full),
        ("{}", Glass::Full),
    ] {
        let prefs: Preferences = serde_json::from_str(&format!(
            r#"{{"density":"compact","accent":"teal","liquid_glass":{value}}}"#
        ))
        .unwrap();
        assert_eq!(prefs.liquid_glass, expected, "{value}");
        assert_eq!(prefs.density, Density::Compact);
        assert_eq!(prefs.accent, Accent::Teal);
    }
    let prefs: Preferences = serde_json::from_str(r#"{"accent":"blue"}"#).unwrap();
    assert_eq!(prefs.liquid_glass, Glass::Full);
    assert_eq!(prefs.accent, Accent::Blue);
}

#[test]
fn explicit_quality_round_trips_without_reverting_to_legacy_booleans() {
    for (glass, stored) in [
        (Glass::Off, "off"),
        (Glass::Light, "light"),
        (Glass::Full, "full"),
    ] {
        let original = Preferences {
            liquid_glass: glass,
            dimming: Dimming::High,
            ..Preferences::default()
        };
        let saved = serde_json::to_value(&original).unwrap();
        assert_eq!(saved["liquid_glass"], stored);
        let loaded: Preferences = serde_json::from_value(saved).unwrap();
        assert_eq!(loaded.liquid_glass, glass);
        assert_eq!(loaded.dimming, Dimming::High);
    }
}
