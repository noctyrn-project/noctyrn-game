use bevy::prelude::*;
use bevy::settings::*;
use bevy::input::keyboard::NativeKeyCode;

use crate::defaults;
use crate::settings::GameSettings;
use crate::player::{PhysicalTranslation, Velocity};

#[derive(Resource, SettingsGroup, Reflect, Debug, Clone)]
#[reflect(Resource, SettingsGroup, Default)]
pub struct Keybinds {
    pub move_forward: KeyCode,
    pub move_backward: KeyCode,
    pub move_left: KeyCode,
    pub move_right: KeyCode,
    pub jump: KeyCode,
    pub sprint: KeyCode,
    pub crouch: KeyCode,
    pub interact: KeyCode,
    pub grenade: KeyCode,
    pub melee: KeyCode,
    pub stats: KeyCode,
    pub pause: KeyCode,
    pub shoot: MouseButton,
    pub ads: MouseButton,
    pub reload: KeyCode,
    pub prone: KeyCode,
    pub lean_left: KeyCode,
    pub lean_right: KeyCode,
    pub scoreboard: KeyCode,
}

impl Default for Keybinds {
    fn default() -> Self {
        let keys = defaults::default_keybinds();
        let mice = defaults::default_mouse_binds();
        Self {
            move_forward: find_key("move_forward", &keys),
            move_backward: find_key("move_backward", &keys),
            move_left: find_key("move_left", &keys),
            move_right: find_key("move_right", &keys),
            jump: find_key("jump", &keys),
            sprint: find_key("sprint", &keys),
            crouch: find_key("crouch", &keys),
            interact: find_key("interact", &keys),
            grenade: find_key("grenade", &keys),
            melee: find_key("melee", &keys),
            stats: find_key("stats", &keys),
            pause: find_key("pause", &keys),
            reload: find_key("reload", &keys),
            prone: find_key("prone", &keys),
            lean_left: find_key("lean_left", &keys),
            lean_right: find_key("lean_right", &keys),
            scoreboard: find_key("scoreboard", &keys),
            shoot: find_mouse("shoot", &mice),
            ads: find_mouse("ads", &mice),
        }
    }
}

fn find_key(name: &str, pairs: &[(String, KeyCode)]) -> KeyCode {
    pairs.iter().find(|(n, _)| n == name).map(|(_, k)| *k).unwrap_or(KeyCode::Space)
}

fn find_mouse(name: &str, pairs: &[(String, MouseButton)]) -> MouseButton {
    pairs.iter().find(|(n, _)| n == name).map(|(_, m)| *m).unwrap_or(MouseButton::Left)
}

#[derive(Debug, Component, Clone, Copy, PartialEq, Default)]
pub struct AccumulatedInput {
    pub movement: Vec3,
    pub raw_movement: Vec2,
    pub jump: bool,
    pub sprint: bool,
    pub crouch: bool,
    pub fire: bool,
    pub aim: bool,
    pub stats: bool,
    pub pause: bool,
    pub prone: bool,
    pub lean_left: bool,
    pub lean_right: bool,
}

pub fn accumulate_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    keybinds: Res<Keybinds>,
    game_settings: Res<GameSettings>,
    player: Single<(&mut AccumulatedInput, &mut PlayerToggleState)>,
    camera: Single<&Transform, With<super::MainCamera>>,
    terminal_open: Res<super::WeaponTerminalOpen>,
    pause_open: Res<super::PauseMenuOpen>,
    chat_open: Res<crate::menu::chat::ChatOpen>,
) {
    if terminal_open.0 || pause_open.0 || chat_open.0 { return; }
    let (mut input, mut toggle_state) = player.into_inner();
    let mut movement = Vec3::ZERO;
    if keyboard_input.pressed(keybinds.move_forward) {
        movement.y += 1.0;
    }
    if keyboard_input.pressed(keybinds.move_backward) {
        movement.y -= 1.0;
    }
    if keyboard_input.pressed(keybinds.move_right) {
        movement.x += 1.0;
    }
    if keyboard_input.pressed(keybinds.move_left) {
        movement.x -= 1.0;
    }

    let forward = camera.forward();
    let right = camera.right();
    
    let forward_flat = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right_flat = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    let mut wish_dir = forward_flat * movement.y + right_flat * movement.x;

    if wish_dir.length_squared() > 1.0 {
        wish_dir = wish_dir.normalize();
    }

    input.movement = wish_dir;
    input.raw_movement = Vec2::new(movement.x, movement.y);
    input.jump = keyboard_input.pressed(keybinds.jump);

    if game_settings.gameplay.toggle_ads {
        if mouse_input.just_pressed(keybinds.ads) {
            toggle_state.ads = !toggle_state.ads;
        }
        input.aim = toggle_state.ads;
    } else {
        input.aim = mouse_input.pressed(keybinds.ads);
        toggle_state.ads = input.aim;
    }
    
    if game_settings.gameplay.toggle_sprint {
        if keyboard_input.just_pressed(keybinds.sprint) {
            toggle_state.sprint = !toggle_state.sprint;
        }
        if movement.y <= 0.0 || input.aim {
            toggle_state.sprint = false;
        }
        input.sprint = toggle_state.sprint;
    } else {
        input.sprint = keyboard_input.pressed(keybinds.sprint) && movement.y > 0.0 && !input.aim;
        toggle_state.sprint = input.sprint;
    }
    
    if game_settings.gameplay.toggle_crouch {
        if keyboard_input.just_pressed(keybinds.crouch) {
            toggle_state.crouch = !toggle_state.crouch;
        }
        input.crouch = toggle_state.crouch;
    } else {
        input.crouch = keyboard_input.pressed(keybinds.crouch);
        toggle_state.crouch = input.crouch;
    }

    input.fire = mouse_input.pressed(keybinds.shoot);
    input.stats = keyboard_input.just_pressed(keybinds.stats);
    input.pause = keyboard_input.just_pressed(keybinds.pause);
    input.prone = keyboard_input.just_pressed(keybinds.prone);
    input.lean_left = keyboard_input.pressed(keybinds.lean_left);
    input.lean_right = keyboard_input.pressed(keybinds.lean_right);
}

pub fn clear_input(mut input: Single<&mut AccumulatedInput>) {
    **input = AccumulatedInput::default();
}

impl Keybinds {
    pub fn set(&mut self, action: &str, key: KeyCode) {
        match action {
            "Move Forward" => self.move_forward = key,
            "Move Backward" => self.move_backward = key,
            "Move Left" => self.move_left = key,
            "Move Right" => self.move_right = key,
            "Jump" => self.jump = key,
            "Sprint" => self.sprint = key,
            "Crouch" => self.crouch = key,
            "Interact" => self.interact = key,
            "Grenade" => self.grenade = key,
            "Melee" => self.melee = key,
            "Stats" => self.stats = key,
            "Pause" => self.pause = key,
            "Reload" => self.reload = key,
            "Prone" => self.prone = key,
            "Lean Left" => self.lean_left = key,
            "Lean Right" => self.lean_right = key,
            "Scoreboard" => self.scoreboard = key,
            _ => warn!("Unknown keybind action: {}", action),
        }
    }

    pub fn get(&self, action: &str) -> KeyCode {
        match action {
            "Move Forward" => self.move_forward,
            "Move Backward" => self.move_backward,
            "Move Left" => self.move_left,
            "Move Right" => self.move_right,
            "Jump" => self.jump,
            "Sprint" => self.sprint,
            "Crouch" => self.crouch,
            "Interact" => self.interact,
            "Grenade" => self.grenade,
            "Melee" => self.melee,
            "Stats" => self.stats,
            "Pause" => self.pause,
            "Reload" => self.reload,
            "Prone" => self.prone,
            "Lean Left" => self.lean_left,
            "Lean Right" => self.lean_right,
            "Scoreboard" => self.scoreboard,
            _ => KeyCode::Unidentified(NativeKeyCode::Unidentified),
        }
    }
}

#[derive(Component, Default)]
pub struct PlayerToggleState {
    pub sprint: bool,
    pub crouch: bool,
    pub ads: bool,
}

#[derive(Resource, Default)]
pub struct InputSequence(pub u32);

pub fn send_player_input(
    udp: Res<crate::net::udp::UdpClient>,
    input: Single<&AccumulatedInput>,
    camera: Single<&Transform, With<super::MainCamera>>,
    mut seq: ResMut<InputSequence>,
    rt: Res<crate::net::TokioRuntime>,
    mut pred_buf: ResMut<crate::net::prediction::PredictionBuffer>,
    player: Single<(&PhysicalTranslation, &Velocity), With<super::LocalPlayer>>,
) {
    if !udp.is_connected() {
        return;
    }

    let accumulated = *input;
    let yaw = camera.rotation.to_euler(bevy::math::EulerRot::YXZ).0;
    let pitch = camera.rotation.to_euler(bevy::math::EulerRot::YXZ).1;

    let movement = [
        accumulated.movement.x,
        accumulated.movement.y,
        accumulated.movement.z,
    ];

    let mut actions = noctyrn_shared::protocol::PlayerActions::empty();
    actions.set_if(noctyrn_shared::protocol::PlayerActions::JUMP, accumulated.jump);
    actions.set_if(noctyrn_shared::protocol::PlayerActions::CROUCH, accumulated.crouch);
    actions.set_if(noctyrn_shared::protocol::PlayerActions::SPRINT, accumulated.sprint);
    actions.set_if(noctyrn_shared::protocol::PlayerActions::SHOOT, accumulated.fire);

    let seq_num = seq.0;
    seq.0 += 1;

    let sess_id = *udp.session_id.lock().unwrap();
    let player_id = *udp.player_id.lock().unwrap();

    let Some(session_id) = sess_id else { return };
    let Some(pid) = player_id else { return };

    let input_packet = noctyrn_shared::protocol::PlayerInput {
        sequence: seq_num,
        timestamp: 0.0,
        session_id,
        player_id: pid,
        movement,
        look_yaw: yaw,
        look_pitch: pitch,
        actions,
    };

    // Record predicted state before sending (store input for replay).
    let (phys, vel) = player.into_inner();
    pred_buf.push([phys.x, phys.y, phys.z], [vel.x, vel.y, vel.z], &input_packet);

    let udp_clone = udp.clone();
    rt.0.spawn(async move {
        let _ = udp_clone.send_input(&input_packet).await;
    });
}

/// Send a `ShotFired` UDP packet when the player fires.
///
/// Runs on each tick the mouse button is held, but the server implements
/// fire-rate limiting so rapid clicks are handled correctly.
pub fn send_shot_fired(
    mouse_input: Res<ButtonInput<MouseButton>>,
    udp: Res<crate::net::udp::UdpClient>,
    camera: Single<&Transform, With<super::MainCamera>>,
    rt: Res<crate::net::TokioRuntime>,
    inventory: Single<&crate::player::inventory::Inventory>,
    registry: Res<crate::weapons::WeaponRegistry>,
) {
    if !udp.is_connected() {
        return;
    }
    if !mouse_input.just_pressed(MouseButton::Left) {
        return;
    }

    let sess_id = match *udp.session_id.lock().unwrap() {
        Some(id) => id,
        None => return,
    };
    let player_id = match *udp.player_id.lock().unwrap() {
        Some(id) => id,
        None => return,
    };

    // Camera forward direction is -Z in Bevy; transform.forward() returns -Z.
    let forward = camera.forward().as_vec3();
    let origin = camera.translation;
    let direction = [forward.x, forward.y, forward.z];

    // Get the actual weapon_id from the active inventory slot.
    let weapon_id = registry
        .by_slot
        .get(&inventory.active_slot)
        .and_then(|ids| ids.first())
        .cloned()
        .unwrap_or_else(|| "colt_m4a1".to_string());

    let shot = noctyrn_shared::protocol::ShotFired::new(
        player_id,
        sess_id,
        [origin.x, origin.y, origin.z],
        direction,
        weapon_id,
        0.0, // timestamp
    );

    let udp_clone = udp.clone();
    rt.0.spawn(async move {
        let _ = udp_clone.send_shot(&shot).await;
    });
}
