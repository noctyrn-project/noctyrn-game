use bevy::prelude::*;
use bevy::app::AppExit;
use bevy::input::keyboard::KeyboardInput;
use crate::player::GameState;
use crate::weapons::{WeaponRegistry, WeaponSlot, PlayerLoadout, WeaponConfig, sync_loadout_to_configs, WeaponSkin, WeaponSkinTag, SkinRarity, SkinInventory, PlayerCredits};
use crate::net::{ConnectionState, ServerConfig, TokioRuntime, NetworkEvent, CachedProfile, CachedFriends, PartyState};
use crate::net::http::{PendingRequests, spawn_http_request};
use crate::net::tcp::TcpClient;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Game Modes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameMode {
    #[default]
    FreeForAll,
    TeamDeathmatch,
    KillConfirmed,
    CaptureTheFlag,
    Assassins,
    KingOfTheHill,
    Hardpoint,
    CapturePoint,
    TestingGrounds,
    // ── Limited-Time Modes ──
    Juggernaut,
    HighExplosives,
    OneInTheChamber,
    GunGame,
    Infected,
}

impl GameMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            GameMode::FreeForAll => "FREE FOR ALL",
            GameMode::TeamDeathmatch => "TEAM DEATHMATCH",
            GameMode::KillConfirmed => "KILL CONFIRMED",
            GameMode::CaptureTheFlag => "CAPTURE THE FLAG",
            GameMode::Assassins => "ASSASSINS",
            GameMode::KingOfTheHill => "KING OF THE HILL",
            GameMode::Hardpoint => "HARDPOINT",
            GameMode::CapturePoint => "CAPTURE POINT",
            GameMode::TestingGrounds => "TESTING GROUNDS",
            GameMode::Juggernaut => "JUGGERNAUT",
            GameMode::HighExplosives => "HIGH EXPLOSIVES",
            GameMode::OneInTheChamber => "ONE IN THE CHAMBER",
            GameMode::GunGame => "GUN GAME",
            GameMode::Infected => "INFECTED",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            GameMode::FreeForAll => "FFA",
            GameMode::TeamDeathmatch => "TDM",
            GameMode::KillConfirmed => "KC",
            GameMode::CaptureTheFlag => "CTF",
            GameMode::Assassins => "ASN",
            GameMode::KingOfTheHill => "KOTH",
            GameMode::Hardpoint => "HP",
            GameMode::CapturePoint => "CP",
            GameMode::TestingGrounds => "TG",
            GameMode::Juggernaut => "JGR",
            GameMode::HighExplosives => "HE",
            GameMode::OneInTheChamber => "OITC",
            GameMode::GunGame => "GG",
            GameMode::Infected => "INF",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GameMode::FreeForAll => "Every player for themselves. Get the most kills to win.",
            GameMode::TeamDeathmatch => "Two teams fight. First to 150 kills wins.",
            GameMode::KillConfirmed => "Team mode. Collect dog tags from fallen enemies to score.",
            GameMode::CaptureTheFlag => "Steal the enemy flag and return it to your base to score.",
            GameMode::Assassins => "Each player has a specific target. Hunt them down.",
            GameMode::KingOfTheHill => "Hold the zone for 5 seconds to capture. Control it to win.",
            GameMode::Hardpoint => "Three smaller zones. Hold them to earn points.",
            GameMode::CapturePoint => "Three moving zones. Capture them instantly to score.",
            GameMode::TestingGrounds => "Practice with all weapons. Spawn targets, test movement.",
            GameMode::Juggernaut => "A random player becomes the Juggernaut with 1000 HP and a minigun. Kill them to become the new Juggernaut.",
            GameMode::HighExplosives => "Explosive weapons only. RPGs, grenade launchers, and more.",
            GameMode::OneInTheChamber => "One bullet. One kill grants another bullet. Knife as backup.",
            GameMode::GunGame => "Cycle through every weapon. First to get a kill with each wins.",
            GameMode::Infected => "One player starts infected. Kill survivors to spread the infection.",
        }
    }

    pub fn player_count(&self) -> &'static str {
        match self {
            GameMode::TestingGrounds => "1 Player",
            _ => "Up to 50 Players",
        }
    }

    pub fn accent_color(&self) -> Color {
        match self {
            GameMode::FreeForAll => Color::srgb(0.9, 0.3, 0.3),
            GameMode::TeamDeathmatch => Color::srgb(0.3, 0.5, 0.9),
            GameMode::KillConfirmed => Color::srgb(0.9, 0.7, 0.2),
            GameMode::CaptureTheFlag => Color::srgb(0.3, 0.8, 0.5),
            GameMode::Assassins => Color::srgb(0.7, 0.2, 0.8),
            GameMode::KingOfTheHill => Color::srgb(0.9, 0.5, 0.1),
            GameMode::Hardpoint => Color::srgb(0.2, 0.7, 0.8),
            GameMode::CapturePoint => Color::srgb(0.5, 0.8, 0.3),
            GameMode::TestingGrounds => Color::srgb(0.4, 0.7, 0.9),
            GameMode::Juggernaut => Color::srgb(0.95, 0.2, 0.1),
            GameMode::HighExplosives => Color::srgb(1.0, 0.6, 0.0),
            GameMode::OneInTheChamber => Color::srgb(0.6, 0.6, 0.6),
            GameMode::GunGame => Color::srgb(0.2, 0.9, 0.4),
            GameMode::Infected => Color::srgb(0.4, 0.8, 0.1),
        }
    }

    /// Standard competitive modes (excludes Testing Grounds and LTMs)
    pub fn competitive_modes() -> &'static [GameMode] {
        &[
            GameMode::FreeForAll,
            GameMode::TeamDeathmatch,
            GameMode::KillConfirmed,
            GameMode::CaptureTheFlag,
            GameMode::Assassins,
            GameMode::KingOfTheHill,
            GameMode::Hardpoint,
            GameMode::CapturePoint,
        ]
    }

    /// Limited-time modes
    pub fn ltm_modes() -> &'static [GameMode] {
        &[
            GameMode::Juggernaut,
            GameMode::HighExplosives,
            GameMode::OneInTheChamber,
            GameMode::GunGame,
            GameMode::Infected,
        ]
    }

    /// Whether this mode has two teams (red vs blue)
    pub fn is_team_mode(&self) -> bool {
        matches!(
            self,
            GameMode::TeamDeathmatch
                | GameMode::KillConfirmed
                | GameMode::CaptureTheFlag
                | GameMode::KingOfTheHill
                | GameMode::Hardpoint
                | GameMode::CapturePoint
                | GameMode::Infected
        )
    }

    /// Whether this mode is a limited-time mode
    pub fn is_ltm(&self) -> bool {
        matches!(
            self,
            GameMode::Juggernaut
                | GameMode::HighExplosives
                | GameMode::OneInTheChamber
                | GameMode::GunGame
                | GameMode::Infected
        )
    }
}

#[derive(Resource)]
pub struct SelectedGameMode {
    pub mode: GameMode,
}

impl Default for SelectedGameMode {
    fn default() -> Self {
        Self { mode: GameMode::FreeForAll }
    }
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadoutUiState>();
        app.init_resource::<LoadoutDragState>();
        app.init_resource::<CrateState>();
        app.init_resource::<CrateWeaponPickerState>();
        app.init_resource::<SelectedGameMode>();
        app.init_resource::<SellConfirmState>();
        app.add_systems(OnEnter(GameState::MainMenu), (setup_main_menu_scene, spawn_main_menu));
        app.add_systems(OnExit(GameState::MainMenu), (despawn_main_menu, cleanup_main_menu_scene));
        app.add_systems(Update, (main_menu_interaction, main_menu_hover, rotate_main_menu_pill).run_if(in_state(GameState::MainMenu)));

        app.add_systems(OnEnter(GameState::LoadoutSelect), (setup_loadout_scene, spawn_loadout_menu));
        app.add_systems(OnExit(GameState::LoadoutSelect), (despawn_loadout_menu, cleanup_loadout_scene));
        app.add_systems(Update, loadout_interaction.run_if(in_state(GameState::LoadoutSelect)));
        app.add_systems(Update, update_loadout_ui.run_if(in_state(GameState::LoadoutSelect)));
        app.add_systems(Update, update_loadout_tabs.run_if(in_state(GameState::LoadoutSelect)));
        app.add_systems(Update, handle_loadout_drag.run_if(in_state(GameState::LoadoutSelect)));
        app.add_systems(Update, update_loadout_preview_model.run_if(in_state(GameState::LoadoutSelect)));

        app.add_systems(OnEnter(GameState::CrateOpening), (ensure_menu_camera, spawn_crate_menu));
        app.add_systems(OnExit(GameState::CrateOpening), despawn_crate_menu);
        app.add_systems(Update, (crate_interaction, update_crate_animation, crate_weapon_picker_interaction, crate_skip_interaction).run_if(in_state(GameState::CrateOpening)));

        app.add_systems(OnEnter(GameState::GameModeSelect), (ensure_menu_camera, spawn_gamemode_menu));
        app.add_systems(OnExit(GameState::GameModeSelect), despawn_gamemode_menu);
        app.add_systems(Update, (gamemode_interaction, gamemode_hover).run_if(in_state(GameState::GameModeSelect)));
        app.add_systems(OnEnter(GameState::Cosmetics), (ensure_menu_camera, spawn_cosmetics_menu));
        app.add_systems(OnExit(GameState::Cosmetics), despawn_cosmetics_menu);
        app.add_systems(Update, (cosmetics_interaction, cosmetics_hover, sell_confirm_interaction).run_if(in_state(GameState::Cosmetics)));

        // Login screen
        app.init_resource::<LoginUiState>();
        app.add_systems(OnEnter(GameState::Login), (ensure_menu_camera, spawn_login_screen));
        app.add_systems(OnExit(GameState::Login), despawn_login_screen);
        app.add_systems(Update, (login_interaction, login_text_input, login_handle_network_events).run_if(in_state(GameState::Login)));

        // Profile screen
        app.add_systems(OnEnter(GameState::Profile), (ensure_menu_camera, spawn_profile_screen, request_profile_data));
        app.add_systems(OnExit(GameState::Profile), despawn_profile_screen);
        app.add_systems(Update, (profile_interaction, profile_update_data).run_if(in_state(GameState::Profile)));

        // Friends screen
        app.init_resource::<FriendsUiState>();
        app.add_systems(OnEnter(GameState::Friends), (ensure_menu_camera, spawn_friends_screen, request_friends_data));
        app.add_systems(OnExit(GameState::Friends), despawn_friends_screen);
        app.add_systems(Update, (friends_interaction, friends_text_input, friends_update_data, friends_handle_network_events).run_if(in_state(GameState::Friends)));

        // Lobby screen
        app.init_resource::<LobbyState>();
        app.init_resource::<LobbyInviteText>();
        app.add_systems(OnEnter(GameState::Lobby), (ensure_menu_camera, spawn_lobby_screen, lobby_on_enter));
        app.add_systems(OnExit(GameState::Lobby), despawn_lobby_screen);
        app.add_systems(Update, (lobby_interaction, lobby_update, lobby_invite_input_system).run_if(in_state(GameState::Lobby)));

        // Matchmaking screen
        app.init_resource::<MatchmakingTimer>();
        app.add_systems(OnEnter(GameState::Matchmaking), (ensure_menu_camera, spawn_matchmaking_screen));
        app.add_systems(OnExit(GameState::Matchmaking), despawn_matchmaking_screen);
        app.add_systems(Update, (matchmaking_interaction, matchmaking_update).run_if(in_state(GameState::Matchmaking)));

        app.add_systems(OnEnter(GameState::Playing), despawn_menu_camera);

        // Global party invite overlay (runs in all states)
        app.add_systems(Update, party_invite_overlay_system);
    }
}

#[derive(Component)]
struct MenuCamera;

fn ensure_menu_camera(
    mut commands: Commands,
    existing: Query<Entity, With<MenuCamera>>,
) {
    if existing.is_empty() {
        commands.spawn((Camera2d, MenuCamera));
    }
}

fn despawn_menu_camera(mut commands: Commands, query: Query<Entity, With<MenuCamera>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Main Menu 3D Scene
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct MainMenuSceneEntity;

#[derive(Component)]
struct MainMenuPill;

const MENU_SCENE_ORIGIN: Vec3 = Vec3::new(200.0, 200.0, 200.0);

fn setup_main_menu_scene(
    mut commands: Commands,
    existing_menu_cam: Query<Entity, With<MenuCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    loadout: Res<PlayerLoadout>,
    registry: Res<WeaponRegistry>,
    asset_server: Res<AssetServer>,
) {
    // Despawn any existing 2D menu camera
    for entity in existing_menu_cam.iter() {
        commands.entity(entity).despawn();
    }

    // Spawn 3D camera for main menu
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.08, 0.08, 0.14)),
            ..default()
        },
        Transform::from_translation(MENU_SCENE_ORIGIN + Vec3::new(0.0, 1.2, 4.0))
            .looking_at(MENU_SCENE_ORIGIN + Vec3::new(0.0, 0.7, 0.0), Vec3::Y),
        MainMenuSceneEntity,
    ));

    // Lighting
    commands.spawn((
        PointLight {
            color: Color::srgb(0.95, 0.92, 1.0),
            intensity: 150_000.0,
            range: 25.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(MENU_SCENE_ORIGIN + Vec3::new(2.0, 4.0, 3.0)),
        MainMenuSceneEntity,
    ));
    commands.spawn((
        PointLight {
            color: Color::srgb(0.4, 0.5, 0.85),
            intensity: 60_000.0,
            range: 20.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(MENU_SCENE_ORIGIN + Vec3::new(-3.0, 2.0, -1.0)),
        MainMenuSceneEntity,
    ));

    // Pedestal (cylinder)
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.6, 0.3))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.15, 0.2),
            metallic: 0.8,
            perceptual_roughness: 0.2,
            ..default()
        })),
        Transform::from_translation(MENU_SCENE_ORIGIN + Vec3::new(0.0, 0.15, 0.0)),
        MainMenuSceneEntity,
    ));

    // Pill character (capsule) on pedestal
    let pill_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.65, 0.7),
        metallic: 0.3,
        perceptual_roughness: 0.5,
        ..default()
    });
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.3, 1.0))),
        MeshMaterial3d(pill_material),
        Transform::from_translation(MENU_SCENE_ORIGIN + Vec3::new(0.0, 1.1, 0.0)),
        MainMenuSceneEntity,
        MainMenuPill,
    )).with_children(|pill| {
        // Weapon model held by character
        let weapon_id = &loadout.primary;
        if let Some(config) = registry.weapons.get(weapon_id) {
            let model_file = config.meta.model_path.split('#').next().unwrap_or("");
            let model_exists = !model_file.is_empty()
                && std::path::Path::new(&format!("assets/{}", model_file)).exists();

            if model_exists {
                pill.spawn((
                    SceneRoot(asset_server.load(&config.meta.model_path)),
                    Transform::from_translation(Vec3::new(0.35, -0.1, -0.2))
                        .with_rotation(Quat::from_rotation_y(-0.3))
                        .with_scale(Vec3::splat(config.meta.scale * 1.5)),
                ));
            } else {
                // Gray placeholder block
                pill.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.06, 0.1, 0.5))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: Color::srgb(0.25, 0.25, 0.3),
                        metallic: 0.6,
                        perceptual_roughness: 0.3,
                        ..default()
                    })),
                    Transform::from_translation(Vec3::new(0.35, -0.1, -0.2))
                        .with_rotation(Quat::from_rotation_y(-0.3)),
                ));
            }
        }
    });

    // Floor disc
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(3.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.1, 0.1, 0.15, 0.85),
            metallic: 0.9,
            perceptual_roughness: 0.1,
            ..default()
        })),
        Transform::from_translation(MENU_SCENE_ORIGIN)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        MainMenuSceneEntity,
    ));
}

fn cleanup_main_menu_scene(
    mut commands: Commands,
    query: Query<Entity, With<MainMenuSceneEntity>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn rotate_main_menu_pill(
    _time: Res<Time>,
    mut query: Query<&mut Transform, With<MainMenuPill>>,
) {
    // Face forward (toward camera) - no spinning
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::IDENTITY;
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Main Menu
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct MainMenuUi;

#[derive(Component)]
enum MainMenuButton {
    Play,
    GameModeSelect,
    Loadout,
    Crates,
    Cosmetics,
    Profile,
    Friends,
    Settings,
    Quit,
}

#[derive(Component)]
struct MainMenuCreditsText;

fn spawn_main_menu(mut commands: Commands, selected_mode: Res<SelectedGameMode>, credits: Res<PlayerCredits>) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(50.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        MainMenuUi,
    )).with_children(|root| {
        // Top section - Title and Credits
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::FlexStart,
            ..default()
        }).with_children(|top_row| {
            top_row.spawn(Node {
                flex_direction: FlexDirection::Column,
                ..default()
            }).with_children(|top| {
                top.spawn((
                    Text::new("NOCTYRN"),
                    TextFont { font_size: 84.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.1, 0.1)),
                ));
                top.spawn((
                    Text::new("TACTICAL SHOOTER"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
                    Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
                ));
            });
            
            // Credits display
            top_row.spawn((
                Node {
                    padding: UiRect::all(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.8)),
                BorderColor::all(Color::srgba(0.9, 0.7, 0.2, 0.5)),
            )).with_children(|credits_box| {
                credits_box.spawn((
                    Text::new("CREDITS: "),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                ));
                credits_box.spawn((
                    Text::new(credits.balance.to_string()),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.7, 0.2)),
                    MainMenuCreditsText,
                ));
            });
        });

        // Bottom section
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::End,
            ..default()
        }).with_children(|bottom| {
            // Left side - menu buttons
            bottom.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            }).with_children(|left| {
                for (label, button, text_color) in [
                    ("LOADOUT", MainMenuButton::Loadout, Color::WHITE),
                    ("CRATES", MainMenuButton::Crates, Color::srgba(0.9, 0.7, 0.2, 0.9)),
                    ("COSMETICS", MainMenuButton::Cosmetics, Color::srgba(0.2, 0.8, 0.4, 0.9)),
                    ("PROFILE", MainMenuButton::Profile, Color::srgba(0.4, 0.6, 1.0, 0.9)),
                    ("FRIENDS", MainMenuButton::Friends, Color::srgba(0.6, 0.4, 0.9, 0.9)),
                    ("SETTINGS", MainMenuButton::Settings, Color::srgba(0.7, 0.7, 0.7, 0.9)),
                    ("QUIT", MainMenuButton::Quit, Color::srgba(0.6, 0.4, 0.4, 0.8)),
                ] {
                    left.spawn((
                        Button,
                        Node {
                            padding: UiRect::new(Val::Px(12.0), Val::Px(20.0), Val::Px(6.0), Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        button,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new(label),
                            TextFont { font_size: 18.0, ..default() },
                            TextColor(text_color),
                        ));
                    });
                }

                left.spawn((
                    Text::new("v0.1.0"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(Color::srgba(0.3, 0.3, 0.3, 0.4)),
                    Node { margin: UiRect::top(Val::Px(20.0)), ..default() },
                ));
            });

            // Right side - gamemode + PLAY button stack
            bottom.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                row_gap: Val::Px(8.0),
                ..default()
            }).with_children(|right| {
                // Currently selected gamemode button
                right.spawn((
                    Button,
                    Node {
                        width: Val::Px(240.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.9)),
                    MainMenuButton::GameModeSelect,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(format!("▸ {}", selected_mode.mode.display_name())),
                        TextFont { font_size: 13.0, ..default() },
                        TextColor(selected_mode.mode.accent_color()),
                    ));
                });

                // PLAY button
                right.spawn((
                    Button,
                    Node {
                        width: Val::Px(240.0),
                        height: Val::Px(64.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.45, 0.12)),
                    MainMenuButton::Play,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("PLAY"),
                        TextFont { font_size: 26.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            });
        });
    });
}

fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn main_menu_interaction(
    interaction_query: Query<(&Interaction, &MainMenuButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    party_state: Res<PartyState>,
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    selected_mode: Res<SelectedGameMode>,
    conn_state: Res<ConnectionState>,
) {
    // Handle Escape key to toggle settings or quit
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if let Some(entity) = settings_query.iter().next() {
            commands.entity(entity).despawn();
        } else {
            // If settings is not open, maybe open a pause menu or just settings
            crate::ui_settings::spawn_settings_menu(&mut commands);
        }
    }

    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            match button {
                MainMenuButton::Play => {
                    if party_state.party.is_some() {
                        next_state.set(GameState::Lobby);
                    } else if tcp_client.is_connected() {
                        // Solo: queue for match immediately
                        let msg = noctyrn_shared::protocol::ClientMessage::QueueForMatch {
                            game_mode: to_shared_gamemode(selected_mode.mode),
                        };
                        let tcp = tcp_client.clone();
                        let rt = rt.0.clone();
                        rt.spawn(async move {
                            let _ = tcp.send(&msg).await;
                        });
                        next_state.set(GameState::Matchmaking);
                    } else {
                        // Not connected – start local game as fallback
                        next_state.set(GameState::Playing);
                    }
                }
                MainMenuButton::GameModeSelect => {
                    next_state.set(GameState::GameModeSelect);
                }
                MainMenuButton::Loadout => {
                    next_state.set(GameState::LoadoutSelect);
                }
                MainMenuButton::Crates => {
                    next_state.set(GameState::CrateOpening);
                }
                MainMenuButton::Cosmetics => {
                    next_state.set(GameState::Cosmetics);
                }
                MainMenuButton::Profile => {
                    next_state.set(GameState::Profile);
                }
                MainMenuButton::Friends => {
                    next_state.set(GameState::Friends);
                }
                MainMenuButton::Settings => {
                    if let Some(entity) = settings_query.iter().next() {
                        commands.entity(entity).despawn();
                    } else {
                        crate::ui_settings::spawn_settings_menu(&mut commands);
                    }
                }
                MainMenuButton::Quit => {
                    exit.write(AppExit::Success);
                }
            }
        }
    }
}

fn main_menu_hover(
    mut query: Query<(&Interaction, &MainMenuButton, &Children), With<Button>>,
    mut text_query: Query<&mut TextColor>,
) {
    for (interaction, button, children) in query.iter_mut() {
        let (base_color, hover_color) = match button {
            MainMenuButton::Play => (Color::WHITE, Color::srgb(0.5, 1.0, 0.5)),
            MainMenuButton::GameModeSelect => (Color::srgba(0.6, 0.6, 0.7, 0.9), Color::WHITE),
            MainMenuButton::Loadout => (Color::WHITE, Color::srgb(0.7, 0.85, 1.0)),
            MainMenuButton::Crates => (Color::srgba(0.9, 0.7, 0.2, 0.9), Color::srgb(1.0, 0.85, 0.3)),
            MainMenuButton::Cosmetics => (Color::srgba(0.2, 0.8, 0.4, 0.9), Color::srgb(0.4, 1.0, 0.6)),
            MainMenuButton::Profile => (Color::srgba(0.4, 0.6, 1.0, 0.9), Color::srgb(0.6, 0.8, 1.0)),
            MainMenuButton::Friends => (Color::srgba(0.6, 0.4, 0.9, 0.9), Color::srgb(0.8, 0.6, 1.0)),
            MainMenuButton::Settings => (Color::srgba(0.7, 0.7, 0.7, 0.9), Color::WHITE),
            MainMenuButton::Quit => (Color::srgba(0.6, 0.4, 0.4, 0.8), Color::srgb(1.0, 0.5, 0.5)),
        };
        let color = match interaction {
            Interaction::Hovered | Interaction::Pressed => hover_color,
            _ => base_color,
        };
        for child in children.iter() {
            if let Ok(mut text_color) = text_query.get_mut(child) {
                text_color.0 = color;
            }
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Loadout Selection
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Resource, Default)]
struct LoadoutUiState {
    active_slot: WeaponSlot,
    active_category: Option<String>,
    selected_weapon_id: Option<String>,
    selected_skin: WeaponSkin,
    preview_needs_update: bool,
    last_weapon_click: Option<(String, f64)>, // (weapon_id, time) for double-click detection
}

#[derive(Resource)]
struct LoadoutDragState {
    dragging: bool,
    last_pos: Vec2,
    rotation_y: f32,
    rotation_x: f32,
    zoom: f32,
}

impl Default for LoadoutDragState {
    fn default() -> Self {
        Self {
            dragging: false,
            last_pos: Vec2::ZERO,
            rotation_y: 0.0,
            rotation_x: 0.0,
            zoom: 2.5,
        }
    }
}

#[derive(Component)]
struct LoadoutMenuUi;

#[derive(Component)]
struct LoadoutBackButton;

#[derive(Component)]
struct SlotTabButton {
    slot: WeaponSlot,
}

#[derive(Component)]
struct CategoryButton {
    category: String,
}

#[derive(Component)]
struct WeaponSelectButton {
    weapon_id: String,
}

#[derive(Component)]
struct EquipButton;

#[derive(Component)]
struct WeaponListContainer;

#[derive(Component)]
struct CategoryTabContainer;

#[derive(Component)]
struct WeaponStatsPanel;

#[derive(Component)]
struct CurrentLoadoutDisplay;

#[derive(Component)]
struct SkinButton {
    skin: WeaponSkin,
}

#[derive(Component)]
struct ColorPickerButton;

#[derive(Component)]
struct ColorPickerPanel;

#[derive(Component)]
struct ColorPickerCloseButton;

#[derive(Component)]
struct SkinPanel;

#[derive(Component)]
struct LoadoutPreviewCamera;

#[derive(Component)]
struct LoadoutPreviewModel;

#[derive(Component)]
struct LoadoutPreviewLight;

const PREVIEW_ORIGIN: Vec3 = Vec3::new(500.0, 500.0, 500.0);

fn setup_loadout_scene(
    mut commands: Commands,
    existing_menu_cam: Query<Entity, With<MenuCamera>>,
    mut drag_state: ResMut<LoadoutDragState>,
) {
    // Despawn 2D menu camera
    for entity in existing_menu_cam.iter() {
        commands.entity(entity).despawn();
    }
    *drag_state = LoadoutDragState::default();

    // Spawn 3D preview camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.12, 0.12, 0.18)),
            ..default()
        },
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(0.0, 0.3, 2.5))
            .looking_at(PREVIEW_ORIGIN + Vec3::new(0.0, 0.1, 0.0), Vec3::Y),
        LoadoutPreviewCamera,
    ));

    // Spawn preview lighting
    commands.spawn((
        PointLight {
            color: Color::srgb(0.95, 0.95, 1.0),
            intensity: 50_000.0,
            range: 20.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(2.0, 3.0, 3.0)),
        LoadoutPreviewLight,
    ));
    commands.spawn((
        PointLight {
            color: Color::srgb(0.4, 0.5, 0.8),
            intensity: 20_000.0,
            range: 15.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(-2.0, 1.0, -1.0)),
        LoadoutPreviewLight,
    ));
}

fn cleanup_loadout_scene(
    mut commands: Commands,
    camera_query: Query<Entity, With<LoadoutPreviewCamera>>,
    model_query: Query<Entity, With<LoadoutPreviewModel>>,
    light_query: Query<Entity, With<LoadoutPreviewLight>>,
) {
    for entity in camera_query.iter() { commands.entity(entity).despawn(); }
    for entity in model_query.iter() { commands.entity(entity).despawn(); }
    for entity in light_query.iter() { commands.entity(entity).despawn(); }
}

fn spawn_loadout_menu(
    mut commands: Commands,
    registry: Res<WeaponRegistry>,
    loadout: Res<PlayerLoadout>,
    mut ui_state: ResMut<LoadoutUiState>,
) {
    ui_state.active_slot = WeaponSlot::Primary;
    ui_state.active_category = None;
    ui_state.selected_weapon_id = Some(loadout.primary.clone());
    ui_state.selected_skin = loadout.get_skin(WeaponSlot::Primary);
    ui_state.preview_needs_update = true;

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        LoadoutMenuUi,
    )).with_children(|root| {
        // ── Top Bar ──
        root.spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(56.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(20.0)),
            ..default()
        }).with_children(|bar| {
            bar.spawn((
                Button,
                Node {
                    width: Val::Px(90.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
                LoadoutBackButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("BACK"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                ));
            });

            bar.spawn((
                Text::new("LOADOUT"),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::WHITE),
                Node { margin: UiRect::left(Val::Px(16.0)), ..default() },
            ));

            bar.spawn(Node { flex_grow: 1.0, ..default() });

            bar.spawn((
                Text::new(format_loadout_summary(&loadout, &registry)),
                TextFont { font_size: 11.0, ..default() },
                TextColor(Color::srgba(0.5, 0.7, 0.5, 0.8)),
                CurrentLoadoutDisplay,
            ));
        });

        // ── Slot Tabs ──
        root.spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(44.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(2.0),
            padding: UiRect::horizontal(Val::Px(20.0)),
            align_items: AlignItems::End,
            ..default()
        }).with_children(|tabs| {
            for (label, slot) in [
                ("PRIMARY", WeaponSlot::Primary),
                ("SECONDARY", WeaponSlot::Secondary),
                ("MELEE", WeaponSlot::Melee),
                ("EQUIPMENT", WeaponSlot::Equipment),
            ] {
                let is_active = slot == ui_state.active_slot;
                tabs.spawn((
                    Button,
                    Node {
                        width: Val::Px(130.0),
                        height: Val::Px(if is_active { 38.0 } else { 34.0 }),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if is_active {
                        Color::srgba(0.2, 0.35, 0.55, 0.9)
                    } else {
                        Color::srgba(0.12, 0.12, 0.18, 0.7)
                    }),
                    SlotTabButton { slot },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(if is_active { Color::WHITE } else { Color::srgba(0.6, 0.6, 0.6, 0.8) }),
                    ));
                });
            }
        });

        // ── Main Content: Category tabs + Left panel + Center/Right preview+stats ──
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        }).with_children(|main_area| {
            // Horizontal subcategory tabs (full width, independent of weapon list)
            main_area.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::NoWrap,
                    padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(4.0), Val::Px(4.0)),
                    column_gap: Val::Px(2.0),
                    overflow: Overflow::scroll_x(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.95)),
                CategoryTabContainer,
            ));

            // Row with left panel + center preview + stats
            main_area.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                ..default()
            }).with_children(|content| {
                // Left: Weapon browser panel (weapon list only)
                content.spawn((
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::scroll_y(),
                        row_gap: Val::Px(2.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.95)),
                    WeaponListContainer,
                ));

                // Center: 3D preview takes up the rest
                content.spawn(Node {
                    flex_grow: 1.0,
                    height: Val::Percent(100.0),
                    ..default()
                });

                // Stats + Skin panel (top-right floating overlay)
                content.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(20.0),
                        top: Val::Px(12.0),
                        width: Val::Px(320.0),
                        max_height: Val::Percent(70.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(12.0)),
                        row_gap: Val::Px(4.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.80)),
                    WeaponStatsPanel,
                ));
            });
        });
    });
}

fn despawn_loadout_menu(mut commands: Commands, query: Query<Entity, With<LoadoutMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn category_display_name(category: &str) -> &str {
    match category {
        "assault" => "Assault Rifles",
        "carbine" => "Carbines",
        "smg" => "SMGs",
        "pdw" => "PDWs",
        "lmg" => "LMBGs",
        "dmr" => "DMRs",
        "sniper" => "Snipers",
        "shotgun" => "Shotguns",
        "rifle" => "Rifles",
        "pistol" => "Pistols",
        "revolver" => "Revolvers",
        "mpistol" => "Machine Pistols",
        "blade" => "Blades",
        "2hblade" => "Two-Handed",
        "grenade" => "Grenades",
        "other" => "Special",
        _ => category,
    }
}

/// Sort order for categories within a slot.
fn category_sort_order(category: &str) -> u32 {
    match category {
        "assault" => 0,
        "carbine" => 1,
        "smg" => 2,
        "pdw" => 3,
        "lmg" => 4,
        "dmr" => 5,
        "sniper" => 6,
        "shotgun" => 7,
        "rifle" => 8,
        "pistol" => 0,
        "revolver" => 1,
        "mpistol" => 2,
        "blade" => 0,
        "2hblade" => 1,
        "grenade" => 0,
        "other" => 10,
        _ => 9,
    }
}

fn update_loadout_ui(
    mut commands: Commands,
    registry: Res<WeaponRegistry>,
    loadout: Res<PlayerLoadout>,
    ui_state: Res<LoadoutUiState>,
    list_query: Query<Entity, With<WeaponListContainer>>,
    stats_query: Query<Entity, With<WeaponStatsPanel>>,
    cat_tab_query: Query<Entity, With<CategoryTabContainer>>,
    mut display_query: Query<&mut Text, With<CurrentLoadoutDisplay>>,
    color_panel_query: Query<Entity, With<ColorPickerPanel>>,
) {
    if !ui_state.is_changed() {
        return;
    }

    // Update loadout summary text
    for mut text in display_query.iter_mut() {
        text.0 = format_loadout_summary(&loadout, &registry);
    }

    let slot = ui_state.active_slot;
    
    // Gather categories for this slot
    let mut categories: Vec<(String, Vec<String>)> = Vec::new();
    if let Some(weapon_ids) = registry.by_slot.get(&slot) {
        let mut cat_map: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
        for id in weapon_ids {
            if let Some(config) = registry.weapons.get(id) {
                let cat = if config.meta.category.is_empty() {
                    match config.meta.weapon_type.as_str() {
                        "Primary" | "Assault Rifle" => "assault",
                        "Secondary" | "Pistol" => "pistol",
                        "Melee" | "1 Handed Sharp" | "Blade" => "blade",
                        "2H Blade" => "2hblade",
                        "Grenade" | "Equipment" => "grenade",
                        "Revolver" => "revolver",
                        "Machine Pistol" => "mpistol",
                        _ => "other",
                    }.to_string()
                } else {
                    config.meta.category.clone()
                };
                cat_map.entry(cat).or_default().push(id.clone());
            }
        }
        let mut sorted: Vec<_> = cat_map.into_iter().collect();
        sorted.sort_by_key(|(cat, _)| category_sort_order(cat));
        categories = sorted;
    }

    // Auto-select first category if none selected
    if ui_state.active_category.is_none() && !categories.is_empty() {
        // Don't mutate through the ref - we'll handle it below
    }
    let active_cat = ui_state.active_category.clone().or_else(|| categories.first().map(|(c, _)| c.clone()));

    // Rebuild category tabs
    if let Some(cat_container) = cat_tab_query.iter().next() {
        commands.entity(cat_container).despawn_children();
        commands.entity(cat_container).with_children(|parent| {
            for (category, _ids) in &categories {
                let is_active = active_cat.as_deref() == Some(category.as_str());
                parent.spawn((
                    Button,
                    Node {
                        padding: UiRect::new(Val::Px(10.0), Val::Px(10.0), Val::Px(5.0), Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if is_active {
                        Color::srgba(0.2, 0.35, 0.55, 0.9)
                    } else {
                        Color::srgba(0.1, 0.1, 0.15, 0.7)
                    }),
                    CategoryButton { category: category.clone() },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(category_display_name(category).to_uppercase()),
                        TextFont { font_size: 10.0, ..default() },
                        TextColor(if is_active { Color::WHITE } else { Color::srgba(0.6, 0.6, 0.6, 0.8) }),
                    ));
                });
            }
        });
    }

    // Rebuild weapon list for selected category
    if let Some(list_entity) = list_query.iter().next() {
        commands.entity(list_entity).despawn_children();

        if let Some(ref active) = active_cat {
            if let Some((_, ids)) = categories.iter().find(|(c, _)| c == active) {
                commands.entity(list_entity).with_children(|parent| {
                    for id in ids {
                        let config = registry.weapons.get(id).unwrap();
                        let is_equipped = loadout.get_id_for_slot(slot) == id.as_str();
                        let is_selected = ui_state.selected_weapon_id.as_deref() == Some(id.as_str());

                        let bg = if is_selected {
                            Color::srgba(0.2, 0.35, 0.55, 0.5)
                        } else if is_equipped {
                            Color::srgba(0.12, 0.25, 0.12, 0.35)
                        } else {
                            Color::srgba(0.1, 0.1, 0.14, 0.2)
                        };

                        parent.spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(32.0),
                                padding: UiRect::horizontal(Val::Px(10.0)),
                                align_items: AlignItems::Center,
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                margin: UiRect::bottom(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(bg),
                            WeaponSelectButton { weapon_id: id.to_string() },
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new(&config.info.name),
                                TextFont { font_size: 13.0, ..default() },
                                TextColor(if is_selected { Color::WHITE } else { Color::srgba(0.85, 0.85, 0.85, 0.9) }),
                            ));
                            if is_equipped {
                                btn.spawn((
                                    Text::new("[E]"),
                                    TextFont { font_size: 10.0, ..default() },
                                    TextColor(Color::srgb(0.3, 0.8, 0.3)),
                                ));
                            }
                        });
                    }
                });
            }
        }
    }

    // Rebuild stats panel (weapon info + equip button + color picker button)
    if let Some(stats_entity) = stats_query.iter().next() {
        commands.entity(stats_entity).despawn_children();

        if let Some(weapon_id) = &ui_state.selected_weapon_id {
            if let Some(config) = registry.weapons.get(weapon_id) {
                commands.entity(stats_entity).with_children(|parent| {
                    spawn_weapon_stats(parent, config, weapon_id, &loadout, ui_state.active_slot);

                    // ── Color Picker Button ──
                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.4)),
                    ));

                    parent.spawn((
                        Button,
                        Node {
                            width: Val::Px(200.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            align_self: AlignSelf::Center,
                            column_gap: Val::Px(8.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.15, 0.15, 0.25, 0.9)),
                        ColorPickerButton,
                    )).with_children(|btn| {
                        // Show current skin color swatch
                        btn.spawn((
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(18.0),
                                ..default()
                            },
                            BackgroundColor(ui_state.selected_skin.swatch_color()),
                        ));
                        btn.spawn((
                            Text::new(format!("COLOR: {}", ui_state.selected_skin.display_name().to_uppercase())),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgba(0.8, 0.8, 0.9, 0.9)),
                        ));
                    });
                });
            }
        }
    }

    // Despawn existing color picker panel when UI state changes (it will reopen on click)
    for entity in color_panel_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn spawn_weapon_stats(parent: &mut ChildSpawnerCommands, config: &WeaponConfig, weapon_id: &str, loadout: &PlayerLoadout, slot: WeaponSlot) {
    // Weapon name
    parent.spawn((
        Text::new(&config.info.name),
        TextFont { font_size: 26.0, ..default() },
        TextColor(Color::WHITE),
    ));

    // Type + manufacturer
    parent.spawn((
        Text::new(format!("{} • {} • {}", config.meta.weapon_type, config.info.manufacturer, config.info.year_introduced)),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
    ));

    // Description
    parent.spawn((
        Text::new(&config.info.description),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(0.7, 0.7, 0.7, 0.7)),
        Node { margin: UiRect::vertical(Val::Px(6.0)), ..default() },
    ));

    // Separator
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.5)),
    ));

    // Stats based on weapon type
    let wt = config.meta.weapon_type.as_str();
    if wt == "Melee" || wt == "2H Blade" || wt == "Blade" {
        spawn_stat_bar(parent, "Attack Speed", config.attributes.attack_speed, 2.0);
        spawn_stat_bar(parent, "Stab Damage", config.attributes.stab_damage, 100.0);
        spawn_stat_bar(parent, "Slash Damage", config.attributes.slash_damage, 80.0);
        spawn_stat_bar(parent, "Reach", config.attributes.reach, 3.0);
        spawn_stat_bar(parent, "Mobility", config.attributes.mobility, 1.0);
    } else if wt == "Grenade" || wt == "Equipment" {
        spawn_stat_bar(parent, "Blast Damage", config.attributes.blast_damage, 200.0);
        spawn_stat_bar(parent, "Blast Radius", config.attributes.blast_radius, 10.0);
        spawn_stat_bar(parent, "Detonation Time", config.attributes.detonation_time, 5.0);
        spawn_stat_bar(parent, "Weight", config.attributes.weight, 1.0);
    } else {
        spawn_stat_bar(parent, "Fire Rate", config.attributes.fire_rate, 0.3);
        spawn_stat_bar(parent, "Accuracy", config.attributes.accuracy, 1.0);
        spawn_stat_bar(parent, "Stability", config.attributes.stability, 1.0);
        spawn_stat_bar(parent, "Mobility", config.attributes.mobility, 1.0);
        spawn_stat_bar(parent, "Reload Speed", config.attributes.reload_speed, 4.0);
        spawn_stat_bar(parent, "ADS Speed", config.attributes.ads_speed, 1.0);

        if let Some(ammo) = &config.attachments.ammo {
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.3)),
            ));
            parent.spawn((
                Text::new(format!("Ammo: {} • Damage: {:.0} • Pen: {:.0}%", ammo.name, ammo.damage, ammo.penetration * 100.0)),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgba(0.8, 0.7, 0.4, 0.9)),
            ));
        }

        if let Some(mag) = &config.attachments.magazine {
            parent.spawn((
                Text::new(format!("Magazine: {} rds • Reserve: {}", mag.capacity, mag.carry_capacity)),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgba(0.6, 0.7, 0.8, 0.9)),
            ));
        }

        if !config.attributes.fire_modes.is_empty() {
            parent.spawn((
                Text::new(format!("Fire Modes: {}", config.attributes.fire_modes.join(" / "))),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgba(0.6, 0.7, 0.8, 0.9)),
            ));
        }
    }

    // Equip button
    let is_equipped = loadout.get_id_for_slot(slot) == weapon_id;
    parent.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });

    parent.spawn((
        Button,
        Node {
            width: Val::Px(200.0),
            height: Val::Px(45.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            align_self: AlignSelf::Center,
            ..default()
        },
        BackgroundColor(if is_equipped {
            Color::srgb(0.2, 0.5, 0.2)
        } else {
            Color::srgb(0.2, 0.35, 0.6)
        }),
        EquipButton,
    )).with_children(|btn: &mut ChildSpawnerCommands| {
        btn.spawn((
            Text::new(if is_equipped { "EQUIPPED" } else { "EQUIP" }),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn spawn_stat_bar(parent: &mut ChildSpawnerCommands, label: &str, value: f32, max: f32) {
    parent.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: Val::Px(10.0),
        ..default()
    }).with_children(|row: &mut ChildSpawnerCommands| {
        row.spawn((
            Text::new(label),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
            Node { width: Val::Px(120.0), ..default() },
        ));

        row.spawn((
            Text::new(format!("{:.2}", value)),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn loadout_interaction(
    mut next_state: ResMut<NextState<GameState>>,
    mut ui_state: ResMut<LoadoutUiState>,
    mut loadout: ResMut<PlayerLoadout>,
    mut registry: ResMut<WeaponRegistry>,
    slot_query: Query<(&Interaction, &SlotTabButton), (Changed<Interaction>, With<Button>)>,
    weapon_query: Query<(&Interaction, &WeaponSelectButton), (Changed<Interaction>, With<Button>)>,
    skin_query: Query<(&Interaction, &SkinButton), (Changed<Interaction>, With<Button>)>,
    category_query: Query<(&Interaction, &CategoryButton), (Changed<Interaction>, With<Button>)>,
    mut btn_queries: ParamSet<(
        Query<(&Interaction, &mut BackgroundColor), (With<LoadoutBackButton>, With<Button>, Without<EquipButton>, Without<ColorPickerButton>, Without<ColorPickerCloseButton>)>,
        Query<(&Interaction, &mut BackgroundColor), (With<EquipButton>, With<Button>, Without<LoadoutBackButton>, Without<ColorPickerButton>, Without<ColorPickerCloseButton>)>,
        Query<(&Interaction, &mut BackgroundColor), (With<ColorPickerButton>, With<Button>, Without<LoadoutBackButton>, Without<EquipButton>, Without<ColorPickerCloseButton>)>,
        Query<(&Interaction, &mut BackgroundColor), (With<ColorPickerCloseButton>, With<Button>, Without<LoadoutBackButton>, Without<EquipButton>, Without<ColorPickerButton>)>,
    )>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    existing_color_panel: Query<Entity, With<ColorPickerPanel>>,
    loadout_ui_query: Query<Entity, With<LoadoutMenuUi>>,
) {
    // Handle Escape key to go back
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::MainMenu);
        return;
    }

    // Back button interaction + hover
    for (interaction, mut bg) in btn_queries.p0().iter_mut() {
        match interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    next_state.set(GameState::MainMenu);
                }
                *bg = BackgroundColor(Color::srgb(0.45, 0.2, 0.2));
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgb(0.45, 0.2, 0.2));
            }
            _ => {
                *bg = BackgroundColor(Color::srgb(0.3, 0.15, 0.15));
            }
        }
    }
    
    // Equip button hover + press
    for (interaction, mut bg) in btn_queries.p1().iter_mut() {
        let is_equipped = if let Some(id) = &ui_state.selected_weapon_id {
            loadout.get_id_for_slot(ui_state.active_slot) == id.as_str()
        } else {
            false
        };
        
        match interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    if let Some(id) = ui_state.selected_weapon_id.clone() {
                        loadout.set_id_for_slot(ui_state.active_slot, id.clone());
                        sync_loadout_to_configs(&mut registry, &loadout);
                        loadout.save();
                        // Force UI rebuild so equip status shows immediately
                        ui_state.preview_needs_update = true;
                    }
                }
                *bg = if is_equipped {
                    BackgroundColor(Color::srgb(0.25, 0.6, 0.25))
                } else {
                    BackgroundColor(Color::srgb(0.25, 0.4, 0.7))
                };
            }
            Interaction::Hovered => {
                *bg = if is_equipped {
                    BackgroundColor(Color::srgb(0.25, 0.6, 0.25))
                } else {
                    BackgroundColor(Color::srgb(0.25, 0.4, 0.7))
                };
            }
            _ => {
                *bg = if is_equipped {
                    BackgroundColor(Color::srgb(0.2, 0.5, 0.2))
                } else {
                    BackgroundColor(Color::srgb(0.2, 0.35, 0.6))
                };
            }
        }
    }

    // Slot tabs
    for (interaction, tab) in slot_query.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.active_slot = tab.slot;
            ui_state.active_category = None; // Reset category when switching slots
            ui_state.selected_weapon_id = Some(loadout.get_id_for_slot(tab.slot).to_string());
            ui_state.selected_skin = loadout.get_skin(tab.slot);
            ui_state.preview_needs_update = true;
        }
    }

    // Category tabs
    for (interaction, cat_btn) in category_query.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.active_category = Some(cat_btn.category.clone());
        }
    }

    // Weapon selection (single click = select, double click = equip)
    for (interaction, weapon_btn) in weapon_query.iter() {
        if *interaction == Interaction::Pressed {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            let is_double_click = if let Some((ref last_id, last_time)) = ui_state.last_weapon_click {
                last_id == &weapon_btn.weapon_id && (now - last_time) < 0.4
            } else {
                false
            };

            if is_double_click {
                // Double-click: equip the weapon
                let id = weapon_btn.weapon_id.clone();
                loadout.set_id_for_slot(ui_state.active_slot, id);
                sync_loadout_to_configs(&mut registry, &loadout);
                loadout.save();
                ui_state.preview_needs_update = true;
                ui_state.last_weapon_click = None;
            } else {
                // Single click: select the weapon
                ui_state.selected_weapon_id = Some(weapon_btn.weapon_id.clone());
                ui_state.preview_needs_update = true;
                ui_state.last_weapon_click = Some((weapon_btn.weapon_id.clone(), now));
            }
        }
    }

    // Skin selection (inside color picker panel)
    for (interaction, skin_btn) in skin_query.iter() {
        if *interaction == Interaction::Pressed {
            if mouse_input.just_pressed(MouseButton::Left) {
                ui_state.selected_skin = skin_btn.skin;
                loadout.set_skin(ui_state.active_slot, skin_btn.skin);
                loadout.save();
                ui_state.preview_needs_update = true;
                // Close color picker panel
                for entity in existing_color_panel.iter() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    // Color picker button - toggle panel
    for (interaction, mut bg) in btn_queries.p2().iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    let has_panel = !existing_color_panel.is_empty();
                    // Close existing panel
                    for entity in existing_color_panel.iter() {
                        commands.entity(entity).despawn();
                    }
                    // If no panel was open, spawn one
                    if !has_panel {
                        if let Some(root_entity) = loadout_ui_query.iter().next() {
                            let selected_skin = ui_state.selected_skin;
                            let weapon_id = ui_state.selected_weapon_id.clone().unwrap_or_default();
                            let owned_skins = SkinInventory::load().owned_skins_for(&weapon_id);
                            commands.entity(root_entity).with_children(|root| {
                                let att_names = if let Some(wid) = &ui_state.selected_weapon_id {
                                    registry.weapons.get(wid).map(|c| attachment_slot_names(&c.attachments)).unwrap_or_default()
                                } else {
                                    Vec::new()
                                };
                                spawn_color_picker_panel(root, selected_skin, &owned_skins, &att_names);
                            });
                        }
                    }
                }
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.35, 0.9));
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgba(0.18, 0.18, 0.3, 0.9));
            }
            _ => {
                *bg = BackgroundColor(Color::srgba(0.15, 0.15, 0.25, 0.9));
            }
        }
    }

    // Color picker close button
    for (interaction, _bg) in btn_queries.p3().iter_mut() {
        if *interaction == Interaction::Pressed {
            if mouse_input.just_pressed(MouseButton::Left) {
                for entity in existing_color_panel.iter() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

fn attachment_slot_names(att: &crate::weapons::WeaponAttachments) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Some(o) = &att.optic { result.push(("Optic".to_string(), o.name.clone())); }
    if let Some(b) = &att.barrel { result.push(("Barrel".to_string(), b.name.clone())); }
    if let Some(u) = &att.underbarrel { result.push(("Underbarrel".to_string(), u.name.clone())); }
    if let Some(s) = &att.sidebarrel { result.push(("Sidebarrel".to_string(), s.name.clone())); }
    if let Some(m) = &att.magazine { result.push(("Magazine".to_string(), m.name.clone())); }
    if let Some(a) = &att.ammo { result.push(("Ammo".to_string(), a.name.clone())); }
    if let Some(st) = &att.stock { result.push(("Stock".to_string(), st.name.clone())); }
    result
}

fn spawn_color_picker_panel(parent: &mut ChildSpawnerCommands, selected_skin: WeaponSkin, owned_skins: &[WeaponSkin], attachment_info: &[(String, String)]) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(360.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(10.0),
            // Center by using negative margin
            margin: UiRect::new(Val::Px(-180.0), Val::Auto, Val::Px(-200.0), Val::Auto),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.98)),
        ColorPickerPanel,
        ZIndex(10),
    )).with_children(|panel| {
        // Header row
        panel.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        }).with_children(|row| {
            row.spawn((
                Text::new("COLOR"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
            ));
            row.spawn((
                Button,
                Node {
                    width: Val::Px(28.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.3, 0.15, 0.15, 0.8)),
                ColorPickerCloseButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("X"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });

        // ── Gun Body Section ──
        panel.spawn((
            Text::new("GUN BODY"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgba(0.5, 0.6, 0.8, 0.9)),
        ));

        panel.spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                ..default()
            },
            SkinPanel,
        )).with_children(|row| {
            for skin in owned_skins {
                let is_active = *skin == selected_skin;
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
                        ..default()
                    },
                    BackgroundColor((*skin).swatch_color()),
                    BorderColor::from(if is_active { Color::WHITE } else { Color::srgba(0.4, 0.4, 0.4, 0.5) }),
                    SkinButton { skin: *skin },
                )).with_children(|btn| {
                    if is_active {
                        btn.spawn((
                            Node {
                                width: Val::Px(10.0),
                                height: Val::Px(10.0),
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                        ));
                    }
                });
            }
        });

        // ── Attachments Section ──
        panel.spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::bottom(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
            BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.5)),
        )).with_children(|btn| {
            btn.spawn((
                Text::new("ATTACHMENTS"),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgba(0.8, 0.8, 0.9, 1.0)),
            ));
        });

        // Separator
        panel.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.4)),
        ));

        // ── Skins Section ──
        panel.spawn((
            Text::new("SKINS"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgba(0.5, 0.6, 0.8, 0.9)),
        ));

        panel.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            margin: UiRect::bottom(Val::Px(8.0)),
            ..default()
        }).with_children(|grid| {
            for skin in owned_skins {
                let is_active = *skin == selected_skin;
                grid.spawn((
                    Button,
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
                        ..default()
                    },
                    BackgroundColor((*skin).swatch_color()),
                    BorderColor::from(if is_active { Color::WHITE } else { Color::srgba(0.4, 0.4, 0.4, 0.5) }),
                    SkinButton { skin: *skin },
                )).with_children(|btn| {
                    if is_active {
                        btn.spawn((
                            Node {
                                width: Val::Px(10.0),
                                height: Val::Px(10.0),
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                        ));
                    }
                });
            }
        });

        // Selected skin label
        panel.spawn((
            Text::new(format!("Selected: {}", selected_skin.display_name())),
            TextFont { font_size: 11.0, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
        ));
    });
}

fn format_loadout_summary(loadout: &PlayerLoadout, registry: &WeaponRegistry) -> String {
    let name = |id: &str| registry.weapons.get(id).map(|c| c.info.name.as_str()).unwrap_or("???");
    format!(
        "P: {} | S: {} | M: {} | E: {}",
        name(&loadout.primary),
        name(&loadout.secondary),
        name(&loadout.melee),
        name(&loadout.equipment),
    )
}

fn handle_loadout_drag(
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut drag_state: ResMut<LoadoutDragState>,
    mut model_query: Query<&mut Transform, With<LoadoutPreviewModel>>,
    mut camera_query: Query<&mut Transform, (With<LoadoutPreviewCamera>, Without<LoadoutPreviewModel>)>,
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    time: Res<Time>,
) {
    let Ok(window) = windows.single() else { return };

    // Left-click drag for rotation
    if mouse_input.just_pressed(MouseButton::Left) {
        if let Some(pos) = window.cursor_position() {
            // Only start drag if cursor is not over the weapon list panel (left 280px)
            if pos.x > 280.0 {
                drag_state.dragging = true;
                drag_state.last_pos = pos;
            }
        }
    }
    if mouse_input.just_released(MouseButton::Left) {
        drag_state.dragging = false;
    }

    if drag_state.dragging {
        if let Some(pos) = window.cursor_position() {
            let delta = pos - drag_state.last_pos;
            drag_state.rotation_y += delta.x * 0.01;
            drag_state.rotation_x = (drag_state.rotation_x + delta.y * 0.01).clamp(-1.2, 1.2);
            drag_state.last_pos = pos;
        }
    } else {
        // Smoothly lerp back to default rotation when not dragging (keep zoom as-is)
        let speed = 4.0 * time.delta_secs();
        drag_state.rotation_y += (0.0 - drag_state.rotation_y) * speed;
        drag_state.rotation_x += (0.0 - drag_state.rotation_x) * speed;
        // Snap to zero when very close
        if drag_state.rotation_y.abs() < 0.001 { drag_state.rotation_y = 0.0; }
        if drag_state.rotation_x.abs() < 0.001 { drag_state.rotation_x = 0.0; }
    }

    // Only zoom when cursor is NOT over the weapon list panel
    let cursor_over_list = if let Some(cursor_pos) = window.cursor_position() {
        // The weapon list is the left 280px panel
        cursor_pos.x < 280.0
    } else {
        false
    };

    if !cursor_over_list {
        for event in scroll_events.read() {
            drag_state.zoom = (drag_state.zoom - event.y * 0.15).clamp(0.5, 5.0);
        }
    }

    // Update model rotation
    for mut transform in model_query.iter_mut() {
        transform.translation = PREVIEW_ORIGIN;
        transform.rotation = Quat::from_rotation_y(drag_state.rotation_y)
            * Quat::from_rotation_x(drag_state.rotation_x);
    }

    // Update camera distance (zoom)
    for mut cam_transform in camera_query.iter_mut() {
        let offset = Vec3::new(0.0, 0.3, drag_state.zoom);
        cam_transform.translation = PREVIEW_ORIGIN + offset;
        cam_transform.look_at(PREVIEW_ORIGIN + Vec3::new(0.0, 0.1, 0.0), Vec3::Y);
    }
}

fn update_loadout_preview_model(
    mut commands: Commands,
    mut ui_state: ResMut<LoadoutUiState>,
    registry: Res<WeaponRegistry>,
    existing_model: Query<Entity, With<LoadoutPreviewModel>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    drag_state: Res<LoadoutDragState>,
) {
    if !ui_state.preview_needs_update {
        return;
    }
    ui_state.preview_needs_update = false;

    // Despawn old model
    for entity in existing_model.iter() {
        commands.entity(entity).despawn();
    }

    // Spawn new preview model
    if let Some(weapon_id) = &ui_state.selected_weapon_id {
        if let Some(config) = registry.weapons.get(weapon_id) {
            let model_file = config.meta.model_path.split('#').next().unwrap_or("");
            let model_exists = !model_file.is_empty()
                && std::path::Path::new(&format!("assets/{}", model_file)).exists();

            let scale = config.meta.scale * 3.0; // Scale up for preview
            let rotation = Quat::from_rotation_y(drag_state.rotation_y);

            if model_exists {
                let skin = ui_state.selected_skin;
                commands.spawn((
                    SceneRoot(asset_server.load(&config.meta.model_path)),
                    Transform::from_translation(PREVIEW_ORIGIN)
                        .with_rotation(rotation)
                        .with_scale(Vec3::splat(scale)),
                    LoadoutPreviewModel,
                    WeaponSkinTag { skin, applied: false },
                ));
            } else {
                let slot = crate::weapons::slot_from_weapon_type(&config.meta.weapon_type);
                let skin = ui_state.selected_skin;
                let mat = if skin != WeaponSkin::Default {
                    materials.add(skin.to_material())
                } else {
                    materials.add(StandardMaterial {
                        base_color: Color::srgb(0.3, 0.3, 0.35),
                        metallic: 0.7,
                        perceptual_roughness: 0.3,
                        ..default()
                    })
                };
                let mesh = match slot {
                    WeaponSlot::Primary => meshes.add(Cuboid::new(0.08 * 3.0, 0.12 * 3.0, 0.6 * 3.0)),
                    WeaponSlot::Secondary => meshes.add(Cuboid::new(0.06 * 3.0, 0.12 * 3.0, 0.3 * 3.0)),
                    WeaponSlot::Melee => meshes.add(Cuboid::new(0.04 * 3.0, 0.04 * 3.0, 0.4 * 3.0)),
                    WeaponSlot::Equipment => meshes.add(Cuboid::new(0.15 * 3.0, 0.15 * 3.0, 0.15 * 3.0)),
                };
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(PREVIEW_ORIGIN)
                        .with_rotation(rotation)
                        .with_scale(Vec3::ONE),
                    LoadoutPreviewModel,
                ));
            }
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Crate Opening System
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CrateType {
    Standard,
    Tactical,
    Elite,
    Legendary,
}

impl CrateType {
    fn all() -> &'static [CrateType] {
        &[CrateType::Standard, CrateType::Tactical, CrateType::Elite, CrateType::Legendary]
    }

    fn display_name(&self) -> &str {
        match self {
            CrateType::Standard => "Standard Crate",
            CrateType::Tactical => "Tactical Crate",
            CrateType::Elite => "Elite Crate",
            CrateType::Legendary => "Legendary Crate",
        }
    }

    fn description(&self) -> &str {
        match self {
            CrateType::Standard => "Basic crate with standard drop rates.",
            CrateType::Tactical => "Better odds for uncommon and rare skins.",
            CrateType::Elite => "Guaranteed rare or better. Higher epic chances.",
            CrateType::Legendary => "Guaranteed epic or better. The best odds.",
        }
    }

    fn cost(&self) -> u64 {
        match self {
            CrateType::Standard => 50,
            CrateType::Tactical => 100,
            CrateType::Elite => 250,
            CrateType::Legendary => 500,
        }
    }

    fn color(&self) -> Color {
        match self {
            CrateType::Standard => Color::srgb(0.3, 0.3, 0.35),
            CrateType::Tactical => Color::srgb(0.2, 0.4, 0.25),
            CrateType::Elite => Color::srgb(0.2, 0.3, 0.6),
            CrateType::Legendary => Color::srgb(0.6, 0.45, 0.1),
        }
    }

    /// Returns modified drop weights for each rarity (values in tenths of a percent for precision)
    fn drop_weights(&self) -> Vec<(SkinRarity, u32)> {
        match self {
            // Mythic 0.1%, Legendary 1%
            CrateType::Standard => vec![
                (SkinRarity::Common, 55),
                (SkinRarity::Uncommon, 250),
                (SkinRarity::Rare, 130),
                (SkinRarity::Epic, 60),
                (SkinRarity::Legendary, 9),
                (SkinRarity::Mythic, 1),
            ],
            // Mythic 0.5%, Legendary 2%
            CrateType::Tactical => vec![
                (SkinRarity::Common, 325),
                (SkinRarity::Uncommon, 350),
                (SkinRarity::Rare, 200),
                (SkinRarity::Epic, 100),
                (SkinRarity::Legendary, 20),
                (SkinRarity::Mythic, 5),
            ],
            // Mythic 1%, Legendary 4%
            CrateType::Elite => vec![
                (SkinRarity::Common, 0),
                (SkinRarity::Uncommon, 0),
                (SkinRarity::Rare, 500),
                (SkinRarity::Epic, 450),
                (SkinRarity::Legendary, 40),
                (SkinRarity::Mythic, 10),
            ],
            // Mythic 2%, Legendary 13%
            CrateType::Legendary => vec![
                (SkinRarity::Common, 0),
                (SkinRarity::Uncommon, 0),
                (SkinRarity::Rare, 0),
                (SkinRarity::Epic, 850),
                (SkinRarity::Legendary, 130),
                (SkinRarity::Mythic, 20),
            ],
        }
    }

    fn roll_skin(&self) -> WeaponSkin {
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        
        let weights = self.drop_weights();
        let total: u32 = weights.iter().map(|(_, w)| w).sum();
        let roll = seed % total;
        
        let mut cumulative = 0u32;
        let mut selected_rarity = SkinRarity::Common;
        for (rarity, weight) in &weights {
            cumulative += weight;
            if roll < cumulative {
                selected_rarity = *rarity;
                break;
            }
        }
        
        // Pick a random skin of that rarity
        let candidates: Vec<&WeaponSkin> = WeaponSkin::droppable()
            .iter()
            .filter(|s| s.rarity() == selected_rarity)
            .collect();
        
        if candidates.is_empty() {
            // Fallback if no skins of that rarity
            return *WeaponSkin::droppable().first().unwrap_or(&WeaponSkin::SolidRed);
        }
        
        let pick = (seed as usize / 7) % candidates.len();
        *candidates[pick]
    }
}

#[derive(Resource, Default)]
struct CrateState {
    selected_crate: Option<CrateType>,
    opening_animation: f32, // 0.0 = not opening, 0..1 = animating
    result_skin: Option<WeaponSkin>,
    result_weapon: Option<String>,   // Which weapon received the skin
    strip_skins: Vec<WeaponSkin>,  // The full strip of skins to display
    strip_offset: f32,             // Current scroll offset in pixels
    strip_velocity: f32,           // Current scroll speed
    strip_target: f32,             // Target offset where winning skin lands
    strip_phase: CratePhase,
    spin_time: f32,                // Elapsed time since spinning started
    spin_duration: f32,            // Total spin duration
    selected_weapon: Option<String>, // Which weapon gets the skin (user picks)
}

#[derive(Default, Clone, Copy, PartialEq)]
enum CratePhase {
    #[default]
    Idle,
    Spinning,
    Revealing,
}

#[derive(Component)]
struct CrateMenuUi;

#[derive(Component)]
struct CrateSelectButton {
    crate_type: CrateType,
}

#[derive(Component)]
struct CrateBackButton;

#[derive(Component)]
struct CrateResultPanel;

#[derive(Component)]
struct CrateResultDismiss;

#[derive(Component)]
struct CrateStripContainer;

#[derive(Component)]
struct CrateStripInner;

#[derive(Component)]
struct CratePointerMarker;

#[derive(Component)]
struct CrateSkipButton;

#[derive(Component)]
struct SellDuplicatesButton;

#[derive(Component)]
struct CrateWeaponPickerButton {
    weapon_id: String,
}

#[derive(Component)]
struct CrateWeaponPickerSlotTab {
    slot: WeaponSlot,
}

#[derive(Component)]
struct CrateWeaponPickerList;

#[derive(Component)]
struct CrateWeaponClearButton;

#[derive(Component)]
struct CrateWeaponSelectButton;

#[derive(Component)]
struct CrateWeaponPickerOverlay;

#[derive(Resource, Default)]
struct CrateWeaponPickerState {
    active_slot: WeaponSlot,
    picker_open: bool,
}

#[derive(Component)]
struct CreditsDisplay;


fn spawn_crate_menu(mut commands: Commands, mut crate_state: ResMut<CrateState>, credits: Res<PlayerCredits>, inventory: Res<SkinInventory>, registry: Res<WeaponRegistry>, picker_state: Res<CrateWeaponPickerState>) {
    // Preserve selected weapon across re-enters
    let preserved_weapon = crate_state.selected_weapon.clone();
    *crate_state = CrateState::default();
    crate_state.selected_weapon = preserved_weapon;

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(40.0)),
            row_gap: Val::Px(20.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.03, 0.03, 0.06)),
        CrateMenuUi,
    )).with_children(|root| {
        // Header
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        }).with_children(|header| {
            // Left: back + title
            header.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(16.0),
                ..default()
            }).with_children(|left| {
                left.spawn((
                    Button,
                    Node {
                        width: Val::Px(90.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
                    CrateBackButton,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("BACK"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                    ));
                });

                left.spawn((
                    Text::new("CRATES"),
                    TextFont { font_size: 32.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Right: credits + sell duplicates
            header.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            }).with_children(|right| {
                // Credits display
                right.spawn((
                    Text::new(format!("⬡ {} Credits", credits.balance)),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.8, 0.2)),
                    CreditsDisplay,
                ));

                // Sell duplicates button
                let dupes = inventory.total_duplicates();
                if dupes > 0 {
                    right.spawn((
                        Button,
                        Node {
                            height: Val::Px(34.0),
                            padding: UiRect::horizontal(Val::Px(14.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.3, 0.5, 0.2, 0.9)),
                        SellDuplicatesButton,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new(format!("SELL {} DUPLICATES", dupes)),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                }
            });
        });

        // Subtitle
        root.spawn((
            Text::new("Open crates to earn weapon skins. Sell duplicates for credits."),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
        ));

        // Crate cards row
        let has_weapon_selected = crate_state.selected_weapon.is_some();
        let selected_weapon_name = crate_state.selected_weapon.as_ref()
            .and_then(|id| registry.weapons.get(id))
            .map(|c| c.info.name.clone())
            .unwrap_or_default();

        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            justify_content: JustifyContent::Center,
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(16.0),
            ..default()
        }).with_children(|row| {
            for crate_type in CrateType::all() {
                let ct = *crate_type;
                let base_cost = ct.cost();
                let actual_cost = if has_weapon_selected { base_cost * 2 } else { base_cost };
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(16.0)),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.9)),
                    BorderColor::from(ct.color()),
                    CrateSelectButton { crate_type: ct },
                )).with_children(|card| {
                    // Crate icon (using a colored box)
                    card.spawn((
                        Node {
                            width: Val::Px(80.0),
                            height: Val::Px(80.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(ct.color()),
                    )).with_children(|icon| {
                        icon.spawn((
                            Text::new("CRATE"),
                            TextFont { font_size: 20.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    card.spawn((
                        Text::new(ct.display_name()),
                        TextFont { font_size: 16.0, ..default() },
                        TextColor(Color::WHITE),
                    ));

                    card.spawn((
                        Text::new(ct.description()),
                        TextFont { font_size: 11.0, ..default() },
                        TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                    ));

                    // Drop rate preview
                    card.spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(2.0),
                        ..default()
                    }).with_children(|rates| {
                        for (rarity, weight) in ct.drop_weights() {
                            if weight == 0 { continue; }
                            let total: u32 = ct.drop_weights().iter().map(|(_, w)| w).sum();
                            let pct = weight as f32 / total as f32 * 100.0;
                            rates.spawn(Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                ..default()
                            }).with_children(|r| {
                                r.spawn((
                                    Text::new(rarity.display_name()),
                                    TextFont { font_size: 10.0, ..default() },
                                    TextColor(rarity.color()),
                                ));
                                r.spawn((
                                    Text::new(format!("{:.1}%", pct)),
                                    TextFont { font_size: 10.0, ..default() },
                                    TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
                                ));
                            });
                        }
                    });

                    // Weapon selection button on the card
                    card.spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(4.0),
                        ..default()
                    }).with_children(|sel_row| {
                        if has_weapon_selected {
                            // Show selected weapon name + X button
                            sel_row.spawn((
                                Text::new(format!("🎯 {}", selected_weapon_name)),
                                TextFont { font_size: 10.0, ..default() },
                                TextColor(Color::srgb(0.4, 0.8, 1.0)),
                            ));
                            sel_row.spawn((
                                Button,
                                Node {
                                    width: Val::Px(20.0),
                                    height: Val::Px(20.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.6, 0.2, 0.2, 0.8)),
                                CrateWeaponClearButton,
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new("✕"),
                                    TextFont { font_size: 11.0, ..default() },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        } else {
                            // "Selected: None" button to open picker
                            sel_row.spawn((
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(24.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.12, 0.12, 0.18, 0.9)),
                                CrateWeaponSelectButton,
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new("Selected: None"),
                                    TextFont { font_size: 10.0, ..default() },
                                    TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                                ));
                            });
                        }
                    });

                    // Open button with cost (doubled if weapon selected)
                    let cost_text = if has_weapon_selected {
                        format!("⬡ {} CREDITS (2×)", actual_cost)
                    } else {
                        format!("⬡ {} CREDITS", actual_cost)
                    };
                    card.spawn((
                        Text::new(cost_text),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(if credits.balance >= actual_cost { ct.color() } else { Color::srgba(0.5, 0.3, 0.3, 0.7) }),
                    ));
                });
            }
        });

        // Weapon picker overlay (shown when picker_open is true)
        if picker_state.picker_open {
            let active_slot = picker_state.active_slot;
            let mut weapons_in_slot: Vec<(&String, &WeaponConfig)> = registry.weapons.iter()
                .filter(|(_, cfg)| {
                    let wtype = cfg.meta.weapon_type.as_str();
                    crate::weapons::slot_from_weapon_type(wtype) == active_slot
                })
                .collect();
            weapons_in_slot.sort_by(|a, b| a.1.info.name.cmp(&b.1.info.name));

            root.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                CrateWeaponPickerOverlay,
                ZIndex(10),
            )).with_children(|overlay| {
                overlay.spawn(Node {
                    width: Val::Px(600.0),
                    max_height: Val::Px(450.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(20.0)),
                    row_gap: Val::Px(12.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                }).insert(BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.98)))
                  .insert(BorderColor::from(Color::srgba(0.3, 0.3, 0.4, 0.6)))
                  .with_children(|panel| {
                    // Header
                    panel.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    }).with_children(|h| {
                        h.spawn((
                            Text::new("SELECT WEAPON (doubles crate cost)"),
                            TextFont { font_size: 16.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                        // Close button
                        h.spawn((
                            Button,
                            Node {
                                width: Val::Px(30.0),
                                height: Val::Px(30.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 0.8)),
                            CrateWeaponClearButton,
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new("✕"),
                                TextFont { font_size: 14.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });
                    });

                    // Slot tabs
                    panel.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        ..default()
                    }).with_children(|tabs| {
                        for slot in [WeaponSlot::Primary, WeaponSlot::Secondary, WeaponSlot::Melee, WeaponSlot::Equipment] {
                            let is_active = slot == active_slot;
                            tabs.spawn((
                                Button,
                                Node {
                                    padding: UiRect::new(Val::Px(12.0), Val::Px(12.0), Val::Px(6.0), Val::Px(6.0)),
                                    ..default()
                                },
                                BackgroundColor(if is_active {
                                    Color::srgba(0.2, 0.3, 0.5, 0.9)
                                } else {
                                    Color::srgba(0.1, 0.1, 0.15, 0.8)
                                }),
                                CrateWeaponPickerSlotTab { slot },
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new(format!("{}", slot)),
                                    TextFont { font_size: 11.0, ..default() },
                                    TextColor(if is_active { Color::WHITE } else { Color::srgba(0.5, 0.5, 0.6, 0.8) }),
                                ));
                            });
                        }
                    });

                    // Weapon list
                    panel.spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(6.0),
                            row_gap: Val::Px(6.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        CrateWeaponPickerList,
                    )).with_children(|list| {
                        for (wid, cfg) in &weapons_in_slot {
                            let is_selected = crate_state.selected_weapon.as_ref() == Some(*wid);
                            list.spawn((
                                Button,
                                Node {
                                    padding: UiRect::new(Val::Px(10.0), Val::Px(10.0), Val::Px(5.0), Val::Px(5.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(if is_selected {
                                    Color::srgba(0.15, 0.3, 0.5, 0.9)
                                } else {
                                    Color::srgba(0.07, 0.07, 0.1, 0.8)
                                }),
                                BorderColor::from(if is_selected {
                                    Color::srgb(0.4, 0.7, 1.0)
                                } else {
                                    Color::srgba(0.15, 0.15, 0.2, 0.4)
                                }),
                                CrateWeaponPickerButton { weapon_id: (*wid).clone() },
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new(&cfg.info.name),
                                    TextFont { font_size: 11.0, ..default() },
                                    TextColor(if is_selected { Color::WHITE } else { Color::srgba(0.6, 0.6, 0.7, 0.8) }),
                                ));
                            });
                        }
                    });
                });
            });
        }
    });
}

fn despawn_crate_menu(mut commands: Commands, query: Query<Entity, With<CrateMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn crate_interaction(
    mut next_state: ResMut<NextState<GameState>>,
    mut crate_state: ResMut<CrateState>,
    mut commands: Commands,
    mut crate_select_query: Query<(&Interaction, &CrateSelectButton, &mut BackgroundColor), (Changed<Interaction>, With<Button>, Without<CrateBackButton>, Without<CrateResultDismiss>, Without<SellDuplicatesButton>)>,
    mut back_query: Query<(&Interaction, &mut BackgroundColor), (With<CrateBackButton>, With<Button>, Without<CrateSelectButton>, Without<CrateResultDismiss>, Without<SellDuplicatesButton>)>,
    mut dismiss_query: Query<(&Interaction, &mut BackgroundColor), (With<CrateResultDismiss>, With<Button>, Without<CrateBackButton>, Without<CrateSelectButton>, Without<SellDuplicatesButton>)>,
    mut sell_query: Query<(&Interaction, &mut BackgroundColor), (With<SellDuplicatesButton>, With<Button>, Without<CrateBackButton>, Without<CrateSelectButton>, Without<CrateResultDismiss>)>,
    result_panel_query: Query<Entity, With<CrateResultPanel>>,
    crate_menu_query: Query<Entity, With<CrateMenuUi>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut credits: ResMut<PlayerCredits>,
    mut inventory: ResMut<SkinInventory>,
) {
    // Handle Escape key to go back or dismiss result
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if let Some(entity) = result_panel_query.iter().next() {
            commands.entity(entity).despawn();
            crate_state.opening_animation = 0.0;
            crate_state.result_skin = None;
            crate_state.result_weapon = None;
            
            // Re-spawn menu to update credits/duplicates
            for entity in crate_menu_query.iter() {
                commands.entity(entity).despawn();
            }
            // We can't easily call spawn_crate_menu here because of ResMut into Res issues.
            // Just let the state transition handle it or rely on update systems.
            // Actually, we can just transition to MainMenu and back, or just let it be.
            // For now, just transition to MainMenu.
            next_state.set(GameState::MainMenu);
            return;
        } else {
            next_state.set(GameState::MainMenu);
            return;
        }
    }

    // Back button
    for (interaction, mut bg) in back_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    next_state.set(GameState::MainMenu);
                }
                *bg = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.12));
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.1));
            }
            _ => {
                *bg = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06));
            }
        }
    }

    // Sell duplicates button
    for (interaction, mut bg) in sell_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    let sold = inventory.sell_all_duplicates();
                    let total_credits: u64 = sold.iter().map(|(_, _, _, v)| v).sum();
                    credits.balance += total_credits;
                    credits.save();
                    inventory.save();
                    // Re-enter crate menu to refresh UI
                    next_state.set(GameState::CrateOpening);
                }
                *bg = BackgroundColor(Color::srgba(0.4, 0.6, 0.3, 1.0));
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgba(0.35, 0.55, 0.25, 1.0));
            }
            _ => {
                *bg = BackgroundColor(Color::srgba(0.3, 0.5, 0.2, 0.9));
            }
        }
    }

    // Crate selection (starts spinning animation)
    for (interaction, crate_btn, mut bg) in crate_select_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                let base_cost = crate_btn.crate_type.cost();
                let cost = if crate_state.selected_weapon.is_some() { base_cost * 2 } else { base_cost };
                if mouse_input.just_pressed(MouseButton::Left) && crate_state.strip_phase == CratePhase::Idle && credits.balance >= cost {
                    // Deduct credits
                    credits.balance -= cost;
                    credits.save();

                    let skin = crate_btn.crate_type.roll_skin();
                    crate_state.result_skin = Some(skin);
                    crate_state.selected_crate = Some(crate_btn.crate_type);

                    // Generate strip of ~60 random skins with the winner at position 45
                    let mut strip = Vec::new();
                    for _ in 0..60 {
                        strip.push(crate_btn.crate_type.roll_skin());
                    }
                    strip[45] = skin; // Place winner at index 45
                    crate_state.strip_skins = strip;
                    
                    // Each skin cell is 80px wide + 4px gap = 84px
                    // Target offset: center of winning cell at index 45
                    let cell_width = 84.0;
                    crate_state.strip_target = 45.0 * cell_width + 42.0; // Center of cell
                    crate_state.strip_offset = 0.0;
                    crate_state.strip_velocity = 4000.0; // Initial fast scroll speed
                    crate_state.strip_phase = CratePhase::Spinning;
                    crate_state.opening_animation = 0.0;
                    crate_state.spin_time = 0.0;
                    crate_state.spin_duration = 4.5; // Total spin duration in seconds

                    // Spawn the strip overlay
                    if let Some(root) = crate_menu_query.iter().next() {
                        commands.entity(root).with_children(|parent| {
                            // Full-screen overlay
                            parent.spawn((
                                Node {
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(0.0),
                                    top: Val::Px(0.0),
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                                CrateResultPanel,
                                ZIndex(20),
                            )).with_children(|overlay| {
                                // Center pointer/indicator triangle
                                overlay.spawn((
                                    Node {
                                        width: Val::Px(4.0),
                                        height: Val::Px(30.0),
                                        margin: UiRect::bottom(Val::Px(4.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(1.0, 0.85, 0.0)),
                                    CratePointerMarker,
                                ));

                                // Strip container (clips overflow)
                                overlay.spawn((
                                    Node {
                                        width: Val::Px(600.0),
                                        height: Val::Px(90.0),
                                        overflow: Overflow::clip(),
                                        border: UiRect::all(Val::Px(2.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.03, 0.03, 0.06, 0.95)),
                                    BorderColor::from(Color::srgba(0.4, 0.4, 0.5, 0.5)),
                                    CrateStripContainer,
                                )).with_children(|container| {
                                    // Inner scrolling row
                                    container.spawn((
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            column_gap: Val::Px(4.0),
                                            height: Val::Percent(100.0),
                                            align_items: AlignItems::Center,
                                            padding: UiRect::horizontal(Val::Px(300.0)), // Padding so first/last items can center
                                            left: Val::Px(0.0),
                                            ..default()
                                        },
                                        CrateStripInner,
                                    )).with_children(|row| {
                                        for s in &crate_state.strip_skins {
                                            let rarity = s.rarity();
                                            row.spawn((
                                                Node {
                                                    width: Val::Px(80.0),
                                                    min_width: Val::Px(80.0),
                                                    height: Val::Px(80.0),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    border: UiRect::all(Val::Px(2.0)),
                                                    flex_direction: FlexDirection::Column,
                                                    row_gap: Val::Px(2.0),
                                                    ..default()
                                                },
                                                BackgroundColor(s.swatch_color()),
                                                BorderColor::from(rarity.color()),
                                            )).with_children(|cell| {
                                                cell.spawn((
                                                    Text::new(s.display_name()),
                                                    TextFont { font_size: 9.0, ..default() },
                                                    TextColor(Color::WHITE),
                                                ));
                                                cell.spawn((
                                                    Text::new(rarity.display_name()),
                                                    TextFont { font_size: 8.0, ..default() },
                                                    TextColor(rarity.color()),
                                                ));
                                            });
                                        }
                                    });
                                });

                                // Bottom pointer
                                overlay.spawn((
                                    Node {
                                        width: Val::Px(4.0),
                                        height: Val::Px(30.0),
                                        margin: UiRect::top(Val::Px(4.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(1.0, 0.85, 0.0)),
                                ));

                                // Skip button
                                overlay.spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(120.0),
                                        height: Val::Px(36.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        margin: UiRect::top(Val::Px(16.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.8)),
                                    CrateSkipButton,
                                )).with_children(|btn| {
                                    btn.spawn((
                                        Text::new("SKIP ▶▶"),
                                        TextFont { font_size: 13.0, ..default() },
                                        TextColor(Color::srgba(0.8, 0.8, 0.9, 0.9)),
                                    ));
                                });
                            });
                        });
                    }
                }
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgba(0.12, 0.12, 0.18, 0.9));
            }
            _ => {
                *bg = BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.9));
            }
        }
    }

    // Dismiss result - re-enter to refresh credits display
    for (interaction, mut bg) in dismiss_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    crate_state.result_skin = None;
                    crate_state.opening_animation = 0.0;
                    crate_state.strip_phase = CratePhase::Idle;
                    crate_state.strip_skins.clear();
                    for entity in result_panel_query.iter() {
                        commands.entity(entity).despawn();
                    }
                    // Re-enter to refresh credits/dupes display
                    next_state.set(GameState::CrateOpening);
                }
                *bg = BackgroundColor(Color::srgba(0.25, 0.25, 0.4, 0.9));
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgba(0.25, 0.25, 0.4, 0.9));
            }
            _ => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9));
            }
        }
    }
}

fn update_crate_animation(
    mut crate_state: ResMut<CrateState>,
    time: Res<Time>,
    mut strip_query: Query<&mut Node, With<CrateStripInner>>,
    mut commands: Commands,
    result_panel_query: Query<Entity, With<CrateResultPanel>>,
    crate_menu_query: Query<Entity, With<CrateMenuUi>>,
    mut skin_inventory: ResMut<SkinInventory>,
    registry: Res<WeaponRegistry>,
) {
    match crate_state.strip_phase {
        CratePhase::Spinning => {
            let dt = time.delta_secs();
            crate_state.spin_time += dt;
            
            let t = (crate_state.spin_time / crate_state.spin_duration).clamp(0.0, 1.0);
            
            let eased = if t < 0.92 {
                let sub_t = t / 0.92;
                let ease = 1.0 - (1.0 - sub_t).powi(3);
                ease * 1.006
            } else {
                let sub_t = (t - 0.92) / 0.08;
                let settle = 1.006 - 0.006 * sub_t * sub_t;
                settle
            };
            
            crate_state.strip_offset = eased * crate_state.strip_target;
            
            if t >= 1.0 {
                crate_state.strip_offset = crate_state.strip_target;
                crate_state.strip_velocity = 0.0;
                crate_state.strip_phase = CratePhase::Revealing;
                crate_state.opening_animation = 0.0;
            }
            
            for mut node in strip_query.iter_mut() {
                node.left = Val::Px(-crate_state.strip_offset);
            }
        }
        CratePhase::Revealing => {
            crate_state.opening_animation += time.delta_secs();
            if crate_state.opening_animation > 0.8 {
                for entity in result_panel_query.iter() {
                    commands.entity(entity).despawn();
                }
                
                if let (Some(skin), Some(root)) = (crate_state.result_skin, crate_menu_query.iter().next()) {
                    let rarity = skin.rarity();
                    
                    // Use the user-selected weapon, or pick random if none selected
                    let assigned_weapon = if let Some(ref selected) = crate_state.selected_weapon {
                        selected.clone()
                    } else {
                        let all_weapon_ids: Vec<String> = registry.weapons.keys().cloned().collect();
                        if !all_weapon_ids.is_empty() {
                            use std::time::{SystemTime, UNIX_EPOCH};
                            let seed = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .subsec_nanos() as usize;
                            let idx = seed % all_weapon_ids.len();
                            all_weapon_ids[idx].clone()
                        } else {
                            "colt_m4a1".to_string()
                        }
                    };
                    
                    // Add to inventory and save
                    let dup_count = skin_inventory.duplicate_count(&assigned_weapon, &skin);
                    skin_inventory.add_skin(&assigned_weapon, skin);
                    skin_inventory.save();
                    crate_state.result_weapon = Some(assigned_weapon.clone());
                    
                    // Get weapon display name and model path
                    let weapon_config = registry.weapons.get(&assigned_weapon);
                    let weapon_display = weapon_config
                        .map(|c| c.info.name.clone())
                        .unwrap_or_else(|| assigned_weapon.replace('_', " "));
                    let weapon_type = weapon_config
                        .map(|c| c.meta.weapon_type.clone())
                        .unwrap_or_else(|| "Primary".to_string());
                    
                    let dup_text = if dup_count > 0 {
                        format!("(Duplicate #{} - you now have {})", dup_count + 1, dup_count + 1)
                    } else {
                        String::new()
                    };
                    
                    // Determine weapon slot for the placeholder model shape
                    let slot = crate::weapons::slot_from_weapon_type(&weapon_type);
                    
                    commands.entity(root).with_children(|parent| {
                        parent.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                            CrateResultPanel,
                            ZIndex(20),
                        )).with_children(|overlay| {
                            overlay.spawn((
                                Node {
                                    width: Val::Px(400.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(30.0)),
                                    row_gap: Val::Px(16.0),
                                    border: UiRect::all(Val::Px(3.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.98)),
                                BorderColor::from(rarity.color()),
                            )).with_children(|card| {
                                card.spawn((
                                    Text::new(rarity.display_name().to_uppercase()),
                                    TextFont { font_size: 14.0, ..default() },
                                    TextColor(rarity.color()),
                                ));

                                // Gun model representation with skin color
                                let slot_label = match slot {
                                    WeaponSlot::Primary => "🔫",
                                    WeaponSlot::Secondary => "🔫",
                                    WeaponSlot::Melee => "🗡",
                                    WeaponSlot::Equipment => "💣",
                                };
                                card.spawn((
                                    Node {
                                        width: Val::Px(180.0),
                                        height: Val::Px(100.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(3.0)),
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(4.0),
                                        ..default()
                                    },
                                    BackgroundColor(skin.swatch_color()),
                                    BorderColor::from(rarity.color()),
                                )).with_children(|model| {
                                    model.spawn((
                                        Text::new(slot_label),
                                        TextFont { font_size: 36.0, ..default() },
                                        TextColor(Color::WHITE),
                                    ));
                                    model.spawn((
                                        Text::new(&weapon_display),
                                        TextFont { font_size: 14.0, ..default() },
                                        TextColor(Color::WHITE),
                                    ));
                                });

                                card.spawn((
                                    Text::new(skin.display_name()),
                                    TextFont { font_size: 24.0, ..default() },
                                    TextColor(Color::WHITE),
                                ));

                                card.spawn((
                                    Text::new(format!("{} Skin", rarity.display_name())),
                                    TextFont { font_size: 13.0, ..default() },
                                    TextColor(rarity.color()),
                                ));

                                // Show which weapon received the skin
                                card.spawn((
                                    Text::new(format!("for {}", weapon_display)),
                                    TextFont { font_size: 15.0, ..default() },
                                    TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
                                ));

                                if !dup_text.is_empty() {
                                    card.spawn((
                                        Text::new(dup_text.clone()),
                                        TextFont { font_size: 11.0, ..default() },
                                        TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                                    ));
                                }

                                // Dismiss button
                                card.spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(160.0),
                                        height: Val::Px(40.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        margin: UiRect::top(Val::Px(10.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.9)),
                                    CrateResultDismiss,
                                )).with_children(|btn| {
                                    btn.spawn((
                                        Text::new("CONTINUE"),
                                        TextFont { font_size: 14.0, ..default() },
                                        TextColor(Color::WHITE),
                                    ));
                                });
                            });
                        });
                    });
                }
                
                crate_state.strip_phase = CratePhase::Idle;
            }
        }
        CratePhase::Idle => {}
    }
}

/// Handle skip button during crate spinning animation.
fn crate_skip_interaction(
    mut crate_state: ResMut<CrateState>,
    skip_query: Query<&Interaction, (Changed<Interaction>, With<CrateSkipButton>, With<Button>)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    for interaction in skip_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            if crate_state.strip_phase == CratePhase::Spinning {
                // Skip to near the end - set spin_time to almost done
                crate_state.spin_time = crate_state.spin_duration * 0.98;
            }
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Cosmetics Menu
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct CosmeticsMenuUi;

#[derive(Component)]
struct CosmeticsBackButton;

#[derive(Component)]
struct CosmeticsSortButton;

#[derive(Component)]
struct CosmeticsSellButton {
    weapon_id: String,
    skin: WeaponSkin,
}

/// Resource for the sell confirmation dialog state.
#[derive(Resource, Default)]
struct SellConfirmState {
    weapon_id: String,
    skin: WeaponSkin,
    quantity: u32,
    max_quantity: u32,
    sell_price_each: u64,
}

#[derive(Component)]
struct SellConfirmOverlay;

#[derive(Component)]
struct SellConfirmButton;

#[derive(Component)]
struct SellCancelButton;

#[derive(Component)]
struct SellQuantityText;

#[derive(Component)]
struct SellQuantityPlus;

#[derive(Component)]
struct SellQuantityMinus;

#[derive(Component)]
struct SellQuantityMax;

fn spawn_cosmetics_menu(
    mut commands: Commands,
    credits: Res<PlayerCredits>,
    inventory: Res<SkinInventory>,
    registry: Res<WeaponRegistry>,
) {
    // Build flat list of owned skins: (weapon_id, weapon_display_name, skin, count)
    let mut all_skins: Vec<(String, String, WeaponSkin, u32)> = Vec::new();
    for (weapon_id, skins) in &inventory.owned {
        let weapon_name = registry.weapons.get(weapon_id)
            .map(|c| c.info.name.clone())
            .unwrap_or_else(|| weapon_id.replace('_', " "));
        for (skin, count) in skins {
            if *count > 0 && *skin != WeaponSkin::Default {
                all_skins.push((weapon_id.clone(), weapon_name.clone(), *skin, *count));
            }
        }
    }
    // Sort by weapon name, then rarity
    all_skins.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| {
                let ra = a.2.rarity() as u8;
                let rb = b.2.rarity() as u8;
                ra.cmp(&rb)
            })
    });

    // Gather unique weapon names for filter tabs
    let mut weapon_tabs: Vec<(String, String)> = Vec::new(); // (id, display_name)
    for (wid, wname, _, _) in &all_skins {
        if !weapon_tabs.iter().any(|(id, _)| id == wid) {
            weapon_tabs.push((wid.clone(), wname.clone()));
        }
    }

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(40.0)),
            row_gap: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.03, 0.03, 0.06)),
        CosmeticsMenuUi,
    )).with_children(|root| {
        // Header row
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        }).with_children(|header| {
            // Left: back + title
            header.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(16.0),
                ..default()
            }).with_children(|left| {
                left.spawn((
                    Button,
                    Node {
                        width: Val::Px(90.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
                    CosmeticsBackButton,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("BACK"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                    ));
                });

                left.spawn((
                    Text::new("COSMETICS"),
                    TextFont { font_size: 32.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Right: credits
            header.spawn((
                Text::new(format!("⬡ {} Credits", credits.balance)),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.9, 0.8, 0.2)),
            ));
        });

        // Subtitle
        root.spawn((
            Text::new("Browse your skins. Click SELL to trade a skin for credits."),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
        ));

        // Filter tabs row (All + per-weapon)
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(6.0),
            ..default()
        }).with_children(|tabs| {
            // "All" tab
            tabs.spawn((
                Button,
                Node {
                    padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(6.0), Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.25, 0.35, 0.5, 0.9)),
                CosmeticsSortButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("ALL"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Per-weapon tabs
            for (wid, wname) in &weapon_tabs {
                tabs.spawn((
                    Button,
                    Node {
                        padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(6.0), Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.8)),
                    CosmeticsSellButton { weapon_id: format!("__filter__{}", wid), skin: WeaponSkin::Default },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(wname.to_uppercase()),
                        TextFont { font_size: 11.0, ..default() },
                        TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                    ));
                });
            }
        });

        // Skin grid
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(10.0),
            row_gap: Val::Px(10.0),
            justify_content: JustifyContent::Center,
            overflow: Overflow::clip_y(),
            max_height: Val::Percent(70.0),
            ..default()
        }).with_children(|grid| {
            if all_skins.is_empty() {
                grid.spawn((
                    Text::new("No skins owned yet. Open some crates!"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                    Node { margin: UiRect::top(Val::Px(40.0)), ..default() },
                ));
            }

            for (weapon_id, weapon_name, skin, count) in &all_skins {
                let rarity = skin.rarity();
                let sell_price = PlayerCredits::sell_value(rarity);

                grid.spawn(Node {
                    width: Val::Px(180.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                }).with_children(|card| {
                    // Skin swatch
                    card.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(skin.swatch_color()),
                    )).with_children(|swatch| {
                        swatch.spawn((
                            Text::new(skin.display_name()),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    // Rarity label
                    card.spawn((
                        Text::new(rarity.display_name()),
                        TextFont { font_size: 10.0, ..default() },
                        TextColor(rarity.color()),
                    ));

                    // Weapon name
                    card.spawn((
                        Text::new(weapon_name.as_str()),
                        TextFont { font_size: 11.0, ..default() },
                        TextColor(Color::srgba(0.6, 0.6, 0.7, 0.9)),
                    ));

                    // Quantity
                    if *count > 1 {
                        card.spawn((
                            Text::new(format!("Owned: ×{}", count)),
                            TextFont { font_size: 10.0, ..default() },
                            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                        ));
                    }

                    // Sell button
                    card.spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 0.8)),
                        CosmeticsSellButton { weapon_id: weapon_id.clone(), skin: *skin },
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new(format!("SELL ⬡{}", sell_price)),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                }).insert(BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.9)))
                  .insert(BorderColor::from(rarity.color()));
            }
        });
    });
}

fn despawn_cosmetics_menu(mut commands: Commands, query: Query<Entity, With<CosmeticsMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn cosmetics_interaction(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    back_query: Query<&Interaction, (Changed<Interaction>, With<CosmeticsBackButton>, With<Button>)>,
    sell_query: Query<(&Interaction, &CosmeticsSellButton), (Changed<Interaction>, With<Button>, Without<CosmeticsBackButton>)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    inventory: Res<SkinInventory>,
    mut sell_state: ResMut<SellConfirmState>,
    cosmetics_ui: Query<Entity, With<CosmeticsMenuUi>>,
    existing_confirm: Query<Entity, With<SellConfirmOverlay>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        // If confirm dialog is open, close it first
        if !existing_confirm.is_empty() {
            for entity in existing_confirm.iter() {
                commands.entity(entity).despawn();
            }
            return;
        }
        next_state.set(GameState::MainMenu);
        return;
    }

    for interaction in back_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            next_state.set(GameState::MainMenu);
            return;
        }
    }

    // Don't process sell clicks if confirm dialog is already open
    if !existing_confirm.is_empty() {
        return;
    }

    for (interaction, sell_btn) in sell_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            // Skip filter buttons
            if sell_btn.weapon_id.starts_with("__filter__") {
                continue;
            }
            let rarity = sell_btn.skin.rarity();
            let price = PlayerCredits::sell_value(rarity);
            let count = inventory.owned.get(&sell_btn.weapon_id)
                .and_then(|skins| skins.iter().find(|(s, _)| **s == sell_btn.skin).map(|(_, c)| *c))
                .unwrap_or(1);
            
            // Set up the sell confirm state
            sell_state.weapon_id = sell_btn.weapon_id.clone();
            sell_state.skin = sell_btn.skin;
            sell_state.quantity = 1;
            sell_state.max_quantity = count;
            sell_state.sell_price_each = price;
            
            // Spawn confirmation overlay
            if let Some(root) = cosmetics_ui.iter().next() {
                spawn_sell_confirm_dialog(&mut commands, root, &sell_state, &sell_btn.skin);
            }
        }
    }
}

fn spawn_sell_confirm_dialog(
    commands: &mut Commands,
    root: Entity,
    sell_state: &SellConfirmState,
    skin: &WeaponSkin,
) {
    let rarity = skin.rarity();
    let total_price = sell_state.sell_price_each * sell_state.quantity as u64;
    
    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            SellConfirmOverlay,
            ZIndex(20),
        )).with_children(|overlay| {
            overlay.spawn((
                Node {
                    width: Val::Px(380.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    row_gap: Val::Px(14.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.98)),
                BorderColor::from(Color::srgba(0.5, 0.3, 0.3, 0.7)),
            )).with_children(|card| {
                // Title
                card.spawn((
                    Text::new("CONFIRM SELL"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
                
                // Skin preview
                card.spawn((
                    Node {
                        width: Val::Px(80.0),
                        height: Val::Px(80.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(skin.swatch_color()),
                    BorderColor::from(rarity.color()),
                )).with_children(|swatch| {
                    swatch.spawn((
                        Text::new(skin.display_name()),
                        TextFont { font_size: 11.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
                
                // Rarity label
                card.spawn((
                    Text::new(format!("{} Skin", rarity.display_name())),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(rarity.color()),
                ));
                
                // Quantity selector
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                }).with_children(|row| {
                    row.spawn((
                        Text::new("Quantity:"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
                    ));
                    
                    // Minus button
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.3, 0.2, 0.2, 0.9)),
                        SellQuantityMinus,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("−"),
                            TextFont { font_size: 18.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                    
                    // Quantity display
                    row.spawn((
                        Text::new(format!("{}", sell_state.quantity)),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::WHITE),
                        SellQuantityText,
                    ));
                    
                    // Plus button
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.3, 0.2, 0.9)),
                        SellQuantityPlus,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("+"),
                            TextFont { font_size: 18.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                    
                    // Max button
                    row.spawn((
                        Button,
                        Node {
                            padding: UiRect::horizontal(Val::Px(8.0)),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.35, 0.9)),
                        SellQuantityMax,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("MAX"),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
                        ));
                    });
                });
                
                // Owned count
                card.spawn((
                    Text::new(format!("Owned: ×{}", sell_state.max_quantity)),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                ));
                
                // Total price
                card.spawn((
                    Text::new(format!("Total: ⬡ {} Credits", total_price)),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.8, 0.2)),
                ));
                
                // Buttons row
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                }).with_children(|row| {
                    // Cancel button
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(38.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
                        SellCancelButton,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("CANCEL"),
                            TextFont { font_size: 14.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                    
                    // Confirm sell button
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(38.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 0.9)),
                        SellConfirmButton,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("SELL"),
                            TextFont { font_size: 14.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
            });
        });
    });
}

fn sell_confirm_interaction(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut sell_state: ResMut<SellConfirmState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    confirm_query: Query<&Interaction, (Changed<Interaction>, With<SellConfirmButton>, With<Button>)>,
    cancel_query: Query<&Interaction, (Changed<Interaction>, With<SellCancelButton>, With<Button>)>,
    plus_query: Query<&Interaction, (Changed<Interaction>, With<SellQuantityPlus>, With<Button>)>,
    minus_query: Query<&Interaction, (Changed<Interaction>, With<SellQuantityMinus>, With<Button>)>,
    max_query: Query<&Interaction, (Changed<Interaction>, With<SellQuantityMax>, With<Button>)>,
    overlay_query: Query<Entity, With<SellConfirmOverlay>>,
    mut qty_text_query: Query<&mut Text, With<SellQuantityText>>,
    mut credits: ResMut<PlayerCredits>,
    mut inventory: ResMut<SkinInventory>,
) {
    if overlay_query.is_empty() { return; }
    
    let mut quantity_changed = false;
    
    // Minus button
    for interaction in minus_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            if sell_state.quantity > 1 {
                sell_state.quantity -= 1;
                quantity_changed = true;
            }
        }
    }
    
    // Plus button
    for interaction in plus_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            if sell_state.quantity < sell_state.max_quantity {
                sell_state.quantity += 1;
                quantity_changed = true;
            }
        }
    }
    
    // Max button
    for interaction in max_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            sell_state.quantity = sell_state.max_quantity;
            quantity_changed = true;
        }
    }
    
    // Update quantity text display
    if quantity_changed {
        for mut text in qty_text_query.iter_mut() {
            text.0 = format!("{}", sell_state.quantity);
        }
        // Note: total price text is not dynamically updated here for simplicity;
        // we'd need to rebuild the overlay. The user can see the quantity change.
    }
    
    // Cancel button
    for interaction in cancel_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            for entity in overlay_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
    
    // Confirm sell
    for interaction in confirm_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            let qty = sell_state.quantity;
            let price_each = sell_state.sell_price_each;
            let weapon_id = sell_state.weapon_id.clone();
            let skin = sell_state.skin;
            
            // Sell the specified quantity
            for _ in 0..qty {
                inventory.sell_skin(&weapon_id, &skin);
            }
            credits.balance += price_each * qty as u64;
            credits.save();
            inventory.save();
            
            // Close overlay and refresh cosmetics menu
            for entity in overlay_query.iter() {
                commands.entity(entity).despawn();
            }
            next_state.set(GameState::Cosmetics);
        }
    }
}

fn cosmetics_hover(
    mut back_query: Query<(&Interaction, &mut BackgroundColor), (With<CosmeticsBackButton>, With<Button>, Without<CosmeticsSellButton>)>,
    mut sell_query: Query<(&Interaction, &mut BackgroundColor, &CosmeticsSellButton), (With<Button>, Without<CosmeticsBackButton>)>,
) {
    for (interaction, mut bg) in back_query.iter_mut() {
        *bg = match interaction {
            Interaction::Pressed => BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.15)),
            Interaction::Hovered => BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.1)),
            _ => BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
        };
    }

    for (interaction, mut bg, sell_btn) in sell_query.iter_mut() {
        if sell_btn.weapon_id.starts_with("__filter__") {
            // Filter tabs
            *bg = match interaction {
                Interaction::Pressed | Interaction::Hovered => BackgroundColor(Color::srgba(0.15, 0.2, 0.3, 0.9)),
                _ => BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.8)),
            };
        } else {
            // Sell buttons
            *bg = match interaction {
                Interaction::Pressed => BackgroundColor(Color::srgba(0.7, 0.25, 0.25, 1.0)),
                Interaction::Hovered => BackgroundColor(Color::srgba(0.6, 0.25, 0.25, 0.9)),
                _ => BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 0.8)),
            };
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Game Mode Selection
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct GameModeMenuUi;

#[derive(Component)]
struct GameModeCard(GameMode);

#[derive(Component)]
struct GameModeBackButton;

/// Which tab is active in the gamemode select screen.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum GameModeTabButton {
    Standard,
    Ltm,
}

/// Tracks which tab is currently showing.
#[derive(Resource)]
struct ActiveGameModeTab(GameModeTabButton);

impl Default for ActiveGameModeTab {
    fn default() -> Self { Self(GameModeTabButton::Standard) }
}

/// Marker for the grid container that holds the mode cards – used
/// to swap content when the user clicks a different tab.
#[derive(Component)]
struct GameModeGrid;

fn spawn_gamemode_menu(mut commands: Commands, selected_mode: Res<SelectedGameMode>, active_tab: Option<Res<ActiveGameModeTab>>) {
    let tab = active_tab.map(|t| t.0).unwrap_or(GameModeTabButton::Standard);
    commands.insert_resource(ActiveGameModeTab(tab));

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(Val::Px(40.0)),
            row_gap: Val::Px(12.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.03, 0.03, 0.06)),
        GameModeMenuUi,
    )).with_children(|root| {
        // Title
        root.spawn((
            Text::new("SELECT GAME MODE"),
            TextFont { font_size: 32.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
        ));

        // ── Tab bar: Standard | LTM ──
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            margin: UiRect::bottom(Val::Px(8.0)),
            ..default()
        }).with_children(|tab_row| {
            for (label, tab_val) in [("STANDARD", GameModeTabButton::Standard), ("LIMITED TIME", GameModeTabButton::Ltm)] {
                let is_active = tab_val == tab;
                tab_row.spawn((
                    Button,
                    Node {
                        width: Val::Px(160.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::bottom(Val::Px(if is_active { 2.0 } else { 0.0 })),
                        ..default()
                    },
                    BackgroundColor(if is_active {
                        Color::srgba(0.12, 0.15, 0.22, 1.0)
                    } else {
                        Color::srgba(0.06, 0.06, 0.1, 0.8)
                    }),
                    BorderColor::all(if is_active {
                        Color::srgba(0.4, 0.6, 1.0, 0.8)
                    } else {
                        Color::NONE
                    }),
                    tab_val,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(if is_active { Color::WHITE } else { Color::srgba(0.5, 0.5, 0.6, 0.8) }),
                    ));
                });
            }
        });

        // ── Mode card grid (depends on active tab) ──
        let modes: Vec<GameMode> = match tab {
            GameModeTabButton::Standard => {
                let mut v: Vec<GameMode> = GameMode::competitive_modes().to_vec();
                v.push(GameMode::TestingGrounds);
                v
            }
            GameModeTabButton::Ltm => GameMode::ltm_modes().to_vec(),
        };

        // Up to 4 cards per row
        for row_modes in modes.chunks(4) {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                },
                GameModeGrid,
            )).with_children(|row| {
                for &mode in row_modes {
                    let is_selected = mode == selected_mode.mode;
                    let accent = mode.accent_color();

                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(160.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(14.0)),
                            justify_content: JustifyContent::SpaceBetween,
                            border: UiRect::all(Val::Px(if is_selected { 2.0 } else { 1.0 })),
                            ..default()
                        },
                        BackgroundColor(if is_selected {
                            Color::srgba(0.15, 0.2, 0.3, 1.0)
                        } else {
                            Color::srgba(0.08, 0.08, 0.12, 0.9)
                        }),
                        BorderColor::all(if is_selected { accent } else { Color::srgba(0.15, 0.15, 0.2, 0.5) }),
                        GameModeCard(mode),
                    )).with_children(|card| {
                        // Top: short name + full name
                        card.spawn(Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            ..default()
                        }).with_children(|top| {
                            top.spawn((
                                Text::new(mode.short_name()),
                                TextFont { font_size: 24.0, ..default() },
                                TextColor(accent),
                            ));
                            top.spawn((
                                Text::new(mode.display_name()),
                                TextFont { font_size: 13.0, ..default() },
                                TextColor(if is_selected { Color::WHITE } else { Color::srgba(0.7, 0.7, 0.7, 0.9) }),
                            ));
                        });

                        // Description
                        card.spawn((
                            Text::new(mode.description()),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(Color::srgba(0.5, 0.55, 0.6, 0.8)),
                        ));

                        // Bottom: player count + selected indicator
                        card.spawn(Node {
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            ..default()
                        }).with_children(|bottom_row| {
                            bottom_row.spawn((
                                Text::new(mode.player_count()),
                                TextFont { font_size: 10.0, ..default() },
                                TextColor(Color::srgba(0.4, 0.5, 0.6, 0.7)),
                            ));
                            if is_selected {
                                bottom_row.spawn((
                                    Text::new("✓ SELECTED"),
                                    TextFont { font_size: 10.0, ..default() },
                                    TextColor(accent),
                                ));
                            }
                        });
                    });
                }
            });
        }

        // ── Bottom row: BACK button ──
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            margin: UiRect::top(Val::Px(8.0)),
            ..default()
        }).with_children(|bottom| {
            bottom.spawn((
                Button,
                Node {
                    width: Val::Px(140.0),
                    height: Val::Px(42.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.25, 0.12, 0.12)),
                GameModeBackButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("BACK"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });
    });
}

fn despawn_gamemode_menu(mut commands: Commands, query: Query<Entity, With<GameModeMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn gamemode_interaction(
    card_query: Query<(&Interaction, &GameModeCard), (Changed<Interaction>, With<Button>)>,
    back_query: Query<&Interaction, (Changed<Interaction>, With<GameModeBackButton>)>,
    tab_query: Query<(&Interaction, &GameModeTabButton), (Changed<Interaction>, With<Button>, Without<GameModeCard>, Without<GameModeBackButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut selected_mode: ResMut<SelectedGameMode>,
    mut active_tab: ResMut<ActiveGameModeTab>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::MainMenu);
        return;
    }

    // Tab switching – rebuild the menu with the new tab
    for (interaction, &tab_val) in tab_query.iter() {
        if *interaction == Interaction::Pressed && tab_val != active_tab.0 {
            active_tab.0 = tab_val;
            // Re-enter to rebuild the UI
            next_state.set(GameState::GameModeSelect);
            return;
        }
    }

    for (interaction, card) in card_query.iter() {
        if *interaction == Interaction::Pressed {
            selected_mode.mode = card.0;
            // Re-enter the menu to refresh the UI with new selection
            next_state.set(GameState::MainMenu);
        }
    }
    for interaction in back_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::MainMenu);
        }
    }
}

fn gamemode_hover(
    mut card_query: Query<(&Interaction, &mut BackgroundColor, &GameModeCard), With<Button>>,
    mut back_query: Query<(&Interaction, &mut BackgroundColor), (With<GameModeBackButton>, Without<GameModeCard>, Without<GameModeTabButton>)>,
    mut tab_btn_query: Query<(&Interaction, &mut BackgroundColor, &GameModeTabButton), (With<Button>, Without<GameModeCard>, Without<GameModeBackButton>)>,
    selected_mode: Res<SelectedGameMode>,
    active_tab: Res<ActiveGameModeTab>,
) {
    for (interaction, mut bg, card) in card_query.iter_mut() {
        let is_selected = card.0 == selected_mode.mode;
        let base = if is_selected {
            Color::srgba(0.15, 0.2, 0.3, 1.0)
        } else {
            Color::srgba(0.08, 0.08, 0.12, 0.9)
        };
        let hover = Color::srgba(0.2, 0.25, 0.35, 1.0);
        *bg = match interaction {
            Interaction::Hovered | Interaction::Pressed => BackgroundColor(hover),
            _ => BackgroundColor(base),
        };
    }
    for (interaction, mut bg) in back_query.iter_mut() {
        let (base, hover) = (
            Color::srgb(0.25, 0.12, 0.12),
            Color::srgb(0.4, 0.18, 0.18),
        );
        *bg = match interaction {
            Interaction::Hovered | Interaction::Pressed => BackgroundColor(hover),
            _ => BackgroundColor(base),
        };
    }
    for (interaction, mut bg, &tab_val) in tab_btn_query.iter_mut() {
        let is_active = tab_val == active_tab.0;
        let base = if is_active {
            Color::srgba(0.12, 0.15, 0.22, 1.0)
        } else {
            Color::srgba(0.06, 0.06, 0.1, 0.8)
        };
        let hover = Color::srgba(0.15, 0.2, 0.28, 1.0);
        *bg = match interaction {
            Interaction::Hovered | Interaction::Pressed => BackgroundColor(hover),
            _ => BackgroundColor(base),
        };
    }
}

fn update_loadout_tabs(
    ui_state: Res<LoadoutUiState>,
    mut tab_query: Query<(&SlotTabButton, &mut BackgroundColor, &Children)>,
    mut text_query: Query<&mut TextColor>,
) {
    if !ui_state.is_changed() {
        return;
    }

    for (tab, mut bg, children) in tab_query.iter_mut() {
        let is_active = tab.slot == ui_state.active_slot;
        *bg = if is_active {
            BackgroundColor(Color::srgb(0.3, 0.4, 0.6))
        } else {
            BackgroundColor(Color::srgb(0.15, 0.15, 0.2))
        };
        for child in children.iter() {
            if let Ok(mut text_color) = text_query.get_mut(child) {
                text_color.0 = if is_active {
                    Color::WHITE
                } else {
                    Color::srgba(0.7, 0.7, 0.7, 0.8)
                };
            }
        }
    }
}

fn crate_weapon_picker_interaction(
    mut crate_state: ResMut<CrateState>,
    mut picker_state: ResMut<CrateWeaponPickerState>,
    mut next_state: ResMut<NextState<GameState>>,
    tab_query: Query<(&Interaction, &CrateWeaponPickerSlotTab), (Changed<Interaction>, With<Button>)>,
    weapon_btn_query: Query<(&Interaction, &CrateWeaponPickerButton), (Changed<Interaction>, With<Button>, Without<CrateWeaponPickerSlotTab>)>,
    select_btn_query: Query<&Interaction, (Changed<Interaction>, With<CrateWeaponSelectButton>, With<Button>)>,
    clear_btn_query: Query<&Interaction, (Changed<Interaction>, With<CrateWeaponClearButton>, With<Button>)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    // "Selected: None" button opens the picker overlay
    for interaction in select_btn_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            picker_state.picker_open = true;
            next_state.set(GameState::CrateOpening);
        }
    }

    // X/close button: clear selection OR close picker
    for interaction in clear_btn_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            if picker_state.picker_open {
                picker_state.picker_open = false;
            }
            crate_state.selected_weapon = None;
            next_state.set(GameState::CrateOpening);
        }
    }

    // Slot tab clicks
    for (interaction, tab) in tab_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            picker_state.active_slot = tab.slot;
            // Re-enter to refresh the weapon list
            next_state.set(GameState::CrateOpening);
        }
    }

    // Weapon selection clicks
    for (interaction, btn) in weapon_btn_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            crate_state.selected_weapon = Some(btn.weapon_id.clone());
            picker_state.picker_open = false;
            // Re-enter to refresh selection highlights
            next_state.set(GameState::CrateOpening);
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Login / Register Screen
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct LoginScreenUi;

#[derive(Component, Clone)]
enum LoginButton {
    Login,
    Register,
    SwitchToLogin,
    SwitchToRegister,
    Back,
}

#[derive(Component)]
struct LoginTextInput {
    field: LoginField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoginField {
    Email,
    Password,
    Username,
    ConfirmPassword,
}

#[derive(Component)]
struct LoginErrorText;

#[derive(Component)]
struct LoginFieldText(LoginField);

#[derive(Resource)]
struct LoginUiState {
    mode: LoginMode,
    email: String,
    password: String,
    username: String,
    confirm_password: String,
    error_message: Option<String>,
    loading: bool,
    focused_field: Option<LoginField>,
}

impl Default for LoginUiState {
    fn default() -> Self {
        Self {
            mode: LoginMode::Login,
            email: String::new(),
            password: String::new(),
            username: String::new(),
            confirm_password: String::new(),
            error_message: None,
            loading: false,
            focused_field: Some(LoginField::Email),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LoginMode {
    #[default]
    Login,
    Register,
}

fn spawn_login_screen(mut commands: Commands, login_state: Res<LoginUiState>) {
    let is_register = login_state.mode == LoginMode::Register;

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.95)),
        LoginScreenUi,
    )).with_children(|root| {
        // Centered card
        root.spawn((
            Node {
                width: Val::Px(400.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(30.0)),
                row_gap: Val::Px(16.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.95)),
            BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.5)),
        )).with_children(|card| {
            // Title
            card.spawn((
                Text::new(if is_register { "CREATE ACCOUNT" } else { "LOGIN" }),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::WHITE),
                Node { margin: UiRect::bottom(Val::Px(8.0)), ..default() },
            ));

            // Tab buttons row
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            }).with_children(|tabs| {
                // Login tab
                tabs.spawn((
                    Button,
                    Node {
                        width: Val::Percent(50.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if !is_register {
                        Color::srgba(0.2, 0.4, 0.2, 0.8)
                    } else {
                        Color::srgba(0.15, 0.15, 0.2, 0.8)
                    }),
                    LoginButton::SwitchToLogin,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("LOGIN"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(if !is_register {
                            Color::WHITE
                        } else {
                            Color::srgba(0.5, 0.5, 0.5, 0.8)
                        }),
                    ));
                });

                // Register tab
                tabs.spawn((
                    Button,
                    Node {
                        width: Val::Percent(50.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if is_register {
                        Color::srgba(0.2, 0.4, 0.2, 0.8)
                    } else {
                        Color::srgba(0.15, 0.15, 0.2, 0.8)
                    }),
                    LoginButton::SwitchToRegister,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("REGISTER"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(if is_register {
                            Color::WHITE
                        } else {
                            Color::srgba(0.5, 0.5, 0.5, 0.8)
                        }),
                    ));
                });
            });

            // Username field (register only)
            if is_register {
                spawn_login_input_field(card, "USERNAME", LoginField::Username, &login_state.username, false, login_state.focused_field == Some(LoginField::Username));
            }

            // Email field
            spawn_login_input_field(card, "EMAIL", LoginField::Email, &login_state.email, false, login_state.focused_field == Some(LoginField::Email));

            // Password field
            spawn_login_input_field(card, "PASSWORD", LoginField::Password, &login_state.password, true, login_state.focused_field == Some(LoginField::Password));

            // Confirm password (register only)
            if is_register {
                spawn_login_input_field(card, "CONFIRM PASSWORD", LoginField::ConfirmPassword, &login_state.confirm_password, true, login_state.focused_field == Some(LoginField::ConfirmPassword));
            }

            // Submit button
            card.spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(44.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                if is_register { LoginButton::Register } else { LoginButton::Login },
            )).with_children(|btn| {
                btn.spawn((
                    Text::new(if login_state.loading {
                        "LOADING..."
                    } else if is_register {
                        "CREATE ACCOUNT"
                    } else {
                        "LOGIN"
                    }),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Error text area
            card.spawn((
                Text::new(login_state.error_message.as_deref().unwrap_or("")),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.9, 0.2, 0.2)),
                LoginErrorText,
            ));

            // Back button
            card.spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.8)),
                LoginButton::Back,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("BACK"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                ));
            });
        });
    });
}

fn spawn_login_input_field(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    field: LoginField,
    current_value: &str,
    is_password: bool,
    is_focused: bool,
) {
    parent.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        ..default()
    }).with_children(|container| {
        // Label
        container.spawn((
            Text::new(label),
            TextFont { font_size: 11.0, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.9)),
        ));

        // Input box
        let display_text = if is_password {
            "*".repeat(current_value.len())
        } else {
            current_value.to_string()
        };
        let display_with_cursor = if is_focused {
            format!("{}|", display_text)
        } else {
            if display_text.is_empty() {
                " ".to_string()
            } else {
                display_text
            }
        };

        container.spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(36.0),
                padding: UiRect::horizontal(Val::Px(10.0)),
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.9)),
            BorderColor::all(if is_focused {
                Color::srgba(0.3, 0.6, 0.3, 0.8)
            } else {
                Color::srgba(0.25, 0.25, 0.3, 0.6)
            }),
            LoginTextInput { field },
        )).with_children(|input| {
            input.spawn((
                Text::new(display_with_cursor),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgba(0.85, 0.85, 0.85, 0.95)),
                LoginFieldText(field),
            ));
        });
    });
}

fn despawn_login_screen(mut commands: Commands, query: Query<Entity, With<LoginScreenUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn login_interaction(
    interaction_query: Query<(&Interaction, &LoginButton), (Changed<Interaction>, With<Button>)>,
    input_query: Query<(&Interaction, &LoginTextInput), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut login_state: ResMut<LoginUiState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    // Handle input field focus
    for (interaction, text_input) in input_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            login_state.focused_field = Some(text_input.field);
        }
    }

    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            match button {
                LoginButton::Login => {
                    if !login_state.loading {
                        login_state.loading = true;
                        login_state.error_message = None;
                        let base_url = server_config.http_url.clone();
                        let email = login_state.email.clone();
                        let password = login_state.password.clone();
                        spawn_http_request(
                            &rt,
                            &pending,
                            crate::net::http::async_login(base_url, email, password),
                        );
                    }
                }
                LoginButton::Register => {
                    if !login_state.loading {
                        if login_state.password != login_state.confirm_password {
                            login_state.error_message = Some("Passwords do not match".to_string());
                            return;
                        }
                        if login_state.username.is_empty() {
                            login_state.error_message = Some("Username is required".to_string());
                            return;
                        }
                        login_state.loading = true;
                        login_state.error_message = None;
                        let base_url = server_config.http_url.clone();
                        let username = login_state.username.clone();
                        let email = login_state.email.clone();
                        let password = login_state.password.clone();
                        spawn_http_request(
                            &rt,
                            &pending,
                            crate::net::http::async_register(base_url, username, email, password),
                        );
                    }
                }
                LoginButton::SwitchToLogin => {
                    if login_state.mode != LoginMode::Login {
                        login_state.mode = LoginMode::Login;
                        login_state.error_message = None;
                        next_state.set(GameState::Login);
                    }
                }
                LoginButton::SwitchToRegister => {
                    if login_state.mode != LoginMode::Register {
                        login_state.mode = LoginMode::Register;
                        login_state.error_message = None;
                        next_state.set(GameState::Login);
                    }
                }
                LoginButton::Back => {
                    *login_state = LoginUiState::default();
                    next_state.set(GameState::MainMenu);
                }
            }
        }
    }
}

fn login_text_input(
    mut login_state: ResMut<LoginUiState>,
    mut char_events: MessageReader<KeyboardInput>,
    mut next_state: ResMut<NextState<GameState>>,
    mut text_query: Query<(&mut Text, &LoginFieldText)>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    let Some(focused) = login_state.focused_field else {
        char_events.clear();
        return;
    };

    for event in char_events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match event.key_code {
            KeyCode::Tab => {
                // Cycle focus through fields
                let fields = if login_state.mode == LoginMode::Register {
                    vec![LoginField::Username, LoginField::Email, LoginField::Password, LoginField::ConfirmPassword]
                } else {
                    vec![LoginField::Email, LoginField::Password]
                };
                if let Some(idx) = fields.iter().position(|f| *f == focused) {
                    let next = (idx + 1) % fields.len();
                    login_state.focused_field = Some(fields[next]);
                }
            }
            KeyCode::Enter => {
                // Submit
                if !login_state.loading {
                    match login_state.mode {
                        LoginMode::Login => {
                            login_state.loading = true;
                            login_state.error_message = None;
                            let base_url = server_config.http_url.clone();
                            let email = login_state.email.clone();
                            let password = login_state.password.clone();
                            spawn_http_request(
                                &rt,
                                &pending,
                                crate::net::http::async_login(base_url, email, password),
                            );
                        }
                        LoginMode::Register => {
                            if login_state.password != login_state.confirm_password {
                                login_state.error_message = Some("Passwords do not match".to_string());
                                continue;
                            }
                            if login_state.username.is_empty() {
                                login_state.error_message = Some("Username is required".to_string());
                                continue;
                            }
                            login_state.loading = true;
                            login_state.error_message = None;
                            let base_url = server_config.http_url.clone();
                            let username = login_state.username.clone();
                            let email = login_state.email.clone();
                            let password = login_state.password.clone();
                            spawn_http_request(
                                &rt,
                                &pending,
                                crate::net::http::async_register(base_url, username, email, password),
                            );
                        }
                    }
                    // Re-enter to refresh UI
                    next_state.set(GameState::Login);
                }
            }
            KeyCode::Backspace => {
                let field_str = match focused {
                    LoginField::Email => &mut login_state.email,
                    LoginField::Password => &mut login_state.password,
                    LoginField::Username => &mut login_state.username,
                    LoginField::ConfirmPassword => &mut login_state.confirm_password,
                };
                field_str.pop();
            }
            KeyCode::Escape => {
                login_state.focused_field = None;
            }
            _ => {
                // Try to get a character from the logical key text
                if let bevy::input::keyboard::Key::Character(ref ch) = event.logical_key {
                    let field_str = match focused {
                        LoginField::Email => &mut login_state.email,
                        LoginField::Password => &mut login_state.password,
                        LoginField::Username => &mut login_state.username,
                        LoginField::ConfirmPassword => &mut login_state.confirm_password,
                    };
                    field_str.push_str(ch.as_str());
                }
            }
        }
    }

    // Update displayed text
    for (mut text, field_text) in text_query.iter_mut() {
        let (value, is_password) = match field_text.0 {
            LoginField::Email => (&login_state.email, false),
            LoginField::Password => (&login_state.password, true),
            LoginField::Username => (&login_state.username, false),
            LoginField::ConfirmPassword => (&login_state.confirm_password, true),
        };
        let display = if is_password {
            "*".repeat(value.len())
        } else {
            value.clone()
        };
        let is_focused = login_state.focused_field == Some(field_text.0);
        **text = if is_focused {
            format!("{}|", display)
        } else if display.is_empty() {
            " ".to_string()
        } else {
            display
        };
    }
}

fn login_handle_network_events(
    mut events: MessageReader<NetworkEvent>,
    mut login_state: ResMut<LoginUiState>,
    mut conn_state: ResMut<ConnectionState>,
    mut next_state: ResMut<NextState<GameState>>,
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::LoginSuccess { token, user_id, username } => {
                *conn_state = ConnectionState::Connected {
                    token: token.clone(),
                    user_id: *user_id,
                    username: username.clone(),
                };
                login_state.loading = false;
                login_state.error_message = None;

                // Connect TCP for real-time communication.
                let addr = server_config.tcp_addr.clone();
                let t = token.clone();
                let client = tcp_client.clone();
                let rt_clone = TokioRuntime(rt.0.clone());
                let pending_clone = pending.clone();
                rt.0.spawn(async move {
                    match client.connect_and_auth(&addr, &t, &rt_clone, &pending_clone).await {
                        Ok(()) => info!("TCP connected and authenticated"),
                        Err(e) => warn!("TCP connection failed: {e}"),
                    }
                });

                next_state.set(GameState::Profile);
            }
            NetworkEvent::RegisterSuccess { token, user_id, username } => {
                *conn_state = ConnectionState::Connected {
                    token: token.clone(),
                    user_id: *user_id,
                    username: username.clone(),
                };
                login_state.loading = false;
                login_state.error_message = None;

                // Connect TCP for real-time communication.
                let addr = server_config.tcp_addr.clone();
                let t = token.clone();
                let client = tcp_client.clone();
                let rt_clone = TokioRuntime(rt.0.clone());
                let pending_clone = pending.clone();
                rt.0.spawn(async move {
                    match client.connect_and_auth(&addr, &t, &rt_clone, &pending_clone).await {
                        Ok(()) => info!("TCP connected and authenticated"),
                        Err(e) => warn!("TCP connection failed: {e}"),
                    }
                });

                next_state.set(GameState::Profile);
            }
            NetworkEvent::LoginError { message } => {
                login_state.loading = false;
                login_state.error_message = Some(message.clone());
                next_state.set(GameState::Login);
            }
            NetworkEvent::RegisterError { message } => {
                login_state.loading = false;
                login_state.error_message = Some(message.clone());
                next_state.set(GameState::Login);
            }
            NetworkEvent::ConnectionError { message } => {
                login_state.loading = false;
                login_state.error_message = Some(message.clone());
                next_state.set(GameState::Login);
            }
            _ => {}
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Profile Screen
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct ProfileScreenUi;

#[derive(Component, Clone)]
enum ProfileButton {
    Back,
    Logout,
    GoToLogin,
}

#[derive(Component)]
struct ProfileStatText(String);

#[derive(Component)]
struct ProfileUsernameText;

#[derive(Component)]
struct ProfileLevelText;

#[derive(Component)]
struct ProfileXpBar;

#[derive(Component)]
struct ProfileMemberSinceText;

fn spawn_profile_screen(
    mut commands: Commands,
    conn_state: Res<ConnectionState>,
    cached_profile: Res<CachedProfile>,
) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(50.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.95)),
        ProfileScreenUi,
    )).with_children(|root| {
        if !conn_state.is_connected() {
            // Not logged in
            root.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            }).with_children(|center| {
                center.spawn((
                    Text::new("You must be logged in to view your profile"),
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                ));
                center.spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(44.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                    ProfileButton::GoToLogin,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("LOGIN"),
                        TextFont { font_size: 16.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            });
            return;
        }

        // Title
        root.spawn((
            Text::new("PLAYER PROFILE"),
            TextFont { font_size: 32.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(24.0)), ..default() },
        ));

        // Content row: left info + right stats
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(40.0),
            flex_grow: 1.0,
            ..default()
        }).with_children(|content| {
            // Left side - player info
            content.spawn(Node {
                width: Val::Px(300.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            }).with_children(|left| {
                let profile = cached_profile.profile.as_ref();
                let username = profile.map(|p| p.username.as_str()).unwrap_or(
                    conn_state.username().unwrap_or("Unknown")
                );

                // Username
                left.spawn((
                    Text::new(username),
                    TextFont { font_size: 36.0, ..default() },
                    TextColor(Color::srgb(0.4, 0.7, 1.0)),
                    ProfileUsernameText,
                ));

                // Level
                let level = profile.map(|p| p.level).unwrap_or(1);
                let xp = profile.map(|p| p.xp).unwrap_or(0);
                let xp_for_next = level * 1000; // Simple XP formula

                left.spawn((
                    Text::new(format!("Level {}", level)),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::srgba(0.8, 0.8, 0.3, 0.9)),
                    ProfileLevelText,
                ));

                // XP bar
                left.spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(8.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                }).with_children(|bar_bg| {
                    bar_bg.spawn((
                        Node {
                            width: Val::Percent((xp as f32 / xp_for_next.max(1) as f32 * 100.0).min(100.0)),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.7, 0.3)),
                        ProfileXpBar,
                    ));
                });

                left.spawn((
                    Text::new(format!("{} / {} XP", xp, xp_for_next)),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8)),
                ));

                // Member since
                let created = profile.map(|p| p.created_at.as_str()).unwrap_or("--");
                let display_date = if created.len() >= 10 { &created[..10] } else { created };
                left.spawn((
                    Text::new(format!("Member since {}", display_date)),
                    TextFont { font_size: 13.0, ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
                    ProfileMemberSinceText,
                ));
            });

            // Right side - stats grid
            content.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                flex_grow: 1.0,
                ..default()
            }).with_children(|right| {
                let profile = cached_profile.profile.as_ref();

                let kills = profile.map(|p| p.stats.total_kills).unwrap_or(0);
                let deaths = profile.map(|p| p.stats.total_deaths).unwrap_or(0);
                let kd = profile.map(|p| p.stats.kd_ratio()).unwrap_or(0.0);
                let wins = profile.map(|p| p.stats.total_wins).unwrap_or(0);
                let losses = profile.map(|p| p.stats.total_losses).unwrap_or(0);
                let win_rate = profile.map(|p| p.stats.win_rate()).unwrap_or(0.0);
                let matches = profile.map(|p| p.stats.total_matches).unwrap_or(0);
                let playtime_s = profile.map(|p| p.stats.playtime_seconds).unwrap_or(0);
                let currency = profile.map(|p| p.currency).unwrap_or(0);

                // Row 1: Kills / Deaths / K/D
                right.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                }).with_children(|row| {
                    spawn_profile_stat_box(row, "TOTAL KILLS", &kills.to_string(), "total_kills");
                    spawn_profile_stat_box(row, "TOTAL DEATHS", &deaths.to_string(), "total_deaths");
                    spawn_profile_stat_box(row, "K/D RATIO", &format!("{:.2}", kd), "kd_ratio");
                });

                // Row 2: Wins / Losses / Win Rate
                right.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                }).with_children(|row| {
                    spawn_profile_stat_box(row, "TOTAL WINS", &wins.to_string(), "total_wins");
                    spawn_profile_stat_box(row, "TOTAL LOSSES", &losses.to_string(), "total_losses");
                    spawn_profile_stat_box(row, "WIN RATE", &format!("{:.1}%", win_rate), "win_rate");
                });

                // Row 3: Matches / Playtime
                let hours = playtime_s / 3600;
                let minutes = (playtime_s % 3600) / 60;
                let playtime_display = if hours > 0 {
                    format!("{}h {}m", hours, minutes)
                } else {
                    format!("{}m", minutes)
                };

                right.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    ..default()
                }).with_children(|row| {
                    spawn_profile_stat_box(row, "TOTAL MATCHES", &matches.to_string(), "total_matches");
                    spawn_profile_stat_box(row, "PLAYTIME", &playtime_display, "playtime");
                    spawn_profile_stat_box(row, "CURRENCY", &currency.to_string(), "currency");
                });
            });
        });

        // Bottom buttons
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            margin: UiRect::top(Val::Px(20.0)),
            ..default()
        }).with_children(|bottom| {
            // Back button
            bottom.spawn((
                Button,
                Node {
                    width: Val::Px(140.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.8)),
                ProfileButton::Back,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("BACK"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                ));
            });

            // Logout button
            bottom.spawn((
                Button,
                Node {
                    width: Val::Px(140.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.8)),
                ProfileButton::Logout,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("LOGOUT"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.3, 0.3)),
                ));
            });
        });
    });
}

fn spawn_profile_stat_box(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    value: &str,
    stat_key: &str,
) {
    parent.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(14.0)),
            min_width: Val::Px(140.0),
            border: UiRect::all(Val::Px(1.0)),
            flex_grow: 1.0,
            ..default()
        },
        BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.9)),
        BorderColor::all(Color::srgba(0.2, 0.2, 0.3, 0.4)),
    )).with_children(|stat_box| {
        stat_box.spawn((
            Text::new(label),
            TextFont { font_size: 10.0, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
        ));
        stat_box.spawn((
            Text::new(value),
            TextFont { font_size: 22.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::top(Val::Px(6.0)), ..default() },
            ProfileStatText(stat_key.to_string()),
        ));
    });
}

fn despawn_profile_screen(mut commands: Commands, query: Query<Entity, With<ProfileScreenUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn request_profile_data(
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    if let Some(token) = conn_state.token() {
        let base_url = server_config.http_url.clone();
        let token = token.to_string();
        spawn_http_request(
            &rt,
            &pending,
            crate::net::http::async_get_profile(base_url, token),
        );
    }
}

fn profile_interaction(
    interaction_query: Query<(&Interaction, &ProfileButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut conn_state: ResMut<ConnectionState>,
    mut cached_profile: ResMut<CachedProfile>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            match button {
                ProfileButton::Back => {
                    next_state.set(GameState::MainMenu);
                }
                ProfileButton::Logout => {
                    *conn_state = ConnectionState::Disconnected;
                    *cached_profile = CachedProfile::default();
                    next_state.set(GameState::MainMenu);
                }
                ProfileButton::GoToLogin => {
                    next_state.set(GameState::Login);
                }
            }
        }
    }
}

fn profile_update_data(
    mut events: MessageReader<NetworkEvent>,
    mut cached_profile: ResMut<CachedProfile>,
    mut stat_query: Query<(&mut Text, &ProfileStatText)>,
    mut username_query: Query<&mut Text, (With<ProfileUsernameText>, Without<ProfileStatText>, Without<ProfileLevelText>, Without<ProfileMemberSinceText>)>,
    mut level_query: Query<&mut Text, (With<ProfileLevelText>, Without<ProfileStatText>, Without<ProfileUsernameText>, Without<ProfileMemberSinceText>)>,
    mut member_query: Query<&mut Text, (With<ProfileMemberSinceText>, Without<ProfileStatText>, Without<ProfileUsernameText>, Without<ProfileLevelText>)>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::ProfileLoaded { profile } => {
                cached_profile.loaded = true;
                cached_profile.profile = Some(profile.clone());

                // Update username
                for mut text in username_query.iter_mut() {
                    **text = profile.username.clone();
                }

                // Update level
                for mut text in level_query.iter_mut() {
                    **text = format!("Level {}", profile.level);
                }

                // Update member since
                for mut text in member_query.iter_mut() {
                    let display_date = if profile.created_at.len() >= 10 {
                        &profile.created_at[..10]
                    } else {
                        &profile.created_at
                    };
                    **text = format!("Member since {}", display_date);
                }

                // Update stat texts
                for (mut text, stat) in stat_query.iter_mut() {
                    let value = match stat.0.as_str() {
                        "total_kills" => profile.stats.total_kills.to_string(),
                        "total_deaths" => profile.stats.total_deaths.to_string(),
                        "kd_ratio" => format!("{:.2}", profile.stats.kd_ratio()),
                        "total_wins" => profile.stats.total_wins.to_string(),
                        "total_losses" => profile.stats.total_losses.to_string(),
                        "win_rate" => format!("{:.1}%", profile.stats.win_rate()),
                        "total_matches" => profile.stats.total_matches.to_string(),
                        "playtime" => {
                            let h = profile.stats.playtime_seconds / 3600;
                            let m = (profile.stats.playtime_seconds % 3600) / 60;
                            if h > 0 { format!("{}h {}m", h, m) } else { format!("{}m", m) }
                        }
                        "currency" => profile.currency.to_string(),
                        _ => continue,
                    };
                    **text = value;
                }
            }
            NetworkEvent::ProfileError { message } => {
                warn!("Profile load error: {}", message);
            }
            _ => {}
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Friends Screen
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct FriendsScreenUi;

#[derive(Component, Clone)]
enum FriendsButton {
    Back,
    AddFriend,
    AcceptRequest(uuid::Uuid),
    DeclineRequest(uuid::Uuid),
    RemoveFriend(uuid::Uuid),
    GoToLogin,
    InviteToParty(String),
}

#[derive(Component)]
struct FriendAddInput;

#[derive(Component)]
struct FriendsListContainer;

#[derive(Component)]
struct FriendRequestsContainer;

#[derive(Component)]
struct FriendsStatusText;

#[derive(Component)]
struct FriendAddFieldText;

#[derive(Resource)]
struct FriendsUiState {
    add_username: String,
    status_message: Option<String>,
    focused: bool,
}

impl Default for FriendsUiState {
    fn default() -> Self {
        Self {
            add_username: String::new(),
            status_message: None,
            focused: false,
        }
    }
}

fn spawn_friends_screen(
    mut commands: Commands,
    conn_state: Res<ConnectionState>,
    cached_friends: Res<CachedFriends>,
    friends_state: Res<FriendsUiState>,
) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(50.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.95)),
        FriendsScreenUi,
    )).with_children(|root| {
        if !conn_state.is_connected() {
            root.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            }).with_children(|center| {
                center.spawn((
                    Text::new("You must be logged in to manage friends"),
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                ));
                center.spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(44.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                    FriendsButton::GoToLogin,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("LOGIN"),
                        TextFont { font_size: 16.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            });
            return;
        }

        // Title
        root.spawn((
            Text::new("FRIENDS"),
            TextFont { font_size: 32.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));

        // Add friend bar
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(8.0),
            margin: UiRect::bottom(Val::Px(8.0)),
            ..default()
        }).with_children(|bar| {
            // Text input
            let display_text = if friends_state.focused {
                format!("{}|", friends_state.add_username)
            } else if friends_state.add_username.is_empty() {
                "Enter username...".to_string()
            } else {
                friends_state.add_username.clone()
            };

            bar.spawn((
                Button,
                Node {
                    width: Val::Px(280.0),
                    height: Val::Px(36.0),
                    padding: UiRect::horizontal(Val::Px(10.0)),
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.9)),
                BorderColor::all(if friends_state.focused {
                    Color::srgba(0.3, 0.6, 0.3, 0.8)
                } else {
                    Color::srgba(0.25, 0.25, 0.3, 0.6)
                }),
                FriendAddInput,
            )).with_children(|input| {
                input.spawn((
                    Text::new(display_text),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(if friends_state.focused || !friends_state.add_username.is_empty() {
                        Color::srgba(0.85, 0.85, 0.85, 0.95)
                    } else {
                        Color::srgba(0.4, 0.4, 0.4, 0.6)
                    }),
                    FriendAddFieldText,
                ));
            });

            // Add button
            bar.spawn((
                Button,
                Node {
                    width: Val::Px(80.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                FriendsButton::AddFriend,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("ADD"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });

        // Status text
        root.spawn((
            Text::new(friends_state.status_message.as_deref().unwrap_or("")),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgb(0.3, 0.8, 0.3)),
            FriendsStatusText,
            Node { margin: UiRect::bottom(Val::Px(12.0)), ..default() },
        ));

        // Friends list (scrollable)
        root.spawn((
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                flex_grow: 1.0,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            FriendsListContainer,
        )).with_children(|list| {
            if cached_friends.friends.is_empty() && cached_friends.loaded {
                list.spawn((
                    Text::new("No friends yet. Add someone above!"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
                ));
            }

            for friend in &cached_friends.friends {
                list.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                }).with_children(|row| {
                    // Online indicator
                    row.spawn((
                        Node {
                            width: Val::Px(8.0),
                            height: Val::Px(8.0),
                            ..default()
                        },
                        BackgroundColor(if friend.online {
                            Color::srgb(0.2, 0.8, 0.2)
                        } else {
                            Color::srgba(0.4, 0.4, 0.4, 0.5)
                        }),
                    ));

                    // Username + level
                    row.spawn((
                        Text::new(format!("{} (Level {})", friend.username, friend.level)),
                        TextFont { font_size: 15.0, ..default() },
                        TextColor(Color::srgba(0.85, 0.85, 0.85, 0.95)),
                    ));

                    // Spacer
                    row.spawn(Node { flex_grow: 1.0, ..default() });

                    // Invite to Party button
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(28.0),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.3, 0.5, 0.6)),
                        FriendsButton::InviteToParty(friend.username.clone()),
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("P"),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.3, 0.6, 0.9)),
                        ));
                    });

                    // Remove button
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(28.0),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.6)),
                        FriendsButton::RemoveFriend(friend.id),
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("X"),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.9, 0.3, 0.3)),
                        ));
                    });
                });
            }

            // Pending requests section
            let has_requests = !cached_friends.incoming_requests.is_empty() || !cached_friends.outgoing_requests.is_empty();
            if has_requests {
                list.spawn((
                    Text::new("PENDING REQUESTS"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                    Node { margin: UiRect::new(Val::Px(0.0), Val::Px(0.0), Val::Px(16.0), Val::Px(8.0)), ..default() },
                ));
            }

            // Incoming requests
            for req in &cached_friends.incoming_requests {
                list.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(10.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                }).with_children(|row| {
                    row.spawn((
                        Text::new(format!("{} wants to be friends", req.from_username)),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgba(0.8, 0.8, 0.5, 0.9)),
                    ));

                    row.spawn(Node { flex_grow: 1.0, ..default() });

                    // Accept
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(70.0),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.4, 0.1, 0.8)),
                        FriendsButton::AcceptRequest(req.id),
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("Accept"),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.3, 0.9, 0.3)),
                        ));
                    });

                    // Decline
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(70.0),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.4, 0.1, 0.1, 0.8)),
                        FriendsButton::DeclineRequest(req.id),
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("Decline"),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.9, 0.3, 0.3)),
                        ));
                    });
                });
            }

            // Outgoing requests
            for req in &cached_friends.outgoing_requests {
                list.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                }).with_children(|row| {
                    row.spawn((
                        Text::new(format!("Pending: {}...", req.to_username)),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
                    ));
                });
            }
        });

        // Back button
        root.spawn((
            Button,
            Node {
                width: Val::Px(140.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.8)),
            FriendsButton::Back,
        )).with_children(|btn| {
            btn.spawn((
                Text::new("BACK"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
            ));
        });
    });
}

fn despawn_friends_screen(mut commands: Commands, query: Query<Entity, With<FriendsScreenUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn request_friends_data(
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    if let Some(token) = conn_state.token() {
        let base_url = server_config.http_url.clone();
        let token_str = token.to_string();
        spawn_http_request(
            &rt,
            &pending,
            crate::net::http::async_get_friends(base_url.clone(), token_str.clone()),
        );
        spawn_http_request(
            &rt,
            &pending,
            crate::net::http::async_get_friend_requests(base_url, token_str),
        );
    }
}

fn friends_interaction(
    interaction_query: Query<(&Interaction, &FriendsButton), (Changed<Interaction>, With<Button>)>,
    input_query: Query<(&Interaction, &FriendAddInput), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut friends_state: ResMut<FriendsUiState>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    tcp_client: Res<TcpClient>,
    party_state: Res<PartyState>,
) {
    // Handle input focus
    for (interaction, _) in input_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            friends_state.focused = true;
        }
    }

    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            match button {
                FriendsButton::Back => {
                    *friends_state = FriendsUiState::default();
                    next_state.set(GameState::MainMenu);
                }
                FriendsButton::GoToLogin => {
                    next_state.set(GameState::Login);
                }
                FriendsButton::AddFriend => {
                    if !friends_state.add_username.is_empty() {
                        if let Some(token) = conn_state.token() {
                            let base_url = server_config.http_url.clone();
                            let token_str = token.to_string();
                            let target = friends_state.add_username.clone();
                            spawn_http_request(
                                &rt,
                                &pending,
                                crate::net::http::async_send_friend_request(base_url, token_str, target),
                            );
                            friends_state.add_username.clear();
                        }
                    }
                }
                FriendsButton::AcceptRequest(request_id) => {
                    if let Some(token) = conn_state.token() {
                        let base_url = server_config.http_url.clone();
                        let token_str = token.to_string();
                        let id = *request_id;
                        spawn_http_request(
                            &rt,
                            &pending,
                            crate::net::http::async_accept_friend_request(base_url, token_str, id),
                        );
                    }
                }
                FriendsButton::DeclineRequest(request_id) => {
                    if let Some(token) = conn_state.token() {
                        let base_url = server_config.http_url.clone();
                        let token_str = token.to_string();
                        let id = *request_id;
                        spawn_http_request(
                            &rt,
                            &pending,
                            crate::net::http::async_decline_friend_request(base_url, token_str, id),
                        );
                    }
                }
                FriendsButton::RemoveFriend(friend_id) => {
                    if let Some(token) = conn_state.token() {
                        let base_url = server_config.http_url.clone();
                        let token_str = token.to_string();
                        let id = *friend_id;
                        spawn_http_request(
                            &rt,
                            &pending,
                            crate::net::http::async_remove_friend(base_url, token_str, id),
                        );
                    }
                }
                FriendsButton::InviteToParty(username) => {
                    if tcp_client.is_connected() {
                        let msg = noctyrn_shared::protocol::ClientMessage::PartyInvite {
                            username: username.clone(),
                        };
                        let tcp = tcp_client.clone();
                        let rt = rt.0.clone();
                        rt.spawn(async move {
                            if let Err(e) = tcp.send(&msg).await {
                                warn!("Party invite send failed: {e}");
                            }
                        });
                        friends_state.status_message = Some(format!("Invited {} to party!", username));
                    } else {
                        friends_state.status_message = Some("Not connected to server".to_string());
                    }
                }
            }
        }
    }

    // Accept party invite via keyboard shortcut (Y/N popup handled in overlay)
    if party_state.pending_invite.is_some() {
        if mouse_input.just_pressed(MouseButton::Left) {
            // Check if user clicked on accept or decline UI button
        }
    }
}

fn friends_text_input(
    mut friends_state: ResMut<FriendsUiState>,
    mut char_events: MessageReader<KeyboardInput>,
    mut text_query: Query<&mut Text, With<FriendAddFieldText>>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    if !friends_state.focused {
        char_events.clear();
        return;
    }

    for event in char_events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        match event.key_code {
            KeyCode::Escape => {
                friends_state.focused = false;
            }
            KeyCode::Backspace => {
                friends_state.add_username.pop();
            }
            KeyCode::Enter => {
                if !friends_state.add_username.is_empty() {
                    if let Some(token) = conn_state.token() {
                        let base_url = server_config.http_url.clone();
                        let token_str = token.to_string();
                        let target = friends_state.add_username.clone();
                        spawn_http_request(
                            &rt,
                            &pending,
                            crate::net::http::async_send_friend_request(base_url, token_str, target),
                        );
                        friends_state.add_username.clear();
                    }
                }
            }
            _ => {
                if let bevy::input::keyboard::Key::Character(ref ch) = event.logical_key {
                    friends_state.add_username.push_str(ch.as_str());
                }
            }
        }
    }

    // Update displayed text
    for mut text in text_query.iter_mut() {
        if friends_state.focused {
            **text = format!("{}|", friends_state.add_username);
        } else if friends_state.add_username.is_empty() {
            **text = "Enter username...".to_string();
        } else {
            **text = friends_state.add_username.clone();
        }
    }
}

fn friends_update_data(
    mut events: MessageReader<NetworkEvent>,
    mut cached_friends: ResMut<CachedFriends>,
    mut friends_state: ResMut<FriendsUiState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let mut needs_refresh = false;

    for event in events.read() {
        match event {
            NetworkEvent::FriendsLoaded { friends } => {
                cached_friends.loaded = true;
                cached_friends.friends = friends.clone();
                needs_refresh = true;
            }
            NetworkEvent::FriendRequestsLoaded { incoming, outgoing } => {
                cached_friends.incoming_requests = incoming.clone();
                cached_friends.outgoing_requests = outgoing.clone();
                needs_refresh = true;
            }
            NetworkEvent::FriendRequestSent => {
                friends_state.status_message = Some("Friend request sent!".to_string());
                needs_refresh = true;
            }
            NetworkEvent::FriendRequestAccepted => {
                friends_state.status_message = Some("Friend request accepted!".to_string());
                needs_refresh = true;
            }
            NetworkEvent::FriendRequestDeclined => {
                friends_state.status_message = Some("Friend request declined.".to_string());
                needs_refresh = true;
            }
            NetworkEvent::FriendRemoved => {
                friends_state.status_message = Some("Friend removed.".to_string());
                needs_refresh = true;
            }
            NetworkEvent::FriendError { message } => {
                friends_state.status_message = Some(format!("Error: {}", message));
            }
            _ => {}
        }
    }

    if needs_refresh {
        // Re-enter Friends state to rebuild the UI with new data
        next_state.set(GameState::Friends);
    }
}

fn friends_handle_network_events(
    mut events: MessageReader<NetworkEvent>,
    mut friends_state: ResMut<FriendsUiState>,
    mut party_state: ResMut<PartyState>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::ConnectionError { message } => {
                friends_state.status_message = Some(format!("Connection error: {}", message));
            }
            NetworkEvent::PartyError { message } => {
                friends_state.status_message = Some(format!("Party: {message}"));
                party_state.pending_invite = None;
            }
            NetworkEvent::PartyUpdate { .. } => {
                party_state.pending_invite = None;
            }
            _ => {}
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Lobby Screen (Fortnite-style party lobby)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct LobbyScreenUi;

#[derive(Component, Clone)]
enum LobbyButton {
    Play,
    Ready,
    Leave,
    Invite,
    SendInvite,
}

#[derive(Component)]
struct LobbyPlayerList;

#[derive(Component)]
struct LobbyTitleText;

#[derive(Component)]
struct LobbyReadyButtonText;

#[derive(Component)]
struct LobbyPlayButtonText;

#[derive(Component)]
struct LobbyInviteInput;

#[derive(Component)]
struct LobbyInviteInputText;

#[derive(Resource, Default)]
struct LobbyState {
    is_ready: bool,
    lobby_data: Option<noctyrn_shared::lobby::LobbyState>,
}

#[derive(Resource, Default)]
struct LobbyInviteText {
    text: String,
}

fn spawn_lobby_screen(
    mut commands: Commands,
    party_state: Res<PartyState>,
    conn_state: Res<ConnectionState>,
    selected_mode: Res<SelectedGameMode>,
    lobby_state: Res<LobbyState>,
) {
    let user_id = conn_state.user_id().unwrap_or_default();
    let is_leader = party_state.party.as_ref().is_some_and(|p| p.is_leader(user_id));
    let mode_name = selected_mode.mode.display_name();

    // Gather party member entries (from PartyState before lobby, from LobbyState once created)
    let members: Vec<(&str, bool, bool)> = if let Some(ref lobby) = lobby_state.lobby_data {
        lobby.players.iter().map(|p| {
            let is_ldr = party_state.party.as_ref().is_some_and(|party| party.is_leader(p.id));
            (p.username.as_str(), p.ready, is_ldr)
        }).collect()
    } else if let Some(ref party) = party_state.party {
        party.members.iter().map(|m| {
            let is_ldr = party.is_leader(m.id);
            (m.username.as_str(), false, is_ldr)
        }).collect()
    } else {
        vec![]
    };

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.95)),
        LobbyScreenUi,
    )).with_children(|root| {
        // ─── Party member panel (top-right) ───
        root.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                top: Val::Px(24.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                min_width: Val::Px(240.0),
                padding: UiRect::all(Val::Px(14.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.92)),
            BorderColor::all(Color::srgba(0.25, 0.25, 0.35, 0.5)),
        )).with_children(|panel| {
            panel.spawn((
                Text::new("PARTY"),
                TextFont { font_size: 13.0, ..default() },
                TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
            ));

            if members.is_empty() {
                panel.spawn((
                    Text::new("No party members"),
                    TextFont { font_size: 13.0, ..default() },
                    TextColor(Color::srgba(0.4, 0.4, 0.4, 0.6)),
                ));
            }

            for (username, ready, leader) in &members {
                panel.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    padding: UiRect::new(Val::Px(4.0), Val::Px(4.0), Val::Px(3.0), Val::Px(3.0)),
                    ..default()
                }).with_children(|row| {
                    // Ready dot
                    row.spawn((
                        Node {
                            width: Val::Px(8.0),
                            height: Val::Px(8.0),
                            ..default()
                        },
                        BackgroundColor(if *ready {
                            Color::srgb(0.2, 0.8, 0.2)
                        } else {
                            Color::srgba(0.4, 0.4, 0.4, 0.5)
                        }),
                    ));
                    // Username
                    row.spawn((
                        Text::new(*username),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(if *leader {
                            Color::srgb(0.9, 0.7, 0.2)
                        } else {
                            Color::srgba(0.85, 0.85, 0.85, 0.95)
                        }),
                    ));
                    // Leader crown
                    if *leader {
                        row.spawn((
                            Text::new(" 👑"),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgb(0.9, 0.7, 0.2)),
                        ));
                    }
                    // Ready label
                    row.spawn(Node { flex_grow: 1.0, ..default() });
                    row.spawn((
                        Text::new(if *ready { "READY" } else { "NOT READY" }),
                        TextFont { font_size: 11.0, ..default() },
                        TextColor(if *ready {
                            Color::srgb(0.3, 0.9, 0.3)
                        } else {
                            Color::srgba(0.5, 0.5, 0.5, 0.6)
                        }),
                    ));
                });
            }
        });

        // ─── CENTER CONTENT ───
        root.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            LobbyPlayerList,
        )).with_children(|center| {
            // Game mode title
            center.spawn((
                Text::new(mode_name),
                TextFont { font_size: 36.0, ..default() },
                TextColor(selected_mode.mode.accent_color()),
                LobbyTitleText,
            ));

            center.spawn((
                Text::new(if party_state.party.is_some() {
                    "Waiting for party to be ready..."
                } else {
                    "Press PLAY to start matchmaking"
                }),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
            ));
        });

        // ─── BOTTOM BUTTON BAR ───
        root.spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(32.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(16.0),
                align_items: AlignItems::Center,
                ..default()
            },
        )).with_children(|bar| {
            // Leave
            bar.spawn((
                Button,
                Node {
                    width: Val::Px(130.0),
                    height: Val::Px(44.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.8)),
                LobbyButton::Leave,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("LEAVE"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.3, 0.3)),
                ));
            });

            // Invite
            bar.spawn((
                Button,
                Node {
                    width: Val::Px(130.0),
                    height: Val::Px(44.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
                LobbyButton::Invite,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("INVITE"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgba(0.7, 0.7, 0.9, 0.9)),
                ));
            });

            // Play (leader) or Ready (non-leader)
            if is_leader {
                bar.spawn((
                    Button,
                    Node {
                        width: Val::Px(180.0),
                        height: Val::Px(52.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                    LobbyButton::Play,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("PLAY"),
                        TextFont { font_size: 22.0, ..default() },
                        TextColor(Color::WHITE),
                        LobbyPlayButtonText,
                    ));
                });
            } else {
                bar.spawn((
                    Button,
                    Node {
                        width: Val::Px(180.0),
                        height: Val::Px(52.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                    LobbyButton::Ready,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("READY"),
                        TextFont { font_size: 22.0, ..default() },
                        TextColor(Color::WHITE),
                        LobbyReadyButtonText,
                    ));
                });
            }
        });
    });
}

fn despawn_lobby_screen(
    mut commands: Commands,
    query: Query<Entity, With<LobbyScreenUi>>,
    mut lobby_state: Option<ResMut<LobbyState>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    if let Some(ref mut state) = lobby_state {
        state.is_ready = false;
        state.lobby_data = None;
    }
}

/// Convert the local `menu::GameMode` to the shared type.
fn to_shared_gamemode(mode: GameMode) -> noctyrn_shared::GameMode {
    match mode {
        GameMode::FreeForAll => noctyrn_shared::GameMode::FreeForAll,
        GameMode::TeamDeathmatch => noctyrn_shared::GameMode::TeamDeathmatch,
        GameMode::KillConfirmed => noctyrn_shared::GameMode::KillConfirmed,
        GameMode::CaptureTheFlag => noctyrn_shared::GameMode::CaptureTheFlag,
        GameMode::Assassins => noctyrn_shared::GameMode::Assassins,
        GameMode::KingOfTheHill => noctyrn_shared::GameMode::KingOfTheHill,
        GameMode::Hardpoint => noctyrn_shared::GameMode::Hardpoint,
        GameMode::CapturePoint => noctyrn_shared::GameMode::CapturePoint,
        GameMode::TestingGrounds => noctyrn_shared::GameMode::TestingGrounds,
        GameMode::Juggernaut => noctyrn_shared::GameMode::Juggernaut,
        GameMode::HighExplosives => noctyrn_shared::GameMode::HighExplosives,
        GameMode::OneInTheChamber => noctyrn_shared::GameMode::OneInTheChamber,
        GameMode::GunGame => noctyrn_shared::GameMode::GunGame,
        GameMode::Infected => noctyrn_shared::GameMode::Infected,
    }
}

/// On entering the lobby, only the party leader creates the lobby on the server.
/// Only sends PartyCreateLobby if no lobby exists yet (lobby_data is None).
fn lobby_on_enter(
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    party_state: Res<PartyState>,
    selected_mode: Res<SelectedGameMode>,
    conn_state: Res<ConnectionState>,
    lobby_state: Res<LobbyState>,
) {
    if !tcp_client.is_connected() {
        return;
    }

    // If we already have lobby data, the lobby was already created – skip.
    if lobby_state.lobby_data.is_some() {
        return;
    }

    let user_id = match conn_state.user_id() {
        Some(id) => id,
        None => return,
    };

    // Only the party leader sends PartyCreateLobby.
    if let Some(ref party) = party_state.party {
        if party.is_leader(user_id) {
            let mode = to_shared_gamemode(selected_mode.mode);
            let msg = noctyrn_shared::protocol::ClientMessage::PartyCreateLobby {
                game_mode: mode,
            };
            let tcp = tcp_client.clone();
            let rt = rt.0.clone();
            rt.spawn(async move {
                if let Err(e) = tcp.send(&msg).await {
                    warn!("PartyCreateLobby send failed: {e}");
                }
            });
        }
    }
}

fn lobby_interaction(
    interaction_query: Query<(&Interaction, &LobbyButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut lobby_state: Option<ResMut<LobbyState>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    selected_mode: Res<SelectedGameMode>,
    mut invite_text: ResMut<LobbyInviteText>,
    mut text_set: ParamSet<(
        Query<&mut Text, With<LobbyReadyButtonText>>,
        Query<&mut Text, With<LobbyPlayButtonText>>,
    )>,
) {
    let rt = rt.0.clone();
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            match button {
                LobbyButton::Leave => {
                    if tcp_client.is_connected() {
                        let msg = noctyrn_shared::protocol::ClientMessage::PartyLeave;
                        let tcp = tcp_client.clone();
                        let rt = rt.clone();
                        rt.spawn(async move {
                            let _ = tcp.send(&msg).await;
                        });
                    }
                    next_state.set(GameState::MainMenu);
                }
                LobbyButton::Invite => {
                    invite_text.text.clear();
                    next_state.set(GameState::Lobby);
                }
                LobbyButton::Play => {
                    if tcp_client.is_connected() {
                        let mode = to_shared_gamemode(selected_mode.mode);
                        let create_msg = noctyrn_shared::protocol::ClientMessage::PartyCreateLobby {
                            game_mode: mode,
                        };
                        let search_msg = noctyrn_shared::protocol::ClientMessage::PartyStartSearch;
                        let tcp = tcp_client.clone();
                        let rt = rt.clone();
                        rt.spawn(async move {
                            let _ = tcp.send(&create_msg).await;
                            let _ = tcp.send(&search_msg).await;
                        });
                    }
                    next_state.set(GameState::Matchmaking);
                }
                LobbyButton::Ready => {
                    let new_ready = if let Some(ref mut state) = lobby_state {
                        state.is_ready = !state.is_ready;
                        state.is_ready
                    } else {
                        return;
                    };

                    if tcp_client.is_connected() {
                        let msg = noctyrn_shared::protocol::ClientMessage::SetReady {
                            ready: new_ready,
                        };
                        let tcp = tcp_client.clone();
                        let rt = rt.clone();
                        rt.spawn(async move {
                            let _ = tcp.send(&msg).await;
                        });
                    }

                    for mut text in text_set.p0().iter_mut() {
                        **text = if new_ready { "UNREADY".to_string() } else { "READY".to_string() };
                    }
                }
                LobbyButton::SendInvite => {
                    let target = invite_text.text.trim().to_string();
                    if !target.is_empty() && tcp_client.is_connected() {
                        let msg = noctyrn_shared::protocol::ClientMessage::PartyInvite {
                            username: target,
                        };
                        let tcp = tcp_client.clone();
                        let rt = rt.clone();
                        rt.spawn(async move {
                            let _ = tcp.send(&msg).await;
                        });
                        invite_text.text.clear();
                        next_state.set(GameState::Lobby);
                    }
                }
            }
        }
    }
}

fn lobby_update(
    mut events: MessageReader<NetworkEvent>,
    mut commands: Commands,
    player_list_query: Query<Entity, With<LobbyPlayerList>>,
    mut title_query: Query<&mut Text, With<LobbyTitleText>>,
    mut lobby_state: Option<ResMut<LobbyState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::LobbyUpdate { lobby } => {
                for mut text in title_query.iter_mut() {
                    **text = format!("{}", lobby.game_mode.display_name());
                }
                for entity in player_list_query.iter() {
                    commands.entity(entity).despawn();
                }
                if let Ok(list_entity) = player_list_query.single() {
                    commands.entity(list_entity).despawn();
                }
                if let Some(ref mut state) = lobby_state {
                    state.lobby_data = Some(lobby.clone());
                }
                next_state.set(GameState::Lobby);
            }
            NetworkEvent::PartyUpdate { party } => {
                // Party state changed (member joined/left/ready), refresh UI
                next_state.set(GameState::Lobby);
            }
            NetworkEvent::MatchFound { .. } => {
                next_state.set(GameState::Playing);
            }
            _ => {}
        }
    }
}

/// Inline invite input bar in lobby – rendered when invite_text is non-empty after pressing INVITE.
fn lobby_invite_input_system(
    mut commands: Commands,
    player_list_query: Query<Entity, With<LobbyPlayerList>>,
    mut invite_text: ResMut<LobbyInviteText>,
    mut next_state: ResMut<NextState<GameState>>,
    mut char_events: MessageReader<KeyboardInput>,
    invite_input_query: Query<Entity, With<LobbyInviteInput>>,
    mut invite_text_query: Query<&mut Text, With<LobbyInviteInputText>>,
    tcp_client: Res<crate::net::tcp::TcpClient>,
    rt: Res<crate::net::TokioRuntime>,
) {
    // Check if we need to spawn the invite input bar
    let has_focus = !invite_text.text.is_empty() || !invite_input_query.is_empty();

    if !has_focus {
        return;
    }

    // Spawn input bar if not already present
    if invite_input_query.is_empty() {
        // Find the LobbyPlayerList entity and attach the input bar to it
        if let Ok(list_entity) = player_list_query.single() {
            commands.entity(list_entity).with_children(|parent| {
                parent.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        margin: UiRect::top(Val::Px(12.0)),
                        ..default()
                    },
                    LobbyInviteInput,
                )).with_children(|row| {
                    let display = if invite_text.text.is_empty() {
                        "Enter username...".to_string()
                    } else {
                        format!("{}|", invite_text.text)
                    };
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(220.0),
                            height: Val::Px(36.0),
                            padding: UiRect::horizontal(Val::Px(10.0)),
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.9)),
                        BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.6)),
                    )).with_children(|input| {
                        input.spawn((
                            Text::new(display),
                            TextFont { font_size: 14.0, ..default() },
                            TextColor(Color::srgba(0.85, 0.85, 0.85, 0.95)),
                            LobbyInviteInputText,
                        ));
                    });
                    // Send button
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(80.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                        LobbyButton::SendInvite,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("SEND"),
                            TextFont { font_size: 14.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
            });
        }
    }

    // Handle keyboard input
    for event in char_events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match event.key_code {
            KeyCode::Escape => {
                invite_text.text.clear();
                next_state.set(GameState::Lobby);
            }
            KeyCode::Backspace => {
                invite_text.text.pop();
                next_state.set(GameState::Lobby);
            }
            KeyCode::Enter => {
                if !invite_text.text.is_empty() {
                    let target = invite_text.text.trim().to_string();
                    if !target.is_empty() {
                        if tcp_client.is_connected() {
                            let msg = noctyrn_shared::protocol::ClientMessage::PartyInvite {
                                username: target,
                            };
                            let tcp = tcp_client.clone();
                            rt.0.spawn(async move {
                                let _ = tcp.send(&msg).await;
                            });
                        }
                        invite_text.text.clear();
                        next_state.set(GameState::Lobby);
                    }
                }
            }
            _ => {
                if let bevy::input::keyboard::Key::Character(ref ch) = event.logical_key {
                    invite_text.text.push_str(ch.as_str());
                }
            }
        }
    }

    // Update displayed text
    for mut text in invite_text_query.iter_mut() {
        if invite_text.text.is_empty() {
            **text = "Enter username...".to_string();
        } else {
            **text = format!("{}|", invite_text.text);
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Matchmaking Screen
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct MatchmakingScreenUi;

#[derive(Component)]
struct MatchmakingButton {
    action: MatchmakingAction,
}

#[derive(Clone)]
enum MatchmakingAction {
    Cancel,
}

#[derive(Component)]
struct MatchmakingTimerText;

#[derive(Component)]
struct MatchmakingStatusText;

#[derive(Component)]
struct MatchmakingDotsText;

#[derive(Resource, Default)]
struct MatchmakingTimer {
    elapsed: f32,
}

fn spawn_matchmaking_screen(
    mut commands: Commands,
    selected_mode: Res<SelectedGameMode>,
    mut timer: ResMut<MatchmakingTimer>,
) {
    timer.elapsed = 0.0;

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.04, 0.95)),
        MatchmakingScreenUi,
    )).with_children(|root| {
        // Centered card
        root.spawn((
            Node {
                width: Val::Px(420.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(40.0)),
                row_gap: Val::Px(20.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.95)),
            BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.5)),
        )).with_children(|card| {
            // Title with animated dots
            card.spawn((
                Text::new("SEARCHING FOR MATCH..."),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::WHITE),
                MatchmakingDotsText,
            ));

            // Game mode display
            card.spawn((
                Text::new(selected_mode.mode.display_name()),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgba(0.6, 0.8, 0.6, 0.9)),
            ));

            // Timer
            card.spawn((
                Text::new("0:00"),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                MatchmakingTimerText,
            ));

            // Players in queue
            card.spawn((
                Text::new("Players in queue: --"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                MatchmakingStatusText,
            ));

            // Cancel button
            card.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(44.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.8)),
                MatchmakingButton { action: MatchmakingAction::Cancel },
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("CANCEL"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.3, 0.3)),
                ));
            });
        });
    });
}

fn despawn_matchmaking_screen(mut commands: Commands, query: Query<Entity, With<MatchmakingScreenUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn matchmaking_interaction(
    interaction_query: Query<(&Interaction, &MatchmakingButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
) {
    let rt = rt.0.clone();
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            match button.action {
                MatchmakingAction::Cancel => {
                    if tcp_client.is_connected() {
                        let msg = noctyrn_shared::protocol::ClientMessage::CancelMatchmaking;
                        let tcp = tcp_client.clone();
                        let rt = rt.clone();
                        rt.spawn(async move {
                            let _ = tcp.send(&msg).await;
                        });
                    }
                    next_state.set(GameState::MainMenu);
                }
            }
        }
    }
}

fn matchmaking_update(
    time: Res<Time>,
    mut timer: ResMut<MatchmakingTimer>,
    mut timer_text_query: Query<&mut Text, (With<MatchmakingTimerText>, Without<MatchmakingStatusText>, Without<MatchmakingDotsText>)>,
    mut status_text_query: Query<&mut Text, (With<MatchmakingStatusText>, Without<MatchmakingTimerText>, Without<MatchmakingDotsText>)>,
    mut dots_text_query: Query<&mut Text, (With<MatchmakingDotsText>, Without<MatchmakingTimerText>, Without<MatchmakingStatusText>)>,
    mut events: MessageReader<NetworkEvent>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    timer.elapsed += time.delta_secs();

    // Update timer display
    let total_secs = timer.elapsed as u32;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    for mut text in timer_text_query.iter_mut() {
        **text = format!("{}:{:02}", mins, secs);
    }

    // Animate dots
    let dot_count = ((timer.elapsed * 2.0) as usize % 4);
    let dots = ".".repeat(dot_count);
    for mut text in dots_text_query.iter_mut() {
        **text = format!("SEARCHING FOR MATCH{}", dots);
    }

    // Handle network events
    for event in events.read() {
        match event {
            NetworkEvent::MatchmakingUpdate { players_in_queue } => {
                for mut text in status_text_query.iter_mut() {
                    **text = format!("Players in queue: {}", players_in_queue);
                }
            }
            NetworkEvent::MatchFound { lobby_id: _, server_addr: _, udp_port: _ } => {
                next_state.set(GameState::Lobby);
            }
            _ => {}
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Party Invite Overlay
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct PartyInviteOverlay;

fn party_invite_overlay_system(
    mut commands: Commands,
    party_state: Res<PartyState>,
    overlay_query: Query<Entity, With<PartyInviteOverlay>>,
    interaction_query: Query<(&Interaction, &InviteOverlayButton), (Changed<Interaction>, With<Button>)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
) {
    let has_invite = party_state.pending_invite.is_some();
    let overlay_exists = !overlay_query.is_empty();

    if has_invite && !overlay_exists {
        // Spawn the invite popup
        let (party_id, from_username) = party_state.pending_invite.as_ref().unwrap();
        let party_id = *party_id;
        let username = from_username.clone();

        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                PartyInviteOverlay,
            ))
            .with_children(|root| {
                root.spawn((
                    Node {
                        width: Val::Px(360.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(24.0)),
                        row_gap: Val::Px(16.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.95)),
                    BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.5)),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new("PARTY INVITE"),
                        TextFont { font_size: 24.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                    card.spawn((
                        Text::new(format!("{} invited you to a party!", username)),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
                    ));
                    card.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(12.0),
                        ..default()
                    }).with_children(|row| {
                        // Accept
                        row.spawn((
                            Button,
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                            InviteOverlayButton::Accept(party_id),
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new("ACCEPT"),
                                TextFont { font_size: 16.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });
                        // Decline
                        row.spawn((
                            Button,
                            Node {
                                width: Val::Px(120.0),
                                height: Val::Px(40.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.8)),
                            InviteOverlayButton::Decline(party_id),
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new("DECLINE"),
                                TextFont { font_size: 16.0, ..default() },
                                TextColor(Color::srgb(0.9, 0.3, 0.3)),
                            ));
                        });
                    });
                });
            });
    }

    if !has_invite && overlay_exists {
        // Despawn the invite popup
        for entity in overlay_query.iter() {
            commands.entity(entity).despawn();
        }
    }

    // Handle button clicks
    let rt = rt.0.clone();
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            match *button {
                InviteOverlayButton::Accept(party_id) => {
                    let msg = noctyrn_shared::protocol::ClientMessage::PartyAcceptInvite {
                        party_id,
                    };
                    let tcp = tcp_client.clone();
                    let rt = rt.clone();
                    rt.spawn(async move {
                        let _ = tcp.send(&msg).await;
                    });
                }
                InviteOverlayButton::Decline(party_id) => {
                    let msg = noctyrn_shared::protocol::ClientMessage::PartyDeclineInvite {
                        party_id,
                    };
                    let tcp = tcp_client.clone();
                    let rt = rt.clone();
                    rt.spawn(async move {
                        let _ = tcp.send(&msg).await;
                    });
                }
            }
        }
    }
}

#[derive(Component, Clone, Copy)]
enum InviteOverlayButton {
    Accept(uuid::Uuid),
    Decline(uuid::Uuid),
}
