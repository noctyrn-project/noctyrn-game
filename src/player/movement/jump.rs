use bevy::prelude::*;
use bevy_rapier3d::rapier::parry::query::{Ray, RayCast};
use bevy_rapier3d::rapier::parry::math::Vector;

use super::components::*;
use super::config::MovementConfig;
use crate::gameplay::Health;
use crate::player::input::AccumulatedInput;
use crate::player::MainCamera;
use crate::world::objects::MeshCollider;

pub fn handle_jump(
    fixed_time: Res<Time<Fixed>>,
    camera: Single<&Transform, With<MainCamera>>,
    mesh_query: Query<&MeshCollider>,
    mut query: Query<(
        &mut Velocity,
        &mut JumpState,
        &mut GroundedState,
        &mut MovementState,
        &mut MantleState,
        &mut CrouchHeight,
        &PhysicalTranslation,
        &AccumulatedInput,
        &MovementConfig,
        Option<&Health>,
    )>,
) {
    let dt = fixed_time.delta_secs();

    for (
        mut velocity,
        mut jump,
        mut ground,
        mut state,
        mut mantle,
        mut crouch_height,
        pos,
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

        if matches!(
            *state,
            MovementState::Crouching | MovementState::Sliding | MovementState::Prone
        ) {
            jump.buffer_timer = (jump.buffer_timer - dt).max(0.0);
            jump.coyote_timer = (jump.coyote_timer - dt).max(0.0);
            continue;
        }

        if ground.is_grounded {
            jump.coyote_timer = config.coyote_time;
            jump.has_jumped = false;
        } else {
            jump.coyote_timer = (jump.coyote_timer - dt).max(0.0);
        }

        if input.jump {
            if !jump.suppress_jump {
                jump.buffer_timer = config.jump_buffer_time;
            }
        } else {
            jump.suppress_jump = false;
            jump.buffer_timer = (jump.buffer_timer - dt).max(0.0);
        }

        // Check for mantle: sprinting + forward input + jump pressed + near obstacle
        if input.jump
            && *state == MovementState::Sprinting
            && input.raw_movement.y > 0.0
            && ground.is_grounded
        {
            if try_start_mantle(
                pos.0,
                camera.forward(),
                &mesh_query,
                config,
                &mut mantle,
            ) {
                *state = MovementState::Mantling;
                crouch_height.target = config.stand_height;
                jump.buffer_timer = 0.0;
                jump.coyote_timer = 0.0;
                continue;
            }
        }

        // Normal jump
        let can_jump = jump.buffer_timer > 0.0
            && (jump.coyote_timer > 0.0 || ground.is_grounded)
            && !jump.has_jumped;

        if can_jump {
            velocity.y = config.jump_force;
            jump.has_jumped = true;
            jump.buffer_timer = 0.0;
            jump.coyote_timer = 0.0;
            ground.is_grounded = false;
            ground.time_since_grounded = 0.001;
        }
    }
}

fn try_start_mantle(
    player_pos: Vec3,
    camera_forward: Dir3,
    mesh_query: &Query<&MeshCollider>,
    config: &MovementConfig,
    mantle: &mut MantleState,
) -> bool {
    let forward_flat = Vec3::new(camera_forward.x, 0.0, camera_forward.z).normalize_or_zero();
    if forward_flat.length_squared() < 0.001 {
        return false;
    }

    // Forward ray from chest height
    let origin = player_pos + Vec3::Y * 1.0;
    let ray = Ray::new(
        Vector::new(origin.x, origin.y, origin.z),
        Vector::new(forward_flat.x, forward_flat.y, forward_flat.z),
    );

    let mut best_dist = config.mantle_range;
    let mut hit_valid = false;

    for mc in mesh_query.iter() {
        if let Some(toi) = mc.mesh.cast_local_ray(&ray, best_dist, true) {
            // Check hit height above ground
            let world_y = origin.y + ray.dir.y * toi;
            let height_above_ground = world_y - player_pos.y;
            if height_above_ground >= config.mantle_min_height
                && height_above_ground <= config.mantle_max_height
            {
                best_dist = toi;
                hit_valid = true;
            }
        }
    }

    if !hit_valid {
        return false;
    }

    // Check clear space above the landing spot
    let landing_pos = origin + forward_flat * best_dist;
    let clear_check_origin = Vec3::new(landing_pos.x, landing_pos.y + config.stand_height + 0.3, landing_pos.z);
    let down_ray = Ray::new(
        Vector::new(clear_check_origin.x, clear_check_origin.y, clear_check_origin.z),
        Vector::new(0.0, -1.0, 0.0),
    );

    let mut hit_ground = false;
    let mut land_y = 0.0;
    for mc in mesh_query.iter() {
        if let Some(toi) = mc.mesh.cast_local_ray(&down_ray, config.mantle_max_height + 0.5, true) {
            hit_ground = true;
            land_y = clear_check_origin.y - toi;
            break;
        }
    }

    if !hit_ground {
        return false;
    }

    // Compute end position on top of the obstacle
    let end_pos = Vec3::new(player_pos.x + forward_flat.x * best_dist, land_y, player_pos.z + forward_flat.z * best_dist);

    mantle.active = true;
    mantle.start_pos = player_pos;
    mantle.end_pos = end_pos;
    mantle.timer = 0.0;
    mantle.duration = config.mantle_duration;

    true
}
