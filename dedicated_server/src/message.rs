use std::cmp::PartialEq;
use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer, GameStream};
use shared::ServerSatus;
use crate::entities::*;
use crate::resources::*;

pub trait  InterShardMessage {
    fn resolve(&mut self, registry: &mut PlayerRegistry, server_config: &mut ServerConfig, socket: &GameSocket, connection: GameConnection, stream: GameStream);
}

//---------------------------------- HandoffRequest ----------------------------------//

pub struct HandoffRequest{
    entity_id: u32,
    pos: Vec2,
    vel: Vec2,
    state: [u8; 64],
}

impl InterShardMessage for HandoffRequest {
    fn resolve(&mut self, registry: &mut PlayerRegistry, server_config: &mut ServerConfig, socket: &GameSocket, connection: GameConnection, stream: GameStream) {
        println!("Resolving Handoff Request for {}", self.entity_id);

        if server_config.status == ServerSatus::Available {
            let new_ghost_player = PlayerEntity{
                id: self.entity_id,
                authority: EntityAuthority::Ghost { source_shard : OtherShardConnectionInfo {
                    connection,
                    stream,
                }},
                position: self.pos,
                rotation: 0.0,
                velocity: self.vel,
            };

            registry.players.insert(self.entity_id, new_ghost_player);

            server_config.verify_status(registry.players.len());

            //send Handoff accepte
            let accept_msg = HandoffAccept {
                entity_id: self.entity_id,
            };

            if let Some(player) = registry.players.get(&self.entity_id) {
                if let EntityAuthority::Ghost { source_shard } = &player.authority {
                    send_inter_shards_packet(&socket.peer, Box::new(accept_msg), &source_shard.connection, &source_shard.stream);
                }
            }

        } else {
            //send Handoff reject
            let reject_msg = HandoffReject {
                entity_id: self.entity_id,
                reason: "Server is full".to_string(),
            };

            send_inter_shards_packet(&socket.peer, Box::new(reject_msg), &connection, &stream);
        }
    }
}

impl HandoffRequest {
    pub fn from_json(json: serde_json::Value) -> Self {
        HandoffRequest {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
            pos: Vec2::new(json["pos"]["x"].as_f64().unwrap() as f32, json["pos"]["y"].as_f64().unwrap() as f32),
            vel: Vec2::new(json["vel"]["x"].as_f64().unwrap() as f32, json["vel"]["y"].as_f64().unwrap() as f32),
            state: json["state"].as_str().unwrap().as_bytes()[..64].try_into().unwrap(),
        }
    }

    pub fn from_data(data: &[u8]) -> Self {
        //format entity_id: u32, pos: Vec2, vel: Vec2, state: [u8; 64]
        HandoffRequest {
            entity_id: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            pos: Vec2::new(
                f32::from_le_bytes(data[4..8].try_into().unwrap()),
                f32::from_le_bytes(data[8..12].try_into().unwrap())
            ),
            vel: Vec2::new(
                f32::from_le_bytes(data[12..16].try_into().unwrap()),
                f32::from_le_bytes(data[16..20].try_into().unwrap())
            ),
            state: data[20..84].try_into().unwrap(),
        }
    }
}

//---------------------------------- HandoffAccept ----------------------------------//

pub struct HandoffAccept {
    entity_id: u32,
}

impl InterShardMessage for HandoffAccept {
    fn resolve(&mut self, registry: &mut PlayerRegistry, server_config: &mut ServerConfig, _socket: &GameSocket, connection: GameConnection, stream: GameStream) {
        if let Some(player) = registry.players.get_mut(&self.entity_id) {
            if let EntityAuthority::PendingHandoff { .. } = &player.authority {
                println!("Handoff accepted for entity {}, GhostUpdates will begin.", self.entity_id);
            }
        }
    }
}

impl HandoffAccept {
    pub fn from_json(json: serde_json::Value) -> Self {
        HandoffAccept {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
        }
    }

    pub fn from_data(data: &[u8]) -> Self {
        //format entity_id: u32
        HandoffAccept {
            entity_id: u32::from_le_bytes(data[0..4].try_into().unwrap()),
        }
    }
}

//---------------------------------- HandoffReject ----------------------------------//

pub struct HandoffReject{
    entity_id: u32,
    reason: String,
}

impl InterShardMessage for HandoffReject {
    fn resolve(&mut self, registry: &mut PlayerRegistry, server_config: &mut ServerConfig, _socket: &GameSocket, _connection: GameConnection, _stream: GameStream) {
        if let Some(player) = registry.players.get_mut(&self.entity_id) {
            if let EntityAuthority::PendingHandoff { .. } = &player.authority {
                println!("Handoff rejected for entity {}, reason: {}. Player entity stays Owned.", self.entity_id, self.reason);
                player.authority = EntityAuthority::Owned;

            }
        }
    }
}

impl HandoffReject {
    pub fn from_json(json: serde_json::Value) -> Self {
        HandoffReject {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
            reason: json["reason"].as_str().unwrap().to_string(),
        }
    }

    pub fn from_data(data: &[u8]) -> Self {
        //format entity_id: u32, reason: String (rest of the data)
        HandoffReject {
            entity_id: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            reason: String::from_utf8_lossy(&data[4..]).to_string(),
        }
    }
}

//---------------------------------- HandoffComplete ----------------------------------//

pub struct HandoffComplete{
    entity_id: u32,
}

impl InterShardMessage for HandoffComplete {
    fn resolve(&mut self, registry: &mut PlayerRegistry, server_config: &mut ServerConfig, _socket: &GameSocket, _connection: GameConnection, _stream: GameStream) {
        if let Some(player) = registry.players.get_mut(&self.entity_id) {
            if let EntityAuthority::Ghost { .. } = &player.authority {
                println!("Handoff complete for entity {}. Player entity is now Owned on this shard.", self.entity_id);
                player.authority = EntityAuthority::Owned;
            } else if let EntityAuthority::PendingHandoff { .. } = &player.authority {
                println!("Handoff complete for entity {}, but it was still pending. Removing player entity.", self.entity_id);
                registry.players.remove(&self.entity_id);
            }
        }
    }
}

impl HandoffComplete {
    pub fn from_json(json: serde_json::Value) -> Self {
        HandoffComplete {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
        }
    }

    pub fn from_data(data: &[u8]) -> Self {
        //format entity_id: u32
        HandoffComplete {
            entity_id: u32::from_le_bytes(data[0..4].try_into().unwrap()),
        }
    }
}

//---------------------------------- GhostUpdate ----------------------------------//

pub struct GhostUpdate{
    entity_id: u32,
    pos: Vec2,
    vel: Vec2,
}

impl InterShardMessage for GhostUpdate {
    fn resolve(&mut self, registry: &mut PlayerRegistry, server_config: &mut ServerConfig, _socket: &GameSocket, _connection: GameConnection, _stream: GameStream) {
        if let Some(player) = registry.players.get_mut(&self.entity_id) {
            if let EntityAuthority::Ghost { .. } = &player.authority {
                player.position = self.pos;
                player.velocity = self.vel;
            }
        }
    }
}

impl GhostUpdate {
    pub fn from_json(json: serde_json::Value) -> Self {
        GhostUpdate {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
            pos: Vec2::new(json["pos"]["x"].as_f64().unwrap() as f32, json["pos"]["y"].as_f64().unwrap() as f32),
            vel: Vec2::new(json["vel"]["x"].as_f64().unwrap() as f32, json["vel"]["y"].as_f64().unwrap() as f32),
        }
    }

    pub fn from_data(data: &[u8]) -> Self {
        //format entity_id: u32, pos: Vec2, vel: Vec2
        GhostUpdate {
            entity_id: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            pos: Vec2::new(
                f32::from_le_bytes(data[4..8].try_into().unwrap()),
                f32::from_le_bytes(data[8..12].try_into().unwrap())
            ),
            vel: Vec2::new(
                f32::from_le_bytes(data[12..16].try_into().unwrap()),
                f32::from_le_bytes(data[16..20].try_into().unwrap())
            ),
        }
    }
}

//---------------------------------- Fonction ----------------------------------//

pub fn send_inter_shards_packet(
    socket: &GamePeer,
    msg: Box<dyn InterShardMessage>,
    connection: &GameConnection,
    stream: &GameStream,
) {
    let json = serde_json::to_string(&msg).unwrap();
    let mut data = vec![0u8; 1 + json.len()];
    data[0] = match msg.as_ref() {
        m if m.is::<HandoffRequest>() => 0x20,
        m if m.is::<HandoffAccept>() => 0x21,
        m if m.is::<HandoffReject>() => 0x22,
        m if m.is::<GhostUpdate>() => 0x23,
        m if m.is::<HandoffComplete>() => 0x24,
        _ => return,
    };
    data[1..].copy_from_slice(json.as_bytes());

    let result = socket.send(connection.connection_id, stream.stream_id, &data);

    if result.is_err() {
        println!("Failed to send inter-shard message to connection id : {}, stream id : {}. Error: {:?}", connection.connection_id, stream.stream_id, result.err());
    }
}