use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer, GameStream};
use shared::ServerStatus;
use crate::entities::*;
use crate::resources::*;

pub trait  InterShardMessage {
    fn resolve(&mut self, registry: &mut PlayerRegistry, server_config: &mut ServerConfig, socket: &GameSocket, connection: GameConnection, stream: GameStream);
    fn to_binary(&self) -> Vec<u8>;
    fn tag(&self) -> u8;
}

//---------------------------------- HandoffRequest ----------------------------------//

pub struct HandoffRequest{
    pub entity_id: u32,
    pub pos: Vec2,
    pub vel: Vec2,
    pub state: [u8; 64],
}

impl InterShardMessage for HandoffRequest {
    fn resolve(&mut self, registry: &mut PlayerRegistry, server_config: &mut ServerConfig, socket: &GameSocket, connection: GameConnection, stream: GameStream) {
        println!("Resolving Handoff Request for {}", self.entity_id);

        if server_config.status == ServerStatus::Available {
            let new_ghost_player = PlayerEntity{
                id: self.entity_id,
                authority: EntityAuthority::Ghost,
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
                println!("Handoff request accepted for entity {}, player entity created with Ghost authority. Sending HandoffAccept message.", self.entity_id);
                send_inter_shards_packet(&socket, Box::new(accept_msg));
            }

        } else {
            //send Handoff reject
            let reject_msg = HandoffReject {
                entity_id: self.entity_id,
                reason: "Server is full".to_string(),
            };

            send_inter_shards_packet(&socket, Box::new(reject_msg));
        }
    }

    fn to_binary(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.entity_id.to_le_bytes());
        buf.extend_from_slice(&self.pos.x.to_le_bytes());
        buf.extend_from_slice(&self.pos.y.to_le_bytes());
        buf.extend_from_slice(&self.vel.x.to_le_bytes());
        buf.extend_from_slice(&self.vel.y.to_le_bytes());
        buf.extend_from_slice(&self.state);
        buf
    }

    fn tag(&self) -> u8 {
        Self::TAG
    }
}

impl HandoffRequest {
    pub const TAG: u8 = 0x20;

    pub fn from_json(json: serde_json::Value) -> Self {
        HandoffRequest {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
            pos: Vec2::new(json["pos"]["x"].as_f64().unwrap() as f32, json["pos"]["y"].as_f64().unwrap() as f32),
            vel: Vec2::new(json["vel"]["x"].as_f64().unwrap() as f32, json["vel"]["y"].as_f64().unwrap() as f32),
            state: json["state"].as_str().unwrap().as_bytes()[..64].try_into().unwrap(),
        }
    }

    pub fn from_binary(data: &[u8]) -> Self {
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
    pub entity_id: u32,
}

impl InterShardMessage for HandoffAccept {
    fn resolve(&mut self, registry: &mut PlayerRegistry, _server_config: &mut ServerConfig, _socket: &GameSocket, _connection: GameConnection, _stream: GameStream) {
        if let Some(player) = registry.players.get_mut(&self.entity_id) {
            if let EntityAuthority::PendingHandoff { .. } = &player.authority {
                println!("Handoff accepted for entity {}, GhostUpdates will begin.", self.entity_id);

                let complete_msg = HandoffComplete { entity_id: self.entity_id };
                send_inter_shards_packet(_socket, Box::new(complete_msg));
            }
        }
    }

    fn to_binary(&self) -> Vec<u8> {
        self.entity_id.to_le_bytes().to_vec()
    }

    fn tag(&self) -> u8 {
        Self::TAG
    }
}

impl HandoffAccept {
    pub const TAG: u8 = 0x21;

    pub fn from_json(json: serde_json::Value) -> Self {
        HandoffAccept {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
        }
    }

    pub fn from_binary(data: &[u8]) -> Self {
        //format entity_id: u32
        HandoffAccept {
            entity_id: u32::from_le_bytes(data[0..4].try_into().unwrap()),
        }
    }
}

//---------------------------------- HandoffReject ----------------------------------//

pub struct HandoffReject{
    pub entity_id: u32,
    pub reason: String,
}

impl InterShardMessage for HandoffReject {
    
    fn resolve(&mut self, registry: &mut PlayerRegistry, _server_config: &mut ServerConfig, _socket: &GameSocket, _connection: GameConnection, _stream: GameStream) {
        if let Some(player) = registry.players.get_mut(&self.entity_id) {
            if let EntityAuthority::PendingHandoff { .. } = &player.authority {
                println!("Handoff rejected for entity {}, reason: {}. Player entity stays Owned.", self.entity_id, self.reason);
                player.authority = EntityAuthority::Owned;

            }
        }
    }

    fn to_binary(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.entity_id.to_le_bytes());
        buf.extend_from_slice(self.reason.as_bytes());
        buf
    }

    fn tag(&self) -> u8 {
        Self::TAG
    }
}

impl HandoffReject {
    pub const TAG: u8 = 0x22;

    pub fn from_json(json: serde_json::Value) -> Self {
        HandoffReject {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
            reason: json["reason"].as_str().unwrap().to_string(),
        }
    }

    pub fn from_binary(data: &[u8]) -> Self {
        //format entity_id: u32, reason: String (rest of the data)
        HandoffReject {
            entity_id: u32::from_le_bytes(data[0..4].try_into().unwrap()),
            reason: String::from_utf8_lossy(&data[4..]).to_string(),
        }
    }
}

//---------------------------------- HandoffComplete ----------------------------------//

pub struct HandoffComplete{
    pub entity_id: u32,
}

impl InterShardMessage for HandoffComplete {
    
    fn resolve(&mut self, registry: &mut PlayerRegistry, _server_config: &mut ServerConfig, _socket: &GameSocket, _connection: GameConnection, _stream: GameStream) {
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

    fn to_binary(&self) -> Vec<u8> {
        self.entity_id.to_le_bytes().to_vec()
    }

    fn tag(&self) -> u8 {
        Self::TAG
    }
}

impl HandoffComplete {
    pub const TAG: u8 = 0x24;

    pub fn from_json(json: serde_json::Value) -> Self {
        HandoffComplete {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
        }
    }

    pub fn from_binary(data: &[u8]) -> Self {
        //format entity_id: u32
        HandoffComplete {
            entity_id: u32::from_le_bytes(data[0..4].try_into().unwrap()),
        }
    }
}

//---------------------------------- GhostUpdate ----------------------------------//

pub struct GhostUpdate{
    pub entity_id: u32,
    pub pos: Vec2,
    pub vel: Vec2,
}

impl InterShardMessage for GhostUpdate {
    fn resolve(&mut self, registry: &mut PlayerRegistry, _server_config: &mut ServerConfig, _socket: &GameSocket, _connection: GameConnection, _stream: GameStream) {
        if let Some(player) = registry.players.get_mut(&self.entity_id) {
            if let EntityAuthority::Ghost { .. } = &player.authority {
                player.position = self.pos;
                player.velocity = self.vel;
            }
        }
    }

    fn to_binary(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.entity_id.to_le_bytes());
        buf.extend_from_slice(&self.pos.x.to_le_bytes());
        buf.extend_from_slice(&self.pos.y.to_le_bytes());
        buf.extend_from_slice(&self.vel.x.to_le_bytes());
        buf.extend_from_slice(&self.vel.y.to_le_bytes());
        buf
    }

    fn tag(&self) -> u8 {
        Self::TAG
    }
}

impl GhostUpdate {
    pub const TAG: u8 = 0x23;

    pub fn from_json(json: serde_json::Value) -> Self {
        GhostUpdate {
            entity_id: json["entity_id"].as_u64().unwrap() as u32,
            pos: Vec2::new(json["pos"]["x"].as_f64().unwrap() as f32, json["pos"]["y"].as_f64().unwrap() as f32),
            vel: Vec2::new(json["vel"]["x"].as_f64().unwrap() as f32, json["vel"]["y"].as_f64().unwrap() as f32),
        }
    }

    pub fn from_binary(data: &[u8]) -> Self {
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
    mut socket: &GameSocket,
    msg: Box<dyn InterShardMessage>,
) {
    let payload = msg.to_binary();
    let mut data = Vec::with_capacity(1 + payload.len());

    data.push(msg.tag());
    data.extend_from_slice(&payload);

    if let (Some(conn), Some(stream)) = (&socket.connection_orch, &socket.stream_orch) {
        let result = socket.peer.send(conn, stream, bytes::Bytes::from(data));
        if result.is_err() {
            println!("Failed to send inter-shard message. Error: {:?}", result.err());
        }
    } else {
        println!("WARNING : Broker connection or stream not established yet");
    }
}

pub fn send_inter_orchestrator_packet(
    mut socket: &GameSocket,
    data: &[u8],
) {
    if let (Some(conn), Some(stream)) = (&socket.connection_orch, &socket.stream_orch) {
        let result = socket.peer.send(conn, stream, bytes::Bytes::from(data.to_vec()));
        if result.is_err() {
            println!("Failed to send inter-orchestrator message. Error: {:?}", result.err());
        }
    } else {
        println!("WARNING : Orchestrator connection or stream not established yet");
    }
}