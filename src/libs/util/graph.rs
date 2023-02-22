// Graph plotting from Minoru Tomobe's RustFT8 at
// https://github.com/w-ockham/RustFT8

use plotters::prelude::*;

pub fn plot_graph(
    path: &str,
    caption: &str,
    plots: &[f32],
    x_min: usize,
    x_max: usize,
    y_min: f32,
    y_max: f32,
) {
    let root = BitMapBackend::new(path, (1024, 1000)).into_drawing_area();

    root.fill(&WHITE).unwrap();

    let font = ("sans-serif", 20);

    let mut chart = ChartBuilder::on(&root)
        .caption(caption, font.into_font())
        .margin(10)
        .x_label_area_size(20)
        .y_label_area_size(20)
        .build_cartesian_2d(x_min..x_max, y_min..y_max) // x軸とy軸の数値の範囲を指定する
        .unwrap();

    chart.configure_mesh().draw().unwrap();
    let line_series = LineSeries::new((0..).zip(plots.iter()).map(|(idx, y)| (idx, *y)), &RED);
    chart.draw_series(line_series).unwrap();
}
