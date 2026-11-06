//! Scripted situations, one per rule in the README.

mod common;

use common::{CENTER, adult, only_individual, scenario_config, simulation_with};
use genetic_algo::{
    individual::Gender::{Female, Male},
    world::{Cell, Position, World},
};

const SEEDS: u64 = 40;

fn fill_ring(world: &mut World, center: Position, cell: &Cell) {
    for position in world.ring(center, 1) {
        world.set(position, cell.clone());
    }
}

fn world_with_center(individual: genetic_algo::individual::Individual) -> World {
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, individual);
    world
}

// ---------- Hazards ----------

#[test]
fn smart_individuals_never_touch_hazards() {
    let config = scenario_config();
    for seed in 0..SEEDS {
        let mut world = world_with_center(adult(1, Male, 100.0));
        fill_ring(&mut world, CENTER, &Cell::Hazard);
        world.set(Position { x: 1, y: 1 }, Cell::Empty);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        let (position, individual) = only_individual(&simulation);
        assert_eq!(
            position,
            Position { x: 1, y: 1 },
            "takes the only safe exit"
        );
        assert_eq!(individual.health, 50 - config.turn_cost);
    }
}

#[test]
fn a_smart_individual_with_no_safe_exit_stays_put() {
    let config = scenario_config();
    let mut world = world_with_center(adult(1, Male, 100.0));
    fill_ring(&mut world, CENTER, &Cell::Hazard);
    let mut simulation = simulation_with(config.clone(), world, 1);
    simulation.step();
    let (position, individual) = only_individual(&simulation);
    assert_eq!(position, CENTER);
    assert_eq!(individual.health, 50 - config.turn_cost);
}

/// Regression: merely standing next to a hazard used to kill instantly.
#[test]
fn touching_a_hazard_hurts_but_does_not_kill_outright_or_consume_it() {
    let config = scenario_config();
    let mut world = world_with_center(adult(1, Male, 0.0));
    fill_ring(&mut world, CENTER, &Cell::Hazard);
    let mut simulation = simulation_with(config.clone(), world, 1);
    simulation.step();
    let (position, individual) = only_individual(&simulation);
    assert_eq!(
        position, CENTER,
        "bounces off instead of standing on the hazard"
    );
    assert_eq!(
        individual.health,
        50 - config.hazard_damage - config.turn_cost
    );
    assert_eq!(simulation.world().count(&Cell::Hazard), 8);
}

#[test]
fn a_fatal_hazard_is_recorded_as_the_cause_of_death() {
    let config = scenario_config();
    let mut lone = adult(1, Male, 0.0);
    lone.health = config.hazard_damage;
    let mut world = world_with_center(lone);
    fill_ring(&mut world, CENTER, &Cell::Hazard);
    let mut simulation = simulation_with(config, world, 1);
    simulation.step();
    assert_eq!(simulation.latest().population, 0);
    assert_eq!(simulation.latest().events.hazard, 1);
    assert_eq!(simulation.latest().events.starved, 0);
}

// ---------- Food ----------

#[test]
fn hungry_smart_individuals_always_eat() {
    let config = scenario_config();
    for seed in 0..SEEDS {
        let mut world = world_with_center(adult(1, Male, 100.0));
        let food = Position { x: 3, y: 2 };
        world.set(food, Cell::Food);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        let (position, individual) = only_individual(&simulation);
        assert_eq!(position, food);
        assert_eq!(individual.health, 50 + config.food_gain);
        assert_eq!(simulation.world().count(&Cell::Food), 0);
        assert_eq!(simulation.latest().events.meals, 1);
    }
}

#[test]
fn sated_smart_individuals_leave_food_alone() {
    let config = scenario_config();
    let mut sated = adult(1, Male, 100.0);
    sated.health = config.hungry_below_health;
    let mut world = world_with_center(sated);
    world.set(Position { x: 3, y: 2 }, Cell::Food);
    let mut simulation = simulation_with(config, world, 1);
    simulation.step();
    assert_eq!(simulation.world().count(&Cell::Food), 1);
}

#[test]
fn overeating_kills_and_still_consumes_the_food() {
    let config = scenario_config();
    let mut full = adult(1, Male, 0.0);
    full.health = config.max_health - config.food_gain;
    let mut world = world_with_center(full);
    fill_ring(&mut world, CENTER, &Cell::Food);
    let mut simulation = simulation_with(config, world, 1);
    simulation.step();
    assert_eq!(simulation.latest().population, 0);
    assert_eq!(simulation.latest().events.overfed, 1);
    assert_eq!(simulation.world().count(&Cell::Food), 7);
}

/// Regression: two individuals could both eat one food cell in the same tick.
#[test]
fn one_food_cell_feeds_only_one_individual() {
    let config = scenario_config();
    for seed in 0..SEEDS {
        let mut world = World::new(5, 5);
        world.place_individual(Position { x: 1, y: 2 }, adult(1, Male, 100.0));
        world.place_individual(Position { x: 3, y: 2 }, adult(2, Male, 100.0));
        world.set(CENTER, Cell::Food);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        assert_eq!(simulation.latest().events.meals, 1);
        assert_eq!(simulation.world().count(&Cell::Food), 0);
        assert_eq!(simulation.latest().population, 2);
    }
}

/// Regression: turns ran in scan order, so the top-left individual always won.
#[test]
fn contested_food_is_won_by_either_side() {
    let config = scenario_config();
    let mut wins = [0_u32; 2];
    for seed in 0..SEEDS {
        let mut world = World::new(5, 5);
        world.place_individual(Position { x: 1, y: 2 }, adult(0, Male, 100.0));
        world.place_individual(Position { x: 3, y: 2 }, adult(1, Male, 100.0));
        world.set(CENTER, Cell::Food);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        let Cell::Individual(winner) = simulation.world().get(CENTER) else {
            panic!("someone must have eaten the food");
        };
        wins[winner.id as usize] += 1;
    }
    assert!(wins[0] >= 8 && wins[1] >= 8, "{wins:?}");
}

// ---------- Metabolism and age ----------

#[test]
fn an_individual_starves_when_health_runs_out() {
    let config = scenario_config();
    let mut weak = adult(1, Male, 100.0);
    weak.health = 1;
    let mut simulation = simulation_with(config, world_with_center(weak), 1);
    simulation.step();
    assert_eq!(simulation.latest().population, 0);
    assert_eq!(simulation.latest().events.starved, 1);
}

#[test]
fn an_individual_dies_exactly_at_its_personal_lifespan() {
    let config = scenario_config();
    let mut old = adult(1, Male, 100.0);
    old.age = 88;
    old.lifespan = 90;
    let mut simulation = simulation_with(config, world_with_center(old), 1);
    simulation.step();
    assert_eq!(simulation.latest().population, 1, "age 89 of 90: alive");
    simulation.step();
    assert_eq!(simulation.latest().population, 0, "age 90 of 90: dead");
    assert_eq!(simulation.latest().events.old_age, 1);
}

#[test]
fn walls_block_movement_and_are_permanent() {
    let config = scenario_config();
    for seed in 0..SEEDS {
        let smartness = if seed % 2 == 0 { 0.0 } else { 100.0 };
        let mut world = world_with_center(adult(1, Male, smartness));
        fill_ring(&mut world, CENTER, &Cell::Wall);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        assert_eq!(only_individual(&simulation).0, CENTER);
        assert_eq!(simulation.world().count(&Cell::Wall), 8);
    }
}

#[test]
fn speed_sets_the_ring_of_destinations() {
    let config = scenario_config();
    for seed in 0..SEEDS {
        let mut fast = adult(1, Male, 100.0);
        fast.speed = 2;
        let mut simulation = simulation_with(config.clone(), world_with_center(fast), seed);
        simulation.step();
        let (position, _) = only_individual(&simulation);
        let distance = position
            .x
            .abs_diff(CENTER.x)
            .max(position.y.abs_diff(CENTER.y));
        assert_eq!(distance, 2);
    }
}

// ---------- Reproduction ----------

fn couple() -> World {
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, adult(1, Male, 50.0));
    world.place_individual(Position { x: 3, y: 2 }, adult(2, Female, 50.0));
    world
}

/// Regression: births were free, so the population created energy from nothing.
#[test]
fn a_birth_conserves_health() {
    let config = scenario_config();
    let mut simulation = simulation_with(config.clone(), couple(), 1);
    let before: u32 = simulation
        .world()
        .individuals()
        .map(|(_, i)| i.health)
        .sum();
    simulation.step();
    assert_eq!(
        simulation.latest().events.births,
        1,
        "one birth per couple per tick"
    );

    let after: Vec<_> = simulation
        .world()
        .individuals()
        .map(|(_, i)| i.clone())
        .collect();
    assert_eq!(after.len(), 3);
    let baby = after.iter().find(|i| i.age == 0).expect("a newborn");
    assert_eq!(baby.health, config.mating_cost * 2);
    assert_eq!(baby.generation, 1);

    // The only energy lost is one turn of upkeep for each parent.
    let total: u32 = after.iter().map(|i| i.health).sum();
    assert_eq!(before - total, config.turn_cost * 2);
}

#[test]
fn newborns_do_not_act_on_the_tick_they_are_born() {
    let mut simulation = simulation_with(scenario_config(), couple(), 2);
    simulation.step();
    let baby_age = simulation.world().individuals().map(|(_, i)| i.age).min();
    assert_eq!(baby_age, Some(0));
}

#[test]
fn cooldown_spaces_out_births() {
    let mut config = scenario_config();
    config.mating_cooldown = 1_000;
    let mut simulation = simulation_with(config, couple(), 3);
    for _ in 0..10 {
        simulation.step();
    }
    let births: u32 = simulation.history().iter().map(|h| h.events.births).sum();
    // The two founders are the only adults and stay on cooldown for the whole test.
    assert_eq!(births, 1);
}

/// Regression: a cooldown of N used to block only N - 1 turns.
#[test]
fn a_short_cooldown_still_blocks_the_next_tick() {
    let mut config = scenario_config();
    config.mating_cooldown = 2;
    for seed in 0..SEEDS {
        let mut simulation = simulation_with(config.clone(), couple(), seed);
        simulation.step();
        assert_eq!(simulation.latest().events.births, 1);
        simulation.step();
        assert_eq!(simulation.latest().events.births, 0, "seed {seed}");
    }
}

#[test]
fn both_parents_enter_cooldown() {
    let mut config = scenario_config();
    config.mating_cooldown = 1_000;
    let mut simulation = simulation_with(config, couple(), 4);
    simulation.step();
    let parents: Vec<_> = simulation
        .world()
        .individuals()
        .filter(|(_, individual)| individual.age > 0)
        .map(|(_, individual)| individual.mating_cooldown)
        .collect();
    assert_eq!(parents.len(), 2);
    assert!(
        parents.iter().all(|cooldown| *cooldown >= 999),
        "{parents:?}"
    );
}

#[test]
fn same_gender_never_mates() {
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, adult(1, Male, 50.0));
    world.place_individual(Position { x: 3, y: 2 }, adult(2, Male, 50.0));
    let mut simulation = simulation_with(scenario_config(), world, 1);
    simulation.step();
    assert_eq!(simulation.latest().events.births, 0);
}

#[test]
fn immature_partners_never_mate() {
    let config = scenario_config();
    let mut young = adult(2, Female, 50.0);
    young.age = config.maturity_age - 2;
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, adult(1, Male, 50.0));
    world.place_individual(Position { x: 3, y: 2 }, young);
    let mut simulation = simulation_with(config, world, 1);
    simulation.step();
    assert_eq!(simulation.latest().events.births, 0);
}

#[test]
fn zero_fertility_never_breeds() {
    let mut world = World::new(5, 5);
    for (id, gender, position) in [(1, Male, CENTER), (2, Female, Position { x: 3, y: 2 })] {
        let mut infertile = adult(id, gender, 50.0);
        infertile.reproduction = 0.0;
        world.place_individual(position, infertile);
    }
    let mut simulation = simulation_with(scenario_config(), world, 1);
    simulation.step();
    assert_eq!(simulation.latest().events.births, 0);
}

#[test]
fn no_room_means_no_birth() {
    let mut world = World::new(2, 1);
    world.place_individual(Position { x: 0, y: 0 }, adult(1, Male, 100.0));
    world.place_individual(Position { x: 1, y: 0 }, adult(2, Female, 100.0));
    let config = genetic_algo::config::Config {
        width: 2,
        height: 1,
        ..scenario_config()
    };
    let mut simulation = simulation_with(config, world, 1);
    simulation.step();
    assert_eq!(simulation.latest().events.births, 0);
    assert_eq!(simulation.latest().population, 2);
}

/// Eligibility covers the birth and a full turn's upkeep, for both parents.
#[test]
fn mating_cannot_kill_either_parent() {
    let config = scenario_config();
    let threshold = config.mating_cost + config.turn_cost + 1;
    for seed in 0..SEEDS {
        let mut world = World::new(5, 5);
        for (id, gender, position) in [(1, Male, CENTER), (2, Female, Position { x: 3, y: 2 })] {
            let mut parent = adult(id, gender, 100.0);
            parent.health = threshold;
            world.place_individual(position, parent);
        }
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        assert_eq!(simulation.latest().events.births, 1);
        assert_eq!(simulation.latest().events.deaths(), 0);
    }
}
