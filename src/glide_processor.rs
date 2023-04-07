//! # Glide Processor
//!
//! The glide processor is used to add variable lag to control signals. Music synthesizers often have a glide control
//! so that new notes slide into each other instead of stepping directly to the new note.
//!
//! The terms glide, lag, and portamento are often used interchangeably.

use crate::utils::*;
use biquad::*;

/// A glide processor for implementing portamento is represented here.
pub struct GlideProcessor {
    // min and max cutoff frequencies
    min_fc: f32,
    max_fc: f32,

    // sampe rate in hertz
    fs: Hertz<f32>,

    // internal lowpass filter to implement the glide
    lpf: DirectForm1<f32>,

    // cached val to avoid recalculating unnecessarily
    cached_t: f32,
}

impl GlideProcessor {
    /// `GlideProcessor::new(sr)` is a new glide processor with sample rate `sr`
    pub fn new(sample_rate_hz: f32) -> Self {
        let max_fc = sample_rate_hz / 2.0_f32;

        let coeffs = coeffs(sample_rate_hz.hz(), max_fc.hz());

        Self {
            max_fc,
            min_fc: 0.1_f32,
            fs: sample_rate_hz.hz(),
            lpf: DirectForm1::<f32>::new(coeffs),
            cached_t: -1.0_f32, // initialized such that it always updates the first go-round
        }
    }

    /// `gp.set_time(t)` sets the portamento time for the glide processor to the new time `t`
    ///
    /// # Arguments:
    ///
    /// * `t` - the new value for the glide control time, in `[0.0, 10.0]`
    ///
    /// Times that would be faster than sample_rate/2 are clamped.
    ///
    /// This function can be somewhat costly, so don't call it more than necessary
    pub fn set_time(&mut self, t: f32) {
        // don't update the coefficients if you don't need to, it is costly
        let epsilon = 0.05_f32;
        if is_almost(t, self.cached_t, epsilon) {
            return;
        }

        self.cached_t = t;

        let f0 = (1.0_f32 / t).max(self.min_fc).min(self.max_fc);
        self.lpf.update_coefficients(coeffs(self.fs, f0.hz()))
    }

    /// `gp.process(v)` is the value `v` processed by the glide processor, must be called periodically at the sample rate
    pub fn process(&mut self, val: f32) -> f32 {
        self.lpf.run(val)
    }
}

/// `coeffs(fs, f0)` is the lowpass filter coefficients for sample rate `fs`, cutoff frequency `f0`, and Q = 0
fn coeffs(fs: Hertz<f32>, f0: Hertz<f32>) -> Coefficients<f32> {
    Coefficients::<f32>::from_params(Type::SinglePoleLowPass, fs, f0, 0.0_f32).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_close_to_target_in_t() {
        let mut glide = GlideProcessor::new(1_000.0);
        glide.set_time(0.5);

        // start at zero
        glide.process(0.0);

        // step to 1
        for _ in 0..499 {
            glide.process(1.0);
        }

        // it should get very close to the target
        assert!(is_almost(glide.process(1.0), 1.0, 0.005));
    }

    #[test]
    fn is_monotonic() {
        let mut glide = GlideProcessor::new(1_000.0);
        glide.set_time(0.5);

        let mut last_res = glide.process(0.0);
        for _ in 0..499 {
            let res = glide.process(1.0);
            assert!(last_res < res);
            last_res = res;
        }
    }
}
