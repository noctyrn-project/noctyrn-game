// UDP client for real-time gameplay synchronization.
// Handles sending player inputs and receiving game state snapshots.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

use bevy::prelude::*;

use noctyrn_shared::protocol::{GameStateSnapshot, PlayerInput};


/// Bevy resource wrapping the UDP connection for game traffic.
///
/// Snapshots received by the background reader are placed in a shared buffer
/// and drained by a Bevy system each frame.
#[derive(Resource, Clone)]
pub struct UdpClient {
    pub socket: Arc<Mutex<Option<Arc<UdpSocket>>>>,
    pub server_addr: Arc<Mutex<Option<SocketAddr>>>,
    pub connected: Arc<std::sync::Mutex<bool>>,
    /// Session id and player id to include in every PlayerInput packet.
    pub session_id: Arc<std::sync::Mutex<Option<uuid::Uuid>>>,
    pub player_id: Arc<std::sync::Mutex<Option<uuid::Uuid>>>,
    /// Most recent snapshot received from the server (shared with Bevy world).
    pub latest_snapshot: Arc<std::sync::Mutex<Option<GameStateSnapshot>>>,
}

impl Default for UdpClient {
    fn default() -> Self {
        Self {
            socket: Arc::new(Mutex::new(None)),
            server_addr: Arc::new(Mutex::new(None)),
            connected: Arc::new(std::sync::Mutex::new(false)),
            session_id: Arc::new(std::sync::Mutex::new(None)),
            player_id: Arc::new(std::sync::Mutex::new(None)),
            latest_snapshot: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

impl UdpClient {
    /// Bind to a local UDP port and store the server address.
    /// Spawns a background reader task that fills `latest_snapshot`.
    pub async fn connect(
        &self,
        server_addr: &str,
        session_id: uuid::Uuid,
        player_id: uuid::Uuid,
    ) -> Result<(), String> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("bind UDP: {e}"))?;
        let addr: SocketAddr = tokio::net::lookup_host(server_addr)
            .await
            .map_err(|e| format!("lookup host {server_addr}: {e}"))?
            .next()
            .ok_or_else(|| format!("no address found for {server_addr}"))?;

        {
            let mut s = self.socket.lock().await;
            *s = Some(Arc::new(socket));
        }
        {
            let mut a = self.server_addr.lock().await;
            *a = Some(addr);
        }
        {
            let mut c = self.connected.lock().unwrap();
            *c = true;
        }
        {
            let mut sid = self.session_id.lock().unwrap();
            *sid = Some(session_id);
        }
        {
            let mut pid = self.player_id.lock().unwrap();
            *pid = Some(player_id);
        }

        // Spawn background reader.
        let this = self.clone();
        tokio::spawn(async move {
            this.background_reader().await;
        });

        Ok(())
    }

    /// Background task: receives snapshots and stores the latest.
    async fn background_reader(&self) {
        loop {
            let snapshot = {
                let guard = self.socket.lock().await;
                let socket = match guard.as_ref() {
                    Some(s) => s.clone(),
                    None => break,
                };
                drop(guard);

                let mut buf = vec![0u8; 65536];
                match socket.recv_from(&mut buf).await {
                    Ok((len, from)) => {
                        trace!("UDP: received {len} bytes from {from}");
                        match serde_json::from_slice::<GameStateSnapshot>(&buf[..len]) {
                            Ok(s) => {
                                trace!("UDP: decoded snapshot tick={} players={}", s.tick, s.players.len());
                                s
                            }
                            Err(e) => {
                                trace!("UDP: failed to decode snapshot: {e}");
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        trace!("UDP: recv error: {e}");
                        break;
                    }
                }
            };

            let mut latest = self.latest_snapshot.lock().unwrap();
            *latest = Some(snapshot);
        }

        let mut c = self.connected.lock().unwrap();
        *c = false;
        trace!("UDP: background reader stopped");
    }

    /// Send a player input packet to the server.
    pub async fn send_input(&self, input: &PlayerInput) -> Result<(), String> {
        let guard = self.socket.lock().await;
        let socket = match guard.as_ref() {
            Some(s) => s.clone(),
            None => return Err("Not connected".into()),
        };
        let addr = {
            let a = self.server_addr.lock().await;
            a.ok_or("No server address")?
        };
        drop(guard);

        let data = serde_json::to_vec(input).map_err(|e| format!("serialize: {e}"))?;
        socket
            .send_to(&data, addr)
            .await
            .map_err(|e| format!("send: {e}"))?;
        Ok(())
    }

    /// Send a shot-fired packet to the server.
    pub async fn send_shot(&self, shot: &noctyrn_shared::protocol::ShotFired) -> Result<(), String> {
        let guard = self.socket.lock().await;
        let socket = match guard.as_ref() {
            Some(s) => s.clone(),
            None => return Err("Not connected".into()),
        };
        let addr = {
            let a = self.server_addr.lock().await;
            a.ok_or("No server address")?
        };
        drop(guard);

        let data = noctyrn_shared::protocol::encode_shot_fired(shot)
            .map_err(|e| format!("serialize: {e}"))?;
        socket
            .send_to(&data, addr)
            .await
            .map_err(|e| format!("send: {e}"))?;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }
}
