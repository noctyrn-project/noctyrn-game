use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

mod objects;
use crate::settings::GameSettings;

pub struct World;

impl Plugin for World {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (init, objects::spawn_objects));
        app.add_systems(Update, update_lighting);
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
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
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
    let plane_size = 100.0;
    let mesh = Mesh::from(Plane3d::default().mesh().size(plane_size, plane_size));
    let mesh_handle = meshes.add(mesh);

    // Spawn the ground plane
    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Add a light so we can see
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.0,
            range: 100.0,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
        MainLight,
    ));
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
