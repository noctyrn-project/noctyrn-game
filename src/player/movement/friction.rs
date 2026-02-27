use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;

/// Applies friction to horizontal velocity when the player is grounded.
///
/// # Friction Model
///
/// Uses a speed-dependent friction formula:
///
/// ```text
/// control   = max(speed, min_control_speed)
/// drop      = control * friction * dt
/// new_speed = max(speed - drop, 0)
/// ```
///
/// The `min_control_speed` (hardcoded at 4.0) ensures consistent stopping
/// behavior at low velocities. Without it, friction at `speed = 0.1` would
/// be nearly zero (`0.1 * friction * dt ≈ 0.002`) and the player would
/// "ice skate" to a halt extremely slowly.
///
/// This is the same friction model used in Quake/Source engines.
///
/// # Interaction with Sliding
///
/// Sliding uses its own reduced friction in the slide system.
/// This system skips entities in the `Sliding` state to avoid
/// applying double friction.
pub fn apply_friction(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Velocity,
        &MovementState,
        &GroundedState,
        &MovementConfig,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (mut velocity, state, ground, config, health) in query.iter_mut() {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        // Only apply friction when grounded
        if !ground.is_grounded {
            continue;
        }

        // Sliding has its own friction in apply_slide_physics
        if *state == MovementState::Sliding {
            continue;
        }

        let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
        let speed = horizontal.length();

        if speed < 0.001 {
            continue;
        }

        // The control value ensures friction is effective even at low speeds.
        // min_control_speed of 4.0 means friction behaves as if the player
        // is moving at least 4 u/s, preventing glacial deceleration.
        let min_control_speed = 4.0;
        let control = speed.max(min_control_speed);
        let drop = control * config.ground_friction * dt;
        let new_speed = (speed - drop).max(0.0);
        let scale = new_speed / speed;

        velocity.x *= scale;
        velocity.z *= scale;
    }
}
