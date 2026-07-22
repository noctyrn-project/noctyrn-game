use bevy::prelude::*;
use crate::player::GameState;
use crate::weapons::{WeaponRegistry, WeaponSkin, SkinRarity, SkinInventory, PlayerCredits};

#[derive(Component)]
pub struct CosmeticsMenuUi;

#[derive(Component)]
pub struct CosmeticsBackButton;

#[derive(Component)]
pub struct CosmeticsSortButton;

#[derive(Component)]
pub struct CosmeticsSellButton {
    pub weapon_id: String,
    pub skin: WeaponSkin,
}

#[derive(Resource, Default)]
pub struct SellConfirmState {
    pub weapon_id: String,
    pub skin: WeaponSkin,
    pub quantity: u32,
    pub max_quantity: u32,
    pub sell_price_each: u64,
}

#[derive(Component)]
pub struct SellConfirmOverlay;

#[derive(Component)]
pub struct SellConfirmButton;

#[derive(Component)]
pub struct SellCancelButton;

#[derive(Component)]
pub struct SellQuantityText;

#[derive(Component)]
pub struct SellQuantityPlus;

#[derive(Component)]
pub struct SellQuantityMinus;

#[derive(Component)]
pub struct SellQuantityMax;

pub fn spawn_cosmetics_menu(
    mut commands: Commands,
    credits: Res<PlayerCredits>,
    inventory: Res<SkinInventory>,
    registry: Res<WeaponRegistry>,
) {
    let mut all_skins: Vec<(String, String, WeaponSkin, u32)> = Vec::new();
    for (weapon_id, skins) in &inventory.owned {
        let weapon_name = registry.weapons.get(weapon_id)
            .map(|c| c.info.name.clone())
            .unwrap_or_else(|| weapon_id.replace('_', " "));
        for (skin, count) in skins {
            if *count > 0 && *skin != WeaponSkin::Default {
                all_skins.push((weapon_id.clone(), weapon_name.clone(), *skin, *count));
            }
        }
    }
    all_skins.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| {
                let ra = a.2.rarity() as u8;
                let rb = b.2.rarity() as u8;
                ra.cmp(&rb)
            })
    });

    let mut weapon_tabs: Vec<(String, String)> = Vec::new();
    for (wid, wname, _, _) in &all_skins {
        if !weapon_tabs.iter().any(|(id, _)| id == wid) {
            weapon_tabs.push((wid.clone(), wname.clone()));
        }
    }

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(40.0)),
            row_gap: Val::Px(16.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.03, 0.03, 0.06)),
        CosmeticsMenuUi,
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
                    CosmeticsBackButton,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("BACK"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                    ));
                });

                left.spawn((
                    Text::new("COSMETICS"),
                    TextFont { font_size: 32.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            header.spawn((
                Text::new(format!("[C] {} Credits", credits.balance)),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.9, 0.8, 0.2)),
            ));
        });

        root.spawn((
            Text::new("Browse your skins. Click SELL to trade a skin for credits."),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
        ));

        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(6.0),
            ..default()
        }).with_children(|tabs| {
            tabs.spawn((
                Button,
                Node {
                    padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(6.0), Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.25, 0.35, 0.5, 0.9)),
                CosmeticsSortButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("ALL"),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            for (wid, wname) in &weapon_tabs {
                tabs.spawn((
                    Button,
                    Node {
                        padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(6.0), Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.8)),
                    CosmeticsSellButton { weapon_id: format!("__filter__{}", wid), skin: WeaponSkin::Default },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(wname.to_uppercase()),
                        TextFont { font_size: 11.0, ..default() },
                        TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                    ));
                });
            }
        });

        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(10.0),
            row_gap: Val::Px(10.0),
            justify_content: JustifyContent::Center,
            overflow: Overflow::clip_y(),
            max_height: Val::Percent(70.0),
            ..default()
        }).with_children(|grid| {
            if all_skins.is_empty() {
                grid.spawn((
                    Text::new("No skins owned yet. Open some crates!"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                    Node { margin: UiRect::top(Val::Px(40.0)), ..default() },
                ));
            }

            for (weapon_id, weapon_name, skin, count) in &all_skins {
                let rarity = skin.rarity();
                let sell_price = PlayerCredits::sell_value(rarity);

                grid.spawn(Node {
                    width: Val::Px(180.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(10.0)),
                    row_gap: Val::Px(4.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                }).with_children(|card| {
                    card.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(skin.swatch_color()),
                    )).with_children(|swatch| {
                        swatch.spawn((
                            Text::new(skin.display_name()),
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    card.spawn((
                        Text::new(rarity.display_name()),
                        TextFont { font_size: 10.0, ..default() },
                        TextColor(rarity.color()),
                    ));

                    card.spawn((
                        Text::new(weapon_name.as_str()),
                        TextFont { font_size: 11.0, ..default() },
                        TextColor(Color::srgba(0.6, 0.6, 0.7, 0.9)),
                    ));

                    if *count > 1 {
                        card.spawn((
                            Text::new(format!("Owned: x{}", count)),
                            TextFont { font_size: 10.0, ..default() },
                            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                        ));
                    }

                    card.spawn((
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 0.8)),
                        CosmeticsSellButton { weapon_id: weapon_id.clone(), skin: *skin },
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new(format!("SELL [C]{}", sell_price)),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                }).insert(BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.9)))
                  .insert(BorderColor::from(rarity.color()));
            }
        });
    });
}

pub fn despawn_cosmetics_menu(mut commands: Commands, query: Query<Entity, With<CosmeticsMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn cosmetics_interaction(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    back_query: Query<&Interaction, (Changed<Interaction>, With<CosmeticsBackButton>, With<Button>)>,
    sell_query: Query<(&Interaction, &CosmeticsSellButton), (Changed<Interaction>, With<Button>, Without<CosmeticsBackButton>)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    inventory: Res<SkinInventory>,
    mut sell_state: ResMut<SellConfirmState>,
    cosmetics_ui: Query<Entity, With<CosmeticsMenuUi>>,
    existing_confirm: Query<Entity, With<SellConfirmOverlay>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if !existing_confirm.is_empty() {
            for entity in existing_confirm.iter() {
                commands.entity(entity).despawn();
            }
            return;
        }
        next_state.set(GameState::MainMenu);
        return;
    }

    for interaction in back_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            next_state.set(GameState::MainMenu);
            return;
        }
    }

    if !existing_confirm.is_empty() {
        return;
    }

    for (interaction, sell_btn) in sell_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            if sell_btn.weapon_id.starts_with("__filter__") {
                continue;
            }
            let rarity = sell_btn.skin.rarity();
            let price = PlayerCredits::sell_value(rarity);
            let count = inventory.owned.get(&sell_btn.weapon_id)
                .and_then(|skins| skins.iter().find(|(s, _)| **s == sell_btn.skin).map(|(_, c)| *c))
                .unwrap_or(1);

            sell_state.weapon_id = sell_btn.weapon_id.clone();
            sell_state.skin = sell_btn.skin;
            sell_state.quantity = 1;
            sell_state.max_quantity = count;
            sell_state.sell_price_each = price;

            if let Some(root) = cosmetics_ui.iter().next() {
                spawn_sell_confirm_dialog(&mut commands, root, &sell_state, &sell_btn.skin);
            }
        }
    }
}

fn spawn_sell_confirm_dialog(
    commands: &mut Commands,
    root: Entity,
    sell_state: &SellConfirmState,
    skin: &WeaponSkin,
) {
    let rarity = skin.rarity();
    let total_price = sell_state.sell_price_each * sell_state.quantity as u64;

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
            SellConfirmOverlay,
            ZIndex(20),
        )).with_children(|overlay| {
            overlay.spawn((
                Node {
                    width: Val::Px(380.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    row_gap: Val::Px(14.0),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.98)),
                BorderColor::from(Color::srgba(0.5, 0.3, 0.3, 0.7)),
            )).with_children(|card| {
                card.spawn((
                    Text::new("CONFIRM SELL"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));

                card.spawn((
                    Node {
                        width: Val::Px(80.0),
                        height: Val::Px(80.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(skin.swatch_color()),
                    BorderColor::from(rarity.color()),
                )).with_children(|swatch| {
                    swatch.spawn((
                        Text::new(skin.display_name()),
                        TextFont { font_size: 11.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });

                card.spawn((
                    Text::new(format!("{} Skin", rarity.display_name())),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(rarity.color()),
                ));

                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                }).with_children(|row| {
                    row.spawn((
                        Text::new("Quantity:"),
                        TextFont { font_size: 14.0, ..default() },
                        TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
                    ));

                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.3, 0.2, 0.2, 0.9)),
                        SellQuantityMinus,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("-"),
                            TextFont { font_size: 18.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    row.spawn((
                        Text::new(format!("{}", sell_state.quantity)),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::WHITE),
                        SellQuantityText,
                    ));

                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.3, 0.2, 0.9)),
                        SellQuantityPlus,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("+"),
                            TextFont { font_size: 18.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    row.spawn((
                        Button,
                        Node {
                            padding: UiRect::horizontal(Val::Px(8.0)),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.35, 0.9)),
                        SellQuantityMax,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("MAX"),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
                        ));
                    });
                });

                card.spawn((
                    Text::new(format!("Owned: ×{}", sell_state.max_quantity)),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
                ));

                card.spawn((
                    Text::new(format!("Total: [C] {} Credits", total_price)),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.9, 0.8, 0.2)),
                ));

                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    margin: UiRect::top(Val::Px(6.0)),
                    ..default()
                }).with_children(|row| {
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(38.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.9)),
                        SellCancelButton,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("CANCEL"),
                            TextFont { font_size: 14.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });

                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(38.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 0.9)),
                        SellConfirmButton,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("SELL"),
                            TextFont { font_size: 14.0, ..default() },
                            TextColor(Color::WHITE),
                        ));
                    });
                });
            });
        });
    });
}

pub fn cosmetics_hover(
    mut back_query: Query<(&Interaction, &mut BackgroundColor), (With<CosmeticsBackButton>, With<Button>, Without<CosmeticsSellButton>)>,
    mut sell_query: Query<(&Interaction, &mut BackgroundColor, &CosmeticsSellButton), (With<Button>, Without<CosmeticsBackButton>)>,
) {
    for (interaction, mut bg) in back_query.iter_mut() {
        *bg = match interaction {
            Interaction::Pressed => BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.15)),
            Interaction::Hovered => BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.1)),
            _ => BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
        };
    }

    for (interaction, mut bg, sell_btn) in sell_query.iter_mut() {
        if sell_btn.weapon_id.starts_with("__filter__") {
            *bg = match interaction {
                Interaction::Pressed | Interaction::Hovered => BackgroundColor(Color::srgba(0.15, 0.2, 0.3, 0.9)),
                _ => BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.8)),
            };
        } else {
            *bg = match interaction {
                Interaction::Pressed => BackgroundColor(Color::srgba(0.7, 0.25, 0.25, 1.0)),
                Interaction::Hovered => BackgroundColor(Color::srgba(0.6, 0.25, 0.25, 0.9)),
                _ => BackgroundColor(Color::srgba(0.5, 0.2, 0.2, 0.8)),
            };
        }
    }
}

pub fn sell_confirm_interaction(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    mut sell_state: ResMut<SellConfirmState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    confirm_query: Query<&Interaction, (Changed<Interaction>, With<SellConfirmButton>, With<Button>)>,
    cancel_query: Query<&Interaction, (Changed<Interaction>, With<SellCancelButton>, With<Button>)>,
    plus_query: Query<&Interaction, (Changed<Interaction>, With<SellQuantityPlus>, With<Button>)>,
    minus_query: Query<&Interaction, (Changed<Interaction>, With<SellQuantityMinus>, With<Button>)>,
    max_query: Query<&Interaction, (Changed<Interaction>, With<SellQuantityMax>, With<Button>)>,
    overlay_query: Query<Entity, With<SellConfirmOverlay>>,
    mut qty_text_query: Query<&mut Text, With<SellQuantityText>>,
    mut credits: ResMut<PlayerCredits>,
    mut inventory: ResMut<SkinInventory>,
) {
    if overlay_query.is_empty() { return; }

    let mut quantity_changed = false;

    for interaction in minus_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            if sell_state.quantity > 1 {
                sell_state.quantity -= 1;
                quantity_changed = true;
            }
        }
    }

    for interaction in plus_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            if sell_state.quantity < sell_state.max_quantity {
                sell_state.quantity += 1;
                quantity_changed = true;
            }
        }
    }

    for interaction in max_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            sell_state.quantity = sell_state.max_quantity;
            quantity_changed = true;
        }
    }

    if quantity_changed {
        for mut text in qty_text_query.iter_mut() {
            text.0 = format!("{}", sell_state.quantity);
        }
    }

    for interaction in cancel_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            for entity in overlay_query.iter() {
                commands.entity(entity).despawn();
            }
        }
    }

    for interaction in confirm_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            let qty = sell_state.quantity;
            let price_each = sell_state.sell_price_each;
            let weapon_id = sell_state.weapon_id.clone();
            let skin = sell_state.skin;

            for _ in 0..qty {
                inventory.sell_skin(&weapon_id, &skin);
            }
            credits.balance += price_each * qty as u64;
            credits.save();
            inventory.save();

            for entity in overlay_query.iter() {
                commands.entity(entity).despawn();
            }
            next_state.set(GameState::Cosmetics);
        }
    }
}
