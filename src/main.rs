use bevy::prelude::*;
use bevy::pbr::wireframe::WireframePlugin;
use bevy::settings::SettingsPlugin;
use bevy::dev_tools::diagnostics_overlay::DiagnosticsOverlayPlugin;

mod player;
mod world;
mod weapons;
mod gameplay;
mod gamemodes;
mod ui_config;
mod settings;
mod ui_settings;
mod menu;
mod net;
mod defaults;
mod storage;
mod setup;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins(SettingsPlugin::new(defaults::APP_ID))
        .add_plugins(DiagnosticsOverlayPlugin)
        .add_plugins(setup::SetupPlugin)
        .add_plugins(weapons::WeaponsPlugin)
        .add_plugins(player::Player)
        .add_plugins(world::World)
        .add_plugins(gameplay::GameplayPlugin)
        .add_plugins(net::NetworkPlugin)
        .run();
}
