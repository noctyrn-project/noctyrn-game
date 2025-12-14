use bevy::prelude::*;
use crate::weapons::{WeaponSlot, spawn_weapon_visual, WeaponRegistry};
use crate::player::input::Keybinds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwitchState {
    #[default]
    Idle,
    Unequipping,
    Equipping,
}

#[derive(Component)]
pub struct Inventory {
    pub active_slot: WeaponSlot,
    pub target_slot: Option<WeaponSlot>,
    pub previous_slot: Option<WeaponSlot>, // For quick melee return
    pub switch_state: SwitchState,
    pub switch_timer: Timer,
    pub quick_melee_timer: Timer, // To detect hold vs tap
    pub auto_attack: bool,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            active_slot: WeaponSlot::Primary,
            target_slot: None,
            previous_slot: None,
            switch_state: SwitchState::Idle,
            switch_timer: Timer::from_seconds(0.2, TimerMode::Once),
            quick_melee_timer: Timer::from_seconds(0.3, TimerMode::Once),
            auto_attack: false,
        }
    }
}

#[derive(Component)]
pub struct WeaponModel;

pub fn handle_weapon_switching(
    mut commands: Commands,
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    keybinds: Res<Keybinds>,
    mut query: Query<&mut Inventory>,
    mut weapon_query: Query<(Entity, &mut crate::weapons::WeaponRecoil), With<WeaponModel>>,
    camera_query: Query<Entity, With<Camera>>,
    asset_server: Res<AssetServer>,
    weapon_registry: Res<WeaponRegistry>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for mut inventory in query.iter_mut() {
        // Input handling
        let mut target = None;
        if keyboard_input.just_pressed(KeyCode::Digit1) { target = Some(WeaponSlot::Primary); }
        else if keyboard_input.just_pressed(KeyCode::Digit2) { target = Some(WeaponSlot::Secondary); }
        // else if keyboard_input.just_pressed(KeyCode::Digit3) { target = Some(WeaponSlot::Melee); } // Removed standard switch
        // else if keyboard_input.just_pressed(KeyCode::Digit4) { target = Some(WeaponSlot::Equipment); }

        // Quick Melee Logic (Key F)
        if keyboard_input.just_pressed(keybinds.melee) {
            if inventory.active_slot != WeaponSlot::Melee {
                inventory.previous_slot = Some(inventory.active_slot);
                target = Some(WeaponSlot::Melee);
                inventory.quick_melee_timer.reset();
                inventory.auto_attack = true;
            }
        }
        
        if keyboard_input.pressed(keybinds.melee) {
            inventory.quick_melee_timer.tick(time.delta());
        }

        if keyboard_input.just_released(keybinds.melee) {
            // Tap (< 0.2s): Quick Melee (Attack + Return)
            // Hold (> 0.2s): Equip (Stay)
            
            if inventory.quick_melee_timer.elapsed_secs() < 0.2 {
                if let Some(prev) = inventory.previous_slot {
                    inventory.target_slot = Some(prev);
                    inventory.previous_slot = None;
                }
            } else {
                // Stay equipped
                inventory.previous_slot = None;
            }
        }

        // Grenade Logic (Hold G to equip, Release to throw)
        if keyboard_input.just_pressed(keybinds.grenade) {
            if inventory.active_slot != WeaponSlot::Equipment {
                inventory.previous_slot = Some(inventory.active_slot);
                target = Some(WeaponSlot::Equipment);
            }
        }
        
        if keyboard_input.just_released(keybinds.grenade) {
            // Throw logic is handled in fire_weapon, but we need to switch back after throw?
            // Or maybe fire_weapon handles the throw, and we switch back here?
            // If we release G, we want to throw.
            // But we also want to switch back to previous weapon.
            // Let's set target back to previous slot.
            if let Some(prev) = inventory.previous_slot {
                inventory.target_slot = Some(prev);
                inventory.previous_slot = None;
            }
        }

        if let Some(t) = target {
            // Allow switching even if not Idle? No, that breaks animation.
            // But if we want to queue it?
            // If we are Unequipping, we can change target?
            // If we are Equipping, we have to wait until Idle.
            
            if t != inventory.active_slot {
                if inventory.switch_state == SwitchState::Idle {
                    inventory.target_slot = Some(t);
                    inventory.switch_state = SwitchState::Unequipping;
                    inventory.switch_timer.reset();
                } else if inventory.switch_state == SwitchState::Unequipping {
                     // Change target mid-unequip
                     inventory.target_slot = Some(t);
                } else if inventory.switch_state == SwitchState::Equipping {
                    // We are equipping X, but want Y.
                    // We should probably finish equipping X, then unequip X to get Y.
                    // So we queue Y in target_slot?
                    // Our logic below:
                    // "if inventory.switch_timer.finished() { inventory.switch_state = SwitchState::Idle; }"
                    // Then next frame it picks up target_slot.
                    inventory.target_slot = Some(t);
                }
            }
        }

        // State Machine
        match inventory.switch_state {
            SwitchState::Idle => {
                // Ensure weapon is in correct position
                if let Some((_, mut recoil)) = weapon_query.iter_mut().next() {
                    recoil.switch_offset = Vec3::ZERO;
                    recoil.switch_rotation = Vec3::ZERO;
                }
            },
            SwitchState::Unequipping => {
                inventory.switch_timer.tick(time.delta());
                let t = inventory.switch_timer.fraction();
                
                // Animate down
                if let Some((_, mut recoil)) = weapon_query.iter_mut().next() {
                    recoil.switch_offset = Vec3::new(0.0, -0.5 * t, 0.0);
                    recoil.switch_rotation = Vec3::new(-1.0 * t, 0.0, 0.0);
                }

                if inventory.switch_timer.is_finished() {
                    // Despawn old
                    for (entity, _) in weapon_query.iter() {
                        commands.entity(entity).despawn();
                    }
                    
                    // Switch slot
                    if let Some(target) = inventory.target_slot {
                        inventory.active_slot = target;
                    }
                    inventory.target_slot = None;
                    
                    // Spawn new
                    if let Some(camera_entity) = camera_query.iter().next() {
                        let weapon_entity = spawn_weapon_visual(
                            &mut commands,
                            inventory.active_slot,
                            &asset_server,
                            &weapon_registry,
                            &mut meshes,
                            &mut materials,
                        );
                        
                        // Initialize recoil with "Equipping" state to avoid flicker
                        let mut recoil = crate::weapons::WeaponRecoil::default();
                        recoil.switch_offset = Vec3::new(0.0, -0.5, 0.0);
                        recoil.switch_rotation = Vec3::new(-1.0, 0.0, 0.0);

                        commands.entity(weapon_entity).insert((WeaponModel, recoil));
                        commands.entity(camera_entity).add_child(weapon_entity);
                    }

                    inventory.switch_state = SwitchState::Equipping;
                    inventory.switch_timer.reset();
                }
            },
            SwitchState::Equipping => {
                inventory.switch_timer.tick(time.delta());
                let t = inventory.switch_timer.fraction();
                
                // Animate up (reverse of down)
                if let Some((_, mut recoil)) = weapon_query.iter_mut().next() {
                    recoil.switch_offset = Vec3::new(0.0, -0.5 * (1.0 - t), 0.0);
                    recoil.switch_rotation = Vec3::new(-1.0 * (1.0 - t), 0.0, 0.0);
                }

                if inventory.switch_timer.is_finished() {
                    inventory.switch_state = SwitchState::Idle;
                }
            }
        }
    }
}

