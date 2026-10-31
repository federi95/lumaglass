use super::*;

/// Native discrete range: labelled value, leading minimum, trailing maximum and a clear thumb.
pub(super) fn draw(
    out: &mut Scene,
    state: &State,
    target: Target,
    rect: Rect,
    title: &str,
    labels: &[String],
    index: usize,
) {
    if labels.is_empty() {
        return;
    }
    let rect = if let Some(form) = &mut out.form {
        let rect = form.rect(crate::features::sliders::HEIGHT, false, false);
        form.painting = true;
        rect
    } else {
        rect
    };
    let index = index.min(labels.len() - 1);
    let available = labels.len() > 1;
    let focused = state.focus == target;
    if focused && let Some(form) = &mut out.form {
        form.detail(title, labels.get(index).map(String::as_str));
        if target == Target::Setting(19) {
            form.hint(state.preferences.language.t("saverLocalHint"));
        }
    }
    let quick = matches!(state.overlay, Some(Overlay::ControlCenter { .. }));
    if quick {
        out.chrome(
            rect,
            36.,
            false,
            false,
            state.pressed.as_ref() == Some(&target),
        );
    } else {
        out.material(rect, 36., false, false);
    }
    if focused {
        out.border(rect, 36., 1.5, c(0xffffff, 0.72));
    }
    out.pressed_feedback(state, &target, rect, 36.);
    let muted = c(0xf1f5fa, if available { 0.70 } else { 0.40 });
    let padding = 32.;
    let value_width = if quick { 138. } else { 180. };
    out.text(
        title,
        Rect::new(
            rect.x + padding,
            rect.y + 23.,
            rect.w - value_width - padding * 2. - 16.,
            32.,
        ),
        text(if quick { 18. } else { 24. }, 500, -0.02),
        WHITE,
        false,
    );
    out.text(
        &labels[index],
        Rect::new(
            rect.x + rect.w - padding - value_width,
            rect.y + 24.,
            value_width,
            30.,
        ),
        text(if quick { 17. } else { 22. }, 500, 0.).right(),
        WHITE,
        false,
    );
    let left = rect.x + crate::features::sliders::INSET;
    let width = (rect.w - 2. * crate::features::sliders::INSET).max(0.);
    let y = rect.y + crate::features::sliders::TRACK_Y;
    let track = 10.;
    let fraction = if available {
        index as f32 / (labels.len() - 1) as f32
    } else {
        0.
    };
    out.shape(
        Rect::new(left, y, width, track),
        track / 2.,
        c(0xe7eff8, 0.18),
    );
    if available {
        let fill = out.accent_color(c(0xf3f7fc, if focused { 1. } else { 0.82 }));
        out.shape(
            Rect::new(left, y, width * fraction, track),
            track / 2.,
            fill,
        );
        for step in 0..labels.len() {
            let x = left + step as f32 * width / (labels.len() - 1) as f32;
            if step != index && step > 0 && step + 1 < labels.len() {
                out.shape(
                    Rect::new(x - 1.5, y + 3.5, 3., 3.),
                    1.5,
                    if step < index {
                        c(0x243447, 0.32)
                    } else {
                        c(0xffffff, 0.52)
                    },
                );
            }
        }
        // Broad, rounded glass thumb; its centre uses exactly the pointer-snapping geometry.
        let x = left + width * fraction;
        let w = if focused { 46. } else { 40. };
        let h = if focused { 30. } else { 26. };
        let thumb = Rect::new(x - w / 2., y + track / 2. - h / 2., w, h);
        out.shadow(thumb, h / 2., 12., 3., 0.24);
        if out.liquid_glass {
            out.glass(thumb, h / 2., c(0xffffff, 0.97), c(0xdce9f5, 0.92));
        } else {
            out.shape(thumb, h / 2., WHITE);
        }
        out.border(thumb, h / 2., 1., c(0xffffff, 0.88));
        // Endpoints stay visible. Short ranges also name their middle choices.
        for (step, label) in labels.iter().enumerate() {
            if labels.len() > 4 && step != 0 && step + 1 != labels.len() {
                continue;
            }
            let x = left + step as f32 * width / (labels.len() - 1) as f32;
            let label_width = if step == 0 || step + 1 == labels.len() {
                116.
            } else {
                (width / (labels.len() - 1) as f32 - 8.).clamp(60., 136.)
            };
            out.text(
                label,
                Rect::new(x - label_width / 2., rect.y + 115., label_width, 24.),
                text(if quick { 12. } else { 14. }, 450, 0.),
                muted,
                true,
            );
        }
    }
    out.control(target, rect, state.overlay.is_none());
    if let Some(form) = &mut out.form {
        form.painting = false;
    }
}
