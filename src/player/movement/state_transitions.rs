use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;
use crate::player::input::AccumulatedInput;

/// Determines the player's [`MovementState`] based on input, ground contact,
/// and current velocity.
///
/// State transitions follow clear priority rules:
/// 1. Dead players don't transition
/// 2. Airborne overrides everything when not grounded
/// 3. Active slide continues until speed/timer threshold is reached
/// 4. Sprint + Crouch while fast enough → Slide entry
/// 5. Crouch alone → Crouch
/// 6. Sprint + movement input → Sprint
/// 7. Any movement input → Walk
/// 8. No input → Idle
///
/// This system also updates [`CrouchHeight`] targets based on state,
/// and manages [`SlideState`] entry/exit.
///
/// # Data flow
///
/// Reads: `GroundedState`, `Velocity`, `AccumulatedInput`, `MovementConfig`, `Health`
/// Writes: `MovementState`, `SlideState`, `CrouchHeight`
pub fn transition_movement_state(
    mut query: Query<(
        &mut MovementState,
        &mut SlideState,
        &mut CrouchHeight,
        &GroundedState,
        &Velocity,
        &AccumulatedInput,
        &MovementConfig,
        Option<&Health>,
    )>,
) {
    for (
        mut state,
        mut slide,
        mut crouch_height,
        ground,
        velocity,
        input,
        config,
        health,
    ) in query.iter_mut()
    {
        // Dead players freeze in current state
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        let horizontal_speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
        let has_movement_input = input.movement.length_squared() > 0.001;

        // ── Update crouch height target based on current state ──
        match *state {
            MovementState::Prone => {
                crouch_height.target = config.prone_height;
            }
            MovementState::Crouching | MovementState::Sliding => {
                crouch_height.target = config.crouch_height_val;
            }
            _ => {
                crouch_height.target = config.stand_height;
            }
        }

        // ── Airborne: highest priority after death ──
        if !ground.is_grounded {
            // If we were sliding and went airborne, end the slide
            // but preserve the momentum (velocity is untouched)
            if slide.active {
                slide.active = false;
            }
            *state = MovementState::Airborne;
            continue;
        }

        // ── From here on, player is grounded ──

        // Continue existing slide as long as player holds crouch and has speed
        if slide.active {
            if input.crouch && horizontal_speed > 0.3 {
                *state = MovementState::Sliding;
                continue;
            } else {
                // Slide ends → transition to crouch or idle
                slide.active = false;
                if input.crouch {
                    *state = MovementState::Crouching;
                } else if has_movement_input {
                    *state = MovementState::Walking;
                } else {
                    *state = MovementState::Idle;
                }
                continue;
            }
        }

        // Slide entry: sprinting + crouch + above speed threshold
        if input.sprint
            && input.crouch
            && horizontal_speed >= config.slide_speed_threshold
        {
            slide.active = true;
            slide.slide_timer = 0.0;
            slide.entry_speed = horizontal_speed * 1.15; // +15% speed boost on entry
            // Capture current velocity direction for the slide
            let horiz_vel = Vec3::new(velocity.x, 0.0, velocity.z);
            slide.slide_direction = horiz_vel.normalize_or_zero();
            // The +15% speed boost is applied by sliding.rs on the first frame
            *state = MovementState::Sliding;
            crouch_height.target = config.crouch_height_val;
            continue;
        }

        // Crouch (without sprint → no slide, just slow crouched movement)
        if input.crouch {
            *state = MovementState::Crouching;
            continue;
        }

        // Prone toggle
        if input.prone {
            if *state == MovementState::Prone {
                // Exit prone → idle or walk
                *state = if has_movement_input { MovementState::Walking } else { MovementState::Idle };
                crouch_height.target = config.stand_height;
            } else {
                *state = MovementState::Prone;
                crouch_height.target = config.prone_height;
            }
            continue;
        }

        // If currently prone and no prone toggle pressed, stay prone
        if *state == MovementState::Prone {
            crouch_height.target = config.prone_height;
            continue;
        }

        // Sprint (must have forward-ish movement input)
        if input.sprint && has_movement_input {
            *state = MovementState::Sprinting;
            continue;
        }

        // Walk
        if has_movement_input {
            *state = MovementState::Walking;
            continue;
        }

        // Idle — no input, grounded
        *state = MovementState::Idle;
    }
}
