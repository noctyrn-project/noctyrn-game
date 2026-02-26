use bevy::prelude::*;
use bevy::app::AppExit;
use crate::player::GameState;
use crate::weapons::{WeaponRegistry, WeaponSlot, PlayerLoadout, WeaponConfig, sync_loadout_to_configs, WeaponSkin, WeaponSkinTag, SkinRarity, SkinInventory};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoadoutUiState>();
        app.init_resource::<LoadoutDragState>();
        app.init_resource::<CrateState>();
        app.add_systems(OnEnter(GameState::MainMenu), (ensure_menu_camera, spawn_main_menu));
        app.add_systems(OnExit(GameState::MainMenu), despawn_main_menu);
        app.add_systems(Update, (main_menu_interaction, main_menu_hover).run_if(in_state(GameState::MainMenu)));

        app.add_systems(OnEnter(GameState::LoadoutSelect), (setup_loadout_scene, spawn_loadout_menu));
        app.add_systems(OnExit(GameState::LoadoutSelect), (despawn_loadout_menu, cleanup_loadout_scene));
        app.add_systems(Update, loadout_interaction.run_if(in_state(GameState::LoadoutSelect)));
        app.add_systems(Update, update_loadout_ui.run_if(in_state(GameState::LoadoutSelect)));
        app.add_systems(Update, update_loadout_tabs.run_if(in_state(GameState::LoadoutSelect)));
        app.add_systems(Update, handle_loadout_drag.run_if(in_state(GameState::LoadoutSelect)));
        app.add_systems(Update, update_loadout_preview_model.run_if(in_state(GameState::LoadoutSelect)));

        app.add_systems(OnEnter(GameState::CrateOpening), (ensure_menu_camera, spawn_crate_menu));
        app.add_systems(OnExit(GameState::CrateOpening), despawn_crate_menu);
        app.add_systems(Update, (crate_interaction, update_crate_animation).run_if(in_state(GameState::CrateOpening)));

        app.add_systems(OnEnter(GameState::GameModeSelect), (ensure_menu_camera, spawn_gamemode_menu));
        app.add_systems(OnExit(GameState::GameModeSelect), despawn_gamemode_menu);
        app.add_systems(Update, (gamemode_interaction, gamemode_hover).run_if(in_state(GameState::GameModeSelect)));
        app.add_systems(OnEnter(GameState::Playing), despawn_menu_camera);
    }
}

#[derive(Component)]
struct MenuCamera;

fn ensure_menu_camera(
    mut commands: Commands,
    existing: Query<Entity, With<MenuCamera>>,
) {
    if existing.is_empty() {
        commands.spawn((Camera2d, MenuCamera));
    }
}

fn despawn_menu_camera(mut commands: Commands, query: Query<Entity, With<MenuCamera>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Main Menu
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct MainMenuUi;

#[derive(Component)]
enum MainMenuButton {
    Play,
    Loadout,
    Crates,
    Settings,
    Quit,
}

fn spawn_main_menu(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(50.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.03, 0.03, 0.06)),
        MainMenuUi,
    )).with_children(|root| {
        // Top section - Title
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            ..default()
        }).with_children(|top| {
            top.spawn((
                Text::new("FEARLYSS"),
                TextFont { font_size: 84.0, ..default() },
                TextColor(Color::srgb(0.9, 0.1, 0.1)),
            ));
            top.spawn((
                Text::new("TACTICAL SHOOTER"),
                TextFont { font_size: 14.0, ..default() },
                TextColor(Color::srgba(0.5, 0.5, 0.5, 0.6)),
                Node { margin: UiRect::top(Val::Px(4.0)), ..default() },
            ));
        });

        // Bottom section
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::End,
            ..default()
        }).with_children(|bottom| {
            // Left side - menu buttons
            bottom.spawn(Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            }).with_children(|left| {
                for (label, button, text_color) in [
                    ("LOADOUT", MainMenuButton::Loadout, Color::WHITE),
                    ("CRATES", MainMenuButton::Crates, Color::srgba(0.9, 0.7, 0.2, 0.9)),
                    ("SETTINGS", MainMenuButton::Settings, Color::srgba(0.7, 0.7, 0.7, 0.9)),
                    ("QUIT", MainMenuButton::Quit, Color::srgba(0.6, 0.4, 0.4, 0.8)),
                ] {
                    left.spawn((
                        Button,
                        Node {
                            padding: UiRect::new(Val::Px(12.0), Val::Px(20.0), Val::Px(6.0), Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        button,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new(label),
                            TextFont { font_size: 18.0, ..default() },
                            TextColor(text_color),
                        ));
                    });
                }

                left.spawn((
                    Text::new("v0.1.0"),
                    TextFont { font_size: 11.0, ..default() },
                    TextColor(Color::srgba(0.3, 0.3, 0.3, 0.4)),
                    Node { margin: UiRect::top(Val::Px(20.0)), ..default() },
                ));
            });

            // Right side - PLAY button
            bottom.spawn((
                Button,
                Node {
                    width: Val::Px(240.0),
                    height: Val::Px(64.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.45, 0.12)),
                MainMenuButton::Play,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("PLAY"),
                    TextFont { font_size: 26.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });
    });
}

fn despawn_main_menu(mut commands: Commands, query: Query<Entity, With<MainMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn main_menu_interaction(
    interaction_query: Query<(&Interaction, &MainMenuButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            match button {
                MainMenuButton::Play => {
                    next_state.set(GameState::GameModeSelect);
                }
                MainMenuButton::Loadout => {
                    next_state.set(GameState::LoadoutSelect);
                }
                MainMenuButton::Crates => {
                    next_state.set(GameState::CrateOpening);
                }
                MainMenuButton::Settings => {
                    if let Some(entity) = settings_query.iter().next() {
                        commands.entity(entity).despawn();
                    } else {
                        crate::ui_settings::spawn_settings_menu(&mut commands);
                    }
                }
                MainMenuButton::Quit => {
                    exit.write(AppExit::Success);
                }
            }
        }
    }
}

fn main_menu_hover(
    mut query: Query<(&Interaction, &MainMenuButton, &Children), With<Button>>,
    mut text_query: Query<&mut TextColor>,
) {
    for (interaction, button, children) in query.iter_mut() {
        let (base_color, hover_color) = match button {
            MainMenuButton::Play => (Color::WHITE, Color::srgb(0.5, 1.0, 0.5)),
            MainMenuButton::Loadout => (Color::WHITE, Color::srgb(0.7, 0.85, 1.0)),
            MainMenuButton::Crates => (Color::srgba(0.9, 0.7, 0.2, 0.9), Color::srgb(1.0, 0.85, 0.3)),
            MainMenuButton::Settings => (Color::srgba(0.7, 0.7, 0.7, 0.9), Color::WHITE),
            MainMenuButton::Quit => (Color::srgba(0.6, 0.4, 0.4, 0.8), Color::srgb(1.0, 0.5, 0.5)),
        };
        let color = match interaction {
            Interaction::Hovered | Interaction::Pressed => hover_color,
            _ => base_color,
        };
        for child in children.iter() {
            if let Ok(mut text_color) = text_query.get_mut(child) {
                text_color.0 = color;
            }
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Loadout Selection
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Resource, Default)]
struct LoadoutUiState {
    active_slot: WeaponSlot,
    active_category: Option<String>,
    selected_weapon_id: Option<String>,
    selected_skin: WeaponSkin,
    preview_needs_update: bool,
    last_weapon_click: Option<(String, f64)>, // (weapon_id, time) for double-click detection
}

#[derive(Resource)]
struct LoadoutDragState {
    dragging: bool,
    last_pos: Vec2,
    rotation_y: f32,
    rotation_x: f32,
    zoom: f32,
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
struct LoadoutMenuUi;

#[derive(Component)]
struct LoadoutBackButton;

#[derive(Component)]
struct SlotTabButton {
    slot: WeaponSlot,
}

#[derive(Component)]
struct CategoryButton {
    category: String,
}

#[derive(Component)]
struct WeaponSelectButton {
    weapon_id: String,
}

#[derive(Component)]
struct EquipButton;

#[derive(Component)]
struct WeaponListContainer;

#[derive(Component)]
struct CategoryTabContainer;

#[derive(Component)]
struct WeaponStatsPanel;

#[derive(Component)]
struct CurrentLoadoutDisplay;

#[derive(Component)]
struct SkinButton {
    skin: WeaponSkin,
}

#[derive(Component)]
struct ColorPickerButton;

#[derive(Component)]
struct ColorPickerPanel;

#[derive(Component)]
struct ColorPickerCloseButton;

#[derive(Component)]
struct SkinPanel;

#[derive(Component)]
struct LoadoutPreviewCamera;

#[derive(Component)]
struct LoadoutPreviewModel;

#[derive(Component)]
struct LoadoutPreviewLight;

const PREVIEW_ORIGIN: Vec3 = Vec3::new(500.0, 500.0, 500.0);

fn setup_loadout_scene(
    mut commands: Commands,
    existing_menu_cam: Query<Entity, With<MenuCamera>>,
    mut drag_state: ResMut<LoadoutDragState>,
) {
    // Despawn 2D menu camera
    for entity in existing_menu_cam.iter() {
        commands.entity(entity).despawn();
    }
    *drag_state = LoadoutDragState::default();

    // Spawn 3D preview camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.04, 0.04, 0.07)),
            ..default()
        },
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(0.0, 0.3, 2.5))
            .looking_at(PREVIEW_ORIGIN + Vec3::new(0.0, 0.1, 0.0), Vec3::Y),
        LoadoutPreviewCamera,
    ));

    // Spawn preview lighting
    commands.spawn((
        PointLight {
            color: Color::srgb(0.95, 0.95, 1.0),
            intensity: 50_000.0,
            range: 20.0,
            shadows_enabled: true,
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
            shadows_enabled: false,
            ..default()
        },
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(-2.0, 1.0, -1.0)),
        LoadoutPreviewLight,
    ));
}

fn cleanup_loadout_scene(
    mut commands: Commands,
    camera_query: Query<Entity, With<LoadoutPreviewCamera>>,
    model_query: Query<Entity, With<LoadoutPreviewModel>>,
    light_query: Query<Entity, With<LoadoutPreviewLight>>,
) {
    for entity in camera_query.iter() { commands.entity(entity).despawn(); }
    for entity in model_query.iter() { commands.entity(entity).despawn(); }
    for entity in light_query.iter() { commands.entity(entity).despawn(); }
}

fn spawn_loadout_menu(
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
        // ── Top Bar ──
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
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                ));
            });

            bar.spawn((
                Text::new("LOADOUT"),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::WHITE),
                Node { margin: UiRect::left(Val::Px(16.0)), ..default() },
            ));

            bar.spawn(Node { flex_grow: 1.0, ..default() });

            bar.spawn((
                Text::new(format_loadout_summary(&loadout, &registry)),
                TextFont { font_size: 11.0, ..default() },
                TextColor(Color::srgba(0.5, 0.7, 0.5, 0.8)),
                CurrentLoadoutDisplay,
            ));
        });

        // ── Slot Tabs ──
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
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(if is_active { Color::WHITE } else { Color::srgba(0.6, 0.6, 0.6, 0.8) }),
                    ));
                });
            }
        });

        // ── Main Content: Category tabs + Left panel + Center/Right preview+stats ──
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            ..default()
        }).with_children(|main_area| {
            // Horizontal subcategory tabs (full width, independent of weapon list)
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

            // Row with left panel + center preview + stats
            main_area.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                ..default()
            }).with_children(|content| {
                // Left: Weapon browser panel (weapon list only)
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

                // Center: 3D preview takes up the rest
                content.spawn(Node {
                    flex_grow: 1.0,
                    height: Val::Percent(100.0),
                    ..default()
                });

                // Stats + Skin panel (top-right floating overlay)
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

fn despawn_loadout_menu(mut commands: Commands, query: Query<Entity, With<LoadoutMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn category_display_name(category: &str) -> &str {
    match category {
        "assault" => "Assault Rifles",
        "carbine" => "Carbines",
        "smg" => "Submachine Guns",
        "pdw" => "PDWs",
        "lmg" => "Light Machine Guns",
        "dmr" => "Marksman Rifles",
        "sniper" => "Sniper Rifles",
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

/// Sort order for categories within a slot.
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

fn update_loadout_ui(
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

    // Update loadout summary text
    for mut text in display_query.iter_mut() {
        text.0 = format_loadout_summary(&loadout, &registry);
    }

    let slot = ui_state.active_slot;
    
    // Gather categories for this slot
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

    // Auto-select first category if none selected
    if ui_state.active_category.is_none() && !categories.is_empty() {
        // Don't mutate through the ref - we'll handle it below
    }
    let active_cat = ui_state.active_category.clone().or_else(|| categories.first().map(|(c, _)| c.clone()));

    // Rebuild category tabs
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
                        TextFont { font_size: 10.0, ..default() },
                        TextColor(if is_active { Color::WHITE } else { Color::srgba(0.6, 0.6, 0.6, 0.8) }),
                    ));
                });
            }
        });
    }

    // Rebuild weapon list for selected category
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
                            Color::srgba(0.2, 0.35, 0.55, 0.9)
                        } else if is_equipped {
                            Color::srgba(0.12, 0.25, 0.12, 0.7)
                        } else {
                            Color::srgba(0.1, 0.1, 0.14, 0.5)
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
                                TextFont { font_size: 13.0, ..default() },
                                TextColor(if is_selected { Color::WHITE } else { Color::srgba(0.85, 0.85, 0.85, 0.9) }),
                            ));
                            if is_equipped {
                                btn.spawn((
                                    Text::new("[E]"),
                                    TextFont { font_size: 10.0, ..default() },
                                    TextColor(Color::srgb(0.3, 0.8, 0.3)),
                                ));
                            }
                        });
                    }
                });
            }
        }
    }

    // Rebuild stats panel (weapon info + equip button + color picker button)
    if let Some(stats_entity) = stats_query.iter().next() {
        commands.entity(stats_entity).despawn_children();

        if let Some(weapon_id) = &ui_state.selected_weapon_id {
            if let Some(config) = registry.weapons.get(weapon_id) {
                commands.entity(stats_entity).with_children(|parent| {
                    spawn_weapon_stats(parent, config, weapon_id, &loadout, ui_state.active_slot);

                    // ── Color Picker Button ──
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
                        // Show current skin color swatch
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
                            TextFont { font_size: 12.0, ..default() },
                            TextColor(Color::srgba(0.8, 0.8, 0.9, 0.9)),
                        ));
                    });
                });
            }
        }
    }

    // Despawn existing color picker panel when UI state changes (it will reopen on click)
    for entity in color_panel_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn spawn_weapon_stats(parent: &mut ChildSpawnerCommands, config: &WeaponConfig, weapon_id: &str, loadout: &PlayerLoadout, slot: WeaponSlot) {
    // Weapon name
    parent.spawn((
        Text::new(&config.info.name),
        TextFont { font_size: 26.0, ..default() },
        TextColor(Color::WHITE),
    ));

    // Type + manufacturer
    parent.spawn((
        Text::new(format!("{} • {} • {}", config.meta.weapon_type, config.info.manufacturer, config.info.year_introduced)),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(0.6, 0.6, 0.6, 0.8)),
    ));

    // Description
    parent.spawn((
        Text::new(&config.info.description),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgba(0.7, 0.7, 0.7, 0.7)),
        Node { margin: UiRect::vertical(Val::Px(6.0)), ..default() },
    ));

    // Separator
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.5)),
    ));

    // Stats based on weapon type
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
                Text::new(format!("Ammo: {} • Damage: {:.0} • Pen: {:.0}%", ammo.name, ammo.damage, ammo.penetration * 100.0)),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgba(0.8, 0.7, 0.4, 0.9)),
            ));
        }

        if let Some(mag) = &config.attachments.magazine {
            parent.spawn((
                Text::new(format!("Magazine: {} rds • Reserve: {}", mag.capacity, mag.carry_capacity)),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgba(0.6, 0.7, 0.8, 0.9)),
            ));
        }

        if !config.attributes.fire_modes.is_empty() {
            parent.spawn((
                Text::new(format!("Fire Modes: {}", config.attributes.fire_modes.join(" / "))),
                TextFont { font_size: 12.0, ..default() },
                TextColor(Color::srgba(0.6, 0.7, 0.8, 0.9)),
            ));
        }
    }

    // Equip button
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
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });
}

fn spawn_stat_bar(parent: &mut ChildSpawnerCommands, label: &str, value: f32, max: f32) {
    let fill_pct = (value / max * 100.0).clamp(0.0, 100.0);

    // For fire rate, lower = better, so invert display
    let is_inverted = label == "Fire Rate" || label == "Reload Speed" || label == "Detonation Time";
    let display_pct = if is_inverted { 100.0 - fill_pct } else { fill_pct };

    let bar_color = if display_pct > 70.0 {
        Color::srgb(0.2, 0.7, 0.3)
    } else if display_pct > 40.0 {
        Color::srgb(0.8, 0.7, 0.2)
    } else {
        Color::srgb(0.7, 0.2, 0.2)
    };

    parent.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(10.0),
        ..default()
    }).with_children(|row: &mut ChildSpawnerCommands| {
        row.spawn((
            Text::new(label),
            TextFont { font_size: 12.0, ..default() },
            TextColor(Color::srgba(0.7, 0.7, 0.7, 0.8)),
            Node { width: Val::Px(100.0), ..default() },
        ));

        // Bar background
        row.spawn(Node {
            width: Val::Px(200.0),
            height: Val::Px(10.0),
            ..default()
        }).with_children(|bar_bg: &mut ChildSpawnerCommands| {
            bar_bg.spawn((
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(10.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.15, 0.15, 0.2, 0.8)),
            )).with_children(|bg: &mut ChildSpawnerCommands| {
                bg.spawn((
                    Node {
                        width: Val::Percent(display_pct),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(bar_color),
                ));
            });
        });

        row.spawn((
            Text::new(format!("{:.1}", value)),
            TextFont { font_size: 11.0, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.5, 0.7)),
        ));
    });
}

fn loadout_interaction(
    mut next_state: ResMut<NextState<GameState>>,
    mut ui_state: ResMut<LoadoutUiState>,
    mut loadout: ResMut<PlayerLoadout>,
    mut registry: ResMut<WeaponRegistry>,
    slot_query: Query<(&Interaction, &SlotTabButton), (Changed<Interaction>, With<Button>)>,
    weapon_query: Query<(&Interaction, &WeaponSelectButton), (Changed<Interaction>, With<Button>)>,
    skin_query: Query<(&Interaction, &SkinButton), (Changed<Interaction>, With<Button>)>,
    category_query: Query<(&Interaction, &CategoryButton), (Changed<Interaction>, With<Button>)>,
    mut back_query: Query<(&Interaction, &mut BackgroundColor), (With<LoadoutBackButton>, With<Button>, Without<EquipButton>, Without<ColorPickerButton>, Without<ColorPickerCloseButton>)>,
    mut equip_query: Query<(&Interaction, &mut BackgroundColor), (With<EquipButton>, With<Button>, Without<LoadoutBackButton>, Without<ColorPickerButton>, Without<ColorPickerCloseButton>)>,
    mut color_btn_query: Query<(&Interaction, &mut BackgroundColor), (With<ColorPickerButton>, With<Button>, Without<LoadoutBackButton>, Without<EquipButton>, Without<ColorPickerCloseButton>)>,
    mut color_close_query: Query<(&Interaction, &mut BackgroundColor), (With<ColorPickerCloseButton>, With<Button>, Without<LoadoutBackButton>, Without<EquipButton>, Without<ColorPickerButton>)>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut commands: Commands,
    existing_color_panel: Query<Entity, With<ColorPickerPanel>>,
    loadout_ui_query: Query<Entity, With<LoadoutMenuUi>>,
) {
    // Back button interaction + hover
    for (interaction, mut bg) in back_query.iter_mut() {
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
    
    // Equip button hover + press
    for (interaction, mut bg) in equip_query.iter_mut() {
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
                        // Force UI rebuild so equip status shows immediately
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

    // Slot tabs
    for (interaction, tab) in slot_query.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.active_slot = tab.slot;
            ui_state.active_category = None; // Reset category when switching slots
            ui_state.selected_weapon_id = Some(loadout.get_id_for_slot(tab.slot).to_string());
            ui_state.selected_skin = loadout.get_skin(tab.slot);
            ui_state.preview_needs_update = true;
        }
    }

    // Category tabs
    for (interaction, cat_btn) in category_query.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.active_category = Some(cat_btn.category.clone());
        }
    }

    // Weapon selection (single click = select, double click = equip)
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
                // Double-click: equip the weapon
                let id = weapon_btn.weapon_id.clone();
                loadout.set_id_for_slot(ui_state.active_slot, id);
                sync_loadout_to_configs(&mut registry, &loadout);
                ui_state.preview_needs_update = true;
                ui_state.last_weapon_click = None;
            } else {
                // Single click: select the weapon
                ui_state.selected_weapon_id = Some(weapon_btn.weapon_id.clone());
                ui_state.preview_needs_update = true;
                ui_state.last_weapon_click = Some((weapon_btn.weapon_id.clone(), now));
            }
        }
    }

    // Skin selection (inside color picker panel)
    for (interaction, skin_btn) in skin_query.iter() {
        if *interaction == Interaction::Pressed {
            if mouse_input.just_pressed(MouseButton::Left) {
                ui_state.selected_skin = skin_btn.skin;
                loadout.set_skin(ui_state.active_slot, skin_btn.skin);
                ui_state.preview_needs_update = true;
                // Close color picker panel
                for entity in existing_color_panel.iter() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }

    // Color picker button - toggle panel
    for (interaction, mut bg) in color_btn_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    let has_panel = !existing_color_panel.is_empty();
                    // Close existing panel
                    for entity in existing_color_panel.iter() {
                        commands.entity(entity).despawn();
                    }
                    // If no panel was open, spawn one
                    if !has_panel {
                        if let Some(root_entity) = loadout_ui_query.iter().next() {
                            let selected_skin = ui_state.selected_skin;
                            let weapon_id = ui_state.selected_weapon_id.clone().unwrap_or_default();
                            let owned_skins = SkinInventory::load().owned_skins_for(&weapon_id);
                            commands.entity(root_entity).with_children(|root| {
                                spawn_color_picker_panel(root, selected_skin, &owned_skins);
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

    // Color picker close button
    for (interaction, _bg) in color_close_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            if mouse_input.just_pressed(MouseButton::Left) {
                for entity in existing_color_panel.iter() {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

fn spawn_color_picker_panel(parent: &mut ChildSpawnerCommands, selected_skin: WeaponSkin, owned_skins: &[WeaponSkin]) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(360.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(16.0)),
            row_gap: Val::Px(10.0),
            // Center by using negative margin
            margin: UiRect::new(Val::Px(-180.0), Val::Auto, Val::Px(-200.0), Val::Auto),
            ..default()
        },
        BackgroundColor(Color::srgba(0.06, 0.06, 0.1, 0.98)),
        ColorPickerPanel,
        ZIndex(10),
    )).with_children(|panel| {
        // Header row
        panel.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        }).with_children(|row| {
            row.spawn((
                Text::new("COLOR"),
                TextFont { font_size: 18.0, ..default() },
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
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });

        // ── Gun Body Section ──
        panel.spawn((
            Text::new("GUN BODY"),
            TextFont { font_size: 13.0, ..default() },
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
                    BackgroundColor(skin.swatch_color()),
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

        // Separator
        panel.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.4)),
        ));

        // ── Attachments Section ──
        panel.spawn((
            Text::new("ATTACHMENTS"),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgba(0.5, 0.6, 0.8, 0.9)),
        ));

        panel.spawn((
            Text::new("Coming soon — customize attachment colors independently"),
            TextFont { font_size: 11.0, ..default() },
            TextColor(Color::srgba(0.4, 0.4, 0.5, 0.6)),
            Node { margin: UiRect::bottom(Val::Px(4.0)), ..default() },
        ));

        // Selected skin label
        panel.spawn((
            Text::new(format!("Selected: {}", selected_skin.display_name())),
            TextFont { font_size: 11.0, ..default() },
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

fn handle_loadout_drag(
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut drag_state: ResMut<LoadoutDragState>,
    mut model_query: Query<&mut Transform, With<LoadoutPreviewModel>>,
    mut camera_query: Query<&mut Transform, (With<LoadoutPreviewCamera>, Without<LoadoutPreviewModel>)>,
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    time: Res<Time>,
) {
    let Ok(window) = windows.single() else { return };

    // Right-click drag for rotation
    if mouse_input.just_pressed(MouseButton::Right) {
        if let Some(pos) = window.cursor_position() {
            drag_state.dragging = true;
            drag_state.last_pos = pos;
        }
    }
    if mouse_input.just_released(MouseButton::Right) {
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
        // Smoothly lerp back to default rotation when not dragging (keep zoom as-is)
        let speed = 4.0 * time.delta_secs();
        drag_state.rotation_y += (0.0 - drag_state.rotation_y) * speed;
        drag_state.rotation_x += (0.0 - drag_state.rotation_x) * speed;
        // Snap to zero when very close
        if drag_state.rotation_y.abs() < 0.001 { drag_state.rotation_y = 0.0; }
        if drag_state.rotation_x.abs() < 0.001 { drag_state.rotation_x = 0.0; }
    }

    // Only zoom when cursor is NOT over the weapon list panel
    let cursor_over_list = if let Some(cursor_pos) = window.cursor_position() {
        // The weapon list is the left 280px panel
        cursor_pos.x < 280.0
    } else {
        false
    };

    if !cursor_over_list {
        for event in scroll_events.read() {
            drag_state.zoom = (drag_state.zoom - event.y * 0.15).clamp(0.5, 5.0);
        }
    }

    // Update model rotation
    for mut transform in model_query.iter_mut() {
        transform.translation = PREVIEW_ORIGIN;
        transform.rotation = Quat::from_rotation_y(drag_state.rotation_y)
            * Quat::from_rotation_x(drag_state.rotation_x);
    }

    // Update camera distance (zoom)
    for mut cam_transform in camera_query.iter_mut() {
        let offset = Vec3::new(0.0, 0.3, drag_state.zoom);
        cam_transform.translation = PREVIEW_ORIGIN + offset;
        cam_transform.look_at(PREVIEW_ORIGIN + Vec3::new(0.0, 0.1, 0.0), Vec3::Y);
    }
}

fn update_loadout_preview_model(
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

    // Despawn old model
    for entity in existing_model.iter() {
        commands.entity(entity).despawn();
    }

    // Spawn new preview model
    if let Some(weapon_id) = &ui_state.selected_weapon_id {
        if let Some(config) = registry.weapons.get(weapon_id) {
            let model_file = config.meta.model_path.split('#').next().unwrap_or("");
            let model_exists = !model_file.is_empty()
                && std::path::Path::new(&format!("assets/{}", model_file)).exists();

            let scale = config.meta.scale * 3.0; // Scale up for preview
            let rotation = Quat::from_rotation_y(drag_state.rotation_y);

            if model_exists {
                let skin = ui_state.selected_skin;
                commands.spawn((
                    SceneRoot(asset_server.load(&config.meta.model_path)),
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Crate Opening System
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CrateType {
    Standard,
    Tactical,
    Elite,
    Legendary,
}

impl CrateType {
    fn all() -> &'static [CrateType] {
        &[CrateType::Standard, CrateType::Tactical, CrateType::Elite, CrateType::Legendary]
    }

    fn display_name(&self) -> &str {
        match self {
            CrateType::Standard => "Standard Crate",
            CrateType::Tactical => "Tactical Crate",
            CrateType::Elite => "Elite Crate",
            CrateType::Legendary => "Legendary Crate",
        }
    }

    fn description(&self) -> &str {
        match self {
            CrateType::Standard => "Basic crate with standard drop rates.",
            CrateType::Tactical => "Better odds for uncommon and rare skins.",
            CrateType::Elite => "Guaranteed rare or better. Higher epic chances.",
            CrateType::Legendary => "Guaranteed epic or better. The best odds.",
        }
    }

    fn color(&self) -> Color {
        match self {
            CrateType::Standard => Color::srgb(0.3, 0.3, 0.35),
            CrateType::Tactical => Color::srgb(0.2, 0.4, 0.25),
            CrateType::Elite => Color::srgb(0.2, 0.3, 0.6),
            CrateType::Legendary => Color::srgb(0.6, 0.45, 0.1),
        }
    }

    /// Returns modified drop weights for each rarity (values in tenths of a percent for precision)
    fn drop_weights(&self) -> Vec<(SkinRarity, u32)> {
        match self {
            // Mythic 0.1%, Legendary 1%
            CrateType::Standard => vec![
                (SkinRarity::Common, 549),
                (SkinRarity::Uncommon, 250),
                (SkinRarity::Rare, 130),
                (SkinRarity::Epic, 60),
                (SkinRarity::Legendary, 10),
                (SkinRarity::Mythic, 1),
            ],
            // Mythic 0.5%, Legendary 2%
            CrateType::Tactical => vec![
                (SkinRarity::Common, 325),
                (SkinRarity::Uncommon, 350),
                (SkinRarity::Rare, 200),
                (SkinRarity::Epic, 100),
                (SkinRarity::Legendary, 20),
                (SkinRarity::Mythic, 5),
            ],
            // Mythic 1%, Legendary 4%
            CrateType::Elite => vec![
                (SkinRarity::Common, 0),
                (SkinRarity::Uncommon, 0),
                (SkinRarity::Rare, 500),
                (SkinRarity::Epic, 450),
                (SkinRarity::Legendary, 40),
                (SkinRarity::Mythic, 10),
            ],
            // Mythic 2%, Legendary 13%
            CrateType::Legendary => vec![
                (SkinRarity::Common, 0),
                (SkinRarity::Uncommon, 0),
                (SkinRarity::Rare, 0),
                (SkinRarity::Epic, 830),
                (SkinRarity::Legendary, 150),
                (SkinRarity::Mythic, 20),
            ],
        }
    }

    fn roll_skin(&self) -> WeaponSkin {
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
        
        // Pick a random skin of that rarity
        let candidates: Vec<&WeaponSkin> = WeaponSkin::droppable()
            .iter()
            .filter(|s| s.rarity() == selected_rarity)
            .collect();
        
        if candidates.is_empty() {
            // Fallback if no skins of that rarity
            return *WeaponSkin::droppable().first().unwrap_or(&WeaponSkin::SolidRed);
        }
        
        let pick = (seed as usize / 7) % candidates.len();
        *candidates[pick]
    }
}

#[derive(Resource, Default)]
struct CrateState {
    selected_crate: Option<CrateType>,
    opening_animation: f32, // 0.0 = not opening, 0..1 = animating
    result_skin: Option<WeaponSkin>,
    result_weapon: Option<String>,   // Which weapon received the skin
    strip_skins: Vec<WeaponSkin>,  // The full strip of skins to display
    strip_offset: f32,             // Current scroll offset in pixels
    strip_velocity: f32,           // Current scroll speed
    strip_target: f32,             // Target offset where winning skin lands
    strip_phase: CratePhase,
    spin_time: f32,                // Elapsed time since spinning started
    spin_duration: f32,            // Total spin duration
}

#[derive(Default, Clone, Copy, PartialEq)]
enum CratePhase {
    #[default]
    Idle,
    Spinning,
    Revealing,
}

#[derive(Component)]
struct CrateMenuUi;

#[derive(Component)]
struct CrateSelectButton {
    crate_type: CrateType,
}

#[derive(Component)]
struct CrateBackButton;

#[derive(Component)]
struct CrateResultPanel;

#[derive(Component)]
struct CrateResultDismiss;

#[derive(Component)]
struct CrateStripContainer;

#[derive(Component)]
struct CrateStripInner;

#[derive(Component)]
struct CratePointerMarker;


fn spawn_crate_menu(mut commands: Commands, mut crate_state: ResMut<CrateState>) {
    *crate_state = CrateState::default();

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
        // Header
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Px(16.0),
            ..default()
        }).with_children(|header| {
            header.spawn((
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

            header.spawn((
                Text::new("CRATES"),
                TextFont { font_size: 32.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });

        // Subtitle
        root.spawn((
            Text::new("Open crates to earn weapon skins. You have unlimited crates of each type."),
            TextFont { font_size: 13.0, ..default() },
            TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
        ));

        // Crate cards row
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
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(280.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(16.0)),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.08, 0.12, 0.9)),
                    BorderColor::from(ct.color()),
                    CrateSelectButton { crate_type: ct },
                )).with_children(|card| {
                    // Crate icon (using a colored box)
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

                    // Drop rate preview
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

                    // Open button
                    card.spawn((
                        Text::new("CLICK TO OPEN"),
                        TextFont { font_size: 12.0, ..default() },
                        TextColor(ct.color()),
                    ));
                });
            }
        });
    });
}

fn despawn_crate_menu(mut commands: Commands, query: Query<Entity, With<CrateMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn crate_interaction(
    mut next_state: ResMut<NextState<GameState>>,
    mut crate_state: ResMut<CrateState>,
    mut commands: Commands,
    mut crate_select_query: Query<(&Interaction, &CrateSelectButton, &mut BackgroundColor), (Changed<Interaction>, With<Button>, Without<CrateBackButton>, Without<CrateResultDismiss>)>,
    mut back_query: Query<(&Interaction, &mut BackgroundColor), (With<CrateBackButton>, With<Button>, Without<CrateSelectButton>, Without<CrateResultDismiss>)>,
    mut dismiss_query: Query<(&Interaction, &mut BackgroundColor), (With<CrateResultDismiss>, With<Button>, Without<CrateBackButton>, Without<CrateSelectButton>)>,
    result_panel_query: Query<Entity, With<CrateResultPanel>>,
    crate_menu_query: Query<Entity, With<CrateMenuUi>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    // Back button
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

    // Crate selection (starts spinning animation)
    for (interaction, crate_btn, mut bg) in crate_select_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) && crate_state.strip_phase == CratePhase::Idle {
                    let skin = crate_btn.crate_type.roll_skin();
                    crate_state.result_skin = Some(skin);
                    crate_state.selected_crate = Some(crate_btn.crate_type);

                    // Generate strip of ~60 random skins with the winner at position 45
                    let mut strip = Vec::new();
                    for _ in 0..60 {
                        strip.push(crate_btn.crate_type.roll_skin());
                    }
                    strip[45] = skin; // Place winner at index 45
                    crate_state.strip_skins = strip;
                    
                    // Each skin cell is 80px wide + 4px gap = 84px
                    // Target offset: center of winning cell at index 45
                    let cell_width = 84.0;
                    crate_state.strip_target = 45.0 * cell_width + 42.0; // Center of cell
                    crate_state.strip_offset = 0.0;
                    crate_state.strip_velocity = 4000.0; // Initial fast scroll speed
                    crate_state.strip_phase = CratePhase::Spinning;
                    crate_state.opening_animation = 0.0;
                    crate_state.spin_time = 0.0;
                    crate_state.spin_duration = 4.5; // Total spin duration in seconds

                    // Spawn the strip overlay
                    if let Some(root) = crate_menu_query.iter().next() {
                        commands.entity(root).with_children(|parent| {
                            // Full-screen overlay
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
                                // Center pointer/indicator triangle
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

                                // Strip container (clips overflow)
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
                                    // Inner scrolling row
                                    container.spawn((
                                        Node {
                                            flex_direction: FlexDirection::Row,
                                            column_gap: Val::Px(4.0),
                                            height: Val::Percent(100.0),
                                            align_items: AlignItems::Center,
                                            padding: UiRect::horizontal(Val::Px(300.0)), // Padding so first/last items can center
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

                                // Bottom pointer
                                overlay.spawn((
                                    Node {
                                        width: Val::Px(4.0),
                                        height: Val::Px(30.0),
                                        margin: UiRect::top(Val::Px(4.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(1.0, 0.85, 0.0)),
                                ));
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

    // Dismiss result
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

fn update_crate_animation(
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
            
            // Use a cubic ease-out curve for smooth deceleration
            // t goes from 0 to 1 over the spin_duration
            let t = (crate_state.spin_time / crate_state.spin_duration).clamp(0.0, 1.0);
            
            // Ease-out cubic: 1 - (1 - t)^3, with a slight overshoot and settle-back
            let eased = if t < 0.92 {
                // Main easing phase with cubic ease-out
                let sub_t = t / 0.92;
                let ease = 1.0 - (1.0 - sub_t).powi(3);
                ease * 1.006 // Slight overshoot past target
            } else {
                // Settle-back phase: ease back from overshoot to exact target
                let sub_t = (t - 0.92) / 0.08;
                let settle = 1.006 - 0.006 * sub_t * sub_t; // Smooth settle to 1.0
                settle
            };
            
            crate_state.strip_offset = eased * crate_state.strip_target;
            
            // Done when we've reached the full duration
            if t >= 1.0 {
                crate_state.strip_offset = crate_state.strip_target;
                crate_state.strip_velocity = 0.0;
                crate_state.strip_phase = CratePhase::Revealing;
                crate_state.opening_animation = 0.0;
            }
            
            // Update the strip position
            for mut node in strip_query.iter_mut() {
                node.left = Val::Px(-crate_state.strip_offset);
            }
        }
        CratePhase::Revealing => {
            // Brief pause then show result
            crate_state.opening_animation += time.delta_secs();
            if crate_state.opening_animation > 0.8 {
                // Despawn the strip overlay and spawn result card
                for entity in result_panel_query.iter() {
                    commands.entity(entity).despawn();
                }
                
                if let (Some(skin), Some(root)) = (crate_state.result_skin, crate_menu_query.iter().next()) {
                    let rarity = skin.rarity();
                    
                    // Pick a random weapon to assign the skin to
                    let all_weapon_ids: Vec<String> = registry.weapons.keys().cloned().collect();
                    let assigned_weapon = if !all_weapon_ids.is_empty() {
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let seed = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .subsec_nanos() as usize;
                        let idx = seed % all_weapon_ids.len();
                        all_weapon_ids[idx].clone()
                    } else {
                        "colt_m4a1".to_string()
                    };
                    
                    // Add to inventory and save
                    let dup_count = skin_inventory.duplicate_count(&assigned_weapon, &skin);
                    skin_inventory.add_skin(&assigned_weapon, skin);
                    skin_inventory.save();
                    crate_state.result_weapon = Some(assigned_weapon.clone());
                    
                    // Get weapon display name
                    let weapon_display = registry.weapons.get(&assigned_weapon)
                        .map(|c| c.info.name.clone())
                        .unwrap_or_else(|| assigned_weapon.replace('_', " "));
                    
                    let dup_text = if dup_count > 0 {
                        format!("(Duplicate #{} - you now have {})", dup_count + 1, dup_count + 1)
                    } else {
                        String::new()
                    };
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

                                // Large skin swatch
                                card.spawn((
                                    Node {
                                        width: Val::Px(120.0),
                                        height: Val::Px(120.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(3.0)),
                                        ..default()
                                    },
                                    BackgroundColor(skin.swatch_color()),
                                    BorderColor::from(rarity.color()),
                                ));

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

                                // Show which weapon received the skin
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

                                // Dismiss button
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Game Mode Selection
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
struct GameModeMenuUi;

#[derive(Component)]
enum GameModeButton {
    TestingGrounds,
    Back,
}

#[derive(Component)]
struct LockedMode;

fn spawn_gamemode_menu(mut commands: Commands) {
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(20.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.05, 0.05, 0.1)),
        GameModeMenuUi,
    )).with_children(|root| {
        root.spawn((
            Text::new("SELECT GAME MODE"),
            TextFont { font_size: 36.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));

        // Game mode cards
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(20.0),
            ..default()
        }).with_children(|row| {
            // Testing Grounds - Available
            row.spawn((
                Button,
                Node {
                    width: Val::Px(250.0),
                    height: Val::Px(300.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(20.0)),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.12, 0.18, 0.25)),
                GameModeButton::TestingGrounds,
            )).with_children(|card| {
                card.spawn((
                    Text::new("TG"),
                    TextFont { font_size: 36.0, ..default() },
                    TextColor(Color::srgb(0.4, 0.7, 0.9)),
                ));
                card.spawn((
                    Text::new("TESTING GROUNDS"),
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(Color::WHITE),
                ));
                card.spawn((
                    Text::new("Practice with all weapons. Spawn targets, test movement, and refine your aim."),
                    TextFont { font_size: 12.0, ..default() },
                    TextColor(Color::srgba(0.6, 0.7, 0.8, 0.8)),
                ));
                card.spawn((
                    Text::new("PLAY"),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::srgb(0.3, 0.8, 0.3)),
                    Node { margin: UiRect::top(Val::Px(10.0)), ..default() },
                ));
            });

            // Deathmatch - Locked
            spawn_locked_mode_card(row, "DM", "DEATHMATCH", "Free-for-all combat against other players.");

            // Team Battle - Locked
            spawn_locked_mode_card(row, "TB", "TEAM BATTLE", "Work with your team to achieve victory.");

            // Capture Point - Locked
            spawn_locked_mode_card(row, "CP", "CAPTURE POINT", "Capture and hold objectives to earn points.");
        });

        // Back button
        root.spawn((
            Button,
            Node {
                width: Val::Px(200.0),
                height: Val::Px(45.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.3, 0.15, 0.15)),
            GameModeButton::Back,
        )).with_children(|btn| {
            btn.spawn((
                Text::new("BACK"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
    });
}

fn spawn_locked_mode_card(parent: &mut ChildSpawnerCommands, icon: &str, name: &str, desc: &str) {
    parent.spawn((
        Node {
            width: Val::Px(250.0),
            height: Val::Px(300.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(20.0)),
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.08, 0.08, 0.1, 0.6)),
        LockedMode,
    )).with_children(|card: &mut ChildSpawnerCommands| {
        card.spawn((
            Text::new(icon),
            TextFont { font_size: 32.0, ..default() },
            TextColor(Color::srgba(0.3, 0.4, 0.5, 0.5)),
        ));
        card.spawn((
            Text::new(name),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::srgba(0.4, 0.4, 0.4, 0.6)),
        ));
        card.spawn((
            Text::new(desc),
            TextFont { font_size: 12.0, ..default() },
            TextColor(Color::srgba(0.3, 0.3, 0.4, 0.5)),
        ));
        card.spawn((
            Text::new("LOCKED"),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::srgba(0.5, 0.4, 0.2, 0.7)),
            Node { margin: UiRect::top(Val::Px(10.0)), ..default() },
        ));
    });
}

fn despawn_gamemode_menu(mut commands: Commands, query: Query<Entity, With<GameModeMenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn gamemode_interaction(
    interaction_query: Query<(&Interaction, &GameModeButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            match button {
                GameModeButton::TestingGrounds => {
                    next_state.set(GameState::Playing);
                }
                GameModeButton::Back => {
                    next_state.set(GameState::MainMenu);
                }
            }
        }
    }
}

fn gamemode_hover(
    mut query: Query<(&Interaction, &mut BackgroundColor, Option<&GameModeButton>), With<Button>>,
) {
    for (interaction, mut bg, button) in query.iter_mut() {
        if let Some(button) = button {
            let (base, hover) = match button {
                GameModeButton::TestingGrounds => (
                    Color::srgb(0.12, 0.18, 0.25),
                    Color::srgb(0.16, 0.24, 0.35),
                ),
                GameModeButton::Back => (
                    Color::srgb(0.3, 0.15, 0.15),
                    Color::srgb(0.45, 0.2, 0.2),
                ),
            };
            *bg = match interaction {
                Interaction::Hovered => BackgroundColor(hover),
                Interaction::Pressed => BackgroundColor(hover),
                _ => BackgroundColor(base),
            };
        }
    }
}

fn update_loadout_tabs(
    ui_state: Res<LoadoutUiState>,
    mut tab_query: Query<(&SlotTabButton, &mut BackgroundColor, &Children)>,
    mut text_query: Query<&mut TextColor>,
) {
    if !ui_state.is_changed() {
        return;
    }

    for (tab, mut bg, children) in tab_query.iter_mut() {
        let is_active = tab.slot == ui_state.active_slot;
        *bg = if is_active {
            BackgroundColor(Color::srgb(0.3, 0.4, 0.6))
        } else {
            BackgroundColor(Color::srgb(0.15, 0.15, 0.2))
        };
        for child in children.iter() {
            if let Ok(mut text_color) = text_query.get_mut(child) {
                text_color.0 = if is_active {
                    Color::WHITE
                } else {
                    Color::srgba(0.7, 0.7, 0.7, 0.8)
                };
            }
        }
    }
}
