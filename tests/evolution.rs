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

/// Mean smartness and speed at the end, across seeds.
fn final_traits(rules: &Config, ticks: u32) -> (f32, f32) {
    let (mut smartness, mut speed) = (0.0, 0.0);
    for seed in 0..SEEDS {
        let config = Config {
            max_ticks: ticks,
            target_smartness: 100.0,
            ..rules.clone()
        };
        let mut simulation = Simulation::new(config, seed).unwrap();
        simulation.run_to_end();
        let latest = simulation.latest();
        smartness += latest.smartness.expect("the population must survive").mean;
        speed += latest.mean_speed.expect("the population must survive");
    }
    (smartness / SEEDS as f32, speed / SEEDS as f32)
}

#[test]
fn a_heavy_smartness_cost_stops_it_evolving() {
    let (free, _) = final_traits(&Config::default(), TICKS);
    let costly_rules = Config {
        smartness_upkeep: 8.0,
        ..Config::default()
    };
    let (costly, _) = final_traits(&costly_rules, TICKS);
    assert!(costly < free - 15.0, "cost {costly:.1} vs free {free:.1}");
}

/// Vision scales with smartness and rounds to nothing below about 10, so its effect
/// only shows once smartness has climbed: this needs a longer run than the other tests.
#[test]
fn vision_offsets_part_of_the_smartness_cost() {
    const VISION_TICKS: u32 = 8_000;
    let blind_rules = Config {
        smartness_upkeep: 4.0,
        ..Config::default()
    };
    let sighted_rules = Config {
        max_vision: 5,
        ..blind_rules.clone()
    };
    let (blind, _) = final_traits(&blind_rules, VISION_TICKS);
    let (sighted, _) = final_traits(&sighted_rules, VISION_TICKS);
    assert!(
        sighted > blind + 3.0,
        "sighted {sighted:.1} vs blind {blind:.1}"
    );
}

#[test]
fn speed_evolves_upward_and_its_cost_restrains_it() {
    let free_rules = Config {
        max_speed: 4,
        ..Config::default()
    };
    let priced_rules = Config {
        speed_upkeep: 2.0,
        ..free_rules.clone()
    };
    let (_, free) = final_traits(&free_rules, TICKS);
    let (_, priced) = final_traits(&priced_rules, TICKS);
    assert!(free > 2.0, "free speed {free:.2}");
    assert!(priced < free - 0.5, "priced {priced:.2} vs free {free:.2}");
}
