use bevy::prelude::*;
use super::input::AccumulatedInput;
use crate::gameplay::Health;
use crate::world::objects::{StaticCollider, RampCollider};

/// A vector representing the player's velocity in the physics simulation.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
pub struct Velocity(pub Vec3);

/// The actual position of the player in the physics simulation.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
pub struct PhysicalTranslation(pub Vec3);

/// The value [`PhysicalTranslation`] had in the last fixed timestep.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
pub struct PreviousPhysicalTranslation(pub Vec3);

#[derive(Debug, Component, Clone, Copy, PartialEq)]
pub struct CrouchHeight {
    pub current: f32,
    pub target: f32,
}

impl Default for CrouchHeight {
    fn default() -> Self {
        Self {
            current: 1.5,
            target: 1.5,
        }
    }
}

/// Advance the physics simulation by one fixed timestep.
pub fn advance_physics(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut PhysicalTranslation,
        &mut PreviousPhysicalTranslation,
        &mut Velocity,
        &mut CrouchHeight,
        &AccumulatedInput,
        Option<&Health>,
    )>,
    collider_query: Query<(&Transform, &StaticCollider)>,
    ramp_query: Query<(&Transform, &RampCollider)>,
) {
    const GRAVITY: f32 = 18.0;
    const JUMP_SPEED: f32 = 6.5;
    const WALK_SPEED: f32 = 8.0;
    const SPRINT_SPEED: f32 = 12.0;
    const CROUCH_SPEED: f32 = 4.0;
    const GROUND_ACCEL: f32 = 70.0;
    const GROUND_FRICTION: f32 = 12.0;
    const AIR_ACCEL: f32 = 25.0;
    const AIR_SPEED_CAP: f32 = 3.5;
    const MAX_HORIZONTAL_SPEED: f32 = 14.0;
    const DIRECTION_CHANGE_PENALTY: f32 = 0.7; // Multiplier applied when changing direction

    for (mut current_physical_translation, mut previous_physical_translation, mut velocity, mut crouch_height, input, health) in
        query.iter_mut()
    {
        if let Some(h) = health {
            if h.current <= 0.0 {
                continue;
            }
        }

        previous_physical_translation.0 = current_physical_translation.0;

        let dt = fixed_time.delta_secs();

        // Crouch Logic
        if input.crouch {
            crouch_height.target = 0.8;
        } else {
            crouch_height.target = 1.5;
        }

        // Determine Max Speed
        let max_speed = if input.crouch {
            CROUCH_SPEED
        } else if input.sprint {
            SPRINT_SPEED
        } else {
            WALK_SPEED
        };

        // ── Ground detection (floor + collider surfaces + ramps) ──
        let player_radius = 0.4;
        let foot_margin = 0.08;
        let mut is_on_ground = current_physical_translation.y <= foot_margin;

        for (col_transform, collider) in collider_query.iter() {
            let col_pos = col_transform.translation;
            let he = collider.half_extents;
            let col_max_y = col_pos.y + he.y;
            if current_physical_translation.x + player_radius > col_pos.x - he.x
                && current_physical_translation.x - player_radius < col_pos.x + he.x
                && current_physical_translation.z + player_radius > col_pos.z - he.z
                && current_physical_translation.z - player_radius < col_pos.z + he.z
            {
                let feet_dist = current_physical_translation.y - col_max_y;
                if feet_dist.abs() < foot_margin && velocity.y <= 0.1 {
                    is_on_ground = true;
                }
            }
        }

        // ── Ramp ground detection ──
        for (ramp_transform, ramp_collider) in ramp_query.iter() {
            if let Some(surface_y) = ramp_surface_y(
                current_physical_translation.0,
                ramp_transform,
                ramp_collider,
            ) {
                let feet_dist = current_physical_translation.y - surface_y;
                if feet_dist.abs() < foot_margin * 2.0 && velocity.y <= 0.5 {
                    is_on_ground = true;
                }
            }
        }

        // ── Friction (on any ground surface) ──
        if is_on_ground {
            let horizontal_velocity = Vec3::new(velocity.x, 0.0, velocity.z);
            let speed = horizontal_velocity.length();
            if speed > 0.001 {
                let control = speed.max(4.0);
                let drop = control * GROUND_FRICTION * dt;
                let new_speed = (speed - drop).max(0.0);
                let scale = new_speed / speed;
                velocity.x *= scale;
                velocity.z *= scale;
            }
        }

        // ── Acceleration ──
        let wish_dir = input.movement;
        if wish_dir.length_squared() > 0.0 {
            // Direction-change penalty: when moving opposite to wish, apply extra decel
            if is_on_ground {
                let horiz_vel = Vec3::new(velocity.x, 0.0, velocity.z);
                let horiz_len = horiz_vel.length();
                if horiz_len > 0.5 {
                    let move_dir = horiz_vel / horiz_len;
                    let dot = move_dir.dot(wish_dir);
                    if dot < 0.0 {
                        // Moving opposite to desired direction - apply snappy decel
                        let penalty = 1.0 + (1.0 - DIRECTION_CHANGE_PENALTY) * (-dot) * dt * 30.0;
                        velocity.x /= penalty;
                        velocity.z /= penalty;
                    }
                }
            }

            if is_on_ground {
                let wish_speed = max_speed;
                let current_speed = velocity.dot(wish_dir);
                let add_speed = wish_speed - current_speed;
                if add_speed > 0.0 {
                    let accel_speed = (GROUND_ACCEL * dt * wish_speed).min(add_speed);
                    velocity.0 += wish_dir * accel_speed;
                }
            } else {
                // Air control: enough to feel responsive but capped to prevent exploits
                let current_speed = velocity.dot(wish_dir);
                let add_speed = AIR_SPEED_CAP - current_speed;
                if add_speed > 0.0 {
                    let accel_speed = (AIR_ACCEL * dt * AIR_SPEED_CAP).min(add_speed);
                    velocity.0 += wish_dir * accel_speed;
                }
            }
        }

        // ── Clamp horizontal speed ──
        let horiz_speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
        if horiz_speed > MAX_HORIZONTAL_SPEED {
            let scale = MAX_HORIZONTAL_SPEED / horiz_speed;
            velocity.x *= scale;
            velocity.z *= scale;
        }

        // Apply Gravity
        velocity.y -= GRAVITY * dt;

        // Apply Jump
        if is_on_ground && input.jump && !input.crouch {
             velocity.y = JUMP_SPEED;
        }

        // Update position
        current_physical_translation.0 += velocity.0 * dt;
        
        // Simple floor collision
        if current_physical_translation.y < 0.0 {
            current_physical_translation.y = 0.0;
            if velocity.y < 0.0 {
                velocity.y = 0.0;
            }
        }

        // Boundary walls (keep player within map)
        let half_map = 48.0;
        if current_physical_translation.x < -half_map {
            current_physical_translation.x = -half_map;
            if velocity.x < 0.0 { velocity.x = 0.0; }
        }
        if current_physical_translation.x > half_map {
            current_physical_translation.x = half_map;
            if velocity.x > 0.0 { velocity.x = 0.0; }
        }
        if current_physical_translation.z < -half_map {
            current_physical_translation.z = -half_map;
            if velocity.z < 0.0 { velocity.z = 0.0; }
        }
        if current_physical_translation.z > half_map {
            current_physical_translation.z = half_map;
            if velocity.z > 0.0 { velocity.z = 0.0; }
        }

        // ── Object collision (AABB slide) ──
        let player_height = crouch_height.current;
        let player_bottom = current_physical_translation.y;
        let player_top = player_bottom + player_height;

        for (col_transform, collider) in collider_query.iter() {
            let col_pos = col_transform.translation;
            let he = collider.half_extents;

            // Player AABB vs collider AABB
            let player_min_x = current_physical_translation.x - player_radius;
            let player_max_x = current_physical_translation.x + player_radius;
            let player_min_z = current_physical_translation.z - player_radius;
            let player_max_z = current_physical_translation.z + player_radius;

            let col_min_x = col_pos.x - he.x;
            let col_max_x = col_pos.x + he.x;
            let col_min_y = col_pos.y - he.y;
            let col_max_y = col_pos.y + he.y;
            let col_min_z = col_pos.z - he.z;
            let col_max_z = col_pos.z + he.z;

            // Check overlap
            if player_max_x > col_min_x && player_min_x < col_max_x
                && player_top > col_min_y && player_bottom < col_max_y
                && player_max_z > col_min_z && player_min_z < col_max_z
            {
                // Find smallest penetration axis to resolve
                let pen_px = player_max_x - col_min_x;
                let pen_nx = col_max_x - player_min_x;
                let pen_py = player_top - col_min_y;
                let pen_ny = col_max_y - player_bottom;
                let pen_pz = player_max_z - col_min_z;
                let pen_nz = col_max_z - player_min_z;

                let min_pen = pen_px.min(pen_nx).min(pen_py).min(pen_ny).min(pen_pz).min(pen_nz);

                if min_pen == pen_ny {
                    // Landing on top of object
                    current_physical_translation.y = col_max_y;
                    if velocity.y < 0.0 { velocity.y = 0.0; }
                } else if min_pen == pen_py {
                    // Head bump (hitting underside of object)
                    current_physical_translation.y = col_min_y - player_height;
                    if velocity.y > 0.0 { velocity.y = 0.0; }
                } else if min_pen == pen_px {
                    current_physical_translation.x -= pen_px;
                    if velocity.x > 0.0 { velocity.x = 0.0; }
                } else if min_pen == pen_nx {
                    current_physical_translation.x += pen_nx;
                    if velocity.x < 0.0 { velocity.x = 0.0; }
                } else if min_pen == pen_pz {
                    current_physical_translation.z -= pen_pz;
                    if velocity.z > 0.0 { velocity.z = 0.0; }
                } else if min_pen == pen_nz {
                    current_physical_translation.z += pen_nz;
                    if velocity.z < 0.0 { velocity.z = 0.0; }
                }
            }
        }

        // ── Ramp collision (oriented box - project player onto surface) ──
        for (ramp_transform, ramp_collider) in ramp_query.iter() {
            if let Some(surface_y) = ramp_surface_y(
                current_physical_translation.0,
                ramp_transform,
                ramp_collider,
            ) {
                // Push player up if they are below the ramp surface
                if current_physical_translation.y < surface_y {
                    current_physical_translation.y = surface_y;
                    if velocity.y < 0.0 {
                        velocity.y = 0.0;
                    }
                }
            }
        }
    }
}

pub fn interpolate_rendered_transform(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Transform,
        &PhysicalTranslation,
        &PreviousPhysicalTranslation,
    )>,
) {
    for (mut transform, current_physical_translation, previous_physical_translation) in
        query.iter_mut()
    {
        let previous = previous_physical_translation.0;
        let current = current_physical_translation.0;
        let alpha = fixed_time.overstep_fraction();

        let rendered_translation = previous.lerp(current, alpha);
        transform.translation = rendered_translation;
    }
}

/// Calculate the surface Y of a ramp at a given world position.
/// Returns None if the player is outside the ramp's XZ footprint or too far above/below.
fn ramp_surface_y(
    player_pos: Vec3,
    ramp_transform: &Transform,
    ramp: &RampCollider,
) -> Option<f32> {
    let inv_rotation = ramp_transform.rotation.inverse();
    // Transform player position into the ramp's local space
    let local_pos = inv_rotation * (player_pos - ramp_transform.translation);

    // Check if player is within the ramp's local XZ bounds (with small margin)
    let margin = 0.4; // player radius
    if local_pos.x.abs() > ramp.half_extents.x + margin
        || local_pos.z.abs() > ramp.half_extents.z + margin
    {
        return None;
    }

    // The ramp surface in local space is at Y = half_extents.y (top surface of the cuboid)
    // Transform that local surface point back to world space
    let surface_local = Vec3::new(local_pos.x, ramp.half_extents.y, local_pos.z);
    let surface_world = ramp_transform.rotation * surface_local + ramp_transform.translation;

    // Only apply ramp collision if player is close to the surface vertically
    // This prevents side-approach teleporting: if player is far below the surface
    // (approaching from the side at ground level), don't snap them up
    let y_distance = player_pos.y - surface_world.y;
    let max_step_up = 0.8; // Maximum height the player can step up onto
    let max_above = 2.0;   // Allow being somewhat above (e.g. jumping onto ramp)
    
    if y_distance < -max_step_up || y_distance > max_above {
        return None;
    }

    Some(surface_world.y)
}
