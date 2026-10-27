//! Local launcher identity; no LG or third-party account is created or accessed.
use super::*;

pub(super) fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let language = state.preferences.language;
    let panel = Rect::new(640., 200., 640., 680.);
    out.modal = Some(panel);
    out.shadow(panel, 40., 40., 10., 0.30);
    out.material(panel, 40., false, true);
    let avatar = Rect::new(916., 240., 88., 88.);
    out.shape(avatar, 44., out.accent_color(c(0xc9b6ff, 1.)));
    let initials = crate::features::header::initials(&state.preferences.greeting_name);
    if state.preferences.header.avatar == crate::features::header::Avatar::Initials
        && !initials.is_empty()
    {
        out.text(
            initials,
            Rect::new(avatar.x, avatar.y + 27., 88., 42.),
            text(30., 600, 0.),
            c(0x20242a, 1.),
            true,
        );
    } else {
        out.icon(assets, "user", avatar.inset(22.), c(0x20242a, 1.));
    }
    let name = state.preferences.greeting_name.trim();
    let title = if name.is_empty() {
        language.t("profileLocal")
    } else {
        name
    };
    let size = (30. * 548. / assets.width(title, text(30., 550, 0.)).max(548.)).max(16.);
    out.text(
        title,
        Rect::new(686., 351., 548., 42.),
        text(size, 550, 0.),
        WHITE,
        true,
    );
    out.text(
        language.t("profileLocalHint"),
        Rect::new(686., 407., 548., 62.),
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
        let rect = Rect::new(676., 496. + index as f32 * 96., 568., 76.);
        let color = out.chrome(
            rect,
            38.,
            state.focus == target,
            false,
            state.pressed.as_ref() == Some(&target),
        );
        out.icon(
            assets,
            icon,
            Rect::new(rect.x + 24., rect.y + 25., 26., 26.),
            color,
        );
        out.text(
            language.t(key),
            Rect::new(
                rect.x + 70.,
                rect.y + 25.,
                if target == Target::ProfileAvatar {
                    260.
                } else {
                    450.
                },
                30.,
            ),
            text(20., 500, 0.),
            color,
            false,
        );
        if target == Target::ProfileAvatar {
            out.text(
                language.t(
                    if state.preferences.header.avatar == crate::features::header::Avatar::Initials
                    {
                        "avatarInitials"
                    } else {
                        "avatarSymbol"
                    },
                ),
                Rect::new(rect.x + 354., rect.y + 27., 188., 26.),
                text(16., 400, 0.).right(),
                color,
                false,
            );
        }
        out.control(target, rect, false);
    }
    let close = Rect::new(1200., 224., 48., 48.);
    let color = out.chrome(
        close,
        24.,
        state.focus == Target::CloseProfile,
        false,
        state.pressed.as_ref() == Some(&Target::CloseProfile),
    );
    out.icon(assets, "close", close.inset(15.), color);
    out.control(Target::CloseProfile, close, false);
}
