#![allow(dead_code)] // Each test binary uses a different part of this module.

use genetic_algo::{
    config::Config,
    individual::{Gender, Individual},
    simulation::Simulation,
    world::{Position, World},
};

pub const CENTER: Position = Position { x: 2, y: 2 };

/// Rules for scripted scenarios: an empty 5x5 map where food never regrows.
pub fn scenario_config() -> Config {
    Config {
        initial_population: 0,
        initial_food: 0,
        wall_count: 0,
        hazard_count: 0,
        food_regrowth_amount: 0,
        // Each scenario isolates one rule; the optional costs and senses opt in.
        smartness_upkeep: 0.0,
        max_vision: 0,
        max_speed: 8,
        speed_upkeep: 0.0,
        ..Config::for_size(5, 5)
    }
}

/// An adult in good condition. Smartness 100 always decides; 0 is always random.
pub fn adult(id: u64, gender: Gender, smartness: f32) -> Individual {
    Individual {
        id,
        generation: 0,
        gender,
        smartness,
        innate_smartness: smartness,
        reproduction: 1.0,
        health: 50,
        age: 10,
        lifespan: 90,
        speed: 1,
        mating_cooldown: 0,
    }
}

pub fn simulation_with(config: Config, world: World, seed: u64) -> Simulation {
    Simulation::new(config, seed)
        .expect("valid config")
        .with_world(world)
}

pub fn only_individual(simulation: &Simulation) -> (Position, Individual) {
    let found: Vec<_> = simulation.world().individuals().collect();
    assert_eq!(found.len(), 1, "expected exactly one individual");
    (found[0].0, found[0].1.clone())
}
