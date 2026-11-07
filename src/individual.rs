//! A single agent, its life-cycle rules and its genetics.

use rand::RngExt;

use crate::{
    config::{Config, MAX_SMARTNESS},
    leadership::Civics,
    sampling::triangular,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
}

impl Gender {
    fn random(rng: &mut impl RngExt) -> Self {
        if rng.random() {
            Self::Male
        } else {
            Self::Female
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeathCause {
    Starved,
    Overfed,
    OldAge,
    Hazard,
    Combat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Individual {
    pub id: u64,
    pub generation: u32,
    pub gender: Gender,
    /// Chance per turn of deciding instead of acting at random. Starts at
    /// `innate_smartness`; fights lower it for the rest of this individual's life.
    pub smartness: f32,
    /// The heritable part of smartness, which fights do not touch.
    pub innate_smartness: f32,
    /// Heritable: chance in `0..=1` that a mating attempt succeeds.
    pub reproduction: f32,
    pub health: u32,
    pub age: u32,
    pub lifespan: u32,
    pub speed: usize,
    pub mating_cooldown: u32,
    pub civics: Civics,
}

impl Individual {
    pub fn founder(id: u64, gender: Gender, config: &Config, rng: &mut impl RngExt) -> Self {
        let smartness = config.founder_smartness.sample(rng);
        Self {
            id,
            generation: 0,
            gender,
            smartness,
            innate_smartness: smartness,
            reproduction: config.founder_reproduction.sample(rng),
            health: config.founder_health.sample(rng) as u32,
            age: 0,
            lifespan: rng.random_range(config.min_lifespan..=config.max_lifespan),
            speed: config.initial_speed,
            mating_cooldown: 0,
            civics: Civics::founder(config, rng),
        }
    }

    /// Traits are drawn between the parents' values, then may mutate.
    ///
    /// Drawing between parents, rather than averaging them, keeps variation from
    /// collapsing: plain blending halves the variance every generation.
    pub fn offspring(
        id: u64,
        parent_a: &Self,
        parent_b: &Self,
        config: &Config,
        rng: &mut impl RngExt,
    ) -> Self {
        let smartness = mutate(
            inherit(parent_a.innate_smartness, parent_b.innate_smartness, rng),
            config.smartness_mutation_step,
            MAX_SMARTNESS,
            config.mutation_rate,
            rng,
        );
        let reproduction = mutate(
            inherit(parent_a.reproduction, parent_b.reproduction, rng),
            config.reproduction_mutation_step,
            1.0,
            config.mutation_rate,
            rng,
        );
        Self {
            id,
            generation: parent_a.generation.max(parent_b.generation) + 1,
            gender: Gender::random(rng),
            smartness,
            innate_smartness: smartness,
            reproduction,
            health: config.mating_cost * 2,
            age: 0,
            lifespan: rng.random_range(config.min_lifespan..=config.max_lifespan),
            speed: inherit_speed(parent_a.speed, parent_b.speed, config, rng),
            mating_cooldown: 0,
            civics: Civics::inherited(parent_a, parent_b, config, rng),
        }
    }

    pub fn can_mate(&self, config: &Config) -> bool {
        // Paying for the birth must leave health above zero: nobody may die outside
        // their own turn. Covering the basic turn cost too avoids most starvation at birth.
        let reserve = config.mating_cost.saturating_add(config.turn_cost);
        self.age >= config.maturity_age && self.mating_cooldown == 0 && self.health > reserve
    }

    /// Health and old-age rules, evaluated at the end of the individual's turn.
    pub fn death_cause(&self, config: &Config) -> Option<DeathCause> {
        if self.health == 0 {
            Some(DeathCause::Starved)
        } else if self.health >= config.max_health {
            Some(DeathCause::Overfed)
        } else if self.age >= self.lifespan {
            Some(DeathCause::OldAge)
        } else {
            None
        }
    }
}

/// Speed is a whole number, so it comes from one parent and mutates by a single step.
fn inherit_speed(speed_a: usize, speed_b: usize, config: &Config, rng: &mut impl RngExt) -> usize {
    let inherited = if rng.random() { speed_a } else { speed_b };
    let mutated = if rng.random::<f32>() >= config.mutation_rate {
        inherited
    } else if rng.random() {
        inherited + 1
    } else {
        inherited.saturating_sub(1)
    };
    mutated.clamp(1, config.max_speed)
}

pub(crate) fn inherit(value_a: f32, value_b: f32, rng: &mut impl RngExt) -> f32 {
    triangular(rng, value_a, f32::midpoint(value_a, value_b), value_b)
}

/// Unbiased: the step is symmetric around zero, so mutation alone does not push the
/// trait up or down, except for the reflection caused by clamping at the scale ends.
pub(crate) fn mutate(value: f32, step: f32, upper: f32, rate: f32, rng: &mut impl RngExt) -> f32 {
    if rng.random::<f32>() >= rate {
        return value;
    }
    (value + triangular(rng, -step, 0.0, step)).clamp(0.0, upper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn parent(config: &Config, smartness: f32) -> Individual {
        let mut rng = StdRng::seed_from_u64(1);
        Individual {
            smartness,
            innate_smartness: smartness,
            ..Individual::founder(0, Gender::Male, config, &mut rng)
        }
    }

    /// Every birth and death allocates one of these. Measured on a 28-core machine with
    /// the 8 evolution tests in parallel: 72 bytes takes 6.7 seconds, 88 bytes takes 46,
    /// nearly all of it waiting on glibc's allocator lock. Keep rare data behind a `Box`.
    #[test]
    fn individuals_stay_small_enough_to_avoid_allocator_contention() {
        assert!(std::mem::size_of::<Individual>() <= 72);
    }

    #[test]
    fn founders_respect_their_configured_ranges() {
        let config = Config::default();
        let mut rng = StdRng::seed_from_u64(1);
        for id in 0..1_000 {
            let founder = Individual::founder(id, Gender::Female, &config, &mut rng);
            assert!((1.0..=5.0).contains(&founder.smartness));
            assert!((0.4..=1.0).contains(&founder.reproduction));
            assert!(founder.health > 0 && founder.health < config.max_health);
            assert!((config.min_lifespan..=config.max_lifespan).contains(&founder.lifespan));
            assert!(founder.death_cause(&config).is_none());
        }
    }

    #[test]
    fn without_mutation_children_stay_between_their_parents() {
        let config = Config {
            mutation_rate: 0.0,
            ..Config::default()
        };
        let (a, b) = (parent(&config, 20.0), parent(&config, 40.0));
        let mut rng = StdRng::seed_from_u64(2);
        for id in 0..1_000 {
            let child = Individual::offspring(id, &a, &b, &config, &mut rng);
            assert!((20.0..=40.0).contains(&child.smartness));
        }
    }

    #[test]
    fn mutation_is_clamped_to_the_scale() {
        let config = Config {
            mutation_rate: 1.0,
            smartness_mutation_step: MAX_SMARTNESS,
            reproduction_mutation_step: 1.0,
            ..Config::default()
        };
        let mut rng = StdRng::seed_from_u64(3);
        let a = parent(&config, 50.0);
        for id in 0..1_000 {
            let child = Individual::offspring(id, &a, &a, &config, &mut rng);
            assert!((0.0..=MAX_SMARTNESS).contains(&child.smartness));
            assert!((0.0..=1.0).contains(&child.reproduction));
        }
    }

    /// Regression: the first mutation had a mean factor of 0.85, dragging smartness down.
    #[test]
    fn mutation_is_unbiased_away_from_the_scale_ends() {
        let config = Config {
            mutation_rate: 1.0,
            ..Config::default()
        };
        let mut rng = StdRng::seed_from_u64(4);
        let a = parent(&config, 50.0);
        let samples = 100_000;
        let total: f32 = (0..samples)
            .map(|id| Individual::offspring(id, &a, &a, &config, &mut rng).smartness)
            .sum();
        assert!((total / samples as f32 - 50.0).abs() < 0.1);
    }

    #[test]
    fn identical_parents_do_not_produce_nan() {
        let config = Config {
            mutation_rate: 0.0,
            ..Config::default()
        };
        let a = parent(&config, 33.0);
        let mut rng = StdRng::seed_from_u64(5);
        assert_eq!(
            Individual::offspring(1, &a, &a, &config, &mut rng).smartness,
            33.0
        );
    }

    /// Damage from fights is a scar, not a gene.
    #[test]
    fn children_inherit_innate_smartness_not_the_current_value() {
        let config = Config {
            mutation_rate: 0.0,
            ..Config::default()
        };
        let scarred = Individual {
            smartness: 0.0,
            ..parent(&config, 60.0)
        };
        let mut rng = StdRng::seed_from_u64(9);
        let child = Individual::offspring(1, &scarred, &scarred, &config, &mut rng);
        assert_eq!(child.smartness, 60.0);
        assert_eq!(child.innate_smartness, 60.0);
    }

    #[test]
    fn child_generation_follows_the_older_lineage() {
        let config = Config::default();
        let a = Individual {
            generation: 2,
            ..parent(&config, 1.0)
        };
        let b = Individual {
            generation: 7,
            ..parent(&config, 1.0)
        };
        let mut rng = StdRng::seed_from_u64(6);
        assert_eq!(
            Individual::offspring(1, &a, &b, &config, &mut rng).generation,
            8
        );
    }

    #[test]
    fn speed_comes_from_a_parent_and_mutates_one_step_within_bounds() {
        let config = Config {
            mutation_rate: 1.0,
            max_speed: 4,
            ..Config::default()
        };
        let mut rng = StdRng::seed_from_u64(7);
        let slow = Individual {
            speed: 1,
            ..parent(&config, 1.0)
        };
        let fast = Individual {
            speed: 4,
            ..parent(&config, 1.0)
        };
        let mut seen = [false; 5];
        for id in 0..1_000 {
            let child = Individual::offspring(id, &slow, &fast, &config, &mut rng);
            assert!((1..=4).contains(&child.speed));
            seen[child.speed] = true;
        }
        assert_eq!(seen, [false, true, true, true, true]);
    }

    #[test]
    fn speed_is_fixed_when_max_speed_equals_it() {
        let config = Config {
            mutation_rate: 1.0,
            ..Config::default()
        };
        let mut rng = StdRng::seed_from_u64(8);
        let a = parent(&config, 1.0);
        for id in 0..200 {
            assert_eq!(
                Individual::offspring(id, &a, &a, &config, &mut rng).speed,
                1
            );
        }
    }

    #[test]
    fn death_rules_match_the_readme() {
        let config = Config::default();
        let alive = Individual {
            health: 50,
            age: 1,
            lifespan: 10,
            ..parent(&config, 1.0)
        };
        assert_eq!(alive.death_cause(&config), None);
        assert_eq!(
            Individual {
                health: 0,
                ..alive.clone()
            }
            .death_cause(&config),
            Some(DeathCause::Starved)
        );
        assert_eq!(
            Individual {
                health: config.max_health,
                ..alive.clone()
            }
            .death_cause(&config),
            Some(DeathCause::Overfed)
        );
        assert_eq!(
            Individual { age: 10, ..alive }.death_cause(&config),
            Some(DeathCause::OldAge)
        );
    }

    #[test]
    fn mating_eligibility_boundaries() {
        let config = Config::default();
        let ready = Individual {
            age: config.maturity_age,
            health: config.mating_cost + config.turn_cost + 1,
            mating_cooldown: 0,
            ..parent(&config, 1.0)
        };
        assert!(ready.can_mate(&config));
        assert!(
            !Individual {
                age: config.maturity_age - 1,
                ..ready.clone()
            }
            .can_mate(&config)
        );
        assert!(
            !Individual {
                health: ready.health - 1,
                ..ready.clone()
            }
            .can_mate(&config)
        );
        assert!(
            !Individual {
                mating_cooldown: 1,
                ..ready
            }
            .can_mate(&config)
        );
    }
}
