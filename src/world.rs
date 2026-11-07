//! The grid. It is the single source of truth for where everything is.

use rand::{RngExt, seq::IndexedRandom};

use crate::individual::Individual;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Empty,
    Food,
    Wall,
    Hazard,
    Individual(Box<Individual>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct World {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl World {
    /// Dimensions must have been checked by `Config::validate`.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::Empty; width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    fn index(&self, position: Position) -> usize {
        assert!(
            position.x < self.width && position.y < self.height,
            "{position:?} is outside the {}x{} grid",
            self.width,
            self.height
        );
        position.y * self.width + position.x
    }

    fn position(&self, index: usize) -> Position {
        Position {
            x: index % self.width,
            y: index / self.width,
        }
    }

    pub fn get(&self, position: Position) -> &Cell {
        &self.cells[self.index(position)]
    }

    pub fn set(&mut self, position: Position, cell: Cell) {
        let index = self.index(position);
        self.cells[index] = cell;
    }

    /// Removes and returns the individual at `position`, if there is one.
    pub fn take_individual(&mut self, position: Position) -> Option<Individual> {
        let index = self.index(position);
        match std::mem::replace(&mut self.cells[index], Cell::Empty) {
            Cell::Individual(individual) => Some(*individual),
            other => {
                self.cells[index] = other;
                None
            }
        }
    }

    /// All individuals in row order, for changes that touch everyone at once.
    pub fn individuals_mut(&mut self) -> impl Iterator<Item = (Position, &mut Individual)> {
        let width = self.width;
        self.cells
            .iter_mut()
            .enumerate()
            .filter_map(move |(index, cell)| match cell {
                Cell::Individual(individual) => Some((
                    Position {
                        x: index % width,
                        y: index / width,
                    },
                    individual.as_mut(),
                )),
                _ => None,
            })
    }

    pub fn individual_mut(&mut self, position: Position) -> Option<&mut Individual> {
        let index = self.index(position);
        match &mut self.cells[index] {
            Cell::Individual(individual) => Some(individual),
            _ => None,
        }
    }

    pub fn place_individual(&mut self, position: Position, individual: Individual) {
        self.set(position, Cell::Individual(Box::new(individual)));
    }

    /// All cells in row order.
    pub fn iter(&self) -> impl Iterator<Item = (Position, &Cell)> {
        self.cells
            .iter()
            .enumerate()
            .map(|(index, cell)| (self.position(index), cell))
    }

    pub fn individuals(&self) -> impl Iterator<Item = (Position, &Individual)> {
        self.iter().filter_map(|(position, cell)| match cell {
            Cell::Individual(individual) => Some((position, individual.as_ref())),
            _ => None,
        })
    }

    pub fn count(&self, wanted: &Cell) -> usize {
        self.cells.iter().filter(|cell| *cell == wanted).count()
    }

    /// In-bounds cells at exactly `radius` king-moves from `center`: at most `8 * radius`,
    /// in a fixed order. The map has hard borders. A zero radius returns nothing.
    pub fn ring(&self, center: Position, radius: usize) -> Vec<Position> {
        let mut positions = Vec::new();
        if radius == 0 {
            return positions;
        }
        let in_range = |value: usize, limit: usize| {
            let low = value.saturating_sub(radius);
            let high = value.saturating_add(radius).min(limit - 1);
            low..=high
        };
        for y in in_range(center.y, self.height) {
            for x in in_range(center.x, self.width) {
                if x.abs_diff(center.x).max(y.abs_diff(center.y)) == radius {
                    positions.push(Position { x, y });
                }
            }
        }
        positions
    }

    /// How many of the eight cells around `center` hold `wanted`. Allocation-free, because
    /// brains call it for every option they score.
    pub fn count_around(&self, center: Position, wanted: &Cell) -> usize {
        let span =
            |value: usize, limit: usize| value.saturating_sub(1)..=(value + 1).min(limit - 1);
        let mut count = 0;
        for y in span(center.y, self.height) {
            for x in span(center.x, self.width) {
                if (x, y) != (center.x, center.y) && self.get(Position { x, y }) == wanted {
                    count += 1;
                }
            }
        }
        count
    }

    /// Puts `cell` on up to `amount` random empty cells and returns how many it placed.
    /// Never retries blindly, so a full grid cannot hang it.
    pub fn scatter(&mut self, cell: &Cell, amount: usize, rng: &mut impl RngExt) -> usize {
        let targets = self.sample_empty(amount, rng);
        for position in &targets {
            self.set(*position, cell.clone());
        }
        targets.len()
    }

    /// Up to `amount` distinct random empty cells.
    pub fn sample_empty(&self, amount: usize, rng: &mut impl RngExt) -> Vec<Position> {
        let empty: Vec<Position> = self
            .iter()
            .filter(|(_, cell)| **cell == Cell::Empty)
            .map(|(position, _)| position)
            .collect();
        empty.sample(rng, amount).copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn count_around_matches_the_ring() {
        let mut world = World::new(4, 3);
        let mut rng = StdRng::seed_from_u64(9);
        world.scatter(&Cell::Food, 6, &mut rng);
        for (position, _) in world.iter() {
            let by_ring = world
                .ring(position, 1)
                .iter()
                .filter(|cell| *world.get(**cell) == Cell::Food)
                .count();
            assert_eq!(
                world.count_around(position, &Cell::Food),
                by_ring,
                "{position:?}"
            );
        }
    }

    #[test]
    fn ring_has_eight_r_cells_in_open_space() {
        let world = World::new(21, 21);
        for radius in 1..=5 {
            let ring = world.ring(Position { x: 10, y: 10 }, radius);
            assert_eq!(ring.len(), 8 * radius);
        }
    }

    #[test]
    fn ring_is_clipped_at_borders_and_corners() {
        let world = World::new(5, 5);
        assert_eq!(world.ring(Position { x: 0, y: 0 }, 1).len(), 3);
        assert_eq!(world.ring(Position { x: 2, y: 0 }, 1).len(), 5);
        assert_eq!(world.ring(Position { x: 4, y: 4 }, 1).len(), 3);
    }

    #[test]
    fn ring_handles_degenerate_sizes() {
        assert_eq!(
            World::new(1, 1).ring(Position { x: 0, y: 0 }, 1),
            Vec::new()
        );
        assert_eq!(World::new(1, 3).ring(Position { x: 0, y: 1 }, 1).len(), 2);
        assert_eq!(
            World::new(3, 3).ring(Position { x: 1, y: 1 }, 0),
            Vec::new()
        );
        assert_eq!(
            World::new(3, 3).ring(Position { x: 1, y: 1 }, 50),
            Vec::new()
        );
        assert_eq!(
            World::new(3, 3).ring(Position { x: 1, y: 1 }, usize::MAX / 16),
            Vec::new()
        );
    }

    #[test]
    fn ring_never_contains_the_center_or_duplicates() {
        let world = World::new(9, 9);
        let center = Position { x: 1, y: 1 };
        let mut ring = world.ring(center, 2);
        assert!(!ring.contains(&center));
        let before = ring.len();
        ring.sort_by_key(|p| (p.x, p.y));
        ring.dedup();
        assert_eq!(ring.len(), before);
    }

    /// Regression: placement used an unbounded retry loop that hung on a full grid.
    #[test]
    fn scatter_stops_when_the_grid_is_full() {
        let mut world = World::new(3, 3);
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(world.scatter(&Cell::Wall, 100, &mut rng), 9);
        assert_eq!(world.scatter(&Cell::Food, 1, &mut rng), 0);
        assert_eq!(world.count(&Cell::Wall), 9);
    }

    #[test]
    fn scatter_only_uses_empty_cells() {
        let mut world = World::new(4, 4);
        let mut rng = StdRng::seed_from_u64(2);
        world.scatter(&Cell::Wall, 8, &mut rng);
        assert_eq!(world.scatter(&Cell::Food, 8, &mut rng), 8);
        assert_eq!(world.count(&Cell::Wall), 8);
        assert_eq!(world.count(&Cell::Food), 8);
    }

    #[test]
    fn take_individual_leaves_other_cells_alone() {
        let mut world = World::new(2, 1);
        let wall = Position { x: 1, y: 0 };
        world.set(wall, Cell::Wall);
        assert!(world.take_individual(wall).is_none());
        assert_eq!(*world.get(wall), Cell::Wall);
    }

    #[test]
    #[should_panic(expected = "outside")]
    fn out_of_bounds_access_panics_instead_of_wrapping() {
        // x == width would silently alias the first cell of the next row.
        let _ = World::new(3, 3).get(Position { x: 3, y: 0 });
    }

    #[test]
    fn positions_round_trip_through_iteration() {
        let world = World::new(3, 2);
        for (position, _) in world.iter() {
            assert_eq!(world.index(position), position.y * 3 + position.x);
        }
        assert_eq!(world.iter().count(), 6);
    }
}
