//! Assassins – each player has a specific assassination target.
//!
//! **Map**: Uses the same compact arena as FFA.
//! **Rules**: You are assigned a single target. Kill them to score (+1) and
//!   get a new target. Killing non-targets gives no score. 8-minute timer.

use bevy::prelude::*;

/// Assassins reuses the FFA arena (compact, lots of angles for sneaking).
pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    super::free_for_all::spawn_map(commands, meshes, materials);
}

/// No extra entities needed – target assignment is handled by the match system.
pub fn spawn_mode_entities(
    _commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {}
