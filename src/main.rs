use bevy::prelude::*;

mod player;
mod world;
mod weapons;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(weapons::WeaponsPlugin)
        .add_plugins(player::Player)
        .add_plugins(world::World)
        .run();
}
