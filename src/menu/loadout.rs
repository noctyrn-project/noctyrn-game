use bevy::prelude::*;
use crate::player::GameState;
use crate::weapons::{
    WeaponRegistry, WeaponSlot, PlayerLoadout, WeaponConfig,
    sync_loadout_to_configs, WeaponSkin, WeaponSkinTag,
    SkinInventory,
};
use crate::menu::MenuCamera;

#[derive(Resource)]
pub struct LoadoutUiState {
    pub active_slot: WeaponSlot,
    pub active_category: Option<String>,
    pub selected_weapon_id: Option<String>,
    pub selected_skin: WeaponSkin,
    pub preview_needs_update: bool,
    pub last_weapon_click: Option<(String, f64)>,
}

impl Default for LoadoutUiState {
    fn default() -> Self {
        Self {
            active_slot: WeaponSlot::Primary,
            active_category: None,
            selected_weapon_id: None,
            selected_skin: WeaponSkin::Default,
            preview_needs_update: false,
            last_weapon_click: None,
        }
    }
}

#[derive(Resource)]
pub struct LoadoutDragState {
    pub dragging: bool,
    pub last_pos: Vec2,
    pub rotation_y: f32,
    pub rotation_x: f32,
    pub zoom: f32,
}

impl Default for LoadoutDragState {
    fn default() -> Self {
        Self {
            dragging: false,
            last_pos: Vec2::ZERO,
            rotation_y: 0.0,
            rotation_x: 0.0,
            zoom: 2.5,
        }
    }
}

#[derive(Component)]
pub struct LoadoutMenuUi;

#[derive(Component)]
pub struct LoadoutBackButton;

#[derive(Component)]
pub struct SlotTabButton {
    pub slot: WeaponSlot,
}

#[derive(Component)]
pub struct CategoryButton {
    pub category: String,
}

#[derive(Component)]
pub struct WeaponSelectButton {
    pub weapon_id: String,
}

#[derive(Component)]
pub struct EquipButton;

#[derive(Component)]
pub struct WeaponListContainer;

#[derive(Component)]
pub struct CategoryTabContainer;

#[derive(Component)]
pub struct WeaponStatsPanel;

#[derive(Component)]
pub struct CurrentLoadoutDisplay;

#[derive(Component)]
pub struct SkinButton {
    pub skin: WeaponSkin,
}

#[derive(Component)]
pub struct ColorPickerButton;

#[derive(Component)]
pub struct ColorPickerPanel;

#[derive(Component)]
pub struct ColorPickerCloseButton;

#[derive(Component)]
pub struct SkinPanel;

#[derive(Component)]
pub struct LoadoutPreviewCamera;

#[derive(Component)]
pub struct LoadoutPreviewModel;

#[derive(Component)]
pub struct LoadoutPreviewLight;

const PREVIEW_ORIGIN: Vec3 = Vec3::new(500.0, 500.0, 500.0);

pub fn setup_loadout_scene(
    mut commands: Commands,
    existing_menu_cam: Query<Entity, With<MenuCamera>>,
    mut drag_state: ResMut<LoadoutDragState>,
) {
    for entity in existing_menu_cam.iter() {
        commands.entity(entity).despawn();
    }
    *drag_state = LoadoutDragState::default();

    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.12, 0.12, 0.18)),
            ..default()
        },
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(0.0, 0.3, 2.5))
            .looking_at(PREVIEW_ORIGIN + Vec3::new(0.0, 0.1, 0.0), Vec3::Y),
        LoadoutPreviewCamera,
    ));

    commands.spawn((
        PointLight {
            color: Color::srgb(0.95, 0.95, 1.0),
            intensity: 50_000.0,
            range: 20.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(2.0, 3.0, 3.0)),
        LoadoutPreviewLight,
    ));
    commands.spawn((
        PointLight {
            color: Color::srgb(0.4, 0.5, 0.8),
            intensity: 20_000.0,
            range: 15.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(-2.0, 1.0, -1.0)),
        LoadoutPreviewLight,
    ));
}

pub fn cleanup_loadout_scene(
    mut commands: Commands,
    camera_query: Query<Entity, With<LoadoutPreviewCamera>>,
    model_query: Query<Entity, With<LoadoutPreviewModel>>,
    light_query: Query<Entity, With<LoadoutPreviewLight>>,
) {
    for entity in camera_query.iter() { commands.entity(entity).despawn(); }
    for entity in model_query.iter() { commands.entity(entity).despawn(); }
    for entity in light_query.iter() { commands.entity(entity).despawn(); }
}

pub fn spawn_loadout_menu(
    mut commands: Commands,
    registry: Res<WeaponRegistry>,
    loadout: Res<PlayerLoadout>,
    mut ui_state: ResMut<LoadoutUiState>,
) {
    ui_state.active_slot = WeaponSlot::Primary;
    ui_state.active_category = None;
    ui_state.selected_weapon_id = Some(loadout.primary.clone());
    ui_state.selected_skin = loadout.get_skin(WeaponSlot::Primary);
    ui_state.preview_needs_update = true;

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        LoadoutMenuUi,
    )).with_children(|root| {
        root.spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(56.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(20.0)),
            ..default()
        }).with_children(|bar| {
            bar.spawn((
                Button,
                Node {
                    width: Val::Px(90.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
                LoadoutBackButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("BACK"),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                ));
            });

            bar.spawn((
                Text::new("LOADOUT"),
                TextFont { font_size: FontSize::Px(28.0), ..default() },
                TextColor(Color::WHITE),
                Node { margin: UiRect::left(Val::Px(16.0)), ..default() },
            ));

            bar.spawn(Node { flex_grow: 1.0, ..default() });

            bar.spawn((
                Text::new(format_loadout_summary(&loadout, &registry)),
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(Color::srgba(0.5, 0.7, 0.5, 0.8)),
                CurrentLoadoutDisplay,
            ));
        });

        root.spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Px(44.0),
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(2.0),
            padding: UiRect::horizontal(Val::Px(20.0)),
            align_items: AlignItems::End,
            ..default()
        }).with_children(|tabs| {
            for (label, slot) in [
                ("PRIMARY", WeaponSlot::Primary),
                ("SECONDARY", WeaponSlot::Secondary),
                ("MELEE", WeaponSlot::Melee),
                ("EQUIPMENT", WeaponSlot::Equipment),
            ] {
                let is_active = slot == ui_state.active_slot;
                tabs.spawn((
                    Button,
                    Node {
                        width: Val::Px(130.0),
                        height: Val::Px(if is_active { 38.0 } else { 34.0 }),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if is_active {
                        Color::srgba(0.2, 0.35, 0.55, 0.9)
                    } else {
                        Color::srgba(0.12, 0.12, 0.18, 0.7)
                    }),
                    SlotTabButton { slot },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextFont { font_size: FontSize::Px(12.0), ..default() },
                        TextColor(if is_active { Color::WHITE } else { Color::srgba(0.6, 0.6, 0.6, 0.8) }),
                    ));
                });
            }
        });

        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        }).with_children(|main_area| {
            main_area.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::NoWrap,
                    padding: UiRect::new(Val::Px(20.0), Val::Px(20.0), Val::Px(4.0), Val::Px(4.0)),
                    column_gap: Val::Px(2.0),
                    overflow: Overflow::scroll_x(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.95)),
                CategoryTabContainer,
            ));

            main_area.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                ..default()
            }).with_children(|content| {
                content.spawn((
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::scroll_y(),
                        row_gap: Val::Px(2.0),
                        padding: UiRect::all(Val::Px(8.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.95)),
                    WeaponListContainer,
                ));

                content.spawn(Node {
                    flex_grow: 1.0,
                    height: Val::Percent(100.0),
                    ..default()
                });

                content.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(20.0),
                        top: Val::Px(12.0),
                        width: Val::Px(320.0),
                        max_height: Val::Percent(70.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(12.0)),
                        row_gap: Val::Px(4.0),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.80)),
                    WeaponStatsPanel,
                ));
            });
        });
    });
}

pub fn despawn_loadout_menu(mut commands: Commands, query: Query<Entity, With<LoadoutMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn category_display_name(category: &str) -> &str {
    match category {
        "assault" => "Assault Rifles",
        "carbine" => "Carbines",
        "smg" => "SMGs",
        "pdw" => "PDWs",
        "lmg" => "LMBGs",
        "dmr" => "DMRs",
        "sniper" => "Snipers",
        "shotgun" => "Shotguns",
        "rifle" => "Rifles",
        "pistol" => "Pistols",
        "revolver" => "Revolvers",
        "mpistol" => "Machine Pistols",
        "blade" => "Blades",
        "2hblade" => "Two-Handed",
        "grenade" => "Grenades",
        "other" => "Special",
        _ => category,
    }
}

fn category_sort_order(category: &str) -> u32 {
    match category {
        "assault" => 0,
        "carbine" => 1,
        "smg" => 2,
        "pdw" => 3,
        "lmg" => 4,
        "dmr" => 5,
        "sniper" => 6,
        "shotgun" => 7,
        "rifle" => 8,
        "pistol" => 0,
        "revolver" => 1,
        "mpistol" => 2,
        "blade" => 0,
        "2hblade" => 1,
        "grenade" => 0,
        "other" => 10,
        _ => 9,
    }
}

pub fn update_loadout_ui(
    mut commands: Commands,
    registry: Res<WeaponRegistry>,
    loadout: Res<PlayerLoadout>,
    ui_state: Res<LoadoutUiState>,
    list_query: Query<Entity, With<WeaponListContainer>>,
    stats_query: Query<Entity, With<WeaponStatsPanel>>,
    cat_tab_query: Query<Entity, With<CategoryTabContainer>>,
    mut display_query: Query<&mut Text, With<CurrentLoadoutDisplay>>,
    color_panel_query: Query<Entity, With<ColorPickerPanel>>,
) {
    if !ui_state.is_changed() {
        return;
    }

    for mut text in display_query.iter_mut() {
        text.0 = format_loadout_summary(&loadout, &registry);
    }

    let slot = ui_state.active_slot;

    let mut categories: Vec<(String, Vec<String>)> = Vec::new();
    if let Some(weapon_ids) = registry.by_slot.get(&slot) {
        let mut cat_map: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
        for id in weapon_ids {
            if let Some(config) = registry.weapons.get(id) {
                let cat = if config.meta.category.is_empty() {
                    match config.meta.weapon_type.as_str() {
                        "Primary" | "Assault Rifle" => "assault",
                        "Secondary" | "Pistol" => "pistol",
                        "Melee" | "1 Handed Sharp" | "Blade" => "blade",
                        "2H Blade" => "2hblade",
                        "Grenade" | "Equipment" => "grenade",
                        "Revolver" => "revolver",
                        "Machine Pistol" => "mpistol",
                        _ => "other",
                    }.to_string()
                } else {
                    config.meta.category.clone()
                };
                cat_map.entry(cat).or_default().push(id.clone());
            }
        }
        let mut sorted: Vec<_> = cat_map.into_iter().collect();
        sorted.sort_by_key(|(cat, _)| category_sort_order(cat));
        categories = sorted;
    }

    if ui_state.active_category.is_none() && !categories.is_empty() {
    }
    let active_cat = ui_state.active_category.clone().or_else(|| categories.first().map(|(c, _)| c.clone()));

    if let Some(cat_container) = cat_tab_query.iter().next() {
        commands.entity(cat_container).despawn_children();
        commands.entity(cat_container).with_children(|parent| {
            for (category, _ids) in &categories {
                let is_active = active_cat.as_deref() == Some(category.as_str());
                parent.spawn((
                    Button,
                    Node {
                        padding: UiRect::new(Val::Px(10.0), Val::Px(10.0), Val::Px(5.0), Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(if is_active {
                        Color::srgba(0.2, 0.35, 0.55, 0.9)
                    } else {
                        Color::srgba(0.1, 0.1, 0.15, 0.7)
                    }),
                    CategoryButton { category: category.clone() },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(category_display_name(category).to_uppercase()),
                        TextFont { font_size: FontSize::Px(10.0), ..default() },
                        TextColor(if is_active { Color::WHITE } else { Color::srgba(0.6, 0.6, 0.6, 0.8) }),
                    ));
                });
            }
        });
    }

    if let Some(list_entity) = list_query.iter().next() {
        commands.entity(list_entity).despawn_children();

        if let Some(ref active) = active_cat {
            if let Some((_, ids)) = categories.iter().find(|(c, _)| c == active) {
                commands.entity(list_entity).with_children(|parent| {
                    for id in ids {
                        let config = registry.weapons.get(id).unwrap();
                        let is_equipped = loadout.get_id_for_slot(slot) == id.as_str();
                        let is_selected = ui_state.selected_weapon_id.as_deref() == Some(id.as_str());

                        let bg = if is_selected {
                            Color::srgba(0.2, 0.35, 0.55, 0.5)
                        } else if is_equipped {
                            Color::srgba(0.12, 0.25, 0.12, 0.35)
                        } else {
                            Color::srgba(0.1, 0.1, 0.14, 0.2)
                        };

                        parent.spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(32.0),
                                padding: UiRect::horizontal(Val::Px(10.0)),
                                align_items: AlignItems::Center,
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                margin: UiRect::bottom(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(bg),
                            WeaponSelectButton { weapon_id: id.to_string() },
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new(&config.info.name),
                                TextFont { font_size: FontSize::Px(13.0), ..default() },
                                TextColor(if is_selected { Color::WHITE } else { Color::srgba(0.85, 0.85, 0.85, 0.9) }),
                            ));
                            if is_equipped {
                                btn.spawn((
                                    Text::new("[E]"),
                                    TextFont { font_size: FontSize::Px(10.0), ..default() },
                                    TextColor(Color::srgb(0.3, 0.8, 0.3)),
                                ));
                            }
                        });
                    }
                });
            }
        }
    }

    if let Some(stats_entity) = stats_query.iter().next() {
        commands.entity(stats_entity).despawn_children();

        if let Some(weapon_id) = &ui_state.selected_weapon_id {
            if let Some(config) = registry.weapons.get(weapon_id) {
                commands.entity(stats_entity).with_children(|parent| {
                    spawn_weapon_stats(parent, config, weapon_id, &loadout, ui_state.active_slot);

                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.4)),
                    ));

                    parent.spawn((
                        Button,
                        Node {
                            width: Val::Px(200.0),
                            height: Val::Px(36.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            align_self: AlignSelf::Center,
                            column_gap: Val::Px(8.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.15, 0.15, 0.25, 0.9)),
                        ColorPickerButton,
                    )).with_children(|btn| {
                        btn.spawn((
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(18.0),
                                ..default()
                            },
                            BackgroundColor(ui_state.selected_skin.swatch_color()),
                        ));
                        btn.spawn((
                            Text::new(format!("COLOR: {}", ui_state.selected_skin.display_name().to_uppercase())),
                            TextFont { font_size: FontSize::Px(12.0), ..default() },
                            TextColor(Color::srgba(0.8, 0.8, 0.9, 0.9)),
                        ));
                    });
                });
            }
        }
    }

    for entity in color_panel_query.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn update_loadout_tabs(
    mut tab_query: Query<(&SlotTabButton, &mut BackgroundColor, &mut Node, &Children), With<Button>>,
    mut text_query: Query<&mut TextColor>,
    ui_state: Res<LoadoutUiState>,
) {
    for (tab, mut bg, mut node, children) in tab_query.iter_mut() {
        let is_active = tab.slot == ui_state.active_slot;
        node.height = Val::Px(if is_active { 38.0 } else { 34.0 });
        bg.0 = if is_active {
            Color::srgba(0.2, 0.35, 0.55, 0.9)
        } else {
            Color::srgba(0.12, 0.12, 0.18, 0.7)
        };
        for &child in children {
            if let Ok(mut text_color) = text_query.get_mut(child) {
                text_color.0 = if is_active {
                    Color::WHITE
                } else {
                    Color::srgba(0.6, 0.6, 0.6, 0.8)
                };
            }
        }
    }
}

fn spawn_weapon_stats(parent: &mut ChildSpawnerCommands, config: &WeaponConfig, weapon_id: &str, loadout: &PlayerLoadout, slot: WeaponSlot) {
    parent.spawn((
        Text::new(&config.info.name),
        TextFont { font_size: FontSize::Px(26.0), ..default() },
        TextColor(Color::WHITE),
    ));

    parent.spawn((
        Text::new(format!("{} - {} - {}", config.meta.weapon_type, config.info.manufacturer, config.info.year_introduced)),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
    ));

    parent.spawn((
        Text::new(&config.info.description),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(Color::srgba(0.7, 0.7, 0.7, 0.7)),
        Node { margin: UiRect::vertical(Val::Px(6.0)), ..default() },
    ));

    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.5)),
    ));

    let wt = config.meta.weapon_type.as_str();
    if wt == "Melee" || wt == "2H Blade" || wt == "Blade" {
        spawn_stat_bar(parent, "Attack Speed", config.attributes.attack_speed, 2.0);
        spawn_stat_bar(parent, "Stab Damage", config.attributes.stab_damage, 100.0);
        spawn_stat_bar(parent, "Slash Damage", config.attributes.slash_damage, 80.0);
        spawn_stat_bar(parent, "Reach", config.attributes.reach, 3.0);
        spawn_stat_bar(parent, "Mobility", config.attributes.mobility, 1.0);
    } else if wt == "Grenade" || wt == "Equipment" {
        spawn_stat_bar(parent, "Blast Damage", config.attributes.blast_damage, 200.0);
        spawn_stat_bar(parent, "Blast Radius", config.attributes.blast_radius, 10.0);
        spawn_stat_bar(parent, "Detonation Time", config.attributes.detonation_time, 5.0);
        spawn_stat_bar(parent, "Weight", config.attributes.weight, 1.0);
    } else {
        spawn_stat_bar(parent, "Fire Rate", config.attributes.fire_rate, 0.3);
        spawn_stat_bar(parent, "Accuracy", config.attributes.accuracy, 1.0);
        spawn_stat_bar(parent, "Stability", config.attributes.stability, 1.0);
        spawn_stat_bar(parent, "Mobility", config.attributes.mobility, 1.0);
        spawn_stat_bar(parent, "Reload Speed", config.attributes.reload_speed, 4.0);
        spawn_stat_bar(parent, "ADS Speed", config.attributes.ads_speed, 1.0);

        if let Some(ammo) = &config.attachments.ammo {
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    margin: UiRect::vertical(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.3)),
            ));
            parent.spawn((
                Text::new(format!("Ammo: {} | Damage: {:.0} | Pen: {:.0}%", ammo.name, ammo.damage, ammo.penetration * 100.0)),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(Color::srgba(0.8, 0.7, 0.4, 0.9)),
            ));
        }

        if let Some(mag) = &config.attachments.magazine {
            parent.spawn((
                Text::new(format!("Magazine: {} rds | Reserve: {}", mag.capacity, mag.carry_capacity)),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(Color::srgba(0.6, 0.7, 0.8, 0.9)),
            ));
        }

        if !config.attributes.fire_modes.is_empty() {
            parent.spawn((
                Text::new(format!("Fire Modes: {}", config.attributes.fire_modes.join(" / "))),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(Color::srgba(0.6, 0.7, 0.8, 0.9)),
            ));
        }
    }

    let is_equipped = loadout.get_id_for_slot(slot) == weapon_id;
    parent.spawn(Node {
        flex_grow: 1.0,
        ..default()
    });

    parent.spawn((
        Button,
        Node {
            width: Val::Px(200.0),
            height: Val::Px(45.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            align_self: AlignSelf::Center,
            ..default()
        },
        BackgroundColor(if is_equipped {
            Color::srgb(0.2, 0.5, 0.2)
        } else {
            Color::srgb(0.2, 0.35, 0.6)
        }),
        EquipButton,
    )).with_children(|btn: &mut ChildSpawnerCommands| {
        btn.spawn((
            Text::new(if is_equipped { "EQUIPPED" } else { "EQUIP" }),
            TextFont { font_size: FontSize::Px(18.0), ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn spawn_stat_bar(parent: &mut ChildSpawnerCommands, label: &str, value: f32, _max: f32) {
    parent.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        column_gap: Val::Px(10.0),
        ..default()
    }).with_children(|row: &mut ChildSpawnerCommands| {
        row.spawn((
            Text::new(label),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(Color::srgba(0.7, 0.7, 0.7, 0.9)),
            Node { width: Val::Px(120.0), ..default() },
        ));

        row.spawn((
            Text::new(format!("{:.2}", value)),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

pub fn loadout_interaction(
    mut next_state: ResMut<NextState<GameState>>,
    mut ui_state: ResMut<LoadoutUiState>,
    mut loadout: ResMut<PlayerLoadout>,
    mut registry: ResMut<WeaponRegistry>,
    slot_query: Query<(&Interaction, &SlotTabButton), (Changed<Interaction>, With<Button>)>,
    weapon_query: Query<(&Interaction, &WeaponSelectButton), (Changed<Interaction>, With<Button>)>,
    skin_query: Query<(&Interaction, &SkinButton), (Changed<Interaction>, With<Button>)>,
    category_query: Query<(&Interaction, &CategoryButton), (Changed<Interaction>, With<Button>)>,
    mut btn_queries: ParamSet<(
        Query<(&Interaction, &mut BackgroundColor), (With<LoadoutBackButton>, With<Button>, Without<EquipButton>, Without<ColorPickerButton>, Without<ColorPickerCloseButton>)>,
        Query<(&Interaction, &mut BackgroundColor), (With<EquipButton>, With<Button>, Without<LoadoutBackButton>, Without<ColorPickerButton>, Without<ColorPickerCloseButton>)>,
        Query<(&Interaction, &mut BackgroundColor), (With<ColorPickerButton>, With<Button>, Without<LoadoutBackButton>, Without<EquipButton>, Without<ColorPickerCloseButton>)>,
        Query<(&Interaction, &mut BackgroundColor), (With<ColorPickerCloseButton>, With<Button>, Without<LoadoutBackButton>, Without<EquipButton>, Without<ColorPickerButton>)>,
    )>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    existing_color_panel: Query<Entity, With<ColorPickerPanel>>,
    loadout_ui_query: Query<Entity, With<LoadoutMenuUi>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::MainMenu);
        return;
    }

    for (interaction, mut bg) in btn_queries.p0().iter_mut() {
        match interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    next_state.set(GameState::MainMenu);
                }
                *bg = BackgroundColor(Color::srgb(0.45, 0.2, 0.2));
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgb(0.45, 0.2, 0.2));
            }
            _ => {
                *bg = BackgroundColor(Color::srgb(0.3, 0.15, 0.15));
            }
        }
    }

    for (interaction, mut bg) in btn_queries.p1().iter_mut() {
        let is_equipped = if let Some(id) = &ui_state.selected_weapon_id {
            loadout.get_id_for_slot(ui_state.active_slot) == id.as_str()
        } else {
            false
        };

        match interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    if let Some(id) = ui_state.selected_weapon_id.clone() {
                        loadout.set_id_for_slot(ui_state.active_slot, id.clone());
                        sync_loadout_to_configs(&mut registry, &loadout);
                        loadout.save();
                        ui_state.preview_needs_update = true;
                    }
                }
                *bg = if is_equipped {
                    BackgroundColor(Color::srgb(0.25, 0.6, 0.25))
                } else {
                    BackgroundColor(Color::srgb(0.25, 0.4, 0.7))
                };
            }
            Interaction::Hovered => {
                *bg = if is_equipped {
                    BackgroundColor(Color::srgb(0.25, 0.6, 0.25))
                } else {
                    BackgroundColor(Color::srgb(0.25, 0.4, 0.7))
                };
            }
            _ => {
                *bg = if is_equipped {
                    BackgroundColor(Color::srgb(0.2, 0.5, 0.2))
                } else {
                    BackgroundColor(Color::srgb(0.2, 0.35, 0.6))
                };
            }
        }
    }

    for (interaction, tab) in slot_query.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.active_slot = tab.slot;
            ui_state.active_category = None;
            ui_state.selected_weapon_id = Some(loadout.get_id_for_slot(tab.slot).to_string());
            ui_state.selected_skin = loadout.get_skin(tab.slot);
            ui_state.preview_needs_update = true;
        }
    }

    for (interaction, cat_btn) in category_query.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.active_category = Some(cat_btn.category.clone());
        }
    }

    for (interaction, weapon_btn) in weapon_query.iter() {
        if *interaction == Interaction::Pressed {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            let is_double_click = if let Some((ref last_id, last_time)) = ui_state.last_weapon_click {
                last_id == &weapon_btn.weapon_id && (now - last_time) < 0.4
            } else {
                false
            };

            if is_double_click {
                let id = weapon_btn.weapon_id.clone();
                loadout.set_id_for_slot(ui_state.active_slot, id);
                sync_loadout_to_configs(&mut registry, &loadout);
                loadout.save();
                ui_state.preview_needs_update = true;
                ui_state.last_weapon_click = None;
            } else {
                ui_state.selected_weapon_id = Some(weapon_btn.weapon_id.clone());
                ui_state.preview_needs_update = true;
                ui_state.last_weapon_click = Some((weapon_btn.weapon_id.clone(), now));
            }
        }
    }

    for (interaction, skin_btn) in skin_query.iter() {
        if *interaction == Interaction::Pressed {
            if mouse_input.just_pressed(MouseButton::Left) {
                ui_state.selected_skin = skin_btn.skin;
                loadout.set_skin(ui_state.active_slot, skin_btn.skin);
                loadout.save();
                ui_state.preview_needs_update = true;
                for entity in existing_color_panel.iter() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    for (interaction, mut bg) in btn_queries.p2().iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    let has_panel = !existing_color_panel.is_empty();
                    for entity in existing_color_panel.iter() {
                        commands.entity(entity).despawn();
                    }
                    if !has_panel {
                        if let Some(root_entity) = loadout_ui_query.iter().next() {
                            let selected_skin = ui_state.selected_skin;
                            let weapon_id = ui_state.selected_weapon_id.clone().unwrap_or_default();
                            let owned_skins = SkinInventory::load().owned_skins_for(&weapon_id);
                            commands.entity(root_entity).with_children(|root| {
                                let att_names = if let Some(wid) = &ui_state.selected_weapon_id {
                                    registry.weapons.get(wid).map(|c| attachment_slot_names(&c.attachments)).unwrap_or_default()
                                } else {
                                    Vec::new()
                                };
                                spawn_color_picker_panel(root, selected_skin, &owned_skins, &att_names);
                            });
                        }
                    }
                }
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.35, 0.9));
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgba(0.18, 0.18, 0.3, 0.9));
            }
            _ => {
                *bg = BackgroundColor(Color::srgba(0.15, 0.15, 0.25, 0.9));
            }
        }
    }

    for (interaction, _bg) in btn_queries.p3().iter_mut() {
        if *interaction == Interaction::Pressed {
            if mouse_input.just_pressed(MouseButton::Left) {
                for entity in existing_color_panel.iter() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

fn attachment_slot_names(att: &crate::weapons::WeaponAttachments) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Some(o) = &att.optic { result.push(("Optic".to_string(), o.name.clone())); }
    if let Some(b) = &att.barrel { result.push(("Barrel".to_string(), b.name.clone())); }
    if let Some(u) = &att.underbarrel { result.push(("Underbarrel".to_string(), u.name.clone())); }
    if let Some(s) = &att.sidebarrel { result.push(("Sidebarrel".to_string(), s.name.clone())); }
    if let Some(m) = &att.magazine { result.push(("Magazine".to_string(), m.name.clone())); }
    if let Some(a) = &att.ammo { result.push(("Ammo".to_string(), a.name.clone())); }
    if let Some(st) = &att.stock { result.push(("Stock".to_string(), st.name.clone())); }
    result
}

fn spawn_color_picker_panel(parent: &mut ChildSpawnerCommands, selected_skin: WeaponSkin, owned_skins: &[WeaponSkin], _attachment_info: &[(String, String)]) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(360.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(10.0),
            margin: UiRect::new(Val::Px(-180.0), Val::Auto, Val::Px(-200.0), Val::Auto),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.98)),
        ColorPickerPanel,
        ZIndex(10),
    )).with_children(|panel| {
        panel.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        }).with_children(|row| {
            row.spawn((
                Text::new("COLOR"),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(Color::WHITE),
            ));
            row.spawn((
                Button,
                Node {
                    width: Val::Px(28.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.3, 0.15, 0.15, 0.8)),
                ColorPickerCloseButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("X"),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });

        panel.spawn((
            Text::new("GUN BODY"),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(Color::srgba(0.5, 0.6, 0.8, 0.9)),
        ));

        panel.spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(6.0),
                row_gap: Val::Px(6.0),
                ..default()
            },
            SkinPanel,
        )).with_children(|row| {
            for skin in owned_skins {
                let is_active = *skin == selected_skin;
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
                        ..default()
                    },
                    BackgroundColor((*skin).swatch_color()),
                    BorderColor::from(if is_active { Color::WHITE } else { Color::srgba(0.4, 0.4, 0.4, 0.5) }),
                    SkinButton { skin: *skin },
                )).with_children(|btn| {
                    if is_active {
                        btn.spawn((
                            Node {
                                width: Val::Px(10.0),
                                height: Val::Px(10.0),
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                        ));
                    }
                });
            }
        });

        panel.spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::bottom(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8)),
            BorderColor::all(Color::srgba(0.4, 0.4, 0.5, 0.5)),
        )).with_children(|btn| {
            btn.spawn((
                Text::new("ATTACHMENTS"),
                TextFont { font_size: FontSize::Px(13.0), ..default() },
                TextColor(Color::srgba(0.8, 0.8, 0.9, 1.0)),
            ));
        });

        panel.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.4)),
        ));

        panel.spawn((
            Text::new("SKINS"),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(Color::srgba(0.5, 0.6, 0.8, 0.9)),
        ));

        panel.spawn(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(4.0),
            row_gap: Val::Px(4.0),
            margin: UiRect::bottom(Val::Px(8.0)),
            ..default()
        }).with_children(|grid| {
            for skin in owned_skins {
                let is_active = *skin == selected_skin;
                grid.spawn((
                    Button,
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
                        ..default()
                    },
                    BackgroundColor((*skin).swatch_color()),
                    BorderColor::from(if is_active { Color::WHITE } else { Color::srgba(0.4, 0.4, 0.4, 0.5) }),
                    SkinButton { skin: *skin },
                )).with_children(|btn| {
                    if is_active {
                        btn.spawn((
                            Node {
                                width: Val::Px(10.0),
                                height: Val::Px(10.0),
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                        ));
                    }
                });
            }
        });

        panel.spawn((
            Text::new(format!("Selected: {}", selected_skin.display_name())),
            TextFont { font_size: FontSize::Px(11.0), ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.7)),
        ));
    });
}

fn format_loadout_summary(loadout: &PlayerLoadout, registry: &WeaponRegistry) -> String {
    let name = |id: &str| registry.weapons.get(id).map(|c| c.info.name.as_str()).unwrap_or("???");
    format!(
        "P: {} | S: {} | M: {} | E: {}",
        name(&loadout.primary),
        name(&loadout.secondary),
        name(&loadout.melee),
        name(&loadout.equipment),
    )
}

pub fn handle_loadout_drag(
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut drag_state: ResMut<LoadoutDragState>,
    mut model_query: Query<&mut Transform, With<LoadoutPreviewModel>>,
    mut camera_query: Query<&mut Transform, (With<LoadoutPreviewCamera>, Without<LoadoutPreviewModel>)>,
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    time: Res<Time>,
) {
    let Ok(window) = windows.single() else { return };

    if mouse_input.just_pressed(MouseButton::Left) {
        if let Some(pos) = window.cursor_position() {
            if pos.x > 280.0 {
                drag_state.dragging = true;
                drag_state.last_pos = pos;
            }
        }
    }
    if mouse_input.just_released(MouseButton::Left) {
        drag_state.dragging = false;
    }

    if drag_state.dragging {
        if let Some(pos) = window.cursor_position() {
            let delta = pos - drag_state.last_pos;
            drag_state.rotation_y += delta.x * 0.01;
            drag_state.rotation_x = (drag_state.rotation_x + delta.y * 0.01).clamp(-1.2, 1.2);
            drag_state.last_pos = pos;
        }
    } else {
        let speed = 4.0 * time.delta_secs();
        drag_state.rotation_y += (0.0 - drag_state.rotation_y) * speed;
        drag_state.rotation_x += (0.0 - drag_state.rotation_x) * speed;
        if drag_state.rotation_y.abs() < 0.001 { drag_state.rotation_y = 0.0; }
        if drag_state.rotation_x.abs() < 0.001 { drag_state.rotation_x = 0.0; }
    }

    let cursor_over_list = if let Some(cursor_pos) = window.cursor_position() {
        cursor_pos.x < 280.0
    } else {
        false
    };

    if !cursor_over_list {
        for event in scroll_events.read() {
            drag_state.zoom = (drag_state.zoom - event.y * 0.15).clamp(0.5, 5.0);
        }
    }

    for mut transform in model_query.iter_mut() {
        transform.translation = PREVIEW_ORIGIN;
        transform.rotation = Quat::from_rotation_y(drag_state.rotation_y)
            * Quat::from_rotation_x(drag_state.rotation_x);
    }

    for mut cam_transform in camera_query.iter_mut() {
        let offset = Vec3::new(0.0, 0.3, drag_state.zoom);
        cam_transform.translation = PREVIEW_ORIGIN + offset;
        cam_transform.look_at(PREVIEW_ORIGIN + Vec3::new(0.0, 0.1, 0.0), Vec3::Y);
    }
}

pub fn update_loadout_preview_model(
    mut commands: Commands,
    mut ui_state: ResMut<LoadoutUiState>,
    registry: Res<WeaponRegistry>,
    existing_model: Query<Entity, With<LoadoutPreviewModel>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    drag_state: Res<LoadoutDragState>,
) {
    if !ui_state.preview_needs_update {
        return;
    }
    ui_state.preview_needs_update = false;

    for entity in existing_model.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(weapon_id) = &ui_state.selected_weapon_id {
        if let Some(config) = registry.weapons.get(weapon_id) {
            let model_file = config.meta.model_path.split('#').next().unwrap_or("");
            let model_exists = !model_file.is_empty()
                && std::path::Path::new(&format!("assets/{}", model_file)).exists();

            let scale = config.meta.scale * 3.0;
            let rotation = Quat::from_rotation_y(drag_state.rotation_y);

            if model_exists {
                let skin = ui_state.selected_skin;
                commands.spawn((
                    WorldAssetRoot(asset_server.load(&config.meta.model_path)),
                    Transform::from_translation(PREVIEW_ORIGIN)
                        .with_rotation(rotation)
                        .with_scale(Vec3::splat(scale)),
                    LoadoutPreviewModel,
                    WeaponSkinTag { skin, applied: false },
                ));
            } else {
                let slot = crate::weapons::slot_from_weapon_type(&config.meta.weapon_type);
                let skin = ui_state.selected_skin;
                let mat = if skin != WeaponSkin::Default {
                    materials.add(skin.to_material())
                } else {
                    materials.add(StandardMaterial {
                        base_color: Color::srgb(0.3, 0.3, 0.35),
                        metallic: 0.7,
                        perceptual_roughness: 0.3,
                        ..default()
                    })
                };
                let mesh = match slot {
                    WeaponSlot::Primary => meshes.add(Cuboid::new(0.08 * 3.0, 0.12 * 3.0, 0.6 * 3.0)),
                    WeaponSlot::Secondary => meshes.add(Cuboid::new(0.06 * 3.0, 0.12 * 3.0, 0.3 * 3.0)),
                    WeaponSlot::Melee => meshes.add(Cuboid::new(0.04 * 3.0, 0.04 * 3.0, 0.4 * 3.0)),
                    WeaponSlot::Equipment => meshes.add(Cuboid::new(0.15 * 3.0, 0.15 * 3.0, 0.15 * 3.0)),
                };
                commands.spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(PREVIEW_ORIGIN)
                        .with_rotation(rotation)
                        .with_scale(Vec3::ONE),
                    LoadoutPreviewModel,
                ));
            }
        }
    }
}
