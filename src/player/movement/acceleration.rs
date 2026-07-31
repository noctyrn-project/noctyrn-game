use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;
use crate::player::input::AccumulatedInput;

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
    ads_query: Query<&crate::player::input::ADSActive>,
) {
    let dt = fixed_time.delta_secs();
    let is_ads = ads_query.iter().next().is_some();

    for (mut velocity, state, ground, input, config, health) in query.iter_mut() {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        if *state == MovementState::Sliding || *state == MovementState::Mantling || *state == MovementState::Diving {
            continue;
        }

        let wish_dir = input.movement;
        if wish_dir.length_squared() < 0.001 {
            continue;
        }

        let on_ground = ground.is_grounded && *state != MovementState::Airborne;

        let wish_speed = if on_ground {
            config.effective_max_speed(*state, input, is_ads)
        } else {
            config.air_speed_cap
        };

        let accel = if on_ground {
            config.ground_acceleration
        } else {
            config.air_acceleration
        };

        let current_speed = velocity.dot(wish_dir);
        let add_speed = wish_speed - current_speed;

        if add_speed > 0.0 {
            let accel_speed = (accel * dt * wish_speed).min(add_speed);
            velocity.0 += wish_dir * accel_speed;
        }

        if on_ground {
            let horiz = Vec2::new(velocity.x, velocity.z);
            let max_horiz = config.effective_max_speed(*state, input, is_ads);
            if horiz.length() > max_horiz {
                let scale = max_horiz / horiz.length();
                velocity.x *= scale;
                velocity.z *= scale;
            }
        }
    }
}
