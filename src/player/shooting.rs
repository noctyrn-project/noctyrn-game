use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use super::inventory::{Inventory, WeaponModel};
use super::movement::Velocity;
use crate::weapons::{WeaponSlot, WeaponRecoil, BaseWeaponTransform, FireMode};
use crate::gameplay::{Health, PlayerBody, Enemy, Regenerating};
use crate::player::{spawn_hit_marker, spawn_damage_number};
use std::collections::HashMap;
use rand::Rng;

#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec3,
    pub timer: Timer,
    pub damage: f32,
    pub from_player: bool,
    pub source_name: String,
}

#[derive(Component)]
pub struct MuzzleFlash {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Target;

#[derive(Component, Default)]
pub struct CameraRecoil {
    pub current_kick: Vec2, // Pitch, Yaw
    pub target_kick: Vec2,
}

#[derive(Component, Default)]
pub struct AmmoStatus {
    pub current_ammo: HashMap<WeaponSlot, u32>,
    pub reserve_ammo: HashMap<WeaponSlot, u32>,
    pub current_fire_mode: HashMap<WeaponSlot, usize>, // Index into config.fire_modes
    pub reloading: Option<(WeaponSlot, Timer)>,
    pub burst_count: u32, // Shots remaining in current burst
    pub heat: f32, // Accuracy decay
}

#[derive(Component)]
pub struct AmmoUi;

#[derive(Component)]
pub struct MeleeSwing {
    pub timer: Timer,
    pub direction: f32, // 1.0 for right, -1.0 for left
}

#[derive(Component)]
pub struct Grenade {
    pub velocity: Vec3,
    pub timer: Timer,
    pub angular_velocity: Vec3,
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

pub fn handle_camera_recoil(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut CameraRecoil)>,
) {
    for (mut transform, mut recoil) in query.iter_mut() {
        let dt = time.delta_secs();
        
        let previous_kick = recoil.current_kick;
        
        // Interpolate current towards target
        recoil.current_kick = recoil.current_kick.lerp(recoil.target_kick, dt * 20.0);
        
        // Decay target back to zero (recovery)
        recoil.target_kick = recoil.target_kick.lerp(Vec2::ZERO, dt * 5.0);
        
        // Apply delta to transform
        let delta = recoil.current_kick - previous_kick;
        
        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw + delta.y, pitch + delta.x, roll);
    }
}

pub fn handle_weapon_recoil(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut WeaponRecoil, &BaseWeaponTransform)>,
    inventory_query: Query<&Inventory>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    let stability = if let Some(inventory) = inventory_query.iter().next() {
        weapon_registry.configs.get(&inventory.active_slot)
            .map(|c| c.attributes.stability)
            .unwrap_or(0.5)
    } else {
        0.5
    };

    let is_aiming = mouse_input.pressed(MouseButton::Right);

    for (mut transform, mut recoil, base) in query.iter_mut() {
        let dt = time.delta_secs();
        
        // Stability affects recovery speed
        let recovery_speed = 5.0 + stability * 15.0; 
        
        // Interpolate current towards target (kick)
        recoil.current_offset = recoil.current_offset.lerp(recoil.target_offset, dt * 20.0);
        recoil.current_rotation = recoil.current_rotation.lerp(recoil.target_rotation, dt * 20.0);
        
        // Decay target back to zero (recovery)
        recoil.target_offset = recoil.target_offset.lerp(Vec3::ZERO, dt * recovery_speed);
        recoil.target_rotation = recoil.target_rotation.lerp(Vec3::ZERO, dt * recovery_speed);
        
        // Apply to transform (Recoil + Sway + Aim + Switch)
        transform.translation = base.0.translation + recoil.current_offset + recoil.sway_offset + recoil.aim_offset + recoil.switch_offset;
        
        let recoil_rot = if is_aiming {
            Quat::IDENTITY
        } else {
            Quat::from_euler(
                EulerRot::XYZ, 
                recoil.current_rotation.x, 
                recoil.current_rotation.y, 
                recoil.current_rotation.z
            )
        };
        
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
    _keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut query: Query<&mut WeaponRecoil, With<WeaponModel>>,
    player_velocity: Single<&Velocity>,
    player_input: Single<&super::input::AccumulatedInput>,
    inventory_query: Query<&Inventory>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
    camera_query: Query<&GlobalTransform, With<super::MainCamera>>,
) {
    let velocity = player_velocity.into_inner();
    let input = player_input.into_inner();
    let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
    let dt = time.delta_secs();
    
    // 1. Movement Sway (Bobbing)
    // Clamp speed for frequency calculation to avoid super fast jitter
    let freq_speed = speed.min(8.0); 
    let (sway_amount, sway_speed) = if speed > 0.1 { 
        (0.005, freq_speed * 0.4) // Reduced vertical sway amount and speed
    } else { 
        (0.001, 0.5) // Idle
    };
    
    // 2. Look Sway (Lag)
    let mouse_delta = accumulated_mouse_motion.delta;
    let target_lag_x = -mouse_delta.x * 0.002; // Adjust sensitivity
    let target_lag_y = mouse_delta.y * 0.002;

    // 3. Sprint Pose (COD-style: tuck gun to chest, rock back and forth)
    let is_sprinting = input.sprint;
    let moving_forward = input.raw_movement.y > 0.0;
    
    let sprint_factor = if is_sprinting && moving_forward && speed > 0.1 {
        1.0
    } else {
        0.0
    };

    // COD-style: gun tucked closer to chest, tilted diagonally
    let sprint_target_pos = Vec3::new(0.0, -0.2, -0.1);
    let sprint_target_rot = Vec3::new(-0.6, 0.8, -0.4);

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
        
        // Smoothly transition sprint factor
        recoil.sprint_blend = recoil.sprint_blend + (sprint_factor - recoil.sprint_blend) * dt * 6.0;
        let blend = recoil.sprint_blend;
        
        let bob_x = recoil.sway_phase.sin() * sway_amount * stability_mult * 1.5; // Added horizontal sway
        let bob_y = (recoil.sway_phase * 2.0).cos().abs() * sway_amount * stability_mult; // Reduced vertical sway multiplier

        // Target Sway (Bobbing + Sprint + Strafe)
        // Disable sway if aiming
        let sway_mult = if is_aiming { 0.1 } else { 1.0 };
        
        // COD-style sprint rock: rock gun back and forth while running
        let sprint_rock_pos = if blend > 0.01 {
            let rock_phase = recoil.sway_phase * 0.8;
            Vec3::new(
                rock_phase.sin() * 0.02 * blend,        // Slight left-right rock
                rock_phase.cos().abs() * 0.01 * blend,   // Subtle up-down bounce
                (rock_phase * 0.5).sin() * 0.015 * blend, // Forward-back rock
            )
        } else {
            Vec3::ZERO
        };
        let sprint_rock_rot = if blend > 0.01 {
            let rock_phase = recoil.sway_phase * 0.8;
            Vec3::new(
                (rock_phase * 0.5).cos() * 0.06 * blend,  // Pitch rock
                rock_phase.sin() * 0.04 * blend,           // Yaw rock
                (rock_phase * 0.7).sin() * 0.03 * blend,   // Roll rock
            )
        } else {
            Vec3::ZERO
        };
        
        let sprint_pos = sprint_target_pos * blend + sprint_rock_pos;
        let sprint_rot = sprint_target_rot * blend + sprint_rock_rot;
        
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

#[derive(Default)]
pub struct FireState {
    pub last_fire: f32,
    pub melee_hold_timer: f32,
    pub last_swing_right: bool,
}

pub fn fire_weapon(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    mut inventory_query: Query<(&mut Inventory, &mut AmmoStatus)>,
    camera: Single<(&GlobalTransform, &Transform), With<super::MainCamera>>,
    mut weapon_query: Query<(Entity, &mut WeaponRecoil, &mut Transform, Option<&MeleeSwing>), (With<WeaponModel>, Without<super::MainCamera>)>,
    mut camera_recoil_query: Query<&mut CameraRecoil, With<super::MainCamera>>,
    mut health_query: Query<(Entity, &GlobalTransform, &mut Health, Option<&PlayerBody>, Option<&Enemy>, Option<&mut Regenerating>), Without<Projectile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut fire_state: Local<FireState>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
    asset_server: Res<AssetServer>,
    pause_open: Res<super::PauseMenuOpen>,
) {
    // Don't allow shooting while paused
    if pause_open.0 { return; }

    let (mut inventory, mut ammo_status) = if let Ok(res) = inventory_query.single_mut() { res } else { return };
    
    // Decay heat
    ammo_status.heat = (ammo_status.heat - time.delta_secs() * 2.0).max(0.0);

    // Prevent firing while sprinting or switching weapons
    if keyboard_input.pressed(keybinds.sprint) || inventory.switch_state != crate::player::inventory::SwitchState::Idle {
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

                if config.attributes.shell_reload_time > 0.0 {
                    // Shell-by-shell: add 1 shell per reload cycle
                    if reserve > 0 && current < mag_size {
                        ammo_status.current_ammo.insert(slot, current + 1);
                        ammo_status.reserve_ammo.insert(slot, reserve - 1);

                        // Continue reloading if not full and have reserve
                        if current + 1 < mag_size && reserve - 1 > 0 {
                            ammo_status.reloading = Some((
                                slot,
                                Timer::from_seconds(config.attributes.shell_reload_time, TimerMode::Once),
                            ));
                        }
                    }
                } else {
                    // Magazine reload: fill entire mag at once
                    let needed = mag_size.saturating_sub(current);
                    let available = reserve.min(needed);
                    
                    ammo_status.current_ammo.insert(slot, current + available);
                    ammo_status.reserve_ammo.insert(slot, reserve - available);
                }
            }
        }
    }

    if ammo_status.reloading.is_some() {
        // Shell-by-shell reload can be cancelled by firing
        if mouse_input.just_pressed(MouseButton::Left) {
            let slot = ammo_status.reloading.as_ref().map(|(s, _)| *s);
            if let Some(slot) = slot {
                if let Some(config) = weapon_registry.configs.get(&slot) {
                    if config.attributes.shell_reload_time > 0.0 {
                        let current = *ammo_status.current_ammo.get(&slot).unwrap_or(&0);
                        if current > 0 {
                            // Cancel shell reload and fire
                            ammo_status.reloading = None;
                            // Fall through to firing logic
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                return;
            }
        } else {
            return; // Can't shoot while reloading (if not cancelling)
        }
    }

    // Manual Reload
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
            let current = *ammo_status.current_ammo.get(&inventory.active_slot).unwrap_or(&0);
            let max_ammo = config.attachments.magazine.as_ref().map(|m| m.carry_capacity).unwrap_or(120);
            let reserve = *ammo_status.reserve_ammo.get(&inventory.active_slot).unwrap_or(&max_ammo);
            let mag_size = config.attachments.magazine.as_ref().map(|m| m.capacity).unwrap_or(30);
            
            if current < mag_size && reserve > 0 {
                let reload_time = if config.attributes.shell_reload_time > 0.0 {
                    config.attributes.shell_reload_time
                } else {
                    config.attributes.reload_speed
                };
                if reload_time > 0.0 {
                    ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(reload_time, TimerMode::Once)));
                    return;
                }
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

    let (fire_rate, speed, color, size, muzzle_offset, v_recoil, h_recoil, fire_mode, damage, accuracy) = if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
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
        
        (config.attributes.fire_rate, 40.0, Color::srgb(1.0, 0.8, 0.2), 0.05, muzzle, config.attributes.vertical_recoil, config.attributes.horizontal_recoil, mode, dmg, config.attributes.accuracy)
    } else {
        match inventory.active_slot {
            WeaponSlot::Melee => (0.5, 0.0, Color::NONE, 0.0, None, 0.0, 0.0, FireMode::Semi, 50.0, 1.0),
            WeaponSlot::Equipment => (1.0, 15.0, Color::srgb(0.2, 0.8, 0.2), 0.2, None, 0.0, 0.0, FireMode::Semi, 100.0, 1.0),
            _ => (0.2, 30.0, Color::WHITE, 0.1, None, 0.1, 0.05, FireMode::Auto, 10.0, 0.8),
        }
    };

    // Simple cooldown
    if fire_state.last_fire + fire_rate > time.elapsed_secs() {
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
        if keyboard_input.just_released(keybinds.grenade) || inventory.throw_queued {
            should_fire = true;
            inventory.throw_queued = false;
        }
    } else if inventory.active_slot == WeaponSlot::Melee {
        // Melee Logic (Hold vs Tap)
        let attack_speed = weapon_registry.configs.get(&WeaponSlot::Melee)
            .map(|c| c.attributes.attack_speed)
            .unwrap_or(0.5);
            
        // Check if already swinging
        let is_swinging = if let Some((_, _, _, swing)) = weapon_query.iter().next() {
            swing.is_some()
        } else {
            false
        };

        if !is_swinging {
            if mouse_input.pressed(MouseButton::Left) {
                fire_state.melee_hold_timer += time.delta_secs();
                if fire_state.melee_hold_timer > attack_speed {
                    should_fire = true;
                    is_slash = true;
                    fire_state.melee_hold_timer = 0.0; 
                }
            } else if mouse_input.just_released(MouseButton::Left) {
                if fire_state.melee_hold_timer < attack_speed {
                    should_fire = true; // Stab
                }
                fire_state.melee_hold_timer = 0.0;
            } else {
                fire_state.melee_hold_timer = 0.0;
            }
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
                        let reload_time = if config.attributes.shell_reload_time > 0.0 {
                            config.attributes.shell_reload_time
                        } else {
                            config.attributes.reload_speed
                        };
                        ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(reload_time, TimerMode::Once)));
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

        fire_state.last_fire = time.elapsed_secs();

        let (global_transform, _) = camera.into_inner();
        let transform = global_transform.compute_transform();
        let forward = transform.forward();
        let spawn_pos = transform.translation + forward * 1.0;

        match inventory.active_slot {
            WeaponSlot::Melee => {
                // Melee Swing Logic
                let attack_speed = weapon_registry.configs.get(&WeaponSlot::Melee)
                    .map(|c| c.attributes.attack_speed)
                    .unwrap_or(0.5);

                if let Some((weapon_entity, _, _, _)) = weapon_query.iter().next() {
                    // Toggle direction
                    fire_state.last_swing_right = !fire_state.last_swing_right;
                    let direction = if fire_state.last_swing_right { 1.0 } else { -1.0 };

                    commands.entity(weapon_entity).insert(MeleeSwing {
                        timer: Timer::from_seconds(attack_speed, TimerMode::Once),
                        direction,
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
                            spawn_hit_marker(&mut commands);
                            spawn_damage_number(&mut commands, final_damage, target_transform.translation());
                            println!("Hit enemy! Health: {} (Type: {})", health.current, if is_slash { "Slash" } else { "Stab" });
                        }
                    }
                }
            },
            WeaponSlot::Equipment => {
                // Grenade Throw Logic
                commands.spawn((
                    WorldAssetRoot(asset_server.load("weapons/models/equipment/grenade/rgd-5.glb#Scene0")),
                    Transform::from_translation(spawn_pos).with_scale(Vec3::splat(0.2)),
                    Grenade {
                        velocity: forward * 15.0 + Vec3::Y * 5.0, // Arc throw
                        timer: Timer::from_seconds(3.0, TimerMode::Once),
                        angular_velocity: Vec3::new(
                            rand::rng().random_range(5.0..15.0),
                            rand::rng().random_range(-3.0..3.0),
                            rand::rng().random_range(-3.0..3.0),
                        ),
                    },
                ));
                
                // Animate hand/weapon throw
                if let Some((_weapon_entity, mut recoil, _, _)) = weapon_query.iter_mut().next() {
                     recoil.target_rotation += Vec3::new(-1.0, 0.0, 0.0); // Throw motion
                }

                // Switch back to primary (or previous)
                if let Some(prev) = inventory.previous_slot {
                    inventory.target_slot = Some(prev);
                    inventory.previous_slot = None;
                } else {
                    inventory.target_slot = Some(WeaponSlot::Primary);
                }
                inventory.switch_state = crate::player::inventory::SwitchState::Unequipping;
                
                // Set timer for unequip (using equip_speed of grenade)
                let speed = weapon_registry.configs.get(&WeaponSlot::Equipment)
                    .map(|c| c.attributes.equip_speed)
                    .unwrap_or(0.5);
                inventory.switch_timer.set_duration(std::time::Duration::from_secs_f32(speed));
                inventory.switch_timer.reset();
            },
            _ => {
                // Gun Logic
                let mut rng = rand::rng();
                
                // Increase heat
                ammo_status.heat = (ammo_status.heat + 0.2).min(1.0); // Max heat 1.0

                // Check if this is a shotgun (pellet_count > 0)
                let pellet_count = weapon_registry.configs.get(&inventory.active_slot)
                    .map(|c| c.attributes.pellet_count)
                    .unwrap_or(0);
                let spread_cone = weapon_registry.configs.get(&inventory.active_slot)
                    .map(|c| c.attributes.spread_cone)
                    .unwrap_or(0.0);

                let num_projectiles = if pellet_count > 0 { pellet_count } else { 1 };
                let per_pellet_damage = if pellet_count > 0 {
                    damage / pellet_count as f32
                } else {
                    damage
                };

                let right = transform.right();
                let up = transform.up();

                for _ in 0..num_projectiles {
                    // Bullet Spread
                    let base_spread = if pellet_count > 0 {
                        // Shotgun uses spread_cone (degrees) for pellet scatter
                        spread_cone.to_radians() * 0.5
                    } else {
                        let max_spread = 0.1;
                        let heat_penalty = ammo_status.heat * 0.05;
                        ((1.0 - accuracy) * max_spread + heat_penalty).max(0.001)
                    };

                    let r1 = rng.random_range(-base_spread..base_spread);
                    let r2 = rng.random_range(-base_spread..base_spread);
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
                            damage: per_pellet_damage,
                            from_player: true,
                            source_name: "Player".to_string(),
                        },
                    ));
                }

                // Apply Camera Recoil
                if let Some(mut camera_recoil) = camera_recoil_query.iter_mut().next() {
                    let v_recoil_rad = v_recoil * 0.01;
                    let h_recoil_rad = h_recoil * 0.01;
                    
                    let mut rng = rand::rng();
                    camera_recoil.target_kick += Vec2::new(
                        rng.random_range(v_recoil_rad * 0.5..v_recoil_rad * 1.5), // Pitch (X)
                        rng.random_range(-h_recoil_rad..h_recoil_rad) // Yaw (Y)
                    );
                }

                // Apply Weapon Recoil & Muzzle Flash
                if let Some((weapon_entity, mut recoil, _, _)) = weapon_query.iter_mut().next() {
                    // Visual Kick only
                    recoil.target_offset += Vec3::new(0.0, 0.0, 0.1); 
                    recoil.target_rotation += Vec3::new(0.1, 0.0, 0.0);

                    if let Some(offset) = muzzle_offset {
                        let muzzle_pos = Vec3::from(offset);
                        let flash_size = 0.12;
                        let flash_mat = materials.add(StandardMaterial {
                            base_color: Color::srgba(1.0, 0.9, 0.3, 0.9),
                            emissive: bevy::color::LinearRgba::new(5.0, 4.0, 1.0, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            unlit: true,
                            ..default()
                        });
                        let quad_mesh = meshes.add(Rectangle::new(flash_size, flash_size * 3.0));
                        
                        commands.entity(weapon_entity).with_children(|parent| {
                            // Point light for muzzle flash illumination
                            parent.spawn((
                                PointLight {
                                    color: Color::srgb(1.0, 0.8, 0.2),
                                    intensity: 1000.0,
                                    range: 5.0,
                                    shadow_maps_enabled: false,
                                    ..default()
                                },
                                Transform::from_translation(muzzle_pos),
                                MuzzleFlash {
                                    timer: Timer::from_seconds(0.05, TimerMode::Once),
                                },
                            ));
                            // Horizontal quad at muzzle
                            parent.spawn((
                                Mesh3d(quad_mesh.clone()),
                                MeshMaterial3d(flash_mat.clone()),
                                Transform::from_translation(muzzle_pos),
                                MuzzleFlash {
                                    timer: Timer::from_seconds(0.05, TimerMode::Once),
                                },
                            ));
                            // Vertical quad (rotated 90 degrees around Z)
                            parent.spawn((
                                Mesh3d(quad_mesh),
                                MeshMaterial3d(flash_mat),
                                Transform::from_translation(muzzle_pos)
                                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
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
        let dir = swing.direction;
        
        // Wind up -> Swipe -> Recover
        let yaw = if t < 0.2 {
            // Wind up: 0 to -dir * 0.5
            let sub_t = t / 0.2;
            -dir * 0.5 * sub_t
        } else if t < 0.6 {
            // Swipe: -dir * 0.5 to dir * 1.5
            let sub_t = (t - 0.2) / 0.4;
            -dir * 0.5 + (dir * 2.0) * sub_t
        } else {
            // Recover: dir * 1.5 to 0
            let sub_t = (t - 0.6) / 0.4;
            (dir * 1.5) * (1.0 - sub_t)
        };
        
        let pitch = if t > 0.2 && t < 0.6 {
             // Dip during swipe
             -0.5 * ((t - 0.2) / 0.4 * std::f32::consts::PI).sin()
        } else {
            0.0
        };
        
        recoil.melee_rotation = Vec3::new(pitch, yaw, 0.0);

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
    collider_query: Query<(&Transform, &crate::world::objects::StaticCollider), Without<Grenade>>,
    camera_query: Query<&Transform, (With<super::MainCamera>, Without<Grenade>, Without<crate::world::objects::StaticCollider>, Without<Health>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut transform, mut grenade) in query.iter_mut() {
        grenade.timer.tick(time.delta());
        let dt = time.delta_secs();
        
        // Gravity
        grenade.velocity.y -= 9.8 * dt;
        transform.translation += grenade.velocity * dt;
        
        // Apply rotation (rolling/tumbling visual)
        let rot = Quat::from_euler(
            EulerRot::XYZ,
            grenade.angular_velocity.x * dt,
            grenade.angular_velocity.y * dt,
            grenade.angular_velocity.z * dt,
        );
        transform.rotation *= rot;
        
        // Floor collision
        if transform.translation.y < 0.2 {
            transform.translation.y = 0.2;
            grenade.velocity.y *= -0.4; // Bounce
            grenade.velocity.x *= 0.7; // Friction
            grenade.velocity.z *= 0.7;
            // Reduce spin on floor contact, add rolling
            grenade.angular_velocity *= 0.6;
            // Convert linear velocity to rolling angular velocity
            grenade.angular_velocity.x += grenade.velocity.z * 2.0;
            grenade.angular_velocity.z -= grenade.velocity.x * 2.0;
        }

        // Wall/StaticCollider collision (OBB-aware bounce)
        let grenade_radius = 0.15;
        for (col_transform, collider) in collider_query.iter() {
            let col_pos = col_transform.translation;
            let col_rot = col_transform.rotation;
            let he = collider.half_extents;
            let pos = transform.translation;

            let angle = col_rot.to_axis_angle().1.abs();
            let is_rotated = angle > 0.01;

            if !is_rotated {
                // Fast AABB path
                let min = col_pos - he;
                let max = col_pos + he;

                if pos.x + grenade_radius > min.x && pos.x - grenade_radius < max.x
                    && pos.y + grenade_radius > min.y && pos.y - grenade_radius < max.y
                    && pos.z + grenade_radius > min.z && pos.z - grenade_radius < max.z
                {
                    let pen_px = (pos.x + grenade_radius) - min.x;
                    let pen_nx = max.x - (pos.x - grenade_radius);
                    let pen_py = (pos.y + grenade_radius) - min.y;
                    let pen_ny = max.y - (pos.y - grenade_radius);
                    let pen_pz = (pos.z + grenade_radius) - min.z;
                    let pen_nz = max.z - (pos.z - grenade_radius);

                    let min_pen = pen_px.min(pen_nx).min(pen_py).min(pen_ny).min(pen_pz).min(pen_nz);
                    let bounce_factor = 0.4;
                    let friction_factor = 0.7;

                    if min_pen == pen_ny {
                        transform.translation.y = max.y + grenade_radius;
                        grenade.velocity.y *= -bounce_factor;
                        grenade.velocity.x *= friction_factor;
                        grenade.velocity.z *= friction_factor;
                    } else if min_pen == pen_py {
                        transform.translation.y = min.y - grenade_radius;
                        grenade.velocity.y *= -bounce_factor;
                    } else if min_pen == pen_px {
                        transform.translation.x = min.x - grenade_radius;
                        grenade.velocity.x *= -bounce_factor;
                        grenade.velocity.z *= friction_factor;
                    } else if min_pen == pen_nx {
                        transform.translation.x = max.x + grenade_radius;
                        grenade.velocity.x *= -bounce_factor;
                        grenade.velocity.z *= friction_factor;
                    } else if min_pen == pen_pz {
                        transform.translation.z = min.z - grenade_radius;
                        grenade.velocity.z *= -bounce_factor;
                        grenade.velocity.x *= friction_factor;
                    } else if min_pen == pen_nz {
                        transform.translation.z = max.z + grenade_radius;
                        grenade.velocity.z *= -bounce_factor;
                        grenade.velocity.x *= friction_factor;
                    }

                    grenade.angular_velocity *= 0.7;
                }
            } else {
                // OBB path for rotated colliders
                let inv_rot = col_rot.inverse();
                let local_pos = inv_rot * (pos - col_pos);

                // Check overlap in local space with grenade radius
                let overlap_x = (he.x + grenade_radius) - local_pos.x.abs();
                let overlap_y = (he.y + grenade_radius) - local_pos.y.abs();
                let overlap_z = (he.z + grenade_radius) - local_pos.z.abs();

                if overlap_x > 0.0 && overlap_y > 0.0 && overlap_z > 0.0 {
                    let min_overlap = overlap_x.min(overlap_y).min(overlap_z);

                    let local_normal = if min_overlap == overlap_y {
                        Vec3::new(0.0, local_pos.y.signum(), 0.0)
                    } else if min_overlap == overlap_x {
                        Vec3::new(local_pos.x.signum(), 0.0, 0.0)
                    } else {
                        Vec3::new(0.0, 0.0, local_pos.z.signum())
                    };

                    let world_normal = col_rot * local_normal;
                    transform.translation += world_normal * min_overlap;

                    // Bounce: reflect velocity along the push normal
                    let vel_along = grenade.velocity.dot(world_normal);
                    if vel_along < 0.0 {
                        grenade.velocity -= world_normal * vel_along * 1.4; // 0.4 bounce factor
                        // Friction on tangent
                        let tangent_vel = grenade.velocity - world_normal * grenade.velocity.dot(world_normal);
                        grenade.velocity = world_normal * grenade.velocity.dot(world_normal) + tangent_vel * 0.7;
                    }

                    grenade.angular_velocity *= 0.7;
                }
            }
        }

        if grenade.timer.is_finished() {
            // Explosion
            commands.entity(entity).despawn();
            
            // Smoke Particles - uneven polygon shapes for realistic explosion
            let mut rng = rand::rng();
            for _ in 0..25 {
                let dir = Vec3::new(
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                ).normalize_or_zero();
                
                let speed = rng.random_range(2.0..8.0);
                let life = rng.random_range(1.0..2.5);
                let scale = rng.random_range(0.5..1.5);

                // Create irregular shapes by using cuboids with random dimensions
                let sx = rng.random_range(0.4..1.6);
                let sy = rng.random_range(0.4..1.6);
                let sz = rng.random_range(0.4..1.6);

                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(sx, sy, sz))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 1.0, 0.8, 0.8),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_translation(transform.translation)
                        .with_scale(Vec3::splat(0.1))
                        .with_rotation(Quat::from_euler(
                            EulerRot::XYZ,
                            rng.random_range(0.0..std::f32::consts::TAU),
                            rng.random_range(0.0..std::f32::consts::TAU),
                            rng.random_range(0.0..std::f32::consts::TAU),
                        )),
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

            // Camera shake from explosion
            let player_distance = if let Some(cam_transform) = camera_query.iter().next() {
                transform.translation.distance(cam_transform.translation)
            } else {
                f32::MAX
            };
            if player_distance < explosion_radius * 3.0 {
                let shake_intensity = (1.0 - player_distance / (explosion_radius * 3.0)) * 8.0;
                crate::player::camera::spawn_camera_shake(&mut commands, shake_intensity, 0.5);
            }

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

        if let Some(mut material) = materials.get_mut(&handle.0) {
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
    terminal_query: Query<(&GlobalTransform, &crate::world::objects::WeaponTerminal), Without<Projectile>>,
    mut terminal_open: ResMut<crate::player::WeaponTerminalOpen>,
    collider_query: Query<(Entity, &Transform, &crate::world::objects::StaticCollider, Option<&crate::world::objects::MaterialType>), Without<Projectile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Track glass entities to despawn after iteration
    let mut glass_to_despawn: Vec<Entity> = Vec::new();

    for (entity, mut transform, mut projectile) in query.iter_mut() {
        projectile.timer.tick(time.delta());
        if projectile.timer.just_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let delta = projectile.velocity * time.delta_secs();
        let _old_pos = transform.translation;
        transform.translation += delta;
        let new_pos = transform.translation;

        // Check terminal hits first
        if projectile.from_player {
            let mut hit_terminal = false;
            for (terminal_transform, _terminal) in terminal_query.iter() {
                if transform.translation.distance(terminal_transform.translation()) < 1.5 {
                    terminal_open.0 = true;
                    commands.entity(entity).despawn();
                    hit_terminal = true;
                    break;
                }
            }
            if hit_terminal { continue; }
        }

        // OBB-aware collision with static colliders (penetration system)
        let mut hit_collider = false;
        for (_col_entity, col_transform, collider, material_type) in collider_query.iter() {
            let col_pos = col_transform.translation;
            let col_rot = col_transform.rotation;
            let he = collider.half_extents;

            // Transform bullet position into collider's local space
            let inv_rot = col_rot.inverse();
            let local_pos = inv_rot * (new_pos - col_pos);

            // Point-in-OBB check in local space
            if local_pos.x.abs() < he.x && local_pos.y.abs() < he.y && local_pos.z.abs() < he.z {
                if let Some(mat_type) = material_type {
                    let _penetration_power = 1.0 - mat_type.resistance();
                    
                    // Check if bullet can penetrate
                    let bullet_pen = projectile.damage / 100.0; // Normalize damage as penetration factor
                    
                    if mat_type.shatters() {
                        // Glass shattering effect
                        let bullet_dir = projectile.velocity.normalize_or_zero();
                        crate::world::objects::spawn_glass_shatter(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            new_pos,
                            bullet_dir,
                        );
                        // Despawn the glass wall entity
                        glass_to_despawn.push(_col_entity);
                        // Bullet passes through glass with slight damage reduction
                        projectile.damage *= mat_type.damage_falloff();
                        // Don't destroy the bullet, it penetrates
                        continue;
                    } else if bullet_pen > mat_type.resistance() * 0.5 {
                        // Bullet penetrates: reduce damage and continue
                        projectile.damage *= mat_type.damage_falloff();
                        // Slow bullet down
                        projectile.velocity *= 0.7;
                        // Don't destroy - bullet continues through
                        continue;
                    } else {
                        // Bullet stopped by material
                        commands.entity(entity).despawn();
                        hit_collider = true;
                        break;
                    }
                } else {
                    // No material type - bullet stops
                    commands.entity(entity).despawn();
                    hit_collider = true;
                    break;
                }
            }
        }
        if hit_collider { continue; }

        // Entity collision check (distance based)
        for (_target_entity, target_transform, mut health, is_player, is_enemy, mut regen) in health_query.iter_mut() {
            if projectile.from_player && is_player.is_some() { continue; }
            if !projectile.from_player && is_enemy.is_some() { continue; }

            if transform.translation.distance(target_transform.translation()) < 1.5 {
                health.current -= projectile.damage;
                if let Some(r) = regen.as_mut() {
                    r.timer.reset();
                    r.current_rate = r.base_rate;
                }
                if projectile.from_player {
                    spawn_hit_marker(&mut commands);
                    spawn_damage_number(&mut commands, projectile.damage, target_transform.translation());
                }
                // Track who killed the player
                if is_player.is_some() && health.current <= 0.0 {
                    commands.insert_resource(crate::gameplay::KillerInfo(projectile.source_name.clone()));
                }
                commands.entity(entity).despawn();
                break;
            }
        }
        
        // Floor collision
        if transform.translation.y < 0.0 {
             commands.entity(entity).despawn();
        }
    }

    // Despawn shattered glass entities
    for glass_entity in glass_to_despawn {
        commands.entity(glass_entity).despawn();
    }
}
