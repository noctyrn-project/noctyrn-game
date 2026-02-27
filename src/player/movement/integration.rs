use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;

/// Clamps horizontal speed and integrates velocity into position.
///
/// This is the **only system that writes to [`PhysicalTranslation`]**
/// (collision resolution also writes, but only to correct penetration).
///
/// # Why Separate Integration?
///
/// By concentrating position updates here, the movement pipeline has
/// a clean data flow:
/// 1. All systems modify [`Velocity`]
/// 2. This system converts velocity → position change
/// 3. Collision fixes any resulting penetration
///
/// This separation makes server reconciliation straightforward:
/// to reconcile, override `PhysicalTranslation` with the server's
/// authoritative position and replay the pipeline from that frame.
///
/// # Speed Clamping
///
/// Before integration, horizontal speed is hard-clamped to
/// `max_horizontal_speed`. This prevents infinite speed exploits
/// from air strafing while still allowing the skill expression
/// of building speed up to that cap.
pub fn integrate_velocity(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut PhysicalTranslation,
        &mut PreviousPhysicalTranslation,
        &mut Velocity,
        &MovementConfig,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (mut position, mut prev_position, mut velocity, config, health) in
        query.iter_mut()
    {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        // Store previous position for render interpolation
        prev_position.0 = position.0;

        // ── Clamp horizontal speed ──
        // Prevents infinite speed exploits from strafe-jumping
        // while preserving the skill expression of building speed
        let horiz_speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
        if horiz_speed > config.max_horizontal_speed {
            let scale = config.max_horizontal_speed / horiz_speed;
            velocity.x *= scale;
            velocity.z *= scale;
        }

        // ── Euler integration: position += velocity * dt ──
        // Using fixed delta time ensures frame-rate independence
        position.0 += velocity.0 * dt;
    }
}

/// Interpolates the rendered [`Transform`] between physics frames
/// for smooth visual presentation at any framerate.
///
/// Without interpolation, the rendered position would snap between
/// fixed timestep positions, causing visible stuttering at lower
/// fixed rates or higher display refresh rates.
///
/// Uses the fixed timestep's overstep fraction as the blend factor:
/// ```text
/// rendered_pos = lerp(previous_physics_pos, current_physics_pos, alpha)
/// ```
/// where `alpha ∈ [0, 1]` represents how far between the last two
/// physics frames the current render frame falls.
pub fn interpolate_rendered_transform(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Transform,
        &PhysicalTranslation,
        &PreviousPhysicalTranslation,
    )>,
) {
    for (mut transform, current, previous) in query.iter_mut() {
        let alpha = fixed_time.overstep_fraction();
        transform.translation = previous.0.lerp(current.0, alpha);
    }
}
