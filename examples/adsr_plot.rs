//! Plot out 1 second of ADSR action
//!
//! Look in /images/ for the resulting plot.
//!
//! Requires plotters lib: https://docs.rs/plotters/latest/plotters/. Tested on an Ubuntu machine.

use plotters::prelude::*;
use synth_utils::adsr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 1_000.0_f32;

    let mut adsr = adsr::Adsr::new(sample_rate);
    // adjust these inputs to taste to see the result
    adsr.set_input(adsr::Input::Attack(0.15_f32.into())); // seconds
    adsr.set_input(adsr::Input::Decay(0.3_f32.into())); // seconds
    adsr.set_input(adsr::Input::Sustain(0.5_f32.into())); // in [0.0, 1.0]
    adsr.set_input(adsr::Input::Release(0.3_f32.into())); // seconds

    // plot 1 second of the ADSR
    let num_points = sample_rate as u32;
    // wait 100mSec to turn the gate on, and then at time 700mSec turn it back off
    let gate_on_sample = 100;
    let gate_off_sample = 700;

    let root = BitMapBackend::new("images/adsr_example_plot_0.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("ADSR", ("Arial", 20).into_font())
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0f32..1f32, 0f32..1f32)?;

    chart
        .configure_mesh()
        .x_desc("Time")
        .y_desc("Amplitude")
        .draw()?;

    // plot out the gate signal in blue
    chart
        .draw_series(LineSeries::new(
            (1..num_points).map(|x| {
                // convert boolean gate signal into [0.0, 1.0]
                let y = (gate_on_sample <= x && x < gate_off_sample) as u32 as f32;
                (x as f32 / num_points as f32, y)
            }),
            BLUE,
        ))?
        .label("Gate input")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    // plot out the ADSR output in red
    chart
        .draw_series(LineSeries::new(
            (1..num_points).map(|x| {
                if x == gate_on_sample {
                    adsr.gate_on();
                }
                if x == gate_off_sample {
                    adsr.gate_off();
                }

                let y = adsr.value();
                adsr.tick();

                (x as f32 / num_points as f32, y)
            }),
            RED,
        ))?
        .label("ADSR output")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}
