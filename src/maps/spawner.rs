use bevy::asset::RenderAssetUsages;
use bevy::light::NotShadowCaster;
use bevy::mesh::PrimitiveTopology;
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use crate::world::GameWorldEntity;
use crate::world::objects::{MaterialType, MeshCollider};
use super::config;

/// Spawn any map by name — GLB, lighting, and collider meshes.
/// This single function replaces the per-map spawn files.
///
/// The visual GLB (`config.glb`) is independent of the collision data:
/// maps may render a fancy textured GLB while colliding against a
/// separately-baked simple GLB (see MAPS.md).
pub fn spawn(
    map_id: &str,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) {
    let cfg = config::load(map_id);
    let map_data = noctyrn_shared::map_data::load_map_data(map_id);
    let colliders = noctyrn_shared::map_data::load_colliders(map_id);
    let scale = map_data.scale;

    commands.spawn((
        WorldAssetRoot(asset_server.load(&cfg.glb)),
        Transform::from_scale(Vec3::splat(scale)),
        Visibility::default(),
        GameWorldEntity,
    ));

    for l in &cfg.lights {
        commands.spawn((
            PointLight {
                shadow_maps_enabled: l.shadows,
                intensity: l.intensity,
                range: 200.0,
                ..default()
            },
            Transform::from_translation(Vec3::from(l.position) * scale),
            GameWorldEntity,
        ));
    }

    // Glass visual material — the engine spawns shatterable glass panels
    // from the glass colliders' geometry, so the map itself never needs
    // to include glass visuals (the fancy map is non-dynamic and omits
    // glass entirely).
    let glass_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.6, 0.8, 0.95, 0.25),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.05,
        metallic: 0.1,
        cull_mode: None, // double-sided so panels read from both sides
        ..default()
    });

    for m in &colliders.colliders {
        if let Some(mc) = MeshCollider::from_json(m, scale) {
            // Attach the baked material's behavior (penetration, shatter)
            // when its name maps to a known type; anything else is the
            // default "world" material.
            let material_type = MaterialType::from_name(m.material.as_deref());
            let collider = commands
                .spawn((mc, material_type, GameWorldEntity, Transform::default()))
                .id();

            // Glass colliders get a dynamic visual child built from their
            // own triangles. It despawns with the collider when a bullet
            // shatters the glass (see shooting.rs).
            if material_type == MaterialType::Glass {
                if let Some(mesh) = build_mesh_from_collider(m, scale) {
                    commands.entity(collider).with_children(|parent| {
                        parent.spawn((
                            Mesh3d(meshes.add(mesh)),
                            MeshMaterial3d(glass_material.clone()),
                            NotShadowCaster,
                            Transform::default(),
                            Visibility::default(),
                        ));
                    });
                }
            }
        }
    }
}

/// Build a renderable mesh from a baked collider's world-space triangles.
/// Glass visuals use this so the engine's glass exactly matches the
/// collision geometry.
fn build_mesh_from_collider(
    data: &noctyrn_shared::map_data::TriangleMesh,
    scale: f32,
) -> Option<Mesh> {
    if data.vertices.is_empty() || data.indices.is_empty() {
        return None;
    }
    let positions: Vec<[f32; 3]> = data
        .vertices
        .iter()
        .map(|v| [v[0] * scale, v[1] * scale, v[2] * scale])
        .collect();
    let indices: Vec<u32> = data.indices.iter().flatten().copied().collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_normals();
    Some(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_collider_builds_a_matching_mesh() {
        // The real baked testing grounds: the glass material wall must
        // produce a renderable mesh with the same triangle count.
        let colliders = noctyrn_shared::map_data::load_colliders("testing_grounds");
        let data = noctyrn_shared::map_data::load_map_data("testing_grounds");
        let glass: Vec<&noctyrn_shared::map_data::TriangleMesh> = colliders
            .colliders
            .iter()
            .filter(|c| MaterialType::from_name(c.material.as_deref()) == MaterialType::Glass)
            .collect();
        assert_eq!(glass.len(), 1, "expected exactly one glass collider in the bake");
        let mesh = build_mesh_from_collider(glass[0], data.scale).expect("glass mesh must build");
        assert_eq!(
            mesh.count_vertices(),
            glass[0].vertices.len(),
            "mesh vertices must match the collider"
        );
        assert_eq!(
            mesh.primitive_topology(),
            PrimitiveTopology::TriangleList
        );
    }
}
