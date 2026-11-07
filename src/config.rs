//! Every tunable rule of the simulation, validated once at start-up.

use std::fmt;

use crate::{
    leadership::{MAX_AGGRESSION, TermJudgement},
    sampling::Spread,
};

/// Smartness is a score in `0..=MAX_SMARTNESS`.
pub const MAX_SMARTNESS: f32 = 100.0;
/// Upper bound on the grid, so a typo cannot exhaust memory.
pub const MAX_CELLS: usize = 1 << 24;
/// Upper bound on run length; one history entry is kept per tick.
pub const MAX_TICKS: u32 = 10_000_000;
/// Upper bound on the health scale; every value up to here is exact in an `f32`.
pub const MAX_HEALTH_SCALE: u32 = 1_000_000;
/// Upper bound on vision and speed, which set how many cells a turn may scan.
pub const MAX_RADIUS: usize = 64;

/// Upper bounds that keep an election's cost reasonable on a crowded map.
pub const MAX_BALLOT: usize = 64;
pub const MAX_MEMORY: usize = 64;
/// Keeps learning numerically sane.
pub const MAX_LEARNING_RATE: f32 = 100.0;

pub const DEFAULT_WIDTH: usize = 32;
pub const DEFAULT_HEIGHT: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    // World
    pub width: usize,
    pub height: usize,
    pub initial_population: usize,
    pub initial_food: usize,
    pub wall_count: usize,
    pub hazard_count: usize,
    pub food_regrowth_interval: u32,
    pub food_regrowth_amount: usize,
    pub max_food: usize,
    pub max_ticks: u32,

    // Smartness
    pub target_smartness: f32,
    /// Below this score an individual always acts at random.
    pub min_smartness_for_decisions: f32,
    /// Experimental control: `false` makes smartness have no effect on behaviour.
    pub smartness_affects_behavior: bool,
    /// Extra health per turn at maximum smartness, proportional below it. 0 disables it.
    pub smartness_upkeep: f32,
    /// How far maximum smartness can see food, proportional below it. 0 disables it.
    pub max_vision: usize,

    // Founders
    pub founder_smartness: Spread,
    pub founder_reproduction: Spread,
    pub founder_health: Spread,

    // Inheritance
    pub mutation_rate: f32,
    pub smartness_mutation_step: f32,
    pub reproduction_mutation_step: f32,

    // Health. Reaching 0 or `max_health` kills.
    pub max_health: u32,
    pub hungry_below_health: u32,
    pub food_gain: u32,
    pub turn_cost: u32,
    pub hazard_damage: u32,

    // Fighting
    /// Chance that a hungry individual strikes a rival racing for the same food.
    pub contest_fight_chance: f32,
    /// Chance per turn of an unprovoked attack, at smartness 0. Falls to 0 at maximum smartness.
    pub random_fight_chance: f32,
    /// Health the victim loses. A hit that leaves 0 kills on the victim's next turn.
    pub fight_damage: u32,
    /// Current smartness the attacker loses. Never inherited.
    pub fight_smartness_loss: f32,

    // Leaders
    /// Adults elect a leader whose policy followers sometimes adopt. Off by default.
    pub leaders_enabled: bool,
    /// Ticks between elections. A leader's death calls one at once.
    pub election_interval: u32,
    /// Candidates drawn at random per election, plus the incumbent if eligible.
    pub ballot_size: usize,
    /// How many times one individual can be elected in its life.
    pub max_terms: u32,
    /// Chance per turn that a follower acts on the leader's policy instead of its own.
    pub leader_influence: f32,
    /// Past elections recalled at maximum smartness, proportional below it.
    pub max_memory: usize,
    /// What voters judge a finished term by.
    pub term_judgement: TermJudgement,
    pub founder_aggression: Spread,
    pub founder_appetite: Spread,
    /// Largest policy mutation, as a share of each scale. Happens at `mutation_rate`.
    pub policy_mutation_step: f32,

    // Evolved brains
    /// When an individual thinks, an inherited neural network chooses instead of the
    /// hand-written rules. Off by default.
    pub brains_enabled: bool,
    /// Experimental control: `false` gives every child a fresh random brain.
    pub brains_inherited: bool,
    /// Chance that each weight mutates in a child.
    pub brain_mutation_rate: f32,
    /// Largest mutation of one weight, in steps of 1/16.
    pub brain_mutation_step: u8,
    /// Brains also learn from experience during their life. Needs brains. Off by default.
    pub learning_enabled: bool,
    pub learning_rate: f32,

    // Life cycle
    pub min_lifespan: u32,
    pub max_lifespan: u32,
    pub maturity_age: u32,
    /// Health each parent gives up per birth; the newborn receives both shares.
    pub mating_cost: u32,
    pub mating_cooldown: u32,
    /// Cells moved per turn: destinations form a square ring of `8 * speed` cells.
    pub initial_speed: usize,
    /// Speed mutates by one step within `1..=max_speed`. Equal to `initial_speed`, it is fixed.
    pub max_speed: usize,
    /// Extra health per point of speed above 1, charged on turns that change cell.
    pub speed_upkeep: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self::for_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
    }
}

impl Config {
    /// Default rules, with element counts proportional to the grid area.
    pub fn for_size(width: usize, height: usize) -> Self {
        let cells = width.saturating_mul(height);
        Self {
            width,
            height,
            initial_population: cells / 8,
            initial_food: cells / 4,
            wall_count: cells / 32,
            hazard_count: cells / 64,
            food_regrowth_interval: 1,
            food_regrowth_amount: (cells / 32).max(1),
            max_food: cells / 4,
            max_ticks: 10_000,

            target_smartness: 80.0,
            min_smartness_for_decisions: 1.0,
            smartness_affects_behavior: true,
            smartness_upkeep: 0.0,
            max_vision: 0,

            founder_smartness: Spread {
                min: 1.0,
                mode: 2.0,
                max: 5.0,
            },
            founder_reproduction: Spread {
                min: 0.4,
                mode: 0.7,
                max: 1.0,
            },
            founder_health: Spread {
                min: 25.0,
                mode: 50.0,
                max: 75.0,
            },

            mutation_rate: 0.5,
            smartness_mutation_step: 8.0,
            reproduction_mutation_step: 0.1,

            max_health: 100,
            hungry_below_health: 70,
            food_gain: 20,
            turn_cost: 1,
            hazard_damage: 15,

            contest_fight_chance: 0.0,
            random_fight_chance: 0.0,
            fight_damage: 10,
            fight_smartness_loss: 5.0,

            leaders_enabled: false,
            election_interval: 30,
            ballot_size: 5,
            max_terms: 2,
            leader_influence: 0.5,
            max_memory: 5,
            term_judgement: TermJudgement::Violence,
            founder_aggression: Spread {
                min: 0.0,
                mode: 1.0,
                max: 2.0,
            },
            founder_appetite: Spread {
                min: 0.4,
                mode: 0.7,
                max: 1.0,
            },
            policy_mutation_step: 0.1,

            brains_enabled: false,
            brains_inherited: true,
            brain_mutation_rate: 0.05,
            brain_mutation_step: 8,
            learning_enabled: false,
            learning_rate: 0.2,

            min_lifespan: 50,
            max_lifespan: 100,
            maturity_age: 5,
            mating_cost: 10,
            mating_cooldown: 3,
            initial_speed: 1,
            max_speed: 1,
            speed_upkeep: 0.0,
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_world()?;
        self.validate_traits()?;
        self.validate_fighting()?;
        self.validate_leaders()?;
        require(
            (0.0..=1.0).contains(&self.brain_mutation_rate),
            "brain_mutation_rate must be within 0..=1",
        )?;
        require(
            self.brain_mutation_step <= 127,
            "brain_mutation_step must be at most 127",
        )?;
        require(
            (0.0..=MAX_LEARNING_RATE).contains(&self.learning_rate),
            "learning_rate is out of range",
        )?;
        self.validate_life_cycle()
    }

    fn validate_world(&self) -> Result<(), ConfigError> {
        require(
            self.width > 0 && self.height > 0,
            "width and height must be positive",
        )?;
        let cells = self
            .width
            .checked_mul(self.height)
            .filter(|cells| *cells <= MAX_CELLS)
            .ok_or(ConfigError(format!(
                "the grid may have at most {MAX_CELLS} cells"
            )))?;
        let requested = [
            self.initial_population,
            self.initial_food,
            self.wall_count,
            self.hazard_count,
        ]
        .iter()
        .try_fold(0_usize, |sum, count| sum.checked_add(*count));
        require(
            requested.is_some_and(|total| total <= cells),
            "population and elements do not fit on the grid",
        )?;
        require(
            self.food_regrowth_interval > 0,
            "food_regrowth_interval must be positive",
        )?;
        require(self.max_ticks <= MAX_TICKS, "max_ticks is too large")
    }

    fn validate_traits(&self) -> Result<(), ConfigError> {
        require(
            self.max_health > 1 && self.max_health <= MAX_HEALTH_SCALE,
            "max_health is out of range",
        )?;
        require(
            (0.0..=MAX_SMARTNESS).contains(&self.target_smartness),
            "target_smartness must be within the smartness scale",
        )?;
        require(
            (0.0..=MAX_SMARTNESS).contains(&self.min_smartness_for_decisions),
            "min_smartness_for_decisions must be within the smartness scale",
        )?;
        require(
            within(self.founder_smartness, 0.0, MAX_SMARTNESS),
            "invalid founder_smartness",
        )?;
        require(
            within(self.founder_reproduction, 0.0, 1.0),
            "invalid founder_reproduction",
        )?;
        require(
            within(self.founder_health, 1.0, self.max_health as f32 - 1.0),
            "founder_health must stay strictly between 0 and max_health",
        )?;
        require(
            (0.0..=1.0).contains(&self.mutation_rate),
            "mutation_rate must be within 0..=1",
        )?;
        require(
            (0.0..=MAX_SMARTNESS).contains(&self.smartness_mutation_step),
            "smartness_mutation_step must be within the smartness scale",
        )?;
        require(
            (0.0..=1.0).contains(&self.reproduction_mutation_step),
            "reproduction_mutation_step must be within 0..=1",
        )
    }

    fn validate_fighting(&self) -> Result<(), ConfigError> {
        require(
            (0.0..=1.0).contains(&self.contest_fight_chance)
                && (0.0..=1.0).contains(&self.random_fight_chance),
            "fight chances must be within 0..=1",
        )?;
        require(
            self.fight_damage > 0 && self.fight_damage <= self.max_health,
            "fight_damage must be between 1 and max_health",
        )?;
        require(
            (0.0..=MAX_SMARTNESS).contains(&self.fight_smartness_loss),
            "fight_smartness_loss must be within the smartness scale",
        )
    }

    fn validate_leaders(&self) -> Result<(), ConfigError> {
        require(
            self.election_interval > 0,
            "election_interval must be positive",
        )?;
        require(
            self.ballot_size > 0 && self.ballot_size <= MAX_BALLOT,
            "ballot_size is out of range",
        )?;
        require(self.max_terms > 0, "max_terms must be positive")?;
        require(
            (0.0..=1.0).contains(&self.leader_influence),
            "leader_influence must be within 0..=1",
        )?;
        require(self.max_memory <= MAX_MEMORY, "max_memory is too large")?;
        require(
            within(self.founder_aggression, 0.0, MAX_AGGRESSION),
            "invalid founder_aggression",
        )?;
        require(
            within(self.founder_appetite, 0.0, 1.0),
            "invalid founder_appetite",
        )?;
        require(
            (0.0..=1.0).contains(&self.policy_mutation_step),
            "policy_mutation_step must be within 0..=1",
        )
    }

    fn validate_life_cycle(&self) -> Result<(), ConfigError> {
        require(self.turn_cost > 0, "turn_cost must be positive")?;
        require(self.hazard_damage > 0, "hazard_damage must be positive")?;
        require(
            self.min_lifespan > 0 && self.min_lifespan <= self.max_lifespan,
            "lifespans must satisfy 0 < min_lifespan <= max_lifespan",
        )?;
        require(
            self.mating_cost > 0,
            "mating_cost must be positive, or births create free energy",
        )?;
        require(
            self.mating_cost
                .checked_mul(2)
                .is_some_and(|share| share < self.max_health),
            "a newborn's health (2 * mating_cost) must be below max_health",
        )?;
        require(
            self.initial_speed > 0 && self.initial_speed <= self.max_speed,
            "speed must satisfy 0 < initial_speed <= max_speed",
        )?;
        require(
            self.max_speed <= MAX_RADIUS && self.max_vision <= MAX_RADIUS,
            "max_speed and max_vision are too large",
        )?;
        let upkeep_limit = self.max_health as f32;
        require(
            (0.0..=upkeep_limit).contains(&self.smartness_upkeep)
                && (0.0..=upkeep_limit).contains(&self.speed_upkeep),
            "upkeep values must be between 0 and max_health",
        )
    }

    /// A lone survivor hitting the target is luck, not an evolved population.
    pub fn min_population_for_target(&self) -> usize {
        (self.initial_population / 4).max(2)
    }
}

fn within(spread: Spread, low: f32, high: f32) -> bool {
    spread.is_valid() && spread.min >= low && spread.max <= high
}

fn require(condition: bool, message: &str) -> Result<(), ConfigError> {
    if condition {
        Ok(())
    } else {
        Err(ConfigError(message.to_owned()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid configuration: {}", self.0)
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn rejects(change: impl FnOnce(&mut Config)) -> bool {
        let mut config = Config::default();
        change(&mut config);
        config.validate().is_err()
    }

    #[test]
    fn defaults_are_valid_at_every_small_size() {
        for width in 1..=20 {
            for height in 1..=20 {
                let config = Config::for_size(width, height);
                assert_eq!(config.validate(), Ok(()), "{width}x{height}");
            }
        }
    }

    #[test]
    fn rejects_empty_or_huge_grids() {
        assert!(Config::for_size(0, 10).validate().is_err());
        assert!(Config::for_size(10, 0).validate().is_err());
        assert!(Config::for_size(usize::MAX, usize::MAX).validate().is_err());
        assert!(Config::for_size(MAX_CELLS, 2).validate().is_err());
    }

    #[test]
    fn rejects_contents_that_do_not_fit() {
        assert!(rejects(|c| c.initial_population = c.width * c.height + 1));
        assert!(rejects(|c| c.initial_food = usize::MAX));
        assert!(rejects(|c| {
            c.initial_population = usize::MAX;
            c.wall_count = usize::MAX;
        }));
    }

    #[test]
    fn rejects_values_that_would_panic_or_break_the_rules() {
        assert!(rejects(|c| c.food_regrowth_interval = 0));
        assert!(rejects(|c| c.max_ticks = MAX_TICKS + 1));
        assert!(rejects(|c| c.target_smartness = f32::NAN));
        assert!(rejects(|c| c.target_smartness = MAX_SMARTNESS + 1.0));
        assert!(rejects(|c| c.mutation_rate = 1.5));
        assert!(rejects(|c| c.smartness_mutation_step = f32::INFINITY));
        assert!(rejects(|c| c.smartness_mutation_step = f32::MAX));
        assert!(rejects(|c| c.reproduction_mutation_step = 2.0));
        assert!(rejects(|c| c.max_health = u32::MAX));
        assert!(rejects(|c| c.turn_cost = 0));
        assert!(rejects(|c| c.hazard_damage = 0));
        assert!(rejects(|c| c.founder_reproduction.max = 1.3));
        assert!(rejects(|c| c.founder_health.max = c.max_health as f32));
        assert!(rejects(|c| c.min_lifespan = 0));
        assert!(rejects(|c| c.min_lifespan = c.max_lifespan + 1));
        assert!(rejects(|c| c.mating_cost = 0));
        assert!(rejects(|c| c.mating_cost = c.max_health));
        assert!(rejects(|c| c.mating_cost = u32::MAX));
        assert!(rejects(|c| c.initial_speed = 0));
        assert!(rejects(|c| c.initial_speed = c.max_speed + 1));
        assert!(rejects(|c| {
            c.initial_speed = MAX_RADIUS + 1;
            c.max_speed = MAX_RADIUS + 1;
        }));
        assert!(rejects(|c| c.max_vision = MAX_RADIUS + 1));
        assert!(rejects(|c| c.smartness_upkeep = f32::NAN));
        assert!(rejects(|c| c.smartness_upkeep = -1.0));
        assert!(rejects(|c| c.speed_upkeep = f32::INFINITY));
        assert!(rejects(|c| c.contest_fight_chance = 1.5));
        assert!(rejects(|c| c.random_fight_chance = f32::NAN));
        assert!(rejects(|c| c.fight_damage = 0));
        assert!(rejects(|c| c.election_interval = 0));
        assert!(rejects(|c| c.ballot_size = 0));
        assert!(rejects(|c| c.ballot_size = MAX_BALLOT + 1));
        assert!(rejects(|c| c.max_terms = 0));
        assert!(rejects(|c| c.leader_influence = 1.5));
        assert!(rejects(|c| c.leader_influence = f32::NAN));
        assert!(rejects(|c| c.max_memory = MAX_MEMORY + 1));
        assert!(rejects(|c| c.founder_aggression.max = MAX_AGGRESSION + 1.0));
        assert!(rejects(|c| c.founder_appetite.max = 1.5));
        assert!(rejects(|c| c.policy_mutation_step = -0.1));
        assert!(rejects(|c| c.brain_mutation_rate = 1.5));
        assert!(rejects(|c| c.brain_mutation_rate = f32::NAN));
        assert!(rejects(|c| c.brain_mutation_step = 128));
        assert!(rejects(|c| c.learning_rate = -0.1));
        assert!(rejects(|c| c.learning_rate = f32::NAN));
        assert!(rejects(|c| c.learning_rate = MAX_LEARNING_RATE + 1.0));
        assert!(rejects(|c| c.fight_damage = c.max_health + 1));
        assert!(rejects(|c| c.fight_smartness_loss = -1.0));
        assert!(rejects(|c| c.fight_smartness_loss = MAX_SMARTNESS + 1.0));
    }
}
