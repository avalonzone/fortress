/// Physics simulation module.
/// Handles gravity, liquids, reactions, explosions, etc.

pub mod gravity;
pub mod liquids;
pub mod reactions;
pub mod temperature;

use crate::grid::Grid;

/// Main physics step: called once per frame.
/// This orchestrates the entire simulation.
pub fn step(grid: &mut Grid, _dt: f32) {
    gravity::apply_gravity(grid);
    gravity::apply_gases(grid);
    liquids::spread_liquids(grid);
    temperature::diffuse_heat(grid);
    reactions::process_reactions(grid);
}
