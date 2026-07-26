//! Per-gamemode modules.
//!
//! Each sub-module exposes:
//! - `spawn_map(commands, meshes, materials)` – spawns the geometry for that mode.
//! - `spawn_mode_entities(commands, meshes, materials)` – spawns objectives, NPCs,
//!   or other mode-specific entities.
//!
//! Note: Only `TestingGrounds` retains its procedural arena. All other gamemodes
//! use GLB-based maps selected from the global map pool.

pub mod testing_grounds;

use bevy::prelude::*;
use crate::menu::GameMode;

/// Convenience: spawn the correct map for a game mode (procedural fallback).
/// Only `TestingGrounds` has a dedicated procedural map; all other modes
/// are handled by the `maps` module via `MapId`.
pub fn spawn_map_for_mode(
    mode: GameMode,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    match mode {
        GameMode::TestingGrounds => testing_grounds::spawn_map(commands, meshes, materials),
        _ => {} // Other modes use GLB-based maps via `SelectedMapId`.
    }
}

/// Convenience: spawn mode-specific entities (objectives, zones, enemies).
pub fn spawn_mode_entities(
    mode: GameMode,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    match mode {
        GameMode::TestingGrounds => {} // enemies spawned via gameplay.rs spawn_enemies
        _ => {} // Other modes use GLB-based maps; entities TBD.
    }
}
