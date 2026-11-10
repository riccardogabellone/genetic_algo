//! Opens the real window, runs it for a while, saves a screenshot and exits.
//! A way to check the UI's appearance without clicking through it.
//!
//! Usage: `cargo run -p genetic_algo_ui --example screenshot -- <out.ppm> [frames] [preset] [map_side] [dark|light]`

use std::{fs::File, io::Write as _, path::PathBuf, time::Instant};

use eframe::egui;
use genetic_algo_ui::{app::App, controller::Preset, renderer::native_options};

struct Screenshotter {
    app: App,
    path: PathBuf,
    frames_left: u32,
    requested: bool,
    started: Instant,
    frames: u32,
}

impl eframe::App for Screenshotter {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.show(ui);
        ui.ctx().request_repaint();
        self.frames += 1;

        if self.frames_left == 0 && !self.requested {
            self.requested = true;
            let seconds = self.started.elapsed().as_secs_f32();
            let simulation = self.app.controller.simulation().latest();
            println!(
                "{:.0} fps, {:.0} ticks/s, population {}",
                self.frames as f32 / seconds,
                simulation.tick as f32 / seconds,
                simulation.population
            );
            let path = self.path.clone();
            let context = ui.ctx().clone();
            ui.ctx().request_screenshot(move |image| {
                save_ppm(&path, &image).expect("the screenshot path is writable");
                context.send_viewport_cmd(egui::ViewportCommand::Close);
                context.request_repaint();
            });
        }
        self.frames_left = self.frames_left.saturating_sub(1);
    }
}

fn save_ppm(path: &PathBuf, image: &egui::ColorImage) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "P6\n{} {}\n255", image.size[0], image.size[1])?;
    for pixel in &image.pixels {
        file.write_all(&[pixel.r(), pixel.g(), pixel.b()])?;
    }
    Ok(())
}

fn main() -> eframe::Result {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(
        args.next()
            .expect("usage: screenshot <out.ppm> [frames] [preset] [map_side] [dark|light]"),
    );
    let frames_left = args
        .next()
        .map_or(240, |frames| frames.parse().expect("frames"));
    let preset = args
        .next()
        .map_or(0, |preset| preset.parse::<usize>().expect("preset"));

    let map_side = args
        .next()
        .map_or(32, |side| side.parse::<usize>().expect("map_side"));
    let theme = match args.next().as_deref() {
        None | Some("dark") => egui::Theme::Dark,
        Some("light") => egui::Theme::Light,
        Some(other) => panic!("theme must be dark or light, not {other}"),
    };

    let options = native_options(egui::ViewportBuilder::default().with_inner_size([1360.0, 820.0]));
    eframe::run_native(
        "genetic_algo_ui_screenshot",
        options,
        Box::new(move |creation| {
            let mut app = App::new(&creation.egui_ctx);
            creation.egui_ctx.set_theme(theme);
            app.controller.apply_preset(Preset::ALL[preset]);
            app.controller.setup.width = map_side;
            app.controller.setup.height = map_side;
            app.controller.start_new_run();
            app.controller.ticks_per_second = 2_000.0;
            app.controller.toggle_running();
            Ok(Box::new(Screenshotter {
                app,
                path,
                frames_left,
                requested: false,
                started: Instant::now(),
                frames: 0,
            }))
        }),
    )
}
