//! Independent translucent lenses, compact quick controls and native timer detail.
use super::*;
use crate::features::{appearance::Setting, control_center::Action};
pub(super) fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let l = state.preferences.language;
    out.modal = Some(Rect::new(1320., 152., 528., 716.));
    out.draw
        .push(Draw::Backdrop(crate::render::backdrop::Layer::Panel));
    out.glass_layer = Some(crate::render::backdrop::Layer::Panel);
    let report = &state.controls.report;
    let unavailable = l.t(if state.webos {
        "infoUnavailable"
    } else {
        "infoNotApplicable"
    });
    let loading = l.t("loading");
    let network = if state.controls.loading {
        loading
    } else {
        l.t(match report.network.as_str() {
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
        loading.into()
    } else if let Some(muted) = report.muted {
        if muted {
            l.t("ccMuted").into()
        } else if let Some(volume) = report.volume {
            format!("{} · {volume}%", l.t("ccUnmuted"))
        } else {
            l.t("ccUnmuted").into()
        }
    } else {
        unavailable.to_owned()
    };
    tile(
        out,
        state,
        assets,
        Action::Standby,
        Rect::new(1352., 184., 204., 204.),
        "power",
        if state.controls.loading {
            loading
        } else if report.power_active {
            l.t("ccStandbyReady")
        } else {
            unavailable
        },
    );
    tile(
        out,
        state,
        assets,
        Action::Network,
        Rect::new(1572., 184., 260., 94.),
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
        Rect::new(1572., 294., 260., 94.),
        "volume",
        &audio,
    );
    tile(
        out,
        state,
        assets,
        Action::Screensaver,
        Rect::new(1352., 404., 232., 88.),
        "grid",
        l.t("ccStartNow"),
    );
    let sleep = if state.controls.loading {
        loading.to_owned()
    } else {
        report
            .sleep_index
            .map(|i| {
                if i == 0 {
                    l.t("stateOff").to_owned()
                } else {
                    format!(
                        "{} {}",
                        crate::features::control_center::SLEEP_MINUTES[i.min(5)],
                        l.t("minutesShort")
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
        Rect::new(1600., 404., 232., 88.),
        "clock",
        &sleep,
    );
    let mut focus_label = None;
    for (index, (action, icon)) in [
        (Action::Search, "search"),
        (Action::Inputs, "tv"),
        (Action::Recents, "recent"),
        (Action::Settings, "settings"),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = Rect::new(1352. + index as f32 * 128., 508., 96., 96.);
        let target = Target::Quick(action);
        let color = surface(out, state, &target, rect, 48., true);
        out.icon(assets, icon, rect.inset(29.), color);
        if state.focus == target {
            focus_label = Some(l.t(action.key()));
        }
        out.control(target, rect, false);
    }
    if let Some(label) = focus_label {
        out.text(
            label,
            Rect::new(1352., 621., 480., 28.),
            text(17., 500, 0.),
            WHITE,
            true,
        );
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
            Rect::new(1352., 664., 480., crate::features::sliders::HEIGHT),
            l.t(title),
            &labels,
            step,
        );
    }
    out.text(
        l.t(if state.cc_timer {
            "ccSliderHint"
        } else {
            "ccDimmingHint"
        }),
        Rect::new(1352., 838., 480., 38.),
        text(13., 400, 0.),
        c(0xffffff, 0.72),
        false,
    );
}
fn surface(
    out: &mut Scene,
    state: &State,
    target: &Target,
    rect: Rect,
    radius: f32,
    enabled: bool,
) -> Color {
    let focused = state.focus == *target;
    let visual = state.feedback_rect(target, rect, false);
    let mut color = out.chrome(
        visual,
        radius,
        focused && enabled,
        false,
        state.pressed.as_ref() == Some(target),
    );
    if !enabled {
        color[3] *= 0.52;
        if focused {
            out.border(visual, radius, 1.5, c(0xffffff, 0.55));
        }
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
        state,
        &target,
        rect,
        if rect.h > 150. { 48. } else { rect.h / 2. },
        enabled,
    );
    let large = rect.h > 150.;
    let size = if large { 78. } else { 34. };
    let mut icon_color = color;
    if state.focus != target && enabled && matches!(action, Action::Network) {
        icon_color = c(0x4bd7ff, 1.);
    }
    out.icon(
        assets,
        icon,
        Rect::new(
            if large {
                rect.x + (rect.w - size) / 2.
            } else {
                rect.x + 22.
            },
            rect.y + if large { 24. } else { (rect.h - size) / 2. },
            size,
            size,
        ),
        icon_color,
    );
    let x = rect.x + if large { 18. } else { 70. };
    let width = rect.w - if large { 36. } else { 88. };
    let label = state.preferences.language.t(action.key());
    out.text(
        label,
        Rect::new(x, rect.y + if large { 122. } else { 19. }, width, 32.),
        assets.fit_line(
            label,
            text(if large { 22. } else { 20. }, 500, -0.025),
            width,
        ),
        color,
        large,
    );
    let mut secondary = color;
    secondary[3] *= 0.74;
    out.text(
        subtitle,
        Rect::new(x, rect.y + if large { 162. } else { 52. }, width, 28.),
        assets.fit_line(subtitle, text(15., 400, 0.), width),
        secondary,
        large,
    );
    out.control(target, rect, false);
}
