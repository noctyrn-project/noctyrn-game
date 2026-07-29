use bevy::prelude::*;
use crate::world::GameWorldEntity;
use crate::world::objects::StaticCollider;
use super::config;

/// Spawn the dust-storm map from config JSON (GLB, lighting) +
/// shared colliders and map data.
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) {
    let cfg = config::load("dust_storm");
    let map_data = noctyrn_shared::map_data::load_map_data("dust_storm");
    let colliders = noctyrn_shared::map_data::load_colliders("dust_storm");
    let scale = map_data.scale;

    // GLB scene
    commands.spawn((
        WorldAssetRoot(asset_server.load(&cfg.glb)),
        Transform::from_scale(Vec3::splat(scale)),
        GameWorldEntity,
    ));

    // Lighting
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

    // Colliders from shared data
    for c in &colliders.colliders {
        let center = Vec3::from(c.center) * scale;
        let he = Vec3::from(c.half_extents) * scale;
        let rot = Quat::from_array(c.rotation);
        commands.spawn((
            StaticCollider { half_extents: he },
            Transform::from_translation(center).with_rotation(rot),
            GameWorldEntity,
        ));
    }
}
