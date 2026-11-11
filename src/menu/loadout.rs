use bevy::prelude::*;
use crate::theme::*;
use crate::player::GameState;
use crate::weapons::{
    WeaponRegistry, WeaponSlot, PlayerLoadout, WeaponConfig,
    sync_loadout_to_configs, WeaponSkinTag, SkinInstance, SkinId, SkinRegistry, Inventory,
    PlayerAttachments, AttachmentRegistry, AmmoRegistry,
    compatible_attachments, compatible_ammo, effective_attachment_name, default_slot_name,
    spawn_weapon_attachments, resolve_attachment_meshes, AttachmentRes, SkinRes, slot_key_to_label,
};
use crate::menu::MenuCamera;

const PREVIEW_DIST: f32 = 1.5;

/// Which level of the left list is shown. Guns, attachment slots of the
/// selected gun, or options for one slot — all in the same list panel.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub enum AttachmentBrowse {
    #[default]
    Closed,
    Slots,
    Options(String),
}

impl AttachmentBrowse {
    fn key(&self) -> String {
        match self {
            AttachmentBrowse::Closed => "closed".to_string(),
            AttachmentBrowse::Slots => "slots".to_string(),
            AttachmentBrowse::Options(slot) => format!("opt:{slot}"),
        }
    }
}

#[derive(Resource)]
pub struct LoadoutUiState {
    pub active_slot: WeaponSlot,
    pub active_category: Option<String>,
    pub selected_weapon_id: Option<String>,
    pub selected_skin: SkinInstance,
    pub preview_needs_update: bool,
    pub browse: AttachmentBrowse,
    /// Inspector shows the extended ballistics readout.
    pub show_advanced: bool,
    /// Bumped on every attachment equip/clear; part of rebuild keys.
    pub attachments_rev: u64,
    /// Last-built content keys (per-panel rebuilds preserve scroll/focus).
    pub last_list_key: String,
    pub last_inspector_key: String,
    /// Browse level the list scroll position belongs to; changing levels
    /// resets the scroll offset.
    pub last_browse: AttachmentBrowse,
}

impl Default for LoadoutUiState {
    fn default() -> Self {
        Self {
            active_slot: WeaponSlot::Primary,
            active_category: None,
            selected_weapon_id: None,
            selected_skin: SkinInstance::default(),
            preview_needs_update: false,
            browse: AttachmentBrowse::Closed,
            show_advanced: false,
            attachments_rev: 0,
            last_list_key: String::new(),
            last_inspector_key: String::new(),
            last_browse: AttachmentBrowse::Closed,
        }
    }
}

#[derive(Resource)]
pub struct LoadoutDragState {
    pub dragging: bool,
    pub last_pos: Vec2,
    pub rotation_y: f32,
    pub rotation_x: f32,
    pub rotation_z: f32,
    pub zoom: f32,
}

impl Default for LoadoutDragState {
    fn default() -> Self {
        Self {
            dragging: false,
            last_pos: Vec2::ZERO,
            rotation_y: 0.0,
            rotation_x: 0.0,
            rotation_z: 0.0,
            zoom: 1.5,
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

/// Attachment slot row in the stats panel (e.g. "OPTIC: Iron Sights").
/// Clicking browses that slot's options in the weapon list.
#[derive(Component)]
pub struct AttachmentSlotButton {
    pub slot: String,
}

/// One attachment option row. `None` = the gun's built-in default.
#[derive(Component)]
pub struct AttachmentOptionButton {
    pub attachment_id: Option<String>,
}

/// Back row shown above attachment options.
#[derive(Component)]
pub struct AttachmentBackButton;

/// Toggles the advanced ballistics readout in the inspector.
#[derive(Component)]
pub struct AdvancedToggleButton;

#[derive(Component)]
pub struct EquipButton;

#[derive(Component)]
pub struct WeaponListContainer;

/// Scrollbar track beside the gun list (thumb sized by the scroll system).
#[derive(Component)]
pub struct WeaponListScrollTrack;

/// Scrollbar thumb: height/offset reflect list scroll position.
#[derive(Component)]
pub struct WeaponListScrollThumb;

#[derive(Component)]
pub struct CategoryTabContainer;

#[derive(Component)]
pub struct WeaponStatsPanel;

#[derive(Component)]
pub struct CurrentLoadoutDisplay;

#[derive(Component)]
pub struct SkinButton {
    pub skin: SkinId,
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
pub struct LoadoutPreviewModel;

/// Default rotation offset so the weapon is viewed from the side (90° Y).
fn base_rotation() -> Quat {
    Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)
}

pub fn setup_loadout_scene(
    mut commands: Commands,
    existing_menu_cam: Query<Entity, With<MenuCamera>>,
    mut drag_state: ResMut<LoadoutDragState>,
) {
    for entity in existing_menu_cam.iter() {
        commands.entity(entity).despawn();
    }
    *drag_state = LoadoutDragState::default();
    // 3D camera + scene are handled by the Menu3dCamera (persistent lobby camera).
    // Weapon preview model is spawned by update_loadout_preview_model.
}

pub fn cleanup_loadout_scene(
    mut commands: Commands,
    model_query: Query<Entity, With<LoadoutPreviewModel>>,
) {
    for entity in model_query.iter() { commands.entity(entity).despawn(); }
}

pub fn spawn_loadout_menu(
    mut commands: Commands,
    registry: Res<WeaponRegistry>,
    loadout: Res<PlayerLoadout>,
    mut ui_state: ResMut<LoadoutUiState>,
) {
    ui_state.active_slot = WeaponSlot::Primary;
    // Open on the equipped gun's category instead of always assault.
    ui_state.active_category = registry
        .weapons
        .get(loadout.primary.as_str())
        .map(|c| c.meta.category.clone())
        .filter(|c| !c.is_empty());
    ui_state.selected_weapon_id = Some(loadout.primary.clone());
    ui_state.selected_skin = loadout.get_skin(WeaponSlot::Primary);
    ui_state.browse = AttachmentBrowse::Closed;
    ui_state.last_list_key.clear();
    ui_state.last_inspector_key.clear();
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
            // Clear the top of the screen (was scraping it).
            margin: UiRect::top(Val::Px(48.0)),
            ..default()
        }).with_children(|bar| {
            bar.spawn((
                Button,
                Node {
                    width: Val::Px(90.0),
                    height: Val::Px(36.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: RADIUS,
                    ..default()
                },
                BackgroundColor(BG_ELEVATED),
                LoadoutBackButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("BACK"),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(TEXT_MUTED),
                ));
            });

            bar.spawn((
                Text::new("LOADOUT"),
                TextFont { font_size: FontSize::Px(28.0), ..default() },
                TextColor(TEXT),
                Node { margin: UiRect::left(Val::Px(16.0)), ..default() },
            ));

            bar.spawn(Node { flex_grow: 1.0, ..default() });

            bar.spawn((
                Text::new(format_loadout_summary(&loadout, &registry)),
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(TEXT_MUTED),
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
                        border_radius: RADIUS,
                        ..default()
                    },
                    BackgroundColor(if is_active { ACCENT } else { BG_ELEVATED }),
                    SlotTabButton { slot },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextFont { font_size: FontSize::Px(12.0), ..default() },
                        TextColor(if is_active { TEXT } else { TEXT_MUTED }),
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
                CategoryTabContainer,
            ));

            main_area.spawn(Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                ..default()
            }).with_children(|content| {
                // Left column: scrollable gun/slot/option list (bounded by
                // the viewport so long catalogs clip) + scrollbar track.
                content.spawn(Node {
                    width: Val::Px(280.0),
                    height: Val::Vh(70.0),
                    flex_direction: FlexDirection::Row,
                    ..default()
                }).with_children(|left| {
                    left.spawn((
                        Node {
                            flex_grow: 1.0,
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::scroll_y(),
                            row_gap: Val::Px(2.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            border_radius: RADIUS,
                            ..default()
                        },
                        ScrollPosition::default(),
                        WeaponListContainer,
                    ));
                    // Scrollbar track (thumb sized by the scroll system).
                    left.spawn((
                        Node {
                            width: Val::Px(8.0),
                            height: Val::Percent(100.0),
                            justify_content: JustifyContent::Start,
                            ..default()
                        },
                        WeaponListScrollTrack,
                    )).with_children(|track| {
                        track.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(20.0),
                                border_radius: RADIUS_SM,
                                ..default()
                            },
                            BackgroundColor(TEXT_MUTED.with_alpha(0.5)),
                            WeaponListScrollThumb,
                        ));
                    });
                });

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
                        bottom: Val::Px(12.0),
                        width: Val::Px(320.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(12.0)),
                        row_gap: Val::Px(4.0),
                        overflow: Overflow::scroll_y(),
                        border_radius: RADIUS,
                        ..default()
                    },
                    BackgroundColor(BG_PANEL.with_alpha(0.8)),
                    BorderColor::all(BORDER),
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

pub(crate) fn category_display_name(category: &str) -> &str {
    match category {
        "assault" => "Assault Rifles",
        "carbine" => "Carbines",
        "smg" => "SMGs",
        "pdw" => "PDWs",
        "lmg" => "LMGs",
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

/// Rebuild keys for the two live panels: browsing attachments changes only
/// the list key, picking a skin or toggling advanced changes only the
/// inspector key, equipping changes both.
fn loadout_panel_keys(
    slot: WeaponSlot,
    category: Option<&str>,
    weapon_id: Option<&str>,
    browse: &AttachmentBrowse,
    attachments_rev: u64,
    skin: SkinInstance,
    show_advanced: bool,
) -> (String, String) {
    let list_key = format!("{slot:?}|{category:?}|{weapon_id:?}|{}|{attachments_rev}", browse.key());
    let inspector_key = format!("{weapon_id:?}|{attachments_rev}|{skin:?}|{show_advanced}");
    (list_key, inspector_key)
}

/// Slot order shared by the browse list (was the stats-panel section).
const SLOT_ORDER: &[&str] = &[
    "optic", "barrel", "muzzle", "underbarrel", "side", "magazine", "stock", "ammo",
];

/// A slot is listed when the gun defines a default part or a socket for it.
fn browse_slot_defined(slot_key: &str, config: &WeaponConfig) -> bool {
    default_slot_name(slot_key, &config.attachments).is_some()
        || crate::weapons::socket_keys_for_slot(slot_key)
            .iter()
            .any(|k| config.sockets.contains_key(*k))
}

/// Effective part name shown on a slot row (ammo resolves from the catalog).
fn browse_slot_current_name(
    slot_key: &str,
    weapon_id: &str,
    config: &WeaponConfig,
    player_attachments: &PlayerAttachments,
    attachment_registry: &AttachmentRegistry,
    ammo_registry: &AmmoRegistry,
) -> String {
    if slot_key == "ammo" {
        return match player_attachments.get(weapon_id, "ammo") {
            Some(id) => ammo_registry
                .by_id
                .get(id)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Default".to_string()),
            None => default_slot_name(slot_key, &config.attachments)
                .unwrap_or_else(|| "—".to_string()),
        };
    }
    match default_slot_name(slot_key, &config.attachments) {
        Some(default) => effective_attachment_name(
            weapon_id,
            slot_key,
            &default,
            player_attachments,
            attachment_registry,
        ),
        None => "—".to_string(),
    }
}

pub(crate) fn category_sort_order(category: &str) -> u32 {    match category {
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

/// Pixels scrolled per wheel notch on the gun list.
const LIST_SCROLL_SPEED: f32 = 20.0;
/// Minimum scrollbar thumb height so it stays grabbable/visible.
const SCROLL_THUMB_MIN: f32 = 20.0;

/// Pure scroll math (unit-tested): advance `offset` by a wheel delta and
/// clamp into the scrollable range.
pub fn list_scroll_offset(offset: f32, wheel: f32, viewport: f32, content: f32) -> f32 {
    let max = (content - viewport).max(0.0);
    (offset - wheel * LIST_SCROLL_SPEED).clamp(0.0, max)
}

/// Pure thumb layout (unit-tested): `(height, y_offset)` inside a track of
/// `track` px given the list viewport/content sizes and scroll offset.
pub fn scroll_thumb_layout(viewport: f32, content: f32, offset: f32, track: f32) -> (f32, f32) {
    if content <= viewport || content <= 0.0 {
        return (track, 0.0);
    }
    let height = (viewport / content * track).max(SCROLL_THUMB_MIN);
    let max_offset = (content - viewport).max(0.0);
    let y = if max_offset <= 0.0 {
        0.0
    } else {
        offset.clamp(0.0, max_offset) / max_offset * (track - height)
    };
    (height, y)
}

/// Drives the gun-list scrollbar from wheel input (Bevy clips `Overflow`
/// content but leaves scrolling to us via `ScrollPosition`) and sizes the
/// thumb from the measured content/viewport ratio.
pub fn scroll_loadout_list(
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    windows: Query<&Window>,
    mut list_query: Query<(&ComputedNode, &mut ScrollPosition), With<WeaponListContainer>>,
    track_query: Query<&ComputedNode, With<WeaponListScrollTrack>>,
    mut thumb_query: Query<&mut Node, With<WeaponListScrollThumb>>,
) {
    let Ok(window) = windows.single() else { return };
    let cursor_over_list = window
        .cursor_position()
        .is_some_and(|pos| pos.x < 280.0);
    let Ok((list_node, mut scroll)) = list_query.single_mut() else {
        return;
    };
    if cursor_over_list {
        for event in scroll_events.read() {
            scroll.y = list_scroll_offset(
                scroll.y,
                event.y,
                list_node.unrounded_size().y,
                list_node.content_size().y,
            );
        }
    }
    // Thumb follows measured geometry every frame (content changes on rebuild).
    let Ok(track_node) = track_query.single() else {
        return;
    };
    let Ok(mut thumb_node) = thumb_query.single_mut() else {
        return;
    };
    let track_h = track_node.unrounded_size().y;
    if track_h <= 0.0 {
        return;
    }
    let (h, y) = scroll_thumb_layout(
        list_node.unrounded_size().y,
        list_node.content_size().y,
        scroll.y,
        track_h,
    );
    thumb_node.height = Val::Px(h);
    thumb_node.top = Val::Px(y);
}

pub fn update_loadout_ui(
    mut commands: Commands,
    registry: Res<WeaponRegistry>,
    skin_registry: Res<SkinRegistry>,
    inventory: Res<Inventory>,
    loadout: Res<PlayerLoadout>,
    player_attachments: Res<PlayerAttachments>,
    attachment_registry: Res<AttachmentRegistry>,
    ammo_registry: Res<AmmoRegistry>,
    mut ui_state: ResMut<LoadoutUiState>,
    list_query: Query<Entity, With<WeaponListContainer>>,
    stats_query: Query<Entity, With<WeaponStatsPanel>>,
    cat_tab_query: Query<Entity, With<CategoryTabContainer>>,
    mut display_query: Query<&mut Text, With<CurrentLoadoutDisplay>>,
    color_panel_query: Query<Entity, With<ColorPickerPanel>>,
    mut scroll_query: Query<&mut ScrollPosition, With<WeaponListContainer>>,
) {
    if !ui_state.is_changed() {
        return;
    }
    // Browse-level change: new content, reset the list scroll offset.
    if ui_state.browse != ui_state.last_browse {
        ui_state.last_browse = ui_state.browse.clone();
        if let Ok(mut scroll) = scroll_query.single_mut() {
            scroll.y = 0.0;
        }
    }

    // Per-panel rebuild keys: browsing attachments rebuilds only the list,
    // picking a skin rebuilds only the inspector, etc. This preserves
    // scroll position and hover state in the untouched panel.
    let (list_key, inspector_key) = loadout_panel_keys(
        ui_state.active_slot,
        ui_state.active_category.as_deref(),
        ui_state.selected_weapon_id.as_deref(),
        &ui_state.browse,
        ui_state.attachments_rev,
        ui_state.selected_skin.clone(),
        ui_state.show_advanced,
    );
    let rebuild_list = list_key != ui_state.last_list_key;
    let rebuild_inspector = inspector_key != ui_state.last_inspector_key;
    if !rebuild_list && !rebuild_inspector {
        return;
    }
    ui_state.last_list_key = list_key;
    ui_state.last_inspector_key = inspector_key;

    for mut text in display_query.iter_mut() {
        text.0 = format_loadout_summary(&loadout, &registry);
    }

    let slot = ui_state.active_slot;

    let mut categories: Vec<(String, Vec<String>)> = Vec::new();
    if let Some(weapon_ids) = registry.by_slot.get(&slot) {
        let mut cat_map: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
        for id in weapon_ids {
            // Melee meshes are collectibles: only archetype defaults are
            // always available, the rest unlock from crates.
            if slot == WeaponSlot::Melee
                && !crate::weapons::melee::is_default_melee(id)
                && !inventory.has_melee(id)
            {
                continue;
            }
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

    if rebuild_list {
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
                        border_radius: RADIUS_SM,
                        ..default()
                    },
                    BackgroundColor(if is_active { ACCENT } else { BG_ELEVATED }),
                    CategoryButton { category: category.clone() },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(category_display_name(category).to_uppercase()),
                        TextFont { font_size: FontSize::Px(10.0), ..default() },
                        TextColor(if is_active { TEXT } else { TEXT_MUTED }),
                    ));
                });
            }
        });
    }

    if let Some(list_entity) = list_query.iter().next() {
        commands.entity(list_entity).despawn_children();

        // Attachment browsing mode: back row + default + alternatives.
        // Ammo is browsed the same way (stat-only options from the ammo
        // catalog, matched by caliber).
        if let AttachmentBrowse::Options(slot_key) = ui_state.browse.clone() {
            if let Some(weapon_id) = &ui_state.selected_weapon_id {
                if let Some(config) = registry.weapons.get(weapon_id) {
                    // (option id, display name, artist suffix, effect lines)
                    // Effect line goodness: Some(true) = green, Some(false) =
                    // red, None = neutral info.
                    let mut options: Vec<(String, String, String, Vec<(String, Option<bool>)>)> =
                        Vec::new();
                    if slot_key == "ammo" {
                        for a in compatible_ammo(&config.meta.caliber, &ammo_registry) {
                            let mut lines = vec![
                                (
                                    format!(
                                        "DMG {:.0} | VEL {:.0} | PEN {:.0}%",
                                        a.damage,
                                        a.velocity,
                                        a.penetration * 100.0
                                    ),
                                    None,
                                ),
                                (
                                    format!(
                                        "RNG {:.0}m | DROP x{:.1}",
                                        a.range, a.drop_factor
                                    ),
                                    None,
                                ),
                            ];
                            // Shotgun shells reshape the scatter itself.
                            if a.pellet_count.is_some() || a.spread_cone.is_some() {
                                let pellets = a
                                    .pellet_count
                                    .map(|p| p.to_string())
                                    .unwrap_or_else(|| "-".to_string());
                                let cone = a
                                    .spread_cone
                                    .map(|c| format!("{c:.1} deg"))
                                    .unwrap_or_else(|| "-".to_string());
                                lines.push((format!("PELLETS {pellets} | SPREAD {cone}"), None));
                            }
                            options.push((
                                a.id.clone(),
                                a.name.clone(),
                                String::new(),
                                lines,
                            ));
                        }
                    } else {
                        let mut alts =
                            compatible_attachments(weapon_id, &slot_key, config, &attachment_registry);
                        // Hide entries that merely duplicate the gun's default mesh.
                        if let Some(default_mesh) =
                            crate::weapons::default_slot_mesh(&slot_key, &config.attachments)
                        {
                            alts.retain(|a| a.meta.mesh_path != default_mesh);
                        }
                        for a in alts {
                            let artist = if a.artist == "Unknown" {
                                String::new()
                            } else {
                                format!(" - by {}", a.artist)
                            };
                            let mut effects: Vec<(String, Option<bool>)> = a
                                .modifiers
                                .iter()
                                .filter_map(|(k, v)| {
                                    crate::weapons::modifier_effect_text(k, *v)
                                        .map(|(t, g)| (t, Some(g)))
                                })
                                .collect();
                            if slot_key == "magazine" {
                                if let (Some(cap), Some(carry)) =
                                    (a.capacity, a.carry_capacity)
                                {
                                    effects.push((
                                        format!("Capacity: {cap} / {carry}"),
                                        None,
                                    ));
                                }
                            }
                            effects.sort();
                            options.push((a.id.clone(), a.name.clone(), artist, effects));
                        }
                    }
                    let equipped_id = player_attachments.get(weapon_id, &slot_key);
                    commands.entity(list_entity).with_children(|parent| {
                        // Breadcrumb back row: gun › slot.
                        parent.spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(32.0),
                                flex_shrink: 0.0,
                                padding: UiRect::horizontal(Val::Px(10.0)),
                                align_items: AlignItems::Center,
                                margin: UiRect::bottom(Val::Px(4.0)),
                                border_radius: RADIUS_SM,
                                ..default()
                            },
                            BackgroundColor(BG_ELEVATED.with_alpha(0.75)),
                            AttachmentBackButton,
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new(format!("← {} > {}", config.info.name, slot_key_to_label(&slot_key).unwrap_or("?").to_uppercase())),
                                TextFont { font_size: FontSize::Px(13.0), ..default() },
                                TextColor(TEXT),
                            ));
                        });
                        // Gun default row.
                        let default_equipped = equipped_id.is_none();
                        parent.spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(32.0),
                                flex_shrink: 0.0,
                                padding: UiRect::horizontal(Val::Px(10.0)),
                                align_items: AlignItems::Center,
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                margin: UiRect::bottom(Val::Px(1.0)),
                                border_radius: RADIUS_SM,
                                ..default()
                            },
                            BackgroundColor(if default_equipped {
                                ACCENT.with_alpha(0.35)
                            } else {
                                BG_ELEVATED.with_alpha(0.75)
                            }),
                            AttachmentOptionButton { attachment_id: None },
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new("Default"),
                                TextFont { font_size: FontSize::Px(13.0), ..default() },
                                TextColor(if default_equipped { TEXT } else { TEXT_MUTED }),
                            ));
                            if default_equipped {
                                btn.spawn((
                                    Text::new("[E]"),
                                    TextFont { font_size: FontSize::Px(10.0), ..default() },
                                    TextColor(SUCCESS),
                                ));
                            }
                        });
                        // Explicit removal row (pools list "__none__" here).
                        if crate::weapons::slot_allows_none(config, &slot_key) {
                            let none_equipped = equipped_id == Some(crate::weapons::ATTACHMENT_NONE);
                            parent.spawn((
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(32.0),
                                    flex_shrink: 0.0,
                                    padding: UiRect::horizontal(Val::Px(10.0)),
                                    align_items: AlignItems::Center,
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    margin: UiRect::bottom(Val::Px(1.0)),
                                    border_radius: RADIUS_SM,
                                    ..default()
                                },
                                BackgroundColor(if none_equipped {
                                    ACCENT.with_alpha(0.35)
                                } else {
                                    BG_ELEVATED.with_alpha(0.75)
                                }),
                                AttachmentOptionButton {
                                    attachment_id: Some(crate::weapons::ATTACHMENT_NONE.to_string()),
                                },
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new("None"),
                                    TextFont { font_size: FontSize::Px(13.0), ..default() },
                                    TextColor(if none_equipped { TEXT } else { TEXT_MUTED }),
                                ));
                                if none_equipped {
                                    btn.spawn((
                                        Text::new("[E]"),
                                        TextFont { font_size: FontSize::Px(10.0), ..default() },
                                        TextColor(SUCCESS),
                                    ));
                                }
                            });
                        }
                        // Registry alternatives as plain name rows.
                        for (alt_id, alt_name, _alt_artist, _alt_effects) in options {
                            let is_equipped = equipped_id == Some(alt_id.as_str());
                            parent.spawn((
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(32.0),
                                    flex_shrink: 0.0,
                                    padding: UiRect::horizontal(Val::Px(10.0)),
                                    align_items: AlignItems::Center,
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    margin: UiRect::bottom(Val::Px(1.0)),
                                    border_radius: RADIUS_SM,
                                    ..default()
                                },
                                BackgroundColor(if is_equipped {
                                    ACCENT.with_alpha(0.35)
                                } else {
                                    BG_ELEVATED.with_alpha(0.75)
                                }),
                                AttachmentOptionButton {
                                    attachment_id: Some(alt_id),
                                },
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new(alt_name),
                                    TextFont { font_size: FontSize::Px(13.0), ..default() },
                                    TextColor(if is_equipped { TEXT } else { TEXT_MUTED }),
                                ));
                                if is_equipped {
                                    btn.spawn((
                                        Text::new("[E]"),
                                        TextFont { font_size: FontSize::Px(10.0), ..default() },
                                        TextColor(SUCCESS),
                                    ));
                                }
                            });
                        }
                    });
                }
            }
        } else if ui_state.browse == AttachmentBrowse::Slots {
            // Slot level: one row per slot the gun defines, showing the
            // effective part name. Tapping a row drills into its options.
            if let Some(weapon_id) = &ui_state.selected_weapon_id {
                if let Some(config) = registry.weapons.get(weapon_id) {
                    commands.entity(list_entity).with_children(|parent| {
                        parent.spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(32.0),
                                flex_shrink: 0.0,
                                padding: UiRect::horizontal(Val::Px(10.0)),
                                align_items: AlignItems::Center,
                                margin: UiRect::bottom(Val::Px(4.0)),
                                border_radius: RADIUS_SM,
                                ..default()
                            },
                            BackgroundColor(BG_ELEVATED.with_alpha(0.75)),
                            AttachmentBackButton,
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new(format!("← {}", config.info.name)),
                                TextFont { font_size: FontSize::Px(13.0), ..default() },
                                TextColor(TEXT),
                            ));
                        });
                        let mut any = false;
                        for slot_key in SLOT_ORDER {
                            if !browse_slot_defined(slot_key, config) {
                                continue;
                            }
                            any = true;
                            let name = browse_slot_current_name(
                                slot_key,
                                weapon_id,
                                config,
                                &player_attachments,
                                &attachment_registry,
                                &ammo_registry,
                            );
                            parent.spawn((
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(32.0),
                                    flex_shrink: 0.0,
                                    padding: UiRect::horizontal(Val::Px(10.0)),
                                    align_items: AlignItems::Center,
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    margin: UiRect::bottom(Val::Px(1.0)),
                                    border_radius: RADIUS_SM,
                                    ..default()
                                },
                                BackgroundColor(BG_ELEVATED.with_alpha(0.75)),
                                AttachmentSlotButton { slot: slot_key.to_string() },
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new(slot_key_to_label(slot_key).unwrap_or("?").to_uppercase()),
                                    TextFont { font_size: FontSize::Px(13.0), ..default() },
                                    TextColor(TEXT_MUTED),
                                ));
                                btn.spawn((
                                    Text::new(name),
                                    TextFont { font_size: FontSize::Px(13.0), ..default() },
                                    TextColor(TEXT),
                                ));
                            });
                        }
                        if !any {
                            parent.spawn((
                                Text::new("No attachment slots"),
                                TextFont { font_size: FontSize::Px(13.0), ..default() },
                                TextColor(TEXT_MUTED),
                            ));
                        }
                    });
                }
            }
        } else if let Some(ref active) = active_cat {
            if let Some((_, ids)) = categories.iter().find(|(c, _)| c == active) {
                commands.entity(list_entity).with_children(|parent| {
                    for id in ids {
                        let config = registry.weapons.get(id).unwrap();
                        let is_equipped = loadout.get_id_for_slot(slot) == id.as_str();
                        let is_selected = ui_state.selected_weapon_id.as_deref() == Some(id.as_str());

                        let bg = if is_selected {
                            ACCENT.with_alpha(0.35)
                        } else if is_equipped {
                            SUCCESS.with_alpha(0.18)
                        } else {
                            BG_ELEVATED.with_alpha(0.75)
                        };

                        parent.spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(32.0),
                                flex_shrink: 0.0,
                                padding: UiRect::horizontal(Val::Px(10.0)),
                                align_items: AlignItems::Center,
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::SpaceBetween,
                                margin: UiRect::bottom(Val::Px(1.0)),
                                border_radius: RADIUS_SM,
                                ..default()
                            },
                            BackgroundColor(bg),
                            WeaponSelectButton { weapon_id: id.to_string() },
                        )).with_children(|btn| {
                            btn.spawn((
                                Text::new(&config.info.name),
                                TextFont { font_size: FontSize::Px(13.0), ..default() },
                                TextColor(if is_selected { TEXT } else { TEXT_MUTED }),
                            ));
                            if is_equipped {
                                btn.spawn((
                                    Text::new("[E]"),
                                    TextFont { font_size: FontSize::Px(10.0), ..default() },
                                    TextColor(SUCCESS),
                                ));
                            }
                        });
                    }
                });
            }
        }
    }
    } // rebuild_list

    if rebuild_inspector {
    if let Some(stats_entity) = stats_query.iter().next() {
        commands.entity(stats_entity).despawn_children();

        if let Some(weapon_id) = &ui_state.selected_weapon_id {
            if let Some(config) = registry.weapons.get(weapon_id) {
                commands.entity(stats_entity).with_children(|parent| {
                    spawn_weapon_stats(
                        parent,
                        config,
                        weapon_id,
                        &loadout,
                        ui_state.active_slot,
                        &player_attachments,
                        &attachment_registry,
                        &ammo_registry,
                        &ui_state.show_advanced,
                    );

                    parent.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(BORDER),
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
                            border_radius: RADIUS,
                            ..default()
                        },
                        BackgroundColor(BG_ELEVATED),
                        ColorPickerButton,
                    )).with_children(|btn| {
                        btn.spawn((
                            Node {
                                width: Val::Px(18.0),
                                height: Val::Px(18.0),
                                ..default()
                            },
                            BackgroundColor(skin_registry.swatch_color(&ui_state.selected_skin.skin)),
                        ));
                        btn.spawn((
                            Text::new(format!("COLOR: {}", skin_registry.display_name(&ui_state.selected_skin.skin).to_uppercase())),
                            TextFont { font_size: FontSize::Px(12.0), ..default() },
                            TextColor(TEXT),
                        ));
                    });
                });
            }
        }
    }
    } // rebuild_inspector

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
        bg.0 = if is_active { ACCENT } else { BG_ELEVATED };
        for &child in children {
            if let Ok(mut text_color) = text_query.get_mut(child) {
                text_color.0 = if is_active { TEXT } else { TEXT_MUTED };
            }
        }
    }
}

fn spawn_weapon_stats(
    parent: &mut ChildSpawnerCommands,
    config: &WeaponConfig,
    weapon_id: &str,
    loadout: &PlayerLoadout,
    slot: WeaponSlot,
    _player_attachments: &PlayerAttachments,
    _attachment_registry: &AttachmentRegistry,
    _ammo_registry: &AmmoRegistry,
    show_advanced: &bool,
) {
    parent.spawn((
        Text::new(&config.info.name),
        TextFont { font_size: FontSize::Px(26.0), ..default() },
        TextColor(TEXT),
    ));

    parent.spawn((
        Text::new(format!("{} - {} - {}", config.meta.weapon_type, config.info.manufacturer, config.info.year_introduced)),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(TEXT_MUTED),
    ));

    parent.spawn((
        Text::new(&config.info.description),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(TEXT_MUTED),
        Node { margin: UiRect::vertical(Val::Px(6.0)), ..default() },
    ));

    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            margin: UiRect::vertical(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(BORDER),
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
                BackgroundColor(BORDER),
            ));
            parent.spawn((
                Text::new(format!("Ammo: {} | Damage: {:.0} | Pen: {:.0}%", ammo.name, ammo.damage, ammo.penetration * 100.0)),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(WARNING),
            ));
        }

        if let Some(mag) = &config.attachments.magazine {
            parent.spawn((
                Text::new(format!("Magazine: {} rds | Reserve: {}", mag.capacity, mag.carry_capacity)),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(TEXT_MUTED),
            ));
        }

        if !config.attributes.fire_modes.is_empty() {
            parent.spawn((
                Text::new(format!("Fire Modes: {}", config.attributes.fire_modes.join(" / "))),
                TextFont { font_size: FontSize::Px(12.0), ..default() },
                TextColor(TEXT_MUTED),
            ));
        }

        parent.spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(28.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::vertical(Val::Px(4.0)),
                border_radius: RADIUS_SM,
                ..default()
            },
            BackgroundColor(if *show_advanced { ACCENT.with_alpha(0.35) } else { BG_ELEVATED }),
            AdvancedToggleButton,
        )).with_children(|btn| {
            btn.spawn((
                Text::new(if *show_advanced { "ADVANCED ▲" } else { "ADVANCED ▼" }),
                TextFont { font_size: FontSize::Px(11.0), ..default() },
                TextColor(TEXT_MUTED),
            ));
        });

        if *show_advanced {
            if let Some(ammo) = &config.attachments.ammo {
                for line in [
                    format!("Damage: {:.0} | Velocity: {:.0} m/s", ammo.damage, ammo.velocity),
                    format!(
                        "Penetration: {:.0}% | Range: {:.0} m | Drop: x{:.1}",
                        ammo.penetration * 100.0,
                        ammo.range,
                        ammo.drop_factor
                    ),
                ] {
                    parent.spawn((
                        Text::new(line),
                        TextFont { font_size: FontSize::Px(12.0), ..default() },
                        TextColor(WARNING),
                    ));
                }
            }
            if let Some(mag) = &config.attachments.magazine {
                parent.spawn((
                    Text::new(format!(
                        "Magazine: {} rds | Reserve: {}",
                        mag.capacity, mag.carry_capacity
                    )),
                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                    TextColor(TEXT_MUTED),
                ));
            }
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
            border_radius: RADIUS,
            ..default()
        },
        BackgroundColor(if is_equipped { SUCCESS } else { ACCENT }),
        EquipButton,
    )).with_children(|btn: &mut ChildSpawnerCommands| {
        btn.spawn((
            Text::new(if is_equipped { "EQUIPPED" } else { "EQUIP" }),
            TextFont { font_size: FontSize::Px(18.0), ..default() },
            TextColor(TEXT),
        ));
    });
}

fn spawn_stat_bar(parent: &mut ChildSpawnerCommands, label: &str, value: f32, _max: f32) {
    parent.spawn(Node {
        width: Val::Percent(100.0),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceBetween,
        ..default()
    }).with_children(|row: &mut ChildSpawnerCommands| {
        row.spawn((
            Text::new(label),
            TextFont { font_size: FontSize::Px(12.0), ..default() },
            TextColor(TEXT_MUTED),
        ));
        row.spawn((
            Text::new(format!("{:.2}", value)),
            TextFont { font_size: FontSize::Px(12.0), ..default() },
            TextColor(TEXT),
        ));
    });
}

pub fn loadout_interaction(
    mut next_state: ResMut<NextState<GameState>>,
    mut ui_state: ResMut<LoadoutUiState>,
    mut loadout: ResMut<PlayerLoadout>,
    mut registry: ResMut<WeaponRegistry>,
    att: AttachmentRes,
    skins: SkinRes,
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
        // Escape steps back out of attachment browsing before leaving.
        match ui_state.browse {
            AttachmentBrowse::Options(_) => ui_state.browse = AttachmentBrowse::Slots,
            AttachmentBrowse::Slots => ui_state.browse = AttachmentBrowse::Closed,
            AttachmentBrowse::Closed => next_state.set(GameState::MainMenu),
        }
        return;
    }

    for (interaction, mut bg) in btn_queries.p0().iter_mut() {
        match interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    next_state.set(GameState::MainMenu);
                }
                *bg = BackgroundColor(BG_HOVER);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(BG_HOVER);
            }
            _ => {
                *bg = BackgroundColor(BG_ELEVATED);
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
                        sync_loadout_to_configs(
                            &mut registry,
                            &loadout,
                            &att.player,
                            &att.ammo,
                            &att.parts,
                        );
                        loadout.save();
                        ui_state.preview_needs_update = true;
                    }
                }
                *bg = if is_equipped {
                    BackgroundColor(SUCCESS)
                } else {
                    BackgroundColor(ACCENT_HOVER)
                };
            }
            Interaction::Hovered => {
                *bg = if is_equipped {
                    BackgroundColor(SUCCESS)
                } else {
                    BackgroundColor(ACCENT_HOVER)
                };
            }
            _ => {
                *bg = if is_equipped {
                    BackgroundColor(SUCCESS)
                } else {
                    BackgroundColor(ACCENT)
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
            ui_state.browse = AttachmentBrowse::Closed;
            ui_state.preview_needs_update = true;
        }
    }

    for (interaction, cat_btn) in category_query.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.active_category = Some(cat_btn.category.clone());
            // Changing sections leaves attachment browsing.
            ui_state.browse = AttachmentBrowse::Closed;
        }
    }

    for (interaction, weapon_btn) in weapon_query.iter() {
        if *interaction == Interaction::Pressed {
            // Click-again on the already-selected gun drills into its
            // attachment slots (same list); equipping stays on EQUIP.
            if ui_state.selected_weapon_id.as_deref() == Some(weapon_btn.weapon_id.as_str()) {
                ui_state.browse = AttachmentBrowse::Slots;
            } else {
                ui_state.selected_weapon_id = Some(weapon_btn.weapon_id.clone());
                ui_state.browse = AttachmentBrowse::Closed;
                ui_state.preview_needs_update = true;
            }
        }
    }

    for (interaction, skin_btn) in skin_query.iter() {
        if *interaction == Interaction::Pressed {
            if mouse_input.just_pressed(MouseButton::Left) {
                // Equip the best (lowest-wear) owned copy of the picked skin.
                let weapon_id = ui_state.selected_weapon_id.clone().unwrap_or_default();
                let instance = skins
                    .inventory
                    .copies_of(&weapon_id, &skin_btn.skin)
                    .into_iter()
                    .min_by(|a, b| a.wear.total_cmp(&b.wear))
                    .unwrap_or_else(|| SkinInstance::factory(skin_btn.skin.clone()));
                ui_state.selected_skin = instance.clone();
                loadout.set_skin(ui_state.active_slot, instance);
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
                            let selected_skin = ui_state.selected_skin.clone();
                            let weapon_id = ui_state.selected_weapon_id.clone().unwrap_or_default();
                            let mut owned_skins = vec![SkinId::default_id()];
                            for skin in skins.inventory.skins_for(&weapon_id) {
                                if !owned_skins.contains(&skin) {
                                    owned_skins.push(skin);
                                }
                            }
                            commands.entity(root_entity).with_children(|root| {
                                spawn_color_picker_panel(
                                    root,
                                    &selected_skin,
                                    &owned_skins,
                                    &skins.registry,
                                );
                            });
                        }
                    }
                }
                *bg = BackgroundColor(BG_HOVER);
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(BG_HOVER);
            }
            _ => {
                *bg = BackgroundColor(BG_ELEVATED);
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

/// Attachment slot/option/back buttons. Split out of `loadout_interaction`
/// (bevy caps systems at 16 params).
pub fn attachment_interaction(
    mut ui_state: ResMut<LoadoutUiState>,
    mut player_attachments: ResMut<PlayerAttachments>,
    mut registry: ResMut<WeaponRegistry>,
    loadout: Res<PlayerLoadout>,
    ammo_registry: Res<AmmoRegistry>,
    attachment_registry: Res<AttachmentRegistry>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    slot_query: Query<(&Interaction, &AttachmentSlotButton), (Changed<Interaction>, With<Button>)>,
    option_query: Query<(&Interaction, &AttachmentOptionButton), (Changed<Interaction>, With<Button>)>,
    back_query: Query<(&Interaction, &AttachmentBackButton), (Changed<Interaction>, With<Button>)>,
    toggle_query: Query<&Interaction, (Changed<Interaction>, With<AdvancedToggleButton>, With<Button>)>,
)
{
    for (interaction, slot_btn) in slot_query.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.browse = AttachmentBrowse::Options(slot_btn.slot.clone());
        }
    }

    for interaction in toggle_query.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.show_advanced = !ui_state.show_advanced;
        }
    }

    for (interaction, _) in back_query.iter() {
        if *interaction == Interaction::Pressed {
            // Options back up to slots, slots back up to the gun list.
            ui_state.browse = match &ui_state.browse {
                AttachmentBrowse::Options(_) => AttachmentBrowse::Slots,
                _ => AttachmentBrowse::Closed,
            };
        }
    }

    for (interaction, opt_btn) in option_query.iter() {
        if *interaction == Interaction::Pressed {
            if mouse_input.just_pressed(MouseButton::Left) {
                if let Some(gun_id) = ui_state.selected_weapon_id.clone() {
                    if let AttachmentBrowse::Options(slot) = ui_state.browse.clone() {
                        match &opt_btn.attachment_id {
                            Some(id) => {
                                player_attachments.set(gun_id, slot, id.clone());
                            }
                            None => {
                                player_attachments.clear(&gun_id, &slot);
                            }
                        }
                        player_attachments.save();
                        // Re-sync so catalog ammo + part modifiers fold into
                        // the configs (mesh-only changes re-apply values).
                        sync_loadout_to_configs(
                            &mut registry,
                            &loadout,
                            &player_attachments,
                            &ammo_registry,
                            &attachment_registry,
                        );
                        ui_state.attachments_rev = ui_state.attachments_rev.wrapping_add(1);
                        ui_state.preview_needs_update = true;
                    }
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
    if let Some(s) = &att.side { result.push(("Side".to_string(), s.name.clone())); }
    if let Some(m) = &att.magazine { result.push(("Magazine".to_string(), m.name.clone())); }
    if let Some(a) = &att.ammo { result.push(("Ammo".to_string(), a.name.clone())); }
    if let Some(st) = &att.stock { result.push(("Stock".to_string(), st.name.clone())); }
    result
}

fn spawn_color_picker_panel(parent: &mut ChildSpawnerCommands, selected_skin: &SkinInstance, owned_skins: &[SkinId], skin_registry: &SkinRegistry) {
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
            border_radius: RADIUS,
            ..default()
        },
        BackgroundColor(BG_PANEL),
        BorderColor::all(BORDER_STRONG),
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
                TextColor(TEXT),
            ));
            row.spawn((
                Button,
                Node {
                    width: Val::Px(28.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: RADIUS_SM,
                    ..default()
                },
                BackgroundColor(BG_ELEVATED),
                ColorPickerCloseButton,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("X"),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(TEXT),
                ));
            });
        });

        panel.spawn((
            Text::new("GUN BODY"),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(TEXT_MUTED),
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
                let is_active = *skin == selected_skin.skin;
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(if is_active { 2.0 } else { 1.0 })),
                        border_radius: RADIUS_SM,
                        ..default()
                    },
                    BackgroundColor(skin_registry.swatch_color(skin)),
                    BorderColor::from(if is_active { ACCENT } else { BORDER }),
                    SkinButton { skin: skin.clone() },
                )).with_children(|btn| {
                    if is_active {
                        btn.spawn((
                            Node {
                                width: Val::Px(10.0),
                                height: Val::Px(10.0),
                                ..default()
                            },
                            BackgroundColor(TEXT),
                        ));
                    }
                });
            }
        });

        panel.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(1.0),
                margin: UiRect::vertical(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(BORDER),
        ));

        panel.spawn((
            Text::new(format!(
                "Selected: {} ({})",
                skin_registry.display_name(&selected_skin.skin),
                selected_skin.wear_name()
            )),
            TextFont { font_size: FontSize::Px(11.0), ..default() },
            TextColor(TEXT_FAINT),
        ));

        let def = skin_registry.def_or_default(&selected_skin.skin);
        panel.spawn((
            Text::new(format!(
                "{} | wear {:.2} | pattern #{}{}",
                def.rarity.display_name(),
                selected_skin.wear,
                selected_skin.seed % 1000,
                if def.finish.is_animated() { " | animated" } else { "" }
            )),
            TextFont { font_size: FontSize::Px(10.0), ..default() },
            TextColor(def.rarity.color()),
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
    keyboard_input: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    mut drag_state: ResMut<LoadoutDragState>,
    mut model_query: Query<&mut Transform, With<LoadoutPreviewModel>>,
    mut scroll_events: MessageReader<bevy::input::mouse::MouseWheel>,
    time: Res<Time>,
    cam_pose: Res<crate::menu::main_menu::CameraPose>,
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
            // Shift + horizontal drag → roll; otherwise → yaw
            if keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight) {
                drag_state.rotation_z += delta.x * 0.01;
            } else {
                drag_state.rotation_y += delta.x * 0.01;
            }
            drag_state.rotation_x = (drag_state.rotation_x + delta.y * 0.01).clamp(-1.2, 1.2);
            drag_state.last_pos = pos;
        }
    } else {
        let speed = 4.0 * time.delta_secs();
        drag_state.rotation_y += (0.0 - drag_state.rotation_y) * speed;
        drag_state.rotation_x += (0.0 - drag_state.rotation_x) * speed;
        drag_state.rotation_z += (0.0 - drag_state.rotation_z) * speed;
        if drag_state.rotation_y.abs() < 0.001 { drag_state.rotation_y = 0.0; }
        if drag_state.rotation_x.abs() < 0.001 { drag_state.rotation_x = 0.0; }
        if drag_state.rotation_z.abs() < 0.001 { drag_state.rotation_z = 0.0; }
    }

    let cursor_over_list = if let Some(cursor_pos) = window.cursor_position() {
        cursor_pos.x < 280.0
    } else {
        false
    };

    if !cursor_over_list {
        for event in scroll_events.read() {
            drag_state.zoom = (drag_state.zoom - event.y * 0.15).clamp(-0.5, 1.5);
        }
    }

    // Position the weapon preview model in front of the lobby camera.
    let preview_pos = cam_pose.translation + cam_pose.forward * (PREVIEW_DIST + drag_state.zoom);
    for mut transform in model_query.iter_mut() {
        transform.translation = preview_pos;
        transform.rotation = base_rotation()
            * Quat::from_rotation_y(drag_state.rotation_y)
            * Quat::from_rotation_x(drag_state.rotation_x)
            * Quat::from_rotation_z(drag_state.rotation_z);
    }
}

pub fn update_loadout_preview_model(
    mut commands: Commands,
    mut ui_state: ResMut<LoadoutUiState>,
    registry: Res<WeaponRegistry>,
    skin_registry: Res<SkinRegistry>,
    player_attachments: Res<PlayerAttachments>,
    attachment_registry: Res<AttachmentRegistry>,
    game_settings: Res<crate::settings::GameSettings>,
    existing_model: Query<Entity, With<LoadoutPreviewModel>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut weapon_materials: ResMut<Assets<crate::weapons::WeaponMaterial>>,
    mut skin_cache: ResMut<crate::weapons::SkinMaterialCache>,
    drag_state: Res<LoadoutDragState>,
    cam_pose: Res<crate::menu::main_menu::CameraPose>,
) {
    if !ui_state.preview_needs_update {
        return;
    }
    ui_state.preview_needs_update = false;

    for entity in existing_model.iter() {
        commands.entity(entity).despawn();
    }

    let preview_pos = cam_pose.translation + cam_pose.forward * (PREVIEW_DIST + drag_state.zoom);

    if let Some(weapon_id) = &ui_state.selected_weapon_id {
        if let Some(config) = registry.weapons.get(weapon_id) {
            let model_file = config.meta.model_path.split('#').next().unwrap_or("");
            let model_exists = !model_file.is_empty()
                && std::path::Path::new(&format!("assets/{}", model_file)).exists();

            let scale = 0.3;
            let rotation = base_rotation()
                * Quat::from_rotation_y(drag_state.rotation_y)
                * Quat::from_rotation_x(drag_state.rotation_x)
                * Quat::from_rotation_z(drag_state.rotation_z);

            if model_exists {
                let skin = ui_state.selected_skin.clone();
                let preview_entity = commands.spawn((
                    WorldAssetRoot(asset_server.load(&config.meta.model_path)),
                    Transform::from_translation(preview_pos)
                        .with_rotation(rotation)
                        .with_scale(Vec3::splat(scale)),
                    LoadoutPreviewModel,
                    WeaponSkinTag::new(skin),
                )).id();
                // Assemble the equipped attachments on the preview too.
                let overrides = resolve_attachment_meshes(
                    weapon_id,
                    config,
                    &player_attachments,
                    &attachment_registry,
                );
                spawn_weapon_attachments(
                    &mut commands,
                    &asset_server,
                    preview_entity,
                    config,
                    &mut meshes,
                    &mut materials,
                    &overrides,
                    game_settings.debug.show_sockets,
                );
            } else {
                let slot = crate::weapons::slot_from_weapon_type(&config.meta.weapon_type);
                let skin = ui_state.selected_skin.clone();
                let mat = crate::weapons::skin_material(
                    &skin,
                    slot,
                    &skin_registry,
                    &asset_server,
                    &mut skin_cache,
                    &mut weapon_materials,
                    &mut materials,
                );
                let mesh = match slot {
                    WeaponSlot::Primary => meshes.add(Cuboid::new(0.08 * 3.0, 0.12 * 3.0, 0.6 * 3.0)),
                    WeaponSlot::Secondary => meshes.add(Cuboid::new(0.06 * 3.0, 0.12 * 3.0, 0.3 * 3.0)),
                    WeaponSlot::Melee => meshes.add(Cuboid::new(0.04 * 3.0, 0.04 * 3.0, 0.4 * 3.0)),
                    WeaponSlot::Equipment => meshes.add(Cuboid::new(0.15 * 3.0, 0.15 * 3.0, 0.15 * 3.0)),
                };
                let entity = commands.spawn((
                    Mesh3d(mesh),
                    Transform::from_translation(preview_pos)
                        .with_rotation(rotation)
                        .with_scale(Vec3::ONE),
                    LoadoutPreviewModel,
                )).id();
                mat.apply(&mut commands, entity);
            }
        }
    }
}

#[cfg(test)]
mod loadout_ui_tests {
    use super::*;
    
    #[test]
    fn panel_keys_split_list_and_inspector() {
        let base = || {
            loadout_panel_keys(
                WeaponSlot::Primary,
                Some("assault"),
                Some("hk416"),
                &AttachmentBrowse::Closed,
                0,
                SkinInstance::default(),
                false,
            )
        };
        let (list0, insp0) = base();
        // Browsing slots, then a slot's options: list only.
        let (list1, insp1) = loadout_panel_keys(
            WeaponSlot::Primary,
            Some("assault"),
            Some("hk416"),
            &AttachmentBrowse::Slots,
            0,
            SkinInstance::default(),
            false,
        );
        assert_ne!(list0, list1);
        assert_eq!(insp0, insp1);
        let (list1b, insp1b) = loadout_panel_keys(
            WeaponSlot::Primary,
            Some("assault"),
            Some("hk416"),
            &AttachmentBrowse::Options("optic".to_string()),
            0,
            SkinInstance::default(),
            false,
        );
        assert_ne!(list1, list1b);
        assert_eq!(insp1, insp1b);
        // Skin change: inspector only.
        let (list2, insp2) = loadout_panel_keys(
            WeaponSlot::Primary,
            Some("assault"),
            Some("hk416"),
            &AttachmentBrowse::Closed,
            0,
            SkinInstance::factory(SkinId::new("solid_red")),
            false,
        );
        assert_eq!(list0, list2);
        assert_ne!(insp0, insp2);
        // Equip (rev bump): both.
        let (list3, insp3) = loadout_panel_keys(
            WeaponSlot::Primary,
            Some("assault"),
            Some("hk416"),
            &AttachmentBrowse::Closed,
            1,
            SkinInstance::default(),
            false,
        );
        assert_ne!(list0, list3);
        assert_ne!(insp0, insp3);
    }

    #[test]
    fn list_scroll_clamps_into_range() {
        // 1000px of content in a 400px viewport scrolls 0..600.
        assert_eq!(list_scroll_offset(0.0, -1.0, 400.0, 1000.0), 20.0);
        assert_eq!(list_scroll_offset(0.0, 1.0, 400.0, 1000.0), 0.0);
        assert_eq!(list_scroll_offset(590.0, -1.0, 400.0, 1000.0), 600.0);
        assert_eq!(list_scroll_offset(600.0, -1.0, 400.0, 1000.0), 600.0);
        // Content fits: no scrolling.
        assert_eq!(list_scroll_offset(50.0, -1.0, 400.0, 300.0), 0.0);
    }

    #[test]
    fn scroll_thumb_tracks_position() {
        // Viewport 400 of 1000 in a 380px track: 152px thumb.
        let (h, y) = scroll_thumb_layout(400.0, 1000.0, 0.0, 380.0);
        assert!((h - 152.0).abs() < 0.01, "thumb height {h}");
        assert_eq!(y, 0.0);
        let (_, y_end) = scroll_thumb_layout(400.0, 1000.0, 600.0, 380.0);
        assert!((y_end - (380.0 - 152.0)).abs() < 0.01, "thumb end {y_end}");
        // Content fits: full-track thumb, no offset.
        assert_eq!(scroll_thumb_layout(400.0, 300.0, 0.0, 380.0), (380.0, 0.0));
    }
}
