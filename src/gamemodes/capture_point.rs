//! Capture Point – three control points A / B / C that move around.
//!
//! **Map**: Same objective arena as KOTH (scaled for 50 players).
//! **Rules**: Stand on a point to instantly capture it. Each captured point
//!   earns 1 point/second. Points relocate every 30 seconds. First to 200 wins.

use bevy::prelude::*;
use crate::gameplay::ObjectiveZone;

/// Reuses the objective arena.
pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    super::king_of_the_hill::spawn_objective_arena(commands, meshes, materials);
}

/// Spawn three capture points A, B, C – these will be moved by gameplay logic.
pub fn spawn_mode_entities(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let points = [
        ("A", Vec3::new(-60.0, 0.03, 0.0)),
        ("B", Vec3::new(0.0, 0.03, -50.0)),
        ("C", Vec3::new(60.0, 0.03, 0.0)),
    ];

    for (_label, pos) in points {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(4.5, 0.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.5, 0.8, 0.3, 0.25),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(pos),
            ObjectiveZone { radius: 4.5, capture_rate: 0.8 },
        ));
    }
}
