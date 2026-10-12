/// Input handling: mouse dig/build mechanics.
/// 
/// This module provides interactive controls for the fortress game:
/// - Left-click: Dig (remove materials)
/// - Right-click: Build (place selected material)
/// - Number keys 1-9: Select materials
/// - Numpad +/-: Change brush size (4x4, 8x8, 16x16, 32x32)

use crate::grid::Grid;
use crate::materials::{MaterialId, SAND, WATER, WOOD, STONE, LAVA, ACID, FIRE, STEAM, ASH, AIR};

/// Selectable brush sizes (cells per side), smallest to largest.
pub const BRUSH_SIZES: [usize; 4] = [4, 8, 16, 32];

/// Tracks player input state: selected material, brush, cursor position.
#[derive(Debug, Clone, Copy)]
pub struct InputState {
    pub selected_material: MaterialId,
    pub brush_size: usize,  // one of BRUSH_SIZES
    pub cursor_x: i32,
    pub cursor_y: i32,
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            selected_material: SAND,
            brush_size: BRUSH_SIZES[1],
            cursor_x: 0,
            cursor_y: 0,
        }
    }
}

impl InputState {
    fn brush_index(&self) -> usize {
        BRUSH_SIZES.iter().position(|&s| s == self.brush_size).unwrap_or(0)
    }

    /// Switch to the next larger brush size (stays at the maximum).
    pub fn increase_brush(&mut self) {
        let next = (self.brush_index() + 1).min(BRUSH_SIZES.len() - 1);
        self.brush_size = BRUSH_SIZES[next];
    }

    /// Switch to the next smaller brush size (stays at the minimum).
    pub fn decrease_brush(&mut self) {
        let prev = self.brush_index().saturating_sub(1);
        self.brush_size = BRUSH_SIZES[prev];
    }
}

/// Apply brush action to grid at cursor position.
/// 
/// The brush covers exactly `brush_size` x `brush_size` cells; (cursor_x, cursor_y)
/// is the cell just right of and below the square's center.
/// If `material` is Some(MaterialId), we build (place that material).
/// If `material` is None, we dig (remove material = place AIR).
pub fn apply_brush(
    grid: &mut Grid,
    cursor_x: i32,
    cursor_y: i32,
    material: Option<MaterialId>,
    brush_size: usize,
) {
    let size = brush_size as i32;
    let start = -(size / 2);
    let target_material = material.unwrap_or(AIR);
    
    for dy in start..start + size {
        for dx in start..start + size {
            let x = cursor_x + dx;
            let y = cursor_y + dy;
            
            // Grid::set already bounds-checks, so this is safe
            if x >= 0 && y >= 0 {
                grid.set(x as usize, y as usize, target_material);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brush_covers_exactly_size_by_size_cells() {
        for size in BRUSH_SIZES {
            let mut grid = Grid::new(100, 100);
            apply_brush(&mut grid, 50, 50, Some(SAND), size);
            assert_eq!(grid.count_material(SAND), size * size, "size {size}");
        }
    }

    #[test]
    fn brush_is_centered_on_the_cursor_cell_corner() {
        let mut grid = Grid::new(20, 20);
        apply_brush(&mut grid, 10, 10, Some(SAND), 4);
        assert_eq!(grid.get(8, 8), SAND);
        assert_eq!(grid.get(11, 11), SAND);
        assert_eq!(grid.get(7, 8), AIR);
        assert_eq!(grid.get(12, 11), AIR);
    }

    #[test]
    fn brush_size_steps_and_clamps() {
        let mut input = InputState::default();
        assert_eq!(input.brush_size, 8);
        input.increase_brush();
        assert_eq!(input.brush_size, 16);
        input.increase_brush();
        input.increase_brush();
        assert_eq!(input.brush_size, 32);
        input.decrease_brush();
        assert_eq!(input.brush_size, 16);
        for _ in 0..10 {
            input.decrease_brush();
        }
        assert_eq!(input.brush_size, 4);
    }
}

/// Select material from keyboard input (1-9 maps to material IDs 1-9).
/// Returns Some(MaterialId) if a valid key was pressed, None otherwise.
pub fn material_from_key(key: u32) -> Option<MaterialId> {
    match key {
        49 => Some(SAND),      // '1' key
        50 => Some(WATER),     // '2' key
        51 => Some(WOOD),      // '3' key
        52 => Some(STONE),     // '4' key
        53 => Some(LAVA),      // '5' key
        54 => Some(ACID),      // '6' key
        55 => Some(FIRE),      // '7' key
        56 => Some(STEAM),     // '8' key
        57 => Some(ASH),       // '9' key
        _ => None,
    }
}
