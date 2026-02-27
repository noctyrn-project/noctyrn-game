//! Testing Grounds – the sandbox practice map.
//!
//! Reuses the existing `world::objects::spawn_objects` to place all the
//! test geometry (ramps, weapon terminals, shooting range, parkour, etc.).

use bevy::prelude::*;
use crate::world::objects;

/// Spawn the full testing-grounds geometry.
pub fn spawn_map(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    objects::spawn_objects(commands, meshes, materials);
}
