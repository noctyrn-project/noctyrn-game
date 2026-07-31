use bevy::prelude::*;
use rand::Rng;
use crate::player::shooting::Target;
use bevy_rapier3d::rapier::parry::shape::TriMesh;
use bevy_rapier3d::rapier::parry::math::Vector;

/// Triangle mesh collider — accurate collision surface for one mesh node.
/// parry3d builds a BVH internally so collision queries are O(log n).
#[derive(Component, Clone, Debug)]
pub struct MeshCollider {
    pub mesh: TriMesh,
}

impl MeshCollider {
    pub fn from_json(data: &noctyrn_shared::map_data::TriangleMesh, scale: f32) -> Option<Self> {
        let vertices: Vec<Vector> = data.vertices.iter().map(|v| {
            Vector::new(v[0] * scale, v[1] * scale, v[2] * scale)
        }).collect();
        let indices: Vec<[u32; 3]> = data.indices.clone();
        TriMesh::new(vertices, indices).ok().map(|mesh| MeshCollider { mesh })
    }

    /// Build a MeshCollider from a cuboid's half-extents.
    pub fn from_cuboid(half_extents: Vec3) -> Self {
        let (x, y, z) = (half_extents.x, half_extents.y, half_extents.z);
        let vertices = vec![
            Vector::new(-x, -y, -z), Vector::new( x, -y, -z),
            Vector::new( x,  y, -z), Vector::new(-x,  y, -z),
            Vector::new(-x, -y,  z), Vector::new( x, -y,  z),
            Vector::new( x,  y,  z), Vector::new(-x,  y,  z),
        ];
        let indices = vec![
            [0, 1, 2], [0, 2, 3],  // -Z
            [4, 6, 5], [4, 7, 6],  // +Z
            [0, 4, 5], [0, 5, 1],  // -X
            [1, 5, 6], [1, 6, 2],  // +X
            [0, 3, 7], [0, 7, 4],  // -Y
            [2, 6, 7], [2, 7, 3],  // +Y
        ];
        MeshCollider { mesh: TriMesh::new(vertices, indices).expect("cuboid trimesh") }
    }
}

/// Ramp collider for inclined surfaces.
#[derive(Component, Clone, Debug)]
pub struct RampCollider {
    pub half_extents: Vec3,
}

/// Material type for bullet penetration/collision behavior
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub enum MaterialType {
    Concrete,
    Metal,
    Wood,
    Glass,
    Drywall,
}

impl MaterialType {
    /// Returns the penetration resistance (0.0 = no resistance, 1.0 = impenetrable)
    pub fn resistance(&self) -> f32 {
        match self {
            MaterialType::Concrete => 0.85,
            MaterialType::Metal => 0.95,
            MaterialType::Wood => 0.4,
            MaterialType::Glass => 0.1,
            MaterialType::Drywall => 0.2,
        }
    }

    /// Returns the damage multiplier after penetrating this material
    pub fn damage_falloff(&self) -> f32 {
        match self {
            MaterialType::Concrete => 0.2,
            MaterialType::Metal => 0.1,
            MaterialType::Wood => 0.6,
            MaterialType::Glass => 0.9,
            MaterialType::Drywall => 0.75,
        }
    }

    /// Whether this material shatters on bullet impact
    pub fn shatters(&self) -> bool {
        matches!(self, MaterialType::Glass)
    }
}

/// Component for glass shatter particles
#[derive(Component)]
pub struct GlassShard {
    pub velocity: Vec3,
    pub timer: Timer,
    pub angular_velocity: Vec3,
}

/// Moving target that slides back and forth along an axis.
#[derive(Component)]
pub struct MovingTarget {
    pub origin: Vec3,
    pub axis: Vec3,
    pub amplitude: f32,
    pub speed: f32,
    pub phase: f32,
}

/// Pop-up target that raises and lowers.
#[derive(Component)]
pub struct PopUpTarget {
    pub base_y: f32,
    pub raised_y: f32,
    pub timer: Timer,
    pub is_up: bool,
}

/// Distance marker label.
#[derive(Component)]
pub struct DistanceMarker;

/// System to update moving targets
pub fn update_moving_targets(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &MovingTarget)>,
) {
    for (mut transform, target) in query.iter_mut() {
        let offset = (time.elapsed_secs() * target.speed + target.phase).sin() * target.amplitude;
        transform.translation = target.origin + target.axis * offset;
    }
}

/// System to update pop-up targets
pub fn update_popup_targets(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut PopUpTarget)>,
) {
    for (mut transform, mut popup) in query.iter_mut() {
        popup.timer.tick(time.delta());
        if popup.timer.just_finished() {
            popup.is_up = !popup.is_up;
        }

        let target_y = if popup.is_up { popup.raised_y } else { popup.base_y };
        transform.translation.y = transform.translation.y
            + (target_y - transform.translation.y) * time.delta_secs() * 8.0;
    }
}

/// System to update glass shards
pub fn update_glass_shards(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut GlassShard)>,
) {
    for (entity, mut transform, mut shard) in query.iter_mut() {
        shard.timer.tick(time.delta());
        if shard.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }

        let dt = time.delta_secs();
        shard.velocity.y -= 9.8 * dt;
        transform.translation += shard.velocity * dt;
        shard.velocity *= 0.98;

        let rot = Quat::from_euler(
            EulerRot::XYZ,
            shard.angular_velocity.x * dt,
            shard.angular_velocity.y * dt,
            shard.angular_velocity.z * dt,
        );
        transform.rotation *= rot;

        if transform.translation.y < 0.05 {
            transform.translation.y = 0.05;
            shard.velocity.y *= -0.3;
            shard.velocity.x *= 0.5;
            shard.velocity.z *= 0.5;
        }
    }
}

/// Store the original pane extents for grid-based glass fracture.
#[derive(Component, Clone, Debug)]
pub struct ShatterInfo {
    pub half_extents: Vec3,
}

/// Spawn glass shatter using grid fracture for pane-like glass or
/// random shards for arbitrary shapes.
///
/// When `glass_transform` and `half_extents` are provided, the pane
/// is subdivided into a grid of shards that radiate from the impact
/// point.  Otherwise falls back to 12 random cuboids at `position`.
pub fn spawn_glass_shatter(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    bullet_dir: Vec3,
    glass_transform: Option<&Transform>,
    half_extents: Option<Vec3>,
) {
    let mut rng = rand::rng();
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.85, 0.95, 0.5),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.1,
        perceptual_roughness: 0.1,
        ..default()
    });

    if let (Some(gt), Some(he)) = (glass_transform, half_extents) {
        // Grid fracture — divide the pane surface into pieces.
        let nx = (he.x * 4.0).max(2.0).round() as usize;
        let ny = (he.y * 4.0).max(2.0).round() as usize;
        let cell_w = he.x * 2.0 / nx as f32;
        let cell_h = he.y * 2.0 / ny as f32;
        let thickness = he.z * 2.0;

        for ix in 0..nx {
            for iy in 0..ny {
                let lx = -he.x + cell_w * (ix as f32 + 0.5 + rng.random_range(-0.15..0.15));
                let ly = -he.y + cell_h * (iy as f32 + 0.5 + rng.random_range(-0.15..0.15));
                let cw = cell_w * rng.random_range(0.7..1.3);
                let ch = cell_h * rng.random_range(0.7..1.3);
                let ct = thickness * rng.random_range(0.5..1.0);

                let world_pos = gt.translation + gt.rotation * Vec3::new(lx, ly, 0.0);
                let offset = world_pos - position;
                let dist = offset.length().max(0.01);
                let outward = offset / dist;
                let speed = (1.5 + rng.random_range(0.0..2.0)) / (1.0 + dist * 0.3);

                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(cw.max(0.01), ch.max(0.01), ct.max(0.005)))),
                    MeshMaterial3d(glass_mat.clone()),
                    Transform::from_translation(world_pos).with_rotation(
                        gt.rotation * Quat::from_euler(
                            EulerRot::XYZ,
                            rng.random_range(-0.2..0.2),
                            rng.random_range(-0.2..0.2),
                            rng.random_range(-0.2..0.2),
                        ),
                    ),
                    GlassShard {
                        velocity: outward * speed + Vec3::new(0.0, rng.random_range(0.0..1.5), 0.0) + bullet_dir * 0.2,
                        timer: Timer::from_seconds(rng.random_range(1.5..3.0), TimerMode::Once),
                        angular_velocity: Vec3::new(
                            rng.random_range(-6.0..6.0),
                            rng.random_range(-6.0..6.0),
                            rng.random_range(-6.0..6.0),
                        ),
                    },
                ));
            }
        }
    } else {
        // Fallback: random shards at impact point.
        for _ in 0..12 {
            let sx = rng.random_range(0.03..0.12);
            let sy = rng.random_range(0.03..0.12);
            let sz = rng.random_range(0.005..0.02);

            let spread = Vec3::new(
                rng.random_range(-2.0..2.0),
                rng.random_range(-1.0..3.0),
                rng.random_range(-2.0..2.0),
            );
            let vel = bullet_dir * rng.random_range(1.0..4.0) + spread;

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(sx, sy, sz))),
                MeshMaterial3d(glass_mat.clone()),
                Transform::from_translation(position)
                    .with_rotation(Quat::from_euler(
                        EulerRot::XYZ,
                        rng.random_range(0.0..std::f32::consts::TAU),
                        rng.random_range(0.0..std::f32::consts::TAU),
                        rng.random_range(0.0..std::f32::consts::TAU),
                    )),
                GlassShard {
                    velocity: vel,
                    timer: Timer::from_seconds(rng.random_range(1.5..3.0), TimerMode::Once),
                    angular_velocity: Vec3::new(
                        rng.random_range(-10.0..10.0),
                        rng.random_range(-10.0..10.0),
                        rng.random_range(-10.0..10.0),
                    ),
                },
            ));
        }
    }
}
