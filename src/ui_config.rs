use bevy::prelude::*;
use bevy::settings::*;

use crate::defaults;

#[derive(Resource, SettingsGroup, Reflect, Debug, Clone)]
#[reflect(Resource, SettingsGroup, Default)]
pub struct UiConfig {
    pub crosshair: CrosshairConfig,
    pub health_bar: HealthBarConfig,
    pub ammo_ui: AmmoUiConfig,
    pub kill_feed: KillFeedConfig,
}

#[derive(Reflect, Debug, Clone)]
pub struct CrosshairConfig {
    pub color: [f32; 4],
    pub size: f32,
    pub thickness: f32,
    pub gap: f32,
    pub dot: bool,
    pub dot_size: f32,
}

#[derive(Reflect, Debug, Clone)]
pub struct HealthBarConfig {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub text_color: [f32; 4],
    pub border_radius: f32,
}

#[derive(Reflect, Debug, Clone)]
pub struct AmmoUiConfig {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub border_radius: f32,
}

#[derive(Reflect, Debug, Clone)]
pub struct KillFeedConfig {
    pub position: [f32; 2],
    pub max_items: usize,
    pub item_duration: f32,
    pub text_color: [f32; 4],
    pub background_color: [f32; 4],
    pub border_radius: f32,
}

impl Default for CrosshairConfig {
    fn default() -> Self {
        Self {
            color: defaults::default_crosshair_color(),
            size: defaults::default_crosshair_size(),
            thickness: defaults::default_crosshair_thickness(),
            gap: defaults::default_crosshair_gap(),
            dot: true,
            dot_size: defaults::default_crosshair_dot_size(),
        }
    }
}

impl Default for HealthBarConfig {
    fn default() -> Self {
        Self {
            position: defaults::default_health_bar_position(),
            size: defaults::default_health_bar_size(),
            color: defaults::default_health_bar_color(),
            text_color: defaults::default_text_color(),
            border_radius: 0.0,
        }
    }
}

impl Default for AmmoUiConfig {
    fn default() -> Self {
        Self {
            position: defaults::default_ammo_position(),
            size: defaults::default_ammo_size(),
            color: defaults::default_text_color(),
            border_radius: 0.0,
        }
    }
}

impl Default for KillFeedConfig {
    fn default() -> Self {
        Self {
            position: defaults::default_kill_feed_position(),
            max_items: 5,
            item_duration: 5.0,
            text_color: defaults::default_text_color(),
            background_color: defaults::default_background_color(),
            border_radius: 0.0,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            crosshair: CrosshairConfig::default(),
            health_bar: HealthBarConfig::default(),
            ammo_ui: AmmoUiConfig::default(),
            kill_feed: KillFeedConfig::default(),
        }
    }
}
