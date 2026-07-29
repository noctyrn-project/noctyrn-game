//! Capture The Flag game mode.
//!
//! Two teams try to capture the enemy's flag and return it to their own base.
//! First team to `score_limit` captures wins.

use bevy::prelude::*;
use crate::gameplay::FlagEntity;

/// Spawn CTF flags at each team's base.
pub fn spawn_flags(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let flag_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.2, 0.2),
        unlit: true,
        ..default()
    });
    let flag_mesh = meshes.add(Cylinder::new(0.3, 2.0));

    for (x, team) in [(-20.0, 0u8), (20.0, 1u8)] {
        commands.spawn((
            Mesh3d(flag_mesh.clone()),
            MeshMaterial3d(flag_mat.clone()),
            Transform::from_xyz(x, 1.0, 0.0),
            FlagEntity { team, held: false, home: Vec3::new(x, 1.0, 0.0) },
        ));
    }
}
