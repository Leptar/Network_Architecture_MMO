use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum ServerStatus{
    Available,
    Full,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Heartbeat {
    pub id: String,
    pub ip: String,
    pub port: u16,
    pub zone: String,
    pub player_count: usize,
    pub max_players: usize,
    pub status: ServerStatus,
}

//------------ Client Message Receive/Send ------------//
pub const INPUT_LEFT:  u8 = 0b00000001; // bit 0
pub const INPUT_RIGHT: u8 = 0b00000010; // bit 1
pub const INPUT_UP:    u8 = 0b00000100; // bit 2
pub const INPUT_DOWN:  u8 = 0b00001000; // bit 3

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientInput {
    pub client_id: u32,
    pub input: [u8; 16]
}

pub struct Broadcast {
    pub playload_len: u16,
    pub playload: [u8],
}