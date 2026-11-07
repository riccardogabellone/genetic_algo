//! Draws the world. Cell colors come from the core's shared palette, so cells look as in the
//! terminal and GIF output; the background is the theme's `map_background`, a navy that
//! matches the app. Shapes distinguish each kind of cell, so identity never relies on
//! color alone. The map stays dark in both themes.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Sense, Shape, Stroke, Vec2};

use crate::theme;
use genetic_algo::{
    config::Config,
    individual::{Gender, Individual},
    leadership::Civics,
    render::palette::{Rgb, appearance},
    world::{Cell, Position, World},
};

const LEADER_RING: Color32 = Color32::from_rgb(255, 230, 120);
/// The map is dark in both themes, so the selection is always white.
const SELECTION: Color32 = Color32::WHITE;

fn color((red, green, blue): Rgb) -> Color32 {
    Color32::from_rgb(red, green, blue)
}

/// Where a map sits inside the space available to it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct MapLayout {
    origin: Pos2,
    cell_size: f32,
}

impl MapLayout {
    /// The largest whole-cell fit, centered. Cells are always square.
    fn fit(area: Rect, width: usize, height: usize) -> Self {
        let cell_size = (area.width() / width as f32)
            .min(area.height() / height as f32)
            .max(1.0);
        let map_size = Vec2::new(width as f32, height as f32) * cell_size;
        Self {
            origin: area.center() - map_size / 2.0,
            cell_size,
        }
    }

    fn cell_rect(&self, position: Position) -> Rect {
        let corner = self.origin + Vec2::new(position.x as f32, position.y as f32) * self.cell_size;
        Rect::from_min_size(corner, Vec2::splat(self.cell_size))
    }

    /// The cell under `pointer`, if it is on the map.
    fn cell_at(&self, pointer: Pos2, width: usize, height: usize) -> Option<Position> {
        let offset = (pointer - self.origin) / self.cell_size;
        if offset.x < 0.0 || offset.y < 0.0 {
            return None;
        }
        let position = Position {
            x: offset.x as usize,
            y: offset.y as usize,
        };
        (position.x < width && position.y < height).then_some(position)
    }
}

/// Draws the map and returns the cell that was clicked, if any. `selected` is the
/// individual the inspector follows, outlined wherever it moves.
pub fn show(
    ui: &mut egui::Ui,
    world: &World,
    leader: Option<u64>,
    selected: Option<u64>,
) -> Option<Position> {
    let (area, response) = ui.allocate_exact_size(ui.available_size(), Sense::click());
    let layout = MapLayout::fit(area, world.width(), world.height());
    let painter = ui.painter_at(area);

    let map_rect = Rect::from_min_size(
        layout.origin,
        Vec2::new(world.width() as f32, world.height() as f32) * layout.cell_size,
    );
    painter.rect_filled(map_rect, 0.0, theme::palette(ui.ctx()).map_background);
    for (position, cell) in world.iter() {
        let rect = layout.cell_rect(position);
        draw_cell(&painter, rect, cell);
        if let Cell::Individual(individual) = cell {
            if Some(individual.id) == leader {
                draw_leader_ring(&painter, rect);
            }
            if Some(individual.id) == selected {
                draw_selection(&painter, rect);
            }
        }
    }

    let pointer_cell = |pointer| layout.cell_at(pointer, world.width(), world.height());
    let clicked = response
        .clicked()
        .then(|| response.interact_pointer_pos().and_then(pointer_cell))
        .flatten();
    if let Some(position) = response.hover_pos().and_then(pointer_cell) {
        let cell = world.get(position);
        if *cell != Cell::Empty {
            response
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_ui_at_pointer(|ui| describe_cell(ui, position, cell));
        }
    }
    clicked
}

/// A square outline, distinct from the leader's ring, drawn over the selected cell.
fn draw_selection(painter: &Painter, rect: Rect) {
    let stroke = Stroke::new((rect.width() * 0.1).max(1.5), SELECTION);
    painter.rect_stroke(
        rect.expand(rect.width() * 0.12),
        rect.width() * 0.2,
        stroke,
        egui::StrokeKind::Outside,
    );
}

fn draw_cell(painter: &Painter, rect: Rect, cell: &Cell) {
    let tint = color(appearance(cell).color);
    let radius = rect.width() * 0.45;
    let center = rect.center();
    match cell {
        Cell::Empty => {}
        Cell::Food => {
            painter.circle_filled(center, rect.width() * 0.2, tint);
        }
        Cell::Wall => {
            painter.rect_filled(rect.shrink(rect.width() * 0.05), 0.0, tint);
        }
        Cell::Hazard => {
            let inner = rect.shrink(rect.width() * 0.2);
            let stroke = Stroke::new((rect.width() * 0.16).max(1.0), tint);
            painter.line_segment([inner.left_top(), inner.right_bottom()], stroke);
            painter.line_segment([inner.right_top(), inner.left_bottom()], stroke);
        }
        Cell::Individual(individual) => match individual.gender {
            Gender::Male => {
                painter.circle_filled(center, radius, tint);
            }
            Gender::Female => {
                let diamond = vec![
                    center + Vec2::new(0.0, -radius),
                    center + Vec2::new(radius, 0.0),
                    center + Vec2::new(0.0, radius),
                    center + Vec2::new(-radius, 0.0),
                ];
                painter.add(Shape::convex_polygon(diamond, tint, Stroke::NONE));
            }
        },
    }
}

/// A ring around the leader, drawn over its cell so it shows at any map size.
fn draw_leader_ring(painter: &Painter, rect: Rect) {
    let stroke = Stroke::new((rect.width() * 0.12).max(1.5), LEADER_RING);
    painter.circle_stroke(rect.center(), rect.width() * 0.62, stroke);
}

fn describe_cell(ui: &mut egui::Ui, position: Position, cell: &Cell) {
    let place = format!("({}, {})", position.x, position.y);
    match cell {
        Cell::Empty => {}
        Cell::Food => {
            ui.label(format!("Food at {place}"));
        }
        Cell::Wall => {
            ui.label(format!("Wall at {place}"));
        }
        Cell::Hazard => {
            ui.label(format!("Hazard at {place}"));
        }
        Cell::Individual(individual) => describe_individual(ui, &place, individual),
    }
}

pub fn describe_individual(ui: &mut egui::Ui, place: &str, individual: &Individual) {
    let gender = match individual.gender {
        Gender::Male => "Male",
        Gender::Female => "Female",
    };
    ui.strong(format!("{gender} #{} at {place}", individual.id));
    egui::Grid::new("individual_details").show(ui, |ui| {
        let mut row = |name: &str, value: String| {
            ui.label(name);
            ui.label(value);
            ui.end_row();
        };
        row("Generation", individual.generation.to_string());
        row("Smartness", format!("{:.1}", individual.smartness));
        row(
            "Innate smartness",
            format!("{:.1}", individual.innate_smartness),
        );
        row("Fertility", format!("{:.2}", individual.reproduction));
        row("Speed", individual.speed.to_string());
        row("Health", individual.health.to_string());
        let civics = &individual.civics;
        row("Aggression", format!("{:.2}", civics.policy.aggression));
        row("Appetite", format!("{:.2}", civics.policy.appetite));
        row("Times elected", civics.terms_served().to_string());
        row("Elections recalled", civics.memories().len().to_string());
        let brain = match individual.brain.as_deref() {
            Some(brain) if brain.learner().is_some() => "evolved, learning",
            Some(_) => "evolved",
            None => "rules",
        };
        row("Decides by", brain.to_owned());
        row(
            "Age",
            format!("{} / {}", individual.age, individual.lifespan),
        );
    });
}

/// One row explaining the symbols, drawn with the same shapes as the map.
pub fn legend(ui: &mut egui::Ui) {
    let sample = |ui: &mut egui::Ui, cell: &Cell, label: &str| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, 2.0, theme::palette(ui.ctx()).map_background);
        draw_cell(ui.painter(), rect, cell);
        ui.label(label);
        ui.add_space(8.0);
    };
    let sample_individual = |gender| {
        Cell::Individual(Box::new(Individual {
            id: 0,
            generation: 0,
            gender,
            smartness: 100.0,
            innate_smartness: 100.0,
            reproduction: 0.0,
            health: 1,
            age: 0,
            lifespan: 1,
            speed: 1,
            mating_cooldown: 0,
            civics: Civics::neutral(&Config::default()),
            brain: None,
        }))
    };
    ui.horizontal_wrapped(|ui| {
        sample(ui, &sample_individual(Gender::Male), "Male");
        sample(ui, &sample_individual(Gender::Female), "Female");
        ui.label("(brighter = smarter)");
        ui.add_space(8.0);
        sample(ui, &Cell::Food, "Food");
        sample(ui, &Cell::Wall, "Wall");
        sample(ui, &Cell::Hazard, "Hazard");
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, 2.0, theme::palette(ui.ctx()).map_background);
        draw_leader_ring(ui.painter(), rect.shrink(2.0));
        ui.label("Leader");
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn area(width: f32, height: f32) -> Rect {
        Rect::from_min_size(pos2(10.0, 20.0), Vec2::new(width, height))
    }

    #[test]
    fn cells_are_square_and_the_map_fits_and_is_centered() {
        let layout = MapLayout::fit(area(400.0, 200.0), 10, 10);
        assert_eq!(layout.cell_size, 20.0);
        assert_eq!(layout.origin, pos2(110.0, 20.0));
    }

    #[test]
    fn non_square_maps_fit_their_long_side() {
        let layout = MapLayout::fit(area(400.0, 400.0), 40, 10);
        assert_eq!(layout.cell_size, 10.0);
    }

    #[test]
    fn a_tiny_area_never_produces_a_zero_sized_cell() {
        assert_eq!(MapLayout::fit(area(0.0, 0.0), 128, 128).cell_size, 1.0);
    }

    #[test]
    fn pointer_lookup_finds_the_right_cell() {
        let layout = MapLayout::fit(area(100.0, 100.0), 10, 10);
        let inside = |x, y| layout.cell_at(pos2(x, y), 10, 10);
        assert_eq!(inside(10.0, 20.0), Some(Position { x: 0, y: 0 }));
        assert_eq!(inside(25.0, 39.0), Some(Position { x: 1, y: 1 }));
        assert_eq!(inside(109.9, 119.9), Some(Position { x: 9, y: 9 }));
    }

    #[test]
    fn pointer_lookup_rejects_everything_off_the_map() {
        let layout = MapLayout::fit(area(100.0, 100.0), 10, 10);
        for (x, y) in [(9.0, 50.0), (50.0, 19.0), (110.0, 50.0), (50.0, 120.0)] {
            assert_eq!(layout.cell_at(pos2(x, y), 10, 10), None, "{x}, {y}");
        }
    }

    #[test]
    fn lookup_and_drawing_agree() {
        let layout = MapLayout::fit(area(333.0, 217.0), 13, 7);
        for y in 0..7 {
            for x in 0..13 {
                let position = Position { x, y };
                let center = layout.cell_rect(position).center();
                assert_eq!(layout.cell_at(center, 13, 7), Some(position));
            }
        }
    }
}
