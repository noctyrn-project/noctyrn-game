use bevy::prelude::*;
use bevy::dev_tools::diagnostics_overlay::*;
use bevy::diagnostic::{DiagnosticPath, Diagnostics, Diagnostic, FrameTimeDiagnosticsPlugin, EntityCountDiagnosticsPlugin, RegisterDiagnostic};

use crate::settings::GameSettings;
use crate::player::Velocity;
use crate::gameplay::PlayerBody;
use crate::menu::main_menu::Menu3dCamera;

pub const ENTITY_COUNT: DiagnosticPath = DiagnosticPath::const_new("noctyrn/entities");
pub const MESH_COUNT: DiagnosticPath = DiagnosticPath::const_new("noctyrn/meshes");
pub const PING: DiagnosticPath = DiagnosticPath::const_new("noctyrn/ping");
pub const SPEED: DiagnosticPath = DiagnosticPath::const_new("noctyrn/speed");

/// Marker for custom debug text entities spawned alongside the overlay.
#[derive(Component)]
struct DebugOverlayText;

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(EntityCountDiagnosticsPlugin::default())
            .register_diagnostic(Diagnostic::new(ENTITY_COUNT))
            .register_diagnostic(Diagnostic::new(MESH_COUNT))
            .register_diagnostic(Diagnostic::new(PING))
            .register_diagnostic(Diagnostic::new(SPEED))
            .add_systems(Update, (spawn_diagnostics, update_debug_text));
    }
}

fn spawn_diagnostics(
    mut commands: Commands,
    settings: Res<GameSettings>,
    overlay: Query<Entity, With<DiagnosticsOverlay>>,
    debug_text: Query<Entity, With<DebugOverlayText>>,
) {
    let dm = settings.debug.debug_mode;
    let need_overlay = dm && settings.debug.show_fps;

    // Spawn or despawn the main diagnostics overlay (only toggled by the
    // master debug_mode + show_fps flags — individual sub-toggles are handled
    // by the custom debug text system instead, to avoid respawn conflicts).
    if need_overlay && overlay.is_empty() {
        let mut items = vec![
            DiagnosticsOverlayItem { path: FrameTimeDiagnosticsPlugin::FPS, statistic: DiagnosticsOverlayStatistic::Smoothed, precision: 1 },
            DiagnosticsOverlayItem { path: EntityCountDiagnosticsPlugin::ENTITY_COUNT, statistic: DiagnosticsOverlayStatistic::Value, precision: 0 },
            DiagnosticsOverlayItem { path: ENTITY_COUNT, statistic: DiagnosticsOverlayStatistic::Value, precision: 0 },
            DiagnosticsOverlayItem { path: MESH_COUNT, statistic: DiagnosticsOverlayStatistic::Value, precision: 0 },
        ];
        if settings.debug.show_ping {
            items.push(DiagnosticsOverlayItem { path: PING, statistic: DiagnosticsOverlayStatistic::Value, precision: 1 });
        }
        if settings.debug.show_speed {
            items.push(DiagnosticsOverlayItem { path: SPEED, statistic: DiagnosticsOverlayStatistic::Value, precision: 1 });
        }
        commands.spawn(DiagnosticsOverlay::new("Debug", items));
    } else if !need_overlay && !overlay.is_empty() {
        for entity in &overlay { commands.entity(entity).despawn(); }
    }

    // Toggle custom debug text (coords / rotation)
    let need_coords = dm && settings.debug.show_coords;
    let need_rot = dm && settings.debug.show_rotation;
    if need_coords || need_rot {
        if debug_text.is_empty() {
            commands.spawn((
                Text::new(""),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(Color::srgb(0.0, 1.0, 0.0)),
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(120.0),
                    left: Val::Px(4.0),
                    ..default()
                },
                DebugOverlayText,
            ));
        }
    } else if !debug_text.is_empty() {
        for entity in &debug_text { commands.entity(entity).despawn(); }
    }
}

/// Update custom debug text (position + rotation) each frame.
fn update_debug_text(
    settings: Res<GameSettings>,
    mut text_query: Query<&mut Text, With<DebugOverlayText>>,
    menu_cam: Query<&Transform, With<Menu3dCamera>>,
    game_cam: Query<&Transform, With<crate::player::MainCamera>>,
) {
    let Ok(mut text) = text_query.single_mut() else { return };
    let Ok(cam) = menu_cam.single().or_else(|_| game_cam.single()) else { return };

    let mut lines = String::new();
    if settings.debug.show_coords {
        let p = cam.translation;
        lines.push_str(&format!("POS: {:.1} {:.1} {:.1}\n", p.x, p.y, p.z));
    }
    if settings.debug.show_rotation {
        let (yaw, pitch, roll) = cam.rotation.to_euler(EulerRot::YXZ);
        lines.push_str(&format!("ROT: {:.1}° {:.1}° {:.1}°",
            yaw.to_degrees(), pitch.to_degrees(), roll.to_degrees()));
    }
    text.0 = lines;
}
