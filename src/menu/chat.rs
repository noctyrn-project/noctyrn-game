use bevy::prelude::*;
use bevy::input::keyboard::{KeyboardInput, Key};
use crate::net::{ConnectionState, TokioRuntime, NetworkEvent};
use crate::net::tcp::TcpClient;
use crate::player::GameState;

#[derive(Resource, Default)]
pub struct ChatOpen(pub bool);

/// Separate from ChatInput so expired messages can be cleaned up
/// while the input box is still open.
#[derive(Component)]
pub struct ChatHistoryUi;

#[derive(Component)]
pub struct ChatInputUi;

#[derive(Clone)]
pub struct MessageEntry {
    pub from: String,
    pub content: String,
    pub created_at: f64,
}

#[derive(Resource)]
pub struct ChatHistory {
    pub messages: Vec<MessageEntry>,
    pub generation: u64,
}

impl Default for ChatHistory {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            generation: 0,
        }
    }
}

#[derive(Resource)]
pub struct ChatInput {
    pub input: String,
    pub open: bool,
    pub time: f64,
}

impl Default for ChatInput {
    fn default() -> Self {
        Self {
            input: String::new(),
            open: false,
            time: 0.0,
        }
    }
}

const MAX_MESSAGES: usize = 100;
const MESSAGE_LIFETIME: f64 = 10.0;

pub fn chat_input(
    mut char_events: MessageReader<KeyboardInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut chat_input: ResMut<ChatInput>,
    mut history: ResMut<ChatHistory>,
    mut chat_open: ResMut<ChatOpen>,
    conn_state: Option<Res<ConnectionState>>,
    tcp: Res<TcpClient>,
    rt: Res<TokioRuntime>,
    time: Res<Time>,
    game_state: Option<Res<State<GameState>>>,
) {
    chat_input.time = time.elapsed_secs_f64();
    let in_main_menu = game_state.as_ref().map_or(false, |s| *s.get() == GameState::MainMenu);

    // In Playing state: Enter toggles input open/closed (same as current behavior)
    // In MainMenu state: input is always rendered but inactive until Enter opens it
    if !chat_input.open {
        if keyboard.just_pressed(KeyCode::Enter) {
            chat_input.open = true;
            chat_open.0 = true;
            chat_input.input.clear();
            history.generation += 1;
        }
        return;
    }

    let mut changed = false;
    if keyboard.just_pressed(KeyCode::Enter) {
        let trimmed = chat_input.input.trim().to_string();
        if trimmed.is_empty() && !in_main_menu {
            // Playing state: empty Enter closes input
            chat_input.open = false;
            chat_open.0 = false;
        } else if trimmed.starts_with('/') {
            let now = chat_input.time;
            handle_command(&trimmed, &tcp, &rt);
            history.messages.push(MessageEntry {
                from: ">".to_string(),
                content: trimmed,
                created_at: now,
            });
            history.generation += 1;
            chat_input.input.clear();
            if !in_main_menu {
                chat_input.open = false;
                chat_open.0 = false;
            }
        } else if !trimmed.is_empty() {
            let now = chat_input.time;
            let username = conn_state.as_ref().and_then(|c| c.username()).unwrap_or(">");
            history.messages.push(MessageEntry {
                from: username.to_string(),
                content: trimmed.clone(),
                created_at: now,
            });
            history.generation += 1;
            let msg = noctyrn_shared::protocol::ClientMessage::ChatMessage {
                content: trimmed,
            };
            let t = tcp.clone();
            let r = rt.0.clone();
            r.spawn(async move {
                if let Err(e) = t.send(&msg).await {
                    warn!("Chat send failed: {e}");
                }
            });
            chat_input.input.clear();
            if !in_main_menu {
                chat_input.open = false;
                chat_open.0 = false;
            }
        }
        changed = true;
    }
    if keyboard.just_pressed(KeyCode::Backspace) {
        chat_input.input.pop();
        changed = true;
    }

    // Process character input from MessageReader
    for event in char_events.read() {
        if !event.state.is_pressed() {
            continue;
        }
        if let Key::Character(ch) = &event.logical_key {
            if ch.len() == 1 {
                chat_input.input.push_str(ch.as_str());
                changed = true;
            }
        } else if event.key_code == KeyCode::Space {
            chat_input.input.push(' ');
            changed = true;
        }
    }

    if changed {
        history.generation += 1;
    }
}

pub fn cleanup_expired_messages(
    mut history: ResMut<ChatHistory>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    let before = history.messages.len();
    history.messages.retain(|m| now - m.created_at < MESSAGE_LIFETIME);
    if history.messages.len() != before {
        history.generation += 1;
    }
}

pub fn chat_history_display(
    mut commands: Commands,
    history: Res<ChatHistory>,
    existing: Query<Entity, With<ChatHistoryUi>>,
    time: Res<Time>,
    mut last_gen: Local<u64>,
) {
    let now = time.elapsed_secs_f64();
    let has_recent = history.messages.iter().any(|m| now - m.created_at < MESSAGE_LIFETIME);

    if !has_recent {
        if *last_gen != 0 {
            for entity in existing.iter() {
                commands.entity(entity).despawn();
            }
            *last_gen = 0;
        }
        return;
    }

    if *last_gen == history.generation && !existing.is_empty() {
        return;
    }

    *last_gen = history.generation;

    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }

    let visible: Vec<_> = history.messages.iter().filter(|m| now - m.created_at < MESSAGE_LIFETIME).collect();

    if visible.is_empty() {
        return;
    }

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(100.0),
            width: Val::Px(400.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(8.0)),
            row_gap: Val::Px(2.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        ZIndex(200),
        ChatHistoryUi,
    )).with_children(|parent| {
        for entry in visible.iter().rev().take(10).rev() {
            let display = if entry.from == ">" || entry.from == "System" {
                entry.content.clone()
            } else {
                format!("{}: {}", entry.from, entry.content)
            };
            let color = if entry.from == ">" || entry.from == "System" {
                Color::srgba(0.7, 0.7, 0.7, 0.9)
            } else {
                Color::WHITE
            };
            parent.spawn((
                Text::new(display),
                TextFont { font_size: 12.0, ..default() },
                TextColor(color),
            ));
        }
    });
}

pub fn chat_input_display(
    mut commands: Commands,
    input: Res<ChatInput>,
    existing: Query<Entity, With<ChatInputUi>>,
    game_state: Option<Res<State<GameState>>>,
    mut last_open: Local<bool>,
) {
    let in_main_menu = game_state.as_ref().map_or(false, |s| *s.get() == GameState::MainMenu);
    let should_show = in_main_menu || input.open;

    if !should_show {
        if *last_open {
            for entity in existing.iter() {
                commands.entity(entity).despawn();
            }
            *last_open = false;
        }
        return;
    }

    // Always despawn and respawn to keep cursor text up to date
    for entity in existing.iter() {
        commands.entity(entity).despawn();
    }
    *last_open = true;

    let cursor = if input.open {
        format!(">{}|", input.input)
    } else {
        "> ".to_string()
    };
    let text_color = if input.open {
        Color::srgb(0.3, 0.8, 0.3)
    } else {
        Color::srgba(0.5, 0.5, 0.5, 0.6)
    };

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            bottom: Val::Px(70.0),
            width: Val::Px(400.0),
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.4)),
        ZIndex(200),
        ChatInputUi,
    )).with_children(|parent| {
        parent.spawn((
            Text::new(cursor),
            TextFont { font_size: 12.0, ..default() },
            TextColor(text_color),
        ));
    });
}

pub fn chat_receive_handler(
    mut events: MessageReader<NetworkEvent>,
    mut history: ResMut<ChatHistory>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs_f64();
    for event in events.read() {
        if let NetworkEvent::ChatReceived { from_username, content } = event {
            history.messages.push(MessageEntry {
                from: from_username.clone(),
                content: content.clone(),
                created_at: now,
            });
            history.generation += 1;
            if history.messages.len() > MAX_MESSAGES {
                history.messages.remove(0);
            }
        }
    }
}

fn handle_command(cmd: &str, tcp: &TcpClient, rt: &TokioRuntime) {
    let parts: Vec<&str> = cmd.trim_start_matches('/').split_whitespace().collect();
    if parts.is_empty() {
        return;
    }
    let r = rt.0.clone();
    let t = tcp.clone();
    match parts[0] {
        "help" | "h" => {
            info!("/help - Show help");
            info!("/invite <username> - Invite to party");
            info!("/kick <username> - Kick from party");
            info!("/add <username> - Send friend request");
            info!("/remove <username> - Remove friend");
            info!("/que <gamemode> - Queue for match (ffa, tdm, kc, ctf, koth, hp, cp)");
        }
        "invite" => {
            if let Some(username) = parts.get(1) {
                let msg = noctyrn_shared::protocol::ClientMessage::PartyInvite {
                    username: username.to_string(),
                };
                r.spawn(async move { let _ = t.send(&msg).await; });
            }
        }
        "kick" => {
            if let Some(username) = parts.get(1) {
                let msg = noctyrn_shared::protocol::ClientMessage::PartyKick {
                    member_id: uuid::Uuid::nil(),
                };
                r.spawn(async move { let _ = t.send(&msg).await; });
            }
        }
        "add" => {
            if let Some(username) = parts.get(1) {
                info!("Friend request to {username} via /add");
            }
        }
        "remove" => {
            if let Some(username) = parts.get(1) {
                info!("Remove friend {username} via /remove");
            }
        }
        "que" | "queue" => {
            let mode = match parts.get(1).map(|s| s.to_lowercase()).as_deref() {
                Some("ffa") | None => noctyrn_shared::GameMode::FreeForAll,
                Some("tdm") => noctyrn_shared::GameMode::TeamDeathmatch,
                Some("kc") => noctyrn_shared::GameMode::KillConfirmed,
                Some("ctf") => noctyrn_shared::GameMode::CaptureTheFlag,
                Some("koth") => noctyrn_shared::GameMode::KingOfTheHill,
                Some("hp") => noctyrn_shared::GameMode::Hardpoint,
                Some("cp") => noctyrn_shared::GameMode::CapturePoint,
                _ => return,
            };
            let msg = noctyrn_shared::protocol::ClientMessage::QueueForMatch { game_mode: mode };
            r.spawn(async move { let _ = t.send(&msg).await; });
        }
        _ => {}
    }
}
