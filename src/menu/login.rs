use bevy::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use crate::player::GameState;
use crate::net::{ConnectionState, ServerConfig, TokioRuntime, NetworkEvent, http::{self, PendingRequests}};
use crate::net::tcp::TcpClient;
use crate::menu::profile::ProfileOverlayState;

const AUTH_TOKEN_PATH: &str = "settings/auth_token.json";

#[derive(Component)]
pub struct LoginOverlayUi;

#[derive(Component)]
pub struct LoginTextInput { field: LoginField }

#[derive(Component)]
pub(crate) struct LoginFieldText(LoginField);

#[derive(Component)]
pub(crate) struct LoginErrorText;

#[derive(Component, Clone)]
pub enum LoginButton {
    Login,
    Register,
    SwitchToLogin,
    SwitchToRegister,
    Back,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginField { Email, Password, Username, ConfirmPassword }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LoginMode { #[default] Login, Register }

#[derive(Resource)]
pub struct LoginUiState {
    pub mode: LoginMode,
    pub email: String,
    pub password: String,
    pub username: String,
    pub confirm_password: String,
    pub error_message: Option<String>,
    pub loading: bool,
    pub focused_field: Option<LoginField>,
    pub show_overlay: bool,
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
            show_overlay: false,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredToken { token: String, username: String, user_id: String }

pub fn load_persisted_token() -> Option<(uuid::Uuid, String, String)> {
    std::fs::read_to_string(AUTH_TOKEN_PATH).ok().and_then(|data| {
        serde_json::from_str::<StoredToken>(&data).ok().and_then(|st| {
            let uid = uuid::Uuid::parse_str(&st.user_id).ok()?;
            Some((uid, st.username, st.token))
        })
    })
}

pub(crate) fn save_token(token: &str, username: &str, user_id: uuid::Uuid) {
    if let Some(parent) = std::path::Path::new(AUTH_TOKEN_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let stored = StoredToken { token: token.to_string(), username: username.to_string(), user_id: user_id.to_string() };
    if let Ok(data) = serde_json::to_string(&stored) {
        let _ = std::fs::write(AUTH_TOKEN_PATH, &data);
    }
}

pub(crate) fn clear_token() {
    let _ = std::fs::remove_file(AUTH_TOKEN_PATH);
}

pub fn spawn_login_overlay(mut commands: Commands, login_state: Res<LoginUiState>) {
    if !login_state.show_overlay {
        return;
    }
    let is_register = login_state.mode == LoginMode::Register;

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
        LoginOverlayUi,
    )).with_children(|root| {
        root.spawn((
            Node { width: Val::Px(400.0), flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(30.0)), row_gap: Val::Px(16.0), border: UiRect::all(Val::Px(1.0)), ..default() },
            BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.95)),
            BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.5)),
        )).with_children(|card| {
            card.spawn((Text::new(if is_register { "CREATE ACCOUNT" } else { "LOGIN" }), TextFont { font_size: 28.0, ..default() }, TextColor(Color::WHITE), Node { margin: UiRect::bottom(Val::Px(8.0)), ..default() }));

            card.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), margin: UiRect::bottom(Val::Px(8.0)), ..default() }).with_children(|tabs| {
                tabs.spawn((Button, Node { width: Val::Percent(50.0), height: Val::Px(36.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    BackgroundColor(if !is_register { Color::srgba(0.2, 0.4, 0.2, 0.8) } else { Color::srgba(0.15, 0.15, 0.2, 0.8) }),
                    LoginButton::SwitchToLogin,
                )).with_children(|btn| {
                    btn.spawn((Text::new("LOGIN"), TextFont { font_size: 14.0, ..default() }, TextColor(if !is_register { Color::WHITE } else { Color::srgba(0.5, 0.5, 0.5, 0.8) })));
                });
                tabs.spawn((Button, Node { width: Val::Percent(50.0), height: Val::Px(36.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    BackgroundColor(if is_register { Color::srgba(0.2, 0.4, 0.2, 0.8) } else { Color::srgba(0.15, 0.15, 0.2, 0.8) }),
                    LoginButton::SwitchToRegister,
                )).with_children(|btn| {
                    btn.spawn((Text::new("REGISTER"), TextFont { font_size: 14.0, ..default() }, TextColor(if is_register { Color::WHITE } else { Color::srgba(0.5, 0.5, 0.5, 0.8) })));
                });
            });

            if is_register {
                spawn_login_input_field(card, "USERNAME", LoginField::Username, &login_state.username, false, login_state.focused_field == Some(LoginField::Username));
            }
            spawn_login_input_field(card, "EMAIL", LoginField::Email, &login_state.email, false, login_state.focused_field == Some(LoginField::Email));
            spawn_login_input_field(card, "PASSWORD", LoginField::Password, &login_state.password, true, login_state.focused_field == Some(LoginField::Password));
            if is_register {
                spawn_login_input_field(card, "CONFIRM PASSWORD", LoginField::ConfirmPassword, &login_state.confirm_password, true, login_state.focused_field == Some(LoginField::ConfirmPassword));
            }

            card.spawn((Button, Node { width: Val::Percent(100.0), height: Val::Px(44.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, margin: UiRect::top(Val::Px(8.0)), ..default() },
                BackgroundColor(Color::srgb(0.15, 0.5, 0.15)),
                if is_register { LoginButton::Register } else { LoginButton::Login },
            )).with_children(|btn| {
                btn.spawn((Text::new(if login_state.loading { "LOADING..." } else if is_register { "CREATE ACCOUNT" } else { "LOGIN" }), TextFont { font_size: 16.0, ..default() }, TextColor(Color::WHITE)));
            });

            card.spawn((Text::new(login_state.error_message.as_deref().unwrap_or("")), TextFont { font_size: 13.0, ..default() }, TextColor(Color::srgb(0.9, 0.2, 0.2)), LoginErrorText));

            card.spawn((Button, Node { width: Val::Percent(100.0), height: Val::Px(36.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, margin: UiRect::top(Val::Px(4.0)), ..default() },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.8)),
                LoginButton::Back,
            )).with_children(|btn| {
                btn.spawn((Text::new("BACK"), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9))));
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
    parent.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() }).with_children(|container| {
        container.spawn((Text::new(label), TextFont { font_size: 11.0, ..default() }, TextColor(Color::srgba(0.5, 0.5, 0.6, 0.9))));

        let display_text = if is_password { "*".repeat(current_value.len()) } else { current_value.to_string() };
        let display_with_cursor = if is_focused { format!("{}|", display_text) } else if display_text.is_empty() { " ".to_string() } else { display_text };

        container.spawn((
            Button,
            Node { width: Val::Percent(100.0), height: Val::Px(36.0), padding: UiRect::horizontal(Val::Px(10.0)), align_items: AlignItems::Center, border: UiRect::all(Val::Px(1.0)), ..default() },
            BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.9)),
            BorderColor::all(if is_focused { Color::srgba(0.3, 0.6, 0.3, 0.8) } else { Color::srgba(0.25, 0.25, 0.3, 0.6) }),
            LoginTextInput { field },
        )).with_children(|input| {
            input.spawn((Text::new(display_with_cursor), TextFont { font_size: 14.0, ..default() }, TextColor(Color::srgba(0.85, 0.85, 0.85, 0.95)), LoginFieldText(field)));
        });
    });
}

pub fn despawn_login_overlay(mut commands: Commands, query: Query<Entity, With<LoginOverlayUi>>, state: Res<LoginUiState>) {
    if state.show_overlay && !query.is_empty() {
        return;
    }
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn force_despawn_login_overlay(mut commands: Commands, query: Query<Entity, With<LoginOverlayUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn spawn_login_overlay_system(
    mut commands: Commands,
    state: Res<LoginUiState>,
    existing: Query<Entity, With<LoginOverlayUi>>,
    mut last_mode: Local<LoginMode>,
) {
    if !state.show_overlay {
        return;
    }
    let mode_changed = state.mode != *last_mode;
    let has_existing = !existing.is_empty();
    if has_existing && !mode_changed {
        return;
    }
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
    *last_mode = state.mode;
    spawn_login_overlay(commands, state);
}

pub fn login_interaction(
    interaction_query: Query<(&Interaction, &LoginButton), (Changed<Interaction>, With<Button>)>,
    input_query: Query<(&Interaction, &LoginTextInput), (Changed<Interaction>, With<Button>)>,
    mut login_state: ResMut<LoginUiState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
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
                        http::spawn_http_request(
                            &rt,
                            &pending,
                            http::async_login(server_config.http_url.clone(), login_state.email.clone(), login_state.password.clone()),
                        );
                    }
                }
                LoginButton::Register => {
                    if !login_state.loading && !login_state.username.is_empty() {
                        if login_state.password != login_state.confirm_password {
                            login_state.error_message = Some("Passwords do not match".to_string());
                            login_state.loading = false;
                            return;
                        }
                        login_state.loading = true;
                        login_state.error_message = None;
                        http::spawn_http_request(
                            &rt,
                            &pending,
                            http::async_register(server_config.http_url.clone(), login_state.username.clone(), login_state.email.clone(), login_state.password.clone()),
                        );
                    }
                }
                LoginButton::SwitchToLogin => {
                    login_state.mode = LoginMode::Login;
                    login_state.error_message = None;
                }
                LoginButton::SwitchToRegister => {
                    login_state.mode = LoginMode::Register;
                    login_state.error_message = None;
                }
                LoginButton::Back => {
                    login_state.show_overlay = false;
                }
            }
        }
    }
}

pub fn login_text_input(
    mut char_events: MessageReader<KeyboardInput>,
    mut login_state: ResMut<LoginUiState>,
) {
    if !login_state.show_overlay {
        return;
    }
    let focused = match login_state.focused_field {
        Some(f) => f,
        None => return,
    };

    for event in char_events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        let target = match focused {
            LoginField::Email => &mut login_state.email,
            LoginField::Password => &mut login_state.password,
            LoginField::Username => &mut login_state.username,
            LoginField::ConfirmPassword => &mut login_state.confirm_password,
        };

        match event.key_code {
            KeyCode::Backspace => { target.pop(); }
            KeyCode::Enter => {
                if !login_state.loading {
                    match login_state.mode {
                        LoginMode::Login => {
                            login_state.loading = true;
                            login_state.error_message = None;
                        }
                        LoginMode::Register => {
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
                        }
                    }
                }
            }
            KeyCode::Tab => {
                login_state.focused_field = Some(match focused {
                    LoginField::Email => LoginField::Password,
                    LoginField::Password => {
                        if login_state.mode == LoginMode::Register {
                            LoginField::ConfirmPassword
                        } else {
                            LoginField::Email
                        }
                    }
                    LoginField::Username => LoginField::Email,
                    LoginField::ConfirmPassword => LoginField::Email,
                });
            }
            _ => {
                if let bevy::input::keyboard::Key::Character(ref ch) = event.logical_key {
                    target.push_str(ch.as_str());
                }
            }
        }
    }
}

pub(crate) fn key_to_char(key: &KeyCode) -> Option<char> {
    match key {
        KeyCode::Space => Some(' '),
        KeyCode::Minus => Some('-'),
        KeyCode::Period => Some('.'),
        KeyCode::Digit0 => Some('0'),
        KeyCode::Digit1 => Some('1'),
        KeyCode::Digit2 => Some('2'),
        KeyCode::Digit3 => Some('3'),
        KeyCode::Digit4 => Some('4'),
        KeyCode::Digit5 => Some('5'),
        KeyCode::Digit6 => Some('6'),
        KeyCode::Digit7 => Some('7'),
        KeyCode::Digit8 => Some('8'),
        KeyCode::Digit9 => Some('9'),
        KeyCode::KeyA => Some('a'),
        KeyCode::KeyB => Some('b'),
        KeyCode::KeyC => Some('c'),
        KeyCode::KeyD => Some('d'),
        KeyCode::KeyE => Some('e'),
        KeyCode::KeyF => Some('f'),
        KeyCode::KeyG => Some('g'),
        KeyCode::KeyH => Some('h'),
        KeyCode::KeyI => Some('i'),
        KeyCode::KeyJ => Some('j'),
        KeyCode::KeyK => Some('k'),
        KeyCode::KeyL => Some('l'),
        KeyCode::KeyM => Some('m'),
        KeyCode::KeyN => Some('n'),
        KeyCode::KeyO => Some('o'),
        KeyCode::KeyP => Some('p'),
        KeyCode::KeyQ => Some('q'),
        KeyCode::KeyR => Some('r'),
        KeyCode::KeyS => Some('s'),
        KeyCode::KeyT => Some('t'),
        KeyCode::KeyU => Some('u'),
        KeyCode::KeyV => Some('v'),
        KeyCode::KeyW => Some('w'),
        KeyCode::KeyX => Some('x'),
        KeyCode::KeyY => Some('y'),
        KeyCode::KeyZ => Some('z'),
        _ => None,
    }
}

pub fn update_login_display(
    login_state: Res<LoginUiState>,
    mut field_query: Query<(&mut Text, &LoginFieldText)>,
) {
    for (mut text, field) in field_query.iter_mut() {
        let (value, is_password) = match field.0 {
            LoginField::Email => (&login_state.email, false),
            LoginField::Password => (&login_state.password, true),
            LoginField::Username => (&login_state.username, false),
            LoginField::ConfirmPassword => (&login_state.confirm_password, true),
        };
        let is_focused = login_state.focused_field == Some(field.0);
        let display = if is_password {
            "*".repeat(value.len())
        } else {
            value.clone()
        };
        **text = if is_focused {
            format!("{}|", if display.is_empty() { " " } else { &display })
        } else if display.is_empty() {
            " ".to_string()
        } else {
            display
        };
    }
}

pub fn try_auto_login(
    mut conn_state: ResMut<ConnectionState>,
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    if !conn_state.is_connected() {
        if let Some((user_id, username, token)) = load_persisted_token() {
            info!("Auto-login: found saved token for {username}");
            *conn_state = ConnectionState::Connected {
                token: token.clone(),
                user_id,
                username: username.clone(),
            };
            let addr = server_config.tcp_addr.clone();
            let t = token.clone();
            let client = tcp_client.clone();
            let rt_clone = TokioRuntime(rt.0.clone());
            let pending_clone = pending.clone();
            rt.0.spawn(async move {
                match client.connect_and_auth(&addr, &t, &rt_clone, &pending_clone).await {
                    Ok(()) => info!("Auto-login: TCP connected and authenticated"),
                    Err(e) => warn!("Auto-login: TCP connection failed: {e}"),
                }
            });
        }
    }
}

pub fn login_handle_network_events(
    mut events: MessageReader<NetworkEvent>,
    mut login_state: ResMut<LoginUiState>,
    mut conn_state: ResMut<ConnectionState>,
    mut next_state: ResMut<NextState<GameState>>,
    tcp_client: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
    mut profile_state: ResMut<ProfileOverlayState>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::LoginSuccess { token, user_id, username }
            | NetworkEvent::RegisterSuccess { token, user_id, username } => {
                *conn_state = ConnectionState::Connected {
                    token: token.clone(),
                    user_id: *user_id,
                    username: username.clone(),
                };
                login_state.loading = false;
                login_state.error_message = None;
                login_state.show_overlay = false;
                profile_state.show = true;
                save_token(token, username, *user_id);

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

                next_state.set(GameState::MainMenu);
            }
            NetworkEvent::LoginError { message }
            | NetworkEvent::RegisterError { message }
            | NetworkEvent::ConnectionError { message } => {
                login_state.loading = false;
                login_state.error_message = Some(message.clone());
            }
            _ => {}
        }
    }
}
