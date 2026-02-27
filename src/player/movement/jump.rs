use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;
use crate::player::input::AccumulatedInput;

/// Handles jumping with coyote time and input buffering.
///
/// # Coyote Time
///
/// After walking off a ledge, the player has a brief grace window
/// (default 120ms) where they can still jump. This compensates for
/// the perceptual delay between the player seeing themselves leave
/// a ledge and the physics simulation losing ground contact.
///
/// Without coyote time, players frequently feel like they "pressed
/// jump but nothing happened" when they were 1-2 frames late.
///
/// # Jump Buffering
///
/// If the player presses jump slightly before landing (default 100ms
/// window), the input is stored and executed the frame they touch down.
/// This prevents "eaten" inputs and makes bunnyhopping more consistent.
///
/// Together these two mechanics make movement feel responsive and
/// forgiving without lowering the skill ceiling—they simply remove
/// frustrating near-misses that feel like engine failures rather than
/// player mistakes.
///
/// # Interaction with Other States
///
/// - Cannot jump while crouching, sliding, or prone
/// - Jump overrides vertical velocity (sets, doesn't add)
/// - Jump clears grounded state to prevent double-application
pub fn handle_jump(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Velocity,
        &mut JumpState,
        &mut GroundedState,
        &MovementState,
        &AccumulatedInput,
        &MovementConfig,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (mut velocity, mut jump, mut ground, state, input, config, health) in
        query.iter_mut()
    {
        // Dead players don't jump
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        // Cannot jump from crouching, sliding, or prone states
        if matches!(
            *state,
            MovementState::Crouching | MovementState::Sliding | MovementState::Prone
        ) {
            // Still tick timers so they expire properly
            jump.buffer_timer = (jump.buffer_timer - dt).max(0.0);
            jump.coyote_timer = (jump.coyote_timer - dt).max(0.0);
            continue;
        }

        // ── Update coyote timer ──
        // Reset to full coyote_time when grounded; count down when airborne
        if ground.is_grounded {
            jump.coyote_timer = config.coyote_time;
            jump.has_jumped = false; // Reset on landing
        } else {
            jump.coyote_timer = (jump.coyote_timer - dt).max(0.0);
        }

        // ── Update jump buffer ──
        // Start buffer window when jump is pressed; count down otherwise
        if input.jump {
            jump.buffer_timer = config.jump_buffer_time;
        } else {
            jump.buffer_timer = (jump.buffer_timer - dt).max(0.0);
        }

        // ── Execute jump if conditions are met ──
        //
        // Can jump when ALL of these are true:
        // 1. Jump input is buffered (pressed within buffer window)
        // 2. Within coyote time OR currently grounded
        // 3. Haven't already consumed the coyote jump this airborne period
        let can_jump = jump.buffer_timer > 0.0
            && (jump.coyote_timer > 0.0 || ground.is_grounded)
            && !jump.has_jumped;

        if can_jump {
            // Set vertical velocity directly (not additive).
            // This gives consistent jump height regardless of current vertical speed.
            velocity.y = config.jump_force;

            // Mark jump as consumed
            jump.has_jumped = true;
            jump.buffer_timer = 0.0;
            jump.coyote_timer = 0.0;

            // Clear grounded state so the ground detection doesn't
            // immediately re-ground the player on the same frame.
            ground.is_grounded = false;
            // Small epsilon indicates "just left ground" rather than
            // "been airborne for a long time"
            ground.time_since_grounded = 0.001;
        }
    }
}
