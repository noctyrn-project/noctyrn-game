use bevy::prelude::*;
use bevy::app::AppExit;
use rand::Rng;
use crate::theme::*;
use crate::player::GameState;
use crate::weapons::PlayerCredits;
use crate::net::{ConnectionState, ServerConfig, TokioRuntime, NetworkEvent, PartyState, TcpConnection};
use crate::net::tcp::TcpClient;
use crate::branding::{LoadingTarget, PendingLoadingTarget};
use crate::menu::{GameMode, SelectedGameMode, to_shared_gamemode, MenuCamera};
use crate::world::SelectedMapId;

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

/// Marker on the 3D camera that persists across menu states.
#[derive(Component)]
pub struct Menu3dCamera;

/// Flag set by the settings close button (ui_settings.rs) so a system here
/// can trigger the camera revert animation.
#[derive(Resource, Default)]
pub struct PendingCameraRevert(pub bool);

/// Animation state for the menu 3D camera.
#[derive(Resource)]
pub struct MenuCameraAnim {
    pub target_translation: Vec3,
    pub target_rotation: Quat,
    pub start_translation: Vec3,
    pub start_rotation: Quat,
    pub progress: f32,
    pub duration: f32,
    pub active: bool,
}

/// Current camera position and forward direction, updated every frame by
/// `update_menu_camera`.  Other systems (e.g. loadout weapon preview) read
/// this instead of querying `Menu3dCamera` directly to avoid ECS conflicts.
#[derive(Resource, Clone, Copy)]
pub struct CameraPose {
    pub translation: Vec3,
    pub forward: Vec3,
}

impl Default for CameraPose {
    fn default() -> Self {
        Self {
            translation: CAMERA_DEFAULT_TRANSLATION,
            forward: (CAMERA_DEFAULT_LOOK_AT - CAMERA_DEFAULT_TRANSLATION).normalize(),
        }
    }
}

impl Default for MenuCameraAnim {
    fn default() -> Self {
        let default_rot = Transform::from_translation(CAMERA_DEFAULT_TRANSLATION)
            .looking_at(CAMERA_DEFAULT_LOOK_AT, Vec3::Y)
            .rotation;
        Self {
            target_translation: CAMERA_DEFAULT_TRANSLATION,
            target_rotation: default_rot,
            start_translation: CAMERA_DEFAULT_TRANSLATION,
            start_rotation: default_rot,
            progress: 0.0,
            duration: 0.25,
            active: false,
        }
    }
}

// ── Camera target constants (tweak these to match your lobby.glb layout) ──
pub const CAMERA_DEFAULT_TRANSLATION: Vec3 = Vec3::new(0.0, 1.7, 5.0);
pub const CAMERA_DEFAULT_LOOK_AT: Vec3 = Vec3::new(0.0, 1.7, -5.0);
// LOADOUT: pan 90° left — towards the gun rack.
pub const CAMERA_LOADOUT_TRANSLATION: Vec3 = Vec3::new(2.0, 1.7, 4.0);
pub const CAMERA_LOADOUT_LOOK_AT: Vec3 = Vec3::new(-11.0, 1.7, 4.0);
// SETTINGS: pure 180° rotation in place — camera doesn't move, just looks behind.
pub const CAMERA_SETTINGS_TRANSLATION: Vec3 = Vec3::new(0.0, 1.7, 3.0);
pub const CAMERA_SETTINGS_LOOK_AT: Vec3 = Vec3::new(0.0, 1.7, 16.0);
// MATCHMAKING: pan 90° right.
pub const CAMERA_MATCHMAKING_TRANSLATION: Vec3 = Vec3::new(-2.0, 1.7, 4.0);
pub const CAMERA_MATCHMAKING_LOOK_AT: Vec3 = Vec3::new(11.0, 1.7, 4.0);
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
    mut cam_params: ParamSet<(
        Query<Entity, With<Menu3dCamera>>,
        Query<&Transform, With<Menu3dCamera>>,
    )>,
    asset_server: Res<AssetServer>,
    mut anim: ResMut<MenuCameraAnim>,
) {
    for entity in existing_menu_cam.iter() {
        commands.entity(entity).despawn();
    }

    let is_empty = cam_params.p0().iter().next().is_none();

    // Only spawn the 3D camera + scene once — keep it alive across menu states.
    if is_empty {
        // 3D camera at eye level inside the lobby room.
        commands.spawn((
            Camera3d::default(),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::srgb(0.08, 0.08, 0.14)),
                ..default()
            },
            Transform::from_translation(CAMERA_DEFAULT_TRANSLATION)
                .looking_at(CAMERA_DEFAULT_LOOK_AT, Vec3::Y),
            Menu3dCamera,
        ));

        // Warm fill light
        commands.spawn((
            PointLight {
                color: Color::srgb(0.95, 0.92, 1.0),
                intensity: 150_000.0,
                range: 25.0,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(0.0, 3.0, 3.0),
            MainMenuSceneEntity,
        ));

        // Cool rim light
        commands.spawn((
            PointLight {
                color: Color::srgb(0.4, 0.5, 0.85),
                intensity: 60_000.0,
                range: 20.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_xyz(-3.0, 2.0, -1.0),
            MainMenuSceneEntity,
        ));

        // The lobby room model — centered at origin, player inside.
        // Tweak LOBBY_SCALE to resize the room.
        const LOBBY_SCALE: f32 = 0.8;
        commands.spawn((
            WorldAssetRoot(asset_server.load("maps/lobby.glb#Scene0")),
            Transform::from_scale(Vec3::splat(LOBBY_SCALE)),
            Visibility::default(),
            MainMenuSceneEntity,
        ));
    }

    // Animate camera to default position.
    let p1 = cam_params.p1();
    if let Ok(cam) = p1.single() {
        start_camera_anim(&mut anim, cam, CAMERA_DEFAULT_TRANSLATION, CAMERA_DEFAULT_LOOK_AT);
    } else {
        anim.target_translation = CAMERA_DEFAULT_TRANSLATION;
        anim.target_rotation = Transform::from_translation(CAMERA_DEFAULT_TRANSLATION)
            .looking_at(CAMERA_DEFAULT_LOOK_AT, Vec3::Y)
            .rotation;
        anim.progress = 1.0;
        anim.active = false;
    }
}


/// Kick off a smooth camera animation towards a target pose.
pub fn start_camera_anim(
    anim: &mut MenuCameraAnim,
    cam: &Transform,
    target_translation: Vec3,
    target_look_at: Vec3,
) {
    anim.start_translation = cam.translation;
    anim.start_rotation = cam.rotation;
    anim.target_translation = target_translation;
    anim.target_rotation = Transform::from_translation(target_translation)
        .looking_at(target_look_at, Vec3::Y)
        .rotation;
    anim.progress = 0.0;
    anim.active = true;
}

/// Ease-in-out quad: accelerates then decelerates.
fn ease_in_out_quad(t: f32) -> f32 {
    if t < 0.5 { 2.0 * t * t }
    else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 }
}

/// System triggered on `OnEnter(LoadoutSelect)`: animate camera to loadout target.
pub fn setup_loadout_camera(
    mut anim: ResMut<MenuCameraAnim>,
    cam_query: Query<&Transform, With<Menu3dCamera>>,
) {
    if let Ok(cam) = cam_query.single() {
        start_camera_anim(&mut anim, cam, CAMERA_LOADOUT_TRANSLATION, CAMERA_LOADOUT_LOOK_AT);
    }
}

/// System triggered on `OnEnter(Playing)`: clean up the menu 3D scene.
pub fn cleanup_menu_3d(
    mut commands: Commands,
    cam_query: Query<Entity, With<Menu3dCamera>>,
    scene_query: Query<Entity, With<MainMenuSceneEntity>>,
) {
    for entity in cam_query.iter() { commands.entity(entity).despawn(); }
    for entity in scene_query.iter() { commands.entity(entity).despawn(); }
}

/// Animate camera back to default. Usable as an `OnExit` system for sub-menus.
pub fn revert_menu_camera(
    mut anim: ResMut<MenuCameraAnim>,
    cam_query: Query<&Transform, With<Menu3dCamera>>,
) {
    if let Ok(cam) = cam_query.single() {
        start_camera_anim(&mut anim, cam, CAMERA_DEFAULT_TRANSLATION, CAMERA_DEFAULT_LOOK_AT);
    }
}

/// Animate the menu 3D camera towards its target position every frame.
/// Uses quaternion SLERP for smooth rotation (no "through-zero" snapping
/// when interpolating opposite directions like the 180° settings turn).
pub fn update_menu_camera(
    time: Res<Time>,
    mut anim: ResMut<MenuCameraAnim>,
    mut cam_query: Query<&mut Transform, With<Menu3dCamera>>,
    mut pose: ResMut<CameraPose>,
    mut pending: ResMut<PendingCameraRevert>,
) {
    // Process pending camera revert (set by settings X button to avoid param conflicts).
    if pending.0 {
        if let Ok(cam) = cam_query.single() {
            start_camera_anim(&mut anim, cam, CAMERA_DEFAULT_TRANSLATION, CAMERA_DEFAULT_LOOK_AT);
        }
        pending.0 = false;
    }

    let Ok(mut cam) = cam_query.single_mut() else { return; };

    if anim.active {
        anim.progress += time.delta_secs() / anim.duration;
        if anim.progress >= 1.0 {
            anim.progress = 1.0;
            anim.active = false;
        }

        let t = ease_in_out_quad(anim.progress);
        cam.translation = anim.start_translation.lerp(anim.target_translation, t);
        cam.rotation = anim.start_rotation.slerp(anim.target_rotation, t);
    }

    pose.translation = cam.translation;
    pose.forward = cam.forward().as_vec3();
}

/// Freecam mouse look for the menu 3D camera (same feel as in-game).
/// Uses `AccumulatedMouseMotion` for proper relative mouse deltas
/// (no feedback-loop spinning like the old cursor-position approach).
pub fn menu_camera_look(
    accumulated_mouse_motion: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mut cam_query: Query<&mut Transform, With<Menu3dCamera>>,
    debug_settings: Res<crate::player::DebugSettings>,
    settings: Res<crate::settings::GameSettings>,
) {
    if !debug_settings.free_cam { return; }
    let Ok(mut transform) = cam_query.single_mut() else { return };

    let delta = accumulated_mouse_motion.delta;
    if delta == Vec2::ZERO { return; }

    let sensitivity_mult = settings.gameplay.sensitivity;
    let delta_yaw = -delta.x * 0.003 * sensitivity_mult;
    let delta_pitch = -delta.y * 0.002 * sensitivity_mult;

    let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
    let yaw = yaw + delta_yaw;
    let pitch = (pitch + delta_pitch).clamp(-1.55, 1.55);
    transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
}

/// Freecam WASD movement for the menu 3D camera (same feel as in-game).
pub fn menu_freecam_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut cam_query: Query<&mut Transform, With<Menu3dCamera>>,
    debug_settings: Res<crate::player::DebugSettings>,
) {
    if !debug_settings.free_cam { return; }
    let Ok(mut transform) = cam_query.single_mut() else { return };

    let speed = 10.0;
    let mut velocity = Vec3::ZERO;
    let forward = transform.forward().as_vec3();
    let right = transform.right().as_vec3();

    if keyboard_input.pressed(KeyCode::KeyW) { velocity += forward; }
    if keyboard_input.pressed(KeyCode::KeyS) { velocity -= forward; }
    if keyboard_input.pressed(KeyCode::KeyA) { velocity -= right; }
    if keyboard_input.pressed(KeyCode::KeyD) { velocity += right; }
    if keyboard_input.pressed(KeyCode::Space) { velocity += Vec3::Y; }
    if keyboard_input.pressed(KeyCode::ControlLeft) { velocity -= Vec3::Y; }

    if velocity != Vec3::ZERO {
        velocity = velocity.normalize() * speed * time.delta_secs();
        transform.translation += velocity;
    }
}

/// Toggle freecam on/off via the K key (same as in-game).
pub fn menu_freecam_toggle(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_settings: ResMut<crate::settings::GameSettings>,
    mut debug_settings: ResMut<crate::player::DebugSettings>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyK) {
        debug_settings.free_cam = !debug_settings.free_cam;
        game_settings.debug.free_cam = debug_settings.free_cam;
    }
}

pub fn spawn_main_menu(
    mut commands: Commands,
    selected_mode: Res<SelectedGameMode>,
    credits: Res<PlayerCredits>,
    asset_server: Res<AssetServer>,
) {
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
                row_gap: Val::Px(4.0),
                ..default()
            }).with_children(|top| {
                top.spawn((
                    ImageNode::new(asset_server.load("ui/noctyrn.png")),
                    Node {
                        height: Val::Px(104.0),
                        aspect_ratio: Some(1024.0 / 171.0),
                        ..default()
                    },
                ));
                top.spawn((
                    Text::new("TACTICAL SHOOTER"),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
                    Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
                ));

                // Menu buttons below title — text-only, purple gradient
                // from light (top) to dark (bottom).
                for (label, button) in [
                    ("LOADOUT", MainMenuButton::Loadout),
                    ("CRATES", MainMenuButton::Crates),
                    ("COSMETICS", MainMenuButton::Cosmetics),
                    ("PROFILE", MainMenuButton::Profile),
                    ("SETTINGS", MainMenuButton::Settings),
                    ("QUIT", MainMenuButton::Quit),
                ] {
                    let color = menu_button_color(&button);
                    top.spawn((
                        Button,
                        Node {
                            padding: UiRect::new(Val::Px(12.0), Val::Px(20.0), Val::Px(4.0), Val::Px(4.0)),
                            ..default()},
                        button,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new(label),
                            TextFont { font_size: FontSize::Px(18.0), ..default() },
                            TextColor(color),
                        ));
                    });
                }

                top.spawn((
                    Text::new("v0.1.0"),
                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                    TextColor(TEXT_FAINT),
                    Node { margin: UiRect::top(Val::Px(16.0)), ..default() },
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
                            border_radius: RADIUS,
                        ..default()},
                    BackgroundColor(BG_PANEL.with_alpha(0.95)),
                    BorderColor::all(BORDER),
                )).with_children(|credits_box| {
                    credits_box.spawn((
                        Text::new("CREDITS: "),
                        TextFont { font_size: FontSize::Px(16.0), ..default() },
                        TextColor(TEXT_MUTED),
                    ));
                    credits_box.spawn((
                        Text::new(credits.balance.to_string()),
                        TextFont { font_size: FontSize::Px(16.0), ..default() },
                        TextColor(WARNING),
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
                        border: UiRect::all(Val::Px(1.0)),
                            border_radius: RADIUS,
                        ..default()},
                    BackgroundColor(BG_ELEVATED),
                    BorderColor::all(BORDER),
                    super::friends::OpenFriendsButton,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("FRIENDS"),
                        TextFont { font_size: FontSize::Px(13.0), ..default() },
                        TextColor(TEXT),
                    ));
                });
            });
        });

        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::FlexEnd,
            align_items: AlignItems::End,
            ..default()
        }).with_children(|bottom| {
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
                        border: UiRect::all(Val::Px(1.0)),
                            border_radius: RADIUS,
                        ..default()},
                    BackgroundColor(BG_ELEVATED),
                    BorderColor::all(BORDER),
                    MainMenuButton::GameModeSelect,
                    GameModeSelectUi,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(format!(">> {}", selected_mode.mode.display_name())),
                        TextFont { font_size: FontSize::Px(13.0), ..default() },
                        TextColor(TEXT),
                    ));
                });

                right.spawn((
                    Button,
                    Node {
                        width: Val::Px(240.0),
                        height: Val::Px(64.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                            border_radius: RADIUS,
                        ..default()},
                    BackgroundColor(ACCENT),
                    MainMenuButton::Play,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("PLAY"),
                        TextFont { font_size: FontSize::Px(26.0), ..default() },
                        TextColor(TEXT),
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
                    border_radius: RADIUS,
                ..default()},
            BackgroundColor(BG_PANEL.with_alpha(0.95)),
            BorderColor::all(BORDER),
            MatchmakingNotifierUi,
        )).with_children(|notifier| {
            notifier.spawn((
                Text::new("SEARCHING FOR MATCH"),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(TEXT),
            ));
            notifier.spawn((
                Text::new("0:00"),
                TextFont { font_size: FontSize::Px(28.0), ..default() },
                TextColor(TEXT),
                MatchmakingTimerText,
            ));
            notifier.spawn((
                Text::new("Players in queue: --"),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(TEXT_MUTED),
            ));
            notifier.spawn((
                Button,
                Node {
                    width: Val::Px(140.0),
                    height: Val::Px(32.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(6.0)),
                    border: UiRect::all(Val::Px(1.0)),
                        border_radius: RADIUS_SM,
                    ..default()},
                BackgroundColor(BG_ELEVATED),
                BorderColor::all(BORDER),
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("CANCEL"),
                    TextFont { font_size: FontSize::Px(13.0), ..default() },
                    TextColor(DANGER),
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
        BackgroundColor(BG_BASE.with_alpha(0.3)),
        EscapeMenuUi,
    )).with_children(|wrapper| {
        wrapper.spawn((
            Node {
                width: Val::Px(220.0),
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(4.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(1.0)),
                    border_radius: RADIUS,
                ..default()},
            BackgroundColor(BG_PANEL.with_alpha(0.95)),
            BorderColor::all(BORDER),
        )).with_children(|menu| {
        for &(label, ref action, enabled, color) in &[
            ("SETTINGS", EscapeAction::Settings, true, TEXT),
            ("LEAVE PARTY", EscapeAction::LeaveParty, in_party, TEXT_MUTED),
            ("PROFILE", EscapeAction::Profile, true, TEXT),
            ("EXIT GAME", EscapeAction::Exit, true, DANGER),
        ] {
            let alpha = if enabled { 1.0 } else { 0.35 };
            menu.spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(34.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                        border_radius: RADIUS_SM,
                    ..default()},
                BackgroundColor(BG_ELEVATED.with_alpha(alpha * 0.8)),
                BorderColor::all(BORDER),
                EscapeButton { action: *action, enabled },
            )).with_children(|btn| {
                btn.spawn((
                    Text::new(label),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(color.with_alpha(alpha)),
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

/// Handles Escape key: closes modals in priority, toggles escape menu.
/// Chat is checked first (chat has its own Escape handler that closes it).
pub fn handle_escape_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>,
    escape_query: Query<Entity, With<EscapeMenuUi>>,
    friends_state: Res<crate::menu::friends::FriendsUiState>,
    mut login_state: ResMut<crate::menu::login::LoginUiState>,
    mut profile_state: ResMut<crate::menu::profile::ProfileOverlayState>,
    party_state: Res<PartyState>,
    mut chat_input: ResMut<crate::menu::chat::ChatInput>,
    mut chat_open: ResMut<crate::menu::chat::ChatOpen>,
    cam_query: Query<&Transform, With<Menu3dCamera>>,
    mut anim: ResMut<MenuCameraAnim>,
) {
    if !keyboard_input.just_pressed(KeyCode::Escape) {
        return;
    }
    // Close chat if open
    if chat_input.open {
        chat_input.open = false;
        chat_input.input.clear();
        chat_open.0 = false;
        return;
    }
    // Close settings if open
    if let Some(entity) = settings_query.iter().next() {
        commands.entity(entity).despawn();
        // Revert camera to default when closing settings via Escape.
        if let Ok(cam) = cam_query.single() {
            start_camera_anim(&mut anim, cam, CAMERA_DEFAULT_TRANSLATION, CAMERA_DEFAULT_LOOK_AT);
        }
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

pub fn main_menu_interaction(
    interaction_query: Query<(&Interaction, &MainMenuButton), (Changed<Interaction>, With<Button>)>,
    cancel_query: Query<&Interaction, (Changed<Interaction>, With<CancelSearchButton>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>,
    party_state: Res<PartyState>,
    friends_state: Res<crate::menu::friends::FriendsUiState>,
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    selected_mode: Res<SelectedGameMode>,
    mut matchmaking_timer: ResMut<MatchmakingTimer>,
    cam_query: Query<&Transform, With<Menu3dCamera>>,
    mut anim: ResMut<MenuCameraAnim>,
    mut loading_target: ResMut<PendingLoadingTarget>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            if friends_state.panel_visible {
                continue;
            }
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
                        // Pan camera right for matchmaking.
                        if let Ok(cam) = cam_query.single() {
                            start_camera_anim(&mut anim, cam, CAMERA_MATCHMAKING_TRANSLATION, CAMERA_MATCHMAKING_LOOK_AT);
                        }
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
                        // Pan camera right for matchmaking.
                        if let Ok(cam) = cam_query.single() {
                            start_camera_anim(&mut anim, cam, CAMERA_MATCHMAKING_TRANSLATION, CAMERA_MATCHMAKING_LOOK_AT);
                        }
                    } else {
                        // Offline: set map based on selected game mode.
                        let map_id = match selected_mode.mode {
                            GameMode::TestingGrounds => "testing_grounds",
                            _ => {
                                // Pick a random map from the global pool.
                                let maps = ["dust_storm", "city"];
                                maps[rand::rng().random_range(0..maps.len())]
                            }
                        };
                        commands.insert_resource(SelectedMapId(map_id.to_string()));
                        loading_target.0 = LoadingTarget::IntoMatch;
                        next_state.set(GameState::Loading);
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
                        // Settings closed → return camera to default.
                        if let Ok(cam) = cam_query.single() {
                            start_camera_anim(&mut anim, cam, CAMERA_DEFAULT_TRANSLATION, CAMERA_DEFAULT_LOOK_AT);
                        }
                    } else {
                        crate::ui_settings::spawn_settings_menu(&mut commands);
                        // Settings opened → pan camera behind player.
                        if let Ok(cam) = cam_query.single() {
                            start_camera_anim(&mut anim, cam, CAMERA_SETTINGS_TRANSLATION, CAMERA_SETTINGS_LOOK_AT);
                        }
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
            if friends_state.panel_visible {
                continue;
            }
            if tcp_client.is_connected() {
                let msg = noctyrn_shared::protocol::ClientMessage::CancelMatchmaking;
                let tcp = tcp_client.clone();
                let rt = rt.0.clone();
                rt.spawn(async move {
                    let _ = tcp.send(&msg).await;
                });
            }
            matchmaking_timer.searching = false;
            // Cancel matchmaking → return camera to default.
            if let Ok(cam) = cam_query.single() {
                start_camera_anim(&mut anim, cam, CAMERA_DEFAULT_TRANSLATION, CAMERA_DEFAULT_LOOK_AT);
            }
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

/// Purple gradient step for the main-menu buttons: light at the top
/// (LOADOUT) fading to dark at the bottom (QUIT).
fn menu_button_color(button: &MainMenuButton) -> Color {
    match button {
        MainMenuButton::Loadout => Color::srgba(0.78, 0.67, 0.96, 1.0),
        MainMenuButton::Crates => Color::srgba(0.70, 0.56, 0.92, 1.0),
        MainMenuButton::Cosmetics => Color::srgba(0.62, 0.46, 0.88, 1.0),
        MainMenuButton::Profile => Color::srgba(0.57, 0.38, 0.87, 1.0),
        MainMenuButton::Settings => Color::srgba(0.48, 0.31, 0.78, 1.0),
        MainMenuButton::Quit => Color::srgba(0.40, 0.25, 0.68, 1.0),
        _ => TEXT,
    }
}

pub fn main_menu_hover(
    mut query: Query<(&Interaction, &MainMenuButton, &Children), With<Button>>,
    mut text_query: Query<&mut TextColor>,
) {
    for (interaction, button, children) in query.iter_mut() {
        let (base_color, hover_color) = match button {
            MainMenuButton::Play => (TEXT, TEXT),
            MainMenuButton::GameModeSelect => (TEXT, ACCENT),
            _ => (menu_button_color(button), TEXT),
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
                border: UiRect::all(Val::Px(1.0)),
                    border_radius: RADIUS,
                ..default()},
            BackgroundColor(BG_PANEL.with_alpha(0.95)),
            BorderColor::all(BORDER),
            ServerDisconnectedNotif,
        )).with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(8.0),
                    height: Val::Px(8.0),
                        border_radius: RADIUS_SM,
                    ..default()},
                BackgroundColor(DANGER),
            ));
            root.spawn((
                Text::new("Not connected to server"),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(TEXT),
            ));
        });
    } else if !is_disconnected && has_notif {
        for entity in existing.iter() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn main_menu_matchmaking_handler(
    mut commands: Commands,
    mut events: MessageReader<NetworkEvent>,
    mut next_state: ResMut<NextState<GameState>>,
    mut timer: ResMut<MatchmakingTimer>,
    udp: Res<crate::net::udp::UdpClient>,
    connection: Res<crate::net::ConnectionState>,
    rt: Res<crate::net::TokioRuntime>,
    server_config: Res<ServerConfig>,
    mut loading_target: ResMut<PendingLoadingTarget>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::MatchmakingUpdate { players_in_queue } => {
                timer.players_in_queue = *players_in_queue;
            }
            NetworkEvent::MatchFound { lobby_id, udp_port, map_id, .. } => {
                let host = server_config.tcp_addr.split(':').next().unwrap_or("127.0.0.1");
                let addr = format!("{}:{}", host, udp_port);
                info!("Match found! lobby={lobby_id} map={map_id} connecting UDP to {addr} (host from server_config.tcp_addr={})", server_config.tcp_addr);
                let sid = *lobby_id;
                let user_id = connection.user_id().unwrap_or_default();
                let udp_clone = udp.clone();
                rt.0.spawn(async move {
                    match udp_clone.connect(&addr, sid, user_id).await {
                        Ok(()) => info!("UDP connect SUCCESS to {addr}"),
                        Err(e) => warn!("UDP connect FAILED to {addr}: {e}"),
                    }
                });
                commands.insert_resource(SelectedMapId(map_id.clone()));
                timer.searching = false;
                loading_target.0 = LoadingTarget::IntoMatch;
                next_state.set(GameState::Loading);
            }
            _ => {}
        }
    }
}
