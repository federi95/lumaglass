//! Original native quick controls inspired by the layout of tvOS Control Center.
use super::*;
use crate::features::{appearance::Setting, control_center::Action};

pub(super) fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let panel = Rect::new(1192., 132., 644., 842.);
    out.modal = Some(panel);
    out.shadow(panel, 36., 32., 8., 0.28);
    out.shape(panel, 36., c(0x1c282b, 0.91));
    out.border(panel, 36., 1., c(0xffffff, 0.24));
    out.text(
        language.t("controlCenter"),
        Rect::new(1224., 156., 580., 30.),
        text(24., 550, -0.02),
        WHITE,
        false,
    );
    let report = &state.controls.report;
    let unavailable = language.t(if state.webos {
        "infoUnavailable"
    } else {
        "infoNotApplicable"
    });
    let network = language.t(match report.network.as_str() {
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
    });
    let audio = if state.controls.loading {
        language.t("loading").to_owned()
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
        Rect::new(1224., 208., 278., 206.),
        "power",
        if report.power_active {
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
        Rect::new(1514., 208., 290., 97.),
        "wifi",
        network,
    );
    tile(
        out,
        state,
        assets,
        Action::Audio,
        Rect::new(1514., 317., 290., 97.),
        "volume",
        &audio,
    );
    tile(
        out,
        state,
        assets,
        Action::Screensaver,
        Rect::new(1224., 426., 278., 98.),
        "grid",
        language.t("ccStartNow"),
    );
    let sleep = report
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
        .unwrap_or_else(|| unavailable.into());
    tile(
        out,
        state,
        assets,
        Action::Sleep,
        Rect::new(1514., 426., 290., 98.),
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
        let rect = Rect::new(1224. + index as f32 * 148., 538., 136., 78.);
        let target = Target::Quick(action);
        let focused = state.focus == target;
        out.pill(rect, focused, false);
        let color = if focused { c(0x20242a, 1.) } else { WHITE };
        out.icon(
            assets,
            icon,
            Rect::new(rect.x + 55., rect.y + 10., 26., 26.),
            color,
        );
        out.text(
            language.t(action.key()),
            Rect::new(rect.x + 4., rect.y + 45., rect.w - 8., 23.),
            text(13., 500, 0.),
            color,
            true,
        );
        out.control(target, rect, false);
    }
    for (target, title, y) in [
        (
            Target::AppearanceOption(Setting::Dimming),
            "appearanceDimming",
            632.,
        ),
        (Target::SleepSlider, "ccSleepTimer", 770.),
    ] {
        if let Some((step, labels)) = state.slider_values(&target) {
            slider::draw(
                out,
                state,
                target,
                Rect::new(1224., y, 580., 124.),
                language.t(title),
                &labels,
                step,
            );
        }
    }
    out.text(
        language.t("ccSliderHint"),
        Rect::new(1224., 912., 580., 40.),
        text(14., 400, 0.),
        c(0xffffff, 0.72),
        false,
    );
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
    let target = Target::Quick(action);
    let focused = state.focus == target;
    out.shape(
        rect,
        30.,
        if focused {
            out.accent_color(WHITE)
        } else {
            c(0xffffff, 0.08)
        },
    );
    out.border(rect, 30., if focused { 2. } else { 1. }, c(0xffffff, 0.22));
    let color = if focused { c(0x20242a, 1.) } else { WHITE };
    let large = rect.h > 150.;
    let size = if large { 60. } else { 30. };
    out.icon(
        assets,
        icon,
        Rect::new(
            if large {
                rect.x + (rect.w - size) / 2.
            } else {
                rect.x + 20.
            },
            rect.y + if large { 30. } else { 33. },
            size,
            size,
        ),
        color,
    );
    out.text(
        state.preferences.language.t(action.key()),
        Rect::new(
            if large { rect.x + 16. } else { rect.x + 64. },
            rect.y + if large { 110. } else { 16. },
            if large { rect.w - 32. } else { rect.w - 80. },
            52.,
        ),
        text(17., 550, 0.),
        color,
        large,
    );
    out.text(
        subtitle,
        Rect::new(
            rect.x + if large { 16. } else { 64. },
            rect.y + if large { 157. } else { 62. },
            rect.w - if large { 32. } else { 80. },
            34.,
        ),
        text(13., 400, 0.),
        color,
        large,
    );
    out.control(target, rect, false);
}
