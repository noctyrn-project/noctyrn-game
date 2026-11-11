//! Debug skin texture viewer: an in-game panel (Settings → Options → Skin
//! Texture Viewer, with Debug Mode on) that applies any skin to the player's
//! current guns so textures can be checked live on the model.
//!
//! The viewer snapshots the live skins when it opens and restores them when
//! it closes; it never touches the saved loadout.

use bevy::prelude::*;

use crate::player::GameState;
use crate::settings::GameSettings;
use crate::theme::*;
use crate::weapons::{SkinId, SkinInstance, SkinRegistry, WeaponSkinTag};

pub struct DebugTexturesPlugin;

impl Plugin for DebugTexturesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewerState>().add_systems(
            Update,
            (manage_viewer_panel, viewer_buttons, apply_viewer_skin).chain(),
        );
    }
}

#[derive(Resource, Default)]
struct ViewerState {
    index: usize,
    applied: Option<SkinId>,
    snapshot: Vec<(Entity, SkinInstance)>,
}

#[derive(Component)]
struct ViewerPanel;
#[derive(Component)]
struct ViewerName;
#[derive(Component)]
struct ViewerInfo;
#[derive(Component)]
struct ViewerThumb;
#[derive(Component)]
struct ViewerPrev;
#[derive(Component)]
struct ViewerNext;

/// Sorted skin ids (rarity, then name), excluding the built-in default.
fn viewer_skins(registry: &SkinRegistry) -> Vec<SkinId> {
    let mut ids: Vec<SkinId> = registry
        .skins
        .values()
        .filter(|d| !d.id.is_default())
        .map(|d| d.id.clone())
        .collect();
    ids.sort_by(|a, b| {
        registry
            .rarity_of(a)
            .cmp(&registry.rarity_of(b))
            .then_with(|| {
                let na = registry.get(a).map(|d| &d.name);
                let nb = registry.get(b).map(|d| &d.name);
                na.cmp(&nb)
            })
    });
    ids
}

/// One-line finish summary for the panel.
fn finish_summary(registry: &SkinRegistry, id: &SkinId) -> String {
    let Some(def) = registry.get(id) else {
        return "unknown skin".to_string();
    };
    let f = &def.finish;
    let mut parts: Vec<String> = Vec::new();
    if let Some(p) = &f.pattern {
        let sheet = p.sheet.rsplit('/').next().unwrap_or(&p.sheet);
        parts.push(format!(
            "pattern {}{}",
            sheet,
            if p.tintable { " tintable" } else { "" }
        ));
    }
    if let Some(g) = &f.galaxy {
        parts.push(format!("galaxy d{:.1} s{:.2}", g.density, g.speed));
    }
    if let Some(n) = &f.nebula {
        parts.push(format!("nebula sc{:.1} sp{:.2}", n.scale, n.speed));
    }
    if let Some(m) = &f.matrix {
        parts.push(format!("matrix d{:.0} s{:.1}", m.density, m.speed));
    }
    if let Some(g) = &f.gradient {
        parts.push(format!("gradient {}", g.axis));
    }
    if let Some(e) = &f.emissive {
        parts.push(format!("glow x{:.1} {}hz", e.strength, e.pulse_hz));
    }
    if let Some(t) = &f.texture {
        parts.push(format!("tex {t}"));
    }
    if parts.is_empty() {
        parts.push("solid".to_string());
    }
    parts.join(" | ")
}

fn viewer_active(settings: &GameSettings, state: &State<GameState>) -> bool {
    settings.debug.debug_mode
        && settings.debug.texture_viewer
        && matches!(
            state.get(),
            GameState::Playing | GameState::Paused
        )
}

/// Spawn the panel when enabled, tear it down (and restore skins) when not.
fn manage_viewer_panel(
    mut commands: Commands,
    settings: Res<GameSettings>,
    state: Res<State<GameState>>,
    panel: Query<Entity, With<ViewerPanel>>,
    mut viewer: ResMut<ViewerState>,
    mut tags: Query<(Entity, &mut WeaponSkinTag)>,
) {
    if viewer_active(&settings, &state) {
        if panel.is_empty() {
            viewer.index = 0;
            viewer.applied = None;
            spawn_panel(&mut commands);
        }
    } else if !panel.is_empty() {
        for entity in panel.iter() {
            commands.entity(entity).despawn();
        }
        // Restore whatever the guns wore before the viewer took over.
        for (entity, skin) in viewer.snapshot.drain(..) {
            if let Ok((_, mut tag)) = tags.get_mut(entity) {
                tag.skin = skin;
                tag.applied = false;
            }
        }
        viewer.applied = None;
    }
}

fn spawn_panel(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(12.0),
                top: Val::Percent(22.0),
                width: Val::Px(300.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(8.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: RADIUS,
                ..default()
            },
            BorderColor::all(BORDER),
            BackgroundColor(BG_PANEL),
            ViewerPanel,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Texture Viewer [debug]"),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(TEXT),
                ViewerName,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: FontSize::Px(12.0),
                    ..default()
                },
                TextColor(TEXT_MUTED),
                ViewerInfo,
            ));
            // Pattern sheet thumbnail (full sheet, so the texture itself
            // can be inspected, not just the gun crop).
            panel.spawn((
                Node {
                    width: Val::Px(276.0),
                    height: Val::Px(138.0),
                    border_radius: RADIUS_SM,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(BG_ELEVATED),
                ViewerThumb,
            ));
            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: RADIUS,
                            ..default()
                        },
                        BorderColor::all(BORDER),
                        BackgroundColor(BG_ELEVATED),
                        ViewerPrev,
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("< Prev"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(TEXT_MUTED),
                        ));
                    });
                    row.spawn((
                        Button,
                        Node {
                            width: Val::Px(120.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: RADIUS,
                            ..default()
                        },
                        BorderColor::all(BORDER),
                        BackgroundColor(BG_ELEVATED),
                        ViewerNext,
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new("Next >"),
                            TextFont {
                                font_size: FontSize::Px(13.0),
                                ..default()
                            },
                            TextColor(TEXT_MUTED),
                        ));
                    });
                });
        });
}

/// Prev/Next paging (wraps around the skin list).
fn viewer_buttons(
    prev: Query<&Interaction, (Changed<Interaction>, With<ViewerPrev>)>,
    next: Query<&Interaction, (Changed<Interaction>, With<ViewerNext>)>,
    mouse: Res<ButtonInput<MouseButton>>,
    registry: Res<SkinRegistry>,
    mut viewer: ResMut<ViewerState>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let skins = viewer_skins(&registry);
    if skins.is_empty() {
        return;
    }
    for interaction in prev.iter() {
        if *interaction == Interaction::Pressed {
            viewer.index = viewer.index.checked_sub(1).unwrap_or(skins.len() - 1);
        }
    }
    for interaction in next.iter() {
        if *interaction == Interaction::Pressed {
            viewer.index = (viewer.index + 1) % skins.len();
        }
    }
}

/// Snapshot unseen guns, apply the selected skin to everything live, and
/// refresh the panel labels + thumbnail.
fn apply_viewer_skin(
    mut commands: Commands,
    settings: Res<GameSettings>,
    state: Res<State<GameState>>,
    registry: Res<SkinRegistry>,
    asset_server: Res<AssetServer>,
    mut viewer: ResMut<ViewerState>,
    mut tags: Query<(Entity, &mut WeaponSkinTag)>,
    mut name_query: Query<&mut Text, (With<ViewerName>, Without<ViewerInfo>)>,
    mut info_query: Query<&mut Text, (With<ViewerInfo>, Without<ViewerName>)>,
    thumb_query: Query<Entity, With<ViewerThumb>>,
    panel_query: Query<Entity, With<ViewerPanel>>,
) {
    if !viewer_active(&settings, &state) || panel_query.is_empty() {
        return;
    }
    let skins = viewer_skins(&registry);
    if skins.is_empty() {
        return;
    }
    viewer.index = viewer.index.min(skins.len() - 1);
    let id = skins[viewer.index].clone();

    // Snapshot any gun we haven't seen so the close path can restore it.
    for (entity, tag) in tags.iter() {
        if !viewer.snapshot.iter().any(|(e, _)| *e == entity) {
            viewer.snapshot.push((entity, tag.skin.clone()));
        }
    }

    if viewer.applied.as_ref() != Some(&id) {
        viewer.applied = Some(id.clone());
        let instance = SkinInstance::factory(id.clone())
            .with_seed(viewer.index as u32 * 7919 + 13);
        for (_, mut tag) in tags.iter_mut() {
            tag.skin = instance.clone();
            tag.applied = false;
        }
    }

    let def = registry.def_or_default(&id);
    if let Ok(mut text) = name_query.single_mut() {
        **text = format!("{} [{}]", def.name, def.rarity.display_name());
    }
    if let Ok(mut text) = info_query.single_mut() {
        **text = finish_summary(&registry, &id);
    }
    // Thumbnail: full pattern sheet when the skin has one, else the
    // representative swatch colour.
    if let Ok(thumb) = thumb_query.single() {
        commands.entity(thumb).despawn_children();
        if let Some(pattern) = &def.finish.pattern {
            commands.entity(thumb).with_children(|t| {
                t.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    ImageNode::new(asset_server.load(&pattern.sheet)),
                ));
            });
        } else {
            commands
                .entity(thumb)
                .insert(BackgroundColor(registry.swatch_color(&id)));
        }
    }
}
