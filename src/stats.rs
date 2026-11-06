//! Population measurements, recorded once per tick.

use crate::{
    individual::{DeathCause, Gender},
    world::World,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SmartnessSummary {
    pub mean: f32,
    pub min: f32,
    pub max: f32,
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
}

impl TickEvents {
    pub fn record_death(&mut self, cause: DeathCause) {
        match cause {
            DeathCause::Starved => self.starved += 1,
            DeathCause::Overfed => self.overfed += 1,
            DeathCause::OldAge => self.old_age += 1,
            DeathCause::Hazard => self.hazard += 1,
        }
    }

    pub fn deaths(&self) -> u32 {
        self.starved + self.overfed + self.old_age + self.hazard
    }

    pub fn add(&mut self, other: &Self) {
        self.births += other.births;
        self.meals += other.meals;
        self.starved += other.starved;
        self.overfed += other.overfed;
        self.old_age += other.old_age;
        self.hazard += other.hazard;
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
    pub max_generation: u32,
    pub events: TickEvents,
}

impl TickStats {
    pub fn measure(tick: u32, world: &World, events: TickEvents) -> Self {
        let (mut males, mut females, mut max_generation, mut total_speed) = (0, 0, 0, 0);
        let (mut sum, mut min, mut max) = (0.0_f64, f32::INFINITY, f32::NEG_INFINITY);
        for (_, individual) in world.individuals() {
            match individual.gender {
                Gender::Male => males += 1,
                Gender::Female => females += 1,
            }
            total_speed += individual.speed;
            sum += f64::from(individual.smartness);
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
            }),
            mean_speed: (population > 0).then(|| total_speed as f32 / population as f32),
            max_generation,
            events,
        }
    }
}
