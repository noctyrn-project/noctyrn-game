use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;
use crate::player::input::{AccumulatedInput, PlayerToggleState};
use crate::player::MainCamera;

pub fn transition_movement_state(
    fixed_time: Res<Time<Fixed>>,
    camera: Single<&Transform, With<MainCamera>>,
    mut toggle_state: Single<&mut PlayerToggleState>,
    mut query: Query<(
        &mut MovementState,
        &mut SlideState,
        &mut DiveState,
        &mut CrouchHeight,
        &mut Velocity,
        &mut JumpState,
        &GroundedState,
        &AccumulatedInput,
        &MovementConfig,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (
        mut state,
        mut slide,
        mut dive,
        mut crouch_height,
        mut velocity,
        mut jump,
        ground,
        input,
        config,
        health,
    ) in query.iter_mut()
    {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        let horizontal_speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
        let has_movement_input = input.movement.length_squared() > 0.001;

        // ── Update crouch height target based on state ──
        match *state {
            MovementState::Prone | MovementState::Diving => {
                crouch_height.target = config.prone_height;
            }
            MovementState::Crouching | MovementState::Sliding => {
                crouch_height.target = config.crouch_height_val;
            }
            _ => {
                crouch_height.target = config.stand_height;
            }
        }

        // ── Handle Mantling (locked state, no normal transitions) ──
        if *state == MovementState::Mantling {
            continue;
        }

        // ── Handle Diving → Prone on land ──
        if *state == MovementState::Diving {
            dive.timer += dt;
            if ground.is_grounded && velocity.y <= 0.0 {
                *state = MovementState::Prone;
                dive.active = false;
                crouch_height.target = config.prone_height;
            }
            continue; // No other transitions while diving
        }

        // ── Dive detection ──
        // Works from a ground sprint (pops off the ground) or in the air.
        // Requires the sprint state + sprint input + dive key.
        if matches!(*state, MovementState::Sprinting | MovementState::Airborne)
            && input.sprint
            && input.dive
        {
            let forward_flat = Vec3::new(camera.forward().x, 0.0, camera.forward().z).normalize_or_zero();
            velocity.0 += forward_flat * config.dive_boost;
            if ground.is_grounded {
                velocity.y = velocity.y.max(config.dive_lift);
            }

            // Cap horizontal speed so the dive never outpaces slide canceling.
            let horiz = Vec2::new(velocity.x, velocity.z);
            if horiz.length() > config.dive_max_speed {
                let scale = config.dive_max_speed / horiz.length();
                velocity.x *= scale;
                velocity.z *= scale;
            }

            *state = MovementState::Diving;
            dive.active = true;
            dive.timer = 0.0;
            crouch_height.target = config.prone_height;
            continue;
        }

        // ── Airborne ──
        if !ground.is_grounded {
            if slide.active {
                slide.active = false;
            }
            *state = MovementState::Airborne;
            continue;
        }

        // ── From here on, player is grounded ──

        // Continue existing slide (fixed duration, independent of crouch input)
        if slide.active {
            // Manual override: re-press crouch during the slide → drop to crouch.
            // Skipped when sprint is pressed the same tick (slide cancel + re-slide).
            if input.crouch_pressed && !input.sprint_pressed && slide.slide_timer > 0.1 {
                slide.active = false;
                *state = MovementState::Crouching;
                continue;
            }

            let slide_finished =
                slide.slide_timer >= config.slide_max_duration || horizontal_speed <= 0.3;
            if slide_finished {
                slide.active = false;
                if input.crouch {
                    *state = MovementState::Crouching;
                } else if input.sprint && has_movement_input {
                    *state = MovementState::Sprinting;
                } else if has_movement_input {
                    *state = MovementState::Walking;
                } else {
                    *state = MovementState::Idle;
                }
            } else {
                *state = MovementState::Sliding;
            }
            continue;
        }

        // Slide entry: crouch *pressed* while sprinting + above speed threshold
        if input.crouch_pressed && input.sprint && horizontal_speed >= config.slide_speed_threshold {
            slide.active = true;
            slide.slide_timer = 0.0;
            slide.entry_speed = horizontal_speed * 1.25;
            let horiz_vel = Vec3::new(velocity.x, 0.0, velocity.z);
            slide.slide_direction = horiz_vel.normalize_or_zero();
            *state = MovementState::Sliding;
            crouch_height.target = config.crouch_height_val;
            continue;
        }

        // Crouch
        if input.crouch {
            *state = MovementState::Crouching;
            continue;
        }

        // Prone toggle — works even while sprinting; cancels sprint.
        if input.prone {
            if *state == MovementState::Prone {
                *state = if has_movement_input { MovementState::Walking } else { MovementState::Idle };
                crouch_height.target = config.stand_height;
            } else {
                toggle_state.sprint = false;
                *state = MovementState::Prone;
                crouch_height.target = config.prone_height;
            }
            continue;
        }

        // Prone state
        if *state == MovementState::Prone {
            // Space → stand up only (no jump; suppressed so the buffered
            // jump input can't launch a jump on the same tick).
            if input.jump {
                jump.suppress_jump = true;
                jump.buffer_timer = 0.0;
                *state = if has_movement_input { MovementState::Walking } else { MovementState::Idle };
                crouch_height.target = config.stand_height;
                continue;
            }

            // Sprinting forward cancels prone.
            if input.sprint && has_movement_input {
                *state = MovementState::Sprinting;
                crouch_height.target = config.stand_height;
                continue;
            }

            crouch_height.target = config.prone_height;
            continue;
        }

        // Sprint
        if input.sprint && has_movement_input {
            *state = MovementState::Sprinting;
            continue;
        }

        // Walk
        if has_movement_input {
            *state = MovementState::Walking;
            continue;
        }

        // Idle
        *state = MovementState::Idle;
    }
}
