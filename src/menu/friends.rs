use bevy::prelude::*;
use bevy::input::keyboard::KeyboardInput;
use crate::net::{CachedFriends, PartyState, TokioRuntime, ConnectionState, NetworkEvent, http::{self, PendingRequests}, ServerConfig};

#[derive(Component)]
pub struct FriendsPanelUi;

#[derive(Component)]
pub struct FriendRemoveButton { friend_id: uuid::Uuid }

#[derive(Component)]
pub struct FriendConfirmRemove { friend_id: uuid::Uuid }

#[derive(Component)]
pub struct FriendCancelRemove { friend_id: uuid::Uuid }

#[derive(Component)]
pub struct FriendAcceptRequest { request_id: uuid::Uuid }

#[derive(Component)]
pub struct FriendDeclineRequest { request_id: uuid::Uuid }

#[derive(Component)]
pub struct FriendSubmitButton;

#[derive(Component)]
pub struct OpenFriendsButton;

#[derive(Component)]
pub struct CloseFriendsPanelButton;

#[derive(Component)]
pub struct FriendsGoToProfileButton;

#[derive(Component)]
pub(crate) struct FriendsSearchInputText;

#[derive(Component)]
pub(crate) struct FriendsSearchInput;

#[derive(Component)]
pub(crate) struct FriendsSearchMessageText;

#[derive(Component)]
pub(crate) struct FriendsTabButton(pub FriendsTab);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FriendsTab {
    Party,
    Friends,
    Add,
}

#[derive(Resource)]
pub struct FriendsUiState {
    pub panel_visible: bool,
    pub search_query: String,
    pub focused: bool,
    pub search_message: Option<String>,
    pub refresh_pending: bool,
    pub active_tab: FriendsTab,
    pub pending_remove: Option<uuid::Uuid>,
    pub pending_add_target: Option<String>,
}

impl Default for FriendsUiState {
    fn default() -> Self {
        Self {
            panel_visible: false,
            search_query: String::new(),
            focused: false,
            search_message: None,
            refresh_pending: false,
            active_tab: FriendsTab::Friends,
            pending_remove: None,
            pending_add_target: None,
        }
    }
}

fn fuzzy_match(query: &str, target: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let query_lower = query.to_lowercase();
    let target_lower = target.to_lowercase();
    let mut query_chars = query_lower.chars().peekable();
    for c in target_lower.chars() {
        if query_chars.peek() == Some(&c) {
            query_chars.next();
        }
    }
    query_chars.peek().is_none()
}

fn spawn_btn_text(parent: &mut ChildSpawnerCommands, text: &str, size: f32, color: Color) {
    parent.spawn((
        Text::new(text),
        TextFont { font_size: size, ..default() },
        TextColor(color),
    ));
}

pub fn toggle_friends_panel(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<OpenFriendsButton>, With<Button>)>,
    mut state: ResMut<FriendsUiState>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            state.panel_visible = !state.panel_visible;
            state.pending_remove = None;
            state.pending_add_target = None;
            if state.panel_visible && conn_state.is_connected() {
                if let Some(token) = conn_state.token() {
                    http::spawn_http_request(&rt, &pending, http::async_get_friends(server_config.http_url.clone(), token.to_string()));
                    http::spawn_http_request(&rt, &pending, http::async_get_friend_requests(server_config.http_url.clone(), token.to_string()));
                }
            }
        }
    }
}

pub fn send_friend_request_from_query(
    state: &mut FriendsUiState,
    conn_state: &ConnectionState,
    rt: &TokioRuntime,
    server_config: &ServerConfig,
    pending: &PendingRequests,
) {
    let query = state.search_query.trim().to_string();
    if query.is_empty() {
        state.search_message = Some("Error: Enter a username".to_string());
        state.refresh_pending = true;
        return;
    }
    if let Some(token) = conn_state.token() {
        info!("Sending friend request to {query}");
        http::spawn_http_request(
            rt,
            pending,
            http::async_send_friend_request(server_config.http_url.clone(), token.to_string(), query.to_lowercase()),
        );
        state.search_message = Some(format!("Sending request to {}...", query));
        state.refresh_pending = true;
    }
}

pub fn spawn_friends_panel(
    mut commands: Commands,
    mut state: ResMut<FriendsUiState>,
    friends: Res<CachedFriends>,
    party_state: Res<PartyState>,
    conn_state: Res<ConnectionState>,
    existing: Query<Entity, With<FriendsPanelUi>>
) {
    if !state.panel_visible {
        return;
    }
    if state.refresh_pending {
        for entity in existing.iter() {
            commands.entity(entity).despawn();
        }
        state.refresh_pending = false;
    } else if !existing.is_empty() {
        return;
    }

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Px(300.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(8.0),
            border: UiRect::left(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.97)),
        BorderColor::all(Color::srgba(0.2, 0.2, 0.3, 0.5)),
        ZIndex(30),
        FriendsPanelUi,
    )).with_children(|panel| {
        panel.spawn(Node {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        }).with_children(|header| {
            header.spawn((Text::new("FRIENDS"), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
            header.spawn((Button, CloseFriendsPanelButton)).with_children(|btn| {
                spawn_btn_text(btn, "X", 16.0, Color::srgba(0.7, 0.7, 0.7, 0.8));
            });
        });

        if !conn_state.is_connected() {
            panel.spawn((
                Text::new("Log in to invite friends"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                Node { margin: UiRect::vertical(Val::Px(20.0)), ..default() },
            ));
            panel.spawn((
                Button,
                Node { width: Val::Percent(100.0), height: Val::Px(40.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                BackgroundColor(Color::srgba(0.2, 0.4, 0.2, 0.9)),
                FriendsGoToProfileButton,
            )).with_children(|btn| {
                spawn_btn_text(btn, "GO TO LOGIN", 14.0, Color::WHITE);
            });
            return;
        }

        panel.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            margin: UiRect::bottom(Val::Px(4.0)),
            ..default()
        }).with_children(|tabs| {
            for (tab, label) in [(FriendsTab::Party, "PARTY"), (FriendsTab::Friends, "FRIENDS"), (FriendsTab::Add, "ADD")] {
                let is_active = state.active_tab == tab;
                tabs.spawn((
                    Button,
                    Node {
                        flex_grow: 1.0,
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::bottom(Val::Px(if is_active { 2.0 } else { 0.0 })),
                        ..default()
                    },
                    BackgroundColor(if is_active { Color::srgba(0.2, 0.4, 0.6, 0.8) } else { Color::srgba(0.1, 0.1, 0.15, 0.8) }),
                    BorderColor::all(Color::srgba(0.4, 0.6, 0.9, 0.8)),
                    FriendsTabButton(tab),
                )).with_children(|btn| {
                    spawn_btn_text(btn, label, 11.0, Color::WHITE);
                });
            }
        });

        panel.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            ..default()
        }).with_children(|search| {
            search.spawn((
                Button,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(6.0), Val::Px(6.0)),
                    flex_grow: 1.0,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.12, 0.12, 0.18, 0.9)),
                FriendsSearchInput,
            )).with_children(|input| {
                let display = if state.search_query.is_empty() {
                    if state.focused { "|".to_string() } else { "Search...".to_string() }
                } else if state.focused {
                    format!("{}|", state.search_query)
                } else {
                    state.search_query.clone()
                };
                input.spawn((
                    Text::new(display),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                    FriendsSearchInputText,
                ));
            });

            if state.active_tab == FriendsTab::Add && !state.search_query.is_empty() {
                search.spawn((
                    Button,
                    Node { width: Val::Px(50.0), height: Val::Px(32.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    BackgroundColor(Color::srgba(0.2, 0.4, 0.2, 0.9)),
                    FriendSubmitButton,
                )).with_children(|btn| {
                    spawn_btn_text(btn, "ADD", 11.0, Color::WHITE);
                });
            }
        });

        if let Some(ref msg) = state.search_message {
            let is_error = msg.starts_with("Error:") || msg.starts_with("Could not");
            panel.spawn((
                Text::new(msg.as_str()),
                TextFont { font_size: 11.0, ..default() },
                TextColor(if is_error { Color::srgba(0.9, 0.4, 0.4, 0.9) } else { Color::srgba(0.4, 0.8, 0.4, 0.9) }),
                FriendsSearchMessageText,
            ));
        }

        match state.active_tab {
            FriendsTab::Party => {
                panel.spawn((
                    Text::new("PARTY MEMBERS"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.6, 0.6, 0.8, 0.7)),
                    Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
                ));
                if let Some(party) = &party_state.party {
                    for member in &party.members {
                        panel.spawn((
                            Node { width: Val::Percent(100.0), flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(6.0)), ..default() },
                            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.7)),
                        )).with_children(|entry| {
                            entry.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|inner| {
                                inner.spawn(Node { width: Val::Px(8.0), height: Val::Px(8.0), ..default() }).insert(BackgroundColor(Color::srgb(0.2, 0.9, 0.2)));
                                spawn_btn_text(inner, &member.username, 14.0, Color::WHITE);
                            });
                        });
                    }
                } else {
                    panel.spawn((
                        Text::new("Not in a party"),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(Color::srgba(0.4, 0.4, 0.5, 0.5)),
                        Node { margin: UiRect::vertical(Val::Px(10.0)), ..default() },
                    ));
                }
            }
            FriendsTab::Friends => {
                panel.spawn((
                    Text::new("YOUR FRIENDS"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.6, 0.6, 0.8, 0.7)),
                    Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
                ));
                let filtered: Vec<_> = friends.friends.iter()
                    .filter(|f| fuzzy_match(&state.search_query, &f.username))
                    .collect();
                let online: Vec<_> = filtered.iter().filter(|f| f.online).collect();
                let offline: Vec<_> = filtered.iter().filter(|f| !f.online).collect();

                for friend in &online {
                    let is_pending_remove = state.pending_remove == Some(friend.id);
                    panel.spawn((
                        Node { width: Val::Percent(100.0), flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(6.0)), ..default() },
                        BackgroundColor(Color::srgba(0.08, 0.15, 0.08, 0.7)),
                    )).with_children(|entry| {
                        entry.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|inner| {
                            inner.spawn(Node { width: Val::Px(8.0), height: Val::Px(8.0), ..default() }).insert(BackgroundColor(Color::srgb(0.2, 0.9, 0.2)));
                            spawn_btn_text(inner, &friend.username, 14.0, Color::WHITE);
                        });
                        if is_pending_remove {
                            spawn_btn_text(entry, "Remove?", 11.0, Color::srgba(0.9, 0.3, 0.3, 0.9));
                            entry.spawn((
                                Button, Node { padding: UiRect::new(Val::Px(5.0), Val::Px(5.0), Val::Px(2.0), Val::Px(2.0)), ..default() },
                                BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.8)),
                                FriendConfirmRemove { friend_id: friend.id },
                            )).with_children(|btn| { spawn_btn_text(btn, "YES", 10.0, Color::WHITE); });
                            entry.spawn((
                                Button, Node { padding: UiRect::new(Val::Px(5.0), Val::Px(5.0), Val::Px(2.0), Val::Px(2.0)), ..default() },
                                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                                FriendCancelRemove { friend_id: friend.id },
                            )).with_children(|btn| { spawn_btn_text(btn, "NO", 10.0, Color::srgba(0.7, 0.7, 0.7, 0.9)); });
                        } else {
                            entry.spawn((
                                Button, Node { padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(3.0), Val::Px(3.0)), ..default() },
                                BackgroundColor(Color::srgba(0.3, 0.1, 0.1, 0.8)),
                                FriendRemoveButton { friend_id: friend.id },
                            )).with_children(|btn| { spawn_btn_text(btn, "X", 11.0, Color::srgba(0.9, 0.3, 0.3, 0.8)); });
                        }
                    });
                }

                for friend in &offline {
                    let is_pending_remove = state.pending_remove == Some(friend.id);
                    panel.spawn((
                        Node { width: Val::Percent(100.0), flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(6.0)), ..default() },
                        BackgroundColor(Color::srgba(0.1, 0.1, 0.12, 0.5)),
                    )).with_children(|entry| {
                        entry.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(6.0), ..default() }).with_children(|inner| {
                            inner.spawn(Node { width: Val::Px(8.0), height: Val::Px(8.0), ..default() }).insert(BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.5)));
                            spawn_btn_text(inner, &friend.username, 14.0, Color::srgba(0.5, 0.5, 0.5, 0.7));
                        });
                        if is_pending_remove {
                            spawn_btn_text(entry, "Remove?", 11.0, Color::srgba(0.9, 0.3, 0.3, 0.9));
                            entry.spawn((
                                Button, Node { padding: UiRect::new(Val::Px(5.0), Val::Px(5.0), Val::Px(2.0), Val::Px(2.0)), ..default() },
                                BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.8)),
                                FriendConfirmRemove { friend_id: friend.id },
                            )).with_children(|btn| { spawn_btn_text(btn, "YES", 10.0, Color::WHITE); });
                            entry.spawn((
                                Button, Node { padding: UiRect::new(Val::Px(5.0), Val::Px(5.0), Val::Px(2.0), Val::Px(2.0)), ..default() },
                                BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                                FriendCancelRemove { friend_id: friend.id },
                            )).with_children(|btn| { spawn_btn_text(btn, "NO", 10.0, Color::srgba(0.7, 0.7, 0.7, 0.9)); });
                        } else {
                            entry.spawn((
                                Button, Node { padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(3.0), Val::Px(3.0)), ..default() },
                                BackgroundColor(Color::srgba(0.3, 0.1, 0.1, 0.8)),
                                FriendRemoveButton { friend_id: friend.id },
                            )).with_children(|btn| { spawn_btn_text(btn, "X", 11.0, Color::srgba(0.9, 0.3, 0.3, 0.8)); });
                        }
                    });
                }

                if filtered.is_empty() && !state.search_query.is_empty() {
                    panel.spawn((
                        Text::new("No friends match your search"),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(Color::srgba(0.4, 0.4, 0.5, 0.5)),
                        Node { margin: UiRect::vertical(Val::Px(10.0)), ..default() },
                    ));
                } else if friends.friends.is_empty() {
                    panel.spawn((
                        Text::new("No friends yet. Go to ADD tab to search for users."),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(Color::srgba(0.4, 0.4, 0.5, 0.5)),
                        Node { margin: UiRect::vertical(Val::Px(10.0)), ..default() },
                    ));
                }
            }
            FriendsTab::Add => {
                panel.spawn((
                    Text::new("ADD FRIENDS"),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.6, 0.6, 0.8, 0.7)),
                    Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
                ));

                if let Some(ref target) = state.pending_add_target {
                    panel.spawn((
                        Node { width: Val::Percent(100.0), flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(6.0)), ..default() },
                        BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.7)),
                    )).with_children(|entry| {
                        spawn_btn_text(entry, target.as_str(), 14.0, Color::WHITE);
                        entry.spawn((
                            Button,
                            Node { padding: UiRect::new(Val::Px(8.0), Val::Px(8.0), Val::Px(4.0), Val::Px(4.0)), ..default() },
                            BackgroundColor(Color::srgba(0.2, 0.5, 0.2, 0.9)),
                            FriendSubmitButton,
                        )).with_children(|btn| { spawn_btn_text(btn, "SEND", 10.0, Color::WHITE); });
                    });
                } else if state.search_query.is_empty() {
                    panel.spawn((
                        Text::new("Type a username and press Enter"),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(Color::srgba(0.4, 0.4, 0.5, 0.5)),
                        Node { margin: UiRect::vertical(Val::Px(10.0)), ..default() },
                    ));
                } else {
                    panel.spawn((
                        Text::new("Press Enter to search"),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(Color::srgba(0.4, 0.4, 0.5, 0.5)),
                        Node { margin: UiRect::vertical(Val::Px(10.0)), ..default() },
                    ));
                }

                if !friends.incoming_requests.is_empty() {
                    panel.spawn(Node { height: Val::Px(8.0), ..default() });
                    panel.spawn((
                        Text::new("PENDING REQUESTS"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgba(0.6, 0.6, 0.8, 0.7)),
                        Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
                    ));
                    for req in &friends.incoming_requests {
                        panel.spawn((
                            Node { width: Val::Percent(100.0), flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(6.0)), ..default() },
                            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.7)),
                        )).with_children(|entry| {
                            spawn_btn_text(entry, &req.from_username, 14.0, Color::WHITE);
                            entry.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), ..default() }).with_children(|actions| {
                                actions.spawn((
                                    Button, Node { padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(3.0), Val::Px(3.0)), ..default() },
                                    BackgroundColor(Color::srgba(0.2, 0.5, 0.2, 0.9)),
                                    FriendAcceptRequest { request_id: req.id },
                                )).with_children(|btn| { spawn_btn_text(btn, "ACCEPT", 9.0, Color::WHITE); });
                                actions.spawn((
                                    Button, Node { padding: UiRect::new(Val::Px(6.0), Val::Px(6.0), Val::Px(3.0), Val::Px(3.0)), ..default() },
                                    BackgroundColor(Color::srgba(0.3, 0.1, 0.1, 0.8)),
                                    FriendDeclineRequest { request_id: req.id },
                                )).with_children(|btn| { spawn_btn_text(btn, "DECLINE", 9.0, Color::srgba(0.9, 0.3, 0.3, 0.8)); });
                            });
                        });
                    }
                }
            }
        }
    });
}

pub fn friends_tab_interaction(
    interaction_query: Query<(&Interaction, &FriendsTabButton), (Changed<Interaction>, With<Button>)>,
    mut state: ResMut<FriendsUiState>,
) {
    for (interaction, tab_button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            state.active_tab = tab_button.0;
            state.pending_remove = None;
            state.pending_add_target = None;
            state.refresh_pending = true;
        }
    }
}

pub fn close_friends_panel(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<CloseFriendsPanelButton>, With<Button>)>,
    mut state: ResMut<FriendsUiState>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            state.panel_visible = false;
        }
    }
}

pub fn friends_click_outside(
    mouse_input: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    mut state: ResMut<FriendsUiState>,
) {
    if !state.panel_visible || !mouse_input.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(cursor) = window.cursor_position() else { return };
    let panel_left = window.width() - 300.0;
    let panel_right = window.width();
    if cursor.x < panel_left || cursor.x > panel_right || cursor.y < 0.0 || cursor.y > window.height() {
        state.panel_visible = false;
    }
}

pub fn despawn_friends_panel(
    mut commands: Commands,
    query: Query<Entity, With<FriendsPanelUi>>,
    state: Res<FriendsUiState>,
) {
    if state.panel_visible && !query.is_empty() {
        return;
    }
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn friends_search_input(
    mut char_events: MessageReader<KeyboardInput>,
    mut state: ResMut<FriendsUiState>,
    mut text_query: Query<&mut Text, (With<FriendsSearchInputText>, Without<FriendsSearchMessageText>)>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    if !state.panel_visible {
        return;
    }

    let mut query_changed = false;
    let mut enter_pressed = false;

    if state.focused {
        for event in char_events.read() {
            if !event.state.is_pressed() {
                continue;
            }
            match event.key_code {
                KeyCode::Backspace => {
                    state.search_query.pop();
                    query_changed = true;
                }
                KeyCode::Enter => {
                    enter_pressed = true;
                    if state.active_tab == FriendsTab::Add {
                        state.pending_add_target = Some(state.search_query.trim().to_string());
                        state.focused = true;
                        query_changed = true;
                    } else {
                        state.focused = false;
                        query_changed = true;
                    }
                }
                _ => {
                    if let bevy::input::keyboard::Key::Character(ref ch) = event.logical_key {
                        state.search_query.push_str(ch.as_str());
                        query_changed = true;
                    }
                }
            }
        }
    }

    if state.focused || query_changed {
        for mut text in text_query.iter_mut() {
            text.0 = if state.search_query.is_empty() && !state.focused {
                "Search...".to_string()
            } else if state.focused {
                format!("{}|", state.search_query)
            } else {
                state.search_query.clone()
            };
        }
        if query_changed {
            state.refresh_pending = true;
        }
    }

    if enter_pressed && state.active_tab == FriendsTab::Add {
        send_friend_request_from_query(&mut state, &conn_state, &rt, &server_config, &pending);
    }
}

pub fn friends_search_focus_handler(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<FriendsSearchInput>, With<Button>)>,
    mut state: ResMut<FriendsUiState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            state.focused = true;
            state.refresh_pending = true;
        }
    }
}

pub fn friends_add_button_handler(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>, With<FriendSubmitButton>)>,
    mut state: ResMut<FriendsUiState>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(ref target) = state.pending_add_target {
                if let Some(token) = conn_state.token() {
                    http::spawn_http_request(
                        &rt,
                        &pending,
                        http::async_send_friend_request(server_config.http_url.clone(), token.to_string(), target.to_lowercase()),
                    );
                    state.search_message = Some(format!("Sending request to {}...", target));
                    state.refresh_pending = true;
                }
            }
        }
    }
}

pub fn friends_confirm_remove_handler(
    interaction_query: Query<(&Interaction, &FriendConfirmRemove), (Changed<Interaction>, With<Button>)>,
    mut state: ResMut<FriendsUiState>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    for (interaction, btn) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            state.pending_remove = None;
            if let Some(token) = conn_state.token() {
                info!("Removing friend {}", btn.friend_id);
                http::spawn_http_request(
                    &rt,
                    &pending,
                    http::async_remove_friend(server_config.http_url.clone(), token.to_string(), btn.friend_id),
                );
            }
        }
    }
}

pub fn friends_cancel_remove_handler(
    interaction_query: Query<(&Interaction, &FriendCancelRemove), (Changed<Interaction>, With<Button>)>,
    mut state: ResMut<FriendsUiState>,
) {
    for (interaction, _btn) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            state.pending_remove = None;
            state.refresh_pending = true;
        }
    }
}

pub fn friends_remove_handler(
    interaction_query: Query<(&Interaction, &FriendRemoveButton), (Changed<Interaction>, With<Button>)>,
    mut state: ResMut<FriendsUiState>,
) {
    for (interaction, btn) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            state.pending_remove = Some(btn.friend_id);
            state.refresh_pending = true;
        }
    }
}

pub fn friends_accept_request_handler(
    interaction_query: Query<(&Interaction, &FriendAcceptRequest), (Changed<Interaction>, With<Button>)>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    for (interaction, btn) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(token) = conn_state.token() {
                info!("Accepting friend request {}", btn.request_id);
                http::spawn_http_request(
                    &rt,
                    &pending,
                    http::async_accept_friend_request(server_config.http_url.clone(), token.to_string(), btn.request_id),
                );
            }
        }
    }
}

pub fn friends_decline_request_handler(
    interaction_query: Query<(&Interaction, &FriendDeclineRequest), (Changed<Interaction>, With<Button>)>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
) {
    for (interaction, btn) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(token) = conn_state.token() {
                http::spawn_http_request(
                    &rt,
                    &pending,
                    http::async_decline_friend_request(server_config.http_url.clone(), token.to_string(), btn.request_id),
                );
            }
        }
    }
}

pub fn friends_handle_network_events(
    mut events: MessageReader<NetworkEvent>,
    mut state: ResMut<FriendsUiState>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
    mut friends: ResMut<CachedFriends>,
) {
    for event in events.read() {
        match event {
            NetworkEvent::FriendsLoaded { friends: loaded_friends } => {
                friends.friends = loaded_friends.clone();
                friends.loaded = true;
                if state.panel_visible { state.refresh_pending = true; }
            }
            NetworkEvent::FriendRequestsLoaded { incoming, outgoing } => {
                friends.incoming_requests = incoming.clone();
                friends.outgoing_requests = outgoing.clone();
                if state.panel_visible { state.refresh_pending = true; }
            }
            NetworkEvent::FriendRequestSent => {
                info!("Friend request sent successfully");
                state.search_message = Some("Friend request sent!".to_string());
                state.search_query.clear();
                state.pending_add_target = None;
                state.refresh_pending = true;
            }
            NetworkEvent::FriendRemoved => {
                state.pending_remove = None;
                if let Some(token) = conn_state.token() {
                    http::spawn_http_request(&rt, &pending, http::async_get_friends(server_config.http_url.clone(), token.to_string()));
                }
            }
            NetworkEvent::FriendRequestAccepted | NetworkEvent::FriendRequestDeclined => {
                if let Some(token) = conn_state.token() {
                    http::spawn_http_request(&rt, &pending, http::async_get_friends(server_config.http_url.clone(), token.to_string()));
                    http::spawn_http_request(&rt, &pending, http::async_get_friend_requests(server_config.http_url.clone(), token.to_string()));
                }
            }
            NetworkEvent::FriendError { message } => {
                info!("Friend request error: {message}");
                state.search_message = Some(format!("Error: {}", message));
                state.pending_add_target = None;
                state.refresh_pending = true;
            }
            NetworkEvent::ConnectionError { message } => {
                info!("Connection error: {message}");
                state.search_message = Some(format!("Error: {}", message));
                state.refresh_pending = true;
            }
            _ => {}
        }
    }
}

pub fn friends_go_to_profile_handler(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<FriendsGoToProfileButton>, With<Button>)>,
    mut friends_state: ResMut<FriendsUiState>,
    mut login_state: ResMut<crate::menu::login::LoginUiState>,
) {
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            friends_state.panel_visible = false;
            login_state.show_overlay = true;
            login_state.focused_field = Some(crate::menu::login::LoginField::Email);
        }
    }
}