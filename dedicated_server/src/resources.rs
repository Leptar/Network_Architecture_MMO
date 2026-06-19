use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer, GameStream};
use uuid::Uuid;
use shared::*;

#[derive(Resource)]
pub struct ServerConfig {
    pub ip: String,
    pub id: Uuid,
    pub port: u16,
    pub zone: String,
    pub max_players: usize,
    pub status: ServerStatus,
    pub state: ServerState,
    pub orchestrator_ip: String,
    pub orchestrator_port: u16,
    pub broker_ip: String,
    pub broker_port: u16,
}

impl ServerConfig {
    pub fn from_env() -> Self {

        let port = std::env::var("DS_PORT")
            .unwrap_or("0".to_string())
            .parse::<u16>()
            .expect("DS_PORT doit être un nombre valide");

        let zone = std::env::var("DS_ZONE")
            .unwrap_or("zone_A".to_string());

        let max_players = std::env::var("MAX_PLAYERS")
            .unwrap_or("100".to_string())
            .parse::<usize>()
            .expect("MAX_PLAYERS doit être un nombre valide");

        // uuid::Uuid::new_v4() génère un identifiant unique aléatoire
        // ex: "550e8400-e29b-41d4-a716-446655440000"
        let id = Uuid::new_v4();

        let socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
        socket.connect("8.8.8.8:80").unwrap();
        let ip = socket.local_addr().unwrap().ip().to_string();

        let status = ServerStatus::Available;
        
        let state = ServerState::WarmUp;

        let orchestrator_ip = std::env::var("ORCHESTRATOR_IP")
            .unwrap_or(ORCH_IP.to_string());
        
        let orchestrator_port = std::env::var("ORCHESTRATOR_PORT")
            .unwrap_or(ORCH_PORT.to_string())
            .parse::<u16>()
            .expect("ORCH_PORT doit être un nombre valide");
        
        let broker_ip = std::env::var("BROKER_IP")
            .unwrap_or(BROK_IP.to_string());
        
        let broker_port = std::env::var("BROKER_PORT")
            .unwrap_or(BROK_PORT.to_string())
            .parse::<u16>()
            .expect("BROK_PORT doit être un nombre valide");
        
        Self {
            id,
            ip,
            port,
            zone,
            max_players,
            status,
            state,
            orchestrator_ip,
            orchestrator_port,
            broker_ip,
            broker_port,
        }
    }

    pub fn verify_status(&mut self, player_count: usize) {
        self.status = if player_count >= self.max_players {
            ServerStatus::Full
        } else {
            ServerStatus::Available
        };
    }
}

#[derive(Resource)]
pub struct GameSocket {
    pub peer: GamePeer,
    
    pub connection_orch: Option<GameConnection>,
    pub stream_orch: Option<GameStream>,
    
    pub connection_broker: Option<GameConnection>,
    pub stream_broker: Option<GameStream>,
}

#[derive(Resource)]
pub struct HeartbeatTimer(pub Timer);
impl Default for HeartbeatTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(5.0, TimerMode::Repeating))
    }
}