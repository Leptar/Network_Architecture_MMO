use bevy::prelude::*;
use game_sockets::{GamePeer, protocols::UdpBackend, GameNetworkEvent};
use crate::resources::*;
use crate::entities::*;
use crate::message::*;
use shared::*;

pub fn bind_socket(mut commands: Commands, config: Res<ServerConfig>) {
    // Se connecter à l'orchestrateur
    let peer_orch = GamePeer::new(UdpBackend::new());
    peer_orch.listen("0.0.0.0", config.port).unwrap();
    
    let parts: Vec<&str> = config.orchestrator_addr.split(':').collect();
    let orch_ip = parts[0];
    let orch_port: u16 = parts[1].parse().unwrap();
    peer_orch.connect(orch_ip, orch_port).unwrap();
    
    //connecte au broker
    let peer_broker = GamePeer::new(UdpBackend::new());
    peer_broker.listen("0.0.0.0", config.port).unwrap();
    
    peer_orch.connect(BROK_IP, BROK_PORT).unwrap();

    commands.insert_resource(GameSocket { peer_orch, peer_broker });
    println!("Serveur démarré sur le port {}", config.port);
}

pub fn receive_packets(
    mut socket: ResMut<GameSocket>,
    mut player_registry: ResMut<PlayerRegistry>,
    mut server_config: ResMut<ServerConfig>
) {
    //receive packet from broker :
    while let Ok(Some(event)) = socket.peer_broker.poll() {
        match event {
            GameNetworkEvent::Message { connection, stream, data } => {
                if data.is_empty() {
                    println!("Received empty message from connection id : {}, with stream id : {}", connection.connection_id, stream.stream_id);
                    continue;
                }

                let msg_tag: u8 = data[0];
                let msg_data = &data[1..];
                let mut msg: Box<dyn InterShardMessage> = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&msg_data)) {
                    match msg_tag {
                        HandoffRequest::tag() => Box::new(HandoffRequest::from_json(json)),
                        HandoffAccept::tag() => Box::new(HandoffAccept::from_json(json)),
                        HandoffReject::tag() => Box::new(HandoffReject::from_json(json)),
                        GhostUpdate::tag() => Box::new(GhostUpdate::from_json(json)),
                        HandoffComplete::tag() => Box::new(HandoffComplete::from_json(json)),
                        _ => {
                            println!("Received message with unknown tag : {}, from connection id : {}, with stream id : {}", msg_tag, connection.connection_id, stream.stream_id);
                            return;
                        },
                    }
                } else {
                    match msg_tag {
                        HandoffRequest::tag() => Box::new(HandoffRequest::from_binary(msg_data)),
                        HandoffAccept::tag() => Box::new(HandoffAccept::from_binary(msg_data)),
                        HandoffReject::tag() => Box::new(HandoffReject::from_binary(msg_data)),
                        GhostUpdate::tag() => Box::new(GhostUpdate::from_binary(msg_data)),
                        HandoffComplete::tag() => Box::new(HandoffComplete::from_binary(msg_data)),
                        _ => {
                            println!("Received message with unknown tag : {}, from connection id : {}, with stream id : {}", msg_tag, connection.connection_id, stream.stream_id);
                            return;
                        },
                    }
                };

                msg.resolve(&mut player_registry, &mut server_config, &socket, connection, stream);
            }
            
            GameNetworkEvent::Connected(connection ) => {
                println!("Connected to broker with connection id : {}", connection.connection_id);
            }

            _ => {
                println!("WARNING : Event received does not match any expected event : {:?}, from broker", event);
            }
        }
    }

    //receive packet from orchestrator :
    while let Ok(Some(event)) = socket.peer_orch.poll() {
        match event {
            GameNetworkEvent::Message { connection, stream, data } => {
                println!("Received message from orchestrator, connection id : {}, stream id : {}, data length : {}", connection.connection_id, stream.stream_id, data.len());
                //handle message from orchestrator
            }
            
            GameNetworkEvent::Connected(connection ) => {
                println!("Connected to orchestrator with connection id : {}", connection.connection_id);
            }
            _ => {
                println!("WARNING : Event received does not match any expected event : {:?}, from orchestrator", event);
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
        status: config.status,
    };

    let json = serde_json::to_string(&heartbeat).unwrap();
    //println!("Envoi heartbeat : {}", json);

    let udp_socket = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    udp_socket.send_to(json.as_bytes(), &config.orchestrator_addr).unwrap();
    //println!("Heartbeat envoyé !");
}

pub fn send_ghost_update(
    mut socket: Res<GameSocket>,
    registry: Res<PlayerRegistry>,
) {
    for player in registry.players.values() {
        if let EntityAuthority::PendingHandoff { target_shard } = &player.authority {
            let ghost_update = GhostUpdate {
                entity_id: player.id,
                pos: player.position,
                vel: player.velocity,
            };

            let msg = Box::new(ghost_update);

            send_inter_shards_packet(
                &socket.peer,
                msg,
                &target_shard.connection,
                &target_shard.stream,
            );
        }
    }
}

pub fn publish(
    registry: Res<PlayerRegistry>
) {
    
}