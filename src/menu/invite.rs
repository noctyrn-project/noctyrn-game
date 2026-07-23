use bevy::prelude::*;
use crate::net::{PartyState, TokioRuntime};
use crate::net::tcp::TcpClient;

#[derive(Component)]
pub struct InviteBannerUi;

#[derive(Component)]
pub struct InviteTimerText;

#[derive(Component)]
pub struct InviteAcceptButton { party_id: uuid::Uuid }

#[derive(Component)]
pub struct InviteDeclineButton { party_id: uuid::Uuid }

#[derive(Resource)]
pub struct InviteTimer {
    pub remaining: f32,
    pub active: bool,
}

impl Default for InviteTimer {
    fn default() -> Self {
        Self { remaining: 10.0, active: false }
    }
}

pub fn spawn_invite_banner(
    mut commands: Commands,
    party_state: Res<PartyState>,
    existing: Query<Entity, With<InviteBannerUi>>,
) {
    if !existing.is_empty() {
        return;
    }
    let (party_id, from_username) = match party_state.pending_invite {
        Some((id, ref name)) => (id, name.clone()),
        None => return,
    };

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(100.0),
            left: Val::Px(50.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(8.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.08, 0.08, 0.14, 0.95)),
        BorderColor::all(Color::srgba(0.4, 0.4, 0.6, 0.5)),
        InviteBannerUi,
    )).with_children(|banner| {
        banner.spawn((
            Text::new("PARTY INVITE"),
            TextFont { font_size: FontSize::Px(16.0), ..default() },
            TextColor(Color::srgba(0.8, 0.8, 0.9, 0.8)),
        ));

        banner.spawn((
            Text::new(format!("{} has invited you!", from_username)),
            TextFont { font_size: FontSize::Px(14.0), ..default() },
            TextColor(Color::WHITE),
        ));

        banner.spawn((
            Text::new("10"),
            TextFont { font_size: FontSize::Px(28.0), ..default() },
            TextColor(Color::srgba(0.9, 0.9, 0.9, 0.8)),
            InviteTimerText,
        ));

        banner.spawn((
            Text::new("Auto-decline"),
            TextFont { font_size: FontSize::Px(11.0), ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.5)),
        ));

        banner.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(12.0),
            margin: UiRect::top(Val::Px(4.0)),
            ..default()
        }).with_children(|actions| {
            actions.spawn((
                Button,
                Node { width: Val::Px(100.0), height: Val::Px(34.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                BackgroundColor(Color::srgba(0.2, 0.5, 0.2, 0.8)),
                InviteAcceptButton { party_id },
            )).with_children(|btn| {
                btn.spawn((Text::new("ACCEPT"), TextFont { font_size: FontSize::Px(14.0), ..default() }, TextColor(Color::WHITE)));
            });

            actions.spawn((
                Button,
                Node { width: Val::Px(100.0), height: Val::Px(34.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                BackgroundColor(Color::srgba(0.5, 0.15, 0.15, 0.8)),
                InviteDeclineButton { party_id },
            )).with_children(|btn| {
                btn.spawn((Text::new("DECLINE"), TextFont { font_size: FontSize::Px(14.0), ..default() }, TextColor(Color::WHITE)));
            });
        });
    });
}

pub fn despawn_invite_banner(
    mut commands: Commands,
    query: Query<Entity, With<InviteBannerUi>>,
    party_state: Res<PartyState>,
) {
    if party_state.pending_invite.is_some() && !query.is_empty() {
        return;
    }
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn invite_timer_tick(
    time: Res<Time>,
    mut timer: ResMut<InviteTimer>,
    mut party_state: ResMut<PartyState>,
    mut timer_text_query: Query<&mut Text, With<InviteTimerText>>,
    mut commands: Commands,
    banner_query: Query<Entity, With<InviteBannerUi>>,
    tcp: Res<TcpClient>,
    rt: Res<TokioRuntime>,
) {
    if party_state.pending_invite.is_none() {
        timer.active = false;
        timer.remaining = 10.0;
        return;
    }

    if !timer.active {
        timer.active = true;
        timer.remaining = 10.0;
    }

    timer.remaining -= time.delta_secs();
    let display = timer.remaining.ceil().max(0.0) as u32;

    for mut text in timer_text_query.iter_mut() {
        **text = display.to_string();
    }

    if timer.remaining <= 0.0 {
        if let Some((party_id, _)) = party_state.pending_invite {
            let msg = noctyrn_shared::protocol::ClientMessage::PartyDeclineInvite { party_id };
            let tcp = tcp.clone();
            rt.0.spawn(async move {
                let _ = tcp.send(&msg).await;
            });
        }
        party_state.pending_invite = None;
        timer.active = false;
        timer.remaining = 10.0;
        for entity in banner_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn invite_accept_handler(
    interaction_query: Query<(&Interaction, &InviteAcceptButton), (Changed<Interaction>, With<Button>)>,
    mut commands: Commands,
    mut party_state: ResMut<PartyState>,
    banner_query: Query<Entity, With<InviteBannerUi>>,
    tcp: Res<TcpClient>,
    rt: Res<TokioRuntime>,
) {
    for (interaction, btn) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            let msg = noctyrn_shared::protocol::ClientMessage::PartyAcceptInvite {
                party_id: btn.party_id,
            };
            let tcp = tcp.clone();
            rt.0.spawn(async move {
                let _ = tcp.send(&msg).await;
            });

            party_state.pending_invite = None;
            for entity in banner_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn invite_decline_handler(
    interaction_query: Query<(&Interaction, &InviteDeclineButton), (Changed<Interaction>, With<Button>)>,
    mut commands: Commands,
    mut party_state: ResMut<PartyState>,
    banner_query: Query<Entity, With<InviteBannerUi>>,
    tcp: Res<TcpClient>,
    rt: Res<TokioRuntime>,
) {
    for (interaction, btn) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            let msg = noctyrn_shared::protocol::ClientMessage::PartyDeclineInvite {
                party_id: btn.party_id,
            };
            let tcp = tcp.clone();
            rt.0.spawn(async move {
                let _ = tcp.send(&msg).await;
            });

            party_state.pending_invite = None;
            for entity in banner_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}
