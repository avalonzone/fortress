/// The pixel grid: the core data structure for the fortress.
/// 
/// This is a flat 2D grid stored as a 1D Vec<u8>.
/// Why flat? 
/// - Single allocation (no nested Vec fragmentation)
/// - Cache-friendly for physics loops (sequential memory access)
/// - Thread-safe indexing (math-based, no locks)
/// - Easy to serialize/save

use crate::materials::{initial_temperature, MaterialId, AIR, AMBIENT_TEMPERATURE, SAND, STONE, WATER};

/// A rectangular pixel grid.
pub struct Grid {
    /// Flat storage: cells[x + y * width]
    cells: Vec<MaterialId>,
    /// Temperature of each cell in °C, same layout as `cells`.
    temps: Vec<f32>,
    pub width: usize,
    pub height: usize,
}

impl Grid {
    /// Create a new grid filled with air.
    pub fn new(width: usize, height: usize) -> Self {
        let size = width * height;
        Grid {
            cells: vec![AIR; size],
            temps: vec![AMBIENT_TEMPERATURE; size],
            width,
            height,
        }
    }

    /// Convert 2D coordinates to 1D index.
    /// This is the core operation: every pixel access goes through here.
    fn index(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(x + y * self.width)
        } else {
            None
        }
    }

    /// Get material at position (panics if out of bounds).
    pub fn get(&self, x: usize, y: usize) -> MaterialId {
        match self.index(x, y) {
            Some(idx) => self.cells[idx],
            None => {
                eprintln!("Grid access out of bounds: ({}, {}) in {}x{}", x, y, self.width, self.height);
                AIR
            }
        }
    }

    /// Set material at position and reset its temperature to the material's
    /// starting temperature (e.g. lava starts at 1200 °C). Used for player placement.
    pub fn set(&mut self, x: usize, y: usize, material: MaterialId) {
        match self.index(x, y) {
            Some(idx) => {
                self.cells[idx] = material;
                self.temps[idx] = initial_temperature(material);
            }
            None => {
                eprintln!("Grid write out of bounds: ({}, {}) in {}x{}", x, y, self.width, self.height);
            }
        }
    }

    /// Change the material but keep the cell's current temperature
    /// (used by reactions such as lava cooling into stone).
    pub fn set_keep_temp(&mut self, x: usize, y: usize, material: MaterialId) {
        if let Some(idx) = self.index(x, y) {
            self.cells[idx] = material;
        }
    }

    /// Temperature of a cell in °C (ambient if out of bounds).
    pub fn temp(&self, x: usize, y: usize) -> f32 {
        self.index(x, y).map_or(AMBIENT_TEMPERATURE, |idx| self.temps[idx])
    }

    pub fn set_temp(&mut self, x: usize, y: usize, temp: f32) {
        if let Some(idx) = self.index(x, y) {
            self.temps[idx] = temp;
        }
    }

    /// Direct access to temperatures (for the heat diffusion loop).
    pub fn temps_mut(&mut self) -> &mut Vec<f32> {
        &mut self.temps
    }

    /// Swap two cells (material and temperature). Useful for falling sand:
    /// instead of copying, swap cell A with cell B.
    pub fn swap(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        if let (Some(idx1), Some(idx2)) = (self.index(x1, y1), self.index(x2, y2)) {
            self.cells.swap(idx1, idx2);
            self.temps.swap(idx1, idx2);
        }
    }

    /// Get a rectangular slice of cells as a flat vec.
    /// Used for rendering (extract visible region).
    pub fn get_slice(&self, x: usize, y: usize, width: usize, height: usize) -> Vec<MaterialId> {
        let mut result = Vec::with_capacity(width * height);
        for row in 0..height {
            for col in 0..width {
                result.push(self.get(x + col, y + row));
            }
        }
        result
    }

    /// Fill a rectangular region with a material.
    /// Used for testing and brush strokes.
    pub fn fill_rect(&mut self, x: usize, y: usize, width: usize, height: usize, material: MaterialId) {
        for row in 0..height {
            for col in 0..width {
                self.set(x + col, y + row, material);
            }
        }
    }

    /// Count how many cells of a given material exist.
    /// Useful for debugging and statistics.
    pub fn count_material(&self, material: MaterialId) -> usize {
        self.cells.iter().filter(|&&m| m == material).count()
    }

    /// Direct access to underlying cells (for physics loops).
    /// DANGER: Use with care! Physics needs to iterate fast.
    pub fn cells_mut(&mut self) -> &mut Vec<MaterialId> {
        &mut self.cells
    }

    pub fn cells(&self) -> &Vec<MaterialId> {
        &self.cells
    }

    /// Fill the starting terrain, from the bottom up: a bedrock (stone) layer,
    /// a sand layer, then a water layer, all spanning the full grid width.
    pub fn fill_base_layers(&mut self, bedrock: usize, sand: usize, water: usize) {
        let mut bottom = self.height;
        for (thickness, material) in [(bedrock, STONE), (sand, SAND), (water, WATER)] {
            let thickness = thickness.min(bottom);
            bottom -= thickness;
            self.fill_rect(0, bottom, self.width, thickness, material);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::SAND;

    #[test]
    fn base_layers_stack_stone_sand_water_from_the_bottom() {
        let mut grid = Grid::new(4, 10);
        grid.fill_base_layers(2, 3, 2);

        for x in 0..4 {
            assert_eq!(grid.get(x, 9), STONE);
            assert_eq!(grid.get(x, 8), STONE);
            assert_eq!(grid.get(x, 7), SAND);
            assert_eq!(grid.get(x, 5), SAND);
            assert_eq!(grid.get(x, 4), WATER);
            assert_eq!(grid.get(x, 3), WATER);
            assert_eq!(grid.get(x, 2), AIR);
        }
    }

    #[test]
    fn base_layers_are_clamped_to_the_grid_height() {
        let mut grid = Grid::new(2, 4);
        grid.fill_base_layers(3, 3, 3);

        assert_eq!(grid.count_material(STONE), 6);
        assert_eq!(grid.count_material(SAND), 2);
        assert_eq!(grid.count_material(WATER), 0);
    }
    #[test]
    fn test_grid_creation() {
        let grid = Grid::new(100, 100);
        assert_eq!(grid.width, 100);
        assert_eq!(grid.height, 100);
        assert_eq!(grid.count_material(AIR), 100 * 100);
    }

    #[test]
    fn test_grid_get_set() {
        let mut grid = Grid::new(10, 10);
        grid.set(5, 5, SAND);
        assert_eq!(grid.get(5, 5), SAND);
        assert_eq!(grid.get(4, 5), AIR);
    }

    #[test]
    fn test_grid_swap() {
        let mut grid = Grid::new(10, 10);
        grid.set(2, 2, SAND);
        grid.set(3, 3, 0); // Stay air for clarity
        grid.swap(2, 2, 3, 3);
        assert_eq!(grid.get(2, 2), AIR);
        assert_eq!(grid.get(3, 3), SAND);
    }

    #[test]
    fn test_fill_rect() {
        let mut grid = Grid::new(10, 10);
        grid.fill_rect(0, 0, 5, 5, SAND);
        assert_eq!(grid.count_material(SAND), 25);
        assert_eq!(grid.get(4, 4), SAND);
        assert_eq!(grid.get(5, 5), AIR);
    }

    #[test]
    fn test_out_of_bounds() {
        let grid = Grid::new(10, 10);
        // Should return AIR without panicking
        assert_eq!(grid.get(100, 100), AIR);
    }
}
