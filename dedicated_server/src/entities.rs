use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use game_sockets::{GameConnection, GamePeer, GameStream};

#[derive(Debug)]
pub enum EntityAuthority {
    Owned,
    PendingHandoff,
    Ghost,
}

/*************************************/
/*           PLAYER ENTITY           */
/*************************************/

#[derive(Debug)]
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