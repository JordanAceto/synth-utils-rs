//! # Softpot resistive ribbon controller
//!
//! Spectra Symbol and others make resistive linear position sensors which may be used as ribbon controllers.
//!
//! Users play the ribbon by sliding their finger up and down a resistive track wired as a voltage divider.
//!
//! The position of the user's finger on the ribbon is represented as a number. The farther to the right the user is
//! pressing the larger the number. The position value is retained even when the user lifts their finger off of the
//! ribbon, similar to a sample-and-hold system. Some averaging is done to smooth out the raw readings and reduce the
//! influence of spurious inputs.
//!
//! Whether or not the user is pressing on the ribbon is represented as a boolean signal.
//!
//! The position value and finger-down signals are then typically used as control signals for other modules, such as
//! oscillators, filters, and amplifiers.
//!
//! # Inputs
//!
//! * Samples are fed into the ribbon controller
//!
//! # Outputs
//!
//! * The average of the most recent samples representing the position of the user's finger on the ribbon
//!
//! * Boolean signals related to the user's finger presses
//!
//! ---
//!
//! ## Note about the hardware
//!
//! The intended hardware setup can be [seen here](https://github.com/JordanAceto/synth-utils-rs/blob/main/images/ribbon_schematic_snippet.png)
//!
//! Referencing the schematic image linked above:
//! The ribbon is wired as a voltage divider between ground and a positive reference with a small series resistor `R1`
//! between the top of the ribbon and the positive ref. When the user is not pressing the ribbon the wiper is open
//! circuit. In order to detect finger presses pullup resistor `R2` is placed from the wiper to the positive reference.
//! With this setup, we can tell if the ribbon is not being pressed because `R2` pulls all the way up to the maximum
//! value. However, when a person is pressing the ribbon the maximum value is limited by the small series resistor `R1`
//! and is always lower than the "no finger pullup" value.
//!
//! Resistor `R1` is chosen to be small enough that not too much range is wasted but large enough that we can reliably
//! detect no-press conditions even with the presence of some noise. Resistor `R2` is chosen to be large enough that it
//! doesn't bend the response of the voltage divider too much but small enough that the voltage shoots up to full scale
//! quickly when the user lifts their finger. The opamp buffer is optional, but recommended to provide a low impedance
//! source to feed the ADC and provide some noise immunity if the ribbon is physically far from the pcb and connected
//! by long wires.
//!
//! It is expected that software external to this module will read the ADC and convert the raw integer-based ADC value
//! into a floating point number in the range `[0.0, 1.0]` before interfacing with this ribbon module.
//!
//! Note that this software module has no direct connection to the physical hardware. It is assumed that samples come
//! from to feed the ribbon the specified hardware setup which is sampled by an Analog to Digital Converter, but we
//! could just as easily feed it made up samples from anywhere. This allows us to have some flexibility in using this
//! module with various microcontroller setups. The above schematic snippet and description are included to illustrate
//! one way that this module could be used.
//!
//! ---

use heapless::HistoryBuffer;

/// A synthesizer ribbon controller is represented here.
///
/// It is expected to use the provided `sample_rate_to_capacity(sr)` const function to calculate the const generic
/// `BUFFER_CAPACITY` argument. If in the future Rust offers a way to calculate the buffer capacity in a more
/// straightforward way this should be changed.
pub struct RibbonController<const BUFFER_CAPACITY: usize> {
    /// Samples below this value indicate that there is a finger pressed down on the ribbon.
    ///
    /// The value must be in [0.0, +1.0], and represents the fraction of the ADC reading which counts as a finger press.
    ///
    /// The exact value depends on the resistor chosen that connects the top of the ribbon to the positive voltage
    /// reference. We "waste" a little bit of the voltage range of the ribbon as a dead-zone so we can clearly detect when
    /// the user is pressing the ribbon or not.
    finger_press_high_boundary: f32,

    /// error scaling constant used to un-bend the ribbon which is non-linear due to the pullup resistor
    error_const: f32,

    /// The current position value of the ribbon
    current_val: f32,

    /// The current gate value of the ribbon
    finger_is_pressing: bool,

    /// True iff the gate is rising after being low
    finger_just_pressed: bool,

    /// True iff the gate is falling after being high
    finger_just_released: bool,

    /// An internal buffer for storing and averaging samples as they come in via the `poll` method
    buff: HistoryBuffer<f32, BUFFER_CAPACITY>,

    /// The number of samples to ignore when the user initially presses their finger
    num_to_ignore_up_front: usize,

    /// The number of the most recent sampes to discard
    num_to_discard_at_end: usize,

    /// The number of samples revieved since the user pressed their finger down
    ///
    /// Resets when the user lifts their finger
    num_samples_received: usize,

    /// The number of samples actually written to the buffer
    ///
    /// Resets when the user lifts their finger
    num_samples_written: usize,
}

impl<const BUFFER_CAPACITY: usize> RibbonController<BUFFER_CAPACITY> {
    /// `Ribbon::new(sr, sp, dr, pu)` is a new Ribbon controller
    ///
    /// # Arguments:
    ///
    /// * `sample_rate_hz` - The sample rate in Hertz
    ///
    /// * `softpot_ohms` - The end-to-end resistance of the softpot used, typically 10k or 20k
    ///
    /// * `dropper_resistor_ohms` - The value of the resistor which sits between the top of the softpot and the positive
    /// voltage reference.
    ///
    /// * `pullup_resistor_ohms` - The value of the wiper pullup reistor, shoudl be at least 10x softpot_ohms or larger
    pub fn new(
        sample_rate_hz: f32,
        softpot_ohms: f32,
        dropper_resistor_ohms: f32,
        pullup_resistor_ohms: f32,
    ) -> Self {
        Self {
            finger_press_high_boundary: 1.0
                - (dropper_resistor_ohms / (dropper_resistor_ohms + softpot_ohms)),
            error_const: (softpot_ohms + dropper_resistor_ohms) / pullup_resistor_ohms,
            current_val: 0.0_f32,
            finger_is_pressing: false,
            finger_just_pressed: false,
            finger_just_released: false,
            buff: HistoryBuffer::new(),
            num_to_ignore_up_front: ((sample_rate_hz as u32 * RIBBON_FALL_TIME_USEC) / 1_000_000)
                as usize,
            num_to_discard_at_end: ((sample_rate_hz as u32 * RIBBON_RISE_TIME_USEC) / 1_000_000)
                as usize,
            num_samples_received: 0,
            num_samples_written: 0,
        }
    }

    /// `rib.poll(raw_adc_value)` updates the controller by polling the raw ADC signal. Must be called at the sample rate
    ///
    /// # Arguments
    ///
    /// * `raw_adc_value` - the raw ADC signal to poll in `[0.0, 1.0]`, represents the finger position on the ribbon.
    /// Inputs outside of the range `[0.0, 1.0]` are undefined.
    /// Note that a small portion of the range at the top near +1.0 is expected to be "eaten" by the series resistor
    pub fn poll(&mut self, raw_adc_value: f32) {
        let user_is_pressing_ribbon = raw_adc_value < self.finger_press_high_boundary;

        if user_is_pressing_ribbon {
            self.num_samples_received += 1;
            self.num_samples_received = self.num_samples_received.min(self.num_to_ignore_up_front);

            // only start adding samples to the buffer after we've ignored a few potentially spurious initial samples
            if self.num_to_ignore_up_front <= self.num_samples_received {
                self.buff.write(raw_adc_value);

                self.num_samples_written += 1;
                self.num_samples_written = self.num_samples_written.min(self.buff.capacity());

                // is the buffer full?
                if self.num_samples_written == self.buff.capacity() {
                    let num_to_take = self.buff.capacity() - self.num_to_discard_at_end;

                    // take the average of the most recent samples, minus a few of the very most recent ones which might be
                    // shooting up towards full scale when the user lifts their finger
                    self.current_val = self.buff.oldest_ordered().take(num_to_take).sum::<f32>()
                        / (num_to_take as f32);

                    self.current_val -= self.error_estimate(self.current_val);

                    // if this flag is false right now then they must have just pressed their finger down
                    if !self.finger_is_pressing {
                        self.finger_just_pressed = true;
                        self.finger_is_pressing = true;
                    }
                }
            }
        } else {
            // if this flag is true right now then they must have just lifted their finger
            if self.finger_is_pressing {
                self.finger_just_released = true;
                self.num_samples_received = 0;
                self.num_samples_written = 0;
                self.finger_is_pressing = false;
            }
        }
    }

    /// `rib.value()` is the current position value of the ribbon in `[0.0, 1.0]`
    ///
    /// If the user's finger is not pressing on the ribbon, the last valid value before they lifted their finger
    /// is returned.
    ///
    /// The value is expanded to take up the whole `[0.0, 1.0]`range, so even though the input will not quite reach
    /// +1.0 at the top end (due to the series resistance) the output will reach or at least come very close to +1.0
    pub fn value(&self) -> f32 {
        // scale the value back to full scale since we loose a tiny bit of range to the high-boundary
        self.current_val / self.finger_press_high_boundary
    }

    /// `rib.finger_is_pressing()` is `true` iff the user is pressing on the ribbon.
    pub fn finger_is_pressing(&self) -> bool {
        self.finger_is_pressing
    }

    /// `rib.finger_just_pressed()` is `true` iff the user has just pressed the ribbon after having not touched it.
    ///
    /// Self clearing
    pub fn finger_just_pressed(&mut self) -> bool {
        if self.finger_just_pressed {
            self.finger_just_pressed = false;
            true
        } else {
            false
        }
    }

    /// `rib.finger_just_released()` is `true` iff the user has just lifted their finger off the ribbon.
    ///
    /// Self clearing
    pub fn finger_just_released(&mut self) -> bool {
        if self.finger_just_released {
            self.finger_just_released = false;
            true
        } else {
            false
        }
    }

    /// `rib.error_estimate(p)` is the estimated error at position `p` resulting from the influence of the pullup resistor
    ///
    /// The softpot is wired as a voltage divider with an additional pullup resistor from the wiper to the positive ref.
    /// The pullup resistor bends the Vout so that it is not linear, Vout rises faster than it would without the pullup.
    ///
    /// This error estimation approximate, but can help straighten out the ribbon response
    ///
    /// # Arguments:
    ///
    /// * `pos` - the position value in `[0.0, 1.0]`
    fn error_estimate(&self, pos: f32) -> f32 {
        (pos - pos * pos) * self.error_const
    }
}

/// The approximate measured time it takes for the ribbon to settle on a low value after the user presses their finger.
///
/// We want to ignore samples taken while the ribbon is settling during a finger-press value.
///
/// Rounded up a bit from the actual measured value, better to take a little extra time than to include bad input.
const RIBBON_FALL_TIME_USEC: u32 = 1_000;

/// The approximate measured time it takes the ribbon to rise to the pull-up value after releasing your finger.
///
/// We want to ignore samples that are taken while the ribbon is shooting up towards full scale after lifting a finger.
///
/// Rounded up a bit from the actual measured value, better to take a little extra time than to include bad input.
const RIBBON_RISE_TIME_USEC: u32 = 2_000;

/// The minimum time required to capture a reading
///
/// Ideally several times longer than the sum of the RISE and FALL times
const MIN_CAPTURE_TIME_USEC: u32 = (RIBBON_FALL_TIME_USEC + RIBBON_RISE_TIME_USEC) * 5;

/// `sample_rate_to_capacity(sr_hz)` is the calculated capacity needed for the internal buffer based on the sample rate.
///
/// Const function allows us to use the result of this expression as a generic argument when we create ribbon objects.
/// If rust support for generic expressions improves, this function could be refactored out.
///
/// The capacity needs space for the main samples that we will actually care about, as well as room for the most
/// recent samples to discard. This is to avoid including spurious readings in the average.
pub const fn sample_rate_to_capacity(sample_rate_hz: u32) -> usize {
    // can't use floats in const function yet
    let num_main_samples_to_care_about =
        ((sample_rate_hz * MIN_CAPTURE_TIME_USEC) / 1_000_000) as usize;
    let num_to_discard_at_end = ((sample_rate_hz * RIBBON_RISE_TIME_USEC) / 1_000_000) as usize;

    num_main_samples_to_care_about + num_to_discard_at_end + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: f32 = 10_000.0;
    const RIBBON_BUFF_CAPACITY: usize = sample_rate_to_capacity(SAMPLE_RATE as u32);

    /// `test_ribbon()` is a basic ribbon controller for testing
    fn test_ribbon() -> RibbonController<RIBBON_BUFF_CAPACITY> {
        RibbonController::new(SAMPLE_RATE as f32, 20E3, 820.0, 1E6)
    }

    // a bit glass-boxy, but hard to test otherwise, hand calculated by inspecting the code
    const _TEST_RIB_NUM_TO_IGNORE_UP_FRONT: u32 = 10;
    const TEST_RIB_NUM_TO_IGNORE_AT_END: u32 = 20;
    const _TEST_RIB_NUM_TO_CARE_ABOUT: u32 = 150;
    const _TEST_RIB_CAPACITY: u32 = 170;
    const TEST_RIB_NUM_FOR_VALID_READING: u32 = 180;

    #[test]
    fn should_have_dead_zone_before_value_is_captured() {
        let mut rib = test_ribbon();

        // poll some samples, but not enough to get a reading yet
        for _ in 0..(TEST_RIB_NUM_FOR_VALID_READING - 1) {
            rib.poll(0.42);
        }
        assert!(!rib.finger_is_pressing());
    }

    #[test]
    fn should_eventually_register_reading_with_enough_polling() {
        let mut rib = test_ribbon();

        // poll some samples, but not enough to get a reading yet
        for _ in 0..TEST_RIB_NUM_FOR_VALID_READING - 1 {
            rib.poll(0.42);
        }
        assert!(!rib.finger_is_pressing());

        rib.poll(0.42);
        assert!(rib.finger_is_pressing());
    }

    #[test]
    fn one_oob_poll_means_finger_not_pressing() {
        let mut rib = test_ribbon();

        // poll enough to register a reading
        for _ in 0..TEST_RIB_NUM_FOR_VALID_READING {
            rib.poll(0.42);
        }
        assert!(rib.finger_is_pressing());

        // 1.0 is always out-of-bounds, there is always some lost to the resistor
        rib.poll(1.0);
        assert!(!rib.finger_is_pressing());
    }

    #[test]
    fn last_val_retained_after_finger_lifted() {
        let mut rib = test_ribbon();

        // poll enough to register a reading
        for _ in 0..TEST_RIB_NUM_FOR_VALID_READING {
            rib.poll(0.42);
        }
        let old_val = rib.value();

        // 1.0 is always out-of-bounds, there is always some lost to the resistor
        rib.poll(1.0);
        assert!(!rib.finger_is_pressing());
        assert_eq!(rib.value(), old_val);
    }

    #[test]
    fn bigger_inputs_increase_output() {
        let mut rib = test_ribbon();

        // poll enough to register a reading
        for _ in 0..TEST_RIB_NUM_FOR_VALID_READING {
            rib.poll(0.1);
        }
        let old_val = rib.value();

        // do some polling with the new val but don't fill the buffer entirely with new stuff
        for _ in 0..TEST_RIB_NUM_FOR_VALID_READING / 2 {
            rib.poll(0.2);
        }
        assert!(old_val < rib.value());
    }

    #[test]
    fn smaller_inputs_decrease_output() {
        let mut rib = test_ribbon();

        // poll enough to register a reading
        for _ in 0..TEST_RIB_NUM_FOR_VALID_READING {
            rib.poll(0.7);
        }
        let old_val = rib.value();

        for _ in 0..TEST_RIB_NUM_FOR_VALID_READING / 4 {
            rib.poll(0.6);
        }
        assert!(rib.value() < old_val);
    }

    #[test]
    fn rising_gate_triggers() {
        let mut rib = test_ribbon();

        for _ in 0..TEST_RIB_NUM_FOR_VALID_READING {
            rib.poll(0.1);
        }
        assert!(rib.finger_just_pressed());
        // it's self clearing
        assert!(!rib.finger_just_pressed());

        // still pressing the ribbon, so no new just-pressed
        rib.poll(0.1);
        assert!(!rib.finger_just_pressed());
    }

    #[test]
    fn last_few_inputs_are_ignored() {
        let mut rib = test_ribbon();

        // poll enough to register a reading
        for _ in 0..TEST_RIB_NUM_FOR_VALID_READING {
            rib.poll(0.0);
        }
        assert!(rib.finger_is_pressing());
        assert_eq!(rib.value(), 0.0);

        //add a few valid readings at the end, which will be ignored
        for _ in 0..TEST_RIB_NUM_TO_IGNORE_AT_END {
            rib.poll(0.9);
        }
        assert_eq!(rib.value(), 0.0);

        // one more sample will be factored in to the average
        rib.poll(0.9);
        assert!(0.0 < rib.value());
    }
}
