use bevy::prelude::*;
use game_sockets::{GamePeer, GameConnection, GameStream};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Clone)]
pub struct DGSNetworkInfo{
    pub connection_dgs: Option<GameConnection>,
    pub stream_dgs: Option<GameStream>,
}

#[derive(Resource)]
pub struct BrokerSocket {
    pub peer: GamePeer,
}

#[derive(Resource, Default)]
pub struct ClientRegistry {
    pub clients: HashMap<u32, GameConnection>, // client_id → connexion
    pub next_id: u32,                          // compteur auto-incrémenté
    pub shards: HashMap<Uuid, DGSNetworkInfo>,
}

#[derive(Resource, Default)]
pub struct SubscriptionMap {
    pub subscriptions: HashMap<String, Vec<u32>>, // topic → liste de client_ids
}

#[derive(Resource, Default)]
pub struct ClientShardMap {
    pub map: HashMap<u32, String>,              // client_id → topic shard
    pub shard_connections: HashMap<String, GameConnection>, // topic → connexion shard
}

#[derive(Resource, Default)]
pub struct AdminRegistry {
    pub admins: HashMap<String, GameConnection>, // ex: "orchestrator" -> Connexion
}