use bevy::prelude::*;
use rand::Rng;
use crate::player::shooting::Target;

pub fn spawn_objects(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut rng = rand::rng();

    // Spawn some random cubes
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    
    for _ in 0..20 {
        let x = rng.random_range(-40.0..40.0);
        let z = rng.random_range(-40.0..40.0);
        let y = 0.5; // Half height so it sits on the floor
        
        let color = Color::srgb(rng.random(), rng.random(), rng.random());

        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                ..default()
            })),
            Transform::from_xyz(x, y, z),
            Target,
        ));
    }

    // Spawn some random spheres
    let sphere_mesh = meshes.add(Sphere::new(0.5));

    for _ in 0..20 {
        let x = rng.random_range(-40.0..40.0);
        let z = rng.random_range(-40.0..40.0);
        let y = 0.5;

        let color = Color::srgb(rng.random(), rng.random(), rng.random());

        commands.spawn((
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.1,
                metallic: 0.5,
                ..default()
            })),
            Transform::from_xyz(x, y, z),
            Target,
        ));
    }
    
    // Spawn a big central pillar
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 10.0, 4.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.2, 0.8))),
        Transform::from_xyz(10.0, 5.0, 10.0),
        Target,
    ));
}
