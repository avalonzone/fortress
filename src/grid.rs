/// Chunked world storage for the fortress.
///
/// World coordinates stay compatible with the original grid API while the
/// underlying cells are stored in independently addressable chunks.
use crate::materials::{
    initial_temperature, MaterialId, AIR, AMBIENT_TEMPERATURE, SAND, STONE, WATER,
};

pub const CHUNK_SIZE: usize = 128;

/// A fixed-size region of material and temperature data.
#[derive(Clone)]
pub struct Chunk {
    cells: Vec<MaterialId>,
    temps: Vec<f32>,
}

impl Chunk {
    fn new() -> Self {
        let size = CHUNK_SIZE * CHUNK_SIZE;
        Self {
            cells: vec![AIR; size],
            temps: vec![AMBIENT_TEMPERATURE; size],
        }
    }

    fn index(x: usize, y: usize) -> usize {
        x + y * CHUNK_SIZE
    }

    pub fn get(&self, x: usize, y: usize) -> MaterialId {
        self.cells[Self::index(x, y)]
    }

    pub fn temp(&self, x: usize, y: usize) -> f32 {
        self.temps[Self::index(x, y)]
    }
}

/// A finite logical world backed by a map of fixed-size chunks.
pub struct World {
    chunks: Vec<Option<Chunk>>,
    chunks_wide: usize,
    pub width: usize,
    pub height: usize,
}

/// Compatibility name retained for existing physics, rendering, and input APIs.
pub type Grid = World;

impl World {
    /// Create an air-filled world with all chunks allocated.
    pub fn new(width: usize, height: usize) -> Self {
        let chunks_wide = width.div_ceil(CHUNK_SIZE);
        let chunks_high = height.div_ceil(CHUNK_SIZE);
        let chunks = (0..chunks_wide * chunks_high)
            .map(|_| Some(Chunk::new()))
            .collect();
        Self {
            chunks,
            chunks_wide,
            width,
            height,
        }
    }

    fn chunk_and_local(&self, x: usize, y: usize) -> Option<(usize, usize)> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let chunk_index = (y / CHUNK_SIZE) * self.chunks_wide + x / CHUNK_SIZE;
        let local_index = Chunk::index(x % CHUNK_SIZE, y % CHUNK_SIZE);
        Some((chunk_index, local_index))
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.iter().filter(|chunk| chunk.is_some()).count()
    }

    /// Read a material by world coordinate; out-of-world reads are air.
    pub fn get(&self, x: usize, y: usize) -> MaterialId {
        let Some((chunk_index, local_index)) = self.chunk_and_local(x, y) else {
            eprintln!(
                "World access out of bounds: ({}, {}) in {}x{}",
                x, y, self.width, self.height
            );
            return AIR;
        };
        self.chunks[chunk_index]
            .as_ref()
            .map_or(AIR, |chunk| chunk.cells[local_index])
    }

    /// Set material by world coordinate and reset temperature to its initial value.
    pub fn set(&mut self, x: usize, y: usize, material: MaterialId) {
        if let Some((chunk_index, local_index)) = self.chunk_and_local(x, y) {
            let chunk = self.chunks[chunk_index].get_or_insert_with(Chunk::new);
            chunk.cells[local_index] = material;
            chunk.temps[local_index] = initial_temperature(material);
        } else {
            eprintln!(
                "World write out of bounds: ({}, {}) in {}x{}",
                x, y, self.width, self.height
            );
        }
    }

    /// Change material without changing the cell's temperature.
    pub fn set_keep_temp(&mut self, x: usize, y: usize, material: MaterialId) {
        if let Some((chunk_index, local_index)) = self.chunk_and_local(x, y) {
            let chunk = self.chunks[chunk_index].get_or_insert_with(Chunk::new);
            chunk.cells[local_index] = material;
        }
    }

    /// Read a cell temperature; out-of-world reads use ambient temperature.
    pub fn temp(&self, x: usize, y: usize) -> f32 {
        let Some((chunk_index, local_index)) = self.chunk_and_local(x, y) else {
            return AMBIENT_TEMPERATURE;
        };
        self.chunks[chunk_index]
            .as_ref()
            .map_or(AMBIENT_TEMPERATURE, |chunk| chunk.temps[local_index])
    }

    pub fn set_temp(&mut self, x: usize, y: usize, temp: f32) {
        if let Some((chunk_index, local_index)) = self.chunk_and_local(x, y) {
            let chunk = self.chunks[chunk_index].get_or_insert_with(Chunk::new);
            chunk.temps[local_index] = temp;
        }
    }

    /// Swap materials and temperatures between two world coordinates.
    pub fn swap(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let Some((chunk1_index, local_index1)) = self.chunk_and_local(x1, y1) else {
            return;
        };
        let Some((chunk2_index, local_index2)) = self.chunk_and_local(x2, y2) else {
            return;
        };
        if chunk1_index == chunk2_index {
            if let Some(chunk) = self.chunks[chunk1_index].as_mut() {
                chunk.cells.swap(local_index1, local_index2);
                chunk.temps.swap(local_index1, local_index2);
            }
        } else {
            let (first_index, second_index, first_local, second_local) =
                if chunk1_index < chunk2_index {
                    (chunk1_index, chunk2_index, local_index1, local_index2)
                } else {
                    (chunk2_index, chunk1_index, local_index2, local_index1)
                };
            let (left, right) = self.chunks.split_at_mut(second_index);
            let first = left[first_index].get_or_insert_with(Chunk::new);
            let second = right[0].get_or_insert_with(Chunk::new);
            std::mem::swap(
                &mut first.cells[first_local],
                &mut second.cells[second_local],
            );
            std::mem::swap(
                &mut first.temps[first_local],
                &mut second.temps[second_local],
            );
        }
    }

    pub fn get_slice(&self, x: usize, y: usize, width: usize, height: usize) -> Vec<MaterialId> {
        let mut result = Vec::with_capacity(width * height);
        for row in 0..height {
            for col in 0..width {
                result.push(self.get(x + col, y + row));
            }
        }
        result
    }

    pub fn fill_rect(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        material: MaterialId,
    ) {
        for row in 0..height {
            for col in 0..width {
                self.set(x + col, y + row, material);
            }
        }
    }

    pub fn count_material(&self, material: MaterialId) -> usize {
        (0..self.height)
            .flat_map(|y| (0..self.width).map(move |x| (x, y)))
            .filter(|&(x, y)| self.get(x, y) == material)
            .count()
    }

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
    fn world_coordinates_cross_chunk_boundaries() {
        let mut world = World::new(CHUNK_SIZE + 2, 2);
        world.set(CHUNK_SIZE - 1, 0, SAND);
        world.set(CHUNK_SIZE, 0, WATER);
        world.set_temp(CHUNK_SIZE, 0, 900.0);

        assert_eq!(world.chunk_count(), 2);
        assert_eq!(world.get(CHUNK_SIZE - 1, 0), SAND);
        assert_eq!(world.get(CHUNK_SIZE, 0), WATER);
        assert_eq!(world.temp(CHUNK_SIZE, 0), 900.0);

        world.swap(CHUNK_SIZE - 1, 0, CHUNK_SIZE, 0);
        assert_eq!(world.get(CHUNK_SIZE - 1, 0), WATER);
        assert_eq!(world.get(CHUNK_SIZE, 0), SAND);
        assert_eq!(world.temp(CHUNK_SIZE - 1, 0), 900.0);
    }

    #[test]
    fn partial_chunks_do_not_count_padding_cells() {
        let grid = World::new(3, 2);
        assert_eq!(grid.count_material(AIR), 6);
        assert_eq!(grid.chunk_count(), 1);
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
        assert_eq!(grid.get(100, 100), AIR);
    }
}
