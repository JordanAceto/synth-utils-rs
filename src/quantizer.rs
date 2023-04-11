//! # Quantizer
//!
//! Quantizers are used to force continuous inputs into discrete output steps. Musically they are used to generate
//! in-tune outputs from various inputs.
//!
//! This quantizer operates similarly to common hardware quantizers, using 1volt/octave scaling. This means that each
//! octave spans 1 volt, and so each semitone spans 1/12 of a volt, or about 83.3mV
//!
//! Specific notes may be allowed or forbidden, allowing the user to program user defined scales.

use heapless::Vec;

/// A quantizer which converts smooth inputs into stairsteps is represented here.
pub struct Quantizer {
    // save the last conversion for hysteresis purposes
    cached_conversion: Conversion,

    // allowed notes are represented as an integer bitfield
    // the 12 lowest bits represent C, C#, D, ... B
    // a set-bit means the note is allowed, cleared-bit means the note is forbidden
    allowed: u16,
}

/// A quantizer conversion is represented here.
///
/// Conversions consist of a stairstep portion and fractional portion.
/// The stairstep is the input value converted to a stairstep with as many steps as there are semitones, and the
/// fractional part is the difference between the actual input value and the quantized stairstep.
///
/// The stairstep will always be positive, the fraction may be positive or negative.
/// The stairstep plus the fraction will get us back to the original input value.
///
/// The integer note number is also included.
#[derive(Clone, Copy)]
pub struct Conversion {
    /// The integer note number of the conversion
    pub note_num: u8,
    /// The conversion as a stairstep pattern, in the same range as the input except quantized to discrete steps
    pub stairstep: f32,
    /// The fractional remainder of the stairstep, `stairstep + fraction` results in the original input value
    pub fraction: f32,
}

impl Conversion {
    /// `Conversion::default()` is a new default conversion
    pub fn default() -> Self {
        Self {
            note_num: 0,
            stairstep: f32::MIN, // initialized so that hysteresis doesn't influence the first conversion
            fraction: 0.0_f32,
        }
    }
}

impl Default for Quantizer {
    /// `Quantizer::default()` is a new default quantizer with all notes allowed.
    fn default() -> Self {
        Self {
            cached_conversion: Conversion::default(),
            allowed: 0b0000_1111_1111_1111, // all 12 notes allowed
        }
    }
}

impl Quantizer {
    /// `Quantizer::new()` is a new quantizer with all notes allowed.
    pub fn new() -> Self {
        Self::default()
    }

    /// `q.convert(val)` is the quantized version of the input value.
    ///
    /// The input is split into a stairstep component and fractional component.
    ///
    /// # Arguments
    ///
    /// * `v_in` - the value to quantize, in volts, clamped to `[0.0, V_MAX]`
    ///
    /// # Returns
    ///
    /// * `Conversion` - the input split into a stairstep and fractional portion
    ///
    /// # Examples
    ///
    /// ```
    /// # use synth_utils::quantizer;
    /// let mut q = quantizer::Quantizer::new();
    /// // input is a bit above C#, but C# is the closest note number
    /// assert_eq!(q.convert(1.5 / 12.).note_num, 1);
    ///
    /// // same input, but since C# is forbidden now D is the closest note
    /// q.forbid(&[quantizer::Note::CSHARP]);
    /// assert_eq!(q.convert(1.5 / 12.).note_num, 2);
    /// ```
    ///
    pub fn convert(&mut self, v_in: f32) -> Conversion {
        // return early if vin is within the window of the last coversion plus a little hysteresis
        if self.is_allowed(self.cached_conversion.note_num.into()) {
            let low_bound = self.cached_conversion.stairstep - HYSTERESIS;
            let high_bound = self.cached_conversion.stairstep + SEMITONE_WIDTH + HYSTERESIS;

            if low_bound < v_in && v_in < high_bound {
                return self.cached_conversion;
            }
        }

        let v_in = v_in.max(0.0_f32).min(V_MAX);

        self.cached_conversion.note_num = self.find_nearest_note(v_in);
        self.cached_conversion.stairstep = self.cached_conversion.note_num as f32 / 12.0_f32;
        self.cached_conversion.fraction = v_in - self.cached_conversion.stairstep;

        self.cached_conversion
    }

    /// `q.find_nearest_note(v)` is 1volt/octave voltage `v` converted to the nearest semitone number
    fn find_nearest_note(&self, v_in: f32) -> u8 {
        let vin_microvolts = (v_in * ONE_OCTAVE_IN_MICROVOLTS as f32) as u32;
        let octave_num_of_vin = vin_microvolts / ONE_OCTAVE_IN_MICROVOLTS;

        // we want to look in either two or three octaves to find the nearest note
        // it might be in the same octave as the input, but the nearest note might also be in the octave above or below
        // we can't go below octave zero or above MAX_OCTAVE, so there might be only two to check if we're near an edge
        let mut octaves_to_search = Vec::<u32, 3>::new();
        octaves_to_search.push(octave_num_of_vin).ok();
        if 1 <= octave_num_of_vin {
            octaves_to_search.push(octave_num_of_vin - 1).ok();
        }
        if octave_num_of_vin < MAX_OCTAVE {
            octaves_to_search.push(octave_num_of_vin + 1).ok();
        }

        let mut nearest_note_so_far_microvolts = 0;
        let mut smallest_delta_so_far = u32::MAX;

        for octave in octaves_to_search {
            for n in 0..12 {
                let this_note_is_enabled = (self.allowed >> n) & 1 == 1;

                if this_note_is_enabled {
                    let candidate_note_microvolts =
                        n * HALF_STEP_IN_MICROVOLTS + octave * ONE_OCTAVE_IN_MICROVOLTS;

                    let delta = delta(vin_microvolts, candidate_note_microvolts);

                    // early return if we get very close to an enabled note, this must be the one
                    if delta < HALF_STEP_IN_MICROVOLTS {
                        return (candidate_note_microvolts / HALF_STEP_IN_MICROVOLTS) as u8;
                    }

                    // early return if delta starts getting bigger, this means that we passed the right note
                    if smallest_delta_so_far < delta {
                        return (nearest_note_so_far_microvolts / HALF_STEP_IN_MICROVOLTS) as u8;
                    }

                    if delta < smallest_delta_so_far {
                        smallest_delta_so_far = delta;
                        nearest_note_so_far_microvolts = candidate_note_microvolts;
                    }
                }
            }
        }

        (nearest_note_so_far_microvolts / HALF_STEP_IN_MICROVOLTS) as u8
    }

    /// `q.allow(ns)` allows notes `ns`, meaning they will be included in conversions
    ///
    /// Any notes in `ns` that are already allowed are left unchanged
    pub fn allow(&mut self, notes: &[Note]) {
        notes.iter().for_each(|n| {
            self.allowed |= 1 << n.0;
        })
    }

    /// `q.forbid(ns)` forbids notes `ns`, they will not be included in conversions even if they are the nearest note
    ///
    /// Any notes in `ns` that are already forbidden are left unchanged
    ///
    /// At least one note must always be left allowed. If `ns` would forbid every note, the last note in `ns` will not
    /// be forbidden and instead will be left allowed.
    pub fn forbid(&mut self, notes: &[Note]) {
        notes.iter().for_each(|n| self.allowed &= !(1 << n.0));
        if self.allowed == 0 {
            self.allow(&notes[notes.len() - 1..])
        }
    }

    /// `q.is_allowed(n)` is true iff note `n` is allowed
    pub fn is_allowed(&self, note: Note) -> bool {
        self.allowed >> note.0 & 1 == 1
    }
}

fn delta(v1: u32, v2: u32) -> u32 {
    if v1 < v2 {
        v2 - v1
    } else {
        v1 - v2
    }
}

/// Note names are represented here, the quantizer can allow and forbid various notes from being converted
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Note(u8);

impl Note {
    pub const C: Self = Self::new(0);
    pub const CSHARP: Self = Self::new(1);
    pub const D: Self = Self::new(2);
    pub const DSHARP: Self = Self::new(3);
    pub const E: Self = Self::new(4);
    pub const F: Self = Self::new(5);
    pub const FSHARP: Self = Self::new(6);
    pub const G: Self = Self::new(7);
    pub const GSHARP: Self = Self::new(8);
    pub const A: Self = Self::new(9);
    pub const ASHARP: Self = Self::new(10);
    pub const B: Self = Self::new(11);

    /// `Note::new(n)` is a new note from `n` clamped to `[0..11]`
    pub const fn new(n: u8) -> Self {
        Self(if n <= 11 { n } else { 11 })
    }
}

impl From<u8> for Note {
    fn from(n: u8) -> Self {
        Self::new(n)
    }
}

impl From<Note> for u8 {
    fn from(n: Note) -> Self {
        n.0
    }
}

pub const NUM_NOTES_PER_OCTAVE: f32 = 12.0_f32;

/// The width of each bucket for the semitones.
pub const SEMITONE_WIDTH: f32 = 1.0_f32 / NUM_NOTES_PER_OCTAVE;
pub const HALF_SEMITONE_WIDTH: f32 = SEMITONE_WIDTH / 2.0_f32;

/// Hysteresis provides some noise immunity and prevents oscillations near transition regions.
const HYSTERESIS: f32 = SEMITONE_WIDTH * 0.1_f32;

const ONE_OCTAVE_IN_MICROVOLTS: u32 = 1_000_000;

const HALF_STEP_IN_MICROVOLTS: u32 = ONE_OCTAVE_IN_MICROVOLTS / 12;

const MAX_OCTAVE: u32 = 10;

const V_MAX: f32 = MAX_OCTAVE as f32;

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn vin_0_is_note_num_zero_with_all_allowed() {
        let mut q = Quantizer::new();
        assert_eq!(q.convert(0.0).note_num, 0);
    }

    #[test]
    fn vin_point_08333_is_note_num_1_with_all_allowed() {
        let mut q = Quantizer::new();
        assert_eq!(q.convert(1. / 12.).note_num, 1);
    }

    #[test]
    fn vin_1_is_note_num_12_with_all_allowed() {
        let mut q = Quantizer::new();
        assert_eq!(q.convert(1.).note_num, 12);
    }

    #[test]
    fn vin_2_point_08333_is_note_num_25_with_all_allowed() {
        let mut q = Quantizer::new();
        assert_eq!(q.convert(2. + 1. / 12.).note_num, 25);
    }

    #[test]
    fn when_C_is_forbidden_vin_0_is_1() {
        let mut q = Quantizer::new();
        q.forbid(&[Note::C]);
        assert_eq!(q.convert(0.0).note_num, 1);
    }

    #[test]
    fn when_only_B_is_allowed_vin_0_is_11() {
        let mut q = Quantizer::new();
        q.forbid(&[
            Note::C,
            Note::CSHARP,
            Note::D,
            Note::DSHARP,
            Note::E,
            Note::F,
            Note::FSHARP,
            Note::G,
            Note::GSHARP,
            Note::A,
            Note::ASHARP,
            // Note::B,
        ]);
        assert_eq!(q.convert(0.0).note_num, 11);
    }

    #[test]
    fn when_only_Dsharp_is_allowed_vin_8_12ths_is_3() {
        let mut q = Quantizer::new();
        q.forbid(&[
            Note::C,
            Note::CSHARP,
            Note::D,
            // Note::Dsharp,
            Note::E,
            Note::F,
            Note::FSHARP,
            Note::G,
            Note::GSHARP,
            Note::A,
            Note::ASHARP,
            Note::B,
        ]);
        // it picks the D# in octave zero
        assert_eq!(q.convert(8. / 12.).note_num, 3);
    }

    #[test]
    fn when_only_Dsharp_is_allowed_vin_10_12ths_is_15() {
        let mut q = Quantizer::new();
        q.forbid(&[
            Note::C,
            Note::CSHARP,
            Note::D,
            // Note::Dsharp,
            Note::E,
            Note::F,
            Note::FSHARP,
            Note::G,
            Note::GSHARP,
            Note::A,
            Note::ASHARP,
            Note::B,
        ]);
        // it picks the D# in octave 1
        assert_eq!(q.convert(10. / 12.).note_num, 15);
    }

    #[test]
    fn can_not_forbid_every_note() {
        let mut q = Quantizer::new();
        // try to forbid every note
        q.forbid(&[
            Note::C,
            Note::CSHARP,
            Note::D,
            Note::DSHARP,
            Note::E,
            Note::F,
            Note::FSHARP,
            Note::G,
            Note::GSHARP,
            Note::A,
            Note::ASHARP,
            Note::B,
        ]);
        // B is still left, because it is the last one we tried to forbid
        assert_eq!(q.convert(0.5).note_num, 11);
    }

    #[test]
    fn hysteresis_widens_window() {
        let mut q = Quantizer::new();

        // register a conversion with note number 1
        assert_eq!(q.convert(1. / 12. + HALF_SEMITONE_WIDTH * 0.99).note_num, 1);

        // it is now a little harder to get back out of 1, due to hysteresis
        assert_eq!(q.convert(1. / 12. - HYSTERESIS * 0.99).note_num, 1);
        assert_eq!(
            q.convert(1. / 12. + SEMITONE_WIDTH + HYSTERESIS * 0.99)
                .note_num,
            1
        );

        // starting from scratch the same input values map to the below and above semitones
        let mut q = Quantizer::new();
        assert_eq!(q.convert(1. / 12. - HYSTERESIS * 0.99).note_num, 0);

        let mut q = Quantizer::new();
        assert_eq!(
            q.convert(1. / 12. + SEMITONE_WIDTH + HYSTERESIS * 0.99)
                .note_num,
            2
        );
    }
}
