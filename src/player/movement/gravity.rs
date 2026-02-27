use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;

/// Applies gravity to the player's vertical velocity.
///
/// Gravity is applied manually (`v.y -= gravity * dt`) rather than
/// relying on a physics engine. This ensures:
///
/// 1. **Deterministic behavior** — Same input always produces same output
/// 2. **Full control over jump feel** — Tune gravity independently of world scale
/// 3. **Frame-rate independence** — Uses fixed delta time from `Time<Fixed>`
/// 4. **Server authority** — Identical code runs on client and server
///
/// The gravity value (default 18.0) is slightly above Earth-like (9.81)
/// to create snappy, responsive jump arcs that feel good in an FPS.
/// Higher gravity = shorter hang time = more grounded, tactical feel.
/// Lower gravity = floatier jumps = more aerial combat emphasis.
pub fn apply_gravity(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Velocity,
        &MovementConfig,
        &GroundedState,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (mut velocity, config, ground, health) in query.iter_mut() {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        // Don't apply gravity when grounded — collision resolution handles
        // surface snapping. This prevents the jitter caused by gravity pulling
        // the player into ramp surfaces every frame.
        if ground.is_grounded && velocity.y <= 0.0 {
            continue;
        }

        // Simple Euler integration: v = v - g * dt
        // Applied every fixed timestep for frame-rate independence
        velocity.y -= config.gravity * dt;
    }
}
