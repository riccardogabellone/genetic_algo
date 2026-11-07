//! Live charts. Each measure has its own card and axis, all sharing the tick axis.

use std::sync::Arc;

use eframe::egui::{self, Id};
use egui_plot::{Corner, HLine, Legend, Line, LineStyle, Plot, PlotPoint, PlotPoints};
use genetic_algo::{config::MAX_SMARTNESS, stats::TickStats};

use crate::{theme, widgets::card};

/// A chart never needs more points than this; long runs are thinned to it.
const MAX_POINTS: usize = 1_200;
const PLOT_GROUP: &str = "tick_axis";
/// A card's title, margins and outline around its chart.
const CARD_CHROME: f32 = 48.0;
const CARD_GAP: f32 = 8.0;
const MIN_HEIGHT: f32 = 140.0;

/// Evenly spaced entries, about `max_points` of them, that always include the latest one,
/// so a chart ends at "now".
pub fn thin(history: &[TickStats], max_points: usize) -> Vec<&TickStats> {
    let stride = (history.len() / max_points).max(1);
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

/// Which optional parts of the charts are worth showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panels {
    pub speed: bool,
    pub innate: bool,
    pub policy: bool,
}

pub fn show(ui: &mut egui::Ui, history: &[TickStats], target: Option<f32>, panels: Panels) {
    let palette = theme::palette(ui.ctx());
    let points = thin(history, MAX_POINTS);
    let count = 2 + usize::from(panels.speed) + usize::from(panels.policy);
    // Charts share what is left of the panel's height, but never get shorter than
    // MIN_HEIGHT: below that the panel scrolls instead.
    let gaps = (count - 1) as f32 * (CARD_GAP + ui.spacing().item_spacing.y);
    let height = ((ui.available_height() - count as f32 * CARD_CHROME - gaps) / count as f32)
        .max(MIN_HEIGHT);
    let gap = |ui: &mut egui::Ui| ui.add_space(CARD_GAP);

    card(ui, "Smartness", None, |ui| {
        base_plot("smartness_plot", height)
            .include_y(f64::from(MAX_SMARTNESS))
            .legend(Legend::default().position(Corner::RightBottom))
            .show(ui, |plot| {
                plot.line(
                    series("Max", &points, |s| s.smartness.map(|v| f64::from(v.max)))
                        .color(palette.dim)
                        .width(1.0),
                );
                plot.line(
                    series("Min", &points, |s| s.smartness.map(|v| f64::from(v.min)))
                        .color(palette.dim)
                        .width(1.0)
                        .style(LineStyle::dotted_dense()),
                );
                if panels.innate {
                    plot.line(
                        series("Innate mean", &points, |s| {
                            s.smartness.map(|v| f64::from(v.innate_mean))
                        })
                        .color(palette.secondary)
                        .width(1.5),
                    );
                }
                // The line takes the colour of the smartness it shows, along the logo's
                // gradient.
                plot.line(
                    series("Mean", &points, |s| s.smartness.map(|v| f64::from(v.mean)))
                        .color(palette.accent)
                        .width(2.5)
                        .gradient_color(
                            Arc::new(|point: PlotPoint| palette.smartness_color(point.y as f32)),
                            false,
                        ),
                );
                if let Some(target) = target {
                    plot.hline(
                        HLine::new("Target", f64::from(target))
                            .color(palette.secondary)
                            .style(LineStyle::dashed_loose()),
                    );
                }
            });
    });
    gap(ui);

    card(ui, "Population", None, |ui| {
        base_plot("population_plot", height).show(ui, |plot| {
            plot.line(
                series("Population", &points, |s| Some(s.population as f64))
                    .color(palette.accent)
                    .width(2.0)
                    .fill(0.0)
                    .fill_alpha(0.12),
            );
        });
    });

    if panels.policy {
        gap(ui);
        card(ui, "Mean policy", None, |ui| {
            base_plot("policy_plot", height)
                .include_y(1.0)
                .legend(Legend::default().position(Corner::RightBottom))
                .show(ui, |plot| {
                    plot.line(
                        series("Aggression", &points, |s| {
                            s.mean_policy.map(|policy| f64::from(policy.aggression))
                        })
                        .color(palette.danger)
                        .width(2.0),
                    );
                    plot.line(
                        series("Appetite", &points, |s| {
                            s.mean_policy.map(|policy| f64::from(policy.appetite))
                        })
                        .color(palette.smartness[1])
                        .width(2.0),
                    );
                });
        });
    }

    if panels.speed {
        gap(ui);
        card(ui, "Mean speed", None, |ui| {
            base_plot("speed_plot", height)
                .include_y(1.0)
                .show(ui, |plot| {
                    plot.line(
                        series("Mean speed", &points, |s| s.mean_speed.map(f64::from))
                            .color(palette.accent)
                            .width(2.0),
                    );
                });
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
        assert_eq!(thin(&history, MAX_POINTS).len(), 100);
    }

    #[test]
    fn long_histories_are_thinned_and_end_at_the_latest_tick() {
        for length in [1_201, 5_000, 123_457, 1_000_000] {
            let history = history(length);
            let points = thin(&history, MAX_POINTS);
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
        assert!(thin(&[], MAX_POINTS).is_empty());
    }
}
