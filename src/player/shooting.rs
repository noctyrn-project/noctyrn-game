







































use bevy::math::Mat3;
use bevy::prelude::*;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::light::NotShadowCaster;
use super::inventory::{Inventory, WeaponModel};
use super::movement::Velocity;
use crate::weapons::{WeaponSlot, WeaponRecoil, BaseWeaponTransform, FireMode};
use crate::gameplay::{Health, KillerInfo, PlayerBody, Enemy, Regenerating};
use crate::player::{spawn_hit_marker, spawn_damage_number};
use std::collections::HashMap;
use rand::Rng;

/// Shared tracer meshes + materials, built once. Uses the built-in
/// StandardMaterial (unlit) with per-mesh `NotShadowCaster` so tracers are
/// shadowless — the reliable path instead of a custom shader material.
#[derive(Resource)]
pub struct TracerAssets {
    /// Streak quad: 0.08 wide, 2.0 long (scaled per frame to the bullet's
    /// travel distance — the classic COD-style light trail). The width is
    /// what keeps shots visible when the card is edge-on (aimed along the
    /// view axis): a 0.03 card degenerates to a sub-pixel sliver at range.
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    /// Hot core: a small bright sphere at the bullet so straight-on shots
    /// stay visible even when the streak is edge-on.
    pub core_mesh: Handle<Mesh>,
    pub core_material: Handle<StandardMaterial>,
}

impl FromWorld for TracerAssets {
    fn from_world(world: &mut World) -> Self {
        let mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Rectangle::new(0.08, 2.0));
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::linear_rgba(1.0, 0.8, 0.2, 1.0),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                ..default()
            });
        let core_mesh = world
            .resource_mut::<Assets<Mesh>>()
            .add(Sphere::new(0.05));
        let core_material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::linear_rgba(1.0, 0.85, 0.3, 1.0),
                unlit: true,
                ..default()
            });
        Self {
            mesh,
            material,
            core_mesh,
            core_material,
        }
    }
}

/// Marker on the tracer bullet entity (holds `Projectile`); its streak
/// child carries `TracerStreakVisual`.
#[derive(Component)]
pub struct TracerStreak;

/// Marker on the stretched streak quad child of a tracer bullet.
#[derive(Component)]
pub struct TracerStreakVisual;

/// Orient a tracer card so its long axis (+Y) rides the flight direction
/// and its face (+Z) tracks the camera horizontally (yaw only — the card
/// stays vertical). Shared by the spawn (initial rotation, no vertical
/// first frame) and the per-frame update.
pub fn tracer_card_rotation(velocity: Vec3, to_cam: Vec3) -> Quat {
    let v = velocity.normalize_or_zero();
    if v.length_squared() < 1e-6 {
        return Quat::IDENTITY;
    }
    // Horizontal direction from the streak toward the camera — the card's
    // facing (yaw only, no pitch/roll).
    let face = Vec3::new(to_cam.x, 0.0, to_cam.z).normalize_or_zero();
    if face.length_squared() < 1e-6 {
        return Quat::from_rotation_arc(Vec3::Y, v);
    }
    // Long axis: the flight direction itself.
    let y = v;
    // Facing: the part of `face` perpendicular to the flight direction, so
    // the card's plane contains the flight path. When the bullet flies
    // straight at/away from the camera this is degenerate — any
    // perpendicular works, since the card is edge-on and invisible.
    let z = (face - v * v.dot(face)).normalize_or_zero();
    let z = if z.length_squared() < 1e-6 {
        let alt = v.cross(Vec3::Y);
        if alt.length_squared() > 1e-6 {
            alt.normalize()
        } else {
            v.cross(Vec3::X).normalize()
        }
    } else {
        z
    };
    // Right-handed orthonormal basis: x = y×z (NOT z×y — that inverts the
    // determinant and mirrors the rotation).
    let x = y.cross(z).normalize();
    Quat::from_mat3(&Mat3::from_cols(x, y, z))
}

/// Spawn a bullet tracer: a bright hot core at the bullet plus a light
/// streak that stretches across the bullet's recent travel (prev → current
/// position) — the classic shooter look. The streak card is scaled each
/// frame by `update_tracer_streaks`; the core keeps straight-on shots
/// visible when the streak is edge-on.
#[allow(clippy::too_many_arguments)]
pub fn spawn_tracer_projectile(
    commands: &mut Commands,
    assets: &TracerAssets,
    origin: Vec3,
    camera_pos: Vec3,
    velocity: Vec3,
    timer: Timer,
    damage: f32,
    from_player: bool,
    source_name: String,
) {
    let bullet = commands
        .spawn((
            Transform::from_translation(origin)
                .with_rotation(tracer_card_rotation(velocity, camera_pos - origin)),
            Visibility::default(),
            NotShadowCaster,
            TracerStreak,
            Projectile {
                velocity,
                prev_pos: origin,
                timer,
                damage,
                from_player,
                source_name,
            },
        ))
        .id();
    commands.entity(bullet).with_children(|parent| {
        // Streak quad: spawns as a tiny sliver (scale.y 0.02 ≈ 0.04m) so
        // the very first frame shows just a muzzle spark — never a 2m trail
        // poking BACK through the gun. `update_tracer_streaks` re-anchors
        // it each frame to span from the bullet back to the previous
        // frame's position (behind the muzzle from frame 1 on).
        parent.spawn((
            Mesh3d(assets.mesh.clone()),
            MeshMaterial3d(assets.material.clone()),
            NotShadowCaster,
            Transform::from_translation(Vec3::new(0.0, -0.02, 0.0))
                .with_scale(Vec3::new(1.0, 0.02, 1.0)),
            Visibility::default(),
            TracerStreakVisual,
        ));
        // Hot core: bright dot at the bullet position.
        parent.spawn((
            Mesh3d(assets.core_mesh.clone()),
            MeshMaterial3d(assets.core_material.clone()),
            NotShadowCaster,
            Transform::IDENTITY,
            Visibility::default(),
        ));
    });
}

/// Update tracer visuals: orient the bullet's card along its flight path
/// (long axis = flight direction, face tracks the camera horizontally) and
/// stretch each streak quad to span the bullet's travel since the last
/// frame — the classic shooter light-trail. The streak is anchored at the
/// bullet and extends BACKWARD along the flight path. Fast bullets (900 m/s
/// ≈ 15 m per frame) leave long continuous streaks instead of hopping
/// between positions.
pub fn update_tracer_streaks(
    camera: Query<(&GlobalTransform, &Transform), (With<super::MainCamera>, Without<TracerStreak>, Without<TracerStreakVisual>)>,
    mut bullets: Query<(Entity, &mut Transform, &Projectile), (With<TracerStreak>, Without<TracerStreakVisual>)>,
    mut streak_visuals: Query<(&mut Transform, &ChildOf), (With<TracerStreakVisual>, Without<TracerStreak>)>,
) {
    let Ok((cam_gt, _)) = camera.single() else { return };
    let cam_pos = cam_gt.translation();

    // Min/max streak length (mesh is 2.0 long; scale.y = len/2).
    const MIN_LEN: f32 = 0.4;
    const MAX_LEN: f32 = 25.0;

    let mut visuals: Vec<(Entity, f32)> = Vec::new();
    for (entity, mut tf, proj) in bullets.iter_mut() {
        tf.rotation = tracer_card_rotation(proj.velocity, cam_pos - tf.translation);
        let len = (tf.translation - proj.prev_pos).length().clamp(MIN_LEN, MAX_LEN);
        visuals.push((entity, len / 2.0));
    }

    for (mut tf, parent) in streak_visuals.iter_mut() {
        if let Some((_, half)) = visuals.iter().find(|(e, _)| *e == parent.0) {
            // Anchor at the bullet and stretch BACKWARD (card local +Y is
            // the flight direction, so -Y trails behind it).
            tf.scale.y = *half;
            tf.translation.y = -*half;
        }
    }
}

#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec3,
    /// World position at the start of the current frame's travel — the
    /// streak stretches between this and the current translation.
    pub prev_pos: Vec3,
    pub timer: Timer,
    pub damage: f32,
    pub from_player: bool,
    pub source_name: String,
}

#[derive(Component)]
pub struct MuzzleFlash {
    pub timer: Timer,
}

#[derive(Component)]
pub struct Target {
    /// Armed dummies hold and shoot guns at the local player.
    pub armed: bool,
}

/// Recoil climb model (COD/Siege/Phantom Forces style).
///
/// Each shot adds a kick to `climb`, but the kick shrinks as `climb`
/// approaches `RECOIL_MAX_PITCH` — the camera rises quickly at first,
/// then levels off at a plateau. `pitch` chases `climb` for a smooth
/// climb. Recovery only runs while the fire trigger is RELEASED, so a
/// burst never decays the climb between shots, and the camera settles
/// the moment you stop holding the button.
#[derive(Component)]
pub struct CameraRecoil {
    /// Current applied pitch offset (radians; positive = looking up).
    pub pitch: f32,
    /// Accumulated climb target, capped at `RECOIL_MAX_PITCH`.
    pub climb: f32,
    /// Current applied yaw offset.
    pub yaw: f32,
    /// Horizontal jitter target.
    pub yaw_target: f32,
}

impl Default for CameraRecoil {
    fn default() -> Self {
        Self {
            pitch: 0.0,
            climb: 0.0,
            yaw: 0.0,
            yaw_target: 0.0,
        }
    }
}

/// Where the vertical climb plateaus (~6.9°).
const RECOIL_MAX_PITCH: f32 = 0.12;
/// How much the climb slows near the plateau (0.8 = slows to 20%).
const RECOIL_FALLOFF: f32 = 0.8;
/// Recoil multiplier while aiming down sights.
const RECOIL_ADS_MULT: f32 = 0.6;
/// How fast the camera settles back to center after recovery kicks in.
/// Applied as an exponential approach (per-second lerp factor): the drop
/// starts fast and eases out as the camera nears center.
const RECOIL_RECOVERY_SPEED: f32 = 4.0;

#[derive(Component, Default)]
pub struct AmmoStatus {
    pub current_ammo: HashMap<WeaponSlot, u32>,
    pub reserve_ammo: HashMap<WeaponSlot, u32>,
    pub current_fire_mode: HashMap<WeaponSlot, usize>, // Index into config.fire_modes
    pub reloading: Option<(WeaponSlot, Timer)>,
    pub burst_count: u32, // Shots remaining in current burst
    pub heat: f32, // Accuracy decay
}

#[derive(Component)]
pub struct AmmoUi;

#[derive(Component)]
pub struct MeleeSwing {
    pub timer: Timer,
    pub direction: f32, // 1.0 for right, -1.0 for left
}

#[derive(Component)]
pub struct Grenade {
    pub velocity: Vec3,
    pub timer: Timer,
    pub angular_velocity: Vec3,
}

#[derive(Component)]
pub struct ExplosionParticle {
    pub velocity: Vec3,
    pub timer: Timer,
    pub max_time: f32,
    pub start_scale: f32,
    pub end_scale: f32,
}

/// Procedural bullet-hole decals: a semi-transparent square RING around an
/// opaque square center — reads as a single bullet hole. Built once via
/// `FromWorld` and shared by every hole.
#[derive(Resource)]
pub struct BulletHoleAssets {
    pub outer_mesh: Handle<Mesh>,
    pub inner_mesh: Handle<Mesh>,
    pub outer_material: Handle<StandardMaterial>,
    pub inner_material: Handle<StandardMaterial>,
}

impl FromWorld for BulletHoleAssets {
    fn from_world(world: &mut World) -> Self {
        // Flat squares facing +Z with outward normals; `spawn_bullet_hole`
        // orients +Z along the surface normal.
        fn solid_square(meshes: &mut Assets<Mesh>, half: f32) -> Handle<Mesh> {
            let positions = vec![
                [-half, -half, 0.0],
                [half, -half, 0.0],
                [half, half, 0.0],
                [-half, half, 0.0],
            ];
            let indices = vec![[0u32, 1, 2], [0, 2, 3]];
            meshes.add(
                Mesh::new(
                    bevy::render::mesh::PrimitiveTopology::TriangleList,
                    bevy::asset::RenderAssetUsages::default(),
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 4])
                .with_inserted_indices(bevy::render::mesh::Indices::U32(indices.into_iter().flat_map(|t| t.into_iter()).collect())),
            )
        }

        // Square annulus: four trapezoid quads between the outer and inner
        // square perimeters.
        fn ring_square(meshes: &mut Assets<Mesh>, outer: f32, inner: f32) -> Handle<Mesh> {
            let mut positions: Vec<[f32; 3]> = Vec::new();
            let mut indices: Vec<[u32; 3]> = Vec::new();
            for i in 0..4 {
                let (sx, sy) = if i == 0 {
                    (-1.0, -1.0)
                } else if i == 1 {
                    (1.0, -1.0)
                } else if i == 2 {
                    (1.0, 1.0)
                } else {
                    (-1.0, 1.0)
                };
                let base = positions.len() as u32;
                // outer corner, inner corner (same direction), then next segment
                positions.push([sx * outer, sy * outer, 0.0]);
                positions.push([sx * inner, sy * inner, 0.0]);
                let (nx, ny) = if i == 3 {
                    (-1.0, -1.0)
                } else if i == 0 {
                    (1.0, -1.0)
                } else if i == 1 {
                    (1.0, 1.0)
                } else {
                    (-1.0, 1.0)
                };
                positions.push([nx * outer, ny * outer, 0.0]);
                positions.push([nx * inner, ny * inner, 0.0]);
                indices.push([base, base + 1, base + 2]);
                indices.push([base + 1, base + 3, base + 2]);
            }
            let normals = vec![[0.0, 0.0, 1.0]; positions.len()];
            let flat_indices: Vec<u32> = indices.into_iter().flat_map(|t| t.into_iter()).collect();
            meshes.add(
                Mesh::new(
                    bevy::render::mesh::PrimitiveTopology::TriangleList,
                    bevy::asset::RenderAssetUsages::default(),
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
                .with_inserted_indices(bevy::render::mesh::Indices::U32(flat_indices)),
            )
        }

        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let outer_mesh = ring_square(&mut meshes, 0.07, 0.045);
        let inner_mesh = solid_square(&mut meshes, 0.045);

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let outer_material = materials.add(StandardMaterial {
            base_color: Color::srgba(0.05, 0.04, 0.08, 0.85),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        let inner_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.012, 0.008, 0.02),
            unlit: true,
            ..default()
        });

        Self {
            outer_mesh,
            inner_mesh,
            outer_material,
            inner_material,
        }
    }
}

/// A live bullet hole decal on world geometry.
#[derive(Component)]
pub struct BulletHole {
    pub timer: Timer,
}

/// Caps the number of active bullet holes; oldest are evicted first.
#[derive(Resource)]
pub struct BulletHolePool {
    pub entities: std::collections::VecDeque<Entity>,
    pub max: usize,
}

impl Default for BulletHolePool {
    fn default() -> Self {
        Self {
            entities: std::collections::VecDeque::new(),
            max: 200,
        }
    }
}

/// Spawn a bullet-hole decal at `pos` on a surface with outward `normal`.
fn spawn_bullet_hole(
    commands: &mut Commands,
    assets: &BulletHoleAssets,
    pool: &mut BulletHolePool,
    pos: Vec3,
    normal: Vec3,
) {
    if pool.entities.len() >= pool.max {
        if let Some(oldest) = pool.entities.pop_front() {
            commands.entity(oldest).try_despawn();
        }
    }
    let roll = rand::rng().random_range(0.0..std::f32::consts::TAU);
    let orient = Quat::from_rotation_arc(Vec3::Z, normal) * Quat::from_rotation_z(roll);
    let entity = commands
        .spawn((
            Transform::from_translation(pos + normal * 0.012).with_rotation(orient),
            Visibility::default(),
            BulletHole {
                timer: Timer::from_seconds(30.0, TimerMode::Once),
            },
            crate::world::GameWorldEntity,
        ))
        .with_children(|parent| {
            // Semi-transparent outer square ring.
            parent.spawn((
                Mesh3d(assets.outer_mesh.clone()),
                MeshMaterial3d(assets.outer_material.clone()),
                Transform::default(),
            ));
            // Opaque inner square, recessed a hair to avoid z-fighting.
            parent.spawn((
                Mesh3d(assets.inner_mesh.clone()),
                MeshMaterial3d(assets.inner_material.clone()),
                Transform::from_xyz(0.0, 0.0, -0.001),
            ));
        })
        .id();
    pool.entities.push_back(entity);
}

/// Ticks bullet-hole lifetimes and despawns expired decals.
pub fn update_bullet_holes(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut BulletHole)>,
    mut pool: ResMut<BulletHolePool>,
) {
    for (entity, mut hole) in query.iter_mut() {
        hole.timer.tick(time.delta());
        if hole.timer.is_finished() {
            commands.entity(entity).try_despawn();
            pool.entities.retain(|e| *e != entity);
        }
    }
}

pub fn handle_muzzle_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut MuzzleFlash)>,
) {
    for (entity, mut flash) in query.iter_mut() {
        flash.timer.tick(time.delta());
        if flash.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Updates the recoil state only — the offsets are composed into the camera
/// rotation by `apply_lean` (camera.rs) so no other system can overwrite them.
pub fn handle_camera_recoil(
    time: Res<Time>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut query: Query<&mut CameraRecoil>,
) {
    let dt = time.delta_secs();
    let trigger_down = mouse_input.pressed(MouseButton::Left);
    for mut recoil in query.iter_mut() {
        // Recovery runs only while the fire trigger is released — between
        // burst shots the climb keeps building instead of resetting, and
        // the moment you stop holding the button the camera settles.
        // Exponential approach: fast at first, easing out near center.
        if !trigger_down {
            recoil.climb = recoil.climb.lerp(0.0, dt * RECOIL_RECOVERY_SPEED);
            recoil.yaw_target = recoil.yaw_target.lerp(0.0, dt * RECOIL_RECOVERY_SPEED);
        }

        // Pitch chases the climb target — fast rise, smooth plateau.
        recoil.pitch = recoil.pitch.lerp(recoil.climb, dt * 25.0);
        recoil.yaw = recoil.yaw.lerp(recoil.yaw_target, dt * 25.0);
    }
}

pub fn handle_weapon_recoil(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut WeaponRecoil, &BaseWeaponTransform)>,
    inventory_query: Query<&Inventory>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    let stability = if let Some(inventory) = inventory_query.iter().next() {
        weapon_registry.configs.get(&inventory.active_slot)
            .map(|c| c.attributes.stability)
            .unwrap_or(0.5)
    } else {
        0.5
    };

    let is_aiming = mouse_input.pressed(MouseButton::Right);

    for (mut transform, mut recoil, base) in query.iter_mut() {
        let dt = time.delta_secs();
        
        // Stability affects recovery speed
        let recovery_speed = 5.0 + stability * 15.0; 
        
        // Interpolate current towards target (kick)
        recoil.current_offset = recoil.current_offset.lerp(recoil.target_offset, dt * 20.0);
        recoil.current_rotation = recoil.current_rotation.lerp(recoil.target_rotation, dt * 20.0);
        
        // Decay target back to zero (recovery)
        recoil.target_offset = recoil.target_offset.lerp(Vec3::ZERO, dt * recovery_speed);
        recoil.target_rotation = recoil.target_rotation.lerp(Vec3::ZERO, dt * recovery_speed);
        
        // Apply to transform (Recoil + Sway + Aim + Switch)
        transform.translation = base.0.translation + recoil.current_offset + recoil.sway_offset + recoil.aim_offset + recoil.switch_offset;
        
        let recoil_rot = if is_aiming {
            Quat::IDENTITY
        } else {
            Quat::from_euler(
                EulerRot::XYZ, 
                recoil.current_rotation.x, 
                recoil.current_rotation.y, 
                recoil.current_rotation.z
            )
        };
        
        let sway_rot = Quat::from_euler(
            EulerRot::XYZ, 
            recoil.sway_rotation.x, 
            recoil.sway_rotation.y, 
            recoil.sway_rotation.z
        );

        let switch_rot = Quat::from_euler(
            EulerRot::XYZ, 
            recoil.switch_rotation.x, 
            recoil.switch_rotation.y, 
            recoil.switch_rotation.z
        );

        let melee_rot = Quat::from_euler(
            EulerRot::XYZ, 
            recoil.melee_rotation.x, 
            recoil.melee_rotation.y, 
            recoil.melee_rotation.z
        );

        transform.rotation = base.0.rotation * recoil_rot * sway_rot * switch_rot * melee_rot;
    }
}

pub fn handle_weapon_sway(
    time: Res<Time>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    _keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut query: Query<&mut WeaponRecoil, With<WeaponModel>>,
    player_velocity: Single<&Velocity>,
    player_input: Single<&super::input::AccumulatedInput>,
    inventory_query: Query<&Inventory>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
    camera_query: Query<&GlobalTransform, With<super::MainCamera>>,
) {
    let velocity = player_velocity.into_inner();
    let input = player_input.into_inner();
    let speed = Vec3::new(velocity.x, 0.0, velocity.z).length();
    let dt = time.delta_secs();
    
    // 1. Movement Sway (Bobbing)
    // Clamp speed for frequency calculation to avoid super fast jitter
    let freq_speed = speed.min(8.0); 
    let (sway_amount, sway_speed) = if speed > 0.1 { 
        (0.005, freq_speed * 0.4) // Reduced vertical sway amount and speed
    } else { 
        (0.001, 0.5) // Idle
    };
    
    // 2. Look Sway (Lag)
    let mouse_delta = accumulated_mouse_motion.delta;
    let target_lag_x = -mouse_delta.x * 0.002; // Adjust sensitivity
    let target_lag_y = mouse_delta.y * 0.002;

    // 3. Sprint Pose (COD-style: tuck gun to chest, rock back and forth)
    let is_sprinting = input.sprint;
    let moving_forward = input.raw_movement.y > 0.0;
    
    let sprint_factor = if is_sprinting && moving_forward && speed > 0.1 {
        1.0
    } else {
        0.0
    };

    // COD-style: gun tucked closer to chest, tilted diagonally
    let sprint_target_pos = Vec3::new(0.0, -0.2, -0.1);
    let sprint_target_rot = Vec3::new(-0.6, 0.8, -0.4);

    // 4. Strafe Sway
    let mut strafe_sway = Vec3::ZERO;
    if let Ok(camera_transform) = camera_query.single() {
        let right = camera_transform.compute_transform().right();
        let local_vel_x = velocity.dot(*right); // Positive = Right, Negative = Left
        // If moving right, gun lags left (negative X)
        strafe_sway.z = -local_vel_x * 0.002; // Reduced from 0.005
        // Add a bit of roll for strafing
        strafe_sway.x = -local_vel_x * 0.005; // Reduced from 0.01
    }

    // 5. Aiming
    let is_aiming = mouse_input.pressed(MouseButton::Right) && !is_sprinting;
    let inventory = inventory_query.iter().next(); // Use iter().next() for safety if single() is weird
    
    let mut target_aim_offset = Vec3::ZERO;
    let mut ads_speed_mult = 15.0;
    let mut stability_mult = 1.0;
    let mut mobility_mult = 1.0;

    if let Some(inv) = inventory {
        if let Some(config) = weapon_registry.configs.get(&inv.active_slot) {
            ads_speed_mult = config.attributes.ads_speed * 20.0;
            stability_mult = 1.0 - (config.attributes.stability * 0.5); // Higher stability = less sway
            mobility_mult = 0.5 + (config.attributes.mobility * 0.5); // Higher mobility = faster sway recovery/movement?

            if is_aiming {
                if let Some(offset) = config.attachments.optic.as_ref().and_then(|o| o.meta.as_ref()).and_then(|m| m.aim_offset) {
                    target_aim_offset = Vec3::from(offset);
                }
            }
        }
    }

    for mut recoil in query.iter_mut() {
        // Update Phase
        recoil.sway_phase += dt * sway_speed * mobility_mult;
        
        // Smoothly transition sprint factor
        recoil.sprint_blend = recoil.sprint_blend + (sprint_factor - recoil.sprint_blend) * dt * 6.0;
        let blend = recoil.sprint_blend;
        
        let bob_x = recoil.sway_phase.sin() * sway_amount * stability_mult * 1.5; // Added horizontal sway
        let bob_y = (recoil.sway_phase * 2.0).cos().abs() * sway_amount * stability_mult; // Reduced vertical sway multiplier

        // Target Sway (Bobbing + Sprint + Strafe)
        // Disable sway if aiming
        let sway_mult = if is_aiming { 0.1 } else { 1.0 };
        
        // COD-style sprint rock: rock gun back and forth while running
        let sprint_rock_pos = if blend > 0.01 {
            let rock_phase = recoil.sway_phase * 0.8;
            Vec3::new(
                rock_phase.sin() * 0.02 * blend,        // Slight left-right rock
                rock_phase.cos().abs() * 0.01 * blend,   // Subtle up-down bounce
                (rock_phase * 0.5).sin() * 0.015 * blend, // Forward-back rock
            )
        } else {
            Vec3::ZERO
        };
        let sprint_rock_rot = if blend > 0.01 {
            let rock_phase = recoil.sway_phase * 0.8;
            Vec3::new(
                (rock_phase * 0.5).cos() * 0.06 * blend,  // Pitch rock
                rock_phase.sin() * 0.04 * blend,           // Yaw rock
                (rock_phase * 0.7).sin() * 0.03 * blend,   // Roll rock
            )
        } else {
            Vec3::ZERO
        };
        
        let sprint_pos = sprint_target_pos * blend + sprint_rock_pos;
        let sprint_rot = sprint_target_rot * blend + sprint_rock_rot;
        
        let target_sway_pos = (Vec3::new(bob_x, bob_y, 0.0) + sprint_pos + Vec3::new(strafe_sway.x, 0.0, 0.0)) * sway_mult;
        
        // Target Rotation (Lag + Sprint + Strafe Roll)
        let target_sway_rot = (Vec3::new(target_lag_y, target_lag_x, strafe_sway.z) + sprint_rot) * sway_mult * stability_mult;
        
        // Smoothly interpolate
        recoil.sway_offset = recoil.sway_offset.lerp(target_sway_pos, dt * 10.0);
        recoil.sway_rotation = recoil.sway_rotation.lerp(target_sway_rot, dt * 5.0);
        recoil.aim_offset = recoil.aim_offset.lerp(target_aim_offset, dt * ads_speed_mult);
    }
}

use crate::player::input::Keybinds;

#[derive(Default)]
pub struct FireState {
    pub last_fire: f32,
    pub melee_hold_timer: f32,
    pub last_swing_right: bool,
}

pub fn fire_weapon(
    mut commands: Commands,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    mut inventory_query: Query<(&mut Inventory, &mut AmmoStatus)>,
    mut camera_set: ParamSet<(
        Single<(&GlobalTransform, &Transform), With<super::MainCamera>>,
        Query<&mut CameraRecoil, With<super::MainCamera>>,
        Query<&GlobalTransform, With<WeaponModel>>,
    )>,
    mut weapon_query: Query<(Entity, &mut WeaponRecoil, &mut Transform, Option<&MeleeSwing>), (With<WeaponModel>, Without<super::MainCamera>)>,
    mut health_query: Query<(Entity, &GlobalTransform, &mut Health, Option<&PlayerBody>, Option<&Enemy>, Option<&mut Regenerating>), Without<Projectile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut fire_state: Local<FireState>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
    tracer_assets: Res<TracerAssets>,
    asset_server: Res<AssetServer>,
    pause_open: Res<super::PauseMenuOpen>,
) {
    // Don't allow shooting while paused
    if pause_open.0 { return; }

    let (mut inventory, mut ammo_status) = if let Ok(res) = inventory_query.single_mut() { res } else { return };
    
    // Decay heat
    ammo_status.heat = (ammo_status.heat - time.delta_secs() * 2.0).max(0.0);

    // Prevent firing while sprinting or switching weapons
    if keyboard_input.pressed(keybinds.sprint) || inventory.switch_state != crate::player::inventory::SwitchState::Idle {
        return;
    }
    
    // Handle Reloading
    let mut finished_reloading = false;
    if let Some((_, timer)) = &mut ammo_status.reloading {
        timer.tick(time.delta());
        if timer.is_finished() {
            finished_reloading = true;
        }
    }

    if finished_reloading {
        if let Some((slot, _)) = ammo_status.reloading.take() {
            if let Some(config) = weapon_registry.configs.get(&slot) {
                let current = *ammo_status.current_ammo.get(&slot).unwrap_or(&0);
                let max_ammo = config.attachments.magazine.as_ref().map(|m| m.carry_capacity).unwrap_or(120);
                let reserve = *ammo_status.reserve_ammo.get(&slot).unwrap_or(&max_ammo);
                let mag_size = config.attachments.magazine.as_ref().map(|m| m.capacity).unwrap_or(30);

                if config.attributes.shell_reload_time > 0.0 {
                    // Shell-by-shell: add 1 shell per reload cycle
                    if reserve > 0 && current < mag_size {
                        ammo_status.current_ammo.insert(slot, current + 1);
                        ammo_status.reserve_ammo.insert(slot, reserve - 1);

                        // Continue reloading if not full and have reserve
                        if current + 1 < mag_size && reserve - 1 > 0 {
                            ammo_status.reloading = Some((
                                slot,
                                Timer::from_seconds(config.attributes.shell_reload_time, TimerMode::Once),
                            ));
                        }
                    }
                } else {
                    // Magazine reload: fill entire mag at once
                    let needed = mag_size.saturating_sub(current);
                    let available = reserve.min(needed);
                    
                    ammo_status.current_ammo.insert(slot, current + available);
                    ammo_status.reserve_ammo.insert(slot, reserve - available);
                }
            }
        }
    }

    if ammo_status.reloading.is_some() {
        // Shell-by-shell reload can be cancelled by firing
        if mouse_input.just_pressed(MouseButton::Left) {
            let slot = ammo_status.reloading.as_ref().map(|(s, _)| *s);
            if let Some(slot) = slot {
                if let Some(config) = weapon_registry.configs.get(&slot) {
                    if config.attributes.shell_reload_time > 0.0 {
                        let current = *ammo_status.current_ammo.get(&slot).unwrap_or(&0);
                        if current > 0 {
                            // Cancel shell reload and fire
                            ammo_status.reloading = None;
                            // Fall through to firing logic
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            } else {
                return;
            }
        } else {
            return; // Can't shoot while reloading (if not cancelling)
        }
    }

    // Manual Reload
    if keyboard_input.just_pressed(KeyCode::KeyR) {
        if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
            let current = *ammo_status.current_ammo.get(&inventory.active_slot).unwrap_or(&0);
            let max_ammo = config.attachments.magazine.as_ref().map(|m| m.carry_capacity).unwrap_or(120);
            let reserve = *ammo_status.reserve_ammo.get(&inventory.active_slot).unwrap_or(&max_ammo);
            let mag_size = config.attachments.magazine.as_ref().map(|m| m.capacity).unwrap_or(30);
            
            if current < mag_size && reserve > 0 {
                let reload_time = if config.attributes.shell_reload_time > 0.0 {
                    config.attributes.shell_reload_time
                } else {
                    config.attributes.reload_speed
                };
                if reload_time > 0.0 {
                    ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(reload_time, TimerMode::Once)));
                    return;
                }
            }
        }
    }

    // Switch Fire Mode
    if keyboard_input.just_pressed(KeyCode::KeyV) {
        if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
            if !config.attributes.fire_modes.is_empty() {
                let current_idx = *ammo_status.current_fire_mode.get(&inventory.active_slot).unwrap_or(&0);
                let next_idx = (current_idx + 1) % config.attributes.fire_modes.len();
                ammo_status.current_fire_mode.insert(inventory.active_slot, next_idx);
            }
        }
    }

    let (fire_rate, speed, _color, _size, muzzle_offset, v_recoil, h_recoil, fire_mode, damage, accuracy) = if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
        let mode_idx = *ammo_status.current_fire_mode.get(&inventory.active_slot).unwrap_or(&0);
        let mode_str = config.attributes.fire_modes.get(mode_idx).map(|s| s.as_str()).unwrap_or("Auto");
        let mode = match mode_str {
            "Auto" => FireMode::Auto,
            "Semi" => FireMode::Semi,
            "III Burst" => FireMode::Burst(3),
            _ => FireMode::Auto,
        };
        
        let muzzle = config.attachments.barrel.as_ref().and_then(|b| b.meta.as_ref()).and_then(|m| m.muzzle_flash_offset);
        let dmg = config.attachments.ammo.as_ref().map(|a| a.damage).unwrap_or(10.0);
        // Bullet speed comes from the ammo's velocity stat (m/s).
        let ammo_velocity = config.attachments.ammo.as_ref().map(|a| a.velocity).unwrap_or(40.0);
        
        (config.attributes.fire_rate, ammo_velocity, Color::srgb(1.0, 0.8, 0.2), 0.05, muzzle, config.attributes.vertical_recoil, config.attributes.horizontal_recoil, mode, dmg, config.attributes.accuracy)
    } else {
        match inventory.active_slot {
            WeaponSlot::Melee => (0.5, 0.0, Color::NONE, 0.0, None, 0.0, 0.0, FireMode::Semi, 50.0, 1.0),
            WeaponSlot::Equipment => (1.0, 15.0, Color::srgb(0.2, 0.8, 0.2), 0.2, None, 0.0, 0.0, FireMode::Semi, 100.0, 1.0),
            _ => (0.2, 30.0, Color::WHITE, 0.1, None, 0.1, 0.05, FireMode::Auto, 10.0, 0.8),
        }
    };

    // Simple cooldown
    if fire_state.last_fire + fire_rate > time.elapsed_secs() {
        return;
    }

    let mut should_fire = false;
    let mut is_slash = false;
    
    // Auto Attack for Quick Melee
    if inventory.auto_attack && inventory.active_slot == WeaponSlot::Melee && inventory.switch_state == crate::player::inventory::SwitchState::Idle {
        should_fire = true;
        inventory.auto_attack = false;
    }
    
    // Grenade Throw Logic (Release G)
    if inventory.active_slot == WeaponSlot::Equipment {
        if keyboard_input.just_released(keybinds.grenade) || inventory.throw_queued {
            should_fire = true;
            inventory.throw_queued = false;
        }
    } else if inventory.active_slot == WeaponSlot::Melee {
        // Melee Logic (Hold vs Tap)
        let attack_speed = weapon_registry.configs.get(&WeaponSlot::Melee)
            .map(|c| c.attributes.attack_speed)
            .unwrap_or(0.5);
            
        // Check if already swinging
        let is_swinging = if let Some((_, _, _, swing)) = weapon_query.iter().next() {
            swing.is_some()
        } else {
            false
        };

        if !is_swinging {
            if mouse_input.pressed(MouseButton::Left) {
                fire_state.melee_hold_timer += time.delta_secs();
                if fire_state.melee_hold_timer > attack_speed {
                    should_fire = true;
                    is_slash = true;
                    fire_state.melee_hold_timer = 0.0; 
                }
            } else if mouse_input.just_released(MouseButton::Left) {
                if fire_state.melee_hold_timer < attack_speed {
                    should_fire = true; // Stab
                }
                fire_state.melee_hold_timer = 0.0;
            } else {
                fire_state.melee_hold_timer = 0.0;
            }
        }
    } else {
        // Gun Logic
        if ammo_status.burst_count > 0 {
            should_fire = true;
        } else {
            match fire_mode {
                FireMode::Auto => {
                    if mouse_input.pressed(MouseButton::Left) {
                        should_fire = true;
                    }
                },
                FireMode::Semi => {
                    if mouse_input.just_pressed(MouseButton::Left) {
                        should_fire = true;
                    }
                },
                FireMode::Burst(count) => {
                    if mouse_input.just_pressed(MouseButton::Left) {
                        ammo_status.burst_count = count;
                        should_fire = true;
                    }
                }
            }
        }
    }

    if should_fire {
        // Check Ammo for guns
        if matches!(inventory.active_slot, WeaponSlot::Primary | WeaponSlot::Secondary) {
            let current = *ammo_status.current_ammo.entry(inventory.active_slot).or_insert_with(|| {
                weapon_registry.configs.get(&inventory.active_slot)
                    .and_then(|c| c.attachments.magazine.as_ref())
                    .map(|m| m.capacity)
                    .unwrap_or(30)
            });
            
            if current == 0 {
                // Auto reload if empty
                if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
                    let max_ammo = config.attachments.magazine.as_ref().map(|m| m.carry_capacity).unwrap_or(120);
                    let reserve = *ammo_status.reserve_ammo.get(&inventory.active_slot).unwrap_or(&max_ammo);
                    if reserve > 0 {
                        let reload_time = if config.attributes.shell_reload_time > 0.0 {
                            config.attributes.shell_reload_time
                        } else {
                            config.attributes.reload_speed
                        };
                        ammo_status.reloading = Some((inventory.active_slot, Timer::from_seconds(reload_time, TimerMode::Once)));
                    }
                }
                ammo_status.burst_count = 0; // Cancel burst
                return;
            }
            
            ammo_status.current_ammo.insert(inventory.active_slot, current - 1);
        }
        
        if ammo_status.burst_count > 0 {
            ammo_status.burst_count -= 1;
        }

        fire_state.last_fire = time.elapsed_secs();

        let (global_transform, _) = camera_set.p0().into_inner();
        let camera_pos = global_transform.translation();
        let transform = global_transform.compute_transform();
        let forward = transform.forward();
        let spawn_pos = transform.translation + forward * 1.0;

        match inventory.active_slot {
            WeaponSlot::Melee => {
                // Melee Swing Logic
                let attack_speed = weapon_registry.configs.get(&WeaponSlot::Melee)
                    .map(|c| c.attributes.attack_speed)
                    .unwrap_or(0.5);

                if let Some((weapon_entity, _, _, _)) = weapon_query.iter().next() {
                    // Toggle direction
                    fire_state.last_swing_right = !fire_state.last_swing_right;
                    let direction = if fire_state.last_swing_right { 1.0 } else { -1.0 };

                    commands.entity(weapon_entity).insert(MeleeSwing {
                        timer: Timer::from_seconds(attack_speed, TimerMode::Once),
                        direction,
                    });
                }
                // Damage Logic
                let melee_range = 2.5;
                let final_damage = if is_slash { 30.0 } else { damage }; // Slash = 30, Stab = 50 (from JSON)
                
                for (_target_entity, target_transform, mut health, _, is_enemy, mut regen) in health_query.iter_mut() {
                    if is_enemy.is_none() { continue; } // Only hit enemies
                    
                    let to_target = target_transform.translation() - transform.translation; // Use camera pos, not spawn_pos
                    let distance = to_target.length();
                    
                    if distance < melee_range {
                        let dir_to_target = to_target.normalize();
                        // Check if in front (cone)
                        let cone = if is_slash { 0.2 } else { 0.8 }; // Slash is wider (0.2 dot product is wide angle), Stab is narrow
                        if forward.dot(dir_to_target) > cone {
                            health.current -= final_damage;
                            if let Some(r) = regen.as_mut() {
                                r.timer.reset();
                                r.current_rate = r.base_rate;
                            }
                            spawn_hit_marker(&mut commands);
                            spawn_damage_number(&mut commands, final_damage, target_transform.translation());
                            println!("Hit enemy! Health: {} (Type: {})", health.current, if is_slash { "Slash" } else { "Stab" });
                        }
                    }
                }
            },
            WeaponSlot::Equipment => {
                // Grenade Throw Logic
                commands.spawn((
                    WorldAssetRoot(asset_server.load("weapons/models/equipment/grenade/rgd-5.glb#Scene0")),
                    Transform::from_translation(spawn_pos).with_scale(Vec3::splat(0.2)),
                    Grenade {
                        velocity: forward * 15.0 + Vec3::Y * 5.0, // Arc throw
                        timer: Timer::from_seconds(3.0, TimerMode::Once),
                        angular_velocity: Vec3::new(
                            rand::rng().random_range(5.0..15.0),
                            rand::rng().random_range(-3.0..3.0),
                            rand::rng().random_range(-3.0..3.0),
                        ),
                    },
                ));
                
                // Animate hand/weapon throw
                if let Some((_weapon_entity, mut recoil, _, _)) = weapon_query.iter_mut().next() {
                     recoil.target_rotation += Vec3::new(-1.0, 0.0, 0.0); // Throw motion
                }

                // Switch back to primary (or previous)
                if let Some(prev) = inventory.previous_slot {
                    inventory.target_slot = Some(prev);
                    inventory.previous_slot = None;
                } else {
                    inventory.target_slot = Some(WeaponSlot::Primary);
                }
                inventory.switch_state = crate::player::inventory::SwitchState::Unequipping;
                
                // Set timer for unequip (using equip_speed of grenade)
                let speed = weapon_registry.configs.get(&WeaponSlot::Equipment)
                    .map(|c| c.attributes.equip_speed)
                    .unwrap_or(0.5);
                inventory.switch_timer.set_duration(std::time::Duration::from_secs_f32(speed));
                inventory.switch_timer.reset();
            },
            _ => {
                // Gun Logic
                let mut rng = rand::rng();
                
                // Increase heat
                ammo_status.heat = (ammo_status.heat + 0.2).min(1.0); // Max heat 1.0

                // Check if this is a shotgun (pellet_count > 0)
                let pellet_count = weapon_registry.configs.get(&inventory.active_slot)
                    .map(|c| c.attributes.pellet_count)
                    .unwrap_or(0);
                let spread_cone = weapon_registry.configs.get(&inventory.active_slot)
                    .map(|c| c.attributes.spread_cone)
                    .unwrap_or(0.0);

                let num_projectiles = if pellet_count > 0 { pellet_count } else { 1 };
                let per_pellet_damage = damage;

                // Bullets leave the gun barrel (same spot as the muzzle
                // flash), falling back to the camera when no barrel exists.
                // The weapon is a child of the camera, so its *Global*
                // Transform is required — the local Transform would be in
                // camera space and trail the player's spawn position.
                // The offset is scaled by the weapon's GlobalTransform scale
                // to match the flash children, which are transformed by the
                // parent's scale in world space.
                let muzzle_world = {
                    let gt = camera_set.p2().iter().next().map(|g| *g);
                    match (gt, muzzle_offset) {
                        (Some(gt), Some(off)) => Some(
                            gt.translation() + gt.rotation() * (gt.scale() * Vec3::from(off)),
                        ),
                        _ => None,
                    }
                };
                let bullet_origin = muzzle_world.unwrap_or(spawn_pos);

                let right = transform.right();
                let up = transform.up();

                for _ in 0..num_projectiles {
                    // Bullet Spread
                    let base_spread = if pellet_count > 0 {
                        // Shotgun uses spread_cone (degrees) for pellet scatter
                        spread_cone.to_radians() * 0.5
                    } else {
                        let max_spread = 0.1;
                        let heat_penalty = ammo_status.heat * 0.05;
                        ((1.0 - accuracy) * max_spread + heat_penalty).max(0.001)
                    };

                    let r1 = rng.random_range(-base_spread..base_spread);
                    let r2 = rng.random_range(-base_spread..base_spread);
                    let final_velocity = (forward.as_vec3() + right.as_vec3() * r1 + up.as_vec3() * r2).normalize() * speed;

                    spawn_tracer_projectile(
                        &mut commands,
                        &tracer_assets,
                        bullet_origin,
                        camera_pos,
                        final_velocity,
                        Timer::from_seconds(3.0, TimerMode::Once),
                        per_pellet_damage,
                        true,
                        "Player".to_string(),
                    );
                }

                // Apply Camera Recoil — climb curve: kicks shrink as the
                // accumulated climb nears the plateau; ADS reduces recoil.
                if let Some(mut camera_recoil) = camera_set.p1().iter_mut().next() {
                    let v_recoil_rad = v_recoil * 0.04;
                    let h_recoil_rad = h_recoil * 0.04;

                    let is_aiming = mouse_input.pressed(MouseButton::Right);
                    let ads_mult = if is_aiming { RECOIL_ADS_MULT } else { 1.0 };
                    let headroom = 1.0 - RECOIL_FALLOFF
                        * (camera_recoil.climb / RECOIL_MAX_PITCH).clamp(0.0, 1.0);

                    let pitch_kick = rng.random_range(v_recoil_rad * 0.5..v_recoil_rad * 1.5)
                        * headroom
                        * ads_mult;
                    camera_recoil.climb = (camera_recoil.climb + pitch_kick).min(RECOIL_MAX_PITCH);
                    // Soft peak: once the climb sits at the cap, the kick is
                    // passed straight to the pitch as a visual bump — the gun
                    // still recoils per shot without forcing the view higher.
                    let bounce = pitch_kick * (1.0 - headroom) * 1.5;
                    camera_recoil.pitch =
                        (camera_recoil.pitch + bounce).min(RECOIL_MAX_PITCH * 1.4);
                    camera_recoil.yaw_target +=
                        rng.random_range(-h_recoil_rad..h_recoil_rad) * headroom;
                }

                // Apply Weapon Recoil & Muzzle Flash
                if let Some((weapon_entity, mut recoil, _, _)) = weapon_query.iter_mut().next() {
                    // Visual Kick only
                    recoil.target_offset += Vec3::new(0.0, 0.0, 0.1); 
                    recoil.target_rotation += Vec3::new(0.1, 0.0, 0.0);

                    if let Some(offset) = muzzle_offset {
                        let muzzle_pos = Vec3::from(offset);
                        let flash_size = 0.12;
                        let flash_mat = materials.add(StandardMaterial {
                            base_color: Color::srgba(1.0, 0.9, 0.3, 0.9),
                            emissive: bevy::color::LinearRgba::new(5.0, 4.0, 1.0, 1.0),
                            alpha_mode: AlphaMode::Blend,
                            unlit: true,
                            ..default()
                        });
                        let quad_mesh = meshes.add(Rectangle::new(flash_size, flash_size * 3.0));
                        
                        commands.entity(weapon_entity).with_children(|parent| {
                            // Point light for muzzle flash illumination
                            parent.spawn((
                                PointLight {
                                    color: Color::srgb(1.0, 0.8, 0.2),
                                    intensity: 1000.0,
                                    range: 5.0,
                                    shadow_maps_enabled: false,
                                    ..default()
                                },
                                Transform::from_translation(muzzle_pos),
                                MuzzleFlash {
                                    timer: Timer::from_seconds(0.05, TimerMode::Once),
                                },
                            ));
                            // Horizontal quad at muzzle
                            parent.spawn((
                                Mesh3d(quad_mesh.clone()),
                                MeshMaterial3d(flash_mat.clone()),
                                Transform::from_translation(muzzle_pos),
                                MuzzleFlash {
                                    timer: Timer::from_seconds(0.05, TimerMode::Once),
                                },
                            ));
                            // Vertical quad (rotated 90 degrees around Z)
                            parent.spawn((
                                Mesh3d(quad_mesh),
                                MeshMaterial3d(flash_mat),
                                Transform::from_translation(muzzle_pos)
                                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                                MuzzleFlash {
                                    timer: Timer::from_seconds(0.05, TimerMode::Once),
                                },
                            ));
                        });
                    }
                }
            }
        }
    }
}

pub fn handle_melee_swing(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut WeaponRecoil, &mut MeleeSwing)>,
) {
    for (entity, mut recoil, mut swing) in query.iter_mut() {
        swing.timer.tick(time.delta());
        let t = swing.timer.fraction();
        let dir = swing.direction;
        
        // Wind up -> Swipe -> Recover
        let yaw = if t < 0.2 {
            // Wind up: 0 to -dir * 0.5
            let sub_t = t / 0.2;
            -dir * 0.5 * sub_t
        } else if t < 0.6 {
            // Swipe: -dir * 0.5 to dir * 1.5
            let sub_t = (t - 0.2) / 0.4;
            -dir * 0.5 + (dir * 2.0) * sub_t
        } else {
            // Recover: dir * 1.5 to 0
            let sub_t = (t - 0.6) / 0.4;
            (dir * 1.5) * (1.0 - sub_t)
        };
        
        let pitch = if t > 0.2 && t < 0.6 {
             // Dip during swipe
             -0.5 * ((t - 0.2) / 0.4 * std::f32::consts::PI).sin()
        } else {
            0.0
        };
        
        recoil.melee_rotation = Vec3::new(pitch, yaw, 0.0);

        if swing.timer.is_finished() {
            commands.entity(entity).remove::<MeleeSwing>();
            recoil.melee_rotation = Vec3::ZERO; // Reset
        }
    }
}

pub fn handle_grenade_throw(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Grenade)>,
    mut health_query: Query<(Entity, &GlobalTransform, &mut Health, Option<&mut Regenerating>), (With<Health>, Without<Grenade>)>,
    mesh_query: Query<&crate::world::objects::MeshCollider, Without<Grenade>>,
    camera_query: Query<&Transform, (With<super::MainCamera>, Without<Grenade>, Without<crate::world::objects::MeshCollider>, Without<Health>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut transform, mut grenade) in query.iter_mut() {
        grenade.timer.tick(time.delta());
        let dt = time.delta_secs();
        
        // Gravity
        grenade.velocity.y -= 9.8 * dt;
        let old_pos = transform.translation;
        transform.translation += grenade.velocity * dt;
        
        // Apply rotation (rolling/tumbling visual)
        let rot = Quat::from_euler(
            EulerRot::XYZ,
            grenade.angular_velocity.x * dt,
            grenade.angular_velocity.y * dt,
            grenade.angular_velocity.z * dt,
        );
        transform.rotation *= rot;
        
        // Floor collision
        if transform.translation.y < 0.2 {
            transform.translation.y = 0.2;
            grenade.velocity.y *= -0.4; // Bounce
            grenade.velocity.x *= 0.7; // Friction
            grenade.velocity.z *= 0.7;
            grenade.angular_velocity *= 0.6;
            grenade.angular_velocity.x += grenade.velocity.z * 2.0;
            grenade.angular_velocity.z -= grenade.velocity.x * 2.0;
        }

        // Mesh collider collision (swept sphere vs TriMesh)
        let grenade_radius = 0.15;
        let mv = transform.translation - old_pos;
        if mv.length_squared() > 0.0001 {
            use bevy_rapier3d::rapier::parry::shape::Ball;
            use bevy_rapier3d::rapier::parry::math::Vector as PVec;
            use bevy_rapier3d::rapier::parry::math::Pose;
            use bevy_rapier3d::rapier::parry::query::{cast_shapes, ShapeCastOptions};

            let ball = Ball::new(grenade_radius);
            let mut earliest_toi = 2.0f32;
            let mut best_normal = PVec::ZERO;

            for mc in mesh_query.iter() {
                let result = cast_shapes(
                    &Pose::translation(old_pos.x, old_pos.y, old_pos.z),
                    PVec::new(mv.x, mv.y, mv.z),
                    &ball,
                    &Pose::identity(),
                    PVec::ZERO,
                    &mc.mesh,
                    ShapeCastOptions {
                        max_time_of_impact: 1.0,
                        target_distance: 0.001,
                        stop_at_penetration: true,
                        compute_impact_geometry_on_penetration: true,
                    },
                );
                if let Ok(Some(hit)) = result {
                    if hit.time_of_impact < earliest_toi {
                        earliest_toi = hit.time_of_impact;
                        best_normal = hit.normal2;
                    }
                }
            }

            if earliest_toi <= 1.0 {
                let hit_pos = old_pos + mv * earliest_toi;
                transform.translation = hit_pos + Vec3::new(best_normal.x, best_normal.y, best_normal.z) * 0.001;
                let vn = grenade.velocity.dot(Vec3::new(best_normal.x, best_normal.y, best_normal.z));
                if vn < 0.0 {
                    grenade.velocity -= Vec3::new(best_normal.x, best_normal.y, best_normal.z) * vn * 1.4;
                    let tangent = grenade.velocity - Vec3::new(best_normal.x, best_normal.y, best_normal.z) * grenade.velocity.dot(Vec3::new(best_normal.x, best_normal.y, best_normal.z));
                    grenade.velocity = Vec3::new(best_normal.x, best_normal.y, best_normal.z) * grenade.velocity.dot(Vec3::new(best_normal.x, best_normal.y, best_normal.z)) + tangent * 0.7;
                }
                grenade.angular_velocity *= 0.7;
            }
        }

        if grenade.timer.is_finished() {
            // Explosion
            commands.entity(entity).despawn();
            
            // Smoke Particles - uneven polygon shapes for realistic explosion
            let mut rng = rand::rng();
            for _ in 0..25 {
                let dir = Vec3::new(
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                    rng.random_range(-1.0..1.0),
                ).normalize_or_zero();
                
                let speed = rng.random_range(2.0..8.0);
                let life = rng.random_range(1.0..2.5);
                let scale = rng.random_range(0.5..1.5);

                // Create irregular shapes by using cuboids with random dimensions
                let sx = rng.random_range(0.4..1.6);
                let sy = rng.random_range(0.4..1.6);
                let sz = rng.random_range(0.4..1.6);

                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(sx, sy, sz))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgba(1.0, 1.0, 0.8, 0.8),
                        alpha_mode: AlphaMode::Blend,
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_translation(transform.translation)
                        .with_scale(Vec3::splat(0.1))
                        .with_rotation(Quat::from_euler(
                            EulerRot::XYZ,
                            rng.random_range(0.0..std::f32::consts::TAU),
                            rng.random_range(0.0..std::f32::consts::TAU),
                            rng.random_range(0.0..std::f32::consts::TAU),
                        )),
                    ExplosionParticle {
                        velocity: dir * speed,
                        timer: Timer::from_seconds(life, TimerMode::Once),
                        max_time: life,
                        start_scale: 0.1,
                        end_scale: scale,
                    },
                ));
            }

            // Damage
            let explosion_radius = 5.0;
            let max_damage = 100.0;

            // Camera shake from explosion
            let player_distance = if let Some(cam_transform) = camera_query.iter().next() {
                transform.translation.distance(cam_transform.translation)
            } else {
                f32::MAX
            };
            if player_distance < explosion_radius * 3.0 {
                let shake_intensity = (1.0 - player_distance / (explosion_radius * 3.0)) * 8.0;
                crate::player::camera::spawn_camera_shake(&mut commands, shake_intensity, 0.5);
            }

            for (_target_entity, target_transform, mut health, mut regen) in health_query.iter_mut() {
                let distance = transform.translation.distance(target_transform.translation());
                if distance < explosion_radius {
                    let damage = max_damage * (1.0 - distance / explosion_radius);
                    health.current -= damage;
                    if let Some(r) = regen.as_mut() {
                        r.timer.reset();
                        r.current_rate = r.base_rate;
                    }
                }
            }
        }
    }
}

pub fn handle_explosion_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut ExplosionParticle, &mut MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut transform, mut particle, handle) in query.iter_mut() {
        particle.timer.tick(time.delta());
        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let t = particle.timer.fraction(); // 0.0 to 1.0
        
        // Movement
        transform.translation += particle.velocity * time.delta_secs();
        particle.velocity *= 0.95; // Drag

        // Scale
        let scale = particle.start_scale + (particle.end_scale - particle.start_scale) * t.sqrt();
        transform.scale = Vec3::splat(scale);

        if let Some(mut material) = materials.get_mut(&handle.0) {
            let color = if t < 0.2 {
                // White-Yellow to Orange
                let sub_t = t / 0.2;
                Color::srgba(1.0, 1.0, 0.8, 0.8).mix(&Color::srgba(1.0, 0.5, 0.0, 0.7), sub_t)
            } else if t < 0.6 {
                // Orange to Gray
                let sub_t = (t - 0.2) / 0.4;
                Color::srgba(1.0, 0.5, 0.0, 0.7).mix(&Color::srgba(0.2, 0.2, 0.2, 0.5), sub_t)
            } else {
                // Gray to Transparent
                let sub_t = (t - 0.6) / 0.4;
                Color::srgba(0.2, 0.2, 0.2, 0.5).mix(&Color::srgba(0.0, 0.0, 0.0, 0.0), sub_t)
            };
            material.base_color = color;
        }
    }
}

pub fn update_ammo_ui(
    inventory_query: Query<(&Inventory, &AmmoStatus)>,
    mut text_query: Query<(&mut Text, &mut TextColor), With<AmmoUi>>,
    weapon_registry: Res<crate::weapons::WeaponRegistry>,
    ui_config: Res<crate::ui_config::UiConfig>,
) {
    let (inventory, ammo_status) = if let Ok(res) = inventory_query.single() { res } else { return };
    let (mut text, mut text_color) = if let Ok(t) = text_query.single_mut() { t } else { return };
    let config = &ui_config.ammo_ui;
    let normal_color = Color::srgba(config.color[0], config.color[1], config.color[2], config.color[3]);

    if let Some((_, timer)) = &ammo_status.reloading {
        **text = format!("Reloading... {:.1}s", timer.remaining_secs());
        text_color.0 = crate::theme::WARNING;
    } else if matches!(inventory.active_slot, WeaponSlot::Primary | WeaponSlot::Secondary) {
        let current = ammo_status.current_ammo.get(&inventory.active_slot).copied().unwrap_or(0);
        let reserve = ammo_status.reserve_ammo.get(&inventory.active_slot).copied().unwrap_or(0);
        
        if let Some(config) = weapon_registry.configs.get(&inventory.active_slot) {
            let mode_idx = *ammo_status.current_fire_mode.get(&inventory.active_slot).unwrap_or(&0);
            let mode_str = config.attributes.fire_modes.get(mode_idx).map(|s| s.as_str()).unwrap_or("Auto");
            let ammo_type = config.attachments.ammo.as_ref().map(|a| a.name.as_str()).unwrap_or("Unknown");
            
            **text = format!("{} | {}\n{} | {}", current, reserve, ammo_type, mode_str);

            // Low-ammo warning: bottom 15% of the magazine.
            let mag_size = config.attachments.magazine.as_ref().map(|m| m.capacity).unwrap_or(30) as f32;
            if current as f32 <= (mag_size * 0.15).max(1.0) && reserve > 0 {
                text_color.0 = crate::theme::DANGER;
            } else {
                text_color.0 = normal_color;
            }
        } else {
             **text = format!("{} | {}", current, reserve);
             text_color.0 = normal_color;
        }
    } else {
        **text = "Ammo: --".to_string();
        text_color.0 = normal_color;
    }
}

pub fn reload_weapon() {} // Placeholder, logic moved to fire_weapon for now to share state access

pub fn move_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Projectile)>,
    mut health_query: Query<(Entity, &GlobalTransform, &mut Health, Option<&PlayerBody>, Option<&Enemy>, Option<&mut Regenerating>), Without<Projectile>>,
    mesh_query: Query<(Entity, &Transform, &crate::world::objects::MeshCollider, Option<&crate::world::objects::MaterialType>), Without<Projectile>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    bullet_hole_assets: Res<BulletHoleAssets>,
    mut bullet_hole_pool: ResMut<BulletHolePool>,
) {
    // Track glass entities to despawn after iteration
    let mut glass_to_despawn: Vec<Entity> = Vec::new();

    for (entity, mut transform, mut projectile) in query.iter_mut() {
        projectile.timer.tick(time.delta());
        if projectile.timer.just_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let delta = projectile.velocity * time.delta_secs();
        let old_pos = transform.translation;
        projectile.prev_pos = old_pos;
        transform.translation += delta;
        let _new_pos = transform.translation;
        // Triangle mesh collider hit-test (ray vs TriMesh)
        let mut hit_collider = false;
        for (_col_entity, col_transform, mesh_collider, material_type) in mesh_query.iter() {
            use bevy_rapier3d::rapier::parry::query::{Ray, RayCast};
            use bevy_rapier3d::rapier::parry::math::Vector;

            // Ray from the PRE-MOVE position along velocity, so the segment
            // covers exactly this frame's travel — impact points land on the
            // surface (e.g. the floor on downward shots), not past it.
            let origin = Vector::new(old_pos.x, old_pos.y, old_pos.z);
            let vel = projectile.velocity;
            let speed = vel.length();
            if speed <= 0.0 { break; }
            // Normalized ray direction — parry's TOI is then a physical
            // distance, so the impact point computed from it is exact.
            let dir = Vector::new(vel.x / speed, vel.y / speed, vel.z / speed);
            let ray = Ray::new(origin, dir);
            let max_toi = speed * time.delta_secs();

            if let Some(hit) = mesh_collider.mesh.cast_local_ray_and_get_normal(&ray, max_toi, true) {
                let bullet_dir = Vec3::new(dir.x, dir.y, dir.z);
                // Impact point + outward surface normal (colliders spawn
                // axis-aligned, so rotate the local normal by the transform).
                let hit_point = old_pos + bullet_dir * hit.time_of_impact;
                let normal = col_transform.rotation * Vec3::new(hit.normal.x, hit.normal.y, hit.normal.z);
                if let Some(mat_type) = material_type {
                    if mat_type.shatters() {
                        let he = mesh_collider.mesh.local_aabb();
                        let half_ext = Vec3::new(
                            (he.maxs.x - he.mins.x) * 0.5,
                            (he.maxs.y - he.mins.y) * 0.5,
                            (he.maxs.z - he.mins.z) * 0.5,
                        );
                        crate::world::objects::spawn_glass_shatter(
                            &mut commands, &mut meshes, &mut materials, hit_point, bullet_dir,
                            Some(col_transform), Some(half_ext),
                        );
                        glass_to_despawn.push(_col_entity);
                        projectile.damage *= mat_type.damage_falloff();
                        projectile.velocity *= 0.7;
                        continue;
                    }
                    let bullet_pen = projectile.damage / 100.0;
                    if bullet_pen > mat_type.resistance() * 0.5 {
                        // Bullet passes through — leave an entry hole.
                        spawn_bullet_hole(
                            &mut commands, &bullet_hole_assets, &mut bullet_hole_pool,
                            hit_point, normal,
                        );
                        projectile.damage *= mat_type.damage_falloff();
                        projectile.velocity *= 0.7;
                        continue;
                    }
                }
                spawn_bullet_hole(
                    &mut commands, &bullet_hole_assets, &mut bullet_hole_pool,
                    hit_point, normal,
                );
                commands.entity(entity).despawn();
                hit_collider = true;
                break;
            }
        }
        if hit_collider { continue; }

        // Entity collision check (distance based)
        for (_target_entity, target_transform, mut health, is_player, is_enemy, mut regen) in health_query.iter_mut() {
            if projectile.from_player && is_player.is_some() { continue; }
            if !projectile.from_player && is_enemy.is_some() { continue; }

            // Entity collision check — swept: the closest point on this
            // frame's travel segment to the target, so fast bullets (280+
            // m/s ≈ 4.7 m/frame) can't tunnel through a target between
            // frames. Enemy radius is generous because dummies are tall
            // and chest-high shots are the norm.
            let hit_radius = if is_enemy.is_some() { 0.9 } else { 1.5 };
            let target_pos = target_transform.translation();
            let travel = transform.translation - old_pos;
            let travel_len2 = travel.length_squared();
            let t = if travel_len2 > 1e-8 {
                ((target_pos - old_pos).dot(travel) / travel_len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let closest = old_pos + travel * t;
            if closest.distance(target_pos) < hit_radius {
                health.current -= projectile.damage;
                if let Some(r) = regen.as_mut() {
                    r.timer.reset();
                    r.current_rate = r.base_rate;
                }
                if projectile.from_player {
                    spawn_hit_marker(&mut commands);
                    spawn_damage_number(&mut commands, projectile.damage, target_transform.translation());
                }
                // Track who killed the player
                if is_player.is_some() && health.current <= 0.0 {
                    commands.insert_resource(KillerInfo { name: projectile.source_name.clone(), server_id: None });
                }
                commands.entity(entity).despawn();
                break;
            }
        }
        
        // Floor collision
        if transform.translation.y < 0.0 {
             commands.entity(entity).despawn();
        }
    }

    // Despawn shattered glass entities
    for glass_entity in glass_to_despawn {
        commands.entity(glass_entity).despawn();
    }
}

/// Deterministic cone spread using a random seed + pellet index.
/// Matches the implementation in `noctyrn_shared::spread`.
pub fn apply_spread_seeded(dir: &[f32; 3], spread_rad: f32, seed: u64, index: u32) -> [f32; 3] {
    noctyrn_shared::spread::apply_spread_seeded(dir, spread_rad, seed, index)
}

#[cfg(test)]
mod streak_orientation_tests {
    use super::*;

    fn run_streak(velocity: Vec3) -> Quat {
        let mut world = World::new();
        world.spawn((
            Transform::from_xyz(0.0, 1.7, 0.0),
            GlobalTransform::from_xyz(0.0, 1.7, 0.0),
            super::super::MainCamera,
        ));
        let streak = world
            .spawn((
                Transform::from_xyz(0.0, 1.5, -9.0),
                Projectile {
                    velocity,
                    prev_pos: Vec3::new(0.0, 1.5, -9.0),
                    timer: Timer::from_seconds(3.0, TimerMode::Once),
                    damage: 10.0,
                    from_player: true,
                    source_name: "test".into(),
                },
                TracerStreak,
            ))
            .id();
        let id = world.register_system(update_tracer_streaks);
        world.run_system(id).unwrap();
        world.get::<Transform>(streak).unwrap().rotation
    }

    fn local_y_world(rot: Quat) -> Vec3 {
        rot * Vec3::Y
    }

    #[test]
    fn side_shot_long_axis_follows_flight() {
        // Bullet crossing the view: long axis must ride the flight dir.
        let rot = run_streak(Vec3::new(1.0, 0.0, 0.0));
        let long_axis = local_y_world(rot);
        assert!(
            long_axis.dot(Vec3::X).abs() > 0.99,
            "side-shot streak long axis should be +X, got {long_axis:?}"
        );
        // And the card must stay vertical (contains world up).
        assert!(
            long_axis.y.abs() < 0.01,
            "side-shot streak must stay vertical, got y={}", long_axis.y
        );
    }

    #[test]
    fn straight_shot_collapses_along_flight() {
        // Bullet flying straight away from the camera: the streak must lie
        // along the flight path (edge-on → dot), never stand sideways.
        let rot = run_streak(Vec3::new(0.0, 0.0, -22.0));
        let long_axis = local_y_world(rot);
        assert!(
            long_axis.dot(Vec3::NEG_Z).abs() > 0.99,
            "straight-shot streak long axis should follow -Z, got {long_axis:?}"
        );
    }

    #[test]
    fn spread_shot_points_along_full_flight() {
        // A shot with spread deviates slightly from the view axis. The
        // streak must point along the FULL flight velocity — not along the
        // tiny perpendicular spread offset (which made bullets look
        // vertical when shot downrange).
        let v = Vec3::new(0.0, 0.03, -1.0).normalize();
        let rot = run_streak(v * 900.0);
        let long_axis = local_y_world(rot);
        assert!(
            long_axis.dot(v).abs() > 0.99,
            "spread-shot streak long axis should follow the flight velocity, got {long_axis:?} vs {v:?}"
        );
    }

    #[test]
    fn streak_trails_behind_the_bullet() {
        // The streak quad must sit BEHIND the bullet along the flight path
        // (card local -Y), never centered on it.
        let mut world = World::new();
        world.spawn((
            Transform::from_xyz(0.0, 1.7, 0.0),
            GlobalTransform::from_xyz(0.0, 1.7, 0.0),
            super::super::MainCamera,
        ));
        let bullet = world
            .spawn((
                Transform::from_xyz(0.0, 1.5, -9.0),
                Projectile {
                    velocity: Vec3::new(1.0, 0.0, 0.0),
                    prev_pos: Vec3::new(0.0, 1.5, -9.0),
                    timer: Timer::from_seconds(3.0, TimerMode::Once),
                    damage: 10.0,
                    from_player: true,
                    source_name: "test".into(),
                },
                TracerStreak,
            ))
            .id();
        let visual = world
            .spawn((Transform::IDENTITY, ChildOf(bullet), TracerStreakVisual))
            .id();
        let id = world.register_system(update_tracer_streaks);
        world.run_system(id).unwrap();
        let tf = world.get::<Transform>(visual).unwrap();
        // No travel yet → len clamps to MIN_LEN (0.4) → half = 0.2 behind.
        assert!(
            (tf.translation.y - (-0.2)).abs() < 1e-4,
            "streak must trail BEHIND the bullet, got translation.y={}",
            tf.translation.y
        );
        assert!(
            (tf.scale.y - 0.2).abs() < 1e-4,
            "streak half-length mismatch, got scale.y={}",
            tf.scale.y
        );
    }

    #[test]
    fn spawn_rotation_aligns_before_first_update() {
        // The tracer must spawn already aligned with its flight direction —
        // no vertical card on the first frame.
        let mut world = World::new();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<TracerAssets>();
        let (mesh, material, core_mesh, core_material) = {
            let a = world.resource::<TracerAssets>();
            (a.mesh.clone(), a.material.clone(), a.core_mesh.clone(), a.core_material.clone())
        };
        let assets = TracerAssets { mesh, material, core_mesh, core_material };
        let camera_pos = Vec3::new(0.0, 1.7, 0.0);
        let origin = Vec3::new(0.0, 1.5, -3.0);
        let velocity = Vec3::new(0.0, 0.0, -900.0);
        let mut commands = world.commands();
        spawn_tracer_projectile(
            &mut commands,
            &assets,
            origin,
            camera_pos,
            velocity,
            Timer::from_seconds(3.0, TimerMode::Once),
            25.0,
            true,
            "test".into(),
        );
        world.flush();
        let bullet = world
            .query_filtered::<&Transform, (With<TracerStreak>, Without<TracerStreakVisual>)>()
            .single(&world)
            .unwrap();
        let long_axis = bullet.rotation * Vec3::Y;
        assert!(
            long_axis.dot(Vec3::NEG_Z).abs() > 0.99,
            "spawned streak long axis should already follow -Z, got {long_axis:?}"
        );
    }
}

#[cfg(test)]
mod recoil_tests {
    use super::*;
    use bevy::time::Time;

    fn run_frames(world: &mut World, n: u32) {
        for _ in 0..n {
            let dt = std::time::Duration::from_secs_f32(1.0 / 60.0);
            world.resource_mut::<Time>().advance_by(dt);
            let id = world.register_system(handle_camera_recoil);
            world.run_system(id).unwrap();
        }
    }

    fn test_world() -> World {
        let mut world = World::new();
        world.insert_resource(Time::<Fixed>::from_hz(60.0));
        world.init_resource::<Time>();
        world
    }

    #[test]
    fn climb_raises_pitch_and_holds() {
        let mut world = test_world();
        let mut input = ButtonInput::<MouseButton>::default();
        input.press(MouseButton::Left);
        world.insert_resource(input);
        world.spawn(CameraRecoil {
            pitch: 0.0,
            climb: 0.1,
            yaw: 0.0,
            yaw_target: 0.0,
        });
        // Trigger held (burst fire): recovery must never decay the climb
        // between shots.
        run_frames(&mut world, 10);
        let r = world.query::<&CameraRecoil>().single(&world).unwrap();
        assert!(r.pitch > 0.05, "climb should raise pitch, got {:.4}", r.pitch);
        assert!((r.pitch - 0.1).abs() < 0.02, "pitch should hold at climb, got {}", r.pitch);
    }

    #[test]
    fn recovery_returns_camera_after_idle() {
        let mut world = test_world();
        world.insert_resource(ButtonInput::<MouseButton>::default());
        world.spawn(CameraRecoil {
            pitch: 0.08,
            climb: 0.1,
            yaw: 0.0,
            yaw_target: 0.0,
        });
        // Trigger released — recovery is active from the first frame.
        // ~1.5s of frames — the climb must decay back toward zero.
        run_frames(&mut world, 90);
        let r = world.query::<&CameraRecoil>().single(&world).unwrap();
        assert!(r.climb < 0.01, "climb should recover after firing stops, got {:.4}", r.climb);
        assert!(r.pitch < 0.05, "pitch should settle back down, got {:.4}", r.pitch);
    }
}


#[cfg(test)]
mod projectile_hit_tests {
    use super::*;
    use crate::gameplay::{Health, PlayerBody, Enemy};
    use bevy::time::Time;

    fn hit_world(from_player: bool, target_enemy: bool) -> (World, f32) {
        let mut world = World::new();
        world.init_resource::<Time>();
        world.init_resource::<Assets<Mesh>>();
        world.init_resource::<Assets<StandardMaterial>>();
        world.init_resource::<BulletHoleAssets>();
        world.init_resource::<BulletHolePool>();
        let (target_pos, bullet_start) = if target_enemy {
            (Vec3::new(0.0, 1.0, -10.0), Vec3::new(0.0, 1.3, -8.0))
        } else {
            (Vec3::new(0.0, 1.7, -20.0), Vec3::new(0.0, 1.3, -18.0))
        };
        let target = if target_enemy {
            world.spawn((
                Transform::from_translation(target_pos),
                GlobalTransform::from_translation(target_pos),
                Health { current: 100.0, max: 100.0 },
                Enemy,
            )).id()
        } else {
            world.spawn((
                Transform::from_translation(target_pos),
                GlobalTransform::from_translation(target_pos),
                Health { current: 100.0, max: 100.0 },
                PlayerBody,
            )).id()
        };
        world.spawn((
            Transform::from_translation(bullet_start),
            Projectile {
                velocity: Vec3::new(0.0, 0.0, -22.0),
                prev_pos: bullet_start,
                timer: Timer::from_seconds(3.0, TimerMode::Once),
                damage: 25.0,
                from_player,
                source_name: "test".into(),
            },
        ));
        let id = world.register_system(move_projectiles);
        for _ in 0..20 {
            world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(1.0 / 60.0));
            world.run_system(id).unwrap();
        }
        let health = world.get::<Health>(target).unwrap().current;
        (world, health)
    }


    #[test]
    fn player_bullet_damages_enemy() {
        let (_, hp) = hit_world(true, true);
        assert!(hp < 100.0, "player bullet should damage enemy, hp={hp}");
    }

    #[test]
    fn dummy_bullet_damages_player() {
        let (_, hp) = hit_world(false, false);
        assert!(hp < 100.0, "dummy bullet should damage player, hp={hp}");
    }
}
