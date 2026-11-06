//! Records the map as an animated GIF. Frames are streamed to disk as they are drawn.

use std::path::Path;

use plotters::prelude::*;

use crate::{
    individual::Gender,
    render::palette::{self, Rgb, appearance},
    world::{Cell, Position, World},
};

const CELL_PIXELS: u32 = 12;
const FRAME_DELAY_MS: u32 = 80;
const MAX_FRAME_PIXELS: u64 = 4096 * 4096;

type Canvas<'a> = DrawingArea<BitMapBackend<'a>, plotters::coord::Shift>;
type DrawResult<T> = Result<T, Box<dyn std::error::Error>>;

pub struct GifRecorder<'a> {
    canvas: Canvas<'a>,
    frames: usize,
}

impl<'a> GifRecorder<'a> {
    pub fn create(path: &'a Path, width: usize, height: usize) -> DrawResult<Self> {
        let dimensions = frame_size(width, height).ok_or("the grid is too large for a GIF")?;
        let backend = BitMapBackend::gif(path, dimensions, FRAME_DELAY_MS)?;
        Ok(Self {
            canvas: backend.into_drawing_area(),
            frames: 0,
        })
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn record(&mut self, world: &World) -> DrawResult<()> {
        self.canvas.fill(&color(palette::BACKGROUND))?;
        for (position, cell) in world.iter() {
            draw_cell(&self.canvas, position, cell)?;
        }
        self.canvas.present()?;
        self.frames += 1;
        Ok(())
    }
}

/// Each frame is held in memory uncompressed, so its total area is bounded too.
fn frame_size(width: usize, height: usize) -> Option<(u32, u32)> {
    let side = |cells: usize| {
        u32::try_from(cells)
            .ok()?
            .checked_mul(CELL_PIXELS)
            .filter(|pixels| u16::try_from(*pixels).is_ok())
    };
    let (frame_width, frame_height) = (side(width)?, side(height)?);
    (u64::from(frame_width) * u64::from(frame_height) <= MAX_FRAME_PIXELS)
        .then_some((frame_width, frame_height))
}

fn color((red, green, blue): Rgb) -> RGBColor {
    RGBColor(red, green, blue)
}

/// Each kind has its own shape, so identity never relies on color alone.
fn draw_cell(canvas: &Canvas<'_>, position: Position, cell: &Cell) -> DrawResult<()> {
    let size = CELL_PIXELS as i32;
    let (left, top) = (position.x as i32 * size, position.y as i32 * size);
    let (right, bottom) = (left + size - 1, top + size - 1);
    let center = (left + size / 2, top + size / 2);
    let radius = size / 2 - 1;
    let style = color(appearance(cell).color);

    match cell {
        Cell::Empty => {}
        Cell::Food => {
            canvas.draw(&Circle::new(center, radius / 2, style.filled()))?;
        }
        Cell::Wall => {
            canvas.draw(&Rectangle::new(
                [(left, top), (right, bottom)],
                style.filled(),
            ))?;
        }
        Cell::Hazard => {
            let stroke = style.stroke_width(2);
            canvas.draw(&PathElement::new(
                [(left + 2, top + 2), (right - 2, bottom - 2)],
                stroke,
            ))?;
            canvas.draw(&PathElement::new(
                [(right - 2, top + 2), (left + 2, bottom - 2)],
                stroke,
            ))?;
        }
        Cell::Individual(individual) => match individual.gender {
            Gender::Male => {
                canvas.draw(&Circle::new(center, radius, style.filled()))?;
            }
            Gender::Female => {
                let diamond = [
                    (center.0, top),
                    (right, center.1),
                    (center.0, bottom),
                    (left, center.1),
                ];
                canvas.draw(&Polygon::new(diamond, style.filled()))?;
            }
        },
    }
    Ok(())
}
