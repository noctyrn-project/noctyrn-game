use bevy::prelude::*;
use rand::Rng;
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
    /// Default material for anything not explicitly assigned one (the GLB's
    /// "world" material, plus any unrecognized/unassigned material name).
    /// Impenetrable — bullets never pass through it.
    World,
}

impl MaterialType {
    /// Map a baked GLB material name to a material type. Names are matched
    /// case-insensitively with Blender's ".NNN" duplicate suffixes stripped
    /// ("wood.001" → Wood). Any unrecognized or missing name becomes the
    /// default `World` material.
    pub fn from_name(name: Option<&str>) -> MaterialType {
        let normalized = name
            .unwrap_or("")
            .trim()
            .to_lowercase()
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .trim_end_matches('.')
            .to_string();
        match normalized.as_str() {
            "concrete" => MaterialType::Concrete,
            "metal" => MaterialType::Metal,
            "wood" => MaterialType::Wood,
            "glass" => MaterialType::Glass,
            "drywall" => MaterialType::Drywall,
            _ => MaterialType::World,
        }
    }

    /// Returns the penetration resistance (0.0 = no resistance, 1.0 = impenetrable)
    pub fn resistance(&self) -> f32 {
        match self {
            MaterialType::Concrete => 0.85,
            MaterialType::Metal => 0.95,
            MaterialType::Wood => 0.4,
            MaterialType::Glass => 0.1,
            MaterialType::Drywall => 0.2,
            // Impenetrable: the bullet's pen (damage/100) must exceed 0.5
            // (a >50-damage shot) to pass — effectively never.
            MaterialType::World => 1.0,
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
            // If a shot somehow passed World, it carries no damage.
            MaterialType::World => 0.0,
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
    pane_center: Vec3,
    half_extents: Vec3,
) {
    let mut rng = rand::rng();
    let glass_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.7, 0.85, 0.95, 0.5),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.1,
        perceptual_roughness: 0.1,
        ..default()
    });

    // Grid fracture — divide the pane surface into pieces. The collider's
    // mesh is baked in world space (the collider entity itself sits at the
    // origin), so the pane's world position is its AABB center — never the
    // collider's transform.
    let he = half_extents;
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

            let world_pos = pane_center + Vec3::new(lx, ly, 0.0);
            let offset = world_pos - position;
            let dist = offset.length().max(0.01);
            let outward = offset / dist;
            let speed = (1.5 + rng.random_range(0.0..2.0)) / (1.0 + dist * 0.3);

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(cw.max(0.01), ch.max(0.01), ct.max(0.005)))),
                MeshMaterial3d(glass_mat.clone()),
                Transform::from_translation(world_pos).with_rotation(Quat::from_euler(
                    EulerRot::XYZ,
                    rng.random_range(-0.2..0.2),
                    rng.random_range(-0.2..0.2),
                    rng.random_range(-0.2..0.2),
                )),
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
}

#[cfg(test)]
mod map_collider_tests {
    use super::*;

    /// Mirrors the runtime path in `maps::spawner::spawn`: parse the baked
    /// collider JSON and build a parry TriMesh for every node.
    #[test]
    fn testing_grounds_colliders_build() {
        let colliders = noctyrn_shared::map_data::load_colliders("testing_grounds");
        let data = noctyrn_shared::map_data::load_map_data("testing_grounds");
        assert!(!colliders.colliders.is_empty(), "no colliders baked");
        for (i, m) in colliders.colliders.iter().enumerate() {
            let mc = MeshCollider::from_json(m, data.scale);
            assert!(mc.is_some(), "collider #{i} failed to build a TriMesh");
            assert!(!m.vertices.is_empty() && !m.indices.is_empty(), "collider #{i} is empty");
        }
    }

    #[test]
    fn testing_grounds_spawn_point_inside_ground() {
        let data = noctyrn_shared::map_data::load_map_data("testing_grounds");
        let spawn = data.spawns.first().copied().unwrap_or([0.0, 1.0, 0.0]);
        assert!(spawn[1] >= 0.0, "spawn must not be below the ground plane");
        // The client-side config (GLB path/scale/lights) must parse too —
        // `maps::spawner::spawn` calls it at match start.
        let cfg = crate::maps::config::load("testing_grounds");
        assert!(cfg.glb.contains("testing_grounds.glb"), "unexpected glb path: {}", cfg.glb);
        assert_eq!(cfg.scale, data.scale, "config scale must match map data scale");
        assert!(!cfg.lights.is_empty(), "map has no lights");
    }
}

#[cfg(test)]
mod material_name_tests {
    use super::*;

    #[test]
    fn known_names_map_to_types() {
        assert!(matches!(MaterialType::from_name(Some("concrete")), MaterialType::Concrete));
        assert!(matches!(MaterialType::from_name(Some("metal")), MaterialType::Metal));
        assert!(matches!(MaterialType::from_name(Some("glass")), MaterialType::Glass));
        assert!(matches!(MaterialType::from_name(Some("drywall")), MaterialType::Drywall));
        // Blender appends .NNN to duplicate material names.
        assert!(matches!(MaterialType::from_name(Some("wood.001")), MaterialType::Wood));
        // Case-insensitive.
        assert!(matches!(MaterialType::from_name(Some("GLASS")), MaterialType::Glass));
    }

    #[test]
    fn everything_else_defaults_to_world() {
        assert!(matches!(MaterialType::from_name(Some("world")), MaterialType::World));
        assert!(matches!(MaterialType::from_name(Some("terrain")), MaterialType::World));
        assert!(matches!(MaterialType::from_name(Some("Material.003")), MaterialType::World));
        assert!(matches!(MaterialType::from_name(Some("banana")), MaterialType::World));
        assert!(matches!(MaterialType::from_name(None), MaterialType::World));
        assert!(matches!(MaterialType::from_name(Some("")), MaterialType::World));
    }

    #[test]
    fn world_is_impenetrable() {
        let world = MaterialType::World;
        assert!(!world.shatters());
        // A 50-damage shot has pen 0.5, which must not exceed World's
        // resistance threshold (1.0 * 0.5) — the bullet stops.
        assert!(0.5 <= world.resistance() * 0.5);
        assert_eq!(world.damage_falloff(), 0.0);
    }

    #[test]
    fn baked_testing_grounds_materials_are_recognized() {
        let colliders = noctyrn_shared::map_data::load_colliders("testing_grounds");
        let names: Vec<Option<String>> = colliders
            .colliders
            .iter()
            .map(|c| c.material.clone())
            .collect();
        assert!(!names.is_empty(), "no colliders baked");
        for name in &names {
            // Every baked name must map to something (never panics, and the
            // known gameplay materials are all present somewhere).
            let _ = MaterialType::from_name(name.as_deref());
        }
        let flat: Vec<&str> = names.iter().flatten().map(|s| s.as_str()).collect();
        for expected in ["concrete", "metal", "wood.001", "glass", "drywall", "world"] {
            assert!(
                flat.contains(&expected),
                "baked colliders missing material {expected:?}: {flat:?}"
            );
        }
    }
}
