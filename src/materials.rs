/// Material types for the grid.
/// Each material has unique properties: density, viscosity, color, reactions.
/// We use an enum to define all possible materials, then store IDs (u8) in the grid
/// for memory efficiency.

use serde::{Deserialize, Serialize};

/// Material ID: ranges from 0-255 (fits in u8).
/// 0 is always AIR (empty space).
pub type MaterialId = u8;

pub const AIR: MaterialId = 0;
pub const SAND: MaterialId = 1;
pub const WATER: MaterialId = 2;
pub const WOOD: MaterialId = 3;
pub const STONE: MaterialId = 4;
pub const LAVA: MaterialId = 5;
pub const ACID: MaterialId = 6;
pub const FIRE: MaterialId = 7;
pub const STEAM: MaterialId = 8;
pub const ASH: MaterialId = 9;

/// Material properties: how each material behaves.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MaterialProperties {
    /// Density determines falling order. Higher density = falls first.
    /// Air = 0, Sand = 100, Water = 50, Stone = 200, Lava = 60
    pub density: u8,

    /// Viscosity: how "thick" a fluid is (0-255).
    /// 0 = water (flows fast), 255 = honey (flows slow).
    /// Only matters for fluids.
    pub viscosity: u8,

    /// Is this a solid (blocks movement) or fluid (flows)?
    pub is_fluid: bool,

    /// RGBA color for rendering (8 bits per channel).
    pub color: [u8; 4],

    /// Can this catch fire? (Used for reactions)
    pub flammable: bool,

    /// Does this material move/simulate each frame?
    pub active: bool,
}

/// Define all materials and their properties.
/// This is a lookup table: `get_properties(SAND)` returns the sand properties.
pub fn get_properties(material_id: MaterialId) -> MaterialProperties {
    match material_id {
        AIR => MaterialProperties {
            density: 0,
            viscosity: 0,
            is_fluid: true, // Air is a "fluid" (moves freely)
            color: [0, 0, 0, 0], // Transparent
            flammable: false,
            active: false, // Air doesn't simulate
        },
        SAND => MaterialProperties {
            density: 100,
            viscosity: 0,
            is_fluid: false,
            color: [194, 178, 128, 255], // Tan/beige
            flammable: false,
            active: true,
        },
        WATER => MaterialProperties {
            density: 50,
            viscosity: 20,
            is_fluid: true,
            color: [64, 164, 223, 200], // Light blue
            flammable: false,
            active: true,
        },
        WOOD => MaterialProperties {
            density: 90,
            viscosity: 0,
            is_fluid: false,
            color: [139, 90, 43, 255], // Brown
            flammable: true,
            active: false,
        },
        STONE => MaterialProperties {
            density: 200,
            viscosity: 0,
            is_fluid: false,
            color: [128, 128, 128, 255], // Gray
            flammable: false,
            active: false,
        },
        LAVA => MaterialProperties {
            density: 60,
            viscosity: 220, // thick: creeps and falls slowly
            is_fluid: true,
            color: [255, 100, 0, 255], // Orange-red
            flammable: false,
            active: true,
        },
        ACID => MaterialProperties {
            density: 55,
            viscosity: 30,
            is_fluid: true,
            color: [0, 255, 0, 200], // Bright green
            flammable: false,
            active: true,
        },
        FIRE => MaterialProperties {
            density: 10,
            viscosity: 5,
            is_fluid: true,
            color: [255, 200, 0, 255], // Yellow-white
            flammable: false, // Can't catch itself
            active: true,
        },
        STEAM => MaterialProperties {
            density: 5,
            viscosity: 5,
            is_fluid: true,
            color: [200, 200, 200, 150], // Light gray
            flammable: false,
            active: true,
        },
        ASH => MaterialProperties {
            density: 40,
            viscosity: 10,
            is_fluid: false,
            color: [100, 100, 100, 255], // Dark gray
            flammable: false,
            active: false,
        },
        _ => {
            // Unknown material ID—treat as air for safety
            eprintln!("Unknown material ID: {}", material_id);
            get_properties(AIR)
        }
    }
}

/// Room temperature in °C; air relaxes toward it.
pub const AMBIENT_TEMPERATURE: f32 = 20.0;

/// Temperature a freshly placed cell of this material starts at.
pub fn initial_temperature(material_id: MaterialId) -> f32 {
    match material_id {
        LAVA => 1200.0,
        FIRE => 800.0,
        STEAM => 110.0,
        _ => AMBIENT_TEMPERATURE,
    }
}

/// How easily heat flows through a material (0..~0.2 per step).
/// Heat flow between two cells uses the lower of the two values, so air
/// insulates while water and lava exchange heat quickly.
pub fn thermal_conductivity(material_id: MaterialId) -> f32 {
    match material_id {
        AIR => 0.0015,
        SAND => 0.05,
        WATER => 0.2,
        WOOD => 0.08,
        STONE => 0.1,
        LAVA => 0.15,
        ACID => 0.1,
        FIRE => 0.15,
        STEAM => 0.03,
        ASH => 0.03,
        _ => 0.005,
    }
}

/// How much energy it takes to change temperature (higher = slower to heat or cool).
/// Water's high value is what lets it soak heat out of lava.
pub fn heat_capacity(material_id: MaterialId) -> f32 {
    match material_id {
        WATER => 4.0,
        LAVA => 5.0, // lava holds its heat for a long time
        ACID => 2.0,
        WOOD | STONE => 1.5,
        _ => 1.0,
    }
}

/// Lava is runny when hot and thickens as it cools toward solidification.
const LAVA_HOT_VISCOSITY: f32 = 30.0;
const LAVA_HOT_TEMPERATURE: f32 = 1200.0;
const LAVA_COLD_TEMPERATURE: f32 = 650.0;

/// Viscosity of a cell, taking temperature into account (only lava changes).
pub fn effective_viscosity(material_id: MaterialId, temp: f32) -> u8 {
    let base = get_properties(material_id).viscosity;
    if material_id != LAVA {
        return base;
    }
    let hotness =
        ((temp - LAVA_COLD_TEMPERATURE) / (LAVA_HOT_TEMPERATURE - LAVA_COLD_TEMPERATURE)).clamp(0.0, 1.0);
    (base as f32 - hotness * (base as f32 - LAVA_HOT_VISCOSITY)) as u8
}

/// Helper: is a material denser than another?
/// Used for settling order (heavier materials fall through lighter ones).
pub fn is_denser_than(a: MaterialId, b: MaterialId) -> bool {
    get_properties(a).density > get_properties(b).density
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_air_is_lowest_density() {
        let air = get_properties(AIR);
        let sand = get_properties(SAND);
        assert!(sand.density > air.density);
    }

    #[test]
    fn test_stone_is_denser_than_sand() {
        assert!(is_denser_than(STONE, SAND));
        assert!(!is_denser_than(SAND, STONE));
    }
}
