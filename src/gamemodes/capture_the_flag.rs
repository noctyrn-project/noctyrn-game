//! Capture the Flag – two flags, each team steals the enemy flag and
//! returns it to their own flag's location to score.
//!
//! **Map**: Large symmetric arena with coloured bases on opposite ends,
//! midfield cover, and side corridors. Designed for 25 vs 25.
//! **Rules**: Grab the enemy flag at their base, bring it back to your own
//!   flag platform to score. First to 3 captures wins. 10-minute timer.

use bevy::prelude::*;
use crate::world::objects::StaticCollider;
use crate::world::GameWorldEntity;
use crate::gameplay::{FlagEntity, ObjectiveZone};
use crate::gamemodes::team_deathmatch::TeamSpawnArea;

/// Spawn the CTF arena – scaled for 50 players.
pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let red_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.15, 0.15),
        perceptual_roughness: 0.8,
        ..default()
    });
    let blue_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.5),
        perceptual_roughness: 0.8,
        ..default()
    });
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

    // ── Red base platform (negative X) ──
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.5, 12.0))),
        MeshMaterial3d(red_mat.clone()),
        Transform::from_xyz(-110.0, 0.25, 0.0),
        StaticCollider { half_extents: Vec3::new(6.0, 0.25, 6.0) },
        GameWorldEntity,
    ));
    // Red base cover walls
    for z in [-5.0_f32, 5.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(6.0, 2.5, 0.5))),
            MeshMaterial3d(red_mat.clone()),
            Transform::from_xyz(-55.0, 1.25, z * 1.5),
            StaticCollider { half_extents: Vec3::new(3.0, 1.25, 0.25) },
            GameWorldEntity,
        ));
    }
    commands.spawn((
        TeamSpawnArea { team: 0, center: Vec3::new(-110.0, 1.0, 0.0), radius: 10.0 },
        GameWorldEntity,
    ));

    // ── Blue base platform (positive X) ──
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(12.0, 0.5, 12.0))),
        MeshMaterial3d(blue_mat.clone()),
        Transform::from_xyz(110.0, 0.25, 0.0),
        StaticCollider { half_extents: Vec3::new(6.0, 0.25, 6.0) },
        GameWorldEntity,
    ));
    for z in [-5.0_f32, 5.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(6.0, 2.5, 0.5))),
            MeshMaterial3d(blue_mat.clone()),
            Transform::from_xyz(55.0, 1.25, z * 1.5),
            StaticCollider { half_extents: Vec3::new(3.0, 1.25, 0.25) },
            GameWorldEntity,
        ));
    }
    commands.spawn((
        TeamSpawnArea { team: 1, center: Vec3::new(110.0, 1.0, 0.0), radius: 10.0 },
        GameWorldEntity,
    ));

    // ── Center walls (midfield cover) ──
    let mid_wall_mesh = meshes.add(Cuboid::new(10.0, 3.5, 0.6));
    for z in [-12.0_f32, 12.0] {
        commands.spawn((
            Mesh3d(mid_wall_mesh.clone()),
            MeshMaterial3d(concrete.clone()),
            Transform::from_xyz(0.0, 1.75, z),
            StaticCollider { half_extents: Vec3::new(5.0, 1.75, 0.3) },
            GameWorldEntity,
        ));
    }
    // Centre pillar
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 4.0, 3.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(0.0, 2.0, 0.0),
        StaticCollider { half_extents: Vec3::new(1.5, 2.0, 1.5) },
        GameWorldEntity,
    ));

    // ── Side corridors cover (4 sets, mirrored) ──
    let corridor_wall = meshes.add(Cuboid::new(0.6, 2.5, 5.0));
    let corridor_positions = [
        (Vec3::new(50.0, 1.25, 40.0), 0.3),
        (Vec3::new(-50.0, 1.25, -40.0), 0.3),
        (Vec3::new(50.0, 1.25, -40.0), -0.3),
        (Vec3::new(-50.0, 1.25, 40.0), -0.3),
    ];
    for (pos, rot) in corridor_positions {
        commands.spawn((
            Mesh3d(corridor_wall.clone()),
            MeshMaterial3d(metal.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(rot)),
            StaticCollider { half_extents: Vec3::new(0.3, 1.25, 2.5) },
            GameWorldEntity,
        ));
    }

    // ── Scattered cover between bases and mid ──
    let cover_wall = meshes.add(Cuboid::new(4.0, 2.0, 0.5));
    let cover = [
        (Vec3::new(-60.0, 1.0, 20.0), 0.5),
        (Vec3::new(-60.0, 1.0, -20.0), -0.5),
        (Vec3::new(60.0, 1.0, 20.0), -0.5),
        (Vec3::new(60.0, 1.0, -20.0), 0.5),
        (Vec3::new(-80.0, 1.0, 30.0), 0.0),
        (Vec3::new(-80.0, 1.0, -30.0), 0.0),
        (Vec3::new(80.0, 1.0, 30.0), 0.0),
        (Vec3::new(80.0, 1.0, -30.0), 0.0),
        (Vec3::new(-30.0, 1.0, 0.0), 0.8),
        (Vec3::new(30.0, 1.0, 0.0), -0.8),
    ];
    for (pos, rot) in cover {
        commands.spawn((
            Mesh3d(cover_wall.clone()),
            MeshMaterial3d(concrete.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(rot)),
            StaticCollider { half_extents: Vec3::new(2.0, 1.0, 0.25) },
            GameWorldEntity,
        ));
    }
}

/// Spawn TWO flags – one at each base. Each team steals the enemy's flag
/// and brings it back to their own flag location to score.
pub fn spawn_mode_entities(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let flag_mesh = meshes.add(Cuboid::new(0.2, 2.5, 0.2));

    // ── Red team's flag (at red base, steal by blue team) ──
    let red_flag_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.2, 0.2),
        ..default()
    });
    commands.spawn((
        Mesh3d(flag_mesh.clone()),
        MeshMaterial3d(red_flag_mat),
        Transform::from_xyz(-110.0, 1.25, 0.0),
        FlagEntity { team: 0, held: false, home: Vec3::new(-110.0, 1.25, 0.0) },
    ));

    // ── Blue team's flag (at blue base, steal by red team) ──
    let blue_flag_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 1.0),
        ..default()
    });
    commands.spawn((
        Mesh3d(flag_mesh),
        MeshMaterial3d(blue_flag_mat),
        Transform::from_xyz(110.0, 1.25, 0.0),
        FlagEntity { team: 1, held: false, home: Vec3::new(110.0, 1.25, 0.0) },
    ));

    // ── Flag base marker zones (glowing circles at each base) ──
    let base_marker = |color: Color| StandardMaterial {
        base_color: color,
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    };
    // Red base marker
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(3.0, 0.05))),
        MeshMaterial3d(materials.add(base_marker(Color::srgba(1.0, 0.3, 0.3, 0.25)))),
        Transform::from_xyz(-110.0, 0.03, 0.0),
        ObjectiveZone { radius: 3.0, capture_rate: 0.0 },
    ));
    // Blue base marker
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(3.0, 0.05))),
        MeshMaterial3d(materials.add(base_marker(Color::srgba(0.3, 0.5, 1.0, 0.25)))),
        Transform::from_xyz(110.0, 0.03, 0.0),
        ObjectiveZone { radius: 3.0, capture_rate: 0.0 },
    ));
}
