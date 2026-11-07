//! A card over the map describing the individual the user clicked. It follows that
//! individual by id as it moves, and says so when it dies.

use eframe::egui::{self, Align2, RichText, Vec2};
use egui_material_icons::icons::ICON_CLOSE;
use genetic_algo::world::{Cell, World};

use crate::{
    map_view, theme,
    widgets::{self, Emphasis},
};

const WIDTH: f32 = 250.0;
const MARGIN: f32 = 12.0;

/// Draws the card in the top-right corner of `map_area`. Closing it clears `selected`.
pub fn show(ui: &egui::Ui, map_area: egui::Rect, world: &World, selected: &mut Option<u64>) {
    let Some(id) = *selected else {
        return;
    };
    let found = world.iter().find_map(|(position, cell)| match cell {
        Cell::Individual(individual) if individual.id == id => Some((position, individual)),
        _ => None,
    });
    let palette = theme::palette(ui.ctx());
    let mut close = false;
    egui::Area::new(egui::Id::unique("inspector"))
        .pivot(Align2::RIGHT_TOP)
        .fixed_pos(map_area.right_top() + Vec2::new(-MARGIN, MARGIN))
        .order(egui::Order::Foreground)
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_width(WIDTH);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Inspector").small().color(palette.secondary));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        close = widgets::icon_button(ui, ICON_CLOSE, "Close", Emphasis::Normal)
                            .clicked();
                    });
                });
                match found {
                    Some((position, individual)) => {
                        let place = format!("({}, {})", position.x, position.y);
                        map_view::describe_individual(ui, &place, individual);
                    }
                    None => {
                        ui.label(format!("#{id} has died."));
                        ui.weak("Click another individual on the map.");
                    }
                }
            });
        });
    if close {
        *selected = None;
    }
}
