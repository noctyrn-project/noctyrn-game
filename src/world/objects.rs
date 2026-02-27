use bevy::prelude::*;
use crate::player::shooting::Target;
use crate::gameplay::Billboard;
use rand::Rng;

/// Axis-aligned bounding box for simple collision detection.
#[derive(Component, Clone, Debug)]
pub struct StaticCollider {
    pub half_extents: Vec3,
}

/// Ramp collider for inclined surfaces.
/// The ramp is defined by its transform (position + rotation) and half-extents in local space.
/// The collision logic projects the player onto the ramp surface.
#[derive(Component, Clone, Debug)]
pub struct RampCollider {
    pub half_extents: Vec3,
}

/// Material type for bullet penetration/collision behavior
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum MaterialType {
    Concrete,
    Metal,
    Wood,
    Glass,
    Drywall,
}

impl MaterialType {
    /// Returns the penetration resistance (0.0 = no resistance, 1.0 = impenetrable)
    pub fn resistance(&self) -> f32 {
        match self {
            MaterialType::Concrete => 0.85,
            MaterialType::Metal => 0.95,
            MaterialType::Wood => 0.4,
            MaterialType::Glass => 0.1,
            MaterialType::Drywall => 0.2,
        }
    }

    /// Returns the damage multiplier after penetrating this material
    pub fn damage_falloff(&self) -> f32 {
        match self {
            MaterialType::Concrete => 0.2,
            MaterialType::Metal => 0.1,
            MaterialType::Wood => 0.6,
            MaterialType::Glass => 0.9,
            MaterialType::Drywall => 0.75,
        }
    }
    
    /// Whether this material shatters on bullet impact
    pub fn shatters(&self) -> bool {
        matches!(self, MaterialType::Glass)
    }
}

/// Component for glass shatter particles
#[derive(Component)]
pub struct GlassShard {
    pub velocity: Vec3,
    pub timer: Timer,
    pub angular_velocity: Vec3,
}

#[derive(Component)]
pub struct WeaponTerminal {
    pub slot_filter: Option<crate::weapons::WeaponSlot>,
}

#[derive(Component)]
pub struct TerminalLabel;

/// Moving target that slides back and forth along an axis.
#[derive(Component)]
pub struct MovingTarget {
    pub origin: Vec3,
    pub axis: Vec3,
    pub amplitude: f32,
    pub speed: f32,
    pub phase: f32,
}

/// Pop-up target that raises and lowers.
#[derive(Component)]
pub struct PopUpTarget {
    pub base_y: f32,
    pub raised_y: f32,
    pub timer: Timer,
    pub is_up: bool,
}

/// Distance marker label.
#[derive(Component)]
pub struct DistanceMarker;

pub fn spawn_objects(
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

    // ── Ramps at various angles ──
    // Gentle ramp (10°)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 10.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(-15.0, 0.8, -15.0)
            .with_rotation(Quat::from_rotation_x(-0.17)),
        Target,
        RampCollider { half_extents: Vec3::new(2.0, 0.5, 5.0) },
    ));
    // Medium ramp (20°)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 8.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(-9.0, 1.2, -15.0)
            .with_rotation(Quat::from_rotation_x(-0.35)),
        Target,
        RampCollider { half_extents: Vec3::new(2.0, 0.5, 4.0) },
    ));
    // Steep ramp (30°)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 6.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(-3.0, 1.5, -15.0)
            .with_rotation(Quat::from_rotation_x(-0.52)),
        Target,
        RampCollider { half_extents: Vec3::new(2.0, 0.5, 3.0) },
    ));
    // Very steep ramp (45°)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 1.0, 5.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(3.0, 1.8, -15.0)
            .with_rotation(Quat::from_rotation_x(-0.78)),
        Target,
        RampCollider { half_extents: Vec3::new(2.0, 0.5, 2.5) },
    ));
    // Side-angled ramp (Z-axis rotation)
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
        Text2d::new("RAMP TEST AREA"),
        TextFont { font_size: 48.0, ..default() },
        TextColor(Color::srgb(0.3, 0.8, 0.3)),
        Transform::from_translation(Vec3::new(-6.0, 4.0, -15.0))
            .with_scale(Vec3::splat(0.025)),
        Billboard,
    ));
    for (label, x) in [("10°", -15.0), ("20°", -9.0), ("30°", -3.0), ("45°", 3.0)] {
        commands.spawn((
            Text2d::new(label),
            TextFont { font_size: 28.0, ..default() },
            TextColor(Color::srgb(0.5, 0.8, 0.5)),
            Transform::from_translation(Vec3::new(x, 3.0, -15.0))
                .with_scale(Vec3::splat(0.02)),
            Billboard,
        ));
    }

    // ── Walls / Cover ──
    // Low wall (half cover)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(6.0, 1.5, 0.5))),
        MeshMaterial3d(dark_concrete.clone()),
        Transform::from_xyz(0.0, 0.75, -8.0),
        Target,
        StaticCollider { half_extents: Vec3::new(3.0, 0.75, 0.25) },
        MaterialType::Concrete,
    ));
    // Tall wall (full cover)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 3.0, 4.0))),
        MeshMaterial3d(dark_concrete.clone()),
        Transform::from_xyz(-8.0, 1.5, -5.0),
        Target,
        StaticCollider { half_extents: Vec3::new(0.25, 1.5, 2.0) },
        MaterialType::Concrete,
    ));
    // L-shaped corner
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 2.5, 0.5))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(10.0, 1.25, 5.0),
        Target,
        StaticCollider { half_extents: Vec3::new(2.0, 1.25, 0.25) },
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 2.5, 4.0))),
        MeshMaterial3d(concrete.clone()),
        Transform::from_xyz(12.0, 1.25, 7.0),
        Target,
        StaticCollider { half_extents: Vec3::new(0.25, 1.25, 2.0) },
    ));

    // ── Platforms ──
    // Elevated platform
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(6.0, 0.4, 6.0))),
        MeshMaterial3d(metal.clone()),
        Transform::from_xyz(-20.0, 2.0, 0.0),
        Target,
        StaticCollider { half_extents: Vec3::new(3.0, 0.2, 3.0) },
    ));
    // Platform supports
    for (dx, dz) in [(-2.5, -2.5), (2.5, -2.5), (-2.5, 2.5), (2.5, 2.5)] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.3, 2.0, 0.3))),
            MeshMaterial3d(metal.clone()),
            Transform::from_xyz(-20.0 + dx, 1.0, 0.0 + dz),
        ));
    }

    // ── Pillar cluster ──
    for (x, z) in [(5.0, -20.0), (8.0, -22.0), (3.0, -23.0)] {
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.5, 4.0))),
            MeshMaterial3d(concrete.clone()),
            Transform::from_xyz(x, 2.0, z),
            Target,
        ));
    }

    // ── Central structure ──
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(4.0, 10.0, 4.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.2, 0.8))),
        Transform::from_xyz(10.0, 5.0, 10.0),
        Target,
        StaticCollider { half_extents: Vec3::new(2.0, 5.0, 2.0) },
    ));

    // ── Crates (various sizes) ──
    let crate_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.35, 0.2),
        perceptual_roughness: 0.85,
        ..default()
    });
    for (x, z, size) in [
        (0.0, -15.0, 1.0),
        (1.2, -15.0, 0.7),
        (0.0, -15.8, 0.8),
        (-12.0, -12.0, 1.2),
        (-11.0, -12.0, 0.9),
        (18.0, -5.0, 1.0),
        (18.0, -3.5, 1.0),
    ] {
        let half = size / 2.0;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size, size, size))),
            MeshMaterial3d(crate_mat.clone()),
            Transform::from_xyz(x, size / 2.0, z),
            Target,
            StaticCollider { half_extents: Vec3::new(half, half, half) },
        ));
    }

    // ── Sandbag walls ──
    let sandbag_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.45, 0.3),
        perceptual_roughness: 1.0,
        ..default()
    });
    // Arc of sandbags
    for i in 0..5 {
        let angle = -0.4 + i as f32 * 0.2;
        let x = 25.0 + angle.cos() * 2.0;
        let z = -15.0 + angle.sin() * 8.0;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.0, 0.8, 0.6))),
            MeshMaterial3d(sandbag_mat.clone()),
            Transform::from_xyz(x, 0.4, z)
                .with_rotation(Quat::from_rotation_y(angle)),
            Target,
        ));
    }

    // ── Archway ──
    let arch_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.3, 0.35),
        perceptual_roughness: 0.7,
        metallic: 0.3,
        ..default()
    });
    // Left pillar
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 5.0, 0.8))),
        MeshMaterial3d(arch_mat.clone()),
        Transform::from_xyz(-25.0, 2.5, -10.0),
        Target,
        StaticCollider { half_extents: Vec3::new(0.4, 2.5, 0.4) },
    ));
    // Right pillar
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.8, 5.0, 0.8))),
        MeshMaterial3d(arch_mat.clone()),
        Transform::from_xyz(-25.0, 2.5, -6.0),
        Target,
        StaticCollider { half_extents: Vec3::new(0.4, 2.5, 0.4) },
    ));
    // Top beam
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 0.5, 5.0))),
        MeshMaterial3d(arch_mat.clone()),
        Transform::from_xyz(-25.0, 5.0, -8.0),
        Target,
    ));
}

fn spawn_weapon_terminals(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    use crate::weapons::WeaponSlot;
    
    let terminal_mesh = meshes.add(Cuboid::new(1.2, 1.6, 0.6));
    let screen_mesh = meshes.add(Cuboid::new(0.9, 0.6, 0.05));
    let base_mesh = meshes.add(Cylinder::new(0.8, 0.1));

    let terminals = [
        ("ALL WEAPONS", None, Vec3::new(5.0, 0.0, 5.0), Color::srgb(0.2, 0.5, 0.7)),
        ("PRIMARY", Some(WeaponSlot::Primary), Vec3::new(8.0, 0.0, 5.0), Color::srgb(0.6, 0.3, 0.1)),
        ("SECONDARY", Some(WeaponSlot::Secondary), Vec3::new(11.0, 0.0, 5.0), Color::srgb(0.5, 0.5, 0.1)),
        ("MELEE", Some(WeaponSlot::Melee), Vec3::new(14.0, 0.0, 5.0), Color::srgb(0.1, 0.5, 0.3)),
        ("EQUIPMENT", Some(WeaponSlot::Equipment), Vec3::new(17.0, 0.0, 5.0), Color::srgb(0.5, 0.1, 0.1)),
    ];

    let body_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.15, 0.18),
        perceptual_roughness: 0.5,
        metallic: 0.7,
        ..default()
    });

    for (label, slot_filter, pos, screen_color) in terminals {
        let screen_mat = materials.add(StandardMaterial {
            base_color: screen_color,
            emissive: LinearRgba::from(screen_color) * 3.0,
            unlit: true,
            ..default()
        });

        // Base plate
        commands.spawn((
            Mesh3d(base_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 0.05, 0.0)),
        ));

        // Terminal body
        commands.spawn((
            Mesh3d(terminal_mesh.clone()),
            MeshMaterial3d(body_mat.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 0.9, 0.0)),
            WeaponTerminal { slot_filter },
            Target,
            StaticCollider { half_extents: Vec3::new(0.6, 0.8, 0.3) },
        ));

        // Screen
        commands.spawn((
            Mesh3d(screen_mesh.clone()),
            MeshMaterial3d(screen_mat),
            Transform::from_translation(pos + Vec3::new(0.0, 1.1, 0.33)),
        ));

        // Floating text label above terminal (Billboard)
        commands.spawn((
            Text2d::new(label),
            TextFont { font_size: 36.0, ..default() },
            TextColor(screen_color),
            Transform::from_translation(pos + Vec3::new(0.0, 2.4, 0.0))
                .with_scale(Vec3::splat(0.02)),
            Billboard,
            TerminalLabel,
        ));
        
        // "SHOOT TO OPEN" sub-label
        commands.spawn((
            Text2d::new("[ SHOOT TO OPEN ]"),
            TextFont { font_size: 22.0, ..default() },
            TextColor(Color::srgba(0.7, 0.7, 0.7, 0.6)),
            Transform::from_translation(pos + Vec3::new(0.0, 2.1, 0.0))
                .with_scale(Vec3::splat(0.015)),
            Billboard,
            TerminalLabel,
        ));

        // Light indicator
        commands.spawn((
            PointLight {
                color: screen_color,
                intensity: 800.0,
                range: 5.0,
                shadows_enabled: false,
                ..default()
            },
            Transform::from_translation(pos + Vec3::new(0.0, 2.5, 0.5)),
        ));
    }
}

// ── Shooting Range with Distance Markers & Moving/Pop-up Targets ──

fn spawn_shooting_range(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let range_origin = Vec3::new(-30.0, 0.0, -20.0);
    let range_dir = Vec3::new(0.0, 0.0, -1.0); // Firing direction

    // Lane dividers
    let divider_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.25, 0.25, 0.28),
        perceptual_roughness: 0.9,
        ..default()
    });
    for lane in 0..3 {
        let x_offset = lane as f32 * 5.0;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.1, 1.5, 30.0))),
            MeshMaterial3d(divider_mat.clone()),
            Transform::from_translation(range_origin + Vec3::new(x_offset - 2.5, 0.75, -15.0)),
        ));
    }

    // Distance markers at 10m, 20m, 30m, 40m
    for dist in [10.0, 20.0, 30.0, 40.0_f32] {
        let marker_pos = range_origin + range_dir * dist;

        // Floor line
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(12.0, 0.02, 0.1))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.6, 0.1),
                emissive: LinearRgba::new(0.8, 0.6, 0.1, 1.0) * 2.0,
                unlit: true,
                ..default()
            })),
            Transform::from_translation(marker_pos + Vec3::new(0.0, 0.01, 0.0)),
            DistanceMarker,
        ));

        // Distance label
        commands.spawn((
            Text2d::new(format!("{}m", dist as i32)),
            TextFont { font_size: 28.0, ..default() },
            TextColor(Color::srgb(0.9, 0.7, 0.2)),
            Transform::from_translation(marker_pos + Vec3::new(6.5, 1.5, 0.0))
                .with_scale(Vec3::splat(0.02)),
            Billboard,
            DistanceMarker,
        ));
    }

    // Static targets at various distances
    let target_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.2, 0.1),
        perceptual_roughness: 0.8,
        ..default()
    });
    for (lane, dist) in [(0.0, 10.0), (5.0, 15.0), (0.0, 25.0), (5.0, 30.0), (0.0, 40.0)] {
        let pos = range_origin + Vec3::new(lane, 1.0, 0.0) + range_dir * dist;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 1.6, 0.2))),
            MeshMaterial3d(target_mat.clone()),
            Transform::from_translation(pos),
            Target,
        ));
    }

    // Moving targets (slide left-right)
    let moving_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.6, 0.8),
        perceptual_roughness: 0.5,
        metallic: 0.3,
        ..default()
    });
    for (dist, speed, amplitude) in [(15.0, 1.5, 3.0), (25.0, 2.5, 4.0), (35.0, 3.5, 5.0)] {
        let pos = range_origin + range_dir * dist + Vec3::new(2.5, 1.0, 0.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.6, 1.4, 0.15))),
            MeshMaterial3d(moving_mat.clone()),
            Transform::from_translation(pos),
            Target,
            MovingTarget {
                origin: pos,
                axis: Vec3::X,
                amplitude,
                speed,
                phase: dist, // Different starting phases
            },
        ));
    }

    // Pop-up targets
    let popup_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.1),
        perceptual_roughness: 0.6,
        ..default()
    });
    for (lane, dist, interval) in [(0.0, 20.0, 3.0), (5.0, 20.0, 4.5), (2.5, 35.0, 2.0)] {
        let pos = range_origin + Vec3::new(lane, 0.0, 0.0) + range_dir * dist;
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.7, 1.5, 0.15))),
            MeshMaterial3d(popup_mat.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, -0.5, 0.0)),
            Target,
            PopUpTarget {
                base_y: pos.y - 0.5,
                raised_y: pos.y + 1.0,
                timer: Timer::from_seconds(interval, TimerMode::Repeating),
                is_up: false,
            },
        ));
    }

    // "SHOOTING RANGE" sign
    commands.spawn((
        Text2d::new("SHOOTING RANGE"),
        TextFont { font_size: 48.0, ..default() },
        TextColor(Color::srgb(0.9, 0.3, 0.1)),
        Transform::from_translation(range_origin + Vec3::new(2.5, 3.5, 1.0))
            .with_scale(Vec3::splat(0.025)),
        Billboard,
    ));
}

// ── Parkour / Movement Course ──

fn spawn_parkour_course(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let course_origin = Vec3::new(25.0, 0.0, 15.0);

    let plat_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.4, 0.5),
        perceptual_roughness: 0.4,
        metallic: 0.5,
        ..default()
    });
    let accent_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.15, 0.4),
        perceptual_roughness: 0.4,
        metallic: 0.5,
        ..default()
    });

    // "PARKOUR COURSE" sign
    commands.spawn((
        Text2d::new("PARKOUR COURSE"),
        TextFont { font_size: 48.0, ..default() },
        TextColor(Color::srgb(0.2, 0.7, 0.9)),
        Transform::from_translation(course_origin + Vec3::new(0.0, 4.0, 0.0))
            .with_scale(Vec3::splat(0.025)),
        Billboard,
    ));

    // Stepping platforms at increasing heights
    let platforms = [
        (Vec3::new(0.0, 0.5, 0.0), Vec3::new(3.0, 0.3, 3.0)),
        (Vec3::new(4.0, 1.0, 0.0), Vec3::new(2.5, 0.3, 2.5)),
        (Vec3::new(7.0, 1.8, 0.0), Vec3::new(2.0, 0.3, 2.0)),
        (Vec3::new(7.0, 2.8, 3.0), Vec3::new(2.0, 0.3, 2.0)),
        (Vec3::new(4.0, 3.8, 3.0), Vec3::new(2.5, 0.3, 2.5)),
        (Vec3::new(0.0, 4.5, 3.0), Vec3::new(3.0, 0.3, 3.0)),
        (Vec3::new(-3.0, 5.5, 1.5), Vec3::new(2.0, 0.3, 2.0)),
        (Vec3::new(-3.0, 6.5, -1.5), Vec3::new(3.0, 0.3, 3.0)),
    ];

    for (i, (offset, size)) in platforms.iter().enumerate() {
        let pos = course_origin + *offset;
        let he = *size * 0.5;
        let mat = if i % 2 == 0 { plat_mat.clone() } else { accent_mat.clone() };

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(mat),
            Transform::from_translation(pos),
            StaticCollider { half_extents: he },
        ));
    }

    // Wall-run walls (tall thin walls to jump between)
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

/// System to update moving targets (call from World plugin)
pub fn update_moving_targets(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &MovingTarget)>,
) {
    for (mut transform, target) in query.iter_mut() {
        let offset = (time.elapsed_secs() * target.speed + target.phase).sin() * target.amplitude;
        transform.translation = target.origin + target.axis * offset;
    }
}

/// System to update pop-up targets
pub fn update_popup_targets(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut PopUpTarget)>,
) {
    for (mut transform, mut popup) in query.iter_mut() {
        popup.timer.tick(time.delta());
        if popup.timer.just_finished() {
            popup.is_up = !popup.is_up;
        }

        let target_y = if popup.is_up { popup.raised_y } else { popup.base_y };
        transform.translation.y = transform.translation.y
            + (target_y - transform.translation.y) * time.delta_secs() * 8.0;
    }
}

/// System to update glass shards
pub fn update_glass_shards(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut GlassShard)>,
) {
    for (entity, mut transform, mut shard) in query.iter_mut() {
        shard.timer.tick(time.delta());
        if shard.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let dt = time.delta_secs();
        shard.velocity.y -= 9.8 * dt; // Gravity
        transform.translation += shard.velocity * dt;
        shard.velocity *= 0.98; // Drag

        // Spin
        let rot = Quat::from_euler(
            EulerRot::XYZ,
            shard.angular_velocity.x * dt,
            shard.angular_velocity.y * dt,
            shard.angular_velocity.z * dt,
        );
        transform.rotation *= rot;

        // Floor collision
        if transform.translation.y < 0.05 {
            transform.translation.y = 0.05;
            shard.velocity.y *= -0.3;
            shard.velocity.x *= 0.5;
            shard.velocity.z *= 0.5;
        }
    }
}

/// Spawn glass shatter effect at a position
pub fn spawn_glass_shatter(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    bullet_dir: Vec3,
) {
    let mut rng = rand::rng();
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.85, 0.95, 0.5),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.1,
        perceptual_roughness: 0.1,
        ..default()
    });

    for _ in 0..12 {
        // Random shard shapes
        let sx = rng.random_range(0.03..0.12);
        let sy = rng.random_range(0.03..0.12);
        let sz = rng.random_range(0.005..0.02);

        let spread = Vec3::new(
            rng.random_range(-2.0..2.0),
            rng.random_range(-1.0..3.0),
            rng.random_range(-2.0..2.0),
        );
        let vel = bullet_dir * rng.random_range(1.0..4.0) + spread;

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(sx, sy, sz))),
            MeshMaterial3d(glass_mat.clone()),
            Transform::from_translation(position)
                .with_rotation(Quat::from_euler(
                    EulerRot::XYZ,
                    rng.random_range(0.0..std::f32::consts::TAU),
                    rng.random_range(0.0..std::f32::consts::TAU),
                    rng.random_range(0.0..std::f32::consts::TAU),
                )),
            GlassShard {
                velocity: vel,
                timer: Timer::from_seconds(rng.random_range(1.5..3.0), TimerMode::Once),
                angular_velocity: Vec3::new(
                    rng.random_range(-10.0..10.0),
                    rng.random_range(-10.0..10.0),
                    rng.random_range(-10.0..10.0),
                ),
            },
        ));
    }
}

// ── Material Test Area ──

fn spawn_material_test_area(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    #[allow(unused_imports)]
    use rand::Rng;
    let area_origin = Vec3::new(-20.0, 0.0, 20.0);

    // "PENETRATION TEST" sign
    commands.spawn((
        Text2d::new("PENETRATION TEST"),
        TextFont { font_size: 48.0, ..default() },
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
        TextFont { font_size: 28.0, ..default() },
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
        TextFont { font_size: 28.0, ..default() },
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
        TextFont { font_size: 28.0, ..default() },
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
        TextFont { font_size: 28.0, ..default() },
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
        TextFont { font_size: 28.0, ..default() },
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
