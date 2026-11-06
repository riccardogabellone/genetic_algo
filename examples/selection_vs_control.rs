//! Does smartness evolve because it helps, or only because mutation spreads it?
//!
//! Runs the real rules against a control where smartness has no effect on behaviour.
//! Mutation alone raises the mean (the trait starts near 0 and is clamped there), so
//! only the gap between the two columns is evidence of selection.
//!
//! Usage: `cargo run --release --example selection_vs_control [runs] [ticks] [size]`

use genetic_algo::{
    config::Config,
    simulation::{Outcome, Simulation},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1).map(|arg| arg.parse::<u32>());
    let runs = args.next().transpose()?.unwrap_or(10);
    let ticks = args.next().transpose()?.unwrap_or(10_000);
    let size = args.next().transpose()?.unwrap_or(32) as usize;

    if runs == 0 || size == 0 {
        return Err("runs and size must be positive".into());
    }

    println!("{runs} runs, up to {ticks} ticks, {size}x{size}");
    println!(
        "{:<10}{:>12}{:>10}{:>10}{:>10}{:>12}",
        "rules", "smartness", "pop", "extinct", "target", "gen"
    );
    for (label, smartness_matters) in [("selection", true), ("control", false)] {
        let (mut smartness, mut population, mut extinct, mut on_target, mut generation) =
            (0.0, 0, 0_usize, 0, 0);
        for seed in 0..u64::from(runs) {
            let config = Config {
                max_ticks: ticks,
                smartness_affects_behavior: smartness_matters,
                // Neither arm stops early at the target, so both run the same length.
                target_smartness: 100.0,
                ..Config::for_size(size, size)
            };
            let mut simulation = Simulation::new(config, seed)?;
            match simulation.run_to_end() {
                Outcome::Extinct => extinct += 1,
                Outcome::TargetReached => on_target += 1,
                Outcome::TicksExhausted => {}
            }
            let latest = simulation.latest();
            // Extinct runs are reported in their own column, not averaged in as zero.
            smartness += latest.smartness.map_or(0.0, |summary| summary.mean);
            population += latest.population;
            generation += latest.max_generation;
        }
        println!(
            "{label:<10}{:>12.2}{:>10}{:>10}{:>10}{:>12}",
            smartness / (runs as usize - extinct).max(1) as f32,
            population / runs as usize,
            extinct,
            on_target,
            generation / runs
        );
    }
    Ok(())
}
