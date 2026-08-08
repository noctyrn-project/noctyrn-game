use bevy::prelude::*;
use bevy_rapier3d::rapier::parry::query::{Ray, RayCast};
use bevy_rapier3d::rapier::parry::math::Vector;

use super::components::*;
use super::config::{MovementConfig, FALL_SNAP_SPEED};
use crate::world::objects::{MeshCollider, RampCollider};

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

        // ── Floor plane (y = 0) ──
        if position.y <= foot_margin {
            ground.is_grounded = true;
        }

        // ── Ramp surfaces ──
        for (ramp_transform, ramp_collider) in ramp_query.iter() {
            if let Some(surface_y) =
                ramp_surface_y(position.0, ramp_transform, ramp_collider)
            {
                let feet_dist = position.y - surface_y;
                // A falling player must not be grounded high above the
                // surface (that cuts gravity and floats the descent) — only
                // slow descents count, like the mesh ray below.
                if feet_dist.abs() < foot_margin * 3.0
                    && velocity.y <= 1.0
                    && velocity.y >= -FALL_SNAP_SPEED
                {
                    ground.is_grounded = true;
                    // Set the ground normal to the ramp's surface normal
                    ground.ground_normal = ramp_transform.rotation * Vec3::Y;
                }
            }
        }

        // ── Mesh collider ground detection ──
        // Directional + walkable-angle filtered: only surfaces BELOW the
        // player's feet with a mostly-upward normal can ground them. (A wall
        // beside the player must not register as ground — otherwise brushing
        // a wall grants infinite jumps and constant friction.)
        let foot_origin = Vec3::new(position.x, position.y + 0.02, position.z);
        let down_ray = Ray::new(
            Vector::new(foot_origin.x, foot_origin.y, foot_origin.z),
            Vector::new(0.0, -1.0, 0.0),
        );
        if velocity.y <= 0.1 {
            // While falling fast, only an actual touch counts as ground —
            // grounding mid-air 24 cm above the surface cuts gravity and
            // turns the landing into a hover-then-snap (jump on a mesh top).
            // Slow descents keep the generous range that lets a downhill
            // rider stay grounded between snap frames.
            let range = if velocity.y < -FALL_SNAP_SPEED {
                foot_margin
            } else {
                foot_margin * 3.0
            };
            for mesh in mesh_query.iter() {
                if let Some(hit) = mesh
                    .mesh
                    .cast_local_ray_and_get_normal(&down_ray, range, true)
                {
                    if hit.normal.y > super::config::WALKABLE_SLOPE_THRESHOLD {
                        ground.is_grounded = true;
                        break;
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_rapier3d::rapier::parry::math::Vector;
    use bevy_rapier3d::rapier::parry::shape::TriMesh;

    fn spawn(world: &mut World, feet: Vec3) {
        world.spawn((
            PhysicalTranslation(feet),
            Velocity(Vec3::ZERO),
            MovementConfig::default(),
            GroundedState::default(),
        ));
    }

    fn run_detect(world: &mut World) -> bool {
        let id = world.register_system(detect_ground);
        world.run_system(id).unwrap();
        world.query::<&GroundedState>().single(world).unwrap().is_grounded
    }

    #[test]
    fn steep_slope_does_not_ground() {
        // 60° surface (y = 1.732·z): steeper than the walkable threshold —
        // the player must NOT be grounded on it (they should slide off, not
        // stick to a near-vertical face).
        let verts = vec![
            Vector::new(-1.0, 0.0, 0.0),
            Vector::new(1.0, 0.0, 0.0),
            Vector::new(1.0, 1.732, 1.0),
            Vector::new(-1.0, 1.732, 1.0),
        ];
        // Winding so the face normal is (0, 0.5, -0.866).
        let idx = vec![[0, 2, 1], [0, 3, 2]];
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider {
            mesh: TriMesh::new(verts, idx).unwrap(),
        });
        // Feet ON the slope (surface at z=0.3 is y=0.52).
        spawn(&mut world, Vec3::new(0.0, 0.52, 0.3));
        assert!(!run_detect(&mut world), "a 60° face must not ground the player");
    }

    #[test]
    fn gentle_slope_grounds() {
        // 30° surface: walkable → grounded.
        let verts = vec![
            Vector::new(-1.0, 0.0, 0.0),
            Vector::new(1.0, 0.0, 0.0),
            Vector::new(1.0, 0.577, 1.0),
            Vector::new(-1.0, 0.577, 1.0),
        ];
        let idx = vec![[0, 2, 1], [0, 3, 2]];
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider {
            mesh: TriMesh::new(verts, idx).unwrap(),
        });
        spawn(&mut world, Vec3::new(0.0, 0.173, 0.3));
        assert!(run_detect(&mut world), "a 30° face must ground the player");
    }

    #[test]
    fn box_top_grounds_and_overhang_does_not() {
        // Outward 2×2×2 box at the origin (top at y=1.0).
        let verts = vec![
            Vector::new(-1.0, 1.0, 1.0), Vector::new(1.0, 1.0, 1.0),
            Vector::new(1.0, 1.0, -1.0), Vector::new(-1.0, 1.0, -1.0),
            Vector::new(1.0, -1.0, -1.0), Vector::new(1.0, 1.0, -1.0),
            Vector::new(1.0, 1.0, 1.0), Vector::new(1.0, -1.0, 1.0),
            Vector::new(-1.0, -1.0, 1.0), Vector::new(-1.0, 1.0, 1.0),
            Vector::new(-1.0, 1.0, -1.0), Vector::new(-1.0, -1.0, -1.0),
            Vector::new(-1.0, -1.0, 1.0), Vector::new(1.0, -1.0, 1.0),
            Vector::new(1.0, 1.0, 1.0), Vector::new(-1.0, 1.0, 1.0),
            Vector::new(-1.0, -1.0, -1.0), Vector::new(1.0, 1.0, -1.0),
            Vector::new(1.0, -1.0, -1.0), Vector::new(-1.0, 1.0, -1.0),
            Vector::new(-1.0, -1.0, 1.0), Vector::new(1.0, -1.0, -1.0),
            Vector::new(1.0, -1.0, 1.0), Vector::new(-1.0, -1.0, -1.0),
        ];
        let idx = vec![
            [0, 1, 2], [0, 2, 3], [4, 5, 6], [4, 6, 7],
            [8, 9, 10], [8, 10, 11], [12, 13, 14], [12, 14, 15],
            [16, 17, 18], [16, 19, 17], [20, 21, 22], [20, 23, 21],
        ];
        let box_mesh = TriMesh::new(verts, idx).unwrap();

        // Standing on the box top → grounded.
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: box_mesh.clone() });
        spawn(&mut world, Vec3::new(0.5, 1.0, 0.0));
        assert!(run_detect(&mut world), "box top must ground the player");

        // Feet overhanging the edge (no surface below) → NOT grounded, even
        // though the box's side face is beside them.
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: box_mesh });
        spawn(&mut world, Vec3::new(1.05, 1.0, 0.0));
        assert!(!run_detect(&mut world), "an overhanging edge must not ground the player");
    }
}
