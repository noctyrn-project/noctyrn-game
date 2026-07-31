//! Modular, momentum-based first-person movement controller.
//!
//! # Architecture
//!
//! The movement pipeline is split into discrete systems that run in
//! a strictly ordered sequence during `FixedUpdate`:
//!
//! | Order | System                 | Reads                          | Writes                    |
//! |-------|------------------------|--------------------------------|---------------------------|
//! | 1     | Ground Detection       | Position, Velocity, Colliders  | GroundedState             |
//! | 2     | State Transitions      | Input, Ground, Velocity, Config| MovementState, SlideState |
//! | 3     | Jump                   | Input, Ground, Config          | Velocity, JumpState       |
//! | 4     | Acceleration           | Input, State, Ground, Config   | Velocity                  |
//! | 5     | Sliding                | State, Config                  | Velocity, SlideState      |
//! | 6     | Friction               | State, Ground, Config          | Velocity                  |
//! | 7     | Gravity                | Config                         | Velocity                  |
//! | 8     | Velocity Integration   | Velocity, Config               | Position                  |
//! | 9     | Collision Resolution   | Position, Colliders            | Position, Velocity        |
//!
//! # Networking
//!
//! The pipeline is fully deterministic and designed for server-authoritative play:
//! - **No random number generation** in any movement system
//! - **All systems use fixed delta time** from `Time<Fixed>`
//! - **Clear input → output data flow** with no frame-order dependence
//! - **Position is only written** in integration + collision steps
//!
//! To implement client-side prediction: run this pipeline locally with player input.
//! To implement server reconciliation: replay the pipeline from a checkpoint with
//! authoritative input to produce the authoritative position.
//!
//! # Skill Depth
//!
//! The Quake-style acceleration model creates emergent movement techniques:
//! - **Bunnyhopping:** Jump on landing + air strafe to exceed walk speed
//! - **Air strafing:** Perpendicular input in air adds speed via projection math
//! - **Slide chaining:** Sprint → slide → stand → sprint for fast traversal
//! - **Momentum preservation:** Ground-to-air transitions keep horizontal speed
//!
//! These techniques arise naturally from the physics values, not hardcoded tricks.

mod acceleration;
mod collision;
mod components;
mod config;
mod friction;
mod gravity;
mod ground_detection;
mod integration;
mod jump;
mod mantling;
mod sliding;
mod state_transitions;

// ── Re-export all public types ──

pub use components::{
    CrouchHeight, DiveState, GroundedState, JumpState, LeanState, MantleState, MovementState,
    PhysicalTranslation, PreviousPhysicalTranslation, SlideState, Velocity,
};
pub use config::MovementConfig;

// ── Re-export systems for registration by the player plugin ──

pub use acceleration::apply_acceleration;
pub use collision::resolve_collisions;
pub use friction::apply_friction;
pub use gravity::apply_gravity;
pub use ground_detection::detect_ground;
pub use integration::{integrate_velocity, interpolate_rendered_transform};
pub use jump::handle_jump;
pub use mantling::update_mantle;
pub use sliding::apply_slide_physics;
pub use state_transitions::transition_movement_state;

use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MovementSet {
    GroundDetection,
    StateTransitions,
    Jump,
    Acceleration,
    Sliding,
    Friction,
    Gravity,
    Integration,
    Collision,
}
