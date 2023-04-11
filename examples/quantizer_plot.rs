//! Plot out the quantizer converting smooth inputs into stairsteps
//!
//! Look in /images/ for the resulting plot.
//!
//! Requires plotters lib: https://docs.rs/plotters/latest/plotters/. Tested on an Ubuntu machine.

use plotters::prelude::*;
use synth_utils::quantizer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const SAMPLE_RATE: f32 = 10_000.0_f32;

    let mut quantizer = quantizer::Quantizer::new();
    quantizer.forbid(&[
        quantizer::Note::CSHARP,
        quantizer::Note::DSHARP,
        quantizer::Note::FSHARP,
        quantizer::Note::GSHARP,
        quantizer::Note::ASHARP,
    ]);

    const NUM_SECS_TO_PLOT: usize = 1;
    const NUM_POINTS: usize = SAMPLE_RATE as usize * NUM_SECS_TO_PLOT;

    // two octaves + 1/2 step
    let max_vin = 25. / 12.;

    let root =
        BitMapBackend::new("images/quantizer_example_plot_0.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let root = root.titled("Quantizer", ("sans-serif", 40))?;

    let sub_areas = root.split_evenly((2, 2));

    let all_notes = [
        quantizer::Note::C,
        quantizer::Note::CSHARP,
        quantizer::Note::DSHARP,
        quantizer::Note::E,
        quantizer::Note::F,
        quantizer::Note::FSHARP,
        quantizer::Note::G,
        quantizer::Note::GSHARP,
        quantizer::Note::A,
        quantizer::Note::ASHARP,
        quantizer::Note::B,
    ];

    // the name and fobidden notes for each scale to plot out
    let details = [
        ("Chromatic", Vec::from([])),
        (
            "Major scale",
            Vec::from([
                quantizer::Note::CSHARP,
                quantizer::Note::DSHARP,
                quantizer::Note::FSHARP,
                quantizer::Note::GSHARP,
                quantizer::Note::ASHARP,
            ]),
        ),
        (
            "Roots and fifths",
            Vec::from([
                // quantizer::Note::C,
                quantizer::Note::CSHARP,
                quantizer::Note::D,
                quantizer::Note::DSHARP,
                quantizer::Note::E,
                quantizer::Note::F,
                quantizer::Note::FSHARP,
                // quantizer::Note::G,
                quantizer::Note::GSHARP,
                quantizer::Note::A,
                quantizer::Note::ASHARP,
                quantizer::Note::B,
            ]),
        ),
        (
            "Octaves only",
            Vec::from([
                // quantizer::Note::C,
                quantizer::Note::CSHARP,
                quantizer::Note::D,
                quantizer::Note::DSHARP,
                quantizer::Note::E,
                quantizer::Note::F,
                quantizer::Note::FSHARP,
                quantizer::Note::G,
                quantizer::Note::GSHARP,
                quantizer::Note::A,
                quantizer::Note::ASHARP,
                quantizer::Note::B,
            ]),
        ),
    ];

    for (idx, area) in (0..).zip(sub_areas.iter()) {
        quantizer.allow(&all_notes);
        details[idx].1.iter().for_each(|n| quantizer.forbid(&[*n]));

        let mut chart = ChartBuilder::on(area)
            .caption(details[idx].0, ("Arial", 15).into_font())
            .x_label_area_size(40)
            .y_label_area_size(40)
            .build_cartesian_2d(0f32..NUM_SECS_TO_PLOT as f32, 0f32..max_vin + 0.1)?;

        chart
            .configure_mesh()
            .x_desc("Time")
            .y_desc("Amplitude (volts)")
            .draw()?;

        // plot the input
        chart
            .draw_series(LineSeries::new(
                (1..NUM_POINTS).map(|x| {
                    let x = x as f32 / NUM_POINTS as f32;
                    let y = x * max_vin;
                    (x, y)
                }),
                BLUE,
            ))?
            .label("Raw input")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

        // plot the quantized output
        chart
            .draw_series(LineSeries::new(
                (1..NUM_POINTS).map(|x| {
                    let x = x as f32 / NUM_POINTS as f32;
                    let y = quantizer.convert(x * max_vin);

                    (x, y.stairstep)
                }),
                RED,
            ))?
            .label("Quantized output")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::LowerRight)
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .draw()?;
    }

    root.present()?;

    Ok(())
}
