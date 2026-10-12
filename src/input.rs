/// Input handling: mouse dig/build mechanics.
/// 
/// This module provides interactive controls for the fortress game:
/// - Left-click: Dig (remove materials)
/// - Right-click: Build (place selected material)
/// - Number keys 1-9: Select materials
/// - Space: Toggle brush size between 3x3 and 5x5

use crate::grid::Grid;
use crate::materials::{MaterialId, SAND, WATER, WOOD, STONE, LAVA, ACID, FIRE, STEAM, ASH, AIR};

/// Tracks player input state: selected material, brush, cursor position.
#[derive(Debug, Clone, Copy)]
pub struct InputState {
    pub selected_material: MaterialId,
    pub brush_size: usize,  // 1 = single cell, 3 = 3x3, 5 = 5x5
    pub cursor_x: i32,
    pub cursor_y: i32,
}

impl Default for InputState {
    fn default() -> Self {
        InputState {
            selected_material: SAND,
            brush_size: 3,
            cursor_x: 0,
            cursor_y: 0,
        }
    }
}

impl InputState {
    /// Returns the offset radius for the brush.
    /// E.g., brush_size=3 gives radius=1, meaning we paint from -1 to +1.
    pub fn radius(&self) -> i32 {
        (self.brush_size as i32 - 1) / 2
    }
}

/// Apply brush action to grid at cursor position.
/// 
/// If `material` is Some(MaterialId), we build (place that material).
/// If `material` is None, we dig (remove material = place AIR).
pub fn apply_brush(
    grid: &mut Grid,
    cursor_x: i32,
    cursor_y: i32,
    material: Option<MaterialId>,
    brush_size: usize,
) {
    let radius = (brush_size as i32 - 1) / 2;
    let target_material = material.unwrap_or(AIR);
    
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let x = cursor_x + dx;
            let y = cursor_y + dy;
            
            // Grid::set already bounds-checks, so this is safe
            if x >= 0 && y >= 0 {
                grid.set(x as usize, y as usize, target_material);
            }
        }
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
