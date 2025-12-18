use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
pub struct GameSettings {
    pub gameplay: GameplaySettings,
    pub graphics: GraphicsSettings,
    pub debug: DebugSettingsConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameplaySettings {
    pub toggle_sprint: bool,
    pub toggle_ads: bool,
    pub toggle_crouch: bool,
    pub sensitivity: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GraphicsSettings {
    pub resolution: [u32; 2], // Width, Height
    pub texture_quality: String, // "Low", "Medium", "High"
    pub shadow_quality: String,
    pub view_distance: f32,
    pub fps_cap: u32, // 0 for unlimited
    pub fov: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DebugSettingsConfig {
    pub show_fps: bool,
    pub max_fps_display: u32,
    pub show_resource_usage: bool,
    pub show_hitboxes: bool,
    pub free_cam: bool,
    #[serde(default)]
    pub god_mode: bool,
    #[serde(default)]
    pub infinite_ammo: bool,
    #[serde(default)]
    pub show_wireframe: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            gameplay: GameplaySettings {
                toggle_sprint: false,
                toggle_ads: false,
                toggle_crouch: false,
                sensitivity: 1.0,
            },
            graphics: GraphicsSettings {
                resolution: [1920, 1080],
                texture_quality: "High".to_string(),
                shadow_quality: "High".to_string(),
                view_distance: 1000.0,
                fps_cap: 0,
                fov: 60.0,
            },
            debug: DebugSettingsConfig {
                show_fps: false,
                max_fps_display: 144,
                show_resource_usage: false,
                show_hitboxes: false,
                free_cam: false,
                god_mode: false,
                infinite_ammo: false,
                show_wireframe: false,
            },
        }
    }
}

pub fn load_game_settings() -> GameSettings {
    let path = "settings/game.toml";
    match fs::read_to_string(path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Failed to parse game.toml: {}. Using defaults.", e);
                GameSettings::default()
            }
        },
        Err(_) => {
            // Create default if not exists
            let settings = GameSettings::default();
            save_game_settings(&settings);
            settings
        }
    }
}

pub fn save_game_settings(settings: &GameSettings) {
    let path = "settings/game.toml";
    if let Ok(content) = toml::to_string_pretty(settings) {
        let _ = fs::write(path, content);
    }
}
