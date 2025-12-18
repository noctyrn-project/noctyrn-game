use bevy::prelude::*;
use serde::Deserialize;
use std::fs;

#[derive(Resource, Deserialize, Debug, Clone)]
pub struct UiConfig {
    pub crosshair: CrosshairConfig,
    pub health_bar: HealthBarConfig,
    pub ammo_ui: AmmoUiConfig,
    pub kill_feed: KillFeedConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CrosshairConfig {
    pub color: [f32; 4],
    pub size: f32,
    pub thickness: f32,
    pub gap: f32,
    pub dot: bool,
    pub dot_size: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct HealthBarConfig {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub text_color: [f32; 4],
    #[serde(default)]
    pub border_radius: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AmmoUiConfig {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    #[serde(default)]
    pub border_radius: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KillFeedConfig {
    pub position: [f32; 2],
    pub max_items: usize,
    pub item_duration: f32,
    pub text_color: [f32; 4],
    pub background_color: [f32; 4],
    #[serde(default)]
    pub border_radius: f32,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            crosshair: CrosshairConfig {
                color: [0.0, 1.0, 0.0, 1.0],
                size: 10.0,
                thickness: 2.0,
                gap: 5.0,
                dot: true,
                dot_size: 2.0,
            },
            health_bar: HealthBarConfig {
                position: [20.0, 20.0],
                size: [200.0, 20.0],
                color: [1.0, 0.0, 0.0, 1.0],
                text_color: [1.0, 1.0, 1.0, 1.0],
                border_radius: 0.0,
            },
            ammo_ui: AmmoUiConfig {
                position: [20.0, 50.0],
                size: [100.0, 30.0],
                color: [1.0, 1.0, 1.0, 1.0],
                border_radius: 0.0,
            },
            kill_feed: KillFeedConfig {
                position: [20.0, 20.0],
                max_items: 5,
                item_duration: 5.0,
                text_color: [1.0, 1.0, 1.0, 1.0],
                background_color: [0.0, 0.0, 0.0, 0.5],
                border_radius: 0.0,
            },
        }
    }
}

pub fn load_ui_config() -> UiConfig {
    let path = "settings/ui.toml";
    match fs::read_to_string(path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Failed to parse ui.toml: {}. Using defaults.", e);
                UiConfig::default()
            }
        },
        Err(e) => {
            eprintln!("Failed to read ui.toml: {}. Using defaults.", e);
            UiConfig::default()
        }
    }
}
