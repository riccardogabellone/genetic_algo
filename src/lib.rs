//! Agent-based evolution sandbox.
//!
//! The simulation core (`config`, `world`, `individual`, `behavior`, `simulation`, `stats`)
//! knows nothing about rendering. Front ends read the world and the history through
//! [`simulation::Simulation`]; `render` holds the terminal, chart and GIF front ends.

pub mod behavior;
pub mod config;
pub mod individual;
pub mod render;
pub mod sampling;
pub mod simulation;
pub mod stats;
pub mod world;
