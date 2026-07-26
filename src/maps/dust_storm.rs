use bevy::prelude::*;
use crate::world::GameWorldEntity;
use super::config;

/// Spawn the dust-storm map from config JSON (GLB + lighting only).
pub fn spawn(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) {
    let cfg = config::load("dust_storm");

    // GLB scene
    commands.spawn((
        WorldAssetRoot(asset_server.load(&cfg.glb)),
        Transform::from_scale(Vec3::splat(cfg.scale)),
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
            Transform::from_translation(Vec3::from(l.position) * cfg.scale),
            GameWorldEntity,
        ));
    }
}
