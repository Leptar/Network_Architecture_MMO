use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ServerSatus{
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
    pub status: ServerSatus,
}