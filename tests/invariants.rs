//! Properties that must hold on every tick, for many seeds and map shapes.

mod common;

use std::collections::HashMap;

use genetic_algo::{
    config::{Config, MAX_SMARTNESS},
    individual::Gender,
    leadership::{Civics, MAX_AGGRESSION},
    simulation::{Outcome, Simulation},
    world::{Cell, Position, World},
};

struct Snapshot {
    individuals: HashMap<u64, (Position, u32, usize)>,
    walls: usize,
    hazards: usize,
}

fn snapshot(simulation: &Simulation) -> Snapshot {
    let world = simulation.world();
    Snapshot {
        individuals: world
            .individuals()
            .map(|(position, individual)| {
                (
                    individual.id,
                    (position, individual.age, usize::from(individual.speed)),
                )
            })
            .collect(),
        walls: world.count(&Cell::Wall),
        hazards: world.count(&Cell::Hazard),
    }
}

fn check_individuals(simulation: &Simulation) {
    let config = simulation.config();
    let mut seen = 0;
    for (_, individual) in simulation.world().individuals() {
        seen += 1;
        let can_be_wounded = config.contest_fight_chance > 0.0 || config.random_fight_chance > 0.0;
        assert!(individual.health > 0 || can_be_wounded);
        assert!(individual.health < config.max_health);
        assert!(individual.age < individual.lifespan);
        assert!((0.0..=MAX_SMARTNESS).contains(&individual.smartness));
        assert!((0.0..=MAX_SMARTNESS).contains(&individual.innate_smartness));
        assert!(individual.smartness <= individual.innate_smartness);
        if config.fight_smartness_loss == 0.0 {
            assert_eq!(individual.smartness, individual.innate_smartness);
        }
        assert!((0.0..=1.0).contains(&individual.reproduction));
        assert!((1..=config.max_speed).contains(&usize::from(individual.speed)));
        let civics = &individual.civics;
        assert!((0.0..=MAX_AGGRESSION).contains(&civics.policy.aggression));
        assert!((0.0..=1.0).contains(&civics.policy.appetite));
        assert!(civics.terms_served() <= config.max_terms);
        assert!(civics.memories().len() <= config.max_memory);
    }
    assert_eq!(seen, simulation.latest().population);
}

fn check_tick(before: &Snapshot, simulation: &Simulation) {
    let now = snapshot(simulation);
    let latest = simulation.latest();

    assert_eq!(
        now.individuals.len(),
        latest.population,
        "ids must be unique"
    );
    assert_eq!(now.walls, before.walls, "walls are permanent");
    assert_eq!(now.hazards, before.hazards, "hazards are permanent");

    let expected =
        before.individuals.len() + latest.events.births as usize - latest.events.deaths() as usize;
    assert_eq!(
        latest.population, expected,
        "births and deaths must balance"
    );

    let mut newborns = 0;
    for (id, (position, age, speed)) in &now.individuals {
        if let Some((old_position, old_age, old_speed)) = before.individuals.get(id) {
            assert_eq!(speed, old_speed, "speed is fixed for life");
            assert_eq!(*age, old_age + 1, "everyone acts exactly once per tick");
            let step = position
                .x
                .abs_diff(old_position.x)
                .max(position.y.abs_diff(old_position.y));
            assert!(step == 0 || step == *speed, "moved {step} cells");
        } else {
            assert_eq!(*age, 0, "only newborns may appear");
            newborns += 1;
        }
    }
    assert_eq!(newborns, latest.events.births);

    let food = simulation.world().count(&Cell::Food);
    assert!(
        food <= simulation
            .config()
            .max_food
            .max(simulation.config().initial_food)
    );
}

fn run_checked(config: Config, seed: u64, ticks: u32) {
    let mut simulation = Simulation::new(config, seed).expect("valid config");
    check_individuals(&simulation);
    for _ in 0..ticks {
        if simulation.outcome().is_some() {
            break;
        }
        let before = snapshot(&simulation);
        simulation.step();
        check_tick(&before, &simulation);
        check_individuals(&simulation);
    }
    for (index, stats) in simulation.history().iter().enumerate() {
        assert_eq!(stats.tick as usize, index, "history has one entry per tick");
    }
}

#[test]
fn invariants_hold_on_the_default_rules() {
    for seed in 0..6 {
        run_checked(Config::default(), seed, 400);
    }
}

#[test]
fn invariants_hold_on_every_small_and_narrow_map() {
    for (width, height) in [
        (1, 1),
        (1, 2),
        (2, 1),
        (2, 2),
        (3, 3),
        (1, 40),
        (40, 1),
        (2, 30),
        (8, 8),
        (16, 16),
    ] {
        for seed in 0..4 {
            // Default density leaves the tiniest maps empty, which would check nothing.
            let config = Config {
                initial_population: (width * height / 2).max(1),
                ..Config::for_size(width, height)
            };
            assert!(config.initial_population > 0);
            run_checked(config, seed, 200);
        }
    }
}

#[test]
fn invariants_hold_when_the_map_starts_full() {
    let config = Config {
        initial_population: 64,
        initial_food: 0,
        wall_count: 0,
        hazard_count: 0,
        ..Config::for_size(8, 8)
    };
    run_checked(config, 1, 200);

    let config = Config {
        initial_population: 16,
        initial_food: 16,
        wall_count: 16,
        hazard_count: 16,
        ..Config::for_size(8, 8)
    };
    run_checked(config, 2, 200);
}

#[test]
fn invariants_hold_under_extreme_rules() {
    let extremes = [
        Config {
            mutation_rate: 1.0,
            smartness_mutation_step: 100.0,
            ..Config::for_size(16, 16)
        },
        Config {
            mutation_rate: 0.0,
            ..Config::for_size(16, 16)
        },
        Config {
            hazard_damage: u32::MAX,
            hazard_count: 64,
            ..Config::for_size(16, 16)
        },
        Config {
            food_gain: u32::MAX,
            ..Config::for_size(16, 16)
        },
        Config {
            turn_cost: u32::MAX,
            ..Config::for_size(16, 16)
        },
        Config {
            min_lifespan: 1,
            max_lifespan: 1,
            ..Config::for_size(16, 16)
        },
        Config {
            initial_speed: 3,
            max_speed: 3,
            ..Config::for_size(16, 16)
        },
        Config {
            initial_speed: 64,
            max_speed: 64,
            ..Config::for_size(16, 16)
        },
        Config {
            maturity_age: 0,
            mating_cooldown: 0,
            ..Config::for_size(16, 16)
        },
        Config {
            food_regrowth_interval: u32::MAX,
            ..Config::for_size(16, 16)
        },
        Config {
            max_food: 0,
            initial_food: 0,
            ..Config::for_size(16, 16)
        },
    ];
    let new_rules = [
        Config {
            smartness_upkeep: 100.0,
            ..Config::for_size(16, 16)
        },
        Config {
            max_vision: 64,
            ..Config::for_size(16, 16)
        },
        Config {
            max_speed: 64,
            speed_upkeep: 100.0,
            mutation_rate: 1.0,
            ..Config::for_size(16, 16)
        },
        Config {
            smartness_upkeep: 2.0,
            max_vision: 5,
            max_speed: 4,
            speed_upkeep: 1.0,
            ..Config::for_size(16, 16)
        },
    ];
    let fights = [
        Config {
            contest_fight_chance: 1.0,
            ..Config::for_size(16, 16)
        },
        Config {
            random_fight_chance: 1.0,
            ..Config::for_size(16, 16)
        },
        Config {
            contest_fight_chance: 1.0,
            random_fight_chance: 1.0,
            fight_damage: 100,
            fight_smartness_loss: 100.0,
            ..Config::for_size(16, 16)
        },
        Config {
            random_fight_chance: 0.5,
            fight_smartness_loss: 0.0,
            max_speed: 4,
            max_vision: 5,
            smartness_upkeep: 2.0,
            ..Config::for_size(16, 16)
        },
    ];
    let leaders = [
        Config {
            leaders_enabled: true,
            ..Config::for_size(16, 16)
        },
        Config {
            leaders_enabled: true,
            election_interval: 1,
            max_terms: 1,
            ballot_size: 64,
            max_memory: 64,
            leader_influence: 1.0,
            mutation_rate: 1.0,
            policy_mutation_step: 1.0,
            ..Config::for_size(16, 16)
        },
        Config {
            leaders_enabled: true,
            contest_fight_chance: 1.0,
            random_fight_chance: 1.0,
            leader_influence: 1.0,
            ..Config::for_size(16, 16)
        },
        Config {
            leaders_enabled: true,
            max_memory: 0,
            leader_influence: 0.0,
            term_judgement: genetic_algo::leadership::TermJudgement::Personal,
            ..Config::for_size(4, 4)
        },
    ];
    let brains = [
        Config {
            brains_enabled: true,
            ..Config::for_size(16, 16)
        },
        Config {
            brains_enabled: true,
            brains_inherited: false,
            ..Config::for_size(16, 16)
        },
        Config {
            brains_enabled: true,
            brain_mutation_rate: 1.0,
            brain_mutation_step: 127,
            random_fight_chance: 0.5,
            contest_fight_chance: 1.0,
            leaders_enabled: true,
            max_speed: 4,
            ..Config::for_size(16, 16)
        },
    ];
    for config in extremes
        .into_iter()
        .chain(new_rules)
        .chain(fights)
        .chain(leaders)
        .chain(brains)
    {
        run_checked(config, 3, 150);
    }
}

#[test]
fn large_maps_stay_consistent() {
    run_checked(Config::for_size(256, 128), 1, 30);
}

#[test]
fn the_same_seed_replays_the_same_run() {
    let run = || {
        let mut simulation = Simulation::new(Config::for_size(16, 16), 42).unwrap();
        for _ in 0..200 {
            simulation.step();
        }
        simulation
    };
    let (first, second) = (run(), run());
    assert_eq!(first.world(), second.world());
    assert_eq!(first.history(), second.history());
}

#[test]
fn different_seeds_produce_different_runs() {
    let world = |seed| {
        Simulation::new(Config::for_size(16, 16), seed)
            .unwrap()
            .world()
            .clone()
    };
    assert_ne!(world(1), world(2));
}

// ---------- Outcomes ----------

#[test]
fn every_run_terminates_with_an_outcome() {
    for (width, height) in [(1, 1), (4, 4), (16, 16)] {
        let config = Config {
            max_ticks: 300,
            ..Config::for_size(width, height)
        };
        let mut simulation = Simulation::new(config, 1).unwrap();
        simulation.run_to_end();
        assert!(simulation.outcome().is_some());
        assert!(simulation.latest().tick <= 300);
    }
}

#[test]
fn an_empty_world_is_extinct_from_the_start() {
    let config = Config {
        initial_population: 0,
        ..Config::for_size(8, 8)
    };
    let simulation = Simulation::new(config, 1).unwrap();
    assert_eq!(simulation.outcome(), Some(Outcome::Extinct));
    assert!(simulation.latest().smartness.is_none());
}

#[test]
fn zero_ticks_ends_immediately() {
    let config = Config {
        max_ticks: 0,
        ..Config::for_size(8, 8)
    };
    let mut simulation = Simulation::new(config, 1).unwrap();
    assert_eq!(simulation.run_to_end(), Outcome::TicksExhausted);
    assert_eq!(simulation.history().len(), 1);
}

#[test]
fn a_single_sex_population_dies_out() {
    let mut world = World::new(8, 8);
    for id in 0..6 {
        let position = Position { x: id, y: 0 };
        world.place_individual(position, common::adult(id as u64, Gender::Male, 50.0));
    }
    let config = Config {
        max_ticks: 10_000,
        ..common::scenario_config()
    };
    let mut simulation = common::simulation_with(config, world, 1);
    assert_eq!(simulation.run_to_end(), Outcome::Extinct);
    let births: u32 = simulation.history().iter().map(|h| h.events.births).sum();
    assert_eq!(births, 0);
}

#[test]
fn stepping_after_extinction_is_harmless() {
    let config = Config {
        initial_population: 0,
        ..Config::for_size(4, 4)
    };
    let mut simulation = Simulation::new(config, 1).unwrap();
    simulation.step();
    assert_eq!(simulation.latest().population, 0);
    assert_eq!(simulation.latest().tick, 1);
}

#[test]
fn the_target_counts_only_for_a_real_population() {
    use genetic_algo::individual::Individual;
    let individual = |id, gender| Individual {
        id,
        generation: 0,
        gender,
        smartness: 100.0,
        innate_smartness: 100.0,
        reproduction: 0.0,
        health: 50,
        age: 0,
        lifespan: 90,
        speed: 1,
        mating_cooldown: 0,
        civics: Civics::neutral(&Config::default()),
        brain: None,
    };
    let config = Config {
        max_ticks: 1_000,
        ..Config::for_size(32, 32)
    };
    let needed = config.min_population_for_target();

    let mut world = World::new(32, 32);
    world.place_individual(Position { x: 0, y: 0 }, individual(0, Gender::Male));
    let lone = Simulation::new(config.clone(), 1)
        .unwrap()
        .with_world(world.clone());
    assert_eq!(
        lone.outcome(),
        None,
        "one genius is not an evolved population"
    );

    for id in 1..needed {
        let position = Position {
            x: id % 32,
            y: 1 + id / 32,
        };
        world.place_individual(position, individual(id as u64, Gender::Female));
    }
    let crowd = Simulation::new(config, 1).unwrap().with_world(world);
    assert_eq!(crowd.outcome(), Some(Outcome::TargetReached));
}

#[test]
fn food_regrows_on_schedule_and_up_to_its_cap() {
    let config = Config {
        initial_population: 0,
        initial_food: 0,
        wall_count: 0,
        hazard_count: 0,
        food_regrowth_interval: 3,
        food_regrowth_amount: 2,
        max_food: 5,
        ..Config::for_size(8, 8)
    };
    let mut simulation = Simulation::new(config, 1).unwrap();
    let mut food = Vec::new();
    for _ in 0..12 {
        simulation.step();
        food.push(simulation.world().count(&Cell::Food));
    }
    assert_eq!(food, [0, 0, 2, 2, 2, 4, 4, 4, 5, 5, 5, 5]);
}

// ---------- Changing rules mid-run ----------

#[test]
fn rules_can_change_during_a_run() {
    let rules = Config {
        max_speed: 4,
        speed_upkeep: 0.5,
        max_vision: 5,
        ..Config::for_size(16, 16)
    };
    let mut simulation = Simulation::new(rules.clone(), 1).unwrap();
    for _ in 0..300 {
        simulation.step();
    }
    // Lowering the maximum must not break individuals that are already faster.
    let stricter = Config {
        max_speed: 1,
        speed_upkeep: 0.0,
        max_vision: 0,
        smartness_upkeep: 1.0,
        ..rules
    };
    simulation.set_config(stricter).unwrap();
    for _ in 0..300 {
        simulation.step();
    }
    assert_eq!(simulation.config().max_speed, 1);
    assert_eq!(simulation.latest().tick, 600);
}

#[test]
fn invalid_or_resized_rules_are_rejected_and_the_old_ones_kept() {
    let original = Config::for_size(16, 16);
    let mut simulation = Simulation::new(original.clone(), 1).unwrap();

    let invalid = Config {
        smartness_upkeep: f32::NAN,
        ..original.clone()
    };
    assert!(simulation.set_config(invalid).is_err());

    assert!(simulation.set_config(Config::for_size(32, 32)).is_err());
    assert_eq!(*simulation.config(), original);
}

#[test]
fn without_fighting_nobody_fights_and_smartness_stays_innate() {
    let mut simulation = Simulation::new(Config::for_size(16, 16), 1).unwrap();
    for _ in 0..300 {
        simulation.step();
    }
    assert!(simulation.history().iter().all(|h| h.events.fights == 0));
    for (_, individual) in simulation.world().individuals() {
        assert_eq!(individual.smartness, individual.innate_smartness);
    }
}
