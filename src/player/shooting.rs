use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use super::inventory::{Inventory, WeaponModel};
use super::movement::Velocity;
use crate::weapons::{WeaponSlot, WeaponRecoil, BaseWeaponTransform, FireMode};
use std::collections::HashMap;
use rand::Rng;

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
    pub reserve_ammo: HashMap<WeaponSlot, u32>,
    pub current_fire_mode: HashMap<WeaponSlot, usize>, // Index into config.fire_modes
    pub reloading: Option<(WeaponSlot, Timer)>,
    pub burst_count: u32, // Shots remaining in current burst
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
        
        // Apply to transform (Recoil + Sway + Aim)
        transform.translation = base.0.translation + recoil.current_offset + recoil.sway_offset + recoil.aim_offset;
        
        let recoil_rot = Quat::from_euler(
            EulerRot::XYZ, 
            recoil.current_rotation.x, 
            recoil.current_rotation.y, 
            recoil.current_rotation.z
        );
        
        let sway_rot = Quat::from_euler(
            EulerRot::XYZ, 
            recoil.sway_rotation.x, 
            recoil.sway_rotation.y, 
            recoil.sway_rotation.z
        );

        transform.rotation = base.0.rotation * recoil_rot * sway_rot;
    }
}

pub fn handle_weapon_sway(
    time: Res<Time>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut query: Query<&mut WeaponRecoil, With<WeaponModel>>,
    player_velocity: Single<&Velocity>,
    inventory_query: Query<&Inventory>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
) {
    let velocity = player_velocity.into_inner();
    let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
    let dt = time.delta_secs();
    
    // 1. Movement Sway (Bobbing)
    // Clamp speed for frequency calculation to avoid super fast jitter
    let freq_speed = speed.min(8.0); 
    let (sway_amount, sway_speed) = if speed > 0.1 { 
        (0.01, freq_speed * 0.5) // Reduced amount and speed multiplier
    } else { 
        (0.002, 1.0) // Idle
    };
    
    // 2. Look Sway (Lag)
    let mouse_delta = accumulated_mouse_motion.delta;
    let target_lag_x = -mouse_delta.x * 0.002; // Adjust sensitivity
    let target_lag_y = mouse_delta.y * 0.002;

    // 3. Sprint Pose
    let is_sprinting = keyboard_input.pressed(KeyCode::ShiftLeft);
    // Only sprint if moving forward (positive Z in local space, but here we check velocity relative to camera forward roughly)
    // Actually, we can just check if speed is high enough, assuming the input logic handles direction.
    // But the user said "if the sprint key is held while the player is moving backwards... the gun will still play the sprinting animation".
    // So we need to check if we are actually sprinting.
    // The input system sets `sprint` only if moving forward. But here we use raw input.
    // Let's use the velocity dot product with camera forward if possible, or just trust the speed + input if we had the input struct.
    // Since we don't have the input struct here easily without querying, let's rely on the fact that we updated input.rs to only set sprint true if moving forward.
    // Wait, we are reading raw keyboard input here. We should probably read the AccumulatedInput component if we want to respect the logic in input.rs.
    // But we removed it. Let's re-add it or duplicate the logic.
    
    // Duplicate logic: Check if W is pressed.
    let moving_forward = keyboard_input.pressed(KeyCode::KeyW);
    
    let (sprint_pos, sprint_rot) = if is_sprinting && moving_forward && speed > 0.1 {
        (Vec3::new(0.1, -0.1, -0.1), Vec3::new(-0.4, 0.8, 0.0)) // Example sprint pose
    } else {
        (Vec3::ZERO, Vec3::ZERO)
    };

    // 4. Aiming
    let is_aiming = mouse_input.pressed(MouseButton::Right) && !is_sprinting;
    let inventory = inventory_query.iter().next(); // Use iter().next() for safety if single() is weird
    
    let mut target_aim_offset = Vec3::ZERO;
    if let Some(inv) = inventory {
        if let Some(config) = weapon_registry.configs.get(&inv.active_slot) {
            if is_aiming {
                if let Some(offset) = config.aim_offset {
                    target_aim_offset = Vec3::from(offset);
                }
            }
        }
    }

    for mut recoil in query.iter_mut() {
        // Update Phase
        recoil.sway_phase += dt * sway_speed;
        
        let bob_x = recoil.sway_phase.sin() * sway_amount;
        let bob_y = (recoil.sway_phase * 2.0).cos().abs() * sway_amount;

        // Target Sway (Bobbing + Sprint)
        // Disable sway if aiming
        let sway_mult = if is_aiming { 0.1 } else { 1.0 };
        
        let target_sway_pos = (Vec3::new(bob_x, bob_y, 0.0) + sprint_pos) * sway_mult;
        
        // Target Rotation (Lag + Sprint)
        let target_sway_rot = (Vec3::new(target_lag_y, target_lag_x, 0.0) + sprint_rot) * sway_mult;
        
        // Smoothly interpolate
        recoil.sway_offset = recoil.sway_offset.lerp(target_sway_pos, dt * 10.0);
        recoil.sway_rotation = recoil.sway_rotation.lerp(target_sway_rot, dt * 5.0);
        recoil.aim_offset = recoil.aim_offset.lerp(target_aim_offset, dt * 15.0);
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
    
    // Prevent firing while sprinting
    if keyboard_input.pressed(KeyCode::ShiftLeft) {
        return;
    }
    
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
                let current = *ammo_status.current_ammo.get(&slot).unwrap_or(&0);
                let reserve = *ammo_status.reserve_ammo.get(&slot).unwrap_or(&config.max_ammo); // Default to max if not set
                
                let needed = config.magazine_size.saturating_sub(current);
                let available = reserve.min(needed);
                
                ammo_status.current_ammo.insert(slot, current + available);
                ammo_status.reserve_ammo.insert(slot, reserve - available);
            }
        }
    }

    if ammo_status.reloading.is_some() {
        return; // Can't shoot while reloading
    }

    // Manual Reload
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
            let current = *ammo_status.current_ammo.get(&inventory.active_slot).unwrap_or(&0);
            let reserve = *ammo_status.reserve_ammo.get(&inventory.active_slot).unwrap_or(&config.max_ammo);
            
            if current < config.magazine_size && reserve > 0 && config.reload_speed > 0.0 {
                ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(config.reload_speed, TimerMode::Once)));
                return;
            }
        }
    }

    // Switch Fire Mode
    if keyboard_input.just_pressed(KeyCode::KeyV) {
        if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
            if !config.fire_modes.is_empty() {
                let current_idx = *ammo_status.current_fire_mode.get(&inventory.active_slot).unwrap_or(&0);
                let next_idx = (current_idx + 1) % config.fire_modes.len();
                ammo_status.current_fire_mode.insert(inventory.active_slot, next_idx);
            }
        }
    }

    let (fire_rate, speed, color, size, muzzle_offset, recoil_factor, fire_mode) = if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
        let mode_idx = *ammo_status.current_fire_mode.get(&inventory.active_slot).unwrap_or(&0);
        let mode = config.fire_modes.get(mode_idx).copied().unwrap_or(FireMode::Auto);
        (config.fire_rate, 40.0, Color::srgb(1.0, 0.8, 0.2), 0.05, config.muzzle_flash_offset, config.recoil_factor, mode)
    } else {
        match inventory.active_slot {
            WeaponSlot::Melee => (0.5, 0.0, Color::NONE, 0.0, None, 0.0, FireMode::Semi),
            WeaponSlot::Equipment => (1.0, 15.0, Color::srgb(0.2, 0.8, 0.2), 0.2, None, 0.0, FireMode::Semi),
            _ => (0.2, 30.0, Color::WHITE, 0.1, None, 0.1, FireMode::Auto),
        }
    };

    // Simple cooldown
    if *last_fire + fire_rate > time.elapsed_secs() {
        return;
    }

    let mut should_fire = false;
    
    // Burst Logic
    if ammo_status.burst_count > 0 {
        should_fire = true;
    } else {
        match fire_mode {
            FireMode::Auto => {
                if mouse_input.pressed(MouseButton::Left) {
                    should_fire = true;
                }
            },
            FireMode::Semi => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    should_fire = true;
                }
            },
            FireMode::Burst(count) => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    ammo_status.burst_count = count;
                    should_fire = true;
                }
            }
        }
    }

    if should_fire {
        // Check Ammo for guns
        if matches!(inventory.active_slot, WeaponSlot::Primary | WeaponSlot::Secondary) {
            let current = *ammo_status.current_ammo.entry(inventory.active_slot).or_insert_with(|| {
                weapon_registry.configs.get(&inventory.active_slot).map(|c| c.magazine_size).unwrap_or(0)
            });
            
            if current == 0 {
                // Auto reload if empty
                if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
                    let reserve = *ammo_status.reserve_ammo.get(&inventory.active_slot).unwrap_or(&config.max_ammo);
                    if reserve > 0 {
                        ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(config.reload_speed, TimerMode::Once)));
                    }
                }
                ammo_status.burst_count = 0; // Cancel burst
                return;
            }
            
            ammo_status.current_ammo.insert(inventory.active_slot, current - 1);
        }
        
        if ammo_status.burst_count > 0 {
            ammo_status.burst_count -= 1;
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
                if let Some((_weapon_entity, mut recoil, _)) = weapon_query.iter_mut().next() {
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
                    let mut rng = rand::rng();
                    let rand_x = rng.random_range(-0.05..0.05) * recoil_factor;
                    let rand_y = rng.random_range(0.05..0.1) * recoil_factor;
                    let rand_rot_x = rng.random_range(0.05..0.15) * recoil_factor;
                    let rand_rot_y = rng.random_range(-0.05..0.05) * recoil_factor;

                    recoil.target_offset += Vec3::new(rand_x, rand_y, 0.1); 
                    recoil.target_rotation += Vec3::new(rand_rot_x, rand_rot_y, 0.0);

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
            swing.start_rotation.slerp(swing.target_rotation, t * 2.0)
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
        let reserve = ammo_status.reserve_ammo.get(&inventory.active_slot).copied().unwrap_or(0);
        
        if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
            let mode_idx = *ammo_status.current_fire_mode.get(&inventory.active_slot).unwrap_or(&0);
            let mode = config.fire_modes.get(mode_idx).copied().unwrap_or(FireMode::Auto);
            let mode_str = match mode {
                FireMode::Auto => "AUTO",
                FireMode::Semi => "SEMI",
                FireMode::Burst(_) => "BURST",
            };
            
            **text = format!("{} | {}\n{} | {}", current, reserve, config.ammo_type, mode_str);
        } else {
             **text = format!("{} | {}", current, reserve);
        }
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
