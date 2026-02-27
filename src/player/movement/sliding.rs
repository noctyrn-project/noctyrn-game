use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;

/// Applies slide-specific physics when the player is in the Sliding state.
///
/// # Slide Mechanics
///
/// Sliding is triggered by sprint + crouch while above a speed threshold
/// (handled by the state transition system). During a slide:
///
/// - **Reduced friction** is applied (much lower than walking friction)
/// - **Momentum is preserved** from the entry speed
/// - A **small boost** is applied at slide entry (first frame only)
/// - The slide **ends** when speed drops below threshold or timer expires
///
/// # Skill Expression
///
/// Slides create emergent movement techniques:
/// - **Slide chaining:** Sprint → slide → stand → sprint → slide for
///   faster-than-sprinting traversal
/// - **Slide jumping:** Slide into a jump to preserve slide momentum in air
/// - **Dodge slides:** Use slides to duck under obstacles or change profile
/// - **Downhill slides:** Gravity assists on slopes extend slide duration
///
/// The interaction between slide boost, reduced friction, and the speed
/// threshold creates a natural risk/reward: you must commit to sprinting
/// to earn a slide, and the slide locks your direction.
pub fn apply_slide_physics(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Velocity,
        &mut SlideState,
        &MovementState,
        &MovementConfig,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (mut velocity, mut slide, state, config, health) in query.iter_mut() {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        if *state != MovementState::Sliding || !slide.active {
            continue;
        }

        // No boost on entry - slide inherits sprint speed naturally (boost applied in state_transitions)

        // ── Advance slide timer ──
        slide.slide_timer += dt;

        // ── Apply +15% speed boost on first frame of slide ──
        if slide.slide_timer <= dt * 1.5 {
            // Apply the boost on the first frame only
            let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
            let speed = horizontal.length();
            if speed > 0.001 {
                let boosted_speed = speed * 1.15;
                let scale = boosted_speed / speed;
                velocity.x *= scale;
                velocity.z *= scale;
            }
        }

        // ── Apply reduced friction to decelerate slowly over ~2.5 seconds ──
        let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
        let speed = horizontal.length();

        if speed > 0.001 {
            let control = speed.max(speed); // Use actual speed (no min_control_speed)
            let drop = control * config.slide_friction * dt;
            let new_speed = (speed - drop).max(0.0);
            let scale = new_speed / speed;
            velocity.x *= scale;
            velocity.z *= scale;
        }
    }
}
