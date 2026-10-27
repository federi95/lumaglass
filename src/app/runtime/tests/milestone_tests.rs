use super::*;
use crate::{
    features::{
        controls::Command,
        privacy::{Observation, Report},
        wallpapers::Photo,
    },
    state::Subpage,
};

#[test]
fn privacy_confirmation_defaults_to_cancel_and_sends_one_exact_intent() {
    let mut state = ready();
    state.webos = true;
    state.open(2);
    state.section = 4;
    state.subpage = Some(Subpage::Privacy);
    let mut report = Report::default();
    report.values[0] = Observation::On;
    state.privacy.report = Some(report);
    state.focus = Target::PrivacyField(0);
    assert!(state.activate().is_empty());
    assert_eq!(state.focus, Target::CloseDialog);
    assert!(state.back().is_empty());
    assert_eq!(state.focus, Target::PrivacyField(0));
    state.activate();
    state.focus = Target::ControlApply;
    let effects = state.activate();
    assert!(matches!(
        effects.as_slice(),
        [Effect::Control {
            command: Command::PrivacySet {
                index: 0,
                expected: true,
                value: false
            },
            ..
        }]
    ));
    state.focus = Target::PrivacyField(0);
    assert!(state.activate().is_empty());
    assert!(state.overlay.is_none());
}

#[test]
fn video_trials_block_launches_recents_and_shortcuts_and_back_requests_safe_mode() {
    let mut state = ready();
    state.display_trial = true;
    state.preferences.shortcuts.enabled = true;
    state.focus = Target::App(state.inventory.apps[0].id.clone());
    assert!(state.activate().is_empty());
    assert!(state.open_recents().is_empty());
    assert!(
        state
            .shortcut_down(crate::features::shortcuts::Button::Red, false, true)
            .is_none()
    );
    state.back();
    assert_eq!(state.display_change, Some(false));
}

#[test]
fn personal_selection_respects_reduced_motion_and_rotation_deduplicates() {
    let mut state = ready();
    state.preferences.reduced_motion = true;
    let id = "personal-0123456789abcdef".to_owned();
    state.wallpapers.photos.push(Photo {
        id: id.clone(),
        title: "My photo".into(),
        preview: Some(std::sync::Arc::new(crate::app_artwork::Artwork {
            key: id.clone(),
            pixels: image::RgbaImage::new(1, 1),
            backdrop: [[0; 3]; 2],
        })),
    });
    state.open(2);
    state.section = 1;
    state.subpage = Some(Subpage::Wallpapers);
    state.focus = Target::Photo(0);
    assert_eq!(state.activate(), [Effect::Save]);
    assert_eq!(state.current_background(), id);
    assert!(state.previous_background.is_none());
    assert!(state.activate().is_empty());
    state.preferences.wallpapers.rotation.selected = vec![id.clone(), id.clone(), "missing".into()];
    assert_eq!(state.rotation_pool(), [id]);
    assert!(state.rotate_background().is_empty());
    let scene = scene(&state);
    assert!(scene.draw.iter().any(|draw|matches!(draw,crate::renderer::Draw::AppImage(image,..) if image.key=="personal-0123456789abcdef")));
}
