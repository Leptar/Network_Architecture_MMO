use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer, GameStream};
use shared::*;
use uuid::Uuid;

#[derive(Clone)]
pub struct DGSNetworkInfo{
    pub connection_dgs: Option<GameConnection>,
    pub stream_dgs: Option<GameStream>,
}

#[derive(Resource)]
pub struct GameSocket {
    pub peer: GamePeer,

    pub dgs_network_info_dictionary: std::collections::HashMap<Uuid, DGSNetworkInfo>,

    pub connection_broker: Option<GameConnection>,
    pub stream_broker: Option<GameStream>,
}

#[derive(Resource)]
pub struct RedisConnection {
    pub client: redis::Client,
}

#[derive(Resource)]
pub struct ScalerLoopTimer {
    pub timer: Timer,
}

const CHECK_TIME_SERVERS_AVAILABLE: f32 = 10.0;
impl Default for ScalerLoopTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(CHECK_TIME_SERVERS_AVAILABLE, TimerMode::Repeating),
        }
    }
}