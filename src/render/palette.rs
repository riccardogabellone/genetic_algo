//! How each cell looks. Single source of truth shared by every front end.

use crate::{
    config::MAX_SMARTNESS,
    individual::{Gender, Individual},
    world::Cell,
};

pub type Rgb = (u8, u8, u8);

pub const BACKGROUND: Rgb = (22, 24, 29);

const FOOD: Rgb = (92, 184, 92);
const WALL: Rgb = (150, 150, 158);
const HAZARD: Rgb = (229, 72, 77);
const MALE: Rgb = (84, 174, 255);
const FEMALE: Rgb = (240, 130, 220);

/// Brightness of the least smart individual; the smartest is at full brightness.
const MIN_BRIGHTNESS: f32 = 0.45;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Appearance {
    pub glyph: char,
    pub color: Rgb,
}

pub fn appearance(cell: &Cell) -> Appearance {
    let (glyph, color) = match cell {
        Cell::Empty => (' ', BACKGROUND),
        Cell::Food => ('*', FOOD),
        Cell::Wall => ('#', WALL),
        Cell::Hazard => ('X', HAZARD),
        Cell::Individual(individual) => (gender_glyph(individual), individual_color(individual)),
    };
    Appearance { glyph, color }
}

fn gender_glyph(individual: &Individual) -> char {
    match individual.gender {
        Gender::Male => 'M',
        Gender::Female => 'F',
    }
}

/// Smarter individuals glow brighter, so evolution is visible on the map itself.
fn individual_color(individual: &Individual) -> Rgb {
    let base = match individual.gender {
        Gender::Male => MALE,
        Gender::Female => FEMALE,
    };
    let score = (individual.smartness / MAX_SMARTNESS).clamp(0.0, 1.0);
    let brightness = MIN_BRIGHTNESS + (1.0 - MIN_BRIGHTNESS) * score;
    let scale = |channel: u8| (f32::from(channel) * brightness).round() as u8;
    (scale(base.0), scale(base.1), scale(base.2))
}
