use bevy::prelude::Resource;
use std::collections::VecDeque;

use noctyrn_shared::movement::{self, BodyState, MovementParams};
use noctyrn_shared::protocol::PlayerInput;

/// A recorded input with the predicted state after applying it.
#[derive(Clone, Debug)]
pub struct PredictedFrame {
    pub sequence: u32,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    /// The input that led to this state. Stored so we can replay it during
    /// reconciliation.
    pub input: PlayerInput,
}

/// Manages client-side prediction and server reconciliation.
///
/// # How it works
///
/// 1. Each tick we push the current player state + the input we're about to
///    send to the server into the buffer.
/// 2. When a server snapshot arrives with `last_processed_input = N`, we pop
///    all frames up to N (they're acknowledged).
/// 3. We compare the predicted position at frame N with the server's
///    authoritative position. If they differ (desync), we **rewind** to the
///    server position and **replay** all unacknowledged inputs on top,
///    producing a corrected current state.
/// 4. The corrected state is smoothed toward using exponential lerp rather
///    than a hard snap, unless the error is very large (>2.0 units).
#[derive(Resource, Default)]
pub struct PredictionBuffer {
    pub frames: VecDeque<PredictedFrame>,
    pub next_sequence: u32,
    pub max_frames: usize,
    /// Fixed timestep delta used during replay (matches server tick rate).
    pub dt: f32,
}

impl PredictionBuffer {
    pub fn new() -> Self {
        Self {
            frames: VecDeque::new(),
            next_sequence: 0,
            max_frames: 128,
            dt: 1.0 / 64.0,
        }
    }

    /// Record a new predicted frame.
    pub fn push(&mut self, position: [f32; 3], velocity: [f32; 3], input: &PlayerInput) -> u32 {
        let seq = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);

        self.frames.push_back(PredictedFrame {
            sequence: seq,
            position,
            velocity,
            input: input.clone(),
        });

        while self.frames.len() > self.max_frames {
            self.frames.pop_front();
        }

        seq
    }

    /// Reconcile with the server: remove acknowledged frames, then if the
    /// server position diverges from our prediction, replay unacknowledged
    /// inputs from the server's state to produce a corrected position.
    ///
    /// Returns `Some((corrected_position, corrected_velocity))` if a
    /// correction is needed, `None` if our prediction matches the server.
    pub fn reconcile_and_replay(
        &mut self,
        last_processed_sequence: u32,
        server_position: [f32; 3],
        server_velocity: [f32; 3],
    ) -> Option<([f32; 3], [f32; 3])> {
        let mut last_ack_state: Option<([f32; 3], [f32; 3])> = None;

        // Remove all acknowledged frames, tracking the last one.
        while let Some(front) = self.frames.front() {
            if front.sequence <= last_processed_sequence {
                last_ack_state = Some((front.position, front.velocity));
                self.frames.pop_front();
            } else {
                break;
            }
        }

        // Check if correction is needed.
        let needs_correction = match last_ack_state {
            Some((predicted_pos, _)) => {
                let dx = server_position[0] - predicted_pos[0];
                let dy = server_position[1] - predicted_pos[1];
                let dz = server_position[2] - predicted_pos[2];
                (dx * dx + dy * dy + dz * dz) > 0.04 // 0.2²
            }
            None => false,
        };

        if !needs_correction {
            return None;
        }

        // Replay: start from the server's authoritative state and re-apply
        // all unacknowledged inputs.
        let mut state = BodyState {
            position: server_position,
            velocity: server_velocity,
            yaw: 0.0,
            pitch: 0.0,
            grounded: false,
        };

        let params = MovementParams::default();
        let dt = self.dt;

        // Collect the frames to replay (they need updating too).
        let unacked: Vec<PredictedFrame> = self.frames.drain(..).collect();

        for frame in &unacked {
            movement::advance(&mut state, &frame.input, &params, dt);

            // Update the stored position/velocity for the replayed frame.
            self.frames.push_back(PredictedFrame {
                sequence: frame.sequence,
                position: state.position,
                velocity: state.velocity,
                input: frame.input.clone(),
            });
        }

        Some((state.position, state.velocity))
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.next_sequence = 0;
    }
}
