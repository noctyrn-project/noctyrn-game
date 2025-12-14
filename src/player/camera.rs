use std::f32::consts::FRAC_PI_2;
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};
use super::input::AccumulatedInput;
use super::movement::CrouchHeight;

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
    player: Single<(&mut Transform, &CameraSensitivity), With<Camera>>,
) {
    let (mut transform, camera_sensitivity) = player.into_inner();

    let delta = accumulated_mouse_motion.delta;

    if delta != Vec2::ZERO {
        let delta_yaw = -delta.x * camera_sensitivity.x;
        let delta_pitch = -delta.y * camera_sensitivity.y;

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
    mut camera: Single<&mut Transform, With<Camera>>,
    player: Single<(&Transform, &mut CrouchHeight), (With<AccumulatedInput>, Without<Camera>)>,
) {
    let (player_transform, mut crouch_height) = player.into_inner();
    
    // Smoothly interpolate crouch height
    let dt = time.delta_secs();
    crouch_height.current = crouch_height.current.lerp(crouch_height.target, dt * 10.0);

    // Add eye height offset
    camera.translation = player_transform.translation + Vec3::Y * crouch_height.current;
}
