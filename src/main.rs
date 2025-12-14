use bevy::prelude::*;

mod player;
mod world;
mod weapons;
mod gameplay;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(weapons::WeaponsPlugin)
        .add_plugins(player::Player)
        .add_plugins(world::World)
        .add_plugins(gameplay::GameplayPlugin)
        .run();
}
