use bevy::platform::collections::HashMap;
use bevy::prelude::*;

#[derive(Debug, Clone)]
enum EntityAuthority {
    Owned,
    PendingHandoff { target_shard_addr: String },
    Ghost { source_shard_addr: String },
}

/*************************************/
/*           PLAYER ENTITY           */
/*************************************/

#[derive(Debug, Clone)]
struct PlayerEntity {
    id      : u32,
    authority: EntityAuthority,
    position: Vec2,
    rotation: f32,
    velocity: Vec2,
}

impl PlayerEntity {
    pub fn new(id: u32) -> Self {
        PlayerEntity {
            id,
            authority: EntityAuthority::Owned,
            position: Vec2::ZERO,
            rotation: 0.0,
            velocity: Vec2::ZERO,
        }
    }

    pub fn interpret_player_input(&mut self, input: [u8; 16]) {
        //TODO : TRAITEMENT DES INPUT
    }
}

#[derive(Resource)]
pub struct PlayerRegistry {
    pub players: HashMap<u32, PlayerEntity>,
}