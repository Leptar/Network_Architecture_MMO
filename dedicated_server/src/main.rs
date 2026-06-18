mod resources;
mod systems;
mod entities;
mod message;

use bevy::prelude::*;
use resources::ServerConfig;
use resources::HeartbeatTimer;
use entities::PlayerRegistry;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(ServerConfig::from_env())
        .init_resource::<PlayerRegistry>()
        .init_resource::<HeartbeatTimer>()
        .add_systems(Startup, systems::bind_socket)
        .add_systems(Update, (systems::receive_packets, systems::send_heartbeat, systems::send_ghost_update, systems::publish).chain())
        .run();
}