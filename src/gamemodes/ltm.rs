//! Limited-Time Mode map helpers.
//!
//! LTMs re-use the standard arena layouts but with different rules.
//! The mode-specific gameplay logic lives in `gameplay.rs`.

use bevy::prelude::*;
use crate::menu::GameMode;

/// Spawn the appropriate map for an LTM mode.
pub fn spawn_map(
    mode: GameMode,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    match mode {
        // FFA-style LTMs → use FFA arena
        GameMode::OneInTheChamber | GameMode::GunGame => {
            super::free_for_all::spawn_map(commands, meshes, materials);
        }
        // Team-style LTMs → use TDM arena
        GameMode::Infected => {
            super::team_deathmatch::spawn_map(commands, meshes, materials);
        }
        // Juggernaut – one tanky player vs everyone else, FFA arena
        GameMode::Juggernaut => {
            super::free_for_all::spawn_map(commands, meshes, materials);
        }
        // High Explosives – explosives only, use FFA arena
        GameMode::HighExplosives => {
            super::free_for_all::spawn_map(commands, meshes, materials);
        }
        _ => {} // non-LTM modes should never reach here
    }
}
