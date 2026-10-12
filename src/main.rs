use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::asset::RenderAssetUsages;
use fortress_lib::grid::Grid;
use fortress_lib::input::{InputState, apply_brush};
use std::sync::Mutex;

/// Orthographic scale: 0.5 = 2x zoom, 0.25 = 4x zoom
const CAMERA_ZOOM_SCALE: f32 = 0.5;

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
        .add_systems(Update, (update_cursor_position, update_debug_text, handle_input, physics_step, render_grid).chain())
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
    
    // IMPORTANT: Add test data so we can see something!
    // Fill a region with sand at the top
    for y in 50..100 {
        for x in 100..412 {
            grid.set(x, y, fortress_lib::materials::SAND);
        }
    }
    
    // Add some water too
    for y in 120..150 {
        for x in 100..412 {
            grid.set(x, y, fortress_lib::materials::WATER);
        }
    }
    
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
    
    // Create cursor (small red square that follows mouse)
    commands.spawn((
        Sprite {
            color: Color::srgb(1.0, 0.0, 0.0),  // Red
            custom_size: Some(Vec2::new(20.0, 20.0)),  // 20x20 world units (more visible)
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
    println!("Sand placed at (100-412, 50-100)");
    println!("Water placed at (100-412, 120-150)");
}

/// Update cursor position based on mouse movement and display it
fn update_cursor_position(
    mut cursor_query: Query<&mut Transform, With<CursorMarker>>,
    q_windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    if let Ok(window) = q_windows.single() {
        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok((camera, camera_transform)) = camera_query.single() {
                if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
                    if let Ok(mut transform) = cursor_query.single_mut() {
                        transform.translation = world_pos.extend(1.0);
                    }
                }
            }
        }
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
    
    // Toggle brush size with Space
    if key_input.just_pressed(KeyCode::Space) {
        player_input.0.brush_size = if player_input.0.brush_size == 3 { 5 } else { 3 };
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



