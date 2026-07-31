use bevy::prelude::*;
use crate::world::GameWorldEntity;
use crate::world::objects::MeshCollider;
use super::config;

/// Spawn any map by name — GLB, lighting, and collider meshes.
/// This single function replaces the per-map spawn files.
pub fn spawn(
    map_id: &str,
    commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) {
    let cfg = config::load(map_id);
    let map_data = noctyrn_shared::map_data::load_map_data(map_id);
    let colliders = noctyrn_shared::map_data::load_colliders(map_id);
    let scale = map_data.scale;

    commands.spawn((
        WorldAssetRoot(asset_server.load(&cfg.glb)),
        Transform::from_scale(Vec3::splat(scale)),
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

    for m in &colliders.colliders {
        if let Some(mc) = MeshCollider::from_json(m, scale) {
            commands.spawn((mc, GameWorldEntity, Transform::default()));
        }
    }
}
