use super::*;

/// Shared padded row; track geometry is also used by pointer input.
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
    let available = labels.len() > 1;
    let focused = state.focus == target;
    let bright = focused && available;
    let color = if bright { c(0x20242a, 1.) } else { WHITE };
    let muted = if bright {
        c(0x20242a, 0.72)
    } else {
        c(0xffffff, 0.64)
    };
    out.material(rect, 40., bright, false);
    if focused && !bright {
        out.border(rect, 40., 2., out.accent_color(c(0xffffff, 0.6)));
    }
    // The title/value share the same generous inset as the surrounding settings rows.
    let padding = 32.;
    let compact = rect.w < 800.;
    let value_width = if compact { 180. } else { 224. };
    out.text(
        title,
        Rect::new(
            rect.x + padding,
            rect.y + 24.,
            rect.w - value_width - padding * 2. - 16.,
            30.,
        ),
        text(if compact { 17.5 } else { 20.8 }, 550, -0.02),
        color,
        false,
    );
    out.text(
        &labels[index.min(labels.len() - 1)],
        Rect::new(
            rect.x + rect.w - padding - value_width,
            rect.y + 25.,
            value_width,
            28.,
        ),
        text(if compact { 16. } else { 18.4 }, 450, 0.).right(),
        muted,
        false,
    );
    let left = rect.x + crate::features::sliders::INSET;
    let width = (rect.w - 2. * crate::features::sliders::INSET).max(0.);
    let y = rect.y + crate::features::sliders::TRACK_Y;
    let fraction = if available {
        index.min(labels.len() - 1) as f32 / (labels.len() - 1) as f32
    } else {
        0.
    };
    out.shape(
        Rect::new(left, y, width, 6.),
        3.,
        if bright {
            c(0x20242a, 0.22)
        } else {
            c(0xffffff, 0.16)
        },
    );
    if available {
        out.shape(
            Rect::new(left, y, width * fraction, 6.),
            3.,
            if bright {
                c(0x20242a, 0.84)
            } else {
                out.accent_color(c(0xffffff, 0.88))
            },
        );
        let gap = width / (labels.len() - 1) as f32;
        let label_width = (gap - 8.).clamp(60., 136.);
        for (step, label) in labels.iter().enumerate() {
            let x = left + step as f32 * gap;
            out.shape(Rect::new(x - 1., y + 15., 2., 5.), 1., muted);
            // Long intermediate labels yield to the selected value and endpoints.
            if compact && labels.len() > 4 && step != 0 && step != labels.len() - 1 && step != index
            {
                continue;
            }
            let width = if step == 0 || step + 1 == labels.len() {
                116.
            } else {
                label_width
            };
            out.text(
                label,
                Rect::new(x - width / 2., rect.y + 112., width, 24.),
                text(13., 450, 0.),
                muted,
                true,
            );
        }
        let x = left + width * fraction;
        let radius = if focused { 15. } else { 12. };
        let thumb = Rect::new(x - radius, y + 3. - radius, radius * 2., radius * 2.);
        if out.liquid_glass {
            out.shadow(thumb, radius, 10., 2., 0.20);
        }
        if out.liquid_glass {
            out.draw
                .push(Draw::Glass(thumb, radius, WHITE, c(0xe5ebf0, 1.)));
        } else {
            out.shape(thumb, radius, WHITE);
        }
        out.border(
            thumb,
            radius,
            1.,
            if bright {
                c(0x20242a, 0.25)
            } else {
                c(0xffffff, 0.70)
            },
        );
    }
    out.control(target, rect, state.overlay.is_none());
}
