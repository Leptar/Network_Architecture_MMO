mod components;
mod quadtree;
mod spatial_logic;
mod test_file;
mod messages;
mod network;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use std::time::Duration;
use crate::messages::{CrossingAlertMessage, SubscribeMessage, UnsubscribeMessage};
use crate::network::receive_position_updates;
use crate::quadtree::QuadTree;
use crate::test_file::move_fake_player;

fn main() {
    println!("Démarrage du Service Spatial...");

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0),
        )))
        .add_message::<SubscribeMessage>()
        .add_message::<UnsubscribeMessage>()
        .add_message::<CrossingAlertMessage>()
        .add_systems(Startup, (
            network::connect_to_broker, // On lance la connexion réseau
            init_spatial_service, // create quadtree
        ))
        .add_systems(Update, (
            network::receive_position_updates,
            spatial_logic::check_shard_transitions,
            network::flush_network_messages
        ).chain())
        .run();
}

fn init_spatial_service(mut commands: Commands) {
    let bounds = Rect::new(-500.0, -500.0, 500.0, 500.0);
    let max_depth = 5;

    let quad_tree = QuadTree::generate(bounds, max_depth);

commands.insert_resource(quad_tree);
}