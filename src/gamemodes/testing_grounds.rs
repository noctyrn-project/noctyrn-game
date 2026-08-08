use bevy::prelude::*;
use crate::player::shooting::Target;
use crate::gameplay::{
    ArmedTargetTimer, ArmedWeapon, Billboard, Health, Enemy, HealthBar, HealthBarForeground,
    KnifeSwinging, KnifeVisual,
};
use crate::weapons::WeaponRegistry;

/// Spawn the test target dummies along the x=-15 lane, all facing down-range
/// (+X — a 90° yaw from the default +Z facing): four plain health dummies
/// (25..500 HP) and five armed dummies (rifle, pistol, sniper, shotgun, and
/// a knife dummy that constantly swings its blade).
/// The map geometry itself is the GLB map `testing_grounds.glb` (loaded by
/// the `maps` module) — nothing procedural spawns here anymore.
pub fn spawn_enemies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selected_mode: Res<crate::menu::SelectedGameMode>,
    asset_server: Res<AssetServer>,
    weapon_registry: Res<WeaponRegistry>,
) {
    if selected_mode.mode != crate::menu::GameMode::TestingGrounds {
        return;
    }

    // Pill capsule mesh for dummies (same as remote player bodies).
    let pill = meshes.add(Capsule3d::new(0.3, 0.6));
    let dummy_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.2, 0.35),
        ..default()
    });
    let bg = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.0, 0.0), unlit: true, ..default() });
    let fg = materials.add(StandardMaterial { base_color: Color::srgb(0.0, 1.0, 0.0), unlit: true, ..default() });
    let bar = meshes.add(Rectangle::new(1.0, 0.15));

    // Facing down-range: 90° yaw (default +Z → +X).
    let facing = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

    // Normal health dummies — HP 25..500, the back row.
    for (hp, z) in [(25.0, 14.0), (100.0, 11.0), (250.0, 8.0), (500.0, 5.0)] {
        spawn_plain_dummy(
            &mut commands, &pill, &dummy_mat, &bg, &fg, &bar,
            Vec3::new(-15.0, 0.9, z), facing, hp,
        );
    }

    // Armed dummies — rifle, pistol, sniper, shotgun.
    for (weapon_id, z) in [
        ("hk416", 2.0),
        ("g17", -1.0),
        ("remington_700", -4.0),
        ("franchi_spas_12", -7.0),
    ] {
        spawn_armed_dummy(
            &mut commands, &pill, &dummy_mat, &bg, &fg, &bar,
            Vec3::new(-15.0, 0.9, z), facing, weapon_id, &asset_server, &weapon_registry,
        );
    }

    // Knife dummy — swings its blade constantly.
    spawn_knife_dummy(
        &mut commands, &pill, &dummy_mat, &bg, &fg, &bar,
        Vec3::new(-15.0, 0.9, -10.0), facing, "km2000", &asset_server, &weapon_registry,
    );
}

/// Spawn a plain health dummy with a health bar above it.
fn spawn_plain_dummy(
    commands: &mut Commands,
    pill: &Handle<Mesh>,
    dummy_mat: &Handle<StandardMaterial>,
    bg: &Handle<StandardMaterial>,
    fg: &Handle<StandardMaterial>,
    bar: &Handle<Mesh>,
    pos: Vec3,
    facing: Quat,
    hp: f32,
) {
    let dummy = commands
        .spawn((
            Mesh3d(pill.clone()),
            MeshMaterial3d(dummy_mat.clone()),
            Transform::from_translation(pos).with_rotation(facing),
            Visibility::default(),
            Enemy,
            Health { current: hp, max: hp },
        ))
        .id();
    spawn_health_bar(commands, dummy, bg, fg, bar);
}

/// Spawn a shooting dummy holding the given weapon (fires with its stats).
fn spawn_armed_dummy(
    commands: &mut Commands,
    pill: &Handle<Mesh>,
    dummy_mat: &Handle<StandardMaterial>,
    bg: &Handle<StandardMaterial>,
    fg: &Handle<StandardMaterial>,
    bar: &Handle<Mesh>,
    pos: Vec3,
    facing: Quat,
    weapon_id: &str,
    asset_server: &AssetServer,
    weapon_registry: &WeaponRegistry,
) {
    let dummy = commands
        .spawn((
            Mesh3d(pill.clone()),
            MeshMaterial3d(dummy_mat.clone()),
            Transform::from_translation(pos).with_rotation(facing),
            Visibility::default(),
            Enemy,
            Target { armed: true },
            ArmedTargetTimer(Timer::from_seconds(2.0, TimerMode::Repeating)),
            ArmedWeapon { weapon_id: weapon_id.to_string() },
            Health { current: 150.0, max: 150.0 },
        ))
        .id();
    spawn_health_bar(commands, dummy, bg, fg, bar);
    spawn_weapon_child(commands, dummy, weapon_id, asset_server, weapon_registry);
}

/// Spawn a dummy that constantly swings a melee weapon (no shooting).
fn spawn_knife_dummy(
    commands: &mut Commands,
    pill: &Handle<Mesh>,
    dummy_mat: &Handle<StandardMaterial>,
    bg: &Handle<StandardMaterial>,
    fg: &Handle<StandardMaterial>,
    bar: &Handle<Mesh>,
    pos: Vec3,
    facing: Quat,
    weapon_id: &str,
    asset_server: &AssetServer,
    weapon_registry: &WeaponRegistry,
) {
    let dummy = commands
        .spawn((
            Mesh3d(pill.clone()),
            MeshMaterial3d(dummy_mat.clone()),
            Transform::from_translation(pos).with_rotation(facing),
            Visibility::default(),
            Enemy,
            Target { armed: false },
            KnifeSwinging {
                timer: Timer::from_seconds(1.6, TimerMode::Repeating),
            },
            Health { current: 150.0, max: 150.0 },
        ))
        .id();
    spawn_health_bar(commands, dummy, bg, fg, bar);

    let Some(config) = weapon_registry.weapons.get(weapon_id) else {
        return;
    };
    let model = config.meta.model_path.clone();
    let scale = config.meta.scale;
    commands.entity(dummy).with_children(|parent| {
        parent.spawn((
            WorldAssetRoot(asset_server.load(&model)),
            Transform::from_xyz(0.4, 0.8, 0.35)
                .with_rotation(Quat::from_rotation_y(1.57))
                .with_scale(Vec3::splat(scale)),
            KnifeVisual { base_yaw: 1.57 },
        ));
    });
}

/// Attach the dummy's health bar (billboarded bg + fg bars).
fn spawn_health_bar(
    commands: &mut Commands,
    dummy: Entity,
    bg: &Handle<StandardMaterial>,
    fg: &Handle<StandardMaterial>,
    bar: &Handle<Mesh>,
) {
    commands.entity(dummy).with_children(|parent| {
        parent.spawn((
            Transform::from_translation(Vec3::new(0.0, 2.2, 0.0)),
            HealthBar { target: dummy, offset: Vec3::new(0.0, 2.2, 0.0) },
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

/// Attach a weapon model to a dummy at the chest-right-front, oriented like
/// the player's viewmodel (rotation_y(1.57)) so its muzzle points along the
/// dummy's forward (+X down-range for a 90°-yawed dummy).
fn spawn_weapon_child(
    commands: &mut Commands,
    dummy: Entity,
    weapon_id: &str,
    asset_server: &AssetServer,
    weapon_registry: &WeaponRegistry,
) {
    let Some(config) = weapon_registry.weapons.get(weapon_id) else {
        return;
    };
    let model = config.meta.model_path.clone();
    let scale = config.meta.scale;
    commands.entity(dummy).with_children(|parent| {
        parent.spawn((
            WorldAssetRoot(asset_server.load(&model)),
            Transform::from_xyz(0.4, 0.8, 0.35)
                .with_rotation(Quat::from_rotation_y(1.57))
                .with_scale(Vec3::splat(scale)),
        ));
    });
}
