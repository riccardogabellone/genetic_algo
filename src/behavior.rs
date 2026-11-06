//! What an individual chooses to do with its turn. Reads the world, changes nothing.

use rand::{RngExt, seq::IndexedRandom};

use crate::{
    config::{Config, MAX_SMARTNESS},
    individual::Individual,
    world::{Cell, Position, World},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Mate {
        partner: Position,
        nursery: Position,
    },
    Eat(Position),
    Move(Position),
    TouchHazard,
    Stay,
}

/// Decides the turn of `individual`, which must already be off the grid at `position`.
pub fn decide(
    individual: &Individual,
    position: Position,
    world: &World,
    config: &Config,
    rng: &mut impl RngExt,
) -> Action {
    let reachable = world.ring(position, individual.speed);
    if let Some(action) = try_to_mate(individual, position, &reachable, world, config, rng) {
        return action;
    }
    if thinks_this_turn(individual, config, rng) {
        deliberate(individual, &reachable, world, config, rng)
    } else {
        act_at_random(&reachable, world, rng)
    }
}

/// Top priority for everyone, as long as a partner and room for the newborn exist.
fn try_to_mate(
    individual: &Individual,
    position: Position,
    reachable: &[Position],
    world: &World,
    config: &Config,
    rng: &mut impl RngExt,
) -> Option<Action> {
    if !individual.can_mate(config) {
        return None;
    }
    let partners: Vec<(Position, &Individual)> = reachable
        .iter()
        .filter_map(|position| match world.get(*position) {
            Cell::Individual(other)
                if other.gender != individual.gender && other.can_mate(config) =>
            {
                Some((*position, other.as_ref()))
            }
            _ => None,
        })
        .collect();
    let (partner_position, partner) = partners.choose(rng)?;
    let nursery = choose_cell(&world.ring(position, 1), world, rng, |cell| {
        *cell == Cell::Empty
    })?;
    let fertility = f64::from(f32::midpoint(individual.reproduction, partner.reproduction));
    rng.random_bool(fertility.clamp(0.0, 1.0))
        .then_some(Action::Mate {
            partner: *partner_position,
            nursery,
        })
}

fn thinks_this_turn(individual: &Individual, config: &Config, rng: &mut impl RngExt) -> bool {
    config.smartness_affects_behavior
        && individual.smartness >= config.min_smartness_for_decisions
        && rng.random_bool(f64::from(individual.smartness / MAX_SMARTNESS).clamp(0.0, 1.0))
}

/// Eats when hungry, otherwise moves to a free cell. Never touches a hazard.
fn deliberate(
    individual: &Individual,
    reachable: &[Position],
    world: &World,
    config: &Config,
    rng: &mut impl RngExt,
) -> Action {
    if individual.health < config.hungry_below_health
        && let Some(food) = choose_cell(reachable, world, rng, |cell| *cell == Cell::Food)
    {
        return Action::Eat(food);
    }
    choose_cell(reachable, world, rng, |cell| *cell == Cell::Empty)
        .map_or(Action::Stay, Action::Move)
}

/// Picks any reachable cell and suffers the consequences.
fn act_at_random(reachable: &[Position], world: &World, rng: &mut impl RngExt) -> Action {
    let Some(target) = reachable.choose(rng) else {
        return Action::Stay;
    };
    match world.get(*target) {
        Cell::Empty => Action::Move(*target),
        Cell::Food => Action::Eat(*target),
        Cell::Hazard => Action::TouchHazard,
        Cell::Wall | Cell::Individual(_) => Action::Stay,
    }
}

fn choose_cell(
    candidates: &[Position],
    world: &World,
    rng: &mut impl RngExt,
    wanted: impl Fn(&Cell) -> bool,
) -> Option<Position> {
    let matching: Vec<Position> = candidates
        .iter()
        .copied()
        .filter(|position| wanted(world.get(*position)))
        .collect();
    matching.choose(rng).copied()
}
