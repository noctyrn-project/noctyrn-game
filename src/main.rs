use bevy::dev_tools::diagnostics_overlay::DiagnosticsOverlayPlugin;
use bevy::pbr::wireframe::WireframePlugin;
use bevy::prelude::*;
use bevy::settings::SettingsPlugin;

mod branding;
mod defaults;
mod gameplay;
mod gamemodes;
mod maps;
mod menu;
mod net;
mod player;
mod settings;
mod setup;
mod storage;
mod theme;
mod ui_config;
mod ui_settings;
mod weapons;
mod world;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins(SettingsPlugin::new(defaults::APP_ID))
        .add_plugins(DiagnosticsOverlayPlugin)
        .add_plugins(setup::SetupPlugin)
        .add_plugins(branding::BrandingPlugin)
        .add_plugins(weapons::WeaponsPlugin)
        .add_plugins(player::Player)
        .add_plugins(world::World)
        .add_plugins(gameplay::GameplayPlugin)
        .add_plugins(net::NetworkPlugin)
        .run();
}
