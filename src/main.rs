use bevy::prelude::*;

mod player;
mod world;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(player::Player)
        .add_plugins(world::World)
        .run();
}
