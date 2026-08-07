//! Per-gamemode modules.
//!
//! Each sub-module exposes:
//! - `spawn_mode_entities(commands, meshes, materials)` – spawns objectives, NPCs,
//!   or other mode-specific entities.
//!
//! Note: All maps are GLB-based (selected from the map pool); gamemodes only
//! add entities on top of the map.

pub mod testing_grounds;
pub mod ffa;
pub mod tdm;
pub mod kc;
pub mod ctf;

use bevy::prelude::*;
use crate::menu::GameMode;

/// Convenience: spawn the correct map for a game mode (procedural fallback).
/// All maps are now GLB-based via the `maps` module — this is a no-op kept
/// for the fallback path in `world::spawn_game_map`.
pub fn spawn_map_for_mode(
    _mode: GameMode,
    _commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // All maps are GLB-based — nothing to spawn procedurally.
}

/// Convenience: spawn mode-specific entities (objectives, zones, enemies).
/// Called from gameplay's OnEnter(Playing) alongside spawn_objectives.
pub fn spawn_mode_entities(
    mode: GameMode,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    match mode {
        GameMode::TestingGrounds => {}
        GameMode::CaptureTheFlag => ctf::spawn_flags(commands, meshes, materials),
        _ => {}
    }
}
