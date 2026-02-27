//! Team Deathmatch – two teams, symmetric mirrored map with three lanes.
//!
//! **Map**: Large symmetric arena with clear red/blue spawn zones on opposite
//! ends, three distinct lanes with cross-walls, elevated catwalks, and side
//! buildings. Designed for 25 vs 25 (50 players total).
//! **Rules**: First team to 150 kills wins. 10-minute timer.

use bevy::prelude::*;
use crate::world::objects::StaticCollider;
use crate::world::GameWorldEntity;

/// Team spawn area marker – used to position players on their side.
#[derive(Component)]
pub struct TeamSpawnArea {
    /// 0 = red (negative X), 1 = blue (positive X)
    pub team: u8,
    pub center: Vec3,
    pub radius: f32,
}

/// Spawn the symmetric TDM arena – scaled for 50 players.
pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let concrete = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        perceptual_roughness: 0.9,
        ..default()
    });
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.42, 0.45),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });
    let red_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.18, 0.18),
        perceptual_roughness: 0.8,
        ..default()
    });
    let blue_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.18, 0.45),
        perceptual_roughness: 0.8,
        ..default()
    });

    // ── Lane-dividing walls (run along Z axis, separating three lanes) ──
    for z_sign in [-1.0_f32, 1.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(80.0, 3.5, 0.6))),
            MeshMaterial3d(concrete.clone()),
            Transform::from_xyz(0.0, 1.75, 20.0 * z_sign),
            StaticCollider { half_extents: Vec3::new(40.0, 1.75, 0.3) },
            GameWorldEntity,
        ));
    }

    // ── Cross-walls with gaps (perpendicular to lanes) ──
    let cross_x_positions = [-30.0_f32, -15.0, 0.0, 15.0, 30.0];
    let cross_wall_mesh = meshes.add(Cuboid::new(0.6, 3.0, 10.0));
    for x in cross_x_positions {
        for z_sign in [-1.0_f32, 1.0] {
            commands.spawn((
                Mesh3d(cross_wall_mesh.clone()),
                MeshMaterial3d(metal.clone()),
                Transform::from_xyz(x, 1.5, 10.0 * z_sign),
                StaticCollider { half_extents: Vec3::new(0.3, 1.5, 5.0) },
                GameWorldEntity,
            ));
        }
    }

    // ── Cover inside lanes (angled barriers) ──
    let barrier_mesh = meshes.add(Cuboid::new(4.0, 2.0, 0.5));
    let lane_cover = [
        // Centre lane
        (Vec3::new(-40.0, 1.0, 0.0), 0.4),
        (Vec3::new(40.0, 1.0, 0.0), -0.4),
        (Vec3::new(0.0, 1.0, 0.0), 0.0),
        // North lane
        (Vec3::new(-50.0, 1.0, 60.0), 0.3),
        (Vec3::new(50.0, 1.0, 60.0), -0.3),
        (Vec3::new(0.0, 1.0, 70.0), 0.7),
        (Vec3::new(-20.0, 1.0, 56.0), -0.5),
        (Vec3::new(20.0, 1.0, 56.0), 0.5),
        // South lane
        (Vec3::new(-50.0, 1.0, -60.0), -0.3),
        (Vec3::new(50.0, 1.0, -60.0), 0.3),
        (Vec3::new(0.0, 1.0, -70.0), -0.7),
        (Vec3::new(-20.0, 1.0, -56.0), 0.5),
        (Vec3::new(20.0, 1.0, -56.0), -0.5),
    ];
    for (pos, rot) in lane_cover {
        commands.spawn((
            Mesh3d(barrier_mesh.clone()),
            MeshMaterial3d(concrete.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(rot)),
            StaticCollider { half_extents: Vec3::new(2.0, 1.0, 0.25) },
            GameWorldEntity,
        ));
    }

    // ── Side buildings (large boxes with openings, mirrored) ──
    let building_mesh = meshes.add(Cuboid::new(8.0, 4.0, 8.0));
    let building_positions = [
        Vec3::new(80.0, 2.0, 70.0),
        Vec3::new(-80.0, 2.0, -70.0),
        Vec3::new(80.0, 2.0, -70.0),
        Vec3::new(-80.0, 2.0, 70.0),
    ];
    for pos in building_positions {
        commands.spawn((
            Mesh3d(building_mesh.clone()),
            MeshMaterial3d(concrete.clone()),
            Transform::from_translation(pos),
            StaticCollider { half_extents: Vec3::new(4.0, 2.0, 4.0) },
            GameWorldEntity,
        ));
    }

    // ── Red spawn zone (negative X end) ──
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.15, 20.0))),
        MeshMaterial3d(red_mat.clone()),
        Transform::from_xyz(-110.0, 0.08, 0.0),
        StaticCollider { half_extents: Vec3::new(6.0, 0.075, 10.0) },
        GameWorldEntity,
    ));
    // Red spawn cover walls
    for z in [-8.0_f32, 0.0, 8.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(4.0, 1.5, 0.5))),
            MeshMaterial3d(red_mat.clone()),
            Transform::from_xyz(-50.0, 0.75, z),
            StaticCollider { half_extents: Vec3::new(2.0, 0.75, 0.25) },
            GameWorldEntity,
        ));
    }
    commands.spawn((
        TeamSpawnArea { team: 0, center: Vec3::new(-110.0, 1.0, 0.0), radius: 12.0 },
        GameWorldEntity,
    ));

    // ── Blue spawn zone (positive X end) ──
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.15, 20.0))),
        MeshMaterial3d(blue_mat.clone()),
        Transform::from_xyz(110.0, 0.08, 0.0),
        StaticCollider { half_extents: Vec3::new(6.0, 0.075, 10.0) },
        GameWorldEntity,
    ));
    for z in [-8.0_f32, 0.0, 8.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(4.0, 1.5, 0.5))),
            MeshMaterial3d(blue_mat.clone()),
            Transform::from_xyz(50.0, 0.75, z),
            StaticCollider { half_extents: Vec3::new(2.0, 0.75, 0.25) },
            GameWorldEntity,
        ));
    }
    commands.spawn((
        TeamSpawnArea { team: 1, center: Vec3::new(110.0, 1.0, 0.0), radius: 12.0 },
        GameWorldEntity,
    ));
}

/// No extra entities for TDM – kills are the objective.
pub fn spawn_mode_entities(
    _commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {}
