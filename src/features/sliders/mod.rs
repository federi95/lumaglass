//! Shared discrete slider geometry for SDL rendering, pointer input and remote steps.
pub const INSET: f32 = 64.;
pub const HEIGHT: f32 = 156.;
pub const TRACK_Y: f32 = 82.;
pub const TRACK_HIT_TOP: f32 = TRACK_Y - 24.;
pub fn index_at(x: f32, left: f32, width: f32, count: usize) -> usize {
    if count < 2 || !x.is_finite() || width <= 0. {
        return 0;
    }
    (((x - left) / width).clamp(0., 1.) * (count - 1) as f32).round() as usize
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pointer_values_snap_clamp_and_preserve_both_endpoints() {
        assert_eq!(index_at(-10., 100., 600., 7), 0);
        assert_eq!(index_at(100., 100., 600., 7), 0);
        assert_eq!(index_at(300., 100., 600., 7), 2);
        assert_eq!(index_at(700., 100., 600., 7), 6);
        assert_eq!(index_at(800., 100., 600., 7), 6);
        assert_eq!(index_at(f32::NAN, 0., 100., 3), 0);
    }
}
