use bevy::prelude::*;
use crate::net::{ConnectionState, CachedProfile, ServerConfig, TokioRuntime, NetworkEvent, http::{self, PendingRequests}};

#[derive(Component)]
pub struct ProfileOverlayUi;

#[derive(Component)]
pub struct ProfileCloseButton;

#[derive(Component)]
pub struct ProfileLogoutButton;

#[derive(Component)]
pub struct ProfileGoToLoginButton;

#[derive(Component)]
pub struct ProfileStatText(String);

#[derive(Resource, Default)]
pub struct ProfileOverlayState {
    pub show: bool,
}

pub fn spawn_profile_overlay(
    mut commands: Commands,
    conn_state: Res<ConnectionState>,
    cached_profile: Res<CachedProfile>,
) {
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
        ProfileOverlayUi,
    )).with_children(|root| {
        root.spawn((
            Node {
                width: Val::Px(450.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(24.0)),
                row_gap: Val::Px(12.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.96)),
            BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.5)),
        )).with_children(|card| {
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            }).with_children(|header| {
                header.spawn((Text::new("PROFILE"), TextFont { font_size: FontSize::Px(22.0), ..default() }, TextColor(Color::WHITE)));
                header.spawn(Button).with_children(|btn| {
                    btn.spawn((Text::new("X"), TextFont { font_size: FontSize::Px(16.0), ..default() }, TextColor(Color::srgba(0.7, 0.7, 0.7, 0.8))));
                }).insert(ProfileCloseButton);
            });

            if !conn_state.is_connected() {
                card.spawn((Text::new("You are not logged in."), TextFont { font_size: FontSize::Px(16.0), ..default() }, TextColor(Color::srgba(0.7, 0.7, 0.7, 0.8)), Node { margin: UiRect::vertical(Val::Px(16.0)), ..default() }));
                card.spawn((
                    Button,
                    Node { width: Val::Percent(100.0), height: Val::Px(40.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    BackgroundColor(Color::srgba(0.2, 0.4, 0.2, 0.9)),
                    ProfileGoToLoginButton,
                )).with_children(|btn| {
                    btn.spawn((Text::new("GO TO LOGIN"), TextFont { font_size: FontSize::Px(15.0), ..default() }, TextColor(Color::WHITE)));
                });
                return;
            }

            let profile = cached_profile.profile.as_ref();
            let username = profile.map(|p| p.username.as_str()).unwrap_or(conn_state.username().unwrap_or("--"));
            card.spawn((Text::new(username), TextFont { font_size: FontSize::Px(30.0), ..default() }, TextColor(Color::srgb(0.4, 0.7, 1.0))));

            let level = profile.map(|p| p.level).unwrap_or(1);
            let xp = profile.map(|p| p.xp).unwrap_or(0);
            let xp_for_next = level * 1000;
            let xp_pct = (xp as f32 / xp_for_next.max(1) as f32 * 100.0).min(100.0);

            card.spawn((Text::new(format!("Level {}", level)), TextFont { font_size: FontSize::Px(18.0), ..default() }, TextColor(Color::srgba(0.8, 0.8, 0.3, 0.9))));

            card.spawn(Node {
                width: Val::Percent(100.0),
                height: Val::Px(8.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            }).with_children(|bar_bg| {
                bar_bg.spawn((
                    Node { width: Val::Percent(xp_pct), height: Val::Percent(100.0), ..default() },
                    BackgroundColor(Color::srgb(0.3, 0.7, 0.3)),
                ));
            });
            card.spawn((Text::new(format!("{} / {} XP", xp, xp_for_next)), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::srgba(0.5, 0.5, 0.5, 0.8))));

            let created = profile.map(|p| p.created_at.as_str()).unwrap_or("--");
            let display_date = if created.len() >= 10 { &created[..10] } else { created };
            card.spawn((Text::new(format!("Member since {}", display_date)), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7))));

            let kills = profile.map(|p| p.stats.total_kills).unwrap_or(0);
            let deaths = profile.map(|p| p.stats.total_deaths).unwrap_or(0);
            let kd = profile.map(|p| p.stats.kd_ratio()).unwrap_or(0.0);
            let wins = profile.map(|p| p.stats.total_wins).unwrap_or(0);
            let losses = profile.map(|p| p.stats.total_losses).unwrap_or(0);
            let win_rate = profile.map(|p| p.stats.win_rate()).unwrap_or(0.0);
            let matches = profile.map(|p| p.stats.total_matches).unwrap_or(0);
            let playtime_s = profile.map(|p| p.stats.playtime_seconds).unwrap_or(0);
            let currency = profile.map(|p| p.currency).unwrap_or(0);

            let hours = playtime_s / 3600;
            let minutes = (playtime_s % 3600) / 60;
            let playtime_display = if hours > 0 { format!("{}h {}m", hours, minutes) } else { format!("{}m", minutes) };

            card.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0), ..default() }).with_children(|row| {
                spawn_stat_pill(row, "KILLS", &kills.to_string());
                spawn_stat_pill(row, "DEATHS", &deaths.to_string());
                spawn_stat_pill(row, "K/D", &format!("{:.2}", kd));
            });
            card.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0), ..default() }).with_children(|row| {
                spawn_stat_pill(row, "WINS", &wins.to_string());
                spawn_stat_pill(row, "LOSSES", &losses.to_string());
                spawn_stat_pill(row, "WIN RATE", &format!("{:.1}%", win_rate));
            });
            card.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(8.0), ..default() }).with_children(|row| {
                spawn_stat_pill(row, "MATCHES", &matches.to_string());
                spawn_stat_pill(row, "PLAYTIME", &playtime_display);
                spawn_stat_pill(row, "CREDITS", &currency.to_string());
            });

            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            }).with_children(|bottom| {
                bottom.spawn((
                    Button,
                    Node { padding: UiRect::new(Val::Px(16.0), Val::Px(16.0), Val::Px(8.0), Val::Px(8.0)), ..default() },
                    BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.8)),
                    ProfileLogoutButton,
                )).with_children(|btn| {
                    btn.spawn((Text::new("LOGOUT"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(Color::srgb(0.9, 0.3, 0.3))));
                });
            });
        });
    });
}

fn spawn_stat_pill(parent: &mut ChildSpawnerCommands, label: &str, value: &str) {
    parent.spawn((
        Node { flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(10.0)), flex_grow: 1.0, border: UiRect::all(Val::Px(1.0)), ..default() },
        BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.8)),
        BorderColor::all(Color::srgba(0.2, 0.2, 0.3, 0.3)),
    )).with_children(|pill| {
        pill.spawn((Text::new(value), TextFont { font_size: FontSize::Px(20.0), ..default() }, TextColor(Color::WHITE)));
        pill.spawn((Text::new(label), TextFont { font_size: FontSize::Px(9.0), ..default() }, TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7))));
    });
}

pub fn spawn_profile_overlay_system(
    commands: Commands,
    state: Res<ProfileOverlayState>,
    existing: Query<Entity, With<ProfileOverlayUi>>,
    conn_state: Res<ConnectionState>,
    cached_profile: Res<CachedProfile>,
) {
    if !state.show || !existing.is_empty() {
        return;
    }
    spawn_profile_overlay(commands, conn_state, cached_profile);
}

pub fn despawn_profile_overlay(
    mut commands: Commands,
    query: Query<Entity, With<ProfileOverlayUi>>,
    state: Res<ProfileOverlayState>,
) {
    if state.show && !query.is_empty() {
        return;
    }
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn force_despawn_profile_overlay(
    mut commands: Commands,
    query: Query<Entity, With<ProfileOverlayUi>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn request_profile_data(
    state: Res<ProfileOverlayState>,
    conn_state: Res<ConnectionState>,
    rt: Res<TokioRuntime>,
    server_config: Res<ServerConfig>,
    pending: Res<PendingRequests>,
    cached: Res<CachedProfile>,
) {
    if !state.show || cached.loaded {
        return;
    }
    if let Some(token) = conn_state.token() {
        http::spawn_http_request(&rt, &pending, http::async_get_profile(server_config.http_url.clone(), token.to_string()));
    }
}

pub fn profile_interaction(
    close_query: Query<&Interaction, (Changed<Interaction>, With<ProfileCloseButton>, With<Button>)>,
    logout_query: Query<&Interaction, (Changed<Interaction>, With<ProfileLogoutButton>, With<Button>)>,
    goto_login_query: Query<&Interaction, (Changed<Interaction>, With<ProfileGoToLoginButton>, With<Button>)>,
    mut profile_state: ResMut<ProfileOverlayState>,
    mut conn_state: ResMut<ConnectionState>,
    mut cached_profile: ResMut<CachedProfile>,
    mut login_state: ResMut<crate::menu::login::LoginUiState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    for interaction in close_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            profile_state.show = false;
        }
    }
    for interaction in logout_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            *conn_state = ConnectionState::Disconnected;
            *cached_profile = CachedProfile::default();
            profile_state.show = false;
            crate::menu::login::clear_token();
        }
    }
    for interaction in goto_login_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            profile_state.show = false;
            login_state.show_overlay = true;
            login_state.focused_field = Some(crate::menu::login::LoginField::Email);
        }
    }
}

pub fn profile_update_data(
    mut events: MessageReader<NetworkEvent>,
    mut cached_profile: ResMut<CachedProfile>,
) {
    for event in events.read() {
        if let NetworkEvent::ProfileLoaded { profile } = event {
            cached_profile.loaded = true;
            cached_profile.profile = Some(profile.clone());
        }
    }
}
