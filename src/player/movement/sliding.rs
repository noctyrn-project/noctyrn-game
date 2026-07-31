use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;
use crate::player::input::AccumulatedInput;

pub fn apply_slide_physics(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Velocity,
        &mut SlideState,
        &mut MovementState,
        &MovementConfig,
        &AccumulatedInput,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (mut velocity, mut slide, mut state, config, input, health) in query.iter_mut() {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        if *state != MovementState::Sliding || !slide.active {
            continue;
        }

        // ── Slide cancel: sprint key *pressed* → exit to Sprinting with momentum ──
        // Guarded to 0.1s so a same-frame entry (crouch+sprint) can't self-cancel.
        if input.sprint_pressed && slide.slide_timer > 0.1 {
            let horiz = Vec2::new(velocity.x, velocity.z);
            if horiz.length() > 0.0 {
                let preserved = horiz.length() * config.slide_cancel_speed_preservation;
                let scale = preserved / horiz.length();
                velocity.x *= scale;
                velocity.z *= scale;
            }
            slide.active = false;
            *state = MovementState::Sprinting;
            continue;
        }

        // ── Advance slide timer ──
        slide.slide_timer += dt;

        // ── Deceleration curve over the slide duration ──
        // Phantom Forces style: keep speed high early, ease down to 25% of
        // entry speed at the end — gives a long, fast, snappy slide instead
        // of a constant-rate slowdown.
        let t = (slide.slide_timer / config.slide_max_duration).min(1.0);
        let target_speed = slide.entry_speed * (0.25 + 0.75 * (1.0 - t).powf(1.2));

        let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
        let mut dir = horizontal.normalize_or_zero();

        // ── Mild steering: blend slide direction toward movement input ──
        let wish = input.movement;
        if wish.length_squared() > 0.001 {
            let steer = (dt * 6.0).min(1.0) * 0.5;
            dir = dir.lerp(wish.normalize_or_zero(), steer).normalize_or_zero();
        }

        if dir.length_squared() > 0.001 {
            velocity.x = dir.x * target_speed;
            velocity.z = dir.z * target_speed;
        }
    }
}
