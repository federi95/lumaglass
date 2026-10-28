//! Bounded native catalog. Visibility actions change only Lumaglass preferences.
use super::*;
use crate::features::apps::catalog::PAGE_SIZE;

struct Item<'a> {
    target: Target,
    title: &'a str,
    value: Option<&'a str>,
    description: Option<&'a str>,
    chevron: bool,
}
fn item(out: &mut Scene, state: &State, assets: &Assets, y: &mut f32, item: Item<'_>) {
    let Item {
        target,
        title,
        value,
        description,
        chevron,
    } = item;
    let width = if value.is_some() { 885. } else { 1031.8 };
    let title_lines = if description.is_some() {
        assets.lines(title, text(20.8, 550, -0.02), width)
    } else {
        1
    };
    let height = if let Some(description) = description {
        34. + title_lines as f32 * 29.12
            + 4.8
            + assets.lines(description, text(15.2, 400, 0.), 1020.) as f32 * 22.8
    } else {
        72.
    };
    let rect = Rect::new(640., *y, 1120., height);
    if rect.y + rect.h < state.content_viewport().y
        || rect.y > state.content_viewport().y + state.content_viewport().h
    {
        out.control(target, rect, true);
    } else {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect,
                title,
                icon: None,
                value,
                description,
                selected: false,
                chevron,
            },
        );
    }
    *y += height + 12.;
}
fn kind(state: &State, system: Option<bool>) -> &'static str {
    state.preferences.language.t(match system {
        Some(true) => "catalogSystem",
        Some(false) => "catalogUser",
        None => "infoUnknown",
    })
}
pub(super) fn render(out: &mut Scene, state: &State, assets: &Assets) {
    if let Some(Subpage::ManagedApp(id)) = &state.subpage {
        detail(out, state, assets, id);
        return;
    }
    let language = state.preferences.language;
    let mut y = 164.8 - state.scroll;
    let hint = if state.catalog.failed {
        "infoReadFailed"
    } else if !state.webos {
        "catalogPreview"
    } else {
        "catalogReadOnly"
    };
    item(
        out,
        state,
        assets,
        &mut y,
        Item {
            target: Target::CatalogRefresh,
            title: language.t("infoRefresh"),
            value: state.catalog.loading.then(|| language.t("infoReading")),
            description: Some(language.t(hint)),
            chevron: false,
        },
    );
    let report = state.catalog.report.as_ref();
    let access = if !state.webos {
        language.t("infoNotApplicable")
    } else if state.catalog.failed || state.catalog.loading {
        language.t("infoUnknown")
    } else {
        language.t(report.map_or("infoUnknown", |report| report.access.key()))
    };
    let root = if !state.webos {
        language.t("infoNotApplicable")
    } else if state.catalog.failed || state.catalog.loading {
        language.t("infoUnknown")
    } else {
        language.t(report.map_or("infoUnknown", |report| report.root.key()))
    };
    let count = report.map_or(0, |report| report.apps.len()).to_string();
    let summary = language.format(
        "catalogSummary",
        &[("count", &count), ("access", access), ("root", root)],
    );
    out.text(
        summary,
        Rect::new(664., y + 6., 1072., 30.),
        text(15., 400, 0.).leading(1.5),
        c(0xffffff, 0.85),
        false,
    );
    y += 42.;
    item(
        out,
        state,
        assets,
        &mut y,
        Item {
            target: Target::CatalogFilter,
            title: language.t("catalogFilter"),
            value: Some(language.t(state.catalog.filter.key())),
            description: None,
            chevron: false,
        },
    );
    for (index, (target, key)) in [
        (Target::CatalogPrevious, "catalogPrevious"),
        (Target::CatalogNext, "catalogNext"),
        (Target::CatalogUndo, "catalogUndo"),
        (Target::CatalogRestoreAll, "catalogRestoreAll"),
    ]
    .into_iter()
    .enumerate()
    {
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(
                    640. + (index % 2) as f32 * 572.,
                    y + (index / 2) as f32 * 84.,
                    548.,
                    72.,
                ),
                title: language.t(key),
                icon: None,
                value: None,
                description: None,
                selected: false,
                chevron: false,
            },
        );
    }
    y += 168.;
    if state.catalog.can_undo() {
        let hint = language.t("catalogUndoHint");
        let height = assets.lines(hint, text(14., 400, 0.), 1072.) as f32 * 21.;
        out.text(
            hint,
            Rect::new(664., y + 4., 1072., height),
            text(14., 400, 0.).leading(1.5),
            c(0xffffff, 0.80),
            false,
        );
        y += height + 12.;
    }
    let pages = format!("{} / {}", state.catalog.page + 1, state.catalog_pages());
    out.text(
        pages,
        Rect::new(664., y + 6., 1050., 28.),
        text(17., 500, 0.),
        c(0xffffff, 0.86),
        false,
    );
    y += 34.;
    let apps = state.catalog_entries();
    if apps.is_empty() {
        out.text(
            language.t("catalogEmpty"),
            Rect::new(664., y + 10., 1000., 60.),
            text(18., 400, 0.),
            WHITE,
            false,
        );
    }
    for app in apps
        .into_iter()
        .skip(state.catalog.page * PAGE_SIZE)
        .take(PAGE_SIZE)
    {
        let description = format!(
            "{} · {}{}",
            app.id,
            kind(state, app.system),
            if app.protected {
                format!(" · {}", language.t("catalogProtected"))
            } else {
                String::new()
            }
        );
        let local_visible = state.inventory.apps.iter().any(|entry| entry.id == app.id)
            && !state.preferences.hidden.contains(&app.id);
        let value = if state.preferences.hidden.contains(&app.id) {
            "catalogHidden"
        } else if local_visible {
            "catalogVisible"
        } else {
            "infoNotApplicable"
        };
        item(
            out,
            state,
            assets,
            &mut y,
            Item {
                target: Target::CatalogApp(app.id.clone()),
                title: &app.title,
                value: Some(language.t(value)),
                description: Some(&description),
                chevron: true,
            },
        );
    }
}
fn detail(out: &mut Scene, state: &State, assets: &Assets, id: &str) {
    let language = state.preferences.language;
    let mut y = 164.8 - state.scroll;
    item(
        out,
        state,
        assets,
        &mut y,
        Item {
            target: Target::CatalogBack,
            title: language.t("catalogBack"),
            value: None,
            description: None,
            chevron: false,
        },
    );
    let Some(app) = state.catalog_entry(id) else {
        return;
    };
    out.text(
        &app.title,
        Rect::new(664., y + 14., 1060., 44.),
        assets.fit_line(&app.title, text(30., 550, -0.02), 1060.),
        WHITE,
        false,
    );
    let metadata = format!(
        "{} · {}{}",
        app.id,
        kind(state, app.system),
        if app.protected {
            format!(" · {}", language.t("catalogProtected"))
        } else {
            String::new()
        }
    );
    out.text(
        metadata,
        Rect::new(664., y + 70., 1060., 54.),
        text(16., 400, 0.).leading(1.5),
        c(0xffffff, 0.80),
        false,
    );
    y += 142.;
    let hidden = state.preferences.hidden.contains(&app.id);
    item(
        out,
        state,
        assets,
        &mut y,
        Item {
            target: Target::CatalogToggle,
            title: language.t(if hidden {
                "catalogShowLocal"
            } else {
                "catalogHideLocal"
            }),
            value: None,
            description: Some(language.t(if !hidden && !state.catalog_can_hide(id) {
                "catalogLocalBlocked"
            } else {
                "catalogLocalHint"
            })),
            chevron: false,
        },
    );
    let report = state.system_visibility_report();
    let observed = report.map(|report| {
        format!(
            "{} · {} · {}",
            report.model.as_deref().unwrap_or(language.t("infoUnknown")),
            report
                .firmware
                .as_deref()
                .unwrap_or(language.t("infoUnknown")),
            report.sdk.as_deref().unwrap_or(language.t("infoUnknown"))
        )
    });
    let hint = observed.as_ref().map_or_else(
        || language.t("systemVisibilityInspectHint").to_owned(),
        |observed| format!("{} {}", language.t("systemVisibilityInspectHint"), observed),
    );
    item(
        out,
        state,
        assets,
        &mut y,
        Item {
            target: Target::CatalogSystemInspect,
            title: language.t("systemVisibilityInspect"),
            value: Some(language.t(if state.catalog.system_visibility.loading {
                "systemVisibilityChecking"
            } else {
                report.map_or("infoUnknown", |report| report.key())
            })),
            description: Some(&hint),
            chevron: false,
        },
    );
    item(
        out,
        state,
        assets,
        &mut y,
        Item {
            target: Target::CatalogSystemRestore,
            title: language.t("systemVisibilityRestore"),
            value: None,
            description: Some(language.t("systemVisibilityRestoreHint")),
            chevron: false,
        },
    );
    let confirming = state.catalog.system_visibility.confirming;
    let qualified = report.is_some_and(crate::features::apps::system_visibility::Report::can_hide);
    if confirming {
        item(
            out,
            state,
            assets,
            &mut y,
            Item {
                target: Target::CatalogSystemCancel,
                title: language.t("uninstallCancel"),
                value: None,
                description: None,
                chevron: false,
            },
        );
    }
    item(
        out,
        state,
        assets,
        &mut y,
        Item {
            target: Target::CatalogLgHome,
            title: language.t(if confirming {
                "systemVisibilityConfirm"
            } else {
                "catalogHideLg"
            }),
            value: Some(language.t(if qualified {
                "infoAvailable"
            } else {
                "infoUnavailable"
            })),
            description: Some(language.t(if qualified {
                "systemVisibilityHideHint"
            } else {
                "catalogLgBlocked"
            })),
            chevron: false,
        },
    );
    item(
        out,
        state,
        assets,
        &mut y,
        Item {
            target: Target::CatalogRemove,
            title: language.t(if state.catalog_removal_status() {
                "uninstallStatus"
            } else {
                "uninstallApp"
            }),
            value: None,
            description: Some(language.t(if state.catalog_removal_status() {
                "catalogReadOnly"
            } else if state.target_enabled(&Target::CatalogRemove) {
                "catalogRemoveHint"
            } else {
                "catalogRemoveBlocked"
            })),
            chevron: true,
        },
    );
    let visibility = language.t(match app.visible {
        Some(true) => "catalogVisible",
        Some(false) => "catalogHidden",
        None => "infoUnknown",
    });
    let sdk = state
        .catalog
        .report
        .as_ref()
        .and_then(|report| report.sdk.as_deref())
        .unwrap_or(language.t("infoUnknown"));
    let detail = language.format(
        "catalogMetadata",
        &[("visibility", visibility), ("sdk", sdk)],
    );
    out.text(
        detail,
        Rect::new(664., y + 12., 1060., 80.),
        text(15.2, 400, 0.).leading(1.5),
        c(0xffffff, 0.82),
        false,
    );
}
