use bevy::prelude::*;
use crate::net::{PartyState, ConnectionState, TokioRuntime};
use crate::net::tcp::TcpClient;

#[derive(Component)]
pub struct PartyIndicatorUi;

#[derive(Component)]
struct PartySlot { slot_index: usize, member_id: Option<uuid::Uuid> }

#[derive(Component)]
pub struct PartyKickButton { target_id: uuid::Uuid }

#[derive(Component)]
pub struct LeavePartyButton;

pub fn spawn_party_indicator(
    mut commands: Commands,
    party_state: Res<PartyState>,
    conn_state: Res<ConnectionState>,
    existing: Query<Entity, With<PartyIndicatorUi>>,
) {
    if !existing.is_empty() {
        return;
    }
    let user_id = conn_state.user_id().unwrap_or_default();
    let party = match party_state.party.as_ref() {
        Some(p) => p,
        None => return,
    };
    let is_leader = party.is_leader(user_id);

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(16.0),
            right: Val::Px(320.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(0.0),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.9)),
        BorderColor::all(Color::srgba(0.3, 0.3, 0.4, 0.4)),
        PartyIndicatorUi,
    )).with_children(|container| {
        for i in 0..4 {
            let member = party.members.get(i);
            let is_member_leader = member.map(|m| party.is_leader(m.id)).unwrap_or(false);
            let is_self = member.map(|m| m.id == user_id).unwrap_or(false);
            let initials = member.map(|m| {
                let parts: Vec<&str> = m.username.split_whitespace().collect();
                if parts.len() >= 2 {
                    format!("{}{}", &parts[0][..1], &parts[1][..1])
                } else {
                    m.username.chars().next().map(|c| c.to_string()).unwrap_or_default()
                }
            }).unwrap_or_default();

            container.spawn((
                Node {
                    width: Val::Px(70.0),
                    height: Val::Px(70.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    row_gap: Val::Px(2.0),
                    border: UiRect::all(if is_member_leader { Val::Px(2.0) } else { Val::Px(1.0) }),
                    ..default()
                },
                BackgroundColor(if member.is_some() {
                    Color::srgba(0.12, 0.12, 0.18, 0.9)
                } else {
                    Color::srgba(0.04, 0.04, 0.06, 0.6)
                }),
                BorderColor::all(if is_member_leader {
                    Color::srgba(0.9, 0.7, 0.2, 0.8)
                } else {
                    Color::srgba(0.2, 0.2, 0.3, 0.3)
                }),
                PartySlot { slot_index: i, member_id: member.map(|m| m.id) },
            )).with_children(|slot| {
                if let Some(m) = member {
                    slot.spawn(Node {
                        width: Val::Px(28.0),
                        height: Val::Px(28.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    }).insert((
                        BackgroundColor(Color::srgba(0.3, 0.3, 0.5, 0.6)),
                        BorderColor::all(Color::srgba(0.5, 0.5, 0.7, 0.4)),
                    )).with_children(|avatar| {
                        avatar.spawn((
                            Text::new(initials),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    slot.spawn((
                        Text::new(if m.username.len() > 7 {
                            format!("{}..", &m.username[..6])
                        } else {
                            m.username.clone()
                        }),
                        TextFont { font_size: 9.0, ..default() },
                        TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                    ));

                    if is_member_leader {
                        slot.spawn((
                            Text::new("*"),
                            TextFont { font_size: 10.0, ..default() },
                            TextColor(Color::srgb(0.9, 0.7, 0.2)),
                        ));
                    }

                    if is_leader && !is_self {
                        slot.spawn((
                            Button,
                            Node {
                                width: Val::Px(24.0),
                                height: Val::Px(18.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.3, 0.1, 0.1, 0.7)),
                            PartyKickButton { target_id: m.id },
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new("X"),
                                TextFont { font_size: 10.0, ..default() },
                                TextColor(Color::srgb(0.9, 0.3, 0.3)),
                            ));
                        });
                    }
                } else {
                    slot.spawn((
                        Text::new("+"),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::srgba(0.3, 0.3, 0.3, 0.4)),
                    ));
                }
            });
        }

        container.spawn((
            Button,
            Node {
                width: Val::Px(40.0),
                height: Val::Px(70.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::left(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.15, 0.05, 0.05, 0.7)),
            BorderColor::all(Color::srgba(0.3, 0.1, 0.1, 0.4)),
            LeavePartyButton,
        )).with_children(|btn| {
            btn.spawn((
                Text::new("LEAVE"),
                TextFont { font_size: 9.0, ..default() },
                TextColor(Color::srgba(0.9, 0.3, 0.3, 0.7)),
            ));
        });
    });
}

pub fn despawn_party_indicator(
    mut commands: Commands,
    query: Query<Entity, With<PartyIndicatorUi>>,
    party_state: Res<PartyState>,
) {
    if party_state.party.is_some() && !query.is_empty() {
        return;
    }
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn party_kick_handler(
    kick_query: Query<(&Interaction, &PartyKickButton), (Changed<Interaction>, With<Button>)>,
    leave_query: Query<&Interaction, (Changed<Interaction>, With<LeavePartyButton>, With<Button>)>,
    tcp: Res<TcpClient>,
    rt: Res<TokioRuntime>,
) {
    for (interaction, btn) in kick_query.iter() {
        if *interaction == Interaction::Pressed {
            let msg = noctyrn_shared::protocol::ClientMessage::PartyKick {
                member_id: btn.target_id,
            };
            let tcp = tcp.clone();
            rt.0.spawn(async move {
                let _ = tcp.send(&msg).await;
            });
        }
    }

    for interaction in leave_query.iter() {
        if *interaction == Interaction::Pressed {
            let msg = noctyrn_shared::protocol::ClientMessage::PartyLeave;
            let tcp = tcp.clone();
            rt.0.spawn(async move {
                let _ = tcp.send(&msg).await;
            });
        }
    }
}
