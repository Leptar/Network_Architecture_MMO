mod components;
mod quadtree;
mod spatial_logic;
mod test_file;
mod messages;
mod network;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use std::time::Duration;
use bevy::platform::collections::HashMap;
use crate::components::PlayerEntities;
use crate::messages::{BootShardMessage, CrossingAlertMessage, SubscribeMessage, UnsubscribeMessage};
use crate::quadtree::QuadTree;

fn main() {
    println!("Démarrage du Service Spatial...");

    App::new()
        // tickrate : 16 ms
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0),
        )))

        // Message bevy
        .add_message::<SubscribeMessage>()
        .add_message::<UnsubscribeMessage>()
        .add_message::<CrossingAlertMessage>()
        .add_message::<BootShardMessage>()

        // Init
        .init_resource::<PlayerEntities>()
        .add_systems(Startup, (
            network::connect_to_broker, // On lance la connexion réseau
            init_spatial_service,
        ))

        // Loop
        .add_systems(Update, (
            network::receive_position_updates,
            spatial_logic::check_shard_transitions,
            spatial_logic::monitor_shard_capacity,
            network::flush_network_messages
        ).chain())
        .run();
}

fn init_spatial_service(
    mut commands: Commands,
    mut boot_evts : MessageWriter<BootShardMessage>,
) {
    let world_bounds = Rect::from_corners(Vec2::new(0.0, 0.0), Vec2::new(10000.0, 10000.0));
    let max_depth = 2;

    let quad_tree = QuadTree::generate(world_bounds, max_depth);
    println!("[STARTUP] Initialisation du QuadTree mondial...");

    for (shard_id, bounds) in quad_tree.get_leaves(){
        println!("Grille initiale : Planification du Shard {} -> Zone: {:?}", shard_id, bounds);
        boot_evts.write(BootShardMessage{shard_id, bounds});
    }

    commands.insert_resource(quad_tree);
    commands.insert_resource(PlayerEntities {
        map: HashMap::new(),
    });

    println!("[STARTUP] QuadTree mondial enregistré avec succès.");
}