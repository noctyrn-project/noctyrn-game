use bevy::prelude::*;
use rand::Rng;
use crate::player::shooting::Target;
use crate::gameplay::{Billboard, Health, Enemy, HealthBar, HealthBarForeground, Turret};
use crate::world::objects::*;
use crate::world::GameWorldEntity;
use crate::weapons::WeaponSlot;
use crate::menu::GameMode;

/// Spawn the full testing-grounds geometry.
/// Layout (looking from origin +Z):
///   -Z: shooting range with distance markers + targets
///   -X: parkour / movement course
///   +X: material penetration test area
///   Center: weapon terminals + spawn point
pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    spawn_weapon_terminals(commands, meshes, materials);
    spawn_shooting_range(commands, meshes, materials);
    spawn_parkour_course(commands, meshes, materials);
    spawn_material_test_area(commands, meshes, materials);
}

// ---------------------------------------------------------------------------
// Materials
// ---------------------------------------------------------------------------

fn concrete() -> StandardMaterial {
    StandardMaterial { base_color: Color::srgb(0.35, 0.35, 0.38), perceptual_roughness: 0.9, ..default() }
}
fn dark_concrete() -> StandardMaterial {
    StandardMaterial { base_color: Color::srgb(0.2, 0.2, 0.22), perceptual_roughness: 0.95, ..default() }
}
fn metal_mat() -> StandardMaterial {
    StandardMaterial { base_color: Color::srgb(0.4, 0.42, 0.45), perceptual_roughness: 0.3, metallic: 0.8, ..default() }
}
fn accent_blue() -> StandardMaterial {
    StandardMaterial { base_color: Color::srgb(0.25, 0.4, 0.6), perceptual_roughness: 0.6, metallic: 0.3, ..default() }
}
fn accent_orange() -> StandardMaterial {
    StandardMaterial { base_color: Color::srgb(0.6, 0.4, 0.2), perceptual_roughness: 0.7, metallic: 0.2, ..default() }
}

fn mat_handle(materials: &mut ResMut<Assets<StandardMaterial>>, m: StandardMaterial) -> Handle<StandardMaterial> {
    materials.add(m)
}

// ---------------------------------------------------------------------------
// Weapon terminals (compact row at spawn)
// ---------------------------------------------------------------------------

fn spawn_weapon_terminals(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let term = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.5, 0.7),
        perceptual_roughness: 0.3, metallic: 0.6, ..default()
    });
    let glow = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.7, 1.0),
        perceptual_roughness: 0.2, metallic: 0.8, ..default()
    });
    let labels = ["ALL", "PRIMARY", "SECONDARY", "MELEE", "EQUIP"];
    let slots = [None, Some(WeaponSlot::Primary), Some(WeaponSlot::Secondary), Some(WeaponSlot::Melee), Some(WeaponSlot::Equipment)];
    for (i, (label, slot)) in labels.iter().zip(slots.iter()).enumerate() {
        let x = (i as f32 - 2.0) * 3.0;
        let pos = Vec3::new(x, 1.0, 0.0);
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 1.8, 0.5))),
            MeshMaterial3d(term.clone()),
            Transform::from_translation(pos),
            StaticCollider { half_extents: Vec3::new(0.4, 0.9, 0.25) },
            WeaponTerminal { slot_filter: *slot },
            GameWorldEntity,
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.6, 0.1, 0.1))),
            MeshMaterial3d(glow.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 0.8, 0.3)),
            GameWorldEntity,
        ));
        commands.spawn((
            Text2d::new(*label),
            TextFont { font_size: FontSize::Px(24.0), ..default() },
            TextColor(Color::srgb(0.5, 0.7, 1.0)),
            Transform::from_translation(pos + Vec3::new(0.0, -1.0, 0.0)).with_scale(Vec3::splat(0.02)),
            Billboard, TerminalLabel,
        ));
    }
}

// ---------------------------------------------------------------------------
// Shooting range (extends in -Z direction from spawn)
// ---------------------------------------------------------------------------

fn spawn_shooting_range(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let origin = Vec3::new(0.0, 0.0, -10.0);
    let red = materials.add(StandardMaterial { base_color: Color::srgb(0.9, 0.2, 0.2), perceptual_roughness: 0.5, ..default() });
    let green = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.8, 0.2), perceptual_roughness: 0.7, ..default() });
    let olive = materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.6, 0.2), perceptual_roughness: 0.8, ..default() });
    let brown = materials.add(StandardMaterial { base_color: Color::srgb(0.8, 0.3, 0.3), perceptual_roughness: 0.7, ..default() });
    let wall = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.2, 0.22), perceptual_roughness: 0.95, ..default() });

    // Backstop wall at 55m
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(8.0, 6.0, 0.5))),
        MeshMaterial3d(wall.clone()),
        Transform::from_translation(origin + Vec3::new(0.0, 3.0, -55.0)),
        StaticCollider { half_extents: Vec3::new(4.0, 3.0, 0.25) },
    ));

    // Side walls for range lane
    for z in [-55.0, -5.0] {
        for (x, w) in [(-4.0, 0.5), (4.0, 0.5)] {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(w, 3.0, 0.5))),
                MeshMaterial3d(wall.clone()),
                Transform::from_translation(origin + Vec3::new(x, 1.5, z * 0.5 - 25.0)),
                StaticCollider { half_extents: Vec3::new(w * 0.5, 1.5, 0.25) },
            ));
        }
    }

    // Distance markers every 5m
    for i in 0..10 {
        let dist = (i as f32 + 1.0) * 5.0;
        let pos = origin + Vec3::new(0.0, 1.0, -dist);
        commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.1, 2.0))),
            MeshMaterial3d(red.clone()),
            Transform::from_translation(pos),
            DistanceMarker,
        ));
        commands.spawn((
            Text2d::new(&format!("{}m", dist as i32)),
            TextFont { font_size: FontSize::Px(32.0), ..default() },
            TextColor(Color::srgb(0.9, 0.2, 0.2)),
            Transform::from_translation(pos + Vec3::new(0.0, 1.5, 0.0)).with_scale(Vec3::splat(0.02)),
            Billboard,
        ));
    }

    // Static silhouette targets at 10, 20, 35, 50m
    for (z, label) in [(-10.0, "10m"), (-20.0, "20m"), (-35.0, "35m"), (-50.0, "50m")] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 1.6, 0.2))),
            MeshMaterial3d(olive.clone()),
            Transform::from_translation(origin + Vec3::new(2.5, 1.0, z)),
            Target,
        ));
        commands.spawn((
            Text2d::new(label),
            TextFont { font_size: FontSize::Px(24.0), ..default() },
            TextColor(Color::srgb(0.5, 0.8, 0.5)),
            Transform::from_translation(origin + Vec3::new(2.5, 2.2, z)).with_scale(Vec3::splat(0.015)),
            Billboard,
        ));
    }

    // Moving target at 25m
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 0.6, 0.3))),
        MeshMaterial3d(green.clone()),
        Transform::from_translation(origin + Vec3::new(0.0, 2.0, -25.0)),
        Target, MovingTarget {
            origin: origin + Vec3::new(0.0, 2.0, -25.0),
            axis: Vec3::new(1.0, 0.0, 0.0), amplitude: 2.5, speed: 1.5, phase: 0.0,
        },
    ));

    // Pop-up target at 40m
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.6, 0.8, 0.2))),
        MeshMaterial3d(brown),
        Transform::from_translation(origin + Vec3::new(-2.5, 0.4, -40.0)),
        Target, PopUpTarget { base_y: 0.4, raised_y: 2.0, timer: Timer::from_seconds(2.0, TimerMode::Repeating), is_up: false },
    ));
}

// ---------------------------------------------------------------------------
// Parkour / movement course (-X side)
// ---------------------------------------------------------------------------

fn spawn_parkour_course(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let origin = Vec3::new(-20.0, 0.0, -5.0);
    let plat = mat_handle(materials, accent_blue());
    let accent = mat_handle(materials, accent_orange());
    let beam_m = mat_handle(materials, metal_mat());

    // Stepping platforms at increasing heights along +Z
    for (i, (z, h)) in [(0.0, 0.5), (4.0, 1.0), (8.0, 1.5), (12.0, 2.0)].iter().enumerate() {
        let m = if i % 2 == 0 { &plat } else { &accent };
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.5, h * 2.0, 1.5))),
            MeshMaterial3d(m.clone()),
            Transform::from_translation(origin + Vec3::new(0.0, *h, *z)),
            StaticCollider { half_extents: Vec3::new(0.75, *h, 0.75) },
        ));
    }

    // Wall-run walls
    let wall = mat_handle(materials, concrete());
    for (x, z) in [(-4.0, 8.0), (-4.0, 12.0)] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.3, 4.0, 4.0))),
            MeshMaterial3d(wall.clone()),
            Transform::from_translation(origin + Vec3::new(x, 2.0, z)),
            StaticCollider { half_extents: Vec3::new(0.15, 2.0, 2.0) },
        ));
    }

    // Narrow balance beam
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.4, 0.2, 8.0))),
        MeshMaterial3d(beam_m.clone()),
        Transform::from_translation(origin + Vec3::new(1.0, 3.0, 10.0)),
        StaticCollider { half_extents: Vec3::new(0.2, 0.1, 4.0) },
    ));

    // Low wall to practice vaulting
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 1.0, 0.5))),
        MeshMaterial3d(accent.clone()),
        Transform::from_translation(origin + Vec3::new(-2.0, 0.5, 2.0)),
        StaticCollider { half_extents: Vec3::new(1.5, 0.5, 0.25) },
    ));
}

// ---------------------------------------------------------------------------
// Material penetration test area (+X side)
// ---------------------------------------------------------------------------

fn spawn_material_test_area(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    let origin = Vec3::new(10.0, 0.0, -20.0);
    let red = mat_handle(materials, StandardMaterial { base_color: Color::srgb(0.8, 0.2, 0.1), perceptual_roughness: 0.8, ..default() });

    let walls: [(&str, Vec3, MaterialType, (f32, f32, f32)); 5] = [
        ("DRYWALL",  Vec3::new(2.5, 2.5, 0.15),  MaterialType::Drywall, (0.85, 0.82, 0.78)),
        ("WOOD",     Vec3::new(2.5, 2.5, 0.3),   MaterialType::Wood,    (0.55, 0.4, 0.2)),
        ("GLASS",    Vec3::new(2.5, 2.5, 0.1),   MaterialType::Glass,   (0.7, 0.85, 0.95)),
        ("METAL",    Vec3::new(2.5, 2.5, 0.2),   MaterialType::Metal,   (0.5, 0.5, 0.55)),
        ("CONCRETE", Vec3::new(2.5, 2.5, 0.5),   MaterialType::Concrete,(0.35, 0.35, 0.38)),
    ];

    for (i, (name, size, mat_type, rgb)) in walls.iter().enumerate() {
        let x = i as f32 * 5.0;
        let color = Color::srgb(rgb.0, rgb.1, rgb.2);
        let m = if *name == "GLASS" {
            materials.add(StandardMaterial { base_color: Color::srgba(rgb.0, rgb.1, rgb.2, 0.3), alpha_mode: AlphaMode::Blend, metallic: 0.1, perceptual_roughness: 0.05, ..default() })
        } else {
            materials.add(StandardMaterial { base_color: color, perceptual_roughness: 0.9, ..default() })
        };
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(m),
            Transform::from_translation(origin + Vec3::new(x, size.y * 0.5, 0.0)),
            Target,
            StaticCollider { half_extents: Vec3::new(size.x * 0.5, size.y * 0.5, size.z * 0.5) },
            mat_type.clone(),
        ));
        commands.spawn((
            Text2d::new(*name),
            TextFont { font_size: FontSize::Px(28.0), ..default() },
            TextColor(color),
            Transform::from_translation(origin + Vec3::new(x, size.y + 0.5, 0.0)).with_scale(Vec3::splat(0.02)),
            Billboard,
        ));

        // Target behind each wall
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(0.8, 1.6, 0.2))),
            MeshMaterial3d(red.clone()),
            Transform::from_translation(origin + Vec3::new(x, 1.0, -3.0)),
            Target,
        ));
    }
}

/// Spawn test target dummies (as pill capsules) and a turret.
pub fn spawn_enemies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selected_mode: Res<crate::menu::SelectedGameMode>,
) {
    if selected_mode.mode != crate::menu::GameMode::TestingGrounds {
        return;
    }
    let healths = [1.0, 50.0, 100.0, 500.0];
    let start_x = -5.0;
    let spacing = 3.0;

    // Pill capsule mesh for dummies (same as remote player bodies)
    let pill = meshes.add(Capsule3d::new(0.3, 0.6));
    let dummy_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.2, 0.35),
        ..default()
    });
    let bg = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.0, 0.0), unlit: true, ..default() });
    let fg = materials.add(StandardMaterial { base_color: Color::srgb(0.0, 1.0, 0.0), unlit: true, ..default() });
    let bar = meshes.add(Rectangle::new(1.0, 0.15));

    for (i, &hp) in healths.iter().enumerate() {
        let pos = Vec3::new(start_x + i as f32 * spacing, 0.0, -10.0);

        let enemy = commands.spawn((
            Mesh3d(pill.clone()),
            MeshMaterial3d(dummy_mat.clone()),
            Transform::from_translation(pos + Vec3::new(0.0, 0.9, 0.0)),
            Visibility::default(),
            Enemy,
            Health { current: hp, max: hp },
        )).id();

        commands.entity(enemy).with_children(|parent| {
            parent.spawn((
                Transform::from_translation(Vec3::new(0.0, 2.2, 0.0)),
                HealthBar { target: enemy, offset: Vec3::new(0.0, 2.2, 0.0) },
                Billboard,
                Visibility::Inherited,
            )).with_children(|hb_parent| {
                hb_parent.spawn((
                    Mesh3d(bar.clone()),
                    MeshMaterial3d(bg.clone()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, -0.01)),
                ));
                hb_parent.spawn((
                    Mesh3d(bar.clone()),
                    MeshMaterial3d(fg.clone()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                    HealthBarForeground,
                ));
            });
        });
    }

    // Spawn Turret as a red cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.1, 0.1))),
        Transform::from_xyz(7.0, 0.5, -10.0).looking_at(Vec3::new(7.0, 0.5, 0.0), Vec3::Y),
        Visibility::default(),
        Turret { fire_timer: Timer::from_seconds(2.0, TimerMode::Repeating) },
        Enemy,
        Health { current: 100.0, max: 100.0 },
    ));
}
