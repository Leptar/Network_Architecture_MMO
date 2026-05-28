mod components;
mod quadtree;
mod spatial_logic;

use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use std::time::Duration;
// use shared::*;

fn main() {
    println!("Démarrage du Service Spatial...");

    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0),
        )))
        .add_systems(Startup, init_spatial_service)
        .run();
}

fn init_spatial_service() {
    println!("Initialisation des structures spatiales...");
}