//! The right panel: headline tiles with recent trends, what has happened so far, the
//! leadership when it is on, and the charts.

use eframe::egui::{self, Color32, RichText, Vec2};
use genetic_algo::stats::{TickEvents, TickStats};

use crate::{
    charts::{self, Panels},
    controller::Controller,
    theme,
    widgets::{self, card},
};

/// How many recent ticks a tile's trend covers, and how many points it draws.
const TREND_TICKS: usize = 600;
const TREND_POINTS: usize = 120;
const DEATH_BAR_HEIGHT: f32 = 10.0;

pub fn show(ui: &mut egui::Ui, controller: &Controller) {
    let simulation = controller.simulation();
    let config = simulation.config();
    let latest = simulation.latest();
    let totals = controller.totals();
    let panels = Panels {
        speed: controller.rules.max_speed > 1 || latest.mean_speed.is_some_and(|speed| speed > 1.0),
        innate: config.contest_fight_chance > 0.0
            || config.random_fight_chance > 0.0
            || totals.fights > 0,
        policy: config.leaders_enabled,
    };

    tiles(ui, simulation.history(), panels.speed);
    ui.add_space(8.0);
    events(ui, totals, panels.innate);
    if config.leaders_enabled || totals.elections > 0 {
        ui.add_space(8.0);
        leadership(ui, controller);
    }
    ui.add_space(8.0);
    let target = controller
        .rules
        .stop_at_target
        .then_some(controller.rules.target);
    charts::show(ui, simulation.history(), target, panels);
}

/// The last `TREND_TICKS` values of a measure, thinned to about `TREND_POINTS`.
fn trend(history: &[TickStats], value: impl Fn(&TickStats) -> Option<f32>) -> Vec<f32> {
    let recent = &history[history.len().saturating_sub(TREND_TICKS)..];
    let stride = (recent.len() / TREND_POINTS).max(1);
    recent.iter().step_by(stride).filter_map(value).collect()
}

fn tiles(ui: &mut egui::Ui, history: &[TickStats], speed: bool) {
    let palette = theme::palette(ui.ctx());
    let latest = history
        .last()
        .expect("a simulation always has its first tick");
    let count = if speed { 3 } else { 2 };
    ui.columns(count, |columns| {
        let population = trend(history, |stats| Some(stats.population as f32));
        widgets::stat_tile(
            &mut columns[0],
            "Population",
            &latest.population.to_string(),
            &format!("{} M / {} F", latest.males, latest.females),
            |ui| widgets::sparkline(ui, &population, palette.accent, |_| palette.accent),
        );

        let smartness = trend(history, |stats| stats.smartness.map(|s| s.mean));
        let (value, detail) = latest.smartness.map_or_else(
            || ("n/a".to_owned(), "extinct".to_owned()),
            |s| {
                (
                    format!("{:.1}", s.mean),
                    format!("range {:.0}-{:.0}", s.min, s.max),
                )
            },
        );
        widgets::stat_tile(&mut columns[1], "Mean smartness", &value, &detail, |ui| {
            widgets::sparkline(ui, &smartness, palette.accent, |value| {
                palette.smartness_color(value)
            });
        });

        if speed {
            let speeds = trend(history, |stats| stats.mean_speed);
            let value = latest
                .mean_speed
                .map_or_else(|| "n/a".to_owned(), |speed| format!("{speed:.2}"));
            widgets::stat_tile(
                &mut columns[2],
                "Speed",
                &value,
                "mean, cells per move",
                |ui| {
                    widgets::sparkline(ui, &speeds, palette.accent, |_| palette.accent);
                },
            );
        }
    });
}

/// Totals since the run started, with deaths split by cause in one bar.
fn events(ui: &mut egui::Ui, totals: &TickEvents, fights: bool) {
    let palette = theme::palette(ui.ctx());
    card(ui, "Since the start", None, |ui| {
        egui::Grid::new("totals")
            .num_columns(4)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.weak("Births");
                ui.label(totals.births.to_string());
                ui.weak("Meals");
                ui.label(totals.meals.to_string());
                ui.end_row();
                if fights {
                    ui.weak("Fights");
                    ui.label(totals.fights.to_string());
                    ui.end_row();
                }
            });
        ui.add_space(4.0);
        let causes = [
            ("Starved", totals.starved, palette.warning),
            ("Overfed", totals.overfed, palette.smartness[1]),
            ("Old age", totals.old_age, palette.secondary),
            ("Hazard", totals.hazard, palette.danger),
            ("Killed in fights", totals.combat, palette.accent),
        ];
        ui.label(
            RichText::new(format!("Deaths: {}", totals.deaths()))
                .small()
                .color(palette.secondary),
        );
        death_bar(ui, &causes);
        ui.horizontal_wrapped(|ui| {
            for (name, count, color) in causes {
                let (swatch, _) = ui.allocate_exact_size(Vec2::splat(9.0), egui::Sense::hover());
                ui.painter().rect_filled(swatch, 2.0, color);
                ui.label(RichText::new(name).small().color(palette.secondary));
                ui.label(RichText::new(count.to_string()).small());
                ui.add_space(4.0);
            }
        });
    });
}

/// Each cause's share of all deaths, side by side.
fn death_bar(ui: &mut egui::Ui, causes: &[(&str, u32, Color32)]) {
    let palette = theme::palette(ui.ctx());
    let size = Vec2::new(ui.available_width(), DEATH_BAR_HEIGHT);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, palette.surface);
    let total: u32 = causes.iter().map(|(_, count, _)| count).sum();
    if total == 0 {
        return;
    }
    let mut left = rect.left();
    for (_, count, color) in causes {
        let width = rect.width() * *count as f32 / total as f32;
        let segment = egui::Rect::from_min_max(
            egui::pos2(left, rect.top()),
            egui::pos2(left + width, rect.bottom()),
        );
        painter.rect_filled(segment, 0.0, *color);
        left += width;
    }
}

fn leadership(ui: &mut egui::Ui, controller: &Controller) {
    let leadership = controller.simulation().leadership();
    let leader = leadership
        .leader()
        .map_or_else(|| "none".to_owned(), |id| format!("#{id}"));
    let policy = leadership.policy().map_or_else(
        || "n/a".to_owned(),
        |policy| {
            format!(
                "aggression {:.2}, appetite {:.2}",
                policy.aggression, policy.appetite
            )
        },
    );
    card(ui, "Leadership", None, |ui| {
        egui::Grid::new("leadership")
            .num_columns(4)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                ui.weak("Leader");
                ui.label(leader);
                ui.weak("Elections");
                ui.label(controller.totals().elections.to_string());
                ui.end_row();
            });
        ui.horizontal(|ui| {
            ui.weak("Leader's policy");
            ui.label(policy);
        });
    });
}
