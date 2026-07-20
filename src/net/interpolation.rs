// Entity interpolation for remote players.
//
// Remote player positions arrive at the server tick rate (64 Hz) but may be
// received irregularly. This module buffers snapshots and interpolates between
// them smoothly to render remote players.

use std::collections::VecDeque;

/// A snapshot of a remote player's state at a specific server time.
#[derive(Clone, Debug)]
pub struct InterpolationSnapshot {
    pub server_time: f64,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub health: f32,
    pub movement_state: u8,
}

/// Manages interpolation for a single remote player.
pub struct InterpolationBuffer {
    pub snapshots: VecDeque<InterpolationSnapshot>,
    /// How far behind real-time to render (in seconds). Higher = smoother but more latency.
    pub interpolation_delay: f64,
    pub max_snapshots: usize,
}

impl Default for InterpolationBuffer {
    fn default() -> Self {
        Self {
            snapshots: VecDeque::new(),
            interpolation_delay: 0.1, // 100ms behind
            max_snapshots: 32,
        }
    }
}

impl InterpolationBuffer {
    pub fn new(delay: f64) -> Self {
        Self {
            snapshots: VecDeque::new(),
            interpolation_delay: delay,
            max_snapshots: 32,
        }
    }

    /// Add a new snapshot from the server.
    pub fn push(&mut self, snapshot: InterpolationSnapshot) {
        self.snapshots.push_back(snapshot);
        while self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
        }
    }

    /// Get the interpolated state at the given render time.
    /// render_time should be current_server_time - interpolation_delay.
    pub fn interpolate(&self, render_time: f64) -> Option<InterpolationSnapshot> {
        if self.snapshots.len() < 2 {
            return self.snapshots.back().cloned();
        }

        // Find the two snapshots that bracket render_time
        let mut before = None;
        let mut after = None;

        for (i, snap) in self.snapshots.iter().enumerate() {
            if snap.server_time <= render_time {
                before = Some(i);
            } else {
                after = Some(i);
                break;
            }
        }

        match (before, after) {
            (Some(b), Some(a)) => {
                let snap_a = &self.snapshots[b];
                let snap_b = &self.snapshots[a];
                let t = if (snap_b.server_time - snap_a.server_time).abs() > f64::EPSILON {
                    ((render_time - snap_a.server_time) / (snap_b.server_time - snap_a.server_time)) as f32
                } else {
                    0.0
                };
                let t = t.clamp(0.0, 1.0);

                Some(InterpolationSnapshot {
                    server_time: render_time,
                    position: lerp3(snap_a.position, snap_b.position, t),
                    velocity: lerp3(snap_a.velocity, snap_b.velocity, t),
                    yaw: lerp_angle(snap_a.yaw, snap_b.yaw, t),
                    pitch: lerp(snap_a.pitch, snap_b.pitch, t),
                    health: lerp(snap_a.health, snap_b.health, t),
                    movement_state: snap_b.movement_state, // Snap to latest
                })
            }
            // If render_time is past all snapshots, extrapolate from last
            (Some(b), None) => {
                self.snapshots.get(b).cloned()
            }
            // If render_time is before all snapshots, use first
            (None, Some(a)) => {
                self.snapshots.get(a).cloned()
            }
            _ => None,
        }
    }

    pub fn clear(&mut self) {
        self.snapshots.clear();
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        lerp(a[0], b[0], t),
        lerp(a[1], b[1], t),
        lerp(a[2], b[2], t),
    ]
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut diff = b - a;
    while diff > std::f32::consts::PI {
        diff -= 2.0 * std::f32::consts::PI;
    }
    while diff < -std::f32::consts::PI {
        diff += 2.0 * std::f32::consts::PI;
    }
    a + diff * t
}
