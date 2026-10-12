/// Rendering module: converts the grid into RGBA pixels, with a heat glow.
///
/// Hot cells (lava, fire, glowing stone...) shift toward an incandescent color
/// ramp (dark red -> orange -> yellow -> white-hot) and bleed a soft bloom into
/// neighboring cells, including empty air.

use crate::grid::Grid;
use crate::materials::{get_properties, AIR, FIRE};
use bevy::prelude::*;

/// Below this temperature (°C) nothing glows.
const GLOW_START: f32 = 450.0;
/// Temperature range over which the glow goes from dim red to white-hot.
const GLOW_RANGE: f32 = 750.0;
/// Bloom blur radius in cells.
const BLOOM_RADIUS: usize = 3;
/// How strongly the blurred glow is added back.
const BLOOM_STRENGTH: f32 = 1.4;
/// Maximum opacity of bloom drawn over empty air.
const MAX_AIR_BLOOM_ALPHA: f32 = 0.7;

pub struct GridRenderPlugin;

impl Plugin for GridRenderPlugin {
    fn build(&self, _app: &mut App) {
        println!("GridRenderPlugin loaded");
    }
}

/// 0.0 (not glowing) to 1.0 (white-hot).
fn glow_amount(temp: f32) -> f32 {
    ((temp - GLOW_START) / GLOW_RANGE).clamp(0.0, 1.0)
}

/// Approximate black-body color for a glow amount in 0..=1.
fn incandescent_color(glow: f32) -> [f32; 3] {
    const STOPS: [(f32, [f32; 3]); 4] = [
        (0.0, [150.0, 20.0, 0.0]),
        (0.35, [255.0, 90.0, 0.0]),
        (0.7, [255.0, 170.0, 30.0]),
        (1.0, [255.0, 235.0, 170.0]),
    ];
    for pair in STOPS.windows(2) {
        let (g0, c0) = pair[0];
        let (g1, c1) = pair[1];
        if glow <= g1 {
            let t = (glow - g0) / (g1 - g0);
            return [
                c0[0] + (c1[0] - c0[0]) * t,
                c0[1] + (c1[1] - c0[1]) * t,
                c0[2] + (c1[2] - c0[2]) * t,
            ];
        }
    }
    STOPS[STOPS.len() - 1].1
}

/// Cheap deterministic hash in 0..1 used for fire flicker.
fn flicker(x: usize, y: usize, frame: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ frame.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xFFFF) as f32 / 65535.0
}

/// Separable box blur (horizontal then vertical) with edge clamping.
fn box_blur(data: &mut [[f32; 3]], width: usize, height: usize, radius: usize) {
    let window = (2 * radius + 1) as f32;
    let r = radius as isize;
    let mut tmp = vec![[0.0f32; 3]; data.len()];

    for y in 0..height {
        for c in 0..3 {
            let sample = |x: isize| data[y * width + x.clamp(0, width as isize - 1) as usize][c];
            let mut sum: f32 = (-r..=r).map(sample).sum();
            for x in 0..width {
                tmp[y * width + x][c] = sum / window;
                sum += sample(x as isize + r + 1) - sample(x as isize - r);
            }
        }
    }

    for x in 0..width {
        for c in 0..3 {
            let sample = |y: isize| tmp[y.clamp(0, height as isize - 1) as usize * width + x][c];
            let mut sum: f32 = (-r..=r).map(sample).sum();
            for y in 0..height {
                data[y * width + x][c] = sum / window;
                sum += sample(y as isize + r + 1) - sample(y as isize - r);
            }
        }
    }
}

/// Render the grid to a flat RGBA8 buffer (row-major, 4 bytes per cell).
/// `frame` drives the fire flicker.
pub fn render_pixels(grid: &Grid, frame: u32) -> Vec<u8> {
    let (width, height) = (grid.width, grid.height);
    let mut pixels = vec![0u8; width * height * 4];
    let mut emission = vec![[0.0f32; 3]; width * height];
    let mut any_glow = false;

    for y in 0..height {
        for x in 0..width {
            let i = x + y * width;
            let material = grid.get(x, y);
            let [r, g, b, a] = get_properties(material).color;
            let mut color = [r as f32, g as f32, b as f32];
            let mut alpha = a as f32;

            let mut glow = glow_amount(grid.temp(x, y));
            if material == FIRE {
                glow = (glow * (0.8 + 0.4 * flicker(x, y, frame))).clamp(0.0, 1.0);
            }
            if glow > 0.0 {
                any_glow = true;
                let hot = incandescent_color(glow);
                let mix = (glow * 2.0).min(1.0);
                for c in 0..3 {
                    color[c] += (hot[c] - color[c]) * mix;
                    emission[i][c] = hot[c] * glow;
                }
                alpha = alpha.max(255.0 * mix);
            }

            pixels[i * 4] = color[0] as u8;
            pixels[i * 4 + 1] = color[1] as u8;
            pixels[i * 4 + 2] = color[2] as u8;
            pixels[i * 4 + 3] = alpha as u8;
        }
    }

    if !any_glow {
        return pixels;
    }

    box_blur(&mut emission, width, height, BLOOM_RADIUS);

    for y in 0..height {
        for x in 0..width {
            let i = x + y * width;
            let bloom = emission[i];
            let strongest = bloom[0].max(bloom[1]).max(bloom[2]);
            if strongest < 1.0 {
                continue;
            }

            if grid.get(x, y) == AIR && pixels[i * 4 + 3] == 0 {
                // Empty air: draw a translucent halo tinted like the heat source
                let alpha = (strongest / 255.0 * BLOOM_STRENGTH).min(MAX_AIR_BLOOM_ALPHA);
                for c in 0..3 {
                    pixels[i * 4 + c] = (bloom[c] / strongest * 255.0) as u8;
                }
                pixels[i * 4 + 3] = (alpha * 255.0) as u8;
            } else {
                for c in 0..3 {
                    let value = pixels[i * 4 + c] as f32 + bloom[c] * BLOOM_STRENGTH * 0.5;
                    pixels[i * 4 + c] = value.min(255.0) as u8;
                }
            }
        }
    }

    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{LAVA, SAND, STONE, WATER};

    fn pixel(pixels: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
        let i = (x + y * width) * 4;
        [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
    }

    #[test]
    fn cold_materials_use_their_base_color() {
        let mut grid = Grid::new(4, 1);
        grid.set(0, 0, SAND);
        grid.set(1, 0, WATER);
        let pixels = render_pixels(&grid, 0);

        assert_eq!(pixel(&pixels, 4, 0, 0), get_properties(SAND).color);
        assert_eq!(pixel(&pixels, 4, 1, 0), get_properties(WATER).color);
        assert_eq!(pixel(&pixels, 4, 3, 0)[3], 0, "empty air stays transparent");
    }

    #[test]
    fn hot_lava_glows_hotter_than_cool_lava() {
        let mut grid = Grid::new(2, 1);
        grid.set(0, 0, LAVA);
        grid.set(1, 0, LAVA);
        grid.set_temp(1, 0, 700.0);
        let pixels = render_pixels(&grid, 0);

        let hot = pixel(&pixels, 2, 0, 0);
        let cool = pixel(&pixels, 2, 1, 0);
        assert!(hot[1] as u32 + hot[2] as u32 > cool[1] as u32 + cool[2] as u32);
    }

    #[test]
    fn hot_stone_glows_red() {
        let mut grid = Grid::new(1, 1);
        grid.set(0, 0, STONE);
        grid.set_temp(0, 0, 600.0);
        let pixels = render_pixels(&grid, 0);
        let p = pixel(&pixels, 1, 0, 0);

        assert!(p[0] > get_properties(STONE).color[0], "glowing stone should turn red");
        assert!(p[0] > p[2]);
    }

    #[test]
    fn glow_bleeds_into_nearby_air_but_not_far_away() {
        let mut grid = Grid::new(20, 1);
        grid.set(0, 0, LAVA);
        let pixels = render_pixels(&grid, 0);

        assert!(pixel(&pixels, 20, 2, 0)[3] > 0, "air near lava gets a halo");
        assert_eq!(pixel(&pixels, 20, 15, 0)[3], 0, "distant air is untouched");
    }

    #[test]
    fn glow_ramp_goes_from_red_to_near_white() {
        let dim = incandescent_color(0.0);
        let bright = incandescent_color(1.0);
        assert!(dim[0] > dim[1] && dim[1] > dim[2]);
        assert!(bright[2] > 150.0);
    }
}
