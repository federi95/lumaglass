//! Native connector controls. No credentials, server address or collection ID is rendered.
use super::*;
use crate::features::wallpapers::cloud::Provider;
pub(super) fn render(out: &mut Scene, state: &State, assets: &Assets) {
    let l = state.preferences.language;
    let s = &state.cloud;
    let info = format!(
        "{}\n{}",
        l.t(if s.provider == Provider::GooglePhotos {
            "cloudPhotosExportHint"
        } else {
            "cloudHint"
        }),
        l.t(s.status)
    );
    out.text(
        info,
        Rect::new(664., 164.8, 1056., 160.),
        text(18., 400, 0.),
        WHITE,
        false,
    );
    for target in state.cloud_rows() {
        let (title, value, selected, description) = match target {
            Target::Cloud(i) => (
                l.t(match i {
                    0 if s.provider == Provider::GooglePhotos => "cloudPhotosLoad",
                    0 => "cloudLoad",
                    1 => "cloudImport",
                    2 | 4 => "cloudForget",
                    3 => "cancel",
                    5 => "catalogPrevious",
                    _ => "catalogNext",
                })
                .to_owned(),
                (i == 1 && s.provider != Provider::GooglePhotos)
                    .then(|| s.selected.len().to_string()),
                false,
                (i == 4).then(|| l.t("cloudForgetHint")),
            ),
            Target::CloudCollection(i) => {
                let c = &s.collections[i];
                (c.title.clone(), None, s.selected.contains(&c.id), None)
            }
            _ => unreachable!(),
        };
        row(
            out,
            state,
            assets,
            Row {
                target,
                rect: Rect::new(640., 0., 1120., 72.),
                title: &title,
                icon: Some("grid"),
                value: value.as_deref(),
                description,
                selected,
                chevron: false,
            },
        );
    }
}
