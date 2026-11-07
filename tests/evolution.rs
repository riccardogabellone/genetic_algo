//! The point of the project: smartness must rise because it helps survival.
//!
//! A rising mean alone proves nothing. The trait starts near zero and is clamped there,
//! so mutation pushes it up even if it is useless. Only the gap against a control run,
//! where smartness has no behavioural effect, is evidence of natural selection.

use genetic_algo::{config::Config, sampling::Spread, simulation::Simulation};

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

/// Runs `rules` to the end on each seed and returns the populations left.
fn survivors(rules: &Config, ticks: u32) -> Vec<usize> {
    (0..SEEDS)
        .map(|seed| {
            let config = Config {
                max_ticks: ticks,
                target_smartness: 100.0,
                ..rules.clone()
            };
            let mut simulation = Simulation::new(config, seed).unwrap();
            simulation.run_to_end();
            simulation.latest().population
        })
        .collect()
}

/// Founders are nearly random, so they are reckless. The same violence that wipes them
/// out barely touches a population that is already smart.
#[test]
fn heavy_violence_destroys_dumb_founders_but_not_smart_ones() {
    let violent = Config {
        random_fight_chance: 0.5,
        fight_damage: 20,
        ..Config::default()
    };
    let smart_start = Config {
        founder_smartness: Spread {
            min: 85.0,
            mode: 90.0,
            max: 95.0,
        },
        ..violent.clone()
    };
    assert_eq!(survivors(&violent, 1_000), vec![0; SEEDS as usize]);
    assert!(
        survivors(&smart_start, 1_000)
            .iter()
            .all(|alive| *alive > 50)
    );
}

#[test]
fn fights_lower_current_smartness_but_not_what_is_inherited() {
    let peaceful = final_innate_and_current(&Config::default());
    let fighting = final_innate_and_current(&Config {
        contest_fight_chance: 0.5,
        random_fight_chance: 0.02,
        ..Config::default()
    });
    assert_eq!(peaceful.0, peaceful.1, "no fights, no scars");
    assert!(fighting.1 < fighting.0 - 0.5, "{fighting:?}");
    assert!(
        (fighting.0 - peaceful.0).abs() < 15.0,
        "{fighting:?} vs {peaceful:?}"
    );
}

/// Mean innate and current smartness at the end, across seeds.
fn final_innate_and_current(rules: &Config) -> (f32, f32) {
    let (mut innate, mut current) = (0.0, 0.0);
    for seed in 0..SEEDS {
        let config = Config {
            max_ticks: TICKS,
            target_smartness: 100.0,
            ..rules.clone()
        };
        let mut simulation = Simulation::new(config, seed).unwrap();
        simulation.run_to_end();
        let summary = simulation
            .latest()
            .smartness
            .expect("the population survives");
        innate += summary.innate_mean;
        current += summary.mean;
    }
    (innate / SEEDS as f32, current / SEEDS as f32)
}

/// Mean aggression at the end, across the seeds that survive.
fn final_aggression(rules: &Config) -> f32 {
    let mut values = Vec::new();
    for seed in 0..SEEDS {
        let config = Config {
            max_ticks: POLICY_TICKS,
            target_smartness: 100.0,
            ..rules.clone()
        };
        let mut simulation = Simulation::new(config, seed).unwrap();
        simulation.run_to_end();
        values.extend(
            simulation
                .latest()
                .mean_policy
                .map(|policy| policy.aggression),
        );
    }
    assert!(!values.is_empty(), "at least one run must survive");
    values.iter().sum::<f32>() / values.len() as f32
}

const POLICY_TICKS: u32 = 12_000;

/// Aggression only drifts without fights, and is selected against with them. When
/// followers act on a leader's policy instead, their own aggression is rarely expressed,
/// so selection cannot see it: leaders keep the population from evolving peace.
#[test]
fn fights_select_against_aggression_unless_leaders_shield_it() {
    let policies_only = Config {
        leaders_enabled: true,
        leader_influence: 0.0,
        ..Config::default()
    };
    let fighting = Config {
        contest_fight_chance: 0.5,
        random_fight_chance: 0.02,
        ..policies_only.clone()
    };
    let followers = Config {
        leader_influence: 1.0,
        ..fighting.clone()
    };
    let (peaceful, free, led) = (
        final_aggression(&policies_only),
        final_aggression(&fighting),
        final_aggression(&followers),
    );
    assert!(
        free < peaceful - 0.2,
        "with fights {free:.2}, without {peaceful:.2}"
    );
    assert!(led > free + 0.15, "led {led:.2}, free {free:.2}");
}

/// Mean smartness at the end with brains on, across seeds.
fn smartness_with_brains(inherited: bool) -> f32 {
    let mut total = 0.0;
    for seed in 0..SEEDS {
        let config = Config {
            max_ticks: TICKS,
            target_smartness: 100.0,
            brains_enabled: true,
            brains_inherited: inherited,
            ..Config::default()
        };
        let mut simulation = Simulation::new(config, seed).unwrap();
        simulation.run_to_end();
        total += simulation
            .latest()
            .smartness
            .expect("the population survives")
            .mean;
    }
    total / SEEDS as f32
}

/// A thinking individual uses its brain, so smartness only pays once brains are good.
/// Inherited brains improve and smartness rises with them; fresh random brains every
/// generation make thinking harmful, which holds smartness down.
#[test]
fn inherited_brains_improve_and_random_ones_do_not() {
    let (inherited, random) = (smartness_with_brains(true), smartness_with_brains(false));
    assert!(
        inherited > random + 10.0,
        "inherited {inherited:.1}, random {random:.1}"
    );
}

/// Nothing is hard-coded: founders' brains are random, yet evolved ones eat when hungry and
/// avoid hazards.
#[test]
fn evolved_brains_learn_to_eat_and_avoid_hazards() {
    use genetic_algo::brain::{Brain, Features};
    let option = |food: bool, hazard: bool| Features {
        empty: !food && !hazard,
        food,
        wall: false,
        hazard,
        individual: false,
        stay: false,
        hunger: 0.8,
        food_nearby: 0.0,
        hazards_nearby: 0.0,
    };
    let config = Config {
        max_ticks: 6_000,
        target_smartness: 100.0,
        brains_enabled: true,
        ..Config::default()
    };
    let mut simulation = Simulation::new(config, 1).unwrap();
    simulation.run_to_end();
    let brains: Vec<&Brain> = simulation
        .world()
        .individuals()
        .filter_map(|(_, individual)| individual.brain.as_deref())
        .collect();
    let share = |test: &dyn Fn(&Brain) -> bool| {
        brains.iter().filter(|brain| test(brain)).count() as f32 / brains.len() as f32
    };
    let eats =
        share(&|brain| brain.score(&option(true, false)) > brain.score(&option(false, false)));
    let avoids =
        share(&|brain| brain.score(&option(false, false)) > brain.score(&option(false, true)));
    assert!(
        eats > 0.9 && avoids > 0.9,
        "eats {eats:.2}, avoids hazards {avoids:.2}"
    );
}

/// Learning during life makes thinking pay even with no inherited brains: every child
/// starts from a random brain, yet the population still evolves higher smartness.
#[test]
fn learning_rescues_random_brains() {
    let smartness = |learning_enabled: bool| {
        let mut total = 0.0;
        for seed in 0..SEEDS {
            let config = Config {
                max_ticks: LEARNING_TICKS,
                target_smartness: 100.0,
                brains_enabled: true,
                brains_inherited: false,
                learning_enabled,
                ..Config::default()
            };
            let mut simulation = Simulation::new(config, seed).unwrap();
            simulation.run_to_end();
            total += simulation
                .latest()
                .smartness
                .expect("the population survives")
                .mean;
        }
        total / SEEDS as f32
    };
    let (learning, fixed) = (smartness(true), smartness(false));
    assert!(
        learning > fixed + 3.0,
        "learning {learning:.1}, fixed {fixed:.1}"
    );
}

const LEARNING_TICKS: u32 = 4_000;
