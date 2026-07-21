use bevy::prelude::*;
use std::sync::Arc;

pub mod http;
pub mod tcp;
pub mod udp;
pub mod prediction;
pub mod interpolation;

// ---------------------------------------------------------------------------
// Server connection configuration
// ---------------------------------------------------------------------------

/// Deserialization helper matching the [server] table in assets/server.toml.
#[derive(serde::Deserialize)]
struct ServerConfigFile {
    server: ServerConfigFileInner,
}

#[derive(serde::Deserialize)]
struct ServerConfigFileInner {
    http_url: String,
    tcp_addr: String,
    udp_addr: String,
}

/// Server connection addresses.  Loaded from `assets/server.toml` at startup;
/// falls back to localhost if the file is missing or unparseable.
#[derive(Resource, Clone)]
pub struct ServerConfig {
    pub http_url: String,
    pub tcp_addr: String,
    pub udp_addr: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let fallback = Self {
            http_url: "http://127.0.0.1:8080".to_string(),
            tcp_addr: "127.0.0.1:7878".to_string(),
            udp_addr: "127.0.0.1:7877".to_string(),
        };

        let raw = match std::fs::read_to_string("assets/server.toml") {
            Ok(s) => s,
            Err(_) => {
                info!("assets/server.toml not found, using localhost defaults");
                return fallback;
            }
        };

        match toml::from_str::<ServerConfigFile>(&raw) {
            Ok(cfg) => {
                info!(
                    "Loaded server config: http={} tcp={} udp={}",
                    cfg.server.http_url, cfg.server.tcp_addr, cfg.server.udp_addr
                );
                Self {
                    http_url: cfg.server.http_url,
                    tcp_addr: cfg.server.tcp_addr,
                    udp_addr: cfg.server.udp_addr,
                }
            }
            Err(e) => {
                warn!("Failed to parse assets/server.toml: {e} — using localhost defaults");
                fallback
            }
        }
    }
}

/// Tracks the player's connection/auth state with the server.
#[derive(Resource, Default, Debug, Clone)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected {
        token: String,
        user_id: uuid::Uuid,
        username: String,
    },
}

impl ConnectionState {
    pub fn is_connected(&self) -> bool {
        matches!(self, ConnectionState::Connected { .. })
    }

    pub fn token(&self) -> Option<&str> {
        match self {
            ConnectionState::Connected { token, .. } => Some(token),
            _ => None,
        }
    }

    pub fn username(&self) -> Option<&str> {
        match self {
            ConnectionState::Connected { username, .. } => Some(username),
            _ => None,
        }
    }

    pub fn user_id(&self) -> Option<uuid::Uuid> {
        match self {
            ConnectionState::Connected { user_id, .. } => Some(*user_id),
            _ => None,
        }
    }
}

/// Resource to toggle between local single-player and networked multiplayer.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub enum MultiplayerMode {
    #[default]
    Local,
    Online,
}

/// Cached player profile from server.
#[derive(Resource, Default, Clone, Debug)]
pub struct CachedProfile {
    pub loaded: bool,
    pub profile: Option<noctyrn_shared::player::PlayerProfile>,
}

/// Cached friends list from server.
#[derive(Resource, Default, Clone, Debug)]
pub struct CachedFriends {
    pub loaded: bool,
    pub friends: Vec<noctyrn_shared::player::FriendEntry>,
    pub incoming_requests: Vec<noctyrn_shared::player::FriendRequestInfo>,
    pub outgoing_requests: Vec<noctyrn_shared::player::FriendRequestInfo>,
}

/// Tracks the player's party state.
#[derive(Resource, Default, Clone, Debug)]
pub struct PartyState {
    pub party: Option<noctyrn_shared::lobby::PartyInfo>,
    pub pending_invite: Option<(uuid::Uuid, String)>, // (party_id, from_username)
}

/// Tracks the TCP connection state.
#[derive(Resource, Default, Clone, Debug)]
pub struct TcpConnection {
    pub connected: bool,
    pub authenticated: bool,
}

/// Events for network responses arriving asynchronously.
#[derive(Message, Debug)]
pub enum NetworkEvent {
    // Auth
    LoginSuccess { token: String, user_id: uuid::Uuid, username: String },
    LoginError { message: String },
    RegisterSuccess { token: String, user_id: uuid::Uuid, username: String },
    RegisterError { message: String },
    // Profile
    ProfileLoaded { profile: noctyrn_shared::player::PlayerProfile },
    ProfileError { message: String },
    // Friends
    FriendsLoaded { friends: Vec<noctyrn_shared::player::FriendEntry> },
    FriendRequestsLoaded {
        incoming: Vec<noctyrn_shared::player::FriendRequestInfo>,
        outgoing: Vec<noctyrn_shared::player::FriendRequestInfo>,
    },
    FriendRequestSent,
    FriendRequestAccepted,
    FriendRequestDeclined,
    FriendRemoved,
    FriendError { message: String },
    // Party
    PartyInviteReceived { party_id: uuid::Uuid, from_username: String },
    PartyUpdate { party: noctyrn_shared::lobby::PartyInfo },
    PartyError { message: String },
    // Matchmaking
    MatchmakingUpdate { players_in_queue: u32 },
    MatchFound { lobby_id: uuid::Uuid, server_addr: String, udp_port: u16 },
    // Lobby
    LobbyUpdate { lobby: noctyrn_shared::lobby::LobbyState },
    // TCP connection
    TcpAuthenticated,
    TcpDisconnected,
    // Errors
    ConnectionError { message: String },
}

/// Shared handle for the tokio runtime so Bevy systems can spawn async tasks.
#[derive(Resource)]
pub struct TokioRuntime(pub Arc<tokio::runtime::Runtime>);

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        // Create a tokio runtime for async networking
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        app.insert_resource(TokioRuntime(Arc::new(rt)));
        app.init_resource::<ServerConfig>();
        app.init_resource::<ConnectionState>();
        app.init_resource::<MultiplayerMode>();
        app.init_resource::<CachedProfile>();
        app.init_resource::<CachedFriends>();
        app.init_resource::<PartyState>();
        app.init_resource::<TcpConnection>();
        app.init_resource::<http::PendingRequests>();
        app.init_resource::<tcp::TcpClient>();
        app.add_message::<NetworkEvent>();

        // Process incoming network events and poll pending HTTP requests
        app.add_systems(Update, (handle_network_events, http::poll_pending_requests));
    }
}

/// System that processes NetworkEvent and updates resources accordingly.
fn handle_network_events(
    mut events: MessageReader<NetworkEvent>,
    mut connection: ResMut<ConnectionState>,
    mut cached_profile: ResMut<CachedProfile>,
    mut cached_friends: ResMut<CachedFriends>,
    mut party_state: ResMut<PartyState>,
    mut tcp: ResMut<TcpConnection>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::LoginSuccess { token, user_id, username } |
            NetworkEvent::RegisterSuccess { token, user_id, username } => {
                *connection = ConnectionState::Connected {
                    token: token.clone(),
                    user_id: *user_id,
                    username: username.clone(),
                };
            }
            NetworkEvent::ProfileLoaded { profile } => {
                cached_profile.loaded = true;
                cached_profile.profile = Some(profile.clone());
            }
            NetworkEvent::FriendsLoaded { friends } => {
                cached_friends.loaded = true;
                cached_friends.friends = friends.clone();
            }
            NetworkEvent::FriendRequestsLoaded { incoming, outgoing } => {
                cached_friends.incoming_requests = incoming.clone();
                cached_friends.outgoing_requests = outgoing.clone();
            }
            // Party
            NetworkEvent::PartyInviteReceived { party_id, from_username } => {
                party_state.pending_invite = Some((*party_id, from_username.clone()));
            }
            NetworkEvent::PartyUpdate { party } => {
                party_state.party = Some(party.clone());
                // Being in a party means any pending invite was resolved.
                party_state.pending_invite = None;
            }
            NetworkEvent::PartyError { message } => {
                warn!("Party error: {message}");
                // Clear pending invite on error (rejected, user not found, etc.)
                party_state.pending_invite = None;
            }
            // TCP
            NetworkEvent::TcpAuthenticated => {
                tcp.connected = true;
                tcp.authenticated = true;
            }
            NetworkEvent::TcpDisconnected => {
                tcp.connected = false;
                tcp.authenticated = false;
            }
            _ => {}
        }
    }
}
