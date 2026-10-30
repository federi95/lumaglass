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
    state.focus = Target::ControlCenter;
    state.activate();
    assert!(state.overlay.is_none());
}
#[test]
fn glass_quality_cycles_locally_and_applies_to_home_and_floating_controls() {
    use crate::{features::appearance::Glass, render::backdrop::Quality, renderer::Draw};
    let mut state = ready();
    assert_eq!(state.preferences.appearance.liquid_glass, Glass::Full);
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Appearance);
    state.focus = Target::AppearanceOption(Setting::LiquidGlass);
    for (mode, expected) in [
        (Glass::Off, None),
        (Glass::Light, Some(Quality::Light)),
        (Glass::Full, Some(Quality::Full)),
    ] {
        assert_eq!(state.activate(), vec![Effect::Save]);
        assert_eq!(state.preferences.appearance.liquid_glass, mode);
        assert_eq!(state.focus, Target::AppearanceOption(Setting::LiquidGlass));
        for overlay in [
            None,
            Some(Overlay::ControlCenter {
                restore: Target::Nav(2),
            }),
            Some(Overlay::Profile),
        ] {
            state.overlay = overlay;
            let current = scene(&state);
            let qualities: Vec<_> = current
                .draw
                .iter()
                .filter_map(|draw| {
                    if let Draw::Glass(_, _, _, _, effect) = draw {
                        Some(effect.quality)
                    } else {
                        None
                    }
                })
                .collect();
            match expected {
                None => assert!(qualities.is_empty()),
                Some(quality) => {
                    assert!(!qualities.is_empty());
                    assert!(qualities.iter().all(|q| *q == quality));
                }
            }
        }
        state.overlay = None;
    }
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

#[test]
fn favorites_dock_and_second_row_are_exclusive_and_keep_favorite_order() {
    use crate::{features::home::Setting as HomeSetting, state::Subpage};
    let mut state = ready();
    let favorites = state.preferences.favorites(&state.inventory.apps);
    state.open(2);
    state.subpage = Some(Subpage::Home);
    state.focus = Target::HomeOption(HomeSetting::FavoritesDock);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.dock_enabled());
    state.subpage = None;
    state.focus = Target::Setting(10);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.two_row_home_favorites);
    assert!(!state.preferences.home.favorites_dock);
    state.subpage = Some(Subpage::Home);
    state.focus = Target::HomeOption(HomeSetting::FavoritesDock);
    assert!(state.activate().is_empty());
    assert!(state.preferences.two_row_home_favorites);
    assert!(!state.dock_enabled());
    state.focus = Target::HomeOption(HomeSetting::TwoRows);
    state.activate();
    state.focus = Target::HomeOption(HomeSetting::FavoritesDock);
    state.activate();
    assert!(state.dock_enabled());
    let mut restored: Preferences =
        serde_json::from_slice(&serde_json::to_vec(&state.preferences).unwrap()).unwrap();
    restored.sanitize();
    assert!(restored.home.favorites_dock);
    assert_eq!(restored.favorites(&state.inventory.apps), favorites);
    restored.two_row_home_favorites = true;
    restored.sanitize();
    assert!(!restored.home.favorites_dock);
}

#[test]
fn favorites_dock_centers_short_rows_and_reveals_scrolled_cards_with_matching_hit_targets() {
    let mut state = ready();
    state.preferences.home.favorites_dock = true;
    state.preferences.favorites = Some(vec!["netflix".into(), "youtube.leanback.v4".into()]);
    let (left, right) = state.home_strip();
    assert!(((left + right) / 2. - 960.).abs() < 0.1);
    assert!(right - left < 1000.);
    let controls = controls(&state);
    let first = controls
        .iter()
        .find(|c| c.target == Target::App("netflix".into()))
        .unwrap();
    assert!((first.rect.x - left).abs() < 0.1);
    assert_eq!(
        state.hit_test(&controls, first.rect.center().0, first.rect.center().1),
        Some(first.target.clone())
    );
    state.scroll_favorites(10000.);
    assert_eq!(state.home_scroll, 0.);
    state.preferences.favorites = Some(
        state
            .inventory
            .apps
            .iter()
            .map(|app| app.id.clone())
            .collect(),
    );
    state.scroll_favorites(10000.);
    let last = state.apps().last().unwrap().id.clone();
    state.focus = Target::App(last);
    state.reveal_focus(&super::controls(&state));
    let controls = super::controls(&state);
    let focused = controls.iter().find(|c| c.target == state.focus).unwrap();
    let (left, right) = state.home_strip();
    assert!(focused.rect.x >= left - 0.1 && focused.rect.x + focused.rect.w <= right + 0.1);
}

#[test]
fn dock_captions_are_hidden_without_losing_the_saved_names_preference() {
    use crate::{features::home::Setting as HomeSetting, state::Subpage};
    let mut state = ready();
    state.preferences.show_home_names = true;
    state.preferences.home.favorites_dock = true;
    assert!(!state.home_names_visible());
    assert!(state.preferences.show_home_names);
    for target in [
        Target::AppNamesHome,
        Target::HomeOption(HomeSetting::FavoriteNames),
    ] {
        assert!(!state.target_enabled(&target));
        state.focus = target;
        assert!(state.activate().is_empty());
        assert!(state.preferences.show_home_names);
    }
    state.open(2);
    state.subpage = Some(Subpage::Home);
    state.focus = Target::HomeOption(HomeSetting::TwoRows);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(!state.dock_enabled());
    assert!(state.home_names_visible());
    state.focus = Target::HomeOption(HomeSetting::FavoritesDock);
    assert!(!state.target_enabled(&state.focus));
    assert!(state.activate().is_empty());
    assert!(state.preferences.two_row_home_favorites);
    state.focus = Target::HomeOption(HomeSetting::TwoRows);
    state.activate();
    state.focus = Target::HomeOption(HomeSetting::FavoriteNames);
    assert!(state.target_enabled(&state.focus));
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(!state.preferences.show_home_names);
}

#[test]
fn xbox_preset_keeps_the_discovered_input_identity_and_only_saves_local_preferences() {
    let mut state = ready();
    state.open(2);
    let id = "com.webos.app.hdmi3".to_owned();
    let original = state
        .inventory
        .inputs
        .iter()
        .find(|input| input.id == id)
        .unwrap()
        .clone();
    state.subpage = Some(crate::state::Subpage::Input(id.clone()));
    state.focus = Target::InputRename;
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::InputXboxPreset);
    assert_eq!(state.activate(), vec![Effect::Save]);
    let input = state
        .inventory
        .inputs
        .iter()
        .find(|input| input.id == id)
        .unwrap();
    assert_eq!(input.title, original.title);
    assert_eq!(input.port, original.port);
    assert_eq!(input.connected, original.connected);
    assert_eq!(state.preferences.input_title(input), "Xbox Series X");
    assert!(state.activate().is_empty());
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::InputNameReset);
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert_eq!(state.preferences.input_title(&original), original.title);
}
