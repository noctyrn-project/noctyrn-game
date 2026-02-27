use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy::camera::visibility::RenderLayers;

pub mod objects;
use crate::settings::GameSettings;
use crate::player::GameState;
use crate::menu::SelectedGameMode;
use crate::gamemodes;

/// Marker component for all game-world entities that should be cleaned up between maps.
#[derive(Component)]
pub struct GameWorldEntity;

pub struct World;

impl Plugin for World {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), spawn_game_map);
        app.add_systems(OnExit(GameState::Playing), despawn_game_map);
        app.add_systems(Update, (update_lighting, objects::update_moving_targets, objects::update_popup_targets, objects::update_glass_shards)
            .run_if(in_state(GameState::Playing)));
    }
}

/// Spawn the appropriate map based on the selected gamemode.
fn spawn_game_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    selected_mode: Res<SelectedGameMode>,
) {
    // Always spawn ground + lighting
    init(&mut commands, &mut meshes, &mut materials, &mut images);

    // Delegate to per-gamemode module for map geometry
    gamemodes::spawn_map_for_mode(
        selected_mode.mode,
        &mut commands,
        &mut meshes,
        &mut materials,
    );

    // Spawn mode-specific entities (flags, zones, etc.)
    gamemodes::spawn_mode_entities(
        selected_mode.mode,
        &mut commands,
        &mut meshes,
        &mut materials,
    );
}

/// Despawn all game-world entities when leaving Playing state.
fn despawn_game_map(
    mut commands: Commands,
    world_query: Query<Entity, With<GameWorldEntity>>,
    collider_query: Query<Entity, With<objects::StaticCollider>>,
    ramp_query: Query<Entity, With<objects::RampCollider>>,
    terminal_query: Query<Entity, With<objects::WeaponTerminal>>,
    material_query: Query<Entity, With<objects::MaterialType>>,
    moving_target_query: Query<Entity, With<objects::MovingTarget>>,
    popup_target_query: Query<Entity, With<objects::PopUpTarget>>,
    distance_marker_query: Query<Entity, With<objects::DistanceMarker>>,
    main_light_query: Query<Entity, With<MainLight>>,
) {
    for entity in world_query.iter()
        .chain(collider_query.iter())
        .chain(ramp_query.iter())
        .chain(terminal_query.iter())
        .chain(material_query.iter())
        .chain(moving_target_query.iter())
        .chain(popup_target_query.iter())
        .chain(distance_marker_query.iter())
        .chain(main_light_query.iter())
    {
        if let Ok(mut cmds) = commands.get_entity(entity) {
            cmds.despawn();
        }
    }
}

fn update_lighting(
    settings: Res<GameSettings>,
    mut query: Query<&mut PointLight, With<MainLight>>,
) {
    if settings.is_changed() {
        for mut light in query.iter_mut() {
            light.shadows_enabled = match settings.graphics.shadow_quality.as_str() {
                "Low" => false,
                _ => true,
            };
            // You could also adjust shadow map size here if Bevy exposed it easily on the component,
            // but usually that's a resource configuration. For now, toggling shadows is a good start.
        }
    }
}

pub fn init(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    images: &mut ResMut<Assets<Image>>,
) {
    // Create a grid texture
    let image = create_grid_image();
    let texture_handle = images.add(image);

    // Create a material with the grid texture
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE, // Tint
        base_color_texture: Some(texture_handle),
        perceptual_roughness: 0.8,
        metallic: 0.2,
        ..default()
    });

    // Create a large plane mesh
    let plane_size = 300.0;
    let mesh = Mesh::from(Plane3d::default().mesh().size(plane_size, plane_size));
    let mesh_handle = meshes.add(mesh);

    // Spawn the ground plane
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, 0.0),
        GameWorldEntity,
    ));

    // Add lights for the larger arena
    let light_positions = [
        Vec3::new(0.0, 25.0, 0.0),
        Vec3::new(50.0, 20.0, 50.0),
        Vec3::new(-50.0, 20.0, -50.0),
    ];
    for (i, pos) in light_positions.iter().enumerate() {
        commands.spawn((
            PointLight {
                shadows_enabled: i == 0, // Only main light casts shadows
                intensity: 15_000_000.0,
                range: 200.0,
                ..default()
            },
            Transform::from_translation(*pos),
            RenderLayers::from_layers(&[0, 1]),
            MainLight,
            GameWorldEntity,
        ));
    }
}

fn create_grid_image() -> Image {
    let width = 1000;
    let height = 1000;
    let size = Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let mut pixel_data = Vec::with_capacity((width * height * 4) as usize);
    
    let dark_grey = [40, 40, 40, 255];
    let light_grey = [100, 100, 100, 255];

    for y in 0..height {
        for x in 0..width {
            // Draw a grid line every 20 pixels (50 cells total)
            if x % 20 < 2 || y % 20 < 2 {
                pixel_data.extend_from_slice(&light_grey);
            } else {
                pixel_data.extend_from_slice(&dark_grey);
            }
        }
    }

    let mut image = Image::new(
        size,
        TextureDimension::D2,
        pixel_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    
    // Set the sampler to repeat (though we map 0..1 so it doesn't matter much, but good practice)
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        ..default()
    });
    
    image
}

#[derive(Component)]
pub struct MainLight;