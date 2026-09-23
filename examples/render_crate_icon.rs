//! Renders the crate model to `assets/ui/crate_icon.png` for the crate menu
//! cards.
//!
//! ```text
//! cargo run --example render_crate_icon
//! ```
//!
//! Re-run it whenever `assets/ui/crate/{body,lid,latches}.glb` changes. The
//! window screenshot API discards alpha, so the crate is rendered on the UI
//! panel colour instead of a transparent background.

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::window::WindowResolution;

const OUT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ui/crate_icon.png"
);
const SIZE: u32 = 512;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/assets").to_string(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "render_crate_icon".into(),
                        resolution: WindowResolution::new(SIZE, SIZE),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, (lighten_crate_materials, capture))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(AmbientLight {
        color: Color::srgb(0.6, 0.65, 0.8),
        brightness: 300.0,
        ..default()
    });

    // Match the crate menu panel so the icon blends into the card.
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.08, 0.09, 0.12)),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 28f32.to_radians(),
            ..default()
        }),
        Transform::from_xyz(4.8, 3.6, 5.0).looking_at(Vec3::new(0.0, 0.85, 0.0), Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.95, 0.93, 1.0),
            illuminance: 8_000.0,
            ..default()
        },
        Transform::from_xyz(4.0, 6.0, 4.0).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y),
    ));
    commands.spawn((
        PointLight {
            color: Color::srgb(0.45, 0.55, 0.9),
            intensity: 40_000.0,
            range: 20.0,
            ..default()
        },
        Transform::from_xyz(-3.0, 2.0, -2.5),
    ));

    commands
        .spawn((CrateRoot, Transform::IDENTITY))
        .with_children(|root| {
            // The split parts are baked with their vertices rebased: the lid
            // around its hinge, the latches around their centre.
            root.spawn((
                WorldAssetRoot(asset_server.load("ui/crate/body.glb#Scene0")),
                Transform::IDENTITY,
            ));
            root.spawn((
                WorldAssetRoot(asset_server.load("ui/crate/lid.glb#Scene0")),
                Transform::from_translation(Vec3::new(-1.0, 1.522, 0.0)),
            ));
            root.spawn((
                WorldAssetRoot(asset_server.load("ui/crate/latches.glb#Scene0")),
                Transform::from_translation(Vec3::new(1.0485, 1.466, 0.0)),
            ));
        });
}

#[derive(Component)]
struct CrateRoot;

/// The crate's authored materials are near-black; give the icon a lighter
/// gray so it reads against the dark card.
fn lighten_crate_materials(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    roots: Query<Entity, With<CrateRoot>>,
    children: Query<&Children>,
    meshes: Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
    mut applied: Local<bool>,
) {
    if *applied {
        return;
    }
    let Some(root) = roots.iter().next() else {
        return;
    };
    let mut found = false;
    let mut stack = vec![root];
    while let Some(entity) = stack.pop() {
        if meshes.contains(entity) {
            found = true;
            commands.entity(entity).insert(MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.46, 0.47, 0.5),
                metallic: 0.0,
                perceptual_roughness: 0.55,
                ..default()
            })));
        }
        if let Ok(kids) = children.get(entity) {
            for child in kids.iter() {
                stack.push(child);
            }
        }
    }
    if found {
        info!("lightened crate icon materials");
    }
    *applied = found;
}

fn capture(
    mut frames: Local<u32>,
    mut commands: Commands,
    mut exit: MessageWriter<AppExit>,
) {
    *frames += 1;
    // Give the GLB a moment to load and the pipeline to warm up.
    if *frames == 30 {
        info!("Capturing crate icon to {OUT_PATH}");
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(OUT_PATH));
    }
    if *frames == 90 {
        exit.write(AppExit::Success);
    }
}
