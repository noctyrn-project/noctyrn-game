use bevy::prelude::*;
use bevy::ui::RelativeCursorPosition;
use bevy::ecs::change_detection::DetectChangesMut;
use crate::settings::{GameSettings, save_game_settings};
use crate::player::{Keybinds, save_keybinds, RemapButton};
use serde::Deserialize;
use std::fs;

#[derive(Component)]
pub struct SettingsMenuUi;

#[derive(Component)]
pub struct SettingsContent;

#[derive(Component, PartialEq, Clone, Copy)]
pub enum SettingsTab {
    Gameplay,
    Keybinds,
    Graphics,
    Debug,
    Info,
}

#[derive(Component)]
pub struct TabButton {
    pub tab: SettingsTab,
}

#[derive(Resource, Default)]
pub struct SettingsState {
    pub active_tab: SettingsTab,
}

impl Default for SettingsTab {
    fn default() -> Self {
        Self::Gameplay
    }
}

pub fn spawn_settings_menu(commands: &mut Commands) {
    let root = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)), // Semi-transparent background
        SettingsMenuUi,
        Interaction::default(), // Block clicks
        GlobalZIndex(100), // Ensure on top
    )).id();

    let main_window = commands.spawn((
        Node {
            width: Val::Percent(90.0),
            height: Val::Percent(90.0),
            flex_direction: FlexDirection::Row, // Horizontal Layout
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.95)),
        BorderRadius::all(Val::Px(20.0)),
    )).id();
    commands.entity(root).add_child(main_window);

    // Sidebar (Left)
    let sidebar = commands.spawn((
        Node {
            width: Val::Px(250.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(20.0)),
            border: UiRect::right(Val::Px(2.0)),
            ..default()
        },
        BorderColor::from(Color::srgba(0.3, 0.3, 0.3, 0.5)),
        BackgroundColor(Color::srgba(0.15, 0.15, 0.15, 1.0)),
        BorderRadius::left(Val::Px(20.0)),
    )).id();
    commands.entity(main_window).add_child(sidebar);

    // Sidebar Title
    commands.entity(sidebar).with_children(|parent| {
        parent.spawn((
            Text::new("SETTINGS"),
            TextFont { font_size: 32.0, ..default() },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(40.0)),
                align_self: AlignSelf::Center,
                ..default()
            },
        ));
    });

    spawn_tab_button(commands, sidebar, "Gameplay", SettingsTab::Gameplay);
    spawn_tab_button(commands, sidebar, "Keybinds", SettingsTab::Keybinds);
    spawn_tab_button(commands, sidebar, "Graphics", SettingsTab::Graphics);
    spawn_tab_button(commands, sidebar, "Debug", SettingsTab::Debug);
    spawn_tab_button(commands, sidebar, "Info", SettingsTab::Info);

    // Spacer
    commands.entity(sidebar).with_children(|parent| {
        parent.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });
    });

    // Close Button (Bottom of Sidebar)
    let close_btn = commands.spawn((
        Button,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(50.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            margin: UiRect::top(Val::Px(20.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.8, 0.2, 0.2, 0.8)),
        BorderRadius::all(Val::Px(10.0)),
        CloseSettingsButton,
    )).id();
    commands.entity(sidebar).add_child(close_btn);

    commands.entity(close_btn).with_children(|parent| {
        parent.spawn((
            Text::new("Close"),
            TextFont { font_size: 20.0, ..default() },
            TextColor(Color::WHITE),
        ));
    });

    // Content Area (Right)
    let content_area = commands.spawn((
        Node {
            flex_grow: 1.0,
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(40.0)),
            overflow: Overflow::clip(), // Clip content if it overflows
            ..default()
        },
    )).id();
    commands.entity(main_window).add_child(content_area);

    // Content Container
    let content = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        SettingsContent,
    )).id();
    commands.entity(content_area).add_child(content);
}

#[derive(Component)]
pub struct CloseSettingsButton;

fn spawn_tab_button(commands: &mut Commands, parent: Entity, text: &str, tab: SettingsTab) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(50.0),
                padding: UiRect::left(Val::Px(20.0)), // Left align text
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            BorderRadius::all(Val::Px(10.0)),
            TabButton { tab },
        )).with_children(|parent| {
            parent.spawn((
                Text::new(text),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::WHITE),
            ));
        });
    });
}

pub fn update_settings_menu(
    mut commands: Commands,
    settings_state: Res<SettingsState>,
    mut content_query: Query<(Entity, Option<&Children>), With<SettingsContent>>,
    game_settings: Res<GameSettings>,
    keybinds: Res<Keybinds>,
) {
    if let Some((entity, children)) = content_query.iter_mut().next() {
        // Populate once on first open, and thereafter only when switching tabs.
        let has_children = children.map(|c| !c.is_empty()).unwrap_or(false);
        if !settings_state.is_changed() && has_children {
            return;
        }

        // Clear existing content
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        match settings_state.active_tab {
            SettingsTab::Gameplay => spawn_gameplay_settings(&mut commands, entity, &game_settings),
            SettingsTab::Keybinds => spawn_keybinds_settings(&mut commands, entity, &keybinds),
            SettingsTab::Graphics => spawn_graphics_settings(&mut commands, entity, &game_settings),
            SettingsTab::Debug => spawn_debug_settings(&mut commands, entity, &game_settings),
            SettingsTab::Info => spawn_info_tab(&mut commands, entity),
        }
    }
}

fn spawn_gameplay_settings(commands: &mut Commands, parent: Entity, settings: &GameSettings) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Text::new("Gameplay Settings"),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));
    });

    spawn_toggle(commands, parent, "Toggle Sprint", settings.gameplay.toggle_sprint, SettingAction::ToggleSprint);
    spawn_toggle(commands, parent, "Toggle ADS", settings.gameplay.toggle_ads, SettingAction::ToggleADS);
    spawn_toggle(commands, parent, "Toggle Crouch", settings.gameplay.toggle_crouch, SettingAction::ToggleCrouch);
    spawn_slider(commands, parent, "Sensitivity", settings.gameplay.sensitivity, 0.1, 5.0, 0.1, SettingAction::CycleSensitivity);
}

fn spawn_graphics_settings(commands: &mut Commands, parent: Entity, settings: &GameSettings) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Text::new("Graphics Settings"),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));
    });

    spawn_selector(commands, parent, "Resolution", 
        vec!["Native".to_string(), "1280x720".to_string(), "1920x1080".to_string(), "2560x1440".to_string()], 
        match settings.graphics.resolution {
            [0, 0] => 0,
            [1280, 720] => 1,
            [1920, 1080] => 2,
            [2560, 1440] => 3,
            _ => 0,
        },
        SettingAction::CycleResolution
    );

    spawn_selector(commands, parent, "Texture Quality", 
        vec!["Low".to_string(), "Medium".to_string(), "High".to_string()],
        match settings.graphics.texture_quality.as_str() {
            "Low" => 0,
            "Medium" => 1,
            "High" => 2,
            _ => 0,
        },
        SettingAction::CycleTextureQuality
    );

    spawn_selector(commands, parent, "Shadow Quality", 
        vec!["Low".to_string(), "Medium".to_string(), "High".to_string()],
        match settings.graphics.shadow_quality.as_str() {
            "Low" => 0,
            "Medium" => 1,
            "High" => 2,
            _ => 0,
        },
        SettingAction::CycleShadowQuality
    );

    spawn_slider(commands, parent, "View Distance", settings.graphics.view_distance, 100.0, 2000.0, 100.0, SettingAction::CycleViewDistance);
    spawn_selector(commands, parent, "FPS Cap", 
        vec!["Unlimited".to_string(), "30".to_string(), "60".to_string(), "144".to_string()],
        match settings.graphics.fps_cap {
            0 => 0,
            30 => 1,
            60 => 2,
            144 => 3,
            _ => 0,
        },
        SettingAction::CycleFpsCap
    );
    spawn_slider(commands, parent, "FOV", settings.graphics.fov, 60.0, 120.0, 1.0, SettingAction::CycleFov);
}

fn spawn_cycler(commands: &mut Commands, parent: Entity, label: &str, value: &str, action: SettingAction) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
        )).with_children(|parent| {
            parent.spawn((Text::new(label), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
            
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                BorderRadius::all(Val::Px(5.0)),
                SettingToggle { action }, // Reusing SettingToggle component for cyclers
            )).with_children(|parent| {
                parent.spawn((
                    Text::new(value),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });
    });
}

fn spawn_debug_settings(commands: &mut Commands, parent: Entity, settings: &GameSettings) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Text::new("Debug Settings"),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));
    });

    spawn_toggle(commands, parent, "Show FPS", settings.debug.show_fps, SettingAction::ToggleShowFPS);
    spawn_toggle(commands, parent, "Show Resource Usage", settings.debug.show_resource_usage, SettingAction::ToggleResourceUsage);
    spawn_toggle(commands, parent, "Show Hitboxes", settings.debug.show_hitboxes, SettingAction::ToggleHitboxes);
    spawn_toggle(commands, parent, "Free Cam", settings.debug.free_cam, SettingAction::ToggleFreeCam);
    spawn_toggle(commands, parent, "God Mode", settings.debug.god_mode, SettingAction::ToggleGodMode);
    spawn_toggle(commands, parent, "Infinite Ammo", settings.debug.infinite_ammo, SettingAction::ToggleInfiniteAmmo);
    spawn_toggle(commands, parent, "Show Wireframe", settings.debug.show_wireframe, SettingAction::ToggleWireframe);
}

fn spawn_info_tab(commands: &mut Commands, parent: Entity) {
    let path = "assets/info.toml";
    let info = if let Ok(content) = fs::read_to_string(path) {
        toml::from_str(&content).unwrap_or_else(|_| InfoConfig {
            title: "Fearlyss".to_string(),
            version: "Unknown".to_string(),
            description: "Failed to load info.".to_string(),
            credits: vec![],
        })
    } else {
        InfoConfig {
            title: "Fearlyss".to_string(),
            version: "Unknown".to_string(),
            description: "Info file not found.".to_string(),
            credits: vec![],
        }
    };

    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Text::new(&info.title),
            TextFont { font_size: 40.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(10.0)), ..default() },
        ));

        parent.spawn((
            Text::new(format!("v{}", info.version)),
            TextFont { font_size: 20.0, ..default() },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));

        parent.spawn((
            Text::new(&info.description),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(30.0)), ..default() },
        ));

        parent.spawn((
            Text::new("Credits:"),
            TextFont { font_size: 24.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(10.0)), ..default() },
        ));

        for credit in info.credits {
            parent.spawn((
                Text::new(credit),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
                Node { margin: UiRect::bottom(Val::Px(5.0)), ..default() },
            ));
        }
    });
}

fn spawn_keybinds_settings(commands: &mut Commands, parent: Entity, keybinds: &Keybinds) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Text::new("Keybindings"),
            TextFont { font_size: 30.0, ..default() },
            TextColor(Color::WHITE),
            Node { margin: UiRect::bottom(Val::Px(20.0)), ..default() },
        ));
        
        let key_bindings = [
            ("Move Forward", keybinds.move_forward),
            ("Move Backward", keybinds.move_backward),
            ("Move Left", keybinds.move_left),
            ("Move Right", keybinds.move_right),
            ("Jump", keybinds.jump),
            ("Sprint", keybinds.sprint),
            ("Crouch", keybinds.crouch),
            ("Reload", keybinds.reload),
            ("Interact", keybinds.interact),
            ("Grenade", keybinds.grenade),
            ("Melee", keybinds.melee),
            ("Stats", keybinds.stats),
            ("Pause", keybinds.pause),
        ];

        for (action, key) in key_bindings {
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            )).with_children(|parent| {
                parent.spawn((Text::new(action), TextFont { font_size: 18.0, ..default() }, TextColor(Color::WHITE)));
                
                // Remap Button
                parent.spawn((
                    Button,
                    Node {
                        width: Val::Px(150.0),
                        height: Val::Px(30.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                    RemapButton { action: action.to_string() },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(format!("{:?}", key)),
                        TextFont { font_size: 18.0, ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
            });
        }

        // Mouse Buttons (Display only for now)
        let mouse_bindings = [
            ("Shoot", keybinds.shoot),
            ("ADS", keybinds.ads),
        ];

        for (action, button) in mouse_bindings {
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            )).with_children(|parent| {
                parent.spawn((Text::new(action), TextFont { font_size: 18.0, ..default() }, TextColor(Color::WHITE)));
                parent.spawn((Text::new(format!("{:?}", button)), TextFont { font_size: 18.0, ..default() }, TextColor(Color::srgb(0.8, 0.8, 0.8))));
            });
        }
    });
}

#[derive(Component)]
pub struct SettingToggle {
    pub action: SettingAction,
}

#[derive(Clone, Copy)]
pub enum SettingAction {
    ToggleSprint,
    ToggleADS,
    ToggleCrouch,
    ToggleShowFPS,
    ToggleResourceUsage,
    ToggleHitboxes,
    ToggleFreeCam,
    ToggleGodMode,
    ToggleInfiniteAmmo,
    ToggleWireframe,
    CycleResolution,
    CycleTextureQuality,
    CycleShadowQuality,
    CycleViewDistance,
    CycleFpsCap,
    CycleSensitivity,
    CycleFov,
}

fn spawn_toggle(commands: &mut Commands, parent: Entity, label: &str, value: bool, action: SettingAction) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
        )).with_children(|parent| {
            parent.spawn((Text::new(label), TextFont { font_size: 20.0, ..default() }, TextColor(Color::WHITE)));
            
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(60.0),
                    height: Val::Px(30.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(if value { Color::srgb(0.2, 0.8, 0.2) } else { Color::srgb(0.8, 0.2, 0.2) }),
                BorderRadius::all(Val::Px(15.0)),
                SettingToggle { action },
            )).with_children(|parent| {
                parent.spawn((
                    Text::new(if value { "ON" } else { "OFF" }),
                    TextFont { font_size: 16.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });
    });
}

pub fn handle_settings_interaction(
    mut commands: Commands,
    mut interaction_query: Query<(Entity, &Interaction, &mut BackgroundColor, Option<&TabButton>, Option<&CloseSettingsButton>, Option<&SettingToggle>, Option<&Slider>, Option<&Selector>), With<Button>>,
    mut settings_state: ResMut<SettingsState>,
    mut game_settings: ResMut<GameSettings>,
    mut menu_query: Query<Entity, With<SettingsMenuUi>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
) {


    for (entity, interaction, mut bg_color, tab_button, close_button, toggle, _slider, selector) in interaction_query.iter_mut() {
        let is_hovered = *interaction == Interaction::Hovered;

        if let Some(tab) = tab_button {
            let is_active = tab.tab == settings_state.active_tab;
            if is_active {
                *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 1.0));
            } else if is_hovered {
                *bg_color = BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.05));
            } else {
                *bg_color = BackgroundColor(Color::NONE);
            }
        } else if close_button.is_some() {
             if is_hovered {
                *bg_color = BackgroundColor(Color::srgba(0.9, 0.3, 0.3, 1.0));
             } else {
                *bg_color = BackgroundColor(Color::srgba(0.8, 0.2, 0.2, 0.8));
             }
        } else if let Some(toggle) = toggle {
             let value = match toggle.action {
                SettingAction::ToggleSprint => game_settings.gameplay.toggle_sprint,
                SettingAction::ToggleADS => game_settings.gameplay.toggle_ads,
                SettingAction::ToggleCrouch => game_settings.gameplay.toggle_crouch,
                SettingAction::ToggleShowFPS => game_settings.debug.show_fps,
                SettingAction::ToggleResourceUsage => game_settings.debug.show_resource_usage,
                SettingAction::ToggleHitboxes => game_settings.debug.show_hitboxes,
                SettingAction::ToggleFreeCam => game_settings.debug.free_cam,
                SettingAction::ToggleGodMode => game_settings.debug.god_mode,
                SettingAction::ToggleInfiniteAmmo => game_settings.debug.infinite_ammo,
                SettingAction::ToggleWireframe => game_settings.debug.show_wireframe,
                _ => false,
            };
            let base_color = if value { Color::srgb(0.2, 0.8, 0.2) } else { Color::srgb(0.8, 0.2, 0.2) };
            if is_hovered {
                *bg_color = BackgroundColor(base_color.mix(&Color::WHITE, 0.2));
            } else {
                *bg_color = BackgroundColor(base_color);
            }
        } else if selector.is_some() {
             if is_hovered {
                *bg_color = BackgroundColor(Color::srgb(0.2, 0.2, 0.2));
             } else {
                *bg_color = BackgroundColor(Color::srgb(0.1, 0.1, 0.1));
             }
        }

        if *interaction == Interaction::Pressed {
            // Click-based interactions (Just Pressed)
            if mouse_button_input.just_pressed(MouseButton::Left) {
                if let Some(tab_button) = tab_button {
                    settings_state.active_tab = match tab_button.tab {
                        SettingsTab::Gameplay => SettingsTab::Gameplay,
                        SettingsTab::Keybinds => SettingsTab::Keybinds,
                        SettingsTab::Graphics => SettingsTab::Graphics,
                        SettingsTab::Debug => SettingsTab::Debug,
                        SettingsTab::Info => SettingsTab::Info,
                    };
                } else if close_button.is_some() {
                    for entity in menu_query.iter() {
                        commands.entity(entity).despawn();
                    }
                } else if let Some(toggle) = toggle {
                    match toggle.action {
                        SettingAction::ToggleSprint => game_settings.gameplay.toggle_sprint = !game_settings.gameplay.toggle_sprint,
                        SettingAction::ToggleADS => game_settings.gameplay.toggle_ads = !game_settings.gameplay.toggle_ads,
                        SettingAction::ToggleCrouch => game_settings.gameplay.toggle_crouch = !game_settings.gameplay.toggle_crouch,
                        SettingAction::ToggleShowFPS => game_settings.debug.show_fps = !game_settings.debug.show_fps,
                        SettingAction::ToggleResourceUsage => game_settings.debug.show_resource_usage = !game_settings.debug.show_resource_usage,
                        SettingAction::ToggleHitboxes => game_settings.debug.show_hitboxes = !game_settings.debug.show_hitboxes,
                        SettingAction::ToggleFreeCam => game_settings.debug.free_cam = !game_settings.debug.free_cam,
                        SettingAction::ToggleGodMode => game_settings.debug.god_mode = !game_settings.debug.god_mode,
                        SettingAction::ToggleInfiniteAmmo => game_settings.debug.infinite_ammo = !game_settings.debug.infinite_ammo,
                        SettingAction::ToggleWireframe => game_settings.debug.show_wireframe = !game_settings.debug.show_wireframe,
                        _ => {}
                    }
                    save_game_settings(&game_settings);
                    settings_state.set_changed();
                } else if let Some(selector) = selector {
                    // Next Option
                    match selector.action {
                        SettingAction::CycleResolution => {
                            let current_res = game_settings.graphics.resolution;
                            game_settings.graphics.resolution = match current_res {
                                [0, 0] => [1280, 720],
                                [1280, 720] => [1920, 1080],
                                [1920, 1080] => [2560, 1440],
                                _ => [0, 0],
                            };
                        },
                        SettingAction::CycleTextureQuality => {
                            let next_idx = (selector.current_index + 1) % selector.options.len();
                            game_settings.graphics.texture_quality = selector.options[next_idx].clone();
                        },
                        SettingAction::CycleShadowQuality => {
                            let next_idx = (selector.current_index + 1) % selector.options.len();
                            game_settings.graphics.shadow_quality = selector.options[next_idx].clone();
                        },
                        SettingAction::CycleFpsCap => {
                            let next_idx = (selector.current_index + 1) % selector.options.len();
                            game_settings.graphics.fps_cap = match next_idx {
                                0 => 0,
                                1 => 30,
                                2 => 60,
                                3 => 144,
                                _ => 0,
                            };
                        },
                        _ => {}
                    }
                    save_game_settings(&game_settings);
                    settings_state.set_changed();
                }
            } else if mouse_button_input.just_pressed(MouseButton::Right) {
                if let Some(selector) = selector {
                    // Previous Option
                    match selector.action {
                        SettingAction::CycleResolution => {
                            let current_res = game_settings.graphics.resolution;
                            game_settings.graphics.resolution = match current_res {
                                [0, 0] => [2560, 1440],
                                [1280, 720] => [0, 0],
                                [1920, 1080] => [1280, 720],
                                [2560, 1440] => [1920, 1080],
                                _ => [0, 0],
                            };
                        },
                        SettingAction::CycleTextureQuality => {
                            let prev_idx = (selector.current_index + selector.options.len() - 1) % selector.options.len();
                            game_settings.graphics.texture_quality = selector.options[prev_idx].clone();
                        },
                        SettingAction::CycleShadowQuality => {
                            let prev_idx = (selector.current_index + selector.options.len() - 1) % selector.options.len();
                            game_settings.graphics.shadow_quality = selector.options[prev_idx].clone();
                        },
                        SettingAction::CycleFpsCap => {
                            let prev_idx = (selector.current_index + selector.options.len() - 1) % selector.options.len();
                            game_settings.graphics.fps_cap = match prev_idx {
                                0 => 0,
                                1 => 30,
                                2 => 60,
                                3 => 144,
                                _ => 0,
                            };
                        },
                        _ => {}
                    }
                    save_game_settings(&game_settings);
                    settings_state.set_changed();
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct InfoConfig {
    title: String,
    version: String,
    description: String,
    credits: Vec<String>,
}

#[derive(Component)]
pub struct Slider {
    pub action: SettingAction,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub value_text_entity: Entity,
    pub knob_entity: Entity,
}

#[derive(Component)]
pub struct SliderKnob;

#[derive(Component)]
pub struct SliderTrack;

#[derive(Component, Default)]
pub struct SliderDragState {
    pub start_value: f32,
}

#[derive(Component)]
pub struct Selector {
    pub action: SettingAction,
    pub options: Vec<String>,
    pub current_index: usize,
}

#[derive(Component)]
pub struct SelectorOption {
    pub index: usize,
}

fn spawn_slider(commands: &mut Commands, parent: Entity, label: &str, value: f32, min: f32, max: f32, step: f32, action: SettingAction) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            margin: UiRect::bottom(Val::Px(15.0)),
            ..default()
        }).with_children(|parent| {
            let mut value_text_entity = Entity::PLACEHOLDER;
            
            // Label + Value
            parent.spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                margin: UiRect::bottom(Val::Px(5.0)),
                ..default()
            }).with_children(|parent| {
                parent.spawn((Text::new(label), TextFont { font_size: 18.0, ..default() }, TextColor(Color::WHITE)));
                value_text_entity = parent.spawn((Text::new(format!("{:.1}", value)), TextFont { font_size: 18.0, ..default() }, TextColor(Color::WHITE))).id();
            });

            // Track
            let mut knob_entity = Entity::PLACEHOLDER;
            
            parent.spawn((
                Button, // Make it interactable
                SliderTrack,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(20.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::FlexStart,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                BorderRadius::all(Val::Px(10.0)),
                RelativeCursorPosition::default(),
                SliderDragState::default(),
            )).with_children(|parent| {
                // Knob
                let percent = if max > min { (value - min) / (max - min) } else { 0.0 };
                let percent = percent.clamp(0.0, 1.0);
                
                knob_entity = parent.spawn((
                    Node {
                        width: Val::Px(20.0),
                        height: Val::Px(20.0),
                        position_type: PositionType::Absolute,
                        left: Val::Percent(percent * 100.0), 
                        ..default()
                    },
                    BackgroundColor(Color::WHITE),
                    BorderRadius::all(Val::Px(10.0)),
                    SliderKnob,
                )).id();
            }).insert(Slider { action, min, max, step, value_text_entity, knob_entity })
            .observe(on_slider_drag_start)
            .observe(on_slider_drag);
        });
    });
}

fn spawn_selector(commands: &mut Commands, parent: Entity, label: &str, options: Vec<String>, current_index: usize, action: SettingAction) {
    commands.entity(parent).with_children(|parent| {
        parent.spawn(Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            margin: UiRect::bottom(Val::Px(15.0)),
            ..default()
        }).with_children(|parent| {
            parent.spawn((Text::new(label), TextFont { font_size: 18.0, ..default() }, TextColor(Color::WHITE)));
            
            // Box
            parent.spawn((
                Button,
                Node {
                    width: Val::Px(250.0),
                    height: Val::Px(40.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::vertical(Val::Px(10.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                BorderColor::from(Color::WHITE),
                BorderRadius::all(Val::Px(5.0)),
                Selector { action, options: options.clone(), current_index },
            )).with_children(|parent| {
                parent.spawn((
                    Text::new(options[current_index].clone()),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::WHITE),
                ));
            });

            // Dots
            parent.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                ..default()
            }).with_children(|parent| {
                for (i, _) in options.iter().enumerate() {
                    let is_selected = i == current_index;
                    let size = if is_selected { 12.0 } else { 8.0 };
                    let color = if is_selected { Color::WHITE } else { Color::srgb(0.5, 0.5, 0.5) };
                    
                    parent.spawn((
                        Node {
                            width: Val::Px(size),
                            height: Val::Px(size),
                            margin: UiRect::horizontal(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(color),
                        BorderRadius::all(Val::Px(size / 2.0)),
                    ));
                }
            });
        });
    });
}

fn on_slider_drag_start(
    trigger: On<Pointer<DragStart>>,
    mut slider_query: Query<(&GlobalTransform, &ComputedNode, &Slider, &mut SliderDragState)>,
    mut game_settings: ResMut<GameSettings>,
    mut settings_state: ResMut<SettingsState>,
    mut text_query: Query<&mut Text>,
    mut node_query: Query<&mut Node>,
) {
    let entity = trigger.entity;
    if let Ok((transform, computed_node, slider, mut drag_state)) = slider_query.get_mut(entity) {
        // If clicking the knob, don't jump the value, just start dragging from current
        if trigger.target() == slider.knob_entity {
             let current_value = match slider.action {
                SettingAction::CycleSensitivity => game_settings.gameplay.sensitivity,
                SettingAction::CycleViewDistance => game_settings.graphics.view_distance,
                SettingAction::CycleFov => game_settings.graphics.fov,
                _ => slider.min,
            };
            drag_state.start_value = current_value;
        } else if let Some(hit_position) = trigger.event.hit.position {
            let new_value = calculate_value_from_position(hit_position, transform, computed_node, slider);
            drag_state.start_value = new_value;
            apply_slider_value(new_value, slider, &mut game_settings, &mut settings_state, &mut text_query, &mut node_query);
        }
    }
}

fn on_slider_drag(
    trigger: On<Pointer<Drag>>,
    mut slider_query: Query<(&GlobalTransform, &ComputedNode, &Slider, &SliderDragState)>,
    mut game_settings: ResMut<GameSettings>,
    mut settings_state: ResMut<SettingsState>,
    mut text_query: Query<&mut Text>,
    mut node_query: Query<&mut Node>,
) {
    let entity = trigger.entity;
    if let Ok((transform, computed_node, slider, drag_state)) = slider_query.get_mut(entity) {
        let (scale, _, _) = transform.to_scale_rotation_translation();
        let slider_width = computed_node.size().x * scale.x;
        
        let delta_ratio = trigger.event.distance.x / slider_width;
        let value_range = slider.max - slider.min;
        let value_delta = delta_ratio * value_range;
        
        let raw_value = drag_state.start_value + value_delta;
        let steps = ((raw_value - slider.min) / slider.step).round();
        let new_value = (slider.min + steps * slider.step).clamp(slider.min, slider.max);
        
        apply_slider_value(new_value, slider, &mut game_settings, &mut settings_state, &mut text_query, &mut node_query);
    }
}

fn calculate_value_from_position(
    cursor_world_pos: Vec3,
    transform: &GlobalTransform,
    computed_node: &ComputedNode,
    slider: &Slider
) -> f32 {
    let (scale, _, translation) = transform.to_scale_rotation_translation();
    let slider_width = computed_node.size().x * scale.x;
    let slider_start_x = translation.x - slider_width / 2.0;
    let relative_x = cursor_world_pos.x - slider_start_x;
    let ratio = (relative_x / slider_width).clamp(0.0, 1.0);
    let raw_value = slider.min + (slider.max - slider.min) * ratio;
    let steps = ((raw_value - slider.min) / slider.step).round();
    (slider.min + steps * slider.step).clamp(slider.min, slider.max)
}

fn apply_slider_value(
    new_value: f32,
    slider: &Slider,
    game_settings: &mut GameSettings,
    settings_state: &mut ResMut<SettingsState>,
    text_query: &mut Query<&mut Text>,
    node_query: &mut Query<&mut Node>,
) {
    match slider.action {
        SettingAction::CycleSensitivity => game_settings.gameplay.sensitivity = new_value,
        SettingAction::CycleViewDistance => game_settings.graphics.view_distance = new_value,
        SettingAction::CycleFov => game_settings.graphics.fov = new_value,
        _ => {}
    }
    save_game_settings(game_settings);
    settings_state.set_changed();

    if let Ok(mut text) = text_query.get_mut(slider.value_text_entity) {
        text.0 = format!("{:.1}", new_value);
    }
    if let Ok(mut node) = node_query.get_mut(slider.knob_entity) {
        let percent = if slider.max > slider.min { (new_value - slider.min) / (slider.max - slider.min) } else { 0.0 };
        node.left = Val::Percent(percent * 100.0);
    }
}
