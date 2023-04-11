//! Write the ribbon value via PWM on pin PA6 and boolean ribbon gate via PA0
//!
//! Hardware setup:
//! Follow the documentation in the ribbon controller module. You'll need a
//! Spectra Symbol softpot or similar ribbon, an 820 ohm resistor, a 1M resistor,
//! and optionally a 3.3v rail-to-rail opamp (opamp may be left out).
//!
//! To flash:
//! $ cargo flash --example ribbon_pwm_write --chip stm32f103rb --release

#![deny(unsafe_code)]
#![no_std]
#![no_main]

use panic_halt as _;

use nb::block;

use cortex_m;
use cortex_m_rt::entry;
use stm32f1xx_hal::{adc, pac, prelude::*, timer::Timer};
use synth_utils::ribbon_controller;

#[entry]
fn main() -> ! {
    ////////////////////////////////////////////////////////////////////////////
    //
    // general stm32 peripheral housekeeping
    //
    ////////////////////////////////////////////////////////////////////////////
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = pac::Peripherals::take().unwrap();
    let mut flash = dp.FLASH.constrain();
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut afio = dp.AFIO.constrain();
    let mut gpioa = dp.GPIOA.split();
    let mut gpiob = dp.GPIOB.split();

    // adc
    let mut adc1 = adc::Adc::adc1(dp.ADC1, clocks);
    let mut ribbon_pin = gpiob.pb0.into_analog(&mut gpiob.crl);

    // timer to set the main sample rate
    const SAMPLE_RATE: u32 = 5_000;
    let mut timer = Timer::syst(cp.SYST, &clocks).counter_hz();
    timer.start(SAMPLE_RATE.Hz()).unwrap();

    // pins and timer for PWM generation
    let pwm_pin = gpioa.pa6.into_alternate_push_pull(&mut gpioa.crl);
    let pwm = Timer::new(dp.TIM3, &clocks).pwm_hz(pwm_pin, &mut afio.mapr, 10.kHz());
    let pwm_max = pwm.get_max_duty();
    let mut pwm_ch = pwm.split();
    pwm_ch.set_duty(pwm_max / 2);
    pwm_ch.enable();

    // ribbon gate pin, goes high when finger is pressing the ribbon
    let mut gate_pin = gpioa.pa0.into_push_pull_output(&mut gpioa.crl);

    ////////////////////////////////////////////////////////////////////////////
    //
    // Create the ribbon controller object
    //
    ////////////////////////////////////////////////////////////////////////////

    // we calculate the internal buffer capacity like this. I don't love how this works, but I couldn't figure another
    // way yet, suggestions welcome :)
    const RIBBON_BUFF_CAPACITY: usize = ribbon_controller::sample_rate_to_capacity(SAMPLE_RATE);

    let mut ribbon = ribbon_controller::RibbonController::<RIBBON_BUFF_CAPACITY>::new(
        SAMPLE_RATE as f32,
        20_000.0, // end-to-end resistance of the softpot, common value for longer softpots. short ones are 10k
        820.0, // resistance of the series resistor going to vref. Value found to work well, feel free to experiment
        1E6,   // pullup resistor from the wiper to the positive voltage refererence
    );

    loop {
        // sample the ADC and scale it to [0.0, 1.0]
        let adc_read: u16 = adc1.read(&mut ribbon_pin).unwrap();
        let scaled_adc = adc_read as f32 / ((1 << 12) - 1) as f32;

        ribbon.poll(scaled_adc);

        // write the ribbon value via PWM, verify with oscilloscope or LED
        let ribbon_as_pwm = (ribbon.value() as f32 * pwm_max as f32) as u16;
        pwm_ch.set_duty(ribbon_as_pwm);

        if ribbon.finger_is_pressing() {
            gate_pin.set_high();
        } else {
            gate_pin.set_low();
        }

        block!(timer.wait()).unwrap();
    }
}
