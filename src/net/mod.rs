use bevy::prelude::*;
use std::sync::Arc;
use rand::Rng;
use crate::player::GameState;
use crate::gameplay::Health;
use crate::player::{PhysicalTranslation, PreviousPhysicalTranslation, Velocity};

pub mod http;
pub mod tcp;
pub mod udp;
pub mod prediction;


#[derive(Component)]
pub struct RemoteHealthBar {
    pub server_id: uuid::Uuid,
}

#[derive(Component)]
pub struct RemoteUsername {
    pub server_id: uuid::Uuid,
}

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

        let raw = match std::fs::read_to_string("assets/server.json") {
            Ok(s) => s,
            Err(_) => {
                info!("assets/server.json not found, using localhost defaults");
                return fallback;
            }
        };

        match serde_json::from_str::<ServerConfigFile>(&raw) {
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
                warn!("Failed to parse assets/server.json: {e} — using localhost defaults");
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
    MatchFound { lobby_id: uuid::Uuid, server_addr: String, udp_port: u16, map_id: String },
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
        app.init_resource::<prediction::PredictionBuffer>();
        app.add_message::<NetworkEvent>();

        app.add_systems(Update, (handle_network_events, http::poll_pending_requests));
        app.add_systems(Update, process_snapshots.run_if(in_state(GameState::Playing)));
        app.add_systems(Update, assign_spectator_target.after(process_snapshots).run_if(in_state(GameState::Playing)));
        app.add_systems(OnEnter(GameState::Playing), send_loadout);
        app.add_systems(Update, reconcile_prediction.run_if(in_state(GameState::Playing)));
        app.add_systems(Update, cleanup_muzzle_flashes.run_if(in_state(GameState::Playing)));
        app.add_systems(Update, sync_remote_ui.after(process_snapshots).run_if(in_state(GameState::Playing)));
        app.add_systems(Update, update_remote_weapons.after(sync_remote_ui).run_if(in_state(GameState::Playing)));
        app.add_systems(Update, update_3d_damage_numbers.after(process_snapshots).run_if(in_state(GameState::Playing)));
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
    mut remote_query: Query<(Entity, &mut crate::player::RemotePlayer, &mut Transform, &mut Visibility)>,
    mut local_query: Query<
        (Entity, &mut Transform, &mut PhysicalTranslation, &mut PreviousPhysicalTranslation, &mut Velocity, &mut Health),
        (With<crate::player::LocalPlayer>, Without<crate::player::RemotePlayer>),
    >,
    bar_ui_query: Query<(Entity, &RemoteHealthBar)>,
    name_ui_query: Query<(Entity, &RemoteUsername)>,
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

    // Sync local player to server's authoritative position.
    // Server sends feet-level positions; client stores feet-level in physics
    // and adds eye offset only for the camera transform.
    if let Some(lid) = local_player_id {
        if let Some(server_pos) = snapshot.players.iter().find(|p| p.id == lid) {
            if let Ok((_entity, mut local_transform, mut phys, mut prev_phys, mut velocity, mut health)) = local_query.single_mut() {
                let server_feet = Vec3::new(
                    server_pos.position[0],
                    server_pos.position[1],
                    server_pos.position[2],
                );
                let server_vel = Vec3::new(
                    server_pos.velocity[0],
                    server_pos.velocity[1],
                    server_pos.velocity[2],
                );

                // Update physics with exponential smoothing.
                // The prediction-buffer reconciliation (reconcile_prediction)
                // handles the rewind+replay. Here we just smooth the render
                // transform directly from the snapshot as a backup.
                let current_feet = phys.0;
                let diff = current_feet.distance(server_feet);

                if diff > 2.0 {
                    // Large error — hard snap.
                    phys.0 = server_feet;
                    prev_phys.0 = server_feet;
                    velocity.0 = server_vel;
                } else if diff > 0.1 {
                    // Small error — smooth toward server.
                    phys.0 = current_feet.lerp(server_feet, 0.15);
                    velocity.0 = server_vel;
                }
                // else: trust local prediction.

                // Render/camera transform uses eye height (feet + 1.7).
                let eye_target = phys.0 + Vec3::Y * 1.7;
                if local_transform.translation.distance(eye_target) > 2.0 {
                    local_transform.translation = eye_target;
                } else if local_transform.translation.distance(eye_target) > 0.1 {
                    local_transform.translation = local_transform.translation.lerp(eye_target, 0.3);
                }

                // Sync authoritative health from server.
                health.current = server_pos.health;
            }
        }
    }

    for p in &snapshot.players {
        scoreboard.names.entry(p.id).or_insert_with(|| p.username.clone());
    }

    for event in &snapshot.events {
        match event {
            noctyrn_shared::protocol::GameEvent::PlayerKilled { killer_id, victim_id, weapon } => {
                let killer_name = scoreboard.get_or_name(killer_id);
                let victim_name = scoreboard.get_or_name(victim_id);
                info!("KILL: {killer_name} killed {victim_name} with {weapon}");
                *scoreboard.kills.entry(*killer_id).or_insert(0) += 1;
                *scoreboard.deaths.entry(*victim_id).or_insert(0) += 1;

                // Set KillerInfo if the local player was killed.
                if let Some(lid) = local_player_id {
                    if *victim_id == lid {
                        commands.insert_resource(crate::gameplay::KillerInfo { name: killer_name.clone(), server_id: Some(*killer_id) });
                    }
                    // Show Kill +100 notification if local player got the kill.
                    if *killer_id == lid {
                        let score_text = format!("KILL +100");
                        // Use the kill feed: push a DeathEvent so the kill-feed UI shows it.
                        commands.spawn((
                            Text2d::new(score_text),
                            TextFont { font_size: FontSize::Px(42.0), ..default() },
                            TextColor(Color::srgb(1.0, 1.0, 0.3)),
                            Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                            crate::gameplay::Billboard,
                            crate::player::DamageNumber {
                                timer: Timer::from_seconds(2.0, TimerMode::Once),
                                velocity: Vec3::new(0.0, 1.5, 0.0),
                            },
                        ));
                    }
                }
            }
            noctyrn_shared::protocol::GameEvent::ProjectileFired { owner_id, origin, direction, weapon, seed, pellet_count } => {
                if let Some(lid) = local_player_id { if *owner_id == lid { continue; } }
                let speed = registry.weapons.get(weapon)
                    .and_then(|w| w.attachments.ammo.as_ref().map(|a| a.velocity))
                    .unwrap_or(600.0);
                let origin_v = Vec3::new(origin[0], origin[1], origin[2]);
                let show_trail = speed > 200.0;
                let pellets = (*pellet_count).max(1) as u32;
                let spread_rad = registry.weapons.get(weapon)
                    .and_then(|w| Some(w.attributes.spread_cone))
                    .unwrap_or(0.0)
                    .to_radians();

                for i in 0..pellets {
                    let dir = crate::player::shooting::apply_spread_seeded(
                        &direction, spread_rad, *seed, i,
                    );
                    let dir_v = Vec3::new(dir[0], dir[1], dir[2]);
                    if show_trail {
                        let dir_norm = dir_v.normalize_or_zero();
                        let rot = if dir_norm.length_squared() > 0.001 {
                            Quat::from_rotation_arc(Vec3::Z, dir_norm)
                        } else {
                            Quat::IDENTITY
                        };
                        let _beam = commands.spawn((
                            Mesh3d(meshes.add(Cuboid::new(0.02, 0.02, 2.5))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(1.0, 0.85, 0.2),
                                emissive: LinearRgba::rgb(5.0, 2.5, 0.4),
                                ..default()
                            })),
                            Transform::from_translation(origin_v)
                                .with_rotation(rot),
                            Visibility::default(),
                            crate::player::shooting::Projectile {
                                velocity: dir_v * speed,
                                prev_pos: origin_v,
                                timer: Timer::from_seconds(
                                    (140.0 / speed.max(1.0)).min(3.0),
                                    TimerMode::Once,
                                ),
                                damage: 0.0,
                                from_player: false,
                                source_name: String::new(),
                            },
                        )).with_children(|b| {
                            b.spawn((
                                PointLight {
                                    color: Color::srgb(1.0, 0.8, 0.2),
                                    intensity: 600.0,
                                    range: 3.0,
                                    shadow_maps_enabled: false,
                                    ..default()
                                },
                            ));
                        });
                    } else {
                        let origin_entity = commands.spawn((
                            Mesh3d(meshes.add(Sphere::new(0.12))),
                            MeshMaterial3d(materials.add(StandardMaterial {
                                base_color: Color::srgb(1.0, 0.9, 0.3),
                                emissive: LinearRgba::rgb(3.0, 2.0, 0.5),
                                ..default()
                            })),
                            Transform::from_translation(origin_v),
                            MuzzleFlash { lifetime: 0.12 },
                        )).id();
                        for (entity, rp, _, _) in remote_query.iter() {
                            if rp.server_id == *owner_id {
                                commands.entity(entity).add_child(origin_entity);
                                break;
                            }
                        }
                    }
                }
            }
            noctyrn_shared::protocol::GameEvent::PlayerRespawned { player_id, position } => {
                info!("Player {player_id} respawned at ({:.1},{:.1},{:.1})", position[0], position[1], position[2]);
                if let Some(lid) = local_player_id {
                    if *player_id == lid {
                        commands.remove_resource::<crate::gameplay::KillerInfo>();
                        commands.remove_resource::<crate::gameplay::RespawnTimer>();
                    }
                }
            }
            noctyrn_shared::protocol::GameEvent::PlayerDamaged { target_id, damage, source_id } => {
                if let Some(lid) = local_player_id {
                    if *target_id == lid {
                        info!("YOU took {damage} damage from {source_id}");
                    } else if *source_id == lid {
                        // We damaged someone — show floating damage number at their position.
                        if let Some(p) = snapshot.players.iter().find(|p| p.id == *target_id) {
                            let pos = Vec3::new(p.position[0], p.position[1] + 1.5, p.position[2]);
                            let color = if *damage >= 50.0 {
                                Color::srgb(1.0, 0.2, 0.2)
                            } else {
                                Color::srgb(1.0, 1.0, 0.3)
                            };
                            let mut rng = rand::rng();
                            commands.spawn((
                                Text2d::new(format!("{:.0}", damage)),
                                TextFont { font_size: FontSize::Px(36.0), ..default() },
                                TextColor(color),
                                Transform::from_translation(pos),
                                crate::gameplay::Billboard,
                                crate::player::DamageNumber {
                                    timer: Timer::from_seconds(1.0, TimerMode::Once),
                                    velocity: Vec3::new(
                                        rng.random_range(-0.5..0.5),
                                        rng.random_range(1.0..2.0),
                                        rng.random_range(-0.5..0.5),
                                    ),
                                },
                            ));
                        }
                    }
                }
            }
            noctyrn_shared::protocol::GameEvent::MatchStateUpdate { scores, .. } => {
                for (player_id, player_score) in scores {
                    scoreboard.scores.insert(*player_id, *player_score);
                }
            }
            noctyrn_shared::protocol::GameEvent::MatchOver { winner_id, scores } => {
                info!("Match over! Winner={winner_id:?} scores={scores:?}");
                // The existing check_match_over system in gameplay.rs will
                // detect the match end via MatchState or the MatchOverScreen.
                // We set a resource to signal game-over to that system.
                commands.insert_resource(crate::gameplay::MatchOverFromServer { winner: *winner_id });
            }
            noctyrn_shared::protocol::GameEvent::GrenadeExploded { owner_id, position, weapon, damage, radius: _radius } => {
                info!("Grenade exploded at ({:.1},{:.1},{:.1}) from {owner_id} ({weapon}, dmg={damage})", position[0], position[1], position[2]);
                let center = Vec3::new(position[0], position[1], position[2]);
                let mut rng = rand::rng();
                for _ in 0..25 {
                    let dir = Vec3::new(
                        rng.random_range(-1.0..1.0),
                        rng.random_range(-1.0..1.0),
                        rng.random_range(-1.0..1.0),
                    ).normalize_or_zero();
                    let speed = rng.random_range(2.0..8.0);
                    let life = rng.random_range(1.0..2.5);
                    let s = rng.random_range(0.5..1.5);
                    let sx = rng.random_range(0.4..1.6);
                    let sy = rng.random_range(0.4..1.6);
                    let sz = rng.random_range(0.4..1.6);
                    commands.spawn((
                        Mesh3d(meshes.add(Cuboid::new(sx, sy, sz))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgba(1.0, 1.0, 0.8, 0.8),
                            alpha_mode: AlphaMode::Blend,
                            unlit: true,
                            ..default()
                        })),
                        Transform::from_translation(center)
                            .with_scale(Vec3::splat(0.1))
                            .with_rotation(Quat::from_euler(
                                EulerRot::XYZ,
                                rng.random_range(0.0..std::f32::consts::TAU),
                                rng.random_range(0.0..std::f32::consts::TAU),
                                rng.random_range(0.0..std::f32::consts::TAU),
                            )),
                        crate::player::shooting::ExplosionParticle {
                            velocity: dir * speed,
                            timer: Timer::from_seconds(life, TimerMode::Once),
                            max_time: life,
                            start_scale: 0.1,
                            end_scale: s,
                        },
                    ));
                }
            }
        }
    }

    let known_ids: std::collections::HashSet<uuid::Uuid> =
        remote_query.iter().map(|(_, rp, _, _)| rp.server_id).collect();

    // Update existing remote players: position, rotation, health, despawn if gone
    for (entity, mut rp, mut transform, mut visibility) in remote_query.iter_mut() {
        if let Some(p) = snapshot.players.iter().find(|p| p.id == rp.server_id) {
            rp.health = p.health;
            rp.weapon_id.clone_from(&p.weapon_id);
            let target = Vec3::new(p.position[0], p.position[1], p.position[2]);
            transform.translation = transform.translation.lerp(target, 0.3);
            transform.rotation = Quat::from_euler(bevy::math::EulerRot::YXZ, p.yaw, p.pitch, 0.0);
            *visibility = if p.health <= 0.0 {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        } else {
            commands.entity(entity).despawn();
        }
    }

    let pill_mesh = meshes.add(Capsule3d::new(0.3, 0.6));
    let pill_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.15, 0.2, 0.35), ..default()
    });

    for p in &snapshot.players {
        if let Some(lid) = local_player_id { if p.id == lid { continue; } }
        if known_ids.contains(&p.id) { continue; }

        info!("Spawning remote player {} ({}) health={:.0}", p.username, p.id, p.health);

            let remote = commands.spawn((
                crate::player::RemotePlayer {
                    server_id: p.id,
                    health: p.health,
                    username: p.username.clone(),
                    weapon_id: p.weapon_id.clone(),
                },
                Transform::from_xyz(p.position[0], p.position[1], p.position[2])
                    .with_rotation(Quat::from_euler(bevy::math::EulerRot::YXZ, p.yaw, p.pitch, 0.0)),
                Visibility::default(),
            )).id();

        // Spawn username text as a SEPARATE top-level entity (not a child),
        // so it doesn't inherit the player's rotation. Billboard handles
        // camera-facing; a sync system updates its position each frame.
        let username_pos = Vec3::new(p.position[0], p.position[1] + 2.3, p.position[2]);
        commands.spawn((
            Text2d::new(p.username.clone()),
            TextFont { font_size: FontSize::Px(24.0), ..default() },
            TextColor(Color::WHITE),
            Transform::from_translation(username_pos),
            crate::gameplay::Billboard,
            RemoteUsername { server_id: p.id },
        ));

        // Health bar fill as a separate entity, same approach.
        let bar_pos = Vec3::new(p.position[0], p.position[1] + 1.9, p.position[2]);
        let bar_width = 0.8;
        commands.spawn((
            Mesh3d(meshes.add(Rectangle::new(bar_width, 0.08))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.8, 0.2),
                ..default()
            })),
            Transform::from_translation(bar_pos),
            crate::gameplay::Billboard,
            RemoteHealthBar { server_id: p.id },
        ));

        // Weapon model as a CHILD (should rotate with player body).
        commands.entity(remote).with_children(|parent| {
            parent.spawn((
                Mesh3d(pill_mesh.clone()),
                MeshMaterial3d(pill_mat.clone()),
                Transform::from_xyz(0.0, 0.9, 0.0),
            ));

            // Weapon model at right hip
            spawn_remote_weapon(parent, &asset_server, &mut meshes, &mut materials, &registry, &p.weapon_id);
        });
    }

    for (entity, rp, _, _) in remote_query.iter_mut() {
        if !snapshot.players.iter().any(|p| p.id == rp.server_id) {
            let name = scoreboard.names.get(&rp.server_id).cloned().unwrap_or_else(|| "Unknown".to_string());
            info!("Player left: {name}");
            scoreboard.kills.remove(&rp.server_id);
            scoreboard.deaths.remove(&rp.server_id);
            scoreboard.scores.remove(&rp.server_id);
            scoreboard.names.remove(&rp.server_id);
            // Despawn remote player entity and its associated UI entities.
            // We find the UI entities by their server_id — they're top-level.
            for (e, bar) in bar_ui_query.iter() {
                if bar.server_id == rp.server_id {
                    commands.entity(e).despawn();
                }
            }
            for (e, name_comp) in name_ui_query.iter() {
                if name_comp.server_id == rp.server_id {
                    commands.entity(e).despawn();
                }
            }
            commands.entity(entity).despawn();
        }
    }
}

/// Prediction-buffer reconciliation with rewind + replay + exponential smoothing.
///
/// 1. Reconcile: pop acknowledged frames, compare last predicted position
///    with server, and if divergent, rewind to server state and replay
///    unacknowledged inputs.
/// 2. Smooth toward the corrected position:
///    - Error > 2.0  → hard snap (network teleport / major desync)
///    - Error > 0.1  → lerp 15% toward correction each frame (exponential smoothing)
///    - Error ≤ 0.1  → trust local prediction

/// Send the player's loadout to the server when entering a match.
fn send_loadout(
    tcp: Option<Res<tcp::TcpClient>>,
    rt: Option<Res<TokioRuntime>>,
    loadout: Option<Res<crate::weapons::PlayerLoadout>>,
    registry: Option<Res<crate::weapons::WeaponRegistry>>,
) {
    if let (Some(tcp), Some(rt), Some(registry)) = (tcp, rt, registry) {
        let tc = (*tcp).clone();
        let rt_h = rt.0.clone();
        let primary = loadout.as_ref().map(|l| l.primary.clone()).unwrap_or_else(|| "colt_m4a1".to_string());
        let secondary = registry.by_slot.get(&crate::weapons::WeaponSlot::Secondary)
            .and_then(|ids| ids.first()).cloned().unwrap_or_default();
        let melee = registry.by_slot.get(&crate::weapons::WeaponSlot::Melee)
            .and_then(|ids| ids.first()).cloned().unwrap_or_default();
        let equipment = registry.by_slot.get(&crate::weapons::WeaponSlot::Equipment)
            .and_then(|ids| ids.first()).cloned().unwrap_or_default();

        rt_h.spawn(async move {
            let msg = noctyrn_shared::protocol::ClientMessage::SetLoadout {
                primary,
                secondary,
                melee,
                equipment,
            };
            let _ = tc.send(&msg).await;
        });
    }
}

fn reconcile_prediction(
    udp: Res<udp::UdpClient>,
    mut pred_buf: ResMut<prediction::PredictionBuffer>,
    mut local_query: Query<
        (&mut PhysicalTranslation, &mut PreviousPhysicalTranslation, &mut Velocity),
        With<crate::player::LocalPlayer>,
    >,
) {
    let snapshot = {
        let guard = udp.latest_snapshot.lock().unwrap();
        guard.clone()
    };
    let Some(ref snapshot) = snapshot else {
        return;
    };

    let local_player_id = match *udp.player_id.lock().unwrap() {
        Some(id) => id,
        None => return,
    };

    let server_pos = match snapshot.players.iter().find(|p| p.id == local_player_id) {
        Some(p) => p,
        None => return,
    };

    if let Ok((mut phys, mut prev_phys, mut velocity)) = local_query.single_mut() {
        let buf = &mut *pred_buf;
        if let Some((corrected_pos, corrected_vel)) = buf.reconcile_and_replay(
            snapshot.last_processed_input,
            server_pos.position,
            server_pos.velocity,
        ) {
            let corrected = Vec3::new(corrected_pos[0], corrected_pos[1], corrected_pos[2]);
            let current = phys.0;
            let error = current.distance(corrected);

            if error > 2.0 {
                // Large error — hard snap (network teleport / server force-reset).
                phys.0 = corrected;
                prev_phys.0 = corrected;
                velocity.0 = Vec3::new(
                    corrected_vel[0], corrected_vel[1], corrected_vel[2],
                );
            } else if error > 0.1 {
                // Small error — exponential smoothing: move 15% toward correction.
                phys.0 = current.lerp(corrected, 0.15);
                // Snap velocity immediately (less noticeable and avoids momentum lag).
                velocity.0 = Vec3::new(
                    corrected_vel[0], corrected_vel[1], corrected_vel[2],
                );
            }
            // Error ≤ 0.1: trust local prediction, do nothing.
        }
    }
}

/// After processing snapshots, assign `SpectatorTarget` to the killer's remote
/// entity so the death-cam follows them.
fn assign_spectator_target(
    killer_info: Option<Res<crate::gameplay::KillerInfo>>,
    remote_query: Query<(Entity, &crate::player::RemotePlayer)>,
    mut commands: Commands,
) {
    let Some(killer) = killer_info.as_ref() else { return };
    let Some(killer_id) = killer.server_id else { return };
    for (entity, rp) in remote_query.iter() {
        if rp.server_id == killer_id {
            commands.entity(entity).insert(crate::gameplay::SpectatorTarget);
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

pub fn update_3d_damage_numbers(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut crate::player::DamageNumber, &mut Transform, &mut TextColor)>,
) {
    for (entity, mut number, mut transform, mut color) in query.iter_mut() {
        number.timer.tick(time.delta());
        let dt = time.delta_secs();
        transform.translation += number.velocity * dt;
        number.velocity.y *= 0.98;
        number.velocity.x *= 0.95;
        number.velocity.z *= 0.95;
        let alpha = 1.0 - number.timer.fraction();
        color.0 = color.0.with_alpha(alpha);
        if number.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Helper: spawn a weapon model as a child of a remote player entity.
pub fn spawn_remote_weapon(
    parent: &mut ChildSpawnerCommands,
    asset_server: &AssetServer,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    registry: &crate::weapons::WeaponRegistry,
    weapon_id: &str,
) {
    if let Some(config) = registry.weapons.get(weapon_id) {
        let mf = config.meta.model_path.split('#').next().unwrap_or("");
        if !mf.is_empty() && std::path::Path::new(&format!("assets/{mf}")).exists() {
            let pos = Vec3::new(
                config.meta.position_offset[0] * 2.0 + 0.2,
                config.meta.position_offset[1] * 2.0 + 1.0,
                config.meta.position_offset[2] * 2.0,
            );
            let rot = Quat::from_euler(
                bevy::math::EulerRot::XYZ,
                config.meta.rotation_offset[0],
                config.meta.rotation_offset[1],
                config.meta.rotation_offset[2],
            );
            parent.spawn((
                RemoteWeaponModel { weapon_id: weapon_id.to_string() },
                Transform::default(),
                Visibility::default(),
            )).with_children(|w| {
                w.spawn((
                    WorldAssetRoot(asset_server.load(&config.meta.model_path)),
                    Transform::from_translation(pos).with_rotation(rot).with_scale(Vec3::splat(config.meta.scale * 0.7)),
                    Visibility::default(),
                ));
            });
        } else {
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.25)))),
                MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.4, 0.4), ..default() })),
                Transform::from_xyz(0.5, 1.0, 0.0),
                RemoteWeaponModel { weapon_id: weapon_id.to_string() },
            ));
        }
    } else {
        parent.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.25)))),
            MeshMaterial3d(materials.add(StandardMaterial { base_color: Color::srgb(0.4, 0.4, 0.4), ..default() })),
            Transform::from_xyz(0.5, 1.0, 0.0),
            RemoteWeaponModel { weapon_id: weapon_id.to_string() },
        ));
    }
}

/// Marker on the weapon-model child, storing which weapon_id was rendered.
#[derive(Component)]
pub struct RemoteWeaponModel {
    pub weapon_id: String,
}

/// Syncs username text and health bar positions for remote players.
/// These entities are top-level (not children) so they don't inherit
/// the player's body rotation. Billboard makes them face the camera.
pub fn sync_remote_ui(
    remote_query: Query<(&crate::player::RemotePlayer, &GlobalTransform)>,
    mut bar_query: Query<(&mut Transform, &RemoteHealthBar), Without<RemoteUsername>>,
    mut name_query: Query<(&mut Transform, &RemoteUsername), Without<RemoteHealthBar>>,
) {
    for (rp, global) in remote_query.iter() {
        let pos = global.translation();
        let ratio = (rp.health / 100.0).clamp(0.0, 1.0);
        let bar_width = 0.8;

        for (mut transform, bar) in bar_query.iter_mut() {
            if bar.server_id == rp.server_id {
                // Position above player head; shift left as health decreases
                // so the bar shrinks toward the left edge.
                transform.translation = pos + Vec3::new((ratio - 1.0) * bar_width * 0.5, 1.9, 0.0);
                transform.scale.x = ratio;
            }
        }
    for (mut transform, name) in name_query.iter_mut() {
            if name.server_id == rp.server_id {
                transform.translation = pos + Vec3::new(0.0, 2.3, 0.0);
            }
        }
    }
}

/// Updates remote player weapon models when their weapon_id changes.
/// Despawns the old weapon child and spawns a new one with the correct model.
pub fn update_remote_weapons(
    mut commands: Commands,
    remote_query: Query<(Entity, &crate::player::RemotePlayer, &Children)>,
    weapon_query: Query<(Entity, &RemoteWeaponModel)>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    registry: Res<crate::weapons::WeaponRegistry>,
) {
    for (entity, rp, children) in remote_query.iter() {
        let mut existing_weapon: Option<Entity> = None;
        for child in children.iter() {
            if let Ok((we, wm)) = weapon_query.get(child) {
                if wm.weapon_id == rp.weapon_id {
                    existing_weapon = None; // already correct
                } else {
                    existing_weapon = Some(we); // needs replacement
                }
                break;
            }
        }
        if let Some(old) = existing_weapon {
            commands.entity(old).try_despawn();
            commands.entity(entity).with_children(|parent| {
                spawn_remote_weapon(parent, &asset_server, &mut meshes, &mut materials, &registry, &rp.weapon_id);
            });
        }
    }
}
