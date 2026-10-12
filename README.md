# 🏰 Fortress: A Destructible Pixel Game in Rust + Bevy

A pixel-based fortress management game with a fully destructible environment. You can dig, build, watch materials fall and interact with physics simulations inspired by **The Powder Toy**.

**Status:** Phase 2 mostly complete (Foundation, Rendering, Input, Liquids & Reactions)

---

## 📋 Project Overview

### Goal
Create a game where:
- You manage a pixel-based fortress
- The entire environment is destructible (dig/build freely)
- Different materials have unique properties (sand, water, lava, wood, acid, etc.)
- Physics simulation handles gravity, liquids, reactions, explosions
- Eventually: chunked world streaming for large maps (2048×2048+)

### Current Scope (MVP)
1. **Phase 1:** Single-chunk proof-of-concept (256×512 or 512×512 grid)
   - Falling sand physics ✅
   - Basic rendering & input ✅
2. **Phase 2:** Materials & reactions
3. **Phase 3:** Chunking & world streaming

### Technology Stack
- **Language:** Rust (2021 edition)
- **Game Engine:** Bevy 0.20
- **Physics Reference:** The Powder Toy (open-source falling-sand engine)
- **Serialization:** serde + serde_json
- **Utilities:** parking_lot (fast mutexes), rand

---

## 🏗️ Project Structure

```
fortress/
├── Cargo.toml                 # Dependencies & project config
├── src/
│   ├── lib.rs                # Library root (exports all modules)
│   ├── main.rs               # App entry point
│   ├── materials.rs          # Material types & properties
│   ├── grid.rs               # Core pixel grid (flat Vec storage)
│   ├── rendering.rs          # Grid texture rendering
│   ├── input.rs              # Mouse input and brush handling
│   └── physics/
│       ├── mod.rs            # Physics orchestrator (main step function)
│       ├── gravity.rs        # Falling sand + rising gases (fire, steam)
│       ├── liquids.rs        # Liquid spreading (water, lava, acid)
│       ├── temperature.rs    # Heat diffusion between cells
│       └── reactions.rs      # Temperature-driven reactions (fire, lava, acid)
└── README.md                 # This file

target/                        # Build artifacts (generated)
```

---

## 🧩 Module Documentation

### `materials.rs` — Material System
**Purpose:** Define all material types and their physical properties.

**Key Concepts:**
- `MaterialId = u8` — Each material is a single byte (0-255). Memory-efficient!
- `MaterialProperties` — Struct containing density, viscosity, color, flammability, etc.
- `get_properties(id)` — Lookup function: returns properties for any material ID

**Materials Defined:**
| ID | Name | Density | Is Fluid | Color | Flammable | Notes |
|----|------|---------|----------|-------|-----------|-------|
| 0 | AIR | 0 | Yes | Transparent | No | Empty space |
| 1 | SAND | 100 | No | Tan | No | Falls & stacks |
| 2 | WATER | 50 | Yes | Blue | No | Flows, spreads |
| 3 | WOOD | 90 | No | Brown | **Yes** | Burns to ash |
| 4 | STONE | 200 | No | Gray | No | Heavy, doesn't fall |
| 5 | LAVA | 60 | Yes | Orange | No | Hot, melts things |
| 6 | ACID | 55 | Yes | Green | No | Eats through materials |
| 7 | FIRE | 10 | Yes | Yellow | No | Rising gas, spreads reactions |
| 8 | STEAM | 5 | Yes | Gray | No | Water heated |
| 9 | ASH | 40 | No | Dark Gray | No | Burned wood |

**Why density matters:** Higher density = falls first. Sand (100) falls through water (50).

---

### `grid.rs` — The Pixel Grid
**Purpose:** Core data structure. Stores all pixel/material state.

**Key Design Decisions:**
1. **Flat Vec, not nested Vec**
   - Storage: `cells: Vec<MaterialId>` (one allocation, cache-friendly)
   - Indexing: `index(x, y) = x + y * width`
   - Why? Fast sequential access for physics loops
   
2. **MaterialId storage (u8), not full Material structs**
   - Each cell = 1 byte (not 20+ bytes)
   - 256×256 grid = 65KB memory (not 1.3MB)
   - Serialization is trivial

3. **Public API:**
   - `new(width, height)` — Create empty grid
   - `get(x, y) → MaterialId` — Read cell
   - `set(x, y, material)` — Write cell
   - `swap(x1, y1, x2, y2)` — Swap cells (fast!)
   - `fill_rect()`, `get_slice()`, `count_material()` — Utilities

**Example Usage:**
```rust
let mut grid = Grid::new(100, 100);  // 100×100 of air
grid.set(50, 50, SAND);               // Place sand at (50, 50)
grid.swap(50, 50, 50, 51);            // Swap with below
```

---

### `physics/gravity.rs` — Falling Sand
**Purpose:** Simulate gravity each frame.

**Algorithm:**
1. Iterate from **bottom-to-top** (Y = height-1 down to 1)
   - Why? Prevents a cell from falling twice in one frame
2. For each cell with density > 0:
   - Is what's below less dense? **Swap down**
   - For fluids: if blocked, try diagonal (left-down or right-down)
3. One pass per frame = stable, fast settling

**Why iterate bottom-to-top?**
```
Frame N (top-to-bottom ❌):
  Iter Y=1: Sand at (5,1) falls to (5,2) ✓
  Iter Y=2: Sand at (5,2) falls to (5,3) ✗ Wrong! Processed twice!

Frame N (bottom-to-top ✅):
  Iter Y=99: Sand at (5,99) can't fall (at bottom)
  ...
  Iter Y=2: Sand at (5,2) falls to (5,3)
  Iter Y=1: Sand at (5,1) falls to (5,2)
  → Each cell falls exactly once per frame ✓
```

**Tests:**
- Sand falls one cell per frame ✓
- Multiple sand cells stack ✓
- Sand stops at grid bottom ✓

**Gases (`apply_gases`):** fire and steam are skipped by `apply_gravity` and instead
rise (up, or up-diagonal / sideways into air, iterating top-to-bottom). Fire stays
in place when touching a flammable cell, otherwise it flickers upward randomly.

---

### `physics/liquids.rs` — Liquid Spreading
Water, lava and acid that cannot fall move horizontally. Each cell first looks up
to 8 cells sideways (through air only) for the nearest ledge with air below and
heads there; otherwise it takes one random step to level the surface. Viscosity
sets the number of flow substeps per step (water: 9). Destinations are reserved so
two cells never move into the same spot.

---

### `physics/temperature.rs` — Heat
Every cell stores a temperature (°C, ambient 20). Each step, heat flows between
neighboring cells in proportion to their temperature difference and the *lower* of
their conductivities (`thermal_conductivity` in `materials.rs`, so air insulates while
water and lava exchange heat quickly). The temperature change is divided by the cell's
`heat_capacity`: water (4) heats up slowly and therefore soaks a lot of heat out of
lava (2). Fire is a constant heat source (≥ 700 °C while it burns), lava starts at
1200 °C and cools down, and air relaxes toward ambient. Temperature travels with the
cell when it moves (`Grid::swap`).

---

### `physics/reactions.rs` — Reactions
Run each step after heat diffusion; they react to temperature:
- **Wood** ignites at ≥ 300 °C (from fire, lava or any hot neighbor) and becomes fire.
- **Water** boils into steam at ≥ 100 °C; **steam** condenses back to water below 90 °C.
- **Lava** solidifies into stone below 650 °C, so a pool cools layer by layer as water
  carries its heat away, instead of only at the contact surface.
- **Fire:** next to fuel it burns out into ash (1 %/step); without fuel it dies quickly
  (8 %/step). Touching water turns it into ash.
- **Acid:** dissolves an adjacent solid (5 %/step), consuming itself; water, steam,
  fire and acid are not dissolved.

Randomized tests use seeded RNGs so they are deterministic.

---

### `physics/mod.rs` — Physics Orchestrator
**Purpose:** Calls all physics subsystems each frame.

**Current structure:**
```rust
pub fn step(grid: &mut Grid, dt: f32) {
    gravity::apply_gravity(grid);        // 1. Make things fall
    gravity::apply_gases(grid);          // 2. Fire and steam rise
    liquids::spread_liquids(grid);       // 3. Spread fluids
    temperature::diffuse_heat(grid);     // 4. Move heat around
    reactions::process_reactions(grid);  // 5. Phase changes, fire, acid
}
```

---

### `rendering.rs` — Grid Rendering
The Bevy app encodes the grid into an RGBA texture (`render_pixels`) and displays it in the game window.

**Heat glow:** cells above 450 °C shift along a black-body ramp (dark red → orange →
yellow → white-hot, fully white-hot at 1200 °C), so fresh lava is bright and cooling
lava or stone fades to red. The glow is blurred (box blur, radius 3) and added back as
a soft bloom, which also draws a translucent halo over nearby empty air. Fire flickers
using a cheap per-cell, per-frame hash.
The camera is zoomed in with an orthographic scale (`CAMERA_ZOOM_SCALE` in `main.rs`,
default `0.5` = 2× zoom; `0.25` = 4×, `1.0` = no zoom). The grid stays 512×512; only the
view is magnified, so grid edges may be cropped in small windows.

---

### `input.rs` — Input & Coordinate Mapping
Handles mouse clicks for digging/building, material selection, and brush size.

The cursor position is converted from window viewport coordinates to camera world
coordinates with Bevy's `viewport_to_world_2d`, then mapped into the centered
512×512 grid. The cursor sprite and dig/build input use this same mapping so the
visible cursor and the affected cell stay aligned, including when the window is
resized or uses a non-square aspect ratio.

Controls: left-click digs, right-click builds, Space toggles the brush between 3×3
and 5×5, and number keys select the material:

| Key | Material | Key | Material |
|---|---|---|---|
| 1 | Sand | 6 | Acid |
| 2 | Water | 7 | Fire |
| 3 | Wood | 8 | Steam |
| 4 | Stone | 9 | Ash |
| 5 | Lava | | |

---

## 🚀 Building & Running

### Prerequisites
- **Rust 1.70+** (install from https://rustup.rs/)
- **Windows 10+**, macOS, or Linux

### Build
```bash
cd fortress
cargo build --release      # Optimized build (slower compile, faster runtime)
cargo build                # Debug build (faster compile, slower runtime)
```

### Run
```bash
cargo run                  # Run with debug info
cargo run --release       # Run optimized
```

**Expected output:**
```
Fortress starting...
Bevy app initialized: 512x512
Sand placed at (100-412, 50-100)
Water placed at (100-412, 120-150)
```

The grid is rendered to the window; sand and water fall under gravity. Use the mouse
to dig/build, keys 1-9 to select materials, and Space to toggle brush size.

### Test
```bash
cargo test --lib          # Run all unit tests
cargo test --lib -- --nocapture  # With println! output
cargo test physics::gravity       # Run only gravity tests
```

**Current test results:** 10/10 passing ✅

---

## 📊 Phase Progress Tracker

### ✅ Phase 1.1: Foundation & Module Structure (COMPLETE)
**Completed:**
- [x] Updated Cargo.toml with dependencies
- [x] Created module structure (lib.rs, materials.rs, grid.rs, physics/)
- [x] Implemented Material system (10 types, properties lookup)
- [x] Implemented Grid (flat Vec storage, get/set/swap)
- [x] Implemented Gravity physics (falling sand algorithm)
- [x] Created placeholders for rendering, input, liquids, reactions
- [x] All tests passing (10/10)
- [x] Code compiles with zero warnings

**Key Learning Outcomes:**
- Flat Vec is faster than Vec<Vec<>>
- Swaps are cheaper than copies for physics
- Bottom-to-top iteration prevents double-updates
- Exhaustive match statements catch edge cases
- Rust tests are your safety net

**Metrics:**
- Lines of code: ~470
- Module count: 7 (lib, main, materials, grid, physics/mod, physics/gravity, physics/liquids, physics/reactions, rendering, input)
- Test coverage: gravity (3 tests), grid (5 tests), materials (2 tests)

---

### ✅ Phase 1.3: Rendering & Visualization (COMPLETE & VERIFIED)
**Status:** Working! Window renders grid, physics simulates in real-time.

**Verified in testing:**
- ✅ Window opens at 512×512 pixels
- ✅ Sand visible and falling
- ✅ Water visible and flowing
- ✅ Smooth 60+ FPS rendering
- ✅ GPU: NVIDIA RTX 3060 Ti (Vulkan backend)
- ✅ Test run: 37 seconds continuous simulation, zero crashes
- ✅ Fixed: Out-of-bounds gravity access bug

**What you see when running:**
1. Black background = air
2. Tan region (rows 50-100) = sand
3. Blue region (rows 120-150) = water
4. Both materials fall and settle due to gravity

**Run it:**
```bash
cargo run --release
```

---

### ✅ Phase 1.3: Rendering & Visualization
**Goal:** Display the grid on screen using Bevy.

**TODO:**
- [x] Create Bevy app and window
- [x] Encode grid into RGBA texture each frame
- [x] Display the grid as a sprite
- [x] Verify real-time rendering

**Why this matters:** You need visual feedback to see physics work.

---

### ✅ Phase 1.5: Input & Interaction
**Goal:** Click to dig/build.

**Complete:**
- [x] Track mouse position and show a cursor sprite
- [x] Dig mode: left-click sets cells to AIR
- [x] Build mode: right-click places the selected material
- [x] Toggle brush size between 3×3 and 5×5
- [x] Select materials with keys 1-9 (sand, water, wood, stone, lava, acid, fire, steam, ash)
- [x] Use the camera-to-world-to-grid mapping for both cursor and grid input

---

### 🌊 Phase 2: Liquids & Reactions
**Goal:** Water spreads, fire spreads, things burn.

**Implemented:** Water, lava and acid spread sideways when they cannot fall, seek
nearby ledges and level out in containers; viscosity scales the flow substeps. Fire
and steam rise. Heat is simulated per cell: fire and lava emit heat, water absorbs it,
hot wood ignites, water boils into steam (which condenses when cold), lava solidifies
as it cools, and acid dissolves solids. The debug overlay shows the temperature under
the cursor. Focused, seeded tests cover each behavior (34 library tests total).

**TODO:**
- [x] Extend liquid flow toward horizontal equilibrium and account for viscosity (water seeks ledges within 8 cells, otherwise random-steps to level out)
- [x] Implement `reactions::process_reactions()` (temperature-driven ignition, boiling, condensation and solidification; fire burns out into ash)
- [x] Add temperature tracking (per-cell temperature, conduction, heat capacity)
- [x] Test: blocked water spreads sideways and water with a fall path keeps falling
- [x] Test: water flows downhill and spreads in containers
- [x] Test: wood catches fire, spreads, leaves ash

---

### 🧩 Phase 3: Chunking & World Streaming
**Goal:** Support large worlds (2048×2048+).

**TODO:**
- [ ] Refactor Grid → Chunk (256×256 or 128×128 blocks)
- [ ] Implement World container (HashMap of chunks)
- [ ] Implement chunk loading/unloading around player viewport
- [ ] Handle chunk boundary simulation (liquids don't stop at edges)
- [ ] Save/load chunks to disk (serde)

---

## 🧠 Key Rust Concepts Used

### 1. **Ownership & Borrowing**
```rust
// Physics needs mutable access to grid
pub fn apply_gravity(grid: &mut Grid) { ... }

// Rendering only reads
pub fn render(grid: &Grid) { ... }
```
**Learning:** Rust prevents data races at compile-time.

---

### 2. **Match Expression (Pattern Matching)**
```rust
match material_id {
    AIR => { ... }
    SAND => { ... }
    WATER => { ... }
    _ => { ... }  // Compiler forces all cases!
}
```
**Learning:** Exhaustive matching is safer than if/else chains.

---

### 3. **Flat Vec Indexing**
```rust
// ❌ Bad: Vec<Vec<>>
cells[x][y]  // Two pointer dereferences, fragmented memory

// ✅ Good: Flat Vec
cells[x + y * width]  // One index calculation, contiguous memory
```
**Learning:** Cache locality matters for performance.

---

### 4. **Vec::swap() for Physics**
```rust
grid.cells.swap(idx1, idx2);  // O(1), no allocation
```
**Learning:** Swapping is faster than cloning for physics loops.

---

### 5. **Type Aliases for Clarity**
```rust
pub type MaterialId = u8;  // Clear intent: "this u8 is a material ID"
```
**Learning:** Type aliases improve code readability.

---

### 6. **Unit Tests with #[test]**
```rust
#[test]
fn test_sand_falls() {
    let mut grid = Grid::new(10, 10);
    grid.set(5, 3, SAND);
    apply_gravity(&mut grid);
    assert_eq!(grid.get(5, 4), SAND);  // Moved down one cell
}
```
**Learning:** Tests live in the same file and catch regressions.

---

## 📚 External References

### The Powder Toy (Physics Reference)
- **GitHub:** https://github.com/The-Powder-Toy/The-Powder-Toy
- **Key files to study:**
  - `src/simulation/Simulation.cpp` — Main physics loop
  - `src/elements/` — Material definitions (how each element behaves)
- **Why:** Open-source, highly optimized, proven stable physics

### Bevy Game Engine
- **Docs:** https://bevyengine.org/
- **Book:** https://bevyengine.org/learn/
- **Why:** ECS architecture, Rust-native, great for 2D games

### Rust Learning Resources
- **The Book:** https://doc.rust-lang.org/book/
- **Rustlings:** Interactive exercises (https://github.com/rust-lang/rustlings)
- **Ownership & Borrowing:** https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html

---

## 🔍 Debugging Tips

### Check compilation
```bash
cargo check                # Faster than cargo build
```

### Run with debug output
```bash
RUST_BACKTRACE=1 cargo run   # See full panic traces
```

### Profile performance
```bash
cargo build --release
time ./target/release/fortress   # Measure runtime
```

### Memory layout (for grid optimization)
```bash
cargo +nightly build --release
# Use tools like `valgrind` or `perf` on Linux
```

---

## 🎯 Next Immediate Steps

1. ✅ **Understand the grid layout** (read through `src/grid.rs` and `src/materials.rs`)
2. ✅ **Run tests:** `cargo test --lib`
3. ✅ **Run the app:** `cargo run --release` — **You will see sand and water falling!**
4. ✅ **Study gravity algorithm** (read comments in `src/physics/gravity.rs`)
5. ✅ **Input is implemented:** click to dig/build, select materials, and change brush size

---

## 📝 Notes for Learning

### Why Rust?
- **Performance:** Zero-cost abstractions (as fast as C++)
- **Memory safety:** No null pointers, no data races
- **Great for game dev:** Strong type system catches bugs early
- **Bevy:** Modern ECS engine designed for Rust

### Why this architecture?
- **Flat Vec grid:** O(1) access, cache-friendly for physics
- **Material IDs:** Memory-efficient (1 byte per cell vs. 20+ bytes)
- **Dual-buffer physics:** Avoids borrow checker nightmares
- **ECS for high-level:** Bevy handles camera/input; physics is a separate system

### Common Pitfalls (and how we avoid them)
1. **Nested Vec** — ❌ Fragmented, slow indexing
2. **Mutex in hot loop** — ❌ Contention, overhead
3. **Cloning cells** — ❌ Expensive; we swap instead
4. **Top-to-bottom gravity** — ❌ Double-updates; we go bottom-to-top
5. **Storing full Material structs** — ❌ 20+ bytes per cell; we store IDs (1 byte)

---

## 🎉 What's Working Now

✅ Grid creation and manipulation  
✅ Falling sand physics (tested in unit tests)  
✅ Liquids that spread and level out (water, lava, acid)  
✅ Fire, steam, lava and acid reactions  
✅ Temperature: heat conduction, boiling, condensation, lava solidification, ignition  
✅ Heat glow with bloom and fire flicker  
✅ Material property system  
✅ Bevy rendering with 2× camera zoom, mouse dig/build and all 9 materials selectable (keys 1-9)  
✅ Cursor and grid actions share camera-based coordinate mapping  
✅ Zero compiler warnings  
✅ Fast, cache-friendly data structure  

## 🚫 What's Not Yet

🚫 Chunking & world streaming (Phase 3)  
🚫 Explosions  

---

## 📞 Quick Commands Cheatsheet

```bash
# Build & run
cargo run                          # Debug mode
cargo run --release                # Optimized
cargo check                        # Check only (fast)

# Testing
cargo test --lib                   # All tests
cargo test --lib physics::gravity  # Specific module
cargo test --lib -- --nocapture    # Show println!

# Code quality
cargo fmt                          # Format code
cargo clippy                       # Lint warnings

# Docs
cargo doc --open                   # Generate & open docs

# Clean
cargo clean                        # Remove build artifacts
```

---

**Last Updated:** 2026-10-12  
**Phase:** 2 (Liquids, Reactions & Temperature) — complete  
**Next:** Phase 3 (Chunking)
