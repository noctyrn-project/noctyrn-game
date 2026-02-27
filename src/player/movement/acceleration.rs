use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;
use crate::player::input::AccumulatedInput;

/// Applies acceleration to velocity based on player input and movement state.
///
/// # Quake-Style Acceleration Model
///
/// Both ground and air acceleration use the same core math:
///
/// ```text
/// current_speed = velocity · wish_dir   (projection onto wish direction)
/// add_speed     = wish_speed - current_speed
/// accel_speed   = min(accel * dt * wish_speed, add_speed)
/// velocity     += wish_dir * accel_speed
/// ```
///
/// The key insight is that `current_speed` is a **projection**, not the
/// magnitude. When the player strafes perpendicular to their velocity,
/// `current_speed ≈ 0` even at high total speed, so `add_speed` is large
/// and acceleration is applied in full. This is the fundamental mechanic
/// behind air strafing and bunnyhopping.
///
/// ## Ground vs Air
///
/// - **Ground:** `wish_speed` = full movement speed. High acceleration.
///   The direction change penalty adds extra deceleration when reversing
///   for snappy, responsive ground movement.
///
/// - **Air:** `wish_speed` = `air_speed_cap` (much lower). Lower acceleration.
///   The player can't directly reach full speed in air, but can *exceed*
///   `air_speed_cap` through perpendicular strafing because the projection
///   math permits speed gain in directions orthogonal to current velocity.
///
/// # Skill Depth
///
/// This model creates a natural skill curve:
/// - **Beginners** move normally with WASD
/// - **Intermediate** players learn to preserve momentum through jumps
/// - **Advanced** players chain air strafes to build speed beyond walk speed
/// - **Experts** combine sliding, bunnyhopping, and strafing for maximum velocity
pub fn apply_acceleration(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Velocity,
        &MovementState,
        &GroundedState,
        &AccumulatedInput,
        &MovementConfig,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (mut velocity, state, ground, input, config, health) in query.iter_mut() {
        // Dead players don't accelerate
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        // Sliding has its own physics in the slide system
        if *state == MovementState::Sliding {
            continue;
        }

        let wish_dir = input.movement;
        if wish_dir.length_squared() < 0.001 {
            continue;
        }

        // Determine the target speed for the current state
        let wish_speed = match *state {
            MovementState::Sprinting => config.max_sprint_speed,
            MovementState::Crouching => config.max_crouch_speed,
            MovementState::Prone => config.max_prone_speed,
            MovementState::Walking | MovementState::Idle => config.max_walk_speed,
            // Air and Sliding handled below / separately
            MovementState::Airborne => config.air_speed_cap,
            MovementState::Sliding => unreachable!(),
        };

        if ground.is_grounded && *state != MovementState::Airborne {
            // ── Ground Acceleration ──

            // Direction change penalty: when reversing direction, apply extra
            // deceleration for that "snappy stop-and-go" FPS feel.
            let horiz_vel = Vec3::new(velocity.x, 0.0, velocity.z);
            let horiz_len = horiz_vel.length();

            if horiz_len > 0.5 {
                let move_dir = horiz_vel / horiz_len;
                let dot = move_dir.dot(wish_dir);
                if dot < 0.0 {
                    // The more opposite the direction (dot → -1), the stronger the penalty
                    let penalty =
                        1.0 + (1.0 - config.direction_change_penalty) * (-dot) * dt * 30.0;
                    velocity.x /= penalty;
                    velocity.z /= penalty;
                }
            }

            // Core Quake acceleration:
            // Project current velocity onto wish direction
            let current_speed = velocity.dot(wish_dir);
            let add_speed = wish_speed - current_speed;

            if add_speed > 0.0 {
                let accel_speed =
                    (config.ground_acceleration * dt * wish_speed).min(add_speed);
                velocity.0 += wish_dir * accel_speed;
            }
        } else {
            // ── Air Acceleration ──
            // Same projection math but with the air speed cap.
            // This creates the strafe-jumping mechanic:
            //   - Forward velocity is high from the ground
            //   - Strafe input is perpendicular → current_speed ≈ 0
            //   - Full air_speed_cap worth of acceleration is applied sideways
            //   - Total speed increases via vector addition
            let current_speed = velocity.dot(wish_dir);
            let add_speed = config.air_speed_cap - current_speed;

            if add_speed > 0.0 {
                let accel_speed =
                    (config.air_acceleration * dt * config.air_speed_cap).min(add_speed);
                velocity.0 += wish_dir * accel_speed;
            }
        }
    }
}
