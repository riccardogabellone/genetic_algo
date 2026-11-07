//! The left panel: settings for the next run, then the live rules as cards. A rule that is
//! either on or off has a switch in its card's header, and its settings show only while it
//! is on.

use std::cell::Cell;

use eframe::egui::{self, Response, Slider, TextEdit};
use genetic_algo::leadership::TermJudgement;

use crate::{
    controller::{Controller, Preset, Rules},
    theme,
    widgets::{self, card},
};

pub const MAX_MAP_SIDE: usize = 128;
pub const MIN_MAP_SIDE: usize = 8;
const RELIABLE_MAP_SIDE: usize = 16;
const CARD_GAP: f32 = 8.0;
/// Wide enough for a slider row (rail, value, longest label) inside a card.
pub const WIDTH: f32 = 320.0;
const SLIDER_WIDTH: f32 = 96.0;

pub fn show(ui: &mut egui::Ui, controller: &mut Controller) {
    ui.spacing_mut().slider_width = SLIDER_WIDTH;
    new_run(ui, controller);
    ui.add_space(14.0);
    ui.heading("Rules");
    ui.weak("Applied immediately, to the current run.");
    ui.add_space(2.0);
    presets(ui, controller);
    ui.add_space(CARD_GAP);
    if rule_cards(ui, &mut controller.rules) {
        controller.apply_rules();
    }
}

fn new_run(ui: &mut egui::Ui, controller: &mut Controller) {
    ui.heading("New run");
    ui.weak("Applied when a new run starts.");
    ui.add_space(2.0);
    let palette = theme::palette(ui.ctx());
    let mut start_new = false;
    let mut replay = false;
    let current_seed = controller.seed();
    let setup = &mut controller.setup;
    card(ui, "World", None, |ui| {
        ui.add(Slider::new(&mut setup.width, MIN_MAP_SIDE..=MAX_MAP_SIDE).text("Width"));
        ui.add(Slider::new(&mut setup.height, MIN_MAP_SIDE..=MAX_MAP_SIDE).text("Height"));
        if setup.width.min(setup.height) < RELIABLE_MAP_SIDE {
            ui.colored_label(palette.warning, "Maps under 16 often go extinct.");
        }
        widgets::switch(ui, &mut setup.start_smart, "Start smart (85-95)");
        ui.horizontal(|ui| {
            let label = ui.label("Seed");
            ui.add(TextEdit::singleline(&mut setup.seed_text).desired_width(110.0))
                .labelled_by(label.id);
        });
        // The field can be edited without replaying, so the run on screen keeps its own.
        ui.weak(format!("This run's seed: {current_seed}"));
        ui.horizontal(|ui| {
            start_new = ui.button("New random run").clicked();
            replay = ui.button("Replay seed").clicked();
        });
    });
    if start_new {
        controller.start_new_run();
    }
    if replay {
        controller.replay_seed();
    }
}

fn presets(ui: &mut egui::Ui, controller: &mut Controller) {
    ui.horizontal_wrapped(|ui| {
        for preset in Preset::ALL {
            if ui
                .button(preset.label())
                .on_hover_text(preset.description())
                .clicked()
            {
                controller.apply_preset(preset);
            }
        }
    });
}

/// Returns whether any rule changed.
fn rule_cards(ui: &mut egui::Ui, rules: &mut Rules) -> bool {
    let changed = Cell::new(false);
    let track = |response: Response| changed.set(changed.get() | response.changed());
    let gap = |ui: &mut egui::Ui| ui.add_space(CARD_GAP);

    card(ui, "Costs and senses", None, |ui| {
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
    });
    gap(ui);

    card(ui, "Ecosystem", None, |ui| {
        track(
            ui.add(
                Slider::new(&mut rules.food_regrowth_percent, 0.0..=12.5).text("Food regrowth %"),
            )
            .on_hover_text("Share of all cells that regrow as food per tick."),
        );
        track(ui.add(Slider::new(&mut rules.food_gain, 1..=60).text("Food gain")));
        track(ui.add(Slider::new(&mut rules.hazard_damage, 1..=100).text("Hazard damage")));
    });
    gap(ui);

    card(ui, "Combat", None, |ui| {
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
    });
    gap(ui);

    let switched = card(
        ui,
        "Politics",
        Some((&mut rules.leaders, "Elect leaders")),
        |ui| {
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
                track(ui.radio_value(
                    &mut rules.term_judgement,
                    TermJudgement::Violence,
                    "Fights overall",
                ));
                track(ui.radio_value(
                    &mut rules.term_judgement,
                    TermJudgement::Personal,
                    "Own health",
                ));
            });
        },
    );
    changed.set(changed.get() | switched);
    gap(ui);

    let switched = card(
        ui,
        "Neural networks",
        Some((&mut rules.brains, "Evolved brains")),
        |ui| {
            ui.weak(
                "An inherited network chooses actions instead of the hand-written rules. \
                 Newborns get brains; individuals alive when this is switched on keep the rules.",
            );
            track(
                widgets::switch(ui, &mut rules.brains_inherited, "Inherit brains").on_hover_text(
                    "Off for the control experiment: every child gets a fresh random brain.",
                ),
            );
            track(
                ui.add(
                    Slider::new(&mut rules.brain_mutation_rate, 0.0..=1.0).text("Weight mutation"),
                )
                .on_hover_text("Chance that each weight mutates in a child."),
            );
            track(
                ui.add(Slider::new(&mut rules.brain_mutation_step, 0..=64).text("Mutation size"))
                    .on_hover_text("Largest change to one weight, in steps of 1/16."),
            );
            track(
                widgets::switch(ui, &mut rules.learning, "Learn during life").on_hover_text(
                    "Each brain also learns what its actions cause. Learned weights die with \
                     the individual; children inherit only the genome.",
                ),
            );
            ui.add_enabled_ui(rules.learning, |ui| {
                track(
                    ui.add(Slider::new(&mut rules.learning_rate, 0.0..=2.0).text("Learning rate"))
                        .on_hover_text("How far each lesson moves a brain's weights."),
                );
            });
        },
    );
    changed.set(changed.get() | switched);
    gap(ui);

    card(ui, "Inheritance", None, |ui| {
        track(ui.add(Slider::new(&mut rules.mutation_rate, 0.0..=1.0).text("Mutation rate")));
        track(ui.add(Slider::new(&mut rules.mutation_step, 0.0..=30.0).text("Mutation step")));
    });
    gap(ui);

    let switched = card(
        ui,
        "Goal",
        Some((&mut rules.stop_at_target, "Stop at target")),
        |ui| {
            track(ui.add(Slider::new(&mut rules.target, 1.0..=99.0).text("Target smartness")));
        },
    );
    changed.set(changed.get() | switched);

    changed.get()
}
