//! Front ends and the command line, including their failure paths.

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

use genetic_algo::{
    config::Config,
    render::{chart::write_chart, gif::GifRecorder},
    simulation::Simulation,
};

fn temp_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name)
}

fn has_png_signature(path: &PathBuf) -> bool {
    fs::read(path).is_ok_and(|bytes| bytes.starts_with(b"\x89PNG"))
}

fn cli(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_genetic_algo"))
        .args(arguments)
        .output()
        .expect("the binary runs")
}

#[test]
fn chart_handles_a_normal_run() {
    let mut simulation = Simulation::new(
        Config {
            max_ticks: 50,
            ..Config::default()
        },
        1,
    )
    .unwrap();
    simulation.run_to_end();
    let path = temp_path("normal.png");
    write_chart(&path, simulation.history(), 80.0).unwrap();
    assert!(has_png_signature(&path));
}

#[test]
fn chart_handles_a_single_tick() {
    let simulation = Simulation::new(
        Config {
            max_ticks: 0,
            ..Config::default()
        },
        1,
    )
    .unwrap();
    let path = temp_path("single.png");
    write_chart(&path, simulation.history(), 80.0).unwrap();
    assert!(has_png_signature(&path));
}

#[test]
fn chart_handles_a_population_that_never_existed() {
    let config = Config {
        initial_population: 0,
        ..Config::for_size(4, 4)
    };
    let simulation = Simulation::new(config, 1).unwrap();
    let path = temp_path("empty.png");
    write_chart(&path, simulation.history(), 80.0).unwrap();
    assert!(has_png_signature(&path));
}

#[test]
fn chart_handles_extinction_midway() {
    let config = Config {
        initial_population: 2,
        food_regrowth_amount: 0,
        initial_food: 0,
        ..Config::for_size(4, 4)
    };
    let mut simulation = Simulation::new(config, 1).unwrap();
    simulation.run_to_end();
    assert_eq!(simulation.latest().population, 0);
    let path = temp_path("extinct.png");
    write_chart(&path, simulation.history(), 80.0).unwrap();
    assert!(has_png_signature(&path));
}

#[test]
fn gif_records_every_frame_it_is_given() {
    let mut simulation = Simulation::new(Config::for_size(8, 8), 1).unwrap();
    let path = temp_path("run.gif");
    {
        let mut recorder = GifRecorder::create(&path, 8, 8).unwrap();
        for _ in 0..5 {
            recorder.record(simulation.world()).unwrap();
            simulation.step();
        }
        assert_eq!(recorder.frames(), 5);
    }
    assert!(fs::read(&path).unwrap().starts_with(b"GIF8"));
}

#[test]
fn gif_rejects_a_map_too_large_for_the_format() {
    let path = temp_path("huge.gif");
    assert!(GifRecorder::create(&path, 100_000, 8).is_err());
}

// ---------- Command line ----------

#[test]
fn a_seed_replays_the_same_output() {
    let chart = temp_path("cli_seed.png");
    let chart = chart.to_str().unwrap();
    let arguments = [
        "--width", "12", "--height", "12", "--ticks", "60", "--seed", "9", "--chart", chart,
    ];
    let (first, second) = (cli(&arguments), cli(&arguments));
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert!(String::from_utf8_lossy(&first.stdout).contains("seed 9"));
}

#[test]
fn no_frames_are_written_when_stdout_is_not_a_terminal() {
    let chart = temp_path("cli_pipe.png");
    let output = cli(&[
        "--width",
        "8",
        "--height",
        "8",
        "--ticks",
        "3",
        "--seed",
        "1",
        "--chart",
        chart.to_str().unwrap(),
    ]);
    assert!(!String::from_utf8_lossy(&output.stdout).contains('\x1B'));
}

#[test]
fn invalid_input_fails_with_a_message() {
    for arguments in [
        &["--width", "0"][..],
        &["--height", "0"],
        &["--width", "99999", "--height", "99999"],
        &["--ticks", "99999999"],
        &["--gif-every", "0"],
        &["--width", "abc"],
        &["--chart", "/definitely/not/a/dir/out.png"],
        &["--gif", "/definitely/not/a/dir/out.gif"],
        &["--smartness-cost", "-1"],
        &["--smartness-cost", "NaN"],
        &["--speed-cost", "inf"],
        &["--vision", "1000"],
        &["--max-speed", "0"],
        &["--max-speed", "1000"],
        &["--contest-fights", "2"],
        &["--random-fights", "-1"],
        &["--fight-damage", "0"],
        &["--fight-smartness-loss", "1000"],
        &["--chart", "not_a_png.txt"],
        &["--chart", "no_extension"],
        &["--chart", "/tmp/"],
        &[
            "--width",
            "4096",
            "--height",
            "4096",
            "--gif",
            "/tmp/never_written.gif",
        ],
    ] {
        let output = cli(arguments);
        assert!(!output.status.success(), "{arguments:?}");
        assert!(!output.stderr.is_empty(), "{arguments:?}");
    }
}
