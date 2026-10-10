/// Physics simulation module.
/// Handles gravity, liquids, reactions, explosions, etc.

pub mod gravity;
pub mod liquids;
pub mod reactions;

use crate::grid::Grid;

/// Main physics step: called once per frame.
/// This orchestrates the entire simulation.
pub fn step(grid: &mut Grid, _dt: f32) {
    gravity::apply_gravity(grid);
    liquids::spread_liquids(grid);
    // reactions::process_reactions(grid);
}
