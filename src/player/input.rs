use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use toml;
use crate::settings::GameSettings;
use serde_json;

#[derive(Resource, Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Keybinds {
    #[serde(alias = "forward")]
    pub move_forward: KeyCode,
    #[serde(alias = "backward")]
    pub move_backward: KeyCode,
    #[serde(alias = "left")]
    pub move_left: KeyCode,
    #[serde(alias = "right")]
    pub move_right: KeyCode,
    pub jump: KeyCode,
    pub sprint: KeyCode,
    pub crouch: KeyCode,
    pub interact: KeyCode,
    pub grenade: KeyCode,
    pub melee: KeyCode,
    pub stats: KeyCode,
    pub pause: KeyCode,
    pub shoot: MouseButton,
    pub ads: MouseButton,
    pub reload: KeyCode,
    pub prone: KeyCode,
    pub lean_left: KeyCode,
    pub lean_right: KeyCode,
    pub scoreboard: KeyCode,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            move_forward: KeyCode::KeyW,
            move_backward: KeyCode::KeyS,
            move_left: KeyCode::KeyA,
            move_right: KeyCode::KeyD,
            jump: KeyCode::Space,
            sprint: KeyCode::ShiftLeft,
            crouch: KeyCode::ControlLeft,
            interact: KeyCode::KeyF,
            grenade: KeyCode::KeyG,
            melee: KeyCode::KeyV,
            stats: KeyCode::Tab,
            pause: KeyCode::Escape,
            shoot: MouseButton::Left,
            ads: MouseButton::Right,
            reload: KeyCode::KeyR,
            prone: KeyCode::KeyZ,
            lean_left: KeyCode::KeyQ,
            lean_right: KeyCode::KeyE,
            scoreboard: KeyCode::Tab,
        }
    }
}

pub fn load_keybinds(mut commands: Commands) {
    let path = "settings/keybinds.toml";
    let old_path = "keybinds.json";

    let keybinds = if let Ok(content) = fs::read_to_string(path) {
        toml::from_str(&content).unwrap_or_else(|e| {
            warn!("Failed to parse keybinds.toml: {}. Using defaults.", e);
            Keybinds::default()
        })
    } else if let Ok(content) = fs::read_to_string(old_path) {
        warn!("Migrating keybinds.json to settings/keybinds.toml");
        let keybinds: Keybinds = serde_json::from_str(&content).unwrap_or_else(|e| {
             warn!("Failed to parse old keybinds.json: {}. Using defaults.", e);
             Keybinds::default()
        });
        save_keybinds(&keybinds);
        let _ = fs::remove_file(old_path);
        keybinds
    } else {
        warn!("keybinds.toml not found. Using defaults.");
        let defaults = Keybinds::default();
        save_keybinds(&defaults);
        defaults
    };
    commands.insert_resource(keybinds);
}

/// A vector representing the player's input, accumulated over all frames that ran
/// since the last time the physics simulation was advanced.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default)]
pub struct AccumulatedInput {
    // The player's movement input (WASD), relative to the world (rotated by camera).
    pub movement: Vec3,
    // The raw local movement input (WASD).
    pub raw_movement: Vec2,
    pub jump: bool,
    pub sprint: bool,
    pub crouch: bool,
    pub fire: bool,
    pub aim: bool,
    pub stats: bool,
    pub pause: bool,
    pub prone: bool,
    pub lean_left: bool,
    pub lean_right: bool,
}

/// Handle keyboard input and accumulate it in the `AccumulatedInput` component.
pub fn accumulate_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keybinds: Res<Keybinds>,
    game_settings: Res<GameSettings>,
    mut player: Single<(&mut AccumulatedInput, &mut PlayerToggleState)>,
    camera: Single<&Transform, With<super::MainCamera>>,
    terminal_open: Res<super::WeaponTerminalOpen>,
    pause_open: Res<super::PauseMenuOpen>,
) {
    if terminal_open.0 || pause_open.0 { return; }
    let (mut input, mut toggle_state) = player.into_inner();
    let mut movement = Vec3::ZERO;
    if keyboard_input.pressed(keybinds.move_forward) {
        movement.y += 1.0;
    }
    if keyboard_input.pressed(keybinds.move_backward) {
        movement.y -= 1.0;
    }
    if keyboard_input.pressed(keybinds.move_right) {
        movement.x += 1.0;
    }
    if keyboard_input.pressed(keybinds.move_left) {
        movement.x -= 1.0;
    }

    // Calculate forward and right vectors on the horizontal plane
    let forward = camera.forward();
    let right = camera.right();
    
    let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    // Calculate wish direction
    let mut wish_dir = forward_flat * movement.y + right_flat * movement.x;

    // Normalize if length > 1 to prevent faster diagonal movement
    if wish_dir.length_squared() > 1.0 {
        wish_dir = wish_dir.normalize();
    }

    input.movement = wish_dir;
    input.raw_movement = Vec2::new(movement.x, movement.y);
    input.jump = keyboard_input.pressed(keybinds.jump);

    // ADS Logic
    if game_settings.gameplay.toggle_ads {
        if mouse_input.just_pressed(keybinds.ads) {
            toggle_state.ads = !toggle_state.ads;
        }
        input.aim = toggle_state.ads;
    } else {
        input.aim = mouse_input.pressed(keybinds.ads);
        toggle_state.ads = input.aim; // Sync state
    }
    
    // Sprint Logic
    if game_settings.gameplay.toggle_sprint {
        if keyboard_input.just_pressed(keybinds.sprint) {
            toggle_state.sprint = !toggle_state.sprint;
        }
        // Reset sprint if stopped moving or aiming
        if movement.y <= 0.0 || input.aim {
            toggle_state.sprint = false;
        }
        input.sprint = toggle_state.sprint;
    } else {
        input.sprint = keyboard_input.pressed(keybinds.sprint) && movement.y > 0.0 && !input.aim;
        toggle_state.sprint = input.sprint;
    }
    
    // Crouch Logic
    if game_settings.gameplay.toggle_crouch {
        if keyboard_input.just_pressed(keybinds.crouch) {
            toggle_state.crouch = !toggle_state.crouch;
        }
        input.crouch = toggle_state.crouch;
    } else {
        input.crouch = keyboard_input.pressed(keybinds.crouch);
        toggle_state.crouch = input.crouch;
    }

    input.fire = mouse_input.pressed(keybinds.shoot);
    input.stats = keyboard_input.just_pressed(keybinds.stats);
    input.pause = keyboard_input.just_pressed(keybinds.pause);
    input.prone = keyboard_input.just_pressed(keybinds.prone);
    input.lean_left = keyboard_input.pressed(keybinds.lean_left);
    input.lean_right = keyboard_input.pressed(keybinds.lean_right);
}

// Clear the input after it was processed in the fixed timestep.
pub fn clear_input(mut input: Single<&mut AccumulatedInput>) {
    **input = AccumulatedInput::default();
}

impl Keybinds {
    pub fn set(&mut self, action: &str, key: KeyCode) {
        match action {
            "Move Forward" => self.move_forward = key,
            "Move Backward" => self.move_backward = key,
            "Move Left" => self.move_left = key,
            "Move Right" => self.move_right = key,
            "Jump" => self.jump = key,
            "Sprint" => self.sprint = key,
            "Crouch" => self.crouch = key,
            "Interact" => self.interact = key,
            "Grenade" => self.grenade = key,
            "Melee" => self.melee = key,
            "Stats" => self.stats = key,
            "Pause" => self.pause = key,
            "Reload" => self.reload = key,
            "Prone" => self.prone = key,
            "Lean Left" => self.lean_left = key,
            "Lean Right" => self.lean_right = key,
            "Scoreboard" => self.scoreboard = key,
            _ => warn!("Unknown keybind action: {}", action),
        }
    }

    pub fn get(&self, action: &str) -> KeyCode {
        match action {
            "Move Forward" => self.move_forward,
            "Move Backward" => self.move_backward,
            "Move Left" => self.move_left,
            "Move Right" => self.move_right,
            "Jump" => self.jump,
            "Sprint" => self.sprint,
            "Crouch" => self.crouch,
            "Interact" => self.interact,
            "Grenade" => self.grenade,
            "Melee" => self.melee,
            "Stats" => self.stats,
            "Pause" => self.pause,
            "Reload" => self.reload,
            "Prone" => self.prone,
            "Lean Left" => self.lean_left,
            "Lean Right" => self.lean_right,
            "Scoreboard" => self.scoreboard,
            _ => KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified),
        }
    }
}

pub fn save_keybinds(keybinds: &Keybinds) {
    let path = "settings/keybinds.toml";
    if let Ok(content) = toml::to_string_pretty(keybinds) {
        if let Err(e) = fs::write(path, content) {
            warn!("Failed to save keybinds.toml: {}", e);
        }
    } else {
        warn!("Failed to serialize keybinds.");
    }
}

#[derive(Component, Default)]
pub struct PlayerToggleState {
    pub sprint: bool,
    pub crouch: bool,
    pub ads: bool,
}
