/// Heat diffusion: every cell exchanges heat with its 4 neighbors.
///
/// Flow between two cells is proportional to their temperature difference and
/// the lower of their conductivities; the resulting temperature change is
/// divided by each cell's heat capacity. Fire is a constant heat source, and
/// air relaxes toward room temperature so heat does not build up forever.

use crate::grid::Grid;
use crate::materials::{heat_capacity, thermal_conductivity, AIR, AMBIENT_TEMPERATURE, FIRE};

/// Minimum temperature of a burning fire cell.
pub const FIRE_TEMPERATURE: f32 = 700.0;
/// Fraction of the gap to ambient that air closes each step.
const AIR_RELAXATION: f32 = 0.03;

pub fn diffuse_heat(grid: &mut Grid) {
    let (width, height) = (grid.width, grid.height);
    if width == 0 || height == 0 {
        return;
    }

    let materials: Vec<_> = (0..width * height)
        .map(|i| grid.get(i % width, i / width))
        .collect();
    let old = grid.temps_mut().clone();
    let temps = grid.temps_mut();

    for y in 0..height {
        for x in 0..width {
            let i = x + y * width;
            // Only look right and down so each pair is exchanged once
            for j in [(x + 1 < width).then(|| i + 1), (y + 1 < height).then(|| i + width)]
                .into_iter()
                .flatten()
            {
                let diff = old[i] - old[j];
                if diff.abs() < 0.01 {
                    continue;
                }
                let k = thermal_conductivity(materials[i]).min(thermal_conductivity(materials[j]));
                let flow = k * diff;
                temps[i] -= flow / heat_capacity(materials[i]);
                temps[j] += flow / heat_capacity(materials[j]);
            }
        }
    }

    for i in 0..width * height {
        if materials[i] == AIR {
            temps[i] += (AMBIENT_TEMPERATURE - temps[i]) * AIR_RELAXATION;
        } else if materials[i] == FIRE && temps[i] < FIRE_TEMPERATURE {
            temps[i] = FIRE_TEMPERATURE;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{LAVA, STONE, WATER};

    #[test]
    fn heat_flows_from_hot_to_cold() {
        let mut grid = Grid::new(2, 1);
        grid.set(0, 0, LAVA);
        grid.set(1, 0, STONE);

        diffuse_heat(&mut grid);

        assert!(grid.temp(0, 0) < 1200.0);
        assert!(grid.temp(1, 0) > AMBIENT_TEMPERATURE);
    }

    #[test]
    fn water_absorbs_more_heat_than_it_gains_in_temperature() {
        let mut grid = Grid::new(2, 1);
        grid.set(0, 0, LAVA);
        grid.set(1, 0, WATER);

        diffuse_heat(&mut grid);

        let lava_drop = 1200.0 - grid.temp(0, 0);
        let water_rise = grid.temp(1, 0) - AMBIENT_TEMPERATURE;
        assert!(lava_drop > 0.0 && water_rise > 0.0);
        // Water has twice the capacity of lava, so it warms up half as much
        assert!((lava_drop / water_rise - 2.0).abs() < 0.01);
    }

    #[test]
    fn air_cools_back_to_ambient() {
        let mut grid = Grid::new(1, 1);
        grid.set_temp(0, 0, 500.0);

        for _ in 0..300 {
            diffuse_heat(&mut grid);
        }

        assert!((grid.temp(0, 0) - AMBIENT_TEMPERATURE).abs() < 1.0);
    }

    #[test]
    fn swapping_cells_carries_temperature() {
        let mut grid = Grid::new(2, 1);
        grid.set(0, 0, LAVA);
        grid.swap(0, 0, 1, 0);

        assert_eq!(grid.get(1, 0), LAVA);
        assert_eq!(grid.temp(1, 0), 1200.0);
        assert_eq!(grid.temp(0, 0), AMBIENT_TEMPERATURE);
    }
}
