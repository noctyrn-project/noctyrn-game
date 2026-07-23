use bevy::prelude::*;
use bevy::settings::*;

use crate::defaults;

#[derive(Resource, SettingsGroup, Reflect, Debug, Clone)]
#[reflect(Resource, SettingsGroup, Default)]
pub struct GameSettings {
    pub gameplay: GameplaySettings,
    pub graphics: GraphicsSettings,
    pub debug: DebugSettingsConfig,
}

#[derive(Reflect, Debug, Clone)]
pub struct GameplaySettings {
    pub toggle_sprint: bool,
    pub toggle_ads: bool,
    pub toggle_crouch: bool,
    pub sensitivity: f32,
}

#[derive(Reflect, Debug, Clone)]
pub struct GraphicsSettings {
    pub resolution: [u32; 2],
    pub texture_quality: String,
    pub shadow_quality: String,
    pub view_distance: f32,
    pub fps_cap: u32,
    pub fov: f32,
}

#[derive(Reflect, Debug, Clone)]
pub struct DebugSettingsConfig {
    pub show_fps: bool,
    pub max_fps_display: u32,
    pub show_resource_usage: bool,
    pub show_hitboxes: bool,
    pub free_cam: bool,
    pub debug_mode: bool,
    pub show_vertex_count: bool,
    pub show_ping: bool,
    pub show_speed: bool,
    pub show_crosshair_debug: bool,
    pub god_mode: bool,
    pub infinite_ammo: bool,
    pub show_wireframe: bool,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        Self {
            toggle_sprint: false,
            toggle_ads: false,
            toggle_crouch: false,
            sensitivity: defaults::default_sensitivity(),
        }
    }
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            resolution: defaults::default_resolution(),
            texture_quality: "High".to_string(),
            shadow_quality: "High".to_string(),
            view_distance: defaults::default_view_distance(),
            fps_cap: 0,
            fov: defaults::default_fov(),
        }
    }
}

impl Default for DebugSettingsConfig {
    fn default() -> Self {
        Self {
            show_fps: false,
            max_fps_display: 144,
            show_resource_usage: false,
            show_hitboxes: false,
            free_cam: false,
            debug_mode: false,
            show_vertex_count: false,
            show_ping: false,
            show_speed: false,
            show_crosshair_debug: false,
            god_mode: false,
            infinite_ammo: false,
            show_wireframe: false,
        }
    }
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            gameplay: GameplaySettings::default(),
            graphics: GraphicsSettings::default(),
            debug: DebugSettingsConfig::default(),
        }
    }
}
