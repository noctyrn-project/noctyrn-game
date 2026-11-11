//! Crate browser + 3D opening sequence.
//!
//! Browsing shows the crate cards and odds. Opening plays in three phases:
//! `Intro` (the crate drops in, latches pop, the lid swings open) → `Reel`
//! (the strip spins to the winning item) → `Reveal` (the item floats above
//! the crate with rarity FX). A click during any phase skips that phase.

use bevy::camera::primitives::MeshAabb;
use bevy::prelude::*;

use crate::menu::main_menu::Menu3dCamera;
use crate::player::GameState;
use crate::theme::*;
use crate::weapons::{
    color_hex_compact, resolve_attachment_meshes, slot_from_weapon_type,
    spawn_weapon_attachments, AttachmentRegistry, CrateDef, CrateRegistry, CrateReward,
    Inventory, PlayerAttachments, PlayerCredits, Rarity, SkinId, SkinInstance, SkinRegistry,
    WeaponRegistry, WeaponSlot, PATTERN_SHEET_PX, PATTERN_STAMP,
};

// ── 3D crate layout (crate-space, matches the split parts) ───────────────
const LID_HINGE: Vec3 = Vec3::new(-1.0, 1.522, 0.0);
const LATCH_CENTER: Vec3 = Vec3::new(1.0485, 1.466, 0.0);
const LATCH_POP: f32 = 0.18;
const LID_OPEN_ANGLE: f32 = 1.95;
/// Half the old 0.34 — the animated crate was crowding the screen.
const CRATE_SCALE: f32 = 0.17;
const CRATE_LOCAL: Vec3 = Vec3::new(0.0, -0.22, -1.45);
/// Crate +X (the latch face) rotated to look at the camera.
const CRATE_YAW: f32 = -std::f32::consts::FRAC_PI_2;

/// Model scale for the revealed prize is fitted at runtime
/// (`align_reveal_model`), so every gun presents at the same size.
const REVEAL_TARGET_SIZE: f32 = 0.45;
/// The prize floats where the crate was, owning the middle of the screen
/// while the result card docks at the bottom.
const REVEAL_MODEL_HEIGHT: f32 = 0.3;

/// Slide the opened crate down and out of frame once the prize is revealed,
/// so the gun owns the screen. The lid/latches ride along (children).
pub fn slide_crate_out(
    crate_state: Res<CrateState>,
    mut roots: Query<&mut Transform, With<CrateRoot>>,
) {
    if crate_state.phase != CratePhase::Reveal {
        return;
    }
    // Ease out over the reveal pop, then hold it out of sight.
    let t = (crate_state.phase_time / REVEAL_DURATION).clamp(0.0, 1.0);
    let drop = 1.6 * t * t * (3.0 - 2.0 * t);
    for mut transform in roots.iter_mut() {
        transform.translation = CRATE_LOCAL + Vec3::new(0.0, -drop, 0.0);
        transform.rotation = Quat::from_rotation_y(CRATE_YAW);
        transform.scale = Vec3::splat(CRATE_SCALE);
    }
}

const INTRO_DURATION: f32 = 1.5;
/// Long, smooth deceleration like CSGO's case opening.
const REEL_DURATION: f32 = 7.0;
const REVEAL_DURATION: f32 = 1.6;
/// Reveal timeline (seconds): the prize lifts out of the crate mouth, grows
/// to hero size as the crate drops away, and only then does the card appear.
const REVEAL_LIFT_END: f32 = 0.7;
const REVEAL_GROW_START: f32 = 0.35;
const REVEAL_GROW_END: f32 = 1.2;
const REVEAL_CARD_AT: f32 = 1.25;
/// Hero size multiplier applied once the prize clears the crate.
const REVEAL_GROW_FACTOR: f32 = 1.7;
/// The prize starts inside the crate mouth and rises out — never pops in
/// from the camera.
const REVEAL_START_Y: f32 = 0.02;
/// Ignore skip clicks for this long so the opening click doesn't skip.
const SKIP_GRACE: f32 = 0.25;

/// Strip length and the cell index the winner sits on: more cells means more
/// options fly past before the slow tail settles.
const STRIP_CELLS: usize = 80;
const WINNER_CELL: usize = 60;

/// Reel cell metrics: width + gap, and the strip's half-width padding so the
/// winning cell sits under the pointer.
const CELL_W: f32 = 120.0;
const CELL_GAP: f32 = 6.0;
const CELL_STEP: f32 = CELL_W + CELL_GAP;
const REEL_W: f32 = 900.0;
const REEL_H: f32 = 150.0;

/// The rendered crate icon used on the crate cards.
#[derive(Resource, Default)]
pub struct CrateAssets {
    pub icon: Handle<Image>,
}

pub fn load_crate_assets(mut assets: ResMut<CrateAssets>, asset_server: Res<AssetServer>) {
    assets.icon = asset_server.load("ui/crate_icon.png");
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CratePhase {
    #[default]
    Browsing,
    Intro,
    Reel,
    Reveal,
}

#[derive(Resource, Default)]
pub struct CrateState {
    pub phase: CratePhase,
    pub phase_time: f32,
    pub selected_crate: Option<String>,
    pub reward: Option<CrateReward>,
    pub strip: Vec<SkinId>,
    pub strip_offset: f32,
    pub spin_target: f32,
    /// Copies of the won skin owned before granting it (0 = new).
    pub owned_before: u32,
    pub card_shown: bool,
    pub reveal_spawned: bool,
    /// Bumped to request a browsing-UI rebuild without touching the 3D crate.
    pub ui_rev: u64,
    pub last_ui_rev: u64,
}

// ── components ───────────────────────────────────────────────────────────
#[derive(Component)]
pub struct CrateMenuUi;
#[derive(Component)]
pub struct CrateOpeningOverlay;
#[derive(Component)]
pub struct CrateReelContainer;
#[derive(Component)]
pub struct CrateStripInner;
#[derive(Component)]
pub struct CrateSkipHint;
#[derive(Component)]
pub struct CrateResultPanel;
#[derive(Component)]
pub struct CrateResultDismiss;

#[derive(Component)]
pub struct CrateBackButton;
#[derive(Component)]
pub struct SellDuplicatesButton;
#[derive(Component)]
pub struct CrateSelectButton {
    pub crate_id: String,
}

// 3D crate entities
#[derive(Component)]
pub struct CrateRoot;
#[derive(Component)]
pub struct CrateLid;
#[derive(Component)]
pub struct CrateLatches;
#[derive(Component)]
pub struct CrateRevealModel;

// ── data gathering ───────────────────────────────────────────────────────
struct CrateUiData {
    crates: Vec<CrateDef>,
    credits: u64,
    dupes: u32,
}

fn gather_ui_data(
    crate_registry: &CrateRegistry,
    credits: &PlayerCredits,
    inventory: &Inventory,
) -> CrateUiData {
    CrateUiData {
        crates: crate_registry.crates.clone(),
        credits: credits.balance,
        dupes: inventory.total_duplicates(),
    }
}

// ── menu lifecycle ───────────────────────────────────────────────────────
pub fn spawn_crate_menu(
    mut commands: Commands,
    mut crate_state: ResMut<CrateState>,
    mut reveal_drag: ResMut<RevealDragState>,
    crate_registry: Res<CrateRegistry>,
    credits: Res<PlayerCredits>,
    inventory: Res<Inventory>,
    crate_assets: Res<CrateAssets>,
    camera: Query<Entity, With<Menu3dCamera>>,
    asset_server: Res<AssetServer>,
) {
    *crate_state = CrateState::default();
    *reveal_drag = RevealDragState::default();

    spawn_crate_scene(&mut commands, &camera, &asset_server);

    let data = gather_ui_data(&crate_registry, &credits, &inventory);

    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(4.0),
                top: Val::Percent(4.0),
                width: Val::Percent(92.0),
                height: Val::Percent(92.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(28.0)),
                row_gap: Val::Px(14.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: RADIUS,
                ..default()
            },
            BackgroundColor(BG_BASE.with_alpha(0.82)),
            BorderColor::all(BORDER),
            CrateMenuUi,
        ))
        .id();
    build_crate_ui(&mut commands, root, &data, &crate_assets);
}

fn spawn_crate_scene(
    commands: &mut Commands,
    camera: &Query<Entity, With<Menu3dCamera>>,
    asset_server: &AssetServer,
) {
    let root = commands
        .spawn((
            Transform::from_translation(CRATE_LOCAL)
                .with_rotation(Quat::from_rotation_y(CRATE_YAW))
                .with_scale(Vec3::splat(CRATE_SCALE)),
            Visibility::default(),
            CrateRoot,
        ))
        .id();

    commands.entity(root).with_children(|crate_root| {
        crate_root.spawn((
            WorldAssetRoot(asset_server.load("ui/crate/body.glb#Scene0")),
            Transform::IDENTITY,
            Visibility::default(),
        ));
        crate_root.spawn((
            WorldAssetRoot(asset_server.load("ui/crate/lid.glb#Scene0")),
            Transform::from_translation(LID_HINGE),
            Visibility::default(),
            CrateLid,
        ));
        crate_root.spawn((
            WorldAssetRoot(asset_server.load("ui/crate/latches.glb#Scene0")),
            Transform::from_translation(LATCH_CENTER),
            Visibility::default(),
            CrateLatches,
        ));
    });

    if let Ok(cam) = camera.single() {
        commands.entity(cam).add_child(root);
    }
}

pub fn despawn_crate_menu(
    mut commands: Commands,
    ui: Query<Entity, With<CrateMenuUi>>,
    overlay: Query<Entity, With<CrateOpeningOverlay>>,
    crate_root: Query<Entity, With<CrateRoot>>,
    reveal: Query<Entity, With<CrateRevealModel>>,
    cards: Query<Entity, With<CrateResultPanel>>,
) {
    // The result card is its own root, so Escape/BACK mid-reveal would
    // otherwise leave the popup hanging over the next menu.
    for entity in ui
        .iter()
        .chain(overlay.iter())
        .chain(crate_root.iter())
        .chain(reveal.iter())
        .chain(cards.iter())
    {
        commands.entity(entity).try_despawn();
    }
}

// ── browsing UI ──────────────────────────────────────────────────────────
fn build_crate_ui(
    commands: &mut Commands,
    root: Entity,
    data: &CrateUiData,
    crate_assets: &CrateAssets,
) {
    commands.entity(root).with_children(|root| {
        // Header
        root.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .with_children(|header| {
            header
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(16.0),
                    ..default()
                })
                .with_children(|left| {
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
                        CrateBackButton,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("BACK"),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(TEXT),
                        ));
                    });

                    left.spawn((
                        Text::new("CRATES"),
                        TextFont {
                            font_size: FontSize::Px(34.0),
                            ..default()
                        },
                        TextColor(TEXT),
                    ));
                });

            header
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(12.0),
                    ..default()
                })
                .with_children(|right| {
                    right.spawn((
                        Text::new(format!("[C] {} Credits", data.credits)),
                        TextFont {
                            font_size: FontSize::Px(16.0),
                            ..default()
                        },
                        TextColor(WARNING),
                    ));

                    if data.dupes > 0 {
                        right
                            .spawn((
                                Button,
                                Node {
                                    height: Val::Px(34.0),
                                    padding: UiRect::horizontal(Val::Px(14.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    border_radius: RADIUS,
                                    ..default()
                                },
                                BackgroundColor(SUCCESS),
                                SellDuplicatesButton,
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(format!("SELL {} DUPLICATES", data.dupes)),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(TEXT),
                                ));
                            });
                    }
                });
        });

        root.spawn((
            Text::new("Open crates for weapon skins and melee unlocks. Sell duplicates for credits."),
            TextFont {
                font_size: FontSize::Px(13.0),
                ..default()
            },
            TextColor(TEXT_FAINT),
        ));

        // Crate cards
        root.spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(16.0),
            justify_content: JustifyContent::Center,
            flex_wrap: FlexWrap::Wrap,
            row_gap: Val::Px(16.0),
            ..default()
        })
        .with_children(|row| {
            for def in &data.crates {
                let accent = def.accent_color();
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(230.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(16.0)),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: RADIUS,
                        row_gap: Val::Px(8.0),
                        ..default()
                    },
                    BackgroundColor(BG_ELEVATED),
                    BorderColor::from(accent),
                    CrateSelectButton {
                        crate_id: def.id.clone(),
                    },
                ))
                .with_children(|card| {
                    // The rendered crate model, framed by the tier accent.
                    card.spawn((
                        Node {
                            width: Val::Px(112.0),
                            height: Val::Px(112.0),
                            border: UiRect::all(Val::Px(2.0)),
                            border_radius: RADIUS_SM,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(BG_PANEL),
                        BorderColor::from(accent),
                        ImageNode::new(crate_assets.icon.clone()),
                    ));

                    card.spawn((
                        Text::new(def.name.clone()),
                        TextFont {
                            font_size: FontSize::Px(17.0),
                            ..default()
                        },
                        TextColor(TEXT),
                    ));

                    card.spawn((
                        Text::new(def.description.clone()),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(TEXT_MUTED),
                    ));

                    card.spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        ..default()
                    })
                    .with_children(|rates| {
                        for (rarity, pct) in def.odds() {
                            rates
                                .spawn(Node {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..default()
                                })
                                .with_children(|r| {
                                    r.spawn((
                                        Text::new(rarity.display_name()),
                                        TextFont {
                                            font_size: FontSize::Px(10.0),
                                            ..default()
                                        },
                                        TextColor(rarity.color()),
                                    ));
                                    r.spawn((
                                        Text::new(format!("{pct:.1}%")),
                                        TextFont {
                                            font_size: FontSize::Px(10.0),
                                            ..default()
                                        },
                                        TextColor(TEXT_FAINT),
                                    ));
                                });
                        }
                        if def.melee_chance > 0.0 {
                            rates.spawn((
                                Text::new(format!(
                                    "Melee unlock: {:.0}%",
                                    def.melee_chance * 100.0
                                )),
                                TextFont {
                                    font_size: FontSize::Px(10.0),
                                    ..default()
                                },
                                TextColor(ACCENT),
                            ));
                        }
                    });

                    card.spawn((
                        Text::new(format!("[C] {} CREDITS", def.cost)),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(if data.credits >= def.cost { WARNING } else { DANGER }),
                    ));
                });
            }
        });
    });
}

/// Rebuild just the browsing UI when `ui_rev` changes (credits spent).
/// The 3D crate and any opening overlay are untouched.
pub fn rebuild_crate_ui(
    mut commands: Commands,
    mut crate_state: ResMut<CrateState>,
    crate_registry: Res<CrateRegistry>,
    credits: Res<PlayerCredits>,
    inventory: Res<Inventory>,
    crate_assets: Res<CrateAssets>,
    ui: Query<Entity, With<CrateMenuUi>>,
) {
    if crate_state.ui_rev == crate_state.last_ui_rev {
        return;
    }
    crate_state.last_ui_rev = crate_state.ui_rev;

    for entity in ui.iter() {
        commands.entity(entity).try_despawn();
    }

    let data = gather_ui_data(&crate_registry, &credits, &inventory);

    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(4.0),
                top: Val::Percent(4.0),
                width: Val::Percent(92.0),
                height: Val::Percent(92.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(28.0)),
                row_gap: Val::Px(14.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: RADIUS,
                ..default()
            },
            BackgroundColor(BG_BASE.with_alpha(0.82)),
            BorderColor::all(BORDER),
            CrateMenuUi,
        ))
        .id();
    build_crate_ui(&mut commands, root, &data, &crate_assets);
}

// ── interaction ──────────────────────────────────────────────────────────
#[allow(clippy::too_many_arguments)]
pub fn crate_interaction(
    mut next_state: ResMut<NextState<GameState>>,
    mut crate_state: ResMut<CrateState>,
    crate_registry: Res<CrateRegistry>,
    skin_registry: Res<SkinRegistry>,
    registry: Res<WeaponRegistry>,
    mut credits: ResMut<PlayerCredits>,
    mut inventory: ResMut<Inventory>,
    mut commands: Commands,
    mut crate_select_query: Query<
        (&Interaction, &CrateSelectButton, &mut BackgroundColor),
        (
            Changed<Interaction>,
            With<Button>,
            Without<CrateBackButton>,
            Without<SellDuplicatesButton>,
        ),
    >,
    mut back_query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            With<CrateBackButton>,
            With<Button>,
            Without<CrateSelectButton>,
            Without<SellDuplicatesButton>,
        ),
    >,
    mut sell_query: Query<
        (&Interaction, &mut BackgroundColor),
        (
            With<SellDuplicatesButton>,
            With<Button>,
            Without<CrateBackButton>,
            Without<CrateSelectButton>,
        ),
    >,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::MainMenu);
        return;
    }

    for (interaction, mut bg) in back_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left) {
                    next_state.set(GameState::MainMenu);
                }
                *bg = BackgroundColor(BG_HOVER);
            }
            Interaction::Hovered => *bg = BackgroundColor(BG_HOVER),
            _ => *bg = BackgroundColor(BG_ELEVATED),
        }
    }

    for (interaction, mut bg) in sell_query.iter_mut() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            let sold = inventory.sell_all_duplicates();
            let total: u64 = sold
                .iter()
                .map(|(_, skin, count)| {
                    PlayerCredits::sell_value(skin_registry.rarity_of(skin)) * *count as u64
                })
                .sum();
            credits.balance += total;
            credits.save();
            inventory.save();
            crate_state.ui_rev += 1;
        }
        *bg = BackgroundColor(SUCCESS);
    }

    // Start an opening.
    for (interaction, select, mut bg) in crate_select_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                if mouse_input.just_pressed(MouseButton::Left)
                    && crate_state.phase == CratePhase::Browsing
                {
                    let Some(def) = crate_registry.get(&select.crate_id) else {
                        continue;
                    };
                    if credits.balance < def.cost {
                        continue;
                    }
                    credits.balance -= def.cost;
                    credits.save();

                    let reward = crate_registry.roll_reward(
                        def,
                        &skin_registry,
                        &registry,
                        &inventory,
                    );

                    // Grant immediately: closing the menu mid-animation must
                    // never lose a paid-for item.
                    let owned_before = match &reward {
                        CrateReward::GunSkin { weapon_id, instance } => {
                            inventory.skin_count(weapon_id, &instance.skin)
                        }
                        CrateReward::MeleeUnlock { .. } => 0,
                    };
                    let _ = crate::weapons::crates::grant_reward(&reward, &mut inventory);
                    inventory.save();

                    // Build the reel, guaranteeing the winner sits under the
                    // pointer at `WINNER_CELL`.
                    let mut strip: Vec<SkinId> = Vec::new();
                    let winner_skin = match &reward {
                        CrateReward::GunSkin { instance, .. } => instance.skin.clone(),
                        CrateReward::MeleeUnlock { .. } => SkinId::default_id(),
                    };
                    for _ in 0..STRIP_CELLS {
                        strip.push(pick_strip_skin(def, &skin_registry));
                    }
                    strip[WINNER_CELL] = winner_skin;
                    crate_state.strip = strip;
                    // Land at a random point inside the winning cell so the
                    // pointer isn't always dead-centre, and never overshoot
                    // (no bounce).
                    let jitter = {
                        use rand::Rng;
                        let mut rng = rand::rng();
                        rng.random_range(CELL_STEP * 0.3..CELL_STEP * 0.7)
                    };
                    crate_state.spin_target = WINNER_CELL as f32 * CELL_STEP + jitter;
                    crate_state.strip_offset = 0.0;
                    crate_state.reward = Some(reward);
                    crate_state.selected_crate = Some(select.crate_id.clone());
                    crate_state.owned_before = owned_before;
                    crate_state.phase = CratePhase::Intro;
                    crate_state.phase_time = 0.0;
                    crate_state.card_shown = false;
                    crate_state.reveal_spawned = false;

                    spawn_opening_overlay(&mut commands, &crate_state, &skin_registry, &asset_server);
                }
                *bg = BackgroundColor(BG_HOVER);
            }
            Interaction::Hovered => *bg = BackgroundColor(BG_HOVER),
            _ => *bg = BackgroundColor(BG_ELEVATED),
        }
    }
}


/// A reel filler cell: rolled with the crate's own rarity weights and theme
/// pool, so the strip looks consistent with the odds printed on the card.
fn pick_strip_skin(def: &CrateDef, skins: &SkinRegistry) -> SkinId {
    crate::weapons::crates::roll_skin(def, skins, &mut rand::rng())
}

fn spawn_opening_overlay(
    commands: &mut Commands,
    crate_state: &CrateState,
    skin_registry: &SkinRegistry,
    asset_server: &AssetServer,
) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(40.0)),
                ..default()
            },
            // Light backdrop only: the crate animation is the point.
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.2)),
            CrateOpeningOverlay,
            ZIndex(20),
        ))
        .with_children(|overlay| {
            overlay.spawn((
                Text::new("CLICK TO SKIP"),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(TEXT_FAINT),
                CrateSkipHint,
            ));

            // Reel sits at the bottom so it never covers the crate. Spawned
            // hidden: the intro phase would otherwise flash the strip for a
            // frame before `update_crate_animation` hides it.
            overlay
                .spawn((
                    Node {
                        width: Val::Px(REEL_W),
                        height: Val::Px(REEL_H),
                        display: Display::None,
                        overflow: Overflow::clip(),
                        border: UiRect::all(Val::Px(2.0)),
                        border_radius: RADIUS,
                        ..default()
                    },
                    BackgroundColor(BG_BASE),
                    BorderColor::from(BORDER_STRONG),
                    CrateReelContainer,
                ))
                .with_children(|container| {
                    container
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Px(CELL_GAP),
                                height: Val::Percent(100.0),
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(REEL_W / 2.0)),
                                left: Val::Px(0.0),
                                ..default()
                            },
                            CrateStripInner,
                        ))
                        .with_children(|row| {
                            for skin in &crate_state.strip {
                                let rarity = skin_registry.rarity_of(skin);
                                let def = skin_registry.def_or_default(skin);
                                let pattern_sheet = def
                                    .finish
                                    .pattern
                                    .as_ref()
                                    .map(|p| asset_server.load(&p.sheet));
                                let base = def.finish.base_color;
                                row.spawn((
                                    Node {
                                        width: Val::Px(CELL_W),
                                        min_width: Val::Px(CELL_W),
                                        height: Val::Px(CELL_W),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(2.0)),
                                        border_radius: RADIUS_SM,
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(3.0),
                                        overflow: Overflow::clip(),
                                        ..default()
                                    },
                                    BackgroundColor(skin_registry.swatch_color(skin)),
                                    BorderColor::from(rarity.color()),
                                ))
                                .with_children(|cell| {
                                    // Patterned skins show a slice of their
                                    // motif sheet tinted by the base colour,
                                    // matching the gun and the cosmetics card.
                                    if let Some(sheet) = pattern_sheet {
                                        let crop = def.finish.pattern_crop_origin(0);
                                        let size = PATTERN_SHEET_PX * PATTERN_STAMP;
                                        cell.spawn((
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
                                                    crop[0] * PATTERN_SHEET_PX,
                                                    crop[1] * PATTERN_SHEET_PX,
                                                    size,
                                                    size,
                                                )),
                                                color: Color::srgb(base[0], base[1], base[2]),
                                                ..ImageNode::new(sheet)
                                            },
                                        ));
                                    }
                                    cell.spawn((
                                        Text::new(skin_registry.display_name(skin)),
                                        TextFont {
                                            font_size: FontSize::Px(11.0),
                                            ..default()
                                        },
                                        TextColor(TEXT),
                                    ));
                                    cell.spawn((
                                        Text::new(rarity.display_name()),
                                        TextFont {
                                            font_size: FontSize::Px(10.0),
                                            ..default()
                                        },
                                        TextColor(rarity.color()),
                                    ));
                                });
                            }
                        });

                    // Centre pointer: a thin accent line through the reel
                    // with a diamond head, marking the selected cell.
                    container.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(REEL_W / 2.0 - 1.5),
                            top: Val::Px(0.0),
                            width: Val::Px(3.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(ACCENT.with_alpha(0.85)),
                        ZIndex(2),
                    ));
                    container.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(REEL_W / 2.0 - 8.0),
                            top: Val::Px(0.0),
                            width: Val::Px(16.0),
                            height: Val::Px(16.0),
                            ..default()
                        },
                        UiTransform::from_rotation(Rot2::degrees(45.0)),
                        BackgroundColor(ACCENT),
                        ZIndex(3),
                    ));
                });

            overlay.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(TEXT_FAINT),
            ));
        });
}

/// Click anywhere to skip the current animation phase.
pub fn crate_skip_interaction(
    mut crate_state: ResMut<CrateState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    if !mouse_input.just_pressed(MouseButton::Left) {
        return;
    }
    if crate_state.phase_time < SKIP_GRACE {
        return;
    }
    match crate_state.phase {
        CratePhase::Intro => crate_state.phase_time = INTRO_DURATION * 0.99,
        CratePhase::Reel => crate_state.phase_time = REEL_DURATION * 0.99,
        CratePhase::Reveal => {
            if !crate_state.card_shown {
                crate_state.phase_time = REVEAL_DURATION * 0.99;
            }
        }
        CratePhase::Browsing => {}
    }
}

/// Dismiss the reveal card and return to browsing.
pub fn crate_result_interaction(
    mut crate_state: ResMut<CrateState>,
    mut reveal_drag: ResMut<RevealDragState>,
    mut commands: Commands,
    dismiss_query: Query<&Interaction, (Changed<Interaction>, With<CrateResultDismiss>)>,
    overlay: Query<Entity, With<CrateOpeningOverlay>>,
    reveal: Query<Entity, With<CrateRevealModel>>,
    cards: Query<Entity, With<CrateResultPanel>>,
    mut crate_roots: Query<&mut Transform, With<CrateRoot>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    for interaction in dismiss_query.iter() {
        if *interaction == Interaction::Pressed && mouse_input.just_pressed(MouseButton::Left) {
            // The result card is its own root (not a child of the overlay),
            // so it has to be despawned explicitly.
            for entity in overlay.iter().chain(reveal.iter()).chain(cards.iter()) {
                commands.entity(entity).try_despawn();
            }
            // Bring the crate back: it slid out from under the prize.
            for mut transform in crate_roots.iter_mut() {
                transform.translation = CRATE_LOCAL;
                transform.rotation = Quat::from_rotation_y(CRATE_YAW);
                transform.scale = Vec3::splat(CRATE_SCALE);
            }
            crate_state.phase = CratePhase::Browsing;
            crate_state.phase_time = 0.0;
            crate_state.reward = None;
            crate_state.strip.clear();
            crate_state.card_shown = false;
            crate_state.reveal_spawned = false;
            crate_state.ui_rev += 1;
            *reveal_drag = RevealDragState::default();
        }
    }
}

/// Bundled resources for spawning the revealed prize (keeps the animation
/// system under Bevy's param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct RevealSpawnCtx<'w> {
    registry: Res<'w, WeaponRegistry>,
    skin_registry: Res<'w, SkinRegistry>,
    asset_server: Res<'w, AssetServer>,
    attachment_registry: Res<'w, AttachmentRegistry>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

// ── animation ────────────────────────────────────────────────────────────
#[allow(clippy::too_many_arguments)]
pub fn update_crate_animation(
    mut commands: Commands,
    time: Res<Time>,
    mut crate_state: ResMut<CrateState>,
    mut strip_query: Query<&mut Node, (With<CrateStripInner>, Without<CrateReelContainer>)>,
    mut reel_query: Query<&mut Node, (With<CrateReelContainer>, Without<CrateStripInner>)>,
    mut lid_query: Query<
        &mut Transform,
        (With<CrateLid>, Without<CrateLatches>, Without<CrateRevealModel>),
    >,
    mut latch_query: Query<
        &mut Transform,
        (With<CrateLatches>, Without<CrateLid>, Without<CrateRevealModel>),
    >,

    mut reveal_query: Query<&mut Transform, With<CrateRevealModel>>,
    // Disjoint from the strip/reel queries above: the browsing menu is never
    // a strip cell or the reel container.
    mut menu_query: Query<
        &mut Node,
        (
            With<CrateMenuUi>,
            Without<CrateReelContainer>,
            Without<CrateStripInner>,
        ),
    >,
    mut reveal_ctx: RevealSpawnCtx,
    reveal_drag: Res<RevealDragState>,
    camera: Query<Entity, With<Menu3dCamera>>,
) {
    if crate_state.phase == CratePhase::Browsing {
        return;
    }

    crate_state.phase_time += time.delta_secs();
    let t = crate_state.phase_time;

    // The browsing UI covers the crate, so hide it for the whole opening.
    if let Ok(mut menu) = menu_query.single_mut() {
        menu.display = Display::None;
    }

    // Hide the reel outside the spin phase.
    if let Ok(mut reel) = reel_query.single_mut() {
        reel.display = if crate_state.phase == CratePhase::Reel {
            Display::Flex
        } else {
            Display::None
        };
    }

    match crate_state.phase {
        CratePhase::Intro => {
            let latch_t = ((t - 0.45) / 0.25).clamp(0.0, 1.0);
            let latch_ease = 1.0 - (1.0 - latch_t).powi(3);

            let lid_t = ((t - 0.6) / 0.4).clamp(0.0, 1.0);
            let lid_ease = lid_t * lid_t * (3.0 - 2.0 * lid_t);
            let lid_rot = Quat::from_rotation_z(LID_OPEN_ANGLE * lid_ease);

            if let Ok(mut lid) = lid_query.single_mut() {
                lid.translation = LID_HINGE;
                lid.rotation = lid_rot;
            }

            // The latches are metal buckles bolted to the lid: they pop out a
            // touch, then ride the lid's rotation about the hinge.
            if let Ok(mut latch) = latch_query.single_mut() {
                let local = (LATCH_CENTER - LID_HINGE) + Vec3::X * LATCH_POP * latch_ease;
                latch.translation = LID_HINGE + lid_rot * local;
                latch.rotation = lid_rot;
            }

            if t >= INTRO_DURATION {
                crate_state.phase = CratePhase::Reel;
                crate_state.phase_time = 0.0;
            }
        }
        CratePhase::Reel => {
            // Long, smooth ease-out that lands exactly on target — no bounce.
            // A high exponent means the strip blasts off and spends most of
            // the time crawling through the last cells.
            let t_norm = (t / REEL_DURATION).clamp(0.0, 1.0);
            let eased = 1.0 - (1.0 - t_norm).powi(6);
            crate_state.strip_offset = eased * crate_state.spin_target;
            if let Ok(mut strip) = strip_query.single_mut() {
                strip.left = Val::Px(-crate_state.strip_offset);
            }

            if t >= REEL_DURATION {
                crate_state.phase = CratePhase::Reveal;
                crate_state.phase_time = 0.0;
            }
        }
        CratePhase::Reveal => {
            if !crate_state.reveal_spawned {
                crate_state.reveal_spawned = true;
                if let Some(reward) = crate_state.reward.clone() {
                    spawn_reveal_model(
                        &mut commands,
                        &reward,
                        &reveal_ctx.registry,
                        &reveal_ctx.asset_server,
                        &reveal_ctx.attachment_registry,
                        &mut reveal_ctx.meshes,
                        &mut reveal_ctx.materials,
                        &camera,
                    );
                }
            }

            // 1. Lift the prize out of the crate mouth to its display height.
            let lift_t = (t / REVEAL_LIFT_END).clamp(0.0, 1.0);
            let lift_ease = 1.0 - (1.0 - lift_t).powi(3);
            let y = REVEAL_START_Y + (REVEAL_MODEL_HEIGHT - REVEAL_START_Y) * lift_ease;
            // 2. Grow to hero size once it clears the crate, while the crate
            // drops away underneath it.
            let grow_t =
                ((t - REVEAL_GROW_START) / (REVEAL_GROW_END - REVEAL_GROW_START)).clamp(0.0, 1.0);
            let grow_ease = grow_t * grow_t * (3.0 - 2.0 * grow_t);
            let size_mult = 1.0 + (REVEAL_GROW_FACTOR - 1.0) * grow_ease;
            // Rotation is the player's to control while the card is up
            // (`handle_reveal_drag`); here we only lift and grow.
            if let Ok(mut reveal) = reveal_query.single_mut() {
                reveal.translation = CRATE_LOCAL + Vec3::new(0.0, y, 0.0);
                reveal.scale =
                    Vec3::splat((reveal_drag.model_scale * size_mult).max(0.001));
            }

            // 3. Card only after the gun is out, grown, and readable.
            if !crate_state.card_shown && t >= REVEAL_CARD_AT {
                crate_state.card_shown = true;
                if let Some(reward) = crate_state.reward.clone() {
                    spawn_reward_card(
                        &mut commands,
                        &reward,
                        crate_state.owned_before,
                        &reveal_ctx.registry,
                        &reveal_ctx.skin_registry,
                        &reveal_ctx.asset_server,
                    );
                }
            }
        }
        CratePhase::Browsing => {}
    }
}

/// Drag-to-inspect state for the revealed gun (mirrors the loadout preview).
#[derive(Resource)]
pub struct RevealDragState {
    pub dragging: bool,
    pub last_pos: Vec2,
    pub rotation_x: f32,
    pub rotation_y: f32,
    /// Extra base yaw so the gun's long axis lies across the screen.
    /// Auto-set once from the model's bounds: some models are built
    /// barrel-along-Z, others barrel-along-X.
    pub base_yaw_offset: f32,
    /// Extra base pitch for models built muzzle-up: lays them flat first.
    pub base_pitch_offset: f32,
    /// Root scale that fits the prize to `REVEAL_TARGET_SIZE` world units,
    /// so rifles and pistols present consistently regardless of authored
    /// scale.
    pub model_scale: f32,
    /// Meshes seen by the last alignment pass; the orientation/scale fit
    /// only locks in once this stops growing (the scene streams in).
    pub align_meshes: usize,
    pub align_stable: u32,
    pub measured: bool,
}

impl Default for RevealDragState {
    fn default() -> Self {
        Self {
            dragging: false,
            last_pos: Vec2::ZERO,
            rotation_x: 0.0,
            rotation_y: 0.0,
            base_yaw_offset: 0.0,
            base_pitch_offset: 0.0,
            model_scale: 1.0,
            align_meshes: 0,
            align_stable: 0,
            measured: false,
        }
    }
}

/// Base presentation yaw for the revealed model.
fn reveal_base_rotation() -> Quat {
    Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)
}

/// Once the revealed model's meshes stream in, make sure it shows broadside
/// at a consistent size: if its depth (toward the camera) clearly exceeds
/// its width, it is pointing at the viewer, so add a quarter turn; upright
/// (muzzle-up) models are laid flat first. The root scale is fitted so the
/// longest dimension lands on `REVEAL_TARGET_SIZE`.
pub fn align_reveal_model(
    mut drag: ResMut<RevealDragState>,
    reveal_roots: Query<(Entity, &GlobalTransform, &Transform), With<CrateRevealModel>>,
    children_query: Query<&Children>,
    mesh_query: Query<(&GlobalTransform, &Mesh3d)>,
    mesh_assets: Res<Assets<Mesh>>,
) {
    if drag.measured {
        return;
    }
    let Some((root_entity, _, root_local)) = reveal_roots.iter().next() else {
        return;
    };
    // Walk only the reveal subtree: the lobby around it is full of unrelated
    // meshes that would pollute the bounds.
    let mut stack = vec![root_entity];
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    let mut count = 0usize;
    while let Some(entity) = stack.pop() {
        if let Ok((global, mesh3d)) = mesh_query.get(entity) {
            if let Some(mesh) = mesh_assets.get(&mesh3d.0) {
                if let Some(aabb) = mesh.compute_aabb() {
                    count += 1;
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
            }
        }
        if let Ok(kids) = children_query.get(entity) {
            for child in kids.iter() {
                stack.push(child);
            }
        }
    }
    if count == 0 || !min.is_finite() || !max.is_finite() {
        return;
    }
    // The scene streams in: only lock the fit once the mesh set stops
    // growing, otherwise a half-loaded receiver would freeze the scale and
    // orientation wrong.
    if count != drag.align_meshes {
        drag.align_meshes = count;
        drag.align_stable = 0;
        return;
    }
    drag.align_stable += 1;
    if drag.align_stable < 10 {
        return;
    }
    let size = max - min;
    if size.y > size.x.max(size.z) * 1.2 {
        // Built muzzle-up: lay it flat (muzzle forward) before the yaw puts
        // it broadside.
        drag.base_pitch_offset = std::f32::consts::FRAC_PI_2;
    } else if size.z > size.x * 1.2 {
        drag.base_yaw_offset = std::f32::consts::FRAC_PI_2;
    }
    // Normalize by the root's current scale (the pop animation may be
    // mid-flight) so the fit lands on the target size exactly.
    let longest = size.x.max(size.y).max(size.z).max(0.01);
    let current = root_local.scale.x.max(0.01);
    drag.model_scale = (REVEAL_TARGET_SIZE / (longest / current)).clamp(0.05, 5.0);
    drag.measured = true;
}

/// Let the player spin the revealed gun around like the loadout preview:
/// drag to yaw/pitch, release to ease back to the presentation angle.
pub fn handle_reveal_drag(
    time: Res<Time>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    mut drag: ResMut<RevealDragState>,
    crate_state: Res<CrateState>,
    mut reveal_query: Query<&mut Transform, With<CrateRevealModel>>,
) {
    if crate_state.phase != CratePhase::Reveal {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };

    if mouse_input.just_pressed(MouseButton::Left) {
        if let Some(pos) = window.cursor_position() {
            drag.dragging = true;
            drag.last_pos = pos;
        }
    }
    if mouse_input.just_released(MouseButton::Left) {
        drag.dragging = false;
    }

    if drag.dragging {
        if let Some(pos) = window.cursor_position() {
            let delta = pos - drag.last_pos;
            drag.rotation_y += delta.x * 0.012;
            drag.rotation_x = (drag.rotation_x + delta.y * 0.012).clamp(-1.2, 1.2);
            drag.last_pos = pos;
        }
    } else {
        let speed = 3.0 * time.delta_secs();
        drag.rotation_y += (0.0 - drag.rotation_y) * speed;
        drag.rotation_x += (0.0 - drag.rotation_x) * speed;
        if drag.rotation_y.abs() < 0.001 {
            drag.rotation_y = 0.0;
        }
        if drag.rotation_x.abs() < 0.001 {
            drag.rotation_x = 0.0;
        }
    }

    let rotation = reveal_base_rotation()
        * Quat::from_rotation_y(drag.base_yaw_offset + drag.rotation_y)
        * Quat::from_rotation_x(drag.base_pitch_offset + drag.rotation_x);
    for mut transform in reveal_query.iter_mut() {
        transform.rotation = rotation;
    }
}

fn reward_rarity(reward: &CrateReward, skins: &SkinRegistry) -> Rarity {
    match reward {
        CrateReward::GunSkin { instance, .. } => skins.rarity_of(&instance.skin),
        CrateReward::MeleeUnlock { .. } => Rarity::Rare,
    }
}

fn spawn_reveal_model(
    commands: &mut Commands,
    reward: &CrateReward,
    registry: &WeaponRegistry,
    asset_server: &AssetServer,
    attachment_registry: &AttachmentRegistry,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    camera: &Query<Entity, With<Menu3dCamera>>,
) {
    let weapon_id = reward.weapon_id();
    let Some(config) = registry.weapons.get(weapon_id) else {
        return;
    };
    let model_file = config.meta.model_path.split('#').next().unwrap_or("");
    if model_file.is_empty() {
        return;
    }

    let skin = match reward {
        CrateReward::GunSkin { instance, .. } => instance.clone(),
        CrateReward::MeleeUnlock { .. } => SkinInstance::default(),
    };

    // Root carries the reveal transform (lift + grow + drag rotation); the
    // model child keeps its own authored scale and gets the skin tag applied,
    // plus the gun's default parts — never just the bare receiver. Parented
    // to the menu camera like the crate. Spawns down inside the crate mouth;
    // the animation lifts it out.
    let mut model_entity = None;
    let root = commands
        .spawn((
            Transform::from_translation(CRATE_LOCAL + Vec3::new(0.0, REVEAL_START_Y, 0.0))
                .with_rotation(reveal_base_rotation()),
            Visibility::default(),
            CrateRevealModel,
        ))
        .with_children(|root| {
            model_entity = Some(
                root.spawn((
                    WorldAssetRoot(asset_server.load(&config.meta.model_path)),
                    Transform::IDENTITY,
                    crate::weapons::WeaponSkinTag::new(skin),
                ))
                .id(),
            );
        })
        .id();

    if let Some(model) = model_entity {
        let overrides = resolve_attachment_meshes(
            weapon_id,
            config,
            &PlayerAttachments::default(),
            attachment_registry,
        );
        spawn_weapon_attachments(
            commands,
            asset_server,
            model,
            config,
            meshes,
            materials,
            &overrides,
            false,
        );
    }

    if let Ok(cam) = camera.single() {
        commands.entity(cam).add_child(root);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_reward_card(
    commands: &mut Commands,
    reward: &CrateReward,
    owned_before: u32,
    registry: &WeaponRegistry,
    skin_registry: &SkinRegistry,
    asset_server: &AssetServer,
) {
    let weapon_id = reward.weapon_id();
    let weapon_config = registry.weapons.get(weapon_id);
    let weapon_display = weapon_config
        .map(|c| c.info.name.clone())
        .unwrap_or_else(|| weapon_id.replace('_', " "));
    let weapon_type = weapon_config
        .map(|c| c.meta.weapon_type.clone())
        .unwrap_or_else(|| "Primary".to_string());
    let slot = slot_from_weapon_type(&weapon_type);

    let rarity = reward_rarity(reward, skin_registry);

    let title = match reward {
        CrateReward::GunSkin { instance, .. } => skin_registry.display_name(&instance.skin),
        CrateReward::MeleeUnlock { .. } => weapon_display.clone(),
    };
    let subtitle = match reward {
        CrateReward::GunSkin { instance, .. } => format!(
            "{} Skin - {} ({:.2})",
            rarity.display_name(),
            instance.wear_name(),
            instance.wear
        ),
        CrateReward::MeleeUnlock { .. } => "Melee mesh unlocked".to_string(),
    };
    let swatch = match reward {
        CrateReward::GunSkin { instance, .. } => skin_registry.swatch_for_instance(instance),
        CrateReward::MeleeUnlock { .. } => rarity.color(),
    };
    // Patterned gun skins also show their motif slice on the card.
    let pattern_thumb = match reward {
        CrateReward::GunSkin { instance, .. } => skin_registry.get(&instance.skin).and_then(|def| {
            def.finish.pattern.as_ref().map(|p| {
                (
                    asset_server.load(&p.sheet),
                    def.finish.pattern_crop_origin(instance.seed),
                    def.finish.base_color,
                )
            })
        }),
        CrateReward::MeleeUnlock { .. } => None,
    };
    let slot_label = match slot {
        WeaponSlot::Primary | WeaponSlot::Secondary => "GUN",
        WeaponSlot::Melee => "MELEE",
        WeaponSlot::Equipment => "EQUIP",
    };
    let pattern_line = match reward {
        CrateReward::GunSkin { instance, .. } => {
            let patterned = skin_registry
                .get(&instance.skin)
                .is_some_and(|d| d.finish.has_pattern());
            patterned.then(|| match instance.tint {
                Some([r, g, b]) => format!(
                    "pattern #{} {}",
                    instance.seed % 1000,
                    color_hex_compact([r, g, b, 1.0])
                ),
                None => format!("pattern #{}", instance.seed % 1000),
            })
        }
        _ => None,
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                // Dock the card low so the revealed gun owns the
                // middle of the screen.
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                padding: UiRect::bottom(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.35)),
            CrateResultPanel,
            ZIndex(30),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(340.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(18.0)),
                        row_gap: Val::Px(8.0),
                        border: UiRect::all(Val::Px(3.0)),
                        border_radius: RADIUS,
                        ..default()
                    },
                    BackgroundColor(BG_PANEL),
                    BorderColor::from(rarity.color()),
                ))
                .with_children(|card| {
                    card.spawn((
                        Text::new(rarity.display_name().to_uppercase()),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(rarity.color()),
                    ));

                    card.spawn((
                        Node {
                            width: Val::Px(150.0),
                            height: Val::Px(80.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(3.0)),
                            border_radius: RADIUS,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(4.0),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(swatch),
                        BorderColor::from(rarity.color()),
                    ))
                    .with_children(|model| {
                        // Patterned skins show their motif slice, matching
                        // the gun and the reel cells.
                        if let Some((sheet, crop, base)) = pattern_thumb.as_ref() {
                            let size = PATTERN_SHEET_PX * PATTERN_STAMP;
                            model.spawn((
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
                                        crop[0] * PATTERN_SHEET_PX,
                                        crop[1] * PATTERN_SHEET_PX,
                                        size,
                                        size,
                                    )),
                                    color: Color::srgb(base[0], base[1], base[2]),
                                    ..ImageNode::new(sheet.clone())
                                },
                            ));
                        }
                        model.spawn((
                            Text::new(slot_label),
                            TextFont {
                                font_size: FontSize::Px(34.0),
                                ..default()
                            },
                            TextColor(TEXT),
                        ));
                        model.spawn((
                            Text::new(&weapon_display),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(TEXT),
                        ));
                    });

                    card.spawn((
                        Text::new(title),
                        TextFont {
                            font_size: FontSize::Px(24.0),
                            ..default()
                        },
                        TextColor(TEXT),
                    ));

                    card.spawn((
                        Text::new(subtitle),
                        TextFont {
                            font_size: FontSize::Px(13.0),
                            ..default()
                        },
                        TextColor(rarity.color()),
                    ));

                    if let Some(line) = pattern_line {
                        card.spawn((
                            Text::new(line),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(ACCENT),
                        ));
                    }

                    card.spawn((
                        Text::new(format!("for {}", weapon_display)),
                        TextFont {
                            font_size: FontSize::Px(15.0),
                            ..default()
                        },
                        TextColor(TEXT_MUTED),
                    ));

                    if owned_before > 0 {
                        card.spawn((
                            Text::new(format!(
                                "Duplicate — you now have {}",
                                owned_before + 1
                            )),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(TEXT_FAINT),
                        ));
                    }

                    card.spawn((
                        Button,
                        Node {
                            width: Val::Px(160.0),
                            height: Val::Px(40.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            margin: UiRect::top(Val::Px(8.0)),
                            border_radius: RADIUS,
                            ..default()
                        },
                        BackgroundColor(ACCENT),
                        CrateResultDismiss,
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new("CONTINUE"),
                            TextFont {
                                font_size: FontSize::Px(14.0),
                                ..default()
                            },
                            TextColor(TEXT),
                        ));
                    });
                });
        });
}
