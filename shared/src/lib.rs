use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum ServerStatus{
    Available,
    Full,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum ServerState{
    WarmUp,
    Idle,
    Running,
    Stopping,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Heartbeat {
    pub id: Uuid,
    pub ip: String,
    pub port: u16,
    pub zone: String,
    pub player_count: usize,
    pub max_players: usize,
    pub status: ServerStatus,
    pub state: ServerState,
}

//------------ Client Message Receive/Send ------------//
pub const INPUT_LEFT:  u8 = 0b00000001; // bit 0
pub const INPUT_RIGHT: u8 = 0b00000010; // bit 1
pub const INPUT_UP:    u8 = 0b00000100; // bit 2
pub const INPUT_DOWN:  u8 = 0b00001000; // bit 3

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInput {
    pub client_id: u32,
    pub sequence_id: u32,
    pub input: [u8; 16]
}

pub struct Broadcast {
    pub playload_len: u16,
    pub playload: [u8],
}

//------------ INFO CONNECTION BROKER ------------//
pub const BROK_IP: &str = "127.0.0.1";
pub const BROK_PORT: u16 = 8080;

//------------ INFO CONNECTION ORCH   ------------//
pub const ORCH_IP: &str = "127.0.0.1";
pub const ORCH_PORT: u16 = 22555;

pub const PAYLOAD_CROSSING_ALERT: u8 = 0x99;

pub const PAYLOAD_BOOT_SHARD: u8 = 0x90;

//------------------------------------------------//

#[derive(Debug, Clone)]
pub struct CrossingAlertData {
    pub client_id: u32,
    pub involved_shards: Vec<u32>,
}

impl CrossingAlertData {
    /// Désérialise les octets bruts reçus du Broker en une structure lisible pour le DGS.
    /// Retourne `None` si le tableau d'octets est invalide ou incomplet.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.is_empty() || data[0] != PAYLOAD_CROSSING_ALERT || data.len() < 6 {
            return None;
        }

        let client_id = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);

        let num_shards = data[5] as usize;

        if data.len() < 6 + (num_shards * 4) {
            return None;
        }

        let mut involved_shards = Vec::with_capacity(num_shards);
        let mut offset = 6;

        for _ in 0..num_shards {
            let shard_id = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            involved_shards.push(shard_id);
            offset += 4;
        }

        Some(Self {
            client_id,
            involved_shards,
        })
    }
}