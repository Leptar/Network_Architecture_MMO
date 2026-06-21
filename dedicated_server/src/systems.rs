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
    println!("Game socket initialized and listening on port {}", config.port);

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
                        
                        let mut msg: Box<dyn InterShardMessage> = match msg_tag {
                            HandoffRequest::TAG => Box::new(HandoffRequest::from_binary(msg_data)),
                            HandoffAccept::TAG => Box::new(HandoffAccept::from_binary(msg_data)),
                            HandoffReject::TAG => Box::new(HandoffReject::from_binary(msg_data)),
                            GhostUpdate::TAG => Box::new(GhostUpdate::from_binary(msg_data)),
                            HandoffComplete::TAG => Box::new(HandoffComplete::from_binary(msg_data)),
                            0x55 => { //format min_x min_y max_x max_y en float32
                                println!("Receive my working zone");

                                if msg_data.len() != 20 {
                                    println!("Invalid working zone data length : {}, expected 16", msg_data.len());
                                    return;
                                }

                                let shard_id = u32::from_le_bytes([msg_data[0], msg_data[1], msg_data[2], msg_data[3]]);
                                let min_x = f32::from_le_bytes([msg_data[4], msg_data[5], msg_data[6], msg_data[7]]);
                                let min_y = f32::from_le_bytes([msg_data[8], msg_data[9], msg_data[10], msg_data[11]]);
                                let max_x = f32::from_le_bytes([msg_data[12], msg_data[13], msg_data[14], msg_data[15]]);
                                let max_y = f32::from_le_bytes([msg_data[16], msg_data[17], msg_data[18], msg_data[19]]);

                                println!("Working zone received : min_x : {}, min_y : {}, max_x : {}, max_y : {}", min_x, min_y, max_x, max_y);

                                server_config.zone = shard_id.to_string();
                                server_config.min_x = min_x;
                                server_config.min_y = min_y;
                                server_config.max_x = max_x;
                                server_config.max_y = max_y;

                                server_config.state = ServerState::Running;
                                println!("Server state set to Running");

                                //send info to broker format : ServerID (u32), min_x, max_x, min_y, max_y (f32)
                                let mut info_packet = Vec::new();
                                info_packet.push(0x00); // Tag 0x00 for server info update
                                info_packet.extend_from_slice(&shard_id.to_le_bytes());
                                info_packet.extend_from_slice(&min_x.to_le_bytes());
                                info_packet.extend_from_slice(&min_y.to_le_bytes());
                                info_packet.extend_from_slice(&max_x.to_le_bytes());
                                info_packet.extend_from_slice(&max_y.to_le_bytes());

                                if let (Some(broker_conn), Some(broker_stream)) = (&socket.connection_broker, &socket.stream_broker) {
                                    match socket.peer.send(broker_conn, broker_stream, bytes::Bytes::from(info_packet)) {
                                        Ok(_) => println!("Server info sent to Broker"),
                                        Err(e) => println!("Failed to send server info to Broker: {:?}", e),
                                    }
                                } else {
                                    println!("WARNING : Broker connection or stream not established yet, cannot send server info");
                                }

                                return;
                            },
                            _ => {
                                println!("Received message with unknown tag : {}, from connection id : {}, with stream id : {}", msg_tag, connection.connection_id, stream.stream_id);
                                return;
                            },
                        };
                        
                        msg.resolve(&mut player_registry, &mut server_config, &socket, connection, stream.clone());
                    }
                }

                //Message du Broker
                if let Some(broker_conn) = &socket.connection_broker {
                    if (connection.connection_id == broker_conn.connection_id) {
                        let msg_tag: u8 = data[0];
                        let msg_data = &data[1..];

                        match msg_tag {
                            0x05 => { //Traitement des input player, format : InputClient de shared
                                if let Ok(input) = serde_json::from_slice::<ClientInput>(msg_data) {
                                    println!("Received input from client_id : {}, sequence_id : {}, input : {:?}", input.client_id, input.sequence_id, input.input);
                                    player_registry.update_player_input(input.client_id, input.input);
                                } else {
                                    println!("Received invalid client input message");
                                }
                            },
                            TAG_ADMIN_ROUTE_RECEIVE => {
                                if msg_data.is_empty() { return; }

                                let internal_tag = msg_data[0];
                                let payload = &msg_data[1..];

                                match internal_tag {
                                    0x07 => {
                                        println!("Receive new player via admin");

                                        if payload.len() != 12 {
                                            println!("Invalid new player data length : {}, expected 12", payload.len());
                                            return;
                                        }

                                        let client_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                                        let init_x = f32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
                                        let init_y = f32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
                                        println!("New player received : client_id : {}, init_x : {}, init_y : {}", client_id, init_x, init_y);

                                        let new_player_entity = PlayerEntity {
                                            id: client_id,
                                            authority: EntityAuthority::PendingHandoff,
                                            position: Vec2::new(init_x, init_y),
                                            rotation: 0.0,
                                            velocity: Vec2::ZERO,
                                        };

                                        player_registry.register_player(new_player_entity);
                                        println!("Player registered in registry, current player count : {}", player_registry.players.len());
                                    },

                                    // TODO : PAYLOAD_CROSSING_ALERT 
                                    _ => {
                                        println!("Unknown internal tag in Admin: {}", internal_tag);
                                    }
                                }
                            },
                            _ => {
                                println!("Received message with unknown tag : {}, from connection id : {}, with stream id : {}, data : {:?}", msg_tag, connection.connection_id, stream.stream_id, msg_data);
                            }
                        }
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
                if let Some(broker_conn) = &socket.connection_broker.clone() {
                    if (connection.connection_id == broker_conn.connection_id) {
                        socket.stream_broker = Some(stream.clone());
                        println!("Stream created with Broker, connection id : {}, stream id : {}", connection.connection_id, stream.stream_id);

                        let mut auth_packet = Vec::new();
                        auth_packet.push(TAG_ADMIN_CONNECT); // Tag 0x0A

                        // Le DGS s'identifie avec son UUID unique (ex: "dgs_550e8400...")
                        let name_str = format!("dgs_{}", server_config.id);
                        let mut name_bytes = [0u8; 32];
                        let bytes = name_str.as_bytes();
                        name_bytes[..bytes.len().min(32)].copy_from_slice(&bytes[..bytes.len().min(32)]);
                        auth_packet.extend_from_slice(&name_bytes);

                        let _ = socket.peer.send(broker_conn, &stream, bytes::Bytes::from(auth_packet));
                        println!("Identification VIP envoyée au Broker : {}", name_str);

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
    mut socket: ResMut<GameSocket>,
    registry: Res<PlayerRegistry>,
    config: Res<ServerConfig>,
) {
    // Pour chaque joueur dont on a l'autorité
    for (player_id, player) in registry.players.iter() {
        if !matches!(player.authority, EntityAuthority::Owned | EntityAuthority::PendingHandoff) {
            continue;
        }

        // Construire le payload : id + position + vélocité
        let mut payload = Vec::new();
        payload.extend_from_slice(&player_id.to_le_bytes());           // 4 bytes
        payload.extend_from_slice(&player.position.x.to_le_bytes());   // 4 bytes
        payload.extend_from_slice(&player.position.y.to_le_bytes());   // 4 bytes
        payload.extend_from_slice(&player.velocity.x.to_le_bytes());   // 4 bytes
        payload.extend_from_slice(&player.velocity.y.to_le_bytes());   // 4 bytes
        // Total : 20 bytes

        // Construire le topic (32 bytes paddés avec des \0)
        let topic_str = format!("s{}p{}", config.id, player_id);
        let mut topic_bytes = [0u8; 32];
        let bytes = topic_str.as_bytes();
        let len = bytes.len().min(32);
        topic_bytes[..len].copy_from_slice(&bytes[..len]);

        // Construire le message complet 0x03 (Publish)
        let mut msg = Vec::new();
        msg.push(0x03u8);
        msg.extend_from_slice(&topic_bytes);
        msg.extend_from_slice(&(payload.len() as u16).to_le_bytes());
        msg.extend_from_slice(&payload);

        // Envoyer au broker
        if let (Some(conn), Some(stream)) = (&socket.connection_broker, &socket.stream_broker) {
            let _ = socket.peer.send(conn, stream, bytes::Bytes::from(msg));
        }
    }
}