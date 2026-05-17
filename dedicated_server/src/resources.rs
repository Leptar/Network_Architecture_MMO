use bevy::prelude::*;
use std::net::SocketAddr;
use std::collections::HashMap;
use game_sockets::{GamePeer, protocols::UdpBackend};

#[derive(Resource)]
pub struct ServerConfig {
    pub id: String,
    pub port: u16,
    pub zone: String,
    pub max_players: usize,
    pub orchestrator_addr: String,
}

#[derive(Resource, Default)]
pub struct PlayerRegistry {
    pub players: HashMap<SocketAddr, String>,
}

// #[derive(Resource)]
// pub struct GameSocket {
//     pub peer: GamePeer,
// }

impl ServerConfig {
    pub fn from_env() -> Self {

        let port = std::env::var("DS_PORT")
            .unwrap_or("7001".to_string())  // si DS_PORT absent → 7001
            .parse::<u16>()                  // convertit le String "7001" en nombre u16
            .expect("DS_PORT doit être un nombre valide");

        let zone = std::env::var("DS_ZONE")
            .unwrap_or("zone_A".to_string());

        let max_players = std::env::var("MAX_PLAYERS")
            .unwrap_or("100".to_string())
            .parse::<usize>()
            .expect("MAX_PLAYERS doit être un nombre valide");

        let orchestrator_addr = std::env::var("ORCH_ADDR")
            .unwrap_or("127.0.0.1:22555".to_string()); // port de l'orchestrateur

        // uuid::Uuid::new_v4() génère un identifiant unique aléatoire
        // ex: "550e8400-e29b-41d4-a716-446655440000"
        let id = uuid::Uuid::new_v4().to_string();

        Self {
            id,
            port,
            zone,
            max_players,
            orchestrator_addr,
        }
    }
}