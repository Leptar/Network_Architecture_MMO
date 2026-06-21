use bevy::prelude::*;
use game_sockets::{GamePeer, protocols::QuicBackend, GameNetworkEvent};
use shared::*;
use crate::resources::*;

pub fn bind_socket(mut commands: Commands) {
    let peer = GamePeer::new(QuicBackend::new());

    peer.listen("0.0.0.0", shared::BROK_PORT).unwrap();
    commands.insert_resource(BrokerSocket { peer });
    println!("Broker démarré sur le port {}", shared::BROK_PORT);
}

pub fn receive_messages(
    mut socket: ResMut<BrokerSocket>,
    mut clients: ResMut<ClientRegistry>,
    mut subs: ResMut<SubscriptionMap>,
    mut shard_map: ResMut<ShardMap>,
    mut admin_registry: ResMut<AdminRegistry>
) {
    while let Ok(Some(event)) = socket.peer.poll() {
        match event {
            GameNetworkEvent::Message { connection, stream, data } => {
                if data.is_empty() { continue; }

                let tag = data[0];
                let rest = &data[1..];

                match tag {
                    0x00 => {
                        if rest.len() < 20 { return; }

                        // Lire le shard_id (4 bytes)
                        let shard_id = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);

                        // Lire les 4 floats (4 bytes chacun)
                        let min_x = f32::from_le_bytes([rest[4],  rest[5],  rest[6],  rest[7]]);
                        let min_y = f32::from_le_bytes([rest[8],  rest[9],  rest[10], rest[11]]);
                        let max_x = f32::from_le_bytes([rest[12], rest[13], rest[14], rest[15]]);
                        let max_y = f32::from_le_bytes([rest[16], rest[17], rest[18], rest[19]]);

                        shard_map.shard_connections.insert(shard_id, DGSNetworkInfo {
                            min_x,
                            max_x,
                            min_y,
                            max_y,
                            connection_dgs: Some(connection),
                            stream_dgs: Some(stream.clone()),
                        });

                        println!(
                            "Shard {} enregistré : x[{}, {}] y[{}, {}]",
                            shard_id, min_x, max_x, min_y, max_y
                        );
                    }
                    0x01 => {
                        if rest.len() < 36 { continue; }

                        let client_id = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
                        let topic = String::from_utf8_lossy(&rest[4..36])
                            .trim_matches('\0')
                            .to_string();

                        let subscribers = subs.subscriptions
                            .entry(topic.clone())
                            .or_insert_with(Vec::new);

                        if !subscribers.contains(&client_id) {
                            subscribers.push(client_id);
                        }

                        println!("Client {} abonné à '{}'", client_id, topic);
                    }
                    0x02 => {
                        if rest.len() < 36 { continue; }

                        let client_id = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
                        let topic = String::from_utf8_lossy(&rest[4..36])
                            .trim_matches('\0')
                            .to_string();

                        if let Some(subscribers) = subs.subscriptions.get_mut(&topic) {
                            subscribers.retain(|&id| id != client_id);
                        }

                        println!("Client {} désabonné de '{}'", client_id, topic);
                    }
                    0x03 => {
                        if rest.len() < 34 { continue; }
                        
                        let topic = String::from_utf8_lossy(&rest[0..32])
                            .trim_matches('\0')
                            .to_string();

                        // Lire payload_len (2 bytes)
                        let payload_len = u16::from_le_bytes([rest[32], rest[33]]) as usize;

                        // Lire le payload
                        if rest.len() < 34 + payload_len { continue; }
                        let payload = &rest[34..34 + payload_len];

                        println!("Publish sur '{}' ({} bytes)", topic, payload_len);

                        // Construire le message Broadcast (0x04)
                        let mut broadcast = Vec::with_capacity(1 + 2 + payload_len);
                        broadcast.push(0x04);
                        broadcast.extend_from_slice(&(payload_len as u16).to_le_bytes());
                        broadcast.extend_from_slice(payload);
                        let broadcast = bytes::Bytes::from(broadcast);

                        // Envoyer à tous les abonnés
                        if let Some(subscribers) = subs.subscriptions.get(&topic) {
                            for &client_id in subscribers {
                                if let Some(info) = clients.clients.get(&client_id) {
                                    let _ = socket.peer.send(
                                        &info.broker_to_client_connexion,
                                        &stream,
                                        broadcast.clone()
                                    );
                                }
                            }
                        }
                    }
                    0x05 => {
                        if rest.len() < 20 { continue; }

                        let client_id = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
                        let input = &rest[4..20];

                        if let Some(client_info) = clients.clients.get(&client_id) {
                            if let (Some(shard_conn), Some(shard_stream)) = (
                                &client_info.broker_to_server_connexion,
                                &client_info.broker_to_server_stream
                            ) {
                                let mut msg = Vec::new();
                                msg.push(0x05);
                                msg.extend_from_slice(&client_id.to_le_bytes());
                                msg.extend_from_slice(input);

                                let _ = socket.peer.send(
                                    shard_conn,
                                    shard_stream,
                                    bytes::Bytes::from(msg)
                                );

                                println!("Input client {} relayé au DGS", client_id);
                            }
                        }
                    }
                    0x07 => {
                        // Désérialiser le ClientInit
                        let client_init: shared::ClientInit = match bincode::deserialize(rest) {
                            Ok(ci) => ci,
                            Err(e) => {
                                println!("Erreur désérialisation ClientInit : {}", e);
                                return;
                            }
                        };

                        // Attribuer un client_id (le client a envoyé 0 en placeholder)
                        let client_id = clients.next_id;
                        clients.next_id += 1;

                        println!(
                            "Nouveau client {} à position ({}, {})",
                            client_id, client_init.pos_x, client_init.pos_y
                        );

                        // Trouver le shard responsable de cette position
                        let mut shard_info: Option<(game_sockets::GameConnection, game_sockets::GameStream)> = None;
                        for (shard_id, dgs) in shard_map.shard_connections.iter() {
                            if client_init.pos_x >= dgs.min_x && client_init.pos_x <= dgs.max_x
                                && client_init.pos_y >= dgs.min_y && client_init.pos_y <= dgs.max_y {
                                if let (Some(conn), Some(s)) = (&dgs.connection_dgs, &dgs.stream_dgs) {
                                    shard_info = Some((conn.clone(), s.clone()));
                                    println!("Client {} assigné au shard {}", client_id, shard_id);
                                    break;
                                }
                            }
                        }

                        // Enregistrer le client dans ClientRegistry
                        clients.clients.insert(client_id, ClientNetworkInfo {
                            broker_to_client_connexion: connection,
                            broker_to_client_stream: stream.clone(),
                            broker_to_server_connexion: shard_info.as_ref().map(|(c, _)| c.clone()),
                            broker_to_server_stream: shard_info.as_ref().map(|(_, s)| s.clone()),
                        });

                        // 1. Envoyer 0x06 au CLIENT avec son client_id
                        let mut response = Vec::new();
                        response.push(0x06u8);
                        response.extend_from_slice(&client_id.to_le_bytes());
                        let _ = socket.peer.send(&connection, &stream, bytes::Bytes::from(response));

                        // 2. Relayer le ClientInit au DGS avec le vrai client_id
                        if let Some((shard_conn, shard_stream)) = shard_info {
                            let updated_init = shared::ClientInit {
                                client_id,
                                pos_x: client_init.pos_x,
                                pos_y: client_init.pos_y,
                            };

                            let serialized = bincode::serialize(&updated_init)
                                .expect("Failed to serialize ClientInit");

                            let mut data = vec![0x07u8];
                            data.extend_from_slice(&serialized);

                            let _ = socket.peer.send(
                                &shard_conn,
                                &shard_stream,
                                bytes::Bytes::from(data)
                            );

                            println!("ClientInit relayé au DGS pour client {}", client_id);
                        } else {
                            println!("Aucun shard trouvé pour le client {} à ({}, {})",
                                     client_id, client_init.pos_x, client_init.pos_y);
                        }
                    }
                    TAG_ADMIN_CONNECT => {
                        if rest.len() < 32 { continue; }

                        // Le serveur envoie son nom. le lit et l'ajoute à la liste des services.
                        let admin_name = String::from_utf8_lossy(&rest[0..32]).trim_matches('\0').to_string();
                        let admin_network_info: AdminNetworkInfo = AdminNetworkInfo{
                            connection_admin: connection,
                            stream_admin: stream.clone(),
                        };

                        admin_registry.admins.insert(admin_name.clone(), admin_network_info);
                        println!("🛡️ Serveur d'infrastructure connecté : {}", admin_name);
                    }

                    // Un service demande à envoyer un message privé à un autre
                    TAG_ADMIN_ROUTE_SEND => {
                        if rest.len() < 34 { continue; }

                        let target_name = String::from_utf8_lossy(&rest[0..32]).trim_matches('\0').to_string();

                        // Lire payload_len (2 bytes)
                        let payload_len = u16::from_le_bytes([rest[32], rest[33]]) as usize;

                        // Lire le payload
                        if rest.len() < 34 + payload_len { continue; }
                        let payload = &rest[34..34 + payload_len];

                        if let Some(target_info) = admin_registry.admins.get(&target_name) {
                            let mut direct_msg = Vec::with_capacity(1 + payload.len());
                            direct_msg.push(TAG_ADMIN_ROUTE_RECEIVE); // Tag 0x0C
                            direct_msg.extend_from_slice(payload);

                            let _ = socket.peer.send(
                                &target_info.connection_admin,
                                &target_info.stream_admin,
                                bytes::Bytes::from(direct_msg) // On envoie le message ré-emballé
                            );
                            /*println!("Message privé envoyé à {} ({} bytes)", target_name, payload_len);*/
                        } else {
                            println!("Serveur cible '{}' introuvable pour message privé", target_name);
                        }
                    }

                    _ => { println!("Tag inconnu : {:#x}", tag); }
                }
            }
            GameNetworkEvent::Connected(conn) => {
                println!("Nouvelle connexion ({}), en attente du tag d'identification...", conn.connection_id);
            }
            _ => {}
        }
    }
}