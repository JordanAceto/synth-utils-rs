//! Write the waveforms of an LFO via PWM on pin PA6
//!
//! To flash:
//! $ cargo flash --example lfo_pwm_loop --chip stm32f103rb --release

#![deny(unsafe_code)]
#![no_std]
#![no_main]

use panic_halt as _;

use nb::block;

use cortex_m;
use cortex_m_rt::entry;
use stm32f1xx_hal::{pac, prelude::*, timer::Timer};
use synth_utils::lfo;

#[entry]
fn main() -> ! {
    ////////////////////////////////////////////////////////////////////////
    //
    // general stm32 peripheral housekeeping
    //
    ////////////////////////////////////////////////////////////////////////
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut afio = dp.AFIO.constrain();
    let mut gpioa = dp.GPIOA.split();
    let mut gpioc = dp.GPIOC.split();

    // timer to set the main sample rate for the ADSR
    let mut timer = Timer::syst(cp.SYST, &clocks).counter_hz();
    timer.start(1_000.Hz()).unwrap();

    // pins and timer for PWM generation
    let pwm_pin = gpioa.pa6.into_alternate_push_pull(&mut gpioa.crl);
    let pwm = Timer::new(dp.TIM3, &clocks).pwm_hz(pwm_pin, &mut afio.mapr, 10.kHz());
    let pwm_max = pwm.get_max_duty();
    let mut pwm_ch = pwm.split();
    pwm_ch.set_duty(pwm_max / 2);
    pwm_ch.enable();

    ////////////////////////////////////////////////////////////////////////
    //
    // Create the LFO object
    //
    ////////////////////////////////////////////////////////////////////////

    let mut lfo = lfo::Lfo::new(1_000.0_f32);
    lfo.set_frequency(1.0_f32);

    // counter so we can change waveshapes once in a while
    let mut counter = 0;
    let mut shape = lfo::Waveshape::Sine;

    loop {
        // write the LFO value via PWM, verify with oscilloscope or LED
        let lfo_as_pwm = scale_lfo_for_pwm(lfo.get(shape), pwm_max);
        pwm_ch.set_duty(lfo_as_pwm);
        lfo.tick();

        counter += 1;
        if counter == 3000 {
            shape = next_shape(shape);
            counter = 0;
        }

        block!(timer.wait()).unwrap();
    }
}

// scale the `[-1.0, +1.0]` lfo value to be in `[0..pwm_max]`
fn scale_lfo_for_pwm(lfo: f32, pwm_max: u16) -> u16 {
    // scale the LFO into [0.0, 1.0]
    let lfo = (lfo / 2.0) + 0.5;
    (lfo * pwm_max as f32) as u16
}

// helper func to cycle through waveshapes in circular fashion
fn next_shape(waveshape: lfo::Waveshape) -> lfo::Waveshape {
    match waveshape {
        lfo::Waveshape::Sine => lfo::Waveshape::Triangle,
        lfo::Waveshape::Triangle => lfo::Waveshape::UpSaw,
        lfo::Waveshape::UpSaw => lfo::Waveshape::DownSaw,
        lfo::Waveshape::DownSaw => lfo::Waveshape::Square,
        lfo::Waveshape::Square => lfo::Waveshape::Sine,
    }
}
