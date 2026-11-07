//! Scripted situations, one per rule in the README.

mod common;

use common::{CENTER, adult, only_individual, scenario_config, simulation_with};
use genetic_algo::{
    behavior::vision_radius,
    brain::{Brain, HIDDEN, INPUTS, WEIGHTS},
    config::Config,
    individual::{
        Gender::{Female, Male},
        Individual,
    },
    simulation::Simulation,
    world::{Cell, Position, World},
};

const SEEDS: u64 = 40;

fn fill_ring(world: &mut World, center: Position, cell: &Cell) {
    for position in world.ring(center, 1) {
        world.set(position, cell.clone());
    }
}

fn world_with_center(individual: Individual) -> World {
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

// ---------- Vision ----------

#[test]
fn a_hungry_thinker_walks_toward_food_it_can_see() {
    let config = Config {
        max_vision: 5,
        ..scenario_config()
    };
    for seed in 0..SEEDS {
        let mut world = world_with_center(adult(1, Male, 100.0));
        world.set(Position { x: 4, y: 2 }, Cell::Food);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        assert_eq!(only_individual(&simulation).0, Position { x: 3, y: 2 });
    }
}

#[test]
fn food_beyond_vision_is_not_noticed() {
    let config = Config {
        max_vision: 1,
        ..scenario_config()
    };
    let mut moved_elsewhere = false;
    for seed in 0..SEEDS {
        let mut world = world_with_center(adult(1, Male, 100.0));
        world.set(Position { x: 4, y: 2 }, Cell::Food);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        moved_elsewhere |= only_individual(&simulation).0.x != 3;
    }
    assert!(moved_elsewhere);
}

#[test]
fn a_sated_thinker_ignores_distant_food() {
    let config = Config {
        max_vision: 5,
        ..scenario_config()
    };
    let mut moved_elsewhere = false;
    for seed in 0..SEEDS {
        let mut sated = adult(1, Male, 100.0);
        sated.health = config.hungry_below_health;
        let mut world = world_with_center(sated);
        world.set(Position { x: 4, y: 2 }, Cell::Food);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        moved_elsewhere |= only_individual(&simulation).0.x != 3;
    }
    assert!(moved_elsewhere);
}

#[test]
fn a_blocked_path_falls_back_to_any_free_cell() {
    let config = Config {
        max_vision: 5,
        ..scenario_config()
    };
    let mut world = world_with_center(adult(1, Male, 100.0));
    world.set(Position { x: 4, y: 2 }, Cell::Food);
    for y in 1..=3 {
        world.set(Position { x: 3, y }, Cell::Wall);
    }
    let mut simulation = simulation_with(config, world, 1);
    simulation.step();
    let (position, _) = only_individual(&simulation);
    assert_ne!(position, CENTER);
    assert!(position.x <= 2);
}

#[test]
fn vision_scales_with_smartness() {
    let config = Config {
        max_vision: 6,
        ..scenario_config()
    };
    assert_eq!(vision_radius(0.0, &config), 0);
    assert_eq!(vision_radius(50.0, &config), 3);
    assert_eq!(vision_radius(100.0, &config), 6);
}

// ---------- Upkeep ----------

fn health_after_one_turn(config: &Config, individual: Individual, seed: u64) -> u32 {
    let mut world = world_with_center(individual);
    fill_ring(&mut world, CENTER, &Cell::Wall);
    let mut simulation = simulation_with(config.clone(), world, seed);
    simulation.step();
    only_individual(&simulation).1.health
}

#[test]
fn smartness_upkeep_is_proportional_to_smartness() {
    let config = Config {
        smartness_upkeep: 4.0,
        ..scenario_config()
    };
    let base = 50 - config.turn_cost;
    assert_eq!(health_after_one_turn(&config, adult(1, Male, 0.0), 1), base);
    assert_eq!(
        health_after_one_turn(&config, adult(1, Male, 50.0), 1),
        base - 2
    );
    assert_eq!(
        health_after_one_turn(&config, adult(1, Male, 100.0), 1),
        base - 4
    );
}

#[test]
fn fractional_upkeep_is_right_on_average() {
    let config = Config {
        smartness_upkeep: 1.0,
        ..scenario_config()
    };
    let base = 50 - config.turn_cost;
    let runs = 2_000;
    let lost: u32 = (0..runs)
        .map(|seed| base - health_after_one_turn(&config, adult(1, Male, 25.0), seed))
        .sum();
    assert!((f64::from(lost) / runs as f64 - 0.25).abs() < 0.04);
}

/// Otherwise the control arm would not pay for a brain it cannot use.
#[test]
fn smartness_upkeep_applies_even_when_smartness_does_nothing() {
    let config = Config {
        smartness_upkeep: 4.0,
        smartness_affects_behavior: false,
        ..scenario_config()
    };
    let health = health_after_one_turn(&config, adult(1, Male, 100.0), 1);
    assert_eq!(health, 50 - config.turn_cost - 4);
}

#[test]
fn upkeep_is_also_paid_on_a_turn_spent_eating() {
    let config = Config {
        smartness_upkeep: 4.0,
        ..scenario_config()
    };
    let mut world = world_with_center(adult(1, Male, 100.0));
    world.set(Position { x: 3, y: 2 }, Cell::Food);
    let mut simulation = simulation_with(config.clone(), world, 1);
    simulation.step();
    assert_eq!(
        only_individual(&simulation).1.health,
        50 + config.food_gain - 4
    );
}

#[test]
fn speed_upkeep_is_charged_only_on_turns_that_change_cell() {
    let config = Config {
        speed_upkeep: 3.0,
        ..scenario_config()
    };
    let mut fast = adult(1, Male, 100.0);
    fast.speed = 2;

    // Boxed in at speed 2: every cell of the outer ring is a wall.
    let mut world = world_with_center(fast.clone());
    for position in world.ring(CENTER, 2) {
        world.set(position, Cell::Wall);
    }
    let mut simulation = simulation_with(config.clone(), world, 1);
    simulation.step();
    assert_eq!(only_individual(&simulation).1.health, 50 - config.turn_cost);

    let mut simulation = simulation_with(config.clone(), world_with_center(fast), 1);
    simulation.step();
    assert_eq!(
        only_individual(&simulation).1.health,
        50 - config.turn_cost - 3
    );

    let mut simulation =
        simulation_with(config.clone(), world_with_center(adult(1, Male, 100.0)), 1);
    simulation.step();
    assert_eq!(only_individual(&simulation).1.health, 50 - config.turn_cost);
}

// ---------- Fighting ----------

const LEFT: Position = Position { x: 1, y: 2 };
const RIGHT: Position = Position { x: 3, y: 2 };

fn individual_with_id(simulation: &Simulation, id: u64) -> Option<Individual> {
    simulation
        .world()
        .individuals()
        .find(|(_, individual)| individual.id == id)
        .map(|(_, individual)| individual.clone())
}

/// A reckless attacker next to a smart victim, which never starts a fight itself.
fn duel() -> World {
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, adult(1, Male, 0.0));
    world.place_individual(RIGHT, adult(2, Male, 100.0));
    world
}

#[test]
fn an_unprovoked_attack_hurts_the_victim_and_costs_the_attacker_smartness() {
    let config = Config {
        random_fight_chance: 1.0,
        ..scenario_config()
    };
    let attacker = adult(1, Male, 20.0);
    let mut world = World::new(5, 5);
    // A chance of 1 at smartness 0 is 0.8 at smartness 20, so give it several tries.
    world.place_individual(CENTER, attacker);
    world.place_individual(RIGHT, adult(2, Male, 100.0));
    let mut simulation = simulation_with(config.clone(), world, 1);
    while simulation.latest().events.fights == 0 {
        simulation.step();
    }
    let attacker = individual_with_id(&simulation, 1).unwrap();
    assert_eq!(attacker.smartness, 20.0 - config.fight_smartness_loss);
    assert_eq!(attacker.innate_smartness, 20.0);
    assert_eq!(simulation.latest().events.fights, 1);
}

#[test]
fn the_exact_cost_of_a_blow() {
    let config = Config {
        random_fight_chance: 1.0,
        ..scenario_config()
    };
    let mut simulation = simulation_with(config.clone(), duel(), 1);
    simulation.step();
    let victim = individual_with_id(&simulation, 2).unwrap();
    let attacker = individual_with_id(&simulation, 1).unwrap();
    assert_eq!(attacker.smartness, 0.0, "smartness never goes below zero");
    assert!(victim.health < 50 - config.fight_damage + 1);
    assert_eq!(simulation.latest().events.fights, 1);
    assert_eq!(
        attacker.health,
        50 - config.turn_cost,
        "the attacker takes no damage"
    );
}

#[test]
fn maximum_smartness_never_attacks_without_a_reason() {
    let config = Config {
        random_fight_chance: 1.0,
        ..scenario_config()
    };
    for seed in 0..SEEDS {
        let mut world = World::new(5, 5);
        world.place_individual(CENTER, adult(1, Male, 100.0));
        world.place_individual(RIGHT, adult(2, Male, 100.0));
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        assert_eq!(simulation.latest().events.fights, 0);
    }
}

#[test]
fn unprovoked_attacks_are_more_frequent_for_the_less_smart() {
    let config = Config {
        random_fight_chance: 0.5,
        ..scenario_config()
    };
    let fights_at = |smartness: f32| -> u32 {
        (0..400)
            .map(|seed| {
                let mut world = World::new(5, 5);
                world.place_individual(CENTER, adult(1, Male, smartness));
                world.place_individual(RIGHT, adult(2, Female, 100.0));
                let mut simulation = simulation_with(config.clone(), world, seed);
                simulation.step();
                simulation.latest().events.fights
            })
            .sum()
    };
    let (reckless, careful) = (fights_at(0.0), fights_at(80.0));
    assert!(reckless > careful * 2, "{reckless} vs {careful}");
}

#[test]
fn nobody_to_hit_means_no_fight() {
    let config = Config {
        random_fight_chance: 1.0,
        ..scenario_config()
    };
    let mut simulation = simulation_with(config, world_with_center(adult(1, Male, 0.0)), 1);
    simulation.step();
    assert_eq!(simulation.latest().events.fights, 0);
    assert_eq!(only_individual(&simulation).1.smartness, 0.0);
}

/// Regression guard for the turn loop: the victim must not vanish during another turn.
#[test]
fn a_fatal_hit_kills_exactly_once_and_on_the_victims_own_turn() {
    let config = Config {
        random_fight_chance: 1.0,
        ..scenario_config()
    };
    for seed in 0..SEEDS {
        let mut world = duel();
        if let Some(victim) = world.individual_mut(RIGHT) {
            victim.health = 1;
        }
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        simulation.step();

        assert!(individual_with_id(&simulation, 2).is_none());
        let history = simulation.history();
        let total = |read: fn(&genetic_algo::stats::TickEvents) -> u32| -> u32 {
            history.iter().map(|h| read(&h.events)).sum()
        };
        // The victim may also starve first if it acts before the blow lands.
        assert_eq!(total(|e| e.combat) + total(|e| e.starved), 1, "seed {seed}");
        assert_eq!(simulation.latest().population, 1);
    }
}

#[test]
fn a_dying_individual_is_not_hit_again_or_able_to_act() {
    let config = Config {
        random_fight_chance: 1.0,
        fight_damage: 100,
        ..scenario_config()
    };
    for seed in 0..SEEDS {
        let mut world = duel();
        world.place_individual(LEFT, adult(3, Male, 0.0));
        world.set(Position { x: 3, y: 3 }, Cell::Food);
        let mut simulation = simulation_with(config.clone(), world, seed);
        for _ in 0..4 {
            simulation.step();
        }
        // Each 100-damage blow fells exactly one individual, which has either died on
        // its turn or is still waiting for it. So nobody was struck after reaching 0.
        let events = simulation.history();
        let fights: u32 = events.iter().map(|h| h.events.fights).sum();
        let combat: u32 = events.iter().map(|h| h.events.combat).sum();
        let waiting = simulation
            .world()
            .individuals()
            .filter(|(_, individual)| individual.health == 0)
            .count() as u32;
        assert_eq!(fights, combat + waiting, "seed {seed}");
    }
}

fn food_race(first_health: u32, second_health: u32) -> World {
    let mut world = World::new(5, 5);
    let mut first = adult(1, Male, 100.0);
    let mut second = adult(2, Male, 100.0);
    first.health = first_health;
    second.health = second_health;
    // Both touch the food and each other, so either can strike the other.
    world.place_individual(LEFT, first);
    world.place_individual(Position { x: 2, y: 1 }, second);
    world.set(CENTER, Cell::Food);
    world
}

#[test]
fn hungry_rivals_fight_and_the_winner_gets_the_food() {
    let config = Config {
        contest_fight_chance: 1.0,
        ..scenario_config()
    };
    for seed in 0..SEEDS {
        let mut simulation = simulation_with(config.clone(), food_race(50, 50), seed);
        simulation.step();
        let events = simulation.latest().events;
        assert_eq!(events.fights, 1, "seed {seed}");
        assert_eq!(events.meals, 1, "seed {seed}");
        let Cell::Individual(winner) = simulation.world().get(CENTER) else {
            panic!("the winner stands where the food was");
        };
        assert_eq!(winner.health, 50 + config.food_gain);
        assert_eq!(winner.smartness, 100.0 - config.fight_smartness_loss);
        let loser_id = 3 - winner.id;
        let loser = individual_with_id(&simulation, loser_id).unwrap();
        assert_eq!(loser.health, 50 - config.fight_damage - config.turn_cost);
        assert_eq!(loser.smartness, 100.0);
    }
}

#[test]
fn a_sated_rival_is_not_a_rival() {
    let config = Config {
        contest_fight_chance: 1.0,
        hungry_below_health: 40,
        ..scenario_config()
    };
    // The hungry one eats; the other has no interest in the food, so no fight starts.
    // Several seeds, because the outcome depends on who acts first.
    for seed in 0..SEEDS {
        let mut simulation = simulation_with(config.clone(), food_race(30, 50), seed);
        simulation.step();
        assert_eq!(simulation.latest().events.fights, 0, "seed {seed}");
        assert_eq!(simulation.latest().events.meals, 1, "seed {seed}");
    }
}

#[test]
fn a_rival_that_cannot_reach_the_food_is_left_alone() {
    let config = Config {
        contest_fight_chance: 1.0,
        ..scenario_config()
    };
    let mut world = World::new(5, 5);
    world.place_individual(CENTER, adult(1, Male, 100.0));
    world.place_individual(LEFT, adult(2, Male, 100.0));
    world.set(RIGHT, Cell::Food);
    // Walled in, so the rival cannot wander within reach of the food first.
    for position in world.ring(LEFT, 1) {
        if position != CENTER {
            world.set(position, Cell::Wall);
        }
    }
    let mut simulation = simulation_with(config, world, 1);
    simulation.step();
    assert_eq!(simulation.latest().events.fights, 0);
}

#[test]
fn contests_can_be_switched_off() {
    let mut simulation = simulation_with(scenario_config(), food_race(50, 50), 1);
    simulation.step();
    assert_eq!(simulation.latest().events.fights, 0);
    assert_eq!(simulation.latest().events.meals, 1);
}

// ---------- Evolved brains ----------

/// A brain whose only path runs from one input through one hidden unit to the score.
fn brain_that_loves(input: usize) -> Brain {
    let mut weights = [0_i8; WEIGHTS];
    weights[input] = 32;
    weights[INPUTS * HIDDEN + HIDDEN] = 32;
    Brain::from_weights(weights)
}

const FOOD_INPUT: usize = 1;
const HAZARD_INPUT: usize = 3;
const INDIVIDUAL_INPUT: usize = 4;

fn with_brain(mut individual: Individual, brain: Brain) -> Individual {
    individual.brain = Some(Box::new(brain));
    individual
}

fn brain_rules() -> Config {
    Config {
        brains_enabled: true,
        ..scenario_config()
    }
}

#[test]
fn a_brain_chooses_what_its_weights_favour() {
    for seed in 0..SEEDS {
        let mut sated = adult(1, Male, 100.0);
        sated.health = 90;
        let mut world = world_with_center(with_brain(sated, brain_that_loves(FOOD_INPUT)));
        world.set(RIGHT, Cell::Food);
        let mut simulation = simulation_with(brain_rules(), world, seed);
        simulation.step();
        // The rules would not eat when this full; this brain always does.
        assert_eq!(simulation.latest().events.meals, 1, "seed {seed}");
        assert_eq!(simulation.latest().events.overfed, 1, "seed {seed}");
    }
}

#[test]
fn a_brain_can_walk_into_a_hazard_the_rules_would_avoid() {
    for seed in 0..SEEDS {
        let mut world = world_with_center(with_brain(
            adult(1, Male, 100.0),
            brain_that_loves(HAZARD_INPUT),
        ));
        world.set(RIGHT, Cell::Hazard);
        let mut simulation = simulation_with(brain_rules(), world, seed);
        simulation.step();
        let (_, individual) = only_individual(&simulation);
        assert_eq!(individual.health, 50 - 15 - 1, "seed {seed}");
    }
}

#[test]
fn a_brain_attacks_only_when_unprovoked_fights_are_allowed() {
    // Both are at smartness 100, so the unprovoked-fight rule itself never fires: any
    // fight is the brain's choice. The victim is walled in, so it cannot walk away first.
    for (random_fight_chance, expected) in [(0.0, 0), (0.001, 1)] {
        let config = Config {
            random_fight_chance,
            ..brain_rules()
        };
        for seed in 0..SEEDS {
            let mut world = World::new(5, 5);
            for position in world.ring(RIGHT, 1) {
                world.set(position, Cell::Wall);
            }
            let attacker = with_brain(adult(1, Male, 100.0), brain_that_loves(INDIVIDUAL_INPUT));
            world.place_individual(CENTER, attacker);
            world.place_individual(RIGHT, adult(2, Male, 100.0));
            let mut simulation = simulation_with(config.clone(), world, seed);
            simulation.step();
            assert_eq!(simulation.latest().events.fights, expected, "seed {seed}");
        }
    }
}

#[test]
fn without_thinking_the_brain_is_not_used() {
    // A brain that loves hazards would touch the one beside it on every turn. Acting at
    // random instead, an individual touches it only on some turns.
    let mut always_hurt = 0;
    for seed in 0..SEEDS {
        let thoughtless = with_brain(adult(1, Male, 0.0), brain_that_loves(HAZARD_INPUT));
        let mut world = world_with_center(thoughtless);
        world.set(RIGHT, Cell::Hazard);
        let mut simulation = simulation_with(brain_rules(), world, seed);
        for _ in 0..3 {
            simulation.step();
        }
        let hurt_every_turn = 50 - 3 * (15 + 1);
        if simulation
            .world()
            .individuals()
            .all(|(_, i)| i.health == hurt_every_turn)
        {
            always_hurt += 1;
        }
    }
    assert!(always_hurt < SEEDS / 4, "{always_hurt} of {SEEDS}");
}

#[test]
fn with_the_switch_off_a_brain_is_ignored() {
    let config = scenario_config();
    for seed in 0..SEEDS {
        let mut world = world_with_center(with_brain(
            adult(1, Male, 100.0),
            brain_that_loves(HAZARD_INPUT),
        ));
        world.set(RIGHT, Cell::Hazard);
        let mut simulation = simulation_with(config.clone(), world, seed);
        simulation.step();
        assert_eq!(
            only_individual(&simulation).1.health,
            50 - 1,
            "the rules avoid hazards"
        );
    }
}

#[test]
fn children_inherit_brains_and_the_control_draws_fresh_ones() {
    for inherited in [true, false] {
        let config = Config {
            brains_inherited: inherited,
            brain_mutation_rate: 0.0,
            ..brain_rules()
        };
        let parent_brain = brain_that_loves(FOOD_INPUT);
        let mut world = World::new(5, 5);
        world.place_individual(
            CENTER,
            with_brain(adult(1, Male, 50.0), parent_brain.clone()),
        );
        world.place_individual(
            RIGHT,
            with_brain(adult(2, Female, 50.0), parent_brain.clone()),
        );
        let mut simulation = simulation_with(config, world, 1);
        simulation.step();
        let baby = simulation
            .world()
            .individuals()
            .find(|(_, individual)| individual.age == 0)
            .map(|(_, individual)| individual.clone())
            .expect("a birth");
        let brain = baby.brain.expect("brains are on");
        assert_eq!(*brain == parent_brain, inherited);
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
    let config = Config {
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
