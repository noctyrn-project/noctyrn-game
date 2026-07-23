use bevy::prelude::*;
use bevy::dev_tools::diagnostics_overlay::*;
use bevy::diagnostic::{DiagnosticPath, Diagnostics, Diagnostic, FrameTimeDiagnosticsPlugin, EntityCountDiagnosticsPlugin, RegisterDiagnostic};

use crate::settings::GameSettings;
use crate::player::Velocity;
use crate::gameplay::PlayerBody;

pub const ENTITY_COUNT: DiagnosticPath = DiagnosticPath::const_new("noctyrn/entities");
pub const MESH_COUNT: DiagnosticPath = DiagnosticPath::const_new("noctyrn/meshes");
pub const PING: DiagnosticPath = DiagnosticPath::const_new("noctyrn/ping");
pub const SPEED: DiagnosticPath = DiagnosticPath::const_new("noctyrn/speed");

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(EntityCountDiagnosticsPlugin::default())
            .register_diagnostic(Diagnostic::new(ENTITY_COUNT))
            .register_diagnostic(Diagnostic::new(MESH_COUNT))
            .register_diagnostic(Diagnostic::new(PING))
            .register_diagnostic(Diagnostic::new(SPEED))
            .add_systems(Update, (spawn_diagnostics, update_game_diagnostics));
    }
}

fn spawn_diagnostics(
    mut commands: Commands,
    settings: Res<GameSettings>,
    overlay: Query<Entity, With<DiagnosticsOverlay>>,
) {
    let dm = settings.debug.debug_mode;
    if (dm && settings.debug.show_fps) && overlay.is_empty() {
        commands.spawn(DiagnosticsOverlay::new("Debug", vec![
            DiagnosticsOverlayItem { path: FrameTimeDiagnosticsPlugin::FPS, statistic: DiagnosticsOverlayStatistic::Smoothed, precision: 1 },
            DiagnosticsOverlayItem { path: EntityCountDiagnosticsPlugin::ENTITY_COUNT, statistic: DiagnosticsOverlayStatistic::Value, precision: 0 },
            DiagnosticsOverlayItem { path: ENTITY_COUNT, statistic: DiagnosticsOverlayStatistic::Value, precision: 0 },
            DiagnosticsOverlayItem { path: MESH_COUNT, statistic: DiagnosticsOverlayStatistic::Value, precision: 0 },
            DiagnosticsOverlayItem { path: PING, statistic: DiagnosticsOverlayStatistic::Value, precision: 1 },
            DiagnosticsOverlayItem { path: SPEED, statistic: DiagnosticsOverlayStatistic::Value, precision: 1 },
        ]));
    } else if !dm && !overlay.is_empty() {
        for entity in &overlay {
            commands.entity(entity).despawn();
        }
    }
}

fn update_game_diagnostics(
    mut diagnostics: Diagnostics,
    meshes: Res<Assets<Mesh>>,
    velocity_query: Query<&Velocity, With<PlayerBody>>,
) {
    diagnostics.add_measurement(&MESH_COUNT, || meshes.iter().count() as f64);
    diagnostics.add_measurement(&PING, || 0.0);
    let speed = velocity_query.single().map(|v| {
        Vec3::new(v.x, 0.0, v.z).length() as f64
    }).unwrap_or(0.0);
    diagnostics.add_measurement(&SPEED, || speed);
}
