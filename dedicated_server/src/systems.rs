// use bevy::prelude::*;
// use game_sockets::{GamePeer, protocols::UdpBackend};
// use crate::resources::{ServerConfig, GameSocket};
// 
// pub fn bind_socket(mut commands: Commands, config: Res<ServerConfig>) {
// 
//     let peer = GamePeer::new(UdpBackend::new());
// 
//     // "0.0.0.0" signifie "écoute sur toutes les interfaces réseau"
//     // config.port vient de ta variable d'environnement DS_PORT (défaut: 7001)
//     peer.listen("0.0.0.0", config.port).unwrap();
// 
//     // On stocke le peer dans une Resource pour que les autres systèmes y accèdent
//     commands.insert_resource(GameSocket { peer });
// 
//     println!("Serveur démarré sur le port {}", config.port);
// }