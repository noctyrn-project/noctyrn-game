use bevy::prelude::*;
use crate::weapons::{WeaponSlot, spawn_weapon_visual, WeaponRegistry};

mod movement;
mod input;
mod camera;
mod inventory;
pub mod shooting;

use movement::{Velocity, PhysicalTranslation, PreviousPhysicalTranslation, advance_physics, interpolate_rendered_transform};
use input::{AccumulatedInput, accumulate_input, clear_input};
use camera::{CameraSensitivity, rotate_camera, translate_camera};
use inventory::{Inventory, WeaponModel, handle_weapon_switching};
use shooting::{fire_weapon, move_projectiles, handle_weapon_recoil, handle_muzzle_flash, handle_melee_swing, handle_grenade_throw, update_ammo_ui, reload_weapon, AmmoStatus, AmmoUi};

pub struct Player;

impl Plugin for Player {
    fn build(&self, app: &mut App) {
        app.init_resource::<DidFixedTimestepRunThisFrame>();
        app.add_systems(Startup, (spawn_text, spawn_player, spawn_crosshair, spawn_ammo_ui));
        app.add_systems(PreUpdate, clear_fixed_timestep_flag);
        app.add_systems(FixedPreUpdate, set_fixed_time_step_flag);
        app.add_systems(FixedUpdate, advance_physics);
        app.add_systems(Update, (
            handle_weapon_switching, 
            fire_weapon, 
            move_projectiles, 
            handle_weapon_recoil, 
            handle_muzzle_flash,
            handle_melee_swing,
            handle_grenade_throw,
            update_ammo_ui,
            reload_weapon
        ));
        app.add_systems(
            RunFixedMainLoop,
            (
                (
                    rotate_camera,
                    accumulate_input,
                )
                    .chain()
                    .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
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
        Inventory::default(),
        AmmoStatus::default(),
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
fn spawn_text(mut commands: Commands) {
    let font = TextFont {
        font_size: 25.0,
        ..default()
    };
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (Text::new("Move: WASD | Jump: Space"), font.clone()),
            (Text::new("Look: Mouse | Fire: Left Click"), font.clone()),
            (Text::new("Switch Weapon: 1-4"), font)
        ],
    ));
}
