//! Evolved brains: a tiny neural network whose weights are a genome.
//!
//! The same network scores every option an individual has: each cell it can reach, and
//! staying put. It sees only a few features of that option, so it works for any speed.
//! The highest score wins. Weights are inherited by crossover and mutation.
//!
//! With lifetime learning on, a brain also learns from experience, by value learning: the
//! score of an option learns to predict what doing it causes, such as a meal or a hazard's
//! damage. Every turn the individual acts on a cell teaches it, whether its brain chose
//! the cell or it acted at random, so random turns serve as exploration. Learned weights
//! are fine-grained `f32` values that start from the inherited genome, and they die with
//! the individual: children inherit only the genome.

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

/// Learned weights are kept within the range the genome can express.
const MAX_WEIGHT: f32 = 127.0 * WEIGHT_SCALE;

#[derive(Debug, Clone, PartialEq)]
pub struct Brain {
    weights: [i8; WEIGHTS],
    /// Allocated on the first lesson, so brains that never learn stay small.
    learner: Option<Box<Learner>>,
}

/// What a brain has learned in its own life.
#[derive(Debug, Clone, PartialEq)]
pub struct Learner {
    pub weights: [f32; WEIGHTS],
}

impl Brain {
    pub fn from_weights(weights: [i8; WEIGHTS]) -> Self {
        Self {
            weights,
            learner: None,
        }
    }

    pub fn learner(&self) -> Option<&Learner> {
        self.learner.as_deref()
    }

    /// The weights in use: learned ones once learning has begun, the genome otherwise.
    fn current_weights(&self) -> [f32; WEIGHTS] {
        match &self.learner {
            Some(learner) => learner.weights,
            None => self.genome_weights(),
        }
    }

    fn genome_weights(&self) -> [f32; WEIGHTS] {
        std::array::from_fn(|index| f32::from(self.weights[index]) * WEIGHT_SCALE)
    }

    pub fn weights(&self) -> &[i8; WEIGHTS] {
        &self.weights
    }

    pub fn random(rng: &mut impl RngExt) -> Self {
        Self::from_weights(std::array::from_fn(|_| {
            rng.random_range(-FOUNDER_RANGE..=FOUNDER_RANGE)
        }))
    }

    /// Each weight comes from one parent at random, then may mutate by a small step.
    pub fn offspring(
        parent_a: &Self,
        parent_b: &Self,
        config: &Config,
        rng: &mut impl RngExt,
    ) -> Self {
        let step = i16::from(config.brain_mutation_step);
        Self::from_weights(std::array::from_fn(|index| {
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
        }))
    }

    /// How much this brain likes an option. Only the comparison between options matters.
    pub fn score(&self, features: &Features) -> f32 {
        forward(&self.current_weights(), &features.inputs()).0
    }

    /// Learns from one turn: `option` is what the individual acted on, and `reward` what
    /// that caused. The option's score moves toward the reward, so scores come to predict
    /// consequences: a meal scores high, a hazard low, an ordinary move near zero.
    pub fn learn(&mut self, option: &Features, reward: f32, config: &Config) {
        let genome = self.genome_weights();
        let learner = self
            .learner
            .get_or_insert_with(|| Box::new(Learner { weights: genome }));
        let inputs = option.inputs();
        let error = reward - forward(&learner.weights, &inputs).0;
        let gradient = score_gradient(&learner.weights, &inputs);
        let step = config.learning_rate * error;
        for (weight, slope) in learner.weights.iter_mut().zip(gradient) {
            *weight = (*weight + step * slope).clamp(-MAX_WEIGHT, MAX_WEIGHT);
        }
    }
}

/// The score, and each hidden unit's output, for one option.
fn forward(weights: &[f32; WEIGHTS], inputs: &[f32; INPUTS]) -> (f32, [f32; HIDDEN]) {
    let mut hidden = [0.0; HIDDEN];
    let mut score = 0.0;
    for (unit, output) in hidden.iter_mut().enumerate() {
        let mut activation = weights[INPUTS * HIDDEN + unit];
        for (input, value) in inputs.iter().enumerate() {
            activation += weights[unit * INPUTS + input] * value;
        }
        *output = activation.tanh();
        score += weights[INPUTS * HIDDEN + HIDDEN + unit] * *output;
    }
    (score, hidden)
}

/// How the score of one option changes with each weight.
fn score_gradient(weights: &[f32; WEIGHTS], inputs: &[f32; INPUTS]) -> [f32; WEIGHTS] {
    let (_, hidden) = forward(weights, inputs);
    let mut gradient = [0.0; WEIGHTS];
    for unit in 0..HIDDEN {
        let output_weight = weights[INPUTS * HIDDEN + HIDDEN + unit];
        // tanh'(x) = 1 - tanh(x)^2
        let through = output_weight * (1.0 - hidden[unit] * hidden[unit]);
        for (input, value) in inputs.iter().enumerate() {
            gradient[unit * INPUTS + input] = through * value;
        }
        gradient[INPUTS * HIDDEN + unit] = through;
        gradient[INPUTS * HIDDEN + HIDDEN + unit] = hidden[unit];
    }
    gradient
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
    use crate::config::MAX_LEARNING_RATE;
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

    fn learning_config() -> Config {
        Config {
            brains_enabled: true,
            learning_enabled: true,
            ..Config::default()
        }
    }

    fn random_inputs(rng: &mut StdRng) -> Features {
        Features {
            empty: rng.random(),
            food: rng.random(),
            wall: false,
            hazard: rng.random(),
            individual: false,
            stay: false,
            hunger: rng.random(),
            food_nearby: rng.random(),
            hazards_nearby: rng.random(),
        }
    }

    /// Checks the hand-written backpropagation against finite differences.
    #[test]
    fn score_gradient_matches_finite_differences() {
        let mut rng = StdRng::seed_from_u64(5);
        for _ in 0..20 {
            let weights: [f32; WEIGHTS] = std::array::from_fn(|_| rng.random_range(-2.0..2.0));
            let inputs = random_inputs(&mut rng).inputs();
            let analytic = score_gradient(&weights, &inputs);
            for index in 0..WEIGHTS {
                let nudge = 1e-3;
                let mut up = weights;
                let mut down = weights;
                up[index] += nudge;
                down[index] -= nudge;
                let numeric = (forward(&up, &inputs).0 - forward(&down, &inputs).0) / (2.0 * nudge);
                assert!((numeric - analytic[index]).abs() < 1e-2, "weight {index}");
            }
        }
    }

    #[test]
    fn scores_learn_to_predict_each_options_consequence() {
        let config = learning_config();
        let mut rng = StdRng::seed_from_u64(8);
        let mut brain = Brain::random(&mut rng);
        for _ in 0..2_000 {
            brain.learn(&option("food"), 0.2, &config);
            brain.learn(&option("hazard"), -0.15, &config);
            brain.learn(&option("empty"), 0.0, &config);
        }
        for (kind, consequence) in [("food", 0.2), ("hazard", -0.15), ("empty", 0.0)] {
            let score = brain.score(&option(kind));
            assert!((score - consequence).abs() < 0.03, "{kind}: {score}");
        }
    }

    #[test]
    fn a_learned_preference_overrides_an_inherited_one() {
        let config = learning_config();
        let mut brain = food_lover();
        assert!(brain.score(&option("food")) > brain.score(&option("empty")));
        for _ in 0..500 {
            brain.learn(&option("food"), -0.2, &config);
            brain.learn(&option("empty"), 0.0, &config);
        }
        assert!(brain.score(&option("food")) < brain.score(&option("empty")));
    }

    #[test]
    fn learning_starts_from_the_genome_and_stays_in_range() {
        let mut brain = food_lover();
        assert!(brain.learner().is_none());
        let exact = brain.score(&option("food"));
        brain.learn(&option("food"), exact, &learning_config());
        let learned = brain
            .learner()
            .expect("allocated on the first lesson")
            .weights;
        assert_eq!(
            learned,
            food_lover().genome_weights(),
            "no surprise, no change"
        );

        let reckless = Config {
            learning_rate: MAX_LEARNING_RATE,
            ..learning_config()
        };
        for _ in 0..50 {
            brain.learn(&option("food"), 1.0, &reckless);
            brain.learn(&option("hazard"), -1.0, &reckless);
        }
        let learned = brain.learner().unwrap().weights;
        assert!(
            learned
                .iter()
                .all(|w| w.is_finite() && w.abs() <= MAX_WEIGHT)
        );
    }

    #[test]
    fn children_inherit_the_genome_not_what_a_parent_learned() {
        let config = Config {
            brain_mutation_rate: 0.0,
            ..learning_config()
        };
        let mut parent = food_lover();
        for _ in 0..50 {
            parent.learn(&option("food"), -1.0, &config);
        }
        assert_ne!(parent, food_lover());
        let mut rng = StdRng::seed_from_u64(9);
        let child = Brain::offspring(&parent, &parent, &config, &mut rng);
        assert_eq!(child, food_lover(), "the genome, with nothing learned");
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
