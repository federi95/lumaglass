use super::*;
use crate::features::{
    appearance::{Dimming, Setting},
    control_center::Action,
    controls::Command,
    header::Side,
};
#[test]
fn quick_panel_restores_focus_and_never_mutates_on_open() {
    let mut state = ready();
    state.focus = Target::App("netflix".into());
    assert!(state.open_control_center().is_empty());
    assert_eq!(state.focus, Target::Quick(Action::Screensaver));
    state.focus = Target::Quick(Action::Standby);
    assert!(state.activate().is_empty());
    assert!(matches!(state.overlay, Some(Overlay::ControlCenter { .. })));
    state.back();
    assert_eq!(state.focus, Target::App("netflix".into()));
    state.webos = true;
    assert!(matches!(
        state.open_control_center().as_slice(),
        [Effect::Control {
            command: Command::QuickStatus,
            ..
        }]
    ));
    state.focus = Target::Quick(Action::Standby);
    assert!(state.activate().is_empty());
    assert!(matches!(state.overlay, Some(Overlay::ControlCenter { .. })));
}
#[test]
fn sliders_clamp_save_only_changes_and_respect_modal_scope() {
    let mut state = ready();
    let target = Target::AppearanceOption(Setting::Dimming);
    assert!(state.set_slider(&target, 2).is_empty());
    state.open_control_center();
    assert_eq!(state.set_slider(&target, 2), vec![Effect::Save]);
    assert_eq!(state.preferences.appearance.dimming, Dimming::Medium);
    assert!(state.set_slider(&target, 2).is_empty());
    state.focus = target.clone();
    state.direction(Direction::Right, &controls(&state));
    assert_eq!(state.preferences.appearance.dimming, Dimming::High);
    assert!(
        state
            .direction(Direction::Right, &controls(&state))
            .is_empty()
    );
    state.quick_action(Action::Sleep);
    assert_eq!(state.focus, Target::SleepSlider);
    state.controls.report.sleep_index = Some(0);
    assert!(state.set_slider(&Target::SleepSlider, 3).is_empty());
    assert_eq!(state.sleep_draft, 3);
    assert_eq!(state.controls.report.sleep_index, Some(0));
    state.back();
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Appearance);
    state.overlay = Some(Overlay::Keyboard);
    assert!(state.set_slider(&target, 0).is_empty());
    assert_eq!(state.preferences.appearance.dimming, Dimming::High);
}
#[test]
fn standby_confirmation_defaults_to_cancel_and_timer_is_only_applied_after_confirmation() {
    let mut state = ready();
    state.webos = true;
    state.open_control_center();
    state.controls.loading = false;
    state.controls.report.power_active = true;
    state.focus = Target::Quick(Action::Standby);
    state.activate();
    assert_eq!(state.focus, Target::CloseDialog);
    state.back();
    assert!(state.overlay.is_none());
    state.open_control_center();
    state.controls.loading = false;
    state.controls.report.sleep_index = Some(0);
    state.quick_action(Action::Sleep);
    state.set_slider(&Target::SleepSlider, 3);
    state.activate();
    assert_eq!(state.focus, Target::CloseDialog);
    state.focus = Target::ControlApply;
    assert!(matches!(
        state.activate().as_slice(),
        [Effect::Control {
            command: Command::SleepTimer {
                expected: 0,
                value: 3
            },
            ..
        }]
    ));
}
#[test]
fn clock_capsule_side_and_identity_persist_without_changing_user_text() {
    let mut state = ready();
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Clock);
    state.preferences.greeting_name = "Ada Lovelace".into();
    state.preferences.header.combined = false;
    state.preferences.header.side = Side::Left;
    state.focus = Target::ClockAvatar;
    assert_eq!(state.activate(), vec![Effect::Save]);
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::ClockSide);
    state.activate();
    assert_eq!(state.preferences.header.side, Side::Right);
    let value = serde_json::to_vec(&state.preferences).unwrap();
    let restored: Preferences = serde_json::from_slice(&value).unwrap();
    assert!(restored.header.combined);
    assert_eq!(restored.header.side, Side::Right);
    assert_eq!(restored.greeting_name, "Ada Lovelace");
    assert_eq!(
        crate::features::header::initials(&restored.greeting_name),
        "AL"
    );
    assert!(
        scene(&state)
            .controls
            .iter()
            .any(|control| control.target == Target::ControlCenter && control.rect.x > 1200.)
    );
}
#[test]
fn manual_screensaver_works_with_idle_timeout_disabled_and_consumes_wake() {
    let mut state = ready();
    state.open_control_center();
    state.quick_action(Action::Screensaver);
    assert!(state.screensaver.active);
    assert!(state.screensaver.manual);
    state.screensaver.tick(
        std::time::Instant::now(),
        true,
        crate::features::screensaver::Timeout::Off,
    );
    assert!(state.screensaver.active);
    assert!(state.screensaver.activity(std::time::Instant::now()));
    assert!(!state.screensaver.manual);
}

#[test]
fn profile_and_quick_controls_have_distinct_actions() {
    let mut state = ready();
    state.focus = Target::Profile;
    state.activate();
    assert_eq!(state.page, crate::state::Page::Home);
    assert!(matches!(state.overlay, Some(Overlay::Profile)));
    state.back();
    assert_eq!(state.focus, Target::Profile);
    state.focus = Target::ControlCenter;
    state.activate();
    assert!(matches!(state.overlay, Some(Overlay::ControlCenter { .. })));
    state.quick_action(Action::Sleep);
    assert!(state.cc_timer);
    state.back();
    assert!(!state.cc_timer);
    assert!(matches!(state.overlay, Some(Overlay::ControlCenter { .. })));
    state.quick_action(Action::Sleep);
    state.focus = Target::CloseControlCenter;
    state.activate();
    assert!(state.overlay.is_none());
}
#[test]
fn old_preferences_enable_glass_and_opaque_mode_removes_glass_commands() {
    let prefs: Preferences = serde_json::from_str(r#"{"appearance":{"accent":"blue"}}"#).unwrap();
    assert!(prefs.appearance.liquid_glass);
    assert!(prefs.header.combined);
    let mut state = ready();
    assert!(
        scene(&state)
            .draw
            .iter()
            .any(|draw| matches!(draw, crate::renderer::Draw::Glass(..)))
    );
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Appearance);
    state.focus = Target::AppearanceOption(Setting::LiquidGlass);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(
        !scene(&state)
            .draw
            .iter()
            .any(|draw| matches!(draw, crate::renderer::Draw::Glass(..)))
    );
}

#[test]
fn profile_name_returns_from_each_keyboard_route_without_editing_a_static_greeting() {
    use crate::{features::home::GreetingMode, state::TextField};
    for route in 0..3 {
        let mut state = ready();
        state.preferences.home.greeting_mode = GreetingMode::Static;
        state.preferences.home.custom_greeting = "Welcome home".into();
        state.focus = Target::Profile;
        state.activate();
        state.activate();
        assert_eq!(state.keyboard_field, TextField::ProfileName);
        state.text("Ada Lovelace");
        let effects = match route {
            0 => state.finish_text_input(),
            1 => state.back(),
            _ => {
                let done = crate::state::KEYS
                    .iter()
                    .position(|key| *key == "Done")
                    .unwrap();
                state.focus = Target::Key(done);
                state.activate()
            }
        };
        assert_eq!(effects, vec![Effect::Save]);
        assert_eq!(state.preferences.greeting_name, "Ada Lovelace");
        assert_eq!(state.preferences.home.custom_greeting, "Welcome home");
        assert!(matches!(state.overlay, Some(Overlay::Profile)));
        assert_eq!(state.focus, Target::ProfileName);
        state.back();
        assert_eq!(state.focus, Target::Profile);
        assert_eq!(state.page, crate::state::Page::Home);
    }
}
