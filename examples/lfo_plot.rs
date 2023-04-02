//! Plot out some LFO waveforms
//!
//! Look in /images/ for the resulting plot.
//!
//! Requires plotters lib: https://docs.rs/plotters/latest/plotters/. Tested on an Ubuntu machine.

use plotters::prelude::*;
use synth_utils::lfo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 1_000.0_f32;

    let mut lfo = lfo::Lfo::new(sample_rate);
    lfo.set_frequency(2.0_f32);

    // plot 1 second
    let num_points = sample_rate as u32;

    let root = BitMapBackend::new("images/lfo_example_plot_0.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let root = root.titled("LFO Waveshapes", ("sans-serif", 40))?;

    let sub_areas = root.split_evenly((2, 2));

    // plot out some waveshapes, down-saw left out because it is just negated up-saw
    let details = [
        (lfo::Waveshape::Sine, "Sine"),
        (lfo::Waveshape::Triangle, "Triangle"),
        (lfo::Waveshape::UpSaw, "Saw"),
        (lfo::Waveshape::Square, "Square"),
    ];

    for (idx, area) in (0..).zip(sub_areas.iter()) {
        let mut chart = ChartBuilder::on(area)
            .caption(details[idx].1, ("sans-serif", 15).into_font())
            .x_label_area_size(40)
            .y_label_area_size(40)
            .build_cartesian_2d(0f32..1f32, -1.25f32..1.25f32)?;

        chart
            .configure_mesh()
            .x_desc("Time")
            .y_desc("Amplitude")
            .draw()?;

        // plot out the LFO output in red
        chart.draw_series(LineSeries::new(
            (1..num_points).map(|x| {
                let y = lfo.get(details[idx].0);
                lfo.tick();

                (x as f32 / num_points as f32, y)
            }),
            RED,
        ))?;
    }

    root.present()?;

    Ok(())
}
