//! Evolved brains: a tiny neural network whose weights are a genome.
//!
//! The same network scores every option an individual has: each cell it can reach, and
//! staying put. It sees only a few features of that option, so it works for any speed.
//! The highest score wins. Weights are inherited by crossover and mutation, and never
//! change during a life: what a brain knows, it knows because its ancestors survived.

use rand::{RngExt, seq::IndexedRandom};

use crate::{config::Config, individual::Individual};

pub const INPUTS: usize = 9;
pub const HIDDEN: usize = 4;
/// Input-to-hidden weights, hidden biases, then hidden-to-output weights. An output bias
/// would add the same amount to every option, so it could never change a choice.
pub const WEIGHTS: usize = INPUTS * HIDDEN + HIDDEN + HIDDEN;

/// Weights are stored as bytes to keep each brain one small allocation; each step is this.
const WEIGHT_SCALE: f32 = 1.0 / 16.0;
/// Founders' weights are uniform in this range of steps, so up to about plus or minus 2.
const FOUNDER_RANGE: i8 = 32;

/// What a brain is told about one option.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Features {
    pub empty: bool,
    pub food: bool,
    pub wall: bool,
    pub hazard: bool,
    pub individual: bool,
    /// Staying put rather than moving to a cell.
    pub stay: bool,
    /// From 0 (full) to 1 (empty).
    pub hunger: f32,
    /// Share of the option's eight neighbours that hold food.
    pub food_nearby: f32,
    /// Share of the option's eight neighbours that hold a hazard.
    pub hazards_nearby: f32,
}

impl Features {
    fn inputs(&self) -> [f32; INPUTS] {
        let flag = |on: bool| if on { 1.0 } else { 0.0 };
        [
            flag(self.empty),
            flag(self.food),
            flag(self.wall),
            flag(self.hazard),
            flag(self.individual),
            flag(self.stay),
            self.hunger,
            self.food_nearby,
            self.hazards_nearby,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brain {
    weights: [i8; WEIGHTS],
}

impl Brain {
    pub fn from_weights(weights: [i8; WEIGHTS]) -> Self {
        Self { weights }
    }

    pub fn weights(&self) -> &[i8; WEIGHTS] {
        &self.weights
    }

    pub fn random(rng: &mut impl RngExt) -> Self {
        Self {
            weights: std::array::from_fn(|_| rng.random_range(-FOUNDER_RANGE..=FOUNDER_RANGE)),
        }
    }

    /// Each weight comes from one parent at random, then may mutate by a small step.
    pub fn offspring(
        parent_a: &Self,
        parent_b: &Self,
        config: &Config,
        rng: &mut impl RngExt,
    ) -> Self {
        let step = i16::from(config.brain_mutation_step);
        Self {
            weights: std::array::from_fn(|index| {
                let inherited = if rng.random() {
                    parent_a.weights[index]
                } else {
                    parent_b.weights[index]
                };
                if step == 0 || rng.random::<f32>() >= config.brain_mutation_rate {
                    return inherited;
                }
                let change = rng.random_range(-step..=step);
                (i16::from(inherited) + change).clamp(i16::from(i8::MIN), i16::from(i8::MAX)) as i8
            }),
        }
    }

    /// How much this brain likes an option. Only the comparison between options matters.
    pub fn score(&self, features: &Features) -> f32 {
        let inputs = features.inputs();
        let weight = |index: usize| f32::from(self.weights[index]) * WEIGHT_SCALE;
        let mut score = 0.0;
        for hidden in 0..HIDDEN {
            let mut activation = weight(INPUTS * HIDDEN + hidden);
            for (input, value) in inputs.iter().enumerate() {
                activation += weight(hidden * INPUTS + input) * value;
            }
            score += weight(INPUTS * HIDDEN + HIDDEN + hidden) * activation.tanh();
        }
        score
    }
}

/// A founder's brain, if brains are on. Without them no random numbers are drawn, so runs
/// replay exactly as they did before brains existed.
pub fn founder_brain(config: &Config, rng: &mut impl RngExt) -> Option<Box<Brain>> {
    config.brains_enabled.then(|| Box::new(Brain::random(rng)))
}

/// A child's brain, if brains are on. With inheritance switched off, as a control, every
/// child gets a fresh random brain, so nothing a brain learned can accumulate.
pub fn inherited_brain(
    parent_a: &Individual,
    parent_b: &Individual,
    config: &Config,
    rng: &mut impl RngExt,
) -> Option<Box<Brain>> {
    if !config.brains_enabled {
        return None;
    }
    let parents: Vec<&Brain> = [&parent_a.brain, &parent_b.brain]
        .into_iter()
        .filter_map(|brain| brain.as_deref())
        .collect();
    let brain = match (config.brains_inherited, parents.as_slice()) {
        (true, [a, b]) => Brain::offspring(a, b, config, rng),
        (true, [only]) => Brain::offspring(only, only, config, rng),
        _ => Brain::random(rng),
    };
    Some(Box::new(brain))
}

/// The index of the best-scoring option, choosing at random among equals.
pub fn best_option(scores: &[f32], rng: &mut impl RngExt) -> Option<usize> {
    let best = scores.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let tied: Vec<usize> = (0..scores.len())
        .filter(|index| scores[*index] == best)
        .collect();
    tied.choose(rng).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn option(kind: &str) -> Features {
        Features {
            empty: kind == "empty",
            food: kind == "food",
            wall: kind == "wall",
            hazard: kind == "hazard",
            individual: kind == "individual",
            stay: kind == "stay",
            hunger: 0.5,
            food_nearby: 0.0,
            hazards_nearby: 0.0,
        }
    }

    /// A brain whose only path runs from the `food` input through one hidden unit.
    fn food_lover() -> Brain {
        let mut weights = [0_i8; WEIGHTS];
        weights[1] = 32; // input 1 (food) into hidden unit 0
        weights[INPUTS * HIDDEN + HIDDEN] = 32; // hidden unit 0 into the score
        Brain::from_weights(weights)
    }

    #[test]
    fn the_genome_is_small() {
        assert_eq!(WEIGHTS, 44);
        assert!(std::mem::size_of::<Brain>() <= 64);
    }

    #[test]
    fn a_brain_scores_what_its_weights_favour() {
        let brain = food_lover();
        assert!(brain.score(&option("food")) > brain.score(&option("empty")));
        assert_eq!(brain.score(&option("empty")), 0.0);
    }

    #[test]
    fn a_blank_brain_is_indifferent() {
        let brain = Brain::from_weights([0; WEIGHTS]);
        for kind in ["empty", "food", "wall", "hazard", "individual", "stay"] {
            assert_eq!(brain.score(&option(kind)), 0.0);
        }
    }

    #[test]
    fn scores_are_finite_even_at_extreme_weights() {
        for value in [i8::MIN, i8::MAX] {
            let brain = Brain::from_weights([value; WEIGHTS]);
            let mut all_on = option("food");
            all_on.hunger = 1.0;
            all_on.food_nearby = 1.0;
            all_on.hazards_nearby = 1.0;
            assert!(brain.score(&all_on).is_finite());
        }
    }

    #[test]
    fn founders_use_the_whole_range_and_differ() {
        let mut rng = StdRng::seed_from_u64(1);
        let (a, b) = (Brain::random(&mut rng), Brain::random(&mut rng));
        assert_ne!(a, b);
        let all: Vec<i8> = (0..200)
            .flat_map(|_| *Brain::random(&mut rng).weights())
            .collect();
        assert!(
            all.iter()
                .all(|w| (-FOUNDER_RANGE..=FOUNDER_RANGE).contains(w))
        );
        assert!(all.contains(&-FOUNDER_RANGE) && all.contains(&FOUNDER_RANGE));
    }

    #[test]
    fn without_mutation_every_weight_comes_from_a_parent() {
        let config = Config {
            brain_mutation_rate: 0.0,
            ..Config::default()
        };
        let mut rng = StdRng::seed_from_u64(2);
        let a = Brain::from_weights([10; WEIGHTS]);
        let b = Brain::from_weights([-10; WEIGHTS]);
        let child = Brain::offspring(&a, &b, &config, &mut rng);
        assert!(child.weights().iter().all(|w| *w == 10 || *w == -10));
        assert!(child.weights().contains(&10) && child.weights().contains(&-10));
    }

    #[test]
    fn mutation_is_bounded_and_saturates() {
        let config = Config {
            brain_mutation_rate: 1.0,
            brain_mutation_step: 127,
            ..Config::default()
        };
        let mut rng = StdRng::seed_from_u64(3);
        let edge = Brain::from_weights([i8::MAX; WEIGHTS]);
        for _ in 0..200 {
            let child = Brain::offspring(&edge, &edge, &config, &mut rng);
            assert!(child.weights().iter().all(|w| *w >= 0));
        }
        let small = Config {
            brain_mutation_step: 3,
            ..config
        };
        let zero = Brain::from_weights([0; WEIGHTS]);
        for _ in 0..200 {
            let child = Brain::offspring(&zero, &zero, &small, &mut rng);
            assert!(child.weights().iter().all(|w| (-3..=3).contains(w)));
        }
    }

    #[test]
    fn ties_are_broken_at_random_and_the_best_always_wins() {
        let mut rng = StdRng::seed_from_u64(4);
        assert_eq!(best_option(&[0.1, 0.9, 0.3], &mut rng), Some(1));
        assert_eq!(best_option(&[], &mut rng), None);
        let picks: Vec<usize> = (0..100)
            .filter_map(|_| best_option(&[1.0, 1.0, 0.0], &mut rng))
            .collect();
        assert!(picks.contains(&0) && picks.contains(&1) && !picks.contains(&2));
    }
}
