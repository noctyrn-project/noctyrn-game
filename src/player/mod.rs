use bevy::prelude::*;
use bevy::window::CursorOptions;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::app::AppExit;
use crate::weapons::{WeaponSlot, spawn_weapon_visual, WeaponRegistry};
use crate::gameplay::{Health, PlayerBody, Regenerating};
use crate::ui_config::UiConfig;
use crate::gameplay::DeathEvent;
use crate::ui_settings::{spawn_settings_menu, update_settings_menu, handle_settings_interaction, SettingsState};
use crate::settings::{GameSettings, DebugSettingsConfig};

mod movement;
mod input;
mod camera;
mod inventory;
pub mod shooting;

use movement::{Velocity, PhysicalTranslation, PreviousPhysicalTranslation, CrouchHeight, advance_physics, interpolate_rendered_transform};
use input::{AccumulatedInput, accumulate_input, clear_input, load_keybinds, PlayerToggleState};
pub use input::{Keybinds, save_keybinds};
use camera::{CameraSensitivity, rotate_camera, translate_camera, free_cam_movement, update_fov};
use inventory::{Inventory, WeaponModel, handle_weapon_switching};
use shooting::{fire_weapon, move_projectiles, handle_weapon_recoil, handle_muzzle_flash, handle_melee_swing, handle_grenade_throw, update_ammo_ui, reload_weapon, handle_weapon_sway, AmmoStatus, AmmoUi, CameraRecoil, handle_camera_recoil};

#[derive(Resource, Default)]
pub struct DebugSettings {
    pub show_hitboxes: bool,
    pub show_directions: bool,
    pub free_cam: bool,
}

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
    Playing,
    Menu,
}

impl Plugin for Player {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        app.init_resource::<DidFixedTimestepRunThisFrame>();
        app.init_state::<GameState>();
        app.init_resource::<DebugSettings>();
        app.init_resource::<RemappingState>();
        app.init_resource::<SettingsState>();
        
        app.add_systems(Startup, (spawn_player, spawn_crosshair, spawn_ammo_ui, spawn_kill_feed, load_keybinds));
        app.add_systems(OnEnter(GameState::Menu), spawn_menu);
        app.add_systems(OnExit(GameState::Menu), despawn_menu);
        app.add_systems(Update, (toggle_pause, grab_cursor, toggle_stats, update_stats_ui, debug_input, sync_settings, update_fov));
        app.add_systems(Update, (menu_action, keybind_remapping_system, update_settings_menu, handle_settings_interaction).run_if(in_state(GameState::Menu)));

        app.add_systems(PreUpdate, clear_fixed_timestep_flag);
        app.add_systems(FixedPreUpdate, set_fixed_time_step_flag);
        app.add_systems(FixedUpdate, advance_physics.run_if(in_state(GameState::Playing)));
        
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
        
        app.add_systems(Update, (
            shooting::handle_explosion_particles,
            handle_weapon_sway,
            update_ammo_ui,
            reload_weapon,
            draw_hitboxes,
            update_crosshair,
            update_kill_feed,
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
                    .in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
            ),
        );

        app.add_systems(Update, draw_hitboxes);
        app.add_systems(Update, sync_settings);
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
) {
    // Spawn Camera
    let camera_entity = commands.spawn((
        Camera3d::default(),
        CameraSensitivity::default(),
        CameraRecoil::default(),
        Transform::from_xyz(0.0, 0.0, 0.0), // Initial pos, will be updated by translate_camera
    )).id();

    // Spawn initial weapon (Primary)
    let weapon_entity = spawn_weapon_visual(
        &mut commands,
        WeaponSlot::Primary,
        &asset_server,
        &weapon_registry,
        &mut meshes,
        &mut materials,
    );
    commands.entity(weapon_entity).insert(WeaponModel);
    commands.entity(camera_entity).add_child(weapon_entity);
    
    let initial_pos = Vec3::new(0.0, 2.0, 0.0);
    commands.spawn((
        Name::new("Player"),
        Transform::from_translation(initial_pos).with_scale(Vec3::splat(0.3)),
        AccumulatedInput::default(),
        PlayerToggleState::default(),
        Velocity::default(),
        PhysicalTranslation(initial_pos),
        PreviousPhysicalTranslation(initial_pos),
        CrouchHeight::default(),
        Inventory::default(),
        AmmoStatus::default(),
        Health { current: 100.0, max: 100.0 },
        Regenerating::default(),
        PlayerBody,
    ));
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
        BorderRadius::all(Val::Px(config.border_radius)),
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
                    BorderRadius::all(Val::Px(config.border_radius)),
                ));
            });
        }
    }

    // Update timers and remove old items
    for (entity, mut item) in item_query.iter_mut() {
        item.timer.tick(time.delta());
        if item.timer.finished() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
struct MenuUi;

#[derive(Component)]
enum MenuButton {
    Resume,
    Settings,
    Quit,
}

fn spawn_menu(mut commands: Commands) {
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            MenuUi,
        ))
        .with_children(|parent| {
            // Resume Button
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                BorderRadius::all(Val::Px(10.0)),
                MenuButton::Resume,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("Resume"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Settings Button
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                BorderRadius::all(Val::Px(10.0)),
                MenuButton::Settings,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("Settings"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Quit Button
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(50.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.5, 0.1, 0.1)),
                BorderRadius::all(Val::Px(10.0)),
                MenuButton::Quit,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("Quit"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });
}

fn despawn_menu(
    mut commands: Commands, 
    query: Query<Entity, With<MenuUi>>,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in settings_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn menu_action(
    interaction_query: Query<(&Interaction, &MenuButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: EventWriter<AppExit>,
    mut commands: Commands,
    settings_query: Query<Entity, With<crate::ui_settings::SettingsMenuUi>>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            match button {
                MenuButton::Resume => {
                    next_state.set(GameState::Playing);
                }
                MenuButton::Settings => {
                    if let Some(entity) = settings_query.iter().next() {
                        commands.entity(entity).despawn();
                    } else {
                        spawn_settings_menu(&mut commands);
                    }
                }
                MenuButton::Quit => {
                    exit.write(AppExit::Success);
                }
            }
        }
    }
}



fn debug_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut debug_settings: ResMut<DebugSettings>,
    mut game_settings: ResMut<crate::settings::GameSettings>,
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

fn toggle_pause(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(keybinds.pause) {
        match state.get() {
            GameState::Playing => next_state.set(GameState::Menu),
            GameState::Menu => next_state.set(GameState::Playing),
        }
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
    game_settings: Res<crate::settings::GameSettings>,
    entities: Query<Entity>,
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
        
        text.0 = output;
    }
}

fn grab_cursor(
    mut cursors: Query<&mut CursorOptions>,
    state: Res<State<GameState>>,
) {
    if let Ok(mut cursor) = cursors.single_mut() {
        match state.get() {
            GameState::Playing => {
                cursor.visible = false;
                cursor.grab_mode = bevy::window::CursorGrabMode::Locked;
            }
            GameState::Menu => {
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

fn draw_hitboxes(
    mut gizmos: Gizmos,
    query: Query<(&GlobalTransform, Option<&crate::gameplay::Turret>), With<crate::gameplay::Enemy>>,
    debug_settings: Res<DebugSettings>,
) {
    if !debug_settings.show_hitboxes {
        return;
    }

    for (transform, turret) in query.iter() {
        let pos = transform.translation();
        let color = Color::srgb(1.0, 0.0, 0.0);
        
        if turret.is_some() {
             gizmos.cuboid(Transform::from_translation(pos).with_scale(Vec3::splat(1.0)), color);
        } else {
             // Character (Approximate hitbox)
             gizmos.cuboid(Transform::from_translation(pos + Vec3::new(0.0, 1.0, 0.0)).with_scale(Vec3::new(0.5, 2.0, 0.5)), color);
        }
    }
}

fn sync_settings(
    game_settings: Res<crate::settings::GameSettings>,
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
