// UDP client for real-time gameplay synchronization.
// Handles sending player inputs and receiving game state snapshots.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

/// Holds the UDP socket for game traffic.
pub struct UdpClient {
    pub socket: Option<Arc<UdpSocket>>,
    pub server_addr: Option<SocketAddr>,
    pub connected: bool,
}

impl Default for UdpClient {
    fn default() -> Self {
        Self {
            socket: None,
            server_addr: None,
            connected: false,
        }
    }
}

impl UdpClient {
    /// Bind to a local UDP port and set the server address.
    pub async fn connect(server_addr: &str) -> Result<Self, std::io::Error> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        let addr: SocketAddr = server_addr.parse().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Invalid address: {}", e))
        })?;

        Ok(Self {
            socket: Some(Arc::new(socket)),
            server_addr: Some(addr),
            connected: true,
        })
    }

    /// Send a player input packet to the server.
    pub async fn send_input(&self, input: &noctyrn_shared::protocol::PlayerInput) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let (Some(socket), Some(addr)) = (&self.socket, &self.server_addr) {
            let data = serde_json::to_vec(input)?;
            socket.send_to(&data, addr).await?;
            Ok(())
        } else {
            Err("Not connected".into())
        }
    }

    /// Receive a game state snapshot from the server.
    pub async fn recv_snapshot(&self) -> Result<noctyrn_shared::protocol::GameStateSnapshot, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(socket) = &self.socket {
            let mut buf = vec![0u8; 65536]; // Max UDP packet size
            let (len, _addr) = socket.recv_from(&mut buf).await?;
            let snapshot: noctyrn_shared::protocol::GameStateSnapshot = serde_json::from_slice(&buf[..len])?;
            Ok(snapshot)
        } else {
            Err("Not connected".into())
        }
    }

    pub fn disconnect(&mut self) {
        self.socket = None;
        self.server_addr = None;
        self.connected = false;
    }
}
