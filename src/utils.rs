/// `linear_interp(y0, y1, frac)` is linear interpolation of `y0` and `y1` with fraction `frac`
///
/// # Arguments:
///
/// * `y0`, `y1` - The two y-values, a straight line can be drawn through these with an x-distance of 1.0
///
/// * `frac` - The fractional x-distance, in `[0.0, 1.0]`
/// ```
pub fn linear_interp(y0: f32, y1: f32, frac: f32) -> f32 {
    y0 + ((y1 - y0) * frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lin_interp_endpoints() {
        assert_eq!(linear_interp(0.0, 1.0, 0.0), 0.0);
        assert_eq!(linear_interp(0.0, 1.0, 1.0), 1.0);
    }

    #[test]
    fn lin_interp_halfway() {
        assert_eq!(linear_interp(0.0, 1.0, 0.5), 0.5);
    }

    #[test]
    fn lin_interp_both_non_zero() {
        assert_eq!(linear_interp(10.0, 40.0, 1.0 / 3.0), 20.0);
    }
}
