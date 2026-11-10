// Release builds on Windows open no console window next to the app.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use genetic_algo_ui::{
    app::App,
    identity::{APP_ID, APP_NAME, window_icon},
    renderer::native_options,
};

fn main() -> eframe::Result {
    let options = native_options(
        egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id(APP_ID)
            .with_icon(window_icon())
            .with_inner_size([1360.0, 820.0])
            .with_min_inner_size([900.0, 560.0]),
    );
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|creation| Ok(Box::new(App::new(&creation.egui_ctx)))),
    )
}
