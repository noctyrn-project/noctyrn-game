use std::sync::Arc;

use bevy::prelude::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use super::http::PendingRequests;
use super::{NetworkEvent, ServerConfig, TokioRuntime};

/// Thread-safe wrapper around the TCP connection.
#[derive(Resource, Clone)]
pub struct TcpClient {
    pub stream: Arc<Mutex<Option<tokio::net::TcpStream>>>,
    pub connected: Arc<std::sync::Mutex<bool>>,
}

impl Default for TcpClient {
    fn default() -> Self {
        Self {
            stream: Arc::new(Mutex::new(None)),
            connected: Arc::new(std::sync::Mutex::new(false)),
        }
    }
}

impl TcpClient {
    /// Connect to the TCP endpoint and perform auth handshake.
    /// On success, spawns a background reader task.
    pub async fn connect_and_auth(
        &self,
        addr: &str,
        token: &str,
        rt: &TokioRuntime,
        pending: &PendingRequests,
    ) -> Result<(), String> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| format!("TCP connect failed: {e}"))?;

        // Send Authenticate message.
        let auth_msg = noctyrn_shared::protocol::ClientMessage::Authenticate {
            token: token.to_string(),
        };
        let data = serde_json::to_vec(&auth_msg).map_err(|e| format!("serialize: {e}"))?;
        let len = (data.len() as u32).to_be_bytes();

        let mut write_stream = stream;
        write_stream
            .write_all(&len)
            .await
            .map_err(|e| format!("write auth: {e}"))?;
        write_stream
            .write_all(&data)
            .await
            .map_err(|e| format!("write auth data: {e}"))?;
        write_stream
            .flush()
            .await
            .map_err(|e| format!("flush: {e}"))?;

        // Read the Authenticated response.
        let mut len_buf = [0u8; 4];
        write_stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|_| "no auth response".to_string())?;
        let msg_len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; msg_len];
        write_stream
            .read_exact(&mut payload)
            .await
            .map_err(|_| "no auth payload".to_string())?;
        let resp: noctyrn_shared::protocol::ServerMessage =
            serde_json::from_slice(&payload).map_err(|e| format!("bad auth response: {e}"))?;

        match resp {
            noctyrn_shared::protocol::ServerMessage::Authenticated { .. } => {
                // Success!
            }
            _ => {
                return Err("Unexpected auth response".into());
            }
        }

        // Store stream and mark connected.
        {
            let mut s = self.stream.lock().await;
            *s = Some(write_stream);
        }
        {
            let mut c = self.connected.lock().unwrap();
            *c = true;
        }

        // Emit TcpAuthenticated event.
        let results = pending.results.clone();
        results.lock().unwrap().push(NetworkEvent::TcpAuthenticated);

        // Spawn background reader.
        let this = self.clone();
        let pending_clone = pending.clone();
        rt.0.spawn(async move {
            this.background_reader(pending_clone).await;
        });

        Ok(())
    }

    /// Background task: continuously read messages from the TCP stream
    /// and push them into PendingRequests.
    async fn background_reader(&self, pending: PendingRequests) {
        loop {
            let msg = {
                let mut guard = self.stream.lock().await;
                let stream = match guard.as_mut() {
                    Some(s) => s,
                    None => break,
                };

                let mut len_buf = [0u8; 4];
                if stream.read_exact(&mut len_buf).await.is_err() {
                    break;
                }
                let msg_len = u32::from_be_bytes(len_buf) as usize;
                if msg_len > 1_048_576 {
                    break;
                }
                let mut payload = vec![0u8; msg_len];
                if stream.read_exact(&mut payload).await.is_err() {
                    break;
                }

                serde_json::from_slice::<noctyrn_shared::protocol::ServerMessage>(&payload).ok()
            };

            match msg {
                Some(server_msg) => {
                    let event = tcp_message_to_event(server_msg);
                    pending.results.lock().unwrap().push(event);
                }
                None => break,
            }
        }

        // Connection lost.
        {
            let mut c = self.connected.lock().unwrap();
            *c = false;
        }
        pending
            .results
            .lock()
            .unwrap()
            .push(NetworkEvent::TcpDisconnected);
    }

    /// Send a ClientMessage over the TCP connection.
    pub async fn send(&self, msg: &noctyrn_shared::protocol::ClientMessage) -> Result<(), String> {
        let mut guard = self.stream.lock().await;
        let stream = guard.as_mut().ok_or("Not connected")?;
        let data = serde_json::to_vec(msg).map_err(|e| format!("serialize: {e}"))?;
        let len = (data.len() as u32).to_be_bytes();
        stream.write_all(&len).await.map_err(|e| format!("write: {e}"))?;
        stream
            .write_all(&data)
            .await
            .map_err(|e| format!("write: {e}"))?;
        stream.flush().await.map_err(|e| format!("flush: {e}"))?;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }
}

/// Convert a ServerMessage to a NetworkEvent for the Bevy event bus.
fn tcp_message_to_event(msg: noctyrn_shared::protocol::ServerMessage) -> NetworkEvent {
    use noctyrn_shared::protocol::ServerMessage;
    match msg {
        ServerMessage::PartyInviteReceived {
            party_id,
            from_username,
        } => NetworkEvent::PartyInviteReceived {
            party_id,
            from_username,
        },
        ServerMessage::PartyUpdate { party } => NetworkEvent::PartyUpdate { party },
        ServerMessage::PartyError { message } => NetworkEvent::PartyError { message },
        ServerMessage::LobbyUpdate { lobby } => NetworkEvent::LobbyUpdate { lobby },
        ServerMessage::MatchmakingStatus {
            players_in_queue, ..
        } => NetworkEvent::MatchmakingUpdate { players_in_queue },
        ServerMessage::MatchFound {
            lobby_id,
            server_addr,
            udp_port,
        } => NetworkEvent::MatchFound {
            lobby_id,
            server_addr,
            udp_port,
        },
        ServerMessage::LobbyError { message } | ServerMessage::AuthError { message } | ServerMessage::Error { message } => {
            NetworkEvent::ConnectionError { message }
        }
        _ => NetworkEvent::ConnectionError {
            message: format!("Unhandled server message: {:?}", std::mem::discriminant(&msg)),
        },
    }
}
