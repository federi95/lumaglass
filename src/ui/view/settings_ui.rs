//! Two-pane settings: contextual artwork/help and a compact focus-driven list.
use super::*;
pub(super) const LEFT: f32 = 1032.;
pub(super) const WIDTH: f32 = 800.;
pub(super) const TOP: f32 = 200.;
pub(super) struct Detail {
    title: String,
    description: Option<String>,
    value: Option<String>,
    icon: Option<String>,
}
pub(super) struct Form {
    pub next: f32,
    pub painting: bool,
    pub notes: Vec<String>,
    focus: Option<Detail>,
    half: bool,
}
impl Form {
    pub fn detail(&mut self, title: &str, value: Option<&str>) {
        self.focus = Some(Detail {
            title: title.into(),
            description: None,
            value: value.map(str::to_owned),
            icon: Some("settings".into()),
        });
    }
    pub fn hint(&mut self, description: &str) {
        if let Some(detail) = &mut self.focus {
            detail.description = Some(description.to_owned());
        }
    }
    fn new(scroll: f32) -> Self {
        Self {
            next: TOP - scroll,
            painting: false,
            notes: vec![],
            focus: None,
            half: false,
        }
    }
    pub fn rect(&mut self, height: f32, half: bool, right: bool) -> Rect {
        if half {
            if right {
                let rect = Rect::new(LEFT + 410., self.next, 390., height);
                self.next += height + 12.;
                self.half = false;
                return rect;
            }
            if self.half {
                self.next += 84.;
            }
            self.half = true;
            return Rect::new(LEFT, self.next, 390., height);
        }
        if self.half {
            self.next += 84.;
            self.half = false;
        }
        let rect = Rect::new(LEFT, self.next, WIDTH, height);
        self.next += height + 12.;
        rect
    }
}
fn clipped_line(assets: &Assets, value: &str, style: TextStyle, width: f32) -> String {
    if assets.width(value, style) <= width {
        return value.into();
    }
    let mut text = value.to_owned();
    while !text.is_empty() {
        text.pop();
        if assets.width(&format!("{text}…"), style) <= width {
            break;
        }
    }
    format!("{text}…")
}
pub(super) fn section(out: &mut Scene, title: &str) {
    let Some(form) = out.form.as_mut() else {
        return;
    };
    let rect = form.rect(40., false, false);
    form.painting = true;
    out.text(
        title,
        Rect::new(rect.x + 28., rect.y + 10., rect.w - 56., 28.),
        text(18., 550, 0.),
        c(0xffffff, 0.60),
        false,
    );
    out.form.as_mut().unwrap().painting = false;
}
pub(super) fn item(out: &mut Scene, state: &State, assets: &Assets, row: Row<'_>) {
    let form = out.form.as_mut().expect("settings form");
    let half = row.rect.w < 700.;
    let rect = form.rect(72., half, row.rect.x > 1000.);
    if state.focus == row.target {
        form.focus = Some(Detail {
            title: row.title.into(),
            description: row.description.map(str::to_owned),
            value: row.value.map(str::to_owned),
            icon: row.icon.map(str::to_owned),
        });
    }
    if rect.y + rect.h < state.content_viewport().y
        || rect.y > state.content_viewport().y + state.content_viewport().h
    {
        out.control(row.target, rect, true);
        return;
    }
    form.painting = true;
    let enabled = state.target_enabled(&row.target);
    let focused = state.focus == row.target;
    let visual = state.feedback_rect(&row.target, rect, false);
    out.material(visual, visual.h / 2., focused && enabled, false);
    if focused && !enabled {
        out.border(visual, visual.h / 2., 1.5, c(0xffffff, 0.64));
    }
    out.pressed_feedback(state, &row.target, visual, visual.h / 2.);
    let color = if focused && enabled {
        c(0x18212b, 1.)
    } else {
        c(0xffffff, if enabled { 0.94 } else { 0.48 })
    };
    let trailing = if row.chevron || row.selected { 30. } else { 0. };
    let value_width = if row.value.is_some() {
        if half {
            112.
        } else if matches!(row.target, Target::SystemField(_)) {
            350.
        } else {
            230.
        }
    } else {
        0.
    };
    let width = rect.w - 56. - trailing - value_width - if row.value.is_some() { 16. } else { 0. };
    let style = text(if half { 24. } else { 28. }, 500, -0.02);
    let label = clipped_line(assets, row.title, style, width);
    out.text(
        label,
        Rect::new(rect.x + 28., rect.y + 18., width, 38.),
        style,
        color,
        false,
    );
    if let Some(value) = row.value {
        let style = text(if half { 21. } else { 25. }, 450, -0.02).right();
        let value = clipped_line(assets, value, style, value_width);
        let mut secondary = color;
        secondary[3] *= 0.72;
        out.text(
            value,
            Rect::new(
                rect.x + rect.w - 28. - trailing - value_width,
                rect.y + 20.,
                value_width,
                34.,
            ),
            style,
            secondary,
            false,
        );
    }
    if row.chevron || row.selected {
        out.icon(
            assets,
            if row.selected { "check" } else { "right" },
            Rect::new(rect.x + rect.w - 47., rect.y + 25., 21., 21.),
            [
                color[0],
                color[1],
                color[2],
                color[3] * if row.selected { 1. } else { 0.5 },
            ],
        );
    }
    out.control(row.target, rect, true);
    out.form.as_mut().unwrap().painting = false;
}
fn opened(state: &State) -> bool {
    state.settings_category
        || state.subpage.is_some()
        || !matches!(
            state.focus,
            Target::Nav(_)
                | Target::Sidebar(_)
                | Target::Profile
                | Target::ControlCenter
                | Target::TvSettings
                | Target::SettingsBack
        )
}
fn category_key(section: usize) -> &'static str {
    ["interface", "background", "weather", "inputs", "system"][section.min(4)]
}
fn category_icon(section: usize) -> &'static str {
    [
        "settings",
        "sparkles",
        "weather-partly-cloudy",
        "tv",
        "control-center",
    ][section.min(4)]
}
fn title(state: &State) -> String {
    let l = state.preferences.language;
    let key = match &state.subpage {
        Some(Subpage::Cloud(provider)) => provider.key(),
        Some(Subpage::Immich) => "immichTitle",
        Some(Subpage::SystemScreensaver) => "systemSaver",
        Some(Subpage::AppManagement | Subpage::ManagedApp(_)) => "catalogTitle",
        Some(Subpage::Display) => "displayResolution",
        Some(Subpage::Rotation) => "wallpaperRotation",
        Some(Subpage::Wallpapers) => "myImages",
        Some(Subpage::Shortcuts) => "remoteShortcuts",
        Some(Subpage::Appearance) => "appearance",
        Some(Subpage::Home) => "homePersonalization",
        Some(Subpage::StartupFocus) => "homeStartupFocus",
        Some(Subpage::Privacy) => "privacy",
        Some(Subpage::SystemInfo) => "tvInformation",
        Some(Subpage::AppNames) => "appNames",
        Some(Subpage::Greeting) => "greeting",
        Some(Subpage::Clock) => "clock",
        Some(Subpage::ClockFormat) => "clockFormat",
        Some(Subpage::TimeFormat) => "timeFormat",
        Some(Subpage::WeatherLocation) => "location",
        Some(Subpage::Temperature) => "temperature",
        Some(Subpage::Language) => "language",
        Some(Subpage::TextRendering) => "textRendering",
        Some(Subpage::Input(id)) => {
            return state
                .inventory
                .inputs
                .iter()
                .find(|i| &i.id == id)
                .map(|i| state.preferences.input_title(i).to_owned())
                .unwrap_or_else(|| l.t("inputs").into());
        }
        None if opened(state) => category_key(state.section),
        None => "settings",
    };
    l.t(key).to_owned()
}
pub(super) fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let heading = title(state);
    out.text(
        &heading,
        Rect::new(170., 140., 668., 48.),
        assets.fit_line(&heading, text(30., 550, -0.025), 668.),
        WHITE,
        false,
    );
    let x = if state.preferences.header.side == crate::features::header::Side::Left {
        430.
    } else {
        84.
    };
    let target = Target::SettingsBack;
    let back = Rect::new(x, 48., 56., 56.);
    let color = out.chrome(
        state.feedback_rect(&target, back, false),
        28.,
        state.focus == target,
        false,
        state.pressed.as_ref() == Some(&target),
    );
    out.icon(assets, "left", back.inset(17.), color);
    out.control(target, back, false);
    out.form = Some(Form::new(if opened(state) { state.scroll } else { 0. }));
    if !opened(state) {
        out.form.as_mut().unwrap().next = 276.;
        for (index, key) in ["interface", "background", "weather", "inputs", "system"]
            .into_iter()
            .enumerate()
        {
            item(
                out,
                state,
                assets,
                Row {
                    target: Target::Sidebar(index),
                    rect: Rect::new(640., 0., 1120., 72.),
                    title: state.preferences.language.t(key),
                    icon: Some(category_icon(index)),
                    value: None,
                    description: Some(state.preferences.language.t([
                        "designGeneralHint",
                        "designBackgroundHint",
                        "designWeatherHint",
                        "designInputsHint",
                        "designSystemHint",
                    ][index])),
                    selected: false,
                    chevron: true,
                },
            );
        }
    } else {
        super::settings_content(out, state, assets);
    }
    let form = out.form.take().unwrap();
    hero(out, state, assets, &form);
}
fn hero(out: &mut Scene, state: &State, assets: &Assets, form: &Form) {
    let l = state.preferences.language;
    let detail = form.focus.as_ref();
    let target = match state.focus {
        Target::Background(i) => BACKGROUNDS.get(i).map(|(id, _)| *id),
        _ => None,
    };
    let personal = if let Target::Photo(index) = state.focus {
        state
            .wallpapers
            .photos
            .get(index)
            .and_then(|p| p.preview.as_ref())
    } else {
        None
    };
    let gallery = state.section == 1 && state.subpage.is_none();
    let card = Rect::new(208., 324., 624., 351.);
    if let Some(image) = personal {
        out.shadow(card, 52., 36., 12., 0.3);
        out.draw.push(Draw::AppImage(image.clone(), card, WHITE));
    } else if gallery {
        let id = target.unwrap_or(&state.preferences.background);
        out.shadow(card, 52., 36., 12., 0.30);
        out.image(
            assets.path(format!("thumbnails/{id}.jpg")),
            card,
            52.,
            false,
        );
    } else {
        let card = Rect::new(288., 284., 432., 432.);
        out.shadow(card, 96., 40., 14., 0.22);
        out.draw.push(Draw::RoundedGradient(
            card,
            96.,
            c(0xb5c7d5, 0.28),
            c(0x657a90, 0.18),
        ));
        out.border(card, 96., 1., c(0xffffff, 0.12));
        out.icon(
            assets,
            detail
                .and_then(|d| d.icon.as_deref())
                .unwrap_or(category_icon(state.section)),
            card.inset(116.),
            c(0xf4f8fc, 0.88),
        );
    }
    let selected_title = if let Some(id) = target {
        l.wallpaper(id).to_owned()
    } else {
        detail.map_or_else(|| title(state), |d| d.title.clone())
    };
    out.text(
        &selected_title,
        Rect::new(160., 747., 688., 52.),
        assets.fit_line(&selected_title, text(30., 550, -0.025), 688.),
        WHITE,
        true,
    );
    let hint = [
        "designGeneralHint",
        "designBackgroundHint",
        "designWeatherHint",
        "designInputsHint",
        "designSystemHint",
    ][state.section.min(4)];
    let mut help = String::new();
    if let Some(value) = detail.and_then(|d| d.value.as_deref()) {
        help.push_str(value);
        help.push('\n');
    }
    if let Some(description) = detail.and_then(|d| d.description.as_deref()) {
        help.push_str(description);
    } else if !form.notes.is_empty() {
        help.push_str(
            &form
                .notes
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"),
        );
    } else {
        help.push_str(l.t(hint));
    }
    let clip = Rect::new(170., 816., 668., 154.);
    out.draw.push(Draw::Clip(Some(clip)));
    out.text(
        help,
        clip,
        text(20., 400, 0.).leading(1.4),
        c(0xf1f3f5, 0.75),
        true,
    );
    out.draw.push(Draw::Clip(Some(state.content_viewport())));
}
pub(super) fn gallery(out: &mut Scene, state: &State, assets: &Assets) {
    let form = out.form.as_mut().unwrap();
    form.painting = true;
    let top = TOP - state.scroll;
    let width = (WIDTH - 40.) / 3.;
    for (index, (id, _)) in BACKGROUNDS.iter().enumerate() {
        let rect = Rect::new(
            LEFT + (index % 3) as f32 * (width + 20.),
            top + (index / 3) as f32 * 194.,
            width,
            width * 0.5625,
        );
        let target = Target::Background(index);
        let visual = state.feedback_rect(&target, rect, true);
        out.shadow(visual, 24., 20., 5., 0.18);
        out.image(
            assets.path(format!("thumbnails/{id}.jpg")),
            visual,
            24.,
            false,
        );
        if state.focus == target {
            out.ring(visual, 24.);
        }
        if state.current_background() == *id {
            let check = Rect::new(rect.x + rect.w - 34., rect.y + rect.h - 34., 26., 26.);
            out.shape(check, 13., c(0xffffff, 0.95));
            out.icon(assets, "check", check.inset(6.), c(0x20242a, 1.));
        }
        let label = state.preferences.language.wallpaper(id);
        out.text(
            label,
            Rect::new(rect.x, rect.y + rect.h + 10., rect.w, 28.),
            assets.fit_line(label, text(20., 500, 0.), rect.w),
            WHITE,
            true,
        );
        out.control(
            target,
            Rect::new(rect.x, rect.y, rect.w, rect.h + 42.),
            true,
        );
    }
    let form = out.form.as_mut().unwrap();
    form.painting = false;
    form.next = top + 582.;
    for (index, key) in [(17, "myImages"), (18, "wallpaperRotation")] {
        item(
            out,
            state,
            assets,
            Row {
                target: Target::Setting(index),
                rect: Rect::new(640., 0., 1120., 72.),
                title: state.preferences.language.t(key),
                icon: Some("sparkles"),
                value: None,
                description: None,
                selected: false,
                chevron: true,
            },
        );
    }
}
