use std::f32::consts::FRAC_PI_2;
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};
use super::input::AccumulatedInput;
use super::movement::{CrouchHeight, Velocity, MovementState};
use super::DebugSettings;
use super::WeaponTerminalOpen;
use crate::settings::GameSettings;
use crate::player::CameraMode;
use crate::gameplay::PlayerBody;

#[derive(Debug, Component, Deref, DerefMut)]
pub struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(
            Vec2::new(0.003, 0.002),
        )
    }
}

pub fn rotate_camera(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    player: Single<(&mut Transform, &CameraSensitivity), With<super::MainCamera>>,
    settings: Res<GameSettings>,
    terminal_open: Res<WeaponTerminalOpen>,
    pause_open: Res<super::PauseMenuOpen>,
) {
    if terminal_open.0 || pause_open.0 { return; }
    let (mut transform, camera_sensitivity) = player.into_inner();

    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        let sensitivity_mult = settings.gameplay.sensitivity;
        let delta_yaw = -delta.x * camera_sensitivity.x * sensitivity_mult;
        let delta_pitch = -delta.y * camera_sensitivity.y * sensitivity_mult;

        let (yaw, pitch, roll) = transform.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw + delta_yaw;

        const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;
        let pitch = (pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);

        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}

// Sync the camera's position with the player's interpolated position
pub fn translate_camera(
    time: Res<Time>,
    mut camera: Single<&mut Transform, With<super::MainCamera>>,
    player: Single<(&Transform, &mut CrouchHeight), (With<AccumulatedInput>, Without<super::MainCamera>)>,
    debug_settings: Res<DebugSettings>,
    camera_mode: Res<CameraMode>,
) {
    if debug_settings.free_cam {
        return;
    }

    let (player_transform, mut crouch_height) = player.into_inner();
    
    // Smoothly interpolate crouch height
    let dt = time.delta_secs();
    crouch_height.current = crouch_height.current.lerp(crouch_height.target, dt * 10.0);

    if camera_mode.third_person {
        // 3rd person: position camera behind and above player
        let eye_pos = player_transform.translation + Vec3::Y * crouch_height.current;
        let backward = -camera.forward().as_vec3();
        let cam_pos = eye_pos + backward * camera_mode.distance + Vec3::Y * camera_mode.height_offset;
        camera.translation = cam_pos;
    } else {
        // 1st person: camera at eye height
        camera.translation = player_transform.translation + Vec3::Y * crouch_height.current;
    }
}

pub fn free_cam_movement(
    time: Res<Time>,
    mut camera: Single<&mut Transform, With<super::MainCamera>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    debug_settings: Res<DebugSettings>,
) {
    if !debug_settings.free_cam {
        return;
    }

    let mut transform = camera.into_inner();
    let mut velocity = Vec3::ZERO;
    let speed = 10.0;

    let forward = transform.forward();
    let right = transform.right();
    let _up = transform.up();

    if keyboard_input.pressed(KeyCode::KeyW) {
        velocity += forward.as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        velocity -= forward.as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        velocity -= right.as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        velocity += right.as_vec3();
    }
    if keyboard_input.pressed(KeyCode::Space) {
        velocity += Vec3::Y;
    }
    if keyboard_input.pressed(KeyCode::ControlLeft) {
        velocity -= Vec3::Y;
    }

    if velocity != Vec3::ZERO {
        transform.translation += velocity.normalize_or_zero() * speed * time.delta_secs();
    }
}

pub fn update_fov(
    settings: Res<GameSettings>,
    mut query: Query<&mut Projection, With<Camera>>,
    added_cameras: Query<(), Added<Camera>>,
) {
    if settings.is_changed() || !added_cameras.is_empty() {
        for mut projection in query.iter_mut() {
            if let Projection::Perspective(ref mut perspective) = *projection {
                perspective.fov = settings.graphics.fov.to_radians();
            }
        }
    }
}

// ── Camera Sway & Shake ──

/// Resource tracking camera sway state driven by player movement.
#[derive(Resource, Default)]
pub struct CameraSway {
    pub phase: f32,
    pub intensity: f32,
    pub target_intensity: f32,
    /// Roll offset computed by sway, to be combined with lean in apply_lean.
    pub roll_offset: f32,
    /// Pitch offset computed by sway, to be combined in apply_lean.
    pub pitch_offset: f32,
    /// Yaw offset for horizontal sway (sprinting).
    pub yaw_offset: f32,
}

/// Component for camera shake effects (e.g. grenade explosions).
#[derive(Component)]
pub struct CameraShake {
    pub timer: Timer,
    pub intensity: f32,
}

/// Compute camera sway offsets based on movement state (walk bob, sprint bob).
/// Does NOT write to the camera transform – offsets are stored in CameraSway
/// and applied together with lean in apply_lean to avoid roll-channel fights.
pub fn apply_camera_sway(
    time: Res<Time>,
    mut sway: ResMut<CameraSway>,
    player: Query<(&Velocity, &MovementState), With<PlayerBody>>,
    debug_settings: Res<DebugSettings>,
) {
    if debug_settings.free_cam {
        sway.roll_offset = 0.0;
        sway.pitch_offset = 0.0;
        return;
    }

    let dt = time.delta_secs();
    let Ok((velocity, state)) = player.single() else {
        sway.roll_offset = 0.0;
        sway.pitch_offset = 0.0;
        return;
    };

    let horiz_speed = Vec3::new(velocity.x, 0.0, velocity.z).length();

    // Determine target sway intensity based on movement state
    sway.target_intensity = match *state {
        MovementState::Sprinting => 0.008,
        MovementState::Walking => 0.005,
        MovementState::Crouching => 0.003,
        _ => 0.0,
    };

    // Smoothly interpolate intensity
    sway.intensity = sway.intensity + (sway.target_intensity - sway.intensity) * dt * 8.0;

    if sway.intensity < 0.001 {
        sway.roll_offset = 0.0;
        sway.pitch_offset = 0.0;
        return;
    }

    // Advance phase based on speed
    let bob_speed = match *state {
        MovementState::Sprinting => 10.0,
        MovementState::Walking => 7.0,
        MovementState::Crouching => 4.0,
        _ => 0.0,
    };
    sway.phase += dt * bob_speed * (horiz_speed / 10.0).min(1.5);

    // Store roll and pitch offsets (applied in apply_lean)
    sway.roll_offset = (sway.phase).sin() * sway.intensity;

    // Vertical bob multiplier: sprinting is full, walking/crouching are reduced
    let pitch_mult = match *state {
        MovementState::Sprinting => 0.5,
        MovementState::Walking => 0.25,
        MovementState::Crouching => 0.15,
        _ => 0.5,
    };
    sway.pitch_offset = (sway.phase * 2.0).sin() * sway.intensity * pitch_mult;

    // Horizontal sway only when sprinting
    sway.yaw_offset = if matches!(*state, MovementState::Sprinting) {
        (sway.phase).cos() * sway.intensity * 0.3
    } else {
        0.0
    };
}

/// Apply camera shake from explosions etc.
pub fn apply_camera_shake(
    time: Res<Time>,
    mut commands: Commands,
    mut shake_query: Query<(Entity, &mut CameraShake)>,
    mut camera: Query<&mut Transform, With<super::MainCamera>>,
) {
    let Ok(mut cam_transform) = camera.single_mut() else { return };
    let dt = time.delta_secs();

    for (entity, mut shake) in shake_query.iter_mut() {
        shake.timer.tick(time.delta());

        let remaining_frac = 1.0 - shake.timer.fraction();
        let shake_amount = shake.intensity * remaining_frac;

        // Apply random-ish shake via sin waves at different frequencies
        let t = time.elapsed_secs() * 40.0;
        let offset_x = (t * 1.3).sin() * shake_amount * 0.01;
        let offset_y = (t * 1.7).cos() * shake_amount * 0.01;

        let (yaw, pitch, roll) = cam_transform.rotation.to_euler(EulerRot::YXZ);
        cam_transform.rotation = Quat::from_euler(
            EulerRot::YXZ,
            yaw + offset_x,
            pitch + offset_y,
            roll,
        );

        if shake.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawn a camera shake effect (call from grenade explosion code etc.)
pub fn spawn_camera_shake(commands: &mut Commands, intensity: f32, duration: f32) {
    commands.spawn(CameraShake {
        timer: Timer::from_seconds(duration, TimerMode::Once),
        intensity,
    });
}

/// Update lean state from input and apply combined lean + sway roll to camera.
/// This is the single authority for the camera roll channel.
pub fn apply_lean(
    time: Res<Time>,
    sway: Res<CameraSway>,
    mut camera: Query<&mut Transform, With<super::MainCamera>>,
    mut player: Query<(&mut super::movement::LeanState, &AccumulatedInput, &super::movement::MovementConfig), With<PlayerBody>>,
    debug_settings: Res<DebugSettings>,
) {
    if debug_settings.free_cam { return; }

    let Ok((mut lean, input, config)) = player.single_mut() else { return };
    let Ok(mut cam_transform) = camera.single_mut() else { return };

    // Determine lean target
    lean.target = if input.lean_left && !input.lean_right {
        config.lean_angle
    } else if input.lean_right && !input.lean_left {
        -config.lean_angle
    } else {
        0.0
    };

    // Smoothly interpolate
    let dt = time.delta_secs();
    lean.current = lean.current + (lean.target - lean.current) * dt * config.lean_speed;

    // Combine lean roll with sway roll + sway pitch, applied as the single roll/pitch authority
    let combined_roll = lean.current + sway.roll_offset;
    let (yaw, pitch, _roll) = cam_transform.rotation.to_euler(EulerRot::YXZ);
    cam_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw + sway.yaw_offset, pitch + sway.pitch_offset, combined_roll);
}
