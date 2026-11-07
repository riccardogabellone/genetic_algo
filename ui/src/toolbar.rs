//! The bar across the top: who the app is, the run controls and the live readouts.

use eframe::egui::{self, RichText, Slider, TextureHandle, ThemePreference};
use egui_material_icons::icons::{
    ICON_BRIGHTNESS_AUTO, ICON_DARK_MODE, ICON_LIGHT_MODE, ICON_PAUSE, ICON_PLAY_ARROW,
    ICON_SKIP_NEXT,
};

use crate::{
    controller::Controller,
    identity::APP_NAME,
    widgets::{self, Emphasis},
};

const LOGO_SIZE: f32 = 22.0;

pub fn show(ui: &mut egui::Ui, controller: &mut Controller, logo: &TextureHandle) {
    ui.horizontal_centered(|ui| {
        ui.add(egui::Image::new((logo.id(), egui::Vec2::splat(LOGO_SIZE))));
        ui.label(RichText::new(APP_NAME).heading());
        ui.separator();
        run_controls(ui, controller);
        ui.separator();
        readouts(ui, controller);
        ui.with_layout(
            egui::Layout::right_to_left(egui::Align::Center),
            theme_toggle,
        );
    });
}

fn run_controls(ui: &mut egui::Ui, controller: &mut Controller) {
    let can_run = controller.simulation().outcome().is_none();
    ui.add_enabled_ui(can_run, |ui| {
        let (icon, label) = if controller.is_running() {
            (ICON_PAUSE, "Pause")
        } else {
            (ICON_PLAY_ARROW, "Play")
        };
        if widgets::icon_button(ui, icon, label, Emphasis::Primary)
            .on_hover_text("Space")
            .clicked()
        {
            controller.toggle_running();
        }
        if widgets::icon_button(ui, ICON_SKIP_NEXT, "Step", Emphasis::Normal).clicked() {
            controller.step_once();
        }
    });
    ui.add(
        Slider::new(&mut controller.ticks_per_second, 1.0..=20_000.0)
            .logarithmic(true)
            .integer()
            .text("ticks/s"),
    );
}

fn readouts(ui: &mut egui::Ui, controller: &Controller) {
    let latest = controller.simulation().latest();
    let smartness = latest
        .smartness
        .map_or_else(|| "n/a".to_owned(), |s| format!("{:.1}", s.mean));
    for (caption, value) in [
        ("Tick", latest.tick.to_string()),
        ("Generation", latest.max_generation.to_string()),
        ("Population", latest.population.to_string()),
        ("Mean smartness", smartness),
    ] {
        widgets::readout(ui, caption, &value);
        ui.add_space(6.0);
    }
}

/// Cycles follow the system, light, dark.
fn theme_toggle(ui: &mut egui::Ui) {
    let preference = ui.ctx().options(|options| options.theme_preference);
    let (icon, name, next) = match preference {
        ThemePreference::System => (
            ICON_BRIGHTNESS_AUTO,
            "Theme: follow the system",
            ThemePreference::Light,
        ),
        ThemePreference::Light => (ICON_LIGHT_MODE, "Theme: light", ThemePreference::Dark),
        ThemePreference::Dark => (ICON_DARK_MODE, "Theme: dark", ThemePreference::System),
    };
    if widgets::icon_button(ui, icon, name, Emphasis::Normal).clicked() {
        ui.ctx().set_theme(next);
    }
}
