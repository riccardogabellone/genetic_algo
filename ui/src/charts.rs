//! Live charts. Each measure has its own panel and axis, all sharing the tick axis.

use eframe::egui::{self, Color32, Id};
use egui_plot::{Corner, HLine, Legend, Line, LineStyle, Plot, PlotPoints};
use genetic_algo::{config::MAX_SMARTNESS, stats::TickStats};

/// A chart never needs more points than this; long runs are thinned to it.
const MAX_POINTS: usize = 1_200;
const SERIES: Color32 = Color32::from_rgb(196, 134, 30);
const SERIES_RANGE: Color32 = Color32::from_rgb(138, 110, 58);
const TARGET: Color32 = Color32::from_rgb(190, 192, 198);
const PLOT_GROUP: &str = "tick_axis";

/// Evenly spaced entries that always include the latest one, so the chart ends at "now".
fn thin(history: &[TickStats]) -> Vec<&TickStats> {
    let stride = (history.len() / MAX_POINTS).max(1);
    let mut points: Vec<&TickStats> = history.iter().step_by(stride).collect();
    if let Some(last) = history.last()
        && stride > 1
        && !std::ptr::eq(*points.last().expect("not empty"), last)
    {
        points.push(last);
    }
    points
}

fn series<'a>(
    name: &str,
    points: &[&TickStats],
    value: impl Fn(&TickStats) -> Option<f64>,
) -> Line<'a> {
    let values: Vec<[f64; 2]> = points
        .iter()
        .filter_map(|stats| value(stats).map(|value| [f64::from(stats.tick), value]))
        .collect();
    Line::new(name.to_owned(), PlotPoints::from(values))
}

fn base_plot(id: &str, height: f32) -> Plot<'static> {
    Plot::new(id.to_owned())
        .height(height)
        .link_axis(Id::unique(PLOT_GROUP), [true, false])
        .allow_drag(false)
        .allow_zoom(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .include_y(0.0)
}

pub fn show(
    ui: &mut egui::Ui,
    history: &[TickStats],
    target: Option<f32>,
    show_speed: bool,
    show_innate: bool,
) {
    let points = thin(history);
    let panels = if show_speed { 3.0 } else { 2.0 };
    let headings = panels * 26.0;
    let height = ((ui.available_height() - headings) / panels).max(70.0);

    ui.strong("Smartness");
    base_plot("smartness_plot", height)
        .include_y(f64::from(MAX_SMARTNESS))
        .legend(Legend::default().position(Corner::LeftTop))
        .show(ui, |plot| {
            plot.line(
                series("Max", &points, |s| s.smartness.map(|v| f64::from(v.max)))
                    .color(SERIES_RANGE)
                    .width(1.0),
            );
            plot.line(
                series("Min", &points, |s| s.smartness.map(|v| f64::from(v.min)))
                    .color(SERIES_RANGE)
                    .width(1.0)
                    .style(LineStyle::dotted_dense()),
            );
            plot.line(
                series("Mean", &points, |s| s.smartness.map(|v| f64::from(v.mean)))
                    .color(SERIES)
                    .width(2.0),
            );
            if show_innate {
                plot.line(
                    series("Innate mean", &points, |s| {
                        s.smartness.map(|v| f64::from(v.innate_mean))
                    })
                    .color(TARGET)
                    .width(1.5),
                );
            }
            if let Some(target) = target {
                plot.hline(
                    HLine::new("Target", f64::from(target))
                        .color(TARGET)
                        .style(LineStyle::dashed_loose()),
                );
            }
        });

    ui.strong("Population");
    base_plot("population_plot", height).show(ui, |plot| {
        plot.line(
            series("Population", &points, |s| Some(s.population as f64))
                .color(SERIES)
                .width(2.0),
        );
    });

    if show_speed {
        ui.strong("Mean speed");
        base_plot("speed_plot", height)
            .include_y(1.0)
            .show(ui, |plot| {
                plot.line(
                    series("Mean speed", &points, |s| s.mean_speed.map(f64::from))
                        .color(SERIES)
                        .width(2.0),
                );
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genetic_algo::{stats::TickEvents, world::World};

    fn history(length: u32) -> Vec<TickStats> {
        let world = World::new(1, 1);
        (0..length)
            .map(|tick| TickStats::measure(tick, &world, TickEvents::default()))
            .collect()
    }

    #[test]
    fn short_histories_are_kept_whole() {
        let history = history(100);
        assert_eq!(thin(&history).len(), 100);
    }

    #[test]
    fn long_histories_are_thinned_and_end_at_the_latest_tick() {
        for length in [1_201, 5_000, 123_457, 1_000_000] {
            let history = history(length);
            let points = thin(&history);
            assert!(
                points.len() <= MAX_POINTS * 2 + 1,
                "{length}: {}",
                points.len()
            );
            assert_eq!(points.first().unwrap().tick, 0);
            assert_eq!(points.last().unwrap().tick, length - 1);
        }
    }

    #[test]
    fn empty_history_is_harmless() {
        assert!(thin(&[]).is_empty());
    }
}
