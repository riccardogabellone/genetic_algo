//! Population measurements, recorded once per tick.

use crate::{
    individual::{DeathCause, Gender},
    leadership::Policy,
    world::World,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmartnessSummary {
    pub mean: f32,
    pub min: f32,
    pub max: f32,
    /// Mean of the heritable part. Falls below `mean` only through fights.
    pub innate_mean: f32,
}

/// What happened during one tick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TickEvents {
    pub births: u32,
    pub meals: u32,
    pub starved: u32,
    pub overfed: u32,
    pub old_age: u32,
    pub hazard: u32,
    pub combat: u32,
    pub fights: u32,
    pub elections: u32,
}

impl TickEvents {
    pub fn record_death(&mut self, cause: DeathCause) {
        match cause {
            DeathCause::Starved => self.starved += 1,
            DeathCause::Overfed => self.overfed += 1,
            DeathCause::OldAge => self.old_age += 1,
            DeathCause::Hazard => self.hazard += 1,
            DeathCause::Combat => self.combat += 1,
        }
    }

    pub fn deaths(&self) -> u32 {
        self.starved + self.overfed + self.old_age + self.hazard + self.combat
    }

    pub fn add(&mut self, other: &Self) {
        self.births += other.births;
        self.meals += other.meals;
        self.starved += other.starved;
        self.overfed += other.overfed;
        self.old_age += other.old_age;
        self.hazard += other.hazard;
        self.combat += other.combat;
        self.fights += other.fights;
        self.elections += other.elections;
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickStats {
    pub tick: u32,
    pub population: usize,
    pub males: usize,
    pub females: usize,
    /// `None` once the population is extinct: there is nothing to measure.
    pub smartness: Option<SmartnessSummary>,
    /// `None` once the population is extinct.
    pub mean_speed: Option<f32>,
    /// The population's average policy. `None` once extinct.
    pub mean_policy: Option<Policy>,
    /// The id of the current leader, if there is one.
    pub leader: Option<u64>,
    pub max_generation: u32,
    pub events: TickEvents,
}

impl TickStats {
    pub fn measure(tick: u32, world: &World, events: TickEvents) -> Self {
        let (mut males, mut females, mut max_generation, mut total_speed) = (0, 0, 0, 0);
        let (mut sum, mut min, mut max) = (0.0_f64, f32::INFINITY, f32::NEG_INFINITY);
        let mut innate_sum = 0.0_f64;
        let (mut aggression_sum, mut appetite_sum) = (0.0_f64, 0.0_f64);
        for (_, individual) in world.individuals() {
            match individual.gender {
                Gender::Male => males += 1,
                Gender::Female => females += 1,
            }
            total_speed += usize::from(individual.speed);
            sum += f64::from(individual.smartness);
            innate_sum += f64::from(individual.innate_smartness);
            aggression_sum += f64::from(individual.civics.policy.aggression);
            appetite_sum += f64::from(individual.civics.policy.appetite);
            min = min.min(individual.smartness);
            max = max.max(individual.smartness);
            max_generation = max_generation.max(individual.generation);
        }
        let population = males + females;
        Self {
            tick,
            population,
            males,
            females,
            smartness: (population > 0).then(|| SmartnessSummary {
                mean: (sum / population as f64) as f32,
                min,
                max,
                innate_mean: (innate_sum / population as f64) as f32,
            }),
            mean_speed: (population > 0).then(|| total_speed as f32 / population as f32),
            mean_policy: (population > 0).then(|| Policy {
                aggression: (aggression_sum / population as f64) as f32,
                appetite: (appetite_sum / population as f64) as f32,
            }),
            leader: None,
            max_generation,
            events,
        }
    }
}
