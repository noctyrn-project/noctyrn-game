use bevy::camera::primitives::MeshAabb;
use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use crate::theme::*;
use crate::player::GameState;
use crate::menu::loadout::{category_display_name, category_sort_order};
use crate::weapons::{
    color_hex_compact, slot_from_weapon_type, WeaponRegistry, WeaponSkinTag, WeaponSlot,
    SkinRegistry, Inventory, PlayerCredits, Rarity, SkinId, SkinInstance, SkinRes,
    PATTERN_SHEET_PX, PATTERN_STAMP,
};

#[derive(Component)]
pub struct CosmeticsMenuUi;

#[derive(Component)]
pub struct CosmeticsBackButton;

/// Category tab (`assault`, `smg`, `blade`, …). `"all"` clears the filter.
#[derive(Component)]
pub struct CosmeticsCategoryButton {
    pub category: String,
}

/// Weapon sub-tab inside the active category. `"all"` clears the filter.
#[derive(Component)]
pub struct CosmeticsWeaponButton {
    pub weapon_id: String,
}

/// Containers rebuilt by `rebuild_cosmetics_ui` when a filter changes.
#[derive(Component)]
pub struct CosmeticsTabsRoot;
#[derive(Component)]
pub struct CosmeticsGridRoot;

#[derive(Component)]
pub struct CosmeticsSellButton {
    pub weapon_id: String,
    pub skin: SkinId,
}

/// Card body: click to show that skin on the 3D preview.
#[derive(Component)]
pub struct CosmeticsSelectButton {
    pub weapon_id: String,
    pub skin: SkinId,
}

/// UI state for the cosmetics browser + preview pane.
#[derive(Resource, Default)]
pub struct CosmeticsUiState {
    /// Weapon + skin currently shown in the 3D preview.
    pub selected: Option<(String, SkinId)>,
    /// Bumped when the preview model needs respawning.
    pub preview_rev: u64,
    pub last_preview_rev: u64,
    /// Offscreen render target the preview camera draws into.
    pub preview_image: Handle<Image>,
    /// Set when a new model spawns: the camera must be framed once its
    /// meshes finish loading.
    pub preview_needs_fit: bool,
    /// Active weapon category (`None` = every category).
    pub active_category: Option<String>,
    /// Active weapon inside the category (`None` = every weapon in it).
    pub active_weapon: Option<String>,
    /// Bumped to rebuild the tabs + grid without touching the preview.
    pub ui_rev: u64,
    pub last_ui_rev: u64,
}

/// Render-to-texture preview: a camera + lights + rotating model in a corner
/// of the world, shown in the UI pane.
#[derive(Component)]
pub struct CosmeticsPreviewCamera;
#[derive(Component)]
pub struct CosmeticsPreviewRoot;
#[derive(Component)]
pub struct CosmeticsPreviewModel;
#[derive(Component)]
pub struct CosmeticsPreviewInfo;

const PREVIEW_W: u32 = 512;
const PREVIEW_H: u32 = 384;
const PREVIEW_ORIGIN: Vec3 = Vec3::new(0.0, -200.0, 0.0);

/// Card swatch size. The motif sheet pixel size lives in `weapons::skins`
/// (`PATTERN_SHEET_PX`) so the crops match the shader.
const SWATCH_W: f32 = 160.0;
const SWATCH_H: f32 = 50.0;

#[derive(Resource, Default)]
pub struct SellConfirmState {
    pub weapon_id: String,
    pub skin: SkinId,
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

fn rarity_color(rarity: Rarity) -> Color {
    match rarity {
        Rarity::Common => TEXT_FAINT,
        Rarity::Uncommon => TEXT_MUTED,
        Rarity::Rare => ACCENT,
        Rarity::Epic => ACCENT_MAGENTA,
        Rarity::Legendary => WARNING,
        Rarity::Mythic => DANGER,
    }
}

/// Set up the offscreen preview: a camera rendering into a texture, its
/// lights, and the rotating root the selected gun is parented to. Everything
/// lives far below the lobby so the menu camera never sees it.
pub fn setup_cosmetics_preview(
    mut commands: Commands,
    mut ui_state: ResMut<CosmeticsUiState>,
    mut images: ResMut<Assets<Image>>,
) {
    let image = images.add(Image::new_target_texture(
        PREVIEW_W,
        PREVIEW_H,
        TextureFormat::Bgra8UnormSrgb,
        None,
    ));
    ui_state.preview_image = image.clone();
    ui_state.preview_rev += 1;

    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.07, 0.07, 0.1)),
            ..default()
        },
        RenderTarget::from(image),
        Projection::Perspective(PerspectiveProjection {
            fov: 40f32.to_radians(),
            ..default()
        }),
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(0.0, 0.15, 1.9))
            .looking_at(PREVIEW_ORIGIN, Vec3::Y),
        CosmeticsPreviewCamera,
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.95, 0.93, 1.0),
            illuminance: 600.0,
            ..default()
        },
        Transform::from_xyz(PREVIEW_ORIGIN.x + 2.0, PREVIEW_ORIGIN.y + 3.0, PREVIEW_ORIGIN.z + 2.0)
            .looking_at(PREVIEW_ORIGIN, Vec3::Y),
        CosmeticsPreviewCamera,
    ));
    commands.spawn((
        PointLight {
            color: Color::srgb(0.45, 0.55, 0.9),
            intensity: 1_500.0,
            range: 12.0,
            ..default()
        },
        Transform::from_translation(PREVIEW_ORIGIN + Vec3::new(-1.6, 0.8, 0.8)),
        CosmeticsPreviewCamera,
    ));
    commands.spawn((
        Transform::from_translation(PREVIEW_ORIGIN),
        Visibility::default(),
        CosmeticsPreviewRoot,
    ));
}

/// Respawn the preview model when the selection changes.
pub fn update_cosmetics_preview(
    mut commands: Commands,
    mut ui_state: ResMut<CosmeticsUiState>,
    registry: Res<WeaponRegistry>,
    skin_registry: Res<SkinRegistry>,
    inventory: Res<Inventory>,
    asset_server: Res<AssetServer>,
    existing: Query<Entity, With<CosmeticsPreviewModel>>,
    root: Query<Entity, With<CosmeticsPreviewRoot>>,
    info: Query<Entity, With<CosmeticsPreviewInfo>>,
) {
    if ui_state.preview_rev == ui_state.last_preview_rev {
        return;
    }
    ui_state.last_preview_rev = ui_state.preview_rev;

    for entity in existing.iter() {
        commands.entity(entity).try_despawn();
    }

    let selected = ui_state.selected.clone();
    for entity in info.iter() {
        commands.entity(entity).despawn_children();
        commands.entity(entity).with_children(|parent| {
            spawn_preview_info(
                parent,
                selected.as_ref().map(|(w, s)| (w.as_str(), s)),
                &registry,
                &skin_registry,
                &inventory,
            );
        });
    }

    let Some(root) = root.iter().next() else {
        return;
    };
    let Some((weapon_id, skin)) = selected else {
        return;
    };
    let Some(config) = registry.weapons.get(&weapon_id) else {
        return;
    };

    // Show the best (lowest-wear) owned copy, matching the card.
    let instance = inventory
        .copies_of(&weapon_id, &skin)
        .into_iter()
        .min_by(|a, b| a.wear.total_cmp(&b.wear))
        .unwrap_or_else(|| SkinInstance::factory(skin.clone()));

    commands.entity(root).with_children(|root| {
        root.spawn((
            WorldAssetRoot(asset_server.load(&config.meta.model_path)),
            Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2))
                .with_scale(Vec3::splat(config.meta.scale)),
            CosmeticsPreviewModel,
            WeaponSkinTag::new(instance),
        ));
    });
    ui_state.preview_needs_fit = true;
}

/// Frame the preview camera on the model once its meshes load: fit the
/// bounding sphere of everything sitting at the preview origin.
pub fn fit_cosmetics_preview(
    mut ui_state: ResMut<CosmeticsUiState>,
    mut cameras: Query<&mut Transform, With<CosmeticsPreviewCamera>>,
    meshes: Query<(&GlobalTransform, &Mesh3d)>,
    mesh_assets: Res<Assets<Mesh>>,
    mut waited: Local<u32>,
) {
    if !ui_state.preview_needs_fit {
        return;
    }
    // Give up after a few seconds so a broken model can't spin this forever.
    *waited += 1;
    if *waited > 300 {
        ui_state.preview_needs_fit = false;
        *waited = 0;
        return;
    }

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for (global, mesh3d) in meshes.iter() {
        if global.translation().distance(PREVIEW_ORIGIN) > 20.0 {
            continue;
        }
        let Some(mesh) = mesh_assets.get(&mesh3d.0) else {
            continue;
        };
        let Some(aabb) = mesh.compute_aabb() else {
            continue;
        };
        for i in 0..8 {
            let corner = Vec3::new(
                if i & 1 == 0 { aabb.min().x } else { aabb.max().x },
                if i & 2 == 0 { aabb.min().y } else { aabb.max().y },
                if i & 4 == 0 { aabb.min().z } else { aabb.max().z },
            );
            let world = global.transform_point(corner);
            min = min.min(world);
            max = max.max(world);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return;
    }

    let center = (min + max) * 0.5;
    let radius = ((max - min) * 0.5).length().max(0.05);
    let vfov = 40f32.to_radians();
    let aspect = PREVIEW_W as f32 / PREVIEW_H as f32;
    let hfov = 2.0 * ((vfov * 0.5).tan() * aspect).atan();
    let distance = (radius / (vfov * 0.5).sin())
        .max(radius / (hfov * 0.5).sin())
        * 1.05;

    for mut transform in cameras.iter_mut() {
        let eye = center + Vec3::new(0.0, 0.18, 1.0).normalize() * distance;
        *transform = Transform::from_translation(eye).looking_at(center, Vec3::Y);
    }
    ui_state.preview_needs_fit = false;
    *waited = 0;
}

/// Slow turntable so the finish catches the light.
pub fn spin_cosmetics_preview(
    time: Res<Time>,
    mut roots: Query<&mut Transform, With<CosmeticsPreviewRoot>>,
) {
    for mut transform in roots.iter_mut() {
        transform.rotate_y(time.delta_secs() * 0.6);
    }
}

/// One owned (weapon, skin) stack, resolved for display and grouping.
struct CosmeticsRow {
    weapon_id: String,
    weapon_name: String,
    category: String,
    /// Primary → Secondary → Melee → Equipment, so categories from different
    /// slots never interleave.
    slot_rank: u8,
    skin: SkinId,
    count: u32,
    best: SkinInstance,
}

fn slot_rank(slot: WeaponSlot) -> u8 {
    match slot {
        WeaponSlot::Primary => 0,
        WeaponSlot::Secondary => 1,
        WeaponSlot::Melee => 2,
        WeaponSlot::Equipment => 3,
    }
}

/// Gather the inventory into display rows, grouped by weapon category (the
/// same categories the loadout uses), then weapon name, then rarity.
fn collect_rows(
    inventory: &Inventory,
    registry: &WeaponRegistry,
    skin_registry: &SkinRegistry,
) -> Vec<CosmeticsRow> {
    let mut rows: Vec<CosmeticsRow> = Vec::new();
    for (weapon_id, skin, count) in inventory.listing() {
        if skin.is_default() {
            continue;
        }
        let config = registry.weapons.get(&weapon_id);
        let weapon_name = config
            .map(|c| c.info.name.clone())
            .unwrap_or_else(|| weapon_id.replace('_', " "));
        let category = config
            .map(|c| c.meta.category.clone())
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| "other".to_string());
        let slot_rank = config
            .map(|c| slot_rank(slot_from_weapon_type(&c.meta.weapon_type)))
            .unwrap_or(0);
        // Best (lowest-wear) copy represents the stack in the browser.
        let best = inventory
            .copies_of(&weapon_id, &skin)
            .into_iter()
            .min_by(|a, b| a.wear.total_cmp(&b.wear))
            .unwrap_or_else(|| SkinInstance::factory(skin.clone()));
        rows.push(CosmeticsRow {
            weapon_id,
            weapon_name,
            category,
            slot_rank,
            skin,
            count,
            best,
        });
    }
    rows.sort_by(|a, b| {
        a.slot_rank
            .cmp(&b.slot_rank)
            .then_with(|| category_sort_order(&a.category).cmp(&category_sort_order(&b.category)))
            .then_with(|| a.weapon_name.cmp(&b.weapon_name))
            .then_with(|| {
                (skin_registry.rarity_of(&a.skin) as u8).cmp(&(skin_registry.rarity_of(&b.skin) as u8))
            })
    });
    rows
}

fn tab_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    active: bool,
    marker: impl Bundle,
    font_size: f32,
) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::new(Val::Px(14.0), Val::Px(14.0), Val::Px(6.0), Val::Px(6.0)),
                border_radius: RADIUS,
                ..default()
            },
            BackgroundColor(if active { ACCENT } else { BG_ELEVATED }),
            marker,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(font_size),
                    ..default()
                },
                TextColor(if active { TEXT } else { TEXT_MUTED }),
            ));
        });
}

/// Category row + the weapon row for the active category, mirroring the
/// loadout's category tabs.
fn build_cosmetics_tabs(
    parent: &mut ChildSpawnerCommands,
    rows: &[CosmeticsRow],
    ui_state: &CosmeticsUiState,
) {
    // Categories in the order they first appear (rows are already sorted by
    // slot then category, so blades stay with blades, pistols with pistols).
    let mut categories: Vec<String> = Vec::new();
    for row in rows {
        if !categories.iter().any(|c| c == &row.category) {
            categories.push(row.category.clone());
        }
    }

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(6.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|tabs| {
            tab_button(
                tabs,
                "ALL",
                ui_state.active_category.is_none(),
                CosmeticsCategoryButton {
                    category: "all".to_string(),
                },
                12.0,
            );
            for category in &categories {
                let active = ui_state.active_category.as_deref() == Some(category.as_str());
                tab_button(
                    tabs,
                    &category_display_name(category).to_uppercase(),
                    active,
                    CosmeticsCategoryButton {
                        category: category.clone(),
                    },
                    12.0,
                );
            }
        });

    // Weapon sub-tabs only make sense inside a category: showing every
    // owned weapon at once is the "scattered" list the grouping replaced.
    let Some(active_category) = ui_state.active_category.as_deref() else {
        return;
    };
    let mut weapons: Vec<(String, String)> = Vec::new();
    for row in rows {
        if row.category == active_category && !weapons.iter().any(|(id, _)| id == &row.weapon_id) {
            weapons.push((row.weapon_id.clone(), row.weapon_name.clone()));
        }
    }
    if weapons.len() <= 1 {
        return;
    }

    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(6.0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|tabs| {
            tab_button(
                tabs,
                "ALL",
                ui_state.active_weapon.is_none(),
                CosmeticsWeaponButton {
                    weapon_id: "all".to_string(),
                },
                11.0,
            );
            for (id, name) in &weapons {
                let active = ui_state.active_weapon.as_deref() == Some(id.as_str());
                tab_button(
                    tabs,
                    &name.to_uppercase(),
                    active,
                    CosmeticsWeaponButton {
                        weapon_id: id.clone(),
                    },
                    11.0,
                );
            }
        });
}

/// Cards for every owned stack matching the active category/weapon filter.
fn build_cosmetics_grid(
    parent: &mut ChildSpawnerCommands,
    rows: &[CosmeticsRow],
    ui_state: &CosmeticsUiState,
    skin_registry: &SkinRegistry,
    asset_server: &AssetServer,
) {
    let visible: Vec<&CosmeticsRow> = rows
        .iter()
        .filter(|row| {
            ui_state
                .active_category
                .as_deref()
                .map_or(true, |c| row.category == c)
                && ui_state
                    .active_weapon
                    .as_deref()
                    .map_or(true, |w| row.weapon_id == w)
        })
        .collect();

    if visible.is_empty() {
        parent.spawn((
            Text::new("No skins here yet. Open some crates!"),
            TextFont {
                font_size: FontSize::Px(16.0),
                ..default()
            },
            TextColor(TEXT_FAINT),
            Node {
                margin: UiRect::top(Val::Px(40.0)),
                ..default()
            },
        ));
        return;
    }

    for row in visible {
        let CosmeticsRow {
            weapon_id,
            weapon_name,
            skin,
            count,
            best,
            ..
        } = row;
        let rarity = skin_registry.rarity_of(skin);
        let sell_price = PlayerCredits::sell_value(rarity);

        parent
            .spawn(Node {
                width: Val::Px(180.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                border: UiRect::all(Val::Px(2.0)),
                border_radius: RADIUS,
                ..default()
            })
            .with_children(|card| {
                // Patterned skins show a slice of their motif sheet, tinted
                // by the base colour; everything else is a solid swatch.
                let def = skin_registry.def_or_default(skin);
                let pattern_sheet = def
                    .finish
                    .pattern
                    .as_ref()
                    .map(|p| asset_server.load(&p.sheet));
                card.spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(SWATCH_H),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border_radius: RADIUS_SM,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(skin_registry.swatch_color(skin)),
                    CosmeticsSelectButton {
                        weapon_id: weapon_id.clone(),
                        skin: skin.clone(),
                    },
                ))
                .with_children(|swatch| {
                    if let Some(sheet) = pattern_sheet {
                        let crop = def.finish.pattern_crop_origin(best.seed);
                        let band_h = PATTERN_SHEET_PX * SWATCH_H / SWATCH_W;
                        let oy = (crop[1] / (1.0 - PATTERN_STAMP)).clamp(0.0, 1.0);
                        swatch.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                top: Val::Px(0.0),
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            ImageNode {
                                rect: Some(Rect::new(
                                    0.0,
                                    oy * (PATTERN_SHEET_PX - band_h),
                                    PATTERN_SHEET_PX,
                                    band_h,
                                )),
                                color: Color::srgb(
                                    def.finish.base_color[0],
                                    def.finish.base_color[1],
                                    def.finish.base_color[2],
                                ),
                                ..ImageNode::new(sheet)
                            },
                        ));
                    }
                    swatch.spawn((
                        Text::new(skin_registry.display_name(skin)),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(TEXT),
                    ));
                });

                card.spawn((
                    Text::new(rarity.display_name()),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(rarity_color(rarity)),
                ));

                card.spawn((
                    Text::new(weapon_name.as_str()),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(TEXT_MUTED),
                ));

                card.spawn((
                    Text::new(format!("{} - {:.2}", best.wear_name(), best.wear)),
                    TextFont {
                        font_size: FontSize::Px(10.0),
                        ..default()
                    },
                    TextColor(TEXT_FAINT),
                ));

                if def.finish.has_pattern() {
                    card.spawn((
                        Text::new(format!("pattern #{}", best.seed % 1000)),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(ACCENT),
                    ));
                }

                if *count > 1 {
                    card.spawn((
                        Text::new(format!("Owned: x{}", count)),
                        TextFont {
                            font_size: FontSize::Px(10.0),
                            ..default()
                        },
                        TextColor(TEXT_FAINT),
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
                        border_radius: RADIUS_SM,
                        ..default()
                    },
                    BackgroundColor(DANGER),
                    CosmeticsSellButton {
                        weapon_id: weapon_id.clone(),
                        skin: skin.clone(),
                    },
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(format!("SELL [C]{}", sell_price)),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(TEXT),
                    ));
                });
            })
            .insert(BackgroundColor(BG_PANEL))
            .insert(BorderColor::from(rarity_color(rarity)));
    }
}

/// Rebuild the tabs + grid after a category/weapon filter change. The
/// preview pane and its camera are deliberately left alone.
pub fn rebuild_cosmetics_ui(
    mut commands: Commands,
    mut ui_state: ResMut<CosmeticsUiState>,
    inventory: Res<Inventory>,
    registry: Res<WeaponRegistry>,
    skin_registry: Res<SkinRegistry>,
    asset_server: Res<AssetServer>,
    tabs: Query<Entity, With<CosmeticsTabsRoot>>,
    grid: Query<Entity, With<CosmeticsGridRoot>>,
) {
    if ui_state.ui_rev == ui_state.last_ui_rev {
        return;
    }
    ui_state.last_ui_rev = ui_state.ui_rev;

    let rows = collect_rows(&inventory, &registry, &skin_registry);
    if let Ok(tabs) = tabs.single() {
        commands.entity(tabs).despawn_children();
        commands.entity(tabs).with_children(|parent| {
            build_cosmetics_tabs(parent, &rows, &ui_state);
        });
    }
    if let Ok(grid) = grid.single() {
        commands.entity(grid).despawn_children();
        commands.entity(grid).with_children(|parent| {
            build_cosmetics_grid(parent, &rows, &ui_state, &skin_registry, &asset_server);
        });
    }
}

pub fn spawn_cosmetics_menu(
    mut commands: Commands,
    credits: Res<PlayerCredits>,
    inventory: Res<Inventory>,
    registry: Res<WeaponRegistry>,
    skin_registry: Res<SkinRegistry>,
    ui_state: Res<CosmeticsUiState>,
    asset_server: Res<AssetServer>,
) {
    let rows = collect_rows(&inventory, &registry, &skin_registry);

    // Centered card with dark background — lobby scene shows through around it.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(5.0),
            top: Val::Percent(5.0),
            width: Val::Percent(90.0),
            height: Val::Percent(90.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(40.0)),
            row_gap: Val::Px(16.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: RADIUS,
            ..default()
        },
        BackgroundColor(BG_BASE),
        BorderColor::all(BORDER),
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
                        border_radius: RADIUS,
                        ..default()
                    },
                    BackgroundColor(BG_ELEVATED),
                    CosmeticsBackButton,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("BACK"),
                        TextFont { font_size: FontSize::Px(14.0), ..default() },
                        TextColor(TEXT),
                    ));
                });

                left.spawn((
                    Text::new("COSMETICS"),
                    TextFont { font_size: FontSize::Px(32.0), ..default() },
                    TextColor(TEXT),
                ));
            });

            header.spawn((
                Text::new(format!("[C] {} Credits", credits.balance)),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                TextColor(WARNING),
            ));
        });

        root.spawn((
            Text::new("Browse your skins. Click SELL to trade a skin for credits."),
            TextFont { font_size: FontSize::Px(13.0), ..default() },
            TextColor(TEXT_FAINT),
        ));

        // Category + weapon tabs. Rebuilt by `rebuild_cosmetics_ui` when a
        // filter changes.
        root.spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                ..default()
            },
            CosmeticsTabsRoot,
        ))
        .with_children(|tabs| {
            build_cosmetics_tabs(tabs, &rows, &ui_state);
        });

        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            flex_grow: 1.0,
            column_gap: Val::Px(16.0),
            min_height: Val::Px(0.0),
            ..default()
        }).with_children(|content| {
            // Card grid, rebuilt by `rebuild_cosmetics_ui` on filter changes.
            content.spawn((
                Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(10.0),
                    row_gap: Val::Px(10.0),
                    align_content: AlignContent::FlexStart,
                    justify_content: JustifyContent::Center,
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                CosmeticsGridRoot,
            ))
            .with_children(|grid| {
                build_cosmetics_grid(grid, &rows, &ui_state, &skin_registry, &asset_server);
            });

        // Preview pane: the selected skin on its actual gun, rendered to a
        // texture by the offscreen camera.
        content.spawn(Node {
            width: Val::Px(400.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(8.0),
            ..default()
        }).with_children(|pane| {
            pane.spawn((
                Node {
                    width: Val::Px(400.0),
                    height: Val::Px(300.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: RADIUS,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(BG_PANEL),
                BorderColor::from(BORDER_STRONG),
                ImageNode::new(ui_state.preview_image.clone()),
            ));

            let selected = ui_state.selected.clone();
            pane.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                CosmeticsPreviewInfo,
            ))
            .with_children(|info| {
                spawn_preview_info(
                    info,
                    selected.as_ref().map(|(w, s)| (w.as_str(), s)),
                    &registry,
                    &skin_registry,
                    &inventory,
                );
            });
        });
        });
    });
}

/// The text under the preview image: skin, rarity, gun, wear + compact hex,
/// and the pattern seed when the finish has a pattern.
fn spawn_preview_info(
    parent: &mut ChildSpawnerCommands,
    selected: Option<(&str, &SkinId)>,
    registry: &WeaponRegistry,
    skin_registry: &SkinRegistry,
    inventory: &Inventory,
) {
    let Some((weapon_id, skin)) = selected else {
        parent.spawn((
            Text::new("Select a skin to preview it"),
            TextFont { font_size: FontSize::Px(14.0), ..default() },
            TextColor(TEXT_FAINT),
            Node { margin: UiRect::top(Val::Px(20.0)), ..default() },
        ));
        return;
    };

    let weapon_name = registry
        .weapons
        .get(weapon_id)
        .map(|c| c.info.name.clone())
        .unwrap_or_else(|| weapon_id.to_string());
    let def = skin_registry.def_or_default(skin);
    let rarity = def.rarity;
    let best = inventory
        .copies_of(weapon_id, skin)
        .into_iter()
        .min_by(|a, b| a.wear.total_cmp(&b.wear));
    let (wear_name, wear, seed) = best
        .map(|b| (b.wear_name(), b.wear, b.seed))
        .unwrap_or(("Factory New", 0.0, 0));

    parent.spawn((
        Text::new(def.name.clone()),
        TextFont { font_size: FontSize::Px(22.0), ..default() },
        TextColor(TEXT),
    ));
    parent.spawn((
        Text::new(rarity.display_name()),
        TextFont { font_size: FontSize::Px(13.0), ..default() },
        TextColor(rarity_color(rarity)),
    ));
    parent.spawn((
        Text::new(format!("for {weapon_name}")),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        TextColor(TEXT_MUTED),
    ));
    parent.spawn((
        Text::new(format!(
            "{} - {:.2}  {}",
            wear_name,
            wear,
            color_hex_compact(def.finish.base_color)
        )),
        TextFont { font_size: FontSize::Px(12.0), ..default() },
        TextColor(TEXT_FAINT),
    ));
    if def.finish.has_pattern() {
        parent.spawn((
            Text::new(format!("pattern #{}", seed % 1000)),
            TextFont { font_size: FontSize::Px(11.0), ..default() },
            TextColor(ACCENT),
        ));
    }
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
    select_query: Query<(&Interaction, &CosmeticsSelectButton), (Changed<Interaction>, With<Button>)>,
    category_query: Query<(&Interaction, &CosmeticsCategoryButton), (Changed<Interaction>, With<Button>)>,
    weapon_query: Query<(&Interaction, &CosmeticsWeaponButton), (Changed<Interaction>, With<Button>)>,
    mut ui_state: ResMut<CosmeticsUiState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    skins: SkinRes,
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

    for (interaction, select_btn) in select_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            ui_state.selected = Some((select_btn.weapon_id.clone(), select_btn.skin.clone()));
            ui_state.preview_rev += 1;
        }
    }

    for (interaction, category_btn) in category_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            ui_state.active_category = if category_btn.category == "all" {
                None
            } else {
                Some(category_btn.category.clone())
            };
            // Weapons belong to one category; a stale weapon filter would
            // hide everything.
            ui_state.active_weapon = None;
            ui_state.ui_rev += 1;
        }
    }

    for (interaction, weapon_btn) in weapon_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            ui_state.active_weapon = if weapon_btn.weapon_id == "all" {
                None
            } else {
                Some(weapon_btn.weapon_id.clone())
            };
            ui_state.ui_rev += 1;
        }
    }

    for (interaction, sell_btn) in sell_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            let rarity = skins.registry.rarity_of(&sell_btn.skin);
            let price = PlayerCredits::sell_value(rarity);
            let count = skins.inventory.skin_count(&sell_btn.weapon_id, &sell_btn.skin).max(1);

            sell_state.weapon_id = sell_btn.weapon_id.clone();
            sell_state.skin = sell_btn.skin.clone();
            sell_state.quantity = 1;
            sell_state.max_quantity = count;
            sell_state.sell_price_each = price;

            if let Some(root) = cosmetics_ui.iter().next() {
                spawn_sell_confirm_dialog(&mut commands, root, &sell_state, &skins.registry);
            }
        }
    }
}

fn spawn_sell_confirm_dialog(
    commands: &mut Commands,
    root: Entity,
    sell_state: &SellConfirmState,
    skin_registry: &SkinRegistry,
) {
    let skin = &sell_state.skin;
    let rarity = skin_registry.rarity_of(skin);
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
                    border_radius: RADIUS,
                    ..default()
                },
                BackgroundColor(BG_PANEL),
                BorderColor::from(DANGER),
            )).with_children(|card| {
                card.spawn((
                    Text::new("CONFIRM SELL"),
                    TextFont { font_size: FontSize::Px(20.0), ..default() },
                    TextColor(TEXT),
                ));

                card.spawn((
                    Node {
                        width: Val::Px(80.0),
                        height: Val::Px(80.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: RADIUS,
                        ..default()
                    },
                    BackgroundColor(skin_registry.swatch_color(skin)),
                    BorderColor::from(rarity_color(rarity)),
                )).with_children(|swatch| {
                    swatch.spawn((
                        Text::new(skin_registry.display_name(skin)),
                        TextFont { font_size: FontSize::Px(11.0), ..default() },
                        TextColor(TEXT),
                    ));
                });

                card.spawn((
                    Text::new(format!("{} Skin", rarity.display_name())),
                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                    TextColor(rarity_color(rarity)),
                ));

                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                }).with_children(|row| {
                    row.spawn((
                        Text::new("Quantity:"),
                        TextFont { font_size: FontSize::Px(14.0), ..default() },
                        TextColor(TEXT_MUTED),
                    ));

                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: RADIUS_SM,
                            ..default()
                        },
                        BackgroundColor(BG_ELEVATED),
                        SellQuantityMinus,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("-"),
                            TextFont { font_size: FontSize::Px(18.0), ..default() },
                            TextColor(TEXT),
                        ));
                    });

                    row.spawn((
                        Text::new(format!("{}", sell_state.quantity)),
                        TextFont { font_size: FontSize::Px(20.0), ..default() },
                        TextColor(TEXT),
                        SellQuantityText,
                    ));

                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: RADIUS_SM,
                            ..default()
                        },
                        BackgroundColor(BG_ELEVATED),
                        SellQuantityPlus,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("+"),
                            TextFont { font_size: FontSize::Px(18.0), ..default() },
                            TextColor(TEXT),
                        ));
                    });

                    row.spawn((
                        Button,
                        Node {
                            padding: UiRect::horizontal(Val::Px(8.0)),
                            height: Val::Px(28.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: RADIUS_SM,
                            ..default()
                        },
                        BackgroundColor(BG_ELEVATED),
                        SellQuantityMax,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("MAX"),
                            TextFont { font_size: FontSize::Px(11.0), ..default() },
                            TextColor(TEXT_MUTED),
                        ));
                    });
                });

                card.spawn((
                    Text::new(format!("Owned: x{}", sell_state.max_quantity)),
                    TextFont { font_size: FontSize::Px(11.0), ..default() },
                    TextColor(TEXT_FAINT),
                ));

                card.spawn((
                    Text::new(format!("Total: [C] {} Credits", total_price)),
                    TextFont { font_size: FontSize::Px(16.0), ..default() },
                    TextColor(WARNING),
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
                            border_radius: RADIUS,
                            ..default()
                        },
                        BackgroundColor(BG_ELEVATED),
                        SellCancelButton,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("CANCEL"),
                            TextFont { font_size: FontSize::Px(14.0), ..default() },
                            TextColor(TEXT),
                        ));
                    });

                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(38.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: RADIUS,
                            ..default()
                        },
                        BackgroundColor(DANGER),
                        SellConfirmButton,
                    )).with_children(|btn| {
                        btn.spawn((
                            Text::new("SELL"),
                            TextFont { font_size: FontSize::Px(14.0), ..default() },
                            TextColor(TEXT),
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
    mut category_query: Query<
        (&Interaction, &mut BackgroundColor, &CosmeticsCategoryButton),
        (With<Button>, Without<CosmeticsBackButton>, Without<CosmeticsSellButton>, Without<CosmeticsWeaponButton>),
    >,
    mut weapon_query: Query<
        (&Interaction, &mut BackgroundColor, &CosmeticsWeaponButton),
        (With<Button>, Without<CosmeticsBackButton>, Without<CosmeticsSellButton>, Without<CosmeticsCategoryButton>),
    >,
) {
    for (interaction, mut bg) in back_query.iter_mut() {
        *bg = match interaction {
            Interaction::Pressed => BackgroundColor(BG_HOVER),
            Interaction::Hovered => BackgroundColor(BG_HOVER),
            _ => BackgroundColor(BG_ELEVATED),
        };
    }

    // Sell buttons keep their danger colour; the rest lighten on hover. The
    // active tab keeps its accent fill (pressed/hovered only nudges it).
    for (interaction, mut bg, _) in sell_query.iter_mut() {
        *bg = match interaction {
            Interaction::Pressed => BackgroundColor(DANGER),
            Interaction::Hovered => BackgroundColor(DANGER),
            _ => BackgroundColor(DANGER),
        };
    }

    for (interaction, mut bg, _) in category_query.iter_mut() {
        if *bg == BackgroundColor(ACCENT) {
            continue;
        }
        *bg = match interaction {
            Interaction::Pressed | Interaction::Hovered => BackgroundColor(BG_HOVER),
            _ => BackgroundColor(BG_ELEVATED),
        };
    }

    for (interaction, mut bg, _) in weapon_query.iter_mut() {
        if *bg == BackgroundColor(ACCENT) {
            continue;
        }
        *bg = match interaction {
            Interaction::Pressed | Interaction::Hovered => BackgroundColor(BG_HOVER),
            _ => BackgroundColor(BG_ELEVATED),
        };
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
    mut inventory: ResMut<Inventory>,
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
            let skin = sell_state.skin.clone();

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
