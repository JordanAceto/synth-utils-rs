//! Read MIDI input and write some values via PWM on pins PA0 and PA1 and GPIO pin PA4
//!
//! Hardware setup:
//!
//! You need to setup a MIDI input circuit according to the MIDI spec:
//!
//! https://www.midi.org/specifications-old/item/midi-din-electrical-specification
//!
//! The output of the optoisolator goes to the stm32 UART1 RX pin PA10
//!
//! Then, connect a MIDI device set to output on channel 1 and play some notes and wiggle the pitch bend wheel.
//!
//! As played notes get higher the PWM ratio on PA0 gets closer to 100%. As the pitch bend goes up and down so does the
//! PWM signal on PA1. When notes are held down PA4 goes high, and when all notes are lifted PA4 goes low.
//!
//! The goal of this demo is simply to verify that we can receive MIDI data and that the MIDI receiver code is
//! able to parse it with a minimal amount of hardware setup. No attempts at generating an accurate 1volt/octave
//! CV signal are made. This example could be used as a starting point to build a MIDI to CV converter with some
//! scaling and a proper DAC.
//!
//! To flash:
//! $ cargo flash --example midi_pwm_write --chip stm32f103rb --release

#![deny(unsafe_code)]
#![no_std]
#![no_main]

use panic_halt as _;

use cortex_m_rt::entry;
use stm32f1xx_hal::{
    pac,
    prelude::*,
    serial::{Config, Serial},
    timer::{Channel, Tim2NoRemap, Timer},
};

use synth_utils::mono_midi_receiver;

#[entry]
fn main() -> ! {
    ////////////////////////////////////////////////////////////////////////////
    //
    // general stm32 peripheral housekeeping
    //
    ////////////////////////////////////////////////////////////////////////////
    let dp = pac::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut afio = dp.AFIO.constrain();
    let mut gpioa = dp.GPIOA.split();

    ////////////////////////////////////////////////////////////////////////////
    //
    // UART for MIDI input
    //
    ////////////////////////////////////////////////////////////////////////////
    let tx = gpioa.pa9.into_alternate_push_pull(&mut gpioa.crh);
    let rx = gpioa.pa10;

    let serial = Serial::new(
        dp.USART1,
        (tx, rx),
        &mut afio.mapr,
        Config::default().baudrate(31_250.bps()), // MIDI baud rate, important!
        &clocks,
    );

    let (mut _tx, mut rx) = serial.split();

    ////////////////////////////////////////////////////////////////////////////
    //
    // PWM and GPIO for outputs
    //
    ////////////////////////////////////////////////////////////////////////////
    let pwm_pins = (
        gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl),
        gpioa.pa1.into_alternate_push_pull(&mut gpioa.crl),
    );

    let mut pwm = Timer::new(dp.TIM2, &clocks).pwm_hz::<Tim2NoRemap, _, _>(
        pwm_pins,
        &mut afio.mapr,
        10.kHz(),
    );

    pwm.enable(Channel::C1);
    pwm.enable(Channel::C2);

    let pwm_max = pwm.get_max_duty();

    let mut gate_pin = gpioa.pa4.into_push_pull_output(&mut gpioa.crl);

    // create the MIDI receiver, note that the MIDI channel to use on external gear will be 1 greater than this number,
    // so set your MIDI keyboard to channel 1 (or n+1 if you want to experiment with a different channel n)
    let zero_indexed_midi_channel = 0;
    let mut midi = mono_midi_receiver::MonoMidiReceiver::new(zero_indexed_midi_channel);

    loop {
        if let Ok(byte) = rx.read() {
            // parse bytes as soon as we receive them
            midi.parse(byte);
        }

        // write the MIDI note and pitch bend via PWM, verify with oscilloscope or LED
        // experiment with various other MIDI signals (mod wheel, velocity, VCF frequency, etc)
        pwm.set_duty(Channel::C1, midi_note_to_pwm(midi.note_num(), pwm_max));
        pwm.set_duty(Channel::C2, midi_pb_to_pwm(midi.pitch_bend(), pwm_max));

        if midi.gate() {
            gate_pin.set_high();
        } else {
            gate_pin.set_low();
        }
    }
}

// `midi_note_to_pwm(n)` is the MIDI note number `n` expanded to fill up the PMW range
fn midi_note_to_pwm(note: u8, pwm_max: u16) -> u16 {
    // 80 is the highest MIDI note we expect, it could be higher but it's not super important
    // we just want to verify that when higher notes are played the PWM signal goes higher
    (note as u16 * pwm_max) / 80
}

// `midi_pb_to_pwm(pb, m)` is the MIDI pitch bend in `[-1.0, 1.0]` expanded to fill up the PMW range
fn midi_pb_to_pwm(pitch_bend: f32, pwm_max: u16) -> u16 {
    ((pitch_bend / 2. + 0.5) * pwm_max as f32) as u16
}
