/// Reaction system: materials interact (fire + wood = more fire, then ash).

use crate::grid::Grid;
use crate::materials::{get_properties, ACID, AIR, ASH, FIRE, LAVA, STEAM, STONE, WATER};
use rand::Rng;

/// Chance per neighboring flammable cell, per step, to catch fire.
const IGNITION_CHANCE: f64 = 0.2;
/// Chance per step that a fire cell burning on fuel burns out.
const BURN_OUT_CHANCE: f64 = 0.01;
/// Chance per step that fire with no fuel nearby dies.
const FLICKER_OUT_CHANCE: f64 = 0.08;
/// Chance per step that acid dissolves a neighboring solid (and is used up).
const ACID_DISSOLVE_CHANCE: f64 = 0.05;

const NEIGHBORS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

pub fn process_reactions(grid: &mut Grid) {
    process_reactions_with_rng(grid, &mut rand::thread_rng());
}

fn process_reactions_with_rng<R: Rng + ?Sized>(grid: &mut Grid, rng: &mut R) {
    let mut changes = Vec::new();

    for y in 0..grid.height {
        for x in 0..grid.width {
            let current = grid.get(x, y);
            if current != FIRE && current != LAVA && current != ACID {
                continue;
            }

            let mut doused = false;
            let mut fuel_nearby = false;
            for (dx, dy) in NEIGHBORS {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 || nx >= grid.width as isize || ny >= grid.height as isize {
                    continue;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                let neighbor = grid.get(nx, ny);
                let flammable = get_properties(neighbor).flammable;

                match current {
                    FIRE | LAVA => {
                        if neighbor == WATER {
                            doused = true;
                            if current == LAVA {
                                changes.push((nx, ny, STEAM));
                            }
                        } else if flammable {
                            fuel_nearby = true;
                            if rng.gen_bool(IGNITION_CHANCE) {
                                changes.push((nx, ny, FIRE));
                            }
                        }
                    }
                    _ => {
                        let dissolvable = neighbor != AIR
                            && neighbor != ACID
                            && neighbor != WATER
                            && neighbor != STEAM
                            && neighbor != FIRE;
                        if dissolvable && rng.gen_bool(ACID_DISSOLVE_CHANCE) {
                            changes.push((nx, ny, AIR));
                            changes.push((x, y, AIR));
                            break;
                        }
                    }
                }
            }

            match current {
                FIRE => {
                    let burn_out = if fuel_nearby { BURN_OUT_CHANCE } else { FLICKER_OUT_CHANCE };
                    if doused {
                        changes.push((x, y, ASH));
                    } else if rng.gen_bool(burn_out) {
                        changes.push((x, y, if fuel_nearby { ASH } else { AIR }));
                    }
                }
                LAVA if doused => changes.push((x, y, STONE)),
                _ => {}
            }
        }
    }

    for (x, y, material) in changes {
        grid.set(x, y, material);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{AIR, STONE, WOOD};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn fire_spreads_through_wood_and_leaves_ash() {
        let mut grid = Grid::new(6, 3);
        grid.fill_rect(0, 1, 6, 1, WOOD);
        grid.set(0, 1, FIRE);
        let mut rng = StdRng::seed_from_u64(1);

        for _ in 0..2000 {
            process_reactions_with_rng(&mut grid, &mut rng);
        }

        assert_eq!(grid.count_material(FIRE), 0);
        assert!(grid.count_material(WOOD) < 6, "fire should consume wood");
        assert!(grid.count_material(ASH) >= 1, "burnt wood should leave ash");
    }

    #[test]
    fn lava_ignites_wood_and_turns_to_stone_on_water() {
        let mut grid = Grid::new(3, 1);
        grid.set(0, 0, WOOD);
        grid.set(1, 0, LAVA);
        grid.set(2, 0, WATER);
        let mut rng = StdRng::seed_from_u64(1);

        for _ in 0..50 {
            process_reactions_with_rng(&mut grid, &mut rng);
        }

        assert_eq!(grid.get(1, 0), STONE);
        assert_eq!(grid.get(2, 0), STEAM);
    }

    #[test]
    fn acid_dissolves_stone() {
        let mut grid = Grid::new(2, 1);
        grid.set(0, 0, ACID);
        grid.set(1, 0, STONE);
        let mut rng = StdRng::seed_from_u64(1);

        for _ in 0..500 {
            process_reactions_with_rng(&mut grid, &mut rng);
        }

        assert_eq!(grid.get(1, 0), AIR);
        assert_eq!(grid.get(0, 0), AIR);
    }

    #[test]
    fn fire_does_not_spread_to_non_flammable_materials() {
        let mut grid = Grid::new(3, 1);
        grid.set(0, 0, FIRE);
        grid.set(1, 0, STONE);
        let mut rng = StdRng::seed_from_u64(1);

        for _ in 0..500 {
            process_reactions_with_rng(&mut grid, &mut rng);
        }

        assert_eq!(grid.get(1, 0), STONE);
        assert_eq!(grid.get(2, 0), AIR);
    }

    #[test]
    fn water_extinguishes_adjacent_fire() {
        let mut grid = Grid::new(3, 1);
        grid.set(0, 0, FIRE);
        grid.set(1, 0, WATER);
        let mut rng = StdRng::seed_from_u64(1);

        process_reactions_with_rng(&mut grid, &mut rng);

        assert_eq!(grid.get(0, 0), ASH);
        assert_eq!(grid.get(1, 0), WATER);
    }
}
