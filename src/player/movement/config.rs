use bevy::prelude::*;

/// All tunable movement parameters exposed as a single component.
///
/// Attach this to any entity that uses the movement pipeline.
/// Every movement system reads from this component—no magic numbers
/// anywhere in the pipeline.
///
/// # Tuning Guide
///
/// **Ground feel:** Adjust `ground_acceleration` and `ground_friction` together.
/// Higher acceleration + higher friction = snappy, responsive movement.
/// Lower values = floatier, momentum-heavy feel.
///
/// **Air control:** `air_speed_cap` is the key bunnyhopping parameter.
/// Lower values = more restricted air movement but stronger strafe gains.
/// `air_acceleration` controls how quickly air strafing takes effect.
///
/// **Slide:** `slide_friction` vs `ground_friction` ratio determines how
/// much faster sliding feels compared to walking. `slide_boost` adds
/// an initial kick. `slide_speed_threshold` sets the entry barrier.
#[derive(Debug, Component, Clone)]
pub struct MovementConfig {
    // ── Ground Movement ──

    /// Maximum horizontal speed when walking (units/sec).
    pub max_walk_speed: f32,

    /// Maximum horizontal speed when sprinting (units/sec).
    pub max_sprint_speed: f32,

    /// Maximum horizontal speed when crouching (units/sec).
    pub max_crouch_speed: f32,

    /// How quickly the player accelerates on the ground (units/sec²).
    pub ground_acceleration: f32,

    /// Friction coefficient applied while grounded.
    /// Higher values = faster deceleration when not providing input.
    pub ground_friction: f32,

    /// Multiplier applied when sharply reversing direction on the ground.
    /// Lower values (e.g. 0.5) = snappier direction changes.
    /// 1.0 = no penalty.
    pub direction_change_penalty: f32,

    // ── Air Movement ──

    /// Air acceleration rate (units/sec²).
    /// Controls how quickly air strafing takes effect.
    pub air_acceleration: f32,

    /// Maximum speed that can be *added* via air strafing per axis.
    /// This is the Quake-style air speed cap that enables bunnyhopping—
    /// the player can exceed this total speed by strafing perpendicular
    /// to their velocity vector.
    pub air_speed_cap: f32,

    /// Absolute maximum horizontal speed (hard clamp).
    /// Prevents infinite speed exploits while preserving skill expression.
    pub max_horizontal_speed: f32,

    // ── Jump ──

    /// Instantaneous vertical velocity applied on jump (units/sec).
    pub jump_force: f32,

    /// Grace period after leaving ground where jump is still allowed (seconds).
    /// Makes ledge jumps feel fair by compensating for perception lag.
    pub coyote_time: f32,

    /// Window before landing where a jump input is queued (seconds).
    /// Prevents "eaten" inputs and enables consistent bunnyhopping.
    pub jump_buffer_time: f32,

    // ── Gravity ──

    /// Downward acceleration applied manually each physics frame (units/sec²).
    /// Not relying on the physics engine default ensures deterministic behavior.
    pub gravity: f32,

    // ── Slide ──

    /// Minimum horizontal speed required to initiate a slide (units/sec).
    /// Must be sprinting + crouching and above this threshold.
    pub slide_speed_threshold: f32,

    /// Horizontal speed at which an active slide auto-ends (units/sec).
    pub slide_end_speed: f32,

    /// Friction applied during a slide (much lower than `ground_friction`).
    pub slide_friction: f32,

    /// Maximum duration of a slide before it auto-ends (seconds).
    pub slide_max_duration: f32,

    /// Speed boost applied in the movement direction when entering a slide.
    pub slide_boost: f32,

    // ── Crouch / Height ──

    /// Eye height when standing (units above feet).
    pub stand_height: f32,

    /// Eye height when crouching or sliding (units above feet).
    pub crouch_height_val: f32,

    /// Speed of the camera height transition (lerp rate per second).
    pub crouch_transition_speed: f32,

    // ── Collision ──

    /// Horizontal collision radius of the player capsule (units).
    pub player_radius: f32,

    /// Small margin below feet used for ground surface detection (units).
    pub foot_margin: f32,

    /// Half-extent of the playable map area. Players are clamped within ±this value.
    pub map_half_extent: f32,

    // ── Prone ──

    /// Maximum horizontal speed when prone (units/sec).
    pub max_prone_speed: f32,

    /// Eye height when prone (units above feet).
    pub prone_height: f32,

    // ── Lean ──

    /// Maximum lean angle in radians (~15°).
    pub lean_angle: f32,

    /// Speed at which lean interpolates (lerp rate per second).
    pub lean_speed: f32,
}

impl Default for MovementConfig {
    fn default() -> Self {
        Self {
            // Ground — tuned down ~20% from previous Phantom Forces values for better feel
            max_walk_speed: 11.0,
            max_sprint_speed: 16.0,
            max_crouch_speed: 5.0,
            ground_acceleration: 55.0, // Lower for gradual ramp-up to max speed
            ground_friction: 10.0,
            direction_change_penalty: 0.6,

            // Air — moderate air control, no bunnyhopping exploits
            air_acceleration: 10.0,
            air_speed_cap: 1.8,
            max_horizontal_speed: 18.0,

            // Jump — slightly toned down
            jump_force: 6.5,
            coyote_time: 0.12,
            jump_buffer_time: 0.1,

            // Gravity — slightly lighter for longer hang time
            gravity: 20.0,

            // Slide — quick burst slide, sprint → slide chaining
            slide_speed_threshold: 8.0,
            slide_end_speed: 1.0,
            slide_friction: 2.0,
            slide_max_duration: 3.0,
            slide_boost: 1.5,

            // Crouch / Height
            stand_height: 1.5,
            crouch_height_val: 0.8,
            crouch_transition_speed: 12.0,

            // Collision
            player_radius: 0.4,
            foot_margin: 0.08,
            map_half_extent: 140.0,

            // Prone
            max_prone_speed: 2.5,
            prone_height: 0.4,

            // Lean
            lean_angle: 0.26, // ~15 degrees
            lean_speed: 10.0,
        }
    }
}
