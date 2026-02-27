//! Kill Confirmed – kill enemies and collect their dog tags to score.
//!
//! **Map**: Same symmetric layout as TDM.
//! **Rules**: Kills drop a floating dog tag. Walk over it to confirm (+1 score).
//!   Enemy dog tags expire after 15 seconds. First to 65 confirmed kills wins.

use bevy::prelude::*;
use crate::world::objects::StaticCollider;
use crate::world::GameWorldEntity;

/// Kill Confirmed uses the same map as TDM (symmetric lanes).
pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // Delegate to TDM map
    super::team_deathmatch::spawn_map(commands, meshes, materials);
}

/// No additional entities needed – dog tags are spawned dynamically on kill
/// by `gameplay::handle_death`.
pub fn spawn_mode_entities(
    _commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {}
