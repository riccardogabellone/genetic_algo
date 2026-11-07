//! Elections and their effects, scripted on small maps.

mod common;

use std::collections::HashMap;

use common::{CENTER, adult, scenario_config, simulation_with};
use genetic_algo::{
    config::Config,
    individual::{
        Gender::{Female, Male},
        Individual,
    },
    leadership::{MAX_AGGRESSION, Policy},
    simulation::Simulation,
    world::{Cell, Position, World},
};

const SEEDS: u64 = 40;
const RIGHT: Position = Position { x: 3, y: 2 };
const FAR: Position = Position { x: 4, y: 4 };

fn rules() -> Config {
    Config {
        leaders_enabled: true,
        election_interval: 1_000,
        ..scenario_config()
    }
}

fn with_policy(mut individual: Individual, aggression: f32, appetite: f32) -> Individual {
    individual.civics.policy = Policy {
        aggression,
        appetite,
    };
    individual
}

/// `candidate` cannot lose: everyone else has already used up their terms.
fn barred(mut individual: Individual, config: &Config) -> Individual {
    individual.civics.record_mut().terms_served = config.max_terms;
    individual
}

fn individual_with_id(simulation: &Simulation, id: u64) -> Option<Individual> {
    simulation
        .world()
        .individuals()
        .find(|(_, individual)| individual.id == id)
        .map(|(_, individual)| individual.clone())
}

fn total_elections(simulation: &Simulation) -> u32 {
    simulation
        .history()
        .iter()
        .map(|h| h.events.elections)
        .sum()
}

// ---------- When elections happen ----------

#[test]
fn without_the_rule_nobody_is_ever_elected() {
    let mut simulation = Simulation::new(Config::for_size(16, 16), 1).unwrap();
    for _ in 0..200 {
        simulation.step();
    }
    assert_eq!(total_elections(&simulation), 0);
    assert_eq!(simulation.leadership().leader(), None);
}

#[test]
fn the_first_tick_elects_an_adult_and_counts_the_term() {
    let config = rules();
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, adult(1, Male, 50.0));
    world.place_individual(FAR, adult(2, Female, 50.0));
    let mut simulation = simulation_with(config, world, 1);
    simulation.step();

    assert_eq!(total_elections(&simulation), 1);
    let leader = simulation
        .leadership()
        .leader()
        .expect("someone was elected");
    assert_eq!(simulation.latest().leader, Some(leader));
    assert_eq!(
        individual_with_id(&simulation, leader)
            .unwrap()
            .civics
            .terms_served(),
        1
    );
}

#[test]
fn elections_follow_the_term_length_and_no_sooner() {
    let config = Config {
        election_interval: 10,
        max_terms: 1_000,
        ..rules()
    };
    let mut world = World::new(5, 5);
    for (id, position) in [(1, CENTER), (2, FAR), (3, Position { x: 0, y: 0 })] {
        let mut long_lived = adult(id, Male, 100.0);
        long_lived.lifespan = 1_000;
        world.place_individual(position, long_lived);
    }
    let mut simulation = simulation_with(config, world, 1);
    for _ in 0..30 {
        simulation.step();
    }
    assert_eq!(total_elections(&simulation), 3);
}

#[test]
fn a_leaders_death_calls_an_election_on_the_next_tick() {
    let config = rules();
    let mut dying = adult(1, Male, 100.0);
    dying.age = dying.lifespan - 1;
    // Becomes an adult on the next tick, so it can only win the second election.
    let mut successor = adult(3, Male, 100.0);
    successor.age = config.maturity_age - 1;
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, dying);
    world.place_individual(FAR, barred(adult(2, Male, 100.0), &config));
    world.place_individual(Position { x: 0, y: 0 }, successor);
    let mut simulation = simulation_with(config, world, 1);

    simulation.step();
    assert!(individual_with_id(&simulation, 1).is_none());
    assert_eq!(total_elections(&simulation), 1);
    assert_eq!(
        simulation.leadership().leader(),
        None,
        "the leader died in office"
    );

    simulation.step();
    assert_eq!(
        total_elections(&simulation),
        2,
        "long before the term was up"
    );
    assert_eq!(simulation.leadership().leader(), Some(3));
}

// ---------- Who can stand and vote ----------

#[test]
fn a_lone_adult_cannot_elect_itself() {
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, adult(1, Male, 50.0));
    let mut simulation = simulation_with(rules(), world, 1);
    simulation.step();
    assert_eq!(total_elections(&simulation), 0);
    assert_eq!(simulation.leadership().leader(), None);
}

#[test]
fn newborns_can_neither_stand_nor_vote() {
    let config = rules();
    let mut young = adult(2, Female, 50.0);
    young.age = 0;
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, adult(1, Male, 50.0));
    world.place_individual(FAR, young);
    let mut simulation = simulation_with(config, world, 1);
    simulation.step();
    assert_eq!(
        total_elections(&simulation),
        0,
        "one adult is not an electorate"
    );
}

#[test]
fn the_term_limit_bars_a_candidate_for_life() {
    let config = rules();
    for seed in 0..SEEDS {
        let mut world = World::new(5, 5);
        world.place_individual(CENTER, barred(adult(1, Male, 50.0), &config));
        world.place_individual(FAR, adult(2, Female, 50.0));
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        assert_eq!(simulation.leadership().leader(), Some(2), "seed {seed}");
    }
}

/// Over whole runs, nobody is ever elected more than the limit allows.
#[test]
fn nobody_is_elected_more_than_the_limit_in_a_real_run() {
    for (max_terms, seed) in [(1, 1), (2, 2), (3, 3)] {
        let config = Config {
            leaders_enabled: true,
            election_interval: 10,
            max_terms,
            max_ticks: 1_000,
            ..Config::for_size(16, 16)
        };
        let mut simulation = Simulation::new(config, seed).unwrap();
        let mut terms: HashMap<u64, u32> = HashMap::new();
        let mut previous = None;
        for _ in 0..1_000 {
            simulation.step();
            let latest = simulation.latest();
            if latest.events.elections > 0
                && let Some(leader) = latest.leader
            {
                let count = terms.entry(leader).or_default();
                *count += 1;
                assert!(*count <= max_terms, "#{leader} elected {count} times");
                previous = Some(leader);
            }
        }
        assert!(previous.is_some(), "the run must have had elections");
    }
}

#[test]
fn voters_elect_the_candidate_closest_to_them() {
    let config = Config {
        ballot_size: 10,
        ..rules()
    };
    for seed in 0..SEEDS {
        let mut world = World::new(5, 5);
        world.place_individual(CENTER, with_policy(adult(1, Male, 0.0), 0.0, 0.5));
        world.place_individual(FAR, with_policy(adult(2, Male, 0.0), 2.0, 0.5));
        for (id, x) in [(3, 0), (4, 1), (5, 2)] {
            world.place_individual(
                Position { x, y: 0 },
                barred(with_policy(adult(id, Male, 0.0), 0.1, 0.5), &config),
            );
        }
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        assert_eq!(simulation.leadership().leader(), Some(1), "seed {seed}");
    }
}

// ---------- What leaders change ----------

/// Food beside the follower only; the follower's own appetite would ignore it.
fn appetite_scene(config: &Config) -> World {
    let mut world = World::new(5, 5);
    let follower = barred(with_policy(adult(2, Male, 100.0), 1.0, 0.0), config);
    world.place_individual(CENTER, follower);
    world.place_individual(FAR, with_policy(adult(1, Male, 100.0), 1.0, 1.0));
    world.set(RIGHT, Cell::Food);
    world
}

#[test]
fn a_follower_acts_on_the_leaders_appetite() {
    let config = Config {
        leader_influence: 1.0,
        ..rules()
    };
    for seed in 0..SEEDS {
        let mut simulation = simulation_with(config.clone(), appetite_scene(&config), seed);
        simulation.step();
        assert_eq!(simulation.leadership().leader(), Some(1));
        assert_eq!(simulation.latest().events.meals, 1, "seed {seed}");
    }
}

#[test]
fn without_influence_a_follower_keeps_its_own_appetite() {
    let config = Config {
        leader_influence: 0.0,
        ..rules()
    };
    for seed in 0..SEEDS {
        let mut simulation = simulation_with(config.clone(), appetite_scene(&config), seed);
        simulation.step();
        assert_eq!(simulation.latest().events.meals, 0, "seed {seed}");
    }
}

#[test]
fn the_leader_itself_acts_on_its_own_policy() {
    let config = Config {
        leader_influence: 1.0,
        ..rules()
    };
    for seed in 0..SEEDS {
        let mut world = World::new(5, 5);
        // The leader will not eat, even though the follower would.
        world.place_individual(CENTER, with_policy(adult(1, Male, 100.0), 1.0, 0.0));
        world.place_individual(
            Position { x: 0, y: 4 },
            barred(with_policy(adult(2, Male, 100.0), 1.0, 1.0), &config),
        );
        world.set(RIGHT, Cell::Food);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        assert_eq!(simulation.leadership().leader(), Some(1));
        assert_eq!(simulation.latest().events.meals, 0, "seed {seed}");
    }
}

fn fights_with_leader_aggression(aggression: f32) -> u32 {
    let config = Config {
        leader_influence: 1.0,
        random_fight_chance: 0.5,
        fight_smartness_loss: 0.0,
        ..rules()
    };
    (0..400)
        .map(|seed| {
            let mut world = World::new(5, 5);
            world.place_individual(FAR, with_policy(adult(1, Male, 100.0), aggression, 0.5));
            world.place_individual(CENTER, barred(adult(2, Male, 0.0), &config));
            world.place_individual(RIGHT, barred(adult(3, Male, 100.0), &config));
            let mut simulation = simulation_with(config.clone(), world, seed);
            simulation.step();
            simulation.latest().events.fights
        })
        .sum()
}

#[test]
fn a_leaders_aggression_sets_how_much_followers_fight() {
    let dove = fights_with_leader_aggression(0.0);
    let neutral = fights_with_leader_aggression(1.0);
    let hawk = fights_with_leader_aggression(MAX_AGGRESSION);
    assert_eq!(dove, 0);
    assert!(hawk > neutral + neutral / 2, "{hawk} vs {neutral}");
}

// ---------- Memory across terms ----------

#[test]
fn voters_remember_how_a_term_went_for_them() {
    let config = Config {
        election_interval: 5,
        max_terms: 100,
        ..rules()
    };
    let mut world = World::new(5, 5);
    for (id, position) in [(1, CENTER), (2, FAR), (3, Position { x: 0, y: 0 })] {
        let mut voter = adult(id, Male, 100.0);
        voter.lifespan = 1_000;
        world.place_individual(position, voter);
    }
    let mut simulation = simulation_with(config, world, 1);
    for _ in 0..6 {
        simulation.step();
    }
    assert_eq!(total_elections(&simulation), 2);
    for (_, voter) in simulation.world().individuals() {
        assert_eq!(voter.civics.memories().len(), 1, "#{}", voter.id);
        assert!(voter.civics.memories()[0].outcome < 0.0, "health only fell");
    }
}

#[test]
fn children_do_not_inherit_memories_or_terms() {
    let config = Config {
        leaders_enabled: true,
        election_interval: 5,
        max_ticks: 500,
        ..Config::for_size(16, 16)
    };
    let mut simulation = Simulation::new(config, 1).unwrap();
    for _ in 0..500 {
        simulation.step();
    }
    for (_, individual) in simulation.world().individuals() {
        if individual.age == 0 {
            assert!(individual.civics.memories().is_empty());
            assert_eq!(individual.civics.terms_served(), 0);
        }
    }
}

// ---------- Switching it live ----------

#[test]
fn the_rule_can_be_switched_on_and_off_during_a_run() {
    let off = Config::for_size(16, 16);
    let mut simulation = Simulation::new(off.clone(), 1).unwrap();
    for _ in 0..50 {
        simulation.step();
    }
    assert_eq!(simulation.leadership().leader(), None);

    simulation
        .set_config(Config {
            leaders_enabled: true,
            ..off.clone()
        })
        .unwrap();
    for _ in 0..50 {
        simulation.step();
    }
    assert!(total_elections(&simulation) > 0);
    assert!(simulation.leadership().leader().is_some());

    simulation.set_config(off).unwrap();
    assert_eq!(simulation.leadership().leader(), None);
    let before = total_elections(&simulation);
    for _ in 0..50 {
        simulation.step();
    }
    assert_eq!(total_elections(&simulation), before);
}

#[test]
fn policies_stay_put_without_mutation_or_a_spread_to_draw_from() {
    let neutral = Config::default();
    let policy = Policy::neutral(&neutral);
    let spread = |value| genetic_algo::sampling::Spread {
        min: value,
        mode: value,
        max: value,
    };
    let config = Config {
        leaders_enabled: true,
        founder_aggression: spread(policy.aggression),
        founder_appetite: spread(policy.appetite),
        policy_mutation_step: 0.0,
        max_ticks: 500,
        target_smartness: 100.0,
        ..Config::for_size(16, 16)
    };
    let mut simulation = Simulation::new(config, 1).unwrap();
    simulation.run_to_end();
    assert!(simulation.latest().population > 0);
    for (_, individual) in simulation.world().individuals() {
        assert_eq!(individual.civics.policy, policy);
    }
}
