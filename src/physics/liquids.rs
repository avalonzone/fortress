/// Horizontal spreading for liquids that cannot fall further.

use crate::grid::Grid;
use crate::materials::{get_properties, ACID, AIR, LAVA, WATER};
use rand::Rng;

const MAX_HORIZONTAL_FLOW_SUBSTEPS: usize = 10;
const DROP_SEARCH_RANGE: usize = 8;

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
            let liquid = grid.get(x, y);
            if liquid != WATER && liquid != LAVA && liquid != ACID {
                continue;
            }

            let density = get_properties(liquid).density;
            let can_fall = y + 1 < grid.height
                && ((get_properties(grid.get(x, y + 1)).density < density)
                    || (x > 0 && get_properties(grid.get(x - 1, y + 1)).density < density)
                    || (x + 1 < grid.width
                        && get_properties(grid.get(x + 1, y + 1)).density < density));

            if can_fall {
                continue;
            }

            let directions = if rng.gen_bool(0.5) { [-1, 1] } else { [1, -1] };

            // Prefer the nearest ledge within reach so water drains toward lower ground
            let mut target = None;
            let mut best_distance = usize::MAX;
            for dx in directions {
                for distance in 1..=DROP_SEARCH_RANGE {
                    let next_x = x as isize + dx * distance as isize;
                    if next_x < 0 || next_x >= grid.width as isize {
                        break;
                    }
                    let next_x = next_x as usize;
                    if grid.get(next_x, y) != AIR {
                        break;
                    }
                    let has_drop = y + 1 < grid.height
                        && get_properties(grid.get(next_x, y + 1)).density < density;
                    if has_drop {
                        if distance < best_distance
                            && !reserved_destinations[next_x + y * grid.width]
                        {
                            best_distance = distance;
                            target = Some(next_x);
                        }
                        break;
                    }
                }
            }

            // Otherwise take a single random step to level out the surface
            if target.is_none() {
                for dx in directions {
                    let next_x = x as isize + dx;
                    if next_x < 0 || next_x >= grid.width as isize {
                        continue;
                    }
                    let next_x = next_x as usize;
                    if grid.get(next_x, y) == AIR
                        && !reserved_destinations[next_x + y * grid.width]
                    {
                        target = Some(next_x);
                        break;
                    }
                }
            }

            if let Some(next_x) = target {
                reserved_destinations[next_x + y * grid.width] = true;
                moves.push((x, y, next_x, y));
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
    fn water_flows_off_a_ledge_and_downhill() {
        let mut grid = Grid::new(12, 6);
        grid.fill_rect(0, 3, 4, 1, STONE);
        grid.fill_rect(0, 5, 12, 1, STONE);
        grid.set(1, 2, WATER);
        let mut rng = StdRng::seed_from_u64(3);

        for _ in 0..40 {
            crate::physics::gravity::apply_gravity(&mut grid);
            spread_liquids_with_rng(&mut grid, &mut rng, get_properties(WATER).viscosity);
        }

        assert_eq!(grid.count_material(WATER), 1);
        assert_eq!(grid.get(1, 2), AIR);
        assert_eq!(grid.get(1, 3), STONE);
        assert!((0..12).any(|x| grid.get(x, 4) == WATER));
    }

    #[test]
    fn water_levels_out_in_a_container() {
        let mut grid = Grid::new(10, 8);
        grid.fill_rect(0, 7, 10, 1, STONE);
        grid.fill_rect(0, 0, 1, 8, STONE);
        grid.fill_rect(9, 0, 1, 8, STONE);
        grid.fill_rect(1, 3, 2, 4, WATER);
        let total = grid.count_material(WATER);
        let mut rng = StdRng::seed_from_u64(11);

        for _ in 0..300 {
            crate::physics::gravity::apply_gravity(&mut grid);
            spread_liquids_with_rng(&mut grid, &mut rng, get_properties(WATER).viscosity);
        }

        assert_eq!(grid.count_material(WATER), total);
        // 8 water cells across 8 interior columns: a single flat layer
        for x in 1..9 {
            assert_eq!(grid.get(x, 6), WATER, "column {x} should be filled");
        }
    }

    #[test]
    fn more_viscous_liquids_get_fewer_flow_substeps() {
        assert!(horizontal_flow_substeps(100) < horizontal_flow_substeps(20));
        assert_eq!(horizontal_flow_substeps(20), 9);
    }
}
