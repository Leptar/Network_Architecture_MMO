use bevy::prelude::*;
use game_sockets::{GamePeer, GameNetworkEvent, GameStreamReliability};
use game_sockets::protocols::QuicBackend;
use crate::resources::*;
use crate::entities::*;
use crate::message::*;
use shared::*;
use std::net::ToSocketAddrs;

pub fn bind_socket(mut commands: Commands, config: Res<ServerConfig>) {
    // Démarre 1 socket qui vas avoir plusieurs connection (0: Orchestrator, 1:Broker)
    let peer = GamePeer::new(QuicBackend::new());
    peer.listen("0.0.0.0", config.port).unwrap();

    //Connection to Orchestrator
    let resolved_addr_orch = format!("{}:{}", &config.orchestrator_ip, config.orchestrator_port)
        .to_socket_addrs()
        .expect("Failed to resolve address")
        .next()
        .expect("No address found");

    peer.connect(&resolved_addr_orch.ip().to_string(), resolved_addr_orch.port()).unwrap();

    println!("Attempting to connect to Orchestrator on {}:{}...", resolved_addr_orch.ip(), resolved_addr_orch.port());

    //Connection to Broker
    let resolved_addr_brok = format!("{}:{}", &config.broker_ip, config.broker_port)
        .to_socket_addrs()
        .expect("Failed to resolve address")
        .next()
        .expect("No address found");
    peer.connect(&resolved_addr_brok.ip().to_string(), resolved_addr_brok.port()).unwrap();

    println!("Attempting to connect to Broker on {}:{}...", resolved_addr_brok.ip(), resolved_addr_brok.port());


    commands.insert_resource(GameSocket { peer, connection_orch: None, stream_orch: None, connection_broker: None, stream_broker: None });
    println!("Game socket initialized and listening on port {}", config.port);
}

pub fn receive_packets(
    mut socket: ResMut<GameSocket>,
    mut player_registry: ResMut<PlayerRegistry>,
    mut server_config: ResMut<ServerConfig>
) {
    //receive packet from broker :
    while let Ok(Some(event)) = socket.peer.poll() {
        match event {
            GameNetworkEvent::Message { connection, stream, data } => {
                if data.is_empty() {
                    println!("Received empty message from connection id : {}, with stream id : {}", connection.connection_id, stream.stream_id);
                    continue;
                }

                //Message de Orchestrator
                if let Some(orch_conn) = &socket.connection_orch {
                    if (connection.connection_id == orch_conn.connection_id) {
                        let msg_tag: u8 = data[0];
                        let msg_data = &data[1..];
                        
                        let mut msg: Box<dyn InterShardMessage> = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&String::from_utf8_lossy(&msg_data)) {
                            match msg_tag {
                                HandoffRequest::TAG => Box::new(HandoffRequest::from_json(json)),
                                HandoffAccept::TAG => Box::new(HandoffAccept::from_json(json)),
                                HandoffReject::TAG => Box::new(HandoffReject::from_json(json)),
                                GhostUpdate::TAG => Box::new(GhostUpdate::from_json(json)),
                                HandoffComplete::TAG => Box::new(HandoffComplete::from_json(json)),
                                _ => {
                                    println!("Received message with unknown tag : {}, from connection id : {}, with stream id : {}", msg_tag, connection.connection_id, stream.stream_id);
                                    return;
                                },
                            }
                        } else {
                            match msg_tag {
                                HandoffRequest::TAG => Box::new(HandoffRequest::from_binary(msg_data)),
                                HandoffAccept::TAG => Box::new(HandoffAccept::from_binary(msg_data)),
                                HandoffReject::TAG => Box::new(HandoffReject::from_binary(msg_data)),
                                GhostUpdate::TAG => Box::new(GhostUpdate::from_binary(msg_data)),
                                HandoffComplete::TAG => Box::new(HandoffComplete::from_binary(msg_data)),
                                _ => {
                                    println!("Received message with unknown tag : {}, from connection id : {}, with stream id : {}", msg_tag, connection.connection_id, stream.stream_id);
                                    return;
                                },
                            }
                        };
                        
                        msg.resolve(&mut player_registry, &mut server_config, &socket, connection, stream.clone());
                    }
                }

                //Message du Broker
                if let Some(broker_conn) = &socket.connection_broker {
                    if (connection.connection_id == broker_conn.connection_id) {
                        println!("Received message from Orchestrator with connection id : {}, with stream id : {}, data : {:?}", connection.connection_id, stream.stream_id, data);

                        //TODO: handle message area to own
                        server_config.state = ServerState::Running;
                    }
                } else {
                    println!("WARNING : Orchestrator connection not established yet, received message from connection id : {}, with stream id : {}, data : {:?}", connection.connection_id, stream.stream_id, data);
                }
            }

            GameNetworkEvent::Connected(connection ) => {
                if(socket.connection_orch == None) {
                    println!("Connected to Orchestrator with connection id : {}", connection.connection_id);
                    socket.connection_orch = Some(connection);

                    //Creation du stream de communication avec l'orchestrator
                    socket.peer.create_stream(connection, GameStreamReliability::Reliable).unwrap();
                } else if(socket.connection_broker == None) {
                    println!("Connected to Broker with connection id : {}", connection.connection_id);
                    socket.connection_broker = Some(connection);

                    //Creation du stream de communication avec le broker
                    socket.peer.create_stream(connection, GameStreamReliability::Unreliable).unwrap();
                }
            }

            GameNetworkEvent::StreamCreated(connection, stream) => {
                //Stream de Orchestrator
                if let Some(orch_conn) = &socket.connection_orch {
                    if (connection.connection_id == orch_conn.connection_id) {
                        socket.stream_orch = Some(stream.clone());
                        println!("Stream created with Orchestrator, connection id : {}, stream id : {}", connection.connection_id, stream.stream_id);
                    }
                } else {
                    println!("WARNING : Orchestrator connection not established yet, received message from connection id : {}, with stream id : {}", connection.connection_id, stream.stream_id);
                }
                
                //Stream du Broker
                if let Some(broker_conn) = &socket.connection_broker {
                    if (connection.connection_id == broker_conn.connection_id) {
                        socket.stream_broker = Some(stream.clone());
                        println!("Stream created with Broker, connection id : {}, stream id : {}", connection.connection_id, stream.stream_id);

                        //Allumer et connecter à tout le monde passage en idle
                        server_config.state = ServerState::Idle;
                    }
                } else {
                    println!("WARNING : Broker connection not established yet, received message from connection id : {}, with stream id : {}", connection.connection_id, stream.stream_id);
                }
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
        status: config.status,
        state: config.state,
    };

    let json = serde_json::to_string(&heartbeat).unwrap();
    //println!("Envoi heartbeat : {}", json);

    send_inter_orchestrator_packet(
        &socket,
        json.as_bytes(),
    );
    //println!("Heartbeat envoyé !");
}

pub fn send_ghost_update(
    socket: Res<GameSocket>,
    registry: Res<PlayerRegistry>,
) {
    for player in registry.players.values() {
        if let EntityAuthority::PendingHandoff = &player.authority {
            let ghost_update = GhostUpdate {
                entity_id: player.id,
                pos: player.position,
                vel: player.velocity,
            };

            let msg = Box::new(ghost_update);

            send_inter_shards_packet(
                &socket,
                msg,
            );
        }
    }
}

pub fn publish(
    registry: Res<PlayerRegistry>,
) {
    //println!("--- Player Registry ---");
}