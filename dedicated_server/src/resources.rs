use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer, GameStream};
use shared::*;
use crate::message::{GhostUpdate, HandoffAccept, HandoffComplete, HandoffReject, HandoffRequest, InterShardMessage};

#[derive(Resource)]
pub struct ServerConfig {
    pub ip: String,
    pub id: String,
    pub port: u16,
    pub zone: String,
    pub max_players: usize,
    pub status: ServerSatus,
    pub orchestrator_addr: String,
}

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
            .unwrap_or("host.docker.internal:22555".to_string());

        // uuid::Uuid::new_v4() génère un identifiant unique aléatoire
        // ex: "550e8400-e29b-41d4-a716-446655440000"
        let id = uuid::Uuid::new_v4().to_string();

        let socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
        socket.connect("8.8.8.8:80").unwrap();
        let ip = socket.local_addr().unwrap().ip().to_string();

        let status = ServerSatus::Available;
        
        Self {
            id,
            ip,
            port,
            zone,
            max_players,
            status,
            orchestrator_addr,
        }
    }

    pub fn verify_status(&mut self, player_count: usize) {
        self.status = if player_count >= self.max_players {
            ServerSatus::Full
        } else {
            ServerSatus::Available
        };
    }
}

#[derive(Resource)]
pub struct GameSocket {
    pub peer: GamePeer,
}

#[derive(Resource)]
pub struct HeartbeatTimer(pub Timer);
impl Default for HeartbeatTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
    }
}