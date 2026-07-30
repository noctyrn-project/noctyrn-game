pub mod config;
pub mod spawner;

use bevy::prelude::*;

pub fn spawn_map_by_id(
    map_id: &str,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
) -> bool {
    match map_id {
        "dust_storm" | "city" => {
            spawner::spawn(map_id, commands, meshes, materials, asset_server);
            true
        }
        _ => false,
    }
}
