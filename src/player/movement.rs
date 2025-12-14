use bevy::prelude::*;
use super::input::AccumulatedInput;

/// A vector representing the player's velocity in the physics simulation.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
pub struct Velocity(pub Vec3);

/// The actual position of the player in the physics simulation.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
pub struct PhysicalTranslation(pub Vec3);

/// The value [`PhysicalTranslation`] had in the last fixed timestep.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
pub struct PreviousPhysicalTranslation(pub Vec3);

/// Advance the physics simulation by one fixed timestep.
pub fn advance_physics(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut PhysicalTranslation,
        &mut PreviousPhysicalTranslation,
        &mut Velocity,
        &AccumulatedInput,
    )>,
) {
    const GRAVITY: f32 = 15.0;
    const JUMP_SPEED: f32 = 6.0;
    const MAX_SPEED: f32 = 8.0;
    const ACCELERATION: f32 = 50.0;
    const FRICTION: f32 = 8.0;

    for (mut current_physical_translation, mut previous_physical_translation, mut velocity, input) in
        query.iter_mut()
    {
        previous_physical_translation.0 = current_physical_translation.0;

        let dt = fixed_time.delta_secs();

        // Ground check
        let is_on_ground = current_physical_translation.y <= 0.0;

        // Apply Friction (only if on ground)
        if is_on_ground {
            let horizontal_velocity = Vec3::new(velocity.x, 0.0, velocity.z);
            let speed = horizontal_velocity.length();
            
            if speed > 0.0 {
                let drop = speed * FRICTION * dt;
                let new_speed = (speed - drop).max(0.0);
                let scale = new_speed / speed;
                velocity.x *= scale;
                velocity.z *= scale;
            }
        }

        // Apply Acceleration
        let wish_dir = input.movement;
        
        if wish_dir.length_squared() > 0.0 {
            let wish_speed = MAX_SPEED;
            
            // Project current velocity onto wish direction
            let current_speed_in_wish_dir = velocity.dot(wish_dir);
            let add_speed = wish_speed - current_speed_in_wish_dir;
            
            if add_speed > 0.0 {
                let accel = if is_on_ground { ACCELERATION } else { ACCELERATION * 0.5 };
                let accel_speed = (accel * dt * wish_speed).min(add_speed);
                velocity.0 += wish_dir * accel_speed;
            }
        }

        // Apply Gravity
        velocity.y -= GRAVITY * dt;

        // Apply Jump
        if is_on_ground && input.jump {
             velocity.y = JUMP_SPEED;
        }

        // Update position
        current_physical_translation.0 += velocity.0 * dt;
        
        // Simple floor collision
        if current_physical_translation.y < 0.0 {
            current_physical_translation.y = 0.0;
            if velocity.y < 0.0 {
                velocity.y = 0.0;
            }
        }
    }
}

pub fn interpolate_rendered_transform(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Transform,
        &PhysicalTranslation,
        &PreviousPhysicalTranslation,
    )>,
) {
    for (mut transform, current_physical_translation, previous_physical_translation) in
        query.iter_mut()
    {
        let previous = previous_physical_translation.0;
        let current = current_physical_translation.0;
        let alpha = fixed_time.overstep_fraction();

        let rendered_translation = previous.lerp(current, alpha);
        transform.translation = rendered_translation;
    }
}
