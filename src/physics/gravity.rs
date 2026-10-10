/// Falling sand physics: gravity simulation.
/// 
/// Algorithm:
/// 1. Iterate from bottom to top (so falling sand doesn't get processed twice)
/// 2. For each cell, if it's denser than what's below, try to move down
/// 3. If blocked straight down, try diagonal (left-down or right-down)
/// 4. Swap cells instead of copying for speed

use crate::grid::Grid;
use crate::materials::{get_properties, AIR};

pub fn apply_gravity(grid: &mut Grid) {
    // Iterate from bottom row upward
    // CRITICAL: Only iterate while y+1 is valid (y < height-1)
    // This prevents accessing out-of-bounds when checking the cell below
    for y in (0..grid.height.saturating_sub(1)).rev() {
        for x in 0..grid.width {
            let current = grid.get(x, y);
            let current_props = get_properties(current);

            // Skip air and non-active materials (they don't fall)
            if current == AIR || !current_props.active {
                continue;
            }

            let below = grid.get(x, y + 1);
            let below_props = get_properties(below);

            // If what's below is less dense, this cell should fall
            if current_props.density > below_props.density {
                // Simple case: swap with what's below
                grid.swap(x, y, x, y + 1);
            } else if current_props.is_fluid {
                // Fluids can also slide diagonally if blocked
                // Try left-down, then right-down
                let mut moved = false;

                if x > 0 {
                    let left_below = grid.get(x - 1, y + 1);
                    let left_below_props = get_properties(left_below);
                    if left_below_props.density < current_props.density {
                        grid.swap(x, y, x - 1, y + 1);
                        moved = true;
                    }
                }

                if !moved && x < grid.width - 1 {
                    let right_below = grid.get(x + 1, y + 1);
                    let right_below_props = get_properties(right_below);
                    if right_below_props.density < current_props.density {
                        grid.swap(x, y, x + 1, y + 1);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::SAND;

    #[test]
    fn test_sand_falls() {
        let mut grid = Grid::new(10, 10);
        // Place sand at (5, 3)
        grid.set(5, 3, SAND);
        
        // Before gravity
        assert_eq!(grid.get(5, 3), SAND);
        assert_eq!(grid.get(5, 4), AIR);

        // Apply gravity
        apply_gravity(&mut grid);

        // After gravity: sand should have moved down
        assert_eq!(grid.get(5, 3), AIR);
        assert_eq!(grid.get(5, 4), SAND);
    }

    #[test]
    fn test_sand_stacks() {
        let mut grid = Grid::new(10, 10);
        // Two sand cells
        grid.set(5, 2, SAND);
        grid.set(5, 3, SAND);

        apply_gravity(&mut grid);

        // Both should move down
        assert_eq!(grid.get(5, 2), AIR);
        assert_eq!(grid.get(5, 3), SAND);
        assert_eq!(grid.get(5, 4), SAND);
    }

    #[test]
    fn test_sand_stops_at_bottom() {
        let mut grid = Grid::new(10, 10);
        grid.set(5, 8, SAND);

        // Apply gravity multiple times
        for _ in 0..10 {
            apply_gravity(&mut grid);
        }

        // Sand should be at bottom
        assert_eq!(grid.get(5, 9), SAND);
    }
}
