//! Compares rule sets, each against a control where smartness has no behavioural effect.
//!
//! Usage: `cargo run --release --example rule_experiments [runs] [ticks] [size]`

use std::thread;

use genetic_algo::{config::Config, simulation::Simulation};

struct RunResult {
    midpoint_smartness: Option<f32>,
    smartness: Option<f32>,
    speed: Option<f32>,
    population: usize,
}

fn run_one(config: Config, seed: u64) -> RunResult {
    let midpoint = (config.max_ticks / 2) as usize;
    let mut simulation = Simulation::new(config, seed).expect("valid rule set");
    simulation.run_to_end();
    let history = simulation.history();
    let latest = simulation.latest();
    RunResult {
        midpoint_smartness: history
            .get(midpoint)
            .and_then(|s| s.smartness)
            .map(|s| s.mean),
        smartness: latest.smartness.map(|summary| summary.mean),
        speed: latest.mean_speed,
        population: latest.population,
    }
}

fn run_all(config: &Config, runs: u64) -> Vec<RunResult> {
    thread::scope(|scope| {
        let handles: Vec<_> = (0..runs)
            .map(|seed| {
                let config = config.clone();
                scope.spawn(move || run_one(config, seed))
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("a run panicked"))
            .collect()
    })
}

fn mean(values: impl Iterator<Item = f32>) -> Option<f32> {
    let values: Vec<f32> = values.collect();
    (!values.is_empty()).then(|| values.iter().sum::<f32>() / values.len() as f32)
}

fn cell(value: Option<f32>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.1}"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1).map(|arg| arg.parse::<u32>());
    let runs = u64::from(args.next().transpose()?.unwrap_or(8));
    let ticks = args.next().transpose()?.unwrap_or(15_000);
    let size = args.next().transpose()?.unwrap_or(32) as usize;
    if runs == 0 || size == 0 {
        return Err("runs and size must be positive".into());
    }

    let base = Config {
        max_ticks: ticks,
        target_smartness: 100.0, // Never stop early: arms must run equally long.
        ..Config::for_size(size, size)
    };
    let cost = |upkeep| Config {
        smartness_upkeep: upkeep,
        ..base.clone()
    };
    let speed = |upkeep| Config {
        max_speed: 4,
        speed_upkeep: upkeep,
        ..base.clone()
    };
    let arms = [
        ("baseline", base.clone()),
        ("cost 0.5", cost(0.5)),
        ("cost 1", cost(1.0)),
        ("cost 2", cost(2.0)),
        ("cost 4", cost(4.0)),
        (
            "vision 3",
            Config {
                max_vision: 3,
                ..base.clone()
            },
        ),
        (
            "vision 5",
            Config {
                max_vision: 5,
                ..base.clone()
            },
        ),
        (
            "vision 8",
            Config {
                max_vision: 8,
                ..base.clone()
            },
        ),
        (
            "vision 5 + cost 1",
            Config {
                max_vision: 5,
                ..cost(1.0)
            },
        ),
        (
            "vision 5 + cost 2",
            Config {
                max_vision: 5,
                ..cost(2.0)
            },
        ),
        (
            "vision 5 + cost 4",
            Config {
                max_vision: 5,
                ..cost(4.0)
            },
        ),
        ("speed free", speed(0.0)),
        ("speed cost 0.5", speed(0.5)),
        ("speed cost 1", speed(1.0)),
        ("speed cost 2", speed(2.0)),
        (
            "all: 2 / 5 / speed 1",
            Config {
                smartness_upkeep: 2.0,
                max_vision: 5,
                ..speed(1.0)
            },
        ),
    ];

    println!("{runs} runs, {ticks} ticks, {size}x{size}; means over surviving runs");
    println!(
        "{:<22}{:>9}{:>9}{:>7}{:>7}{:>6} |{:>9}{:>7}{:>6}",
        "rules", "mid", "end", "speed", "pop", "ext", "ctrl end", "pop", "ext"
    );
    for (name, config) in arms {
        let real = run_all(&config, runs);
        let control_config = Config {
            smartness_affects_behavior: false,
            ..config
        };
        let control = run_all(&control_config, runs);
        let alive = |results: &[RunResult]| results.iter().filter(|r| r.population > 0).count();
        let population = |results: &[RunResult]| {
            mean(
                results
                    .iter()
                    .filter(|r| r.population > 0)
                    .map(|r| r.population as f32),
            )
        };
        println!(
            "{name:<22}{:>9}{:>9}{:>7}{:>7}{:>6} |{:>9}{:>7}{:>6}",
            cell(mean(real.iter().filter_map(|r| r.midpoint_smartness))),
            cell(mean(real.iter().filter_map(|r| r.smartness))),
            mean(real.iter().filter_map(|r| r.speed))
                .map_or_else(|| "-".to_owned(), |speed| format!("{speed:.2}")),
            cell(population(&real)),
            real.len() - alive(&real),
            cell(mean(control.iter().filter_map(|r| r.smartness))),
            cell(population(&control)),
            control.len() - alive(&control),
        );
    }
    Ok(())
}
