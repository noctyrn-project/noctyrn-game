use bevy::prelude::*;
use crate::player::shooting::Target;
use crate::gameplay::{Billboard, Health, Enemy, HealthBar, HealthBarForeground};

/// Spawn the test target dummies (pill capsules) and an armed dummy.
/// The map geometry itself is the GLB map `testing_grounds.glb` (loaded by
/// the `maps` module) — nothing procedural spawns here anymore.
pub fn spawn_enemies(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selected_mode: Res<crate::menu::SelectedGameMode>,
    asset_server: Res<AssetServer>,
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

    // Armed dummy: holds an HK416 and shoots at the local player using the
    // gun's real stats (see armed_target_fire). Fires straight ahead (no
    // tracking), so its bullets cross the spawn line and the whole lane.
    let armed_pos = Vec3::new(0.0, 0.0, -10.0);
    let armed_dummy = commands.spawn((
        Mesh3d(pill.clone()),
        MeshMaterial3d(dummy_mat.clone()),
        Transform::from_translation(armed_pos + Vec3::new(0.0, 0.9, 0.0)),
        Visibility::default(),
        Enemy,
        Target { armed: true },
        crate::gameplay::ArmedTargetTimer(Timer::from_seconds(2.0, TimerMode::Repeating)),
        Health { current: 150.0, max: 150.0 },
    )).id();

    commands.entity(armed_dummy).with_children(|parent| {
        // Holds a real HK416 (same model as the player's weapon), facing
        // forward at the dummy's right side.
        parent.spawn((
            WorldAssetRoot(asset_server.load("weapons/models/primary/assault/hk416.glb#Scene0")),
            Transform::from_xyz(0.4, 0.8, 0.35)
                .with_rotation(Quat::from_rotation_y(1.57))
                .with_scale(Vec3::splat(0.2)),
        ));
        parent.spawn((
            Transform::from_translation(Vec3::new(0.0, 2.2, 0.0)),
            HealthBar { target: armed_dummy, offset: Vec3::new(0.0, 2.2, 0.0) },
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
