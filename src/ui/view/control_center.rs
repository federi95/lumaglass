//! Floating optical lenses and a timer detail view, navigable with the TV remote.
use super::*;
use crate::features::{appearance::Setting, control_center::Action};

pub(super) fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let panel = Rect::new(1188., 136., 648., 780.);
    out.modal = Some(panel);
    // Capture the actual parent once, before any quick-control icon or label.
    // Each tile is an independent lens: no opaque card covering the whole group.
    out.draw
        .push(Draw::Backdrop(crate::render::backdrop::Layer::Panel));
    out.glass_layer = Some(crate::render::backdrop::Layer::Panel);
    out.text(
        language.t("controlCenter"),
        Rect::new(1220., 150., 480., 32.),
        text(24., 550, -0.02),
        WHITE,
        false,
    );
    let close = Rect::new(1762., 144., 42., 42.);
    let color = surface(
        out,
        close,
        21.,
        state.focus == Target::CloseControlCenter,
        state.pressed.as_ref() == Some(&Target::CloseControlCenter),
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
        Rect::new(1220., 204., 220., 220.),
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
        Rect::new(1456., 204., 348., 102.),
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
        Rect::new(1456., 322., 348., 102.),
        "volume",
        &audio,
    );
    tile(
        out,
        state,
        assets,
        Action::Screensaver,
        Rect::new(1220., 440., 284., 96.),
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
        Rect::new(1520., 440., 284., 96.),
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
        let rect = Rect::new(1220. + index as f32 * 160., 552., 104., 104.);
        let target = Target::Quick(action);
        let color = surface(
            out,
            rect,
            52.,
            state.focus == target,
            state.pressed.as_ref() == Some(&target),
            true,
        );
        out.icon(
            assets,
            icon,
            Rect::new(rect.x + 32., rect.y + 22., 40., 40.),
            color,
        );
        let label = language.t(action.key());
        let size = (13. * 114. / assets.width(label, text(13., 500, 0.)).max(114.)).max(10.5);
        out.text(
            label,
            Rect::new(rect.x - 12., rect.y + 74., 128., 24.),
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
            Rect::new(1220., 678., 584., crate::features::sliders::HEIGHT),
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
        Rect::new(1220., 850., 584., 42.),
        text(13., 400, 0.),
        c(0xffffff, if out.liquid_glass { 0.95 } else { 0.64 }),
        false,
    );
}

/// Light content and a clear focus ring on glass; solid accessible fallback.
fn surface(
    out: &mut Scene,
    rect: Rect,
    radius: f32,
    focused: bool,
    pressed: bool,
    enabled: bool,
) -> Color {
    let mut color = out.chrome(rect, radius, focused, false, pressed);
    if !enabled {
        color[3] *= 0.60;
    }
    color
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
    let color = surface(
        out,
        rect,
        if rect.h > 150. { 44. } else { rect.h / 2. },
        state.focus == target,
        state.pressed.as_ref() == Some(&target),
        enabled,
    );
    let large = rect.h > 150.;
    let size = if large { 76. } else { 34. };
    out.icon(
        assets,
        icon,
        Rect::new(
            if large {
                rect.x + (rect.w - size) / 2.
            } else {
                rect.x + 22.
            },
            rect.y + if large { 26. } else { (rect.h - size) / 2. },
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
        Rect::new(x, rect.y + if large { 128. } else { 23. }, width, 30.),
        text(font, 550, 0.),
        color,
        large,
    );
    let mut secondary = color;
    secondary[3] *= if out.liquid_glass { 0.95 } else { 0.74 };
    out.text(
        subtitle,
        Rect::new(x, rect.y + if large { 170. } else { 57. }, width, 36.),
        text(13., 400, 0.),
        secondary,
        large,
    );
    out.control(target, rect, false);
}
