use bevy::prelude::*;

use super::components::*;

pub fn update_mantle(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut PhysicalTranslation,
        &mut Velocity,
        &mut MantleState,
        &mut MovementState,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (mut pos, mut velocity, mut mantle, mut state) in query.iter_mut() {
        if !mantle.active || *state != MovementState::Mantling {
            continue;
        }

        mantle.timer += dt;
        let t = (mantle.timer / mantle.duration).min(1.0);
        let eased = ease_out_cubic(t);

        pos.0 = mantle.start_pos.lerp(mantle.end_pos, eased);
        velocity.0 = Vec3::ZERO;

        if t >= 1.0 {
            mantle.active = false;
            *state = MovementState::Walking;
        }
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}
