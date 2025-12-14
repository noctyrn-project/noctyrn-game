use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use super::inventory::{Inventory, WeaponModel};
use super::movement::Velocity;
use crate::weapons::{WeaponSlot, WeaponRecoil, BaseWeaponTransform, FireMode};
use crate::gameplay::{Health, PlayerBody, Enemy, Regenerating};
use std::collections::HashMap;
use rand::Rng;

#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec3,
    pub timer: Timer,
    pub damage: f32,
    pub from_player: bool,
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
}

#[derive(Component)]
pub struct Grenade {
    pub velocity: Vec3,
    pub timer: Timer,
}

#[derive(Component)]
pub struct ExplosionParticle {
    pub velocity: Vec3,
    pub timer: Timer,
    pub max_time: f32,
    pub start_scale: f32,
    pub end_scale: f32,
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
        
        // Apply to transform (Recoil + Sway + Aim + Switch)
        transform.translation = base.0.translation + recoil.current_offset + recoil.sway_offset + recoil.aim_offset + recoil.switch_offset;
        
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

        let switch_rot = Quat::from_euler(
            EulerRot::XYZ, 
            recoil.switch_rotation.x, 
            recoil.switch_rotation.y, 
            recoil.switch_rotation.z
        );

        let melee_rot = Quat::from_euler(
            EulerRot::XYZ, 
            recoil.melee_rotation.x, 
            recoil.melee_rotation.y, 
            recoil.melee_rotation.z
        );

        transform.rotation = base.0.rotation * recoil_rot * sway_rot * switch_rot * melee_rot;
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
    camera_query: Query<&GlobalTransform, With<Camera>>,
) {
    let velocity = player_velocity.into_inner();
    let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
    let dt = time.delta_secs();
    
    // 1. Movement Sway (Bobbing)
    // Clamp speed for frequency calculation to avoid super fast jitter
    let freq_speed = speed.min(8.0); 
    let (sway_amount, sway_speed) = if speed > 0.1 { 
        (0.01, freq_speed * 0.5)
    } else { 
        (0.002, 1.0) // Idle
    };
    
    // 2. Look Sway (Lag)
    let mouse_delta = accumulated_mouse_motion.delta;
    let target_lag_x = -mouse_delta.x * 0.002; // Adjust sensitivity
    let target_lag_y = mouse_delta.y * 0.002;

    // 3. Sprint Pose
    let is_sprinting = keyboard_input.pressed(KeyCode::ShiftLeft);
    let moving_forward = keyboard_input.pressed(KeyCode::KeyW);
    
    let (sprint_pos, sprint_rot) = if is_sprinting && moving_forward && speed > 0.1 {
        (Vec3::new(0.1, -0.1, -0.1), Vec3::new(-0.4, 0.8, 0.0)) // Example sprint pose
    } else {
        (Vec3::ZERO, Vec3::ZERO)
    };

    // 4. Strafe Sway
    let mut strafe_sway = Vec3::ZERO;
    if let Ok(camera_transform) = camera_query.single() {
        let right = camera_transform.compute_transform().right();
        let local_vel_x = velocity.dot(*right); // Positive = Right, Negative = Left
        // If moving right, gun lags left (negative X)
        strafe_sway.z = -local_vel_x * 0.002; // Reduced from 0.005
        // Add a bit of roll for strafing
        strafe_sway.x = -local_vel_x * 0.005; // Reduced from 0.01
    }

    // 5. Aiming
    let is_aiming = mouse_input.pressed(MouseButton::Right) && !is_sprinting;
    let inventory = inventory_query.iter().next(); // Use iter().next() for safety if single() is weird
    
    let mut target_aim_offset = Vec3::ZERO;
    let mut ads_speed_mult = 15.0;
    let mut stability_mult = 1.0;
    let mut mobility_mult = 1.0;

    if let Some(inv) = inventory {
        if let Some(config) = weapon_registry.configs.get(&inv.active_slot) {
            ads_speed_mult = config.attributes.ads_speed * 20.0;
            stability_mult = 1.0 - (config.attributes.stability * 0.5); // Higher stability = less sway
            mobility_mult = 0.5 + (config.attributes.mobility * 0.5); // Higher mobility = faster sway recovery/movement?

            if is_aiming {
                if let Some(offset) = config.attachments.optic.as_ref().and_then(|o| o.meta.as_ref()).and_then(|m| m.aim_offset) {
                    target_aim_offset = Vec3::from(offset);
                }
            }
        }
    }

    for mut recoil in query.iter_mut() {
        // Update Phase
        recoil.sway_phase += dt * sway_speed * mobility_mult;
        
        let bob_x = recoil.sway_phase.sin() * sway_amount * stability_mult;
        let bob_y = (recoil.sway_phase * 2.0).cos().abs() * sway_amount * 2.0 * stability_mult; // More vertical bob

        // Target Sway (Bobbing + Sprint + Strafe)
        // Disable sway if aiming
        let sway_mult = if is_aiming { 0.1 } else { 1.0 };
        
        let target_sway_pos = (Vec3::new(bob_x, bob_y, 0.0) + sprint_pos + Vec3::new(strafe_sway.x, 0.0, 0.0)) * sway_mult;
        
        // Target Rotation (Lag + Sprint + Strafe Roll)
        let target_sway_rot = (Vec3::new(target_lag_y, target_lag_x, strafe_sway.z) + sprint_rot) * sway_mult * stability_mult;
        
        // Smoothly interpolate
        recoil.sway_offset = recoil.sway_offset.lerp(target_sway_pos, dt * 10.0);
        recoil.sway_rotation = recoil.sway_rotation.lerp(target_sway_rot, dt * 5.0);
        recoil.aim_offset = recoil.aim_offset.lerp(target_aim_offset, dt * ads_speed_mult);
    }
}

use crate::player::input::Keybinds;

pub fn fire_weapon(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    mut inventory_query: Query<(&mut Inventory, &mut AmmoStatus)>,
    camera: Single<(&GlobalTransform, &mut Transform), With<Camera>>,
    mut weapon_query: Query<(Entity, &mut WeaponRecoil, &mut Transform), (With<WeaponModel>, Without<Camera>)>,
    mut health_query: Query<(Entity, &GlobalTransform, &mut Health, Option<&PlayerBody>, Option<&Enemy>, Option<&mut Regenerating>), Without<Projectile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut last_fire: Local<f32>,
    mut melee_hold_timer: Local<f32>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
    asset_server: Res<AssetServer>,
) {
    let (mut inventory, mut ammo_status) = if let Ok(res) = inventory_query.single_mut() { res } else { return };
    
    // Prevent firing while sprinting
    if keyboard_input.pressed(keybinds.sprint) {
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
                let max_ammo = config.attachments.magazine.as_ref().map(|m| m.carry_capacity).unwrap_or(120);
                let reserve = *ammo_status.reserve_ammo.get(&slot).unwrap_or(&max_ammo);
                
                let mag_size = config.attachments.magazine.as_ref().map(|m| m.capacity).unwrap_or(30);
                let needed = mag_size.saturating_sub(current);
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
            let max_ammo = config.attachments.magazine.as_ref().map(|m| m.carry_capacity).unwrap_or(120);
            let reserve = *ammo_status.reserve_ammo.get(&inventory.active_slot).unwrap_or(&max_ammo);
            let mag_size = config.attachments.magazine.as_ref().map(|m| m.capacity).unwrap_or(30);
            
            if current < mag_size && reserve > 0 && config.attributes.reload_speed > 0.0 {
                ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(config.attributes.reload_speed, TimerMode::Once)));
                return;
            }
        }
    }

    // Switch Fire Mode
    if keyboard_input.just_pressed(KeyCode::KeyV) {
        if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
            if !config.attributes.fire_modes.is_empty() {
                let current_idx = *ammo_status.current_fire_mode.get(&inventory.active_slot).unwrap_or(&0);
                let next_idx = (current_idx + 1) % config.attributes.fire_modes.len();
                ammo_status.current_fire_mode.insert(inventory.active_slot, next_idx);
            }
        }
    }

    let (fire_rate, speed, color, size, muzzle_offset, v_recoil, h_recoil, fire_mode, damage) = if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
        let mode_idx = *ammo_status.current_fire_mode.get(&inventory.active_slot).unwrap_or(&0);
        let mode_str = config.attributes.fire_modes.get(mode_idx).map(|s| s.as_str()).unwrap_or("Auto");
        let mode = match mode_str {
            "Auto" => FireMode::Auto,
            "Semi" => FireMode::Semi,
            "III Burst" => FireMode::Burst(3),
            _ => FireMode::Auto,
        };
        
        let muzzle = config.attachments.barrel.as_ref().and_then(|b| b.meta.as_ref()).and_then(|m| m.muzzle_flash_offset);
        let dmg = config.attachments.ammo.as_ref().map(|a| a.damage).unwrap_or(10.0);
        
        (config.attributes.fire_rate, 40.0, Color::srgb(1.0, 0.8, 0.2), 0.05, muzzle, config.attributes.vertical_recoil, config.attributes.horizontal_recoil, mode, dmg)
    } else {
        match inventory.active_slot {
            WeaponSlot::Melee => (0.5, 0.0, Color::NONE, 0.0, None, 0.0, 0.0, FireMode::Semi, 50.0),
            WeaponSlot::Equipment => (1.0, 15.0, Color::srgb(0.2, 0.8, 0.2), 0.2, None, 0.0, 0.0, FireMode::Semi, 100.0),
            _ => (0.2, 30.0, Color::WHITE, 0.1, None, 0.1, 0.05, FireMode::Auto, 10.0),
        }
    };

    // Simple cooldown
    if *last_fire + fire_rate > time.elapsed_secs() {
        return;
    }

    let mut should_fire = false;
    let mut is_slash = false;
    
    // Auto Attack for Quick Melee
    if inventory.auto_attack && inventory.active_slot == WeaponSlot::Melee && inventory.switch_state == crate::player::inventory::SwitchState::Idle {
        should_fire = true;
        inventory.auto_attack = false;
    }
    
    // Grenade Throw Logic (Release G)
    if inventory.active_slot == WeaponSlot::Equipment {
        if keyboard_input.just_released(keybinds.grenade) {
            should_fire = true;
        }
    } else if inventory.active_slot == WeaponSlot::Melee {
        // Melee Logic (Hold vs Tap)
        let attack_speed = weapon_registry.configs.get(&WeaponSlot::Melee)
            .map(|c| c.attributes.attack_speed)
            .unwrap_or(0.5);
            
        if mouse_input.pressed(MouseButton::Left) {
            *melee_hold_timer += time.delta_secs();
            if *melee_hold_timer > 0.2 {
                should_fire = true;
                is_slash = true;
                *melee_hold_timer = 0.0; // Reset to allow repeated slashes if held? Or wait for release?
                // If we want continuous slashes, we keep firing.
            }
        } else if mouse_input.just_released(MouseButton::Left) {
            if *melee_hold_timer < 0.2 {
                should_fire = true; // Stab
            }
            *melee_hold_timer = 0.0;
        } else {
            *melee_hold_timer = 0.0;
        }
    } else {
        // Gun Logic
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
    }

    if should_fire {
        // Check Ammo for guns
        if matches!(inventory.active_slot, WeaponSlot::Primary | WeaponSlot::Secondary) {
            let current = *ammo_status.current_ammo.entry(inventory.active_slot).or_insert_with(|| {
                weapon_registry.configs.get(&inventory.active_slot)
                    .and_then(|c| c.attachments.magazine.as_ref())
                    .map(|m| m.capacity)
                    .unwrap_or(30)
            });
            
            if current == 0 {
                // Auto reload if empty
                if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
                    let max_ammo = config.attachments.magazine.as_ref().map(|m| m.carry_capacity).unwrap_or(120);
                    let reserve = *ammo_status.reserve_ammo.get(&inventory.active_slot).unwrap_or(&max_ammo);
                    if reserve > 0 {
                        ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(config.attributes.reload_speed, TimerMode::Once)));
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
                let attack_speed = weapon_registry.configs.get(&WeaponSlot::Melee)
                    .map(|c| c.attributes.attack_speed)
                    .unwrap_or(0.5);

                if let Some((weapon_entity, _, _)) = weapon_query.iter().next() {
                    commands.entity(weapon_entity).insert(MeleeSwing {
                        timer: Timer::from_seconds(attack_speed, TimerMode::Once),
                    });
                }
                // Damage Logic
                let melee_range = 2.5;
                let final_damage = if is_slash { 30.0 } else { damage }; // Slash = 30, Stab = 50 (from JSON)
                
                for (_target_entity, target_transform, mut health, _, is_enemy, mut regen) in health_query.iter_mut() {
                    if is_enemy.is_none() { continue; } // Only hit enemies
                    
                    let to_target = target_transform.translation() - transform.translation; // Use camera pos, not spawn_pos
                    let distance = to_target.length();
                    
                    if distance < melee_range {
                        let dir_to_target = to_target.normalize();
                        // Check if in front (cone)
                        let cone = if is_slash { 0.2 } else { 0.8 }; // Slash is wider (0.2 dot product is wide angle), Stab is narrow
                        if forward.dot(dir_to_target) > cone {
                            health.current -= final_damage;
                            if let Some(r) = regen.as_mut() {
                                r.timer.reset();
                                r.current_rate = r.base_rate;
                            }
                            println!("Hit enemy! Health: {} (Type: {})", health.current, if is_slash { "Slash" } else { "Stab" });
                        }
                    }
                }
            },
            WeaponSlot::Equipment => {
                // Grenade Throw Logic
                commands.spawn((
                    SceneRoot(asset_server.load("weapons/models/equipment/rgd-5.glb#Scene0")),
                    Transform::from_translation(spawn_pos).with_scale(Vec3::splat(0.2)),
                    Grenade {
                        velocity: forward * 15.0 + Vec3::Y * 5.0, // Arc throw
                        timer: Timer::from_seconds(3.0, TimerMode::Once),
                    },
                ));
                
                // Animate hand/weapon throw
                if let Some((_weapon_entity, mut recoil, _)) = weapon_query.iter_mut().next() {
                     recoil.target_rotation += Vec3::new(-1.0, 0.0, 0.0); // Throw motion
                }

                // Switch back to primary
                inventory.target_slot = Some(WeaponSlot::Primary);
                inventory.switch_state = crate::player::inventory::SwitchState::Unequipping;
                inventory.switch_timer.reset();
            },
            _ => {
                // Gun Logic
                let mut rng = rand::rng();
                
                // Bullet Spread
                let spread_angle = 0.02; // Adjust as needed
                // Apply spread to the global forward.
                let right = transform.right();
                let up = transform.up();
                let r1 = rng.random_range(-spread_angle..spread_angle);
                let r2 = rng.random_range(-spread_angle..spread_angle);
                let final_velocity = (forward.as_vec3() + right.as_vec3() * r1 + up.as_vec3() * r2).normalize() * speed;

                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(size))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        emissive: LinearRgba::from(color) * 5.0,
                        ..default()
                    })),
                    Transform::from_translation(spawn_pos),
                    Projectile {
                        velocity: final_velocity,
                        timer: Timer::from_seconds(3.0, TimerMode::Once),
                        damage,
                        from_player: true,
                    },
                ));

                // Apply Camera Recoil
                let (yaw, pitch, roll) = local_transform.rotation.to_euler(EulerRot::YXZ);
                local_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch + 0.005, roll);

                // Apply Weapon Recoil & Muzzle Flash
                if let Some((weapon_entity, mut recoil, _)) = weapon_query.iter_mut().next() {
                    let mut rng = rand::rng();
                    let rand_x = rng.random_range(-0.5..0.5) * h_recoil;
                    let rand_y = rng.random_range(0.5..1.0) * v_recoil;
                    let rand_rot_x = rng.random_range(0.5..1.5) * v_recoil;
                    let rand_rot_y = rng.random_range(-0.5..0.5) * h_recoil;

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
    mut query: Query<(Entity, &mut WeaponRecoil, &mut MeleeSwing)>,
) {
    for (entity, mut recoil, mut swing) in query.iter_mut() {
        swing.timer.tick(time.delta());
        let t = swing.timer.fraction();
        
        // Simple swing animation: Rotate out and back
        // We want to animate Euler angles for the recoil struct
        // Pitch down (X), Yaw right (Y)
        let target_euler = Vec3::new(-0.5, 1.5, 0.0); 
        
        let current_euler = if t < 0.5 {
            // Out
            Vec3::ZERO.lerp(target_euler, t * 2.0)
        } else {
            // Back
            target_euler.lerp(Vec3::ZERO, (t - 0.5) * 2.0)
        };
        
        recoil.melee_rotation = current_euler;

        if swing.timer.is_finished() {
            commands.entity(entity).remove::<MeleeSwing>();
            recoil.melee_rotation = Vec3::ZERO; // Reset
        }
    }
}

pub fn handle_grenade_throw(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Grenade)>,
    mut health_query: Query<(Entity, &GlobalTransform, &mut Health, Option<&mut Regenerating>), (With<Health>, Without<Grenade>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut transform, mut grenade) in query.iter_mut() {
        grenade.timer.tick(time.delta());
        
        // Physics
        grenade.velocity.y -= 9.8 * time.delta_secs(); // Gravity
        transform.translation += grenade.velocity * time.delta_secs();
        
        // Floor collision
        if transform.translation.y < 0.2 {
            transform.translation.y = 0.2;
            grenade.velocity.y *= -0.5; // Bounce
            grenade.velocity.x *= 0.8; // Friction
            grenade.velocity.z *= 0.8;
        }

        if grenade.timer.is_finished() {
            // Explosion
            commands.entity(entity).despawn();
            
            // Smoke Particles
            let mut rng = rand::rng();
            for _ in 0..20 {
                let dir = Vec3::new(
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                ).normalize_or_zero();
                
                let speed = rng.random_range(2.0..8.0);
                let life = rng.random_range(1.0..2.5);
                let scale = rng.random_range(0.5..1.5);

                commands.spawn((
                    Mesh3d(meshes.add(Sphere::new(1.0))), // Low poly sphere or IcoSphere
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 1.0, 0.8, 0.8), // Start White-Yellow
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_translation(transform.translation).with_scale(Vec3::splat(0.1)),
                    ExplosionParticle {
                        velocity: dir * speed,
                        timer: Timer::from_seconds(life, TimerMode::Once),
                        max_time: life,
                        start_scale: 0.1,
                        end_scale: scale,
                    },
                ));
            }

            // Damage
            let explosion_radius = 5.0;
            let max_damage = 100.0;

            for (_target_entity, target_transform, mut health, mut regen) in health_query.iter_mut() {
                let distance = transform.translation.distance(target_transform.translation());
                if distance < explosion_radius {
                    let damage = max_damage * (1.0 - distance / explosion_radius);
                    health.current -= damage;
                    if let Some(r) = regen.as_mut() {
                        r.timer.reset();
                        r.current_rate = r.base_rate;
                    }
                }
            }
        }
    }
}

pub fn handle_explosion_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut ExplosionParticle, &mut MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut transform, mut particle, handle) in query.iter_mut() {
        particle.timer.tick(time.delta());
        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let t = particle.timer.fraction(); // 0.0 to 1.0
        
        // Movement
        transform.translation += particle.velocity * time.delta_secs();
        particle.velocity *= 0.95; // Drag

        // Scale
        let scale = particle.start_scale + (particle.end_scale - particle.start_scale) * t.sqrt();
        transform.scale = Vec3::splat(scale);

        // Color Fade: White-Yellow -> Orange-Red -> Gray-Black
        if let Some(material) = materials.get_mut(&handle.0) {
            let color = if t < 0.2 {
                // White-Yellow to Orange
                let sub_t = t / 0.2;
                Color::srgba(1.0, 1.0, 0.8, 0.8).mix(&Color::srgba(1.0, 0.5, 0.0, 0.7), sub_t)
            } else if t < 0.6 {
                // Orange to Gray
                let sub_t = (t - 0.2) / 0.4;
                Color::srgba(1.0, 0.5, 0.0, 0.7).mix(&Color::srgba(0.2, 0.2, 0.2, 0.5), sub_t)
            } else {
                // Gray to Transparent
                let sub_t = (t - 0.6) / 0.4;
                Color::srgba(0.2, 0.2, 0.2, 0.5).mix(&Color::srgba(0.0, 0.0, 0.0, 0.0), sub_t)
            };
            material.base_color = color;
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
            let mode_str = config.attributes.fire_modes.get(mode_idx).map(|s| s.as_str()).unwrap_or("Auto");
            let ammo_type = config.attachments.ammo.as_ref().map(|a| a.name.as_str()).unwrap_or("Unknown");
            
            **text = format!("{} | {}\n{} | {}", current, reserve, ammo_type, mode_str);
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
    mut health_query: Query<(Entity, &GlobalTransform, &mut Health, Option<&PlayerBody>, Option<&Enemy>, Option<&mut Regenerating>), Without<Projectile>>,
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
        for (_target_entity, target_transform, mut health, is_player, is_enemy, mut regen) in health_query.iter_mut() {
            // Friendly fire check
            if projectile.from_player && is_player.is_some() { continue; }
            if !projectile.from_player && is_enemy.is_some() { continue; }

            if transform.translation.distance(target_transform.translation()) < 1.5 {
                // Hit!
                health.current -= projectile.damage;
                if let Some(r) = regen.as_mut() {
                    r.timer.reset();
                    r.current_rate = r.base_rate;
                }
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
