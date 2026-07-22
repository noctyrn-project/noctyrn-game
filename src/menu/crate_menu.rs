use bevy::prelude::*;

use crate::player::GameState;
use crate::weapons::{slot_from_weapon_type, PlayerCredits, SkinInventory, SkinRarity, WeaponConfig, WeaponRegistry, WeaponSkin, WeaponSlot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrateType {
    Standard,
    Tactical,
    Elite,
    Legendary,
}

impl CrateType {
    pub fn all() -> &'static [CrateType] {
        &[CrateType::Standard, CrateType::Tactical, CrateType::Elite, CrateType::Legendary]
    }

    pub fn display_name(&self) -> &str {
        match self {
            CrateType::Standard => "Standard Crate",
            CrateType::Tactical => "Tactical Crate",
            CrateType::Elite => "Elite Crate",
            CrateType::Legendary => "Legendary Crate",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            CrateType::Standard => "Basic crate with standard drop rates.",
            CrateType::Tactical => "Better odds for uncommon and rare skins.",
            CrateType::Elite => "Guaranteed rare or better. Higher epic chances.",
            CrateType::Legendary => "Guaranteed epic or better. The best odds.",
        }
    }

    pub fn cost(&self) -> u64 {
        match self {
            CrateType::Standard => 50,
            CrateType::Tactical => 100,
            CrateType::Elite => 250,
            CrateType::Legendary => 500,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            CrateType::Standard => Color::srgb(0.3, 0.3, 0.35),
            CrateType::Tactical => Color::srgb(0.2, 0.4, 0.25),
            CrateType::Elite => Color::srgb(0.2, 0.3, 0.6),
            CrateType::Legendary => Color::srgb(0.6, 0.45, 0.1),
        }
    }

    pub fn drop_weights(&self) -> Vec<(SkinRarity, u32)> {
        match self {
            CrateType::Standard => vec![
                (SkinRarity::Common, 55),
                (SkinRarity::Uncommon, 250),
                (SkinRarity::Rare, 130),
                (SkinRarity::Epic, 60),
                (SkinRarity::Legendary, 9),
                (SkinRarity::Mythic, 1),
            ],
            CrateType::Tactical => vec![
                (SkinRarity::Common, 325),
                (SkinRarity::Uncommon, 350),
                (SkinRarity::Rare, 200),
                (SkinRarity::Epic, 100),
                (SkinRarity::Legendary, 20),
                (SkinRarity::Mythic, 5),
            ],
            CrateType::Elite => vec![
                (SkinRarity::Common, 0),
                (SkinRarity::Uncommon, 0),
                (SkinRarity::Rare, 500),
                (SkinRarity::Epic, 450),
                (SkinRarity::Legendary, 40),
                (SkinRarity::Mythic, 10),
            ],
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

    pub fn roll_skin(&self) -> WeaponSkin {
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

        let candidates: Vec<&WeaponSkin> = WeaponSkin::droppable()
            .iter()
            .filter(|s| s.rarity() == selected_rarity)
            .collect();

        if candidates.is_empty() {
            return *WeaponSkin::droppable().first().unwrap_or(&WeaponSkin::SolidRed);
        }

        let pick = (seed as usize / 7) % candidates.len();
        *candidates[pick]
    }
}

#[derive(Default, Clone, Copy, PartialEq)]
pub enum CratePhase {
    #[default]
    Idle,
    Spinning,
    Revealing,
}

#[derive(Resource, Default)]
pub struct CrateState {
    pub selected_crate: Option<CrateType>,
    pub opening_animation: f32,
    pub result_skin: Option<WeaponSkin>,
    pub result_weapon: Option<String>,
    pub strip_skins: Vec<WeaponSkin>,
    pub strip_offset: f32,
    pub strip_velocity: f32,
    pub strip_target: f32,
    pub strip_phase: CratePhase,
    pub spin_time: f32,
    pub spin_duration: f32,
    pub selected_weapon: Option<String>,
}

#[derive(Component)]
pub struct CrateMenuUi;

#[derive(Component)]
pub struct CrateSelectButton {
    pub crate_type: CrateType,
}

#[derive(Component)]
pub struct CrateBackButton;

#[derive(Component)]
pub struct CrateResultPanel;

#[derive(Component)]
pub struct CrateResultDismiss;

#[derive(Component)]
pub struct CrateStripContainer;

#[derive(Component)]
pub struct CrateStripInner;

#[derive(Component)]
pub struct CratePointerMarker;

#[derive(Component)]
pub struct CrateSkipButton;

#[derive(Component)]
pub struct SellDuplicatesButton;

#[derive(Component)]
pub struct CrateWeaponPickerButton {
    pub weapon_id: String,
}

#[derive(Component)]
pub struct CrateWeaponPickerSlotTab {
    pub slot: WeaponSlot,
}

#[derive(Component)]
pub struct CrateWeaponPickerList;

#[derive(Component)]
pub struct CrateWeaponClearButton;

#[derive(Component)]
pub struct CrateWeaponSelectButton;

#[derive(Component)]
pub struct CrateWeaponPickerOverlay;

#[derive(Resource, Default)]
pub struct CrateWeaponPickerState {
    pub active_slot: WeaponSlot,
    pub picker_open: bool,
}

#[derive(Component)]
pub struct CreditsDisplay;

pub fn spawn_crate_menu(mut commands: Commands, mut crate_state: ResMut<CrateState>, credits: Res<PlayerCredits>, inventory: Res<SkinInventory>, registry: Res<WeaponRegistry>, picker_state: Res<CrateWeaponPickerState>) {
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
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        }).with_children(|header| {
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

            header.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(12.0),
                ..default()
            }).with_children(|right| {
                right.spawn((
                    Text::new(format!("[C] {} Credits", credits.balance)),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.8, 0.2)),
                    CreditsDisplay,
                ));

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

        root.spawn((
            Text::new("Open crates to earn weapon skins. Sell duplicates for credits."),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
        ));

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

                    card.spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        column_gap: Val::Px(4.0),
                        ..default()
                    }).with_children(|sel_row| {
                        if has_weapon_selected {
                            sel_row.spawn((
                                Text::new(format!("[{}]", selected_weapon_name)),
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
                                    Text::new("X"),
                                    TextFont { font_size: 11.0, ..default() },
                                    TextColor(Color::WHITE),
                                ));
                            });
                        } else {
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

        if picker_state.picker_open {
            let active_slot = picker_state.active_slot;
            let mut weapons_in_slot: Vec<(&String, &WeaponConfig)> = registry.weapons.iter()
                .filter(|(_, cfg)| {
                    let wtype = cfg.meta.weapon_type.as_str();
                    slot_from_weapon_type(wtype) == active_slot
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
                                Text::new("X"),
                                TextFont { font_size: 14.0, ..default() },
                                TextColor(Color::WHITE),
                            ));
                        });
                    });

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

pub fn despawn_crate_menu(mut commands: Commands, query: Query<Entity, With<CrateMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn crate_interaction(
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
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if let Some(entity) = result_panel_query.iter().next() {
            commands.entity(entity).despawn();
            crate_state.opening_animation = 0.0;
            crate_state.result_skin = None;
            crate_state.result_weapon = None;

            for entity in crate_menu_query.iter() {
                commands.entity(entity).despawn();
            }
            next_state.set(GameState::MainMenu);
            return;
        } else {
            next_state.set(GameState::MainMenu);
            return;
        }
    }

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

    for (interaction, mut bg) in sell_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    let sold = inventory.sell_all_duplicates();
                    let total_credits: u64 = sold.iter().map(|(_, _, _, v)| v).sum();
                    credits.balance += total_credits;
                    credits.save();
                    inventory.save();
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

    for (interaction, crate_btn, mut bg) in crate_select_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                let base_cost = crate_btn.crate_type.cost();
                let cost = if crate_state.selected_weapon.is_some() { base_cost * 2 } else { base_cost };
                if mouse_input.just_pressed(MouseButton::Left) && crate_state.strip_phase == CratePhase::Idle && credits.balance >= cost {
                    credits.balance -= cost;
                    credits.save();

                    let skin = crate_btn.crate_type.roll_skin();
                    crate_state.result_skin = Some(skin);
                    crate_state.selected_crate = Some(crate_btn.crate_type);

                    let mut strip = Vec::new();
                    for _ in 0..60 {
                        strip.push(crate_btn.crate_type.roll_skin());
                    }
                    strip[45] = skin;
                    crate_state.strip_skins = strip;

                    let cell_width = 84.0;
                    crate_state.strip_target = 45.0 * cell_width + 42.0;
                    crate_state.strip_offset = 0.0;
                    crate_state.strip_velocity = 4000.0;
                    crate_state.strip_phase = CratePhase::Spinning;
                    crate_state.opening_animation = 0.0;
                    crate_state.spin_time = 0.0;
                    crate_state.spin_duration = 4.5;

                    if let Some(root) = crate_menu_query.iter().next() {
                        commands.entity(root).with_children(|parent| {
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
                                    container.spawn((
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            column_gap: Val::Px(4.0),
                                            height: Val::Percent(100.0),
                                            align_items: AlignItems::Center,
                                            padding: UiRect::horizontal(Val::Px(300.0)),
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

                                overlay.spawn((
                                    Node {
                                        width: Val::Px(4.0),
                                        height: Val::Px(30.0),
                                        margin: UiRect::top(Val::Px(4.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(1.0, 0.85, 0.0)),
                                ));

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
                                        Text::new("SKIP >>"),
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

pub fn update_crate_animation(
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

                    let dup_count = skin_inventory.duplicate_count(&assigned_weapon, &skin);
                    skin_inventory.add_skin(&assigned_weapon, skin);
                    skin_inventory.save();
                    crate_state.result_weapon = Some(assigned_weapon.clone());

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

                    let slot = slot_from_weapon_type(&weapon_type);

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

pub fn crate_skip_interaction(
    mut crate_state: ResMut<CrateState>,
    skip_query: Query<&Interaction, (Changed<Interaction>, With<CrateSkipButton>, With<Button>)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    for interaction in skip_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            if crate_state.strip_phase == CratePhase::Spinning {
                crate_state.spin_time = crate_state.spin_duration * 0.98;
            }
        }
    }
}

pub fn crate_weapon_picker_interaction(
    mut crate_state: ResMut<CrateState>,
    mut picker_state: ResMut<CrateWeaponPickerState>,
    mut next_state: ResMut<NextState<GameState>>,
    tab_query: Query<(&Interaction, &CrateWeaponPickerSlotTab), (Changed<Interaction>, With<Button>)>,
    weapon_btn_query: Query<(&Interaction, &CrateWeaponPickerButton), (Changed<Interaction>, With<Button>, Without<CrateWeaponPickerSlotTab>)>,
    select_btn_query: Query<&Interaction, (Changed<Interaction>, With<CrateWeaponSelectButton>, With<Button>)>,
    clear_btn_query: Query<&Interaction, (Changed<Interaction>, With<CrateWeaponClearButton>, With<Button>)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    for interaction in select_btn_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            picker_state.picker_open = true;
            next_state.set(GameState::CrateOpening);
        }
    }

    for interaction in clear_btn_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            if picker_state.picker_open {
                picker_state.picker_open = false;
            }
            crate_state.selected_weapon = None;
            next_state.set(GameState::CrateOpening);
        }
    }

    for (interaction, tab) in tab_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            picker_state.active_slot = tab.slot;
            next_state.set(GameState::CrateOpening);
        }
    }

    for (interaction, btn) in weapon_btn_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            crate_state.selected_weapon = Some(btn.weapon_id.clone());
            picker_state.picker_open = false;
            next_state.set(GameState::CrateOpening);
        }
    }
}
