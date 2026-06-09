use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use game_sockets::{GameConnection, GameStream};

#[derive(Debug, Clone, PartialEq)]
pub struct OtherShardConnectionInfo{
    pub connection: GameConnection,
    pub stream: GameStream,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntityAuthority {
    Owned,
    PendingHandoff { target_shard: OtherShardConnectionInfo },
    Ghost { source_shard: OtherShardConnectionInfo },
}

/*************************************/
/*           PLAYER ENTITY           */
/*************************************/

#[derive(Debug, Clone)]
pub struct PlayerEntity {
    pub id      : u32,
    pub authority: EntityAuthority,
    pub position: Vec2,
    pub rotation: f32,
    pub velocity: Vec2,
}

impl PlayerEntity {
    pub fn interpret_player_input(&mut self, input: [u8; 16]) {
        //TODO : TRAITEMENT DES INPUT
    }
}

#[derive(Resource)]
pub struct PlayerRegistry {
    pub players: HashMap<u32, PlayerEntity>,
}

impl Default for PlayerRegistry {
    fn default() -> Self {
        PlayerRegistry {
            players: HashMap::new(),
        }
    }
}