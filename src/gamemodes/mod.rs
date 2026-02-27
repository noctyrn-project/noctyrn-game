//! Per-gamemode modules.
//!
//! Each sub-module exposes:
//! - `spawn_map(commands, meshes, materials)` – spawns the geometry for that mode.
//! - `spawn_mode_entities(commands, meshes, materials)` – spawns objectives, NPCs,
//!   or other mode-specific entities.

pub mod testing_grounds;
pub mod free_for_all;
pub mod team_deathmatch;
pub mod kill_confirmed;
pub mod capture_the_flag;
pub mod assassins;
pub mod king_of_the_hill;
pub mod hardpoint;
pub mod capture_point;
pub mod ltm;

use bevy::prelude::*;
use crate::menu::GameMode;

/// Convenience: spawn the correct map for a game mode.
pub fn spawn_map_for_mode(
    mode: GameMode,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    match mode {
        GameMode::TestingGrounds => testing_grounds::spawn_map(commands, meshes, materials),
        GameMode::FreeForAll => free_for_all::spawn_map(commands, meshes, materials),
        GameMode::TeamDeathmatch => team_deathmatch::spawn_map(commands, meshes, materials),
        GameMode::KillConfirmed => kill_confirmed::spawn_map(commands, meshes, materials),
        GameMode::CaptureTheFlag => capture_the_flag::spawn_map(commands, meshes, materials),
        GameMode::Assassins => assassins::spawn_map(commands, meshes, materials),
        GameMode::KingOfTheHill => king_of_the_hill::spawn_map(commands, meshes, materials),
        GameMode::Hardpoint => hardpoint::spawn_map(commands, meshes, materials),
        GameMode::CapturePoint => capture_point::spawn_map(commands, meshes, materials),
        // LTM modes reuse standard maps
        GameMode::Juggernaut | GameMode::HighExplosives
        | GameMode::OneInTheChamber | GameMode::GunGame
        | GameMode::Infected => ltm::spawn_map(mode, commands, meshes, materials),
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
        GameMode::FreeForAll => free_for_all::spawn_mode_entities(commands, meshes, materials),
        GameMode::TeamDeathmatch => team_deathmatch::spawn_mode_entities(commands, meshes, materials),
        GameMode::KillConfirmed => kill_confirmed::spawn_mode_entities(commands, meshes, materials),
        GameMode::CaptureTheFlag => capture_the_flag::spawn_mode_entities(commands, meshes, materials),
        GameMode::Assassins => assassins::spawn_mode_entities(commands, meshes, materials),
        GameMode::KingOfTheHill => king_of_the_hill::spawn_mode_entities(commands, meshes, materials),
        GameMode::Hardpoint => hardpoint::spawn_mode_entities(commands, meshes, materials),
        GameMode::CapturePoint => capture_point::spawn_mode_entities(commands, meshes, materials),
        GameMode::Juggernaut | GameMode::HighExplosives
        | GameMode::OneInTheChamber | GameMode::GunGame
        | GameMode::Infected => {} // LTMs have no special entities
    }
}
