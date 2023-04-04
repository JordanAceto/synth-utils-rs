//! # Attack, Decay, Sustain, Release generator
//!
//! ## Acronyms used:
//!
//! - `ADSR`: Attack Decay Sustain Release generator
//! - `LUT`:  Look Up Table
//! - `DDS`:  Direct Digital Synthesis
//!
//! ADSRs are a standard component of most analog synthesizers, used to
//! dynamically modulate various parameters of synthesizers, most commonly
//! loudness, timbre, or pitch.
//!
//! This ADSR simulates the RC curves typically found in analog ADSRs, where
//! the attack curve is a truncated up-going convex RC curve, and the decay and
//! release curves are down-going concave RC curves.
//!
//! This ADSR has four variable input parameters:
//!
//! - Attack time
//! - Decay time
//! - Sustain level
//! - Release time
//!
//! This ADSR responds to two types of time based events:
//!     
//! - Gate On events initiate an attack phase
//! - Gate Off events initiate a release phase
//!
//! This ADSR has a single output:
//!
//! - The current sample of the ADSR waveform in the range [0.0, 1.0].
//!
//! A Phase-Accumulator and Look-Up-Table (LUT) approach is used.
//! This is known as "Direct Digital Synthesis", or DDS.
//!
//! LUTs are used to store the Attack and Decay curves for the ADSRs. These
//! curves simulate the typical resistor/capacitor time constant curves used in
//! analog ADSRs.

use crate::{lookup_tables, phase_accumulator::PhaseAccumulator, utils::*};

/// An ADSR envelope generator is represented here
pub struct Adsr {
    attack_time: TimePeriod,
    decay_time: TimePeriod,
    sustain_level: SustainLevel,
    release_time: TimePeriod,

    phase_accumulator: PhaseAccumulator<TOT_NUM_ACCUM_BITS, NUM_LUT_INDEX_BITS>,

    state: State,

    value_when_gate_on_received: f32,
    value_when_gate_off_received: f32,
    value: f32,
}

impl Adsr {
    /// `Adrs::new(sr)` is a new ADSR with sample rate `sr`
    pub fn new(sample_rate_hz: f32) -> Self {
        Self {
            // set defaults for very fast times and 100% on sustain
            attack_time: MIN_TIME_PERIOD_SEC.into(),
            decay_time: MIN_TIME_PERIOD_SEC.into(),
            sustain_level: 1.0_f32.into(),
            release_time: MIN_TIME_PERIOD_SEC.into(),

            phase_accumulator: PhaseAccumulator::new(sample_rate_hz),
            state: State::AtRest,
            value_when_gate_on_received: 0.0_f32,
            value_when_gate_off_received: 0.0_f32,
            value: 0.0f32,
        }
    }

    /// `adsr.tick()` advances the ADSR by 1 tick, must be called at the sample rate
    pub fn tick(&mut self) {
        // only calculate frequency and tick the accumulator for tick-able states
        if self.state == State::Attack || self.state == State::Decay || self.state == State::Release
        {
            let period_of_this_phase = match self.state {
                State::Attack => self.attack_time.0,
                State::Decay => self.decay_time.0,
                State::Release => self.release_time.0,
                // SUSTAIN and AT-REST have no period, these can never happen here. But don't use wildcards, we want the
                // compiler to complain if anyone adds more stages to make more complex envelopes (hold time, whatever)
                State::Sustain => MIN_TIME_PERIOD_SEC,
                State::AtRest => MIN_TIME_PERIOD_SEC,
            };

            self.phase_accumulator.set_period(period_of_this_phase);

            self.phase_accumulator.tick();

            if self.phase_accumulator.rolled_over() {
                self.state = match self.state {
                    State::Attack => State::Decay,
                    State::Decay => State::Sustain,
                    State::Release => State::AtRest,
                    // SUSTAIN and AT-REST can't happen here, but explicitly match all arms
                    State::Sustain => State::Sustain,
                    State::AtRest => State::AtRest,
                };
            }
        }

        // calculate the output no matter which state
        self.value = self.calc_value();
    }

    /// `adsr.gate_on()` sends a gate-on message to the ADSR, triggering an ATTACK phase if it's not already in ATTACK
    ///
    /// Attack phases may be re-triggered by sending a new gate-on message during any phase.
    pub fn gate_on(&mut self) {
        match self.state {
            State::AtRest | State::Decay | State::Sustain | State::Release => {
                self.value_when_gate_on_received = self.value;
                self.phase_accumulator.reset();
                self.state = State::Attack;
            }
            State::Attack => (), // ignore the message, we're already in an attack phase
        }
    }

    /// `adsr.gate_off()` sends a gate-off message to the ADSR, triggering a RELEASE phase unless it's already RELEASED
    pub fn gate_off(&mut self) {
        match self.state {
            State::Attack | State::Decay | State::Sustain => {
                self.value_when_gate_off_received = self.value;
                self.phase_accumulator.reset();
                self.state = State::Release;
            }
            State::Release | State::AtRest => (), // ignore the message, we're already in a release or at-rest phase
        }
    }

    /// `adsr.value()` is the current value of the ADSR in `[0.0, 1.0]`
    pub fn value(&self) -> f32 {
        self.value
    }

    /// `adsr.set_input(i)` sets the given ADSR input
    ///
    /// # Examples
    ///
    /// ```
    /// # use synth_utils::adsr;
    /// # let mut adsr = adsr::Adsr::new(1_000.0_f32);
    ///
    /// // set attack time to 30 milliseconds
    /// adsr.set_input(adsr::Input::Attack(0.03_f32.into()));
    ///
    /// // set decay time to 100 milliseconds
    /// adsr.set_input(adsr::Input::Decay(0.1_f32.into()));
    ///
    /// // set sustain level to 3/4 way up
    /// adsr.set_input(adsr::Input::Sustain(0.75_f32.into()));
    ///
    /// // set release time to 150 milliseconds
    /// adsr.set_input(adsr::Input::Release(0.15_f32.into()));
    /// ```
    pub fn set_input(&mut self, input: Input) {
        match input {
            Input::Attack(a) => self.attack_time = a,
            Input::Decay(d) => self.decay_time = d,
            Input::Sustain(s) => self.sustain_level = s,
            Input::Release(r) => self.release_time = r,
        }
    }

    /// `adsr.calc_value()` is a private helper function to calculate the current ADSR value
    fn calc_value(&self) -> f32 {
        // The coefficient for the sample is between 0 and 1.0. This is used to
        // "squish" the attack, decay, and release curves as needed.

        // Example: the decay curve starts at full scale, and ramps down to the sustain
        // level. The range of the decay curve from top to bottom is full-scale at the
        // top to sustain level at the bottom. The decay curve must be compressed to
        // fit in this reduced range. The coefficient variable helps accomplish this.
        let coefficient: f32;

        // The value of the current sample. This will come from the attack LUT if the
        // current state is attack, from the decay LUT if the current state is decay
        // or release, and from the sustain level input if the current state is
        // sustain. If the current state is at-rest, the value of the sample will be zero
        let sample: f32;

        // The offset for the current sample. This is only non-zero when an attack
        // phase begins while the ADSR is not at rest, or a decay phase begins while
        // the sustain level is non-zero. Basically this is how much to "push up" the
        // ADSR curve, so that it fits between the starting value for the curve segment
        // and the target value for the curve segment.
        let offset: f32;

        let lut_idx = self.phase_accumulator.index();
        // next idx is for interpolation, clamp at the end to avoid bad behavior, we don't want to wrap around here
        let next_lut_idx = (lut_idx + 1).min(lookup_tables::ADSR_CURVE_LUT_SIZE - 1);

        match self.state {
            State::Attack => {
                let y0 = lookup_tables::ADSR_ATTACK_TABLE[lut_idx];
                let y1 = lookup_tables::ADSR_ATTACK_TABLE[next_lut_idx];
                coefficient = 1.0_f32 - self.value_when_gate_on_received;
                sample = linear_interp(y0, y1, self.phase_accumulator.fraction());
                offset = self.value_when_gate_on_received;
            }
            State::Decay => {
                let y0 = lookup_tables::ADSR_DECAY_TABLE[lut_idx];
                let y1 = lookup_tables::ADSR_DECAY_TABLE[next_lut_idx];
                coefficient = 1.0_f32 - self.sustain_level.0;
                sample = linear_interp(y0, y1, self.phase_accumulator.fraction());
                offset = self.sustain_level.0;
            }
            State::Sustain => {
                coefficient = 1.0_f32;
                sample = self.sustain_level.0;
                offset = 0.0;
            }
            State::Release => {
                let y0 = lookup_tables::ADSR_DECAY_TABLE[lut_idx];
                let y1 = lookup_tables::ADSR_DECAY_TABLE[next_lut_idx];
                coefficient = self.value_when_gate_off_received;
                sample = linear_interp(y0, y1, self.phase_accumulator.fraction());
                offset = 0.0;
            }
            State::AtRest => {
                coefficient = 0.0_f32;
                sample = 0.0_f32;
                offset = 0.0;
            }
        };

        coefficient * sample + offset
    }
}

/// ADSR input types are represented here
///
/// A, D, and S are represented as positive-only time periods, S is represented as a number in `[0.0, 1.0]`
pub enum Input {
    Attack(TimePeriod),
    Decay(TimePeriod),
    Sustain(SustainLevel),
    Release(TimePeriod),
}

/// A time period in seconds is represented here
///
/// Time periods are positive only numbers with min and max values in a pleasing range for users of the ADSR
pub struct TimePeriod(f32);

impl From<f32> for TimePeriod {
    fn from(p: f32) -> Self {
        Self(p.max(MIN_TIME_PERIOD_SEC).min(MAX_TIME_PERIOD_SEC))
    }
}

/// A sustain level in the range `[0.0, 1.0]` is represented here
pub struct SustainLevel(f32);

impl From<f32> for SustainLevel {
    fn from(val: f32) -> Self {
        Self(val.max(0.0_f32).min(1.0_f32))
    }
}

/// ADSR states are represented here
///
/// An ADSR is in exactly one of these states at any given time
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum State {
    AtRest,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// The minimum time period for an ADSR state period
pub const MIN_TIME_PERIOD_SEC: f32 = 0.001_f32;

/// The maximum time period for an ADSR state period
pub const MAX_TIME_PERIOD_SEC: f32 = 20.0_f32;

/// The total number of bits to use for the phase accumulator
///
/// Must be in `[1..32]`
const TOT_NUM_ACCUM_BITS: u32 = 24;

/// The number of index bits, depends on the lookup tables used
///
/// Note that the lookup table size MUST be a power of 2
const NUM_LUT_INDEX_BITS: u32 = ilog_2(lookup_tables::ADSR_CURVE_LUT_SIZE);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_on_starts_attack_phase_from_at_rest() {
        let mut adsr = Adsr::new(1_000.0_f32);

        //. it starts out at-rest
        assert_eq!(adsr.state, State::AtRest);

        adsr.gate_on();
        assert_eq!(adsr.state, State::Attack);
    }

    #[test]
    fn attack_transitions_to_decay_after_ticks() {
        let mut adsr = Adsr::new(1_000.0_f32);

        // 100 millisecond stages at 1kHz sample rate should complete after 101 ticks
        adsr.set_input(Input::Attack(0.1.into()));

        adsr.gate_on();
        assert_eq!(adsr.state, State::Attack);

        // almost done with attack phase
        for _ in 0..100 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::Attack);

        // one more tick puts us into a decay phase
        adsr.tick();
        assert_eq!(adsr.state, State::Decay);
    }

    #[test]
    fn transition_through_phases() {
        let mut adsr = Adsr::new(1_000.0_f32);

        // 100 millisecond stages at 1kHz sample rate should complete after 101 ticks
        adsr.set_input(Input::Attack(0.1.into()));
        adsr.set_input(Input::Decay(0.1.into()));
        adsr.set_input(Input::Sustain(0.5.into()));
        adsr.set_input(Input::Release(0.1.into()));

        adsr.gate_on();

        for _ in 0..101 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::Decay);

        for _ in 0..101 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::Sustain);

        // only a gate-off message initiates a release phase
        adsr.gate_off();
        assert_eq!(adsr.state, State::Release);

        for _ in 0..101 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::AtRest);
    }

    #[test]
    fn attack_can_retrigger_in_any_phase() {
        let mut adsr = Adsr::new(1_000.0_f32);

        // 100 millisecond stages at 1kHz sample rate should complete after 101 ticks
        adsr.set_input(Input::Attack(0.1.into()));
        adsr.set_input(Input::Decay(0.1.into()));
        adsr.set_input(Input::Sustain(0.5.into()));
        adsr.set_input(Input::Release(0.1.into()));

        adsr.gate_on();

        for _ in 0..101 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::Decay);

        adsr.gate_on();
        assert_eq!(adsr.state, State::Attack);

        for _ in 0..202 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::Sustain);

        adsr.gate_on();
        assert_eq!(adsr.state, State::Attack);

        for _ in 0..202 {
            adsr.tick();
        }
        adsr.gate_off();
        assert_eq!(adsr.state, State::Release);

        adsr.gate_on();
        assert_eq!(adsr.state, State::Attack);
    }

    #[test]
    fn release_can_start_from_any_phase_but_at_rest() {
        let mut adsr = Adsr::new(1_000.0_f32);

        // 100 millisecond stages at 1kHz sample rate should complete after 101 ticks
        adsr.set_input(Input::Attack(0.1.into()));
        adsr.set_input(Input::Decay(0.1.into()));
        adsr.set_input(Input::Sustain(0.5.into()));
        adsr.set_input(Input::Release(0.1.into()));

        adsr.gate_on();

        for _ in 0..50 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::Attack);

        adsr.gate_off();
        assert_eq!(adsr.state, State::Release);

        adsr.gate_on();
        for _ in 0..101 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::Decay);

        adsr.gate_off();
        assert_eq!(adsr.state, State::Release);

        adsr.gate_on();
        for _ in 0..202 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::Sustain);

        adsr.gate_off();
        assert_eq!(adsr.state, State::Release);

        for _ in 0..101 {
            adsr.tick();
        }
        assert_eq!(adsr.state, State::AtRest);

        // turning the gate off when we're already at rest doesn't start a new release phase
        adsr.gate_off();
        assert_eq!(adsr.state, State::AtRest);
    }
}
