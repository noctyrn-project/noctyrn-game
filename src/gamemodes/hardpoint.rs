//! Hardpoint – three smaller capture zones.
//!
//! **Map**: Same objective arena as KOTH (scaled for 50 players).
//! **Rules**: Three hardpoints, each with a 5-second capture timer.
//!   Hold zones to earn points. First to 250 wins.

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

/// Spawn three hardpoint zones spread across the map.
pub fn spawn_mode_entities(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let positions = [
        Vec3::new(-50.0, 0.03, 30.0),
        Vec3::new(0.0, 0.03, -40.0),
        Vec3::new(50.0, 0.03, 30.0),
    ];

    for (i, pos) in positions.iter().enumerate() {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(5.0, 0.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgba(0.2, 0.7, 0.8, if i == 0 { 0.3 } else { 0.15 }),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(*pos),
            ObjectiveZone { radius: 5.0, capture_rate: 1.0 },
        ));
    }
}
