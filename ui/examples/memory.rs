//! Opens the real window, plays several runs back to back with the optional rules on, and
//! reports the memory in use after each one. Fails if memory keeps growing from run to run.
//!
//! Usage: `cargo run --release -p genetic_algo_ui --example memory -- [runs] [frames_per_run]`
//!
//! Two numbers are checked, because they grow for different reasons. The heap is what the
//! app's own data takes, counted allocation by allocation: it grows if anything outlives
//! the run it belongs to. The process total also holds what the graphics driver takes for
//! each frame drawn: it grows if the renderer leaks, as OpenGL did on Windows.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    process::ExitCode,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering::Relaxed},
    },
};

use eframe::egui;
use genetic_algo_ui::{app::App, controller::Preset, renderer::native_options};

const MEGABYTE: f64 = 1_048_576.0;
/// The first runs fill caches that then stay: fonts, buffers, the driver's own pools.
const WARM_UP_RUNS: usize = 2;
/// Growth after the warm-up that counts as a leak. Run-to-run noise measured well below
/// both; the OpenGL leak on Windows added about 6 MB per run of 300 frames.
const HEAP_LIMIT_MB: f64 = 1.0;
const PROCESS_LIMIT_MB: f64 = 16.0;
const MAP_SIDE: usize = 64;

/// Bytes the app has allocated and not yet freed.
static HEAP_BYTES: AtomicUsize = AtomicUsize::new(0);
static SAMPLES: Mutex<Vec<Sample>> = Mutex::new(Vec::new());

struct CountingAllocator;

// SAFETY: every call is passed on to the system allocator unchanged; this only counts.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP_BYTES.fetch_add(layout.size(), Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        HEAP_BYTES.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        HEAP_BYTES.fetch_add(new_size, Relaxed);
        HEAP_BYTES.fetch_sub(layout.size(), Relaxed);
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Memory only this process holds, in megabytes: the driver's share included.
#[cfg(target_os = "linux")]
fn process_megabytes() -> Option<f64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("RssAnon:"))?;
    let kilobytes: f64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kilobytes / 1024.0)
}

#[cfg(windows)]
fn process_megabytes() -> Option<f64> {
    /// `PROCESS_MEMORY_COUNTERS` from the Windows API.
    #[repr(C)]
    #[derive(Default)]
    struct Counters {
        size: u32,
        page_faults: u32,
        peak_working_set: usize,
        working_set: usize,
        pool_quotas: [usize; 4],
        private_bytes: usize,
        peak_private_bytes: usize,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn K32GetProcessMemoryInfo(process: isize, counters: *mut Counters, size: u32) -> i32;
    }

    let size = u32::try_from(std::mem::size_of::<Counters>()).ok()?;
    let mut counters = Counters {
        size,
        ..Counters::default()
    };
    // SAFETY: `counters` is a valid, writable struct of the size passed along with it.
    let filled = unsafe { K32GetProcessMemoryInfo(GetCurrentProcess(), &raw mut counters, size) };
    (filled != 0).then_some(counters.private_bytes as f64 / MEGABYTE)
}

/// Not measured on this system: only the heap is checked.
#[cfg(not(any(target_os = "linux", windows)))]
fn process_megabytes() -> Option<f64> {
    None
}

#[derive(Debug, Clone, Copy)]
struct Sample {
    heap: f64,
    process: Option<f64>,
}

impl Sample {
    fn take() -> Self {
        Self {
            heap: HEAP_BYTES.load(Relaxed) as f64 / MEGABYTE,
            process: process_megabytes(),
        }
    }
}

struct MemoryCheck {
    app: App,
    runs: usize,
    frames_per_run: u32,
    frames_left: u32,
}

impl MemoryCheck {
    fn start_run(&mut self) {
        self.app.controller.start_new_run();
        self.app.controller.toggle_running();
        self.frames_left = self.frames_per_run;
    }
}

impl eframe::App for MemoryCheck {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.app.show(ui);
        ui.ctx().request_repaint();
        self.frames_left = self.frames_left.saturating_sub(1);
        if self.frames_left > 0 {
            return;
        }

        let finished = *self.app.controller.simulation().latest();
        // Measured after the next run replaced this one, when nothing of it should be left.
        self.start_run();
        let sample = Sample::take();
        let mut samples = SAMPLES.lock().expect("no other thread takes samples");
        samples.push(sample);
        let process = sample.process.map_or_else(
            || "n/a".to_owned(),
            |megabytes| format!("{megabytes:.1} MB"),
        );
        println!(
            "run {:>2}: {:>6} ticks, population {:>5}, heap {:>6.2} MB, process {process}",
            samples.len(),
            finished.tick,
            finished.population,
            sample.heap,
        );
        if samples.len() >= self.runs {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// Every optional rule at once, so each kind of per-individual data is allocated.
fn all_rules_on(app: &mut App) {
    let controller = &mut app.controller;
    controller.apply_preset(Preset::AllRules);
    let rules = &mut controller.rules;
    rules.contest_fights = 0.5;
    rules.random_fights = 0.02;
    rules.leaders = true;
    rules.election_interval = 2;
    rules.brains = true;
    rules.learning = true;
    rules.stop_at_target = false;
    controller.setup.width = MAP_SIDE;
    controller.setup.height = MAP_SIDE;
    controller.ticks_per_second = 2_000.0;
}

/// How much `measure` grew between the end of the warm-up and the last run.
fn growth(samples: &[Sample], measure: impl Fn(&Sample) -> Option<f64>) -> Option<f64> {
    let settled = measure(samples.get(WARM_UP_RUNS - 1)?)?;
    Some(measure(samples.last()?)? - settled)
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let runs = args.next().map_or(10, |runs| runs.parse().expect("runs"));
    let frames_per_run = args
        .next()
        .map_or(300, |frames| frames.parse().expect("frames_per_run"));
    assert!(runs > WARM_UP_RUNS, "at least {} runs", WARM_UP_RUNS + 1);

    let options = native_options(egui::ViewportBuilder::default().with_inner_size([1360.0, 820.0]));
    let shown = eframe::run_native(
        "genetic_algo_ui_memory",
        options,
        Box::new(move |creation| {
            let mut check = MemoryCheck {
                app: App::new(&creation.egui_ctx),
                runs,
                frames_per_run,
                frames_left: 0,
            };
            all_rules_on(&mut check.app);
            check.start_run();
            Ok(Box::new(check))
        }),
    );
    if let Err(error) = shown {
        eprintln!("the window could not be shown: {error}");
        return ExitCode::FAILURE;
    }

    let samples = SAMPLES.lock().expect("the window is closed");
    if samples.len() < runs {
        eprintln!("closed after {} of {runs} runs", samples.len());
        return ExitCode::FAILURE;
    }
    let mut leaked = false;
    for (name, grown, limit) in [
        (
            "heap",
            growth(&samples, |sample| Some(sample.heap)),
            HEAP_LIMIT_MB,
        ),
        (
            "process",
            growth(&samples, |sample| sample.process),
            PROCESS_LIMIT_MB,
        ),
    ] {
        let Some(grown) = grown else {
            println!("{name}: not measured on this system");
            continue;
        };
        let verdict = if grown > limit { "LEAK" } else { "ok" };
        println!("{name}: {grown:+.2} MB since run {WARM_UP_RUNS} (limit {limit} MB): {verdict}");
        leaked |= grown > limit;
    }
    if leaked {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
