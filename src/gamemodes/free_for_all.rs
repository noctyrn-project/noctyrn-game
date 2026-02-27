//! Free-For-All – every player for themselves.
//!
//! **Map**: Large open arena with scattered angled cover walls, pillars,
//! buildings, and elevated sniper platforms. Designed for up to 50 players.
//! **Rules**: First to 30 kills wins. 10-minute timer.

use bevy::prelude::*;
use crate::world::objects::StaticCollider;
use crate::world::GameWorldEntity;

/// Helper: spawn a cover wall with rotation.
fn spawn_cover(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    pos: Vec3,
    rot_y: f32,
    half_extents: Vec3,
) {
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(pos)
            .with_rotation(Quat::from_rotation_y(rot_y)),
        StaticCollider { half_extents },
        GameWorldEntity,
    ));
}

/// Spawn the FFA arena geometry – scaled for 50 players.
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
    let dark_concrete = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.25, 0.28),
        perceptual_roughness: 0.95,
        ..default()
    });
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.42, 0.45),
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });
    let platform_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.3, 0.34),
        perceptual_roughness: 0.85,
        ..default()
    });

    // ── Inner ring: 8 angled cover walls (radius ~20) ──
    let inner_walls = [
        (Vec3::new(30.0, 1.0, 30.0),  0.7),
        (Vec3::new(-30.0, 1.0, -30.0), 0.7),
        (Vec3::new(36.0, 1.0, -16.0),  -0.4),
        (Vec3::new(-36.0, 1.0, 16.0),  -0.4),
        (Vec3::new(0.0, 1.0, 44.0),   1.2),
        (Vec3::new(0.0, 1.0, -44.0),  -1.2),
        (Vec3::new(44.0, 1.0, 0.0),   0.0),
        (Vec3::new(-44.0, 1.0, 0.0),  0.0),
    ];
    let wall_mesh = meshes.add(Cuboid::new(4.0, 2.5, 0.5));
    for (pos, rot) in inner_walls {
        spawn_cover(commands, wall_mesh.clone(), concrete.clone(), pos, rot,
            Vec3::new(2.0, 1.25, 0.25));
    }

    // ── Outer ring: 12 cover walls (radius ~45-55) ──
    let outer_walls = [
        (Vec3::new(80.0, 1.0, 0.0),    0.3),
        (Vec3::new(-80.0, 1.0, 0.0),  -0.3),
        (Vec3::new(0.0, 1.0, 80.0),    0.0),
        (Vec3::new(0.0, 1.0, -80.0),   0.0),
        (Vec3::new(60.0, 1.0, 60.0),   0.8),
        (Vec3::new(-60.0, 1.0, -60.0), 0.8),
        (Vec3::new(60.0, 1.0, -60.0), -0.8),
        (Vec3::new(-60.0, 1.0, 60.0), -0.8),
        (Vec3::new(100.0, 1.0, 40.0),   0.5),
        (Vec3::new(-100.0, 1.0, -40.0), 0.5),
        (Vec3::new(40.0, 1.0, 100.0),  -0.5),
        (Vec3::new(-40.0, 1.0, -100.0),-0.5),
    ];
    let outer_wall_mesh = meshes.add(Cuboid::new(5.0, 2.5, 0.5));
    for (pos, rot) in outer_walls {
        spawn_cover(commands, outer_wall_mesh.clone(), dark_concrete.clone(), pos, rot,
            Vec3::new(2.5, 1.25, 0.25));
    }

    // ── Mid-range structures: L-shaped cover (pairs of walls) ──
    let l_shapes: [(Vec3, f32); 6] = [
        (Vec3::new(70.0, 1.0, 20.0),  0.0),
        (Vec3::new(-70.0, 1.0, -20.0), 0.0),
        (Vec3::new(20.0, 1.0, 70.0),  std::f32::consts::FRAC_PI_2),
        (Vec3::new(-20.0, 1.0, -70.0), std::f32::consts::FRAC_PI_2),
        (Vec3::new(90.0, 1.0, -60.0), 0.4),
        (Vec3::new(-90.0, 1.0, 60.0), 0.4),
    ];
    let l_wall = meshes.add(Cuboid::new(4.0, 2.5, 0.5));
    let l_perp = meshes.add(Cuboid::new(0.5, 2.5, 3.0));
    for (pos, rot) in l_shapes {
        let q = Quat::from_rotation_y(rot);
        // Long segment
        commands.spawn((
            Mesh3d(l_wall.clone()),
            MeshMaterial3d(metal.clone()),
            Transform::from_translation(pos).with_rotation(q),
            StaticCollider { half_extents: Vec3::new(2.0, 1.25, 0.25) },
            GameWorldEntity,
        ));
        // Perpendicular wing
        let wing_offset = q * Vec3::new(4.0, 0.0, 3.0);
        commands.spawn((
            Mesh3d(l_perp.clone()),
            MeshMaterial3d(metal.clone()),
            Transform::from_translation(pos + wing_offset).with_rotation(q),
            StaticCollider { half_extents: Vec3::new(0.25, 1.25, 1.5) },
            GameWorldEntity,
        ));
    }

    // ── Central structure: large pillar with small walls ──
    let pillar_mesh = meshes.add(Cuboid::new(3.0, 4.0, 3.0));
    commands.spawn((
        Mesh3d(pillar_mesh),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(0.0, 2.0, 0.0),
        StaticCollider { half_extents: Vec3::new(1.5, 2.0, 1.5) },
        GameWorldEntity,
    ));
    // Buttress walls around central pillar
    let buttress_mesh = meshes.add(Cuboid::new(2.0, 2.0, 0.4));
    for angle in [0.0_f32, 1.57, 3.14, 4.71] {
        let offset = Vec3::new(angle.cos() * 5.0, 1.0, angle.sin() * 5.0);
        spawn_cover(commands, buttress_mesh.clone(), concrete.clone(), offset, angle + 0.8,
            Vec3::new(1.0, 1.0, 0.2));
    }

    // ── Elevated sniper platforms (4 corners) ──
    let plat_mesh = meshes.add(Cuboid::new(6.0, 0.3, 6.0));
    let plat_positions = [
        Vec3::new(110.0, 2.5, 110.0),
        Vec3::new(-110.0, 2.5, -110.0),
        Vec3::new(110.0, 2.5, -110.0),
        Vec3::new(-110.0, 2.5, 110.0),
    ];
    for pos in plat_positions {
        commands.spawn((
            Mesh3d(plat_mesh.clone()),
            MeshMaterial3d(platform_mat.clone()),
            Transform::from_translation(pos),
            StaticCollider { half_extents: Vec3::new(3.0, 0.15, 3.0) },
            GameWorldEntity,
        ));
        // Ramp up to platform
        let toward_center = -pos.normalize() * 5.0;
        let ramp_pos = pos + toward_center + Vec3::new(0.0, -1.0, 0.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.0, 0.2, 6.0))),
            MeshMaterial3d(platform_mat.clone()),
            Transform::from_translation(ramp_pos)
                .with_rotation(Quat::from_rotation_x(0.25)
                    * Quat::from_rotation_y(pos.z.atan2(pos.x))),
            StaticCollider { half_extents: Vec3::new(1.0, 0.1, 3.0) },
            GameWorldEntity,
        ));
    }

    // ── Shipping containers / crates (scattered) ──
    let crate_mesh = meshes.add(Cuboid::new(2.5, 2.5, 5.0));
    let crate_positions = [
        (Vec3::new(50.0, 1.25, -30.0), 0.3),
        (Vec3::new(-50.0, 1.25, 30.0), -0.3),
        (Vec3::new(30.0, 1.25, -80.0), 1.0),
        (Vec3::new(-30.0, 1.25, 80.0), -1.0),
        (Vec3::new(80.0, 1.25, 80.0), 0.6),
        (Vec3::new(-80.0, 1.25, -80.0), -0.6),
    ];
    for (pos, rot) in crate_positions {
        commands.spawn((
            Mesh3d(crate_mesh.clone()),
            MeshMaterial3d(dark_concrete.clone()),
            Transform::from_translation(pos)
                .with_rotation(Quat::from_rotation_y(rot)),
            StaticCollider { half_extents: Vec3::new(1.25, 1.25, 2.5) },
            GameWorldEntity,
        ));
    }
}

/// No mode-specific entities for FFA (kills are the objective).
pub fn spawn_mode_entities(
    _commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {}
