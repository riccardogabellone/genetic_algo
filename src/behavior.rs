//! What an individual chooses to do with its turn. Reads the world, changes nothing.

use rand::{RngExt, seq::IndexedRandom};

use crate::{
    brain::{Brain, Features, best_option},
    config::{Config, MAX_SMARTNESS},
    individual::Individual,
    leadership::{Policy, Stance},
    world::{Cell, Position, World},
};

/// The rate of unprovoked attacks when smartness is switched off for a control run.
const CONTROL_RECKLESSNESS: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Mate {
        partner: Position,
        nursery: Position,
    },
    Eat(Position),
    /// Strikes a rival racing for the same food, then eats it.
    FightForFood {
        food: Position,
        victim: Position,
    },
    Attack {
        victim: Position,
    },
    Move(Position),
    TouchHazard,
    Stay,
}

/// Decides the turn of `individual`, which must already be off the grid at `position`.
pub fn decide(
    individual: &Individual,
    position: Position,
    world: &World,
    leader_policy: Option<Policy>,
    config: &Config,
    rng: &mut impl RngExt,
) -> Action {
    let stance = Stance::for_turn(individual, leader_policy, config, rng);
    let reachable = world.ring(position, usize::from(individual.speed));
    if let Some(action) = try_to_mate(individual, position, &reachable, world, config, rng) {
        return action;
    }
    if let Some(action) = try_random_attack(individual, stance, &reachable, world, config, rng) {
        return action;
    }
    if thinks_this_turn(individual, config, rng) {
        match individual.brain.as_deref() {
            Some(brain) if config.brains_enabled => use_brain(
                individual, brain, stance, position, &reachable, world, config, rng,
            ),
            _ => deliberate(individual, stance, position, &reachable, world, config, rng),
        }
    } else {
        act_at_random(individual, stance, &reachable, world, config, rng)
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

/// An attack for no reason, more likely the less smart the attacker is.
fn try_random_attack(
    individual: &Individual,
    stance: Stance,
    reachable: &[Position],
    world: &World,
    config: &Config,
    rng: &mut impl RngExt,
) -> Option<Action> {
    if config.random_fight_chance <= 0.0 {
        return None;
    }
    // In the control experiment smartness must have no effect, so the rate is flat.
    let recklessness = if config.smartness_affects_behavior {
        1.0 - individual.smartness / MAX_SMARTNESS
    } else {
        CONTROL_RECKLESSNESS
    };
    let chance = f64::from(config.random_fight_chance * recklessness * stance.aggression);
    if !rng.random_bool(chance.clamp(0.0, 1.0)) {
        return None;
    }
    let victims: Vec<Position> = reachable
        .iter()
        .copied()
        .filter(
            |position| matches!(world.get(*position), Cell::Individual(other) if other.health > 0),
        )
        .collect();
    let victim = *victims.choose(rng)?;
    Some(Action::Attack { victim })
}

/// Eats `food`, possibly striking a hungry rival that could also reach it.
fn eat(
    individual: &Individual,
    stance: Stance,
    food: Position,
    reachable: &[Position],
    world: &World,
    config: &Config,
    rng: &mut impl RngExt,
) -> Action {
    if config.contest_fight_chance <= 0.0 || individual.health >= stance.hungry_below {
        return Action::Eat(food);
    }
    let rivals: Vec<Position> = reachable
        .iter()
        .copied()
        .filter(|position| match world.get(*position) {
            Cell::Individual(other) => {
                other.health > 0
                    && other.health < Stance::own(other, config).hungry_below
                    && world
                        .ring(*position, usize::from(other.speed))
                        .contains(&food)
            }
            _ => false,
        })
        .collect();
    match rivals.choose(rng) {
        Some(victim)
            if rng.random_bool(
                f64::from(config.contest_fight_chance * stance.aggression).clamp(0.0, 1.0),
            ) =>
        {
            Action::FightForFood {
                food,
                victim: *victim,
            }
        }
        _ => Action::Eat(food),
    }
}

fn thinks_this_turn(individual: &Individual, config: &Config, rng: &mut impl RngExt) -> bool {
    config.smartness_affects_behavior
        && individual.smartness >= config.min_smartness_for_decisions
        && rng.random_bool(f64::from(individual.smartness / MAX_SMARTNESS).clamp(0.0, 1.0))
}

/// Eats when hungry, or walks toward food it can see. Otherwise moves to a free cell.
/// Never touches a hazard.
fn deliberate(
    individual: &Individual,
    stance: Stance,
    position: Position,
    reachable: &[Position],
    world: &World,
    config: &Config,
    rng: &mut impl RngExt,
) -> Action {
    if individual.health < stance.hungry_below {
        if let Some(food) = choose_cell(reachable, world, rng, |cell| *cell == Cell::Food) {
            return eat(individual, stance, food, reachable, world, config, rng);
        }
        let vision = vision_radius(individual.smartness, config);
        if let Some(step) = step_toward_food(position, vision, reachable, world, rng) {
            return Action::Move(step);
        }
    }
    choose_cell(reachable, world, rng, |cell| *cell == Cell::Empty)
        .map_or(Action::Stay, Action::Move)
}

/// Smarter individuals see farther.
pub fn vision_radius(smartness: f32, config: &Config) -> usize {
    (smartness / MAX_SMARTNESS * config.max_vision as f32).round() as usize
}

/// A free reachable cell strictly closer to the nearest visible food, if there is one.
fn step_toward_food(
    position: Position,
    vision: usize,
    reachable: &[Position],
    world: &World,
    rng: &mut impl RngExt,
) -> Option<Position> {
    let food = (1..=vision).find_map(|radius| {
        choose_cell(&world.ring(position, radius), world, rng, |cell| {
            *cell == Cell::Food
        })
    })?;
    let distance = |from: Position| {
        let (dx, dy) = (from.x.abs_diff(food.x), from.y.abs_diff(food.y));
        dx * dx + dy * dy
    };
    let closer: Vec<Position> = reachable
        .iter()
        .copied()
        .filter(|cell| *world.get(*cell) == Cell::Empty && distance(*cell) < distance(position))
        .collect();
    let best = closer.iter().map(|cell| distance(*cell)).min()?;
    let nearest: Vec<Position> = closer
        .into_iter()
        .filter(|cell| distance(*cell) == best)
        .collect();
    nearest.choose(rng).copied()
}

/// Picks any reachable cell and suffers the consequences.
/// Lets an evolved brain pick among staying put and every reachable cell. The cell then
/// decides what happens, exactly as for any other individual: a brain can choose food,
/// a hazard, or, when unprovoked fights are allowed, another individual to attack.
#[allow(clippy::too_many_arguments)]
fn use_brain(
    individual: &Individual,
    brain: &Brain,
    stance: Stance,
    position: Position,
    reachable: &[Position],
    world: &World,
    config: &Config,
    rng: &mut impl RngExt,
) -> Action {
    let hunger = 1.0 - individual.health as f32 / config.max_health as f32;
    let options: Vec<Option<Position>> = std::iter::once(None)
        .chain(reachable.iter().copied().map(Some))
        .collect();
    let scores: Vec<f32> = options
        .iter()
        .map(|option| brain.score(&features(*option, position, hunger, world)))
        .collect();
    let Some(Some(target)) = best_option(&scores, rng).map(|index| options[index]) else {
        return Action::Stay;
    };
    match world.get(target) {
        Cell::Empty => Action::Move(target),
        Cell::Food => eat(individual, stance, target, reachable, world, config, rng),
        Cell::Hazard => Action::TouchHazard,
        Cell::Individual(other) if config.random_fight_chance > 0.0 && other.health > 0 => {
            Action::Attack { victim: target }
        }
        Cell::Wall | Cell::Individual(_) => Action::Stay,
    }
}

/// What a brain sees of one option: the cell itself, or its own cell when staying.
fn features(option: Option<Position>, position: Position, hunger: f32, world: &World) -> Features {
    let around = option.unwrap_or(position);
    let share = |wanted: &Cell| world.count_around(around, wanted) as f32 / 8.0;
    let cell = option.map(|cell| world.get(cell));
    Features {
        empty: matches!(cell, Some(Cell::Empty)),
        food: matches!(cell, Some(Cell::Food)),
        wall: matches!(cell, Some(Cell::Wall)),
        hazard: matches!(cell, Some(Cell::Hazard)),
        individual: matches!(cell, Some(Cell::Individual(_))),
        stay: option.is_none(),
        hunger,
        food_nearby: share(&Cell::Food),
        hazards_nearby: share(&Cell::Hazard),
    }
}

fn act_at_random(
    individual: &Individual,
    stance: Stance,
    reachable: &[Position],
    world: &World,
    config: &Config,
    rng: &mut impl RngExt,
) -> Action {
    let Some(target) = reachable.choose(rng) else {
        return Action::Stay;
    };
    match world.get(*target) {
        Cell::Empty => Action::Move(*target),
        Cell::Food => eat(individual, stance, *target, reachable, world, config, rng),
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
