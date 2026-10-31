//! CI-only regression checks for the native two-pane design and stable interaction.
use super::*;
#[test]
fn categories_open_explicitly_and_back_restores_the_root_menu() {
    let mut state = ready();
    state.open(2);
    let root = scene(&state);
    let categories: Vec<_> = root
        .controls
        .iter()
        .filter(|c| matches!(c.target, Target::Sidebar(_)))
        .collect();
    assert_eq!(categories.len(), 5);
    assert!(
        categories
            .iter()
            .all(|c| c.rect.x >= 1000. && c.rect.w == 800.)
    );
    assert!(!root.controls.iter().any(|c| c.target == Target::Setting(0)));
    state.focus(Target::Sidebar(0), &root.controls);
    state.activate();
    assert!(state.settings_category);
    let form = scene(&state);
    assert!(
        !form
            .controls
            .iter()
            .any(|c| matches!(c.target, Target::Sidebar(_)))
    );
    assert!(
        form.controls
            .iter()
            .any(|c| c.target == Target::Setting(0) && c.rect.x >= 1000.)
    );
    state.back();
    assert_eq!(state.focus, Target::Sidebar(0));
    assert!(!state.settings_category);
    assert!(
        scene(&state)
            .controls
            .iter()
            .any(|c| c.target == Target::Sidebar(0))
    );
}
#[test]
fn compact_lists_keep_saved_selection_distinct_from_focus() {
    let mut state = ready();
    state.open(2);
    state.subpage = Some(crate::state::Subpage::ClockFormat);
    state.focus = Target::Choice(0);
    state.preferences.hour_24 = true;
    let current = scene(&state);
    assert!(
        current
            .controls
            .iter()
            .filter(|c| matches!(c.target, Target::Choice(_)))
            .all(|c| c.rect.x >= 1000. && c.rect.h == 72.)
    );
    let checks=current.draw.iter().filter(|d|matches!(d,crate::renderer::Draw::TintedImage(path,..)if path.ends_with("check.lvec"))).count();
    assert_eq!(checks, 1);
    assert!(state.preferences.hour_24);
}
#[test]
fn settings_sliders_use_rendered_track_for_pointer_snapping() {
    let mut state = ready();
    state.open(2);
    state.subpage = Some(crate::state::Subpage::Appearance);
    let target = Target::AppearanceOption(crate::features::appearance::Setting::Dimming);
    state.focus = target.clone();
    let current = scene(&state);
    let control = current
        .controls
        .iter()
        .find(|c| c.target == target)
        .unwrap();
    assert_eq!(control.rect.x, 1032.);
    assert_eq!(control.rect.w, 800.);
    assert_eq!(control.rect.h, crate::features::sliders::HEIGHT);
    assert!(state.slider_values(&target).is_some());
}
#[test]
fn settings_header_back_is_reachable_and_returns_home_at_root() {
    let mut state = ready();
    state.open(2);
    state.focus = Target::Nav(0);
    state.direction(Direction::Left, &controls(&state));
    assert_eq!(state.focus, Target::SettingsBack);
    state.activate();
    assert_eq!(state.page, Page::Home);
}

#[test]
fn weather_remote_follows_the_vertical_query_search_result_order() {
    let mut state = ready();
    state.open(2);
    state.settings_category = true;
    state.subpage = Some(crate::state::Subpage::WeatherLocation);
    state.focus = Target::WeatherQuery;
    state.direction(Direction::Down, &controls(&state));
    assert_eq!(state.focus, Target::WeatherSearch);
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::WeatherQuery);
    state.focus = Target::WeatherResult(0);
    state.direction(Direction::Up, &controls(&state));
    assert_eq!(state.focus, Target::WeatherSearch);
}

#[test]
fn holding_home_favorite_opens_actions_before_explicit_move() {
    let mut state = ready();
    let id = state.inventory.apps[0].id.clone();
    state.focus = Target::App(id.clone());
    state.hold();
    assert!(matches!(state.overlay, Some(Overlay::Options { .. })));
    assert!(state.moving.is_none());
    let current = scene(&state);
    assert!(
        current
            .controls
            .iter()
            .any(|c| c.target == Target::Option(0))
    );
    assert!(
        current
            .controls
            .iter()
            .any(|c| c.target == Target::Option(3))
    );
    assert!(
        !current
            .controls
            .iter()
            .any(|c| c.target == Target::CloseDialog)
    );
    state.focus = Target::Option(3);
    state.activate();
    assert_eq!(state.moving, Some(id));
    assert!(state.overlay.is_none());
}

#[test]
fn information_values_are_visible_and_read_only_activation_never_exits() {
    let mut state = ready();
    state.open(2);
    state.section = crate::state::SYSTEM_SECTION;
    state.subpage = Some(crate::state::Subpage::SystemInfo);
    state.system.report = Some(crate::features::system::Report::from_reply(&serde_json::json!({
        "schema":1,"model":"65QNED756RA","firmware":"33.31.75","sdk":"10.3.2","systemAccess":"available"
    }), crate::features::system::Installation::Developer).unwrap());
    for index in 0..7 {
        state.focus = Target::SystemField(index);
        assert!(state.activate().is_empty());
        assert_eq!(state.subpage, Some(crate::state::Subpage::SystemInfo));
    }
    let current = scene(&state);
    for value in ["65QNED756RA", "33.31.75", "10.3.2"] {
        assert!(
            current
                .draw
                .iter()
                .any(|d| matches!(d, crate::renderer::Draw::Text(text, ..) if text == value))
        );
    }
    for index in 0..3 {
        assert!(
            current
                .controls
                .iter()
                .any(|c| c.target == Target::Nav(index))
        );
    }
}

#[test]
fn control_center_keeps_cached_wifi_and_blocker_observations_separate() {
    use crate::features::controls::{Command, Controls, Report};
    let mut controls = Controls::default();
    let (generation, _) = controls.request(Command::QuickStatus).unwrap();
    controls.accept(
        generation,
        Ok(Report {
            schema: 1,
            network: "wifi".into(),
            ssid: Some("Demo Wi-Fi".into()),
            muted: Some(false),
            volume: Some(25),
            power_active: true,
            ..Default::default()
        }),
    );
    let (generation, _) = controls.request(Command::BlockerStatus).unwrap();
    controls.accept(
        generation,
        Ok(Report {
            schema: 1,
            blocker: "off".into(),
            ..Default::default()
        }),
    );
    assert_eq!(controls.report.ssid.as_deref(), Some("Demo Wi-Fi"));
    assert_eq!(controls.report.muted, Some(false));
    let (generation, _) = controls.request(Command::QuickStatus).unwrap();
    controls.accept(generation, Err("offline".into()));
    assert_eq!(controls.report.ssid.as_deref(), Some("Demo Wi-Fi"));
    assert_eq!(controls.report.blocker, "off");
}
