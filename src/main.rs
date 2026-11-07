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
    leadership::TermJudgement,
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
    /// Chance that a hungry individual strikes a rival racing for the same food.
    #[arg(long, default_value_t = 0.0)]
    contest_fights: f32,
    /// Chance per turn of an unprovoked attack at smartness 0, falling to 0 at maximum.
    #[arg(long, default_value_t = 0.0)]
    random_fights: f32,
    /// Health the victim of a fight loses [default: from the rules].
    #[arg(long)]
    fight_damage: Option<u32>,
    /// Current smartness the attacker loses; not inherited [default: from the rules].
    #[arg(long)]
    fight_smartness_loss: Option<f32>,
    /// Adults elect a leader, whose policy followers sometimes adopt.
    #[arg(long)]
    leaders: bool,
    /// Ticks between elections [default: from the rules].
    #[arg(long)]
    election_interval: Option<u32>,
    /// Chance per turn that a follower acts on the leader's policy [default: from the rules].
    #[arg(long)]
    leader_influence: Option<f32>,
    /// Times one individual can be elected in its life [default: from the rules].
    #[arg(long)]
    max_terms: Option<u32>,
    /// Past elections recalled at maximum smartness [default: from the rules].
    #[arg(long)]
    memory: Option<usize>,
    /// What voters judge a finished term by.
    #[arg(long, value_enum, default_value_t = Judgement::Violence)]
    judge_terms: Judgement,
    /// When an individual thinks, an inherited neural network chooses its action.
    #[arg(long)]
    brains: bool,
    /// Control experiment: every child gets a fresh random brain instead of inheriting one.
    #[arg(long, requires = "brains")]
    random_brains: bool,
    /// Chance that each brain weight mutates in a child [default: from the rules].
    #[arg(long)]
    brain_mutation_rate: Option<f32>,
    /// Largest mutation of one brain weight, in steps of 1/16 [default: from the rules].
    #[arg(long)]
    brain_mutation_step: Option<u8>,
    /// Brains also learn from experience during their life.
    #[arg(long, requires = "brains")]
    learning: bool,
    /// How fast brains learn [default: from the rules].
    #[arg(long)]
    learning_rate: Option<f32>,
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

/// Mirrors `TermJudgement`, so the core does not depend on the command-line parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum Judgement {
    /// The voter's own health over the term.
    Personal,
    /// The population's fight rate over the term, compared with typical terms.
    Violence,
}

impl From<Judgement> for TermJudgement {
    fn from(judgement: Judgement) -> Self {
        match judgement {
            Judgement::Personal => Self::Personal,
            Judgement::Violence => Self::Violence,
        }
    }
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
    config.contest_fight_chance = args.contest_fights;
    config.random_fight_chance = args.random_fights;
    if let Some(damage) = args.fight_damage {
        config.fight_damage = damage;
    }
    if let Some(loss) = args.fight_smartness_loss {
        config.fight_smartness_loss = loss;
    }
    config.leaders_enabled = args.leaders;
    config.term_judgement = args.judge_terms.into();
    if let Some(interval) = args.election_interval {
        config.election_interval = interval;
    }
    if let Some(influence) = args.leader_influence {
        config.leader_influence = influence;
    }
    if let Some(terms) = args.max_terms {
        config.max_terms = terms;
    }
    if let Some(memory) = args.memory {
        config.max_memory = memory;
    }
    config.brains_enabled = args.brains;
    config.brains_inherited = !args.random_brains;
    if let Some(rate) = args.brain_mutation_rate {
        config.brain_mutation_rate = rate;
    }
    if let Some(step) = args.brain_mutation_step {
        config.brain_mutation_step = step;
    }
    config.learning_enabled = args.learning;
    if let Some(rate) = args.learning_rate {
        config.learning_rate = rate;
    }
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
    )?;
    if totals.elections > 0 {
        writeln!(out, "elections {}", totals.elections)?;
    }
    if totals.fights > 0 {
        writeln!(
            out,
            "fights {} | killed in fights {}",
            totals.fights, totals.combat
        )?;
    }
    Ok(())
}
