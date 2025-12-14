use bevy::prelude::*;
use super::inventory::{Inventory, WeaponModel};
use crate::weapons::{WeaponSlot, WeaponRecoil, BaseWeaponTransform};
use std::collections::HashMap;

#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec3,
    pub timer: Timer,
}

#[derive(Component)]
pub struct MuzzleFlash {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Target;

#[derive(Component, Default)]
pub struct AmmoStatus {
    pub current_ammo: HashMap<WeaponSlot, u32>,
    pub reloading: Option<(WeaponSlot, Timer)>,
}

#[derive(Component)]
pub struct AmmoUi;

#[derive(Component)]
pub struct MeleeSwing {
    pub timer: Timer,
    pub start_rotation: Quat,
    pub target_rotation: Quat,
}

#[derive(Component)]
pub struct Grenade {
    pub velocity: Vec3,
    pub timer: Timer,
}

pub fn handle_muzzle_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut MuzzleFlash)>,
) {
    for (entity, mut flash) in query.iter_mut() {
        flash.timer.tick(time.delta());
        if flash.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn handle_weapon_recoil(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut WeaponRecoil, &BaseWeaponTransform)>,
) {
    for (mut transform, mut recoil, base) in query.iter_mut() {
        let dt = time.delta_secs();
        
        // Interpolate current towards target (kick)
        recoil.current_offset = recoil.current_offset.lerp(recoil.target_offset, dt * 20.0);
        recoil.current_rotation = recoil.current_rotation.lerp(recoil.target_rotation, dt * 20.0);
        
        // Decay target back to zero (recovery)
        recoil.target_offset = recoil.target_offset.lerp(Vec3::ZERO, dt * 10.0);
        recoil.target_rotation = recoil.target_rotation.lerp(Vec3::ZERO, dt * 10.0);
        
        // Apply to transform
        transform.translation = base.0.translation + recoil.current_offset;
        transform.rotation = base.0.rotation * Quat::from_euler(
            EulerRot::XYZ, 
            recoil.current_rotation.x, 
            recoil.current_rotation.y, 
            recoil.current_rotation.z
        );
    }
}

pub fn fire_weapon(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut inventory_query: Query<(&Inventory, &mut AmmoStatus)>,
    camera: Single<(&GlobalTransform, &mut Transform), With<Camera>>,
    mut weapon_query: Query<(Entity, &mut WeaponRecoil, &mut Transform), (With<WeaponModel>, Without<Camera>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut last_fire: Local<f32>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
) {
    let (inventory, mut ammo_status) = if let Ok(res) = inventory_query.single_mut() { res } else { return };
    
    // Handle Reloading
    let mut finished_reloading = false;
    if let Some((_, timer)) = &mut ammo_status.reloading {
        timer.tick(time.delta());
        if timer.is_finished() {
            finished_reloading = true;
        }
    }

    if finished_reloading {
        if let Some((slot, _)) = ammo_status.reloading.take() {
            if let Some(config) = weapon_registry.configs.get(&slot) {
                ammo_status.current_ammo.insert(slot, config.magazine_size);
            }
        }
    }

    if ammo_status.reloading.is_some() {
        return; // Can't shoot while reloading
    }

    // Manual Reload
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
            if config.reload_speed > 0.0 {
                ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(config.reload_speed, TimerMode::Once)));
                return;
            }
        }
    }

    let (fire_rate, speed, color, size, muzzle_offset) = if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
        (config.fire_rate, 40.0, Color::srgb(1.0, 0.8, 0.2), 0.05, config.muzzle_flash_offset)
    } else {
        match inventory.active_slot {
            WeaponSlot::Melee => (0.5, 0.0, Color::NONE, 0.0, None),
            WeaponSlot::Equipment => (1.0, 15.0, Color::srgb(0.2, 0.8, 0.2), 0.2, None),
            _ => (0.2, 30.0, Color::WHITE, 0.1, None),
        }
    };

    // Simple cooldown
    if *last_fire + fire_rate > time.elapsed_secs() {
        return;
    }

    if mouse_input.pressed(MouseButton::Left) {
        // Check Ammo for guns
        if matches!(inventory.active_slot, WeaponSlot::Primary | WeaponSlot::Secondary) {
            let current = *ammo_status.current_ammo.entry(inventory.active_slot).or_insert_with(|| {
                weapon_registry.configs.get(&inventory.active_slot).map(|c| c.magazine_size).unwrap_or(0)
            });
            
            if current == 0 {
                // Auto reload if empty
                if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
                    ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(config.reload_speed, TimerMode::Once)));
                }
                return;
            }
            
            ammo_status.current_ammo.insert(inventory.active_slot, current - 1);
        }

        *last_fire = time.elapsed_secs();

        let (global_transform, mut local_transform) = camera.into_inner();
        let transform = global_transform.compute_transform();
        let forward = transform.forward();
        let spawn_pos = transform.translation + forward * 1.0;

        match inventory.active_slot {
            WeaponSlot::Melee => {
                // Melee Swing Logic
                if let Some((weapon_entity, _, _)) = weapon_query.iter().next() {
                    commands.entity(weapon_entity).insert(MeleeSwing {
                        timer: Timer::from_seconds(0.2, TimerMode::Once),
                        start_rotation: Quat::IDENTITY, // Will be set in system
                        target_rotation: Quat::from_rotation_x(-1.0) * Quat::from_rotation_y(1.0),
                    });
                }
            },
            WeaponSlot::Equipment => {
                // Grenade Throw Logic
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(0.2))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.1, 0.5, 0.1),
                        ..default()
                    })),
                    Transform::from_translation(spawn_pos),
                    Grenade {
                        velocity: forward * 15.0 + Vec3::Y * 5.0, // Arc throw
                        timer: Timer::from_seconds(3.0, TimerMode::Once),
                    },
                ));
                
                // Animate hand/weapon throw
                if let Some((weapon_entity, mut recoil, _)) = weapon_query.iter_mut().next() {
                     recoil.target_rotation += Vec3::new(-1.0, 0.0, 0.0); // Throw motion
                }
            },
            _ => {
                // Gun Logic
                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(size))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        emissive: LinearRgba::from(color) * 5.0,
                        ..default()
                    })),
                    Transform::from_translation(spawn_pos),
                    Projectile {
                        velocity: forward * speed,
                        timer: Timer::from_seconds(3.0, TimerMode::Once),
                    },
                ));

                // Apply Camera Recoil
                let (yaw, pitch, roll) = local_transform.rotation.to_euler(EulerRot::YXZ);
                local_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch + 0.005, roll);

                // Apply Weapon Recoil & Muzzle Flash
                if let Some((weapon_entity, mut recoil, _)) = weapon_query.iter_mut().next() {
                    recoil.target_offset += Vec3::new(0.0, 0.0, 0.1); 
                    recoil.target_rotation += Vec3::new(0.1, 0.0, 0.0);

                    if let Some(offset) = muzzle_offset {
                        commands.entity(weapon_entity).with_children(|parent| {
                            parent.spawn((
                                PointLight {
                                    color: Color::srgb(1.0, 0.8, 0.2),
                                    intensity: 1000.0,
                                    range: 5.0,
                                    shadows_enabled: false,
                                    ..default()
                                },
                                Transform::from_translation(Vec3::from(offset)),
                                MuzzleFlash {
                                    timer: Timer::from_seconds(0.05, TimerMode::Once),
                                },
                            ));
                        });
                    }
                }
            }
        }
    }
}

pub fn handle_melee_swing(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut MeleeSwing, &BaseWeaponTransform)>,
) {
    for (entity, mut transform, mut swing, base) in query.iter_mut() {
        swing.timer.tick(time.delta());
        let t = swing.timer.fraction();
        
        // Simple swing animation: Rotate out and back
        let rotation = if t < 0.5 {
            Quat::IDENTITY.slerp(swing.target_rotation, t * 2.0)
        } else {
            swing.target_rotation.slerp(Quat::IDENTITY, (t - 0.5) * 2.0)
        };
        
        transform.rotation = base.0.rotation * rotation;

        if swing.timer.is_finished() {
            commands.entity(entity).remove::<MeleeSwing>();
            transform.rotation = base.0.rotation; // Reset
        }
    }
}

pub fn handle_grenade_throw(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Grenade)>,
) {
    for (entity, mut transform, mut grenade) in query.iter_mut() {
        grenade.timer.tick(time.delta());
        
        // Physics
        grenade.velocity.y -= 9.8 * time.delta_secs(); // Gravity
        transform.translation += grenade.velocity * time.delta_secs();
        
        // Floor collision
        if transform.translation.y < 0.0 {
            transform.translation.y = 0.0;
            grenade.velocity.y *= -0.5; // Bounce
            grenade.velocity.x *= 0.8; // Friction
            grenade.velocity.z *= 0.8;
        }

        if grenade.timer.is_finished() {
            // Explosion (placeholder)
            commands.entity(entity).despawn();
            println!("BOOM!"); 
        }
    }
}

pub fn update_ammo_ui(
    inventory_query: Query<(&Inventory, &AmmoStatus)>,
    mut text_query: Query<&mut Text, With<AmmoUi>>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
) {
    let (inventory, ammo_status) = if let Ok(res) = inventory_query.single() { res } else { return };
    let mut text = if let Ok(t) = text_query.single_mut() { t } else { return };

    if let Some((_, timer)) = &ammo_status.reloading {
        **text = format!("Reloading... {:.1}s", timer.remaining_secs());
    } else if matches!(inventory.active_slot, WeaponSlot::Primary | WeaponSlot::Secondary) {
        let current = ammo_status.current_ammo.get(&inventory.active_slot).copied().unwrap_or(0);
        let max = weapon_registry.configs.get(&inventory.active_slot).map(|c| c.magazine_size).unwrap_or(0);
        **text = format!("Ammo: {} / {}", current, max);
    } else {
        **text = "Ammo: --".to_string();
    }
}

pub fn reload_weapon() {} // Placeholder, logic moved to fire_weapon for now to share state access

pub fn move_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Projectile)>,
    targets: Query<(Entity, &Transform), (With<Target>, Without<Projectile>)>,
) {
    for (entity, mut transform, mut projectile) in query.iter_mut() {
        projectile.timer.tick(time.delta());
        if projectile.timer.just_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let delta = projectile.velocity * time.delta_secs();
        transform.translation += delta;

        // Simple collision check (distance based)
        for (target_entity, target_transform) in targets.iter() {
            if transform.translation.distance(target_transform.translation) < 1.5 {
                // Hit!
                commands.entity(target_entity).despawn(); // Destroy target
                commands.entity(entity).despawn(); // Destroy projectile
                break;
            }
        }
        
        // Floor collision
        if transform.translation.y < 0.0 {
             commands.entity(entity).despawn();
        }
    }
}
