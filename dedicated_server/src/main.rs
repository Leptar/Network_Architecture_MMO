mod resources;
mod systems;

use bevy::prelude::*;
use resources::ServerConfig;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(ServerConfig::from_env())
        .add_systems(Startup, systems::bind_socket)
        .add_systems(Update, (systems::receive_packets, systems::send_heartbeat).chain())
        .run();
}