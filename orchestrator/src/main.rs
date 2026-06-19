mod resources;
mod systems;

use bevy::prelude::*;
use resources::RedisConnection;
use crate::resources::ScalerLoopTimer;

fn main() {
    let client = redis::Client::open("redis://127.0.0.1:6379").expect("Failed to create Redis client");

    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(RedisConnection { client })
        .init_resource::<ScalerLoopTimer>()
        .add_systems(Startup, systems::startup_orchestrator)
        .add_systems(Update, (systems::receive_packets, systems::scaler_loop).chain())
        .run();
}