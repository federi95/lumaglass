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
