//! Run logic behind the window: what is running, how fast, and with which rules.

use std::time::{Duration, Instant};

use genetic_algo::{
    config::{Config, ConfigError, MAX_SMARTNESS},
    leadership::TermJudgement,
    sampling::Spread,
    simulation::{Outcome, Simulation},
    stats::TickEvents,
};

/// Long enough that a run only stops for a reason the user cares about.
const MAX_RUN_TICKS: u32 = 1_000_000;
/// At most this much simulated time is caught up per frame, so a slow frame cannot snowball.
const MAX_CATCH_UP_SECONDS: f32 = 0.25;
const SMART_START: Spread = Spread {
    min: 85.0,
    mode: 90.0,
    max: 95.0,
};

/// Rules that can change while a run is in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct Rules {
    pub smartness_cost: f32,
    pub vision: usize,
    pub max_speed: usize,
    pub speed_cost: f32,
    /// Percentage of all cells that regrow as food per tick.
    pub food_regrowth_percent: f32,
    pub food_gain: u32,
    pub hazard_damage: u32,
    pub mutation_rate: f32,
    pub mutation_step: f32,
    pub contest_fights: f32,
    pub random_fights: f32,
    pub fight_damage: u32,
    pub fight_smartness_loss: f32,
    pub leaders: bool,
    pub election_interval: u32,
    pub leader_influence: f32,
    pub max_terms: u32,
    pub memory: usize,
    pub term_judgement: TermJudgement,
    pub brains: bool,
    pub brains_inherited: bool,
    pub brain_mutation_rate: f32,
    pub brain_mutation_step: u8,
    pub stop_at_target: bool,
    pub target: f32,
}

impl Default for Rules {
    fn default() -> Self {
        let defaults = Config::default();
        let cells = (defaults.width * defaults.height) as f32;
        Self {
            smartness_cost: defaults.smartness_upkeep,
            vision: defaults.max_vision,
            max_speed: defaults.max_speed,
            speed_cost: defaults.speed_upkeep,
            food_regrowth_percent: defaults.food_regrowth_amount as f32 / cells * 100.0,
            food_gain: defaults.food_gain,
            hazard_damage: defaults.hazard_damage,
            mutation_rate: defaults.mutation_rate,
            mutation_step: defaults.smartness_mutation_step,
            contest_fights: defaults.contest_fight_chance,
            random_fights: defaults.random_fight_chance,
            fight_damage: defaults.fight_damage,
            fight_smartness_loss: defaults.fight_smartness_loss,
            leaders: defaults.leaders_enabled,
            election_interval: defaults.election_interval,
            leader_influence: defaults.leader_influence,
            max_terms: defaults.max_terms,
            memory: defaults.max_memory,
            term_judgement: defaults.term_judgement,
            brains: defaults.brains_enabled,
            brains_inherited: defaults.brains_inherited,
            brain_mutation_rate: defaults.brain_mutation_rate,
            brain_mutation_step: defaults.brain_mutation_step,
            stop_at_target: true,
            target: defaults.target_smartness,
        }
    }
}

impl Rules {
    fn apply_to(&self, config: &mut Config) {
        config.smartness_upkeep = self.smartness_cost;
        config.max_vision = self.vision;
        config.max_speed = self.max_speed;
        config.speed_upkeep = self.speed_cost;
        let cells = (config.width * config.height) as f32;
        let regrowth = (cells * self.food_regrowth_percent / 100.0).round() as usize;
        config.food_regrowth_amount = if self.food_regrowth_percent > 0.0 {
            regrowth.max(1)
        } else {
            0
        };
        config.food_gain = self.food_gain;
        config.hazard_damage = self.hazard_damage;
        config.mutation_rate = self.mutation_rate;
        config.smartness_mutation_step = self.mutation_step;
        config.contest_fight_chance = self.contest_fights;
        config.random_fight_chance = self.random_fights;
        config.fight_damage = self.fight_damage;
        config.fight_smartness_loss = self.fight_smartness_loss;
        config.leaders_enabled = self.leaders;
        config.election_interval = self.election_interval;
        config.leader_influence = self.leader_influence;
        config.max_terms = self.max_terms;
        config.max_memory = self.memory;
        config.term_judgement = self.term_judgement;
        config.brains_enabled = self.brains;
        config.brains_inherited = self.brains_inherited;
        config.brain_mutation_rate = self.brain_mutation_rate;
        config.brain_mutation_step = self.brain_mutation_step;
        // The scale's maximum is unreachable in practice, because mutation keeps some
        // individuals below it. That is how "never stop for the target" is expressed.
        config.target_smartness = if self.stop_at_target {
            self.target
        } else {
            MAX_SMARTNESS
        };
    }
}

/// Rule sets from the experiments in the README.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Default,
    VisionAndCost,
    AllRules,
    TwoStableOutcomes,
    Fighting,
    Leaders,
    Brains,
}

impl Preset {
    pub const ALL: [Self; 7] = [
        Self::Default,
        Self::VisionAndCost,
        Self::AllRules,
        Self::TwoStableOutcomes,
        Self::Fighting,
        Self::Leaders,
        Self::Brains,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::VisionAndCost => "Vision + cost",
            Self::AllRules => "All rules",
            Self::TwoStableOutcomes => "Two outcomes",
            Self::Fighting => "Fighting",
            Self::Leaders => "Leaders",
            Self::Brains => "Brains",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Default => "No optional rules.",
            Self::VisionAndCost => "Vision 5, smartness cost 2. Steady, reaches the target.",
            Self::AllRules => "Adds evolving speed. Fast, but fragile on small maps.",
            Self::Fighting => "Fights over food, and a few for no reason.",
            Self::Leaders => "Elected leaders in a world with fights, so their aggression matters.",
            Self::Brains => {
                "Evolved neural networks choose actions instead of the hand-written rules."
            }
            Self::TwoStableOutcomes => {
                "Vision 5, smartness cost 8. Stays dumb from a normal start; \
                 stays smart with \"Start smart\" enabled."
            }
        }
    }

    fn rules(self) -> Rules {
        let base = Rules::default();
        match self {
            Self::Default => base,
            Self::VisionAndCost => Rules {
                smartness_cost: 2.0,
                vision: 5,
                ..base
            },
            Self::AllRules => Rules {
                smartness_cost: 2.0,
                vision: 5,
                max_speed: 4,
                speed_cost: 1.0,
                ..base
            },
            Self::Brains => Rules {
                brains: true,
                ..base
            },
            Self::Leaders => Rules {
                leaders: true,
                contest_fights: 0.5,
                random_fights: 0.02,
                ..base
            },
            Self::Fighting => Rules {
                contest_fights: 0.5,
                random_fights: 0.02,
                ..base
            },
            Self::TwoStableOutcomes => Rules {
                smartness_cost: 8.0,
                vision: 5,
                ..base
            },
        }
    }
}

/// Settings that only take effect when a new run starts.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldSetup {
    pub width: usize,
    pub height: usize,
    pub start_smart: bool,
    pub seed_text: String,
}

pub struct Controller {
    pub rules: Rules,
    pub setup: WorldSetup,
    pub ticks_per_second: f32,
    simulation: Simulation,
    seed: u64,
    running: bool,
    pending_ticks: f32,
    totals: TickEvents,
    counted_entries: usize,
    error: Option<String>,
}

impl Controller {
    pub fn new(seed: u64) -> Self {
        let rules = Rules::default();
        let defaults = Config::default();
        let setup = WorldSetup {
            width: defaults.width,
            height: defaults.height,
            start_smart: false,
            seed_text: seed.to_string(),
        };
        let simulation = Simulation::new(build_config(&setup, &rules), seed)
            .expect("the default rules are valid");
        Self {
            rules,
            setup,
            ticks_per_second: 30.0,
            simulation,
            seed,
            running: false,
            pending_ticks: 0.0,
            totals: TickEvents::default(),
            counted_entries: 1,
            error: None,
        }
    }

    pub fn simulation(&self) -> &Simulation {
        &self.simulation
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn totals(&self) -> &TickEvents {
        &self.totals
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn outcome_message(&self) -> Option<String> {
        let tick = self.simulation.latest().tick;
        self.simulation.outcome().map(|outcome| match outcome {
            Outcome::Extinct => format!("Extinct at tick {tick}."),
            Outcome::TargetReached => {
                format!("Target reached at tick {tick}. Untick \"Stop at target\" to continue.")
            }
            Outcome::TicksExhausted => format!("Tick limit reached at tick {tick}."),
        })
    }

    /// Running is only possible while the run has no outcome.
    pub fn toggle_running(&mut self) {
        self.running = !self.running && self.simulation.outcome().is_none();
        self.pending_ticks = 0.0;
    }

    /// Advances exactly one tick, unless the run is over.
    pub fn step_once(&mut self) {
        if self.simulation.outcome().is_none() {
            self.simulation.step();
            self.count_new_events();
        }
        self.pause_if_finished();
    }

    /// Runs the ticks due after `elapsed_seconds`, giving up after `budget` so the window
    /// stays responsive. Returns how many ticks ran. Ticks that do not fit are dropped.
    pub fn advance(&mut self, elapsed_seconds: f32, budget: Duration) -> u32 {
        if !self.running {
            return 0;
        }
        let due = elapsed_seconds.min(MAX_CATCH_UP_SECONDS) * self.ticks_per_second;
        self.pending_ticks += due;
        let started = Instant::now();
        let mut ran = 0;
        while self.running && self.pending_ticks >= 1.0 {
            if started.elapsed() >= budget {
                self.pending_ticks = 0.0;
                break;
            }
            self.step_once();
            self.pending_ticks -= 1.0;
            ran += 1;
        }
        ran
    }

    /// Applies edited rules to the current run. On failure the run keeps its old rules.
    pub fn apply_rules(&mut self) {
        let mut config = self.simulation.config().clone();
        self.rules.apply_to(&mut config);
        self.error = self
            .simulation
            .set_config(config)
            .err()
            .map(|error: ConfigError| error.to_string());
    }

    pub fn apply_preset(&mut self, preset: Preset) {
        self.rules = preset.rules();
        self.apply_rules();
    }

    /// Starts over with the seed in the text field. Invalid input leaves the run untouched.
    pub fn replay_seed(&mut self) {
        match self.setup.seed_text.trim().parse() {
            Ok(seed) => self.restart(seed),
            Err(_) => self.error = Some("The seed must be a whole number.".to_owned()),
        }
    }

    pub fn start_new_run(&mut self) {
        self.restart(u64::from(rand::random::<u32>()));
    }

    fn restart(&mut self, seed: u64) {
        match Simulation::new(build_config(&self.setup, &self.rules), seed) {
            Ok(simulation) => {
                self.simulation = simulation;
                self.seed = seed;
                self.setup.seed_text = seed.to_string();
                self.running = false;
                self.pending_ticks = 0.0;
                self.totals = TickEvents::default();
                self.counted_entries = 1;
                self.error = None;
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }

    fn count_new_events(&mut self) {
        let history = self.simulation.history();
        for stats in &history[self.counted_entries..] {
            self.totals.add(&stats.events);
        }
        self.counted_entries = history.len();
    }

    fn pause_if_finished(&mut self) {
        if self.simulation.outcome().is_some() {
            self.running = false;
        }
    }
}

fn build_config(setup: &WorldSetup, rules: &Rules) -> Config {
    let mut config = Config {
        max_ticks: MAX_RUN_TICKS,
        ..Config::for_size(setup.width, setup.height)
    };
    if setup.start_smart {
        config.founder_smartness = SMART_START;
    }
    rules.apply_to(&mut config);
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOTS_OF_TIME: Duration = Duration::from_secs(60);

    fn controller() -> Controller {
        Controller::new(1)
    }

    fn tick(controller: &Controller) -> u32 {
        controller.simulation().latest().tick
    }

    #[test]
    fn default_rules_match_the_core_defaults() {
        let controller = controller();
        let core = Config {
            max_ticks: MAX_RUN_TICKS,
            ..Config::default()
        };
        assert_eq!(*controller.simulation().config(), core);
    }

    #[test]
    fn nothing_advances_while_paused() {
        let mut controller = controller();
        assert_eq!(controller.advance(1.0, LOTS_OF_TIME), 0);
        assert_eq!(tick(&controller), 0);
    }

    #[test]
    fn ticks_follow_elapsed_time_and_speed() {
        let mut controller = controller();
        controller.ticks_per_second = 100.0;
        controller.toggle_running();
        assert_eq!(controller.advance(0.1, LOTS_OF_TIME), 10);
        assert_eq!(tick(&controller), 10);
    }

    #[test]
    fn fractions_of_a_tick_accumulate_across_frames() {
        let mut controller = controller();
        controller.ticks_per_second = 10.0;
        controller.toggle_running();
        let total: u32 = (0..10)
            .map(|_| controller.advance(0.025, LOTS_OF_TIME))
            .sum();
        assert_eq!(total, 2);
    }

    #[test]
    fn a_long_frame_is_capped_instead_of_snowballing() {
        let mut controller = controller();
        controller.ticks_per_second = 100.0;
        controller.toggle_running();
        assert_eq!(controller.advance(1_000.0, LOTS_OF_TIME), 25);
    }

    #[test]
    fn an_exhausted_budget_drops_the_remaining_ticks() {
        let mut controller = controller();
        controller.ticks_per_second = 1_000_000.0;
        controller.toggle_running();
        assert_eq!(controller.advance(0.25, Duration::ZERO), 0);
        assert_eq!(controller.advance(0.0, LOTS_OF_TIME), 0);
    }

    #[test]
    fn step_once_advances_exactly_one_tick() {
        let mut controller = controller();
        controller.step_once();
        assert_eq!(tick(&controller), 1);
        assert!(!controller.is_running());
    }

    fn extinct_controller() -> Controller {
        let mut controller = controller();
        controller.setup.width = 8;
        controller.setup.height = 8;
        controller.rules.food_regrowth_percent = 0.0;
        controller.setup.seed_text = "3".to_owned();
        controller.replay_seed();
        controller.ticks_per_second = 10_000.0;
        controller.toggle_running();
        while controller.is_running() {
            controller.advance(0.25, LOTS_OF_TIME);
        }
        controller
    }

    #[test]
    fn extinction_pauses_and_blocks_resuming_and_stepping() {
        let mut controller = extinct_controller();
        assert_eq!(controller.simulation().latest().population, 0);
        assert!(controller.outcome_message().unwrap().contains("Extinct"));

        let final_tick = tick(&controller);
        controller.toggle_running();
        assert!(!controller.is_running());
        controller.step_once();
        assert_eq!(tick(&controller), final_tick);
    }

    #[test]
    fn a_new_run_recovers_from_extinction() {
        let mut controller = extinct_controller();
        controller.rules = Rules::default();
        controller.start_new_run();
        assert!(controller.simulation().latest().population > 0);
        assert_eq!(tick(&controller), 0);
        assert_eq!(controller.outcome_message(), None);
        assert_eq!(*controller.totals(), TickEvents::default());
    }

    #[test]
    fn reaching_the_target_pauses_and_disabling_it_allows_resuming() {
        let mut controller = controller();
        controller.setup.start_smart = true;
        controller.rules.target = 50.0;
        controller.replay_seed();
        assert!(controller.outcome_message().unwrap().contains("Target"));

        controller.toggle_running();
        assert!(!controller.is_running());

        controller.rules.stop_at_target = false;
        controller.apply_rules();
        assert_eq!(controller.outcome_message(), None);
        controller.toggle_running();
        assert!(controller.is_running());
    }

    #[test]
    fn rule_edits_apply_to_the_running_simulation() {
        let mut controller = controller();
        controller.rules.smartness_cost = 3.5;
        controller.rules.vision = 7;
        controller.rules.max_speed = 4;
        controller.rules.speed_cost = 1.5;
        controller.apply_rules();
        let config = controller.simulation().config();
        assert_eq!(config.smartness_upkeep, 3.5);
        assert_eq!(config.max_vision, 7);
        assert_eq!(config.max_speed, 4);
        assert_eq!(config.speed_upkeep, 1.5);
    }

    #[test]
    fn fight_edits_apply_to_the_running_simulation() {
        let mut controller = controller();
        controller.rules.contest_fights = 0.75;
        controller.rules.random_fights = 0.25;
        controller.rules.fight_damage = 40;
        controller.rules.fight_smartness_loss = 12.0;
        controller.apply_rules();
        let config = controller.simulation().config();
        assert_eq!(config.contest_fight_chance, 0.75);
        assert_eq!(config.random_fight_chance, 0.25);
        assert_eq!(config.fight_damage, 40);
        assert_eq!(config.fight_smartness_loss, 12.0);
    }

    #[test]
    fn leader_edits_apply_to_the_running_simulation() {
        let mut controller = controller();
        controller.rules.leaders = true;
        controller.rules.election_interval = 12;
        controller.rules.leader_influence = 0.9;
        controller.rules.max_terms = 3;
        controller.rules.memory = 7;
        controller.rules.term_judgement = TermJudgement::Personal;
        controller.apply_rules();
        let config = controller.simulation().config();
        assert!(config.leaders_enabled);
        assert_eq!(config.election_interval, 12);
        assert_eq!(config.leader_influence, 0.9);
        assert_eq!(config.max_terms, 3);
        assert_eq!(config.max_memory, 7);
        assert_eq!(config.term_judgement, TermJudgement::Personal);
    }

    #[test]
    fn brain_edits_apply_to_the_running_simulation() {
        let mut controller = controller();
        controller.rules.brains = true;
        controller.rules.brains_inherited = false;
        controller.rules.brain_mutation_rate = 0.3;
        controller.rules.brain_mutation_step = 20;
        controller.apply_rules();
        let config = controller.simulation().config();
        assert!(config.brains_enabled);
        assert!(!config.brains_inherited);
        assert_eq!(config.brain_mutation_rate, 0.3);
        assert_eq!(config.brain_mutation_step, 20);
    }

    #[test]
    fn invalid_rule_edits_keep_the_old_rules_and_report_why() {
        let mut controller = controller();
        controller.rules.smartness_cost = f32::NAN;
        controller.apply_rules();
        assert!(controller.error().is_some());
        assert_eq!(controller.simulation().config().smartness_upkeep, 0.0);

        controller.rules.smartness_cost = 1.0;
        controller.apply_rules();
        assert_eq!(controller.error(), None);
    }

    #[test]
    fn world_setup_waits_for_a_new_run() {
        let mut controller = controller();
        controller.setup.width = 64;
        controller.apply_rules();
        assert_eq!(controller.simulation().config().width, 32);
        assert_eq!(controller.error(), None);

        controller.start_new_run();
        assert_eq!(controller.simulation().config().width, 64);
    }

    #[test]
    fn food_regrowth_scales_with_the_map_and_can_be_off() {
        let mut controller = controller();
        assert_eq!(controller.simulation().config().food_regrowth_amount, 32);

        controller.setup.width = 16;
        controller.setup.height = 16;
        controller.start_new_run();
        assert_eq!(controller.simulation().config().food_regrowth_amount, 8);

        controller.rules.food_regrowth_percent = 0.0;
        controller.apply_rules();
        assert_eq!(controller.simulation().config().food_regrowth_amount, 0);

        controller.rules.food_regrowth_percent = 0.001;
        controller.apply_rules();
        assert_eq!(controller.simulation().config().food_regrowth_amount, 1);
    }

    #[test]
    fn replaying_a_seed_repeats_the_run() {
        let run = |controller: &mut Controller| {
            controller.setup.seed_text = "77".to_owned();
            controller.replay_seed();
            for _ in 0..50 {
                controller.step_once();
            }
            controller.simulation().world().clone()
        };
        let mut controller = controller();
        let first = run(&mut controller);
        let second = run(&mut controller);
        assert_eq!(first, second);
        assert_eq!(controller.seed(), 77);
    }

    #[test]
    fn an_invalid_seed_reports_an_error_and_keeps_the_run() {
        let mut controller = controller();
        controller.step_once();
        for text in ["", "abc", "-1", "1.5"] {
            controller.setup.seed_text = text.to_owned();
            controller.replay_seed();
            assert!(controller.error().is_some(), "{text:?}");
            assert_eq!(tick(&controller), 1);
        }
    }

    #[test]
    fn totals_count_every_event_exactly_once() {
        let mut controller = controller();
        for _ in 0..100 {
            controller.step_once();
        }
        let mut expected = TickEvents::default();
        for stats in controller.simulation().history() {
            expected.add(&stats.events);
        }
        assert_eq!(*controller.totals(), expected);
        assert!(expected.meals > 0);
    }

    #[test]
    fn every_preset_is_valid_and_applies_live() {
        let mut controller = controller();
        for preset in Preset::ALL {
            controller.apply_preset(preset);
            assert_eq!(controller.error(), None, "{preset:?}");
            assert!(!preset.label().is_empty() && !preset.description().is_empty());
            controller.step_once();
        }
    }
}
