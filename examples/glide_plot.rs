//! Plot out the glide controller creating some portamento
//!
//! Look in /images/ for the resulting plot.
//!
//! Requires plotters lib: https://docs.rs/plotters/latest/plotters/. Tested on an Ubuntu machine.

use plotters::prelude::*;
use synth_utils::glide_processor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const SAMPLE_RATE: f32 = 10_000.0_f32;

    let mut glide = glide_processor::GlideProcessor::new(SAMPLE_RATE);
    // inial glide time set to 1 second
    glide.set_time(1.0);

    const NUM_SECS_TO_PLOT: usize = 4;
    const NUM_POINTS: usize = SAMPLE_RATE as usize * NUM_SECS_TO_PLOT;

    // the input is stepped, the output will glide between the steps
    let mut mock_input: [f32; NUM_POINTS] = [0.0; NUM_POINTS];
    mock_input[2500..10000].iter_mut().for_each(|x| {
        *x = 0.5;
    });
    mock_input[10000..20000].iter_mut().for_each(|x| {
        *x = 0.75;
    });
    mock_input[20000..30000].iter_mut().for_each(|x| {
        *x = 0.2;
    });
    mock_input[30000..40000].iter_mut().for_each(|x| {
        *x = 1.0;
    });

    let root =
        BitMapBackend::new("images/glide_example_plot_0.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Glide Processor", ("Arial", 20).into_font())
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0f32..NUM_SECS_TO_PLOT as f32, 0f32..1f32)?;

    chart
        .configure_mesh()
        .x_desc("Time")
        .y_desc("Amplitude")
        .draw()?;

    // plot out the input
    chart
        .draw_series(LineSeries::new(
            mock_input
                .iter()
                .enumerate()
                .map(|(i, x)| (i as f32 / SAMPLE_RATE as f32, *x)),
            BLUE,
        ))?
        .label("Raw input")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    // plot out the lagged version of the input
    chart
        .draw_series(LineSeries::new(
            mock_input.iter().enumerate().map(|(i, x)| {
                // change the glide time to see that it speeds up in the plotted output
                if i == 20000 {
                    glide.set_time(0.5);
                }
                if i == 30000 {
                    glide.set_time(0.0);
                }

                let y = glide.process(*x);
                (i as f32 / SAMPLE_RATE as f32, y)
            }),
            RED,
        ))?
        .label("Glide output")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::MiddleRight)
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}
