use super::*;
use crate::model::{Preferences, demo_inventory};
use serde_json::json;

fn ready() -> State {
    let mut state = State::new(Preferences::default());
    state.set_inventory(demo_inventory(), None);
    let effects = state.open_app_catalog();
    let [Effect::AppCatalog { generation }] = effects.as_slice() else {
        panic!("catalog read expected");
    };
    state.catalog_accept(*generation, Ok(Report::preview(&state.inventory)));
    state
}
fn detail(state: &mut State, id: &str) {
    state.subpage = Some(Subpage::ManagedApp(id.into()));
    state.focus = Target::CatalogToggle;
}

#[test]
fn raw_catalog_keeps_system_invisible_and_protected_entries_without_coercing_unknowns() {
    let mut raw = json!({"schema":1,"returnValue":true,"inventoryAccess":"available","rootAccess":"denied","apps":[
        {"id":"com.webos.app.home","title":"LG Home","visible":false,"systemApp":true},
        {"id":"com.webos.optional","title":"Optional","visible":false,"systemApp":true},
        {"id":"org.example.user","title":"User","visible":"false","systemApp":"false","removable":true}
    ]});
    let report = Report::from_reply(&raw).unwrap();
    assert_eq!(report.apps.len(), 3);
    assert!(
        report
            .apps
            .iter()
            .find(|app| app.id == "com.webos.app.home")
            .unwrap()
            .protected
    );
    let unknown = report
        .apps
        .iter()
        .find(|app| app.id == "org.example.user")
        .unwrap();
    assert_eq!(unknown.system, None);
    assert_eq!(unknown.visible, None);
    assert!(unknown.removal.is_none());
    raw["apps"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"org.example.user"}));
    assert!(Report::from_reply(&raw).is_err());
    raw["apps"] = json!([]);
    assert!(Report::from_reply(&raw).is_err());
    raw["inventoryAccess"] = json!("denied");
    let denied = Report::from_reply(&raw).unwrap();
    assert!(!denied.available());
    assert!(denied.apps.is_empty());
}

#[test]
fn stale_reads_and_failed_refresh_never_authorize_a_new_hide_or_a_firmware_mutation() {
    let mut state = ready();
    detail(&mut state, "netflix");
    assert!(state.catalog_can_hide("netflix"));
    let effects = state.catalog_request();
    let [Effect::AppCatalog { generation }] = effects.as_slice() else {
        panic!();
    };
    assert!(!state.catalog_can_hide("netflix"));
    state.catalog_accept(generation + 1, Ok(Report::preview(&state.inventory)));
    assert!(state.catalog.loading);
    assert!(state.activate().is_empty());
    assert!(!state.preferences.hidden.contains(&"netflix".into()));
    state.catalog_accept(*generation, Err("denied".into()));
    assert!(state.catalog.failed);
    assert!(!state.catalog_can_hide("netflix"));
    state.focus = Target::CatalogLgHome;
    assert!(!state.target_enabled(&state.focus));
    assert!(state.activate().is_empty());
}

#[test]
fn local_hide_and_undo_preserve_favorites_and_never_generate_platform_mutations() {
    let mut state = ready();
    let favorites = state.preferences.favorites(&state.inventory.apps);
    detail(&mut state, "netflix");
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.hidden.contains(&"netflix".into()));
    let persisted: Preferences =
        serde_json::from_slice(&serde_json::to_vec(&state.preferences).unwrap()).unwrap();
    assert!(persisted.hidden.contains(&"netflix".into()));
    state.catalog_back();
    state.focus = Target::CatalogUndo;
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(!state.preferences.hidden.contains(&"netflix".into()));
    assert_eq!(
        state.preferences.favorites(&state.inventory.apps),
        favorites
    );
    assert!(!state.catalog.can_undo());
    detail(&mut state, "org.webosbrew.hbchannel");
    assert!(!state.catalog_can_hide("org.webosbrew.hbchannel"));
    assert!(state.activate().is_empty());
}

#[test]
fn restore_is_available_without_a_catalog_and_undo_preserves_subsequent_choices() {
    let mut state = ready();
    state.preferences.hidden = vec![
        "netflix".into(),
        "youtube.leanback.v4".into(),
        "org.webosbrew.hbchannel".into(),
    ];
    state.catalog.report = None;
    state.focus = Target::CatalogRestoreAll;
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.hidden.is_empty());
    // A later change from another UI must not be clobbered by catalog undo.
    state.preferences.hidden.push("netflix".into());
    state.focus = Target::CatalogUndo;
    assert_eq!(state.activate(), vec![Effect::Save]);
    assert!(state.preferences.hidden.contains(&"netflix".into()));
    assert!(
        state
            .preferences
            .hidden
            .contains(&"youtube.leanback.v4".into())
    );
    assert!(
        !state
            .preferences
            .hidden
            .contains(&"org.webosbrew.hbchannel".into())
    );
}

#[test]
fn a_confirmed_removal_cannot_leave_a_stale_catalog_restore_target() {
    let mut state = ready();
    detail(&mut state, "org.xbmc.kodi");
    assert!(state.catalog_removal().is_some());
    state.focus = Target::CatalogRemove;
    assert!(state.activate().is_empty());
    assert!(matches!(
        state.overlay,
        Some(Overlay::RemovalConfirm { .. })
    ));
    assert_eq!(state.focus, Target::RemovalCancel);
    state.back();
    state.catalog_invalidate_removed("org.xbmc.kodi");
    state.focus = Target::CatalogRemove;
    state.reconcile_focus();
    assert_eq!(state.subpage, Some(Subpage::AppManagement));
    assert_eq!(state.focus, Target::CatalogRefresh);
    assert!(state.catalog_entry("org.xbmc.kodi").is_none());
}
