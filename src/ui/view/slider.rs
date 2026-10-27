use super::*;

/// Draw one labelled, discrete slider; State uses the exact same inset/step count.
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
    let focused = state.focus == target;
    out.shadow(rect, 22., 24., 6., 0.10);
    out.shape(rect, 22., c(0x171c20, 0.78));
    out.border(
        rect,
        22.,
        if focused { 2. } else { 1. },
        if focused {
            out.accent_color(c(0xffffff, 0.85))
        } else {
            c(0xffffff, 0.12)
        },
    );
    out.text(
        title,
        Rect::new(rect.x + 24., rect.y + 15., rect.w - 260., 26.),
        text(if rect.w < 800. { 17. } else { 20.8 }, 550, -0.02),
        WHITE,
        false,
    );
    out.text(
        &labels[index.min(labels.len() - 1)],
        Rect::new(rect.x + rect.w - 226., rect.y + 16., 202., 24.),
        text(17., 500, 0.).right(),
        c(0xffffff, 0.84),
        false,
    );
    let left = rect.x + crate::features::sliders::INSET;
    let width = (rect.w - 2. * crate::features::sliders::INSET).max(0.);
    let y = rect.y + 64.;
    let fraction = if labels.len() > 1 {
        index.min(labels.len() - 1) as f32 / (labels.len() - 1) as f32
    } else {
        0.
    };
    out.shape(Rect::new(left, y, width, 6.), 3., c(0xffffff, 0.16));
    out.shape(
        Rect::new(left, y, width * fraction, 6.),
        3.,
        out.accent_color(c(0xffffff, 0.88)),
    );
    for (step, label) in labels.iter().enumerate() {
        let x = left
            + if labels.len() > 1 {
                step as f32 / (labels.len() - 1) as f32 * width
            } else {
                0.
            };
        out.shape(Rect::new(x - 1., y + 10., 2., 6.), 1., c(0xffffff, 0.45));
        out.text(
            label,
            Rect::new(x - 54., y + 20., 108., 22.),
            text(12.5, 450, 0.),
            c(0xffffff, 0.72),
            true,
        );
    }
    let x = left + width * fraction;
    let radius = if focused { 13. } else { 10. };
    out.shadow(
        Rect::new(x - radius, y + 3. - radius, radius * 2., radius * 2.),
        radius,
        16.,
        3.,
        0.35,
    );
    out.shape(
        Rect::new(x - radius, y + 3. - radius, radius * 2., radius * 2.),
        radius,
        WHITE,
    );
    if focused {
        out.border(
            Rect::new(x - 18., y - 15., 36., 36.),
            18.,
            2.,
            out.accent_color(c(0xffffff, 0.78)),
        );
    }
    out.control(target, rect, state.overlay.is_none());
}
