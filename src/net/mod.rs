use bevy::prelude::*;
use std::sync::Arc;
use crate::player::GameState;

pub mod http;
pub mod tcp;
pub mod udp;
pub mod prediction;
pub mod interpolation;

// ---------------------------------------------------------------------------
// Server connection configuration
// ---------------------------------------------------------------------------

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

#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub enum MultiplayerMode {
    #[default]
    Local,
    Online,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct CachedProfile {
    pub loaded: bool,
    pub profile: Option<noctyrn_shared::player::PlayerProfile>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct CachedFriends {
    pub loaded: bool,
    pub friends: Vec<noctyrn_shared::player::FriendEntry>,
    pub incoming_requests: Vec<noctyrn_shared::player::FriendRequestInfo>,
    pub outgoing_requests: Vec<noctyrn_shared::player::FriendRequestInfo>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct LobbyPlayers {
    pub players: Vec<noctyrn_shared::lobby::LobbyPlayer>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct ScoreboardData {
    pub kills: std::collections::HashMap<uuid::Uuid, u32>,
    pub deaths: std::collections::HashMap<uuid::Uuid, u32>,
    pub scores: std::collections::HashMap<uuid::Uuid, i32>,
    pub names: std::collections::HashMap<uuid::Uuid, String>,
}

impl ScoreboardData {
    pub fn get_or_name(&self, id: &uuid::Uuid) -> String {
        self.names.get(id).cloned().unwrap_or_else(|| format!("Player {}", &id.to_string()[..8]))
    }
}

#[derive(Resource, Default, Clone, Debug)]
pub struct PartyState {
    pub party: Option<noctyrn_shared::lobby::PartyInfo>,
    pub pending_invite: Option<(uuid::Uuid, String)>,
}

#[derive(Resource, Default, Clone, Debug)]
pub struct TcpConnection {
    pub connected: bool,
    pub authenticated: bool,
}

#[derive(Message, Debug)]
pub enum NetworkEvent {
    LoginSuccess { token: String, user_id: uuid::Uuid, username: String },
    LoginError { message: String },
    RegisterSuccess { token: String, user_id: uuid::Uuid, username: String },
    RegisterError { message: String },
    ProfileLoaded { profile: noctyrn_shared::player::PlayerProfile },
    ProfileError { message: String },
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
    PartyInviteReceived { party_id: uuid::Uuid, from_username: String },
    PartyUpdate { party: noctyrn_shared::lobby::PartyInfo },
    PartyError { message: String },
    MatchmakingUpdate { players_in_queue: u32 },
    MatchFound { lobby_id: uuid::Uuid, server_addr: String, udp_port: u16 },
    LobbyUpdate { lobby: noctyrn_shared::lobby::LobbyState },
    TcpAuthenticated,
    TcpDisconnected,
    ConnectionError { message: String },
    ChatReceived { from_username: String, content: String },
}

#[derive(Resource)]
pub struct TokioRuntime(pub Arc<tokio::runtime::Runtime>);

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
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
        app.init_resource::<LobbyPlayers>();
        app.init_resource::<ScoreboardData>();
        app.init_resource::<PartyState>();
        app.init_resource::<TcpConnection>();
        app.init_resource::<http::PendingRequests>();
        app.init_resource::<tcp::TcpClient>();
        app.init_resource::<udp::UdpClient>();
        app.add_message::<NetworkEvent>();

        app.add_systems(Update, (handle_network_events, http::poll_pending_requests));
        app.add_systems(Update, process_snapshots.run_if(in_state(GameState::Playing)));
        app.add_systems(Update, cleanup_muzzle_flashes.run_if(in_state(GameState::Playing)));
    }
}

fn handle_network_events(
    mut events: MessageReader<NetworkEvent>,
    mut connection: ResMut<ConnectionState>,
    mut cached_profile: ResMut<CachedProfile>,
    mut cached_friends: ResMut<CachedFriends>,
    mut party_state: ResMut<PartyState>,
    mut tcp: ResMut<TcpConnection>,
    mut lobby_players: ResMut<LobbyPlayers>,
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
            NetworkEvent::PartyInviteReceived { party_id, from_username } => {
                party_state.pending_invite = Some((*party_id, from_username.clone()));
            }
            NetworkEvent::PartyUpdate { party } => {
                party_state.party = Some(party.clone());
                party_state.pending_invite = None;
            }
            NetworkEvent::PartyError { message } => {
                warn!("Party error: {message}");
                party_state.pending_invite = None;
            }
            NetworkEvent::LobbyUpdate { lobby } => {
                info!("LobbyUpdate: {} players in lobby", lobby.players.len());
                for p in &lobby.players {
                    info!("  Lobby player: {} ({}) ready={}", p.username, p.id, p.ready);
                }
                lobby_players.players = lobby.players.clone();
            }
            NetworkEvent::TcpAuthenticated => {
                tcp.connected = true;
                tcp.authenticated = true;
                info!("TCP authenticated");
            }
            NetworkEvent::TcpDisconnected => {
                tcp.connected = false;
                tcp.authenticated = false;
                info!("TCP disconnected");
            }
            NetworkEvent::ChatReceived { from_username, content } => {
                info!("CHAT [{}]: {}", from_username, content);
            }
            _ => {}
        }
    }
}

fn process_snapshots(
    udp: Res<udp::UdpClient>,
    mut commands: Commands,
    mut remote_query: Query<(Entity, &mut crate::player::RemotePlayer, &mut Transform)>,
    mut local_query: Query<(Entity, &mut Transform), (With<crate::player::LocalPlayer>, Without<crate::player::RemotePlayer>)>,
    mut scoreboard: ResMut<ScoreboardData>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    registry: Res<crate::weapons::WeaponRegistry>,
) {
    let snapshot = {
        let mut guard = udp.latest_snapshot.lock().unwrap();
        guard.take()
    };
    let Some(ref snapshot) = snapshot else {
        return;
    };

    let local_player_id = *udp.player_id.lock().unwrap();

    // Sync local player to server's authoritative position
    if let Some(lid) = local_player_id {
        if let Some(server_pos) = snapshot.players.iter().find(|p| p.id == lid) {
            if let Ok((_entity, mut local_transform)) = local_query.single_mut() {
                let target = Vec3::new(server_pos.position[0], server_pos.position[1] + 1.5, server_pos.position[2]);
                if local_transform.translation.distance(target) > 0.2 {
                    local_transform.translation = target;
                }
            }
        }
    }

    for p in &snapshot.players {
        scoreboard.names.entry(p.id).or_insert_with(|| p.username.clone());
    }

    for event in &snapshot.events {
        match event {
            noctyrn_shared::protocol::GameEvent::PlayerKilled { killer_id, victim_id, weapon } => {
                info!("KILL: {} killed {} with {}", scoreboard.get_or_name(killer_id), scoreboard.get_or_name(victim_id), weapon);
                *scoreboard.kills.entry(*killer_id).or_insert(0) += 1;
                *scoreboard.deaths.entry(*victim_id).or_insert(0) += 1;
            }
            noctyrn_shared::protocol::GameEvent::ProjectileFired { owner_id, .. } => {
                if let Some(lid) = local_player_id { if *owner_id == lid { continue; } }
                for (entity, rp, _) in remote_query.iter() {
                    if rp.server_id == *owner_id {
                        let flash = commands.spawn((
                            Mesh3d(meshes.add(Sphere::new(0.12))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(1.0, 0.9, 0.3),
                                emissive: LinearRgba::rgb(3.0, 2.0, 0.5),
                                ..default()
                            })),
                            Transform::from_xyz(0.5, 0.8, 0.0),
                            MuzzleFlash { lifetime: 0.12 },
                        )).id();
                        commands.entity(entity).add_child(flash);
                        break;
                    }
                }
            }
            noctyrn_shared::protocol::GameEvent::MatchStateUpdate { scores, .. } => {
                for (player_id, player_score) in scores {
                    scoreboard.scores.insert(*player_id, *player_score);
                }
            }
            _ => {}
        }
    }

    let known_ids: std::collections::HashSet<uuid::Uuid> =
        remote_query.iter().map(|(_, rp, _)| rp.server_id).collect();

    // Update existing remote players: position, rotation, despawn if gone
    for (entity, rp, mut transform) in remote_query.iter_mut() {
        if let Some(p) = snapshot.players.iter().find(|p| p.id == rp.server_id) {
            let target = Vec3::new(p.position[0], p.position[1], p.position[2]);
            transform.translation = transform.translation.lerp(target, 0.3);
            // Apply yaw rotation (around Y axis)
            transform.rotation = Quat::from_rotation_y(p.yaw);
        } else {
            commands.entity(entity).despawn();
        }
    }

    let pill_mesh = meshes.add(Capsule3d::new(0.3, 0.6));
    let pill_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.2, 0.35), ..default()
    });
    let bar_bg_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.1, 0.1, 0.1), ..default() });
    let bar_fill_mat = materials.add(StandardMaterial { base_color: Color::srgb(0.2, 0.8, 0.2), ..default() });
    let bar_mesh = meshes.add(Rectangle::new(0.8, 0.08));

    for p in &snapshot.players {
        if let Some(lid) = local_player_id { if p.id == lid { continue; } }
        if known_ids.contains(&p.id) { continue; }

        info!("Spawning remote player {} ({}) health={:.0}", p.username, p.id, p.health);

        let remote = commands.spawn((
            crate::player::RemotePlayer { server_id: p.id },
            Transform::from_xyz(p.position[0], p.position[1], p.position[2])
                .with_rotation(Quat::from_rotation_y(p.yaw)),
            Visibility::default(),
            crate::gameplay::PlayerBody,
        )).id();

        commands.entity(remote).with_children(|parent| {
            parent.spawn((
                Mesh3d(pill_mesh.clone()),
                MeshMaterial3d(pill_mat.clone()),
                Transform::from_xyz(0.0, 0.9, 0.0),
            ));

            // Username text with Billboard so it always faces the camera
            parent.spawn((
                Text2d::new(p.username.clone()),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::WHITE),
                Transform::from_translation(Vec3::new(0.0, 2.3, 0.0)),
                crate::gameplay::Billboard,
            ));

            // Health bar (Billboard already applied)
            parent.spawn((
                Mesh3d(bar_mesh.clone()),
                MeshMaterial3d(bar_bg_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, 1.9, 0.0)),
                crate::gameplay::Billboard,
            ));
            parent.spawn((
                Mesh3d(bar_mesh.clone()),
                MeshMaterial3d(bar_fill_mat.clone()),
                Transform::from_translation(Vec3::new(0.0, 1.9, 0.01)),
                crate::gameplay::Billboard,
            ));

            // Weapon model at right hip, reasonable third-person scale
            let weapon_key = if registry.weapons.contains_key(&p.weapon_id) { &p.weapon_id } else { "colt_m4a1" };
            if let Some(config) = registry.weapons.get(weapon_key) {
                let mf = config.meta.model_path.split('#').next().unwrap_or("");
                if !mf.is_empty() && std::path::Path::new(&format!("assets/{mf}")).exists() {
                    parent.spawn((
                        SceneRoot(asset_server.load(&config.meta.model_path)),
                        Transform::from_xyz(0.6, 0.3, 0.0)
                            .with_rotation(Quat::from_rotation_y(-0.5))
                            .with_scale(Vec3::splat(0.35)),
                    ));
                }
            }
        });
    }

    for (entity, rp, _) in remote_query.iter_mut() {
        if !snapshot.players.iter().any(|p| p.id == rp.server_id) {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
pub struct MuzzleFlash { pub lifetime: f32 }

pub fn cleanup_muzzle_flashes(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut MuzzleFlash)>,
) {
    for (entity, mut flash) in query.iter_mut() {
        flash.lifetime -= time.delta_secs();
        if flash.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
