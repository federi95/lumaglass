use super::*;
use crate::{
    features::shortcuts::{Action, Button},
    state::{Subpage, TextField},
};

#[test]
fn malformed_shortcuts_preserve_unrelated_preferences_and_safe_defaults() {
    let defaults: Preferences = serde_json::from_value(json!({})).unwrap();
    assert!(!defaults.shortcuts.enabled);
    assert_eq!(defaults.shortcuts.red, Action::Search);
    for block in [json!(null), json!([]), json!("invalid")] {
        let preferences: Preferences = serde_json::from_value(json!({
            "greeting_name": "Ana", "shortcuts": block
        }))
        .unwrap();
        assert_eq!(preferences.greeting_name, "Ana");
        assert!(!preferences.shortcuts.enabled);
    }
    let preferences: Preferences = serde_json::from_value(json!({
        "shortcuts": {"enabled":"yes", "red":"unknown", "green":"apps", "blue":7}
    }))
    .unwrap();
    assert!(!preferences.shortcuts.enabled);
    assert_eq!(preferences.shortcuts.red, Action::None);
    assert_eq!(preferences.shortcuts.green, Action::Apps);
    assert_eq!(preferences.shortcuts.yellow, Action::Inputs);
    assert_eq!(preferences.shortcuts.blue, Action::None);
}

#[test]
fn held_shortcuts_cannot_retrigger_after_a_scope_or_preference_change() {
    let mut state = ready();
    assert!(state.shortcut_down(Button::Red, false, true).is_none());
    assert!(state.shortcut_session.observed(Button::Red));
    state.preferences.shortcuts.enabled = true;
    assert!(state.shortcut_down(Button::Red, false, true).is_none());
    assert!(state.shortcut_down(Button::Red, true, true).is_none());
    state.shortcut_session.release(Button::Red);
    assert!(state.shortcut_down(Button::Red, false, false).is_none());
    assert!(state.shortcut_down(Button::Red, false, true).is_none());
    state.shortcut_session.cancel();
    assert!(state.shortcut_session.observed(Button::Red));
    // A repeat arriving after focus loss still cannot navigate.
    assert!(state.shortcut_down(Button::Red, true, true).is_none());
    state.shortcut_session.release(Button::Red);
    assert_eq!(state.shortcut_down(Button::Red, false, true), Some(vec![]));
    assert_eq!(state.page, Page::Apps);
    assert!(state.overlay.is_some());
}

#[test]
fn modal_editing_reordering_launching_and_shortcut_setup_consume_the_press() {
    for context in 0..6 {
        let mut state = ready();
        state.preferences.shortcuts.enabled = true;
        match context {
            0 => state.overlay = Some(Overlay::Keyboard),
            1 => state.overlay = Some(Overlay::SystemKeyboard),
            2 => {
                state.overlay = Some(Overlay::RemovalStatus {
                    restore: Target::Nav(0),
                })
            }
            3 => state.moving = Some(state.inventory.apps[0].id.clone()),
            4 => state.launching = true,
            _ => state.subpage = Some(Subpage::Shortcuts),
        }
        let focus = state.focus.clone();
        assert!(state.shortcut_down(Button::Red, false, true).is_none());
        assert_eq!(state.focus, focus);
        state.overlay = None;
        state.moving = None;
        state.launching = false;
        state.subpage = None;
        assert!(state.shortcut_down(Button::Red, false, true).is_none());
        state.shortcut_session.release(Button::Red);
        assert!(state.shortcut_down(Button::Red, false, true).is_some());
    }
}

#[test]
fn shortcut_actions_only_navigate_and_recent_history_only_reads() {
    for action in [
        Action::Search,
        Action::Favorites,
        Action::Inputs,
        Action::Apps,
        Action::Settings,
        Action::Recents,
    ] {
        let mut state = ready();
        state.preferences.shortcuts.enabled = true;
        state.preferences.shortcuts.red = action;
        state.open(2);
        state.section = 3;
        let effects = state.shortcut_down(Button::Red, false, true).unwrap();
        match action {
            Action::Search => {
                assert_eq!(state.page, Page::Apps);
                assert_eq!(state.keyboard_field, TextField::Apps);
                assert!(state.overlay.is_some());
            }
            Action::Favorites => {
                assert_eq!(state.page, Page::Home);
                assert!(matches!(state.focus, Target::App(_) | Target::Browse));
            }
            Action::Inputs => {
                assert_eq!(state.page, Page::Home);
                assert!(matches!(state.focus, Target::Input(_)));
            }
            Action::Apps => assert_eq!(state.page, Page::Apps),
            Action::Settings => {
                assert_eq!(state.page, Page::Settings);
                assert_eq!(state.section, 0);
                assert_eq!(state.focus, Target::Sidebar(0));
            }
            Action::Recents => assert!(matches!(state.overlay, Some(Overlay::Recents { .. }))),
            Action::None => unreachable!(),
        }
        if action == Action::Recents {
            assert!(matches!(
                effects.as_slice(),
                [Effect::Recent {
                    command: crate::recents::Command::List,
                    ..
                }]
            ));
        } else {
            assert!(effects.is_empty());
        }
    }
}

#[test]
fn hidden_sections_and_empty_favorites_are_not_activated_by_shortcuts() {
    let mut state = ready();
    state.preferences.shortcuts.enabled = true;
    state.preferences.home.show_inputs = false;
    state.open(1);
    let focus = state.focus.clone();
    assert_eq!(
        state.shortcut_down(Button::Yellow, false, true),
        Some(vec![])
    );
    assert_eq!(state.page, Page::Apps);
    assert_eq!(state.focus, focus);
    assert!(state.notice.is_some());
    state.preferences.hidden = state
        .inventory
        .apps
        .iter()
        .map(|app| app.id.clone())
        .collect();
    assert_eq!(
        state.shortcut_down(Button::Green, false, true),
        Some(vec![])
    );
    assert_eq!(state.focus, Target::Browse);
    state.shortcut_session.release(Button::Red);
    state.preferences.shortcuts.red = Action::None;
    assert!(state.shortcut_down(Button::Red, false, true).is_none());
}

#[test]
fn shortcut_settings_cycle_save_and_reset_to_disabled() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Setting(13);
    assert!(state.activate().is_empty());
    assert_eq!(state.subpage, Some(Subpage::Shortcuts));
    assert_eq!(state.focus, Target::ShortcutsEnabled);
    assert_eq!(state.activate(), [Effect::Save]);
    assert!(state.preferences.shortcuts.enabled);
    state.focus = Target::ShortcutBinding(Button::Red);
    assert_eq!(state.activate(), [Effect::Save]);
    assert_eq!(state.preferences.shortcuts.red, Action::Favorites);
    state.focus = Target::ShortcutsReset;
    assert_eq!(state.activate(), [Effect::Save]);
    assert!(!state.preferences.shortcuts.enabled);
    assert_eq!(state.preferences.shortcuts.red, Action::Search);
    state.back();
    assert_eq!(state.focus, Target::Setting(13));
}
