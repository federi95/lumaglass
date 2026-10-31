//! Local launcher identity; no LG or third-party account is created or accessed.
use super::*;

pub(super) fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    // The group owns a modal hit area, without drawing an enclosing container.
    // Each row is its own clear optical lens, like the Control Center capsules.
    out.modal = Some(Rect::new(608., 168., 704., 740.));
    out.draw
        .push(Draw::Backdrop(crate::render::backdrop::Layer::Panel));
    out.glass_layer = Some(crate::render::backdrop::Layer::Panel);
    avatar(out, state, assets, Rect::new(912., 200., 96., 96.));
    let name = state.preferences.greeting_name.trim();
    let title = if name.is_empty() {
        language.t("profileLocal")
    } else {
        name
    };
    let size = (30. * 548. / assets.width(title, text(30., 550, 0.)).max(548.)).max(16.);
    out.text(
        title,
        Rect::new(648., 328., 624., 48.),
        text(size, 550, 0.),
        WHITE,
        true,
    );
    out.text(
        language.t("profileLocalHint"),
        Rect::new(648., 392., 624., 76.),
        text(16., 400, 0.).leading(1.4),
        c(0xffffff, 0.82),
        true,
    );
    for (index, (target, key, icon)) in [
        (Target::ProfileName, "profileName", "type"),
        (Target::ProfileAvatar, "avatarStyle", "user"),
        (Target::ProfileClearName, "clearName", "refresh"),
    ]
    .into_iter()
    .enumerate()
    {
        let value = (target == Target::ProfileAvatar)
            .then(|| language.t(state.preferences.header.avatar.key()));
        row(
            out,
            state,
            assets,
            Row {
                target: target.clone(),
                rect: Rect::new(640., 492. + index as f32 * 100., 640., 80.),
                title: language.t(key),
                icon: Some(icon),
                value,
                description: None,
                selected: false,
                chevron: target == Target::ProfileName,
            },
        );
    }
}

pub(super) fn avatar(out: &mut Scene, state: &State, assets: &Assets, rect: Rect) {
    let style = state.preferences.header.avatar;
    out.shape(rect, rect.w / 2., out.accent_color(c(style.color(), 1.)));
    let initials = crate::features::header::initials(&state.preferences.greeting_name);
    if style == crate::features::header::Avatar::Initials && !initials.is_empty() {
        let size = rect.w * 0.38;
        out.text(
            initials,
            Rect::new(
                rect.x,
                rect.y + (rect.h - size * 1.4) / 2.,
                rect.w,
                size * 1.4,
            ),
            text(size, 600, 0.),
            c(0x20242a, 1.),
            true,
        );
    } else {
        out.icon(
            assets,
            style.icon(),
            rect.inset(rect.w * 0.23),
            c(0x20242a, 1.),
        );
    }
}
