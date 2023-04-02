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

/// Returns the base 2 logarithm of the number, rounded down.
///
/// When `x` is zero the result is undefined
pub const fn ilog_2(x: usize) -> u32 {
    let mut x_ = x;
    let mut res = 0;
    while 1 < x_ {
        x_ /= 2;
        res += 1;
    }
    res
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

    #[test]
    fn ilog_2_of_1_is_zero() {
        assert_eq!(ilog_2(1), 0);
    }

    #[test]
    fn ilog_2_of_2_is_1() {
        assert_eq!(ilog_2(2), 1);
    }

    #[test]
    fn ilog_2_of_256_is_8() {
        assert_eq!(ilog_2(256), 8);
    }

    #[test]
    fn ilog_2_of_1023_is_9() {
        assert_eq!(ilog_2(1023), 9);
    }

    #[test]
    fn ilog_2_of_1024_is_10() {
        assert_eq!(ilog_2(1024), 10);
    }
}
