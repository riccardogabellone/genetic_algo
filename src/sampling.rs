//! Random sampling helpers.

use rand::RngExt;

/// A triangular distribution, used for every "min / most likely / max" parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spread {
    pub min: f32,
    pub mode: f32,
    pub max: f32,
}

impl Spread {
    pub fn sample(&self, rng: &mut impl RngExt) -> f32 {
        triangular(rng, self.min, self.mode, self.max)
    }

    /// True when all values are finite and ordered `min <= mode <= max`.
    pub fn is_valid(&self) -> bool {
        self.min.is_finite()
            && self.mode.is_finite()
            && self.max.is_finite()
            && self.min <= self.mode
            && self.mode <= self.max
    }
}

/// Samples a triangular distribution by inverse transform.
///
/// Swapped bounds are reordered, the mode is clamped into range, and a zero-width
/// range returns its only possible value. All inputs must be finite.
pub fn triangular(rng: &mut impl RngExt, bound_a: f32, mode: f32, bound_b: f32) -> f32 {
    let (low, high) = if bound_a <= bound_b {
        (bound_a, bound_b)
    } else {
        (bound_b, bound_a)
    };
    let span = high - low;
    if span <= 0.0 {
        return low;
    }
    let mode_fraction = (mode.clamp(low, high) - low) / span;
    let u: f32 = rng.random();
    let value = if u < mode_fraction {
        low + span * (u * mode_fraction).sqrt()
    } else {
        high - span * ((1.0 - u) * (1.0 - mode_fraction)).sqrt()
    };
    value.clamp(low, high) // Guards against rounding just past a bound.
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    const SAMPLES: usize = 200_000;

    fn samples(low: f32, mode: f32, high: f32) -> Vec<f32> {
        let mut rng = StdRng::seed_from_u64(7);
        (0..SAMPLES)
            .map(|_| triangular(&mut rng, low, mode, high))
            .collect()
    }

    fn mean(values: &[f32]) -> f32 {
        values.iter().sum::<f32>() / values.len() as f32
    }

    #[test]
    fn mean_matches_the_analytic_value() {
        for (low, mode, high) in [(1.0, 2.0, 5.0), (25.0, 50.0, 75.0), (-4.0, 0.0, 4.0)] {
            let expected = (low + mode + high) / 3.0;
            let actual = mean(&samples(low, mode, high));
            assert!((actual - expected).abs() < 0.05, "{actual} vs {expected}");
        }
    }

    /// Regression: the first implementation could never output values in (1.5, 3.5).
    #[test]
    fn support_has_no_holes() {
        let values = samples(1.0, 2.0, 5.0);
        let bins = 16;
        let mut counts = vec![0_usize; bins];
        for value in values {
            let bin = (((value - 1.0) / 4.0) * bins as f32) as usize;
            counts[bin.min(bins - 1)] += 1;
        }
        assert!(counts.iter().all(|count| *count > 0), "{counts:?}");
    }

    #[test]
    fn most_samples_fall_near_the_mode() {
        let values = samples(0.0, 2.0, 10.0);
        let near_mode = values.iter().filter(|v| (1.0..3.0).contains(*v)).count();
        let far_tail = values.iter().filter(|v| (8.0..10.0).contains(*v)).count();
        assert!(near_mode > far_tail * 5);
    }

    /// Regression: `min == max` returned NaN.
    #[test]
    fn zero_width_range_returns_the_value() {
        assert_eq!(samples(3.0, 3.0, 3.0)[0], 3.0);
    }

    /// Regression: swapped bounds collapsed the range to the smaller one.
    #[test]
    fn swapped_bounds_are_reordered() {
        let values = samples(5.0, 2.0, 1.0);
        assert!(values.iter().all(|v| (1.0..=5.0).contains(v)));
        assert!(values.iter().any(|v| *v > 3.0));
    }

    #[test]
    fn mode_at_either_bound_stays_in_range() {
        for (low, mode, high) in [(0.0, 0.0, 1.0), (0.0, 1.0, 1.0), (0.0, 9.0, 1.0)] {
            assert!(
                samples(low, mode, high)
                    .iter()
                    .all(|v| (0.0..=1.0).contains(v))
            );
        }
    }

    #[test]
    fn spread_validation() {
        let valid = Spread {
            min: 1.0,
            mode: 2.0,
            max: 3.0,
        };
        assert!(valid.is_valid());
        assert!(!Spread { mode: 9.0, ..valid }.is_valid());
        assert!(
            !Spread {
                min: f32::NAN,
                ..valid
            }
            .is_valid()
        );
    }
}
