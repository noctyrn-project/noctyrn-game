use bevy::prelude::*;
use bevy::app::AppExit;
use crate::player::GameState;
use crate::weapons::{WeaponRegistry, PlayerLoadout, PlayerCredits};
use crate::net::{ConnectionState, TokioRuntime, NetworkEvent, PartyState, TcpConnection};
use crate::net::tcp::TcpClient;
use crate::menu::{SelectedGameMode, to_shared_gamemode, MenuCamera};

#[derive(Component)]
pub struct MainMenuUi;

#[derive(Component)]
pub enum MainMenuButton {
    Play,
    GameModeSelect,
    Loadout,
    Crates,
    Cosmetics,
    Profile,
    Settings,
    Quit,
}

#[derive(Component)]
struct MainMenuCreditsText;

#[derive(Component)]
pub struct MainMenuSceneEntity;

#[derive(Component)]
pub struct MainMenuPill;

#[derive(Component)]
pub struct ServerDisconnectedNotif;

const MENU_SCENE_ORIGIN: Vec3 = Vec3::new(200.0, 200.0, 200.0);

#[derive(Resource, Default)]
pub struct MatchmakingTimer {
    pub elapsed: f32,
    pub searching: bool,
    pub players_in_queue: u32,
}

#[derive(Component)]
pub struct MatchmakingNotifierUi;

#[derive(Component)]
pub struct GameModeSelectUi;

pub fn setup_main_menu_scene(
    mut commands: Commands,
    existing_menu_cam: Query<Entity, With<MenuCamera>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    loadout: Res<PlayerLoadout>,
    registry: Res<WeaponRegistry>,
    asset_server: Res<AssetServer>,
) {
    for entity in existing_menu_cam.iter() {
        commands.entity(entity).despawn();
    }

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

pub fn cleanup_main_menu_scene(
    mut commands: Commands,
    query: Query<Entity, With<MainMenuSceneEntity>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn rotate_main_menu_pill(
    mut query: Query<&mut Transform, With<MainMenuPill>>,
) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::IDENTITY;
    }
}

pub fn spawn_main_menu(mut commands: Commands, selected_mode: Res<SelectedGameMode>, credits: Res<PlayerCredits>) {
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

            top_row.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            }).with_children(|right_top| {
                right_top.spawn((
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

                right_top.spawn((
                    Button,
                    Node {
                        width: Val::Px(100.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.4, 0.3, 0.6, 0.8)),
                    super::friends::OpenFriendsButton,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("FRIENDS"),
                        TextFont { font_size: 13.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            });
        });

        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::End,
            ..default()
        }).with_children(|bottom| {
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

            bottom.spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::End,
                row_gap: Val::Px(8.0),
                ..default()
            }).with_children(|right| {
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
                    GameModeSelectUi,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(format!(">> {}", selected_mode.mode.display_name())),
                        TextFont { font_size: 13.0, ..default() },
                        TextColor(selected_mode.mode.accent_color()),
                    ));
                });

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

        // Matchmaking notifier (hidden by default, shown when searching)
        root.spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(140.0),
                right: Val::Px(50.0),
                width: Val::Px(240.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(14.0)),
                row_gap: Val::Px(6.0),
                border: UiRect::all(Val::Px(1.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.95)),
            BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.5)),
            MatchmakingNotifierUi,
        )).with_children(|notifier| {
            notifier.spawn((
                Text::new("SEARCHING FOR MATCH"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::WHITE),
            ));
            notifier.spawn((
                Text::new("0:00"),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                MatchmakingTimerText,
            ));
            notifier.spawn((
                Text::new("Players in queue: --"),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
            ));
            notifier.spawn((
                Button,
                Node {
                    width: Val::Px(140.0),
                    height: Val::Px(32.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.8)),
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("CANCEL"),
                    TextFont { font_size: 13.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.3, 0.3)),
                ));
            }).insert(CancelSearchButton);
        });
    });
}

pub fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

#[derive(Component)]
pub struct CancelSearchButton;

#[derive(Component)]
pub struct EscapeMenuUi;

pub fn spawn_escape_menu(mut commands: Commands, in_party: bool) {
    commands.spawn((
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
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.3)),
        EscapeMenuUi,
    )).with_children(|wrapper| {
        wrapper.spawn((
            Node {
                width: Val::Px(220.0),
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(4.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.95)),
            BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.5)),
        )).with_children(|menu| {
        for &(label, ref action, enabled, r, g, b) in &[
            ("SETTINGS", EscapeAction::Settings, true, 0.9, 0.9, 0.9),
            ("LEAVE PARTY", EscapeAction::LeaveParty, in_party, 0.7, 0.7, 0.7),
            ("PROFILE", EscapeAction::Profile, true, 0.9, 0.9, 0.9),
            ("EXIT GAME", EscapeAction::Exit, true, 0.9, 0.3, 0.3),
        ] {
            let alpha = if enabled { 1.0 } else { 0.35 };
            menu.spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(34.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.15, alpha * 0.8)),
                EscapeButton { action: *action, enabled },
            )).with_children(|btn| {
                btn.spawn((
                    Text::new(label),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(r, g, b, alpha)),
                ));
            });
        }
    });
    });
}

pub fn despawn_server_notification(mut commands: Commands, query: Query<Entity, With<ServerDisconnectedNotif>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn despawn_escape_menu(mut commands: Commands, query: Query<Entity, With<EscapeMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

#[derive(Component)]
pub struct EscapeButton { pub action: EscapeAction, pub enabled: bool }

#[derive(Clone, Copy)]
pub enum EscapeAction { Settings, LeaveParty, Profile, Exit }

pub fn escape_menu_interaction(
    interaction_query: Query<(&Interaction, &EscapeButton), (Changed<Interaction>, With<Button>)>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    escape_query: Query<Entity, With<EscapeMenuUi>>,
    tcp: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    conn_state: Res<ConnectionState>,
    mut login_state: ResMut<crate::menu::login::LoginUiState>,
    mut profile_state: ResMut<crate::menu::profile::ProfileOverlayState>,
) {
    for (interaction, btn) in interaction_query.iter() {
        if *interaction == Interaction::Pressed && btn.enabled {
            match btn.action {
                EscapeAction::Settings => {
                    crate::ui_settings::spawn_settings_menu(&mut commands);
                    for entity in escape_query.iter() {
                        commands.entity(entity).despawn();
                    }
                }
                EscapeAction::LeaveParty => {
                    let msg = noctyrn_shared::protocol::ClientMessage::PartyLeave;
                    let t = tcp.clone();
                    let r = rt.0.clone();
                    r.spawn(async move { let _ = t.send(&msg).await; });
                }
                EscapeAction::Profile => {
                    if conn_state.is_connected() {
                        profile_state.show = true;
                    } else {
                        login_state.show_overlay = true;
                        login_state.focused_field = Some(crate::menu::login::LoginField::Email);
                    }
                    for entity in escape_query.iter() {
                        commands.entity(entity).despawn();
                    }
                }
                EscapeAction::Exit => { exit.write(AppExit::Success); },
            }
        }
    }
}

pub fn main_menu_interaction(
    interaction_query: Query<(&Interaction, &MainMenuButton), (Changed<Interaction>, With<Button>)>,
    cancel_query: Query<&Interaction, (Changed<Interaction>, With<CancelSearchButton>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>,
    escape_query: Query<Entity, With<EscapeMenuUi>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    party_state: Res<PartyState>,
    friends_state: Res<crate::menu::friends::FriendsUiState>,
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    selected_mode: Res<SelectedGameMode>,
    mut matchmaking_timer: ResMut<MatchmakingTimer>,
    mut login_state: ResMut<crate::menu::login::LoginUiState>,
    mut profile_state: ResMut<crate::menu::profile::ProfileOverlayState>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        // Close settings if open
        if let Some(entity) = settings_query.iter().next() {
            commands.entity(entity).despawn();
            return;
        }
        // Close profile overlay if open
        if profile_state.show {
            profile_state.show = false;
            return;
        }
        // Close login overlay if open
        if login_state.show_overlay {
            login_state.show_overlay = false;
            return;
        }
        // Close friends panel if open
        if friends_state.panel_visible {
            return;
        }
        // Toggle escape menu
        let escape_open = !escape_query.is_empty();
        if escape_open {
            for entity in escape_query.iter() {
                commands.entity(entity).despawn();
            }
        } else {
            spawn_escape_menu(commands.reborrow(), party_state.party.is_some());
        }
    }

    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            match button {
                MainMenuButton::Play => {
                    if party_state.party.is_some() {
                        // Party: start matchmaking immediately
                        if tcp_client.is_connected() {
                            let mode = to_shared_gamemode(selected_mode.mode);
                            let msg = noctyrn_shared::protocol::ClientMessage::QueueForMatch {
                                game_mode: mode,
                            };
                            let tcp = tcp_client.clone();
                            let rt = rt.0.clone();
                            rt.spawn(async move {
                                let _ = tcp.send(&msg).await;
                            });
                        }
                        matchmaking_timer.searching = true;
                        matchmaking_timer.elapsed = 0.0;
                    } else if tcp_client.is_connected() {
                        let msg = noctyrn_shared::protocol::ClientMessage::QueueForMatch {
                            game_mode: to_shared_gamemode(selected_mode.mode),
                        };
                        let tcp = tcp_client.clone();
                        let rt = rt.0.clone();
                        rt.spawn(async move {
                            let _ = tcp.send(&msg).await;
                        });
                        matchmaking_timer.searching = true;
                        matchmaking_timer.elapsed = 0.0;
                    } else {
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
                MainMenuButton::Profile => {}
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

    for interaction in cancel_query.iter() {
        if *interaction == Interaction::Pressed {
            if tcp_client.is_connected() {
                let msg = noctyrn_shared::protocol::ClientMessage::CancelMatchmaking;
                let tcp = tcp_client.clone();
                let rt = rt.0.clone();
                rt.spawn(async move {
                    let _ = tcp.send(&msg).await;
                });
            }
            matchmaking_timer.searching = false;
        }
    }
}

pub fn main_menu_profile_handler(
    interaction_query: Query<(&Interaction, &MainMenuButton), (Changed<Interaction>, With<Button>)>,
    conn_state: Res<ConnectionState>,
    mut login_state: ResMut<crate::menu::login::LoginUiState>,
    mut profile_state: ResMut<crate::menu::profile::ProfileOverlayState>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            if let MainMenuButton::Profile = button {
                if conn_state.is_connected() {
                    profile_state.show = true;
                } else {
                    login_state.show_overlay = true;
                    login_state.focused_field = Some(crate::menu::login::LoginField::Email);
                }
            }
        }
    }
}

pub fn main_menu_hover(
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

pub fn game_mode_selector_visibility(
    timer: Res<MatchmakingTimer>,
    mut query: Query<&mut Node, With<GameModeSelectUi>>,
) {
    for mut node in query.iter_mut() {
        node.display = if timer.searching { Display::None } else { Display::Flex };
    }
}

pub fn matchmaking_notifier_update(
    time: Res<Time>,
    mut timer: ResMut<MatchmakingTimer>,
    mut notifier_query: Query<&mut Node, With<MatchmakingNotifierUi>>,
    mut timer_text_query: Query<&mut Text, With<MatchmakingTimerText>>,
) {
    if !timer.searching {
        for mut node in notifier_query.iter_mut() {
            node.display = Display::None;
        }
        return;
    }

    timer.elapsed += time.delta_secs();

    for mut node in notifier_query.iter_mut() {
        node.display = Display::Flex;
    }

    let total_secs = timer.elapsed as u32;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    for mut text in timer_text_query.iter_mut() {
        **text = format!("{}:{:02}", mins, secs);
    }
}

#[derive(Component)]
pub struct MatchmakingTimerText;

pub fn server_connection_notification(
    mut commands: Commands,
    tcp: Res<TcpConnection>,
    existing: Query<Entity, With<ServerDisconnectedNotif>>,
) {
    let has_notif = !existing.is_empty();
    let is_disconnected = !tcp.connected;
    
    if is_disconnected && !has_notif {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                bottom: Val::Px(16.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.05, 0.05, 0.85)),
            ServerDisconnectedNotif,
        )).with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(8.0),
                    height: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.9, 0.1, 0.1)),
            ));
            root.spawn((
                Text::new("Not connected to server"),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
            ));
        });
    } else if !is_disconnected && has_notif {
        for entity in existing.iter() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn main_menu_matchmaking_handler(
    mut events: MessageReader<NetworkEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    mut timer: ResMut<MatchmakingTimer>,
    udp: Res<crate::net::udp::UdpClient>,
    connection: Res<crate::net::ConnectionState>,
    rt: Res<crate::net::TokioRuntime>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::MatchmakingUpdate { players_in_queue } => {
                timer.players_in_queue = *players_in_queue;
            }
            NetworkEvent::MatchFound { lobby_id, server_addr, udp_port } => {
                info!("Match found! lobby={lobby_id} addr={server_addr}:{udp_port}");
                let addr = format!("{}:{}", server_addr, udp_port);
                let sid = *lobby_id;
                let user_id = connection.user_id().unwrap_or_default();
                let udp_clone = udp.clone();
                rt.0.spawn(async move {
                    let _ = udp_clone.connect(&addr, sid, user_id).await;
                    info!("Connected UDP to {addr}");
                });
                timer.searching = false;
                next_state.set(GameState::Playing);
            }
            _ => {}
        }
    }
}
