use bevy::prelude::*;
use crate::player::GameState;
use crate::player::shooting::Projectile;
use bevy::ecs::relationship::Relationship;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_enemies, spawn_player_ui));
        app.add_systems(Update, (
            update_health_bars,
            update_player_health_ui,
            turret_fire,
            handle_death,
        ).run_if(in_state(GameState::Playing)));
        app.add_systems(Update, (
            check_player_death,
            spectate_camera,
            respawn_player,
            billboard_system,
        ).run_if(in_state(GameState::Playing)));
    }
}

#[derive(Resource)]
pub struct RespawnTimer(pub Timer);

#[derive(Component)]
pub struct SpectatorTarget;

#[derive(Component)]
pub struct Billboard;

fn billboard_system(
    mut query: Query<&mut Transform, With<Billboard>>,
    camera_query: Query<&GlobalTransform, With<Camera>>,
) {
    let camera_transform = if let Some(t) = camera_query.iter().next() { t } else { return };
    
    for mut transform in query.iter_mut() {
        transform.look_at(camera_transform.translation(), Vec3::Y);
    }
}

#[derive(Component)]
pub struct PlayerHealthUi;

#[derive(Component)]
pub struct DeathScreen;

fn spawn_player_ui(mut commands: Commands) {
     commands.spawn((
        Text::new("Health: 100"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::srgb(0.0, 1.0, 0.0)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            left: Val::Px(20.0),
            ..default()
        },
        PlayerHealthUi,
    ));

    // Death Screen (Red overlay)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::None, // Hidden by default
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 0.0, 0.0, 0.3)),
        DeathScreen,
    ));
}

fn update_player_health_ui(
    mut query: Query<&mut Text, With<PlayerHealthUi>>,
    mut death_screen_query: Query<&mut Node, With<DeathScreen>>,
    player_query: Query<&Health, With<PlayerBody>>,
) {
    let mut text = if let Ok(t) = query.single_mut() { t } else { return };
    let mut death_screen = if let Ok(d) = death_screen_query.single_mut() { d } else { return };
    
    if let Ok(health) = player_query.single() {
        text.0 = format!("Health: {}", health.current.ceil());
        
        if health.current <= 0.0 {
            death_screen.display = Display::Flex;
        } else {
            death_screen.display = Display::None;
        }
    }
}

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component)]
pub struct Enemy;

#[derive(Component)]
pub struct PlayerBody; // Tag for player to take damage

#[derive(Component)]
pub struct HealthBar {
    pub target: Entity,
    pub offset: Vec3,
}

#[derive(Component)]
pub struct Turret {
    pub fire_timer: Timer,
}

#[derive(Component)]
pub struct HealthBarForeground;

fn spawn_enemies(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let healths = [1.0, 50.0, 100.0, 500.0];
    let start_x = -5.0;
    let spacing = 3.0;

    // Create Health Bar Materials
    let bg_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.0, 0.0), // Dark Red
        unlit: true,
        ..default()
    });
    let fg_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 0.0), // Green
        unlit: true,
        ..default()
    });
    let bar_mesh = meshes.add(Rectangle::new(1.0, 0.15));

    for (i, &hp) in healths.iter().enumerate() {
        let pos = Vec3::new(start_x + i as f32 * spacing, 0.0, -10.0);
        
        let enemy = commands.spawn((
            SceneRoot(asset_server.load("characters/default.glb#Scene0")),
            Transform::from_translation(pos).with_scale(Vec3::splat(1.0)), // Adjust scale if needed
            Enemy,
            Health { current: hp, max: hp },
            SpectatorTarget,
        )).id();

        commands.entity(enemy).with_children(|parent| {
            parent.spawn((
                Transform::from_translation(Vec3::new(0.0, 2.2, 0.0)),
                HealthBar { target: enemy, offset: Vec3::new(0.0, 2.2, 0.0) },
                Billboard,
                Visibility::Inherited,
            )).with_children(|hb_parent| {
                // Background
                hb_parent.spawn((
                    Mesh3d(bar_mesh.clone()),
                    MeshMaterial3d(bg_material.clone()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, -0.01)), // Slightly behind
                ));
                // Foreground
                hb_parent.spawn((
                    Mesh3d(bar_mesh.clone()),
                    MeshMaterial3d(fg_material.clone()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                    HealthBarForeground,
                ));
            });
        });
    }

    // Spawn Turret
    let turret = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.1, 0.1))),
        Transform::from_xyz(7.0, 0.5, -10.0).looking_at(Vec3::new(7.0, 0.5, 0.0), Vec3::Y), // Alongside enemies, looking forward
        Turret {
            fire_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
        },
        Health { current: 200.0, max: 200.0 },
        Enemy,
        SpectatorTarget,
    )).id();

    commands.entity(turret).with_children(|parent| {
        parent.spawn((
            Transform::from_translation(Vec3::new(0.0, 1.5, 0.0)),
            HealthBar { target: turret, offset: Vec3::new(0.0, 1.5, 0.0) },
            Billboard,
            Visibility::Inherited,
        )).with_children(|hb_parent| {
            // Background
            hb_parent.spawn((
                Mesh3d(bar_mesh.clone()),
                MeshMaterial3d(bg_material.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, -0.01)),
            ));
            // Foreground
            hb_parent.spawn((
                Mesh3d(bar_mesh.clone()),
                MeshMaterial3d(fg_material.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                HealthBarForeground,
            ));
        });
    });
}

fn update_health_bars(
    mut query: Query<(&mut Transform, &ChildOf), With<HealthBarForeground>>,
    health_bar_query: Query<&ChildOf, With<HealthBar>>,
    health_query: Query<&Health>,
) {
    for (mut transform, parent) in query.iter_mut() {
        // parent is the HealthBar container
        if let Ok(grandparent) = health_bar_query.get(parent.get()) {
            // grandparent is the Enemy/Turret
            if let Ok(health) = health_query.get(grandparent.get()) {
                let percent = (health.current / health.max).clamp(0.0, 1.0);
                transform.scale.x = percent;
                // Anchor to left: Move x by (1.0 - percent) * width / 2.0 * -1.0 ?
                // Default quad is centered.
                // If scale is 0.5, it shrinks to center.
                // To anchor left, we need to shift it left by (1.0 - percent) * 0.5
                transform.translation.x = -0.5 * (1.0 - percent);
            }
        }
    }
}

fn turret_fire(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(&mut Turret, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut turret, transform) in query.iter_mut() {
        turret.fire_timer.tick(time.delta());
        if turret.fire_timer.just_finished() {
            let forward = transform.forward();
            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(0.2))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.0),
                    emissive: LinearRgba::RED * 5.0,
                    ..default()
                })),
                Transform::from_translation(transform.translation + forward * 1.0),
                Projectile {
                    velocity: forward * 20.0,
                    timer: Timer::from_seconds(5.0, TimerMode::Once),
                    damage: 25.0,
                    from_player: false,
                },
                TurretProjectile, // Tag to distinguish if needed, or just use Projectile
            ));
        }
    }
}

#[derive(Component)]
pub struct TurretProjectile;

fn handle_death(
    mut commands: Commands,
    query: Query<(Entity, &Health), Without<PlayerBody>>,
) {
    for (entity, health) in query.iter() {
        if health.current <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn check_player_death(
    mut commands: Commands,
    mut player_query: Query<(Entity, &Health), With<PlayerBody>>,
    timer: Option<Res<RespawnTimer>>,
) {
    if timer.is_some() { return; }

    if let Some((_entity, health)) = player_query.iter_mut().next() {
        if health.current <= 0.0 {
            commands.insert_resource(RespawnTimer(Timer::from_seconds(5.0, TimerMode::Once)));
        }
    }
}

fn spectate_camera(
    mut camera_query: Query<&mut Transform, With<Camera>>,
    targets: Query<&GlobalTransform, With<SpectatorTarget>>,
    time: Res<Time>,
    mut timer: Option<ResMut<RespawnTimer>>,
) {
    if let Some(timer) = timer.as_mut() {
        timer.0.tick(time.delta());
        
        if let Some(mut cam_transform) = camera_query.iter_mut().next() {
            // Find a target to spectate (just pick first for now)
            if let Some(target) = targets.iter().next() {
                let target_pos = target.translation();
                let target_look = target_pos + Vec3::Y * 1.0;
                let cam_pos = target_pos + Vec3::new(0.0, 5.0, 5.0);
                
                cam_transform.translation = cam_transform.translation.lerp(cam_pos, time.delta_secs() * 2.0);
                cam_transform.look_at(target_look, Vec3::Y);
            }
        }
    }
}

fn respawn_player(
    mut commands: Commands,
    mut query: Query<(&mut Health, &mut Transform), With<PlayerBody>>,
    timer: Option<Res<RespawnTimer>>,
) {
    if let Some(timer) = timer {
        if timer.0.is_finished() {
            if let Some((mut health, mut transform)) = query.iter_mut().next() {
                health.current = health.max;
                transform.translation = Vec3::new(0.0, 1.0, 0.0);
                commands.remove_resource::<RespawnTimer>();
            }
        }
    }
} 
