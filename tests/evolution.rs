//! The point of the project: smartness must rise because it helps survival.
//!
//! A rising mean alone proves nothing. The trait starts near zero and is clamped there,
//! so mutation pushes it up even if it is useless. Only the gap against a control run,
//! where smartness has no behavioural effect, is evidence of natural selection.

use genetic_algo::{config::Config, simulation::Simulation};

const SEEDS: u64 = 4;
const TICKS: u32 = 2_000;

/// Population-averaged smartness at the end, across seeds.
fn final_smartness(smartness_affects_behavior: bool) -> f32 {
    let mut total = 0.0;
    for seed in 0..SEEDS {
        let config = Config {
            max_ticks: TICKS,
            target_smartness: 100.0, // Never stop early: both arms must run equally long.
            smartness_affects_behavior,
            ..Config::default()
        };
        let mut simulation = Simulation::new(config, seed).unwrap();
        simulation.run_to_end();

        // An extinct arm would score 0 and make the comparison pass for the wrong reason.
        let survivors = simulation.latest().smartness;
        total += survivors
            .expect("both arms must survive the whole run")
            .mean;
    }
    total / SEEDS as f32
}

/// Regression: the first implementation stayed flat, because smartness neither mutated
/// nor changed behaviour below 50.
#[test]
fn smartness_rises_far_above_the_control() {
    let selected = final_smartness(true);
    let control = final_smartness(false);
    assert!(
        selected > control + 15.0,
        "selection {selected:.1} vs control {control:.1}"
    );
}

#[test]
fn a_real_population_survives_while_it_evolves() {
    for seed in 0..SEEDS {
        let config = Config {
            max_ticks: TICKS,
            ..Config::default()
        };
        let mut simulation = Simulation::new(config, seed).unwrap();
        simulation.run_to_end();
        assert!(simulation.latest().population > 50, "seed {seed}");
    }
}

#[test]
fn without_mutation_or_selection_the_mean_stays_within_founder_range() {
    let config = Config {
        max_ticks: 500,
        mutation_rate: 0.0,
        smartness_affects_behavior: false,
        ..Config::for_size(16, 16)
    };
    let mut simulation = Simulation::new(config, 1).unwrap();
    simulation.run_to_end();
    for stats in simulation.history() {
        if let Some(smartness) = stats.smartness {
            assert!(smartness.min >= 1.0 && smartness.max <= 5.0);
        }
    }
}
