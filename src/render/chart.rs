//! Writes the end-of-run chart: smartness on top, population below.
//!
//! Two measures with different scales get two panels, never a shared dual axis.

use std::path::Path;

use plotters::prelude::*;

use crate::{config::MAX_SMARTNESS, stats::TickStats};

const SIZE: (u32, u32) = (1000, 700);
const SERIES_COLOR: RGBColor = RGBColor(32, 112, 200);
const TARGET_COLOR: RGBColor = RGBColor(40, 40, 40);
const INK: RGBColor = RGBColor(70, 74, 82);
const GRID: RGBColor = RGBColor(232, 233, 236);

type DrawResult = Result<(), Box<dyn std::error::Error>>;

/// `history` must contain at least tick 0.
pub fn write_chart(path: &Path, history: &[TickStats], target: f32) -> DrawResult {
    let root = BitMapBackend::new(path, SIZE).into_drawing_area();
    root.fill(&WHITE)?;
    let (top, bottom) = root.split_vertically(SIZE.1 * 62 / 100);
    let last_tick = history.last().map_or(1, |stats| stats.tick.max(1));

    draw_smartness(&top, history, last_tick, target)?;
    draw_population(&bottom, history, last_tick)?;
    root.present()?;
    Ok(())
}

fn draw_smartness(
    area: &DrawingArea<BitMapBackend<'_>, plotters::coord::Shift>,
    history: &[TickStats],
    last_tick: u32,
    target: f32,
) -> DrawResult {
    let mut chart = ChartBuilder::on(area)
        .caption(
            "Smartness per tick",
            ("sans-serif", 22).into_font().color(&INK),
        )
        .margin(14)
        .x_label_area_size(28)
        .y_label_area_size(48)
        .build_cartesian_2d(0..last_tick, 0_f32..MAX_SMARTNESS)?;
    chart
        .configure_mesh()
        .light_line_style(TRANSPARENT)
        .bold_line_style(GRID)
        .axis_style(INK)
        .label_style(("sans-serif", 13).into_font().color(&INK))
        .y_desc("Smartness (0-100)")
        .draw()?;

    let measured: Vec<_> = history
        .iter()
        .filter_map(|stats| stats.smartness.map(|smartness| (stats.tick, smartness)))
        .collect();
    let range_outline: Vec<(u32, f32)> = measured
        .iter()
        .map(|(tick, smartness)| (*tick, smartness.max))
        .chain(
            measured
                .iter()
                .rev()
                .map(|(tick, smartness)| (*tick, smartness.min)),
        )
        .collect();
    chart
        .draw_series(std::iter::once(Polygon::new(
            range_outline,
            SERIES_COLOR.mix(0.18).filled(),
        )))?
        .label("Min-max range")
        .legend(|(x, y)| {
            Rectangle::new(
                [(x, y - 5), (x + 20, y + 5)],
                SERIES_COLOR.mix(0.18).filled(),
            )
        });
    chart
        .draw_series(LineSeries::new(
            measured
                .iter()
                .map(|(tick, smartness)| (*tick, smartness.mean)),
            SERIES_COLOR.stroke_width(2),
        ))?
        .label("Population mean")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], SERIES_COLOR.stroke_width(2)));
    chart
        .draw_series(DashedLineSeries::new(
            [(0, target), (last_tick, target)],
            6,
            5,
            TARGET_COLOR.into(),
        ))?
        .label(format!("Target ({target:.0})"))
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], TARGET_COLOR));
    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperLeft)
        .background_style(WHITE.mix(0.9))
        .border_style(GRID)
        .label_font(("sans-serif", 13).into_font().color(&INK))
        .draw()?;
    Ok(())
}

fn draw_population(
    area: &DrawingArea<BitMapBackend<'_>, plotters::coord::Shift>,
    history: &[TickStats],
    last_tick: u32,
) -> DrawResult {
    let peak = history
        .iter()
        .map(|stats| stats.population)
        .max()
        .unwrap_or(0)
        .max(1);
    let mut chart = ChartBuilder::on(area)
        .caption(
            "Population per tick",
            ("sans-serif", 22).into_font().color(&INK),
        )
        .margin(14)
        .x_label_area_size(36)
        .y_label_area_size(48)
        .build_cartesian_2d(0..last_tick, 0..peak + peak / 10 + 1)?;
    chart
        .configure_mesh()
        .light_line_style(TRANSPARENT)
        .bold_line_style(GRID)
        .axis_style(INK)
        .label_style(("sans-serif", 13).into_font().color(&INK))
        .x_desc("Tick")
        .y_desc("Individuals")
        .draw()?;
    chart.draw_series(LineSeries::new(
        history.iter().map(|stats| (stats.tick, stats.population)),
        SERIES_COLOR.stroke_width(2),
    ))?;
    Ok(())
}
