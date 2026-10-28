use super::*;
use crate::{
    features::apps::catalog,
    model::{Preferences, demo_inventory},
};
use serde_json::json;

fn ready() -> State {
    let mut state = State::new(Preferences::default());
    state.set_inventory(demo_inventory(), None);
    let [Effect::AppCatalog { generation }] = state.open_app_catalog()[..] else {
        panic!()
    };
    state.catalog_accept(generation, Ok(catalog::Report::preview(&state.inventory)));
    state.subpage = Some(Subpage::ManagedApp("netflix".into()));
    state.focus = Target::CatalogSystemInspect;
    state
}
fn report(id: &str, status: &str) -> Report {
    Report::parse(
        id,
        &json!({"schema":1,"returnValue":true,"id":id,"status":status,"canRestore":true}),
    )
    .unwrap()
}
#[test]
fn capability_replies_do_not_coerce_flags_or_trust_foreign_identities() {
    for status in [
        "ready",
        "unverified",
        "conflict",
        "protected",
        "unavailable",
    ] {
        assert!(!report("netflix", status).can_restore);
    }
    assert!(!report("com.webos.app.home", "owned").can_restore);
    let malformed =
        json!({"schema":1,"returnValue":true,"id":"netflix","status":"owned","canRestore":"true"});
    assert!(!Report::parse("netflix", &malformed).unwrap().can_restore);
    assert!(Report::parse("another.app", &malformed).is_err());
    assert!(
        Report::parse(
            "netflix",
            &json!({"schema":1,"returnValue":true,"id":"netflix","status":"future-unsafe"})
        )
        .is_err()
    );
}
#[test]
fn stale_observations_and_catalog_refresh_cannot_enable_recovery_or_global_hiding() {
    let mut state = ready();
    let [Effect::SystemVisibility { generation, .. }] = state.activate()[..] else {
        panic!()
    };
    state.system_visibility_accept(generation + 1, "netflix", Ok(report("netflix", "owned")));
    assert!(!state.target_enabled(&Target::CatalogSystemRestore));
    state.catalog_request();
    state.system_visibility_accept(generation, "netflix", Ok(report("netflix", "owned")));
    assert!(!state.target_enabled(&Target::CatalogSystemRestore));
    assert!(!state.target_enabled(&Target::CatalogLgHome));
}
#[test]
fn ownership_scoped_recovery_never_changes_launcher_visibility_or_sends_an_uninstall() {
    let mut state = ready();
    let before = state.preferences.hidden.clone();
    let [Effect::SystemVisibility { generation, .. }] = state.activate()[..] else {
        panic!()
    };
    state.system_visibility_accept(generation, "netflix", Ok(report("netflix", "owned")));
    state.focus = Target::CatalogSystemRestore;
    let effects = state.activate();
    assert!(
        matches!(&effects[..], [Effect::SystemVisibility { id, command: Command::Restore, .. }] if id == "netflix")
    );
    assert_eq!(state.preferences.hidden, before);
    assert!(!state.target_enabled(&Target::CatalogSystemRestore));
    assert!(!state.target_enabled(&Target::CatalogLgHome));
}

#[test]
fn a_qualified_fixture_requires_cancel_first_then_an_explicit_confirmed_hide() {
    let mut state = ready();
    let [Effect::SystemVisibility { generation, .. }] = state.activate()[..] else {
        panic!()
    };
    let plan = "b".repeat(64);
    let qualified = Report::parse("netflix", &json!({"schema":1,"returnValue":true,"id":"netflix","status":"ready","model":"FixtureTV","firmware":"1.2.3","sdk":"10.0.0","manifestHash":"a".repeat(64),"plan":plan})).unwrap();
    state.system_visibility_accept(generation, "netflix", Ok(qualified));
    state.focus = Target::CatalogLgHome;
    assert!(state.activate().is_empty());
    assert_eq!(state.focus, Target::CatalogSystemCancel);
    assert!(!state.target_enabled(&Target::CatalogRemove));
    assert!(state.activate().is_empty());
    assert!(!state.catalog.system_visibility.confirming);
    assert!(state.activate().is_empty());
    assert_eq!(state.focus, Target::CatalogSystemCancel);
    state.focus = Target::CatalogLgHome;
    let effects = state.activate();
    assert!(
        matches!(&effects[..], [Effect::SystemVisibility { id, command: Command::Hide, plan: Some(token), .. }] if id == "netflix" && token == &plan)
    );
    assert!(!state.catalog.system_visibility.confirming);
    assert!(!state.target_enabled(&Target::CatalogLgHome));
}
