//! Drives the real widgets, so these fail if a control is miswired or missing.

use eframe::egui;
use egui_kittest::{Harness, kittest::Queryable as _};
use genetic_algo_ui::app::App;

fn harness() -> Harness<'static, App> {
    let context = egui::Context::default();
    let app = App::new(&context);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1360.0, 1000.0))
        .build_ui_state(|ui, app: &mut App| app.show(ui), app);
    harness.step();
    harness
}

fn click(harness: &mut Harness<'static, App>, label: &str) {
    harness.get_by_label(label).click();
    harness.step();
    harness.step();
}

#[test]
fn play_and_pause_buttons_swap_and_control_the_run() {
    let mut harness = harness();
    click(&mut harness, "Play");
    assert!(harness.state().controller.is_running());

    click(&mut harness, "Pause");
    assert!(!harness.state().controller.is_running());
    assert!(harness.query_by_label("Play").is_some());
}

#[test]
fn step_advances_one_tick() {
    let mut harness = harness();
    click(&mut harness, "Step");
    assert_eq!(harness.state().controller.simulation().latest().tick, 1);
}

#[test]
fn new_random_run_resets_the_tick_and_changes_the_seed() {
    let mut harness = harness();
    click(&mut harness, "Step");
    let seed = harness.state().controller.seed();
    click(&mut harness, "New random run");
    let controller = &harness.state().controller;
    assert_eq!(controller.simulation().latest().tick, 0);
    assert_ne!(controller.seed(), seed);
}

#[test]
fn replay_seed_restarts_with_the_same_seed() {
    let mut harness = harness();
    let seed = harness.state().controller.seed();
    click(&mut harness, "Step");
    click(&mut harness, "Replay seed");
    let controller = &harness.state().controller;
    assert_eq!(controller.simulation().latest().tick, 0);
    assert_eq!(controller.seed(), seed);
}

#[test]
fn preset_buttons_change_the_live_rules_and_reveal_the_speed_chart() {
    let mut harness = harness();
    assert!(harness.query_by_label("Mean speed").is_none());
    click(&mut harness, "All rules");

    let config = harness.state().controller.simulation().config();
    assert_eq!(config.smartness_upkeep, 2.0);
    assert_eq!(config.max_vision, 5);
    assert_eq!(config.max_speed, 4);
    assert_eq!(config.speed_upkeep, 1.0);
    assert!(harness.query_by_label("Mean speed").is_some());

    click(&mut harness, "Default");
    assert_eq!(
        harness.state().controller.simulation().config().max_vision,
        0
    );
}

#[test]
fn stop_at_target_checkbox_reaches_the_simulation() {
    let mut harness = harness();
    assert_eq!(
        harness
            .state()
            .controller
            .simulation()
            .config()
            .target_smartness,
        80.0
    );
    click(&mut harness, "Stop at target");
    assert_eq!(
        harness
            .state()
            .controller
            .simulation()
            .config()
            .target_smartness,
        100.0
    );
}

#[test]
fn start_smart_waits_for_a_new_run() {
    let mut harness = harness();
    click(&mut harness, "Start smart (85-95)");
    let before = harness
        .state()
        .controller
        .simulation()
        .latest()
        .smartness
        .unwrap();
    assert!(before.mean < 10.0, "not applied until a new run");

    click(&mut harness, "New random run");
    let after = harness
        .state()
        .controller
        .simulation()
        .latest()
        .smartness
        .unwrap();
    assert!(after.mean > 80.0);
}

#[test]
fn hovering_every_part_of_the_map_does_not_panic() {
    let mut harness = harness();
    for x in (300..920).step_by(40) {
        for y in (40..800).step_by(40) {
            harness.hover_at(egui::pos2(x as f32, y as f32));
            harness.step();
        }
    }
}

#[test]
fn the_fighting_preset_reaches_the_simulation_and_the_counters_show() {
    let mut harness = harness();
    click(&mut harness, "Fighting");
    let config = harness.state().controller.simulation().config();
    assert_eq!(config.contest_fight_chance, 0.5);
    assert_eq!(config.random_fight_chance, 0.02);
    assert!(harness.query_by_label("Killed in fights").is_some());
    assert!(harness.query_by_label("Fights").is_some());
}
