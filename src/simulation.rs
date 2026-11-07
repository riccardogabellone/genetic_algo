//! Advances the world one tick at a time and records its history.

use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};

use crate::{
    behavior::{Action, decide},
    config::{Config, ConfigError, MAX_SMARTNESS},
    individual::{DeathCause, Gender, Individual},
    leadership::Leadership,
    sampling::stochastic_round,
    stats::{TickEvents, TickStats},
    world::{Cell, Position, World},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Extinct,
    TargetReached,
    TicksExhausted,
}

pub struct Simulation {
    config: Config,
    world: World,
    rng: StdRng,
    next_id: u64,
    history: Vec<TickStats>,
    leadership: Leadership,
}

/// The running costs of a big brain and of fast legs.
fn upkeep_due(individual: &Individual, moved: bool, config: &Config, rng: &mut StdRng) -> u32 {
    let brain = individual.smartness / MAX_SMARTNESS * config.smartness_upkeep;
    let legs = if moved {
        f32::from(individual.speed - 1) * config.speed_upkeep
    } else {
        0.0
    };
    stochastic_round(rng, brain + legs)
}

impl Simulation {
    /// The same config and seed always replay the same run.
    pub fn new(config: Config, seed: u64) -> Result<Self, ConfigError> {
        config.validate()?;
        let mut simulation = Self {
            world: World::new(config.width, config.height),
            rng: StdRng::seed_from_u64(seed),
            next_id: 0,
            history: Vec::new(),
            leadership: Leadership::default(),
            config,
        };
        simulation.populate();
        simulation.scatter_elements();
        simulation.record(TickEvents::default());
        Ok(simulation)
    }

    /// Replaces the starting layout, for scripted scenarios. Tick 0 is re-measured.
    #[must_use]
    pub fn with_world(mut self, world: World) -> Self {
        self.next_id = world
            .individuals()
            .map(|(_, i)| i.id + 1)
            .max()
            .unwrap_or(0);
        self.world = world;
        self.leadership = Leadership::default();
        self.history.clear();
        self.record(TickEvents::default());
        self
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Changes the rules mid-run. Living individuals keep the traits they already have.
    /// The map size is fixed for the life of a simulation.
    pub fn set_config(&mut self, config: Config) -> Result<(), ConfigError> {
        config.validate()?;
        if (config.width, config.height) != (self.config.width, self.config.height) {
            return Err(ConfigError("the map size cannot change mid-run".to_owned()));
        }
        if !config.leaders_enabled {
            self.leadership = Leadership::default();
        }
        self.config = config;
        Ok(())
    }

    pub fn leadership(&self) -> &Leadership {
        &self.leadership
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn history(&self) -> &[TickStats] {
        &self.history
    }

    pub fn latest(&self) -> &TickStats {
        self.history.last().expect("tick 0 is recorded at creation")
    }

    /// `Some` once the run is over. Extinction outranks everything else.
    pub fn outcome(&self) -> Option<Outcome> {
        let latest = self.latest();
        if latest.population == 0 {
            return Some(Outcome::Extinct);
        }
        let on_target = latest.population >= self.config.min_population_for_target()
            && latest
                .smartness
                .is_some_and(|smartness| smartness.mean >= self.config.target_smartness);
        if on_target {
            Some(Outcome::TargetReached)
        } else if latest.tick >= self.config.max_ticks {
            Some(Outcome::TicksExhausted)
        } else {
            None
        }
    }

    /// Runs until an outcome is reached.
    pub fn run_to_end(&mut self) -> Outcome {
        loop {
            if let Some(outcome) = self.outcome() {
                return outcome;
            }
            self.step();
        }
    }

    /// One tick: everyone alive at the start acts exactly once, in a fresh random
    /// order so no corner of the map is favoured, then food regrows.
    pub fn step(&mut self) {
        let mut events = TickEvents::default();
        self.hold_election_if_due(&mut events);

        let mut turn_order: Vec<Position> = self.world.individuals().map(|(p, _)| p).collect();
        turn_order.shuffle(&mut self.rng);

        for position in turn_order {
            self.take_turn(position, &mut events);
        }

        let population = self.world.individuals().count();
        self.leadership.observe_tick(events.fights, population);

        let tick = self.latest().tick + 1;
        self.regrow_food(tick);
        self.record(events);
    }

    fn hold_election_if_due(&mut self, events: &mut TickEvents) {
        let tick = self.latest().tick;
        if self.leadership.election_due(tick, &self.config)
            && self
                .leadership
                .hold_election(&mut self.world, tick, &self.config, &mut self.rng)
        {
            events.elections += 1;
        }
    }

    /// Nobody else can move or remove an individual, and nothing can enter its cell while
    /// it stands there, so each scheduled turn finds its individual still in place. A
    /// fatal hit leaves the victim at 0 health, and it dies when its own turn comes.
    fn take_turn(&mut self, position: Position, events: &mut TickEvents) {
        let Some(mut individual) = self.world.take_individual(position) else {
            unreachable!("individuals only leave their cell on their own turn");
        };
        // A fatal hit from another individual's turn takes effect here.
        if individual.health == 0 {
            events.record_death(DeathCause::Combat);
            self.leadership.record_death(individual.id);
            return;
        }
        let action = decide(
            &individual,
            position,
            &self.world,
            self.leadership.policy(),
            &self.config,
            &mut self.rng,
        );
        let mut destination = position;
        let mut ate = false;
        let mut touched_hazard = false;
        // Counted down only on turns that did not start a new cooldown.
        if !matches!(action, Action::Mate { .. }) {
            individual.mating_cooldown = individual.mating_cooldown.saturating_sub(1);
        }
        match action {
            Action::Mate { partner, nursery } => {
                self.give_birth(&mut individual, partner, nursery);
                events.births += 1;
            }
            Action::Eat(food) => {
                destination = food;
                ate = true;
                events.meals += 1;
            }
            Action::FightForFood { food, victim } => {
                self.strike(&mut individual, victim, events);
                destination = food;
                ate = true;
                events.meals += 1;
            }
            Action::Attack { victim } => self.strike(&mut individual, victim, events),
            Action::Move(free_cell) => {
                destination = free_cell;
            }
            Action::TouchHazard => touched_hazard = true,
            Action::Stay => {}
        }

        let config = &self.config;
        if touched_hazard {
            individual.health = individual.health.saturating_sub(config.hazard_damage);
        }
        if ate {
            individual.health = individual.health.saturating_add(config.food_gain);
        } else {
            individual.health = individual.health.saturating_sub(config.turn_cost);
        }
        let upkeep = upkeep_due(&individual, destination != position, config, &mut self.rng);
        individual.health = individual.health.saturating_sub(upkeep);
        individual.age += 1;

        let cause = individual.death_cause(config).map(|cause| match cause {
            DeathCause::Starved if touched_hazard => DeathCause::Hazard,
            other => other,
        });
        match cause {
            Some(cause) => {
                events.record_death(cause);
                self.leadership.record_death(individual.id);
                // A meal is consumed even if it was the fatal one.
                if ate {
                    self.world.set(destination, Cell::Empty);
                }
            }
            None => self.world.place_individual(destination, individual),
        }
    }

    fn strike(&mut self, attacker: &mut Individual, victim: Position, events: &mut TickEvents) {
        let Some(target) = self.world.individual_mut(victim) else {
            unreachable!("the victim was seen on the grid during this turn");
        };
        target.health = target.health.saturating_sub(self.config.fight_damage);
        attacker.smartness = (attacker.smartness - self.config.fight_smartness_loss).max(0.0);
        events.fights += 1;
    }

    fn give_birth(&mut self, parent: &mut Individual, partner: Position, nursery: Position) {
        let id = self.next_id;
        self.next_id += 1;
        let (mating_cost, cooldown) = (self.config.mating_cost, self.config.mating_cooldown);

        let Some(other_parent) = self.world.individual_mut(partner) else {
            unreachable!("the partner was seen on the grid during this turn");
        };
        other_parent.health -= mating_cost;
        other_parent.mating_cooldown = cooldown;
        let other_parent = other_parent.clone();

        parent.health -= mating_cost;
        parent.mating_cooldown = cooldown;

        let baby = Individual::offspring(id, parent, &other_parent, &self.config, &mut self.rng);
        self.world.place_individual(nursery, baby);
    }

    fn populate(&mut self) {
        let count = self.config.initial_population;
        let positions = self.world.sample_empty(count, &mut self.rng);
        for (index, position) in positions.into_iter().enumerate() {
            let gender = if index < count / 2 {
                Gender::Male
            } else {
                Gender::Female
            };
            let id = self.next_id;
            self.next_id += 1;
            let founder = Individual::founder(id, gender, &self.config, &mut self.rng);
            self.world.place_individual(position, founder);
        }
    }

    fn scatter_elements(&mut self) {
        for (cell, amount) in [
            (Cell::Food, self.config.initial_food),
            (Cell::Wall, self.config.wall_count),
            (Cell::Hazard, self.config.hazard_count),
        ] {
            self.world.scatter(&cell, amount, &mut self.rng);
        }
    }

    fn regrow_food(&mut self, tick: u32) {
        if !tick.is_multiple_of(self.config.food_regrowth_interval) {
            return;
        }
        let present = self.world.count(&Cell::Food);
        let room = self.config.max_food.saturating_sub(present);
        let amount = self.config.food_regrowth_amount.min(room);
        self.world.scatter(&Cell::Food, amount, &mut self.rng);
    }

    fn record(&mut self, events: TickEvents) {
        let tick = self.history.last().map_or(0, |latest| latest.tick + 1);
        let mut stats = TickStats::measure(tick, &self.world, events);
        stats.leader = self.leadership.leader();
        self.history.push(stats);
    }
}
