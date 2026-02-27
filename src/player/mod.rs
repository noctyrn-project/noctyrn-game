use bevy::prelude::*;
use bevy::window::CursorOptions;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::app::AppExit;
use bevy::ecs::relationship::Relationship;
use bevy::camera::visibility::RenderLayers;
use crate::weapons::{WeaponSlot, spawn_weapon_visual_skinned, WeaponRegistry, PlayerLoadout, WeaponConfig, sync_loadout_to_configs, slot_from_weapon_type};
use crate::gameplay::{Health, PlayerBody, Regenerating, TurretProjectile, NeedsTeamSpawn};
use crate::ui_config::UiConfig;
use crate::gameplay::DeathEvent;
use crate::ui_settings::{spawn_settings_menu, update_settings_menu, handle_settings_interaction, handle_slider_drag, SettingsState};
use crate::settings::GameSettings;
use crate::menu::{MenuPlugin};
use rand::Rng;

mod movement;
mod input;
mod camera;
mod inventory;
pub mod shooting;

use movement::{
    CrouchHeight,
    GroundedState, MovementState, JumpState, SlideState, MovementConfig, MovementSet,
    detect_ground, transition_movement_state, handle_jump, apply_acceleration,
    apply_slide_physics, apply_friction, apply_gravity, integrate_velocity,
    resolve_collisions, interpolate_rendered_transform,
};
use input::{AccumulatedInput, accumulate_input, clear_input, load_keybinds, PlayerToggleState};
pub use input::{Keybinds, save_keybinds};
use camera::{CameraSensitivity, rotate_camera, translate_camera, free_cam_movement, update_fov, CameraSway, apply_camera_sway, apply_camera_shake, apply_lean};
use inventory::{Inventory, WeaponModel, handle_weapon_switching, SwitchState};
use shooting::{fire_weapon, move_projectiles, handle_weapon_recoil, handle_muzzle_flash, handle_melee_swing, handle_grenade_throw, update_ammo_ui, reload_weapon, handle_weapon_sway, AmmoStatus, AmmoUi, CameraRecoil, handle_camera_recoil, Projectile, MuzzleFlash, Grenade, ExplosionParticle};

pub use movement::{Velocity, PhysicalTranslation, PreviousPhysicalTranslation};

/// Tag component for the main (world) camera. Used to distinguish from weapon camera.
#[derive(Component)]
pub struct MainCamera;

/// Marker for entities that should render on the weapon layer (layer 1).
/// Propagated automatically to children of WeaponModel entities.
#[derive(Component)]
pub struct WeaponLayerEntity;

#[derive(Resource, Default)]
pub struct DebugSettings {
    pub show_hitboxes: bool,
    pub show_directions: bool,
    pub free_cam: bool,
}

/// Resource to track whether the in-game weapon terminal overlay is open.
#[derive(Resource, Default)]
pub struct WeaponTerminalOpen(pub bool);

/// Resource to track whether the pause menu overlay is visible.
/// When true, the overlay is shown but the game keeps running underneath.
#[derive(Resource, Default)]
pub struct PauseMenuOpen(pub bool);

/// Resource to track camera perspective mode (1st vs 3rd person).
#[derive(Resource)]
pub struct CameraMode {
    pub third_person: bool,
    pub distance: f32,
    pub height_offset: f32,
}

impl Default for CameraMode {
    fn default() -> Self {
        Self {
            third_person: false,
            distance: 4.0,
            height_offset: 1.5,
        }
    }
}

/// Tag component for the player's visible pill-shaped model.
#[derive(Component)]
pub struct PlayerModel;

#[derive(Component)]
pub struct WeaponTerminalUi;

#[derive(Component)]
pub struct WeaponTerminalItem {
    pub weapon_id: String,
    pub slot: WeaponSlot,
}

#[derive(Component)]
pub struct WeaponTerminalClose;

#[derive(Resource, Default)]
pub struct RemappingState {
    pub active_action: Option<String>,
}

#[derive(Component)]
pub struct KeybindingsUi;

#[derive(Component)]
pub struct DebugUi;

#[derive(Component)]
pub struct StatsUi;

#[derive(Component)]
pub struct StatsText;

#[derive(Component)]
pub struct RemapButton {
    pub action: String,
}

pub struct Player;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    LoadoutSelect,
    CrateOpening,
    GameModeSelect,
    Cosmetics,
    Playing,
    Paused,
}

impl Plugin for Player {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        app.add_plugins(MenuPlugin);
        app.init_resource::<DidFixedTimestepRunThisFrame>();
        app.init_state::<GameState>();
        app.init_resource::<DebugSettings>();
        app.init_resource::<RemappingState>();
        app.init_resource::<SettingsState>();
        app.init_resource::<WeaponTerminalOpen>();
        app.init_resource::<CameraMode>();
        app.init_resource::<PauseMenuOpen>();
        app.init_resource::<CameraSway>();
        
        app.add_systems(Startup, load_keybinds);
        app.add_systems(OnEnter(GameState::Playing), (spawn_player, spawn_crosshair, spawn_ammo_ui, spawn_kill_feed));
        app.add_systems(OnExit(GameState::Playing), (despawn_gameplay_ui, cleanup_pause_menu_on_exit));
        app.add_systems(Update, grab_cursor);
        app.add_systems(Update, (toggle_pause, update_stats_ui, debug_input, sync_settings, update_fov, toggle_camera_mode, animate_player_model).run_if(in_state(GameState::Playing)));
        app.add_systems(Update, (manage_pause_overlay, pause_menu_action, keybind_remapping_system, update_settings_menu, handle_settings_interaction, handle_slider_drag).run_if(in_state(GameState::Playing)));
        app.add_systems(Update, (keybind_remapping_system, update_settings_menu, handle_settings_interaction, handle_slider_drag).run_if(in_state(GameState::MainMenu)));

        app.add_systems(PreUpdate, clear_fixed_timestep_flag);
        app.add_systems(FixedPreUpdate, set_fixed_time_step_flag);
        // Configure movement pipeline ordering within FixedUpdate
        app.configure_sets(FixedUpdate, (
            MovementSet::GroundDetection,
            MovementSet::StateTransitions,
            MovementSet::Jump,
            MovementSet::Acceleration,
            MovementSet::Sliding,
            MovementSet::Friction,
            MovementSet::Gravity,
            MovementSet::Integration,
            MovementSet::Collision,
        ).chain());

        // Register each movement system in its ordered set
        app.add_systems(FixedUpdate, (
            detect_ground.in_set(MovementSet::GroundDetection),
            transition_movement_state.in_set(MovementSet::StateTransitions),
            handle_jump.in_set(MovementSet::Jump),
            apply_acceleration.in_set(MovementSet::Acceleration),
            apply_slide_physics.in_set(MovementSet::Sliding),
            apply_friction.in_set(MovementSet::Friction),
            apply_gravity.in_set(MovementSet::Gravity),
            integrate_velocity.in_set(MovementSet::Integration),
            resolve_collisions.in_set(MovementSet::Collision),
        ).run_if(in_state(GameState::Playing)));
        
        app.add_systems(Update, (
            handle_weapon_switching, 
            fire_weapon, 
            move_projectiles, 
            handle_weapon_recoil, 
            handle_muzzle_flash,
            handle_melee_swing,
            handle_grenade_throw,
        ).run_if(in_state(GameState::Playing)));

        app.add_systems(Update, handle_camera_recoil.run_if(in_state(GameState::Playing)));
        app.add_systems(Update, (apply_camera_sway, apply_camera_shake, apply_lean).run_if(in_state(GameState::Playing)));
        app.add_systems(Update, ensure_weapon_render_layers.run_if(in_state(GameState::Playing)));
        
        app.add_systems(Update, (
            shooting::handle_explosion_particles,
            handle_weapon_sway,
            update_ammo_ui,
            reload_weapon,
            draw_hitboxes,
            update_crosshair,
            update_kill_feed,
            update_hit_markers,
        ).run_if(in_state(GameState::Playing)));

        // Weapon terminal overlay systems
        app.add_systems(Update, (
            spawn_weapon_terminal_overlay,
            weapon_terminal_interaction,
            close_weapon_terminal,
        ).run_if(in_state(GameState::Playing)));

        app.add_systems(
            RunFixedMainLoop,
            (
                (
                    rotate_camera,
                    accumulate_input,
                )
                    .chain()
                    .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop)
                    .run_if(in_state(GameState::Playing)),
                (
                    clear_input.run_if(did_fixed_timestep_run_this_frame),
                    interpolate_rendered_transform,
                    translate_camera,
                    free_cam_movement,
                )
                    .chain()
                    .in_set(RunFixedMainLoopSystems::AfterFixedMainLoop)
                    .run_if(in_state(GameState::Playing)),
            ),
        );

        app.add_systems(Update, sync_settings.run_if(in_state(GameState::Playing)));
    }
}

/// A simple resource that tells us whether the fixed timestep ran this frame.
#[derive(Resource, Debug, Deref, DerefMut, Default)]
pub struct DidFixedTimestepRunThisFrame(bool);

fn clear_fixed_timestep_flag(mut did_fixed_timestep_run_this_frame: ResMut<DidFixedTimestepRunThisFrame>) {
    did_fixed_timestep_run_this_frame.0 = false;
}

fn set_fixed_time_step_flag(mut did_fixed_timestep_run_this_frame: ResMut<DidFixedTimestepRunThisFrame>) {
    did_fixed_timestep_run_this_frame.0 = true;
}

fn did_fixed_timestep_run_this_frame(did_fixed_timestep_run_this_frame: Res<DidFixedTimestepRunThisFrame>) -> bool {
    did_fixed_timestep_run_this_frame.0
}

fn spawn_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    weapon_registry: Res<WeaponRegistry>,
    loadout: Res<PlayerLoadout>,
    game_settings: Res<GameSettings>,
) {
    // Spawn Camera with settings-based FOV
    let camera_entity = commands.spawn((
        Camera3d::default(),
        MainCamera,
        CameraSensitivity::default(),
        CameraRecoil::default(),
        Transform::from_xyz(0.0, 0.0, 0.0), // Initial pos, will be updated by translate_camera
        Projection::Perspective(PerspectiveProjection {
            fov: game_settings.graphics.fov.to_radians(),
            ..default()
        }),
    )).id();

    // Weapon camera: renders only layer 1 (weapons) on top of the world.
    // Clears depth but not colour so weapons never clip into walls.
    let weapon_cam = commands.spawn((
        Camera3d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: game_settings.graphics.fov.to_radians(),
            near: 0.01,
            ..default()
        }),
        RenderLayers::layer(1),
        Transform::default(),
    )).id();
    commands.entity(camera_entity).add_child(weapon_cam);

    // Spawn initial weapon with loadout skin
    let skin = loadout.get_skin(WeaponSlot::Primary);
    let weapon_entity = spawn_weapon_visual_skinned(
        &mut commands,
        WeaponSlot::Primary,
        skin,
        &asset_server,
        &weapon_registry,
        &mut meshes,
        &mut materials,
    );

    // Initialize with equipping animation offset so the weapon rises into view
    let mut initial_recoil = crate::weapons::WeaponRecoil::default();
    initial_recoil.switch_offset = Vec3::new(0.0, -0.5, 0.0);
    initial_recoil.switch_rotation = Vec3::new(-1.0, 0.0, 0.0);
    commands.entity(weapon_entity).insert((WeaponModel, initial_recoil, WeaponLayerEntity, RenderLayers::layer(1)));
    commands.entity(camera_entity).add_child(weapon_entity);
    
    // Start in Equipping state so the weapon animates up and firing is blocked
    let mut initial_inventory = Inventory::default();
    initial_inventory.switch_state = SwitchState::Equipping;
    initial_inventory.switch_timer = Timer::from_seconds(0.4, TimerMode::Once);

    let initial_pos = Vec3::new(0.0, 2.0, 0.0);
    let player_entity = commands.spawn((
        Name::new("Player"),
        Transform::from_translation(initial_pos).with_scale(Vec3::splat(1.0)),
        Visibility::default(),
        AccumulatedInput::default(),
        PlayerToggleState::default(),
        Velocity::default(),
        PhysicalTranslation(initial_pos),
        PreviousPhysicalTranslation(initial_pos),
        CrouchHeight::default(),
        GroundedState::default(),
        MovementState::default(),
        JumpState::default(),
        SlideState::default(),
        movement::LeanState::default(),
    )).insert((
        MovementConfig::default(),
        initial_inventory,
        AmmoStatus::default(),
        Health { current: 100.0, max: 100.0 },
        Regenerating::default(),
        PlayerBody,
        NeedsTeamSpawn,
    )).id();

    // Spawn pill-shaped player model as a child (capsule mesh)
    let pill_mesh = meshes.add(Capsule3d::new(0.4, 1.8));
    let pill_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.3, 0.5, 0.7, 0.8),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    commands.entity(player_entity).with_children(|parent| {
        parent.spawn((
            Mesh3d(pill_mesh),
            MeshMaterial3d(pill_material),
            // Offset so capsule center aligns with body center
            Transform::from_xyz(0.0, 1.3, 0.0),
            PlayerModel,
            // Hidden in 1st person by default
            Visibility::Hidden,
        ));
    });
}

fn spawn_ammo_ui(mut commands: Commands, ui_config: Res<UiConfig>) {
    let config = &ui_config.ammo_ui;
    commands.spawn((
        Text::new("Ammo: -- / --"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::srgba(config.color[0], config.color[1], config.color[2], config.color[3])),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(config.position[0]),
            bottom: Val::Px(config.position[1]),
            ..default()
        },
        AmmoUi,
    ));
}

#[derive(Component)]
pub struct CrosshairTop;
#[derive(Component)]
pub struct CrosshairBottom;
#[derive(Component)]
pub struct CrosshairLeft;
#[derive(Component)]
pub struct CrosshairRight;

fn spawn_crosshair(mut commands: Commands, ui_config: Res<UiConfig>) {
    let config = &ui_config.crosshair;
    let color = Color::srgba(config.color[0], config.color[1], config.color[2], config.color[3]);
    
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(0.0),
            height: Val::Px(0.0),
            ..default()
        },
        GameplayUi,
        // Parent node for crosshair
    )).with_children(|parent| {
        // Dot
        if config.dot {
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(-config.dot_size / 2.0),
                    top: Val::Px(-config.dot_size / 2.0),
                    width: Val::Px(config.dot_size),
                    height: Val::Px(config.dot_size),
                    ..default()
                },
                BackgroundColor(color),
            ));
        }

        // Top
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-config.thickness / 2.0),
                bottom: Val::Px(config.gap),
                width: Val::Px(config.thickness),
                height: Val::Px(config.size),
                ..default()
            },
            BackgroundColor(color),
            CrosshairTop,
        ));

        // Bottom
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(-config.thickness / 2.0),
                top: Val::Px(config.gap),
                width: Val::Px(config.thickness),
                height: Val::Px(config.size),
                ..default()
            },
            BackgroundColor(color),
            CrosshairBottom,
        ));

        // Left
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(config.gap),
                top: Val::Px(-config.thickness / 2.0),
                width: Val::Px(config.size),
                height: Val::Px(config.thickness),
                ..default()
            },
            BackgroundColor(color),
            CrosshairLeft,
        ));

        // Right
        parent.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(config.gap),
                top: Val::Px(-config.thickness / 2.0),
                width: Val::Px(config.size),
                height: Val::Px(config.thickness),
                ..default()
            },
            BackgroundColor(color),
            CrosshairRight,
        ));
    });
}

fn update_crosshair(
    ui_config: Res<UiConfig>,
    ammo_status_query: Query<&AmmoStatus>,
    inventory_query: Query<&Inventory>,
    weapon_registry: Res<WeaponRegistry>,
    mut top_query: Query<&mut Node, (With<CrosshairTop>, Without<CrosshairBottom>, Without<CrosshairLeft>, Without<CrosshairRight>)>,
    mut bottom_query: Query<&mut Node, (With<CrosshairBottom>, Without<CrosshairTop>, Without<CrosshairLeft>, Without<CrosshairRight>)>,
    mut left_query: Query<&mut Node, (With<CrosshairLeft>, Without<CrosshairTop>, Without<CrosshairBottom>, Without<CrosshairRight>)>,
    mut right_query: Query<&mut Node, (With<CrosshairRight>, Without<CrosshairTop>, Without<CrosshairBottom>, Without<CrosshairLeft>)>,
) {
    let (heat, accuracy) = if let Some(status) = ammo_status_query.iter().next() {
        let accuracy = if let Some(inventory) = inventory_query.iter().next() {
             weapon_registry.configs.get(&inventory.active_slot)
                .map(|c| c.attributes.accuracy)
                .unwrap_or(1.0)
        } else {
            1.0
        };
        (status.heat, accuracy)
    } else {
        (0.0, 1.0)
    };

    let max_spread = 0.1; 
    let heat_penalty = heat * 0.05;
    let spread_angle = ((1.0 - accuracy) * max_spread + heat_penalty).max(0.001);
    
    // Convert spread angle to pixels (Approximate)
    let spread_pixels = spread_angle * 1000.0; 

    let config = &ui_config.crosshair;
    let gap = config.gap + spread_pixels;

    for mut node in top_query.iter_mut() {
        node.bottom = Val::Px(gap);
    }
    for mut node in bottom_query.iter_mut() {
        node.top = Val::Px(gap);
    }
    for mut node in left_query.iter_mut() {
        node.right = Val::Px(gap);
    }
    for mut node in right_query.iter_mut() {
        node.left = Val::Px(gap);
    }
}

#[derive(Component)]
pub struct KillFeedContainer;

#[derive(Component)]
pub struct KillFeedItem {
    pub timer: Timer,
}

fn spawn_kill_feed(mut commands: Commands, ui_config: Res<UiConfig>) {
    let config = &ui_config.kill_feed;
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(config.position[0]),
            top: Val::Px(config.position[1]),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        KillFeedContainer,
    ));
}

fn update_kill_feed(
    mut commands: Commands,
    mut events: MessageReader<DeathEvent>,
    ui_config: Res<UiConfig>,
    container_query: Query<Entity, With<KillFeedContainer>>,
    mut item_query: Query<(Entity, &mut KillFeedItem)>,
    time: Res<Time>,
) {
    let config = &ui_config.kill_feed;
    if let Some(container) = container_query.iter().next() {
        // Add new items
        for event in events.read() {
            commands.entity(container).with_children(|parent| {
                parent.spawn((
                    Text::new(&event.message),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::srgba(config.text_color[0], config.text_color[1], config.text_color[2], config.text_color[3])),
                    BackgroundColor(Color::srgba(config.background_color[0], config.background_color[1], config.background_color[2], config.background_color[3])),
                    Node {
                        margin: UiRect::bottom(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    KillFeedItem {
                        timer: Timer::from_seconds(config.item_duration, TimerMode::Once),
                    },
                ));
            });
        }
    }

    // Update timers and remove old items
    for (entity, mut item) in item_query.iter_mut() {
        item.timer.tick(time.delta());
        if item.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
struct PauseMenuUi;

#[derive(Component)]
enum PauseMenuButton {
    Resume,
    Settings,
    Reset,
    MainMenu,
    Quit,
}

#[derive(Component)]
pub struct GameplayUi;

fn despawn_gameplay_ui(
    mut commands: Commands,
    query: Query<Entity, Or<(With<AmmoUi>, With<CrosshairTop>, With<CrosshairBottom>, With<CrosshairLeft>, With<CrosshairRight>, With<KillFeedContainer>, With<StatsUi>, With<GameplayUi>)>>,
    health_ui_query: Query<Entity, Or<(With<crate::gameplay::PlayerHealthUi>, With<crate::gameplay::PlayerHealthBar>, With<crate::gameplay::DeathScreen>)>>,
    camera_query: Query<Entity, With<Camera>>,
    player_query: Query<Entity, With<PlayerBody>>,
    projectile_query: Query<Entity, With<Projectile>>,
    flash_query: Query<Entity, With<MuzzleFlash>>,
    grenade_query: Query<Entity, With<Grenade>>,
    weapon_model_query: Query<Entity, With<WeaponModel>>,
    explosion_query: Query<Entity, With<ExplosionParticle>>,
    turret_proj_query: Query<Entity, With<TurretProjectile>>,
) {
    for entity in query.iter()
        .chain(health_ui_query.iter())
        .chain(camera_query.iter())
        .chain(player_query.iter())
        .chain(projectile_query.iter())
        .chain(flash_query.iter())
        .chain(grenade_query.iter())
        .chain(weapon_model_query.iter())
        .chain(explosion_query.iter())
        .chain(turret_proj_query.iter())
    {
        if let Ok(mut cmds) = commands.get_entity(entity) {
            cmds.despawn();
        }
    }
}

fn spawn_pause_menu(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            GlobalZIndex(200),
            PauseMenuUi,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont { font_size: 48.0, ..default() },
                TextColor(Color::WHITE),
                Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
            ));

            for (label, button, color) in [
                ("Resume", PauseMenuButton::Resume, Color::srgb(0.3, 0.3, 0.3)),
                ("Settings", PauseMenuButton::Settings, Color::srgb(0.3, 0.3, 0.3)),
                ("Reset", PauseMenuButton::Reset, Color::srgb(0.3, 0.35, 0.5)),
                ("Main Menu", PauseMenuButton::MainMenu, Color::srgb(0.4, 0.3, 0.1)),
                ("Quit", PauseMenuButton::Quit, Color::srgb(0.5, 0.1, 0.1)),
            ] {
                parent.spawn((
                    Button,
                    Node {
                        width: Val::Px(200.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(color),
                    button,
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new(label),
                        TextFont { font_size: 20.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            }
        });
}

/// Clean up pause menu when leaving the Playing state entirely (e.g. going to main menu)
fn cleanup_pause_menu_on_exit(
    mut commands: Commands,
    query: Query<Entity, With<PauseMenuUi>>,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>,
    mut pause_open: ResMut<PauseMenuOpen>,
) {
    pause_open.0 = false;
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in settings_query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Manages spawning/despawning the pause overlay based on PauseMenuOpen resource
fn manage_pause_overlay(
    mut commands: Commands,
    pause_open: Res<PauseMenuOpen>,
    mut existing_ui: Query<(Entity, &mut Visibility), With<PauseMenuUi>>,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>,
) {
    let settings_open = !settings_query.is_empty();

    // Hide pause menu UI when settings overlay is open
    for (_entity, mut vis) in existing_ui.iter_mut() {
        *vis = if settings_open { Visibility::Hidden } else { Visibility::Inherited };
    }

    if !pause_open.is_changed() {
        return;
    }
    if pause_open.0 {
        // Spawn pause overlay if not already present
        if existing_ui.is_empty() {
            spawn_pause_menu(&mut commands);
        }
    } else {
        // Despawn pause overlay
        for (entity, _) in existing_ui.iter() {
            commands.entity(entity).despawn();
        }
        for entity in settings_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

fn pause_menu_action(
    interaction_query: Query<(&Interaction, &PauseMenuButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>,
    mut pause_open: ResMut<PauseMenuOpen>,
    mut player_query: Query<(&mut Health, &mut PhysicalTranslation, &mut Velocity), With<PlayerBody>>,
) {
    if !pause_open.0 { return; }

    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            match button {
                PauseMenuButton::Resume => {
                    pause_open.0 = false;
                }
                PauseMenuButton::Settings => {
                    if let Some(entity) = settings_query.iter().next() {
                        commands.entity(entity).despawn();
                    } else {
                        spawn_settings_menu(&mut commands);
                    }
                }
                PauseMenuButton::Reset => {
                    // Reset player position and health
                    for (mut health, mut position, mut velocity) in player_query.iter_mut() {
                        health.current = health.max;
                        position.0 = Vec3::new(0.0, 2.0, 0.0);
                        velocity.0 = Vec3::ZERO;
                    }
                    pause_open.0 = false;
                }
                PauseMenuButton::MainMenu => {
                    pause_open.0 = false;
                    next_state.set(GameState::MainMenu);
                }
                PauseMenuButton::Quit => {
                    exit.write(AppExit::Success);
                }
            }
        }
    }
}



fn debug_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut debug_settings: ResMut<DebugSettings>,
    mut game_settings: ResMut<GameSettings>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyH) {
        debug_settings.show_hitboxes = !debug_settings.show_hitboxes;
        game_settings.debug.show_hitboxes = debug_settings.show_hitboxes;
        println!("Hitboxes: {}", debug_settings.show_hitboxes);
    }
    if keyboard_input.just_pressed(KeyCode::KeyJ) {
        debug_settings.show_directions = !debug_settings.show_directions;
        println!("Directions: {}", debug_settings.show_directions);
    }
    if keyboard_input.just_pressed(KeyCode::KeyK) {
        debug_settings.free_cam = !debug_settings.free_cam;
        game_settings.debug.free_cam = debug_settings.free_cam;
        println!("Free Cam: {}", debug_settings.free_cam);
    }
}

fn toggle_camera_mode(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_mode: ResMut<CameraMode>,
    mut model_query: Query<&mut Visibility, With<PlayerModel>>,
    mut weapon_query: Query<&mut Visibility, (With<inventory::WeaponModel>, Without<PlayerModel>)>,
) {
    if keyboard_input.just_pressed(KeyCode::F5) {
        camera_mode.third_person = !camera_mode.third_person;
        println!("Camera Mode: {}", if camera_mode.third_person { "3rd Person" } else { "1st Person" });

        // Show/hide player model and weapon model
        for mut vis in model_query.iter_mut() {
            *vis = if camera_mode.third_person { Visibility::Inherited } else { Visibility::Hidden };
        }
        for mut vis in weapon_query.iter_mut() {
            *vis = if camera_mode.third_person { Visibility::Hidden } else { Visibility::Inherited };
        }
    }
}

/// Makes the pill model lean/tilt based on the player's movement state and lean.
fn animate_player_model(
    time: Res<Time>,
    mut model_query: Query<&mut Transform, With<PlayerModel>>,
    state_query: Query<(&MovementState, &Velocity, &movement::LeanState), With<PlayerBody>>,
) {
    let Ok((state, velocity, lean)) = state_query.single() else { return };

    let target_pitch = match *state {
        MovementState::Sprinting => -0.15,  // Lean forward when sprinting
        MovementState::Crouching => -0.08,  // Slight lean forward when crouching
        MovementState::Sliding => 0.25,     // Lean back when sliding
        MovementState::Airborne => {
            if velocity.y > 0.0 { -0.1 } else { 0.05 } // Forward on jump up, back on fall
        }
        MovementState::Prone => 1.4,        // Nearly horizontal
        _ => 0.0,                            // Upright for idle/walking
    };

    // Slight roll based on horizontal velocity for strafing lean
    let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
    let speed = horizontal.length();
    let strafe_roll = if speed > 1.0 {
        let right_dot = velocity.x * 0.01;
        (-right_dot * 0.05).clamp(-0.08, 0.08)
    } else {
        0.0
    };

    // Add lean tilt to model (from Q/E lean)
    let lean_roll = lean.current;

    let target_rot = Quat::from_euler(EulerRot::XZY, target_pitch, 0.0, strafe_roll + lean_roll);

    for mut transform in model_query.iter_mut() {
        transform.rotation = transform.rotation.slerp(target_rot, time.delta_secs() * 6.0);
    }
}

fn toggle_pause(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    mut pause_open: ResMut<PauseMenuOpen>,
) {
    if keyboard_input.just_pressed(keybinds.pause) {
        pause_open.0 = !pause_open.0;
    }
}

fn toggle_stats(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    mut commands: Commands,
    stats_query: Query<Entity, With<StatsUi>>,
) {
    if keyboard_input.just_pressed(keybinds.stats) {
        if let Some(entity) = stats_query.iter().next() {
            commands.entity(entity).despawn();
        } else {
            spawn_stats_ui(&mut commands);
        }
    }
}

fn spawn_stats_ui(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        StatsUi,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("FPS: 0.0"),
            TextFont { font_size: 16.0, ..default() },
            TextColor(Color::srgb(0.0, 1.0, 0.0)),
            StatsText,
        ));
    });
}

fn update_stats_ui(
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<StatsText>>,
    game_settings: Res<GameSettings>,
    entities: Query<Entity>,
    velocity_query: Query<&Velocity, With<PlayerBody>>,
) {
    for mut text in query.iter_mut() {
        let mut output = String::new();
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                output.push_str(&format!("FPS: {:.1}\n", value));
            }
        }
        
        if game_settings.debug.show_resource_usage {
            output.push_str(&format!("Entities: {}\n", entities.iter().count()));
        }

        // Show player speed
        if let Ok(velocity) = velocity_query.single() {
            let horiz = Vec3::new(velocity.x, 0.0, velocity.z).length();
            output.push_str(&format!("Speed: {:.1}\n", horiz));
        }
        
        text.0 = output;
    }
}

fn grab_cursor(
    mut cursors: Query<&mut CursorOptions>,
    state: Res<State<GameState>>,
    terminal_open: Res<WeaponTerminalOpen>,
    pause_open: Res<PauseMenuOpen>,
) {
    if let Ok(mut cursor) = cursors.single_mut() {
        match state.get() {
            GameState::Playing if !terminal_open.0 && !pause_open.0 => {
                cursor.visible = false;
                cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
            }
            _ => {
                cursor.visible = true;
                cursor.grab_mode = bevy::window::CursorGrabMode::None;
            }
        }
    }
}

fn keybind_remapping_system(
    mut interaction_query: Query<(&Interaction, &RemapButton), (Changed<Interaction>, With<Button>)>,
    mut all_buttons: Query<(&RemapButton, &Children, &mut BackgroundColor)>,
    mut text_query: Query<&mut Text>,
    mut remapping_state: ResMut<RemappingState>,
    mut keybinds: ResMut<Keybinds>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // Handle clicks
    for (interaction, button) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            remapping_state.active_action = Some(button.action.clone());
        }
    }

    // Handle input
    if let Some(action) = remapping_state.active_action.clone() {
        if let Some(key) = keyboard_input.get_just_pressed().next() {
            if *key != KeyCode::Escape {
                keybinds.set(&action, *key);
                save_keybinds(&keybinds);
            }
            remapping_state.active_action = None;
        }
    }
    
    // Update visuals
    for (button, children, mut bg_color) in all_buttons.iter_mut() {
        if let Some(active) = &remapping_state.active_action {
            if active == &button.action {
                *bg_color = BackgroundColor(Color::srgb(0.8, 0.8, 0.2));
                if let Some(child) = children.first() {
                    if let Ok(mut text) = text_query.get_mut(*child) {
                        text.0 = "Press...".to_string();
                    }
                }
                continue;
            }
        }
        
        *bg_color = BackgroundColor(Color::srgb(0.3, 0.3, 0.3));
        if let Some(child) = children.first() {
            if let Ok(mut text) = text_query.get_mut(*child) {
                let key = keybinds.get(&button.action);
                text.0 = format!("{:?}", key);
            }
        }
    }
}

/// Propagate weapon render layer to all descendants of WeaponModel entities.
/// Runs every frame; once all descendants are tagged the query is empty (free).
fn ensure_weapon_render_layers(
    mut commands: Commands,
    weapon_query: Query<Entity, (With<WeaponModel>, Without<WeaponLayerEntity>)>,
    untagged_children: Query<(Entity, &ChildOf), Without<WeaponLayerEntity>>,
    weapon_parents: Query<(), With<WeaponLayerEntity>>,
) {
    // Tag any WeaponModel entity that doesn't have the marker yet
    for entity in weapon_query.iter() {
        commands.entity(entity).insert((WeaponLayerEntity, RenderLayers::layer(1)));
    }
    // Propagate one level of children per frame
    for (entity, child_of) in untagged_children.iter() {
        if weapon_parents.get(child_of.get()).is_ok() {
            commands.entity(entity).insert((WeaponLayerEntity, RenderLayers::layer(1)));
        }
    }
}

fn draw_hitboxes(
    mut gizmos: Gizmos,
    enemy_query: Query<(&GlobalTransform, Option<&crate::gameplay::Turret>), With<crate::gameplay::Enemy>>,
    collider_query: Query<(&GlobalTransform, &crate::world::objects::StaticCollider)>,
    ramp_query: Query<(&GlobalTransform, &crate::world::objects::RampCollider)>,
    debug_settings: Res<DebugSettings>,
) {
    if !debug_settings.show_hitboxes {
        return;
    }

    // Draw all static colliders (green wireframe) — respecting rotation
    for (transform, collider) in collider_query.iter() {
        let pos = transform.translation();
        let rot = transform.to_scale_rotation_translation().1;
        let size = collider.half_extents * 2.0;
        gizmos.cube(
            Transform::from_translation(pos)
                .with_rotation(rot)
                .with_scale(size),
            Color::srgba(0.0, 1.0, 0.0, 0.4),
        );
    }

    // Draw ramp colliders (cyan wireframe, respecting rotation)
    for (transform, ramp) in ramp_query.iter() {
        let size = ramp.half_extents * 2.0;
        gizmos.cube(
            Transform::from_translation(transform.translation())
                .with_rotation(transform.to_scale_rotation_translation().1)
                .with_scale(size),
            Color::srgba(0.0, 1.0, 1.0, 0.4),
        );
    }

    // Draw enemy hitboxes (red)
    for (transform, turret) in enemy_query.iter() {
        let pos = transform.translation();
        let color = Color::srgb(1.0, 0.0, 0.0);
        if turret.is_some() {
            gizmos.cube(Transform::from_translation(pos).with_scale(Vec3::splat(1.0)), color);
        } else {
            gizmos.cube(Transform::from_translation(pos + Vec3::new(0.0, 1.0, 0.0)).with_scale(Vec3::new(0.5, 2.0, 0.5)), color);
        }
    }
}

fn sync_settings(
    game_settings: Res<GameSettings>,
    mut debug_settings: ResMut<DebugSettings>,
    mut commands: Commands,
    stats_query: Query<Entity, With<StatsUi>>,
    mut window_query: Query<&mut Window>,
    mut wireframe_config: ResMut<bevy::pbr::wireframe::WireframeConfig>,
) {
    if game_settings.is_changed() {
        debug_settings.show_hitboxes = game_settings.debug.show_hitboxes;
        debug_settings.free_cam = game_settings.debug.free_cam;
        wireframe_config.global = game_settings.debug.show_wireframe;
        
        // Sync FPS counter
        if game_settings.debug.show_fps {
            if stats_query.iter().next().is_none() {
                spawn_stats_ui(&mut commands);
            }
        } else {
            if let Some(entity) = stats_query.iter().next() {
                commands.entity(entity).despawn();
            }
        }

        // Sync Graphics
        if let Some(mut window) = window_query.iter_mut().next() {
            if game_settings.graphics.resolution == [0, 0] {
                window.mode = bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current);
            } else {
                window.mode = bevy::window::WindowMode::Windowed;
                let width = game_settings.graphics.resolution[0] as f32;
                let height = game_settings.graphics.resolution[1] as f32;
                if window.resolution.width() != width || window.resolution.height() != height {
                    window.resolution.set(width, height);
                }
            }
            
            // Simple FPS Cap via VSync
            let target_mode = if game_settings.graphics.fps_cap > 0 {
                bevy::window::PresentMode::AutoVsync
            } else {
                bevy::window::PresentMode::AutoNoVsync
            };
            
            if window.present_mode != target_mode {
                window.present_mode = target_mode;
            }
        }
    }
}

// ── Hit Marker System ──

#[derive(Component)]
pub struct HitMarker {
    pub timer: Timer,
}

#[derive(Component)]
pub struct DamageNumber {
    pub timer: Timer,
    pub velocity: Vec3,
}

pub fn spawn_hit_marker(commands: &mut Commands) {
    let arm_length = 12.0;
    let thickness = 2.0;
    let gap = 4.0;
    let color = Color::srgba(1.0, 1.0, 1.0, 0.9);

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(0.0),
            height: Val::Px(0.0),
            ..default()
        },
        HitMarker { timer: Timer::from_seconds(0.3, TimerMode::Once) },
    )).with_children(|parent| {
        // Build X-shaped hitmarker using small square segments along diagonals
        // Each arm is a series of small segments going outward at 45 degrees
        let segment_size = thickness;
        let segments = (arm_length / segment_size) as i32;
        
        for (dir_x, dir_y) in [(-1.0_f32, -1.0_f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            for i in 0..segments {
                let dist = gap + (i as f32) * segment_size * 0.707; // 0.707 ≈ 1/sqrt(2)
                let px = dir_x * dist;
                let py = dir_y * dist;
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(px - segment_size / 2.0),
                        top: Val::Px(py - segment_size / 2.0),
                        width: Val::Px(segment_size),
                        height: Val::Px(segment_size),
                        ..default()
                    },
                    BackgroundColor(color),
                ));
            }
        }
    });
}

pub fn spawn_damage_number(commands: &mut Commands, damage: f32, _position: Vec3) {
    let color = if damage >= 50.0 {
        Color::srgb(1.0, 0.2, 0.2)
    } else {
        Color::srgb(1.0, 1.0, 0.3)
    };

    let mut rng = rand::rng();
    let offset_x: f32 = rng.random_range(-4.0..4.0);
    let vel_x: f32 = rng.random_range(-30.0..30.0);
    let vel_y: f32 = rng.random_range(-65.0..-35.0);

    commands.spawn((
        Text::new(format!("{:.0}", damage)),
        TextFont { font_size: 18.0, ..default() },
        TextColor(color),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(52.0 + offset_x),
            top: Val::Percent(45.0),
            ..default()
        },
        DamageNumber {
            timer: Timer::from_seconds(0.8, TimerMode::Once),
            velocity: Vec3::new(vel_x, vel_y, 0.0),
        },
    ));
}

fn update_hit_markers(
    mut commands: Commands,
    time: Res<Time>,
    mut marker_query: Query<(Entity, &mut HitMarker)>,
    mut number_query: Query<(Entity, &mut DamageNumber, &mut Node, &mut TextColor)>,
) {
    for (entity, mut marker) in marker_query.iter_mut() {
        marker.timer.tick(time.delta());
        if marker.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut number, mut node, mut color) in number_query.iter_mut() {
        number.timer.tick(time.delta());
        let dt = time.delta_secs();

        if let Val::Percent(top) = node.top {
            node.top = Val::Percent(top + number.velocity.y * dt * 0.05);
        }
        if let Val::Percent(left) = node.left {
            node.left = Val::Percent(left + number.velocity.x * dt * 0.03);
        }

        // Decelerate horizontal movement
        number.velocity.x *= 0.95;

        let alpha = 1.0 - number.timer.fraction();
        color.0 = color.0.with_alpha(alpha);

        if number.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// In-Game Weapon Terminal Overlay
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn spawn_weapon_terminal_overlay(
    mut commands: Commands,
    terminal_open: Res<WeaponTerminalOpen>,
    existing_ui: Query<Entity, With<WeaponTerminalUi>>,
    registry: Res<WeaponRegistry>,
    loadout: Res<PlayerLoadout>,
) {
    // Only spawn when first opened and no UI exists yet
    if !terminal_open.0 || !existing_ui.is_empty() {
        return;
    }

    // Build a simplified weapon picker overlay
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        GlobalZIndex(100),
        WeaponTerminalUi,
    )).with_children(|overlay| {
        // Main panel
        overlay.spawn(
            Node {
                width: Val::Px(700.0),
                max_height: Val::Percent(80.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(20.0)),
                row_gap: Val::Px(12.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ).with_children(|panel| {
            // Title
            panel.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(8.0)),
                ..default()
            }).with_children(|title_row| {
                title_row.spawn((
                    Text::new("WEAPON TERMINAL"),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(Color::srgba(0.9, 0.3, 0.3, 0.95)),
                ));
                // Close button
                title_row.spawn((
                    Button,
                    Node {
                        width: Val::Px(32.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.5, 0.1, 0.1, 0.8)),
                    WeaponTerminalClose,
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new("✕"),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            });

            // Show weapons grouped by slot
            let slots = [
                (WeaponSlot::Primary, "PRIMARY", &loadout.primary),
                (WeaponSlot::Secondary, "SECONDARY", &loadout.secondary),
                (WeaponSlot::Melee, "MELEE", &loadout.melee),
                (WeaponSlot::Equipment, "EQUIPMENT", &loadout.equipment),
            ];

            for (slot, label, equipped_id) in &slots {
                panel.spawn((
                    Text::new(label.to_string()),
                    TextFont { font_size: 14.0, ..default() },
                    TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                    Node { margin: UiRect::top(Val::Px(6.0)), ..default() },
                ));

                // Weapon items for this slot
                panel.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                }).with_children(|slot_list| {
                    let mut weapons_for_slot: Vec<(&String, &WeaponConfig)> = registry.weapons.iter()
                        .filter(|(_, c)| slot_from_weapon_type(&c.meta.weapon_type) == *slot)
                        .collect();
                    weapons_for_slot.sort_by(|a, b| a.1.info.name.cmp(&b.1.info.name));

                    for (wid, config) in weapons_for_slot {
                        let is_equipped = *equipped_id == wid.as_str();
                        let bg = if is_equipped {
                            Color::srgba(0.15, 0.35, 0.15, 0.7)
                        } else {
                            Color::srgba(0.12, 0.12, 0.18, 0.7)
                        };

                        slot_list.spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(38.0),
                                padding: UiRect::horizontal(Val::Px(14.0)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::SpaceBetween,
                                ..default()
                            },
                            BackgroundColor(bg),
                            WeaponTerminalItem {
                                weapon_id: wid.clone(),
                                slot: *slot,
                            },
                        )).with_children(|item| {
                            item.spawn((
                                Text::new(&config.info.name),
                                TextFont { font_size: 15.0, ..default() },
                                TextColor(if is_equipped { Color::srgb(0.5, 1.0, 0.5) } else { Color::srgba(0.85, 0.85, 0.9, 0.9) }),
                            ));
                            if is_equipped {
                                item.spawn((
                                    Text::new("EQUIPPED"),
                                    TextFont { font_size: 11.0, ..default() },
                                    TextColor(Color::srgba(0.4, 0.8, 0.4, 0.7)),
                                ));
                            }
                        });
                    }
                });
            }
        });
    });
}

fn weapon_terminal_interaction(
    mut item_query: Query<(&Interaction, &WeaponTerminalItem, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
    mut loadout: ResMut<PlayerLoadout>,
    mut commands: Commands,
    existing_ui: Query<Entity, With<WeaponTerminalUi>>,
    mut terminal_open: ResMut<WeaponTerminalOpen>,
    mut registry: ResMut<WeaponRegistry>,
    weapon_model_query: Query<(Entity, &WeaponModel)>,
    camera_query: Query<Entity, With<Camera3d>>,
    inventory_query: Query<&Inventory>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !terminal_open.0 { return; }

    for (interaction, item, mut bg) in item_query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                // Equip the weapon
                match item.slot {
                    WeaponSlot::Primary => loadout.primary = item.weapon_id.clone(),
                    WeaponSlot::Secondary => loadout.secondary = item.weapon_id.clone(),
                    WeaponSlot::Melee => loadout.melee = item.weapon_id.clone(),
                    WeaponSlot::Equipment => loadout.equipment = item.weapon_id.clone(),
                }

                sync_loadout_to_configs(&mut registry, &loadout);

                // Rebuild the in-hand weapon model if we're holding the same slot
                if let Ok(inventory) = inventory_query.single() {
                    if inventory.active_slot == item.slot {
                        // Despawn old weapon model
                        for (entity, _) in weapon_model_query.iter() {
                            commands.entity(entity).despawn();
                        }
                        // Only spawn new model if we have a config for this slot
                        if registry.configs.contains_key(&item.slot) {
                            let skin = loadout.get_skin(item.slot);
                            if let Some(camera_entity) = camera_query.iter().next() {
                                let weapon_entity = spawn_weapon_visual_skinned(
                                    &mut commands,
                                    item.slot,
                                    skin,
                                    &asset_server,
                                    &registry,
                                    &mut meshes,
                                    &mut materials,
                                );
                                commands.entity(weapon_entity).insert(WeaponModel);
                                commands.entity(camera_entity).add_child(weapon_entity);
                            }
                        }
                    }
                }

                // Close the overlay
                terminal_open.0 = false;
                for entity in existing_ui.iter() {
                    commands.entity(entity).despawn();
                }
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgba(0.2, 0.2, 0.3, 0.8));
            }
            Interaction::None => {
                let is_equipped = match item.slot {
                    WeaponSlot::Primary => loadout.primary == item.weapon_id,
                    WeaponSlot::Secondary => loadout.secondary == item.weapon_id,
                    WeaponSlot::Melee => loadout.melee == item.weapon_id,
                    WeaponSlot::Equipment => loadout.equipment == item.weapon_id,
                };
                *bg = if is_equipped {
                    BackgroundColor(Color::srgba(0.15, 0.35, 0.15, 0.7))
                } else {
                    BackgroundColor(Color::srgba(0.12, 0.12, 0.18, 0.7))
                };
            }
        }
    }
}

fn close_weapon_terminal(
    mut commands: Commands,
    mut terminal_open: ResMut<WeaponTerminalOpen>,
    close_query: Query<&Interaction, (Changed<Interaction>, With<WeaponTerminalClose>)>,
    existing_ui: Query<Entity, With<WeaponTerminalUi>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let should_close = keyboard_input.just_pressed(KeyCode::Escape)
        || close_query.iter().any(|i| *i == Interaction::Pressed);

    if terminal_open.0 && should_close {
        terminal_open.0 = false;
        for entity in existing_ui.iter() {
            commands.entity(entity).despawn();
        }
    }
}
