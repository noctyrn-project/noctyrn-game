use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Resource, Debug, Serialize, Deserialize, Clone)]
pub struct Keybinds {
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub jump: KeyCode,
    pub sprint: KeyCode,
    pub crouch: KeyCode,
    pub interact: KeyCode,
    pub grenade: KeyCode,
    pub melee: KeyCode,
    pub stats: KeyCode,
    pub pause: KeyCode,
}

impl Default for Keybinds {
    fn default() -> Self {
        Self {
            forward: KeyCode::KeyW,
            backward: KeyCode::KeyS,
            left: KeyCode::KeyA,
            right: KeyCode::KeyD,
            jump: KeyCode::Space,
            sprint: KeyCode::ShiftLeft,
            crouch: KeyCode::ControlLeft,
            interact: KeyCode::KeyE,
            grenade: KeyCode::KeyG,
            melee: KeyCode::KeyF,
            stats: KeyCode::F3,
            pause: KeyCode::Escape,
        }
    }
}

pub fn load_keybinds(mut commands: Commands) {
    let path = "keybinds.json";
    let keybinds = if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or_else(|e| {
            warn!("Failed to parse keybinds.json: {}. Using defaults.", e);
            Keybinds::default()
        })
    } else {
        warn!("keybinds.json not found. Using defaults.");
        Keybinds::default()
    };
    commands.insert_resource(keybinds);
}

/// A vector representing the player's input, accumulated over all frames that ran
/// since the last time the physics simulation was advanced.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default)]
pub struct AccumulatedInput {
    // The player's movement input (WASD), relative to the world (rotated by camera).
    pub movement: Vec3,
    pub jump: bool,
    pub sprint: bool,
    pub crouch: bool,
    pub fire: bool,
    pub aim: bool,
    pub stats: bool,
    pub pause: bool,
}

/// Handle keyboard input and accumulate it in the `AccumulatedInput` component.
pub fn accumulate_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keybinds: Res<Keybinds>,
    mut player: Single<&mut AccumulatedInput>,
    camera: Single<&Transform, With<Camera>>,
) {
    let mut movement = Vec2::ZERO;
    if keyboard_input.pressed(keybinds.forward) {
        movement.y += 1.0;
    }
    if keyboard_input.pressed(keybinds.backward) {
        movement.y -= 1.0;
    }
    if keyboard_input.pressed(keybinds.left) {
        movement.x -= 1.0;
    }
    if keyboard_input.pressed(keybinds.right) {
        movement.x += 1.0;
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

    player.movement = wish_dir;
    player.jump = keyboard_input.pressed(keybinds.jump);
    player.aim = mouse_input.pressed(MouseButton::Right);
    
    // Only sprint if moving forward and not aiming
    player.sprint = keyboard_input.pressed(keybinds.sprint) && movement.y > 0.0 && !player.aim;
    
    player.crouch = keyboard_input.pressed(keybinds.crouch); // Removed hardcoded KeyC
    player.fire = mouse_input.pressed(MouseButton::Left);
    player.stats = keyboard_input.just_pressed(keybinds.stats);
    player.pause = keyboard_input.just_pressed(keybinds.pause);
}

// Clear the input after it was processed in the fixed timestep.
pub fn clear_input(mut input: Single<&mut AccumulatedInput>) {
    **input = AccumulatedInput::default();
}

impl Keybinds {
    pub fn set(&mut self, action: &str, key: KeyCode) {
        match action {
            "Forward" => self.forward = key,
            "Backward" => self.backward = key,
            "Left" => self.left = key,
            "Right" => self.right = key,
            "Jump" => self.jump = key,
            "Sprint" => self.sprint = key,
            "Crouch" => self.crouch = key,
            "Interact" => self.interact = key,
            "Grenade" => self.grenade = key,
            "Melee" => self.melee = key,
            "Stats" => self.stats = key,
            "Pause" => self.pause = key,
            _ => warn!("Unknown keybind action: {}", action),
        }
    }

    pub fn get(&self, action: &str) -> KeyCode {
        match action {
            "Forward" => self.forward,
            "Backward" => self.backward,
            "Left" => self.left,
            "Right" => self.right,
            "Jump" => self.jump,
            "Sprint" => self.sprint,
            "Crouch" => self.crouch,
            "Interact" => self.interact,
            "Grenade" => self.grenade,
            "Melee" => self.melee,
            "Stats" => self.stats,
            "Pause" => self.pause,
            _ => KeyCode::KeyW,
        }
    }
}

pub fn save_keybinds(keybinds: &Keybinds) {
    let path = "keybinds.json";
    if let Ok(content) = serde_json::to_string_pretty(keybinds) {
        if let Err(e) = fs::write(path, content) {
            warn!("Failed to save keybinds.json: {}", e);
        }
    } else {
        warn!("Failed to serialize keybinds.");
    }
}
