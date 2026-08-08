use bevy::prelude::*;
use bevy_rapier3d::rapier::parry::query::{cast_shapes, ShapeCastOptions, Ray, RayCast};
use bevy_rapier3d::rapier::parry::query::contact::contact;
use bevy_rapier3d::rapier::parry::shape::Capsule;
use bevy_rapier3d::rapier::parry::math::{Pose, Vector};

use super::components::*;
use super::config::{MovementConfig, WALKABLE_SLOPE_THRESHOLD};
use super::ground_detection::ramp_surface_y;
use crate::gameplay::Health;
use crate::world::objects::{MeshCollider, RampCollider};

/// Max distance a single CCD substep sweeps (meters). Sprint (16 m/s) yields
/// ~3 substeps per frame; shorter sweeps make every cast robust so fast
/// movement can't tunnel through walls.
const MAX_SUBSTEP: f32 = 0.12;
/// Hard cap on substeps per frame (~0.96 m/frame ≈ 58 m/s — far beyond any
/// gameplay speed).
const MAX_SUBSTEPS: usize = 8;
/// How far below the feet a walkable surface is snapped to (slope riding).
const SURFACE_SNAP_DIST: f32 = 0.3;
/// Cap on a single depenetration push (meters).
const MAX_DEPENETRATION: f32 = 0.25;
/// A contact is a floor/slope (rideable) only if it sits within this height
/// of the surface the player is standing on. Higher contacts are ledges or
/// corner grazes — walls.
const CLIMB_CONTACT_HEIGHT: f32 = 0.5;

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

        // ── Triangle mesh collider collision (CCD substeps) ──
        // The capsule is centered at feet + half_height + radius so its BOTTOM
        // sits exactly at the feet. (Previously the center sat at feet +
        // half_height, leaving the bottom hemisphere hanging radius units
        // BELOW the feet — when standing on top of a box that hemisphere
        // poked through the top face and overlapped the box's SIDE faces near
        // edges. Those side-face contacts report horizontal normals, so the
        // resolver pushed the player sideways — "sliding" off box tops.)
        let player_half_height = player_height * 0.5;
        let capsule = Capsule::new_y(player_half_height.max(0.01), player_radius);

        let movement = velocity.0 * dt;
        if movement.length_squared() > 0.0001 {
            // The integration already moved the position by `movement`.
            // Rewind to the frame start so the substeps sweep the ENTIRE
            // segment — a sweep that starts at the integrated position
            // misses geometry the segment already crossed (the fast
            // "phase through walls" bug).
            position.0 -= movement;

            // ── Walkable surface snap (slope riding) ──
            // Snap the feet onto any walkable surface directly below (slopes,
            // box tops, stairs) and ride it: zero the fall and follow the
            // surface as it rises/falls. This is how UE4-style movement walks
            // up ramps instead of fighting the surface normal every frame.
            // The snapped surface also decides whether a hit is a floor (a
            // contact within a climbable height of it) or a wall (a ledge or
            // a corner graze whose contact normal happens to look walkable).
            // Runs on the REWOUND (frame-start) position — sampling the
            // integrated position misses surfaces at exact edges.
            let mut snapped = false;
            let mut stand_surface_y = 0.0f32;
            if velocity.y <= 0.1 {
                let foot_ray = Ray::new(
                    Vector::new(position.x, position.y + 0.02, position.z),
                    Vector::new(0.0, -1.0, 0.0),
                );
                let mut snap: Option<(f32, Vec3)> = None;
                for mesh_collider in mesh_query.iter() {
                    if let Some(hit) = mesh_collider
                        .mesh
                        .cast_local_ray_and_get_normal(&foot_ray, SURFACE_SNAP_DIST, true)
                    {
                        if hit.normal.y > WALKABLE_SLOPE_THRESHOLD {
                            let y = position.y + 0.02 - hit.time_of_impact;
                            if snap.map_or(true, |(prev_y, _)| y > prev_y) {
                                snap = Some((
                                    y,
                                    Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z),
                                ));
                            }
                        }
                    }
                }
                if let Some((surface_y, _surface_normal)) = snap {
                    let diff = position.y - surface_y;
                    if diff <= SURFACE_SNAP_DIST && diff >= -0.05 {
                        position.y = surface_y;
                        snapped = true;
                        stand_surface_y = surface_y;
                        if velocity.y < 0.0 {
                            velocity.y = 0.0;
                        }
                    }
                }
            }

            let frame_center =
                Vec3::new(position.x, position.y + player_half_height + player_radius, position.z);

            // CCD substeps with a movement budget: each pass advances at most
            // MAX_SUBSTEP and `remaining` shrinks, so fast frames can't
            // tunnel and a brushing player keeps the frame's full movement.
            let mut remaining = movement;
            let mut stepped = false;
            for _ in 0..MAX_SUBSTEPS {
                let rlen = remaining.length();
                if rlen < 0.001 {
                    break;
                }
                let chunk = remaining * ((MAX_SUBSTEP / rlen).min(1.0));
                let old_center =
                    Vec3::new(position.x, position.y + player_half_height + player_radius, position.z);
                let sweep_dir = chunk;

                let mut earliest_toi = 2.0f32;
                let mut best_normal = Vector::ZERO;
                let mut best_witness = Vector::ZERO;

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
                            best_witness = hit.witness2;
                        }
                    }
                }

                if earliest_toi > 1.0 {
                    position.0 += chunk;
                    remaining -= chunk;
                    continue;
                }

                let pseudo_normal = Vec3::new(best_normal.x, best_normal.y, best_normal.z);
                let witness = Vec3::new(best_witness.x, best_witness.y, best_witness.z);
                // The TRUE surface normal (triangle winding), not the witness
                // pseudo-normal — used for the walkable gate and the velocity
                // projection so walls push straight back, never sideways.
                // Flipped to agree with the pseudo-normal's direction: the
                // pseudo always points OUT of the geometry (closest-point
                // direction), so inward-wound meshes (e.g. the cone in the
                // testing grounds) get corrected to their real outward side.
                let mut normal = true_surface_normal(witness, pseudo_normal, &mesh_query)
                    .unwrap_or(pseudo_normal);
                if normal.dot(pseudo_normal) < 0.0 {
                    normal = -normal;
                }
                // A hit is a floor/slope when EITHER:
                //  - the feet are on a walkable surface, the contact sits
                //    within a climbable height of it, AND the contact normal
                //    is walkable (ramps, stairs, box tops), or
                //  - the contact is directly under the feet (flat ground —
                //    parry's resting-contact normal is a witness pseudo-
                //    normal that goes diagonal on big flat triangles, so the
                //    normal gate can't be required here).
                // Everything else — wall faces, corner grazes, cone sides —
                // is a wall: the normal gate keeps low wall contacts from
                // being misclassified as floors (which let the player clip
                // into or slide along walls while brushing).
                if position.z > 37.9 && position.z < 39.5 {
                }
                let directly_below = witness.y <= position.y + 0.15
                    && Vec3::new(witness.x, 0.0, witness.z)
                        .distance(Vec3::new(position.x, 0.0, position.z))
                        <= 0.15;
                let walkable = directly_below
                    || (snapped
                        && normal.y > WALKABLE_SLOPE_THRESHOLD
                        && witness.y - stand_surface_y <= CLIMB_CONTACT_HEIGHT);

                // ── Auto step-up (once per frame, from the frame start) ──
                // Only while grounded (runs after GroundDetection) — an
                // airborne player near a wall must not be lifted at the top
                // of a jump.
                if !stepped && !walkable && velocity.y <= 0.5 && ground.is_grounded {
                    stepped = true;
                    if try_step_up(
                        &mut position,
                        frame_center,
                        movement,
                        &capsule,
                        &mesh_query,
                        config,
                        player_half_height,
                    ) {
                        break;
                    }
                }

                if walkable {
                    // Floor/slope: never bounce, launch, or stop — the
                    // surface snap follows the ground and zeroes the fall,
                    // and the feet are lifted onto the surface. Stopping on
                    // a walkable contact would eat forward progress while
                    // riding a slope.
                    if velocity.y < 0.0 {
                        velocity.y = 0.0;
                    }
                    // Lift the feet onto the surface (vertical only, capped)
                    // so rising slopes and steps are climbed smoothly. Only
                    // genuine floor contacts count — a wall graze's depth
                    // must not pop the player up (that jittered the position
                    // when walking next to obstacles).
                    let center = Vec3::new(
                        position.x,
                        position.y + player_half_height + player_radius,
                        position.z,
                    );
                    let pose = Pose::translation(center.x, center.y, center.z);
                    let mut lift = 0.0f32;
                    for mesh_collider in mesh_query.iter() {
                        if let Ok(Some(c)) = contact(
                            &pose,
                            &capsule,
                            &Pose::identity(),
                            &mesh_collider.mesh,
                            0.01,
                        ) {
                            if c.dist < 0.0 && c.normal2.y > WALKABLE_SLOPE_THRESHOLD {
                                lift = lift.max(-c.dist);
                            }
                        }
                    }
                    if lift > 0.001 {
                        position.y += lift.min(MAX_DEPENETRATION);
                    }
                    position.0 += chunk;
                    remaining -= chunk;
                    continue;
                }

                // ── Wall: advance the chunk, then depenetrate ──
                // Advancing first lets a brushing player slide along the
                // wall at full speed (the depenetration only strips the
                // overlap); a head-on approach nets back to the impact
                // point because the depenetration pushes out exactly what
                // was advanced.
                position.0 += chunk;
                remaining -= chunk;
                depenetrate(
                    &mut position,
                    &capsule,
                    &mesh_query,
                    normal,
                    player_half_height,
                    player_radius,
                    velocity.y < 0.0,
                    (-velocity.0.dot(normal) * dt).max(0.0),
                );

                // Wall: remove only the into-wall HORIZONTAL component.
                // Vertical velocity is never touched by wall hits — a
                // diagonal edge/corner normal must not launch the player
                // upward (the old "jump when hitting walls" bug).
                let n_h = Vec3::new(normal.x, 0.0, normal.z).normalize_or_zero();
                if n_h.length_squared() > 0.5 {
                    let v_h = Vec3::new(velocity.x, 0.0, velocity.z);
                    let into = v_h.dot(n_h);
                    if into < 0.0 {
                        velocity.x -= n_h.x * into;
                        velocity.z -= n_h.z * into;
                    }
                }

                if velocity.0.length_squared() < 1e-6 {
                    break;
                }
            }

            // ── Overlap recovery ──
            // The swept casts can MISS grazing contacts against thin/complex
            // geometry (GJK vs zero-thickness triangles is unreliable
            // edge-on — the cone's side and the torus's tube let players
            // walk through them). If the capsule ends the frame overlapping
            // anything that ISN'T the floor beneath it (the floor overlap is
            // legitimate riding/resting, handled by the snap + lift), push it
            // straight out along the contact direction.
            let center = Vec3::new(
                position.x,
                position.y + player_half_height + player_radius,
                position.z,
            );
            let pose = Pose::translation(center.x, center.y, center.z);
            let mut deepest = 0.0f32;
            let mut contact_point = Vec3::ZERO;
            let mut pseudo = Vec3::Y;
            for mesh_collider in mesh_query.iter() {
                if let Ok(Some(c)) =
                    contact(&pose, &capsule, &Pose::identity(), &mesh_collider.mesh, 0.0)
                {
                    let p = Vec3::new(c.point2.x, c.point2.y, c.point2.z);
                    // The legitimate floor/riding overlap is directly BELOW
                    // the feet (the capsule's bottom in the ground/slope);
                    // overlaps at body height are geometry the player walked
                    // into (a missed sweep) and must be pushed out. Floor-
                    // like contacts (normal mostly up) are riding — skip.
                    let under_feet = p.y <= position.y + 0.15
                        && Vec3::new(p.x, 0.0, p.z)
                            .distance(Vec3::new(position.x, 0.0, position.z))
                            <= 0.15;
                    let floor_like = c.normal2.y > WALKABLE_SLOPE_THRESHOLD;
                    if c.dist < deepest && !under_feet && !floor_like {
                        deepest = c.dist;
                        contact_point = p;
                        pseudo = Vec3::new(c.normal2.x, c.normal2.y, c.normal2.z);
                    }
                }
            }
            // Only fire for a MEANINGFUL overlap: the swept casts miss
            // grazing contacts against thin/complex geometry (the torus's
            // tube) and the capsule ends up clearly inside — but the shallow
            // riding contact against a slope/side the sweep DID catch must
            // not push the player back (it would eat the wall-slide).
            if deepest < -0.05 {
                // Push out along the TRUE outward surface normal (flipped to
                // agree with the contact normal), HORIZONTAL only: the player
                // is pushed out of the geometry's side — never sunk into the
                // ground, never climbed over the obstacle.
                let mut n = true_surface_normal(contact_point, pseudo, &mesh_query)
                    .unwrap_or(pseudo);
                if n.dot(pseudo) < 0.0 {
                    n = -n;
                }
                let mut d = Vec3::new(n.x, 0.0, n.z).normalize_or_zero();
                if d.length_squared() < 0.5 {
                    d = Vec3::Y;
                }
                position.0 += d * ((-deepest).min(MAX_DEPENETRATION) + 0.001);
            }
        }
    }
}

/// Push the player out of penetrating geometry along the true contact normal
/// by the true penetration depth (capped, iterated).
///
/// Rules:
/// - Wall-like contacts are pushed horizontally only — never vertically — so
///   edge/corner geometry can't pop the player up.
/// - A non-falling player is never lifted: floor-ish contacts resolve along
///   the horizontal approach direction instead (a corner graze must not lift
///   the feet, and the surface snap handles genuine floors).
/// - When no contact is found at all (a capsule fully past an open,
///   zero-thickness surface), push back along the cast normal by the frame's
///   into-surface movement (bounded).
fn depenetrate(
    position: &mut PhysicalTranslation,
    capsule: &Capsule,
    mesh_query: &Query<&MeshCollider>,
    cast_normal: Vec3,
    player_half_height: f32,
    player_radius: f32,
    falling: bool,
    into_depth: f32,
) {
    let mut feet = position.0;
    for _ in 0..3 {
        let center = Vec3::new(feet.x, feet.y + player_half_height + player_radius, feet.z);
        let pose = Pose::translation(center.x, center.y, center.z);

        let mut deepest = 0.0f32;
        let mut push_dir = cast_normal;
        let mut found_contact = false;
        // Prediction ~1cm: a capsule touching a surface (even a hair of
        // gap) reports a contact. Without it, "touching" looked like "no
        // contact" and the fallback shoved the player ~0.25m+ out of walls
        // every frame they pressed against one.
        for mesh_collider in mesh_query.iter() {
            if let Ok(Some(c)) =
                contact(&pose, capsule, &Pose::identity(), &mesh_collider.mesh, 0.01)
            {
                found_contact = true;
                if c.dist < deepest {
                    deepest = c.dist;
                    push_dir = Vec3::new(c.normal2.x, c.normal2.y, c.normal2.z);
                }
            }
        }

        if !found_contact {
            // The capsule is fully past an open (zero-thickness) surface —
            // no contact anywhere within range. Push back along the cast
            // normal by the frame's into-surface movement (bounded); the
            // re-check then finds the surface and stops.
            let mut dir = Vec3::new(cast_normal.x, 0.0, cast_normal.z).normalize_or_zero();
            if dir.length_squared() < 0.5 {
                dir = cast_normal.normalize_or_zero();
            }
            if dir.length_squared() < 0.5 {
                break;
            }
            feet += dir * into_depth.min(0.6);
            continue;
        }

        if deepest >= -0.0005 {
            // Touching or separated within the prediction — nothing to
            // resolve; the sweep's target_distance already accounts for it.
            break;
        }

        let mut dir = push_dir.normalize_or_zero();
        if !falling {
            let h = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
            dir = if h.length_squared() > 0.5 {
                h
            } else {
                Vec3::new(cast_normal.x, 0.0, cast_normal.z).normalize_or_zero()
            };
        } else if dir.y < WALKABLE_SLOPE_THRESHOLD {
            let h = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
            if h.length_squared() > 0.5 {
                dir = h;
            }
        }

        if dir.length_squared() < 0.5 {
            break;
        }

        feet += dir * ((-deepest).min(MAX_DEPENETRATION) + 0.001);
    }
    position.0 = feet;
}

/// The TRUE surface normal at a contact point. parry's shape-cast contact
/// normal is a witness-based pseudo-normal that goes diagonal on flat
/// surfaces and near edges — projecting velocity along it slides the player
/// sideways instead of straight out of walls. A tiny ray from just outside
/// the surface, cast INTO it, reports the actual triangle winding normal.
fn true_surface_normal(
    witness: Vec3,
    pseudo_normal: Vec3,
    mesh_query: &Query<&MeshCollider>,
) -> Option<Vec3> {
    let n = pseudo_normal.normalize_or_zero();
    if n.length_squared() < 0.5 {
        return None;
    }
    let origin = witness + n * 0.001;
    let ray = Ray::new(Vector::new(origin.x, origin.y, origin.z), Vector::new(-n.x, -n.y, -n.z));
    let mut best: Option<Vec3> = None;
    let mut best_toi = f32::MAX;
    for mesh_collider in mesh_query.iter() {
        if let Some(hit) = mesh_collider
            .mesh
            .cast_local_ray_and_get_normal(&ray, 0.2, true)
        {
            if hit.time_of_impact < best_toi {
                best_toi = hit.time_of_impact;
                best = Some(Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z));
            }
        }
    }
    best
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

    let original_feet_y = old_center.y - player_half_height - config.player_radius;
    let step_height = surface_y - original_feet_y;
    if step_height > config.step_up_height + 0.05 || step_height < -0.1 {
        return false;
    }

    // Land with the feet exactly on the surface (position is the feet; the
    // capsule center sits half_height + radius above them).
    position.0 = Vec3::new(final_center.x, surface_y, final_center.z);
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

    /// Outward-wound 2×2×2 box at the origin (top face at y=1.0).
    fn box_mesh() -> TriMesh {
        let verts = vec![
            // +Y top (normal up)
            Vector::new(-1.0, 1.0, 1.0), Vector::new(1.0, 1.0, 1.0),
            Vector::new(1.0, 1.0, -1.0), Vector::new(-1.0, 1.0, -1.0),
            // +X side
            Vector::new(1.0, -1.0, -1.0), Vector::new(1.0, 1.0, -1.0),
            Vector::new(1.0, 1.0, 1.0), Vector::new(1.0, -1.0, 1.0),
            // -X side
            Vector::new(-1.0, -1.0, 1.0), Vector::new(-1.0, 1.0, 1.0),
            Vector::new(-1.0, 1.0, -1.0), Vector::new(-1.0, -1.0, -1.0),
            // +Z side
            Vector::new(-1.0, -1.0, 1.0), Vector::new(1.0, -1.0, 1.0),
            Vector::new(1.0, 1.0, 1.0), Vector::new(-1.0, 1.0, 1.0),
            // -Z side
            Vector::new(-1.0, -1.0, -1.0), Vector::new(1.0, 1.0, -1.0),
            Vector::new(1.0, -1.0, -1.0), Vector::new(-1.0, 1.0, -1.0),
            // -Y bottom
            Vector::new(-1.0, -1.0, 1.0), Vector::new(1.0, -1.0, -1.0),
            Vector::new(1.0, -1.0, 1.0), Vector::new(-1.0, -1.0, -1.0),
        ];
        let idx = vec![
            [0, 1, 2], [0, 2, 3],
            [4, 5, 6], [4, 6, 7],
            [8, 9, 10], [8, 10, 11],
            [12, 13, 14], [12, 14, 15],
            [16, 17, 18], [16, 19, 17],
            [20, 21, 22], [20, 23, 21],
        ];
        TriMesh::new(verts, idx).unwrap()
    }

    /// Simulate one fixed frame of gravity → integration → collision
    /// resolution for the single player in `world`. Mirrors the real
    /// pipeline: gravity is skipped while the feet ray finds a surface.
    fn step_frame(world: &mut World, dt: f32) {
        let grounded = {
            let pos = *world.query::<&PhysicalTranslation>().single(world).unwrap();
            let ray = Ray::new(
                Vector::new(pos.0.x, pos.0.y + 0.02, pos.0.z),
                Vector::new(0.0, -1.0, 0.0),
            );
            world
                .query::<&MeshCollider>()
                .iter(world)
                .any(|mc| mc.mesh.cast_local_ray(&ray, 0.24, true).is_some())
        };
        let mut vel = world.query::<&mut Velocity>().single_mut(world).unwrap();
        if !grounded {
            vel.0.y -= 20.0 * dt;
        }
        drop(vel);
        let vel = *world.query::<&Velocity>().single(world).unwrap();
        let mut pos = world
            .query::<&mut PhysicalTranslation>()
            .single_mut(world)
            .unwrap();
        pos.0 += vel.0 * dt;
        drop(pos);
        let sys = world
            .get_resource::<ResolverId>()
            .copied()
            .unwrap_or_else(|| {
                let id = world.register_system(resolve_collisions);
                world.insert_resource(ResolverId(id));
                ResolverId(id)
            });
        world.run_system(sys.0).unwrap();
    }

    #[derive(Resource, Copy, Clone)]
    struct ResolverId(bevy::ecs::system::SystemId<(), ()>);

    fn spawn_player(world: &mut World, feet: Vec3, velocity: Vec3) {
        world.spawn((
            PhysicalTranslation(feet),
            Velocity(velocity),
            CrouchHeight { current: 1.5, target: 1.5 },
            MovementConfig::default(),
            GroundedState { is_grounded: true, ..default() },
        ));
    }

    #[test]
    fn stands_stable_on_box_top_near_edge() {
        use bevy::time::Time;
        use std::time::Duration;

        // Regression: standing on top of a trimesh box near an edge. The
        // capsule's bottom hemisphere used to hang radius units BELOW the
        // feet, overlapping the box's side faces near edges; those contacts
        // report horizontal normals and the resolver pushed the player
        // sideways — "sliding" off the box with no way to counter it.
        for (x, label) in [(0.7, "0.7"), (0.85, "0.85"), (0.97, "0.97")] {
            let mut world = World::new();
            world.insert_resource(Time::<Fixed>::from_hz(60.0));
            world.spawn(MeshCollider { mesh: box_mesh() });
            spawn_player(&mut world, Vec3::new(x, 1.0, 0.0), Vec3::ZERO);

            let dt = 1.0 / 60.0;
            for _ in 0..240 {
                world
                    .resource_mut::<Time::<Fixed>>()
                    .advance_by(Duration::from_secs_f32(dt));
                step_frame(&mut world, dt);
            }

            let (pos, vel) = world
                .query::<(&PhysicalTranslation, &Velocity)>()
                .single(&world)
                .unwrap();
            assert!(
                (pos.0.x - x).abs() < 0.05,
                "[{label}] player slid off the box top: x={:.4} (start {x})",
                pos.0.x
            );
            assert!(
                (pos.0.y - 1.0).abs() < 0.02,
                "[{label}] player left the box top surface: y={:.4}",
                pos.0.y
            );
            assert!(
                vel.0.length() < 0.1,
                "[{label}] player accumulated velocity on a box top: {:.4}",
                vel.0.length()
            );
        }
    }

    #[test]
    fn walks_along_box_edge_without_sliding_off() {
        use bevy::time::Time;
        use std::time::Duration;

        // Walking along the edge of a box top must not drift sideways: the
        // capsule's bottom sphere (below the feet) used to overlap the side
        // face and get pushed outward every frame, and its horizontal normal
        // also ate the walk velocity.
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: box_mesh() });
        spawn_player(&mut world, Vec3::new(0.85, 1.0, 0.0), Vec3::new(0.0, 0.0, 2.0));

        let dt = 1.0 / 60.0;
        for _ in 0..60 {
            world
                .resource_mut::<Time::<Fixed>>()
                .advance_by(Duration::from_secs_f32(dt));
            step_frame(&mut world, dt);
        }

        let (pos, vel) = world
            .query::<(&PhysicalTranslation, &Velocity)>()
            .single(&world)
            .unwrap();
        // 60 frames at 2 m/s ≈ 2.0m of travel along +Z (box spans z∈[-1,1],
        // so the player walks past the +Z edge and keeps going over the open
        // ground beyond — that's fine as long as they never drifted in X).
        assert!(
            (pos.0.x - 0.85).abs() < 0.05,
            "player drifted sideways while walking along the box edge: x={:.4} (start 0.85)",
            pos.0.x
        );
        assert!(
            pos.0.z > 1.5,
            "player lost forward progress along the box edge: z={:.4}",
            pos.0.z
        );
        assert!(
            vel.0.z > 1.5,
            "walk velocity along the box edge was eaten: vz={:.4}",
            vel.0.z
        );
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
    fn presses_into_wall_without_bounce() {
        use bevy::time::Time;

        // Holding forward into a wall: the capsule must rest against the
        // wall, not get shoved backward repeatedly. (Regression: the
        // depenetration's "no contact" fallback fired for touching-but-
        // separated contacts and bounced the player ~0.25m+ per frame.)
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: wall_mesh() });
        spawn_player(&mut world, Vec3::new(0.0, 0.9, 2.0), Vec3::new(0.0, 0.0, -6.0));

        let system_id = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;
        for _ in 0..120 {
            // Player holds forward the whole time.
            let mut vel = world.query::<&mut Velocity>().single_mut(&mut world).unwrap();
            vel.0.z = -6.0;
            drop(vel);
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world
                .query::<&mut PhysicalTranslation>()
                .single_mut(&mut world)
                .unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            world.resource_mut::<Time::<Fixed>>().advance_by(std::time::Duration::from_secs_f32(dt));
            world.run_system(system_id).unwrap();
            let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
            min_z = min_z.min(pos.0.z);
            if pos.0.z < 0.45 {
                max_z = max_z.max(pos.0.z);
            }
        }

        let wall_face = 0.0;
        let radius = 0.4;
        assert!(
            min_z >= wall_face + radius - 0.02,
            "player passed through the wall: min z={:.4}",
            min_z
        );
        assert!(
            max_z <= wall_face + radius + 0.15,
            "player was violently pushed out of the wall: max z={:.4} (resting ~{:.4})",
            max_z,
            wall_face + radius
        );
    }

    #[test]
    fn walks_on_flat_ground_without_jitter() {
        use bevy::time::Time;

        // Walking on a flat ground plane beside a box: the feet must stay on
        // the ground — the walkable-lift must not pop the player up from
        // wall-graze contacts (the "jitter for a couple seconds" bug).
        let ground = TriMesh::new(
            vec![
                Vector::new(-6.0, 0.0, -6.0),
                Vector::new(6.0, 0.0, -6.0),
                Vector::new(6.0, 0.0, 6.0),
                Vector::new(-6.0, 0.0, 6.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        )
        .unwrap();

        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: ground });
        // A box the player brushes past while walking (top at y=1.0).
        world.spawn(MeshCollider { mesh: box_mesh() });
        spawn_player(&mut world, Vec3::new(1.6, 0.0, -3.0), Vec3::new(0.0, 0.0, 3.0));

        let system_id = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for _ in 0..120 {
            let mut vel = world.query::<&mut Velocity>().single_mut(&mut world).unwrap();
            vel.0.z = 3.0;
            drop(vel);
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world
                .query::<&mut PhysicalTranslation>()
                .single_mut(&mut world)
                .unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            world.resource_mut::<Time::<Fixed>>().advance_by(std::time::Duration::from_secs_f32(dt));
            world.run_system(system_id).unwrap();
            let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
            min_y = min_y.min(pos.0.y);
            max_y = max_y.max(pos.0.y);
        }

        assert!(
            min_y >= -0.02,
            "player fell through the ground: min y={:.4}",
            min_y
        );
        assert!(
            max_y <= 0.02,
            "player jittered off the ground: max y={:.4}",
            max_y
        );
    }

    #[test]
    fn no_tunnel_at_sprint_speed() {
        use bevy::time::Time;

        // Twice sprint speed straight into a wall: the CCD substeps must
        // keep the capsule on the near side of the wall every single frame.
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: wall_mesh() });
        spawn_player(&mut world, Vec3::new(0.0, 0.9, 2.0), Vec3::new(0.0, 0.0, -32.0));

        let system_id = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        let mut min_z = f32::MAX;
        for _ in 0..60 {
            let mut vel = world.query::<&mut Velocity>().single_mut(&mut world).unwrap();
            vel.0.z = -32.0;
            drop(vel);
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world
                .query::<&mut PhysicalTranslation>()
                .single_mut(&mut world)
                .unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            world.resource_mut::<Time::<Fixed>>().advance_by(std::time::Duration::from_secs_f32(dt));
            world.run_system(system_id).unwrap();
            let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
            min_z = min_z.min(pos.0.z);
        }

        let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
        let wall_face = 0.0;
        let radius = 0.4;
        assert!(
            pos.0.z >= wall_face + radius - 0.02,
            "player phased through the wall at sprint speed: z={:.4}",
            pos.0.z
        );
        assert!(
            min_z >= wall_face + radius - 0.02,
            "player crossed the wall plane mid-simulation: min z={:.4}",
            min_z
        );
    }

    #[test]
    fn wall_contact_never_launches_vertically() {
        use bevy::time::Time;

        // Sprint into a box corner: the impact must only strip the into-wall
        // horizontal velocity. Vertical velocity (and height) must never
        // change — a diagonal edge/corner normal used to launch the player
        // upward ("jump when hitting walls").
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: box_mesh() });
        // Diagonal run aimed at the box's +X/+Z corner region.
        spawn_player(
            &mut world,
            Vec3::new(1.7, 0.9, 1.7),
            Vec3::new(-4.0, 0.0, -4.0),
        );

        let system_id = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;

        let mut max_y = 0.0f32;
        for _ in 0..30 {
            let mut vel = world.query::<&mut Velocity>().single_mut(&mut world).unwrap();
            vel.0.x = -4.0;
            vel.0.z = -4.0;
            drop(vel);
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world
                .query::<&mut PhysicalTranslation>()
                .single_mut(&mut world)
                .unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            world.resource_mut::<Time::<Fixed>>().advance_by(std::time::Duration::from_secs_f32(dt));
            world.run_system(system_id).unwrap();
            let (pos, vel) = world
                .query::<(&PhysicalTranslation, &Velocity)>()
                .single(&world)
                .unwrap();
            max_y = max_y.max(pos.0.y);
            assert!(
                vel.0.y.abs() < 0.05,
                "wall contact launched the player vertically: vy={:.4}",
                vel.0.y
            );
        }
        assert!(
            max_y <= 0.95,
            "wall contact raised the player: max y={:.4}",
            max_y
        );
    }

    #[test]
    fn walks_up_slope() {
        use bevy::time::Time;

        // A 30° walkable ramp: the player must ride up it (snap + velocity
        // projection), never stick at the base or fall through.
        // Surface y = 0.577·(z + 2) for z ∈ [-2, 2] (meets the ground at
        // z=-2, top ~2.3m at z=2).
        let verts = vec![
            Vector::new(-2.0, 0.0, -2.0),
            Vector::new(2.0, 0.0, -2.0),
            Vector::new(2.0, 2.309, 2.0),
            Vector::new(-2.0, 2.309, 2.0),
        ];
        let idx = vec![[0, 1, 2], [0, 2, 3]];
        let ramp = TriMesh::new(verts, idx).unwrap();
        let ramp = MeshCollider { mesh: ramp };

        // Ground plane under the whole test (like the real maps — the player
        // stands on the ground while approaching the ramp's base).
        let ground = TriMesh::new(
            vec![
                Vector::new(-6.0, 0.0, -6.0),
                Vector::new(6.0, 0.0, -6.0),
                Vector::new(6.0, 0.0, 6.0),
                Vector::new(-6.0, 0.0, 6.0),
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        )
        .unwrap();

        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: ground });
        world.spawn(ramp);
        spawn_player(&mut world, Vec3::new(0.0, 0.0, -2.5), Vec3::new(0.0, 0.0, 3.0));

        let system_id = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        let mut min_y = f32::MAX;
        for _ in 0..70 {
            let mut vel = world.query::<&mut Velocity>().single_mut(&mut world).unwrap();
            vel.0.z = 3.0;
            drop(vel);
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world
                .query::<&mut PhysicalTranslation>()
                .single_mut(&mut world)
                .unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            world.resource_mut::<Time::<Fixed>>().advance_by(std::time::Duration::from_secs_f32(dt));
            world.run_system(system_id).unwrap();
            let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
            min_y = min_y.min(pos.0.y);
        }

        let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
        assert!(
            min_y >= -0.05,
            "player fell through the ramp: min y={:.4}",
            min_y
        );
        assert!(
            pos.0.z > 0.5 && pos.0.y > 0.8,
            "player did not climb the ramp: z={:.4} y={:.4}",
            pos.0.z,
            pos.0.y
        );
    }

    fn spawn_real_testing_grounds(world: &mut World) {
        let colliders = noctyrn_shared::map_data::load_colliders("testing_grounds");
        let data = noctyrn_shared::map_data::load_map_data("testing_grounds");
        for c in &colliders.colliders {
            if let Some(mc) = MeshCollider::from_json(c, data.scale) {
                world.spawn(mc);
            }
        }
    }

    /// One frame of gravity → integrate → resolve for a walking player on
    /// the real map (mirrors the actual movement pipeline).
    fn step_walk(
        world: &mut World,
        resolver: bevy::ecs::system::SystemId<(), ()>,
        dx: f32,
        dz: f32,
        dt: f32,
    ) {
        let grounded = {
            let pos = *world.query::<&PhysicalTranslation>().single(world).unwrap();
            let ray = Ray::new(
                Vector::new(pos.0.x, pos.0.y + 0.02, pos.0.z),
                Vector::new(0.0, -1.0, 0.0),
            );
            world
                .query::<&MeshCollider>()
                .iter(world)
                .any(|mc| mc.mesh.cast_local_ray(&ray, 0.24, true).is_some())
        };
        let mut vel = world.query::<&mut Velocity>().single_mut(world).unwrap();
        if !grounded {
            vel.0.y -= 20.0 * dt;
        }
        vel.0.x = dx;
        vel.0.z = dz;
        drop(vel);
        let vel = *world.query::<&Velocity>().single(world).unwrap();
        let mut pos = world
            .query::<&mut PhysicalTranslation>()
            .single_mut(world)
            .unwrap();
        pos.0 += vel.0 * dt;
        drop(pos);
        world.run_system(resolver).unwrap();
    }

    #[test]
    fn cone_side_blocks_without_freezing() {
        use bevy::time::Time;
        use std::time::Duration;

        // Walking into the cone's underside on the real map: the player must
        // reach the cone and be blocked at its side — not pass through, and
        // not get immobilized far away.
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        spawn_real_testing_grounds(&mut world);
        // Cone at (3.28, 2.03, 41.4), base radius ~2.62, tilted. Walk into
        // its side from 3m out.
        spawn_player(&mut world, Vec3::new(3.28, 0.0, 35.0), Vec3::new(0.0, 0.0, 6.0));
        let resolver = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        for _ in 0..120 {
            world
                .resource_mut::<Time::<Fixed>>()
                .advance_by(Duration::from_secs_f32(dt));
            step_walk(&mut world, resolver, 0.0, 6.0, dt);
        }
        let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
        // Pushing head-on into the cone's side must block the player at the
        // surface (like a pillar): they reach the cone, never pass through,
        // and stay on the ground — no jitter, no being stuck far away.
        assert!(
            pos.0.z > 37.0,
            "player got stuck before reaching the cone: z={:.3}",
            pos.0.z
        );
        assert!(
            pos.0.z < 44.0,
            "player passed through the cone: z={:.3}",
            pos.0.z
        );
        assert!(
            (pos.0.y - 0.0).abs() < 0.05,
            "player left the ground: y={:.3}",
            pos.0.y
        );
    }

    #[test]
    fn torus_side_blocks() {
        use bevy::time::Time;
        use std::time::Duration;

        // Walking into the torus's side on the real map: it must block like
        // the top (regression: the side acted like there was no collision).
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        spawn_real_testing_grounds(&mut world);
        // Torus at (-2.26, 0.27, 33.01), outer radius ~3.68 — approach +X.
        spawn_player(&mut world, Vec3::new(-8.0, 0.0, 33.01), Vec3::new(6.0, 0.0, 0.0));
        let resolver = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        for _ in 0..120 {
            world
                .resource_mut::<Time::<Fixed>>()
                .advance_by(Duration::from_secs_f32(dt));
            step_walk(&mut world, resolver, 6.0, 0.0, dt);
        }
        let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
        assert!(
            pos.0.x > -7.0,
            "player immobilized before the torus: x={:.3}",
            pos.0.x
        );
        assert!(
            pos.0.x < 2.5,
            "player passed through the torus's side: x={:.3}",
            pos.0.x
        );
    }

    #[test]
    fn sliding_into_wall_preserves_tangential_speed() {
        use bevy::time::Time;

        // Running into a wall at an angle: the into-wall component dies, the
        // tangential slide keeps full speed (regression: brushing froze or
        // heavily slowed the player).
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.spawn(MeshCollider { mesh: wall_mesh() });
        spawn_player(&mut world, Vec3::new(0.0, 0.9, 3.0), Vec3::new(2.0, 0.0, -6.0));
        let resolver = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        for _ in 0..90 {
            let mut vel = world.query::<&mut Velocity>().single_mut(&mut world).unwrap();
            vel.0.x = 2.0;
            vel.0.z = -6.0;
            drop(vel);
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world
                .query::<&mut PhysicalTranslation>()
                .single_mut(&mut world)
                .unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            world.resource_mut::<Time::<Fixed>>().advance_by(std::time::Duration::from_secs_f32(dt));
            world.run_system(resolver).unwrap();
        }
        let (pos, vel) = world
            .query::<(&PhysicalTranslation, &Velocity)>()
            .single(&world)
            .unwrap();
        assert!(
            pos.0.z >= 0.38,
            "player passed through the wall: z={:.3}",
            pos.0.z
        );
        assert!(
            pos.0.x > 2.5,
            "tangential slide was eaten: x={:.3}",
            pos.0.x
        );
        assert!(
            vel.0.x > 1.8,
            "tangential velocity was eaten: vx={:.3}",
            vel.0.x
        );
    }

    #[test]
    fn walks_on_real_testing_grounds() {
        use bevy::time::Time;
        use std::time::Duration;

        // The real baked testing-grounds colliders: the player spawns 1m
        // above the 100×100m ground plane and must be able to walk freely
        // (regression: "cannot move on testing grounds at all").
        let colliders = noctyrn_shared::map_data::load_colliders("testing_grounds");
        let data = noctyrn_shared::map_data::load_map_data("testing_grounds");
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        for c in &colliders.colliders {
            if let Some(mc) = MeshCollider::from_json(c, data.scale) {
                world.spawn(mc);
            }
        }
        spawn_player(&mut world, Vec3::new(0.0, 1.0, 0.0), Vec3::new(6.0, 0.0, 0.0));

        let resolver = world.register_system(resolve_collisions);
        let dt = 1.0 / 60.0;
        for _ in 0..90 {
            // Gravity applies only while airborne (mirrors apply_gravity).
            let grounded = {
                let pos = *world.query::<&PhysicalTranslation>().single(&world).unwrap();
                let ray = Ray::new(
                    Vector::new(pos.0.x, pos.0.y + 0.02, pos.0.z),
                    Vector::new(0.0, -1.0, 0.0),
                );
                world
                    .query::<&MeshCollider>()
                    .iter(&world)
                    .any(|mc| mc.mesh.cast_local_ray(&ray, 0.24, true).is_some())
            };
            let mut vel = world.query::<&mut Velocity>().single_mut(&mut world).unwrap();
            if !grounded {
                vel.0.y -= 20.0 * dt;
            }
            vel.0.x = 6.0;
            drop(vel);
            let vel = *world.query::<&Velocity>().single(&world).unwrap();
            let mut pos = world
                .query::<&mut PhysicalTranslation>()
                .single_mut(&mut world)
                .unwrap();
            pos.0 += vel.0 * dt;
            drop(pos);
            world.resource_mut::<Time::<Fixed>>().advance_by(Duration::from_secs_f32(dt));
            world.run_system(resolver).unwrap();
        }

        let pos = world.query::<&PhysicalTranslation>().single(&world).unwrap();
        assert!(
            pos.0.x > 5.0,
            "player could not move on the testing grounds: x={:.3}",
            pos.0.x
        );
        assert!(
            (pos.0.y - 0.0).abs() < 0.05,
            "player left the ground: y={:.3}",
            pos.0.y
        );
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
