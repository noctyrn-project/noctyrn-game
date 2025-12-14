use bevy::prelude::*;

/// A vector representing the player's input, accumulated over all frames that ran
/// since the last time the physics simulation was advanced.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default)]
pub struct AccumulatedInput {
    // The player's movement input (WASD), relative to the world (rotated by camera).
    pub movement: Vec3,
    pub jump: bool,
    pub fire: bool,
}

/// Handle keyboard input and accumulate it in the `AccumulatedInput` component.
pub fn accumulate_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut player: Single<&mut AccumulatedInput>,
    camera: Single<&Transform, With<Camera>>,
) {
    let mut movement = Vec2::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) {
        movement.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        movement.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        movement.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
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
    player.jump = keyboard_input.pressed(KeyCode::Space);
    player.fire = mouse_input.pressed(MouseButton::Left);
}

// Clear the input after it was processed in the fixed timestep.
pub fn clear_input(mut input: Single<&mut AccumulatedInput>) {
    **input = AccumulatedInput::default();
}
