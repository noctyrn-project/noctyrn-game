use bevy::prelude::*;

use super::components::*;
use crate::gameplay::Health;

pub fn integrate_velocity(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut PhysicalTranslation,
        &mut PreviousPhysicalTranslation,
        &mut Velocity,
        &MovementState,
        &MantleState,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (
        mut position,
        mut prev_position,
        velocity,
        state,
        mantle,
        health,
    ) in query.iter_mut()
    {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        prev_position.0 = position.0;

        // Skip standard integration during mantle (update_mantle handles position)
        if *state == MovementState::Mantling && mantle.active {
            continue;
        }

        // Skip integration during dive (gravity+collision handles trajectory)
        if *state == MovementState::Diving {
            // Still apply gravity during dive
            position.0 += velocity.0 * dt;
            continue;
        }

        // ── Euler integration: position += velocity * dt ──
        position.0 += velocity.0 * dt;
    }
}

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
