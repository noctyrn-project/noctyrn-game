//! Testing Grounds – the sandbox practice map.
//!
//! Contains all the test geometry: ramps, weapon terminals, shooting range,
//! parkour course, material test area, etc.

use bevy::prelude::*;
use rand::Rng;
use crate::player::shooting::Target;
use crate::gameplay::Billboard;
use crate::world::objects::*;
use crate::world::GameWorldEntity;

/// Spawn the full testing-grounds geometry.
pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_objects(commands, meshes, materials);
}

fn spawn_objects(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_geometry(commands, meshes, materials);
    spawn_weapon_terminals(commands, meshes, materials);
    spawn_shooting_range(commands, meshes, materials);
    spawn_parkour_course(commands, meshes, materials);
    spawn_material_test_area(commands, meshes, materials);
}

fn spawn_geometry(
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
        base_color: Color::srgb(0.2, 0.2, 0.22),
        perceptual_roughness: 0.95,
        ..default()
    });
    let metal = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.42, 0.45),
        perceptual_roughness: 0.3,
        metallic: 0.8,
        ..default()
    });

    // Ramps at various angles
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 10.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(-15.0, 0.8, -15.0)
            .with_rotation(Quat::from_rotation_x(-0.17)),
        Target,
        RampCollider { half_extents: Vec3::new(2.0, 0.5, 5.0) },
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 8.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(-9.0, 1.2, -15.0)
            .with_rotation(Quat::from_rotation_x(-0.35)),
        Target,
        RampCollider { half_extents: Vec3::new(2.0, 0.5, 4.0) },
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 6.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(-3.0, 1.5, -15.0)
            .with_rotation(Quat::from_rotation_x(-0.52)),
        Target,
        RampCollider { half_extents: Vec3::new(2.0, 0.5, 3.0) },
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 5.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(3.0, 1.8, -15.0)
            .with_rotation(Quat::from_rotation_x(-0.78)),
        Target,
        RampCollider { half_extents: Vec3::new(2.0, 0.5, 2.5) },
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 8.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(15.0, 1.0, -15.0)
            .with_rotation(Quat::from_rotation_x(-0.25)),
        Target,
        RampCollider { half_extents: Vec3::new(2.0, 0.5, 4.0) },
    ));

    // Ramp labels
    commands.spawn((
        Text2d::new("10°"),
        TextFont { font_size: FontSize::Px(36.0), ..default() },
        TextColor(Color::WHITE),
        Transform::from_xyz(-15.0, 1.2, -20.0).with_scale(Vec3::splat(0.015)),
        Billboard,
    ));
    commands.spawn((
        Text2d::new("20°"),
        TextFont { font_size: FontSize::Px(36.0), ..default() },
        TextColor(Color::WHITE),
        Transform::from_xyz(-9.0, 1.6, -20.0).with_scale(Vec3::splat(0.015)),
        Billboard,
    ));
    commands.spawn((
        Text2d::new("30°"),
        TextFont { font_size: FontSize::Px(36.0), ..default() },
        TextColor(Color::WHITE),
        Transform::from_xyz(-3.0, 2.0, -20.0).with_scale(Vec3::splat(0.015)),
        Billboard,
    ));
    commands.spawn((
        Text2d::new("45°"),
        TextFont { font_size: FontSize::Px(36.0), ..default() },
        TextColor(Color::WHITE),
        Transform::from_xyz(3.0, 2.3, -20.0).with_scale(Vec3::splat(0.015)),
        Billboard,
    ));
    commands.spawn((
        Text2d::new("SIDE"),
        TextFont { font_size: FontSize::Px(36.0), ..default() },
        TextColor(Color::WHITE),
        Transform::from_xyz(15.0, 1.5, -20.0).with_scale(Vec3::splat(0.015)),
        Billboard,
    ));

    // Low wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(6.0, 1.0, 0.5))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(0.0, 0.5, -5.0),
        Target,
        StaticCollider { half_extents: Vec3::new(3.0, 0.5, 0.25) },
    ));

    // Tall wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(6.0, 2.5, 0.5))),
        MeshMaterial3d(dark_concrete.clone()),
        Transform::from_xyz(0.0, 1.25, 5.0),
        Target,
        StaticCollider { half_extents: Vec3::new(3.0, 1.25, 0.25) },
    ));

    // L-shaped wall
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 2.0, 0.5))),
        MeshMaterial3d(metal.clone()),
        Transform::from_xyz(8.0, 1.0, 3.0)
            .with_rotation(Quat::from_rotation_y(0.3)),
        Target,
        StaticCollider { half_extents: Vec3::new(1.5, 1.0, 0.25) },
    ));

    // Elevated platform
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 0.3, 4.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(-5.0, 1.5, 5.0),
        Target,
        StaticCollider { half_extents: Vec3::new(2.0, 0.15, 2.0) },
    ));

    // Pillars
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.35),
        perceptual_roughness: 0.6,
        metallic: 0.5,
        ..default()
    });
    for (px, pz) in [(-5.0, -5.0), (5.0, 5.0), (-5.0, 5.0)] {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.4, 4.0))),
            MeshMaterial3d(pillar_mat.clone()),
            Transform::from_xyz(px, 2.0, pz),
            StaticCollider { half_extents: Vec3::new(0.4, 2.0, 0.4) },
        ));
    }

    // Shipping container
    let container_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.3, 0.4),
        perceptual_roughness: 0.4,
        metallic: 0.7,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(6.0, 3.0, 3.0))),
        MeshMaterial3d(container_mat),
        Transform::from_xyz(10.0, 1.5, -5.0),
        StaticCollider { half_extents: Vec3::new(3.0, 1.5, 1.5) },
    ));

    // Stacked sandbags
    let sandbag_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.4, 0.25),
        perceptual_roughness: 1.0,
        ..default()
    });
    for (i, y_off) in [0.0, 0.5, 1.0].iter().enumerate() {
        let x_off = if i % 2 == 0 { 0.0 } else { 0.25 };
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 0.4, 0.6))),
            MeshMaterial3d(sandbag_mat.clone()),
            Transform::from_xyz(-10.0 + x_off, *y_off + 0.2, 0.0),
            Target,
            StaticCollider { half_extents: Vec3::new(0.4, 0.2, 0.3) },
        ));
    }

    // Archway
    let arch_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        perceptual_roughness: 0.9,
        ..default()
    });
    // Left pillar
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 3.0, 0.5))),
        MeshMaterial3d(arch_mat.clone()),
        Transform::from_xyz(-12.0, 1.5, 8.0),
        StaticCollider { half_extents: Vec3::new(0.25, 1.5, 0.25) },
    ));
    // Right pillar
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 3.0, 0.5))),
        MeshMaterial3d(arch_mat.clone()),
        Transform::from_xyz(-8.0, 1.5, 8.0),
        StaticCollider { half_extents: Vec3::new(0.25, 1.5, 0.25) },
    ));
    // Cross beam
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 0.3, 0.5))),
        MeshMaterial3d(arch_mat.clone()),
        Transform::from_xyz(-10.0, 3.15, 8.0),
        StaticCollider { half_extents: Vec3::new(2.0, 0.15, 0.25) },
    ));

    // Circular platform
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(2.0, 0.3))),
        MeshMaterial3d(dark_concrete.clone()),
        Transform::from_xyz(5.0, 0.15, 8.0),
        StaticCollider { half_extents: Vec3::new(2.0, 0.15, 2.0) },
    ));
}

fn spawn_weapon_terminals(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let terminal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.5, 0.7),
        perceptual_roughness: 0.3,
        metallic: 0.6,
        ..default()
    });
    let terminal_highlight = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.7, 1.0),
        perceptual_roughness: 0.2,
        metallic: 0.8,
        ..default()
    });

    let terminal_positions = [
        (Vec3::new(-15.0, 1.0, 0.0), None),
        (Vec3::new(-10.0, 1.0, 0.0), Some(crate::weapons::WeaponSlot::Primary)),
        (Vec3::new(-5.0, 1.0, 0.0), Some(crate::weapons::WeaponSlot::Secondary)),
        (Vec3::new(0.0, 1.0, 0.0), Some(crate::weapons::WeaponSlot::Melee)),
        (Vec3::new(5.0, 1.0, 0.0), Some(crate::weapons::WeaponSlot::Equipment)),
    ];

    for (pos, slot_filter) in &terminal_positions {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 1.8, 0.5))),
            MeshMaterial3d(terminal_mat.clone()),
            Transform::from_translation(*pos),
            StaticCollider { half_extents: Vec3::new(0.4, 0.9, 0.25) },
            WeaponTerminal { slot_filter: *slot_filter },
            GameWorldEntity,
        ));
        // Glow strip
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.6, 0.1, 0.1))),
            MeshMaterial3d(terminal_highlight.clone()),
            Transform::from_translation(*pos + Vec3::new(0.0, 0.8, 0.3)),
            GameWorldEntity,
        ));
        // Label
        let label = match slot_filter {
            None => "ALL",
            Some(crate::weapons::WeaponSlot::Primary) => "PRIMARY",
            Some(crate::weapons::WeaponSlot::Secondary) => "SECONDARY",
            Some(crate::weapons::WeaponSlot::Melee) => "MELEE",
            Some(crate::weapons::WeaponSlot::Equipment) => "EQUIPMENT",
        };
        commands.spawn((
            Text2d::new(label),
            TextFont { font_size: FontSize::Px(24.0), ..default() },
            TextColor(Color::srgb(0.5, 0.7, 1.0)),
            Transform::from_translation(*pos + Vec3::new(0.0, -1.0, 0.0))
                .with_scale(Vec3::splat(0.02)),
            Billboard,
            TerminalLabel,
        ));
    }
}

fn spawn_shooting_range(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let range_origin = Vec3::new(-20.0, 0.0, -10.0);

    // Distance markers (5m increments)
    let marker_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.2, 0.2),
        perceptual_roughness: 0.5,
        ..default()
    });
    for i in 0..8 {
        let dist = (i as f32 + 1.0) * 5.0;
        let pos = range_origin + Vec3::new(dist, 1.0, 0.0);
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.1, 2.0))),
            MeshMaterial3d(marker_mat.clone()),
            Transform::from_translation(pos),
            DistanceMarker,
        ));
        commands.spawn((
            Text2d::new(&format!("{}m", dist as i32)),
            TextFont { font_size: FontSize::Px(32.0), ..default() },
            TextColor(Color::srgb(0.9, 0.2, 0.2)),
            Transform::from_translation(pos + Vec3::new(0.0, 1.5, 0.0))
                .with_scale(Vec3::splat(0.02)),
            Billboard,
        ));
    }

    // Moving target (slides left-right)
    let moving_target_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.8, 0.2),
        perceptual_roughness: 0.7,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 0.6, 0.3))),
        MeshMaterial3d(moving_target_mat.clone()),
        Transform::from_translation(range_origin + Vec3::new(20.0, 2.0, 0.0)),
        Target,
        MovingTarget {
            origin: range_origin + Vec3::new(20.0, 2.0, 0.0),
            axis: Vec3::new(0.0, 0.0, 1.0),
            amplitude: 3.0,
            speed: 1.5,
            phase: 0.0,
        },
    ));

    // Pop-up target
    let popup_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.3, 0.3),
        perceptual_roughness: 0.7,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 0.8, 0.2))),
        MeshMaterial3d(popup_mat),
        Transform::from_translation(range_origin + Vec3::new(30.0, 0.4, 0.0)),
        Target,
        PopUpTarget {
            base_y: 0.4,
            raised_y: 2.0,
            timer: Timer::from_seconds(2.0, TimerMode::Repeating),
            is_up: false,
        },
    ));

    // Static silhouette targets at various ranges
    let target_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.6, 0.2),
        perceptual_roughness: 0.8,
        ..default()
    });
    for (x_off, label) in [(10.0, "10m"), (20.0, "20m"), (35.0, "35m"), (50.0, "50m")] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 1.6, 0.2))),
            MeshMaterial3d(target_mat.clone()),
            Transform::from_translation(range_origin + Vec3::new(x_off, 1.0, 3.0)),
            Target,
        ));
        commands.spawn((
            Text2d::new(label),
            TextFont { font_size: FontSize::Px(24.0), ..default() },
            TextColor(Color::srgb(0.5, 0.8, 0.5)),
            Transform::from_translation(range_origin + Vec3::new(x_off, 2.2, 3.0))
                .with_scale(Vec3::splat(0.015)),
            Billboard,
        ));
    }
}

fn spawn_parkour_course(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let course_origin = Vec3::new(20.0, 0.0, 5.0);
    let plat_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.4, 0.6),
        perceptual_roughness: 0.6,
        metallic: 0.3,
        ..default()
    });
    let accent_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.4, 0.2),
        perceptual_roughness: 0.7,
        metallic: 0.2,
        ..default()
    });

    // Series of platforms at increasing heights
    let platforms = [
        (Vec3::new(0.0, 0.5, 0.0), Vec3::new(2.0, 1.0, 2.0)),
        (Vec3::new(3.0, 1.0, 0.0), Vec3::new(1.5, 2.0, 1.5)),
        (Vec3::new(6.0, 1.5, 0.0), Vec3::new(1.0, 3.0, 1.0)),
        (Vec3::new(9.0, 2.0, 0.0), Vec3::new(1.0, 4.0, 1.0)),
    ];

    for (i, (offset, size)) in platforms.iter().enumerate() {
        let pos = course_origin + offset;
        let he = size * 0.5;
        let mat = if i % 2 == 0 { plat_mat.clone() } else { accent_mat.clone() };

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(mat),
            Transform::from_translation(pos),
            StaticCollider { half_extents: he },
        ));
    }

    // Wall-run walls
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.35),
        perceptual_roughness: 0.7,
        metallic: 0.4,
        ..default()
    });
    for (offset, size) in [
        (Vec3::new(10.0, 2.0, -2.0), Vec3::new(0.3, 4.0, 4.0)),
        (Vec3::new(13.0, 2.0, -2.0), Vec3::new(0.3, 4.0, 4.0)),
    ] {
        let pos = course_origin + offset;
        let he = size * 0.5;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(pos),
            StaticCollider { half_extents: he },
        ));
    }

    // Narrow beam bridge
    let beam_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.5, 0.1),
        perceptual_roughness: 0.5,
        metallic: 0.6,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 0.2, 8.0))),
        MeshMaterial3d(beam_mat),
        Transform::from_translation(course_origin + Vec3::new(-6.0, 3.0, 1.5)),
        StaticCollider { half_extents: Vec3::new(0.2, 0.1, 4.0) },
    ));

    // Supports for beam
    for z_off in [-3.5, 3.5] {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.15, 3.0))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_translation(course_origin + Vec3::new(-6.0, 1.5, 1.5 + z_off)),
        ));
    }
}

fn spawn_material_test_area(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let area_origin = Vec3::new(-20.0, 0.0, 20.0);

    commands.spawn((
        Text2d::new("PENETRATION TEST"),
        TextFont { font_size: FontSize::Px(48.0), ..default() },
        TextColor(Color::srgb(0.9, 0.6, 0.1)),
        Transform::from_translation(area_origin + Vec3::new(5.0, 4.0, 0.0))
            .with_scale(Vec3::splat(0.025)),
        Billboard,
    ));

    // Wood wall
    let wood_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.4, 0.2),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 2.5, 0.3))),
        MeshMaterial3d(wood_mat.clone()),
        Transform::from_translation(area_origin + Vec3::new(0.0, 1.25, 0.0)),
        Target,
        StaticCollider { half_extents: Vec3::new(1.5, 1.25, 0.15) },
        MaterialType::Wood,
    ));
    commands.spawn((
        Text2d::new("WOOD"),
        TextFont { font_size: FontSize::Px(28.0), ..default() },
        TextColor(Color::srgb(0.55, 0.4, 0.2)),
        Transform::from_translation(area_origin + Vec3::new(0.0, 3.0, 0.0))
            .with_scale(Vec3::splat(0.02)),
        Billboard,
    ));

    // Glass wall
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.85, 0.95, 0.3),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.1,
        perceptual_roughness: 0.05,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 2.5, 0.1))),
        MeshMaterial3d(glass_mat),
        Transform::from_translation(area_origin + Vec3::new(4.0, 1.25, 0.0)),
        Target,
        StaticCollider { half_extents: Vec3::new(1.5, 1.25, 0.05) },
        MaterialType::Glass,
    ));
    commands.spawn((
        Text2d::new("GLASS"),
        TextFont { font_size: FontSize::Px(28.0), ..default() },
        TextColor(Color::srgb(0.5, 0.7, 0.9)),
        Transform::from_translation(area_origin + Vec3::new(4.0, 3.0, 0.0))
            .with_scale(Vec3::splat(0.02)),
        Billboard,
    ));

    // Metal wall
    let metal_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.5, 0.55),
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 2.5, 0.2))),
        MeshMaterial3d(metal_mat),
        Transform::from_translation(area_origin + Vec3::new(8.0, 1.25, 0.0)),
        Target,
        StaticCollider { half_extents: Vec3::new(1.5, 1.25, 0.1) },
        MaterialType::Metal,
    ));
    commands.spawn((
        Text2d::new("METAL"),
        TextFont { font_size: FontSize::Px(28.0), ..default() },
        TextColor(Color::srgb(0.6, 0.6, 0.65)),
        Transform::from_translation(area_origin + Vec3::new(8.0, 3.0, 0.0))
            .with_scale(Vec3::splat(0.02)),
        Billboard,
    ));

    // Drywall
    let drywall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.85, 0.82, 0.78),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 2.5, 0.15))),
        MeshMaterial3d(drywall_mat),
        Transform::from_translation(area_origin + Vec3::new(12.0, 1.25, 0.0)),
        Target,
        StaticCollider { half_extents: Vec3::new(1.5, 1.25, 0.075) },
        MaterialType::Drywall,
    ));
    commands.spawn((
        Text2d::new("DRYWALL"),
        TextFont { font_size: FontSize::Px(28.0), ..default() },
        TextColor(Color::srgb(0.8, 0.78, 0.74)),
        Transform::from_translation(area_origin + Vec3::new(12.0, 3.0, 0.0))
            .with_scale(Vec3::splat(0.02)),
        Billboard,
    ));

    // Concrete wall
    let concrete_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 2.5, 0.5))),
        MeshMaterial3d(concrete_mat),
        Transform::from_translation(area_origin + Vec3::new(16.0, 1.25, 0.0)),
        Target,
        StaticCollider { half_extents: Vec3::new(1.5, 1.25, 0.25) },
        MaterialType::Concrete,
    ));
    commands.spawn((
        Text2d::new("CONCRETE"),
        TextFont { font_size: FontSize::Px(28.0), ..default() },
        TextColor(Color::srgb(0.5, 0.5, 0.53)),
        Transform::from_translation(area_origin + Vec3::new(16.0, 3.0, 0.0))
            .with_scale(Vec3::splat(0.02)),
        Billboard,
    ));

    // Targets behind each wall for testing penetration
    let target_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.2, 0.1),
        perceptual_roughness: 0.8,
        ..default()
    });
    for x_offset in [0.0, 4.0, 8.0, 12.0, 16.0] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 1.6, 0.2))),
            MeshMaterial3d(target_mat.clone()),
            Transform::from_translation(area_origin + Vec3::new(x_offset, 1.0, -3.0)),
            Target,
        ));
    }
}
