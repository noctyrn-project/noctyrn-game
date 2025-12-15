use bevy::prelude::*;
use bevy::window::CursorOptions;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::app::AppExit;
use crate::weapons::{WeaponSlot, spawn_weapon_visual, WeaponRegistry};
use crate::gameplay::{Health, PlayerBody, Regenerating};

mod movement;
mod input;
mod camera;
mod inventory;
pub mod shooting;

use movement::{Velocity, PhysicalTranslation, PreviousPhysicalTranslation, CrouchHeight, advance_physics, interpolate_rendered_transform};
use input::{AccumulatedInput, accumulate_input, clear_input, Keybinds, load_keybinds, save_keybinds};
use camera::{CameraSensitivity, rotate_camera, translate_camera, free_cam_movement};
use inventory::{Inventory, WeaponModel, handle_weapon_switching};
use shooting::{fire_weapon, move_projectiles, handle_weapon_recoil, handle_muzzle_flash, handle_melee_swing, handle_grenade_throw, update_ammo_ui, reload_weapon, handle_weapon_sway, AmmoStatus, AmmoUi, CameraRecoil, handle_camera_recoil};

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
        
        app.add_systems(Startup, (spawn_player, spawn_crosshair, spawn_ammo_ui, load_keybinds));
        app.add_systems(OnEnter(GameState::Menu), spawn_menu);
        app.add_systems(OnExit(GameState::Menu), despawn_menu);
        app.add_systems(Update, (toggle_pause, grab_cursor, toggle_stats, update_stats_ui, debug_input));
        app.add_systems(Update, (menu_action, keybind_remapping_system).run_if(in_state(GameState::Menu)));

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
            draw_hitboxes
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

fn spawn_ammo_ui(mut commands: Commands) {
    commands.spawn((
        Text::new("Ammo: -- / --"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            right: Val::Px(20.0),
            ..default()
        },
        AmmoUi,
    ));
}

fn spawn_crosshair(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(4.0),
            height: Val::Px(4.0),
            margin: UiRect {
                left: Val::Px(-2.0),
                top: Val::Px(-2.0),
                ..default()
            },
            ..default()
        },
        BackgroundColor(Color::WHITE),
    ));
}

#[derive(Component)]
struct MenuUi;

#[derive(Component)]
enum MenuButton {
    Resume,
    Stats,
    Keybindings,
    Debug,
    Quit,
}

#[derive(Component)]
struct StatsUi;

#[derive(Component)]
struct StatsText;

#[derive(Resource, Default)]
pub struct DebugSettings {
    pub show_hitboxes: bool,
    pub show_directions: bool,
    pub free_cam: bool,
}

#[derive(Component)]
struct KeybindingsUi;

#[derive(Component)]
struct DebugUi;

#[derive(Resource, Default)]
pub struct RemappingState {
    pub active_action: Option<String>,
}

#[derive(Component)]
pub struct RemapButton {
    pub action: String,
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
                MenuButton::Resume,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("Resume"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Stats Button
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
                MenuButton::Stats,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("Toggle Stats"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Keybindings Button
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
                MenuButton::Keybindings,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("Keybindings"),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Debug Button
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
                MenuButton::Debug,
            )).with_children(|parent| {
                parent.spawn((
                    Text::new("Debug Mode"),
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

fn despawn_menu(mut commands: Commands, query: Query<Entity, With<MenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn menu_action(
    interaction_query: Query<(&Interaction, &MenuButton), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: EventWriter<AppExit>,
    mut commands: Commands,
    stats_query: Query<Entity, With<StatsUi>>,
    keybinds_query: Query<Entity, With<KeybindingsUi>>,
    debug_query: Query<Entity, With<DebugUi>>,
    keybinds: Res<Keybinds>,
) {
    for (interaction, button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            match button {
                MenuButton::Resume => {
                    next_state.set(GameState::Playing);
                }
                MenuButton::Stats => {
                    if let Some(entity) = stats_query.iter().next() {
                        commands.entity(entity).despawn();
                    } else {
                        spawn_stats_ui(&mut commands);
                    }
                }
                MenuButton::Keybindings => {
                    // Close other menus?
                    if let Some(entity) = keybinds_query.iter().next() {
                        commands.entity(entity).despawn();
                    } else {
                        spawn_keybindings_ui(&mut commands, &keybinds);
                    }
                }
                MenuButton::Debug => {
                    if let Some(entity) = debug_query.iter().next() {
                        commands.entity(entity).despawn();
                    } else {
                        spawn_debug_ui(&mut commands);
                    }
                }
                MenuButton::Quit => {
                    exit.write(AppExit::Success);
                }
            }
        }
    }
}

fn spawn_keybindings_ui(commands: &mut Commands, keybinds: &Keybinds) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(80.0),
            height: Val::Percent(80.0),
            left: Val::Percent(10.0),
            top: Val::Percent(10.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.95)),
        KeybindingsUi,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Keybindings (Click to remap)"),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::WHITE),
        ));
        
        let keys = [
            ("Forward", keybinds.forward),
            ("Backward", keybinds.backward),
            ("Left", keybinds.left),
            ("Right", keybinds.right),
            ("Jump", keybinds.jump),
            ("Sprint", keybinds.sprint),
            ("Crouch", keybinds.crouch),
            ("Interact", keybinds.interact),
            ("Grenade", keybinds.grenade),
            ("Melee", keybinds.melee),
        ];
        
        for (name, key) in keys {
            parent.spawn((
                Node {
                    width: Val::Px(400.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    margin: UiRect::all(Val::Px(5.0)),
                    ..default()
                },
            )).with_children(|row| {
                row.spawn((
                    Text::new(name),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
                
                row.spawn((
                    Button,
                    Node {
                        width: Val::Px(150.0),
                        height: Val::Px(35.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                    RemapButton { action: name.to_string() },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(format!("{:?}", key)),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            });
        }
        
        parent.spawn((
            Button,
            Node {
                margin: UiRect::top(Val::Px(20.0)),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.5, 0.1, 0.1)),
            MenuButton::Keybindings, // Re-use to close
        )).with_children(|parent| {
            parent.spawn((Text::new("Close"), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
        });
    });
}

fn spawn_debug_ui(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(50.0),
            height: Val::Percent(50.0),
            left: Val::Percent(25.0),
            top: Val::Percent(25.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.95)),
        DebugUi,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Debug Mode"),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::WHITE),
        ));
        
        // Toggles (Simplified as text for now)
        parent.spawn((Text::new("Hitboxes: [H]"), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
        parent.spawn((Text::new("Directions: [J]"), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
        parent.spawn((Text::new("Free Cam: [K]"), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
        
        parent.spawn((
            Button,
            Node {
                margin: UiRect::top(Val::Px(20.0)),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.5, 0.1, 0.1)),
            MenuButton::Debug, // Re-use to close
        )).with_children(|parent| {
            parent.spawn((Text::new("Close"), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
        });
    });
}

fn debug_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut debug_settings: ResMut<DebugSettings>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyH) {
        debug_settings.show_hitboxes = !debug_settings.show_hitboxes;
        println!("Hitboxes: {}", debug_settings.show_hitboxes);
    }
    if keyboard_input.just_pressed(KeyCode::KeyJ) {
        debug_settings.show_directions = !debug_settings.show_directions;
        println!("Directions: {}", debug_settings.show_directions);
    }
    if keyboard_input.just_pressed(KeyCode::KeyK) {
        debug_settings.free_cam = !debug_settings.free_cam;
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
) {
    for mut text in query.iter_mut() {
        if let Some(fps) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps.smoothed() {
                text.0 = format!("FPS: {:.1}", value);
            }
        }
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
