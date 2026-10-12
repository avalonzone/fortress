/// Reaction system: temperature-driven phase changes plus fire and acid behavior.
///
/// Heat itself is moved around by `physics::temperature`; this module reacts to it:
/// wood ignites when hot, water boils, steam condenses and lava solidifies.

use crate::grid::Grid;
use crate::materials::{
    get_properties, initial_temperature, ACID, AIR, ASH, FIRE, LAVA, STEAM, STONE, WATER, WOOD,
};
use rand::Rng;

/// Wood catches fire at or above this temperature (°C).
const WOOD_IGNITION_TEMPERATURE: f32 = 300.0;
/// Water turns to steam at or above this temperature.
const BOILING_POINT: f32 = 100.0;
/// Steam turns back into water below this temperature.
const CONDENSATION_POINT: f32 = 90.0;
/// Lava hardens into stone below this temperature.
const SOLIDIFICATION_POINT: f32 = 650.0;
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
    let mut changes: Vec<(usize, usize, u8, Option<f32>)> = Vec::new();

    for y in 0..grid.height {
        for x in 0..grid.width {
            let current = grid.get(x, y);
            let temp = grid.temp(x, y);

            // Phase changes and ignition driven by temperature
            match current {
                WOOD if temp >= WOOD_IGNITION_TEMPERATURE => {
                    changes.push((x, y, FIRE, Some(initial_temperature(FIRE))));
                    continue;
                }
                WATER if temp >= BOILING_POINT => {
                    changes.push((x, y, STEAM, Some(initial_temperature(STEAM))));
                    continue;
                }
                STEAM if temp < CONDENSATION_POINT => {
                    changes.push((x, y, WATER, None));
                    continue;
                }
                LAVA if temp < SOLIDIFICATION_POINT => {
                    changes.push((x, y, STONE, None));
                    continue;
                }
                _ => {}
            }

            if current != FIRE && current != ACID {
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

                if current == FIRE {
                    if neighbor == WATER {
                        doused = true;
                    } else if get_properties(neighbor).flammable {
                        fuel_nearby = true;
                    }
                } else {
                    let dissolvable = neighbor != AIR
                        && neighbor != ACID
                        && neighbor != WATER
                        && neighbor != STEAM
                        && neighbor != FIRE;
                    if dissolvable && rng.gen_bool(ACID_DISSOLVE_CHANCE) {
                        changes.push((nx, ny, AIR, None));
                        changes.push((x, y, AIR, None));
                        break;
                    }
                }
            }

            if current == FIRE {
                let burn_out = if fuel_nearby { BURN_OUT_CHANCE } else { FLICKER_OUT_CHANCE };
                if doused {
                    changes.push((x, y, ASH, None));
                } else if rng.gen_bool(burn_out) {
                    changes.push((x, y, if fuel_nearby { ASH } else { AIR }, None));
                }
            }
        }
    }

    for (x, y, material, temp) in changes {
        grid.set_keep_temp(x, y, material);
        if let Some(temp) = temp {
            grid.set_temp(x, y, temp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::AMBIENT_TEMPERATURE;
    use crate::physics::temperature::diffuse_heat;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// One simulation tick of heat + reactions (what `physics::step` does, minus movement).
    fn tick(grid: &mut Grid, rng: &mut StdRng) {
        diffuse_heat(grid);
        process_reactions_with_rng(grid, rng);
    }

    #[test]
    fn fire_spreads_through_wood_and_leaves_ash() {
        let mut grid = Grid::new(6, 3);
        grid.fill_rect(0, 1, 6, 1, WOOD);
        grid.set(0, 1, FIRE);
        let mut rng = StdRng::seed_from_u64(1);

        for _ in 0..2000 {
            tick(&mut grid, &mut rng);
        }

        assert_eq!(grid.count_material(FIRE), 0);
        assert!(grid.count_material(WOOD) < 6, "fire should consume wood");
        assert!(grid.count_material(ASH) >= 1, "burnt wood should leave ash");
    }

    #[test]
    fn hot_wood_ignites() {
        let mut grid = Grid::new(1, 1);
        grid.set(0, 0, WOOD);
        let mut rng = StdRng::seed_from_u64(1);

        process_reactions_with_rng(&mut grid, &mut rng);
        assert_eq!(grid.get(0, 0), WOOD, "cold wood must not burn");

        grid.set_temp(0, 0, 350.0);
        process_reactions_with_rng(&mut grid, &mut rng);
        assert_eq!(grid.get(0, 0), FIRE);
    }

    #[test]
    fn lava_heats_adjacent_wood_until_it_burns() {
        let mut grid = Grid::new(2, 1);
        grid.set(0, 0, LAVA);
        grid.set(1, 0, WOOD);
        let mut rng = StdRng::seed_from_u64(1);

        let mut ignited = false;
        for _ in 0..100 {
            tick(&mut grid, &mut rng);
            if grid.get(1, 0) != WOOD {
                ignited = true;
                break;
            }
        }

        assert!(ignited, "wood next to lava should eventually catch fire");
    }

    #[test]
    fn lava_boils_adjacent_water() {
        let mut grid = Grid::new(2, 1);
        grid.set(0, 0, LAVA);
        grid.set(1, 0, WATER);
        let mut rng = StdRng::seed_from_u64(1);

        let mut boiled = false;
        for _ in 0..20 {
            tick(&mut grid, &mut rng);
            if grid.get(1, 0) == STEAM {
                boiled = true;
                break;
            }
        }

        assert!(boiled);
    }

    #[test]
    fn water_cools_lava_gradually_not_just_one_layer() {
        // A lava column (x = 0..4) next to a large pool of water
        let mut grid = Grid::new(24, 1);
        grid.fill_rect(0, 0, 4, 1, LAVA);
        grid.fill_rect(4, 0, 20, 1, WATER);
        let mut rng = StdRng::seed_from_u64(1);

        let mut first_stone_step = None;
        let mut all_stone_step = None;
        for step in 0..3000 {
            tick(&mut grid, &mut rng);
            let stones = grid.count_material(STONE);
            if stones > 0 && first_stone_step.is_none() {
                first_stone_step = Some(step);
            }
            if stones == 4 {
                all_stone_step = Some(step);
                break;
            }
        }

        let first = first_stone_step.expect("some lava should solidify");
        let all = all_stone_step.expect("a big water pool should eventually cool all the lava");
        assert!(all > first, "cooling should progress layer by layer");
        assert_eq!(grid.get(0, 0), STONE);
    }

    #[test]
    fn cold_steam_condenses_into_water() {
        let mut grid = Grid::new(1, 1);
        grid.set(0, 0, STEAM);
        grid.set_temp(0, 0, 50.0);
        let mut rng = StdRng::seed_from_u64(1);

        process_reactions_with_rng(&mut grid, &mut rng);

        assert_eq!(grid.get(0, 0), WATER);
    }

    #[test]
    fn cooled_lava_becomes_stone_and_keeps_its_heat() {
        let mut grid = Grid::new(1, 1);
        grid.set(0, 0, LAVA);
        grid.set_temp(0, 0, 600.0);
        let mut rng = StdRng::seed_from_u64(1);

        process_reactions_with_rng(&mut grid, &mut rng);

        assert_eq!(grid.get(0, 0), STONE);
        assert_eq!(grid.temp(0, 0), 600.0);
        assert!(grid.temp(0, 0) > AMBIENT_TEMPERATURE);
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
