/// A phase accumulator is represented here
///
/// # Generic arguments:
///
/// * `TOTAL_NUM_BITS` - the total number of bits to use for the accumulator, in `[1..31]`
///
/// * `NUM_INDEX_BITS` - the number of bits to use as index bits, in `[1..TOTAL_NUM_BITS]`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseAccumulator<const TOTAL_NUM_BITS: u32, const NUM_INDEX_BITS: u32> {
    sample_rate_hz: f32,
    rollover_mask: u32,
    accumulator: u32,
    last_accumulator: u32,
    increment: u32,
    rolled_over: bool,
}

impl<const TOTAL_NUM_BITS: u32, const NUM_INDEX_BITS: u32>
    PhaseAccumulator<TOTAL_NUM_BITS, NUM_INDEX_BITS>
{
    /// `PhaseAccumulatorU32::new(sr)` is a new phase accumulator with sample rate `sr`
    pub fn new(sample_rate_hz: f32) -> Self {
        Self {
            sample_rate_hz,
            rollover_mask: (1 << TOTAL_NUM_BITS) - 1,
            accumulator: 0,
            last_accumulator: 0,
            increment: 0,
            rolled_over: false,
        }
    }

    /// `pa.tick()` advances the phase accumulator by 1 tick, expected to be called at the sample rate
    pub fn tick(&mut self) {
        self.accumulator += self.increment;
        self.accumulator &= self.rollover_mask;

        if self.accumulator < self.last_accumulator {
            self.rolled_over = true;
        }

        self.last_accumulator = self.accumulator
    }

    /// `pa.set_frequency(f)` sets the frequency of the phase accumulator to frequency `f`
    pub fn set_frequency(&mut self, freq_hz: f32) {
        self.increment = (((1 << TOTAL_NUM_BITS) as f32 * freq_hz) / self.sample_rate_hz) as u32;
    }

    /// `pa.set_period(p)` sets the frequency of the phase accumulator to the reciprocal of the time period `p`
    pub fn set_period(&mut self, period_sec: f32) {
        self.set_frequency(1.0_f32 / period_sec)
    }

    /// `lfo.set_phase()` sets the accumulator into a certain phase `[0.0, 1.0]`
    pub fn set_phase(&mut self, phase: f32) {
        self.reset();
        self.accumulator = (self.rollover_mask as f32 * phase) as u32;
    }

    // `pa.ramp()` is the current value of the phase accumulator as a number in `[0.0, 1.0]`
    pub fn ramp(&self) -> f32 {
        self.accumulator as f32 / ((1 << TOTAL_NUM_BITS) as f32)
    }

    /// `pa.index()` is the current value of the index bits of the phase accumulator
    pub fn index(&self) -> usize {
        (self.accumulator >> (TOTAL_NUM_BITS - NUM_INDEX_BITS)) as usize
    }

    /// `pa.fraction()` is the fractional part of the accumulator as a floating point number in `[0.0, 1.0]`
    pub fn fraction(&self) -> f32 {
        ((self.accumulator & self.rollover_mask) as f32) / (self.rollover_mask as f32)
    }

    /// `pa.rolled_over()` is true iff the phase accumulator has rolled over into a new cycle since checking
    ///
    /// Self clearing
    pub fn rolled_over(&mut self) -> bool {
        if self.rolled_over {
            self.rolled_over = false;
            true
        } else {
            false
        }
    }

    /// `pa.reset()` resets the phase accumulator to zero
    pub fn reset(&mut self) {
        self.accumulator = 0;
        self.last_accumulator = 0;
        self.rolled_over = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::*;

    #[test]
    fn tick_advances_output() {
        let sample_rate = 1_000.0_f32;
        let mut pa = PhaseAccumulator::<24, 8>::new(sample_rate);

        // one tick should increase the output
        pa.set_period(0.001_f32);

        assert_eq!(pa.ramp(), 0.0);
        pa.tick();
        assert!(0.0 < pa.ramp());
    }

    #[test]
    fn index_bits_update_after_ticks() {
        let sample_rate = 1_000.0_f32;
        let mut pa = PhaseAccumulator::<24, 8>::new(sample_rate);
        pa.set_period(1.0_f32);

        assert_eq!(pa.index(), 0);

        // tick half way through one cycle
        for _ in 0..500 {
            pa.tick();
        }
        assert_eq!(pa.index(), 127);
    }

    #[test]
    fn reset_zeros_the_accum() {
        let sample_rate = 1_000.0_f32;
        let mut pa = PhaseAccumulator::<24, 8>::new(sample_rate);
        pa.set_period(1.0_f32);

        for _ in 0..100 {
            pa.tick();
        }
        assert!(pa.ramp() != 0.0);
        pa.reset();
        assert!(pa.ramp() == 0.0);
    }

    #[test]
    fn accum_rolls_over() {
        let sample_rate = 1_000.0_f32;
        let mut pa = PhaseAccumulator::<24, 8>::new(sample_rate);
        pa.set_period(1.0_f32);

        // tick all the way through one cycle to the very end but don't roll over
        for _ in 0..1000 {
            pa.tick();
        }
        let epsilon = 0.001;
        assert!(is_almost(pa.ramp(), 1.0, epsilon));

        // one more tick rolls back to the beginning
        pa.tick();
        assert!(is_almost(pa.ramp(), 0.0, epsilon));
    }
}
