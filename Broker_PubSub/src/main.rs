mod resources;
mod systems;

use bevy::prelude::*;
use resources::ClientRegistry;
use resources::SubscriptionMap;
use resources::ShardMap;
use crate::resources::AdminRegistry;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .init_resource::<AdminRegistry>()
        .init_resource::<ClientRegistry>()
        .init_resource::<SubscriptionMap>()
        .init_resource::<ShardMap>()
        .add_systems(Startup, systems::bind_socket)
        .add_systems(Update, systems::receive_messages)
        .run();
}