use bevy::prelude::*;
use crate::weapons::{WeaponSlot, spawn_weapon_visual, WeaponRegistry};

#[derive(Component)]
pub struct Inventory {
    pub active_slot: WeaponSlot,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            active_slot: WeaponSlot::Primary,
        }
    }
}

#[derive(Component)]
pub struct WeaponModel;

pub fn handle_weapon_switching(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Inventory>,
    weapon_query: Query<Entity, With<WeaponModel>>,
    camera_query: Query<Entity, With<Camera>>,
    asset_server: Res<AssetServer>,
    weapon_registry: Res<WeaponRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for mut inventory in query.iter_mut() {
        let mut changed = false;
        if keyboard_input.just_pressed(KeyCode::Digit1) {
            inventory.active_slot = WeaponSlot::Primary;
            changed = true;
        } else if keyboard_input.just_pressed(KeyCode::Digit2) {
            inventory.active_slot = WeaponSlot::Secondary;
            changed = true;
        } else if keyboard_input.just_pressed(KeyCode::Digit3) {
            inventory.active_slot = WeaponSlot::Melee;
            changed = true;
        } else if keyboard_input.just_pressed(KeyCode::Digit4) {
            inventory.active_slot = WeaponSlot::Equipment;
            changed = true;
        }

        if changed {
            // Despawn old weapon
            for entity in weapon_query.iter() {
                commands.entity(entity).despawn();
            }

            // Spawn new weapon
            if let Some(camera_entity) = camera_query.iter().next() {
                let weapon_entity = spawn_weapon_visual(
                    &mut commands,
                    inventory.active_slot,
                    &asset_server,
                    &weapon_registry,
                    &mut meshes,
                    &mut materials,
                );
                
                commands.entity(weapon_entity).insert(WeaponModel);
                commands.entity(camera_entity).add_child(weapon_entity);
            }
        }
    }
}

