use bevy::prelude::*;
use game_sockets::{GamePeer, protocols::UdpBackend, GameNetworkEvent};
use crate::resources::*;
use crate::entities::*;
use crate::message::*;

pub fn bind_socket(mut commands: Commands, config: Res<ServerConfig>) {
    let peer = GamePeer::new(UdpBackend::new());
    peer.listen("0.0.0.0", config.port).unwrap();

    // Se connecter à l'orchestrateur
    let parts: Vec<&str> = config.orchestrator_addr.split(':').collect();
    let orch_ip = parts[0];
    let orch_port: u16 = parts[1].parse().unwrap();
    peer.connect(orch_ip, orch_port).unwrap();

    commands.insert_resource(GameSocket { peer });
    println!("Serveur démarré sur le port {}", config.port);
}

pub fn receive_packets(
    mut socket: ResMut<GameSocket>,
    mut player_registry: PlayerRegistry
) {
    while let Ok(Some(event)) = socket.peer.poll() {
        match event {
            GameNetworkEvent::Message { connection, stream, data } => {
                if data.is_empty() {
                    println!("Received empty message from connection id : {}, with stream id : {}", connection.connection_id, stream.stream_id);
                    continue;
                }

                let msg_tag : u8 = data[0];
                let msg_data = &data[1..];
                let mut msg: Box<dyn ShardMessage> = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&msg_data)) {
                    match msg_tag {
                        0x20 => Box::new(HandoffRequest::from_json(json)),
                        0x21 => Box::new(HandoffAccept::from_json(json)),
                        0x22 => Box::new(HandoffReject::from_json(json)),
                        0x23 => Box::new(GhostUpdate::from_json(json)),
                        0x24 => Box::new(HandoffComplete::from_json(json)),
                        _ => return,
                    }
                } else {
                    match msg_tag {
                        0x20 => Box::new(HandoffRequest::from_data(msg_data)),
                        0x21 => Box::new(HandoffAccept::from_data(msg_data)),
                        0x22 => Box::new(HandoffReject::from_data(msg_data)),
                        0x23 => Box::new(GhostUpdate::from_data(msg_data)),
                        0x24 => Box::new(HandoffComplete::from_data(msg_data)),
                        _ => return,
                    }
                };
                
                msg.resolve(&mut player_registry);
            }


            _ => {
                println!("WARNING : Event received does not match any expected event : {:?}", event);
            }
        }
    }
}

pub fn send_heartbeat(
    mut socket: ResMut<GameSocket>,
    config: Res<ServerConfig>,
    registry: Res<PlayerRegistry>,
    mut timer: ResMut<HeartbeatTimer>,
    time: Res<Time>,
) {
    // Avance le timer
    timer.0.tick(time.delta());

    if !timer.0.just_finished() {
        return;
    }

    // Construire le heartbeat
    let heartbeat = shared::Heartbeat {
        id: config.id.clone(),
        ip: config.ip.clone(),
        port: config.port,
        zone: config.zone.clone(),
        player_count: registry.players.len(),
        max_players: config.max_players,
        status: if registry.players.len() < config.max_players { "available".to_string() } else { "full".to_string() }
    };

    let json = serde_json::to_string(&heartbeat).unwrap();
    //println!("Envoi heartbeat : {}", json);

    let udp_socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    udp_socket.send_to(json.as_bytes(), &config.orchestrator_addr).unwrap();
    //println!("Heartbeat envoyé !");
}