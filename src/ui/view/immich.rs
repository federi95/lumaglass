//! Bounded native album selector. No credential text or server URL is rendered.
use super::*;
pub fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let l = state.preferences.language;
    let session = &state.immich;
    let status = l.t(if session.status.is_empty() {
        "immichDisconnected"
    } else {
        session.status
    });
    let info = format!("{}\n{} {}", l.t("immichHint"), status, session.version);
    let mut y = 164.8 - state.scroll;
    let height = assets.lines(&info, text(18., 400, 0.), 1056.) as f32 * 27. + 32.;
    out.text(
        info,
        Rect::new(664., y + 16., 1056., height),
        text(18., 400, 0.).leading(1.5),
        c(0xe0e3e8, 1.),
        false,
    );
    y += height + 24.;
    for target in state.immich_rows() {
        let (title, value, selected, desc) = match &target {
            Target::Immich(index) => (
                l.t(match index {
                    0 => "immichLoad",
                    1 => "immichImport",
                    2 | 4 => "immichForget",
                    3 => "cancel",
                    5 => "catalogPrevious",
                    _ => "catalogNext",
                })
                .to_owned(),
                if *index == 1 {
                    Some(session.selected.len().to_string())
                } else {
                    None
                },
                false,
                (*index == 4).then(|| l.t("immichForgetHint")),
            ),
            Target::ImmichAlbum(id) => {
                let album = session
                    .albums
                    .iter()
                    .find(|a| &a.id == id)
                    .expect("album row");
                (
                    album.title.clone(),
                    None,
                    session.selected.contains(id),
                    None,
                )
            }
            _ => unreachable!(),
        };
        let base =
            (34. + assets.lines(&title, text(20.8, 550, -0.02), 1020.) as f32 * 29.12).max(72.);
        let height = desc.map_or(base, |d| {
            base + assets.lines(d, text(15.2, 400, 0.), 1020.) as f32 * 22.8
        });
        let rect = Rect::new(640., y, 1120., height);
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
                    title: &title,
                    icon: None,
                    value: value.as_deref(),
                    description: desc,
                    selected,
                    chevron: false,
                },
            );
        }
        y += height + 12.;
    }
}
