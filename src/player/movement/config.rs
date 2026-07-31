use bevy::prelude::*;

use super::components::MovementState;
use crate::player::input::AccumulatedInput;

#[derive(Debug, Component, Clone)]
pub struct MovementConfig {
    // ── Ground Movement ──
    pub max_walk_speed: f32,
    pub max_sprint_speed: f32,
    pub max_crouch_speed: f32,
    pub ground_acceleration: f32,
    pub ground_friction: f32,

    // ── Air Movement (near-zero for COD style) ──
    pub air_acceleration: f32,
    pub air_speed_cap: f32,

    // ── Jump ──
    pub jump_force: f32,
    pub coyote_time: f32,
    pub jump_buffer_time: f32,

    // ── Gravity ──
    pub gravity: f32,

    // ── Slide ──
    pub slide_speed_threshold: f32,
    pub slide_end_speed: f32,
    pub slide_friction: f32,
    pub slide_max_duration: f32,
    pub slide_boost: f32,
    pub slide_cancel_speed_preservation: f32,

    // ── Step-up ──
    pub step_up_height: f32,

    // ── Directional Speed Multipliers ──
    pub strafe_speed_multiplier: f32,
    pub backpedal_speed_multiplier: f32,
    pub ads_speed_multiplier: f32,

    // ── Dolphin Dive ──
    pub dive_boost: f32,
    pub dive_duration: f32,
    /// Vertical pop applied when diving from a ground sprint.
    pub dive_lift: f32,
    /// Hard cap on dive horizontal speed — never faster than slide canceling
    /// at its fastest (sprint 16 × 1.25 slide boost × 0.85 preservation = 17).
    pub dive_max_speed: f32,

    // ── Mantle ──
    pub mantle_range: f32,
    pub mantle_max_height: f32,
    pub mantle_min_height: f32,
    pub mantle_duration: f32,

    // ── Crouch / Height ──
    pub stand_height: f32,
    pub crouch_height_val: f32,
    pub crouch_transition_speed: f32,

    // ── Collision ──
    pub player_radius: f32,
    pub foot_margin: f32,

    // ── Prone ──
    pub max_prone_speed: f32,
    pub prone_height: f32,

    // ── Lean ──
    pub lean_angle: f32,
    pub lean_speed: f32,
    /// Lateral camera offset (units) at full lean, for peeking around cover.
    pub lean_translate: f32,
}

impl MovementConfig {
    pub fn effective_max_speed(
        &self,
        state: MovementState,
        input: &AccumulatedInput,
        is_ads: bool,
    ) -> f32 {
        let base = match state {
            MovementState::Sprinting | MovementState::Sliding => self.max_sprint_speed,
            MovementState::Crouching => self.max_crouch_speed,
            MovementState::Prone => self.max_prone_speed,
            _ => self.max_walk_speed,
        };

        let raw = input.raw_movement;
        let dir_mult = if raw.y < 0.0 {
            self.backpedal_speed_multiplier
        } else if raw.y == 0.0 && raw.x.abs() > 0.0 {
            self.strafe_speed_multiplier
        } else {
            1.0
        };

        let ads_mult = if is_ads { self.ads_speed_multiplier } else { 1.0 };

        base * dir_mult * ads_mult
    }
}

impl Default for MovementConfig {
    fn default() -> Self {
        Self {
            // Ground — high accel + high friction for snappy COD feel
            max_walk_speed: 11.0,
            max_sprint_speed: 16.0,
            max_crouch_speed: 5.0,
            ground_acceleration: 80.0,
            ground_friction: 15.0,

            // Air — near-zero control, no bunnyhopping
            air_acceleration: 0.5,
            air_speed_cap: 0.1,

            // Jump
            jump_force: 6.5,
            coyote_time: 0.12,
            jump_buffer_time: 0.1,

            // Gravity
            gravity: 20.0,

            // Slide
            slide_speed_threshold: 8.0,
            slide_end_speed: 1.0,
            slide_friction: 2.0,
            slide_max_duration: 1.0,
            slide_boost: 1.5,
            slide_cancel_speed_preservation: 0.85,

            // Step-up (auto-step obstacles up to 0.25m)
            step_up_height: 0.25,

            // Directional speed
            strafe_speed_multiplier: 0.85,
            backpedal_speed_multiplier: 0.65,
            ads_speed_multiplier: 0.5,

            // Dive
            dive_boost: 8.0,
            dive_duration: 0.4,
            dive_lift: 3.5,
            dive_max_speed: 17.0,

            // Mantle
            mantle_range: 1.5,
            mantle_max_height: 1.5,
            mantle_min_height: 0.5,
            mantle_duration: 0.5,

            // Heights
            stand_height: 1.5,
            crouch_height_val: 0.8,
            crouch_transition_speed: 12.0,

            // Collision
            player_radius: 0.4,
            foot_margin: 0.08,

            // Prone
            max_prone_speed: 2.5,
            prone_height: 0.25,

            // Lean
            lean_angle: 0.26,
            lean_speed: 10.0,
            lean_translate: 0.35,
        }
    }
}
