//! Graphical front end. `controller` holds all run logic and has no drawing code, so it
//! can be tested without a window; `app` and its view modules only draw and report input.

pub mod app;
pub mod charts;
pub mod controller;
pub mod identity;
pub mod map_view;
pub mod rules_panel;
pub mod stats_panel;
pub mod theme;
pub mod toolbar;
pub mod widgets;
