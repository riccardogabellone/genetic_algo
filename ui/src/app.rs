//! The window: controls on the left, the map in the middle, stats and charts on the right.

use std::time::Duration;

use eframe::egui::{self, Color32, Key, Slider, TextEdit};
use genetic_algo::leadership::TermJudgement;

use crate::{
    charts::{self, Panels},
    controller::{Controller, Preset},
    map_view,
};

/// Simulation time allowed per frame, which keeps a 60 fps window responsive.
const FRAME_BUDGET: Duration = Duration::from_millis(10);
const MAX_MAP_SIDE: usize = 128;
const MIN_MAP_SIDE: usize = 8;
const RELIABLE_MAP_SIDE: usize = 16;
const ERROR_COLOR: Color32 = Color32::from_rgb(255, 130, 130);

pub struct App {
    pub controller: Controller,
}

impl App {
    pub fn new(context: &egui::Context) -> Self {
        // The map has a dark background, so the whole window matches it.
        context.set_theme(egui::Theme::Dark);
        Self {
            controller: Controller::new(u64::from(rand::random::<u32>())),
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

        egui::Panel::left("controls")
            .resizable(false)
            .exact_size(290.0)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.controls(ui));
            });
        egui::Panel::right("stats")
            .resizable(true)
            .default_size(440.0)
            .min_size(300.0)
            .show(ui, |ui| self.stats_and_charts(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            map_view::legend(ui);
            let simulation = self.controller.simulation();
            map_view::show(ui, simulation.world(), simulation.leadership().leader());
        });

        if self.controller.is_running() {
            ui.ctx().request_repaint();
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        self.run_controls(ui);
        ui.separator();
        self.new_run_controls(ui);
        ui.separator();
        self.rule_controls(ui);
    }

    fn run_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Run");
        ui.horizontal(|ui| {
            let label = if self.controller.is_running() {
                "Pause"
            } else {
                "Play"
            };
            let can_run = self.controller.simulation().outcome().is_none();
            if ui.add_enabled(can_run, egui::Button::new(label)).clicked() {
                self.controller.toggle_running();
            }
            if ui.add_enabled(can_run, egui::Button::new("Step")).clicked() {
                self.controller.step_once();
            }
            ui.weak("Space: play / pause");
        });
        ui.add(
            Slider::new(&mut self.controller.ticks_per_second, 1.0..=20_000.0)
                .logarithmic(true)
                .integer()
                .text("ticks/s"),
        );
        if let Some(message) = self.controller.outcome_message() {
            ui.colored_label(Color32::from_rgb(196, 134, 30), message);
        }
        if let Some(error) = self.controller.error() {
            ui.colored_label(ERROR_COLOR, error);
        }
    }

    fn new_run_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("New run");
        ui.weak("Applied when a new run starts.");
        let setup = &mut self.controller.setup;
        ui.add(Slider::new(&mut setup.width, MIN_MAP_SIDE..=MAX_MAP_SIDE).text("Width"));
        ui.add(Slider::new(&mut setup.height, MIN_MAP_SIDE..=MAX_MAP_SIDE).text("Height"));
        if setup.width.min(setup.height) < RELIABLE_MAP_SIDE {
            ui.colored_label(ERROR_COLOR, "Maps under 16 often go extinct.");
        }
        ui.checkbox(&mut setup.start_smart, "Start smart (85-95)");
        ui.horizontal(|ui| {
            let label = ui.label("Seed");
            ui.add(TextEdit::singleline(&mut setup.seed_text).desired_width(110.0))
                .labelled_by(label.id);
        });
        ui.horizontal(|ui| {
            if ui.button("New random run").clicked() {
                self.controller.start_new_run();
            }
            if ui.button("Replay seed").clicked() {
                self.controller.replay_seed();
            }
        });
    }

    fn rule_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Rules");
        ui.weak("Applied immediately, to the current run.");
        ui.horizontal_wrapped(|ui| {
            for preset in Preset::ALL {
                if ui
                    .button(preset.label())
                    .on_hover_text(preset.description())
                    .clicked()
                {
                    self.controller.apply_preset(preset);
                }
            }
        });

        let rules = &mut self.controller.rules;
        let mut changed = false;
        let mut track = |response: egui::Response| changed |= response.changed();

        ui.add_space(4.0);
        ui.strong("Optional rules");
        track(
            ui.add(Slider::new(&mut rules.smartness_cost, 0.0..=16.0).text("Smartness cost"))
                .on_hover_text("Extra health per turn at smartness 100, proportional below it."),
        );
        track(
            ui.add(Slider::new(&mut rules.vision, 0..=16).text("Vision"))
                .on_hover_text("How far smartness 100 sees food, proportional below it."),
        );
        track(
            ui.add(Slider::new(&mut rules.max_speed, 1..=8).text("Max speed"))
                .on_hover_text("Speed can evolve up to this value. 1 keeps it fixed."),
        );
        track(
            ui.add(Slider::new(&mut rules.speed_cost, 0.0..=8.0).text("Speed cost"))
                .on_hover_text("Extra health per point of speed above 1, on turns that move."),
        );

        ui.add_space(4.0);
        ui.strong("Ecosystem");
        track(
            ui.add(
                Slider::new(&mut rules.food_regrowth_percent, 0.0..=12.5).text("Food regrowth %"),
            )
            .on_hover_text("Share of all cells that regrow as food per tick."),
        );
        track(ui.add(Slider::new(&mut rules.food_gain, 1..=60).text("Food gain")));
        track(ui.add(Slider::new(&mut rules.hazard_damage, 1..=100).text("Hazard damage")));

        ui.add_space(4.0);
        ui.strong("Combat");
        track(
            ui.add(Slider::new(&mut rules.contest_fights, 0.0..=1.0).text("Over food"))
                .on_hover_text(
                    "Chance that a hungry individual strikes a rival racing for the same food.",
                ),
        );
        track(
            ui.add(Slider::new(&mut rules.random_fights, 0.0..=1.0).text("For no reason"))
                .on_hover_text(
                    "Chance per turn of an unprovoked attack at smartness 0, falling to 0 at maximum.",
                ),
        );
        track(
            ui.add(Slider::new(&mut rules.fight_damage, 1..=100).text("Damage"))
                .on_hover_text("Health the victim loses."),
        );
        track(
            ui.add(
                Slider::new(&mut rules.fight_smartness_loss, 0.0..=30.0).text("Attacker's loss"),
            )
            .on_hover_text(
                "Current smartness the attacker loses. Children do not inherit the loss.",
            ),
        );

        ui.add_space(4.0);
        ui.strong("Politics");
        track(ui.checkbox(&mut rules.leaders, "Elect leaders"));
        ui.add_enabled_ui(rules.leaders, |ui| {
            track(
                ui.add(Slider::new(&mut rules.election_interval, 1..=200).text("Term (ticks)"))
                    .on_hover_text("Ticks between elections. A leader's death calls one at once."),
            );
            track(
                ui.add(Slider::new(&mut rules.leader_influence, 0.0..=1.0).text("Influence"))
                    .on_hover_text("Chance per turn that a follower acts on the leader's policy."),
            );
            track(
                ui.add(Slider::new(&mut rules.max_terms, 1..=10).text("Max terms"))
                    .on_hover_text("Times one individual can be elected in its life."),
            );
            track(
                ui.add(Slider::new(&mut rules.memory, 0..=20).text("Memory"))
                    .on_hover_text(
                        "Past elections recalled at smartness 100, proportional below it.",
                    ),
            );
            ui.label("Voters judge a term by").on_hover_text(
                "Fights overall: how violent the whole population was, compared with typical terms. \
                 Own health: mostly luck with food, so memory learns nothing from it.",
            );
            ui.horizontal(|ui| {
                track(ui.radio_value(&mut rules.term_judgement, TermJudgement::Violence, "Fights overall"));
                track(ui.radio_value(&mut rules.term_judgement, TermJudgement::Personal, "Own health"));
            });
        });

        ui.add_space(4.0);
        ui.strong("Inheritance");
        track(ui.add(Slider::new(&mut rules.mutation_rate, 0.0..=1.0).text("Mutation rate")));
        track(ui.add(Slider::new(&mut rules.mutation_step, 0.0..=30.0).text("Mutation step")));

        ui.add_space(4.0);
        ui.strong("Goal");
        track(ui.checkbox(&mut rules.stop_at_target, "Stop at target"));
        ui.add_enabled_ui(rules.stop_at_target, |ui| {
            track(ui.add(Slider::new(&mut rules.target, 1.0..=99.0).text("Target smartness")));
        });

        if changed {
            self.controller.apply_rules();
        }
    }

    fn stats_and_charts(&self, ui: &mut egui::Ui) {
        let controller = &self.controller;
        let simulation = controller.simulation();
        let latest = simulation.latest();
        let totals = controller.totals();
        let config = simulation.config();

        egui::Grid::new("stats")
            .num_columns(4)
            .spacing([18.0, 2.0])
            .show(ui, |ui| {
                let mut row = |left: (&str, String), right: (&str, String)| {
                    ui.weak(left.0);
                    ui.label(left.1);
                    ui.weak(right.0);
                    ui.label(right.1);
                    ui.end_row();
                };
                let smartness = latest.smartness.map_or_else(
                    || "n/a".to_owned(),
                    |s| format!("{:.1} ({:.0}-{:.0})", s.mean, s.min, s.max),
                );
                let speed = latest
                    .mean_speed
                    .map_or_else(|| "n/a".to_owned(), |speed| format!("{speed:.2}"));
                row(
                    ("Tick", latest.tick.to_string()),
                    ("Seed", controller.seed().to_string()),
                );
                row(
                    (
                        "Population",
                        format!(
                            "{} ({}M/{}F)",
                            latest.population, latest.males, latest.females
                        ),
                    ),
                    ("Generation", latest.max_generation.to_string()),
                );
                row(("Smartness", smartness), ("Speed", speed));
                row(
                    ("Births", totals.births.to_string()),
                    ("Meals", totals.meals.to_string()),
                );
                row(
                    ("Starved", totals.starved.to_string()),
                    ("Overfed", totals.overfed.to_string()),
                );
                row(
                    ("Old age", totals.old_age.to_string()),
                    ("Hazard", totals.hazard.to_string()),
                );
                row(
                    ("Fights", totals.fights.to_string()),
                    ("Killed in fights", totals.combat.to_string()),
                );
                if config.leaders_enabled || totals.elections > 0 {
                    let leader = simulation
                        .leadership()
                        .leader()
                        .map_or_else(|| "none".to_owned(), |id| format!("#{id}"));
                    let policy = simulation.leadership().policy().map_or_else(
                        || "n/a".to_owned(),
                        |policy| {
                            format!(
                                "aggr {:.2}, appetite {:.2}",
                                policy.aggression, policy.appetite
                            )
                        },
                    );
                    row(
                        ("Leader", leader),
                        ("Elections", totals.elections.to_string()),
                    );
                    row(("Leader's policy", policy), ("", String::new()));
                }
            });
        ui.separator();

        let target = controller
            .rules
            .stop_at_target
            .then_some(controller.rules.target);
        let show_speed =
            controller.rules.max_speed > 1 || latest.mean_speed.is_some_and(|speed| speed > 1.0);
        let show_innate = config.contest_fight_chance > 0.0
            || config.random_fight_chance > 0.0
            || totals.fights > 0;
        let show_policy = config.leaders_enabled;
        charts::show(
            ui,
            simulation.history(),
            target,
            Panels {
                speed: show_speed,
                innate: show_innate,
                policy: show_policy,
            },
        );
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
            app.controller.apply_rules();
            assert_eq!(app.controller.error(), None);
            frame(&mut app, &context, Vec::new());
        }
        assert!(Config::default().validate().is_ok());
    }
}
