use bevy::prelude::*;
use crate::theme::*;

pub const APP_ID: &str = "com.noctyrn.game";

/// Convert a theme color to the [f32; 4] rgba array format used by configs.
fn to_rgba(color: Color, alpha: f32) -> [f32; 4] {
    let c = color.to_srgba();
    [c.red, c.green, c.blue, alpha]
}

pub fn default_sensitivity() -> f32 { 1.0 }
pub fn default_resolution() -> [u32; 2] { [1920, 1080] }
pub fn default_fov() -> f32 { 60.0 }
pub fn default_view_distance() -> f32 { 1000.0 }

pub fn default_crosshair_color() -> [f32; 4] { to_rgba(TEXT, 1.0) }
pub fn default_crosshair_size() -> f32 { 10.0 }
pub fn default_crosshair_thickness() -> f32 { 2.0 }
pub fn default_crosshair_gap() -> f32 { 5.0 }
pub fn default_crosshair_dot_size() -> f32 { 2.0 }

pub fn default_health_bar_color() -> [f32; 4] { to_rgba(SUCCESS, 1.0) }
pub fn default_text_color() -> [f32; 4] { to_rgba(TEXT, 1.0) }
pub fn default_background_color() -> [f32; 4] { to_rgba(BG_PANEL, 0.85) }
pub fn default_health_bar_position() -> [f32; 2] { [20.0, 20.0] }
pub fn default_health_bar_size() -> [f32; 2] { [200.0, 20.0] }

pub fn default_ammo_position() -> [f32; 2] { [20.0, 50.0] }
pub fn default_ammo_size() -> [f32; 2] { [100.0, 30.0] }

pub fn default_kill_feed_position() -> [f32; 2] { [20.0, 20.0] }

pub fn default_keybinds() -> [(String, KeyCode); 18] {
    [
        ("move_forward".into(), KeyCode::KeyW),
        ("move_backward".into(), KeyCode::KeyS),
        ("move_left".into(), KeyCode::KeyA),
        ("move_right".into(), KeyCode::KeyD),
        ("jump".into(), KeyCode::Space),
        ("sprint".into(), KeyCode::ShiftLeft),
        ("crouch".into(), KeyCode::ControlLeft),
        ("interact".into(), KeyCode::KeyF),
        ("grenade".into(), KeyCode::KeyG),
        ("melee".into(), KeyCode::KeyV),
        ("stats".into(), KeyCode::Tab),
        ("pause".into(), KeyCode::Escape),
        ("reload".into(), KeyCode::KeyR),
        ("prone".into(), KeyCode::KeyZ),
        ("dive".into(), KeyCode::KeyC),
        ("lean_left".into(), KeyCode::KeyQ),
        ("lean_right".into(), KeyCode::KeyE),
        ("scoreboard".into(), KeyCode::Tab),
    ]
}

pub fn default_mouse_binds() -> [(String, MouseButton); 2] {
    [
        ("shoot".into(), MouseButton::Left),
        ("ads".into(), MouseButton::Right),
    ]
}

pub fn default_starting_credits() -> u64 { 500 }
