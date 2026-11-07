# AGENTS.md

Instructions for AI coding agents working on this repository. Humans should start with [README.md](README.md).

## Project overview
An agent-based evolution sandbox in Rust: small individuals roam a grid and evolve over generations by natural selection. The README documents every rule; keep it in sync with the code.

- `src/`: the simulation core and the terminal front end (`genetic_algo` crate)
- `ui/`: the egui front end (`genetic_algo_ui` crate), kept separate so the core does not depend on egui
- `tests/`, `ui/tests/`: integration tests; `examples/`: experiments behind the README's tables

## Setup and commands
```sh
cargo build --workspace
cargo test --workspace                                  # all tests
cargo fmt --all --check                                 # formatting
cargo clippy --workspace --all-targets -- -D warnings   # lints, warnings are errors
cargo run --release -- --help                           # CLI options
cargo run --release -p genetic_algo_ui                  # graphical UI
```
On WSL/Linux the UI needs `sudo apt install libegl1 libegl-mesa0`.

## Code style
- Use clear and descriptive names.
- Write comments to explain complex logic, not to restate the code.
- Keep functions small and focused on a single task.
- Every rule value lives in `src/config.rs` and is validated there, once, at start-up.
- `behavior` only reads the world; state changes happen in `simulation`.
- The core must not depend on `render` or on the `ui` crate.
- Code, comments, docs and commit messages are in English.

## Testing
- Every rule in the README has a scripted scenario in `tests/scenarios.rs`; add one when you add or change a rule.
- Runs are reproducible from a seed, so tests that need randomness use fixed seeds.
- A rise in smartness alone does not prove evolution: compare against the control run (`cargo run --release --example selection_vs_control`).

## Pull requests
- `main` is protected: work on a branch and open a PR.
- One feature per commit and per PR.
- Before opening a PR, run the format, lint and test commands above, and update the README if behaviour changed.
