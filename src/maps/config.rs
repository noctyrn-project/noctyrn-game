use serde::Deserialize;

/// Per-map configuration loaded from `assets/maps/configs/{name}.json`.
/// Only contains client-side visual data (GLB path, lighting).
/// Colliders, spawns, and other gameplay data come from
/// `noctyrn_shared::map_data`.
#[derive(Debug, Deserialize)]
pub struct MapConfig {
    /// Path to the GLB scene (with `#Scene0` label).
    pub glb: String,
    /// Uniform scale applied to the entire map.
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// Scene lighting.
    #[serde(default)]
    pub lights: Vec<LightConfig>,
}

fn default_scale() -> f32 {
    1.0
}

#[derive(Debug, Deserialize)]
pub struct LightConfig {
    pub position: [f32; 3],
    #[serde(default = "default_light_intensity")]
    pub intensity: f32,
    #[serde(default = "default_shadows")]
    pub shadows: bool,
}

fn default_light_intensity() -> f32 {
    15_000_000.0
}

fn default_shadows() -> bool {
    false
}

/// Load a map config from the assets/maps/configs/ directory via include_str.
pub fn load(name: &str) -> MapConfig {
    let json = match name {
        "dust_storm" => include_str!("../../assets/maps/configs/dust_storm.json"),
        "city" => include_str!("../../assets/maps/configs/city.json"),
        _ => panic!("Unknown map config: {name}"),
    };
    serde_json::from_str(json).expect("Invalid map config JSON")
}
