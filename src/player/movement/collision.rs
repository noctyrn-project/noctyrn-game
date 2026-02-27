use bevy::prelude::*;

use super::components::*;
use super::config::MovementConfig;
use super::ground_detection::ramp_surface_y;
use crate::gameplay::Health;
use crate::world::objects::{RampCollider, StaticCollider};

/// Resolves collisions between the player and world geometry.
///
/// Runs after velocity integration to fix any penetration that
/// the position update caused.
///
/// # Resolution Strategy
///
/// Uses **Minimum Translation Vector (MTV)** resolution:
/// 1. Find all overlapping AABBs
/// 2. For each overlap, compute penetration depth along each axis
/// 3. Resolve along the axis with the smallest penetration
/// 4. Zero velocity on the resolved axis to prevent re-penetration
///
/// This gives slide-along-walls behavior naturally: if the player
/// walks into a wall at an angle, only the perpendicular component
/// is zeroed and they continue sliding along the wall.
///
/// # Collision Types
///
/// - **Floor plane** (y = 0): Simple clamp
/// - **Map boundaries**: Clamp to ±`map_half_extent`
/// - **Static colliders**: AABB vs AABB with MTV resolution
/// - **Ramp surfaces**: Project player onto rotated surface
pub fn resolve_collisions(
    mut query: Query<(
        &mut PhysicalTranslation,
        &mut Velocity,
        &CrouchHeight,
        &MovementConfig,
        Option<&Health>,
    )>,
    collider_query: Query<(&Transform, &StaticCollider)>,
    ramp_query: Query<(&Transform, &RampCollider)>,
) {
    for (mut position, mut velocity, crouch_height, config, health) in
        query.iter_mut()
    {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        let player_radius = config.player_radius;

        // ── Floor collision (y = 0 plane) ──
        if position.y < 0.0 {
            position.y = 0.0;
            if velocity.y < 0.0 {
                velocity.y = 0.0;
            }
        }

        // ── Map boundary clamping ──
        let half_map = config.map_half_extent;

        if position.x < -half_map {
            position.x = -half_map;
            if velocity.x < 0.0 {
                velocity.x = 0.0;
            }
        }
        if position.x > half_map {
            position.x = half_map;
            if velocity.x > 0.0 {
                velocity.x = 0.0;
            }
        }
        if position.z < -half_map {
            position.z = -half_map;
            if velocity.z < 0.0 {
                velocity.z = 0.0;
            }
        }
        if position.z > half_map {
            position.z = half_map;
            if velocity.z > 0.0 {
                velocity.z = 0.0;
            }
        }

        // ── Collision with static colliders (OBB-aware) ──
        let player_height = crouch_height.current;
        let player_bottom = position.y;
        let player_top = player_bottom + player_height;

        for (col_transform, collider) in collider_query.iter() {
            let col_pos = col_transform.translation;
            let col_rot = col_transform.rotation;
            let he = collider.half_extents;

            // Check if the collider has significant rotation
            let angle = col_rot.to_axis_angle().1.abs();
            let is_rotated = angle > 0.01;

            if !is_rotated {
                // Fast AABB path for axis-aligned colliders
                let player_min_x = position.x - player_radius;
                let player_max_x = position.x + player_radius;
                let player_min_z = position.z - player_radius;
                let player_max_z = position.z + player_radius;

                let col_min_x = col_pos.x - he.x;
                let col_max_x = col_pos.x + he.x;
                let col_min_y = col_pos.y - he.y;
                let col_max_y = col_pos.y + he.y;
                let col_min_z = col_pos.z - he.z;
                let col_max_z = col_pos.z + he.z;

                if player_max_x > col_min_x
                    && player_min_x < col_max_x
                    && player_top > col_min_y
                    && player_bottom < col_max_y
                    && player_max_z > col_min_z
                    && player_min_z < col_max_z
                {
                    let pen_px = player_max_x - col_min_x;
                    let pen_nx = col_max_x - player_min_x;
                    let pen_py = player_top - col_min_y;
                    let pen_ny = col_max_y - player_bottom;
                    let pen_pz = player_max_z - col_min_z;
                    let pen_nz = col_max_z - player_min_z;

                    // Bias vertical resolution to prevent camera drift when
                    // walking face-first into a wall. When standing on or near
                    // a surface, pen_ny is tiny; adding bias ensures horizontal
                    // axes are preferred for push-out.
                    let vert_bias = 0.1;
                    let pen_ny_biased = pen_ny + vert_bias;
                    let pen_py_biased = pen_py + vert_bias;

                    let min_pen = pen_px
                        .min(pen_nx)
                        .min(pen_py_biased)
                        .min(pen_ny_biased)
                        .min(pen_pz)
                        .min(pen_nz);

                    if min_pen == pen_ny_biased {
                        position.y = col_max_y;
                        if velocity.y < 0.0 { velocity.y = 0.0; }
                    } else if min_pen == pen_py_biased {
                        position.y = col_min_y - player_height;
                        if velocity.y > 0.0 { velocity.y = 0.0; }
                    } else if min_pen == pen_px {
                        position.x -= pen_px;
                        if velocity.x > 0.0 { velocity.x = 0.0; }
                    } else if min_pen == pen_nx {
                        position.x += pen_nx;
                        if velocity.x < 0.0 { velocity.x = 0.0; }
                    } else if min_pen == pen_pz {
                        position.z -= pen_pz;
                        if velocity.z > 0.0 { velocity.z = 0.0; }
                    } else if min_pen == pen_nz {
                        position.z += pen_nz;
                        if velocity.z < 0.0 { velocity.z = 0.0; }
                    }
                }
            } else {
                // OBB collision for rotated colliders
                let inv_rot = col_rot.inverse();
                let player_center = Vec3::new(position.x, player_bottom + player_height * 0.5, position.z);
                let local_player = inv_rot * (player_center - col_pos);
                let player_half_h = player_height * 0.5;
                let player_he = Vec3::new(player_radius, player_half_h, player_radius);

                let overlap_x = (he.x + player_he.x) - local_player.x.abs();
                let overlap_y = (he.y + player_he.y) - local_player.y.abs();
                let overlap_z = (he.z + player_he.z) - local_player.z.abs();

                if overlap_x > 0.0 && overlap_y > 0.0 && overlap_z > 0.0 {
                    let min_overlap = overlap_x.min(overlap_y).min(overlap_z);

                    let local_normal = if min_overlap == overlap_y {
                        Vec3::new(0.0, local_player.y.signum(), 0.0)
                    } else if min_overlap == overlap_x {
                        Vec3::new(local_player.x.signum(), 0.0, 0.0)
                    } else {
                        Vec3::new(0.0, 0.0, local_player.z.signum())
                    };

                    let world_normal = col_rot * local_normal;
                    let push = world_normal * min_overlap;

                    if min_overlap == overlap_y && local_player.y > 0.0 {
                        // Landing on top
                        position.y += push.y;
                        if velocity.y < 0.0 { velocity.y = 0.0; }
                    } else if min_overlap == overlap_y && local_player.y < 0.0 {
                        // Head bump
                        position.y += push.y;
                        if velocity.y > 0.0 { velocity.y = 0.0; }
                    } else {
                        position.0 += push;
                        let vel_along = velocity.0.dot(world_normal);
                        if vel_along < 0.0 {
                            velocity.0 -= world_normal * vel_along;
                        }
                    }
                }
            }
        }

        // ── Ramp collision ──
        // Surface snapping from above + OBB collision for side/underneath.
        // When on top of the ramp surface, snap Y and project velocity along the
        // ramp surface to prevent jittery "slide" behavior when going downhill.
        for (ramp_transform, ramp_collider) in ramp_query.iter() {
            // Surface snapping (from above) — snaps Y and projects velocity along surface
            let mut on_surface = false;
            if let Some(surface_y) =
                ramp_surface_y(position.0, ramp_transform, ramp_collider)
            {
                let y_diff = position.y - surface_y;
                // Snap if player is at or below the surface (within a generous tolerance)
                if y_diff < 0.15 {
                    position.y = surface_y;

                    // Compute the ramp surface normal in world space
                    let ramp_normal = ramp_transform.rotation * Vec3::Y;

                    // Project velocity onto the ramp surface plane so the player
                    // moves smoothly along it instead of fighting gravity/snap.
                    // Remove the component of velocity going INTO the surface.
                    let vel_along_normal = velocity.0.dot(ramp_normal);
                    if vel_along_normal < 0.0 {
                        velocity.0 -= ramp_normal * vel_along_normal;
                    }

                    on_surface = true;
                }
            }

            // OBB vs player sphere collision (blocks side/underneath entry)
            // Skip OBB resolution if we're already snapped to the surface from above
            if on_surface {
                continue;
            }

            let inv_rot = ramp_transform.rotation.inverse();
            let local_pos = inv_rot * (position.0 - ramp_transform.translation);
            let he = ramp_collider.half_extents;

            // Clamp the local player position to the ramp OBB
            let clamped = Vec3::new(
                local_pos.x.clamp(-he.x, he.x),
                local_pos.y.clamp(-he.y, he.y),
                local_pos.z.clamp(-he.z, he.z),
            );

            let diff = local_pos - clamped;
            let dist_sq = diff.length_squared();
            let combined_radius = player_radius;

            if dist_sq < combined_radius * combined_radius && dist_sq > 0.0001 {
                let dist = dist_sq.sqrt();
                let local_normal = diff / dist;
                let penetration = combined_radius - dist;

                // Push player out in world space
                let world_normal = ramp_transform.rotation * local_normal;
                position.0 += world_normal * penetration;

                // Zero velocity along the push direction
                let vel_along = velocity.0.dot(world_normal);
                if vel_along < 0.0 {
                    velocity.0 -= world_normal * vel_along;
                }
            } else if dist_sq <= 0.0001 {
                // Player center is inside the OBB — find the axis with least penetration
                let pen_x = he.x - local_pos.x.abs();
                let pen_y = he.y - local_pos.y.abs();
                let pen_z = he.z - local_pos.z.abs();
                let min_pen = pen_x.min(pen_y).min(pen_z);

                let local_normal = if min_pen == pen_x {
                    Vec3::new(local_pos.x.signum(), 0.0, 0.0)
                } else if min_pen == pen_y {
                    Vec3::new(0.0, local_pos.y.signum(), 0.0)
                } else {
                    Vec3::new(0.0, 0.0, local_pos.z.signum())
                };

                let world_normal = ramp_transform.rotation * local_normal;
                position.0 += world_normal * (min_pen + combined_radius);

                let vel_along = velocity.0.dot(world_normal);
                if vel_along < 0.0 {
                    velocity.0 -= world_normal * vel_along;
                }
            }
        }
    }
}
