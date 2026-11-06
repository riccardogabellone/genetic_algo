use std::{
    error::Error,
    io::{self, IsTerminal, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use clap::Parser;
use genetic_algo::{
    config::{Config, DEFAULT_HEIGHT, DEFAULT_WIDTH},
    render::{
        ascii::{CLEAR_SCREEN, legend, map_frame, stats_line},
        chart::write_chart,
        gif::GifRecorder,
    },
    simulation::{Outcome, Simulation},
    stats::TickEvents,
};

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Watch a free-roaming population evolve smartness on a grid."
)]
struct Args {
    #[arg(long, default_value_t = DEFAULT_WIDTH)]
    width: usize,
    #[arg(long, default_value_t = DEFAULT_HEIGHT)]
    height: usize,
    /// Maximum ticks before the run stops [default: from the rules].
    #[arg(long)]
    ticks: Option<u32>,
    /// Extra health per turn at maximum smartness, proportional below it.
    #[arg(long, default_value_t = 0.0)]
    smartness_cost: f32,
    /// How far maximum smartness can see food, proportional below it. 0 disables it.
    #[arg(long, default_value_t = 0)]
    vision: usize,
    /// Lets speed evolve up to this value. 1 keeps it fixed.
    #[arg(long, default_value_t = 1)]
    max_speed: usize,
    /// Extra health per point of speed above 1, on turns that change cell.
    #[arg(long, default_value_t = 0.0)]
    speed_cost: f32,
    /// Replays an earlier run. A random seed is chosen and printed when omitted.
    #[arg(long)]
    seed: Option<u64>,
    /// Pause between terminal frames, in milliseconds.
    #[arg(long, default_value_t = 60)]
    delay_ms: u64,
    /// Skip the live map. Also implied when stdout is not a terminal.
    #[arg(long)]
    headless: bool,
    /// Where to write the end-of-run chart (PNG).
    #[arg(long, default_value = "results.png")]
    chart: PathBuf,
    /// Also record the map as an animated GIF at this path.
    #[arg(long)]
    gif: Option<PathBuf>,
    /// Record one GIF frame every N ticks.
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..))]
    gif_every: u32,
}

fn main() {
    if let Err(error) = run(&Args::parse()) {
        let closed_pipe = error
            .downcast_ref::<io::Error>()
            .is_some_and(|io_error| io_error.kind() == io::ErrorKind::BrokenPipe);
        if !closed_pipe {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

fn run(args: &Args) -> Result<(), Box<dyn Error>> {
    // Fail before a long run, not after it.
    check_chart_path(&args.chart)?;

    let mut config = Config {
        smartness_upkeep: args.smartness_cost,
        max_vision: args.vision,
        max_speed: args.max_speed,
        speed_upkeep: args.speed_cost,
        ..Config::for_size(args.width, args.height)
    };
    if let Some(ticks) = args.ticks {
        config.max_ticks = ticks;
    }
    let seed = args.seed.unwrap_or_else(rand::random);
    let mut simulation = Simulation::new(config, seed)?;
    let mut recorder = match &args.gif {
        Some(path) => Some(GifRecorder::create(
            path,
            simulation.config().width,
            simulation.config().height,
        )?),
        None => None,
    };

    let mut stdout = io::stdout().lock();
    let live_view = !args.headless && stdout.is_terminal();
    writeln!(
        stdout,
        "seed {seed} | {}x{} | target smartness {} | at most {} ticks",
        simulation.config().width,
        simulation.config().height,
        simulation.config().target_smartness,
        simulation.config().max_ticks
    )?;

    let outcome = loop {
        if live_view {
            draw_live_frame(&mut stdout, &simulation, seed)?;
            thread::sleep(Duration::from_millis(args.delay_ms));
        }
        if let Some(recorder) = recorder.as_mut()
            && simulation.latest().tick % args.gif_every == 0
        {
            recorder.record(simulation.world())?;
        }
        if let Some(outcome) = simulation.outcome() {
            break outcome;
        }
        simulation.step();
    };

    write_chart(
        &args.chart,
        simulation.history(),
        simulation.config().target_smartness,
    )
    .map_err(|error| format!("could not write {}: {error}", args.chart.display()))?;

    print_summary(&mut stdout, &simulation, outcome)?;
    writeln!(stdout, "chart: {}", args.chart.display())?;
    if let (Some(path), Some(recorder)) = (&args.gif, &recorder) {
        writeln!(
            stdout,
            "gif: {} ({} frames)",
            path.display(),
            recorder.frames()
        )?;
    }
    Ok(())
}

fn check_chart_path(path: &Path) -> Result<(), String> {
    let is_png = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"));
    if !is_png || path.is_dir() {
        return Err(format!(
            "--chart must be a .png file, got {}",
            path.display()
        ));
    }
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() && !parent.is_dir() => {
            Err(format!("directory {} does not exist", parent.display()))
        }
        _ => Ok(()),
    }
}

fn draw_live_frame(out: &mut impl Write, simulation: &Simulation, seed: u64) -> io::Result<()> {
    let frame = map_frame(simulation.world(), true);
    let stats = stats_line(simulation.latest());
    write!(
        out,
        "{CLEAR_SCREEN}seed {seed}\n{stats}\n{frame}{}\n",
        legend()
    )?;
    out.flush()
}

fn print_summary(
    out: &mut impl Write,
    simulation: &Simulation,
    outcome: Outcome,
) -> io::Result<()> {
    let verdict = match outcome {
        Outcome::Extinct => "the population went extinct",
        Outcome::TargetReached => "the population reached the target smartness",
        Outcome::TicksExhausted => "the tick limit was reached",
    };
    let mut totals = TickEvents::default();
    for stats in simulation.history() {
        totals.add(&stats.events);
    }
    writeln!(out, "\nEnded because {verdict}.")?;
    writeln!(out, "{}", stats_line(simulation.latest()))?;
    writeln!(
        out,
        "births {} | meals {} | deaths {}: starved {}, overfed {}, old age {}, hazard {}",
        totals.births,
        totals.meals,
        totals.deaths(),
        totals.starved,
        totals.overfed,
        totals.old_age,
        totals.hazard
    )
}
