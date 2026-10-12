use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::asset::RenderAssetUsages;
use fortress_lib::grid::Grid;
use fortress_lib::input::{InputState, apply_brush};
use std::sync::Mutex;

/// Orthographic scale: 0.5 = 2x zoom, 0.25 = 4x zoom
const CAMERA_ZOOM_SCALE: f32 = 0.5;

/// Starting terrain layer thicknesses, in grid rows.
const BEDROCK_THICKNESS: usize = 20;
const SAND_THICKNESS: usize = 60;
const WATER_THICKNESS: usize = 50;

// Shared grid state
#[derive(Resource)]
struct SharedGrid(Mutex<Grid>);

// Track the image handle
#[derive(Resource)]
struct GridImageHandle(Handle<Image>);

// Player input state
#[derive(Resource, Default)]
struct PlayerInput(InputState);

// Marker for cursor entity
#[derive(Component)]
struct CursorMarker;

// Marker for debug text entity
#[derive(Component)]
struct DebugText;

/// Convert camera world coordinates to grid coordinates for the centered 512x512 image.
fn world_to_grid_coords(world_pos: Vec2) -> (i32, i32) {
    let grid_x = (world_pos.x + 256.0).floor().clamp(0.0, 511.0) as i32;
    let grid_y = (256.0 - world_pos.y).floor().clamp(0.0, 511.0) as i32;
    (grid_x, grid_y)
}

fn main() {
    println!("Fortress starting...");
    
    let grid = Grid::new(512, 512);
    
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(SharedGrid(Mutex::new(grid)))
        .insert_resource(GridImageHandle(Handle::default()))
        .insert_resource(PlayerInput::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (update_debug_text, handle_input, update_cursor_position, physics_step, render_grid).chain())
        .run();
}

/// Startup: create camera, texture, and sprite
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    shared_grid: Res<SharedGrid>,
) {
    // Create camera
    // Zoom in: scale < 1 shows less world, so each grid pixel appears bigger
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scale: CAMERA_ZOOM_SCALE,
            ..OrthographicProjection::default_2d()
        }),
    ));
    
    // Get grid size
    let mut grid = shared_grid.0.lock().unwrap();
    let width = grid.width as u32;
    let height = grid.height as u32;
    
    // Starting terrain: bedrock at the bottom, sand above it, water on top
    grid.fill_base_layers(BEDROCK_THICKNESS, SAND_THICKNESS, WATER_THICKNESS);
    
    drop(grid);
    
    // Create initial image data
    let data = vec![0u8; (width * height * 4) as usize];
    
    // Create image
    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    
    let image_handle = images.add(image);
    commands.insert_resource(GridImageHandle(image_handle.clone()));
    
    // Create sprite and scale it to be visible
    // The sprite is 512x512 pixels, scale up for visibility
    commands.spawn(Sprite {
        image: image_handle,
        // Scale by 2 so 512x512 becomes 1024x1024 in world units
        custom_size: Some(Vec2::new(512.0, 512.0)),
        ..default()
    });
    
    // Create cursor (translucent red square matching the brush size, resized every frame)
    commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 0.0, 0.0, 0.4),
            custom_size: Some(Vec2::splat(8.0)),
            ..default()
        },
        Transform::default(),
        CursorMarker,
    ));
    
    // Create debug text display (using simpler Bevy 0.20 API)
    commands.spawn((
        Text::new("Debug Info"),
        TextColor(Color::srgb(1.0, 1.0, 1.0)),  // White text
        Transform::from_xyz(-240.0, 240.0, 2.0),  // Top-left corner
        DebugText,
    ));
    
    println!("Bevy app initialized: {}x{}", width, height);
    println!("Terrain layers (bottom to top): bedrock {BEDROCK_THICKNESS}, sand {SAND_THICKNESS}, water {WATER_THICKNESS} rows");
}

/// Keep the red cursor square matched to the brush: same size in cells, snapped to
/// the grid so it covers exactly the cells the brush will paint.
fn update_cursor_position(
    mut cursor_query: Query<(&mut Transform, &mut Sprite), With<CursorMarker>>,
    player_input: Res<PlayerInput>,
) {
    if let Ok((mut transform, mut sprite)) = cursor_query.single_mut() {
        let size = player_input.0.brush_size as f32;
        sprite.custom_size = Some(Vec2::splat(size));
        // Inverse of world_to_grid_coords: the brush is centered on the cell's top-left corner
        transform.translation = Vec3::new(
            player_input.0.cursor_x as f32 - 256.0,
            256.0 - player_input.0.cursor_y as f32,
            1.0,
        );
    }
}

/// Update debug text display with cursor position and material info
fn update_debug_text(
    mut text_query: Query<&mut Text, With<DebugText>>,
    player_input: Res<PlayerInput>,
    q_windows: Query<&Window>,
    shared_grid: Res<SharedGrid>,
) {
    let cursor_temp = shared_grid
        .0
        .lock()
        .unwrap()
        .temp(player_input.0.cursor_x as usize, player_input.0.cursor_y as usize);

    // Material names
    let material_names = ["AIR", "SAND", "WATER", "WOOD", "STONE", "LAVA", "ACID", "FIRE", "STEAM", "ASH"];
    let material_name = if (player_input.0.selected_material as usize) < material_names.len() {
        material_names[player_input.0.selected_material as usize]
    } else {
        "???"
    };
    
    let window_info = if let Ok(window) = q_windows.single() {
        let size = window.resolution.physical_size();
        format!("Window: {}x{}", size.x, size.y)
    } else {
        "Window: ???".to_string()
    };
    
    let debug_text = format!(
        "Mouse: ({}, {})\nTemp: {:.0} C\nMaterial: {} ({})\nBrush: {}x{}\n{}",
        player_input.0.cursor_x,
        player_input.0.cursor_y,
        cursor_temp,
        material_name,
        player_input.0.selected_material,
        player_input.0.brush_size,
        player_input.0.brush_size,
        window_info
    );
    
    if let Ok(mut text) = text_query.single_mut() {
        text.0 = debug_text;
    }
}
fn handle_input(
    mut player_input: ResMut<PlayerInput>,
    shared_grid: ResMut<SharedGrid>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    key_input: Res<ButtonInput<KeyCode>>,
    q_windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    // Update cursor position from window
    if let Ok(window) = q_windows.single() {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = camera_query.single() {
                if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    let (grid_x, grid_y) = world_to_grid_coords(world_pos);
                    player_input.0.cursor_x = grid_x;
                    player_input.0.cursor_y = grid_y;
                }
            }
        }
    }
    
    // Handle number keys for material selection
    for key_code in [
        KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4,
        KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8,
        KeyCode::Digit9,
    ] {
        if key_input.just_pressed(key_code) {
            let key_num = match key_code {
                KeyCode::Digit1 => 1,
                KeyCode::Digit2 => 2,
                KeyCode::Digit3 => 3,
                KeyCode::Digit4 => 4,
                KeyCode::Digit5 => 5,
                KeyCode::Digit6 => 6,
                KeyCode::Digit7 => 7,
                KeyCode::Digit8 => 8,
                KeyCode::Digit9 => 9,
                _ => 0,
            };
            
            if let Some(material) = fortress_lib::input::material_from_key(48 + key_num) {
                player_input.0.selected_material = material;
                let material_names = ["AIR", "SAND", "WATER", "WOOD", "STONE", "LAVA", "ACID", "FIRE", "STEAM", "ASH"];
                if (material as usize) < material_names.len() {
                    println!("Selected: {}", material_names[material as usize]);
                }
            }
        }
    }
    
    // Numpad +/- steps through the brush sizes
    let mut size_changed = false;
    if key_input.just_pressed(KeyCode::NumpadAdd) {
        player_input.0.increase_brush();
        size_changed = true;
    }
    if key_input.just_pressed(KeyCode::NumpadSubtract) {
        player_input.0.decrease_brush();
        size_changed = true;
    }
    if size_changed {
        println!("Brush size: {}x{}", player_input.0.brush_size, player_input.0.brush_size);
    }
    
    // Left-click: dig (remove material)
    if mouse_button_input.pressed(MouseButton::Left) {
        let mut grid = shared_grid.0.lock().unwrap();
        apply_brush(
            &mut grid,
            player_input.0.cursor_x,
            player_input.0.cursor_y,
            None,  // None = dig = place AIR
            player_input.0.brush_size,
        );
    }
    
    // Right-click: build (place selected material)
    if mouse_button_input.pressed(MouseButton::Right) {
        let mut grid = shared_grid.0.lock().unwrap();
        apply_brush(
            &mut grid,
            player_input.0.cursor_x,
            player_input.0.cursor_y,
            Some(player_input.0.selected_material),
            player_input.0.brush_size,
        );
    }
}
fn physics_step(shared_grid: ResMut<SharedGrid>) {
    let mut grid = shared_grid.0.lock().unwrap();
    fortress_lib::physics::step(&mut grid, 0.016);
}

/// Render: grid to texture
fn render_grid(
    mut images: ResMut<Assets<Image>>,
    shared_grid: Res<SharedGrid>,
    image_handle: Res<GridImageHandle>,
    mut frame: Local<u32>,
) {
    *frame = frame.wrapping_add(1);
    let pixels = {
        let grid = shared_grid.0.lock().unwrap();
        fortress_lib::rendering::render_pixels(&grid, *frame)
    };
    
    // Update image
    if let Some(mut image) = images.get_mut(&image_handle.0) {
        image.data = Some(pixels);
    }
}



