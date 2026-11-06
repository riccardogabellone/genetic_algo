//! Terminal front end.

use colored::Colorize;

use crate::{
    stats::{SmartnessSummary, TickStats},
    world::{Cell, Position, World},
};

use super::palette::appearance;

/// Moves the cursor home and clears the screen.
pub const CLEAR_SCREEN: &str = "\x1B[2J\x1B[1;1H";

/// One frame of the map as a single string, so it can be written without flicker.
pub fn map_frame(world: &World, use_color: bool) -> String {
    let border = format!("+{}+", "-".repeat(world.width() * 2));
    let mut frame = String::with_capacity((world.width() * 2 + 3) * (world.height() + 2));
    frame.push_str(&border);
    frame.push('\n');
    for y in 0..world.height() {
        frame.push('|');
        for x in 0..world.width() {
            push_cell(&mut frame, world.get(Position { x, y }), use_color);
            frame.push(' ');
        }
        frame.push_str("|\n");
    }
    frame.push_str(&border);
    frame.push('\n');
    frame
}

fn push_cell(frame: &mut String, cell: &Cell, use_color: bool) {
    let look = appearance(cell);
    if use_color && *cell != Cell::Empty {
        let (red, green, blue) = look.color;
        let glyph = look.glyph.to_string().truecolor(red, green, blue);
        frame.push_str(&glyph.to_string());
    } else {
        frame.push(look.glyph);
    }
}

pub fn legend() -> &'static str {
    "M male   F female (brighter = smarter)   * food   # wall   X hazard"
}

pub fn stats_line(stats: &TickStats) -> String {
    let smartness = stats.smartness.as_ref().map_or_else(
        || "n/a".to_owned(),
        |SmartnessSummary { mean, min, max }| format!("{mean:.2} (min {min:.2}, max {max:.2})"),
    );
    let speed = stats
        .mean_speed
        .map_or_else(|| "n/a".to_owned(), |speed| format!("{speed:.2}"));
    format!(
        "tick {:>6} | population {:>4} ({}M/{}F) | generation {:>4} | smartness {smartness} | speed {speed}",
        stats.tick, stats.population, stats.males, stats.females, stats.max_generation
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        individual::{Gender, Individual},
        render::palette::BACKGROUND,
        stats::TickEvents,
    };
    use rand::{SeedableRng, rngs::StdRng};

    fn individual(gender: Gender, smartness: f32) -> Individual {
        let mut rng = StdRng::seed_from_u64(1);
        Individual {
            smartness,
            ..Individual::founder(1, gender, &Config::default(), &mut rng)
        }
    }

    /// Regression: food and females were both drawn as `F`.
    #[test]
    fn every_kind_of_cell_has_its_own_glyph() {
        let cells = [
            Cell::Empty,
            Cell::Food,
            Cell::Wall,
            Cell::Hazard,
            Cell::Individual(Box::new(individual(Gender::Male, 1.0))),
            Cell::Individual(Box::new(individual(Gender::Female, 1.0))),
        ];
        let mut glyphs: Vec<char> = cells.iter().map(|cell| appearance(cell).glyph).collect();
        glyphs.sort_unstable();
        glyphs.dedup();
        assert_eq!(glyphs.len(), cells.len());
    }

    #[test]
    fn plain_frame_layout_is_exact() {
        let mut world = World::new(2, 2);
        world.set(Position { x: 0, y: 0 }, Cell::Food);
        world.set(Position { x: 1, y: 0 }, Cell::Wall);
        world.set(Position { x: 1, y: 1 }, Cell::Hazard);
        assert_eq!(map_frame(&world, false), "+----+\n|* # |\n|  X |\n+----+\n");
    }

    #[test]
    fn single_cell_world_renders() {
        assert_eq!(map_frame(&World::new(1, 1), false), "+--+\n|  |\n+--+\n");
    }

    #[test]
    fn smarter_individuals_are_brighter_and_never_invisible() {
        let dull = appearance(&Cell::Individual(Box::new(individual(Gender::Male, 0.0))));
        let bright = appearance(&Cell::Individual(Box::new(individual(Gender::Male, 100.0))));
        assert!(bright.color.2 > dull.color.2);
        assert_ne!(dull.color, BACKGROUND);
    }

    #[test]
    fn extinct_stats_do_not_invent_a_score() {
        let stats = TickStats::measure(3, &World::new(2, 2), TickEvents::default());
        assert!(stats.smartness.is_none());
        assert!(stats_line(&stats).contains("smartness n/a"));
    }
}
