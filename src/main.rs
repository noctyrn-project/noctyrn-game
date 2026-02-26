use bevy::prelude::*;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};

mod player;
mod world;
mod weapons;
mod gameplay;
mod ui_config;
mod settings;
mod ui_settings;
mod menu;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(WireframePlugin::default())
        .add_plugins(weapons::WeaponsPlugin)
        .add_plugins(player::Player)
        .add_plugins(world::World)
        .add_plugins(gameplay::GameplayPlugin)
        .insert_resource(ui_config::load_ui_config())
        .insert_resource(settings::load_game_settings())
        .run();
}
