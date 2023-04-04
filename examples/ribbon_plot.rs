//! Plot out a simulated wiggle on the ribbon controller
//!
//! Look in /images/ for the resulting plot.
//!
//! Requires plotters lib: https://docs.rs/plotters/latest/plotters/. Tested on an Ubuntu machine.

use plotters::prelude::*;
use synth_utils::ribbon_controller;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const SAMPLE_RATE: f32 = 10_000.0_f32;

    const RIBBON_BUFF_CAPACITY: usize =
        ribbon_controller::sample_rate_to_capacity(SAMPLE_RATE as u32);

    let mut ribbon = ribbon_controller::RibbonController::<RIBBON_BUFF_CAPACITY>::new(
        SAMPLE_RATE as f32,
        20_000.0, // end-to-end resistance of the softpot, common value for longer softpots. short ones are 10k
        820.0, // resistance of the series resistor going to vref. Value found to work well, feel free to experiment
    );

    const NUM_SECS_TO_PLOT: usize = 4;
    const NUM_POINTS: usize = SAMPLE_RATE as usize * NUM_SECS_TO_PLOT;

    // init the simulated ribbon samples to the out-of-bounds high value 1.0
    let mut mock_adc_signal: [f32; NUM_POINTS] = [1.0; NUM_POINTS];
    // simulate some finger wiggling
    // stairstep down pattern
    mock_adc_signal[1000..5000]
        .iter_mut()
        .for_each(|x| *x = 0.80);
    mock_adc_signal[5000..6000]
        .iter_mut()
        .for_each(|x| *x = 0.60);
    mock_adc_signal[6000..7000]
        .iter_mut()
        .for_each(|x| *x = 0.40);
    mock_adc_signal[7000..8000]
        .iter_mut()
        .for_each(|x| *x = 0.30);
    mock_adc_signal[8000..9000]
        .iter_mut()
        .for_each(|x| *x = 0.25);
    mock_adc_signal[9000..10000]
        .iter_mut()
        .for_each(|x| *x = 0.20);

    // sine simulates vibrato
    mock_adc_signal[11000..21000]
        .iter_mut()
        .enumerate()
        .for_each(|(i, x)| {
            *x = f32::sin(6.238 * (i as f32 / 5000.)) * 0.15 + 0.5;
        });

    // finger slide from high to low
    mock_adc_signal[25000..37000]
        .iter_mut()
        .enumerate()
        .for_each(|(i, x)| {
            *x = ((37000. - (i as f32 + 25000.)) / 15000.) + 0.15;
        });

    let root =
        BitMapBackend::new("images/ribbon_example_plot_0.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Ribbon Controller", ("Arial", 20).into_font())
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0f32..NUM_SECS_TO_PLOT as f32, 0f32..1f32)?;

    chart
        .configure_mesh()
        .x_desc("Time")
        .y_desc("Amplitude")
        .draw()?;

    // plot out the raw ribbon input
    chart
        .draw_series(LineSeries::new(
            mock_adc_signal
                .iter()
                .enumerate()
                .map(|(i, x)| (i as f32 / SAMPLE_RATE as f32, *x)),
            BLACK,
        ))?
        .label("Raw input")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLACK));

    // plot out the ribbon value
    chart
        .draw_series(LineSeries::new(
            mock_adc_signal.iter().enumerate().map(|(i, x)| {
                ribbon.poll(*x);
                (i as f32 / SAMPLE_RATE as f32, ribbon.value())
            }),
            RED,
        ))?
        .label("Ribbon value")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED));

    // plot out the ribbon gate
    chart
        .draw_series(LineSeries::new(
            mock_adc_signal.iter().enumerate().map(|(i, x)| {
                ribbon.poll(*x);
                let y = if ribbon.finger_is_pressing() {
                    // attenuate the gate so we can easily distinguish it on the graph
                    0.10
                } else {
                    0.0
                };

                (i as f32 / SAMPLE_RATE as f32, y)
            }),
            BLUE,
        ))?
        .label("Ribbon gate")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE));

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::MiddleRight)
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}
