//! King of the Hill – control a single hill zone to earn points.
//!
//! **Map**: Large objective-centric arena with cover surrounding a central zone.
//! Designed for 50 players. 5-second capture, 5-second uncapture.
//! **Rules**: Stand on the hill to earn 1 point/second. First to 250 wins.

use bevy::prelude::*;
use crate::world::objects::StaticCollider;
use crate::world::GameWorldEntity;
use crate::gameplay::ObjectiveZone;
use crate::gamemodes::team_deathmatch::TeamSpawnArea;

/// Spawn the KOTH arena.
pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_objective_arena(commands, meshes, materials);
}

/// Spawn the hill zone at centre.
pub fn spawn_mode_entities(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(7.0, 0.05))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.9, 0.5, 0.1, 0.3),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.03, 0.0),
        ObjectiveZone { radius: 7.0, capture_rate: 1.0 },
    ));
}

// ── Shared objective-arena layout ──────────────────────────────

/// A reusable arena layout with cover surrounding a central objective area.
/// Scaled for 50 players with team spawns on opposite ends.
pub(crate) fn spawn_objective_arena(
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
    let zone_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.2, 0.6, 0.3, 0.5),
        perceptual_roughness: 0.5,
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

    // Central objective zone marker (flat square on ground)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 0.05, 10.0))),
        MeshMaterial3d(zone_mat),
        Transform::from_xyz(0.0, 0.03, 0.0),
        GameWorldEntity,
    ));

    // ── Inner ring of cover (radius ~15) ──
    let inner_wall = meshes.add(Cuboid::new(4.0, 2.5, 0.5));
    let inner_cover: [(Vec3, f32); 8] = [
        (Vec3::new(24.0, 1.25, 0.0), 0.0),
        (Vec3::new(-24.0, 1.25, 0.0), 0.0),
        (Vec3::new(0.0, 1.25, 24.0), std::f32::consts::FRAC_PI_2),
        (Vec3::new(0.0, 1.25, -24.0), std::f32::consts::FRAC_PI_2),
        (Vec3::new(20.0, 1.25, 20.0), 0.7),
        (Vec3::new(-20.0, 1.25, -20.0), 0.7),
        (Vec3::new(20.0, 1.25, -20.0), -0.7),
        (Vec3::new(-20.0, 1.25, 20.0), -0.7),
    ];
    for (pos, rot) in inner_cover {
        commands.spawn((
            Mesh3d(inner_wall.clone()),
            MeshMaterial3d(concrete.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(rot)),
            StaticCollider { half_extents: Vec3::new(2.0, 1.25, 0.25) },
            GameWorldEntity,
        ));
    }

    // ── Outer ring of cover (radius ~35-45) ──
    let outer_wall = meshes.add(Cuboid::new(5.0, 2.5, 0.5));
    let outer_cover: [(Vec3, f32); 12] = [
        (Vec3::new(70.0, 1.25, 0.0), 0.3),
        (Vec3::new(-70.0, 1.25, 0.0), -0.3),
        (Vec3::new(0.0, 1.25, 70.0), 0.0),
        (Vec3::new(0.0, 1.25, -70.0), 0.0),
        (Vec3::new(56.0, 1.25, 56.0), 0.8),
        (Vec3::new(-56.0, 1.25, -56.0), 0.8),
        (Vec3::new(56.0, 1.25, -56.0), -0.8),
        (Vec3::new(-56.0, 1.25, 56.0), -0.8),
        (Vec3::new(80.0, 1.25, 30.0), 0.5),
        (Vec3::new(-80.0, 1.25, -30.0), 0.5),
        (Vec3::new(30.0, 1.25, 80.0), -0.5),
        (Vec3::new(-30.0, 1.25, -80.0), -0.5),
    ];
    for (pos, rot) in outer_cover {
        commands.spawn((
            Mesh3d(outer_wall.clone()),
            MeshMaterial3d(concrete.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(rot)),
            StaticCollider { half_extents: Vec3::new(2.5, 1.25, 0.25) },
            GameWorldEntity,
        ));
    }

    // ── Buildings / large structures at corners ──
    let building = meshes.add(Cuboid::new(6.0, 3.5, 6.0));
    let building_positions = [
        Vec3::new(80.0, 1.75, 80.0),
        Vec3::new(-80.0, 1.75, -80.0),
        Vec3::new(80.0, 1.75, -80.0),
        Vec3::new(-80.0, 1.75, 80.0),
    ];
    for pos in building_positions {
        commands.spawn((
            Mesh3d(building.clone()),
            MeshMaterial3d(metal.clone()),
            Transform::from_translation(pos),
            StaticCollider { half_extents: Vec3::new(3.0, 1.75, 3.0) },
            GameWorldEntity,
        ));
    }

    // ── Team spawn areas ──
    // Red spawn (negative X)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 0.1, 16.0))),
        MeshMaterial3d(red_mat),
        Transform::from_xyz(-110.0, 0.05, 0.0),
        GameWorldEntity,
    ));
    commands.spawn((
        TeamSpawnArea { team: 0, center: Vec3::new(-110.0, 1.0, 0.0), radius: 10.0 },
        GameWorldEntity,
    ));
    // Blue spawn (positive X)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(10.0, 0.1, 16.0))),
        MeshMaterial3d(blue_mat),
        Transform::from_xyz(110.0, 0.05, 0.0),
        GameWorldEntity,
    ));
    commands.spawn((
        TeamSpawnArea { team: 1, center: Vec3::new(110.0, 1.0, 0.0), radius: 10.0 },
        GameWorldEntity,
    ));
}
