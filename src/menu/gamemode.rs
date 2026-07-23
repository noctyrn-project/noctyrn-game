use bevy::prelude::*;
use crate::player::GameState;
use crate::menu::{GameMode, SelectedGameMode};

#[derive(Component)]
pub struct GameModeMenuUi;

#[derive(Component)]
pub struct GameModeCard(GameMode);

#[derive(Component)]
pub struct GameModeBackButton;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum GameModeTabButton { Standard, Ltm }

#[derive(Resource)]
pub struct ActiveGameModeTab(GameModeTabButton);

impl Default for ActiveGameModeTab {
    fn default() -> Self { Self(GameModeTabButton::Standard) }
}

pub fn spawn_gamemode_menu(mut commands: Commands, selected_mode: Res<SelectedGameMode>, active_tab: Res<ActiveGameModeTab>) {
    let tab = active_tab.0;

    commands.spawn((
        Node { width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, padding: UiRect::all(Val::Px(40.0)), row_gap: Val::Px(12.0), ..default() },
        BackgroundColor(Color::srgb(0.03, 0.03, 0.06)),
        GameModeMenuUi,
    )).with_children(|root| {
        root.spawn((Text::new("SELECT GAME MODE"), TextFont { font_size: FontSize::Px(32.0), ..default() }, TextColor(Color::WHITE), Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() }));

        root.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(4.0), margin: UiRect::bottom(Val::Px(8.0)), ..default() }).with_children(|tab_row| {
            for (label, tab_val) in [("STANDARD", GameModeTabButton::Standard), ("LIMITED TIME", GameModeTabButton::Ltm)] {
                let is_active = tab_val == tab;
                tab_row.spawn((
                    Button, Node { width: Val::Px(160.0), height: Val::Px(36.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, border: UiRect::bottom(Val::Px(if is_active { 2.0 } else { 0.0 })), ..default() },
                    BackgroundColor(if is_active { Color::srgba(0.12, 0.15, 0.22, 1.0) } else { Color::srgba(0.06, 0.06, 0.1, 0.8) }),
                    BorderColor::all(if is_active { Color::srgba(0.4, 0.6, 1.0, 0.8) } else { Color::NONE }),
                    tab_val,
                )).with_children(|btn| {
                    btn.spawn((Text::new(label), TextFont { font_size: FontSize::Px(14.0), ..default() }, TextColor(if is_active { Color::WHITE } else { Color::srgba(0.5, 0.5, 0.6, 0.8) })));
                });
            }
        });

        let modes: Vec<GameMode> = match tab {
            GameModeTabButton::Standard => {
                let mut v: Vec<GameMode> = GameMode::competitive_modes().to_vec();
                v.push(GameMode::TestingGrounds);
                v
            }
            GameModeTabButton::Ltm => GameMode::ltm_modes().to_vec(),
        };

        for row_modes in modes.chunks(4) {
            root.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(12.0), ..default() }).with_children(|row| {
                for &mode in row_modes {
                    let is_selected = mode == selected_mode.mode;
                    let accent = mode.accent_color();
                    row.spawn((
                        Button, Node { width: Val::Px(220.0), height: Val::Px(160.0), flex_direction: FlexDirection::Column, padding: UiRect::all(Val::Px(14.0)), justify_content: JustifyContent::SpaceBetween, border: UiRect::all(Val::Px(if is_selected { 2.0 } else { 1.0 })), ..default() },
                        BackgroundColor(if is_selected { Color::srgba(0.15, 0.2, 0.3, 1.0) } else { Color::srgba(0.08, 0.08, 0.12, 0.9) }),
                        BorderColor::all(if is_selected { accent } else { Color::srgba(0.15, 0.15, 0.2, 0.5) }),
                        GameModeCard(mode),
                    )).with_children(|card| {
                        card.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() }).with_children(|top| {
                            top.spawn((Text::new(mode.short_name()), TextFont { font_size: FontSize::Px(24.0), ..default() }, TextColor(accent)));
                            top.spawn((Text::new(mode.display_name()), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(if is_selected { Color::WHITE } else { Color::srgba(0.7, 0.7, 0.7, 0.9) })));
                        });
                        card.spawn((Text::new(mode.description()), TextFont { font_size: FontSize::Px(11.0), ..default() }, TextColor(Color::srgba(0.5, 0.55, 0.6, 0.8))));
                        card.spawn(Node { flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceBetween, align_items: AlignItems::Center, ..default() }).with_children(|bottom_row| {
                            bottom_row.spawn((Text::new(mode.player_count()), TextFont { font_size: FontSize::Px(10.0), ..default() }, TextColor(Color::srgba(0.4, 0.5, 0.6, 0.7))));
                            if is_selected {
                                bottom_row.spawn((Text::new("[SELECTED]"), TextFont { font_size: FontSize::Px(10.0), ..default() }, TextColor(accent)));
                            }
                        });
                    });
                }
            });
        }

        root.spawn(Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(12.0), margin: UiRect::top(Val::Px(8.0)), ..default() }).with_children(|bottom| {
            bottom.spawn((Button, Node { width: Val::Px(140.0), height: Val::Px(42.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                BackgroundColor(Color::srgb(0.25, 0.12, 0.12)), GameModeBackButton,
            )).with_children(|btn| { btn.spawn((Text::new("BACK"), TextFont { font_size: FontSize::Px(14.0), ..default() }, TextColor(Color::WHITE))); });
        });
    });
}

pub fn despawn_gamemode_menu(mut commands: Commands, query: Query<Entity, With<GameModeMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn gamemode_interaction(
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
    for (interaction, &tab_val) in tab_query.iter() {
        if *interaction == Interaction::Pressed && tab_val != active_tab.0 {
            active_tab.0 = tab_val;
            next_state.set(GameState::GameModeSelect);
            return;
        }
    }
    for (interaction, card) in card_query.iter() {
        if *interaction == Interaction::Pressed {
            selected_mode.mode = card.0;
            next_state.set(GameState::MainMenu);
        }
    }
    for interaction in back_query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::MainMenu);
        }
    }
}

pub fn gamemode_hover(
    mut card_query: Query<(&Interaction, &mut BackgroundColor, &GameModeCard), With<Button>>,
    mut back_query: Query<(&Interaction, &mut BackgroundColor), (With<GameModeBackButton>, Without<GameModeCard>, Without<GameModeTabButton>)>,
    mut tab_btn_query: Query<(&Interaction, &mut BackgroundColor, &GameModeTabButton), (With<Button>, Without<GameModeCard>, Without<GameModeBackButton>)>,
    selected_mode: Res<SelectedGameMode>,
    active_tab: Res<ActiveGameModeTab>,
) {
    for (interaction, mut bg, card) in card_query.iter_mut() {
        let is_selected = card.0 == selected_mode.mode;
        let base = if is_selected { Color::srgba(0.15, 0.2, 0.3, 1.0) } else { Color::srgba(0.08, 0.08, 0.12, 0.9) };
        *bg = match interaction {
            Interaction::Hovered | Interaction::Pressed => BackgroundColor(Color::srgba(0.2, 0.25, 0.35, 1.0)),
            _ => BackgroundColor(base),
        };
    }
    for (interaction, mut bg) in back_query.iter_mut() {
        *bg = match interaction {
            Interaction::Hovered | Interaction::Pressed => BackgroundColor(Color::srgb(0.4, 0.18, 0.18)),
            _ => BackgroundColor(Color::srgb(0.25, 0.12, 0.12)),
        };
    }
    for (interaction, mut bg, &tab_val) in tab_btn_query.iter_mut() {
        let is_active = tab_val == active_tab.0;
        let base = if is_active { Color::srgba(0.12, 0.15, 0.22, 1.0) } else { Color::srgba(0.06, 0.06, 0.1, 0.8) };
        *bg = match interaction {
            Interaction::Hovered | Interaction::Pressed => BackgroundColor(Color::srgba(0.15, 0.2, 0.28, 1.0)),
            _ => BackgroundColor(base),
        };
    }
}
