use bevy::prelude::*;

/// Player velocity in world space (units/sec).
///
/// This is the single source of truth for player motion.
/// Movement systems modify this; integration converts it to position.
/// Separating velocity from position enables clean server reconciliation.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
pub struct Velocity(pub Vec3);

/// The actual position of the player in the physics simulation.
///
/// Only modified by the integration and collision systems.
/// All other systems work exclusively through [`Velocity`].
/// This ensures a single write-point for position, making the
/// pipeline deterministic and easy to reconcile.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
pub struct PhysicalTranslation(pub Vec3);

/// The value [`PhysicalTranslation`] had in the previous fixed timestep.
///
/// Used for render interpolation between physics frames so that
/// visuals remain smooth regardless of framerate.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
pub struct PreviousPhysicalTranslation(pub Vec3);

/// Tracks the player's eye height for camera positioning.
///
/// `target` is set by the state transition system based on standing/crouching.
/// `current` is smoothly interpolated toward `target` each frame for visual polish.
#[derive(Debug, Component, Clone, Copy, PartialEq)]
pub struct CrouchHeight {
    pub current: f32,
    pub target: f32,
}

impl Default for CrouchHeight {
    fn default() -> Self {
        Self {
            current: 1.5,
            target: 1.5,
        }
    }
}

/// Ground contact information, updated each physics frame.
///
/// This component is written by the ground detection system and read
/// by jump, friction, acceleration, and state transition systems.
/// Centralizing ground state avoids redundant raycasts across systems.
#[derive(Debug, Component, Clone, Copy, PartialEq)]
pub struct GroundedState {
    /// Whether the player is currently touching a ground surface.
    pub is_grounded: bool,

    /// Whether the player was grounded in the previous physics frame.
    /// Useful for detecting landing events (was_grounded=false → is_grounded=true).
    pub was_grounded: bool,

    /// Seconds elapsed since the player was last grounded.
    /// Used for coyote time calculations in the jump system.
    pub time_since_grounded: f32,

    /// Normal vector of the ground surface.
    /// Vec3::Y for flat ground; angled for ramps.
    pub ground_normal: Vec3,
}

impl Default for GroundedState {
    fn default() -> Self {
        Self {
            is_grounded: false,
            was_grounded: false,
            // Start as "long time since grounded" so coyote time
            // doesn't accidentally fire on spawn.
            time_since_grounded: f32::MAX,
            ground_normal: Vec3::Y,
        }
    }
}

/// The player's current movement state.
///
/// Drives which movement physics are applied each frame.
/// State transitions are handled by a dedicated system—movement
/// systems read this but never write it (clean data flow).
#[derive(Debug, Component, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MovementState {
    /// No movement input; grounded.
    #[default]
    Idle,
    /// Moving at walk speed; grounded.
    Walking,
    /// Moving at sprint speed; grounded.
    Sprinting,
    /// Crouched; grounded. Reduced speed and lower eye height.
    Crouching,
    /// Momentum slide triggered by sprint+crouch. Reduced friction.
    Sliding,
    /// Not touching any ground surface.
    Airborne,
    /// Prone position. Lowest speed and eye height.
    Prone,
}

/// Tracks jump mechanics: coyote time and input buffering.
///
/// # Coyote Time
/// After walking off a ledge, the player has a brief grace window
/// where they can still jump. This compensates for the perceptual
/// delay between the player seeing themselves leave a ledge and the
/// physics simulation losing ground contact.
///
/// # Jump Buffering
/// If the player presses jump slightly before landing, the input
/// is stored and executed the frame they touch down. This prevents
/// "eaten" inputs and makes bunnyhopping more consistent.
#[derive(Debug, Component, Clone, Copy, PartialEq)]
pub struct JumpState {
    /// Time remaining in the coyote window.
    /// Set to `coyote_time` on ground contact; counts down while airborne.
    pub coyote_timer: f32,

    /// Time remaining in the jump buffer window.
    /// Set to `jump_buffer_time` when jump is pressed; counts down each frame.
    pub buffer_timer: f32,

    /// Whether the player has consumed their jump this airborne period.
    /// Reset when grounded again. Prevents double-jumping from coyote time.
    pub has_jumped: bool,
}

impl Default for JumpState {
    fn default() -> Self {
        Self {
            coyote_timer: 0.0,
            buffer_timer: 0.0,
            has_jumped: false,
        }
    }
}

/// Tracks the state and timing of a momentum slide.
///
/// Slides are triggered by sprint+crouch while above a speed threshold.
/// The slide preserves momentum with reduced friction and ends either
/// when speed drops below a threshold or a maximum duration is reached.
#[derive(Debug, Component, Clone, Copy, PartialEq)]
pub struct SlideState {
    /// Whether the player is currently in a slide.
    pub active: bool,

    /// How long the current slide has lasted (seconds).
    pub slide_timer: f32,

    /// Horizontal speed when the slide was initiated.
    /// Stored for potential speed-dependent slide effects.
    pub entry_speed: f32,

    /// World-space direction the player was moving when slide began.
    /// The slide continues in this direction with reduced friction.
    pub slide_direction: Vec3,
}

impl Default for SlideState {
    fn default() -> Self {
        Self {
            active: false,
            slide_timer: 0.0,
            entry_speed: 0.0,
            slide_direction: Vec3::ZERO,
        }
    }
}

/// Tracks the player's lean state (Q/E leaning).
///
/// The `current` value smoothly interpolates toward `target`.
/// Negative = leaning left, Positive = leaning right.
/// The value represents the roll angle in radians.
#[derive(Debug, Component, Clone, Copy, PartialEq)]
pub struct LeanState {
    /// Current lean angle (radians). Negative=left, positive=right.
    pub current: f32,
    /// Target lean angle based on input.
    pub target: f32,
}

impl Default for LeanState {
    fn default() -> Self {
        Self {
            current: 0.0,
            target: 0.0,
        }
    }
}
