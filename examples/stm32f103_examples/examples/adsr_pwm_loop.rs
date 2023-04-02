//! Continuously trigger an ADSR and write the value via PWM on pin PA6
//!
//! To flash:
//! $ cargo flash --example adsr_pwm_loop --chip stm32f103rb --release

#![deny(unsafe_code)]
#![no_std]
#![no_main]

use panic_halt as _;

use nb::block;

use cortex_m;
use cortex_m_rt::entry;
use stm32f1xx_hal::{pac, prelude::*, timer::Timer};
use synth_utils::adsr;

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
    // Create the ADSR object
    //
    ////////////////////////////////////////////////////////////////////////

    let mut adsr = adsr::Adsr::new(1_000.0_f32);

    // some reasonable settings, adjust to taste
    adsr.set_input(adsr::Input::Attack(0.15_f32.into())); // seconds
    adsr.set_input(adsr::Input::Decay(0.3_f32.into())); // seconds
    adsr.set_input(adsr::Input::Sustain(0.5_f32.into())); // in [0.0, 1.0]
    adsr.set_input(adsr::Input::Release(0.3_f32.into())); // seconds

    // counter so we can trigger the ADSR on/off when we want to
    let mut counter = 0;

    loop {
        // trigger the ADSR to turn on and off once per second
        if counter == 0 {
            adsr.gate_on();
        }
        if counter == 500 {
            adsr.gate_off();
        }
        counter += 1;
        if 1000 < counter {
            counter = 0;
        }

        // write the ADSR value via PWM, verify with oscilloscope or LED
        let adsr_as_pwm = (adsr.get_value() * pwm_max as f32) as u16;
        pwm_ch.set_duty(adsr_as_pwm);
        adsr.tick();

        block!(timer.wait()).unwrap();
    }
}
