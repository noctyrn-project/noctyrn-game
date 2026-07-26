pub mod config;
pub mod dust_storm;
pub mod city;

use bevy::prelude::*;
use crate::world::GameWorldEntity;

/// Try to spawn a map by its MapId.
/// Returns `true` if the map was handled, `false` if unknown (caller should
/// fall back to procedural per-gamemode map).
pub fn spawn_map_by_id(
    map_id: &str,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) -> bool {
    match map_id {
        "dust_storm" => {
            dust_storm::spawn(commands, meshes, materials, asset_server);
            true
        }
        "city" => {
            city::spawn(commands, meshes, materials, asset_server);
            true
        }
        _ => false,
    }
}
