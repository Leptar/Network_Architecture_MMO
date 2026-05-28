mod components;
mod quadtree;
mod spatial_logic;
mod test_file;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use std::time::Duration;
use crate::test_file::move_fake_player;
// use shared::*;

fn main() {
    println!("Démarrage du Service Spatial...");

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0),
        )))
        .add_systems(Startup, test_file::setup_simulation)
        .add_systems(Update, (move_fake_player, spatial_logic::check_shard_transitions))
        .run();
}

fn init_spatial_service() {
    println!("Initialisation des structures spatiales...");
}