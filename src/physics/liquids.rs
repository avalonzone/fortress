/// Horizontal spreading for liquids that cannot fall further.

use crate::grid::Grid;
use crate::materials::{get_properties, AIR, WATER};
use rand::Rng;

const MAX_HORIZONTAL_FLOW_SUBSTEPS: usize = 10;

fn horizontal_flow_substeps(viscosity: u8) -> usize {
    let viscosity_range = u8::MAX as usize;
    1 + (MAX_HORIZONTAL_FLOW_SUBSTEPS - 1)
        * (viscosity_range - viscosity as usize)
        / viscosity_range
}

pub fn spread_liquids(grid: &mut Grid) {
    let viscosity = get_properties(WATER).viscosity;
    spread_liquids_with_rng(grid, &mut rand::thread_rng(), viscosity);
}

fn spread_liquids_with_rng<R: Rng + ?Sized>(grid: &mut Grid, rng: &mut R, viscosity: u8) {
    for _ in 0..horizontal_flow_substeps(viscosity) {
        spread_liquids_once(grid, rng);
    }
}

fn spread_liquids_once<R: Rng + ?Sized>(grid: &mut Grid, rng: &mut R) {
    if grid.width == 0 || grid.height == 0 {
        return;
    }

    let mut reserved_destinations = vec![false; grid.width * grid.height];
    let mut moves = Vec::new();

    for y in 0..grid.height {
        for x in 0..grid.width {
            if grid.get(x, y) != WATER {
                continue;
            }

            let density = get_properties(WATER).density;
            let can_fall = y + 1 < grid.height
                && ((get_properties(grid.get(x, y + 1)).density < density)
                    || (x > 0 && get_properties(grid.get(x - 1, y + 1)).density < density)
                    || (x + 1 < grid.width
                        && get_properties(grid.get(x + 1, y + 1)).density < density));

            if can_fall {
                continue;
            }

            let directions = if rng.gen_bool(0.5) { [-1, 1] } else { [1, -1] };
            for dx in directions {
                let next_x = x as isize + dx;
                if next_x < 0 || next_x >= grid.width as isize {
                    continue;
                }

                let next_x = next_x as usize;
                let destination_index = next_x + y * grid.width;
                if grid.get(next_x, y) == AIR && !reserved_destinations[destination_index] {
                    reserved_destinations[destination_index] = true;
                    moves.push((x, y, next_x, y));
                    break;
                }
            }
        }
    }

    for (from_x, from_y, to_x, to_y) in moves {
        grid.swap(from_x, from_y, to_x, to_y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::STONE;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    #[test]
    fn blocked_water_spreads_sideways_without_losing_material() {
        let mut grid = Grid::new(5, 3);
        grid.set(2, 1, WATER);
        grid.fill_rect(0, 2, 5, 1, STONE);
        let mut rng = StdRng::seed_from_u64(7);

        spread_liquids_with_rng(&mut grid, &mut rng, get_properties(WATER).viscosity);

        assert_eq!(grid.count_material(WATER), 1);
        assert_eq!(grid.get(2, 1), AIR);
        assert!(grid.get(1, 1) == WATER || grid.get(3, 1) == WATER);
    }

    #[test]
    fn water_with_a_fall_path_does_not_spread_sideways() {
        let mut grid = Grid::new(5, 4);
        grid.set(2, 1, WATER);
        let mut rng = StdRng::seed_from_u64(7);

        spread_liquids_with_rng(&mut grid, &mut rng, get_properties(WATER).viscosity);

        assert_eq!(grid.get(2, 1), WATER);
        assert_eq!(grid.get(1, 1), AIR);
        assert_eq!(grid.get(3, 1), AIR);
    }

    #[test]
    fn more_viscous_liquids_get_fewer_flow_substeps() {
        assert!(horizontal_flow_substeps(100) < horizontal_flow_substeps(20));
        assert_eq!(horizontal_flow_substeps(20), 9);
    }
}
