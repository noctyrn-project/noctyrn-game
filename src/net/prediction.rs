// Client-side prediction and server reconciliation.
//
// This module buffers local player inputs and predicted positions so that
// when a server snapshot arrives, the client can compare its prediction
// with the authoritative server state and correct if needed.

use std::collections::VecDeque;

/// A recorded input with the predicted position after applying it.
#[derive(Clone, Debug)]
pub struct PredictedFrame {
    pub sequence: u32,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
}

/// Manages the prediction buffer for the local player.
#[derive(Default)]
pub struct PredictionBuffer {
    pub frames: VecDeque<PredictedFrame>,
    pub next_sequence: u32,
    /// Maximum number of frames to keep in the buffer.
    pub max_frames: usize,
}

impl PredictionBuffer {
    pub fn new() -> Self {
        Self {
            frames: VecDeque::new(),
            next_sequence: 0,
            max_frames: 128,
        }
    }

    /// Record a new predicted frame.
    pub fn push(&mut self, position: [f32; 3], velocity: [f32; 3]) -> u32 {
        let seq = self.next_sequence;
        self.next_sequence = self.next_sequence.wrapping_add(1);

        self.frames.push_back(PredictedFrame {
            sequence: seq,
            position,
            velocity,
        });

        // Trim old frames
        while self.frames.len() > self.max_frames {
            self.frames.pop_front();
        }

        seq
    }

    /// Acknowledge all inputs up to and including the given sequence number.
    /// Returns the server position if correction is needed.
    pub fn reconcile(
        &mut self,
        last_processed_sequence: u32,
        server_position: [f32; 3],
        threshold: f32,
    ) -> Option<[f32; 3]> {
        // Remove all acknowledged frames
        while let Some(front) = self.frames.front() {
            if front.sequence <= last_processed_sequence {
                self.frames.pop_front();
            } else {
                break;
            }
        }

        // Check if correction is needed by comparing the predicted position
        // at the acknowledged sequence with the server's position
        let dist_sq = (server_position[0] - self.last_acknowledged_position().unwrap_or(server_position)[0]).powi(2)
            + (server_position[1] - self.last_acknowledged_position().unwrap_or(server_position)[1]).powi(2)
            + (server_position[2] - self.last_acknowledged_position().unwrap_or(server_position)[2]).powi(2);

        if dist_sq > threshold * threshold {
            Some(server_position)
        } else {
            None
        }
    }

    fn last_acknowledged_position(&self) -> Option<[f32; 3]> {
        self.frames.front().map(|f| f.position)
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.next_sequence = 0;
    }
}
