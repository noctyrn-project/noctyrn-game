use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use crate::world::objects::{RampCollider, StaticCollider};

/// Updates [`GroundedState`] by checking the player's position against
/// the floor plane, static box colliders, and ramp surfaces.
///
/// Runs first in the movement pipeline so all subsequent systems
/// have accurate ground contact information for the current frame.
///
/// # Ground detection method
///
/// Uses a small `foot_margin` below the player's feet. If any surface
/// is within this margin and the player is not moving upward too fast,
/// they are considered grounded. This is a simple but effective approach
/// that works well with AABB colliders.
pub fn detect_ground(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &PhysicalTranslation,
        &Velocity,
        &MovementConfig,
        &mut GroundedState,
    )>,
    collider_query: Query<(&Transform, &StaticCollider)>,
    ramp_query: Query<(&Transform, &RampCollider)>,
) {
    let dt = fixed_time.delta_secs();

    for (position, velocity, config, mut ground) in query.iter_mut() {
        // Snapshot previous state for landing detection
        ground.was_grounded = ground.is_grounded;
        ground.is_grounded = false;
        ground.ground_normal = Vec3::Y;

        let foot_margin = config.foot_margin;
        let player_radius = config.player_radius;

        // ── Floor plane (y = 0) ──
        if position.y <= foot_margin {
            ground.is_grounded = true;
        }

        // ── Static collider top surfaces ──
        for (col_transform, collider) in collider_query.iter() {
            let col_pos = col_transform.translation;
            let he = collider.half_extents;
            let col_max_y = col_pos.y + he.y;

            // Check horizontal overlap (AABB test)
            let overlaps_xz = position.x + player_radius > col_pos.x - he.x
                && position.x - player_radius < col_pos.x + he.x
                && position.z + player_radius > col_pos.z - he.z
                && position.z - player_radius < col_pos.z + he.z;

            if overlaps_xz {
                let feet_dist = position.y - col_max_y;
                // Within margin and not moving upward significantly
                if feet_dist.abs() < foot_margin && velocity.y <= 0.1 {
                    ground.is_grounded = true;
                }
            }
        }

        // ── Ramp surfaces ──
        for (ramp_transform, ramp_collider) in ramp_query.iter() {
            if let Some(surface_y) =
                ramp_surface_y(position.0, ramp_transform, ramp_collider)
            {
                let feet_dist = position.y - surface_y;
                if feet_dist.abs() < foot_margin * 3.0 && velocity.y <= 1.0 {
                    ground.is_grounded = true;
                    // Set the ground normal to the ramp's surface normal
                    ground.ground_normal = ramp_transform.rotation * Vec3::Y;
                }
            }
        }

        // ── Track time since last grounded (for coyote time) ──
        if ground.is_grounded {
            ground.time_since_grounded = 0.0;
        } else {
            ground.time_since_grounded += dt;
        }
    }
}

/// Calculate the surface Y of a ramp at a given world position.
///
/// Uses the ramp's surface plane (defined by position, rotation, and top of the OBB)
/// to compute an accurate world-space Y for the player to stand on.
///
/// Returns `None` if the player is outside the ramp's footprint
/// or too far above/below the surface.
pub fn ramp_surface_y(
    player_pos: Vec3,
    ramp_transform: &Transform,
    ramp: &RampCollider,
) -> Option<f32> {
    let inv_rotation = ramp_transform.rotation.inverse();
    // Transform player position into the ramp's local space
    let local_pos = inv_rotation * (player_pos - ramp_transform.translation);

    // Check if player is within the ramp's local XZ bounds (with margin for player radius)
    let margin = 0.4;
    if local_pos.x.abs() > ramp.half_extents.x + margin
        || local_pos.z.abs() > ramp.half_extents.z + margin
    {
        return None;
    }

    // Compute the surface point in local space at the player's XZ position.
    // The surface is at local Y = half_extents.y (top face of the cuboid).
    // We sample the surface at the player's local XZ to get the correct
    // world Y after the ramp's rotation is applied.
    let surface_local = Vec3::new(local_pos.x, ramp.half_extents.y, local_pos.z);
    let surface_world =
        ramp_transform.rotation * surface_local + ramp_transform.translation;

    // Allow stepping up onto the ramp and standing on it
    let y_distance = player_pos.y - surface_world.y;
    let max_step_up = 1.2; // Generous step-up for steep ramps
    let max_above = 2.5;

    if y_distance < -max_step_up || y_distance > max_above {
        return None;
    }

    Some(surface_world.y)
}
