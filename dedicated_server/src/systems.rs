use bevy::prelude::*;
use game_sockets::{GamePeer, GameNetworkEvent, GameStreamReliability};
use game_sockets::protocols::QuicBackend;
use crate::resources::*;
use crate::entities::*;
use crate::message::*;
use shared::*;
use std::net::ToSocketAddrs;
use bincode;

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
                            0x07 => {
                                println!("Demande de Spawn (ClientInit) binaire relayée par le Broker !");

                                match bincode::deserialize::<ClientInit>(msg_data) {
                                    Ok(client_init) => {
                                        let new_player = PlayerEntity {
                                            id: client_init.client_id,
                                            authority: EntityAuthority::Owned,
                                            position: Vec2::new(client_init.pos_x, client_init.pos_y),
                                            rotation: 0.0,
                                            velocity: Vec2::ZERO,
                                            involved_shards: Vec::new(),
                                        };

                                        player_registry.register_player(new_player);
                                        println!("Joueur {} apparu avec succès en ({}, {})",
                                                 client_init.client_id, client_init.pos_x, client_init.pos_y
                                        );
                                    },
                                    Err(e) => println!("Erreur bincode ClientInit : {}", e),
                                }
                            },

                            0x05 => {
                                match bincode::deserialize::<ClientInput>(msg_data) {
                                    Ok(input) => {
                                        // On met à jour la physique sans inonder la console
                                        player_registry.update_player_input(input.client_id, input.input);
                                    },
                                    Err(e) => println!(" Erreur bincode ClientInput : {}", e),
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
                                            involved_shards: Vec::new(),
                                        };

                                        player_registry.register_player(new_player_entity);
                                        println!("Player registered in registry, current player count : {}", player_registry.players.len());
                                    },

                                    PAYLOAD_WAKEUP_COMMAND => {
                                        println!("Message de réveil (WakeUp) reçu via le Broker");

                                        // 4 octets (ID) + 16 octets (coordonnées f32) = 20 octets minimum attendus
                                        if payload.len() < 20 {
                                            println!("Invalid WakeUp data length: {}, expected at least 20", payload.len());
                                            return;
                                        }

                                        // Note: `payload` commence APRÈS le tag, donc l'index 0 est le début du shard_id
                                        let shard_id = u32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                                        let min_x = f32::from_le_bytes([payload[4], payload[5], payload[6], payload[7]]);
                                        let min_y = f32::from_le_bytes([payload[8], payload[9], payload[10], payload[11]]);
                                        let max_x = f32::from_le_bytes([payload[12], payload[13], payload[14], payload[15]]);
                                        let max_y = f32::from_le_bytes([payload[16], payload[17], payload[18], payload[19]]);

                                        println!("Working zone received : Shard: {}, min_x : {}, min_y : {}, max_x : {}, max_y : {}", shard_id, min_x, min_y, max_x, max_y);

                                        // Mise à jour du serveur global
                                        server_config.zone = shard_id.to_string();
                                        server_config.min_x = min_x;
                                        server_config.min_y = min_y;
                                        server_config.max_x = max_x;
                                        server_config.max_y = max_y;
                                        server_config.state = ServerState::Running;

                                        println!("Server state set to Running for Shard {}", shard_id);

                                        // previen le brocker quon est le "shard:X"
                                        let mut auth_packet = Vec::new();
                                        auth_packet.push(TAG_ADMIN_CONNECT); // Tag 0x0A

                                        let name_str = format!("shard:{}", shard_id);
                                        let mut name_bytes = [0u8; 32];
                                        let bytes = name_str.as_bytes();
                                        name_bytes[..bytes.len().min(32)].copy_from_slice(&bytes[..bytes.len().min(32)]);
                                        auth_packet.extend_from_slice(&name_bytes);

                                        if let (Some(broker_conn), Some(broker_stream)) = (&socket.connection_broker, &socket.stream_broker) {
                                            let _ = socket.peer.send(broker_conn, broker_stream, bytes::Bytes::from(auth_packet));
                                            println!("Identité mise à jour sur le Broker : {}", name_str);
                                        }

                                        std::thread::sleep(std::time::Duration::from_millis(50));

                                        let mut bounds_packet = Vec::new();
                                        bounds_packet.push(0x00); // Tag 0x00 for server info update
                                        bounds_packet.extend_from_slice(&shard_id.to_le_bytes());
                                        bounds_packet.extend_from_slice(&min_x.to_le_bytes());
                                        bounds_packet.extend_from_slice(&min_y.to_le_bytes());
                                        bounds_packet.extend_from_slice(&max_x.to_le_bytes());
                                        bounds_packet.extend_from_slice(&max_y.to_le_bytes());

                                        if let (Some(broker_conn), Some(broker_stream)) = (&socket.connection_broker, &socket.stream_broker) {
                                            let _ = socket.peer.send(broker_conn, broker_stream, bytes::Bytes::from(bounds_packet));
                                            println!("Frontières envoyées au Broker pour le routage des joueurs !");
                                        }
                                    },

                                    PAYLOAD_CROSSING_ALERT => {
                                        println!("Alerte de Frontière (CrossingAlert) reçue via le tunnel");

                                        // Note: la methode from byte est dans lib
                                        if let Some(alert_data) = CrossingAlertData::from_bytes(msg_data) {
                                            println!(
                                                "Le joueur {} s'approche des bordures. Shards impliqués : {:?}",
                                                alert_data.client_id,
                                                alert_data.involved_shards
                                            );

                                            if let Some(player) = player_registry.players.get_mut(&alert_data.client_id) {
                                                if matches!(player.authority, EntityAuthority::Owned) {
                                                    //active l'émission des GhostUpdates
                                                    player.authority = EntityAuthority::PendingHandoff;
                                                    player.involved_shards = alert_data.involved_shards.clone();
                                                    println!("Joueur {} proche de la frontière : passage en PendingHandoff", alert_data.client_id);
                                                }
                                            } else {
                                                //autre serveur : on lui prépare un réceptacle vide
                                                let ghost = PlayerEntity {
                                                    id: alert_data.client_id,
                                                    authority: EntityAuthority::Ghost,
                                                    position: Vec2::ZERO, // Sera corrigé instantanément par le premier GhostUpdate (normalement)
                                                    rotation: 0.0,
                                                    velocity: Vec2::ZERO,
                                                    involved_shards: Vec::new(),
                                                };
                                                player_registry.register_player(ghost);
                                                println!("Fantôme préparé pour le joueur {} venant d'une autre zone", alert_data.client_id);
                                            }

                                        } else {
                                            println!("Erreur : Impossible de désérialiser le CrossingAlert (données corrompues ou incomplètes).");
                                        }
                                    },
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

        let mut buffer = Vec::new();
        buffer.push(TAG_ADMIN_ROUTE_SEND); // Tag 0x0B

        // La cible est le Serveur Spatial (sur 32 octets)
        let mut target_bytes = [0u8; 32];
        let target_str = b"spatial";
        target_bytes[..target_str.len()].copy_from_slice(target_str);
        buffer.extend_from_slice(&target_bytes);

        let mut payload = Vec::new();
        payload.push(0x10); // Le Tag de position attendu par le Serveur Spatial

        // Le Spatial lit exactement 12 octets : ID (4) + X (4) + Y (4)
        payload.extend_from_slice(&player_id.to_le_bytes());
        payload.extend_from_slice(&player.position.x.to_le_bytes());
        payload.extend_from_slice(&player.position.y.to_le_bytes());

        let payload_len = payload.len() as u16;
        buffer.extend_from_slice(&payload_len.to_le_bytes());
        buffer.extend_from_slice(&payload);

        // Envoyer au Broker via le stream fiable
        if let (Some(conn), Some(stream)) = (&socket.connection_broker, &socket.stream_broker) {
            let _ = socket.peer.send(conn, stream, bytes::Bytes::from(buffer));
        }
    }
}

pub fn process_handoffs(
    socket: Res<GameSocket>,
    mut registry: ResMut<PlayerRegistry>,
    config: Res<ServerConfig>,
) {
    let mut handoff_triggered = Vec::new();
    let my_shard_id = config.zone.parse::<u32>().unwrap_or(0);

    for player in registry.players.values() {
        if matches!(player.authority, EntityAuthority::PendingHandoff) {
            // Est-il physiquement sorti de la zone de simu ?
            if player.position.x < config.min_x || player.position.x > config.max_x ||
                player.position.y < config.min_y || player.position.y > config.max_y {
                handoff_triggered.push(player.clone());
            }
        }
    }

    for player in handoff_triggered {
        println!("Joueur {} hors frontières. Envoi du HandoffRequest à l'Orchestrateur.", player.id);

        let req = HandoffRequest {
            entity_id: player.id,
            source_shard: my_shard_id,
            pos: player.position,
            vel: player.velocity,
            state: [0; 64],
        };

        // On utilise ta fonction native qui pointe vers socket.connection_orch
        send_inter_shards_packet(&socket, Box::new(req));

        // Passage en Ghost local
        if let Some(p) = registry.players.get_mut(&player.id) {
            p.authority = EntityAuthority::Ghost;
        }
    }
}