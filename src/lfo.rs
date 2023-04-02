//! # Low Frequency Oscillator
//!
//! ## Acronyms used:
//!
//! - `LFO`: Low Frequency Oscillator
//! - `LUT`: Look Up Table
//! - `DDS`: Direct Digital Synthesis
//!
//! LFOs are a standard component of most analog synthesizers. They are used to
//! modulate various parameters such as loudness, timbre, or pitch.
//!
//! This LFO has a variety of common waveforms available.
//!
//! Since this oscillator is intended as a low frequency control source, no
//! attempts at antialiasing are made. The harmonically rich waveforms (saw, square)
//! will alias even well below nyquist/2. Since there is no reconstruction
//! filter built in even the sine output will alias when the frequency is high.
//!
//! This is not objectionable when the frequency of the LFO is much lower than
//! audio frequencies and it is used to modulate parameters like filter cutoff
//! or provide VCO vibrato, which is the typical use case of this module.
//! Further, the user may wish to create crazy sci-fi effects by intentionally
//! setting the frequency high enough to cause audible aliasing, I don't judge.

use crate::{lookup_tables, phase_accumulator::PhaseAccumulator, utils::*};

/// A Low Frequency Oscillator is represented here
pub struct Lfo {
    phase_accumulator: PhaseAccumulator<TOT_NUM_ACCUM_BITS, NUM_LUT_INDEX_BITS>,
}

impl Lfo {
    /// `Lfo::new(sr)` is a new LFO with sample rate `sr`
    pub fn new(sample_rate_hz: f32) -> Self {
        Self {
            phase_accumulator: PhaseAccumulator::new(sample_rate_hz),
        }
    }

    /// `lfo.tick()` advances the LFO by 1 tick, must be called at the sample rate
    pub fn tick(&mut self) {
        self.phase_accumulator.tick()
    }

    /// `lfo.set_frequency(f)` sets the frequency of the LFO to `f`
    pub fn set_frequency(&mut self, freq: f32) {
        self.phase_accumulator.set_frequency(freq)
    }

    /// `lfo.get(ws)` is the current value of the given waveshape in `[-1.0, +1.0]`
    pub fn get(&self, waveshape: Waveshape) -> f32 {
        match waveshape {
            Waveshape::Sine => {
                let lut_idx = self.phase_accumulator.get_index();
                let next_lut_idx = (lut_idx + 1) % (lookup_tables::SINE_LUT_SIZE - 1);
                let y0 = lookup_tables::SINE_TABLE[lut_idx];
                let y1 = lookup_tables::SINE_TABLE[next_lut_idx];
                linear_interp(y0, y1, self.phase_accumulator.get_fraction())
            }
            Waveshape::Triangle => {
                // convert the phase accum ramp into a triangle in-phase with the sine
                let raw_ramp = self.phase_accumulator.get_ramp() * 4.0;
                if raw_ramp < 1.0_f32 {
                    // starting at zero and ramping up towards positive 1
                    raw_ramp
                } else if raw_ramp < 3.0_f32 {
                    // ramping down through zero towards negative 1
                    2.0_f32 - raw_ramp
                } else {
                    // ramping back up towards zero
                    raw_ramp - 4.0_f32
                }
            }
            Waveshape::UpSaw => (self.phase_accumulator.get_ramp() * 2.0_f32) - 1.0_f32,
            Waveshape::DownSaw => -self.get(Waveshape::UpSaw),
            Waveshape::Square => {
                if self.phase_accumulator.get_ramp() < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
}

/// LFO waveshapes are represented here
///
/// All waveshapes are simultaneously available
#[derive(Clone, Copy)]
pub enum Waveshape {
    Sine,
    Triangle,
    UpSaw,
    DownSaw,
    Square,
}

/// The total number of bits to use for the phase accumulator
///
/// Must be in `[1..32]`
const TOT_NUM_ACCUM_BITS: u32 = 24;

/// The number of index bits, depends on the lookup tables used
///
/// Note that the lookup table size MUST be a power of 2
const NUM_LUT_INDEX_BITS: u32 = ilog_2(lookup_tables::SINE_LUT_SIZE);
