use std::f32::consts::FRAC_PI_2;
use bevy::{input::mouse::AccumulatedMouseMotion, prelude::*};
use super::input::AccumulatedInput;

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
    mut camera: Single<&mut Transform, With<Camera>>,
    player: Single<&Transform, (With<AccumulatedInput>, Without<Camera>)>,
) {
    // Add eye height offset
    camera.translation = player.translation + Vec3::Y * 1.5;
}
