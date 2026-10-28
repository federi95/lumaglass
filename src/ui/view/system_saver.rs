//! Separate settings for LG's global saver; local idle settings stay independent.
use super::*;
pub fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let session = &state.system_saver;
    let mut y = 164.8 - state.scroll;
    let status = if session.loading {
        language.t("systemSaverLoading")
    } else {
        session
            .report
            .as_ref()
            .map_or(language.t("infoUnavailable"), |r| language.t(r.key()))
    };
    let context = session
        .report
        .as_ref()
        .map(|r| {
            [r.model.as_deref(), r.firmware.as_deref(), r.sdk.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_default();
    let description = format!(
        "{}\n{}\n{}",
        language.t("systemSaverDescription"),
        status,
        context
    );
    let height = 32. + assets.lines(&description, text(18., 400, 0.), 1056.) as f32 * 27.;
    out.text(
        &description,
        Rect::new(664., y + 16., 1056., height),
        text(18., 400, 0.).leading(1.5),
        c(0xe0e3e8, 1.),
        false,
    );
    y += height + 24.;
    for (index, key, description) in [
        (0, "systemSaverInspect", None),
        (1, "systemSaverStyle", Some("systemSaverPhotosHint")),
        (2, "systemSaverRequest", Some("systemSaverRequestHint")),
        (3, "systemSaverRestore", None),
        (5, "cancel", Some("systemSaverConfirm")),
        (4, "systemSaverApply", Some("systemSaverApplyHint")),
    ] {
        if index == 5 && !session.confirming {
            continue;
        }
        let desc = description.map(|k| language.t(k));
        let height = if let Some(desc) = desc {
            72. + assets.lines(desc, text(15.2, 400, 0.), 1020.) as f32 * 22.8
        } else {
            72.
        };
        row(
            out,
            state,
            assets,
            Row {
                target: Target::SystemSaver(index),
                rect: Rect::new(640., y, 1120., height),
                title: language.t(key),
                icon: None,
                value: if index == 1 {
                    Some(language.t(if state.preferences.screensaver.system_photos {
                        "saverPhotos"
                    } else {
                        "clock"
                    }))
                } else {
                    None
                },
                description: desc,
                selected: false,
                chevron: false,
            },
        );
        y += height + 12.;
    }
}
