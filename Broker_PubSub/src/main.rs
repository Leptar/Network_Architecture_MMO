mod resources;
mod systems;

use bevy::prelude::*;
use resources::ClientRegistry;
use resources::SubscriptionMap;
use resources::ClientShardMap;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .init_resource::<ClientRegistry>()
        .init_resource::<SubscriptionMap>()
        .init_resource::<ClientShardMap>()
        .add_systems(Startup, systems::bind_socket)
        .add_systems(Update, systems::receive_messages)
        .run();
}