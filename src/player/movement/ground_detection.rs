use bevy::prelude::*;
use bevy_rapier3d::rapier::parry::shape::Capsule;
use bevy_rapier3d::rapier::parry::math::Pose;
use bevy_rapier3d::rapier::parry::query::distance;

use super::components::*;
use super::config::MovementConfig;
use crate::world::objects::{MeshCollider, RampCollider, StaticCollider};

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
/// they are considered grounded. Works with both AABB and OBB colliders.
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
    mesh_query: Query<&MeshCollider>,
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

        // ── Static collider top surfaces (supports rotated OBBs) ──
        for (col_transform, collider) in collider_query.iter() {
            let col_pos = col_transform.translation;
            let col_rot = col_transform.rotation;
            let he = collider.half_extents;

            // Fast AABB path for unrotated colliders
            let angle = col_rot.to_axis_angle().1.abs();
            let is_rotated = angle > 0.01;

            if !is_rotated {
                let col_max_y = col_pos.y + he.y;
                let overlaps_xz = position.x + player_radius > col_pos.x - he.x
                    && position.x - player_radius < col_pos.x + he.x
                    && position.z + player_radius > col_pos.z - he.z
                    && position.z - player_radius < col_pos.z + he.z;

                if overlaps_xz {
                    let feet_dist = position.y - col_max_y;
                    if feet_dist.abs() < foot_margin && velocity.y <= 0.1 {
                        ground.is_grounded = true;
                    }
                }
            } else {
                // OBB path: transform player into local space of the collider.
                // The top face of the OBB in local space is at local_y = he.y.
                let inv_rot = col_rot.inverse();
                let local_pos = inv_rot * (position.0 - col_pos);

                // Check horizontal (local XZ) overlap with the OBB's footprint,
                // with a margin for the player radius (approximated as a sphere
                // in local XZ).
                if local_pos.x.abs() < he.x + player_radius
                    && local_pos.z.abs() < he.z + player_radius
                {
                    // Compute the world-space Y of the OBB's top surface
                    // at the player's local X position.
                    let surface_local = Vec3::new(local_pos.x, he.y, local_pos.z);
                    let surface_world =
                        col_rot * surface_local + col_pos;
                    let feet_dist = position.y - surface_world.y;

                    if feet_dist.abs() < foot_margin * 3.0 && velocity.y <= 0.1 {
                        ground.is_grounded = true;
                        ground.ground_normal = (col_rot * Vec3::Y).normalize();
                    }
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

        // ── Mesh collider ground detection ──
        let foot_capsule = Capsule::new_y(0.01, player_radius);
        for mesh in mesh_query.iter() {
            let foot_pos = Vec3::new(position.x, position.y + 0.01, position.z);
            let iso = Pose::translation(foot_pos.x, foot_pos.y, foot_pos.z);
            if let Ok(d) = distance(&iso, &foot_capsule, &Pose::identity(), &mesh.mesh) {
                if d < foot_margin * 3.0 && velocity.y <= 0.1 {
                    ground.is_grounded = true;
                    break;
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
    let margin = 0.5;
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
