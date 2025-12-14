use bevy::prelude::*;
use bevy::window::CursorOptions;
use crate::weapons::{WeaponSlot, spawn_weapon_visual, WeaponRegistry};
use crate::gameplay::{Health, PlayerBody};

mod movement;
mod input;
mod camera;
mod inventory;
pub mod shooting;

use movement::{Velocity, PhysicalTranslation, PreviousPhysicalTranslation, CrouchHeight, advance_physics, interpolate_rendered_transform};
use input::{AccumulatedInput, accumulate_input, clear_input};
use camera::{CameraSensitivity, rotate_camera, translate_camera};
use inventory::{Inventory, WeaponModel, handle_weapon_switching};
use shooting::{fire_weapon, move_projectiles, handle_weapon_recoil, handle_muzzle_flash, handle_melee_swing, handle_grenade_throw, update_ammo_ui, reload_weapon, handle_weapon_sway, AmmoStatus, AmmoUi};

pub struct Player;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    #[default]
    Playing,
    Menu,
}

impl Plugin for Player {
    fn build(&self, app: &mut App) {
        app.init_resource::<DidFixedTimestepRunThisFrame>();
        app.init_state::<GameState>();
        
        app.add_systems(Startup, (spawn_player, spawn_crosshair, spawn_ammo_ui));
        app.add_systems(OnEnter(GameState::Menu), spawn_menu);
        app.add_systems(OnExit(GameState::Menu), despawn_menu);
        app.add_systems(Update, (toggle_pause, grab_cursor));

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
            shooting::handle_explosion_particles,
            handle_weapon_sway,
            update_ammo_ui,
            reload_weapon
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
                )
                    .chain()
                    .in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
            ),
        );
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

fn spawn_menu(mut commands: Commands) {
    let font = TextFont {
        font_size: 30.0,
        ..default()
    };
    
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        MenuUi,
    )).with_children(|parent| {
        parent.spawn((
            Text::new("PAUSED"),
            TextFont {
                font_size: 60.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(40.0)),
                ..default()
            },
        ));
        
        parent.spawn((Text::new("Controls:"), font.clone()));
        parent.spawn((Text::new("WASD - Move"), font.clone()));
        parent.spawn((Text::new("Space - Jump"), font.clone()));
        parent.spawn((Text::new("Shift - Sprint"), font.clone()));
        parent.spawn((Text::new("Ctrl/C - Crouch"), font.clone()));
        parent.spawn((Text::new("Left Click - Fire"), font.clone()));
        parent.spawn((Text::new("1-4 - Switch Weapon"), font.clone()));
        parent.spawn((Text::new("R - Reload"), font.clone()));
        parent.spawn((Text::new("ESC - Resume"), font.clone()));
    });
}

fn despawn_menu(mut commands: Commands, query: Query<Entity, With<MenuUi>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn toggle_pause(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match state.get() {
            GameState::Playing => next_state.set(GameState::Menu),
            GameState::Menu => next_state.set(GameState::Playing),
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
