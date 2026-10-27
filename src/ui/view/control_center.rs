//! Compact quick controls with a single-pass glass material and a timer detail view.
use super::*;
use crate::features::{appearance::Setting, control_center::Action};

pub(super) fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let panel = Rect::new(1188., 140., 648., 744.);
    out.modal = Some(panel);
    if out.liquid_glass {
        out.shadow(panel, 40., 24., 8., 0.24);
    }
    out.material(panel, 40., false, true);
    out.draw
        .push(Draw::Backdrop(crate::render::backdrop::Layer::Panel));
    out.glass_layer = Some(crate::render::backdrop::Layer::Panel);
    out.text(
        language.t("controlCenter"),
        Rect::new(1220., 167., 480., 32.),
        text(24., 550, -0.02),
        WHITE,
        false,
    );
    let close = Rect::new(1762., 159., 42., 42.);
    let color = surface(
        out,
        close,
        21.,
        state.focus == Target::CloseControlCenter,
        true,
    );
    out.icon(assets, "close", close.inset(12.), color);
    out.control(Target::CloseControlCenter, close, false);
    let report = &state.controls.report;
    let unavailable = language.t(if state.webos {
        "infoUnavailable"
    } else {
        "infoNotApplicable"
    });
    let loading = language.t("loading");
    let network = if state.controls.loading {
        loading
    } else {
        language.t(match report.network.as_str() {
            "wifi" => "ccWifi",
            "wired" => "ccEthernet",
            "offline" => "ccOffline",
            _ => {
                if state.webos {
                    "infoUnavailable"
                } else {
                    "infoNotApplicable"
                }
            }
        })
    };
    let audio = if state.controls.loading {
        loading.to_owned()
    } else if let Some(muted) = report.muted {
        if muted {
            language.t("ccMuted").to_owned()
        } else if let Some(volume) = report.volume {
            format!("{} · {volume}%", language.t("ccUnmuted"))
        } else {
            language.t("ccUnmuted").to_owned()
        }
    } else {
        unavailable.to_owned()
    };
    tile(
        out,
        state,
        assets,
        Action::Standby,
        Rect::new(1220., 220., 276., 202.),
        "power",
        if state.controls.loading {
            loading
        } else if report.power_active {
            language.t("ccStandbyReady")
        } else {
            unavailable
        },
    );
    tile(
        out,
        state,
        assets,
        Action::Network,
        Rect::new(1512., 220., 292., 93.),
        if report.network == "wired" {
            "tv"
        } else {
            "wifi"
        },
        network,
    );
    tile(
        out,
        state,
        assets,
        Action::Audio,
        Rect::new(1512., 329., 292., 93.),
        "volume",
        &audio,
    );
    tile(
        out,
        state,
        assets,
        Action::Screensaver,
        Rect::new(1220., 438., 284., 92.),
        "grid",
        language.t("ccStartNow"),
    );
    let sleep = if state.controls.loading {
        loading.to_owned()
    } else {
        report
            .sleep_index
            .map(|index| {
                if index == 0 {
                    language.t("stateOff").to_owned()
                } else {
                    format!(
                        "{} {}",
                        crate::features::control_center::SLEEP_MINUTES[index.min(5)],
                        language.t("minutesShort")
                    )
                }
            })
            .unwrap_or_else(|| unavailable.into())
    };
    tile(
        out,
        state,
        assets,
        Action::Sleep,
        Rect::new(1520., 438., 284., 92.),
        "clock",
        &sleep,
    );
    for (index, (action, icon)) in [
        (Action::Search, "search"),
        (Action::Inputs, "tv"),
        (Action::Recents, "recent"),
        (Action::Settings, "settings"),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = Rect::new(1220. + index as f32 * 150., 546., 134., 82.);
        let target = Target::Quick(action);
        let color = surface(out, rect, 32., state.focus == target, true);
        out.icon(
            assets,
            icon,
            Rect::new(rect.x + 54., rect.y + 12., 26., 26.),
            color,
        );
        let label = language.t(action.key());
        let size = (13. * 114. / assets.width(label, text(13., 500, 0.)).max(114.)).max(10.5);
        out.text(
            label,
            Rect::new(rect.x + 10., rect.y + 50., 114., 24.),
            text(size, 500, 0.),
            color,
            true,
        );
        out.control(target, rect, false);
    }
    let (target, title) = if state.cc_timer {
        (Target::SleepSlider, "ccSleepTimer")
    } else {
        (
            Target::AppearanceOption(Setting::Dimming),
            "appearanceDimming",
        )
    };
    if let Some((step, labels)) = state.slider_values(&target) {
        slider::draw(
            out,
            state,
            target,
            Rect::new(1220., 646., 584., crate::features::sliders::HEIGHT),
            language.t(title),
            &labels,
            step,
        );
    }
    out.text(
        language.t(if state.cc_timer {
            "ccSliderHint"
        } else {
            "ccDimmingHint"
        }),
        Rect::new(1220., 824., 584., 38.),
        text(13., 400, 0.),
        c(0xffffff, if out.liquid_glass { 0.95 } else { 0.64 }),
        false,
    );
}

/// Text stays light on translucent glass; the opaque focused fallback uses dark text.
fn surface(out: &mut Scene, rect: Rect, radius: f32, focused: bool, enabled: bool) -> Color {
    if out.liquid_glass && focused {
        out.glass(
            rect,
            radius,
            out.accent_color(c(0x71879a, 0.32)),
            c(0x2c4050, 0.48),
        );
        out.border(rect, radius, 2., out.accent_color(c(0xffffff, 0.88)));
    } else {
        out.material(rect, radius, focused && enabled, false);
    }
    if !out.liquid_glass && focused && !enabled {
        out.border(rect, radius, 2., out.accent_color(WHITE));
    }
    if !out.liquid_glass && focused && enabled {
        c(0x20242a, 1.)
    } else {
        c(0xffffff, if enabled { 1. } else { 0.60 })
    }
}
fn tile(
    out: &mut Scene,
    state: &State,
    assets: &Assets,
    action: Action,
    rect: Rect,
    icon: &str,
    subtitle: &str,
) {
    let enabled = match action {
        Action::Standby => {
            state.webos && state.controls.report.power_active && !state.controls.loading
        }
        Action::Audio => {
            state.webos && state.controls.report.muted.is_some() && !state.controls.loading
        }
        _ => true,
    };
    let target = Target::Quick(action);
    let color = surface(out, rect, 32., state.focus == target, enabled);
    let large = rect.h > 150.;
    let size = if large { 58. } else { 28. };
    out.icon(
        assets,
        icon,
        Rect::new(
            if large {
                rect.x + (rect.w - size) / 2.
            } else {
                rect.x + 22.
            },
            rect.y + if large { 30. } else { (rect.h - size) / 2. },
            size,
            size,
        ),
        color,
    );
    let x = rect.x + if large { 20. } else { 66. };
    let width = rect.w - if large { 40. } else { 84. };
    let label = state.preferences.language.t(action.key());
    let font = (17. * width / assets.width(label, text(17., 550, 0.)).max(width)).max(14.);
    out.text(
        label,
        Rect::new(x, rect.y + if large { 112. } else { 18. }, width, 30.),
        text(font, 550, 0.),
        color,
        large,
    );
    let mut secondary = color;
    secondary[3] *= if out.liquid_glass { 0.95 } else { 0.74 };
    out.text(
        subtitle,
        Rect::new(x, rect.y + if large { 155. } else { 57. }, width, 36.),
        text(13., 400, 0.),
        secondary,
        large,
    );
    out.control(target, rect, false);
}
