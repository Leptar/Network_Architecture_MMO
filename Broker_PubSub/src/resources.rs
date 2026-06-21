use bevy::prelude::*;
use game_sockets::{GamePeer, GameConnection, GameStream};
use std::collections::HashMap;

#[derive(Resource)]
pub struct BrokerSocket {
    pub peer: GamePeer,
}

//------------------- INFO CLIENT -------------------//
pub struct ClientNetworkInfo {
    pub broker_to_client_connexion: GameConnection,
    pub broker_to_client_stream: GameStream,
    pub broker_to_server_connexion: Option<GameConnection>,
    pub broker_to_server_stream: Option<GameStream>,
}

#[derive(Resource, Default)]
pub struct ClientRegistry {
    pub next_id: u32,                          // compteur auto-incrémenté
    pub clients: HashMap<u32, ClientNetworkInfo>,
}

//------------------- INFO DGS -------------------//
#[derive(Clone)]
pub struct DGSNetworkInfo{
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
    pub connection_dgs: Option<GameConnection>,
    pub stream_dgs: Option<GameStream>,
}
#[derive(Resource, Default)]
pub struct ShardMap {
    pub shard_connections: HashMap<u32, DGSNetworkInfo>, // topic → connexion shard
}

pub struct AdminNetworkInfo {
    pub connection_admin: GameConnection,
    pub stream_admin: GameStream,
}
#[derive(Resource, Default)]
pub struct AdminRegistry {
    pub admins: HashMap<String, AdminNetworkInfo>, // ex: "orchestrator" -> Connexion
}

#[derive(Resource, Default)]
pub struct SubscriptionMap {
    pub subscriptions: HashMap<String, Vec<u32>>, // topic → liste de client_ids
}