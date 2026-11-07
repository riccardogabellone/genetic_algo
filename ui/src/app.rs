//! The window: controls on the left, the map in the middle, stats and charts on the right.

use std::time::Duration;

use eframe::egui::{self, Key};
use egui_notify::{Anchor, Toasts};
use genetic_algo::world::Cell;

use crate::{
    controller::Controller, identity, inspector, map_view, rules_panel, stats_panel, theme, toolbar,
};

/// Simulation time allowed per frame, which keeps a 60 fps window responsive.
const FRAME_BUDGET: Duration = Duration::from_millis(10);
const TOAST_DURATION: Duration = Duration::from_secs(6);

pub struct App {
    pub controller: Controller,
    /// The individual the inspector shows, by id.
    pub selected: Option<u64>,
    logo: egui::TextureHandle,
    toasts: Toasts,
    /// What the views last saw: the run, and whether it had finished.
    seen_run: u64,
    seen_finished: bool,
}

impl App {
    pub fn new(context: &egui::Context) -> Self {
        theme::install(context);
        Self {
            controller: Controller::new(u64::from(rand::random::<u32>())),
            selected: None,
            logo: identity::logo_texture(context),
            toasts: Toasts::default()
                .with_anchor(Anchor::TopRight)
                .with_margin(egui::vec2(12.0, 60.0)),
            seen_run: 0,
            seen_finished: false,
        }
    }

    /// Draws one frame and advances the simulation by the time since the last one.
    pub fn show(&mut self, ui: &mut egui::Ui) {
        let (elapsed, toggle) = ui.input(|input| {
            (
                input.stable_dt,
                input.key_pressed(Key::Space) && !input.modifiers.any(),
            )
        });
        if toggle && !ui.ctx().egui_wants_keyboard_input() {
            self.controller.toggle_running();
        }
        self.controller.advance(elapsed, FRAME_BUDGET);
        self.notice_changes();

        egui::Panel::top("toolbar")
            .exact_size(48.0)
            .show(ui, |ui| toolbar::show(ui, &mut self.controller, &self.logo));
        self.status_bar(ui);
        egui::Panel::left("controls")
            .resizable(false)
            .exact_size(rules_panel::WIDTH)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .show(ui, |ui| rules_panel::show(ui, &mut self.controller));
            });
        egui::Panel::right("stats")
            .resizable(true)
            .default_size(440.0)
            .min_size(300.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| stats_panel::show(ui, &self.controller));
            });
        let map_frame =
            egui::Frame::central_panel(ui.style()).fill(theme::palette(ui.ctx()).window);
        egui::CentralPanel::default()
            .frame(map_frame)
            .show(ui, |ui| {
                map_view::legend(ui);
                let simulation = self.controller.simulation();
                let world = simulation.world();
                let leader = simulation.leadership().leader();
                let map_area = ui.available_rect_before_wrap();
                if let Some(position) = map_view::show(ui, world, leader, self.selected) {
                    self.selected = match world.get(position) {
                        Cell::Individual(individual) => Some(individual.id),
                        _ => None,
                    };
                }
                inspector::show(ui, map_area, world, &mut self.selected);
            });
        self.toasts.show(ui.ctx());

        if self.controller.is_running() {
            ui.ctx().request_repaint();
        }
    }

    /// Reacts to what changed since the last frame: a new run drops the selection (ids
    /// restart with it), and a run that just finished says why in a toast.
    fn notice_changes(&mut self) {
        if self.controller.run() != self.seen_run {
            self.seen_run = self.controller.run();
            self.seen_finished = false;
            self.selected = None;
        }
        let finished = self.controller.simulation().outcome().is_some();
        if finished
            && !self.seen_finished
            && let Some(message) = self.controller.outcome_message()
        {
            // Not closable: its close button is a glyph the bundled fonts lack, and the
            // toast leaves on its own while the status strip keeps the message.
            self.toasts
                .warning(message)
                .duration(TOAST_DURATION)
                .closable(false);
        }
        self.seen_finished = finished;
    }

    /// Why the run stopped, or why an edit was refused. Shown only when there is something
    /// to say.
    fn status_bar(&self, ui: &mut egui::Ui) {
        let palette = theme::palette(ui.ctx());
        let message = self
            .controller
            .error()
            .map(|error| (error.to_owned(), palette.danger))
            .or_else(|| {
                self.controller
                    .outcome_message()
                    .map(|message| (message, palette.warning))
            });
        if let Some((message, color)) = message {
            egui::Panel::top("status").show(ui, |ui| {
                ui.colored_label(color, message);
            });
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules_panel::{MAX_MAP_SIDE, MIN_MAP_SIDE};
    use eframe::egui::{Event, Modifiers, Pos2, RawInput, Rect, Vec2};
    use genetic_algo::config::Config;

    fn input(events: Vec<Event>) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360.0, 820.0))),
            events,
            ..RawInput::default()
        }
    }

    fn frame(app: &mut App, context: &egui::Context, events: Vec<Event>) -> egui::FullOutput {
        let mut output = context.run_ui(input(events), |ui| app.show(ui));
        output.textures_delta.clear(); // There is no renderer here to consume them.
        output
    }

    fn space() -> Event {
        Event::Key {
            key: Key::Space,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn app() -> (App, egui::Context) {
        let context = egui::Context::default();
        (App::new(&context), context)
    }

    #[test]
    fn frames_render_while_paused_and_running() {
        let (mut app, context) = app();
        frame(&mut app, &context, Vec::new());
        app.controller.toggle_running();
        app.controller.ticks_per_second = 1_000.0;
        for _ in 0..30 {
            frame(&mut app, &context, Vec::new());
        }
        assert!(app.controller.simulation().latest().tick > 0);
    }

    #[test]
    fn space_toggles_play_and_pause() {
        let (mut app, context) = app();
        frame(&mut app, &context, vec![space()]);
        assert!(app.controller.is_running());
        frame(&mut app, &context, vec![space()]);
        assert!(!app.controller.is_running());
    }

    #[test]
    fn running_requests_another_frame_and_pausing_does_not() {
        let (mut app, context) = app();
        let delay = |output: &egui::FullOutput| {
            output
                .viewport_output
                .values()
                .next()
                .map_or(Duration::MAX, |viewport| viewport.repaint_delay)
        };
        // Short animations such as a scrollbar fading out may take a few frames, but a
        // paused window must then go idle instead of burning CPU forever.
        let settled = (0..300)
            .any(|_| delay(&frame(&mut app, &context, Vec::new())) > Duration::from_secs(1));
        assert!(settled);

        app.controller.toggle_running();
        let running = frame(&mut app, &context, Vec::new());
        assert_eq!(delay(&running), Duration::ZERO);
    }

    #[test]
    fn extreme_maps_and_an_extinct_world_still_render() {
        let (mut app, context) = app();
        for (width, height) in [
            (MIN_MAP_SIDE, MIN_MAP_SIDE),
            (MAX_MAP_SIDE, MAX_MAP_SIDE),
            (MAX_MAP_SIDE, MIN_MAP_SIDE),
        ] {
            app.controller.setup.width = width;
            app.controller.setup.height = height;
            app.controller.start_new_run();
            frame(&mut app, &context, Vec::new());
        }

        app.controller.setup.width = MIN_MAP_SIDE;
        app.controller.setup.height = MIN_MAP_SIDE;
        app.controller.rules.food_regrowth_percent = 0.0;
        app.controller.start_new_run();
        app.controller.ticks_per_second = 20_000.0;
        app.controller.toggle_running();
        while app.controller.is_running() {
            app.controller.advance(0.25, Duration::from_secs(5));
        }
        assert_eq!(app.controller.simulation().latest().population, 0);
        frame(&mut app, &context, Vec::new());
    }

    #[test]
    fn the_slider_limits_cannot_produce_an_invalid_config() {
        // Every corner of the rule sliders must pass validation, or the UI could get stuck.
        let (mut app, context) = app();
        for (
            cost,
            vision,
            speed,
            speed_cost,
            regrowth,
            gain,
            damage,
            rate,
            step,
            fights,
            hit,
            loss,
        ) in [
            (0.0, 0, 1, 0.0, 0.0, 1, 1, 0.0, 0.0, 0.0, 1, 0.0),
            (16.0, 16, 8, 8.0, 12.5, 60, 100, 1.0, 30.0, 1.0, 100, 30.0),
        ] {
            let rules = &mut app.controller.rules;
            rules.smartness_cost = cost;
            rules.vision = vision;
            rules.max_speed = speed;
            rules.speed_cost = speed_cost;
            rules.food_regrowth_percent = regrowth;
            rules.food_gain = gain;
            rules.hazard_damage = damage;
            rules.mutation_rate = rate;
            rules.mutation_step = step;
            rules.contest_fights = fights;
            rules.random_fights = fights;
            rules.fight_damage = hit;
            rules.fight_smartness_loss = loss;
            rules.leaders = fights > 0.0;
            rules.brains = fights > 0.0;
            rules.learning = fights > 0.0;
            rules.learning_rate = rate * 2.0;
            rules.brain_mutation_rate = rate;
            rules.brain_mutation_step = (step as u8).min(64);
            app.controller.apply_rules();
            assert_eq!(app.controller.error(), None);
            frame(&mut app, &context, Vec::new());
        }
        assert!(Config::default().validate().is_ok());
    }
}
