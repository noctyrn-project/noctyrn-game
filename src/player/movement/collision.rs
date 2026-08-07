use bevy::prelude::*;
use bevy_rapier3d::rapier::parry::query::{cast_shapes, ShapeCastOptions, Ray, RayCast};
use bevy_rapier3d::rapier::parry::shape::Capsule;
use bevy_rapier3d::rapier::parry::math::{Pose, Vector};

use super::components::*;
use super::config::MovementConfig;
use super::ground_detection::ramp_surface_y;
use crate::gameplay::Health;
use crate::world::objects::{MeshCollider, RampCollider};

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
/// - **Static colliders**: AABB vs AABB with MTV resolution
/// - **Ramp surfaces**: Project player onto rotated surface
pub fn resolve_collisions(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut PhysicalTranslation,
        &mut Velocity,
        &CrouchHeight,
        &MovementConfig,
        &GroundedState,
        Option<&Health>,
    )>,
    ramp_query: Query<(&Transform, &RampCollider)>,
    mesh_query: Query<&MeshCollider>,
) {
    let dt = fixed_time.delta_secs();

    for (mut position, mut velocity, crouch_height, config, ground, health) in
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

        let player_height = crouch_height.current;

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
                if y_diff < 0.25 {
                    position.y = surface_y;

                    let ramp_normal = ramp_transform.rotation * Vec3::Y;

                    // Project velocity onto the ramp surface plane
                    let vel_along_normal = velocity.0.dot(ramp_normal);
                    if vel_along_normal < 0.0 {
                        velocity.0 -= ramp_normal * vel_along_normal;
                    }
                    // Clamp upward velocity when on ramp to prevent sliding up
                    if velocity.y > 0.1 {
                        velocity.y *= 0.5;
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
            let combined_radius = player_radius + 0.05;

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

        // ── Triangle mesh collider collision (swept / CCD) ──
        // Cast the player capsule from its current position along its movement
        // vector. If a hit occurs, stop at the hit point and kill velocity
        // along the hit normal.  This prevents both tunneling and jitter.
        let player_half_height = player_height * 0.5;
        let capsule = Capsule::new_y(player_half_height.max(0.01), player_radius);
        let movement = velocity.0 * dt;

        if movement.length_squared() > 0.0001 {
            let old_center = Vec3::new(position.x, position.y + player_half_height, position.z) - movement;
            let new_center = Vec3::new(position.x, position.y + player_half_height, position.z);
            let sweep_dir = new_center - old_center;

            let mut earliest_toi = 2.0f32;
            let mut best_normal = Vector::ZERO;

            for mesh_collider in mesh_query.iter() {
                if let Ok(Some(hit)) = cast_shapes(
                    &Pose::translation(old_center.x, old_center.y, old_center.z),
                    Vector::new(sweep_dir.x, sweep_dir.y, sweep_dir.z),
                    &capsule,
                    &Pose::identity(),
                    Vector::ZERO,
                    &mesh_collider.mesh,
                    ShapeCastOptions {
                        max_time_of_impact: 1.0,
                        target_distance: 0.001,
                        stop_at_penetration: true,
                        compute_impact_geometry_on_penetration: true,
                    },
                ) {
                    if hit.time_of_impact < earliest_toi {
                        earliest_toi = hit.time_of_impact;
                        best_normal = hit.normal2; // outward normal of the mesh
                    }
                }
            }

            if earliest_toi <= 1.0 {
                let hit_pos = old_center + sweep_dir * earliest_toi;
                let normal = Vec3::new(best_normal.x, best_normal.y, best_normal.z);

                // ── Auto step-up ──
                // Only while grounded (runs after GroundDetection) — an airborne
                // player near a wall must not be lifted at the top of a jump.
                let wall_like = normal.y.abs() < 0.35;
                let mut stepped = false;
                if wall_like && velocity.y <= 0.5 && ground.is_grounded {
                    stepped = try_step_up(
                        &mut position,
                        old_center,
                        sweep_dir,
                        &capsule,
                        &mesh_query,
                        config,
                        player_half_height,
                    );
                }

                if !stepped {
                    // Compare the hit by PHYSICAL distance, not raw TOI —
                    // TOI is a fraction of the sweep, so at speed even a
                    // hair of contact gap yields a misleading TOI.
                    let sweep_len = sweep_dir.length();
                    let hit_distance = earliest_toi * sweep_len;

                    if hit_distance >= 0.01 {
                        // Real obstacle: move to the impact point minus a margin.
                        let margin = normal * 0.001;
                        position.0 = hit_pos + margin - Vec3::Y * player_half_height;
                    } else {
                        // Contact/brushing: the integration already moved the
                        // capsule into the wall this frame. Restore the
                        // position to the surface by pushing OUT along the
                        // normal (the penetration depth), keeping tangential
                        // progress intact — otherwise the player sinks
                        // through the wall while pressing into it.
                        let penetration = (-velocity.0.dot(normal) * dt).max(0.0);
                        position.0 += normal * penetration;
                        position.0 += normal * 0.001;
                    }

                    // Kill velocity along the hit normal
                    let vn = velocity.0.dot(normal);
                    if vn < 0.0 {
                        velocity.0 -= normal * vn;
                    }
                }
            }
        }
    }
}

/// Attempt to step the player up onto an obstacle ≤ `step_up_height` tall.
///
/// Returns true if the player was stepped up. The player is moved the full
/// sweep distance at the elevated position, and horizontal velocity is kept.
fn try_step_up(
    position: &mut PhysicalTranslation,
    old_center: Vec3,
    sweep_dir: Vec3,
    capsule: &Capsule,
    mesh_query: &Query<&MeshCollider>,
    config: &MovementConfig,
    player_half_height: f32,
) -> bool {
    // Probe forward at step height (slightly above so a max-height step clears).
    // The probe sweeps TWICE the movement distance: a wall whose contact
    // boundary sits exactly at the movement end (toi ≈ 1.0) is not reported
    // by cast_shapes, which would let the step-up teleport the player through
    // tall walls.
    let probe_center = old_center + Vec3::Y * (config.step_up_height * 1.1);

    let options = ShapeCastOptions {
        max_time_of_impact: 1.0,
        target_distance: 0.001,
        stop_at_penetration: true,
        compute_impact_geometry_on_penetration: true,
    };

    for mesh_collider in mesh_query.iter() {
        if let Ok(Some(_)) = cast_shapes(
            &Pose::translation(probe_center.x, probe_center.y, probe_center.z),
            Vector::new(sweep_dir.x * 2.0, sweep_dir.y * 2.0, sweep_dir.z * 2.0),
            capsule,
            &Pose::identity(),
            Vector::ZERO,
            &mesh_collider.mesh,
            options,
        ) {
            // Something taller than the step blocks the way.
            return false;
        }
    }

    // Find the surface under the step destination.
    let final_center = probe_center + sweep_dir;
    let down_ray = Ray::new(
        Vector::new(final_center.x, final_center.y + 0.5, final_center.z),
        Vector::new(0.0, -1.0, 0.0),
    );

    let mut surface_y: Option<f32> = None;
    for mesh_collider in mesh_query.iter() {
        if let Some(toi) = mesh_collider.mesh.cast_local_ray(&down_ray, 2.0, true) {
            let y = down_ray.origin.y - toi;
            surface_y = Some(match surface_y {
                Some(prev) => prev.max(y),
                None => y,
            });
        }
    }

    let Some(surface_y) = surface_y else { return false };

    let original_feet_y = old_center.y - player_half_height;
    let step_height = surface_y - original_feet_y;
    if step_height > config.step_up_height + 0.05 || step_height < -0.1 {
        return false;
    }

    // Land on the surface at the full forward sweep distance.
    position.0 = Vec3::new(final_center.x, surface_y + player_half_height, final_center.z);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_rapier3d::rapier::parry::shape::TriMesh;
    use bevy_rapier3d::rapier::parry::math::Vector;

    fn wall_mesh() -> TriMesh {
        let verts = vec![
            Vector::new(-5.0, 0.0, 0.0),
            Vector::new( 5.0, 0.0, 0.0),
            Vector::new( 5.0, 5.0, 0.0),
            Vector::new(-5.0, 5.0, 0.0),
        ];
        let idx = vec![[0, 1, 2], [0, 2, 3]];
        TriMesh::new(verts, idx).unwrap()
    }

    fn capsule() -> Capsule {
        Capsule::new_y(0.9, 0.4)
    }

    /// Sweep `capsule` from `start` by `sweep` against `mesh`.
    /// Panics if the TOI doesn't match `expect_toi` within `tolerance`.
    fn check_sweep(mesh: &TriMesh, start: Vec3, sweep: Vec3, expect_toi: f32, tolerance: f32) {
        let result = cast_shapes(
            &Pose::translation(start.x, start.y, start.z),
            Vector::new(sweep.x, sweep.y, sweep.z), &capsule(),
            &Pose::identity(), Vector::ZERO, mesh,
            ShapeCastOptions {
                max_time_of_impact: 1.0,
                target_distance: 0.001,
                stop_at_penetration: true,
                compute_impact_geometry_on_penetration: true,
            },
        );
        match result.unwrap() {
            Some(hit) => {
                let diff = (hit.time_of_impact - expect_toi).abs();
                assert!(diff <= tolerance,
                    "TOI {:.4} differs from expected {:.4} by {:.4} (start {:?} sweep {:?})",
                    hit.time_of_impact, expect_toi, diff, start, sweep);
            }
            None => {
                if expect_toi < 1.0 {
                    panic!("expected hit at TOI {:.4} but no hit detected (start {:?} sweep {:?})",
                        expect_toi, start, sweep);
                }
            }
        }
    }

    #[test]
    fn stops_at_wall() {
        let m = wall_mesh();
        // Sweep from 4 units away toward wall at z=0 — hit at ~0.9 (capsule radius 0.4)
        check_sweep(&m, Vec3::new(0.0, 0.9, 4.0), Vec3::new(0.0, 0.0, -4.0), 0.9, 0.02);
        // Sweep from 1 unit away
        check_sweep(&m, Vec3::new(0.0, 0.9, 1.0), Vec3::new(0.0, 0.0, -1.0), 0.6, 0.02);
    }

    #[test]
    fn fast_no_tunnel() {
        // Sweeping fast through the wall from behind must be caught
        check_sweep(&wall_mesh(),
            Vec3::new(0.0, 0.9, 5.0), Vec3::new(0.0, 0.0, -12.0), 0.383, 0.02);
    }

    #[test]
    fn push_normal_points_away() {
        let m = wall_mesh();
        let result = cast_shapes(
            &Pose::translation(0.0, 0.9, 3.0),
            Vector::new(0.0, 0.0, -5.0), &capsule(),
            &Pose::identity(), Vector::ZERO, &m,
            ShapeCastOptions {
                max_time_of_impact: 1.0,
                target_distance: 0.001,
                stop_at_penetration: true,
                compute_impact_geometry_on_penetration: true,
            },
        );
        if let Some(hit) = result.unwrap() {
            assert!(hit.normal2.z > 0.5,
                "wall normal should point in +Z, got z={:.4}", hit.normal2.z);
        }
    }

    #[test]
    fn floor_detected() {
        let verts = vec![
            Vector::new(-5.0, 0.0, -5.0),
            Vector::new( 5.0, 0.0, -5.0),
            Vector::new( 5.0, 0.0,  5.0),
            Vector::new(-5.0, 0.0,  5.0),
        ];
        let idx = vec![[0, 1, 2], [0, 2, 3]];
        let floor = TriMesh::new(verts, idx).unwrap();
        let cap = Capsule::new_y(0.01, 0.4);

        // Capsule on floor: sweeping down should hit immediately
        let r1 = cast_shapes(
            &Pose::translation(0.0, 0.01, 0.0),
            Vector::new(0.0, -0.1, 0.0), &cap,
            &Pose::identity(), Vector::ZERO, &floor,
            ShapeCastOptions {
                max_time_of_impact: 1.0, target_distance: 0.001,
                stop_at_penetration: true, compute_impact_geometry_on_penetration: true,
            },
        );
        assert!(r1.unwrap().is_some(), "capsule on floor must detect floor");

        // Capsule 1 unit above: sweep down should hit near the floor
        let r2 = cast_shapes(
            &Pose::translation(0.0, 1.0, 0.0),
            Vector::new(0.0, -1.0, 0.0), &cap,
            &Pose::identity(), Vector::ZERO, &floor,
            ShapeCastOptions {
                max_time_of_impact: 1.0, target_distance: 0.001,
                stop_at_penetration: true, compute_impact_geometry_on_penetration: true,
            },
        );
        if let Some(h) = r2.unwrap() {
            assert!(h.time_of_impact > 0.5,
                "floor sweep from 1u should hit well before the end, got TOI={:.4}", h.time_of_impact);
        }
    }

    #[test]
    fn tangent_slide() {
        // Sweep into the wall at a 45° angle — should stop along the wall normal
        // while allowing movement parallel to the wall.
        let m = wall_mesh();
        let result = cast_shapes(
            &Pose::translation(0.0, 0.9, 4.0),
            Vector::new(-1.0, 0.0, -3.0), &capsule(),
            &Pose::identity(), Vector::ZERO, &m,
            ShapeCastOptions {
                max_time_of_impact: 1.0, target_distance: 0.001,
                stop_at_penetration: true, compute_impact_geometry_on_penetration: true,
            },
        );
        if let Some(hit) = result.unwrap() {
            // The hit normal's Z should dominate (wall face), X component should be small
            assert!(hit.normal2.z.abs() > hit.normal2.x.abs(),
                "wall hit normal z={:.4} should dominate x={:.4}", hit.normal2.z, hit.normal2.x);
        }
    }

    #[test]
    fn brush_contact_preserves_slide() {
        use bevy::time::Time;

        // Brushing a wall: the capsule starts slightly penetrated and moves
        // parallel to the wall face. The resolver must NOT snap the position
        // back to the sweep origin (that zeroes forward progress) — it should
        // keep the along-wall position/velocity and only strip the normal
        // component.
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: wall_mesh() });
        world.spawn((
            PhysicalTranslation(Vec3::new(0.0, 0.9, 0.35)),
            Velocity(Vec3::new(3.0, 0.0, 0.0)),
            CrouchHeight { current: 1.8, target: 1.8 },
            MovementConfig::default(),
            GroundedState { is_grounded: true, ..default() },
        ));

        let system_id = world.register_system(resolve_collisions);
        world.run_system(system_id).unwrap();

        let (pos, vel) = world
            .query::<(&PhysicalTranslation, &Velocity)>()
            .single(&world)
            .unwrap();
        assert!(pos.0.x >= 0.0,
            "brushing a wall snapped the player backward: x={:.4}", pos.0.x);
        assert!(vel.0.x > 2.0,
            "along-wall velocity was lost: x={:.4}", vel.0.x);
        assert!(vel.0.z >= -0.001,
            "velocity into the wall not removed: z={:.4}", vel.0.z);
    }

    #[test]
    fn near_contact_brush_slides_full_speed() {
        use bevy::time::Time;

        // Brushing with a hair of gap (capsule at z=0.401, radius 0.4 — 1mm
        // from the wall face) moving parallel to it. The sweep reports a
        // contact; the resolver must not snap the position back to the sweep
        // start (which previously made brushing feel like wading through mud).
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: wall_mesh() });
        world.spawn((
            PhysicalTranslation(Vec3::new(0.0, 0.9, 0.401)),
            Velocity(Vec3::new(3.0, 0.0, 0.0)),
            CrouchHeight { current: 1.8, target: 1.8 },
            MovementConfig::default(),
            GroundedState { is_grounded: true, ..default() },
        ));

        let system_id = world.register_system(resolve_collisions);
        // Simulate two frames of the real pipeline: integrate, then resolve.
        let dt = 1.0 / 60.0;
        for _ in 0..2 {
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world.query::<&mut PhysicalTranslation>().single_mut(&mut world).unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            world.run_system(system_id).unwrap();
        }

        let (pos, vel) = world
            .query::<(&PhysicalTranslation, &Velocity)>()
            .single(&world)
            .unwrap();
        // Two frames at 3 m/s / 60 Hz ≈ 0.10 units. A snap-back leaves x ≈ 0.
        assert!(pos.0.x > 0.08,
            "near-contact brushing must keep tangential progress, x={:.4}", pos.0.x);
        assert!(vel.0.x > 2.0,
            "along-wall velocity was lost: x={:.4}", vel.0.x);
    }

    #[test]
    fn head_on_never_passes_through_wall() {
        use bevy::time::Time;

        // Sprint straight into a wall for 2 seconds of frames: the capsule
        // center must never cross the wall plane minus the capsule radius.
        // (Regression: an earlier brushing fix let head-on approaches walk
        // through walls.)
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: wall_mesh() });
        world.spawn((
            PhysicalTranslation(Vec3::new(0.0, 0.9, 2.0)),
            Velocity(Vec3::new(0.0, 0.0, -3.0)),
            CrouchHeight { current: 1.8, target: 1.8 },
            MovementConfig::default(),
            GroundedState { is_grounded: true, ..default() },
        ));

        let system_id = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        let mut min_z = f32::MAX;
        for _ in 0..120 {
            // Player holds forward: keep pushing into the wall via velocity.
            let mut vel = world.query::<&mut Velocity>().single_mut(&mut world).unwrap();
            vel.0.z = -3.0;
            drop(vel);
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world.query::<&mut PhysicalTranslation>().single_mut(&mut world).unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            // Advance the fixed clock so the resolver sees dt > 0.
            world.resource_mut::<Time::<Fixed>>().advance_by(std::time::Duration::from_secs_f32(dt));
            world.run_system(system_id).unwrap();
            // Track the POST-resolve position — the pre-resolve position
            // legitimately dips a frame's worth into the wall before the
            // resolver pushes it back out.
            let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
            min_z = min_z.min(pos.0.z);
        }

        let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
        let wall_face = 0.0;
        let radius = 0.4;
        assert!(pos.0.z >= wall_face + radius - 0.02,
            "player passed through the wall: z={:.4} (wall face at {wall_face}, radius {radius})", pos.0.z);
        assert!(min_z >= wall_face + radius - 0.02,
            "player crossed the wall plane mid-simulation: min z={:.4}", min_z);
    }

    #[test]
    fn slow_brush_keeps_full_speed() {
        use bevy::time::Time;

        // Brushing at walking pace (1 m/s) along a wall in contact — must not
        // be slowed by the collision resolver.
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: wall_mesh() });
        world.spawn((
            PhysicalTranslation(Vec3::new(0.0, 0.9, 0.401)),
            Velocity(Vec3::new(1.0, 0.0, 0.0)),
            CrouchHeight { current: 1.8, target: 1.8 },
            MovementConfig::default(),
            GroundedState { is_grounded: true, ..default() },
        ));

        let system_id = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        for _ in 0..10 {
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world.query::<&mut PhysicalTranslation>().single_mut(&mut world).unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            world.run_system(system_id).unwrap();
        }

        let (pos, vel) = world
            .query::<(&PhysicalTranslation, &Velocity)>()
            .single(&world)
            .unwrap();
        // 10 frames at 1 m/s ≈ 0.167 units; snap-back would leave x ≈ 0.
        assert!(pos.0.x > 0.15,
            "slow brushing must keep tangential progress, x={:.4}", pos.0.x);
        assert!(vel.0.x > 0.8,
            "along-wall velocity was lost: x={:.4}", vel.0.x);
    }
}
