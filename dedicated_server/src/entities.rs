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
        //prend le premier input de la liste et l'applique au pos du joueur
            let direction = input[0];
            let speed = 5.0; // Vitesse de déplacement du joueur
            
            match direction {
                0 => self.position.y += speed, // Haut
                1 => self.position.y -= speed, // Bas
                2 => self.position.x -= speed, // Gauche
                3 => self.position.x += speed, // Droite
                _ => (), // Aucun mouvement
            }
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

impl PlayerRegistry {
    pub fn update_player_input(&mut self, client_id: u32, input: [u8; 16]) {
        if let Some(player) = self.players.get_mut(&client_id) {
            player.interpret_player_input(input);
        }
    }
}